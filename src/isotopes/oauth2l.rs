#![allow(dead_code)]

use std::path::{Path, PathBuf};

pub fn install_is_insecure() -> Result<bool, String> {
    install_insecurity_reasons().map(|reasons| !reasons.is_empty())
}

pub fn install_insecurity_reasons() -> Result<Vec<String>, String> {
    let path = home_dir()?.join(".oauth2l");
    if path.exists() && oauth2l_cache_contains_token(&read_to_string(&path)?) {
        Ok(vec![format!(
            "oauth2l default cache contains plaintext OAuth tokens: {}",
            path.display()
        )])
    } else {
        Ok(Vec::new())
    }
}

fn home_dir() -> Result<PathBuf, String> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())
}

fn read_to_string(path: &Path) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|err| format!("failed to read {}: {err}", path.display()))
}

fn oauth2l_cache_contains_token(contents: &str) -> bool {
    ["access_token", "refresh_token", "ya29.", "\"token\""]
        .iter()
        .any(|needle| contents.contains(needle))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_cache_tokens() {
        assert!(oauth2l_cache_contains_token(
            r#"{"access_token":"ya29.example","refresh_token":"refresh"}"#
        ));
        assert!(!oauth2l_cache_contains_token("{}"));
    }
}

pub(crate) fn findings(home: &std::path::Path) -> Vec<crate::Finding> {
    super::radioisotope::findings("oauth2l", install_insecurity_reasons, home)
}
