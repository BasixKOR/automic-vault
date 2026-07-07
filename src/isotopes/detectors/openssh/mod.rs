#![allow(dead_code)]

use std::path::{Path, PathBuf};

const MAX_KEY_BYTES: u64 = 1024 * 1024;

pub fn install_is_insecure() -> Result<bool, String> {
    install_insecurity_reasons().map(|reasons| !reasons.is_empty())
}

pub fn install_insecurity_reasons() -> Result<Vec<String>, String> {
    let mut paths = ssh_candidate_paths()?;
    paths.sort();
    paths.dedup();

    let mut reasons = Vec::new();
    for path in paths {
        if path.exists() && file_contains_unencrypted_private_key(&path)? {
            reasons.push(format!(
                "SSH private key is stored without passphrase encryption: {}",
                path.display()
            ));
        }
    }
    Ok(reasons)
}

fn ssh_candidate_paths() -> Result<Vec<PathBuf>, String> {
    let home = home_dir()?;
    let ssh_dir = home.join(".ssh");
    let mut paths = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&ssh_dir) {
        for entry in entries {
            let entry =
                entry.map_err(|err| format!("failed to read {}: {err}", ssh_dir.display()))?;
            if entry
                .file_type()
                .map(|kind| kind.is_file())
                .unwrap_or(false)
            {
                paths.push(entry.path());
            }
        }
    }

    let config = ssh_dir.join("config");
    if config.exists() {
        let contents = read_to_string(&config)?;
        for path in identity_files_from_config(&contents)? {
            paths.push(path);
        }
    }
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

fn identity_files_from_config(contents: &str) -> Result<Vec<PathBuf>, String> {
    let mut paths = Vec::new();
    for line in contents.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let mut words = trimmed.split_whitespace();
        let Some(key) = words.next() else {
            continue;
        };
        if !key.eq_ignore_ascii_case("IdentityFile") {
            continue;
        }
        if let Some(path) = words.next() {
            paths.push(expand_home_path(path)?);
        }
    }
    Ok(paths)
}

fn expand_home_path(value: &str) -> Result<PathBuf, String> {
    let value = value.trim_matches('"').trim_matches('\'');
    if value == "~" {
        return home_dir();
    }
    if let Some(rest) = value.strip_prefix("~/") {
        return Ok(home_dir()?.join(rest));
    }
    Ok(PathBuf::from(value))
}

fn file_contains_unencrypted_private_key(path: &Path) -> Result<bool, String> {
    let metadata = std::fs::metadata(path)
        .map_err(|err| format!("failed to stat {}: {err}", path.display()))?;
    if !metadata.is_file() || metadata.len() > MAX_KEY_BYTES {
        return Ok(false);
    }
    let bytes =
        std::fs::read(path).map_err(|err| format!("failed to read {}: {err}", path.display()))?;
    if !bytes_contain_private_key_marker(&bytes) {
        return Ok(false);
    }
    let Ok(contents) = String::from_utf8(bytes) else {
        return Ok(false);
    };
    Ok(private_key_contents_are_unencrypted(&contents))
}

fn bytes_contain_private_key_marker(bytes: &[u8]) -> bool {
    [
        b"-----BEGIN ENCRYPTED PRIVATE KEY-----".as_slice(),
        b"-----BEGIN OPENSSH PRIVATE KEY-----".as_slice(),
        b"-----BEGIN PRIVATE KEY-----".as_slice(),
        b"-----BEGIN RSA PRIVATE KEY-----".as_slice(),
        b"-----BEGIN DSA PRIVATE KEY-----".as_slice(),
        b"-----BEGIN EC PRIVATE KEY-----".as_slice(),
    ]
    .iter()
    .any(|marker| bytes.windows(marker.len()).any(|window| window == *marker))
}

fn private_key_contents_are_unencrypted(contents: &str) -> bool {
    if contents.contains("-----BEGIN ENCRYPTED PRIVATE KEY-----") {
        return false;
    }
    if contents.contains("-----BEGIN OPENSSH PRIVATE KEY-----") {
        return openssh_private_key_is_unencrypted(contents);
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

fn openssh_private_key_is_unencrypted(contents: &str) -> bool {
    let Some(body) = pem_body(contents, "OPENSSH PRIVATE KEY") else {
        return false;
    };
    let Some(bytes) = decode_base64(&body) else {
        return false;
    };
    let magic = b"openssh-key-v1\0";
    if !bytes.starts_with(magic) {
        return false;
    }
    let mut offset = magic.len();
    let Some(cipher_name) = read_ssh_string(&bytes, &mut offset) else {
        return false;
    };
    cipher_name == b"none"
}

fn pem_body(contents: &str, label: &str) -> Option<String> {
    let begin = format!("-----BEGIN {label}-----");
    let end = format!("-----END {label}-----");
    let (_, after_begin) = contents.split_once(&begin)?;
    let (body, _) = after_begin.split_once(&end)?;
    Some(body.lines().map(str::trim).collect::<String>())
}

fn read_ssh_string<'a>(bytes: &'a [u8], offset: &mut usize) -> Option<&'a [u8]> {
    let len_bytes: [u8; 4] = bytes.get(*offset..*offset + 4)?.try_into().ok()?;
    *offset += 4;
    let len = u32::from_be_bytes(len_bytes) as usize;
    let value = bytes.get(*offset..*offset + len)?;
    *offset += len;
    Some(value)
}

fn decode_base64(value: &str) -> Option<Vec<u8>> {
    let mut output = Vec::new();
    let mut buffer = 0u32;
    let mut bits = 0u8;
    for byte in value.bytes().filter(|byte| !byte.is_ascii_whitespace()) {
        if byte == b'=' {
            break;
        }
        let value = match byte {
            b'A'..=b'Z' => byte - b'A',
            b'a'..=b'z' => byte - b'a' + 26,
            b'0'..=b'9' => byte - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            _ => return None,
        } as u32;
        buffer = (buffer << 6) | value;
        bits += 6;
        while bits >= 8 {
            bits -= 8;
            output.push(((buffer >> bits) & 0xff) as u8);
        }
    }
    Some(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_unencrypted_pem_keys() {
        assert!(private_key_contents_are_unencrypted(
            "-----BEGIN RSA PRIVATE KEY-----\nkey\n-----END RSA PRIVATE KEY-----"
        ));
        assert!(!private_key_contents_are_unencrypted(
            "-----BEGIN RSA PRIVATE KEY-----\nProc-Type: 4,ENCRYPTED\nkey\n-----END RSA PRIVATE KEY-----"
        ));
        assert!(!private_key_contents_are_unencrypted(
            "-----BEGIN ENCRYPTED PRIVATE KEY-----\nkey\n-----END ENCRYPTED PRIVATE KEY-----"
        ));
    }

    #[test]
    fn detects_identity_file_paths() {
        let _lock = crate::global_test_env_lock().lock().unwrap();
        let home = std::env::temp_dir().join(format!("ssh-detect-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(home.join(".ssh")).unwrap();
        std::fs::write(
            home.join(".ssh/id_test"),
            "-----BEGIN PRIVATE KEY-----\nkey\n",
        )
        .unwrap();
        std::fs::write(
            home.join(".ssh/config"),
            "Host x\n  IdentityFile ~/.ssh/id_test\n",
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

    #[test]
    fn ignores_binary_files_in_ssh_directory() {
        let _lock = crate::global_test_env_lock().lock().unwrap();
        let home = std::env::temp_dir().join(format!("ssh-binary-detect-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(home.join(".ssh")).unwrap();
        std::fs::write(home.join(".ssh/.DS_Store"), [0xff, 0xfe, 0xfd, 0x00]).unwrap();
        let previous_home = std::env::var_os("HOME");
        unsafe { std::env::set_var("HOME", &home) };

        let reasons = install_insecurity_reasons().unwrap();

        unsafe {
            match previous_home {
                Some(value) => std::env::set_var("HOME", value),
                None => std::env::remove_var("HOME"),
            }
        }
        assert!(reasons.is_empty());
        std::fs::remove_dir_all(home).unwrap();
    }
}

pub(crate) fn findings(home: &std::path::Path) -> Vec<crate::Finding> {
    super::radioisotope::findings("openssh", install_insecurity_reasons, home)
}
