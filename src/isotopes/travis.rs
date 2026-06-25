#![allow(dead_code)]

use std::path::{Path, PathBuf};

pub fn install_is_insecure() -> Result<bool, String> {
    install_insecurity_reasons().map(|reasons| !reasons.is_empty())
}

pub fn install_insecurity_reasons() -> Result<Vec<String>, String> {
    let path = travis_config_path()?;
    if path.exists() && config_contains_access_token(&read_to_string(&path)?) {
        return Ok(vec![format!(
            "Travis CLI config contains a plaintext access token: {}",
            path.display()
        )]);
    }
    Ok(Vec::new())
}

fn travis_config_path() -> Result<PathBuf, String> {
    let home = user_home()?;
    Ok(home.join(".travis/config.yml"))
}

fn user_home() -> Result<PathBuf, String> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())
}

fn read_to_string(path: &Path) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|err| format!("failed to read {}: {err}", path.display()))
}

fn config_contains_access_token(contents: &str) -> bool {
    contents.lines().any(line_has_access_token)
}

fn line_has_access_token(line: &str) -> bool {
    let line = line.trim();
    if line.is_empty() || line.starts_with('#') {
        return false;
    }
    let Some((key, value)) = line.split_once(':') else {
        return false;
    };
    key.trim() == "access_token" && !yaml_scalar_value(value).unwrap_or_default().is_empty()
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

    #[test]
    fn detects_access_token() {
        assert!(config_contains_access_token(
            "endpoints:\n  https://api.travis-ci.com/:\n    access_token: fake-travis-token\n"
        ));
        assert!(config_contains_access_token(
            "access_token: \"fake-travis-token\"\n"
        ));
    }

    #[test]
    fn ignores_empty_or_commented_access_token() {
        assert!(!config_contains_access_token("access_token:\n"));
        assert!(!config_contains_access_token("access_token: ''\n"));
        assert!(!config_contains_access_token(
            "# access_token: fake-travis-token\n"
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
    super::radioisotope::findings("travis", install_insecurity_reasons, home)
}
