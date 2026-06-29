#![allow(dead_code)]

use std::path::{Path, PathBuf};

const MAX_FILE_BYTES: u64 = 1024 * 1024;
const MAX_SCAN_DEPTH: usize = 6;

pub fn install_is_insecure() -> Result<bool, String> {
    install_insecurity_reasons().map(|reasons| !reasons.is_empty())
}

pub fn install_insecurity_reasons() -> Result<Vec<String>, String> {
    let mut reasons = Vec::new();
    for root in candidate_roots()? {
        if root.is_dir() {
            scan_dir(&root, 0, &mut reasons)?;
        } else if root.is_file() {
            inspect_file(&root, &mut reasons)?;
        }
    }
    reasons.sort();
    reasons.dedup();
    Ok(reasons)
}

fn candidate_roots() -> Result<Vec<PathBuf>, String> {
    let home = home_dir()?;
    let config_home = xdg_config_home().unwrap_or_else(|| home.join(".config"));
    Ok(vec![
        home.join(".openvpn"),
        config_home.join("openvpn"),
        home.join(".config/openvpn"),
        home.join("Library/Application Support/OpenVPN"),
        home.join("Library/Application Support/Tunnelblick/Configurations"),
    ])
}

fn home_dir() -> Result<PathBuf, String> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())
}

fn xdg_config_home() -> Option<PathBuf> {
    std::env::var_os("XDG_CONFIG_HOME")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
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
            inspect_file(&entry.path(), reasons)?;
        }
    }
    Ok(())
}

fn inspect_file(path: &Path, reasons: &mut Vec<String>) -> Result<(), String> {
    let metadata = std::fs::metadata(path)
        .map_err(|err| format!("failed to stat {}: {err}", path.display()))?;
    if metadata.len() > MAX_FILE_BYTES {
        return Ok(());
    }
    let contents = std::fs::read_to_string(path)
        .map_err(|err| format!("failed to read {}: {err}", path.display()))?;
    if openvpn_config_contains_inline_secret(&contents) {
        reasons.push(format!(
            "OpenVPN profile contains inline plaintext key or password material: {}",
            path.display()
        ));
    }
    if is_openvpn_config(path) {
        for auth_path in auth_user_pass_paths(&contents, path)? {
            if auth_file_contains_password(&auth_path)? {
                reasons.push(format!(
                    "OpenVPN auth-user-pass file contains plaintext credentials: {}",
                    auth_path.display()
                ));
            }
        }
    }
    Ok(())
}

fn is_openvpn_config(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|value| value.to_str()),
        Some("ovpn") | Some("conf")
    )
}

fn openvpn_config_contains_inline_secret(contents: &str) -> bool {
    contains_private_key(contents)
        || contents.contains("-----BEGIN OpenVPN Static key V1-----")
        || inline_block_has_secret(contents, "auth-user-pass")
        || inline_block_has_secret(contents, "tls-auth")
        || inline_block_has_secret(contents, "tls-crypt")
        || inline_block_has_secret(contents, "tls-crypt-v2")
}

fn contains_private_key(contents: &str) -> bool {
    contents.contains("-----BEGIN PRIVATE KEY-----")
        || contents.contains("-----BEGIN RSA PRIVATE KEY-----")
        || contents.contains("-----BEGIN EC PRIVATE KEY-----")
}

fn inline_block_has_secret(contents: &str, name: &str) -> bool {
    let begin = format!("<{name}>");
    let end = format!("</{name}>");
    let Some((_, after_begin)) = contents.split_once(&begin) else {
        return false;
    };
    let Some((body, _)) = after_begin.split_once(&end) else {
        return false;
    };
    body.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .count()
        >= 2
}

fn auth_user_pass_paths(contents: &str, config_path: &Path) -> Result<Vec<PathBuf>, String> {
    let mut paths = Vec::new();
    for line in contents.lines() {
        let line = line.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        let mut words = line.split_whitespace();
        let Some(key) = words.next() else {
            continue;
        };
        if key != "auth-user-pass" {
            continue;
        }
        if let Some(path) = words.next() {
            let path = trim_quotes(path);
            let expanded = if let Some(rest) = path.strip_prefix("~/") {
                home_dir()?.join(rest)
            } else {
                let candidate = PathBuf::from(path);
                if candidate.is_absolute() {
                    candidate
                } else {
                    config_path
                        .parent()
                        .unwrap_or_else(|| Path::new("."))
                        .join(candidate)
                }
            };
            paths.push(expanded);
        }
    }
    Ok(paths)
}

fn auth_file_contains_password(path: &Path) -> Result<bool, String> {
    if !path.exists() {
        return Ok(false);
    }
    let contents = std::fs::read_to_string(path)
        .map_err(|err| format!("failed to read {}: {err}", path.display()))?;
    Ok(contents
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .count()
        >= 2)
}

fn trim_quotes(value: &str) -> &str {
    value.trim_matches('"').trim_matches('\'')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_inline_and_referenced_auth() {
        assert!(openvpn_config_contains_inline_secret(
            "<auth-user-pass>\nuser\npass\n</auth-user-pass>\n"
        ));
        assert!(!openvpn_config_contains_inline_secret("auth-user-pass\n"));
        let paths =
            auth_user_pass_paths("auth-user-pass secrets.txt\n", Path::new("/tmp/vpn.ovpn"))
                .unwrap();
        assert_eq!(paths, vec![PathBuf::from("/tmp/secrets.txt")]);
    }
}

pub(crate) fn findings(home: &std::path::Path) -> Vec<crate::Finding> {
    super::radioisotope::findings("openvpn", install_insecurity_reasons, home)
}
