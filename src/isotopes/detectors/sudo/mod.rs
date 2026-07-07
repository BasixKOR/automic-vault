#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::process::Command;

const PAM_DIR: &str = "/etc/pam.d";

pub fn install_is_insecure() -> Result<bool, String> {
    install_insecurity_reasons().map(|reasons| !reasons.is_empty())
}

pub fn install_insecurity_reasons() -> Result<Vec<String>, String> {
    if std::env::var_os("AUTOMIC_VAULT_DISABLE_SUDO_DETECTOR").is_some() {
        return Ok(Vec::new());
    }

    if !cfg!(target_os = "macos") {
        return Ok(Vec::new());
    }

    if !biometrics_available() {
        return Ok(Vec::new());
    }

    insecurity_reasons(&pam_dir())
}

fn pam_dir() -> PathBuf {
    std::env::var_os("AUTOMIC_VAULT_TEST_SUDO_PAM_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(PAM_DIR))
}

fn biometrics_available() -> bool {
    if let Some(value) = std::env::var_os("AUTOMIC_VAULT_TEST_BIOMETRICS_AVAILABLE") {
        return value != "0";
    }

    Command::new("bioutil")
        .arg("-r")
        .output()
        .is_ok_and(|output| {
            output.status.success()
                && String::from_utf8_lossy(&output.stdout)
                    .lines()
                    .any(|line| line.trim() == "Effective biometrics for unlock: 1")
        })
}

fn insecurity_reasons(pam_dir: &Path) -> Result<Vec<String>, String> {
    let mut reasons = Vec::new();

    match pam_tid_enabled(pam_dir) {
        Ok(true) => {}
        Ok(false) => reasons.push(format!(
            "sudo Touch ID authentication is not enabled: {}",
            pam_dir.join("sudo_local").display()
        )),
        Err(path) => reasons.push(format!(
            "sudo Touch ID authentication could not be checked: {}",
            path.display()
        )),
    }

    Ok(reasons)
}

fn pam_tid_enabled(pam_dir: &Path) -> Result<bool, PathBuf> {
    for name in ["sudo_local", "sudo"] {
        let path = pam_dir.join(name);
        if path.exists() && file_has_line(&path, line_enables_pam_tid)? {
            return Ok(true);
        }
    }
    Ok(false)
}

fn file_has_line(path: &Path, predicate: fn(&str) -> bool) -> Result<bool, PathBuf> {
    let contents = std::fs::read_to_string(path).map_err(|_| path.to_path_buf())?;
    Ok(contents.lines().any(predicate))
}

fn line_enables_pam_tid(line: &str) -> bool {
    let line = line.trim_start();
    !line.starts_with('#')
        && line
            .split_whitespace()
            .next()
            .is_some_and(|field| field == "auth")
        && line.split_whitespace().any(|field| field == "pam_tid.so")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn detects_hardened_sudo_config() {
        let temp = temp_dir("hardened");
        let pam = temp.join("pam.d");
        fs::create_dir_all(&pam).unwrap();
        fs::write(pam.join("sudo_local"), "auth sufficient pam_tid.so\n").unwrap();

        assert_eq!(insecurity_reasons(&pam).unwrap(), Vec::<String>::new());

        let _ = fs::remove_dir_all(temp);
    }

    #[test]
    fn reports_missing_touch_id() {
        let temp = temp_dir("weak");
        let pam = temp.join("pam.d");
        fs::create_dir_all(&pam).unwrap();
        fs::write(pam.join("sudo_local"), "#auth sufficient pam_tid.so\n").unwrap();

        let reasons = insecurity_reasons(&pam).unwrap();

        assert_eq!(reasons.len(), 1);
        assert!(reasons[0].contains("Touch ID authentication is not enabled"));

        let _ = fs::remove_dir_all(temp);
    }

    #[test]
    fn top_level_detector_skips_sudo_when_biometrics_are_unavailable() {
        let _guard = crate::global_test_env_lock().lock().unwrap();
        let temp = temp_dir("unavailable");
        let pam = temp.join("pam.d");
        fs::create_dir_all(&pam).unwrap();
        fs::write(pam.join("sudo_local"), "#auth sufficient pam_tid.so\n").unwrap();
        unsafe {
            std::env::set_var("AUTOMIC_VAULT_TEST_SUDO_PAM_DIR", &pam);
            std::env::set_var("AUTOMIC_VAULT_TEST_BIOMETRICS_AVAILABLE", "0");
        }

        assert_eq!(install_insecurity_reasons().unwrap(), Vec::<String>::new());

        unsafe {
            std::env::remove_var("AUTOMIC_VAULT_TEST_SUDO_PAM_DIR");
            std::env::remove_var("AUTOMIC_VAULT_TEST_BIOMETRICS_AVAILABLE");
        }
        let _ = fs::remove_dir_all(temp);
    }

    fn temp_dir(label: &str) -> PathBuf {
        let path =
            std::env::temp_dir().join(format!("sudo-detector-{label}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        path
    }
}

pub(crate) fn findings(home: &std::path::Path) -> Vec<crate::Finding> {
    let _ = home;
    super::radioisotope::findings("sudo", install_insecurity_reasons, home)
}
