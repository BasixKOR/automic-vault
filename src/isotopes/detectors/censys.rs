#![allow(dead_code)]

use std::path::{Path, PathBuf};

const SECRET_KEYS: &[&str] = &["api_id", "api_secret", "asm_api_key"];

pub fn install_is_insecure() -> Result<bool, String> {
    install_insecurity_reasons().map(|reasons| !reasons.is_empty())
}

pub fn install_insecurity_reasons() -> Result<Vec<String>, String> {
    let path = censys_config_path()?;
    if path.exists() && config_contains_credentials(&read_to_string(&path)?) {
        return Ok(vec![format!(
            "Censys config contains plaintext API credentials: {}",
            path.display()
        )]);
    }
    Ok(Vec::new())
}

fn censys_config_path() -> Result<PathBuf, String> {
    let home = user_home()?;
    Ok(home.join(".config/censys/censys.cfg"))
}

fn user_home() -> Result<PathBuf, String> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())
}

fn read_to_string(path: &Path) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|err| format!("failed to read {}: {err}", path.display()))
}

fn config_contains_credentials(contents: &str) -> bool {
    contents.lines().any(line_has_secret)
}

fn line_has_secret(line: &str) -> bool {
    let line = line.trim();
    if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
        return false;
    }
    let Some((key, value)) = line.split_once('=') else {
        return false;
    };
    SECRET_KEYS.contains(&key.trim()) && !ini_value(value).is_empty()
}

fn ini_value(value: &str) -> &str {
    value.trim().trim_matches('"').trim_matches('\'').trim()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_search_and_asm_credentials() {
        assert!(config_contains_credentials(
            "[DEFAULT]\napi_id = fake-censys-id\napi_secret = fake-censys-secret\n"
        ));
        assert!(config_contains_credentials(
            "[DEFAULT]\nasm_api_key = fake-censys-asm-key\n"
        ));
    }

    #[test]
    fn ignores_empty_and_commented_credentials() {
        assert!(!config_contains_credentials(
            "[DEFAULT]\napi_id =\n; api_secret = fake\n# asm_api_key = fake\n"
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
    super::radioisotope::findings("censys", install_insecurity_reasons, home)
}
