#![allow(dead_code)]

use std::path::{Path, PathBuf};

pub fn install_is_insecure() -> Result<bool, String> {
    install_insecurity_reasons().map(|reasons| !reasons.is_empty())
}

pub fn install_insecurity_reasons() -> Result<Vec<String>, String> {
    let mut reasons = Vec::new();
    for path in vultr_config_paths()? {
        if path.exists() && config_contains_api_key(&read_to_string(&path)?) {
            reasons.push(format!(
                "vultr-cli config contains a plaintext API key: {}",
                path.display()
            ));
        }
    }
    Ok(reasons)
}

fn vultr_config_paths() -> Result<Vec<PathBuf>, String> {
    let home = user_home()?;
    Ok(vec![
        home.join("Library/Application Support/vultr-cli.yaml"),
        home.join(".vultr-cli.yaml"),
    ])
}

fn user_home() -> Result<PathBuf, String> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())
}

fn read_to_string(path: &Path) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|err| format!("failed to read {}: {err}", path.display()))
}

fn config_contains_api_key(contents: &str) -> bool {
    contents.lines().any(line_has_api_key)
}

fn line_has_api_key(line: &str) -> bool {
    let line = line.trim();
    if line.is_empty() || line.starts_with('#') {
        return false;
    }
    let Some((key, value)) = line.split_once(':') else {
        return false;
    };
    key.trim() == "api-key" && !yaml_scalar_value(value).unwrap_or_default().is_empty()
}

fn yaml_scalar_value(value: &str) -> Option<&str> {
    let value = value.trim();
    if value.is_empty() {
        return None;
    }
    Some(value.trim_matches('"').trim_matches('\'').trim())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn detects_api_key() {
        assert!(config_contains_api_key("api-key: fake-vultr-key\n"));
        assert!(config_contains_api_key("api-key: \"fake-vultr-key\"\n"));
    }

    #[test]
    fn ignores_empty_or_commented_api_key() {
        assert!(!config_contains_api_key("api-key:\n"));
        assert!(!config_contains_api_key("api-key: ''\n"));
        assert!(!config_contains_api_key("# api-key: fake-vultr-key\n"));
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
    fn helper_paths_and_yaml_values_cover_edge_cases() {
        let _lock = crate::global_test_env_lock().lock().unwrap();
        let home = std::env::temp_dir().join(format!("vultr-detect-home-{}", std::process::id()));
        let previous_home = std::env::var_os("HOME");
        unsafe {
            std::env::set_var("HOME", &home);
        }
        let paths = vultr_config_paths().unwrap();
        assert_eq!(
            paths[0],
            home.join("Library/Application Support/vultr-cli.yaml")
        );
        assert_eq!(paths[1], home.join(".vultr-cli.yaml"));
        unsafe {
            std::env::remove_var("HOME");
        }
        assert_eq!(vultr_config_paths().unwrap_err(), "HOME is not set");
        unsafe {
            match previous_home {
                Some(value) => std::env::set_var("HOME", value),
                None => std::env::remove_var("HOME"),
            }
        }

        assert_eq!(yaml_scalar_value(""), None);
        assert_eq!(yaml_scalar_value("  \"value\"  "), Some("value"));
        assert!(!line_has_api_key("other-key: value"));
    }

    #[test]
    fn install_insecurity_reasons_reports_both_vultr_paths() {
        let _lock = crate::global_test_env_lock().lock().unwrap();
        let home = std::env::temp_dir().join(format!("vultr-detect-report-{}", std::process::id()));
        let app_support = home.join("Library/Application Support");
        let _ = fs::remove_dir_all(&home);
        fs::create_dir_all(&app_support).unwrap();
        let primary = app_support.join("vultr-cli.yaml");
        let legacy = home.join(".vultr-cli.yaml");
        fs::write(&primary, "api-key: first\n").unwrap();
        fs::write(&legacy, "api-key: second\n").unwrap();
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

        assert_eq!(reasons.len(), 2);
        assert!(
            reasons
                .iter()
                .any(|reason| reason.contains(primary.to_str().unwrap()))
        );
        assert!(
            reasons
                .iter()
                .any(|reason| reason.contains(legacy.to_str().unwrap()))
        );
        fs::remove_dir_all(home).unwrap();
    }
}

pub(crate) fn findings(home: &std::path::Path) -> Vec<crate::Finding> {
    super::radioisotope::findings("vultr", install_insecurity_reasons, home)
}
