#![allow(dead_code)]

use std::path::{Path, PathBuf};

pub fn install_is_insecure() -> Result<bool, String> {
    install_insecurity_reasons().map(|reasons| !reasons.is_empty())
}

pub fn install_insecurity_reasons() -> Result<Vec<String>, String> {
    let path = algolia_config_path()?;
    if path.exists() && config_contains_secret(&read_to_string(&path)?) {
        return Ok(vec![format!(
            "algolia config contains plaintext API keys: {}",
            path.display()
        )]);
    }
    Ok(Vec::new())
}

fn algolia_config_path() -> Result<PathBuf, String> {
    if let Some(config_home) = std::env::var_os("XDG_CONFIG_HOME").filter(|value| !value.is_empty())
    {
        return Ok(PathBuf::from(config_home).join("algolia/config.toml"));
    }
    Ok(user_home()?.join(".config/algolia/config.toml"))
}

fn user_home() -> Result<PathBuf, String> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())
}

fn read_to_string(path: &Path) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|err| format!("failed to read {}: {err}", path.display()))
}

fn config_contains_secret(contents: &str) -> bool {
    ["api_key", "admin_api_key", "crawler_api_key"]
        .iter()
        .any(|field| toml_string_field_is_present(contents, field))
}

fn toml_string_field_is_present(contents: &str, field: &str) -> bool {
    contents.lines().any(|line| {
        let line = line.split('#').next().unwrap_or("").trim();
        let Some((name, value)) = line.split_once('=') else {
            return false;
        };
        name.trim() == field && toml_string_value(value).is_some_and(|value| !value.is_empty())
    })
}

fn toml_string_value(value: &str) -> Option<String> {
    let value = value.trim();
    let quote = value.chars().next()?;
    if quote != '"' && quote != '\'' {
        return None;
    }
    let mut escaped = false;
    let mut output = String::new();
    for character in value[quote.len_utf8()..].chars() {
        if escaped {
            output.push(character);
            escaped = false;
            continue;
        }
        match character {
            '\\' if quote == '"' => escaped = true,
            character if character == quote => return Some(output),
            _ => output.push(character),
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_algolia_api_keys() {
        assert!(config_contains_secret(
            r#"[default]
application_id = "APPID"
api_key = "fake-algolia-key"
"#
        ));
        assert!(config_contains_secret(
            r#"[crawler]
crawler_api_key = 'fake-crawler-key'
"#
        ));
    }

    #[test]
    fn ignores_missing_or_empty_api_keys() {
        assert!(!config_contains_secret(
            r#"[default]
application_id = "APPID"
"#
        ));
        assert!(!config_contains_secret("api_key = \"\""));
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
        let previous_config = std::env::var_os("XDG_CONFIG_HOME");
        unsafe {
            std::env::set_var("HOME", &home);
            std::env::remove_var("XDG_CONFIG_HOME");
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
        }

        assert!(!result);
        std::fs::remove_dir_all(home).unwrap();
    }
}

pub(crate) fn findings(home: &std::path::Path) -> Vec<crate::Finding> {
    super::radioisotope::findings("algolia", install_insecurity_reasons, home)
}
