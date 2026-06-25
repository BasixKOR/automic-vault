#![allow(dead_code)]

use std::path::{Path, PathBuf};

pub fn install_is_insecure() -> Result<bool, String> {
    install_insecurity_reasons().map(|reasons| !reasons.is_empty())
}

pub fn install_insecurity_reasons() -> Result<Vec<String>, String> {
    let path = oxide_credentials_path()?;
    if path.exists() && credentials_contain_token(&read_to_string(&path)?) {
        return Ok(vec![format!(
            "Oxide CLI credentials contain plaintext access tokens: {}",
            path.display()
        )]);
    }
    Ok(Vec::new())
}

fn oxide_credentials_path() -> Result<PathBuf, String> {
    Ok(user_home()?
        .join(".config")
        .join("oxide")
        .join("credentials.toml"))
}

fn user_home() -> Result<PathBuf, String> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())
}

fn read_to_string(path: &Path) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|err| format!("failed to read {}: {err}", path.display()))
}

fn credentials_contain_token(contents: &str) -> bool {
    contents.lines().any(|line| {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            return false;
        }
        let Some((key, value)) = trimmed.split_once('=') else {
            return false;
        };
        key.trim() == "token" && !toml_string_value(value).unwrap_or_default().is_empty()
    })
}

fn toml_string_value(value: &str) -> Option<&str> {
    let value = value.trim();
    value
        .strip_prefix('"')
        .and_then(|value| value.split_once('"').map(|(value, _)| value))
        .or_else(|| {
            value
                .strip_prefix('\'')
                .and_then(|value| value.split_once('\'').map(|(value, _)| value))
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_profile_tokens() {
        assert!(credentials_contain_token(
            "[profile.prod]\nhost = \"https://oxide.example\"\ntoken = \"fake-oxide-token\"\n"
        ));
    }

    #[test]
    fn ignores_comments_and_empty_tokens() {
        assert!(!credentials_contain_token(
            "# token = \"fake-token\"\n[profile.prod]\ntoken = \"\"\n"
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
    super::radioisotope::findings("oxide-cli", install_insecurity_reasons, home)
}
