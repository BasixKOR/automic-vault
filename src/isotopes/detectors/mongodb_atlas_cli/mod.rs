#![allow(dead_code)]

use std::path::{Path, PathBuf};

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
        if atlas_config_contains_plaintext_secret(&contents) {
            reasons.push(format!(
                "MongoDB Atlas CLI config contains plaintext credentials: {}",
                path.display()
            ));
        }
    }
    Ok(reasons)
}

fn candidate_config_paths() -> Result<Vec<PathBuf>, String> {
    let home = home_dir()?;
    let mut paths = vec![
        home.join("Library/Application Support/atlascli/config.toml"),
        home.join(".config/atlascli/config.toml"),
    ];
    if let Some(xdg_config_home) =
        std::env::var_os("XDG_CONFIG_HOME").filter(|value| !value.is_empty())
    {
        paths.push(PathBuf::from(xdg_config_home).join("atlascli/config.toml"));
    }
    paths.sort();
    paths.dedup();
    Ok(paths)
}

fn home_dir() -> Result<PathBuf, String> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())
}

fn read_to_string(path: &Path) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|err| format!("failed to read {}: {err}", path.display()))
}

fn atlas_config_contains_plaintext_secret(contents: &str) -> bool {
    contents.lines().any(|line| {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            return false;
        }
        let Some((key, value)) = trimmed.split_once('=') else {
            return false;
        };
        let key = key.trim();
        let value = value.trim().trim_matches('"').trim_matches('\'');
        matches!(
            key,
            "private_api_key" | "access_token" | "refresh_token" | "client_secret"
        ) && secret_value_is_real(value)
    })
}

fn secret_value_is_real(value: &str) -> bool {
    let value = value.trim();
    if value.len() < 6 || value.contains("${") {
        return false;
    }
    let lower = value.to_ascii_lowercase();
    !matches!(
        lower.as_str(),
        "secret" | "password" | "token" | "example" | "redacted" | "changeme"
    ) && !lower.contains("example")
        && !lower.contains("placeholder")
        && !value.starts_with('<')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_plaintext_atlas_credentials() {
        assert!(atlas_config_contains_plaintext_secret(
            "[default]\nprivate_api_key = \"private-secret\"\n"
        ));
        assert!(atlas_config_contains_plaintext_secret(
            "[default]\naccess_token = \"access-secret\"\n"
        ));
        assert!(!atlas_config_contains_plaintext_secret(
            "[default]\npublic_api_key = \"public-value\"\n"
        ));
    }
}

pub(crate) fn findings(home: &std::path::Path) -> Vec<crate::Finding> {
    super::radioisotope::findings("mongodb-atlas-cli", install_insecurity_reasons, home)
}
