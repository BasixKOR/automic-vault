#![allow(dead_code)]

use std::path::{Path, PathBuf};

pub fn install_is_insecure() -> Result<bool, String> {
    install_insecurity_reasons().map(|reasons| !reasons.is_empty())
}

pub fn install_insecurity_reasons() -> Result<Vec<String>, String> {
    let mut reasons = Vec::new();
    for file in candidate_cache_files()? {
        if !file.path.exists() {
            continue;
        }
        let contents = read_to_string(&file.path)?;
        if azure_cache_contains_plaintext_secret(&contents)? {
            reasons.push(format!("{}: {}", file.reason, file.path.display()));
        }
    }
    Ok(reasons)
}

fn candidate_cache_files() -> Result<Vec<CandidateFile>, String> {
    let config_dir = azure_config_dir()?;
    Ok(vec![
        CandidateFile {
            path: config_dir.join("msal_token_cache.json"),
            reason: "Azure CLI MSAL token cache contains plaintext credentials",
        },
        CandidateFile {
            path: config_dir.join("service_principal_entries.json"),
            reason: "Azure CLI service principal cache contains plaintext credentials",
        },
        CandidateFile {
            path: config_dir.join("accessTokens.json"),
            reason: "Azure CLI legacy token cache contains plaintext credentials",
        },
    ])
}

struct CandidateFile {
    path: PathBuf,
    reason: &'static str,
}

fn azure_config_dir() -> Result<PathBuf, String> {
    if let Some(path) = std::env::var_os("AZURE_CONFIG_DIR").filter(|value| !value.is_empty()) {
        return Ok(PathBuf::from(path));
    }
    Ok(home_dir()?.join(".azure"))
}

fn home_dir() -> Result<PathBuf, String> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())
}

fn read_to_string(path: &Path) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|err| format!("failed to read {}: {err}", path.display()))
}

fn azure_cache_contains_plaintext_secret(contents: &str) -> Result<bool, String> {
    let value: serde_json::Value = serde_json::from_str(contents)
        .map_err(|err| format!("failed to parse Azure CLI credential JSON: {err}"))?;
    Ok(json_value_has_secret(&value))
}

fn json_value_has_secret(value: &serde_json::Value) -> bool {
    match value {
        serde_json::Value::Object(object) => object.iter().any(|(key, value)| {
            (secret_key_name(key) && value.as_str().is_some_and(|value| !value.trim().is_empty()))
                || json_value_has_secret(value)
        }),
        serde_json::Value::Array(values) => values.iter().any(json_value_has_secret),
        _ => false,
    }
}

fn secret_key_name(key: &str) -> bool {
    matches!(
        key,
        "secret"
            | "access_token"
            | "refresh_token"
            | "accessToken"
            | "refreshToken"
            | "client_secret"
            | "clientSecret"
            | "password"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_msal_and_service_principal_secrets() {
        let msal = r#"{
          "AccessToken": {
            "entry": {
              "secret": "access-token"
            }
          },
          "RefreshToken": {
            "entry": {
              "secret": "refresh-token"
            }
          }
        }"#;
        let service_principal = r#"[{
          "client_id": "app-id",
          "client_secret": "client-secret"
        }]"#;

        assert!(azure_cache_contains_plaintext_secret(msal).unwrap());
        assert!(azure_cache_contains_plaintext_secret(service_principal).unwrap());
        assert!(!azure_cache_contains_plaintext_secret(r#"{"Account":{}}"#).unwrap());
    }

    #[test]
    fn uses_azure_config_dir_when_set() {
        let _lock = crate::global_test_env_lock().lock().unwrap();
        let config_dir = std::env::temp_dir().join(format!(
            "{}-azure-config-{}",
            module_path!().replace(':', "_"),
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&config_dir);
        std::fs::create_dir_all(&config_dir).unwrap();
        std::fs::write(
            config_dir.join("msal_token_cache.json"),
            r#"{"AccessToken":{"entry":{"secret":"access-token"}}}"#,
        )
        .unwrap();

        let previous = std::env::var_os("AZURE_CONFIG_DIR");
        unsafe { std::env::set_var("AZURE_CONFIG_DIR", &config_dir) };

        let reasons = install_insecurity_reasons().unwrap();

        unsafe {
            match previous {
                Some(value) => std::env::set_var("AZURE_CONFIG_DIR", value),
                None => std::env::remove_var("AZURE_CONFIG_DIR"),
            }
        }

        assert_eq!(reasons.len(), 1);
        std::fs::remove_dir_all(config_dir).unwrap();
    }
}

pub(crate) fn findings(home: &std::path::Path) -> Vec<crate::Finding> {
    super::radioisotope::findings("azure-cli", install_insecurity_reasons, home)
}
