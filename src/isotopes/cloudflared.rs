#![allow(dead_code)]

use std::path::{Path, PathBuf};

const MAX_FILE_BYTES: u64 = 1024 * 1024;
const MAX_SCAN_DEPTH: usize = 3;

pub fn install_is_insecure() -> Result<bool, String> {
    install_insecurity_reasons().map(|reasons| !reasons.is_empty())
}

pub fn install_insecurity_reasons() -> Result<Vec<String>, String> {
    let mut reasons = Vec::new();
    for root in candidate_roots()? {
        if root.is_dir() {
            scan_dir(&root, 0, &mut reasons)?;
        } else if root.is_file() {
            push_file_reason(&root, &mut reasons)?;
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
        home.join(".cloudflared"),
        config_home.join("cloudflared"),
        home.join(".config/cloudflared"),
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
            push_file_reason(&entry.path(), reasons)?;
        }
    }
    Ok(())
}

fn push_file_reason(path: &Path, reasons: &mut Vec<String>) -> Result<(), String> {
    let metadata = std::fs::metadata(path)
        .map_err(|err| format!("failed to stat {}: {err}", path.display()))?;
    if metadata.len() > MAX_FILE_BYTES {
        return Ok(());
    }
    let contents = std::fs::read_to_string(path)
        .map_err(|err| format!("failed to read {}: {err}", path.display()))?;
    if contains_private_key(&contents) {
        reasons.push(format!(
            "cloudflared certificate contains a plaintext private key: {}",
            path.display()
        ));
    } else if contains_tunnel_secret(&contents) {
        reasons.push(format!(
            "cloudflared tunnel credentials are stored in plaintext: {}",
            path.display()
        ));
    }
    Ok(())
}

fn contains_private_key(contents: &str) -> bool {
    contents.contains("-----BEGIN PRIVATE KEY-----")
        || contents.contains("-----BEGIN RSA PRIVATE KEY-----")
        || contents.contains("-----BEGIN EC PRIVATE KEY-----")
}

fn contains_tunnel_secret(contents: &str) -> bool {
    string_values_for_key(contents, "TunnelSecret")
        .into_iter()
        .chain(string_values_for_key(contents, "tunnelSecret"))
        .any(|value| !value.trim().is_empty())
}

fn string_values_for_key(contents: &str, key: &str) -> Vec<String> {
    let needle = format!("\"{key}\"");
    contents
        .split(&needle)
        .skip(1)
        .filter_map(|after_key| after_key.split_once(':').map(|(_, value)| value))
        .filter_map(|value| json_string_value(value.trim_start()).map(str::to_string))
        .collect()
}

fn json_string_value(value: &str) -> Option<&str> {
    let value = value.strip_prefix('"')?;
    let mut escaped = false;
    for (index, ch) in value.char_indices() {
        if escaped {
            escaped = false;
        } else if ch == '\\' {
            escaped = true;
        } else if ch == '"' {
            return Some(&value[..index]);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_tunnel_credentials() {
        assert!(contains_tunnel_secret(r#"{"TunnelSecret":"secret"}"#));
        assert!(!contains_tunnel_secret(r#"{"TunnelID":"id"}"#));
    }
}

pub(crate) fn findings(home: &std::path::Path) -> Vec<crate::Finding> {
    super::radioisotope::findings("cloudflared", install_insecurity_reasons, home)
}
