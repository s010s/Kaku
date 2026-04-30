use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OpenTargetOption {
    pub label: &'static str,
    pub enabled: bool,
}

#[derive(Clone, Debug)]
pub struct OpenTargetDefinition {
    pub label: &'static str,
    pub cli_names: &'static [&'static str],
    pub app_bundles: &'static [&'static str],
    pub base: bool,
}

#[derive(Clone, Debug)]
pub struct DetectedOpenTargets {
    options: Vec<OpenTargetOption>,
}

const OPEN_TARGETS: &[OpenTargetDefinition] = &[
    OpenTargetDefinition {
        label: "Auto",
        cli_names: &[],
        app_bundles: &[],
        base: true,
    },
    OpenTargetDefinition {
        label: "Default app",
        cli_names: &[],
        app_bundles: &[],
        base: true,
    },
    OpenTargetDefinition {
        label: "Finder",
        cli_names: &[],
        app_bundles: &[],
        base: true,
    },
    OpenTargetDefinition {
        label: "Terminal",
        cli_names: &[],
        app_bundles: &[],
        base: true,
    },
    OpenTargetDefinition {
        label: "VS Code",
        cli_names: &["code"],
        app_bundles: &["/Applications/Visual Studio Code.app"],
        base: false,
    },
    OpenTargetDefinition {
        label: "Cursor",
        cli_names: &["cursor"],
        app_bundles: &["/Applications/Cursor.app"],
        base: false,
    },
    OpenTargetDefinition {
        label: "Windsurf",
        cli_names: &["windsurf"],
        app_bundles: &["/Applications/Windsurf.app"],
        base: false,
    },
    OpenTargetDefinition {
        label: "Kiro",
        cli_names: &["kiro"],
        app_bundles: &["/Applications/Kiro.app"],
        base: false,
    },
    OpenTargetDefinition {
        label: "Antigravity",
        cli_names: &["antigravity"],
        app_bundles: &["/Applications/Antigravity.app"],
        base: false,
    },
    OpenTargetDefinition {
        label: "Zed",
        cli_names: &["zed"],
        app_bundles: &["/Applications/Zed.app"],
        base: false,
    },
    OpenTargetDefinition {
        label: "IntelliJ IDEA",
        cli_names: &["idea"],
        app_bundles: &["/Applications/IntelliJ IDEA.app"],
        base: false,
    },
    OpenTargetDefinition {
        label: "iTerm2",
        cli_names: &[],
        app_bundles: &["/Applications/iTerm.app"],
        base: false,
    },
    OpenTargetDefinition {
        label: "Ghostty",
        cli_names: &["ghostty"],
        app_bundles: &["/Applications/Ghostty.app"],
        base: false,
    },
    OpenTargetDefinition {
        label: "WezTerm",
        cli_names: &["wezterm"],
        app_bundles: &["/Applications/WezTerm.app"],
        base: false,
    },
    OpenTargetDefinition {
        label: "cmux",
        cli_names: &["cmux"],
        app_bundles: &["/Applications/cmux.app"],
        base: false,
    },
];

impl DetectedOpenTargets {
    pub fn detect(current: Option<&str>) -> Self {
        Self::from_probe_results(probe_cli_names(), probe_app_bundles(), current)
    }

    pub fn from_probe_results(
        cli_names: BTreeSet<String>,
        app_bundles: BTreeSet<String>,
        current: Option<&str>,
    ) -> Self {
        let mut options = Vec::new();

        for definition in OPEN_TARGETS {
            let detected = definition
                .cli_names
                .iter()
                .any(|name| cli_names.contains(*name))
                || definition
                    .app_bundles
                    .iter()
                    .any(|bundle| app_bundles.contains(*bundle));
            let enabled = definition.base || detected;
            let is_current = current == Some(definition.label);

            if enabled || is_current {
                options.push(OpenTargetOption {
                    label: definition.label,
                    enabled,
                });
            }
        }

        Self { options }
    }

    pub fn all_options(&self) -> &[OpenTargetOption] {
        &self.options
    }

    #[cfg(test)]
    pub fn selectable_labels(&self) -> Vec<&'static str> {
        self.options
            .iter()
            .filter(|option| option.enabled)
            .map(|option| option.label)
            .collect()
    }

    #[cfg(test)]
    pub fn option(&self, label: &str) -> Option<&OpenTargetOption> {
        self.options.iter().find(|option| option.label == label)
    }
}

fn probe_cli_names() -> BTreeSet<String> {
    let path = match std::env::var_os("PATH") {
        Some(path) => path,
        None => return BTreeSet::new(),
    };
    let mut cli_names = BTreeSet::new();

    for dir in std::env::split_paths(&path) {
        for definition in OPEN_TARGETS {
            for name in definition.cli_names {
                if is_available_cli(&dir.join(name)) {
                    cli_names.insert((*name).to_string());
                }
            }
        }
    }

    cli_names
}

fn probe_app_bundles() -> BTreeSet<String> {
    let home = std::env::var_os("HOME").map(PathBuf::from);
    let mut app_bundles = BTreeSet::new();

    for definition in OPEN_TARGETS {
        for bundle in definition.app_bundles {
            if is_app_bundle(Path::new(bundle))
                || is_app_bundle(&user_application_bundle(&home, bundle))
            {
                app_bundles.insert((*bundle).to_string());
            }
        }
    }

    app_bundles
}

fn user_application_bundle(home: &Option<PathBuf>, canonical_bundle: &str) -> PathBuf {
    let bundle_name = Path::new(canonical_bundle)
        .file_name()
        .expect("catalog app bundle has a file name");

    home.as_ref()
        .map(|home| home.join("Applications").join(bundle_name))
        .unwrap_or_default()
}

#[cfg(unix)]
fn is_available_cli(path: &Path) -> bool {
    path.is_file()
        && fs::metadata(path)
            .map(|metadata| metadata.permissions().mode() & 0o111 != 0)
            .unwrap_or(false)
}

#[cfg(not(unix))]
fn is_available_cli(path: &Path) -> bool {
    path.is_file()
}

fn is_app_bundle(path: &Path) -> bool {
    path.is_dir()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;
    use std::fs;

    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;

    fn set(items: &[&str]) -> BTreeSet<String> {
        items.iter().map(|item| item.to_string()).collect()
    }

    #[test]
    fn base_targets_are_always_available() {
        let detected = DetectedOpenTargets::from_probe_results(set(&[]), set(&[]), None);
        assert!(detected.selectable_labels().contains(&"Auto"));
        assert!(detected.selectable_labels().contains(&"Default app"));
        assert!(detected.selectable_labels().contains(&"Finder"));
        assert!(detected.selectable_labels().contains(&"Terminal"));
    }

    #[test]
    fn unavailable_targets_are_not_displayed_without_current_value() {
        let detected = DetectedOpenTargets::from_probe_results(set(&[]), set(&[]), None);
        assert!(!detected
            .all_options()
            .iter()
            .any(|option| option.label == "Cursor"));
        assert!(!detected
            .all_options()
            .iter()
            .any(|option| option.label == "Ghostty"));
    }

    #[test]
    fn detected_cli_targets_are_selectable() {
        let detected =
            DetectedOpenTargets::from_probe_results(set(&["cursor", "ghostty"]), set(&[]), None);
        assert!(detected.option("Cursor").expect("Cursor option").enabled);
        assert!(detected.option("Ghostty").expect("Ghostty option").enabled);
    }

    #[test]
    fn missing_current_value_is_visible_but_disabled() {
        let detected = DetectedOpenTargets::from_probe_results(set(&[]), set(&[]), Some("Cursor"));
        let option = detected.option("Cursor").expect("Cursor current option");
        assert!(!option.enabled);
        assert_eq!(option.label, "Cursor");
    }

    #[test]
    fn app_bundle_detection_enables_targets() {
        let detected = DetectedOpenTargets::from_probe_results(
            set(&[]),
            set(&["/Applications/Antigravity.app"]),
            None,
        );
        assert!(
            detected
                .option("Antigravity")
                .expect("Antigravity option")
                .enabled
        );
    }

    #[test]
    fn detect_includes_base_targets() {
        let detected = DetectedOpenTargets::detect(None);
        assert!(detected.option("Auto").expect("Auto option").enabled);
        assert!(
            detected
                .option("Default app")
                .expect("Default app option")
                .enabled
        );
    }

    #[cfg(unix)]
    #[test]
    fn non_executable_path_file_is_not_available_cli() {
        let temp_dir = tempfile::tempdir().expect("temp dir");
        let path = temp_dir.path().join("cursor");
        fs::write(&path, "").expect("write cli file");
        fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).expect("set cli permissions");

        assert!(!is_available_cli(&path));
    }

    #[cfg(unix)]
    #[test]
    fn executable_path_file_is_available_cli() {
        let temp_dir = tempfile::tempdir().expect("temp dir");
        let path = temp_dir.path().join("cursor");
        fs::write(&path, "").expect("write cli file");
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).expect("set cli permissions");

        assert!(is_available_cli(&path));
    }

    #[test]
    fn regular_file_named_app_bundle_is_not_detected() {
        let temp_dir = tempfile::tempdir().expect("temp dir");
        let path = temp_dir.path().join("Cursor.app");
        fs::write(&path, "").expect("write app file");

        assert!(!is_app_bundle(&path));
    }

    #[test]
    fn directory_named_app_bundle_is_detected() {
        let temp_dir = tempfile::tempdir().expect("temp dir");
        let path = temp_dir.path().join("Cursor.app");
        fs::create_dir(&path).expect("create app bundle directory");

        assert!(is_app_bundle(&path));
    }
}
