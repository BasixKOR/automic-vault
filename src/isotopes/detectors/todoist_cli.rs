#![allow(dead_code)]

use std::path::{Path, PathBuf};

pub fn install_is_insecure() -> Result<bool, String> {
    install_insecurity_reasons().map(|reasons| !reasons.is_empty())
}

pub fn install_insecurity_reasons() -> Result<Vec<String>, String> {
    let mut reasons = Vec::new();
    let config_path = todoist_config_path()?;
    if config_path.exists()
        && json_contains_non_empty_string_key(&read_to_string(&config_path)?, "token")
    {
        reasons.push(format!(
            "Todoist config contains a plaintext API token: {}",
            config_path.display()
        ));
    }

    let cache_path = todoist_cache_path()?;
    if cache_path.exists()
        && json_contains_non_empty_string_key(&read_to_string(&cache_path)?, "token")
    {
        reasons.push(format!(
            "Todoist cache contains a plaintext API token: {}",
            cache_path.display()
        ));
    }
    Ok(reasons)
}

fn todoist_config_path() -> Result<PathBuf, String> {
    Ok(config_home()?.join("todoist/config.json"))
}

fn todoist_cache_path() -> Result<PathBuf, String> {
    Ok(cache_home()?.join("todoist/cache.json"))
}

fn config_home() -> Result<PathBuf, String> {
    if let Some(path) = std::env::var_os("XDG_CONFIG_HOME").filter(|value| !value.is_empty()) {
        return Ok(PathBuf::from(path));
    }
    Ok(user_home()?.join(".config"))
}

fn cache_home() -> Result<PathBuf, String> {
    if let Some(path) = std::env::var_os("XDG_CACHE_HOME").filter(|value| !value.is_empty()) {
        return Ok(PathBuf::from(path));
    }
    Ok(user_home()?.join(".cache"))
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
            r#"{"token":"fake-todoist-token","color":true}"#,
            "token"
        ));
        assert!(!json_contains_non_empty_string_key(
            r#"{"token":""}"#,
            "token"
        ));
        assert!(!json_contains_non_empty_string_key(
            r#"{"not_token":"value"}"#,
            "token"
        ));
    }

    #[test]
    fn top_level_install_is_insecure_returns_false_when_default_locations_are_missing() {
        let _lock = crate::global_test_env_lock().lock().unwrap();
        let home = std::env::temp_dir().join(format!(
            "{}-detect-missing-{}",
            module_path!().replace(':', "_"),
            std::process::id()
        ));
        let xdg = home.join("xdg");
        let cache = home.join("cache");
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(&xdg).unwrap();
        std::fs::create_dir_all(&cache).unwrap();

        let previous_home = std::env::var_os("HOME");
        let previous_config = std::env::var_os("XDG_CONFIG_HOME");
        let previous_cache = std::env::var_os("XDG_CACHE_HOME");
        unsafe {
            std::env::set_var("HOME", &home);
            std::env::set_var("XDG_CONFIG_HOME", &xdg);
            std::env::set_var("XDG_CACHE_HOME", &cache);
        }

        let result = install_is_insecure().unwrap();

        unsafe {
            match previous_home {
                Some(value) => std::env::set_var("HOME", value),
                None => std::env::remove_var("HOME"),
            }
            match previous_config {
                Some(value) => std::env::set_var("XDG_CONFIG_HOME", value),
                None => std::env::remove_var("XDG_CONFIG_HOME"),
            }
            match previous_cache {
                Some(value) => std::env::set_var("XDG_CACHE_HOME", value),
                None => std::env::remove_var("XDG_CACHE_HOME"),
            }
        }

        assert!(!result);
        std::fs::remove_dir_all(home).unwrap();
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
    fn todoist_paths_prefer_xdg_and_cache_env_and_require_home() {
        let _lock = crate::global_test_env_lock().lock().unwrap();
        let home = std::env::temp_dir().join(format!("todoist-detect-home-{}", std::process::id()));
        let xdg = home.join("xdg");
        let cache = home.join("cache");
        let previous_home = std::env::var_os("HOME");
        let previous_config = std::env::var_os("XDG_CONFIG_HOME");
        let previous_cache = std::env::var_os("XDG_CACHE_HOME");

        unsafe {
            std::env::set_var("HOME", &home);
            std::env::set_var("XDG_CONFIG_HOME", &xdg);
            std::env::set_var("XDG_CACHE_HOME", &cache);
        }
        assert_eq!(
            todoist_config_path().unwrap(),
            xdg.join("todoist/config.json")
        );
        assert_eq!(
            todoist_cache_path().unwrap(),
            cache.join("todoist/cache.json")
        );

        unsafe {
            std::env::remove_var("HOME");
            std::env::remove_var("XDG_CONFIG_HOME");
            std::env::remove_var("XDG_CACHE_HOME");
        }
        assert_eq!(todoist_config_path().unwrap_err(), "HOME is not set");
        assert_eq!(todoist_cache_path().unwrap_err(), "HOME is not set");

        unsafe {
            match previous_home {
                Some(value) => std::env::set_var("HOME", value),
                None => std::env::remove_var("HOME"),
            }
            match previous_config {
                Some(value) => std::env::set_var("XDG_CONFIG_HOME", value),
                None => std::env::remove_var("XDG_CONFIG_HOME"),
            }
            match previous_cache {
                Some(value) => std::env::set_var("XDG_CACHE_HOME", value),
                None => std::env::remove_var("XDG_CACHE_HOME"),
            }
        }
    }

    #[test]
    fn install_insecurity_reasons_reports_config_and_cache() {
        let _lock = crate::global_test_env_lock().lock().unwrap();
        let temp =
            std::env::temp_dir().join(format!("todoist-detect-report-{}", std::process::id()));
        let xdg = temp.join("xdg");
        let cache_home = temp.join("cache");
        let config_dir = xdg.join("todoist");
        let cache_dir = cache_home.join("todoist");
        let _ = fs::remove_dir_all(&temp);
        fs::create_dir_all(&config_dir).unwrap();
        fs::create_dir_all(&cache_dir).unwrap();
        let config = config_dir.join("config.json");
        let cache = cache_dir.join("cache.json");
        fs::write(&config, r#"{"token":"config-token"}"#).unwrap();
        fs::write(&cache, r#"{"token":"cache-token"}"#).unwrap();

        let previous_home = std::env::var_os("HOME");
        let previous_config = std::env::var_os("XDG_CONFIG_HOME");
        let previous_cache = std::env::var_os("XDG_CACHE_HOME");
        unsafe {
            std::env::set_var("HOME", &temp);
            std::env::set_var("XDG_CONFIG_HOME", &xdg);
            std::env::set_var("XDG_CACHE_HOME", &cache_home);
        }

        let reasons = install_insecurity_reasons().unwrap();

        unsafe {
            match previous_home {
                Some(value) => std::env::set_var("HOME", value),
                None => std::env::remove_var("HOME"),
            }
            match previous_config {
                Some(value) => std::env::set_var("XDG_CONFIG_HOME", value),
                None => std::env::remove_var("XDG_CONFIG_HOME"),
            }
            match previous_cache {
                Some(value) => std::env::set_var("XDG_CACHE_HOME", value),
                None => std::env::remove_var("XDG_CACHE_HOME"),
            }
        }

        assert_eq!(reasons.len(), 2);
        assert!(
            reasons
                .iter()
                .any(|reason| reason.contains(config.to_str().unwrap()))
        );
        assert!(
            reasons
                .iter()
                .any(|reason| reason.contains(cache.to_str().unwrap()))
        );
        fs::remove_dir_all(temp).unwrap();
    }
}

pub(crate) fn findings(home: &std::path::Path) -> Vec<crate::Finding> {
    super::radioisotope::findings("todoist-cli", install_insecurity_reasons, home)
}
