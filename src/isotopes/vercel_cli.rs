#![allow(dead_code)]

use std::path::{Path, PathBuf};

pub fn install_is_insecure() -> Result<bool, String> {
    install_insecurity_reasons().map(|reasons| !reasons.is_empty())
}

pub fn install_insecurity_reasons() -> Result<Vec<String>, String> {
    let mut reasons = Vec::new();
    for path in candidate_auth_paths()? {
        if !path.exists() {
            continue;
        }
        let contents = read_to_string(&path)?;
        if auth_config_contains_credentials(&contents)? {
            reasons.push(format!(
                "Vercel CLI auth config contains plaintext credentials: {}",
                path.display()
            ));
        }
    }
    Ok(reasons)
}

fn candidate_auth_paths() -> Result<Vec<PathBuf>, String> {
    let home = home_dir()?;
    let mut paths = vec![
        home.join("Library/Application Support/com.vercel.cli/auth.json"),
        home.join(".local/share/com.vercel.cli/auth.json"),
        home.join(".now/auth.json"),
        home.join("Library/Application Support/now/auth.json"),
        home.join(".local/share/now/auth.json"),
        home.join(".vercel/auth.json"),
    ];
    if let Some(xdg_data_home) = std::env::var_os("XDG_DATA_HOME").filter(|value| !value.is_empty())
    {
        let xdg_data_home = PathBuf::from(xdg_data_home);
        paths.push(xdg_data_home.join("com.vercel.cli/auth.json"));
        paths.push(xdg_data_home.join("now/auth.json"));
    }
    paths.sort();
    paths.dedup();
    Ok(paths)
}

fn home_dir() -> Result<PathBuf, String> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())
}

fn read_to_string(path: &Path) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|err| format!("failed to read {}: {err}", path.display()))
}

fn auth_config_contains_credentials(contents: &str) -> Result<bool, String> {
    let value: serde_json::Value = serde_json::from_str(contents)
        .map_err(|err| format!("failed to parse Vercel auth JSON: {err}"))?;
    Ok(["token", "refreshToken"].iter().any(|key| {
        value
            .get(key)
            .and_then(serde_json::Value::as_str)
            .is_some_and(|value| !value.trim().is_empty())
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_access_and_refresh_tokens() {
        assert!(auth_config_contains_credentials(r#"{"token":"token-value"}"#).unwrap());
        assert!(auth_config_contains_credentials(r#"{"refreshToken":"refresh-value"}"#).unwrap());
        assert!(!auth_config_contains_credentials(r#"{"token":"","refreshToken":""}"#).unwrap());
    }

    #[test]
    fn top_level_install_is_insecure_returns_false_when_default_locations_are_missing() {
        let _lock = crate::global_test_env_lock().lock().unwrap();
        let home = std::env::temp_dir().join(format!(
            "{}-detect-missing-{}",
            module_path!().replace(':', "_"),
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(&home).unwrap();

        let previous_home = std::env::var_os("HOME");
        let previous_xdg = std::env::var_os("XDG_DATA_HOME");
        unsafe {
            std::env::set_var("HOME", &home);
            std::env::remove_var("XDG_DATA_HOME");
        }

        let result = install_is_insecure().unwrap();

        unsafe {
            match previous_home {
                Some(value) => std::env::set_var("HOME", value),
                None => std::env::remove_var("HOME"),
            }
            match previous_xdg {
                Some(value) => std::env::set_var("XDG_DATA_HOME", value),
                None => std::env::remove_var("XDG_DATA_HOME"),
            }
        }

        assert!(!result);
        std::fs::remove_dir_all(home).unwrap();
    }
}

pub(crate) fn findings(home: &std::path::Path) -> Vec<crate::Finding> {
    super::radioisotope::findings("vercel-cli", install_insecurity_reasons, home)
}
