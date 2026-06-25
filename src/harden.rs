use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

unsafe extern "C" {
    fn geteuid() -> u32;
}

pub(crate) fn run(target: &Path) -> Result<String, String> {
    if unsafe { geteuid() } != 0 {
        return Err("must be run as root, use: sudo av harden PATH".to_string());
    }
    if !target.is_absolute() {
        return Err("target path must be absolute".to_string());
    }
    if !target.exists() {
        return Err(format!("{} does not exist", target.display()));
    }

    let original = original_path(target);
    if original.exists() {
        return Ok(format!("{} is already hardened", target.display()));
    }

    let av =
        std::env::current_exe().map_err(|err| format!("failed to locate av executable: {err}"))?;
    fs::rename(target, &original).map_err(|err| {
        format!(
            "failed to move {} to {}: {err}",
            target.display(),
            original.display()
        )
    })?;
    fs::copy(&av, target).map_err(|err| {
        let _ = fs::rename(&original, target);
        format!("failed to install stub at {}: {err}", target.display())
    })?;
    fs::set_permissions(target, fs::Permissions::from_mode(0o755))
        .map_err(|err| format!("failed to chmod {}: {err}", target.display()))?;

    Ok(format!(
        "hardened {} -> {}",
        target.display(),
        original.display()
    ))
}

pub(crate) fn original_path(path: &Path) -> std::path::PathBuf {
    let mut value = path.as_os_str().to_os_string();
    value.push(".av-orig");
    value.into()
}
