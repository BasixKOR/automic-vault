#![allow(dead_code)]

use std::path::{Path, PathBuf};

const SECRET_KEYS: &[&str] = &["password", "token", "application_credential_secret"];

pub fn install_is_insecure() -> Result<bool, String> {
    install_insecurity_reasons().map(|reasons| !reasons.is_empty())
}

pub fn install_insecurity_reasons() -> Result<Vec<String>, String> {
    let mut reasons = Vec::new();

    for path in candidate_config_paths()? {
        if !path.exists() {
            continue;
        }

        let contents = read_to_string(&path)?;
        if config_has_sensitive_values(&contents) {
            reasons.push(format!(
                "OpenStack config contains plaintext credentials: {}",
                path.display()
            ));
        }
    }

    Ok(reasons)
}

fn candidate_config_paths() -> Result<Vec<PathBuf>, String> {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())?;
    Ok(vec![
        home.join(".config/openstack/clouds.yaml"),
        home.join(".config/openstack/secure.yaml"),
    ])
}

fn read_to_string(path: &Path) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|err| format!("failed to read {}: {err}", path.display()))
}

fn config_has_sensitive_values(contents: &str) -> bool {
    contents.lines().any(line_has_sensitive_value)
}

fn line_has_sensitive_value(line: &str) -> bool {
    let trimmed = trim_yaml_list_marker(line.trim_start());
    let Some((key, value)) = trimmed.split_once(':') else {
        return false;
    };
    let key = key.trim().to_ascii_lowercase();
    if !SECRET_KEYS.contains(&key.as_str()) {
        return false;
    }

    let value = value.trim();
    !value.is_empty() && value != "\"\"" && value != "''"
}

fn trim_yaml_list_marker(line: &str) -> &str {
    line.strip_prefix("- ").unwrap_or(line)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_passwords_tokens_and_app_credential_secrets() {
        assert!(config_has_sensitive_values("password: hunter2\n"));
        assert!(config_has_sensitive_values("token: abc123\n"));
        assert!(config_has_sensitive_values(
            "application_credential_secret: topsecret\n"
        ));
    }

    #[test]
    fn ignores_empty_secret_values() {
        assert!(!config_has_sensitive_values("password: \"\"\n"));
        assert!(!config_has_sensitive_values("token: ''\n"));
        assert!(!config_has_sensitive_values(
            "application_credential_secret:\n"
        ));
    }

    #[test]
    fn top_level_install_is_insecure_returns_false_when_default_locations_are_missing() {
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
}

pub(crate) fn findings(home: &std::path::Path) -> Vec<crate::Finding> {
    super::radioisotope::findings("openstackclient", install_insecurity_reasons, home)
}
