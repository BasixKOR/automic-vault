#![allow(dead_code)]

use std::path::{Path, PathBuf};

pub fn install_is_insecure() -> Result<bool, String> {
    install_insecurity_reasons().map(|reasons| !reasons.is_empty())
}

pub fn install_insecurity_reasons() -> Result<Vec<String>, String> {
    let mut reasons = Vec::new();
    for path in candidate_config_files()? {
        if path.exists() && databricks_config_contains_secret(&read_to_string(&path)?) {
            reasons.push(format!(
                "Databricks config contains plaintext profile credentials: {}",
                path.display()
            ));
        }
    }
    reasons.sort();
    reasons.dedup();
    Ok(reasons)
}

fn candidate_config_files() -> Result<Vec<PathBuf>, String> {
    let home = home_dir()?;
    let mut paths = vec![home.join(".databrickscfg")];
    if let Some(config_home) = xdg_config_home() {
        paths.push(config_home.join("databricks/config"));
        paths.push(config_home.join("databricks/databrickscfg"));
    }
    paths.push(home.join(".config/databricks/config"));
    paths.push(home.join(".config/databricks/databrickscfg"));
    Ok(paths)
}

fn home_dir() -> Result<PathBuf, String> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())
}

fn xdg_config_home() -> Option<PathBuf> {
    std::env::var_os("XDG_CONFIG_HOME")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

fn read_to_string(path: &Path) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|err| format!("failed to read {}: {err}", path.display()))
}

fn databricks_config_contains_secret(contents: &str) -> bool {
    contents.lines().any(|line| {
        let line = line.split('#').next().unwrap_or("").trim();
        let Some((key, value)) = line.split_once('=') else {
            return false;
        };
        let key = key.trim();
        let value = trim_quotes(value.trim());
        matches!(
            key,
            "token" | "password" | "client_secret" | "azure_client_secret" | "google_credentials"
        ) && secret_value(value)
    })
}

fn trim_quotes(value: &str) -> &str {
    value.trim_matches('"').trim_matches('\'')
}

fn secret_value(value: &str) -> bool {
    let value = value.trim();
    !value.is_empty() && !value.contains("${") && value != "null"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_profile_tokens() {
        assert!(databricks_config_contains_secret(
            "[prod]\nhost = https://example.cloud.databricks.com\ntoken = dapi-secret\n"
        ));
        assert!(!databricks_config_contains_secret(
            "[prod]\nhost = https://example.cloud.databricks.com\n"
        ));
    }
}

pub(crate) fn findings(home: &std::path::Path) -> Vec<crate::Finding> {
    super::radioisotope::findings("databricks", install_insecurity_reasons, home)
}
