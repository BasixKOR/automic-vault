#![allow(dead_code)]

use std::path::{Path, PathBuf};

const MAX_SCAN_DEPTH: usize = 4;
const MAX_FILE_BYTES: u64 = 1024 * 1024;

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
        home.join(".fastlane/spaceship"),
        home.join(".spaceship"),
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
        } else if file_type.is_file()
            && entry.file_name().to_string_lossy() == "cookie"
            && cookie_file_contains_session(&entry.path())?
        {
            reasons.push(format!(
                "fastlane Spaceship session cookie is stored in plaintext: {}",
                entry.path().display()
            ));
        }
    }
    Ok(())
}

fn cookie_file_contains_session(path: &Path) -> Result<bool, String> {
    let metadata = std::fs::metadata(path)
        .map_err(|err| format!("failed to stat {}: {err}", path.display()))?;
    if metadata.len() > MAX_FILE_BYTES {
        return Ok(false);
    }
    let contents = std::fs::read_to_string(path)
        .map_err(|err| format!("failed to read {}: {err}", path.display()))?;
    Ok(fastlane_cookie_contents_contain_session(&contents))
}

fn fastlane_cookie_contents_contain_session(contents: &str) -> bool {
    let contents = contents.trim();
    !contents.is_empty()
        && ["myacinfo", "DES", "dqsid", "itctx", "session"]
            .iter()
            .any(|needle| contents.contains(needle))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_spaceship_cookie_markers() {
        assert!(fastlane_cookie_contents_contain_session(
            "---\n- !ruby/object:HTTP::Cookie\n  name: myacinfo\n  value: secret\n"
        ));
        assert!(!fastlane_cookie_contents_contain_session(""));
    }
}

pub(crate) fn findings(home: &std::path::Path) -> Vec<crate::Finding> {
    super::radioisotope::findings("fastlane", install_insecurity_reasons, home)
}
