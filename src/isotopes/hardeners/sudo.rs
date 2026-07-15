use std::io::Write;
use std::path::{Path, PathBuf};

use super::HardenerDetection;

const PAM_DIR: &str = "/etc/pam.d";
const SUDO_LOCAL_PATH: &str = "/etc/pam.d/sudo_local";
const ENABLE_TOUCH_ID_COMMAND: &str =
    "echo 'auth sufficient pam_tid.so' | sudo tee -a /etc/pam.d/sudo_local >/dev/null";

pub(crate) fn run(stdout: &mut dyn Write, color: bool) -> Result<(), String> {
    writeln!(stdout, "╭─ harden sudo").ok();
    writeln!(stdout, "│").ok();
    writeln!(stdout, "◇ enables biometric authentication for sudo").ok();
    writeln!(stdout, "│").ok();
    if pam_tid_enabled(&pam_dir())? {
        writeln!(stdout, "╰─ {}", green("already hardened ✔︎", color)).ok();
    } else {
        writeln!(stdout, "╰─ run:").ok();
        writeln!(stdout).ok();
        writeln!(stdout, "        {ENABLE_TOUCH_ID_COMMAND}").ok();
    }
    Ok(())
}

pub(crate) fn detect() -> HardenerDetection {
    let target = Some(SUDO_LOCAL_PATH.to_string());
    HardenerDetection::configuration(
        pam_tid_enabled(&pam_dir()).unwrap_or(false),
        crate::isotopes::detectors::sudo::biometrics_available(),
        target,
    )
}

fn green(text: &str, color: bool) -> String {
    if color {
        format!("\x1b[32m{text}\x1b[0m")
    } else {
        text.to_string()
    }
}

fn pam_dir() -> PathBuf {
    crate::test_env_var("AUTOMIC_VAULT_TEST_SUDO_PAM_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(PAM_DIR))
}

fn pam_tid_enabled(pam_dir: &Path) -> Result<bool, String> {
    for name in ["sudo_local", "sudo"] {
        let path = pam_dir.join(name);
        if path.exists() && file_has_line(&path, line_enables_pam_tid)? {
            return Ok(true);
        }
    }
    Ok(false)
}

fn file_has_line(path: &Path, predicate: fn(&str) -> bool) -> Result<bool, String> {
    let contents = std::fs::read_to_string(path)
        .map_err(|err| format!("failed to read {}: {err}", path.display()))?;
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
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn prints_touch_id_command_when_unhardened() {
        let _guard = crate::global_test_env_lock().lock().unwrap();
        let pam = temp_dir("unhardened");
        fs::write(pam.join("sudo_local"), "#auth sufficient pam_tid.so\n").unwrap();
        unsafe {
            std::env::set_var("AUTOMIC_VAULT_TEST_SUDO_PAM_DIR", &pam);
        }
        let mut stdout = Vec::new();

        run(&mut stdout, false).unwrap();

        unsafe {
            std::env::remove_var("AUTOMIC_VAULT_TEST_SUDO_PAM_DIR");
        }
        let stdout = String::from_utf8(stdout).unwrap();
        assert_eq!(
            stdout,
            "╭─ harden sudo\n│\n◇ enables biometric authentication for sudo\n│\n╰─ run:\n\n        echo 'auth sufficient pam_tid.so' | sudo tee -a /etc/pam.d/sudo_local >/dev/null\n"
        );
        let _ = fs::remove_dir_all(pam);
    }

    #[test]
    fn reports_already_hardened() {
        let _guard = crate::global_test_env_lock().lock().unwrap();
        let pam = temp_dir("hardened");
        fs::write(pam.join("sudo_local"), "auth sufficient pam_tid.so\n").unwrap();
        unsafe {
            std::env::set_var("AUTOMIC_VAULT_TEST_SUDO_PAM_DIR", &pam);
        }
        let mut stdout = Vec::new();

        run(&mut stdout, false).unwrap();

        unsafe {
            std::env::remove_var("AUTOMIC_VAULT_TEST_SUDO_PAM_DIR");
        }
        let stdout = String::from_utf8(stdout).unwrap();
        assert!(stdout.contains("◇ enables biometric authentication for sudo"));
        assert!(stdout.contains("already hardened ✔︎"));
        let _ = fs::remove_dir_all(pam);
    }

    #[test]
    fn colors_already_hardened_when_enabled() {
        assert_eq!(
            green("already hardened ✔︎", true),
            "\x1b[32malready hardened ✔︎\x1b[0m"
        );
    }

    fn temp_dir(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path =
            std::env::temp_dir().join(format!("av-sudo-{label}-{}-{nanos}", std::process::id()));
        fs::create_dir_all(&path).unwrap();
        path
    }
}
