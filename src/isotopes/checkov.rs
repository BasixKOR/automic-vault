#![allow(dead_code)]

use std::path::PathBuf;

pub fn install_is_insecure() -> Result<bool, String> {
    install_insecurity_reasons().map(|reasons| !reasons.is_empty())
}

pub fn install_insecurity_reasons() -> Result<Vec<String>, String> {
    let path = bridgecrew_credentials_path()?;
    if path.exists() && credentials_file_has_secret(&read_to_string(&path)?) {
        return Ok(vec![format!(
            "Checkov API key is stored in plaintext credentials file: {}",
            path.display()
        )]);
    }
    Ok(Vec::new())
}

fn bridgecrew_credentials_path() -> Result<PathBuf, String> {
    Ok(user_home()?.join(".bridgecrew/credentials"))
}

fn user_home() -> Result<PathBuf, String> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())
}

fn read_to_string(path: &std::path::Path) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|err| format!("failed to read {}: {err}", path.display()))
}

fn credentials_file_has_secret(contents: &str) -> bool {
    !contents.trim().is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_non_empty_credentials_file() {
        assert!(credentials_file_has_secret("access_key::secret_key\n"));
        assert!(credentials_file_has_secret("bridgecrew-token"));
    }

    #[test]
    fn ignores_empty_credentials_file() {
        assert!(!credentials_file_has_secret(""));
        assert!(!credentials_file_has_secret("\n  \t"));
    }

    #[test]
    fn top_level_install_is_insecure_returns_false_when_default_locations_are_missing() {
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
    super::radioisotope::findings("checkov", install_insecurity_reasons, home)
}
