#![allow(dead_code)]

use std::path::{Path, PathBuf};

pub fn install_is_insecure() -> Result<bool, String> {
    install_insecurity_reasons().map(|reasons| !reasons.is_empty())
}

pub fn install_insecurity_reasons() -> Result<Vec<String>, String> {
    let home = home_dir()?;
    let mut reasons = Vec::new();

    let netrc = home.join(".netrc");
    if netrc.exists() && netrc_contains_password(&read_to_string(&netrc)?) {
        reasons.push(format!(
            "Wget netrc file contains plaintext credentials: {}",
            netrc.display()
        ));
    }

    let wgetrc = home.join(".wgetrc");
    if wgetrc.exists() && wgetrc_contains_password(&read_to_string(&wgetrc)?) {
        reasons.push(format!(
            "Wget config contains plaintext password options: {}",
            wgetrc.display()
        ));
    }

    Ok(reasons)
}

fn home_dir() -> Result<PathBuf, String> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())
}

fn read_to_string(path: &Path) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|err| format!("failed to read {}: {err}", path.display()))
}

fn netrc_contains_password(contents: &str) -> bool {
    let tokens = contents
        .lines()
        .filter_map(|line| line.split_once('#').map(|(line, _)| line).or(Some(line)))
        .flat_map(str::split_whitespace)
        .collect::<Vec<_>>();
    tokens.windows(2).any(|pair| {
        pair[0] == "password" && secret_value_is_real(pair[1].trim_matches('"').trim_matches('\''))
    })
}

fn wgetrc_contains_password(contents: &str) -> bool {
    contents.lines().any(|line| {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            return false;
        }
        let Some((key, value)) = parse_assignment(trimmed) else {
            return false;
        };
        password_key_name(key) && secret_value_is_real(value)
    })
}

fn parse_assignment(line: &str) -> Option<(&str, &str)> {
    let (key, value) = line
        .split_once('=')
        .or_else(|| line.split_once(char::is_whitespace))?;
    Some((
        key.trim(),
        value.trim().trim_matches('"').trim_matches('\''),
    ))
}

fn password_key_name(key: &str) -> bool {
    matches!(
        key.trim_start_matches('-').replace('-', "_").as_str(),
        "password" | "http_password" | "https_password" | "ftp_password" | "proxy_password"
    )
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
    fn detects_netrc_and_wgetrc_passwords() {
        assert!(netrc_contains_password(
            "machine example.com login me password supersecret\n"
        ));
        assert!(wgetrc_contains_password("http_password = supersecret\n"));
        assert!(!wgetrc_contains_password("# http_password = supersecret\n"));
    }
}

pub(crate) fn findings(home: &std::path::Path) -> Vec<crate::Finding> {
    super::radioisotope::findings("wget", install_insecurity_reasons, home)
}
