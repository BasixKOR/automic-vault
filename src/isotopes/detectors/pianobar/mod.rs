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
        if pianobar_config_contains_password(&contents) {
            reasons.push(format!(
                "pianobar config contains a plaintext password: {}",
                path.display()
            ));
        }
    }
    Ok(reasons)
}

fn candidate_config_paths() -> Result<Vec<PathBuf>, String> {
    let home = home_dir()?;
    let mut paths = vec![
        home.join(".config/pianobar/config"),
        home.join(".pianobar/config"),
    ];
    if let Some(xdg_config_home) =
        std::env::var_os("XDG_CONFIG_HOME").filter(|value| !value.is_empty())
    {
        paths.push(PathBuf::from(xdg_config_home).join("pianobar/config"));
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

fn pianobar_config_contains_password(contents: &str) -> bool {
    contents.lines().any(|line| {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            return false;
        }
        let Some((key, value)) = parse_assignment(trimmed) else {
            return false;
        };
        key == "password" && secret_value_is_real(value)
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
    fn detects_password_config() {
        assert!(pianobar_config_contains_password(
            "user = me\npassword = supersecret\n"
        ));
        assert!(!pianobar_config_contains_password(
            "password_command = security find-generic-password\n"
        ));
        assert!(!pianobar_config_contains_password(
            "# password = supersecret\n"
        ));
    }
}

pub(crate) fn findings(home: &std::path::Path) -> Vec<crate::Finding> {
    super::radioisotope::findings("pianobar", install_insecurity_reasons, home)
}
