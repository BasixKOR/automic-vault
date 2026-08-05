#![allow(dead_code)]

use std::path::{Path, PathBuf};

pub fn install_is_insecure() -> Result<bool, String> {
    install_insecurity_reasons().map(|reasons| !reasons.is_empty())
}

pub fn install_insecurity_reasons() -> Result<Vec<String>, String> {
    let path = auth_path()?;
    if path.exists() && auth_file_has_credentials(&read_to_string(&path)?)? {
        return Ok(vec![format!(
            "Codex CLI auth file contains plaintext credentials: {}",
            path.display()
        )]);
    }
    Ok(Vec::new())
}

fn codex_home() -> Result<PathBuf, String> {
    if let Some(dir) = std::env::var_os("CODEX_HOME").filter(|dir| !dir.is_empty()) {
        return Ok(PathBuf::from(dir));
    }
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .map(|home| home.join(".codex"))
        .ok_or_else(|| "HOME is not set".to_string())
}

fn auth_path() -> Result<PathBuf, String> {
    codex_home().map(|home| home.join("auth.json"))
}

fn read_to_string(path: &Path) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|err| format!("failed to read {}: {err}", path.display()))
}

/// Codex writes an API key, a ChatGPT token set, or both, depending on how the
/// user signed in. Any of them is a long-lived credential: the token set
/// carries a refresh token, so a stolen copy keeps working after the access
/// token expires.
fn auth_file_has_credentials(contents: &str) -> Result<bool, String> {
    let value: serde_json::Value = serde_json::from_str(contents)
        .map_err(|err| format!("failed to parse Codex auth JSON: {err}"))?;
    let Some(object) = value.as_object() else {
        return Ok(false);
    };
    let has_api_key = ["OPENAI_API_KEY", "personal_access_token", "bedrock_api_key"]
        .iter()
        .any(|field| is_nonempty_string(object.get(*field)));
    let has_tokens = object
        .get("tokens")
        .and_then(serde_json::Value::as_object)
        .is_some_and(|tokens| {
            ["access_token", "refresh_token", "id_token"]
                .iter()
                .any(|field| is_nonempty_string(tokens.get(*field)))
        });
    Ok(has_api_key || has_tokens)
}

fn is_nonempty_string(value: Option<&serde_json::Value>) -> bool {
    value
        .and_then(serde_json::Value::as_str)
        .is_some_and(|value| !value.trim().is_empty())
}

pub(crate) fn findings(home: &std::path::Path) -> Vec<crate::Finding> {
    super::radioisotope::findings("codex", install_insecurity_reasons, home)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_an_api_key() {
        let contents = r#"{"auth_mode":"apikey","OPENAI_API_KEY":"sk-proj-example"}"#;
        assert!(auth_file_has_credentials(contents).unwrap());
    }

    #[test]
    fn detects_a_chatgpt_token_set() {
        let contents = r#"{
            "auth_mode": "chatgpt",
            "OPENAI_API_KEY": null,
            "tokens": {
                "id_token": "eyJexample",
                "access_token": "example-access",
                "refresh_token": "example-refresh"
            }
        }"#;
        assert!(auth_file_has_credentials(contents).unwrap());
    }

    #[test]
    fn detects_a_refresh_token_on_its_own() {
        let contents = r#"{"tokens":{"refresh_token":"example-refresh"}}"#;
        assert!(auth_file_has_credentials(contents).unwrap());
    }

    #[test]
    fn ignores_an_auth_file_holding_no_credentials() {
        let contents = r#"{
            "auth_mode": "chatgpt",
            "OPENAI_API_KEY": null,
            "tokens": {"id_token": "", "access_token": "   "},
            "last_refresh": "2026-08-05T00:00:00Z"
        }"#;
        assert!(!auth_file_has_credentials(contents).unwrap());
    }

    #[test]
    fn ignores_an_empty_object() {
        assert!(!auth_file_has_credentials("{}").unwrap());
    }

    #[test]
    fn ignores_a_non_object_document() {
        assert!(!auth_file_has_credentials("[]").unwrap());
    }

    #[test]
    fn reports_unparseable_json_rather_than_staying_silent() {
        assert!(auth_file_has_credentials("not json").is_err());
    }

    #[test]
    fn top_level_install_is_insecure_returns_false_when_the_auth_file_is_missing() {
        let home = std::env::temp_dir().join(format!(
            "{}-detect-missing-{}",
            module_path!().replace(':', "_"),
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(&home).unwrap();

        let previous_home = std::env::var_os("HOME");
        let previous_codex_home = std::env::var_os("CODEX_HOME");
        unsafe {
            std::env::set_var("HOME", &home);
            std::env::remove_var("CODEX_HOME");
        }

        let result = install_is_insecure().unwrap();

        unsafe {
            match previous_home {
                Some(value) => std::env::set_var("HOME", value),
                None => std::env::remove_var("HOME"),
            }
            match previous_codex_home {
                Some(value) => std::env::set_var("CODEX_HOME", value),
                None => std::env::remove_var("CODEX_HOME"),
            }
        }

        assert!(!result);
        std::fs::remove_dir_all(home).unwrap();
    }

    #[test]
    fn codex_home_overrides_the_default_location() {
        let directory = std::env::temp_dir().join(format!(
            "{}-codex-home-{}",
            module_path!().replace(':', "_"),
            std::process::id()
        ));

        let previous_codex_home = std::env::var_os("CODEX_HOME");
        unsafe {
            std::env::set_var("CODEX_HOME", &directory);
        }

        let path = auth_path();

        unsafe {
            match previous_codex_home {
                Some(value) => std::env::set_var("CODEX_HOME", value),
                None => std::env::remove_var("CODEX_HOME"),
            }
        }

        assert_eq!(path.unwrap(), directory.join("auth.json"));
    }
}
