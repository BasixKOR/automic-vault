#![allow(dead_code)]

use std::path::{Path, PathBuf};

const MAX_KEY_BYTES: u64 = 1024 * 1024;
const SECURITY_KEY_REASON: &str = "SSH security-key handle is stored without passphrase encryption";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum UnencryptedPrivateKeyKind {
    Exportable,
    SecurityKeyHandle,
}

pub fn install_is_insecure() -> Result<bool, String> {
    install_insecurity_reasons().map(|reasons| !reasons.is_empty())
}

pub fn install_insecurity_reasons() -> Result<Vec<String>, String> {
    let mut paths = ssh_candidate_paths()?;
    paths.sort();
    paths.dedup();

    let mut reasons = Vec::new();
    for path in paths {
        if !path.exists() {
            continue;
        }
        if let Some(kind) = file_unencrypted_private_key_kind(&path)? {
            let reason = match kind {
                UnencryptedPrivateKeyKind::Exportable => {
                    "SSH private key is stored without passphrase encryption"
                }
                UnencryptedPrivateKeyKind::SecurityKeyHandle => SECURITY_KEY_REASON,
            };
            reasons.push(format!("{reason}: {}", path.display()));
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

fn file_unencrypted_private_key_kind(
    path: &Path,
) -> Result<Option<UnencryptedPrivateKeyKind>, String> {
    let metadata = std::fs::metadata(path)
        .map_err(|err| format!("failed to stat {}: {err}", path.display()))?;
    if !metadata.is_file() || metadata.len() > MAX_KEY_BYTES {
        return Ok(None);
    }
    let bytes =
        std::fs::read(path).map_err(|err| format!("failed to read {}: {err}", path.display()))?;
    if !bytes_contain_private_key_marker(&bytes) {
        return Ok(None);
    }
    let Ok(contents) = String::from_utf8(bytes) else {
        return Ok(None);
    };
    Ok(private_key_contents_unencrypted_kind(&contents))
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
    private_key_contents_unencrypted_kind(contents).is_some()
}

fn private_key_contents_unencrypted_kind(contents: &str) -> Option<UnencryptedPrivateKeyKind> {
    if contents.contains("-----BEGIN ENCRYPTED PRIVATE KEY-----") {
        return None;
    }
    if contents.contains("-----BEGIN OPENSSH PRIVATE KEY-----") {
        return openssh_private_key_unencrypted_kind(contents);
    }
    if contents.contains("-----BEGIN PRIVATE KEY-----") {
        return Some(UnencryptedPrivateKeyKind::Exportable);
    }
    for marker in [
        "-----BEGIN RSA PRIVATE KEY-----",
        "-----BEGIN DSA PRIVATE KEY-----",
        "-----BEGIN EC PRIVATE KEY-----",
    ] {
        if contents.contains(marker) {
            return (!contents.contains("ENCRYPTED"))
                .then_some(UnencryptedPrivateKeyKind::Exportable);
        }
    }
    None
}

fn openssh_private_key_unencrypted_kind(contents: &str) -> Option<UnencryptedPrivateKeyKind> {
    let Some(body) = pem_body(contents, "OPENSSH PRIVATE KEY") else {
        return None;
    };
    let Some(bytes) = decode_base64(&body) else {
        return None;
    };
    let magic = b"openssh-key-v1\0";
    if !bytes.starts_with(magic) {
        return None;
    }
    let mut offset = magic.len();
    let Some(cipher_name) = read_ssh_string(&bytes, &mut offset) else {
        return None;
    };
    if cipher_name != b"none" {
        return None;
    }

    let kind = read_ssh_string(&bytes, &mut offset)
        .and_then(|_| read_ssh_string(&bytes, &mut offset))
        .and_then(|_| read_ssh_u32(&bytes, &mut offset))
        .filter(|key_count| *key_count == 1)
        .and_then(|_| read_ssh_string(&bytes, &mut offset))
        .and_then(|public_key| {
            let mut public_key_offset = 0;
            read_ssh_string(public_key, &mut public_key_offset)
        })
        .and_then(|public_key_type| {
            let private_keys = read_ssh_string(&bytes, &mut offset)?;
            let mut private_key_offset = 0;
            read_ssh_u32(private_keys, &mut private_key_offset)?;
            read_ssh_u32(private_keys, &mut private_key_offset)?;
            let private_key_type = read_ssh_string(private_keys, &mut private_key_offset)?;
            (public_key_type == private_key_type).then_some(private_key_type)
        })
        .filter(|key_type| security_key_type(key_type))
        .map(|_| UnencryptedPrivateKeyKind::SecurityKeyHandle)
        .unwrap_or(UnencryptedPrivateKeyKind::Exportable);
    Some(kind)
}

fn security_key_type(key_type: &[u8]) -> bool {
    matches!(
        key_type,
        b"sk-ecdsa-sha2-nistp256@openssh.com"
            | b"sk-ssh-ed25519@openssh.com"
            | b"sk-ecdsa-sha2-nistp256-cert-v01@openssh.com"
            | b"sk-ssh-ed25519-cert-v01@openssh.com"
    )
}

fn pem_body(contents: &str, label: &str) -> Option<String> {
    let begin = format!("-----BEGIN {label}-----");
    let end = format!("-----END {label}-----");
    let (_, after_begin) = contents.split_once(&begin)?;
    let (body, _) = after_begin.split_once(&end)?;
    Some(body.lines().map(str::trim).collect::<String>())
}

fn read_ssh_string<'a>(bytes: &'a [u8], offset: &mut usize) -> Option<&'a [u8]> {
    let len = read_ssh_u32(bytes, offset)? as usize;
    let value = bytes.get(*offset..*offset + len)?;
    *offset += len;
    Some(value)
}

fn read_ssh_u32(bytes: &[u8], offset: &mut usize) -> Option<u32> {
    let value = u32::from_be_bytes(bytes.get(*offset..*offset + 4)?.try_into().ok()?);
    *offset += 4;
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

    #[test]
    fn classifies_unencrypted_security_key_handles() {
        for body in [
            "b3BlbnNzaC1rZXktdjEAAAAABG5vbmUAAAAEbm9uZQAAAAAAAAABAAAAHgAAABpzay1zc2gtZWQyNTUxOUBvcGVuc3NoLmNvbQAAACYAAAAqAAAAKgAAABpzay1zc2gtZWQyNTUxOUBvcGVuc3NoLmNvbQ==",
            "b3BlbnNzaC1rZXktdjEAAAAABG5vbmUAAAAEbm9uZQAAAAAAAAABAAAAJgAAACJzay1lY2RzYS1zaGEyLW5pc3RwMjU2QG9wZW5zc2guY29tAAAALgAAACoAAAAqAAAAInNrLWVjZHNhLXNoYTItbmlzdHAyNTZAb3BlbnNzaC5jb20=",
        ] {
            let key = format!(
                "-----BEGIN OPENSSH PRIVATE KEY-----\n{body}\n-----END OPENSSH PRIVATE KEY-----"
            );
            assert_eq!(
                private_key_contents_unencrypted_kind(&key),
                Some(UnencryptedPrivateKeyKind::SecurityKeyHandle)
            );
        }
        assert_eq!(finding_severity(SECURITY_KEY_REASON), "medium");
        assert_eq!(finding_severity("SSH private key"), "high");
    }
}

fn finding_severity(explanation: &str) -> &'static str {
    if explanation.starts_with(SECURITY_KEY_REASON) {
        "medium"
    } else {
        "high"
    }
}

pub(crate) fn findings(home: &std::path::Path) -> Vec<crate::Finding> {
    let mut findings = super::radioisotope::findings("openssh", install_insecurity_reasons, home);
    for finding in &mut findings {
        finding.severity = finding_severity(&finding.explanation);
    }
    findings
}
