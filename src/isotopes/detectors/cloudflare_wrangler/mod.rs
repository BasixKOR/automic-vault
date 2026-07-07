#![allow(dead_code)]

use std::path::{Path, PathBuf};

pub fn install_is_insecure() -> Result<bool, String> {
    install_insecurity_reasons().map(|reasons| !reasons.is_empty())
}

pub fn install_insecurity_reasons() -> Result<Vec<String>, String> {
    let mut reasons = Vec::new();
    for path in candidate_auth_files()? {
        if path.exists() && wrangler_auth_file_contains_secret(&read_to_string(&path)?) {
            reasons.push(format!(
                "Wrangler auth config contains plaintext Cloudflare tokens: {}",
                path.display()
            ));
        }
    }
    reasons.sort();
    reasons.dedup();
    Ok(reasons)
}

fn candidate_auth_files() -> Result<Vec<PathBuf>, String> {
    let home = home_dir()?;
    let mut roots = vec![
        home.join(".wrangler"),
        home.join("Library/Preferences/.wrangler"),
        home.join(".config/.wrangler"),
    ];
    if let Some(config_home) = xdg_config_home() {
        roots.push(config_home.join(".wrangler"));
    }

    let mut paths = Vec::new();
    for root in roots {
        paths.push(root.join("config/default.toml"));
        if let Ok(entries) = std::fs::read_dir(root.join("config")) {
            for entry in entries.flatten() {
                if entry
                    .file_type()
                    .map(|kind| kind.is_file())
                    .unwrap_or(false)
                    && entry.path().extension().and_then(|value| value.to_str()) == Some("toml")
                {
                    paths.push(entry.path());
                }
            }
        }
    }
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

fn wrangler_auth_file_contains_secret(contents: &str) -> bool {
    ["oauth_token", "refresh_token", "api_token"]
        .iter()
        .any(|key| assignment_value(contents, key).is_some_and(|value| secret_value(&value)))
}

fn assignment_value(contents: &str, name: &str) -> Option<String> {
    for line in contents.lines() {
        let line = line.split('#').next().unwrap_or("").trim();
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        if key.trim() == name {
            return Some(trim_quotes(value.trim()).to_string());
        }
    }
    None
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
    fn detects_oauth_tokens() {
        assert!(wrangler_auth_file_contains_secret(
            "oauth_token = \"access\"\nrefresh_token = \"refresh\"\n"
        ));
        assert!(!wrangler_auth_file_contains_secret(
            "scopes = [\"user:read\"]\n"
        ));
    }
}

pub(crate) fn findings(home: &std::path::Path) -> Vec<crate::Finding> {
    super::radioisotope::findings("cloudflare-wrangler", install_insecurity_reasons, home)
}
