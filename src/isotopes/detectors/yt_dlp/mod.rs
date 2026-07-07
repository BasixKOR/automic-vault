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
            "yt-dlp netrc file contains plaintext credentials: {}",
            netrc.display()
        ));
    }

    for path in candidate_config_paths(&home) {
        if !path.exists() {
            continue;
        }
        if ytdlp_config_contains_password(&read_to_string(&path)?) {
            reasons.push(format!(
                "yt-dlp config contains plaintext password options: {}",
                path.display()
            ));
        }
    }

    Ok(reasons)
}

fn candidate_config_paths(home: &Path) -> Vec<PathBuf> {
    let mut paths = vec![
        home.join(".config/yt-dlp/config"),
        home.join(".config/yt-dlp.conf"),
        home.join(".yt-dlp.conf"),
    ];
    if let Some(xdg_config_home) =
        std::env::var_os("XDG_CONFIG_HOME").filter(|value| !value.is_empty())
    {
        let xdg_config_home = PathBuf::from(xdg_config_home);
        paths.push(xdg_config_home.join("yt-dlp/config"));
        paths.push(xdg_config_home.join("yt-dlp.conf"));
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

fn ytdlp_config_contains_password(contents: &str) -> bool {
    let tokens = contents
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                None
            } else {
                Some(line.split_once('#').map(|(line, _)| line).unwrap_or(line))
            }
        })
        .flat_map(str::split_whitespace)
        .collect::<Vec<_>>();

    tokens.windows(2).any(|pair| {
        password_option(pair[0])
            && secret_value_is_real(pair[1].trim_matches('"').trim_matches('\''))
    })
}

fn password_option(option: &str) -> bool {
    matches!(
        option,
        "-p" | "--password"
            | "--video-password"
            | "--ap-password"
            | "--client-certificate-password"
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
    fn detects_netrc_and_config_passwords() {
        assert!(netrc_contains_password(
            "machine example.com login me password supersecret\n"
        ));
        assert!(ytdlp_config_contains_password(
            "--username me --password supersecret\n"
        ));
        assert!(!ytdlp_config_contains_password("--username me\n"));
    }
}

pub(crate) fn findings(home: &std::path::Path) -> Vec<crate::Finding> {
    super::radioisotope::findings("yt-dlp", install_insecurity_reasons, home)
}
