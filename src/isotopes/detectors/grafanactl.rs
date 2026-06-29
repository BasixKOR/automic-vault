#![allow(dead_code)]

use std::path::{Path, PathBuf};

pub fn install_is_insecure() -> Result<bool, String> {
    install_insecurity_reasons().map(|reasons| !reasons.is_empty())
}

pub fn install_insecurity_reasons() -> Result<Vec<String>, String> {
    let path = grafanactl_config_path()?;
    if path.exists() && config_contains_secret(&read_to_string(&path)?) {
        return Ok(vec![format!(
            "grafanactl config contains plaintext credentials: {}",
            path.display()
        )]);
    }
    Ok(Vec::new())
}

fn grafanactl_config_path() -> Result<PathBuf, String> {
    if let Some(config_home) = std::env::var_os("XDG_CONFIG_HOME").filter(|value| !value.is_empty())
    {
        return Ok(PathBuf::from(config_home).join("grafanactl/config.yaml"));
    }
    Ok(user_home()?.join(".config/grafanactl/config.yaml"))
}

fn user_home() -> Result<PathBuf, String> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())
}

fn read_to_string(path: &Path) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|err| format!("failed to read {}: {err}", path.display()))
}

fn config_contains_secret(contents: &str) -> bool {
    contents.lines().any(yaml_secret_line_is_present)
}

fn yaml_secret_line_is_present(line: &str) -> bool {
    let line = line.split('#').next().unwrap_or("").trim();
    let Some((name, value)) = line.split_once(':') else {
        return false;
    };
    let name = name.trim();
    let value = value.trim().trim_matches('"').trim_matches('\'');
    matches!(name, "token" | "password") && !value.is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_tokens_and_passwords() {
        assert!(config_contains_secret(
            "contexts:\n  default:\n    grafana:\n      token: fake-token\n"
        ));
        assert!(config_contains_secret(
            "contexts:\n  default:\n    grafana:\n      password: \"fake-password\"\n"
        ));
    }

    #[test]
    fn ignores_empty_or_unrelated_config() {
        assert!(!config_contains_secret(
            "contexts:\n  default:\n    token: ''\n"
        ));
        assert!(!config_contains_secret(
            "contexts:\n  default:\n    grafana:\n      server: http://localhost:3000\n"
        ));
    }

    #[test]
    fn top_level_install_is_insecure_returns_false_when_default_location_is_missing() {
        let home = std::env::temp_dir().join(format!(
            "{}-detect-missing-{}",
            module_path!().replace(':', "_"),
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(&home).unwrap();

        let previous_home = std::env::var_os("HOME");
        let previous_config = std::env::var_os("XDG_CONFIG_HOME");
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
            match previous_config {
                Some(value) => std::env::set_var("XDG_CONFIG_HOME", value),
                None => std::env::remove_var("XDG_CONFIG_HOME"),
            }
        }

        assert!(!result);
        std::fs::remove_dir_all(home).unwrap();
    }
}

pub(crate) fn findings(home: &std::path::Path) -> Vec<crate::Finding> {
    super::radioisotope::findings("grafanactl", install_insecurity_reasons, home)
}
