#![allow(dead_code)]

use std::path::{Path, PathBuf};

pub fn install_is_insecure() -> Result<bool, String> {
    install_insecurity_reasons().map(|reasons| !reasons.is_empty())
}

pub fn install_insecurity_reasons() -> Result<Vec<String>, String> {
    let mut reasons = Vec::new();
    for path in fauna_credential_paths()? {
        if path.exists() && credentials_file_contains_secret(&read_to_string(&path)?) {
            reasons.push(format!(
                "fauna-shell credential file contains plaintext local credentials: {}",
                path.display()
            ));
        }
    }
    Ok(reasons)
}

fn fauna_credential_paths() -> Result<Vec<PathBuf>, String> {
    let home = user_home()?;
    Ok(vec![
        home.join(".fauna/credentials/account_keys"),
        home.join(".fauna/credentials/secret_keys"),
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

fn credentials_file_contains_secret(contents: &str) -> bool {
    const SECRET_KEYS: &[&str] = &[
        "secret",
        "account_key",
        "accountKey",
        "access_token",
        "accessToken",
        "refresh_token",
        "refreshToken",
        "key",
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
    fn detects_fauna_account_keys_and_database_secrets() {
        assert!(credentials_file_contains_secret(
            r#"{"default":{"account_key":"fake-fauna-account-key"}}"#
        ));
        assert!(credentials_file_contains_secret(
            r#"{"account":{"db:admin":{"secret":"fake-fauna-secret","expiresAt":"never"}}}"#
        ));
    }

    #[test]
    fn ignores_empty_or_missing_secret_values() {
        assert!(!credentials_file_contains_secret(
            r#"{"default":{"account_key":""}}"#
        ));
        assert!(!credentials_file_contains_secret(r#"{"databases":{}}"#));
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
    super::radioisotope::findings("fauna-shell", install_insecurity_reasons, home)
}
