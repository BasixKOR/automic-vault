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
            "Wget2 netrc file contains plaintext credentials: {}",
            netrc.display()
        ));
    }

    for path in candidate_config_paths(&home) {
        if !path.exists() {
            continue;
        }
        if wgetrc_contains_password(&read_to_string(&path)?) {
            reasons.push(format!(
                "Wget2 config contains plaintext password options: {}",
                path.display()
            ));
        }
    }

    Ok(reasons)
}

fn candidate_config_paths(home: &Path) -> Vec<PathBuf> {
    let mut paths = vec![
        home.join(".wget2rc"),
        home.join(".config/wget/wget2rc"),
        home.join(".config/wget2/wget2rc"),
    ];
    if let Some(xdg_config_home) =
        std::env::var_os("XDG_CONFIG_HOME").filter(|value| !value.is_empty())
    {
        let xdg_config_home = PathBuf::from(xdg_config_home);
        paths.push(xdg_config_home.join("wget/wget2rc"));
        paths.push(xdg_config_home.join("wget2/wget2rc"));
    }
    paths.sort();
    paths.dedup();
    paths
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
    fn detects_netrc_and_wget2rc_passwords() {
        assert!(netrc_contains_password(
            "machine example.com login me password supersecret\n"
        ));
        assert!(wgetrc_contains_password("http-password = supersecret\n"));
        assert!(!wgetrc_contains_password("# http-password = supersecret\n"));
    }
}

pub(crate) fn findings(home: &std::path::Path) -> Vec<crate::Finding> {
    super::radioisotope::findings("wget2", install_insecurity_reasons, home)
}
