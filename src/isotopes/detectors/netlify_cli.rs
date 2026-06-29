#![allow(dead_code)]

use std::path::{Path, PathBuf};

pub fn install_is_insecure() -> Result<bool, String> {
    install_insecurity_reasons().map(|reasons| !reasons.is_empty())
}

pub fn install_insecurity_reasons() -> Result<Vec<String>, String> {
    let mut reasons = Vec::new();

    for path in candidate_config_paths()? {
        if !path.exists() {
            continue;
        }

        let contents = read_to_string(&path)?;
        if config_has_sensitive_tokens(&contents)? {
            reasons.push(format!(
                "Netlify CLI config contains plaintext credentials: {}",
                path.display()
            ));
        }
    }

    Ok(reasons)
}

fn candidate_config_paths() -> Result<Vec<PathBuf>, String> {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())?;
    Ok(vec![
        home.join("Library/Preferences/netlify/config.json"),
        home.join(".netlify/config.json"),
    ])
}

fn read_to_string(path: &Path) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|err| format!("failed to read {}: {err}", path.display()))
}

fn config_has_sensitive_tokens(contents: &str) -> Result<bool, String> {
    let value: serde_json::Value = serde_json::from_str(contents)
        .map_err(|err| format!("failed to parse netlify config JSON: {err}"))?;
    Ok(user_objects(&value)
        .iter()
        .any(|user| user_has_sensitive_tokens(user)))
}

fn user_objects(value: &serde_json::Value) -> Vec<&serde_json::Map<String, serde_json::Value>> {
    value
        .get("users")
        .and_then(serde_json::Value::as_object)
        .into_iter()
        .flat_map(|users| users.values())
        .filter_map(serde_json::Value::as_object)
        .collect()
}

fn user_has_sensitive_tokens(user: &serde_json::Map<String, serde_json::Value>) -> bool {
    let Some(auth) = user.get("auth").and_then(serde_json::Value::as_object) else {
        return false;
    };

    auth.get("token")
        .and_then(serde_json::Value::as_str)
        .is_some_and(|value| !value.is_empty())
        || auth
            .get("github")
            .and_then(serde_json::Value::as_object)
            .and_then(|github| github.get("token"))
            .and_then(serde_json::Value::as_str)
            .is_some_and(|value| !value.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_user_and_github_tokens() {
        let contents = r#"{
          "userId": "user-1",
          "users": {
            "user-1": {
              "id": "user-1",
              "auth": {
                "token": "ntl_secret",
                "github": {
                  "token": "gho_secret"
                }
              }
            }
          }
        }"#;

        assert!(config_has_sensitive_tokens(contents).unwrap());
    }

    #[test]
    fn ignores_config_without_tokens() {
        let contents = r#"{
          "userId": "user-1",
          "users": {
            "user-1": {
              "id": "user-1",
              "auth": {
                "token": "",
                "github": {
                  "user": "example"
                }
              }
            }
          }
        }"#;

        assert!(!config_has_sensitive_tokens(contents).unwrap());
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
    super::radioisotope::findings("netlify-cli", install_insecurity_reasons, home)
}
