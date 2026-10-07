//! Which files a diagram may read (`!include`, `<img>`), as PlantUML's security profiles decide
//! (`SFile.isFileOk`). A refused file is treated as missing.

use std::path::Path;

use crate::host::Host;
use crate::security_profile::SecurityProfile;

/// The allowlist profiles admit only the folders `plantuml.include.path` and `plantuml.allowlist.path`
/// name, which rockuml does not read, so they refuse every file.
pub(crate) fn is_forbidden(path: &Path, host: &dyn Host) -> bool {
    match SecurityProfile::init(host) {
        SecurityProfile::Insecure => false,
        SecurityProfile::Legacy => {
            is_system_path(&clean_path_secure(&host.current_directory().join(path)))
        }
        SecurityProfile::Sandbox
        | SecurityProfile::Allowlist
        | SecurityProfile::Internet
        | SecurityProfile::InternetWithDotSvg => true,
    }
}

/// The absolute path in which only doubled backslashes become slashes.
fn clean_path_secure(absolute: &Path) -> String {
    absolute
        .to_string_lossy()
        .replace('\0', "")
        .replace(r"\\", "/")
}

/// Unix system folders and network paths.
fn is_system_path(path: &str) -> bool {
    const SYSTEM_FOLDERS: [&str; 5] = ["/etc/", "/dev/", "/boot/", "/proc/", "/sys/"];
    SYSTEM_FOLDERS.iter().any(|folder| path.starts_with(folder)) || path.starts_with("//")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::host::FakeHost;

    fn forbidden(path: &str, profile: Option<&str>) -> bool {
        let mut host = FakeHost::default();
        if let Some(profile) = profile {
            host.environment
                .insert("PLANTUML_SECURITY_PROFILE".to_owned(), profile.to_owned());
        }
        is_forbidden(Path::new(path), &host)
    }

    #[test]
    fn system_folders_and_network_paths_are_refused() {
        assert!(is_system_path("/etc/passwd"));
        assert!(is_system_path("/proc/self/environ"));
        assert!(is_system_path("//server/share/x.png"));
        assert!(!is_system_path("/etc"), "only what is inside the folders");
        assert!(!is_system_path("/usr/share/logo.png"));
    }

    #[test]
    fn doubled_backslashes_count_as_slashes() {
        assert_eq!(clean_path_secure(Path::new(r"a\\b\c")), r"a/b\c");
    }

    #[test]
    fn the_profile_comes_from_the_environment() {
        assert!(!forbidden("logo.png", None));
        assert!(forbidden("/etc/passwd", None));
        assert!(forbidden("logo.png", Some("sandbox")));
        assert!(forbidden("logo.png", Some("INTERNET")));
        assert!(!forbidden("/etc/passwd", Some("unsecure")));
    }
}
