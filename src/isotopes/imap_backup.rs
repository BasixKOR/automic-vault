#![allow(dead_code)]

use std::path::{Path, PathBuf};

pub fn install_is_insecure() -> Result<bool, String> {
    install_insecurity_reasons().map(|reasons| !reasons.is_empty())
}

pub fn install_insecurity_reasons() -> Result<Vec<String>, String> {
    let path = imap_backup_config_path()?;
    if path.exists() && config_contains_password(&read_to_string(&path)?) {
        return Ok(vec![format!(
            "imap-backup config contains plaintext account passwords: {}",
            path.display()
        )]);
    }
    Ok(Vec::new())
}

fn imap_backup_config_path() -> Result<PathBuf, String> {
    Ok(user_home()?.join(".imap-backup/config.json"))
}

fn user_home() -> Result<PathBuf, String> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())
}

fn read_to_string(path: &Path) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|err| format!("failed to read {}: {err}", path.display()))
}

fn config_contains_password(contents: &str) -> bool {
    let bytes = contents.as_bytes();
    let mut index = 0;
    while let Some(offset) = contents[index..].find("\"password\"") {
        index += offset + "\"password\"".len();
        let Some(colon) = contents[index..].find(':') else {
            return false;
        };
        index += colon + 1;
        while index < bytes.len() && bytes[index].is_ascii_whitespace() {
            index += 1;
        }
        if index >= bytes.len() || bytes[index] != b'"' {
            continue;
        }
        if let Some(value) = parse_json_string(&contents[index..]) {
            if !value.is_empty() {
                return true;
            }
        }
        index += 1;
    }
    false
}

fn parse_json_string(input: &str) -> Option<String> {
    let mut chars = input.chars();
    if chars.next()? != '"' {
        return None;
    }

    let mut value = String::new();
    let mut escaped = false;
    for ch in chars {
        if escaped {
            value.push(ch);
            escaped = false;
            continue;
        }
        match ch {
            '\\' => escaped = true,
            '"' => return Some(value),
            _ => value.push(ch),
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_non_empty_password() {
        assert!(config_contains_password(
            r#"{"accounts":[{"username":"a@example.com","password":"fake-password"}]}"#
        ));
    }

    #[test]
    fn ignores_empty_or_missing_password() {
        assert!(!config_contains_password(
            r#"{"accounts":[{"username":"a@example.com","password":""}]}"#
        ));
        assert!(!config_contains_password(
            r#"{"accounts":[{"username":"a@example.com"}]}"#
        ));
    }

    #[test]
    fn top_level_install_is_insecure_returns_false_when_default_location_is_missing() {
        let home = std::env::temp_dir().join(format!(
            "{}-detect-missing-{}",
            module_path!().replace(':', "_"),
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(&home).unwrap();

        let previous_home = std::env::var_os("HOME");
        unsafe {
            std::env::set_var("HOME", &home);
        }

        let result = install_is_insecure().unwrap();

        unsafe {
            match previous_home {
                Some(value) => std::env::set_var("HOME", value),
                None => std::env::remove_var("HOME"),
            }
        }

        assert!(!result);
        std::fs::remove_dir_all(home).unwrap();
    }
}

pub(crate) fn findings(home: &std::path::Path) -> Vec<crate::Finding> {
    super::radioisotope::findings("imap-backup", install_insecurity_reasons, home)
}
