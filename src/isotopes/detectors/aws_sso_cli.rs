#![allow(dead_code)]

use std::path::{Path, PathBuf};

pub fn install_is_insecure() -> Result<bool, String> {
    install_insecurity_reasons().map(|reasons| !reasons.is_empty())
}

pub fn install_insecurity_reasons() -> Result<Vec<String>, String> {
    let home = home_dir()?;
    let mut reasons = Vec::new();
    for dir in [home.join(".aws/sso/cache"), home.join(".aws/cli/cache")] {
        if dir.is_dir() {
            scan_cache_dir(&dir, &mut reasons)?;
        }
    }
    reasons.sort();
    reasons.dedup();
    Ok(reasons)
}

fn home_dir() -> Result<PathBuf, String> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())
}

fn scan_cache_dir(dir: &Path, reasons: &mut Vec<String>) -> Result<(), String> {
    let entries =
        std::fs::read_dir(dir).map_err(|err| format!("failed to read {}: {err}", dir.display()))?;
    for entry in entries {
        let entry = entry.map_err(|err| format!("failed to read {}: {err}", dir.display()))?;
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
            continue;
        }
        if aws_sso_cache_contains_secret(&read_to_string(&path)?) {
            reasons.push(format!(
                "AWS SSO cache contains plaintext token or role credentials: {}",
                path.display()
            ));
        }
    }
    Ok(())
}

fn read_to_string(path: &Path) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|err| format!("failed to read {}: {err}", path.display()))
}

fn aws_sso_cache_contains_secret(contents: &str) -> bool {
    [
        "\"accessToken\"",
        "\"clientSecret\"",
        "\"secretAccessKey\"",
        "\"sessionToken\"",
        "\"SecretAccessKey\"",
        "\"SessionToken\"",
    ]
    .iter()
    .any(|needle| json_key_has_string_value(contents, needle))
}

fn json_key_has_string_value(contents: &str, key: &str) -> bool {
    let mut rest = contents;
    while let Some(index) = rest.find(key) {
        let Some(value) = rest[index + key.len()..]
            .split_once(':')
            .map(|(_, value)| value)
        else {
            return false;
        };
        let value = value.trim_start();
        if value.starts_with('"') && value.get(1..).is_some_and(|value| !value.starts_with('"')) {
            return true;
        }
        rest = &value[1.min(value.len())..];
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_sso_and_sts_cache_secrets() {
        assert!(aws_sso_cache_contains_secret(
            r#"{"accessToken":"token","clientSecret":"secret"}"#
        ));
        assert!(aws_sso_cache_contains_secret(
            r#"{"Credentials":{"SecretAccessKey":"secret","SessionToken":"token"}}"#
        ));
        assert!(!aws_sso_cache_contains_secret(r#"{"accessToken":""}"#));
    }
}

pub(crate) fn findings(home: &std::path::Path) -> Vec<crate::Finding> {
    super::radioisotope::findings("aws-sso-cli", install_insecurity_reasons, home)
}
