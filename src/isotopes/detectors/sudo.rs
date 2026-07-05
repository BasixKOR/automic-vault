#![allow(dead_code)]

use std::path::{Path, PathBuf};

const PAM_DIR: &str = "/etc/pam.d";
const SUDOERS: &str = "/etc/sudoers";
const SUDOERS_D: &str = "/etc/sudoers.d";

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

    insecurity_reasons(Path::new(PAM_DIR), Path::new(SUDOERS), Path::new(SUDOERS_D))
}

fn insecurity_reasons(
    pam_dir: &Path,
    sudoers: &Path,
    sudoers_d: &Path,
) -> Result<Vec<String>, String> {
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

    match sudo_timestamp_timeout_is_zero(sudoers, sudoers_d) {
        TimestampTimeout::Zero => {}
        TimestampTimeout::NonZero(path) => reasons.push(format!(
            "sudo allows credential reuse instead of asking again immediately: {}",
            path.display()
        )),
        TimestampTimeout::Missing => reasons.push(format!(
            "sudo does not set timestamp_timeout=0: {}",
            sudoers.display()
        )),
        TimestampTimeout::Unreadable(path) => reasons.push(format!(
            "sudo timestamp_timeout could not be checked: {}",
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

enum TimestampTimeout {
    Zero,
    NonZero(PathBuf),
    Unreadable(PathBuf),
    Missing,
}

fn sudo_timestamp_timeout_is_zero(sudoers: &Path, sudoers_d: &Path) -> TimestampTimeout {
    let mut saw_zero = false;
    for path in sudoers_paths(sudoers, sudoers_d) {
        let Ok(contents) = std::fs::read_to_string(&path) else {
            return TimestampTimeout::Unreadable(path);
        };
        for line in contents.lines() {
            let Some(zero) = timestamp_timeout_is_zero(line) else {
                continue;
            };
            if !zero {
                return TimestampTimeout::NonZero(path);
            }
            saw_zero = true;
        }
    }

    if saw_zero {
        TimestampTimeout::Zero
    } else {
        TimestampTimeout::Missing
    }
}

fn sudoers_paths(sudoers: &Path, sudoers_d: &Path) -> Vec<PathBuf> {
    let mut paths = Vec::from([sudoers.to_path_buf()]);
    if sudoers_d.is_dir() {
        let Ok(entries) = std::fs::read_dir(sudoers_d) else {
            paths.push(sudoers_d.to_path_buf());
            return paths;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                paths.push(path);
            }
        }
    }
    paths.sort();
    paths
}

fn timestamp_timeout_is_zero(line: &str) -> Option<bool> {
    let line = line.split('#').next()?.trim_start();
    if !line.starts_with("Defaults") {
        return None;
    }

    let compact: String = line.chars().filter(|ch| !ch.is_whitespace()).collect();
    let value = compact
        .split("timestamp_timeout=")
        .nth(1)?
        .split(',')
        .next()?;
    value.parse::<f64>().ok().map(|value| value == 0.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn detects_hardened_sudo_config() {
        let temp = temp_dir("hardened");
        let pam = temp.join("pam.d");
        let sudoers_d = temp.join("sudoers.d");
        fs::create_dir_all(&pam).unwrap();
        fs::create_dir_all(&sudoers_d).unwrap();
        fs::write(pam.join("sudo_local"), "auth sufficient pam_tid.so\n").unwrap();
        fs::write(temp.join("sudoers"), "Defaults timestamp_timeout=0\n").unwrap();

        assert_eq!(
            insecurity_reasons(&pam, &temp.join("sudoers"), &sudoers_d).unwrap(),
            Vec::<String>::new()
        );

        let _ = fs::remove_dir_all(temp);
    }

    #[test]
    fn reports_missing_touch_id_and_cached_sudo_credentials() {
        let temp = temp_dir("weak");
        let pam = temp.join("pam.d");
        let sudoers_d = temp.join("sudoers.d");
        fs::create_dir_all(&pam).unwrap();
        fs::create_dir_all(&sudoers_d).unwrap();
        fs::write(pam.join("sudo_local"), "#auth sufficient pam_tid.so\n").unwrap();
        fs::write(temp.join("sudoers"), "Defaults timestamp_timeout = 5\n").unwrap();

        let reasons = insecurity_reasons(&pam, &temp.join("sudoers"), &sudoers_d).unwrap();

        assert_eq!(reasons.len(), 2);
        assert!(reasons[0].contains("Touch ID authentication is not enabled"));
        assert!(reasons[1].contains("credential reuse"));

        let _ = fs::remove_dir_all(temp);
    }

    #[test]
    fn parses_common_sudoers_timeout_lines() {
        assert_eq!(
            timestamp_timeout_is_zero("Defaults timestamp_timeout=0"),
            Some(true)
        );
        assert_eq!(
            timestamp_timeout_is_zero("Defaults lecture=never, timestamp_timeout = 0 # ok"),
            Some(true)
        );
        assert_eq!(
            timestamp_timeout_is_zero("Defaults timestamp_timeout=0.0"),
            Some(true)
        );
        assert_eq!(
            timestamp_timeout_is_zero("Defaults:mxcl timestamp_timeout=5"),
            Some(false)
        );
        assert_eq!(
            timestamp_timeout_is_zero("# Defaults timestamp_timeout=5"),
            None
        );
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
