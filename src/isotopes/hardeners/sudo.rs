use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};

use super::{HardenerDetection, RootOnlyOutcome};

const PAM_DIR: &str = "/etc/pam.d";
const SUDO_LOCAL_PATH: &str = "/etc/pam.d/sudo_local";
const PAM_TID_LINE: &str = "auth sufficient pam_tid.so";
const PRIVILEGE_MODE: super::PrivilegeMode = super::PrivilegeMode::RootOnly;

pub(crate) fn run(stdout: &mut dyn Write, color: bool) -> Result<RootOnlyOutcome, String> {
    let pam_dir = pam_dir();
    writeln!(stdout, "╭─ harden sudo").ok();
    if pam_tid_enabled(&pam_dir)? {
        writeln!(stdout, "╰─ {}", green("already hardened ✔︎", color)).ok();
        return Ok(RootOnlyOutcome::Hardened);
    }
    if PRIVILEGE_MODE == super::PrivilegeMode::RootOnly && super::effective_uid() != 0 {
        writeln!(stdout, "│").ok();
        writeln!(stdout, "├─ enable biometric authentication for sudo").ok();
        writeln!(stdout, "╰─ next: sudo av harden sudo").ok();
        return Ok(RootOnlyOutcome::Previewed);
    }

    enable_pam_tid(&pam_dir)?;
    writeln!(stdout, "╰─ hardened sudo").ok();
    Ok(RootOnlyOutcome::Hardened)
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

fn enable_pam_tid(pam_dir: &Path) -> Result<(), String> {
    let test_override = crate::test_env_var("AUTOMIC_VAULT_TEST_SUDO_PAM_DIR").is_some();
    let directory = fs::symlink_metadata(pam_dir)
        .map_err(|err| format!("failed to inspect {}: {err}", pam_dir.display()))?;
    if !directory.file_type().is_dir()
        || !test_override && (directory.uid() != 0 || directory.permissions().mode() & 0o022 != 0)
    {
        return Err(format!(
            "refusing to modify untrusted {}",
            pam_dir.display()
        ));
    }

    let path = pam_dir.join("sudo_local");
    let mut file = OpenOptions::new()
        .read(true)
        .append(true)
        .create(true)
        .mode(0o644)
        .custom_flags(libc::O_NOFOLLOW)
        .open(&path)
        .map_err(|err| format!("failed to open {}: {err}", path.display()))?;
    let metadata = file
        .metadata()
        .map_err(|err| format!("failed to inspect {}: {err}", path.display()))?;
    if !metadata.file_type().is_file()
        || !test_override && (metadata.uid() != 0 || metadata.permissions().mode() & 0o022 != 0)
    {
        return Err(format!("refusing to modify untrusted {}", path.display()));
    }
    let mut contents = String::new();
    file.read_to_string(&mut contents)
        .map_err(|err| format!("failed to read {}: {err}", path.display()))?;
    if contents.lines().any(line_enables_pam_tid) {
        return Ok(());
    }
    if !contents.is_empty() && !contents.ends_with('\n') {
        writeln!(file).map_err(|err| format!("failed to update {}: {err}", path.display()))?;
    }
    writeln!(file, "{PAM_TID_LINE}")
        .and_then(|()| file.sync_all())
        .map_err(|err| format!("failed to update {}: {err}", path.display()))?;
    pam_tid_enabled(pam_dir)?
        .then_some(())
        .ok_or_else(|| format!("failed to verify {}", path.display()))
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
    fn previews_touch_id_change_when_unprivileged() {
        let _guard = crate::global_test_env_lock().lock().unwrap();
        let pam = temp_dir("unhardened");
        fs::write(pam.join("sudo_local"), "#auth sufficient pam_tid.so\n").unwrap();
        unsafe {
            std::env::set_var("AUTOMIC_VAULT_TEST_SUDO_PAM_DIR", &pam);
        }
        let mut stdout = Vec::new();

        assert_eq!(run(&mut stdout, false).unwrap(), RootOnlyOutcome::Previewed);

        unsafe {
            std::env::remove_var("AUTOMIC_VAULT_TEST_SUDO_PAM_DIR");
        }
        let stdout = String::from_utf8(stdout).unwrap();
        assert_eq!(
            stdout,
            "╭─ harden sudo\n│\n├─ enable biometric authentication for sudo\n╰─ next: sudo av harden sudo\n"
        );
        let _ = fs::remove_dir_all(pam);
    }

    #[test]
    fn privileged_run_hardens_without_a_prompt() {
        let _guard = crate::global_test_env_lock().lock().unwrap();
        let pam = temp_dir("privileged");
        fs::write(pam.join("sudo_local"), "#auth sufficient pam_tid.so\n").unwrap();
        unsafe {
            std::env::set_var("AUTOMIC_VAULT_TEST_SUDO_PAM_DIR", &pam);
            std::env::set_var("AUTOMIC_VAULT_TEST_EUID", "0");
        }
        let mut stdout = Vec::new();

        assert_eq!(run(&mut stdout, false).unwrap(), RootOnlyOutcome::Hardened);

        unsafe {
            std::env::remove_var("AUTOMIC_VAULT_TEST_SUDO_PAM_DIR");
            std::env::remove_var("AUTOMIC_VAULT_TEST_EUID");
        }
        assert_eq!(
            String::from_utf8(stdout).unwrap(),
            "╭─ harden sudo\n╰─ hardened sudo\n"
        );
        assert_eq!(
            fs::read_to_string(pam.join("sudo_local")).unwrap(),
            "#auth sufficient pam_tid.so\nauth sufficient pam_tid.so\n"
        );
        let _ = fs::remove_dir_all(pam);
    }

    #[test]
    fn privileged_run_refuses_a_symlinked_pam_file() {
        use std::os::unix::fs::symlink;

        let _guard = crate::global_test_env_lock().lock().unwrap();
        let pam = temp_dir("symlink");
        let victim = pam.join("victim");
        fs::write(&victim, "leave me alone\n").unwrap();
        symlink(&victim, pam.join("sudo_local")).unwrap();
        unsafe {
            std::env::set_var("AUTOMIC_VAULT_TEST_SUDO_PAM_DIR", &pam);
            std::env::set_var("AUTOMIC_VAULT_TEST_EUID", "0");
        }

        let err = run(&mut Vec::new(), false).unwrap_err();

        unsafe {
            std::env::remove_var("AUTOMIC_VAULT_TEST_SUDO_PAM_DIR");
            std::env::remove_var("AUTOMIC_VAULT_TEST_EUID");
        }
        assert!(err.contains("failed to open"));
        assert_eq!(fs::read_to_string(&victim).unwrap(), "leave me alone\n");
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

        assert_eq!(run(&mut stdout, false).unwrap(), RootOnlyOutcome::Hardened);

        unsafe {
            std::env::remove_var("AUTOMIC_VAULT_TEST_SUDO_PAM_DIR");
        }
        let stdout = String::from_utf8(stdout).unwrap();
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
