#![allow(dead_code)]

use std::path::{Path, PathBuf};

pub fn install_is_insecure() -> Result<bool, String> {
    install_insecurity_reasons().map(|reasons| !reasons.is_empty())
}

pub fn install_insecurity_reasons() -> Result<Vec<String>, String> {
    let path = dropbox_uploader_config_path()?;
    if path.exists() && has_secret_assignment(&read_to_string(&path)?) {
        return Ok(vec![format!(
            "Dropbox Uploader config contains plaintext OAuth credentials: {}",
            path.display()
        )]);
    }
    Ok(Vec::new())
}

fn dropbox_uploader_config_path() -> Result<PathBuf, String> {
    Ok(user_home()?.join(".dropbox_uploader"))
}

fn user_home() -> Result<PathBuf, String> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())
}

fn read_to_string(path: &Path) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|err| format!("failed to read {}: {err}", path.display()))
}

fn has_secret_assignment(contents: &str) -> bool {
    contents.lines().any(|line| {
        parse_assignment(line)
            .filter(|(name, value)| is_secret_name(name) && !value.trim().is_empty())
            .is_some()
    })
}

fn parse_assignment(line: &str) -> Option<(&str, &str)> {
    let trimmed = line.trim_start();
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return None;
    }
    let (name, value) = trimmed.split_once('=')?;
    Some((name.trim(), value.trim()))
}

fn is_secret_name(name: &str) -> bool {
    matches!(
        name,
        "OAUTH_ACCESS_TOKEN" | "OAUTH_ACCESS_TOKEN_SECRET" | "APPSECRET"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_oauth_token_and_legacy_secrets() {
        assert!(has_secret_assignment("OAUTH_ACCESS_TOKEN=fake-token\n"));
        assert!(has_secret_assignment("APPSECRET=fake-secret\n"));
        assert!(has_secret_assignment(
            "OAUTH_ACCESS_TOKEN_SECRET=fake-secret\n"
        ));
    }

    #[test]
    fn ignores_comments_non_secret_values_and_empty_tokens() {
        assert!(!has_secret_assignment(
            "# OAUTH_ACCESS_TOKEN=fake\nAPPKEY=fake-app\nOAUTH_ACCESS_TOKEN=\n"
        ));
    }
}

pub(crate) fn findings(home: &std::path::Path) -> Vec<crate::Finding> {
    super::radioisotope::findings("dropbox-uploader", install_insecurity_reasons, home)
}
