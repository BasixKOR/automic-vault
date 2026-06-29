#![allow(dead_code)]

use std::path::{Path, PathBuf};

pub fn install_is_insecure() -> Result<bool, String> {
    install_insecurity_reasons().map(|reasons| !reasons.is_empty())
}

pub fn install_insecurity_reasons() -> Result<Vec<String>, String> {
    let path = sslmate_config_path()?;
    if path.exists() && sslmate_api_key(&read_to_string(&path)?).is_some() {
        return Ok(vec![format!(
            "SSLMate config contains a plaintext API key: {}",
            path.display()
        )]);
    }
    Ok(Vec::new())
}

fn sslmate_config_path() -> Result<PathBuf, String> {
    Ok(user_home()?.join(".sslmate"))
}

fn user_home() -> Result<PathBuf, String> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())
}

fn read_to_string(path: &Path) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|err| format!("failed to read {}: {err}", path.display()))
}

fn sslmate_api_key(contents: &str) -> Option<String> {
    let api_key = config_value(contents, &["api_key", "api-key"])?;
    let api_key = api_key.trim();
    if api_key.is_empty() {
        return None;
    }

    let account_id = config_value(contents, &["account_id", "account-id"]);
    Some(normalize_api_key(account_id.as_deref(), api_key))
}

fn normalize_api_key(account_id: Option<&str>, api_key: &str) -> String {
    if api_key.contains('_') {
        return api_key.to_string();
    }

    account_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| format!("{value}_{api_key}"))
        .unwrap_or_else(|| api_key.to_string())
}

fn config_value(contents: &str, names: &[&str]) -> Option<String> {
    contents.lines().find_map(|line| {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            return None;
        }
        let (name, value) = trimmed.split_once(char::is_whitespace)?;
        if names.iter().any(|candidate| name == *candidate) {
            Some(value.trim().to_string())
        } else {
            None
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_current_and_legacy_api_key_names() {
        assert_eq!(
            sslmate_api_key("api_key acct_fake-secret\n"),
            Some("acct_fake-secret".to_string())
        );
        assert_eq!(
            sslmate_api_key("account-id acct\napi-key fake-secret\n"),
            Some("acct_fake-secret".to_string())
        );
    }

    #[test]
    fn ignores_comments_and_empty_keys() {
        assert_eq!(sslmate_api_key("# api_key fake-secret\napi_key   \n"), None);
    }
}

pub(crate) fn findings(home: &std::path::Path) -> Vec<crate::Finding> {
    super::radioisotope::findings("sslmate", install_insecurity_reasons, home)
}
