#![allow(dead_code)]

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

pub fn install_is_insecure() -> Result<bool, String> {
    install_insecurity_reasons().map(|reasons| !reasons.is_empty())
}

pub fn install_insecurity_reasons() -> Result<Vec<String>, String> {
    let mut reasons = Vec::new();
    for path in candidate_state_paths()? {
        if !path.exists() {
            continue;
        }
        let contents = match read_to_string(&path) {
            Ok(contents) => contents,
            Err(err) if err.contains("Permission denied") => continue,
            Err(err) => return Err(err),
        };
        if tailscale_state_contains_plaintext_identity(&contents)? {
            reasons.push(format!(
                "Tailscale state file contains plaintext node identity state: {}",
                path.display()
            ));
        }
    }
    Ok(reasons)
}

fn candidate_state_paths() -> Result<Vec<PathBuf>, String> {
    let home = home_dir()?;
    let mut paths = vec![
        PathBuf::from("/Library/Tailscale/tailscaled.state"),
        home.join(".local/share/tailscale/tailscaled.state"),
        PathBuf::from("/opt/homebrew/var/lib/tailscale/tailscaled.state"),
        PathBuf::from("/usr/local/var/lib/tailscale/tailscaled.state"),
    ];
    if let Some(xdg_data_home) = std::env::var_os("XDG_DATA_HOME").filter(|value| !value.is_empty())
    {
        paths.push(PathBuf::from(xdg_data_home).join("tailscale/tailscaled.state"));
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

fn tailscale_state_contains_plaintext_identity(contents: &str) -> Result<bool, String> {
    let value: serde_json::Value = serde_json::from_str(contents)
        .map_err(|err| format!("failed to parse Tailscale state JSON: {err}"))?;
    let Some(object) = value.as_object() else {
        return Ok(false);
    };
    if object.is_empty() || is_encrypted_state_object(object.keys().map(String::as_str)) {
        return Ok(false);
    }
    if object
        .get("_machinekey")
        .and_then(serde_json::Value::as_str)
        .is_some_and(|value| !value.trim().is_empty())
    {
        return Ok(true);
    }
    Ok(object
        .keys()
        .any(|key| key == "_daemon" || key == "Config" || key.starts_with("profile-")))
}

fn is_encrypted_state_object<'a>(keys: impl Iterator<Item = &'a str>) -> bool {
    let keys = keys.collect::<BTreeSet<_>>();
    keys == BTreeSet::from(["data", "key", "nonce"])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_plaintext_tailscale_state() {
        assert!(
            tailscale_state_contains_plaintext_identity(r#"{"_machinekey":"mkey:secret"}"#)
                .unwrap()
        );
        assert!(
            tailscale_state_contains_plaintext_identity(
                r#"{"profile-abc123":"eyJDb25maWciOnt9fQ=="}"#
            )
            .unwrap()
        );
        assert!(!tailscale_state_contains_plaintext_identity("{}").unwrap());
        assert!(
            !tailscale_state_contains_plaintext_identity(
                r#"{"key":"sealed-key","nonce":"sealed-nonce","data":"sealed-data"}"#
            )
            .unwrap()
        );
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
    super::radioisotope::findings("tailscale", install_insecurity_reasons, home)
}
