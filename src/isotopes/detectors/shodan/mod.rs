#![allow(dead_code)]

use std::path::{Path, PathBuf};

pub fn install_is_insecure() -> Result<bool, String> {
    install_insecurity_reasons().map(|reasons| !reasons.is_empty())
}

pub fn install_insecurity_reasons() -> Result<Vec<String>, String> {
    let mut reasons = Vec::new();
    for path in shodan_api_key_paths()? {
        if path.exists() && api_key_file_contains_secret(&read_to_string(&path)?) {
            reasons.push(format!(
                "Shodan config contains a plaintext API key: {}",
                path.display()
            ));
        }
    }
    Ok(reasons)
}

fn shodan_api_key_paths() -> Result<Vec<PathBuf>, String> {
    let home = user_home()?;
    Ok(vec![
        home.join(".shodan/api_key"),
        config_home()?.join("shodan/api_key"),
    ])
}

fn user_home() -> Result<PathBuf, String> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())
}

fn config_home() -> Result<PathBuf, String> {
    if let Some(path) = std::env::var_os("XDG_CONFIG_HOME").filter(|value| !value.is_empty()) {
        return Ok(PathBuf::from(path));
    }
    Ok(user_home()?.join(".config"))
}

fn read_to_string(path: &Path) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|err| format!("failed to read {}: {err}", path.display()))
}

fn api_key_file_contains_secret(contents: &str) -> bool {
    !contents.trim().is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn detects_non_empty_api_key_file() {
        assert!(api_key_file_contains_secret("fake-shodan-key\n"));
        assert!(!api_key_file_contains_secret("\n \t\n"));
    }

    #[test]
    fn top_level_install_is_insecure_returns_false_when_default_locations_are_missing() {
        let _lock = crate::global_test_env_lock().lock().unwrap();
        let home = std::env::temp_dir().join(format!(
            "{}-detect-missing-{}",
            module_path!().replace(':', "_"),
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(&home).unwrap();

        let previous_home = std::env::var_os("HOME");
        let previous_config_home = std::env::var_os("XDG_CONFIG_HOME");
        unsafe {
            std::env::set_var("HOME", &home);
            std::env::remove_var("XDG_CONFIG_HOME");
        }

        let result = install_is_insecure().unwrap();

        unsafe {
            match previous_home {
                Some(value) => std::env::set_var("HOME", value),
                None => std::env::remove_var("HOME"),
            }
            match previous_config_home {
                Some(value) => std::env::set_var("XDG_CONFIG_HOME", value),
                None => std::env::remove_var("XDG_CONFIG_HOME"),
            }
        }

        assert!(!result);
        std::fs::remove_dir_all(home).unwrap();
    }

    #[test]
    fn shodan_api_key_paths_prefer_xdg_and_require_home() {
        let _lock = crate::global_test_env_lock().lock().unwrap();
        let home = std::env::temp_dir().join(format!("shodan-detect-home-{}", std::process::id()));
        let xdg = home.join("xdg");
        let previous_home = std::env::var_os("HOME");
        let previous_config_home = std::env::var_os("XDG_CONFIG_HOME");

        unsafe {
            std::env::set_var("HOME", &home);
            std::env::set_var("XDG_CONFIG_HOME", &xdg);
        }
        let paths = shodan_api_key_paths().unwrap();
        assert_eq!(paths[0], home.join(".shodan/api_key"));
        assert_eq!(paths[1], xdg.join("shodan/api_key"));

        unsafe {
            std::env::remove_var("HOME");
            std::env::remove_var("XDG_CONFIG_HOME");
        }
        assert_eq!(shodan_api_key_paths().unwrap_err(), "HOME is not set");

        unsafe {
            match previous_home {
                Some(value) => std::env::set_var("HOME", value),
                None => std::env::remove_var("HOME"),
            }
            match previous_config_home {
                Some(value) => std::env::set_var("XDG_CONFIG_HOME", value),
                None => std::env::remove_var("XDG_CONFIG_HOME"),
            }
        }
    }

    #[test]
    fn install_insecurity_reasons_reports_both_known_paths() {
        let _lock = crate::global_test_env_lock().lock().unwrap();
        let temp =
            std::env::temp_dir().join(format!("shodan-detect-report-{}", std::process::id()));
        let xdg = temp.join("xdg");
        let legacy_dir = temp.join(".shodan");
        let config_dir = xdg.join("shodan");
        let _ = fs::remove_dir_all(&temp);
        fs::create_dir_all(&legacy_dir).unwrap();
        fs::create_dir_all(&config_dir).unwrap();
        let legacy = legacy_dir.join("api_key");
        let config = config_dir.join("api_key");
        fs::write(&legacy, "legacy-key\n").unwrap();
        fs::write(&config, "config-key\n").unwrap();

        let previous_home = std::env::var_os("HOME");
        let previous_config_home = std::env::var_os("XDG_CONFIG_HOME");
        unsafe {
            std::env::set_var("HOME", &temp);
            std::env::set_var("XDG_CONFIG_HOME", &xdg);
        }

        let reasons = install_insecurity_reasons().unwrap();

        unsafe {
            match previous_home {
                Some(value) => std::env::set_var("HOME", value),
                None => std::env::remove_var("HOME"),
            }
            match previous_config_home {
                Some(value) => std::env::set_var("XDG_CONFIG_HOME", value),
                None => std::env::remove_var("XDG_CONFIG_HOME"),
            }
        }

        assert_eq!(reasons.len(), 2);
        assert!(
            reasons
                .iter()
                .any(|reason| reason.contains(legacy.to_str().unwrap()))
        );
        assert!(
            reasons
                .iter()
                .any(|reason| reason.contains(config.to_str().unwrap()))
        );
        fs::remove_dir_all(temp).unwrap();
    }
}

pub(crate) fn findings(home: &std::path::Path) -> Vec<crate::Finding> {
    super::radioisotope::findings("shodan", install_insecurity_reasons, home)
}
