#![allow(dead_code)]

use std::path::{Path, PathBuf};

pub fn install_is_insecure() -> Result<bool, String> {
    install_insecurity_reasons().map(|reasons| !reasons.is_empty())
}

pub fn install_insecurity_reasons() -> Result<Vec<String>, String> {
    let path = plumber_config_path()?;
    if path.exists() && config_contains_secret(&read_to_string(&path)?) {
        return Ok(vec![format!(
            "Plumber config contains plaintext local credentials: {}",
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

fn config_contains_secret(contents: &str) -> bool {
    const SECRET_KEYS: &[&str] = &[
        "token",
        "auth_token",
        "api_token",
        "collection_token",
        "peer_token",
        "password",
        "sasl_password",
        "auth_secret",
        "secret",
        "credentials",
        "client_key",
    ];

    SECRET_KEYS
        .iter()
        .any(|key| json_string_key_has_nonempty_value(contents, key))
}

fn json_string_key_has_nonempty_value(contents: &str, key: &str) -> bool {
    let quoted_key = format!("\"{key}\"");
    let mut rest = contents;
    while let Some(index) = rest.find(&quoted_key) {
        let after_key = &rest[index + quoted_key.len()..];
        let Some(colon_index) = after_key.find(':') else {
            return false;
        };
        let value = after_key[colon_index + 1..].trim_start();
        if value.starts_with('"') {
            return !value.starts_with("\"\"");
        }
        rest = &after_key[colon_index + 1..];
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_plumber_tokens_and_connection_secrets() {
        assert!(config_contains_secret(
            r#"{"token":"fake-streamdal-token","connections":{}}"#
        ));
        assert!(config_contains_secret(
            r#"{"connections":{"kafka":{"sasl_password":"fake-password"}}}"#
        ));
        assert!(config_contains_secret(
            r#"{"relays":{"one":{"collection_token":"fake-relay-token"}}}"#
        ));
    }

    #[test]
    fn ignores_empty_non_secret_configs_and_the_marker() {
        assert!(!config_contains_secret(
            r#"{"token":"","connections":{"kafka":{"address":"localhost:9092"}}}"#
        ));
        assert!(!config_contains_secret(r#"{"connections":{}}"#));
        assert!(!config_contains_secret(
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
