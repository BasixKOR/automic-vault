use std::ffi::CString;
use std::fs;
use std::io::{self, IsTerminal, Write};
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::Command;

use super::{
    HardenerDetection, HardenerDiagnostic, RequiredIdentity, SecretGateDescriptor, SecretGateRoute,
    StubRequirements,
};

const AUTOMIC_USER: &str = "automic";
const VAULT_GROUP: &str = "vault";
const BREW_PREFIX: &str = "/opt/homebrew";
const BREW_TARGET: &str = "/opt/homebrew/bin/brew";
const BREW_STUB: &str = "/usr/local/bin/brew";
const APP_BREW_STUB: &str = "/Applications/Automic Vault.app/Contents/MacOS/av-brew-stub";
const STUB_MARKER: &[u8] = b"AUTOMIC_VAULT_BREW_STUB_V1";
const ID_RANGE: std::ops::RangeInclusive<u32> = 550..=599;

unsafe extern "C" {
    fn geteuid() -> u32;
}

pub(crate) fn run(stdout: &mut dyn Write, yes: bool) -> Result<(), String> {
    let prefix = brew_prefix();
    let target = brew_target_path();
    let stub = brew_stub_path();
    let source = brew_stub_source_path()?;

    if !target.exists() {
        return Err(format!("Homebrew is not installed at {}", target.display()));
    }
    if effective_uid() != 0 {
        return Err("run `sudo av harden brew`".to_string());
    }
    if stub.exists() && !is_managed_stub_file(&stub) {
        return Err(format!(
            "{} already exists and is not an Automic Vault brew stub",
            stub.display()
        ));
    }
    if !is_managed_stub_file(&source) {
        return Err(format!(
            "{} is not an Automic Vault brew stub",
            source.display()
        ));
    }

    writeln!(stdout, "╭─ harden brew").ok();
    writeln!(stdout, "│").ok();
    writeln!(
        stdout,
        "├─ ensure {AUTOMIC_USER} user and {VAULT_GROUP} group"
    )
    .ok();
    writeln!(
        stdout,
        "├─ chown -R -h {AUTOMIC_USER}:{VAULT_GROUP} {}",
        prefix.display()
    )
    .ok();
    writeln!(stdout, "├─ install {}", stub.display()).ok();
    writeln!(stdout, "│").ok();
    if !confirm(stdout, yes)? {
        writeln!(stdout, "╰─ cancelled").ok();
        return Ok(());
    }

    let gid = ensure_group()?;
    let uid = ensure_user(gid)?;
    fs::create_dir_all(prefix.join("var/automic/tmp"))
        .map_err(|err| format!("failed to create Homebrew automic state dir: {err}"))?;
    fs::create_dir_all(prefix.join("var/automic/cache"))
        .map_err(|err| format!("failed to create Homebrew automic cache dir: {err}"))?;
    chown_recursive(&prefix)?;
    install_stub(&source, &stub, uid, gid)?;
    writeln!(
        stdout,
        "╰─ hardened brew; run `hash -r` (or start a new shell) before using brew"
    )
    .ok();
    Ok(())
}

pub(crate) fn detect() -> HardenerDetection {
    let prefix = brew_prefix();
    let stub = brew_stub_path();
    let target = brew_target_path();
    let uid = automic_uid();
    let gid = vault_gid();
    let hardened = if let (Some(uid), Some(gid)) = (uid, gid) {
        is_hardened_stub(&stub, uid, gid)
    } else {
        false
    };
    let mut detection = HardenerDetection::command(
        hardened,
        "brew",
        Some(stub.display().to_string()),
        target.display().to_string(),
    );
    detection.commands[0].stub_valid = stub_matches_source(&stub);
    detection.commands[0].stub_requirements = Some(StubRequirements {
        mode: 0o6755,
        owner: RequiredIdentity {
            name: AUTOMIC_USER,
            id: uid,
        },
        group: RequiredIdentity {
            name: VAULT_GROUP,
            id: gid,
        },
    });
    detection.diagnostics = doctor_diagnostics(&prefix, uid, gid);
    detection
}

pub(crate) fn secret_gate() -> SecretGateDescriptor {
    SecretGateDescriptor {
        id: "brew",
        key_patterns: Vec::new(),
        routes: vec![SecretGateRoute {
            operation: "authorize",
            script_path: None,
            target_path: brew_target_path().display().to_string(),
            caller_identifiers: vec!["com.automicvault.av-brew-stub"],
            key_patterns: Vec::new(),
            replace_existing_env: false,
            allow_missing_keys: false,
        }],
    }
}

fn doctor_diagnostics(
    prefix: &Path,
    uid: Option<u32>,
    gid: Option<u32>,
) -> Vec<HardenerDiagnostic> {
    let mut diagnostics = match (uid, gid) {
        (Some(uid), Some(gid)) => state_directory_diagnostics(prefix, uid, gid),
        _ => Vec::new(),
    };
    if std::env::var_os("AUTOMIC_VAULT_TEST_AUTOMIC_UID").is_none()
        && let Some(gid) = gid
    {
        diagnostics.extend(account_diagnostics(gid));
    }
    diagnostics
}

fn state_directory_diagnostics(prefix: &Path, uid: u32, gid: u32) -> Vec<HardenerDiagnostic> {
    [
        prefix.join("var/automic"),
        prefix.join("var/automic/tmp"),
        prefix.join("var/automic/cache"),
    ]
    .into_iter()
    .filter_map(|path| {
        let path_text = path.display().to_string();
        let metadata = match fs::symlink_metadata(&path) {
            Ok(metadata) if metadata.file_type().is_dir() => metadata,
            Ok(metadata) => {
                let actual = if metadata.file_type().is_symlink() {
                    "a symbolic link"
                } else {
                    "not a directory"
                };
                return Some(HardenerDiagnostic {
                    kind: "state_directory_wrong_type",
                    message: format!(
                        "Homebrew hardening state path {path_text} is {actual}; expected a directory"
                    ),
                    remediation: format!(
                        "Review and remove {path_text}, then run `sudo install -d -o {AUTOMIC_USER} -g {VAULT_GROUP} -m 0755 {path_text}`."
                    ),
                    path: Some(path_text),
                });
            }
            Err(err) if err.kind() == io::ErrorKind::NotFound => {
                return Some(HardenerDiagnostic {
                    kind: "state_directory_missing",
                    message: format!(
                        "Homebrew's hardened launcher requires state directory {path_text}, but it is missing"
                    ),
                    remediation: format!(
                        "Create it with `sudo install -d -o {AUTOMIC_USER} -g {VAULT_GROUP} -m 0755 {path_text}`, then rerun `av doctor brew`."
                    ),
                    path: Some(path_text),
                });
            }
            Err(err) => {
                return Some(HardenerDiagnostic {
                    kind: "state_directory_unreadable",
                    message: format!("cannot inspect Homebrew state directory {path_text}: {err}"),
                    remediation: format!(
                        "Check metadata and parent-directory permissions for {path_text}, then rerun `av doctor brew`."
                    ),
                    path: Some(path_text),
                });
            }
        };
        let mode = metadata.permissions().mode() & 0o7777;
        if metadata.uid() != uid
            || metadata.gid() != gid
            || mode & 0o700 != 0o700
        {
            return Some(HardenerDiagnostic {
                kind: "state_directory_permissions_invalid",
                message: format!(
                    "Homebrew state directory {path_text} has uid {}, gid {}, and mode {mode:#06o}; expected {AUTOMIC_USER} ({uid}), {VAULT_GROUP} ({gid}), with owner read/write/search access",
                    metadata.uid(), metadata.gid()
                ),
                remediation: format!(
                    "Run `sudo chown {AUTOMIC_USER}:{VAULT_GROUP} {path_text} && sudo chmod u+rwx {path_text}`, then rerun `av doctor brew`."
                ),
                path: Some(path_text),
            });
        }
        None
    })
    .collect()
}

fn account_diagnostics(expected_gid: u32) -> Vec<HardenerDiagnostic> {
    let actual_gid = dscl_read("/Users/automic", "PrimaryGroupID");
    let home = dscl_read("/Users/automic", "NFSHomeDirectory");
    let shell = dscl_read("/Users/automic", "UserShell");
    let errors = [&actual_gid, &home, &shell]
        .into_iter()
        .filter_map(|result| result.as_ref().err())
        .cloned()
        .collect::<Vec<_>>();
    if !errors.is_empty() {
        return vec![HardenerDiagnostic {
            kind: "account_unreadable",
            message: format!(
                "cannot inspect the local `automic` account with dscl: {}",
                errors.join("; ")
            ),
            remediation: "Run `dscl . -read /Users/automic PrimaryGroupID NFSHomeDirectory UserShell` to inspect the failure, then rerun `sudo av harden brew`.".to_string(),
            path: None,
        }];
    }
    let (Ok(actual_gid), Ok(home), Ok(shell)) = (actual_gid, home, shell) else {
        unreachable!();
    };
    let expected_gid = expected_gid.to_string();
    if actual_gid.as_deref() == Some(expected_gid.as_str())
        && home.as_deref() == Some("/opt/homebrew/var/automic")
        && shell.as_deref() == Some("/usr/bin/false")
    {
        return Vec::new();
    }
    vec![HardenerDiagnostic {
        kind: "account_configuration_invalid",
        message: format!(
            "local `automic` account has PrimaryGroupID {}, NFSHomeDirectory {}, and UserShell {}; expected {expected_gid}, /opt/homebrew/var/automic, and /usr/bin/false",
            actual_gid.as_deref().unwrap_or("missing"),
            home.as_deref().unwrap_or("missing"),
            shell.as_deref().unwrap_or("missing"),
        ),
        remediation: format!(
            "Repair it with `sudo dscl . -create /Users/automic PrimaryGroupID {expected_gid}`, `sudo dscl . -create /Users/automic NFSHomeDirectory /opt/homebrew/var/automic`, and `sudo dscl . -create /Users/automic UserShell /usr/bin/false`, then rerun `av doctor brew`."
        ),
        path: None,
    }]
}

fn confirm(stdout: &mut dyn Write, yes: bool) -> Result<bool, String> {
    if yes {
        writeln!(stdout, "◇ Continue? yes (--yes)").ok();
        return Ok(true);
    }

    write!(stdout, "◇ Continue? [y/N] ").ok();
    stdout
        .flush()
        .map_err(|err| format!("failed to flush prompt: {err}"))?;
    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .map_err(|err| format!("failed to read confirmation: {err}"))?;
    if !io::stdin().is_terminal() {
        writeln!(stdout).ok();
    }
    Ok(matches!(
        input.trim().to_ascii_lowercase().as_str(),
        "y" | "yes"
    ))
}

fn ensure_group() -> Result<u32, String> {
    if let Some(gid) = vault_gid() {
        return Ok(gid);
    }
    let gid = first_unused_id("/Groups", "PrimaryGroupID")?;
    dscl([".", "-create", "/Groups/vault"])?;
    dscl([".", "-create", "/Groups/vault", "RealName", "Automic Vault"])?;
    dscl([
        ".",
        "-create",
        "/Groups/vault",
        "PrimaryGroupID",
        &gid.to_string(),
    ])?;
    Ok(gid)
}

fn ensure_user(gid: u32) -> Result<u32, String> {
    if let Some(uid) = automic_uid() {
        let got_gid = dscl_read("/Users/automic", "PrimaryGroupID")?
            .and_then(|value| value.parse::<u32>().ok());
        let home = dscl_read("/Users/automic", "NFSHomeDirectory")?;
        let shell = dscl_read("/Users/automic", "UserShell")?;
        if got_gid != Some(gid)
            || home.as_deref() != Some("/opt/homebrew/var/automic")
            || shell.as_deref() != Some("/usr/bin/false")
        {
            return Err("existing automic user is not compatible".to_string());
        }
        return Ok(uid);
    }

    let uid = first_unused_id("/Users", "UniqueID")?;
    dscl([".", "-create", "/Users/automic"])?;
    dscl([
        ".",
        "-create",
        "/Users/automic",
        "RealName",
        "Automic Vault Homebrew",
    ])?;
    dscl([
        ".",
        "-create",
        "/Users/automic",
        "UserShell",
        "/usr/bin/false",
    ])?;
    dscl([
        ".",
        "-create",
        "/Users/automic",
        "NFSHomeDirectory",
        "/opt/homebrew/var/automic",
    ])?;
    dscl([
        ".",
        "-create",
        "/Users/automic",
        "UniqueID",
        &uid.to_string(),
    ])?;
    dscl([
        ".",
        "-create",
        "/Users/automic",
        "PrimaryGroupID",
        &gid.to_string(),
    ])?;
    dscl([".", "-create", "/Users/automic", "Password", "*"])?;
    Ok(uid)
}

fn first_unused_id(record_type: &str, attribute: &str) -> Result<u32, String> {
    let output = Command::new("/usr/bin/dscl")
        .args([".", "-list", record_type, attribute])
        .output()
        .map_err(|err| format!("failed to run dscl: {err}"))?;
    if !output.status.success() {
        return Err(format!(
            "failed to list {record_type} ids: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let used = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| line.split_whitespace().last()?.parse::<u32>().ok())
        .collect::<std::collections::BTreeSet<_>>();
    ID_RANGE
        .clone()
        .find(|id| !used.contains(id))
        .ok_or_else(|| "no free local user/group ids in 550-599".to_string())
}

fn dscl<const N: usize>(args: [&str; N]) -> Result<(), String> {
    let output = Command::new("/usr/bin/dscl")
        .args(args)
        .output()
        .map_err(|err| format!("failed to run dscl: {err}"))?;
    if output.status.success() {
        return Ok(());
    }
    Err(format!(
        "dscl failed: {}",
        String::from_utf8_lossy(&output.stderr).trim()
    ))
}

fn dscl_read(record: &str, attribute: &str) -> Result<Option<String>, String> {
    let output = Command::new("/usr/bin/dscl")
        .args([".", "-read", record, attribute])
        .output()
        .map_err(|err| format!("failed to run dscl: {err}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("eDSRecordNotFound") || stderr.contains("No such key") {
            return Ok(None);
        }
        return Err(format!("dscl failed: {}", stderr.trim()));
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout
        .split_once(':')
        .map(|(_, value)| value.trim().to_string())
        .filter(|value| !value.is_empty()))
}

fn chown_recursive(prefix: &Path) -> Result<(), String> {
    let output = Command::new("/usr/sbin/chown")
        .args(["-R", "-h", "automic:vault"])
        .arg(prefix)
        .output()
        .map_err(|err| format!("failed to run chown: {err}"))?;
    if output.status.success() {
        return Ok(());
    }
    Err(format!(
        "chown failed: {}",
        String::from_utf8_lossy(&output.stderr).trim()
    ))
}

fn install_stub(source: &Path, stub: &Path, uid: u32, gid: u32) -> Result<(), String> {
    if let Some(parent) = stub.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("failed to create {}: {err}", parent.display()))?;
    }
    fs::copy(source, stub).map_err(|err| {
        format!(
            "failed to copy {} to {}: {err}",
            source.display(),
            stub.display()
        )
    })?;
    chown(stub, uid, gid)?;
    fs::set_permissions(stub, fs::Permissions::from_mode(0o6755))
        .map_err(|err| format!("failed to chmod {}: {err}", stub.display()))
}

fn chown(path: &Path, uid: u32, gid: u32) -> Result<(), String> {
    let path = CString::new(path.as_os_str().as_encoded_bytes())
        .map_err(|_| "path contains NUL byte".to_string())?;
    if unsafe { libc::chown(path.as_ptr(), uid, gid) } == 0 {
        return Ok(());
    }
    Err(format!(
        "failed to chown stub: {}",
        std::io::Error::last_os_error()
    ))
}

fn is_hardened_stub(path: &Path, uid: u32, gid: u32) -> bool {
    let Ok(metadata) = fs::metadata(path) else {
        return false;
    };
    metadata.uid() == uid
        && metadata.gid() == gid
        && metadata.mode() & 0o7777 == 0o6755
        && is_managed_stub_file(path)
}

fn stub_matches_source(path: &Path) -> bool {
    let Ok(source) = brew_stub_source_path() else {
        return false;
    };
    fs::read(path)
        .ok()
        .zip(fs::read(source).ok())
        .is_some_and(|(installed, source)| installed == source)
}

fn is_managed_stub_file(path: &Path) -> bool {
    fs::read(path)
        .map(|bytes| {
            bytes
                .windows(STUB_MARKER.len())
                .any(|window| window == STUB_MARKER)
        })
        .unwrap_or(false)
}

fn automic_uid() -> Option<u32> {
    test_u32("AUTOMIC_VAULT_TEST_AUTOMIC_UID").or_else(|| {
        dscl_read("/Users/automic", "UniqueID")
            .ok()
            .flatten()?
            .parse()
            .ok()
    })
}

fn vault_gid() -> Option<u32> {
    test_u32("AUTOMIC_VAULT_TEST_VAULT_GID").or_else(|| {
        dscl_read("/Groups/vault", "PrimaryGroupID")
            .ok()
            .flatten()?
            .parse()
            .ok()
    })
}

fn effective_uid() -> u32 {
    test_u32("AUTOMIC_VAULT_TEST_EUID").unwrap_or_else(|| unsafe { geteuid() })
}

fn test_u32(name: &str) -> Option<u32> {
    std::env::var(name).ok()?.parse().ok()
}

fn brew_prefix() -> PathBuf {
    std::env::var_os("AUTOMIC_VAULT_TEST_BREW_PREFIX")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(BREW_PREFIX))
}

fn brew_target_path() -> PathBuf {
    std::env::var_os("AUTOMIC_VAULT_TEST_BREW_TARGET")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(BREW_TARGET))
}

fn brew_stub_path() -> PathBuf {
    std::env::var_os("AUTOMIC_VAULT_TEST_BREW_STUB")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(BREW_STUB))
}

fn brew_stub_source_path() -> Result<PathBuf, String> {
    if let Some(path) = std::env::var_os("AUTOMIC_VAULT_TEST_BREW_STUB_SOURCE") {
        return Ok(PathBuf::from(path));
    }
    let app_stub = PathBuf::from(APP_BREW_STUB);
    if app_stub.exists() {
        return Ok(app_stub);
    }
    let exe = std::env::current_exe().map_err(|err| format!("failed to locate av: {err}"))?;
    Ok(exe
        .parent()
        .ok_or_else(|| "failed to locate av directory".to_string())?
        .join("av-brew-stub"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn detects_stub_marker_owner_group_and_mode() {
        let _guard = crate::global_test_env_lock().lock().unwrap();
        let path = temp_path("brew-stub-detect");
        fs::write(&path, STUB_MARKER).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o6755)).unwrap();
        unsafe {
            std::env::set_var("AUTOMIC_VAULT_TEST_BREW_STUB", &path);
            std::env::set_var("AUTOMIC_VAULT_TEST_BREW_STUB_SOURCE", &path);
            std::env::set_var("AUTOMIC_VAULT_TEST_BREW_TARGET", "/tmp/brew-target");
            std::env::set_var("AUTOMIC_VAULT_TEST_AUTOMIC_UID", libc::getuid().to_string());
            std::env::set_var("AUTOMIC_VAULT_TEST_VAULT_GID", libc::getgid().to_string());
        }

        let detection = detect();

        unsafe {
            std::env::remove_var("AUTOMIC_VAULT_TEST_BREW_STUB");
            std::env::remove_var("AUTOMIC_VAULT_TEST_BREW_STUB_SOURCE");
            std::env::remove_var("AUTOMIC_VAULT_TEST_BREW_TARGET");
            std::env::remove_var("AUTOMIC_VAULT_TEST_AUTOMIC_UID");
            std::env::remove_var("AUTOMIC_VAULT_TEST_VAULT_GID");
        }
        assert!(detection.hardened);
        assert_eq!(detection.stub_path, Some(path.display().to_string()));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn diagnoses_homebrew_state_directories() {
        let prefix = temp_path("brew-doctor-state");
        let home = prefix.join("var/automic");
        let tmp = home.join("tmp");
        let cache = home.join("cache");
        fs::create_dir_all(&tmp).unwrap();
        fs::create_dir_all(&cache).unwrap();
        let metadata = home.metadata().unwrap();

        assert!(state_directory_diagnostics(&prefix, metadata.uid(), metadata.gid()).is_empty());

        fs::remove_dir(&cache).unwrap();
        let diagnostics = state_directory_diagnostics(&prefix, metadata.uid(), metadata.gid());
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].kind, "state_directory_missing");
        assert!(diagnostics[0].remediation.contains("sudo install -d"));

        fs::create_dir(&cache).unwrap();
        fs::set_permissions(&tmp, fs::Permissions::from_mode(0o600)).unwrap();
        let diagnostics = state_directory_diagnostics(&prefix, metadata.uid(), metadata.gid());
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].kind, "state_directory_permissions_invalid");
        assert!(diagnostics[0].message.contains("mode 0o0600"));

        let _ = fs::remove_dir_all(prefix);
    }

    #[test]
    fn stub_validation_requires_the_exact_bundled_binary() {
        let _guard = crate::global_test_env_lock().lock().unwrap();
        let source = temp_path("brew-stub-source");
        let installed = temp_path("brew-stub-installed");
        fs::write(&source, [STUB_MARKER, b" source"].concat()).unwrap();
        fs::copy(&source, &installed).unwrap();
        unsafe {
            std::env::set_var("AUTOMIC_VAULT_TEST_BREW_STUB_SOURCE", &source);
        }
        assert!(stub_matches_source(&installed));

        fs::write(&installed, [STUB_MARKER, b" modified"].concat()).unwrap();
        assert!(is_managed_stub_file(&installed));
        assert!(!stub_matches_source(&installed));

        unsafe {
            std::env::remove_var("AUTOMIC_VAULT_TEST_BREW_STUB_SOURCE");
        }
        let _ = fs::remove_file(source);
        let _ = fs::remove_file(installed);
    }

    #[test]
    fn managed_outdated_stub_remains_hardened() {
        let path = temp_path("brew-stub-outdated");
        fs::write(&path, [STUB_MARKER, b" old"].concat()).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o6755)).unwrap();
        let metadata = path.metadata().unwrap();

        assert!(is_hardened_stub(&path, metadata.uid(), metadata.gid()));

        let _ = fs::remove_file(path);
    }

    #[test]
    fn run_requires_root() {
        let _guard = crate::global_test_env_lock().lock().unwrap();
        let dir = temp_path("brew-root");
        let target = dir.join("bin/brew");
        let source = dir.join("av-brew-stub");
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::write(&target, "").unwrap();
        fs::write(&source, STUB_MARKER).unwrap();
        unsafe {
            std::env::set_var("AUTOMIC_VAULT_TEST_BREW_PREFIX", &dir);
            std::env::set_var("AUTOMIC_VAULT_TEST_BREW_TARGET", &target);
            std::env::set_var("AUTOMIC_VAULT_TEST_BREW_STUB_SOURCE", &source);
            std::env::set_var("AUTOMIC_VAULT_TEST_EUID", "501");
        }

        let err = run(&mut Vec::new(), true).unwrap_err();

        unsafe {
            std::env::remove_var("AUTOMIC_VAULT_TEST_BREW_PREFIX");
            std::env::remove_var("AUTOMIC_VAULT_TEST_BREW_TARGET");
            std::env::remove_var("AUTOMIC_VAULT_TEST_BREW_STUB_SOURCE");
            std::env::remove_var("AUTOMIC_VAULT_TEST_EUID");
        }
        assert_eq!(err, "run `sudo av harden brew`");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn missing_homebrew_reports_target_path() {
        let _guard = crate::global_test_env_lock().lock().unwrap();
        let missing = temp_path("missing-brew");
        unsafe {
            std::env::set_var("AUTOMIC_VAULT_TEST_BREW_TARGET", &missing);
            std::env::set_var("AUTOMIC_VAULT_TEST_EUID", "0");
        }

        let err = run(&mut Vec::new(), true).unwrap_err();

        unsafe {
            std::env::remove_var("AUTOMIC_VAULT_TEST_BREW_TARGET");
            std::env::remove_var("AUTOMIC_VAULT_TEST_EUID");
        }
        assert_eq!(
            err,
            format!("Homebrew is not installed at {}", missing.display())
        );
    }

    #[test]
    fn test_stub_source_override_wins() {
        let _guard = crate::global_test_env_lock().lock().unwrap();
        let source = temp_path("brew-source");
        unsafe {
            std::env::set_var("AUTOMIC_VAULT_TEST_BREW_STUB_SOURCE", &source);
        }

        let got = brew_stub_source_path().unwrap();

        unsafe {
            std::env::remove_var("AUTOMIC_VAULT_TEST_BREW_STUB_SOURCE");
        }
        assert_eq!(got, source);
    }

    fn temp_path(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("av-{label}-{}-{nanos}", std::process::id()))
    }
}
