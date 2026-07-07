#![allow(dead_code)]

use std::path::{Path, PathBuf};

const MAX_KEY_BYTES: u64 = 1024 * 1024;
const MAX_SCAN_DEPTH: usize = 5;

pub fn install_is_insecure() -> Result<bool, String> {
    install_insecurity_reasons().map(|reasons| !reasons.is_empty())
}

pub fn install_insecurity_reasons() -> Result<Vec<String>, String> {
    let root = home_dir()?.join(".docker/machine");
    let mut reasons = Vec::new();
    if root.is_dir() {
        scan_dir(&root, 0, &mut reasons)?;
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
        } else if file_type.is_file() && file_contains_unencrypted_private_key(&entry.path())? {
            reasons.push(format!(
                "Docker Machine private key is stored without passphrase encryption: {}",
                entry.path().display()
            ));
        }
    }
    Ok(())
}

fn file_contains_unencrypted_private_key(path: &Path) -> Result<bool, String> {
    let metadata = std::fs::metadata(path)
        .map_err(|err| format!("failed to stat {}: {err}", path.display()))?;
    if metadata.len() > MAX_KEY_BYTES {
        return Ok(false);
    }
    let contents = std::fs::read_to_string(path)
        .map_err(|err| format!("failed to read {}: {err}", path.display()))?;
    Ok(private_key_contents_are_unencrypted(&contents))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_unencrypted_machine_keys() {
        assert!(private_key_contents_are_unencrypted(
            "-----BEGIN RSA PRIVATE KEY-----\nkey\n-----END RSA PRIVATE KEY-----"
        ));
        assert!(!private_key_contents_are_unencrypted(
            "-----BEGIN ENCRYPTED PRIVATE KEY-----\nkey\n-----END ENCRYPTED PRIVATE KEY-----"
        ));
    }
}

pub(crate) fn findings(home: &std::path::Path) -> Vec<crate::Finding> {
    super::radioisotope::findings("docker-machine", install_insecurity_reasons, home)
}
