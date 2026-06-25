#![allow(dead_code)]

use std::path::{Path, PathBuf};

pub fn install_is_insecure() -> Result<bool, String> {
    install_insecurity_reasons().map(|reasons| !reasons.is_empty())
}

pub fn install_insecurity_reasons() -> Result<Vec<String>, String> {
    let path = default_profile_path()?;
    if path.exists() && profile_contains_secret(&read_to_string(&path)?) {
        return Ok(vec![format!(
            "soracom-cli default profile contains plaintext local credentials: {}",
            path.display()
        )]);
    }
    Ok(Vec::new())
}

fn default_profile_path() -> Result<PathBuf, String> {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())?;
    Ok(home.join(".soracom/default.json"))
}

fn read_to_string(path: &Path) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|err| format!("failed to read {}: {err}", path.display()))
}

fn profile_contains_secret(contents: &str) -> bool {
    const SECRET_KEYS: &[&str] = &[
        "authKey",
        "password",
        "apiKey",
        "apiToken",
        "token",
        "authToken",
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
    fn detects_soracom_auth_keys_and_passwords() {
        assert!(profile_contains_secret(
            r#"{"authKeyId":"keyId-example","authKey":"secret-example"}"#
        ));
        assert!(profile_contains_secret(
            r#"{"operatorId":"OP123","username":"sam","password":"fake-password"}"#
        ));
    }

    #[test]
    fn ignores_empty_or_missing_secret_values() {
        assert!(!profile_contains_secret(
            r#"{"authKeyId":"keyId-example","authKey":""}"#
        ));
        assert!(!profile_contains_secret(r#"{"coverageType":"g"}"#));
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
    super::radioisotope::findings("soracom-cli", install_insecurity_reasons, home)
}
