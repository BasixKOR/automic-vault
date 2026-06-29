#![allow(dead_code)]

use std::path::{Path, PathBuf};

const MAX_KEY_BYTES: u64 = 1024 * 1024;
const MAX_SCAN_DEPTH: usize = 8;

pub fn install_is_insecure() -> Result<bool, String> {
    install_insecurity_reasons().map(|reasons| !reasons.is_empty())
}

pub fn install_insecurity_reasons() -> Result<Vec<String>, String> {
    let mut reasons = Vec::new();
    for root in candidate_roots()? {
        if root.is_dir() {
            scan_dir(&root, 0, &mut reasons)?;
        }
    }
    reasons.sort();
    reasons.dedup();
    Ok(reasons)
}

fn candidate_roots() -> Result<Vec<PathBuf>, String> {
    let home = home_dir()?;
    Ok(vec![
        home.join(".config/letsencrypt"),
        home.join(".letsencrypt"),
        home.join("Library/Application Support/letsencrypt"),
    ])
}

fn home_dir() -> Result<PathBuf, String> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())
}

fn scan_dir(path: &Path, depth: usize, reasons: &mut Vec<String>) -> Result<(), String> {
    if depth > MAX_SCAN_DEPTH {
        return Ok(());
    }
    let entries = std::fs::read_dir(path)
        .map_err(|err| format!("failed to read {}: {err}", path.display()))?;
    for entry in entries {
        let entry =
            entry.map_err(|err| format!("failed to read entry in {}: {err}", path.display()))?;
        let file_type = entry
            .file_type()
            .map_err(|err| format!("failed to stat {}: {err}", entry.path().display()))?;
        if file_type.is_dir() {
            scan_dir(&entry.path(), depth + 1, reasons)?;
        } else if file_type.is_file() {
            push_key_reason(&entry.path(), reasons)?;
        }
    }
    Ok(())
}

fn push_key_reason(path: &Path, reasons: &mut Vec<String>) -> Result<(), String> {
    let metadata = std::fs::metadata(path)
        .map_err(|err| format!("failed to stat {}: {err}", path.display()))?;
    if metadata.len() > MAX_KEY_BYTES {
        return Ok(());
    }
    let contents = std::fs::read_to_string(path)
        .map_err(|err| format!("failed to read {}: {err}", path.display()))?;
    if private_key_contents_are_unencrypted(&contents)
        || certbot_jwk_contains_private_key(&contents)
    {
        reasons.push(format!(
            "Certbot key material is stored without passphrase encryption: {}",
            path.display()
        ));
    }
    Ok(())
}

fn private_key_contents_are_unencrypted(contents: &str) -> bool {
    if contents.contains("-----BEGIN ENCRYPTED PRIVATE KEY-----") {
        return false;
    }
    if contents.contains("-----BEGIN PRIVATE KEY-----") {
        return true;
    }
    for marker in [
        "-----BEGIN RSA PRIVATE KEY-----",
        "-----BEGIN DSA PRIVATE KEY-----",
        "-----BEGIN EC PRIVATE KEY-----",
    ] {
        if contents.contains(marker) {
            return !contents.contains("ENCRYPTED");
        }
    }
    false
}

fn certbot_jwk_contains_private_key(contents: &str) -> bool {
    contents.contains("\"kty\"")
        && contents.contains("\"d\"")
        && (contents.contains("\"p\"") || contents.contains("\"crv\""))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_certbot_private_key_formats() {
        assert!(private_key_contents_are_unencrypted(
            "-----BEGIN PRIVATE KEY-----\nkey\n-----END PRIVATE KEY-----"
        ));
        assert!(certbot_jwk_contains_private_key(
            r#"{"kty":"RSA","d":"secret","p":"prime"}"#
        ));
        assert!(!certbot_jwk_contains_private_key(
            r#"{"kty":"RSA","n":"pub"}"#
        ));
    }
}

pub(crate) fn findings(home: &std::path::Path) -> Vec<crate::Finding> {
    super::radioisotope::findings("certbot", install_insecurity_reasons, home)
}
