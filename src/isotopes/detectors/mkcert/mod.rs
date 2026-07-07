#![allow(dead_code)]

use std::path::{Path, PathBuf};

pub fn install_is_insecure() -> Result<bool, String> {
    install_insecurity_reasons().map(|reasons| !reasons.is_empty())
}

pub fn install_insecurity_reasons() -> Result<Vec<String>, String> {
    let mut reasons = Vec::new();
    let key_path = caroot_path()?.join("rootCA-key.pem");
    if key_path.exists() && file_is_non_empty(&key_path)? {
        reasons.push(format!(
            "mkcert CAROOT contains a plaintext root CA private key: {}",
            key_path.display()
        ));
    }
    Ok(reasons)
}

fn caroot_path() -> Result<PathBuf, String> {
    if let Some(path) = std::env::var_os("CAROOT").filter(|value| !value.is_empty()) {
        return Ok(PathBuf::from(path));
    }

    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())?;
    Ok(home.join("Library/Application Support/mkcert"))
}

fn file_is_non_empty(path: &Path) -> Result<bool, String> {
    let metadata = std::fs::metadata(path)
        .map_err(|err| format!("failed to stat {}: {err}", path.display()))?;
    Ok(metadata.is_file() && metadata.len() > 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    struct EnvGuard {
        key: &'static str,
        previous: Option<std::ffi::OsString>,
    }

    impl EnvGuard {
        fn set(key: &'static str, value: &Path) -> Self {
            let previous = std::env::var_os(key);
            unsafe { std::env::set_var(key, value) };
            Self { key, previous }
        }

        fn unset(key: &'static str) -> Self {
            let previous = std::env::var_os(key);
            unsafe { std::env::remove_var(key) };
            Self { key, previous }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            match &self.previous {
                Some(value) => unsafe { std::env::set_var(self.key, value) },
                None => unsafe { std::env::remove_var(self.key) },
            }
        }
    }

    #[test]
    fn install_detection_uses_explicit_caroot_and_reports_plaintext_key() {
        let _lock = crate::global_test_env_lock().lock().unwrap();
        let temp = std::env::temp_dir().join(format!("mkcert-detect-{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp);
        fs::create_dir_all(&temp).unwrap();
        fs::write(temp.join("rootCA-key.pem"), "PRIVATE KEY").unwrap();
        let _caroot = EnvGuard::set("CAROOT", &temp);

        let reasons = install_insecurity_reasons().unwrap();

        assert_eq!(reasons.len(), 1);
        assert!(reasons[0].contains("plaintext root CA private key"));
        let _ = fs::remove_dir_all(temp);
    }

    #[test]
    fn install_detection_uses_default_home_path_and_ignores_empty_or_non_file_keys() {
        let _lock = crate::global_test_env_lock().lock().unwrap();
        let temp = std::env::temp_dir().join(format!("mkcert-home-{}", std::process::id()));
        let caroot = temp.join("Library/Application Support/mkcert");
        let _ = fs::remove_dir_all(&temp);
        fs::create_dir_all(&caroot).unwrap();
        let _home = EnvGuard::set("HOME", &temp);
        let _caroot = EnvGuard::unset("CAROOT");

        assert_eq!(caroot_path().unwrap(), caroot);
        assert!(!install_is_insecure().unwrap());

        fs::write(caroot.join("rootCA-key.pem"), "").unwrap();
        assert!(!install_is_insecure().unwrap());

        fs::remove_file(caroot.join("rootCA-key.pem")).unwrap();
        fs::create_dir(caroot.join("rootCA-key.pem")).unwrap();
        assert!(!file_is_non_empty(&caroot.join("rootCA-key.pem")).unwrap());

        let _ = fs::remove_dir_all(temp);
    }

    #[test]
    fn caroot_path_requires_home_when_override_is_missing() {
        let _lock = crate::global_test_env_lock().lock().unwrap();
        let _home = EnvGuard::unset("HOME");
        let _caroot = EnvGuard::unset("CAROOT");
        assert_eq!(caroot_path().unwrap_err(), "HOME is not set");
    }

    #[test]
    fn top_level_install_is_insecure_returns_false_when_default_locations_are_missing() {
        let _lock = crate::global_test_env_lock().lock().unwrap();
        let home = std::env::temp_dir().join(format!(
            "{}-detect-missing-{}",
            module_path!().replace(':', "_"),
            std::process::id()
        ));
        let xdg = home.join("xdg");
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(&xdg).unwrap();

        let previous_home = std::env::var_os("HOME");
        let previous_xdg = std::env::var_os("XDG_CONFIG_HOME");
        unsafe {
            std::env::set_var("HOME", &home);
            std::env::set_var("XDG_CONFIG_HOME", &xdg);
        }

        let result = install_is_insecure().unwrap();

        unsafe {
            match previous_home {
                Some(value) => std::env::set_var("HOME", value),
                None => std::env::remove_var("HOME"),
            }
            match previous_xdg {
                Some(value) => std::env::set_var("XDG_CONFIG_HOME", value),
                None => std::env::remove_var("XDG_CONFIG_HOME"),
            }
        }

        assert!(!result);
        std::fs::remove_dir_all(home).unwrap();
    }
}

pub(crate) fn findings(home: &std::path::Path) -> Vec<crate::Finding> {
    super::radioisotope::findings("mkcert", install_insecurity_reasons, home)
}
