#![allow(dead_code)]

use std::path::{Path, PathBuf};

pub fn install_is_insecure() -> Result<bool, String> {
    install_insecurity_reasons().map(|reasons| !reasons.is_empty())
}

pub fn install_insecurity_reasons() -> Result<Vec<String>, String> {
    let path = uaa_config_path()?;
    if path.exists() && config_contains_token(&read_to_string(&path)?) {
        return Ok(vec![format!(
            "UAA CLI config contains plaintext OAuth tokens: {}",
            path.display()
        )]);
    }
    Ok(Vec::new())
}

fn uaa_config_path() -> Result<PathBuf, String> {
    if let Some(path) = std::env::var_os("UAA_HOME") {
        return Ok(PathBuf::from(path).join("config.json"));
    }
    Ok(user_home()?.join(".uaa").join("config.json"))
}

fn user_home() -> Result<PathBuf, String> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())
}

fn read_to_string(path: &Path) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|err| format!("failed to read {}: {err}", path.display()))
}

fn config_contains_token(contents: &str) -> bool {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(contents) else {
        return contains_non_empty_json_string(contents, "access_token")
            || contains_non_empty_json_string(contents, "refresh_token");
    };
    value
        .get("Targets")
        .and_then(serde_json::Value::as_object)
        .into_iter()
        .flat_map(|targets| targets.values())
        .filter_map(|target| {
            target
                .get("Contexts")
                .and_then(serde_json::Value::as_object)
        })
        .flat_map(|contexts| contexts.values())
        .filter_map(|context| context.get("Token").and_then(serde_json::Value::as_object))
        .any(|token| {
            ["access_token", "refresh_token"].iter().any(|key| {
                token
                    .get(*key)
                    .and_then(serde_json::Value::as_str)
                    .is_some_and(|value| !value.is_empty() && value != "@av")
            })
        })
}

fn contains_non_empty_json_string(contents: &str, key: &str) -> bool {
    let needle = format!("\"{key}\"");
    contents.lines().any(|line| {
        let Some(start) = line.find(&needle) else {
            return false;
        };
        let Some(after_key) = line[start + needle.len()..].split_once(':') else {
            return false;
        };
        json_string_value(after_key.1).is_some_and(|value| !value.is_empty() && value != "@av")
    })
}

fn json_string_value(value: &str) -> Option<String> {
    let value = value.trim_start();
    let mut chars = value.strip_prefix('"')?.chars();
    let mut out = String::new();
    while let Some(ch) = chars.next() {
        match ch {
            '"' => return Some(out),
            '\\' => {
                if let Some(next) = chars.next() {
                    out.push(next);
                }
            }
            _ => out.push(ch),
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_access_and_refresh_tokens() {
        assert!(config_contains_token(
            r#"{"Targets":{"url:https://uaa.example":{"Contexts":{"ctx":{"Token":{"access_token":"fake-access"}}}}}}"#
        ));
        assert!(config_contains_token(
            r#"{"Targets":{"url:https://uaa.example":{"Contexts":{"ctx":{"Token":{"refresh_token":"fake-refresh"}}}}}}"#
        ));
    }

    #[test]
    fn ignores_empty_or_absent_tokens() {
        assert!(!config_contains_token(r#"{"Token":{"access_token":""}}"#));
        assert!(!config_contains_token(r#"{"Token":{"expires_in":3600}}"#));
        assert!(!config_contains_token(
            r#"{"Targets":{"url:https://uaa.example":{"Contexts":{"ctx":{"Token":{"access_token":"@av","refresh_token":"@av"}}}}}}"#
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

    #[test]
    fn uaa_home_selects_the_same_config_boundary_as_the_target() {
        let _guard = crate::global_test_env_lock().lock().unwrap();
        let root = std::env::temp_dir().join(format!("av-uaa-home-{}", std::process::id()));
        unsafe {
            std::env::set_var("UAA_HOME", &root);
        }
        assert_eq!(uaa_config_path().unwrap(), root.join("config.json"));
        unsafe {
            std::env::remove_var("UAA_HOME");
        }
    }
}

pub(crate) fn findings(home: &std::path::Path) -> Vec<crate::Finding> {
    super::radioisotope::findings("uaa-cli", install_insecurity_reasons, home)
}
