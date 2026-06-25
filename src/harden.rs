use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

const STUB_DIR: &str = "/usr/local/bin";
const TARGET_SUFFIX: &str = ".av-target";

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

    if !Path::new("/usr/local/bin/av").exists() {
        return Err("/usr/local/bin/av is not installed".to_string());
    }

    let stub = stub_path(target)?;
    let sidecar = target_path_sidecar(&stub);
    if stub.exists() && !sidecar.exists() {
        return Err(format!(
            "{} already exists and is not an av hardened stub",
            stub.display()
        ));
    }

    fs::create_dir_all(STUB_DIR).map_err(|err| format!("failed to create {STUB_DIR}: {err}"))?;
    if stub.exists() {
        fs::remove_file(&stub)
            .map_err(|err| format!("failed to replace {}: {err}", stub.display()))?;
    }
    fs::write(&stub, stub_script(target)?)
        .map_err(|err| format!("failed to install stub at {}: {err}", stub.display()))?;
    fs::write(&sidecar, format!("{}\n", target.display()))
        .map_err(|err| format!("failed to write {}: {err}", sidecar.display()))?;
    fs::set_permissions(&stub, fs::Permissions::from_mode(0o755))
        .map_err(|err| format!("failed to chmod {}: {err}", stub.display()))?;
    fs::set_permissions(&sidecar, fs::Permissions::from_mode(0o644))
        .map_err(|err| format!("failed to chmod {}: {err}", sidecar.display()))?;

    Ok(format!(
        "hardened {} with stub {}",
        target.display(),
        stub.display()
    ))
}

pub(crate) fn target_path_sidecar(path: &Path) -> PathBuf {
    let mut value = path.as_os_str().to_os_string();
    value.push(TARGET_SUFFIX);
    value.into()
}

fn stub_path(target: &Path) -> Result<PathBuf, String> {
    let name = target
        .file_name()
        .ok_or_else(|| "target path must end in a file name".to_string())?;
    Ok(Path::new(STUB_DIR).join(name))
}

fn stub_script(target: &Path) -> Result<String, String> {
    let name = target
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| "target path must end in a UTF-8 file name".to_string())?;
    Ok(format!(
        "#!/bin/sh\nexec /usr/local/bin/av stub-exec '{}' \"$@\"\n",
        name.replace('\'', "'\\''")
    ))
}
