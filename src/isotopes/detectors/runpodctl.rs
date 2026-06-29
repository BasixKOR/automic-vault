#![allow(dead_code)]

use std::path::{Path, PathBuf};

pub fn install_is_insecure() -> Result<bool, String> {
    install_insecurity_reasons().map(|reasons| !reasons.is_empty())
}

pub fn install_insecurity_reasons() -> Result<Vec<String>, String> {
    let mut reasons = Vec::new();
    for path in runpod_config_paths()? {
        if path.exists() && config_contains_api_key(&read_to_string(&path)?) {
            reasons.push(format!(
                "runpodctl config contains a plaintext API key: {}",
                path.display()
            ));
        }
    }
    Ok(reasons)
}

fn runpod_config_paths() -> Result<Vec<PathBuf>, String> {
    let home = user_home()?;
    Ok(vec![
        home.join(".runpod/config.toml"),
        home.join(".runpod.yaml"),
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

fn config_contains_api_key(contents: &str) -> bool {
    contents.lines().any(line_has_api_key)
}

fn line_has_api_key(line: &str) -> bool {
    let line = line.trim();
    if line.is_empty() || line.starts_with('#') {
        return false;
    }
    let Some((key, value)) = line.split_once(['=', ':']) else {
        return false;
    };
    matches!(key.trim(), "apiKey" | "api_key")
        && !quoted_config_value(value).unwrap_or_default().is_empty()
}

fn quoted_config_value(value: &str) -> Option<&str> {
    let value = value.trim();
    if let Some(value) = value.strip_prefix('"') {
        return value.split_once('"').map(|(value, _)| value);
    }
    if let Some(value) = value.strip_prefix('\'') {
        return value.split_once('\'').map(|(value, _)| value);
    }
    value.split_whitespace().next()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_toml_and_legacy_yaml_api_keys() {
        assert!(config_contains_api_key("apiKey = \"fake-runpod-key\"\n"));
        assert!(config_contains_api_key("apiKey: fake-runpod-key\n"));
    }

    #[test]
    fn ignores_empty_or_commented_api_keys() {
        assert!(!config_contains_api_key("apiKey = \"\"\n"));
        assert!(!config_contains_api_key("# apiKey = \"fake-runpod-key\"\n"));
    }

    #[test]
    fn top_level_install_is_insecure_returns_false_when_default_locations_are_missing() {
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
    super::radioisotope::findings("runpodctl", install_insecurity_reasons, home)
}
