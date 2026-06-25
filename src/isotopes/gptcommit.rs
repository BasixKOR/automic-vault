#![allow(dead_code)]

use std::path::{Path, PathBuf};

pub fn install_is_insecure() -> Result<bool, String> {
    install_insecurity_reasons().map(|reasons| !reasons.is_empty())
}

pub fn install_insecurity_reasons() -> Result<Vec<String>, String> {
    let mut reasons = Vec::new();
    let path = gptcommit_config_path()?;
    if path.exists() && gptcommit_config_contains_api_key(&read_to_string(&path)?) {
        reasons.push(format!(
            "gptcommit global config contains a plaintext API key: {}",
            path.display()
        ));
    }

    if let Ok(current_dir) = std::env::current_dir() {
        let local = current_dir.join("gptcommit.toml");
        if local.exists() && gptcommit_config_contains_api_key(&read_to_string(&local)?) {
            reasons.push(format!(
                "gptcommit repository config contains a plaintext API key: {}",
                local.display()
            ));
        }
    }

    Ok(reasons)
}

fn gptcommit_config_path() -> Result<PathBuf, String> {
    Ok(user_home()?
        .join(".config")
        .join("gptcommit")
        .join("config.toml"))
}

fn user_home() -> Result<PathBuf, String> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())
}

fn read_to_string(path: &Path) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|err| format!("failed to read {}: {err}", path.display()))
}

fn gptcommit_config_contains_api_key(contents: &str) -> bool {
    toml_string_value_for_key(contents, "openai", "api_key")
        .or_else(|| toml_string_value_for_dotted_key(contents, "openai.api_key"))
        .is_some_and(|value| !value.is_empty())
}

fn toml_string_value_for_key<'a>(contents: &'a str, section: &str, key: &str) -> Option<&'a str> {
    let mut in_section = false;
    for line in contents.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if let Some(name) = table_name(trimmed) {
            in_section = name == section;
            continue;
        }
        if !in_section {
            continue;
        }
        let (line_key, value) = trimmed.split_once('=')?;
        if line_key.trim() == key {
            return toml_string_value(value);
        }
    }
    None
}

fn toml_string_value_for_dotted_key<'a>(contents: &'a str, key: &str) -> Option<&'a str> {
    contents.lines().find_map(|line| {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            return None;
        }
        let (line_key, value) = trimmed.split_once('=')?;
        (line_key.trim() == key)
            .then(|| toml_string_value(value))
            .flatten()
    })
}

fn table_name(line: &str) -> Option<&str> {
    line.strip_prefix('[')
        .and_then(|line| line.strip_suffix(']'))
        .map(str::trim)
}

fn toml_string_value(value: &str) -> Option<&str> {
    let value = value.trim();
    value
        .strip_prefix('"')
        .and_then(|value| value.split_once('"').map(|(value, _)| value))
        .or_else(|| {
            value
                .strip_prefix('\'')
                .and_then(|value| value.split_once('\'').map(|(value, _)| value))
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_openai_api_key_in_section_or_dotted_key() {
        assert!(gptcommit_config_contains_api_key(
            "model_provider = \"openai\"\n[openai]\napi_key = \"fake-openai-key\"\n"
        ));
        assert!(gptcommit_config_contains_api_key(
            "openai.api_key = 'fake-openai-key'\n"
        ));
    }

    #[test]
    fn ignores_empty_or_commented_api_key() {
        assert!(!gptcommit_config_contains_api_key(
            "[openai]\napi_key = \"\"\n"
        ));
        assert!(!gptcommit_config_contains_api_key(
            "# openai.api_key = \"fake-openai-key\"\n[other]\napi_key = \"fake\"\n"
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
    super::radioisotope::findings("gptcommit", install_insecurity_reasons, home)
}
