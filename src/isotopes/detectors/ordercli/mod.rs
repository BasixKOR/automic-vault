#![allow(dead_code)]

use std::path::PathBuf;

pub fn install_is_insecure() -> Result<bool, String> {
    install_insecurity_reasons().map(|reasons| !reasons.is_empty())
}

pub fn install_insecurity_reasons() -> Result<Vec<String>, String> {
    for path in candidate_config_paths()? {
        if path.exists() && config_has_secrets(&read_to_string(&path)?) {
            return Ok(vec![format!(
                "ordercli session state is stored in plaintext config: {}",
                path.display()
            )]);
        }
    }
    Ok(Vec::new())
}

fn candidate_config_paths() -> Result<Vec<PathBuf>, String> {
    let dir = user_config_dir()?;
    Ok(vec![
        dir.join("ordercli/config.json"),
        dir.join("foodcli/config.json"),
        dir.join("foodoracli/config.json"),
    ])
}

fn user_config_dir() -> Result<PathBuf, String> {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())?;
    Ok(home.join("Library/Application Support"))
}

fn read_to_string(path: &std::path::Path) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|err| format!("failed to read {}: {err}", path.display()))
}

fn config_has_secrets(contents: &str) -> bool {
    const KEYS: [&str; 4] = [
        "access_token",
        "refresh_token",
        "client_secret",
        "pending_mfa_token",
    ];
    let Ok(root) = serde_json::from_str::<serde_json::Value>(contents) else {
        return KEYS
            .iter()
            .chain(["cookies_by_host"].iter())
            .any(|key| contents.contains(&format!(r#""{key}""#)));
    };
    let foodora = root
        .get("providers")
        .and_then(|providers| providers.get("foodora"))
        .unwrap_or(&root);
    KEYS.iter().any(|key| {
        foodora
            .get(*key)
            .and_then(serde_json::Value::as_str)
            .is_some_and(|value| !value.is_empty() && value != "@av")
    }) || foodora
        .get("cookies_by_host")
        .and_then(serde_json::Value::as_object)
        .is_some_and(|cookies| {
            !cookies.is_empty()
                && !(cookies.len() == 1
                    && cookies.get("@av").and_then(serde_json::Value::as_str) == Some("@av"))
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_provider_tokens() {
        assert!(config_has_secrets(r#"{"access_token":"a"}"#));
        assert!(config_has_secrets(r#"{"refresh_token":"r"}"#));
        assert!(config_has_secrets(r#"{"client_secret":"s"}"#));
        assert!(config_has_secrets(r#"{"pending_mfa_token":"m"}"#));
        assert!(config_has_secrets(
            r#"{"cookies_by_host":{"example":"a=b"}}"#
        ));
    }

    #[test]
    fn ignores_non_secret_config() {
        assert!(!config_has_secrets(
            r#"{"version":1,"providers":{"foodora":{"base_url":"https://example.com"}}}"#
        ));
        assert!(!config_has_secrets(
            r#"{"providers":{"foodora":{"access_token":"@av","refresh_token":"@av","client_secret":"@av","pending_mfa_token":"@av","cookies_by_host":{"@av":"@av"}}}}"#
        ));
    }

    #[test]
    fn top_level_install_is_insecure_returns_false_when_default_locations_are_missing() {
        let home = std::env::temp_dir().join(format!(
            "{}-detect-missing-{}",
            module_path!().replace(':', "_"),
            std::process::id()
        ));
        let xdg = home.join("xdg");
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(&xdg).unwrap();

        let previous_home = std::env::var_os("HOME");
        let previous_xdg = std::env::var_os("XDG_CONFIG_HOME");
        unsafe {
            std::env::set_var("HOME", &home);
            std::env::set_var("XDG_CONFIG_HOME", &xdg);
        }

        let result = install_is_insecure().unwrap();

        unsafe {
            match previous_home {
                Some(value) => std::env::set_var("HOME", value),
                None => std::env::remove_var("HOME"),
            }
            match previous_xdg {
                Some(value) => std::env::set_var("XDG_CONFIG_HOME", value),
                None => std::env::remove_var("XDG_CONFIG_HOME"),
            }
        }

        assert!(!result);
        std::fs::remove_dir_all(home).unwrap();
    }
}

pub(crate) fn findings(home: &std::path::Path) -> Vec<crate::Finding> {
    super::radioisotope::findings("ordercli", install_insecurity_reasons, home)
}
