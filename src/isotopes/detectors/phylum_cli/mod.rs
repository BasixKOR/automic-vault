#![allow(dead_code)]

use std::path::{Path, PathBuf};

pub fn install_is_insecure() -> Result<bool, String> {
    install_insecurity_reasons().map(|reasons| !reasons.is_empty())
}

pub fn install_insecurity_reasons() -> Result<Vec<String>, String> {
    let path = phylum_config_path()?;
    if path.exists() && yaml_value(&read_to_string(&path)?, "offline_access").is_some() {
        return Ok(vec![format!(
            "Phylum config contains a plaintext API token: {}",
            path.display()
        )]);
    }
    Ok(Vec::new())
}

fn phylum_config_path() -> Result<PathBuf, String> {
    Ok(config_home()?.join("phylum/settings.yaml"))
}

fn config_home() -> Result<PathBuf, String> {
    if let Some(path) = std::env::var_os("XDG_CONFIG_HOME").filter(|value| !value.is_empty()) {
        return Ok(PathBuf::from(path));
    }
    Ok(user_home()?.join(".config"))
}

fn user_home() -> Result<PathBuf, String> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())
}

fn read_to_string(path: &Path) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|err| format!("failed to read {}: {err}", path.display()))
}

fn yaml_value(contents: &str, key: &str) -> Option<String> {
    contents.lines().find_map(|line| {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            return None;
        }
        let (name, value) = trimmed.split_once(':')?;
        if name.trim() != key {
            return None;
        }
        let value = value.trim().trim_matches('"').trim_matches('\'');
        if value.is_empty() || value == "null" {
            None
        } else {
            Some(value.to_string())
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_offline_access_token() {
        assert_eq!(
            yaml_value(
                "auth_info:\n  offline_access: ph0_fake-token\n",
                "offline_access"
            ),
            Some("ph0_fake-token".to_string())
        );
    }

    #[test]
    fn ignores_empty_or_null_tokens() {
        assert_eq!(yaml_value("offline_access:\n", "offline_access"), None);
        assert_eq!(yaml_value("offline_access: null\n", "offline_access"), None);
    }
}

pub(crate) fn findings(home: &std::path::Path) -> Vec<crate::Finding> {
    super::radioisotope::findings("phylum-cli", install_insecurity_reasons, home)
}
