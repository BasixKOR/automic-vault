#![allow(dead_code)]

use std::path::{Path, PathBuf};

pub fn install_is_insecure() -> Result<bool, String> {
    install_insecurity_reasons().map(|reasons| !reasons.is_empty())
}

pub fn install_insecurity_reasons() -> Result<Vec<String>, String> {
    let path = vt_config_path()?;
    if path.exists() && toml_contains_non_empty_string_key(&read_to_string(&path)?, "apikey") {
        return Ok(vec![format!(
            "VirusTotal config contains a plaintext API key: {}",
            path.display()
        )]);
    }
    Ok(Vec::new())
}

fn vt_config_path() -> Result<PathBuf, String> {
    Ok(user_home()?.join(".vt.toml"))
}

fn user_home() -> Result<PathBuf, String> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())
}

fn read_to_string(path: &Path) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|err| format!("failed to read {}: {err}", path.display()))
}

fn toml_contains_non_empty_string_key(contents: &str, key: &str) -> bool {
    contents.lines().any(|line| {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            return false;
        }
        let Some((line_key, value)) = line.split_once('=') else {
            return false;
        };
        line_key.trim() == key && !toml_string_value(value).unwrap_or_default().is_empty()
    })
}

fn toml_string_value(value: &str) -> Option<&str> {
    value
        .trim()
        .strip_prefix('"')
        .and_then(|value| value.split_once('"').map(|(value, _)| value))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn detects_apikey_in_toml() {
        assert!(toml_contains_non_empty_string_key(
            "apikey=\"fake-vt-key\"\nproxy=\"http://proxy\"\n",
            "apikey"
        ));
        assert!(!toml_contains_non_empty_string_key(
            "apikey=\"\"\n",
            "apikey"
        ));
        assert!(!toml_contains_non_empty_string_key(
            "# apikey=\"fake-vt-key\"\n",
            "apikey"
        ));
    }

    #[test]
    fn top_level_install_is_insecure_returns_false_when_default_location_is_missing() {
        let _lock = crate::global_test_env_lock().lock().unwrap();
        let home = std::env::temp_dir().join(format!(
            "{}-detect-missing-{}",
            module_path!().replace(':', "_"),
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(&home).unwrap();

        let previous_home = std::env::var_os("HOME");
        unsafe {
            std::env::set_var("HOME", &home);
        }

        let result = install_is_insecure().unwrap();

        unsafe {
            match previous_home {
                Some(value) => std::env::set_var("HOME", value),
                None => std::env::remove_var("HOME"),
            }
        }

        assert!(!result);
        std::fs::remove_dir_all(home).unwrap();
    }

    #[test]
    fn path_and_toml_helpers_cover_edge_cases() {
        let _lock = crate::global_test_env_lock().lock().unwrap();
        let home = std::env::temp_dir().join(format!("vt-detect-home-{}", std::process::id()));
        let previous_home = std::env::var_os("HOME");
        unsafe {
            std::env::set_var("HOME", &home);
        }
        assert_eq!(vt_config_path().unwrap(), home.join(".vt.toml"));
        unsafe {
            std::env::remove_var("HOME");
        }
        assert_eq!(vt_config_path().unwrap_err(), "HOME is not set");
        unsafe {
            match previous_home {
                Some(value) => std::env::set_var("HOME", value),
                None => std::env::remove_var("HOME"),
            }
        }

        assert_eq!(toml_string_value("plain"), None);
        assert_eq!(toml_string_value(r#""unterminated"#), None);
        assert!(!toml_contains_non_empty_string_key(
            "apikey = 1\n",
            "apikey"
        ));
    }

    #[test]
    fn install_insecurity_reasons_reports_vt_config_path() {
        let _lock = crate::global_test_env_lock().lock().unwrap();
        let home = std::env::temp_dir().join(format!("vt-detect-report-{}", std::process::id()));
        let _ = fs::remove_dir_all(&home);
        fs::create_dir_all(&home).unwrap();
        let config = home.join(".vt.toml");
        fs::write(&config, "apikey=\"fake-vt-key\"\n").unwrap();
        let previous_home = std::env::var_os("HOME");
        unsafe {
            std::env::set_var("HOME", &home);
        }

        let reasons = install_insecurity_reasons().unwrap();

        unsafe {
            match previous_home {
                Some(value) => std::env::set_var("HOME", value),
                None => std::env::remove_var("HOME"),
            }
        }

        assert_eq!(reasons.len(), 1);
        assert!(reasons[0].contains(config.to_str().unwrap()));
        fs::remove_dir_all(home).unwrap();
    }
}

pub(crate) fn findings(home: &std::path::Path) -> Vec<crate::Finding> {
    super::radioisotope::findings("virustotal-cli", install_insecurity_reasons, home)
}
