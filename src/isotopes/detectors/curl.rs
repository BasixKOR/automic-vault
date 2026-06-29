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
            "curl netrc file contains plaintext credentials: {}",
            netrc.display()
        ));
    }

    let curlrc = home.join(".curlrc");
    if curlrc.exists() && curlrc_contains_auth_material(&read_to_string(&curlrc)?) {
        reasons.push(format!(
            "curl config contains plaintext auth material: {}",
            curlrc.display()
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

fn curlrc_contains_auth_material(contents: &str) -> bool {
    contents.lines().any(|line| {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            return false;
        }
        let (key, value) = parse_curlrc_option(trimmed);
        let key = key.trim().trim_start_matches('-').to_ascii_lowercase();
        let value = value.trim().trim_matches('"').trim_matches('\'').trim();
        if !secret_value_is_real(value) {
            return false;
        }
        matches!(
            key.as_str(),
            "user" | "u" | "proxy-user" | "oauth2-bearer" | "aws-sigv4"
        ) || (key == "header" && value.to_ascii_lowercase().contains("authorization:"))
    })
}

fn parse_curlrc_option(line: &str) -> (&str, &str) {
    for separator in ['=', ':'] {
        if let Some((key, value)) = line.split_once(separator) {
            return (key, value);
        }
    }
    line.split_once(char::is_whitespace).unwrap_or((line, ""))
}

fn secret_value_is_real(value: &str) -> bool {
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
    fn detects_netrc_passwords() {
        assert!(netrc_contains_password(
            "machine example.com login me password supersecret\n"
        ));
        assert!(!netrc_contains_password(
            "machine example.com login me password ${TOKEN}\n"
        ));
    }

    #[test]
    fn detects_curlrc_auth_options() {
        assert!(curlrc_contains_auth_material("user = \"me:supersecret\"\n"));
        assert!(curlrc_contains_auth_material(
            "header = \"Authorization: Bearer abcdefgh\"\n"
        ));
        assert!(!curlrc_contains_auth_material("# user = me:secret\n"));
    }

    #[test]
    fn top_level_detection_reports_user_files() {
        let _lock = crate::global_test_env_lock().lock().unwrap();
        let home = std::env::temp_dir().join(format!("curl-detect-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(&home).unwrap();
        std::fs::write(
            home.join(".netrc"),
            "machine x login me password abcdefgh\n",
        )
        .unwrap();
        let previous_home = std::env::var_os("HOME");
        unsafe { std::env::set_var("HOME", &home) };

        let reasons = install_insecurity_reasons().unwrap();

        unsafe {
            match previous_home {
                Some(value) => std::env::set_var("HOME", value),
                None => std::env::remove_var("HOME"),
            }
        }
        assert_eq!(reasons.len(), 1);
        std::fs::remove_dir_all(home).unwrap();
    }
}

pub(crate) fn findings(home: &std::path::Path) -> Vec<crate::Finding> {
    super::radioisotope::findings("curl", install_insecurity_reasons, home)
}
