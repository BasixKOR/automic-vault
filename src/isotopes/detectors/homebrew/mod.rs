use std::path::{Path, PathBuf};

#[cfg(unix)]
use std::{ffi::CString, os::unix::ffi::OsStrExt};

use crate::{AffectedFile, Finding};

const NAME: &str = "homebrew";
const DOCS_URL: &str = "https://github.com/automic-vault/automic-vault/blob/main/src/isotopes/detectors/homebrew/detector.md";

pub(crate) fn findings(_home: &Path) -> Vec<Finding> {
    let affected = writable_homebrew_paths()
        .unwrap_or_default()
        .into_iter()
        .map(|path| AffectedFile {
            path: path.display().to_string(),
            line: None,
        })
        .collect::<Vec<_>>();
    if affected.is_empty() {
        Vec::new()
    } else {
        vec![Finding {
            source: NAME,
            homepage: DOCS_URL,
            severity: "medium",
            explanation: "Homebrew installation contains paths writable by the current user"
                .to_string(),
            solution: "Run `sudo av harden brew`.".to_string(),
            affected,
            docs_url: DOCS_URL,
        }]
    }
}

fn writable_homebrew_paths() -> Result<Vec<PathBuf>, String> {
    let target = brew_target_path();
    if !target.exists() {
        return Ok(Vec::new());
    }

    let prefix = brew_prefix();
    let mut paths = vec![prefix.clone()];
    for entry in std::fs::read_dir(&prefix)
        .map_err(|err| format!("failed to read {}: {err}", prefix.display()))?
    {
        let path = entry
            .map_err(|err| format!("failed to read entry in {}: {err}", prefix.display()))?
            .path();
        if path.is_dir() {
            paths.push(path);
        }
    }
    paths.sort();
    let checked_count = paths.len();
    let mut writable = paths
        .into_iter()
        .filter_map(|path| match current_user_can_modify_directory(&path) {
            Ok(true) => Some(Ok(path)),
            Ok(false) => None,
            Err(err) => Some(Err(err)),
        })
        .collect::<Result<Vec<_>, _>>()?;
    if writable.len() == checked_count {
        writable.truncate(1);
    }
    Ok(writable)
}

#[cfg(unix)]
fn current_user_can_modify_directory(path: &Path) -> Result<bool, String> {
    let path_bytes = path.as_os_str().as_bytes();
    let path_c = CString::new(path_bytes)
        .map_err(|_| format!("path contains a NUL byte: {}", path.display()))?;
    Ok(unsafe { libc::access(path_c.as_ptr(), libc::W_OK | libc::X_OK) } == 0)
}

#[cfg(not(unix))]
fn current_user_can_modify_directory(path: &Path) -> Result<bool, String> {
    let metadata = std::fs::metadata(path)
        .map_err(|err| format!("failed to stat {}: {err}", path.display()))?;
    Ok(!metadata.permissions().readonly())
}

fn brew_prefix() -> PathBuf {
    crate::test_env_var("AUTOMIC_VAULT_TEST_BREW_PREFIX")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/opt/homebrew"))
}

fn brew_target_path() -> PathBuf {
    crate::test_env_var("AUTOMIC_VAULT_TEST_BREW_TARGET")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/opt/homebrew/bin/brew"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn ignores_missing_homebrew() {
        let _guard = crate::global_test_env_lock().lock().unwrap();
        let dir = temp_path("homebrew-missing");
        let target = dir.join("bin/brew");
        set_env([
            ("AUTOMIC_VAULT_TEST_BREW_PREFIX", dir.as_path()),
            ("AUTOMIC_VAULT_TEST_BREW_TARGET", target.as_path()),
        ]);

        assert_eq!(writable_homebrew_paths().unwrap(), Vec::<PathBuf>::new());

        clear_env([
            "AUTOMIC_VAULT_TEST_BREW_PREFIX",
            "AUTOMIC_VAULT_TEST_BREW_TARGET",
        ]);
    }

    #[test]
    fn collapses_fully_writable_installation_to_prefix() {
        let _guard = crate::global_test_env_lock().lock().unwrap();
        let dir = temp_path("homebrew-mutable");
        let target = dir.join("bin/brew");
        std::fs::create_dir_all(target.parent().unwrap()).unwrap();
        std::fs::write(&target, "").unwrap();
        set_env([
            ("AUTOMIC_VAULT_TEST_BREW_PREFIX", dir.as_path()),
            ("AUTOMIC_VAULT_TEST_BREW_TARGET", target.as_path()),
        ]);

        let paths = writable_homebrew_paths().unwrap();
        let findings = findings(&dir);

        clear_env([
            "AUTOMIC_VAULT_TEST_BREW_PREFIX",
            "AUTOMIC_VAULT_TEST_BREW_TARGET",
        ]);
        assert_eq!(paths, vec![dir.clone()]);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].severity, "medium");
        assert_eq!(findings[0].affected.len(), 1);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn ignores_stub_state_when_installation_is_not_writable() {
        let _guard = crate::global_test_env_lock().lock().unwrap();
        let dir = temp_path("homebrew-protected");
        let target = dir.join("bin/brew");
        let invalid_stub = dir.join("ordinary-brew");
        std::fs::create_dir_all(target.parent().unwrap()).unwrap();
        std::fs::write(&target, "").unwrap();
        std::fs::write(&invalid_stub, "").unwrap();
        set_mode(target.parent().unwrap(), 0o555);
        set_mode(&dir, 0o555);
        set_env([
            ("AUTOMIC_VAULT_TEST_BREW_PREFIX", dir.as_path()),
            ("AUTOMIC_VAULT_TEST_BREW_TARGET", target.as_path()),
            ("AUTOMIC_VAULT_TEST_BREW_STUB", invalid_stub.as_path()),
        ]);

        let paths = writable_homebrew_paths().unwrap();

        clear_env([
            "AUTOMIC_VAULT_TEST_BREW_PREFIX",
            "AUTOMIC_VAULT_TEST_BREW_TARGET",
            "AUTOMIC_VAULT_TEST_BREW_STUB",
        ]);
        assert_eq!(paths, Vec::<PathBuf>::new());
        set_mode(&dir, 0o755);
        set_mode(target.parent().unwrap(), 0o755);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn reports_writable_first_level_directory_but_not_deeper_directories() {
        let _guard = crate::global_test_env_lock().lock().unwrap();
        let dir = temp_path("homebrew-first-level");
        let target = dir.join("bin/brew");
        let cellar = dir.join("Cellar");
        let nested = dir.join("var/cache");
        std::fs::create_dir_all(target.parent().unwrap()).unwrap();
        std::fs::create_dir_all(&cellar).unwrap();
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::write(&target, "").unwrap();
        set_mode(target.parent().unwrap(), 0o555);
        set_mode(nested.parent().unwrap(), 0o555);
        set_mode(&dir, 0o555);
        set_env([
            ("AUTOMIC_VAULT_TEST_BREW_PREFIX", dir.as_path()),
            ("AUTOMIC_VAULT_TEST_BREW_TARGET", target.as_path()),
        ]);

        let paths = writable_homebrew_paths().unwrap();
        let findings = findings(&dir);

        clear_env([
            "AUTOMIC_VAULT_TEST_BREW_PREFIX",
            "AUTOMIC_VAULT_TEST_BREW_TARGET",
        ]);
        assert_eq!(paths, vec![cellar.clone()]);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].affected[0].path, cellar.display().to_string());
        set_mode(&dir, 0o755);
        set_mode(target.parent().unwrap(), 0o755);
        set_mode(nested.parent().unwrap(), 0o755);
        let _ = std::fs::remove_dir_all(dir);
    }

    fn set_env<const N: usize>(pairs: [(&str, &Path); N]) {
        for (key, value) in pairs {
            unsafe {
                std::env::set_var(key, value);
            }
        }
    }

    fn clear_env<const N: usize>(keys: [&str; N]) {
        for key in keys {
            unsafe {
                std::env::remove_var(key);
            }
        }
    }

    fn set_mode(path: &Path, mode: u32) {
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode)).unwrap();
    }

    fn temp_path(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("av-{label}-{}-{nanos}", std::process::id()))
    }
}
