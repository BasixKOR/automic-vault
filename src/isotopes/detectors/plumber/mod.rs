#![allow(dead_code)]

use std::path::{Path, PathBuf};

pub fn install_is_insecure() -> Result<bool, String> {
    install_insecurity_reasons().map(|reasons| !reasons.is_empty())
}

pub fn install_insecurity_reasons() -> Result<Vec<String>, String> {
    let path = plumber_config_path()?;
    if path.exists() && config_requires_custody(&read_to_string(&path)?) {
        return Ok(vec![format!(
            "Plumber local config is outside Automic Vault custody: {}",
            path.display()
        )]);
    }
    Ok(Vec::new())
}

fn plumber_config_path() -> Result<PathBuf, String> {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())?;
    Ok(home.join(".batchsh/plumber.json"))
}

fn read_to_string(path: &Path) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|err| format!("failed to read {}: {err}", path.display()))
}

fn config_requires_custody(contents: &str) -> bool {
    let Ok(serde_json::Value::Object(object)) = serde_json::from_str(contents) else {
        return true;
    };
    !(object.len() == 1
        && object
            .get("automic_vault")
            .and_then(serde_json::Value::as_str)
            == Some("plumber-config-v1"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_plumber_tokens_and_connection_secrets() {
        assert!(config_requires_custody(
            r#"{"token":"fake-streamdal-token","connections":{}}"#
        ));
        assert!(config_requires_custody(
            r#"{"connections":{"kafka":{"sasl_password":"fake-password"}}}"#
        ));
        assert!(config_requires_custody(
            r#"{"relays":{"one":{"collection_token":"fake-relay-token"}}}"#
        ));
    }

    #[test]
    fn requires_custody_for_the_complete_config_and_ignores_only_the_marker() {
        assert!(config_requires_custody(
            r#"{"token":"","connections":{"kafka":{"address":"localhost:9092"}}}"#
        ));
        assert!(config_requires_custody(r#"{"connections":{}}"#));
        assert!(!config_requires_custody(
            r#"{"automic_vault":"plumber-config-v1"}"#
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
    super::radioisotope::findings("plumber", install_insecurity_reasons, home)
}
