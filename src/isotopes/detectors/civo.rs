#![allow(dead_code)]

use std::path::{Path, PathBuf};

pub fn install_is_insecure() -> Result<bool, String> {
    install_insecurity_reasons().map(|reasons| !reasons.is_empty())
}

pub fn install_insecurity_reasons() -> Result<Vec<String>, String> {
    let path = civo_config_path()?;
    if path.exists() && config_contains_api_key(&read_to_string(&path)?) {
        return Ok(vec![format!(
            "civo config contains plaintext API keys: {}",
            path.display()
        )]);
    }
    Ok(Vec::new())
}

fn civo_config_path() -> Result<PathBuf, String> {
    if let Some(path) = std::env::var_os("CIVO_CONFIG").filter(|value| !value.is_empty()) {
        return Ok(PathBuf::from(path));
    }
    Ok(user_home()?.join(".civo.json"))
}

fn user_home() -> Result<PathBuf, String> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())
}

fn read_to_string(path: &Path) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|err| format!("failed to read {}: {err}", path.display()))
}

fn config_contains_api_key(contents: &str) -> bool {
    json_string_field_is_present(contents, "apikey")
        || json_object_field_is_present(contents, "apikeys")
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

fn json_object_field_is_present(contents: &str, field: &str) -> bool {
    let needle = format!("\"{field}\"");
    let Some(start) = contents.find(&needle) else {
        return false;
    };
    let after_field = &contents[start + needle.len()..];
    let Some((_, value)) = after_field.split_once(':') else {
        return false;
    };
    value.trim_start().starts_with('{') && value.contains("\":")
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
    fn detects_default_api_key() {
        assert!(config_contains_api_key(
            r#"{"apikey":"fake-civo-key","region":"NYC1"}"#
        ));
    }

    #[test]
    fn detects_named_api_keys() {
        assert!(config_contains_api_key(
            r#"{"apikeys":{"work":"fake-civo-key"},"current_apikey":"work"}"#
        ));
    }

    #[test]
    fn ignores_missing_or_empty_api_key() {
        assert!(!config_contains_api_key(r#"{"region":"NYC1"}"#));
        assert!(!config_contains_api_key(r#"{"apikey":""}"#));
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
        let previous_config = std::env::var_os("CIVO_CONFIG");
        unsafe {
            std::env::set_var("HOME", &home);
            std::env::remove_var("CIVO_CONFIG");
        }

        let result = install_is_insecure().unwrap();

        unsafe {
            match previous_home {
                Some(value) => std::env::set_var("HOME", value),
                None => std::env::remove_var("HOME"),
            }
            match previous_config {
                Some(value) => std::env::set_var("CIVO_CONFIG", value),
                None => std::env::remove_var("CIVO_CONFIG"),
            }
        }

        assert!(!result);
        std::fs::remove_dir_all(home).unwrap();
    }
}

pub(crate) fn findings(home: &std::path::Path) -> Vec<crate::Finding> {
    super::radioisotope::findings("civo", install_insecurity_reasons, home)
}
