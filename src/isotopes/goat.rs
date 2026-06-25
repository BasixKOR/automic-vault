#![allow(dead_code)]

use std::path::{Path, PathBuf};

pub fn install_is_insecure() -> Result<bool, String> {
    install_insecurity_reasons().map(|reasons| !reasons.is_empty())
}

pub fn install_insecurity_reasons() -> Result<Vec<String>, String> {
    for path in goat_session_paths()? {
        if path.exists() && session_contains_secret(&read_to_string(&path)?) {
            return Ok(vec![format!(
                "goat auth session contains plaintext credentials: {}",
                path.display()
            )]);
        }
    }
    Ok(Vec::new())
}

fn goat_session_paths() -> Result<Vec<PathBuf>, String> {
    if let Some(state_home) = std::env::var_os("XDG_STATE_HOME").filter(|value| !value.is_empty()) {
        return Ok(vec![
            PathBuf::from(state_home).join("goat/auth-session.json"),
        ]);
    }

    Ok(vec![
        user_home()?.join(".local/state/goat/auth-session.json"),
    ])
}

fn user_home() -> Result<PathBuf, String> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())
}

fn read_to_string(path: &Path) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|err| format!("failed to read {}: {err}", path.display()))
}

fn session_contains_secret(contents: &str) -> bool {
    ["password", "access_token", "session_token"]
        .iter()
        .any(|field| json_string_field_is_present(contents, field))
}

fn json_string_field_is_present(contents: &str, field: &str) -> bool {
    let needle = format!("\"{field}\"");
    let Some(start) = contents.find(&needle) else {
        return false;
    };
    let after_field = &contents[start + needle.len()..];
    let Some((_, value)) = after_field.split_once(':') else {
        return false;
    };
    json_string_value(value).is_some_and(|value| !value.is_empty())
}

fn json_string_value(value: &str) -> Option<String> {
    let value = value.trim_start();
    if !value.starts_with('"') {
        return None;
    }
    let mut escaped = false;
    let mut output = String::new();
    for character in value[1..].chars() {
        if escaped {
            output.push(character);
            escaped = false;
            continue;
        }
        match character {
            '\\' => escaped = true,
            '"' => return Some(output),
            _ => output.push(character),
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_goat_session_secrets() {
        assert!(session_contains_secret(
            r#"{"password":"fake-app-password","access_token":"fake-access"}"#
        ));
        assert!(session_contains_secret(
            r#"{"session_token":"fake-refresh"}"#
        ));
    }

    #[test]
    fn ignores_missing_or_empty_session_secrets() {
        assert!(!session_contains_secret(r#"{"did":"did:plc:example"}"#));
        assert!(!session_contains_secret(r#"{"password":""}"#));
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
        let previous_state = std::env::var_os("XDG_STATE_HOME");
        unsafe {
            std::env::set_var("HOME", &home);
            std::env::remove_var("XDG_STATE_HOME");
        }

        let result = install_is_insecure().unwrap();

        unsafe {
            match previous_home {
                Some(value) => std::env::set_var("HOME", value),
                None => std::env::remove_var("HOME"),
            }
            match previous_state {
                Some(value) => std::env::set_var("XDG_STATE_HOME", value),
                None => std::env::remove_var("XDG_STATE_HOME"),
            }
        }

        assert!(!result);
        std::fs::remove_dir_all(home).unwrap();
    }
}

pub(crate) fn findings(home: &std::path::Path) -> Vec<crate::Finding> {
    super::radioisotope::findings("goat", install_insecurity_reasons, home)
}
