use std::path::{Path, PathBuf};

#[cfg(unix)]
use std::os::unix::fs::MetadataExt;

use crate::{AffectedFile, Finding};

const NAME: &str = "homebrew";
const DOCS_URL: &str = "https://github.com/automic-vault/radioisotopes/tree/main/homebrew";

pub(crate) fn findings(_home: &Path) -> Vec<Finding> {
    install_insecurity_reasons()
        .unwrap_or_default()
        .into_iter()
        .map(|reason| Finding {
            source: NAME,
            homepage: DOCS_URL,
            severity: "medium",
            explanation: reason,
            solution: "Run `sudo av harden brew`.".to_string(),
            affected: vec![AffectedFile {
                path: brew_prefix().display().to_string(),
                line: None,
            }],
            docs_url: DOCS_URL,
        })
        .collect()
}

pub fn install_insecurity_reasons() -> Result<Vec<String>, String> {
    let target = brew_target_path();
    if !target.exists() {
        return Ok(Vec::new());
    }

    let mut reasons = Vec::new();
    if !prefix_owned_by_automic_vault()? {
        reasons.push(format!(
            "Homebrew prefix is mutable: {}",
            brew_prefix().display()
        ));
    }
    Ok(reasons)
}

#[cfg(unix)]
fn prefix_owned_by_automic_vault() -> Result<bool, String> {
    let Some(uid) = test_u32("AUTOMIC_VAULT_TEST_AUTOMIC_UID").or_else(automic_uid) else {
        return Ok(false);
    };
    let Some(gid) = test_u32("AUTOMIC_VAULT_TEST_VAULT_GID").or_else(vault_gid) else {
        return Ok(false);
    };
    let metadata = std::fs::metadata(brew_prefix())
        .map_err(|err| format!("failed to stat {}: {err}", brew_prefix().display()))?;
    Ok(metadata.uid() == uid && metadata.gid() == gid)
}

#[cfg(not(unix))]
fn prefix_owned_by_automic_vault() -> Result<bool, String> {
    Ok(false)
}

fn automic_uid() -> Option<u32> {
    dscl_read("/Users/automic", "UniqueID")
        .ok()
        .flatten()?
        .parse()
        .ok()
}

fn vault_gid() -> Option<u32> {
    dscl_read("/Groups/vault", "PrimaryGroupID")
        .ok()
        .flatten()?
        .parse()
        .ok()
}

fn dscl_read(record: &str, attribute: &str) -> Result<Option<String>, String> {
    let output = std::process::Command::new("/usr/bin/dscl")
        .args([".", "-read", record, attribute])
        .output()
        .map_err(|err| format!("failed to run dscl: {err}"))?;
    if !output.status.success() {
        return Ok(None);
    }
    Ok(String::from_utf8_lossy(&output.stdout)
        .split_once(':')
        .map(|(_, value)| value.trim().to_string())
        .filter(|value| !value.is_empty()))
}

fn test_u32(name: &str) -> Option<u32> {
    std::env::var(name).ok()?.parse().ok()
}

fn brew_prefix() -> PathBuf {
    std::env::var_os("AUTOMIC_VAULT_TEST_BREW_PREFIX")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/opt/homebrew"))
}

fn brew_target_path() -> PathBuf {
    std::env::var_os("AUTOMIC_VAULT_TEST_BREW_TARGET")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/opt/homebrew/bin/brew"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn ignores_missing_homebrew() {
        let _guard = crate::global_test_env_lock().lock().unwrap();
        let dir = temp_path("homebrew-missing");
        let target = dir.join("bin/brew");
        set_env([
            ("AUTOMIC_VAULT_TEST_BREW_PREFIX", dir.as_path()),
            ("AUTOMIC_VAULT_TEST_BREW_TARGET", target.as_path()),
        ]);

        assert_eq!(install_insecurity_reasons().unwrap(), Vec::<String>::new());

        clear_env([
            "AUTOMIC_VAULT_TEST_BREW_PREFIX",
            "AUTOMIC_VAULT_TEST_BREW_TARGET",
        ]);
    }

    #[test]
    fn reports_mutable_homebrew_prefix() {
        let _guard = crate::global_test_env_lock().lock().unwrap();
        let dir = temp_path("homebrew-mutable");
        let target = dir.join("bin/brew");
        std::fs::create_dir_all(target.parent().unwrap()).unwrap();
        std::fs::write(&target, "").unwrap();
        set_env([
            ("AUTOMIC_VAULT_TEST_BREW_PREFIX", dir.as_path()),
            ("AUTOMIC_VAULT_TEST_BREW_TARGET", target.as_path()),
            ("AUTOMIC_VAULT_TEST_AUTOMIC_UID", "99999".as_ref()),
            ("AUTOMIC_VAULT_TEST_VAULT_GID", "99999".as_ref()),
        ]);

        let reasons = install_insecurity_reasons().unwrap();
        let findings = findings(&dir);

        clear_env([
            "AUTOMIC_VAULT_TEST_BREW_PREFIX",
            "AUTOMIC_VAULT_TEST_BREW_TARGET",
            "AUTOMIC_VAULT_TEST_AUTOMIC_UID",
            "AUTOMIC_VAULT_TEST_VAULT_GID",
        ]);
        assert_eq!(reasons.len(), 1);
        assert_eq!(
            reasons[0],
            format!("Homebrew prefix is mutable: {}", dir.display())
        );
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].severity, "medium");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn ignores_stub_state_when_prefix_is_protected() {
        let _guard = crate::global_test_env_lock().lock().unwrap();
        let dir = temp_path("homebrew-protected");
        let target = dir.join("bin/brew");
        let invalid_stub = dir.join("ordinary-brew");
        std::fs::create_dir_all(target.parent().unwrap()).unwrap();
        std::fs::write(&target, "").unwrap();
        std::fs::write(&invalid_stub, "").unwrap();
        set_env([
            ("AUTOMIC_VAULT_TEST_BREW_PREFIX", dir.as_path()),
            ("AUTOMIC_VAULT_TEST_BREW_TARGET", target.as_path()),
            ("AUTOMIC_VAULT_TEST_BREW_STUB", invalid_stub.as_path()),
            ("AUTOMIC_VAULT_TEST_AUTOMIC_UID", uid_string().as_ref()),
            ("AUTOMIC_VAULT_TEST_VAULT_GID", gid_string().as_ref()),
        ]);

        let reasons = install_insecurity_reasons().unwrap();

        clear_env([
            "AUTOMIC_VAULT_TEST_BREW_PREFIX",
            "AUTOMIC_VAULT_TEST_BREW_TARGET",
            "AUTOMIC_VAULT_TEST_BREW_STUB",
            "AUTOMIC_VAULT_TEST_AUTOMIC_UID",
            "AUTOMIC_VAULT_TEST_VAULT_GID",
        ]);
        assert_eq!(reasons, Vec::<String>::new());
        let _ = std::fs::remove_dir_all(dir);
    }

    fn set_env<const N: usize>(pairs: [(&str, &Path); N]) {
        for (key, value) in pairs {
            unsafe {
                std::env::set_var(key, value);
            }
        }
    }

    fn clear_env<const N: usize>(keys: [&str; N]) {
        for key in keys {
            unsafe {
                std::env::remove_var(key);
            }
        }
    }

    fn uid_string() -> String {
        unsafe { libc::getuid() }.to_string()
    }

    fn gid_string() -> String {
        unsafe { libc::getgid() }.to_string()
    }

    fn temp_path(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("av-{label}-{}-{nanos}", std::process::id()))
    }
}
