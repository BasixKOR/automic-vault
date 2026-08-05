#![allow(dead_code)]

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

pub fn install_is_insecure() -> Result<bool, String> {
    install_insecurity_reasons().map(|reasons| !reasons.is_empty())
}

pub fn install_insecurity_reasons() -> Result<Vec<String>, String> {
    Ok(insecurity_reasons_for(&auth_path()?))
}

/// Reports the auth file when it holds credentials, and also when it cannot be
/// read or parsed.
///
/// An unreadable credential file is not evidence of safety, and detector errors
/// are dropped by the scan, so returning `Err` here would silently hide a file
/// that may well be full of plaintext tokens.
fn insecurity_reasons_for(path: &Path) -> Vec<String> {
    if !path.exists() {
        return Vec::new();
    }
    match std::fs::read_to_string(path) {
        Ok(contents) => match auth_file_has_credentials(&contents) {
            Ok(true) => vec![format!(
                "Codex CLI auth file contains plaintext credentials: {}",
                path.display()
            )],
            Ok(false) => Vec::new(),
            Err(_) => vec![format!(
                "Codex CLI auth file could not be parsed and may contain plaintext credentials: {}",
                path.display()
            )],
        },
        Err(_) => vec![format!(
            "Codex CLI auth file could not be read and may contain plaintext credentials: {}",
            path.display()
        )],
    }
}

pub(crate) fn auth_path() -> Result<PathBuf, String> {
    auth_path_in(
        std::env::var_os("CODEX_HOME").as_deref(),
        std::env::var_os("HOME").as_deref(),
    )
}

pub(crate) fn config_path() -> Result<PathBuf, String> {
    auth_path().and_then(|path| {
        path.parent()
            .map(|parent| parent.join("config.toml"))
            .ok_or_else(|| "Codex auth path has no parent directory".to_string())
    })
}

fn auth_path_in(codex_home: Option<&OsStr>, home: Option<&OsStr>) -> Result<PathBuf, String> {
    if let Some(codex_home) = codex_home.filter(|value| !value.is_empty()) {
        return Ok(PathBuf::from(codex_home).join("auth.json"));
    }
    home.filter(|value| !value.is_empty())
        .map(|home| PathBuf::from(home).join(".codex/auth.json"))
        .ok_or_else(|| "HOME is not set".to_string())
}

/// Codex stores whichever credentials the chosen sign-in produced, and not all
/// of them are plain strings: `bedrock_api_key` is an object, and
/// `agent_identity` is either a bare JWT or a record holding a private key.
fn auth_file_has_credentials(contents: &str) -> Result<bool, String> {
    let value: serde_json::Value = serde_json::from_str(contents)
        .map_err(|err| format!("failed to parse Codex auth JSON: {err}"))?;
    let Some(object) = value.as_object() else {
        return Ok(false);
    };

    if ["OPENAI_API_KEY", "personal_access_token"]
        .iter()
        .any(|field| is_nonempty_string(object.get(*field)))
    {
        return Ok(true);
    }

    if object
        .get("tokens")
        .and_then(serde_json::Value::as_object)
        .is_some_and(|tokens| {
            ["access_token", "refresh_token", "id_token"]
                .iter()
                .any(|field| holds_secret_material(tokens.get(*field)))
        })
    {
        return Ok(true);
    }

    Ok(["bedrock_api_key", "agent_identity"]
        .iter()
        .any(|field| holds_secret_material(object.get(*field))))
}

/// True when the value is a non-empty string, or a structure containing one.
///
/// Codex has changed these shapes before, so this walks whatever is there
/// rather than assuming a particular layout.
fn holds_secret_material(value: Option<&serde_json::Value>) -> bool {
    match value {
        Some(serde_json::Value::String(text)) => !text.trim().is_empty(),
        Some(serde_json::Value::Object(fields)) => fields
            .values()
            .any(|value| holds_secret_material(Some(value))),
        Some(serde_json::Value::Array(values)) => values
            .iter()
            .any(|value| holds_secret_material(Some(value))),
        _ => false,
    }
}

fn is_nonempty_string(value: Option<&serde_json::Value>) -> bool {
    value
        .and_then(serde_json::Value::as_str)
        .is_some_and(|value| !value.trim().is_empty())
}

pub(crate) fn findings(home: &std::path::Path) -> Vec<crate::Finding> {
    super::radioisotope::findings("codex", install_insecurity_reasons, home)
        .into_iter()
        .map(|mut finding| {
            finding.solution = "Run `av harden codex`. Codex inside the ChatGPT desktop app shares this configuration and may require sign-in again; OpenAI does not document whether this CLI setting affects its existing session.".to_string();
            finding
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temporary_directory(label: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "{}-{label}-{}",
            module_path!().replace(':', "_"),
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).unwrap();
        path
    }

    #[test]
    fn detects_an_api_key() {
        let contents = r#"{"auth_mode":"apikey","OPENAI_API_KEY":"sk-proj-example"}"#;
        assert!(auth_file_has_credentials(contents).unwrap());
    }

    #[test]
    fn detects_a_personal_access_token() {
        let contents = r#"{"personal_access_token":"example-pat"}"#;
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
                "refresh_token": "example-refresh",
                "account_id": "acct_example"
            },
            "last_refresh": "2026-08-05T00:00:00Z"
        }"#;
        assert!(auth_file_has_credentials(contents).unwrap());
    }

    #[test]
    fn detects_a_refresh_token_on_its_own() {
        let contents = r#"{"tokens":{"refresh_token":"example-refresh"}}"#;
        assert!(auth_file_has_credentials(contents).unwrap());
    }

    #[test]
    fn detects_a_structured_bedrock_api_key() {
        let contents = r#"{"bedrock_api_key":{"api_key":"example-bedrock-key"}}"#;
        assert!(auth_file_has_credentials(contents).unwrap());
    }

    #[test]
    fn detects_an_agent_identity_jwt() {
        let contents = r#"{"agent_identity":"eyJexample.agent.jwt"}"#;
        assert!(auth_file_has_credentials(contents).unwrap());
    }

    #[test]
    fn detects_an_agent_identity_record() {
        let contents = r#"{
            "agent_identity": {
                "agent_runtime_id": "runtime-example",
                "agent_private_key": "example-private-key",
                "account_id": "acct_example",
                "chatgpt_user_id": "user-example",
                "email": null,
                "plan_type": "pro",
                "chatgpt_account_is_fedramp": false,
                "task_id": null
            }
        }"#;
        assert!(auth_file_has_credentials(contents).unwrap());
    }

    #[test]
    fn ignores_an_auth_file_holding_no_credentials() {
        let contents = r#"{
            "auth_mode": "chatgpt",
            "OPENAI_API_KEY": null,
            "tokens": {"id_token": "", "access_token": "   ", "refresh_token": ""},
            "bedrock_api_key": {"api_key": ""},
            "agent_identity": null,
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
    fn reports_a_credential_file_it_cannot_parse() {
        let directory = temporary_directory("unparseable");
        let path = directory.join("auth.json");
        std::fs::write(&path, "not json").unwrap();

        let reasons = insecurity_reasons_for(&path);

        assert_eq!(reasons.len(), 1);
        assert!(reasons[0].contains("could not be parsed"));
        assert!(reasons[0].ends_with(&path.display().to_string()));
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn reports_a_credential_file_it_cannot_read() {
        let directory = temporary_directory("unreadable");
        let path = directory.join("auth.json");
        std::fs::create_dir(&path).unwrap();

        let reasons = insecurity_reasons_for(&path);

        assert_eq!(reasons.len(), 1);
        assert!(reasons[0].contains("could not be read"));
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn stays_quiet_when_there_is_no_auth_file() {
        let directory = temporary_directory("missing");
        let reasons = insecurity_reasons_for(&directory.join("auth.json"));
        assert!(reasons.is_empty());
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn reports_a_credential_file_holding_secrets() {
        let directory = temporary_directory("credentials");
        let path = directory.join("auth.json");
        std::fs::write(&path, r#"{"OPENAI_API_KEY":"sk-proj-example"}"#).unwrap();

        let reasons = insecurity_reasons_for(&path);

        assert_eq!(reasons.len(), 1);
        assert!(reasons[0].contains("contains plaintext credentials"));
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn the_scan_reports_a_credential_file_it_cannot_parse() {
        let home = temporary_directory("scan-unparseable");
        std::fs::create_dir_all(home.join(".codex")).unwrap();
        std::fs::write(home.join(".codex/auth.json"), "not json").unwrap();

        let findings = findings(&home);

        assert_eq!(findings.len(), 1);
        assert!(findings[0].solution.contains("ChatGPT desktop app"));
        std::fs::remove_dir_all(home).unwrap();
    }

    #[test]
    fn the_scan_stays_quiet_without_an_auth_file() {
        let home = temporary_directory("scan-missing");
        assert!(findings(&home).is_empty());
        std::fs::remove_dir_all(home).unwrap();
    }

    #[test]
    fn codex_home_overrides_the_default_location() {
        let path = auth_path_in(
            Some(OsStr::new("/example/codex")),
            Some(OsStr::new("/example")),
        );
        assert_eq!(path.unwrap(), PathBuf::from("/example/codex/auth.json"));
    }

    #[test]
    fn an_empty_codex_home_falls_back_to_the_home_directory() {
        let path = auth_path_in(Some(OsStr::new("")), Some(OsStr::new("/example")));
        assert_eq!(path.unwrap(), PathBuf::from("/example/.codex/auth.json"));
    }

    #[test]
    fn resolution_fails_without_a_home_directory() {
        assert!(auth_path_in(None, None).is_err());
    }
}
