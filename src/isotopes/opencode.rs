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
        if opencode_auth_contains_plaintext_secret(&contents)? {
            reasons.push(format!(
                "opencode auth state contains plaintext credentials: {}",
                path.display()
            ));
        }
    }
    Ok(reasons)
}

fn candidate_auth_paths() -> Result<Vec<PathBuf>, String> {
    let home = home_dir()?;
    let mut bases = vec![
        home.join(".local/share/opencode"),
        home.join("Library/Application Support/opencode"),
    ];
    if let Some(xdg_data_home) = std::env::var_os("XDG_DATA_HOME").filter(|value| !value.is_empty())
    {
        bases.push(PathBuf::from(xdg_data_home).join("opencode"));
    }
    let mut paths = Vec::new();
    for base in bases {
        paths.push(base.join("auth.json"));
        paths.push(base.join("account.json"));
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

fn opencode_auth_contains_plaintext_secret(contents: &str) -> Result<bool, String> {
    let value: serde_json::Value = serde_json::from_str(contents)
        .map_err(|err| format!("failed to parse opencode auth JSON: {err}"))?;
    Ok(json_value_has_secret(&value))
}

fn json_value_has_secret(value: &serde_json::Value) -> bool {
    match value {
        serde_json::Value::Object(object) => object.iter().any(|(key, value)| {
            (secret_key_name(key) && json_leaf_has_real_string(value))
                || json_value_has_secret(value)
        }),
        serde_json::Value::Array(values) => values.iter().any(json_value_has_secret),
        _ => false,
    }
}

fn secret_key_name(key: &str) -> bool {
    matches!(key, "access" | "refresh" | "key" | "token" | "credential")
}

fn json_leaf_has_real_string(value: &serde_json::Value) -> bool {
    match value {
        serde_json::Value::String(value) => secret_value_is_real(value),
        serde_json::Value::Object(object) => object.values().any(json_leaf_has_real_string),
        serde_json::Value::Array(values) => values.iter().any(json_leaf_has_real_string),
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
    fn detects_legacy_and_current_auth_shapes() {
        assert!(
            opencode_auth_contains_plaintext_secret(
                r#"{"anthropic":{"type":"api","key":"sk-ant-secret"}}"#
            )
            .unwrap()
        );
        assert!(opencode_auth_contains_plaintext_secret(
            r#"{"version":2,"accounts":{"acc":{"credential":{"type":"oauth","access":"access-token","refresh":"refresh-token","expires":1}}}}"#
        )
        .unwrap());
        assert!(
            !opencode_auth_contains_plaintext_secret(r#"{"version":2,"accounts":{}}"#).unwrap()
        );
    }
}

pub(crate) fn findings(home: &std::path::Path) -> Vec<crate::Finding> {
    super::radioisotope::findings("opencode", install_insecurity_reasons, home)
}
