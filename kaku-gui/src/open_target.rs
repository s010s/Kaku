use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::Context;
use config::DefaultOpenTarget;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileOpenTarget {
    pub path: PathBuf,
    pub line: Option<usize>,
    pub col: Option<usize>,
    pub is_dir: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenCommand {
    pub program: String,
    pub args: Vec<String>,
}

pub trait CommandRunner {
    fn run(&mut self, command: &OpenCommand) -> anyhow::Result<bool>;
}

pub struct ProcessCommandRunner;

impl CommandRunner for ProcessCommandRunner {
    fn run(&mut self, command: &OpenCommand) -> anyhow::Result<bool> {
        let status = Command::new(&command.program)
            .args(&command.args)
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status();

        match status {
            Ok(status) if status.success() => Ok(true),
            Ok(status) => {
                log::debug!("`{}` exited with status {status}", command.program);
                Ok(false)
            }
            Err(err) => {
                log::debug!(
                    "open target command `{}` failed to launch: {err}",
                    command.program
                );
                Ok(false)
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EditorEnv {
    pub visual: Option<String>,
    pub editor: Option<String>,
}

impl EditorEnv {
    pub fn current() -> Self {
        Self {
            visual: std::env::var_os("VISUAL").map(|value| value.to_string_lossy().into_owned()),
            editor: std::env::var_os("EDITOR").map(|value| value.to_string_lossy().into_owned()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocationStyle {
    GotoFlag,
    PathSuffix,
    LineColumnFlags,
    PathOnly,
}

pub fn terminal_cwd(target: &FileOpenTarget) -> PathBuf {
    if target.is_dir {
        return target.path.clone();
    }

    target
        .path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .map(Path::to_path_buf)
        .unwrap_or_else(|| target.path.clone())
}

pub fn build_cli_location_command(
    program: &str,
    style: LocationStyle,
    target: &FileOpenTarget,
) -> OpenCommand {
    let args = match style {
        LocationStyle::GotoFlag => match location_arg(target) {
            Some(location) => vec!["-g".to_string(), location],
            None => vec![path_arg(&target.path)],
        },
        LocationStyle::PathSuffix => match location_arg(target) {
            Some(location) => vec![location],
            None => vec![path_arg(&target.path)],
        },
        LocationStyle::LineColumnFlags => {
            let Some(line) = target.line else {
                return OpenCommand {
                    program: program.to_string(),
                    args: vec![path_arg(&target.path)],
                };
            };

            let mut args = vec!["--line".to_string(), line.to_string()];
            if let Some(col) = target.col {
                args.push("--column".to_string());
                args.push(col.to_string());
            }
            args.push(path_arg(&target.path));
            args
        }
        LocationStyle::PathOnly => vec![path_arg(&target.path)],
    };

    OpenCommand {
        program: program.to_string(),
        args,
    }
}

pub fn build_finder_command(target: &FileOpenTarget) -> OpenCommand {
    let path = path_arg(&target.path);
    let args = if target.is_dir {
        vec![path]
    } else {
        vec!["-R".to_string(), path]
    };

    OpenCommand {
        program: "/usr/bin/open".to_string(),
        args,
    }
}

pub fn build_default_app_command(target: &FileOpenTarget) -> OpenCommand {
    OpenCommand {
        program: "/usr/bin/open".to_string(),
        args: vec![path_arg(&target.path)],
    }
}

pub fn build_wezterm_command(program: &str, target: &FileOpenTarget) -> OpenCommand {
    OpenCommand {
        program: program.to_string(),
        args: vec![
            "start".to_string(),
            "--cwd".to_string(),
            path_arg(&terminal_cwd(target)),
        ],
    }
}

const AUTO_FALLBACK_TARGETS: &[DefaultOpenTarget] = &[
    DefaultOpenTarget::Cursor,
    DefaultOpenTarget::Windsurf,
    DefaultOpenTarget::Kiro,
    DefaultOpenTarget::Antigravity,
    DefaultOpenTarget::Zed,
    DefaultOpenTarget::IntelliJIdea,
    DefaultOpenTarget::VsCode,
    DefaultOpenTarget::DefaultApp,
    DefaultOpenTarget::Finder,
];

pub fn commands_for_target(
    configured: DefaultOpenTarget,
    target: &FileOpenTarget,
) -> Vec<OpenCommand> {
    match configured {
        DefaultOpenTarget::Auto => Vec::new(),
        DefaultOpenTarget::VsCode => editor_target_commands(
            "code",
            "Visual Studio Code",
            LocationStyle::GotoFlag,
            &[
                "/usr/local/bin/code",
                "/opt/homebrew/bin/code",
                "/opt/local/bin/code",
                "/Applications/Visual Studio Code.app/Contents/Resources/app/bin/code",
            ],
            target,
        ),
        DefaultOpenTarget::Cursor => editor_target_commands(
            "cursor",
            "Cursor",
            LocationStyle::GotoFlag,
            &[
                "/usr/local/bin/cursor",
                "/opt/homebrew/bin/cursor",
                "/opt/local/bin/cursor",
                "/Applications/Cursor.app/Contents/Resources/app/bin/cursor",
            ],
            target,
        ),
        DefaultOpenTarget::Windsurf => editor_target_commands(
            "windsurf",
            "Windsurf",
            LocationStyle::GotoFlag,
            &[
                "/usr/local/bin/windsurf",
                "/opt/homebrew/bin/windsurf",
                "/opt/local/bin/windsurf",
                "/Applications/Windsurf.app/Contents/Resources/app/bin/windsurf",
            ],
            target,
        ),
        DefaultOpenTarget::Kiro => editor_target_commands(
            "kiro",
            "Kiro",
            LocationStyle::PathOnly,
            &[
                "/usr/local/bin/kiro",
                "/opt/homebrew/bin/kiro",
                "/opt/local/bin/kiro",
                "/Applications/Kiro.app/Contents/Resources/app/bin/kiro",
            ],
            target,
        ),
        DefaultOpenTarget::Antigravity => antigravity_target_commands(target),
        DefaultOpenTarget::Zed => editor_target_commands(
            "zed",
            "Zed",
            LocationStyle::PathSuffix,
            &[
                "/usr/local/bin/zed",
                "/opt/homebrew/bin/zed",
                "/opt/local/bin/zed",
                "/Applications/Zed.app/Contents/MacOS/cli",
                "/Applications/Zed.app/Contents/MacOS/zed",
            ],
            target,
        ),
        DefaultOpenTarget::IntelliJIdea => editor_target_commands(
            "idea",
            "IntelliJ IDEA",
            LocationStyle::LineColumnFlags,
            &[
                "/usr/local/bin/idea",
                "/opt/homebrew/bin/idea",
                "/opt/local/bin/idea",
                "/Applications/IntelliJ IDEA.app/Contents/MacOS/idea",
            ],
            target,
        ),
        DefaultOpenTarget::DefaultApp => vec![build_default_app_command(target)],
        DefaultOpenTarget::Finder => vec![build_finder_command(target)],
        DefaultOpenTarget::Terminal => vec![build_terminal_app_command("Terminal", target)],
        DefaultOpenTarget::ITerm2 => vec![build_terminal_app_command("iTerm", target)],
        DefaultOpenTarget::Ghostty => vec![build_terminal_app_command("Ghostty", target)],
        DefaultOpenTarget::WezTerm => wezterm_target_commands(target),
        DefaultOpenTarget::Cmux => cmux_target_commands(target),
    }
}

pub fn open_with_runner(
    configured: DefaultOpenTarget,
    target: &FileOpenTarget,
    runner: &mut dyn CommandRunner,
) -> anyhow::Result<()> {
    open_with_runner_and_env(configured, target, EditorEnv::current(), runner)
}

fn open_with_runner_and_env(
    configured: DefaultOpenTarget,
    target: &FileOpenTarget,
    env: EditorEnv,
    runner: &mut dyn CommandRunner,
) -> anyhow::Result<()> {
    if configured == DefaultOpenTarget::Auto {
        return open_auto_with_runner(target, env, runner);
    }

    let mut failed_commands = Vec::new();
    for command in commands_for_target(configured, target) {
        if runner.run(&command)? {
            return Ok(());
        }
        failed_commands.push(command);
    }

    log::debug!(
        "configured open target {} did not launch; falling back to auto",
        configured.as_str()
    );
    open_auto_with_runner_skipping_failed_commands(
        target,
        env,
        runner,
        Some(configured),
        &failed_commands,
    )
}

pub fn open_auto_with_runner(
    target: &FileOpenTarget,
    env: EditorEnv,
    runner: &mut dyn CommandRunner,
) -> anyhow::Result<()> {
    open_auto_with_runner_skipping(target, env, runner, None)
}

fn open_auto_with_runner_skipping(
    target: &FileOpenTarget,
    env: EditorEnv,
    runner: &mut dyn CommandRunner,
    skip: Option<DefaultOpenTarget>,
) -> anyhow::Result<()> {
    open_auto_with_runner_skipping_failed_commands(target, env, runner, skip, &[])
}

fn open_auto_with_runner_skipping_failed_commands(
    target: &FileOpenTarget,
    env: EditorEnv,
    runner: &mut dyn CommandRunner,
    skip: Option<DefaultOpenTarget>,
    initial_failed_commands: &[OpenCommand],
) -> anyhow::Result<()> {
    let mut failed_commands = initial_failed_commands.to_vec();

    for raw in [env.visual.as_deref(), env.editor.as_deref()]
        .iter()
        .copied()
        .flatten()
    {
        match build_editor_env_command(raw, target) {
            Ok(command) => {
                if command_was_attempted(&command, &failed_commands)
                    || skip.is_some_and(|configured| {
                        command_targets_configured(&command, configured, target)
                    })
                {
                    log::debug!(
                        "skipping env editor command for failed configured open target: {:?}",
                        command
                    );
                    continue;
                }

                if runner.run(&command)? {
                    return Ok(());
                }
                failed_commands.push(command);
            }
            Err(err) => log::warn!("skipping invalid editor command `{raw}`: {err:#}"),
        }
    }

    for configured in AUTO_FALLBACK_TARGETS {
        if Some(*configured) == skip {
            continue;
        }

        for command in commands_for_target(*configured, target) {
            if command_was_attempted(&command, &failed_commands) {
                log::debug!(
                    "skipping repeated failed open target command: {:?}",
                    command
                );
                continue;
            }

            if runner.run(&command)? {
                return Ok(());
            }
            failed_commands.push(command);
        }
    }

    anyhow::bail!("failed to open {}", target.path.display())
}

fn editor_target_commands(
    cli_name: &str,
    app_name: &str,
    style: LocationStyle,
    app_cli_paths: &[&str],
    target: &FileOpenTarget,
) -> Vec<OpenCommand> {
    let mut commands = vec![build_cli_location_command(cli_name, style, target)];

    for cli_path in app_cli_paths {
        commands.push(build_cli_location_command(cli_path, style, target));
        if let Some(user_cli_path) = user_application_cli_path(cli_path) {
            commands.push(build_cli_location_command(&user_cli_path, style, target));
        }
    }

    commands.push(build_open_app_command(app_name, target));
    commands
}

fn antigravity_target_commands(target: &FileOpenTarget) -> Vec<OpenCommand> {
    let style = LocationStyle::GotoFlag;
    let mut commands = vec![build_cli_location_command("antigravity", style, target)];

    if let Some(cli_path) = home_relative_cli_path(".antigravity/antigravity/bin/antigravity") {
        commands.push(build_cli_location_command(&cli_path, style, target));
    }

    for cli_path in [
        "/usr/local/bin/antigravity",
        "/opt/homebrew/bin/antigravity",
        "/opt/local/bin/antigravity",
        "/Applications/Antigravity.app/Contents/Resources/app/bin/antigravity",
    ] {
        commands.push(build_cli_location_command(cli_path, style, target));
        if let Some(user_cli_path) = user_application_cli_path(cli_path) {
            commands.push(build_cli_location_command(&user_cli_path, style, target));
        }
    }

    commands.push(build_open_app_command("Antigravity", target));
    commands
}

fn build_open_app_command(app_name: &str, target: &FileOpenTarget) -> OpenCommand {
    OpenCommand {
        program: "/usr/bin/open".to_string(),
        args: vec![
            "-a".to_string(),
            app_name.to_string(),
            path_arg(&target.path),
        ],
    }
}

fn build_terminal_path_command(program: &str, target: &FileOpenTarget) -> OpenCommand {
    OpenCommand {
        program: program.to_string(),
        args: vec![path_arg(&terminal_cwd(target))],
    }
}

fn wezterm_target_commands(target: &FileOpenTarget) -> Vec<OpenCommand> {
    let mut commands = vec![build_wezterm_command("wezterm", target)];

    for cli_path in [
        "/usr/local/bin/wezterm",
        "/opt/homebrew/bin/wezterm",
        "/opt/local/bin/wezterm",
        "/Applications/WezTerm.app/Contents/MacOS/wezterm",
    ] {
        commands.push(build_wezterm_command(cli_path, target));
        if let Some(user_cli_path) = user_application_cli_path(cli_path) {
            commands.push(build_wezterm_command(&user_cli_path, target));
        }
    }

    commands.push(build_terminal_app_command("WezTerm", target));
    commands
}

fn cmux_target_commands(target: &FileOpenTarget) -> Vec<OpenCommand> {
    let mut commands = vec![build_terminal_path_command("cmux", target)];

    for cli_path in [
        "/usr/local/bin/cmux",
        "/opt/homebrew/bin/cmux",
        "/opt/local/bin/cmux",
        "/Applications/cmux.app/Contents/Resources/bin/cmux",
        "/Applications/cmux.app/Contents/MacOS/cmux",
    ] {
        commands.push(build_terminal_path_command(cli_path, target));
        if let Some(user_cli_path) = user_application_cli_path(cli_path) {
            commands.push(build_terminal_path_command(&user_cli_path, target));
        }
    }

    commands.push(build_terminal_app_command("cmux", target));
    commands
}

fn build_terminal_app_command(app_name: &str, target: &FileOpenTarget) -> OpenCommand {
    OpenCommand {
        program: "/usr/bin/open".to_string(),
        args: vec![
            "-a".to_string(),
            app_name.to_string(),
            path_arg(&terminal_cwd(target)),
        ],
    }
}

fn home_relative_cli_path(relative: &str) -> Option<String> {
    Some(
        PathBuf::from(std::env::var_os("HOME")?)
            .join(relative)
            .to_string_lossy()
            .into_owned(),
    )
}

fn user_application_cli_path(canonical_cli_path: &str) -> Option<String> {
    let home = std::env::var_os("HOME")?;
    let relative = Path::new(canonical_cli_path)
        .strip_prefix("/Applications")
        .ok()?;

    Some(
        PathBuf::from(home)
            .join("Applications")
            .join(relative)
            .to_string_lossy()
            .into_owned(),
    )
}

fn build_editor_env_command(raw: &str, target: &FileOpenTarget) -> anyhow::Result<OpenCommand> {
    let mut parts = shlex::split(raw.trim()).context("invalid shell quoting")?;
    anyhow::ensure!(!parts.is_empty(), "editor command is empty");
    let program = parts.remove(0);
    parts.push(path_arg(&target.path));

    Ok(OpenCommand {
        program,
        args: parts,
    })
}

fn command_targets_configured(
    command: &OpenCommand,
    configured: DefaultOpenTarget,
    target: &FileOpenTarget,
) -> bool {
    let program_name = command_program_name(command);

    if program_name != "open"
        && commands_for_target(configured, &skip_probe_target())
            .iter()
            .any(|target_command| {
                target_command.program == command.program
                    && command_program_name(target_command) != "open"
            })
    {
        return true;
    }

    match configured {
        DefaultOpenTarget::DefaultApp => return command_opens_default_app(command, target),
        DefaultOpenTarget::Finder => return command_reveals_in_finder(command, target),
        _ => {}
    }

    let Some(app_name) = target_app_name(configured) else {
        return false;
    };

    program_name == "open"
        && command.args.len() >= 2
        && command.args[0] == "-a"
        && command.args[1] == app_name
}

fn command_was_attempted(command: &OpenCommand, attempted: &[OpenCommand]) -> bool {
    attempted.iter().any(|attempted| attempted == command)
}

fn command_opens_default_app(command: &OpenCommand, target: &FileOpenTarget) -> bool {
    let path = path_arg(&target.path);

    command_program_name(command) == "open" && command.args.len() == 1 && command.args[0] == path
}

fn command_reveals_in_finder(command: &OpenCommand, target: &FileOpenTarget) -> bool {
    let path = path_arg(&target.path);

    command_program_name(command) == "open"
        && command.args.len() == 2
        && command.args[0] == "-R"
        && command.args[1] == path
}

fn command_program_name(command: &OpenCommand) -> &str {
    Path::new(&command.program)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(command.program.as_str())
}

fn skip_probe_target() -> FileOpenTarget {
    FileOpenTarget {
        path: PathBuf::from("/tmp/kaku-open-target-probe"),
        line: Some(1),
        col: Some(1),
        is_dir: false,
    }
}

fn target_app_name(configured: DefaultOpenTarget) -> Option<&'static str> {
    match configured {
        DefaultOpenTarget::VsCode => Some("Visual Studio Code"),
        DefaultOpenTarget::Cursor => Some("Cursor"),
        DefaultOpenTarget::Windsurf => Some("Windsurf"),
        DefaultOpenTarget::Kiro => Some("Kiro"),
        DefaultOpenTarget::Antigravity => Some("Antigravity"),
        DefaultOpenTarget::Zed => Some("Zed"),
        DefaultOpenTarget::IntelliJIdea => Some("IntelliJ IDEA"),
        DefaultOpenTarget::Terminal => Some("Terminal"),
        DefaultOpenTarget::ITerm2 => Some("iTerm"),
        DefaultOpenTarget::Ghostty => Some("Ghostty"),
        DefaultOpenTarget::WezTerm => Some("WezTerm"),
        DefaultOpenTarget::Cmux => Some("cmux"),
        _ => None,
    }
}

fn location_arg(target: &FileOpenTarget) -> Option<String> {
    let line = target.line?;
    let mut location = format!("{}:{}", path_arg(&target.path), line);

    if let Some(col) = target.col {
        location.push(':');
        location.push_str(&col.to_string());
    }

    Some(location)
}

fn path_arg(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use config::DefaultOpenTarget;
    use std::collections::VecDeque;

    #[derive(Default)]
    struct FakeRunner {
        results: VecDeque<anyhow::Result<bool>>,
        successes: Vec<OpenCommand>,
        attempts: Vec<OpenCommand>,
    }

    impl FakeRunner {
        fn new(results: impl IntoIterator<Item = anyhow::Result<bool>>) -> Self {
            Self {
                results: results.into_iter().collect(),
                successes: Vec::new(),
                attempts: Vec::new(),
            }
        }

        fn with_successes(successes: impl IntoIterator<Item = OpenCommand>) -> Self {
            Self {
                results: VecDeque::new(),
                successes: successes.into_iter().collect(),
                attempts: Vec::new(),
            }
        }
    }

    impl CommandRunner for FakeRunner {
        fn run(&mut self, command: &OpenCommand) -> anyhow::Result<bool> {
            self.attempts.push(command.clone());
            if let Some(result) = self.results.pop_front() {
                return result;
            }

            Ok(self.successes.iter().any(|success| success == command))
        }
    }

    fn file_target() -> FileOpenTarget {
        FileOpenTarget {
            path: PathBuf::from("/tmp/project/src/main.rs"),
            line: Some(12),
            col: Some(4),
            is_dir: false,
        }
    }

    fn dir_target() -> FileOpenTarget {
        FileOpenTarget {
            path: PathBuf::from("/tmp/project"),
            line: None,
            col: None,
            is_dir: true,
        }
    }

    #[test]
    fn terminal_targets_open_file_parent_directory() {
        assert_eq!(
            terminal_cwd(&file_target()),
            PathBuf::from("/tmp/project/src")
        );
    }

    #[test]
    fn terminal_targets_open_directory_itself() {
        assert_eq!(terminal_cwd(&dir_target()), PathBuf::from("/tmp/project"));
    }

    #[test]
    fn terminal_targets_fall_back_to_path_when_file_has_no_parent() {
        let target = FileOpenTarget {
            path: PathBuf::from("main.rs"),
            line: None,
            col: None,
            is_dir: false,
        };

        assert_eq!(terminal_cwd(&target), PathBuf::from("main.rs"));
    }

    #[test]
    fn vscode_style_goto_command_includes_g_flag_and_location() {
        let command = build_cli_location_command("code", LocationStyle::GotoFlag, &file_target());

        assert_eq!(command.program, "code");
        assert_eq!(
            command.args,
            vec![
                "-g".to_string(),
                "/tmp/project/src/main.rs:12:4".to_string()
            ]
        );
    }

    #[test]
    fn zed_style_suffix_command_includes_location() {
        let command = build_cli_location_command("zed", LocationStyle::PathSuffix, &file_target());

        assert_eq!(command.program, "zed");
        assert_eq!(
            command.args,
            vec!["/tmp/project/src/main.rs:12:4".to_string()]
        );
    }

    #[test]
    fn intellij_style_command_includes_line_and_column_flags() {
        let command =
            build_cli_location_command("idea", LocationStyle::LineColumnFlags, &file_target());

        assert_eq!(command.program, "idea");
        assert_eq!(
            command.args,
            vec![
                "--line".to_string(),
                "12".to_string(),
                "--column".to_string(),
                "4".to_string(),
                "/tmp/project/src/main.rs".to_string(),
            ]
        );
    }

    #[test]
    fn goto_command_without_line_uses_bare_path() {
        let target = FileOpenTarget {
            path: PathBuf::from("/tmp/project/src/main.rs"),
            line: None,
            col: Some(4),
            is_dir: false,
        };

        let command = build_cli_location_command("code", LocationStyle::GotoFlag, &target);

        assert_eq!(command.args, vec!["/tmp/project/src/main.rs".to_string()]);
    }

    #[test]
    fn suffix_command_without_line_uses_bare_path() {
        let target = FileOpenTarget {
            path: PathBuf::from("/tmp/project/src/main.rs"),
            line: None,
            col: Some(4),
            is_dir: false,
        };

        let command = build_cli_location_command("zed", LocationStyle::PathSuffix, &target);

        assert_eq!(command.args, vec!["/tmp/project/src/main.rs".to_string()]);
    }

    #[test]
    fn line_column_command_without_line_uses_bare_path() {
        let target = FileOpenTarget {
            path: PathBuf::from("/tmp/project/src/main.rs"),
            line: None,
            col: Some(4),
            is_dir: false,
        };

        let command = build_cli_location_command("idea", LocationStyle::LineColumnFlags, &target);

        assert_eq!(command.args, vec!["/tmp/project/src/main.rs".to_string()]);
    }

    #[test]
    fn path_only_command_uses_bare_path_even_with_line() {
        let command = build_cli_location_command("editor", LocationStyle::PathOnly, &file_target());

        assert_eq!(command.program, "editor");
        assert_eq!(command.args, vec!["/tmp/project/src/main.rs".to_string()]);
    }

    #[test]
    fn finder_reveals_files() {
        let command = build_finder_command(&file_target());

        assert_eq!(command.program, "/usr/bin/open");
        assert_eq!(
            command.args,
            vec!["-R".to_string(), "/tmp/project/src/main.rs".to_string()]
        );
    }

    #[test]
    fn finder_opens_directories() {
        let command = build_finder_command(&dir_target());

        assert_eq!(command.program, "/usr/bin/open");
        assert_eq!(command.args, vec!["/tmp/project".to_string()]);
    }

    #[test]
    fn default_app_opens_target_path() {
        let command = build_default_app_command(&file_target());

        assert_eq!(command.program, "/usr/bin/open");
        assert_eq!(command.args, vec!["/tmp/project/src/main.rs".to_string()]);
    }

    #[test]
    fn editor_target_commands_include_app_bundle_cli_and_open_app_fallback() {
        let commands = commands_for_target(DefaultOpenTarget::Cursor, &file_target());

        assert!(commands.iter().any(|command| command.program
            == "/Applications/Cursor.app/Contents/Resources/app/bin/cursor"));
        assert!(commands
            .iter()
            .any(|command| command.program == "/usr/bin/open"
                && command.args
                    == vec![
                        "-a".to_string(),
                        "Cursor".to_string(),
                        "/tmp/project/src/main.rs".to_string(),
                    ]));
    }

    #[test]
    fn vscode_commands_include_legacy_app_bundle_cli_path() {
        let commands = commands_for_target(DefaultOpenTarget::VsCode, &file_target());

        assert!(commands.iter().any(|command| command.program
            == "/Applications/Visual Studio Code.app/Contents/Resources/app/bin/code"));
    }

    #[test]
    fn antigravity_commands_use_goto_location_and_home_cli_path() {
        let commands = commands_for_target(DefaultOpenTarget::Antigravity, &file_target());

        assert_eq!(commands[0].program, "antigravity");
        assert_eq!(
            commands[0].args,
            vec![
                "-g".to_string(),
                "/tmp/project/src/main.rs:12:4".to_string()
            ]
        );
        assert!(commands.iter().any(|command| command.program
            == format!(
                "{}/.antigravity/antigravity/bin/antigravity",
                std::env::var("HOME").expect("HOME")
            )
            && command.args
                == vec![
                    "-g".to_string(),
                    "/tmp/project/src/main.rs:12:4".to_string(),
                ]));
    }

    #[test]
    fn intellij_commands_use_line_column_location() {
        let command = commands_for_target(DefaultOpenTarget::IntelliJIdea, &file_target())
            .into_iter()
            .next()
            .unwrap();

        assert_eq!(command.program, "idea");
        assert_eq!(
            command.args,
            vec![
                "--line".to_string(),
                "12".to_string(),
                "--column".to_string(),
                "4".to_string(),
                "/tmp/project/src/main.rs".to_string(),
            ]
        );
    }

    #[test]
    fn wezterm_uses_start_with_cwd() {
        let command = build_wezterm_command("wezterm", &file_target());

        assert_eq!(command.program, "wezterm");
        assert_eq!(
            command.args,
            vec![
                "start".to_string(),
                "--cwd".to_string(),
                "/tmp/project/src".to_string()
            ]
        );
    }

    #[test]
    fn wezterm_target_commands_include_app_bundle_cli_and_open_app_fallback() {
        let commands = commands_for_target(DefaultOpenTarget::WezTerm, &file_target());

        assert!(commands
            .iter()
            .any(|command| command.program == "/Applications/WezTerm.app/Contents/MacOS/wezterm"));
        assert!(commands
            .iter()
            .any(|command| command.program == "/usr/bin/open"
                && command.args
                    == vec![
                        "-a".to_string(),
                        "WezTerm".to_string(),
                        "/tmp/project/src".to_string(),
                    ]));
    }

    #[test]
    fn cmux_target_commands_include_absolute_cli_and_open_app_fallback() {
        let commands = commands_for_target(DefaultOpenTarget::Cmux, &file_target());

        assert!(commands
            .iter()
            .any(|command| command.program == "/usr/local/bin/cmux"
                && command.args == vec!["/tmp/project/src".to_string()]));
        assert!(commands.iter().any(|command| command.program
            == "/Applications/cmux.app/Contents/Resources/bin/cmux"
            && command.args == vec!["/tmp/project/src".to_string()]));
        assert!(commands
            .iter()
            .any(|command| command.program == "/usr/bin/open"
                && command.args
                    == vec![
                        "-a".to_string(),
                        "cmux".to_string(),
                        "/tmp/project/src".to_string(),
                    ]));
    }

    #[test]
    fn selected_missing_target_falls_back_to_default_app_without_retrying_selected_target() {
        let target = file_target();
        let mut runner = FakeRunner::with_successes([build_default_app_command(&target)]);

        open_with_runner(DefaultOpenTarget::Cursor, &target, &mut runner).unwrap();

        assert_eq!(runner.attempts[0].program, "cursor");
        assert_eq!(
            runner
                .attempts
                .iter()
                .filter(|command| command.program == "cursor")
                .count(),
            1
        );
        assert_eq!(
            runner
                .attempts
                .iter()
                .filter(|command| command.program == "/usr/bin/open"
                    && command
                        .args
                        .starts_with(&["-a".to_string(), "Cursor".to_string()]))
                .count(),
            1
        );
        assert!(runner
            .attempts
            .iter()
            .any(|command| command == &build_default_app_command(&target)));
    }

    #[test]
    fn auto_tries_editor_env_before_default_app() {
        let mut runner = FakeRunner::new([Ok(true)]);

        open_auto_with_runner(
            &file_target(),
            EditorEnv {
                visual: Some("vim".to_string()),
                editor: None,
            },
            &mut runner,
        )
        .unwrap();

        assert_eq!(
            runner.attempts[0],
            OpenCommand {
                program: "vim".to_string(),
                args: vec!["/tmp/project/src/main.rs".to_string()],
            }
        );
    }

    #[test]
    fn selected_target_fallback_skips_matching_env_editor_command() {
        let target = file_target();
        let mut runner = FakeRunner::with_successes([build_default_app_command(&target)]);

        open_auto_with_runner_skipping(
            &target,
            EditorEnv {
                visual: Some("cursor".to_string()),
                editor: None,
            },
            &mut runner,
            Some(DefaultOpenTarget::Cursor),
        )
        .unwrap();

        assert!(!runner
            .attempts
            .iter()
            .any(|command| command.program == "cursor"));
        assert!(runner
            .attempts
            .iter()
            .any(|command| command == &build_default_app_command(&target)));
    }

    #[test]
    fn selected_target_fallback_skips_matching_env_open_app_command() {
        let target = file_target();
        let mut runner = FakeRunner::with_successes([build_default_app_command(&target)]);

        open_auto_with_runner_skipping(
            &target,
            EditorEnv {
                visual: Some("/usr/bin/open -a Cursor".to_string()),
                editor: None,
            },
            &mut runner,
            Some(DefaultOpenTarget::Cursor),
        )
        .unwrap();

        assert!(!runner
            .attempts
            .iter()
            .any(|command| command.program == "/usr/bin/open"
                && command
                    .args
                    .starts_with(&["-a".to_string(), "Cursor".to_string()])));
        assert!(runner
            .attempts
            .iter()
            .any(|command| command == &build_default_app_command(&target)));
    }

    #[test]
    fn selected_target_fallback_skips_matching_env_app_cli_command() {
        let target = file_target();
        let mut runner = FakeRunner::with_successes([build_default_app_command(&target)]);

        open_auto_with_runner_skipping(
            &target,
            EditorEnv {
                visual: Some("/Applications/Zed.app/Contents/MacOS/cli".to_string()),
                editor: None,
            },
            &mut runner,
            Some(DefaultOpenTarget::Zed),
        )
        .unwrap();

        assert!(!runner
            .attempts
            .iter()
            .any(|command| command.program == "/Applications/Zed.app/Contents/MacOS/cli"));
        assert!(runner
            .attempts
            .iter()
            .any(|command| command == &build_default_app_command(&target)));
    }

    #[test]
    fn selected_default_app_fallback_skips_matching_env_open_command() {
        let target = file_target();
        let env_open = OpenCommand {
            program: "open".to_string(),
            args: vec!["/tmp/project/src/main.rs".to_string()],
        };
        let mut runner = FakeRunner::with_successes([build_finder_command(&target)]);

        open_auto_with_runner_skipping(
            &target,
            EditorEnv {
                visual: Some("open".to_string()),
                editor: None,
            },
            &mut runner,
            Some(DefaultOpenTarget::DefaultApp),
        )
        .unwrap();

        assert!(!runner.attempts.iter().any(|command| command == &env_open));
        assert!(runner
            .attempts
            .iter()
            .any(|command| command == &build_finder_command(&target)));
    }

    #[test]
    fn selected_finder_fallback_skips_matching_env_open_reveal_command() {
        let target = file_target();
        let env_finder = OpenCommand {
            program: "open".to_string(),
            args: vec!["-R".to_string(), "/tmp/project/src/main.rs".to_string()],
        };
        let mut runner = FakeRunner::with_successes([build_default_app_command(&target)]);

        open_auto_with_runner_skipping(
            &target,
            EditorEnv {
                visual: Some("open -R".to_string()),
                editor: None,
            },
            &mut runner,
            Some(DefaultOpenTarget::Finder),
        )
        .unwrap();

        assert!(!runner.attempts.iter().any(|command| command == &env_finder));
        assert!(runner
            .attempts
            .iter()
            .any(|command| command == &build_default_app_command(&target)));
    }

    #[test]
    fn selected_finder_directory_fallback_skips_duplicate_default_app_command() {
        let target = dir_target();
        let duplicate = build_default_app_command(&target);
        let mut runner = FakeRunner::with_successes([]);

        open_with_runner_and_env(
            DefaultOpenTarget::Finder,
            &target,
            EditorEnv::default(),
            &mut runner,
        )
        .expect_err("all fallback commands should fail");

        assert_eq!(
            runner
                .attempts
                .iter()
                .filter(|command| *command == &duplicate)
                .count(),
            1
        );
    }

    #[test]
    fn selected_default_app_directory_fallback_skips_duplicate_finder_command() {
        let target = dir_target();
        let duplicate = build_finder_command(&target);
        let mut runner = FakeRunner::with_successes([]);

        open_with_runner_and_env(
            DefaultOpenTarget::DefaultApp,
            &target,
            EditorEnv::default(),
            &mut runner,
        )
        .expect_err("all fallback commands should fail");

        assert_eq!(
            runner
                .attempts
                .iter()
                .filter(|command| *command == &duplicate)
                .count(),
            1
        );
    }

    #[test]
    fn auto_fallback_skips_env_command_after_it_fails() {
        let target = dir_target();
        let duplicate = build_default_app_command(&target);
        let mut runner = FakeRunner::with_successes([]);

        open_auto_with_runner(
            &target,
            EditorEnv {
                visual: Some("/usr/bin/open".to_string()),
                editor: None,
            },
            &mut runner,
        )
        .expect_err("all fallback commands should fail");

        assert_eq!(
            runner
                .attempts
                .iter()
                .filter(|command| *command == &duplicate)
                .count(),
            1
        );
    }

    #[cfg(unix)]
    #[test]
    fn process_runner_treats_launch_errors_as_command_failures() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().expect("tempdir");
        let launcher = dir.path().join("not-executable");
        std::fs::write(&launcher, "#!/bin/sh\nexit 0\n").expect("write launcher");
        std::fs::set_permissions(&launcher, std::fs::Permissions::from_mode(0o644))
            .expect("set permissions");
        let command = OpenCommand {
            program: launcher.to_string_lossy().into_owned(),
            args: Vec::new(),
        };
        let mut runner = ProcessCommandRunner;

        assert!(!runner.run(&command).expect("launch failure is non-fatal"));
    }

    #[test]
    fn selected_target_fallback_keeps_unrelated_env_open_app_command() {
        let target = file_target();
        let textedit = OpenCommand {
            program: "/usr/bin/open".to_string(),
            args: vec![
                "-a".to_string(),
                "TextEdit".to_string(),
                "/tmp/project/src/main.rs".to_string(),
            ],
        };
        let mut runner = FakeRunner::with_successes([textedit.clone()]);

        open_auto_with_runner_skipping(
            &target,
            EditorEnv {
                visual: Some("/usr/bin/open -a TextEdit".to_string()),
                editor: None,
            },
            &mut runner,
            Some(DefaultOpenTarget::Zed),
        )
        .unwrap();

        assert_eq!(runner.attempts, vec![textedit]);
    }

    #[test]
    fn selected_non_auto_success_does_not_fall_through_to_default_app() {
        let mut runner = FakeRunner::new([Ok(true)]);

        open_with_runner(DefaultOpenTarget::Zed, &file_target(), &mut runner).unwrap();

        assert_eq!(runner.attempts.len(), 1);
        assert_eq!(runner.attempts[0].program, "zed");
    }

    #[test]
    fn auto_returns_error_when_no_command_succeeds() {
        let mut runner = FakeRunner::new(std::iter::repeat_with(|| Ok(false)).take(16));

        let err = open_auto_with_runner(&file_target(), EditorEnv::default(), &mut runner)
            .expect_err("auto should fail when every command fails");

        assert!(err
            .to_string()
            .contains("failed to open /tmp/project/src/main.rs"));
    }

    #[test]
    fn terminal_target_uses_open_app_with_terminal_cwd() {
        let command = commands_for_target(DefaultOpenTarget::Terminal, &file_target())
            .into_iter()
            .next()
            .unwrap();

        assert_eq!(command.program, "/usr/bin/open");
        assert_eq!(
            command.args,
            vec![
                "-a".to_string(),
                "Terminal".to_string(),
                "/tmp/project/src".to_string()
            ]
        );
    }
}
