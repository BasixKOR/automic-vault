#![allow(dead_code)]

use std::path::{Path, PathBuf};

pub fn install_is_insecure() -> Result<bool, String> {
    install_insecurity_reasons().map(|reasons| !reasons.is_empty())
}

pub fn install_insecurity_reasons() -> Result<Vec<String>, String> {
    let config = docker_config_path()?;
    if config.exists() && docker_config_uses_packaged_helper(&read_to_string(&config)?) {
        Ok(vec![format!(
            "Docker config uses an ambient Docker credential helper: {}",
            config.display()
        )])
    } else {
        Ok(Vec::new())
    }
}

fn docker_config_path() -> Result<PathBuf, String> {
    if let Some(config) = std::env::var_os("DOCKER_CONFIG").filter(|value| !value.is_empty()) {
        return Ok(PathBuf::from(config).join("config.json"));
    }
    Ok(home_dir()?.join(".docker/config.json"))
}

fn home_dir() -> Result<PathBuf, String> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())
}

fn read_to_string(path: &Path) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|err| format!("failed to read {}: {err}", path.display()))
}

fn docker_config_uses_packaged_helper(contents: &str) -> bool {
    ["osxkeychain", "secretservice", "pass", "wincred"]
        .iter()
        .any(|helper| {
            contents.contains(&format!(r#""credsStore":"{helper}""#))
                || contents.contains(&format!(r#""credsStore": "{helper}""#))
                || contents.contains(&format!(r#"":"{helper}""#))
                || contents.contains(&format!(r#"": "{helper}""#))
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_docker_helper_config() {
        assert!(docker_config_uses_packaged_helper(
            r#"{"credsStore":"osxkeychain"}"#
        ));
        assert!(docker_config_uses_packaged_helper(
            r#"{"credHelpers":{"registry.example.com":"pass"}}"#
        ));
        assert!(!docker_config_uses_packaged_helper(r#"{"auths":{}}"#));
    }
}

pub(crate) fn findings(home: &std::path::Path) -> Vec<crate::Finding> {
    super::radioisotope::findings("docker-credential-helper", install_insecurity_reasons, home)
}
