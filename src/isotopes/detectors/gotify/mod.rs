#![allow(dead_code)]

use std::path::{Path, PathBuf};

pub fn install_is_insecure() -> Result<bool, String> {
    install_insecurity_reasons().map(|reasons| !reasons.is_empty())
}

pub fn install_insecurity_reasons() -> Result<Vec<String>, String> {
    let mut reasons = Vec::new();
    for path in gotify_user_config_paths()? {
        if path.exists() && json_contains_non_empty_string_key(&read_to_string(&path)?, "token") {
            reasons.push(format!(
                "Gotify config contains a plaintext application token: {}",
                path.display()
            ));
        }
    }
    Ok(reasons)
}

fn gotify_user_config_paths() -> Result<Vec<PathBuf>, String> {
    Ok(vec![
        config_home()?.join("gotify/cli.json"),
        user_home()?.join(".gotify/cli.json"),
    ])
}

fn config_home() -> Result<PathBuf, String> {
    if let Some(path) = std::env::var_os("XDG_CONFIG_HOME").filter(|value| !value.is_empty()) {
        return Ok(PathBuf::from(path));
    }
    Ok(user_home()?.join(".config"))
}

fn user_home() -> Result<PathBuf, String> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())
}

fn read_to_string(path: &Path) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|err| format!("failed to read {}: {err}", path.display()))
}

fn json_contains_non_empty_string_key(contents: &str, key: &str) -> bool {
    json_string_value(contents, key).is_some_and(|value| !value.is_empty())
}

fn json_string_value<'a>(contents: &'a str, key: &str) -> Option<&'a str> {
    let needle = format!("\"{key}\"");
    let mut remaining = contents;
    while let Some(index) = remaining.find(&needle) {
        let after_key = &remaining[index + needle.len()..];
        let Some(after_colon) = after_key
            .split_once(':')
            .map(|(_, value)| value.trim_start())
        else {
            return None;
        };
        if let Some(value) = after_colon.strip_prefix('"') {
            let end = value.find('"')?;
            return Some(&value[..end]);
        }
        if after_key.is_empty() {
            return None;
        }
        remaining = &after_key[1..];
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn detects_token_in_json() {
        assert!(json_contains_non_empty_string_key(
            r#"{"token":"fake-gotify-token","url":"https://push.example"}"#,
            "token"
        ));
        assert!(!json_contains_non_empty_string_key(
            r#"{"token":"","url":"https://push.example"}"#,
            "token"
        ));
        assert!(!json_contains_non_empty_string_key(
            r#"{"not_token":"fake-gotify-token"}"#,
            "token"
        ));
    }

    #[test]
    fn json_string_value_rejects_non_string_and_truncated_values() {
        assert_eq!(json_string_value(r#"{"token":1}"#, "token"), None);
        assert_eq!(
            json_string_value(r#"{"token":"unterminated}"#, "token"),
            None
        );
    }

    #[test]
    fn gotify_user_config_paths_prefer_xdg_and_require_home() {
        let _lock = crate::global_test_env_lock().lock().unwrap();
        let home = std::env::temp_dir().join(format!("gotify-detect-home-{}", std::process::id()));
        let xdg = home.join("xdg");
        let previous_home = std::env::var_os("HOME");
        let previous_config_home = std::env::var_os("XDG_CONFIG_HOME");

        unsafe {
            std::env::set_var("HOME", &home);
            std::env::set_var("XDG_CONFIG_HOME", &xdg);
        }
        let paths = gotify_user_config_paths().unwrap();
        assert_eq!(paths[0], xdg.join("gotify/cli.json"));
        assert_eq!(paths[1], home.join(".gotify/cli.json"));

        unsafe {
            std::env::remove_var("HOME");
            std::env::remove_var("XDG_CONFIG_HOME");
        }
        assert_eq!(gotify_user_config_paths().unwrap_err(), "HOME is not set");

        unsafe {
            match previous_home {
                Some(value) => std::env::set_var("HOME", value),
                None => std::env::remove_var("HOME"),
            }
            match previous_config_home {
                Some(value) => std::env::set_var("XDG_CONFIG_HOME", value),
                None => std::env::remove_var("XDG_CONFIG_HOME"),
            }
        }
    }

    #[test]
    fn install_insecurity_reasons_reports_both_paths() {
        let _lock = crate::global_test_env_lock().lock().unwrap();
        let temp =
            std::env::temp_dir().join(format!("gotify-detect-report-{}", std::process::id()));
        let xdg = temp.join("xdg");
        let xdg_dir = xdg.join("gotify");
        let legacy_dir = temp.join(".gotify");
        let _ = fs::remove_dir_all(&temp);
        fs::create_dir_all(&xdg_dir).unwrap();
        fs::create_dir_all(&legacy_dir).unwrap();
        let xdg_file = xdg_dir.join("cli.json");
        let legacy_file = legacy_dir.join("cli.json");
        fs::write(&xdg_file, r#"{"token":"xdg-token"}"#).unwrap();
        fs::write(&legacy_file, r#"{"token":"legacy-token"}"#).unwrap();

        let previous_home = std::env::var_os("HOME");
        let previous_config_home = std::env::var_os("XDG_CONFIG_HOME");
        unsafe {
            std::env::set_var("HOME", &temp);
            std::env::set_var("XDG_CONFIG_HOME", &xdg);
        }

        let reasons = install_insecurity_reasons().unwrap();

        unsafe {
            match previous_home {
                Some(value) => std::env::set_var("HOME", value),
                None => std::env::remove_var("HOME"),
            }
            match previous_config_home {
                Some(value) => std::env::set_var("XDG_CONFIG_HOME", value),
                None => std::env::remove_var("XDG_CONFIG_HOME"),
            }
        }

        assert_eq!(reasons.len(), 2);
        assert!(
            reasons
                .iter()
                .any(|reason| reason.contains(xdg_file.to_str().unwrap()))
        );
        assert!(
            reasons
                .iter()
                .any(|reason| reason.contains(legacy_file.to_str().unwrap()))
        );
        fs::remove_dir_all(temp).unwrap();
    }
}

pub(crate) fn findings(home: &std::path::Path) -> Vec<crate::Finding> {
    super::radioisotope::findings("gotify", install_insecurity_reasons, home)
}
