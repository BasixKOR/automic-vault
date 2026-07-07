#![allow(dead_code)]

use std::path::{Path, PathBuf};

const MAX_KEY_BYTES: u64 = 1024 * 1024;
const MAX_SCAN_DEPTH: usize = 4;

pub fn install_is_insecure() -> Result<bool, String> {
    install_insecurity_reasons().map(|reasons| !reasons.is_empty())
}

pub fn install_insecurity_reasons() -> Result<Vec<String>, String> {
    let mut reasons = Vec::new();
    for root in candidate_roots()? {
        if root.is_file() {
            push_key_reason(&mut reasons, &root)?;
        } else if root.is_dir() {
            scan_directory(&root, 0, &mut reasons)?;
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

fn candidate_roots() -> Result<Vec<PathBuf>, String> {
    let home = home_dir()?;
    Ok(vec![
        home.join(".ssl"),
        home.join(".certs"),
        home.join("certs"),
        home.join(".config/openssl"),
    ])
}

fn scan_directory(path: &Path, depth: usize, reasons: &mut Vec<String>) -> Result<(), String> {
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
            scan_directory(&entry.path(), depth + 1, reasons)?;
        } else if file_type.is_file() {
            push_key_reason(reasons, &entry.path())?;
        }
    }
    Ok(())
}

fn push_key_reason(reasons: &mut Vec<String>, path: &Path) -> Result<(), String> {
    if file_contains_unencrypted_private_key(path)? {
        reasons.push(format!(
            "OpenSSL private key is stored without passphrase encryption: {}",
            path.display()
        ));
    }
    Ok(())
}

fn file_contains_unencrypted_private_key(path: &Path) -> Result<bool, String> {
    let metadata = std::fs::metadata(path)
        .map_err(|err| format!("failed to stat {}: {err}", path.display()))?;
    if !metadata.is_file() || metadata.len() > MAX_KEY_BYTES {
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
    fn detects_unencrypted_pem_keys() {
        assert!(private_key_contents_are_unencrypted(
            "-----BEGIN PRIVATE KEY-----\nkey\n-----END PRIVATE KEY-----"
        ));
        assert!(!private_key_contents_are_unencrypted(
            "-----BEGIN ENCRYPTED PRIVATE KEY-----\nkey\n-----END ENCRYPTED PRIVATE KEY-----"
        ));
        assert!(!private_key_contents_are_unencrypted(
            "-----BEGIN EC PRIVATE KEY-----\nProc-Type: 4,ENCRYPTED\nkey\n-----END EC PRIVATE KEY-----"
        ));
    }

    #[test]
    fn top_level_detection_scans_bounded_pki_locations() {
        let _lock = crate::global_test_env_lock().lock().unwrap();
        let home = std::env::temp_dir().join(format!("openssl-detect-{}", std::process::id()));
        let key = home.join(".ssl/private/key.pem");
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(key.parent().unwrap()).unwrap();
        std::fs::write(
            &key,
            "-----BEGIN PRIVATE KEY-----\nkey\n-----END PRIVATE KEY-----\n",
        )
        .unwrap();
        let previous_home = std::env::var_os("HOME");
        unsafe { std::env::set_var("HOME", &home) };

        let reasons = install_insecurity_reasons().unwrap();

        unsafe {
            match previous_home {
                Some(value) => std::env::set_var("HOME", value),
                None => std::env::remove_var("HOME"),
            }
        }
        assert_eq!(reasons.len(), 1);
        std::fs::remove_dir_all(home).unwrap();
    }
}

pub(crate) fn findings(home: &std::path::Path) -> Vec<crate::Finding> {
    super::radioisotope::findings("openssl@3", install_insecurity_reasons, home)
}
