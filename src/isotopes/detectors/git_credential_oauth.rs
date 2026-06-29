#![allow(dead_code)]

use std::path::{Path, PathBuf};

pub fn install_is_insecure() -> Result<bool, String> {
    install_insecurity_reasons().map(|reasons| !reasons.is_empty())
}

pub fn install_insecurity_reasons() -> Result<Vec<String>, String> {
    let mut reasons = Vec::new();
    for path in git_config_paths()? {
        if !path.exists() {
            continue;
        }
        let contents = read_to_string(&path)?;
        if git_config_uses_oauth_helper(&contents) {
            reasons.push(format!(
                "Git config enables git-credential-oauth as an ambient credential helper: {}",
                path.display()
            ));
        }
        if git_config_contains_oauth_client_secret(&contents) {
            reasons.push(format!(
                "Git config contains a plaintext OAuth client secret: {}",
                path.display()
            ));
        }
    }
    Ok(reasons)
}

fn git_config_paths() -> Result<Vec<PathBuf>, String> {
    let home = home_dir()?;
    let mut paths = vec![home.join(".gitconfig")];
    if let Some(config_home) = std::env::var_os("XDG_CONFIG_HOME").filter(|value| !value.is_empty())
    {
        paths.push(PathBuf::from(config_home).join("git/config"));
    } else {
        paths.push(home.join(".config/git/config"));
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

fn git_config_uses_oauth_helper(contents: &str) -> bool {
    contents.lines().any(|line| {
        let line = uncomment(line).trim();
        let Some((key, value)) = line.split_once('=') else {
            return false;
        };
        key.trim().ends_with("helper")
            && value
                .split_whitespace()
                .any(|word| word.trim_matches('"').trim_matches('\'') == "oauth")
    })
}

fn git_config_contains_oauth_client_secret(contents: &str) -> bool {
    contents.lines().any(|line| {
        let line = uncomment(line).trim();
        let Some((key, value)) = line.split_once('=') else {
            return false;
        };
        key.trim().ends_with("oauthClientSecret") && secret_value_is_real(value)
    })
}

fn uncomment(line: &str) -> &str {
    line.split(['#', ';']).next().unwrap_or("")
}

fn secret_value_is_real(value: &str) -> bool {
    let value = value.trim().trim_matches('"').trim_matches('\'');
    value.len() >= 6 && !value.contains("${") && !value.eq_ignore_ascii_case("secret")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_oauth_helper_and_client_secret() {
        assert!(git_config_uses_oauth_helper(
            "[credential]\nhelper = cache --timeout 21600\nhelper = oauth -device\n"
        ));
        assert!(git_config_contains_oauth_client_secret(
            "[credential \"https://gitlab.example.com\"]\noauthClientSecret = abcdefgh\n"
        ));
    }
}

pub(crate) fn findings(home: &std::path::Path) -> Vec<crate::Finding> {
    super::radioisotope::findings("git-credential-oauth", install_insecurity_reasons, home)
}
