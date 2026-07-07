#![allow(dead_code)]

use std::path::{Path, PathBuf};

pub fn install_is_insecure() -> Result<bool, String> {
    install_insecurity_reasons().map(|reasons| !reasons.is_empty())
}

pub fn install_insecurity_reasons() -> Result<Vec<String>, String> {
    let mut reasons = Vec::new();
    for dir in candidate_session_dirs()? {
        if !dir.exists() {
            continue;
        }
        for path in json_files_under(&dir)? {
            let contents = match std::fs::read_to_string(&path) {
                Ok(contents) => contents,
                Err(err) if err.kind() == std::io::ErrorKind::PermissionDenied => continue,
                Err(err) => return Err(format!("failed to read {}: {err}", path.display())),
            };
            if httpie_session_contains_secret(&contents) {
                reasons.push(format!(
                    "HTTPie session contains plaintext auth material: {}",
                    path.display()
                ));
            }
        }
    }
    Ok(reasons)
}

fn candidate_session_dirs() -> Result<Vec<PathBuf>, String> {
    let home = home_dir()?;
    let mut paths = vec![
        home.join(".config/httpie/sessions"),
        home.join(".httpie/sessions"),
    ];
    if let Some(xdg_config_home) =
        std::env::var_os("XDG_CONFIG_HOME").filter(|value| !value.is_empty())
    {
        paths.push(PathBuf::from(xdg_config_home).join("httpie/sessions"));
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

fn json_files_under(root: &Path) -> Result<Vec<PathBuf>, String> {
    let mut stack = vec![root.to_path_buf()];
    let mut files = Vec::new();
    while let Some(path) = stack.pop() {
        let entries = match std::fs::read_dir(&path) {
            Ok(entries) => entries,
            Err(err) if err.kind() == std::io::ErrorKind::PermissionDenied => continue,
            Err(err) => return Err(format!("failed to read {}: {err}", path.display())),
        };
        for entry in entries {
            let entry = entry.map_err(|err| format!("failed to read {}: {err}", path.display()))?;
            let path = entry.path();
            let metadata = match entry.metadata() {
                Ok(metadata) => metadata,
                Err(err) if err.kind() == std::io::ErrorKind::PermissionDenied => continue,
                Err(err) => return Err(format!("failed to stat {}: {err}", path.display())),
            };
            if metadata.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|ext| ext == "json") {
                files.push(path);
            }
        }
    }
    Ok(files)
}

fn httpie_session_contains_secret(contents: &str) -> bool {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(contents) else {
        return false;
    };
    json_value_has_secret(&value)
}

fn json_value_has_secret(value: &serde_json::Value) -> bool {
    match value {
        serde_json::Value::Object(object) => object.iter().any(|(key, value)| {
            (secret_key_name(key) && json_leaf_has_real_string(value))
                || json_value_has_secret(value)
        }),
        serde_json::Value::Array(values) => values.iter().any(json_value_has_secret),
        serde_json::Value::String(value) => {
            value.to_ascii_lowercase().starts_with("authorization:")
        }
        _ => false,
    }
}

fn secret_key_name(key: &str) -> bool {
    let key = key.to_ascii_lowercase();
    matches!(
        key.as_str(),
        "authorization" | "cookie" | "cookies" | "password" | "token" | "auth"
    )
}

fn json_leaf_has_real_string(value: &serde_json::Value) -> bool {
    match value {
        serde_json::Value::String(value) => secret_value_is_real(value),
        serde_json::Value::Array(values) => values.iter().any(json_leaf_has_real_string),
        serde_json::Value::Object(object) => object.values().any(json_leaf_has_real_string),
        _ => false,
    }
}

fn secret_value_is_real(value: &str) -> bool {
    let value = value.trim();
    if value.len() < 6 || value.contains("${") {
        return false;
    }
    let lower = value.to_ascii_lowercase();
    !matches!(
        lower.as_str(),
        "secret" | "password" | "token" | "example" | "redacted" | "changeme"
    ) && !lower.contains("example")
        && !lower.contains("placeholder")
        && !value.starts_with('<')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_session_auth_material() {
        assert!(httpie_session_contains_secret(
            r#"{"headers":{"Authorization":"Bearer abcdefgh"}}"#
        ));
        assert!(httpie_session_contains_secret(
            r#"{"auth":{"password":"supersecret"}}"#
        ));
        assert!(!httpie_session_contains_secret(
            r#"{"headers":{"Accept":"application/json"}}"#
        ));
    }

    #[test]
    fn top_level_detection_reports_session_files() {
        let _lock = crate::global_test_env_lock().lock().unwrap();
        let home = std::env::temp_dir().join(format!("httpie-detect-{}", std::process::id()));
        let session_dir = home.join(".config/httpie/sessions/example.org");
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(&session_dir).unwrap();
        std::fs::write(
            session_dir.join("default.json"),
            r#"{"headers":{"Authorization":"Bearer abcdefgh"}}"#,
        )
        .unwrap();
        let previous_home = std::env::var_os("HOME");
        let previous_xdg = std::env::var_os("XDG_CONFIG_HOME");
        unsafe {
            std::env::set_var("HOME", &home);
            std::env::remove_var("XDG_CONFIG_HOME");
        }

        let reasons = install_insecurity_reasons().unwrap();

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
        assert_eq!(reasons.len(), 1);
        std::fs::remove_dir_all(home).unwrap();
    }
}

pub(crate) fn findings(home: &std::path::Path) -> Vec<crate::Finding> {
    super::radioisotope::findings("httpie", install_insecurity_reasons, home)
}
