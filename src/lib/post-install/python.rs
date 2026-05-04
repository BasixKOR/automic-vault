use std::fs;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};

use semver::Version;

use super::PostInstallOutcome;

#[derive(Debug, Clone, PartialEq, Eq)]
struct InstalledPython {
    version: Version,
    python_stub: PathBuf,
    pip_stub: PathBuf,
}

pub(super) fn supports(formula: &str) -> bool {
    parse_formula_version(formula).is_some()
}

pub(super) fn post_install(prefix: &Path, bin_dir: &Path) -> Result<PostInstallOutcome, String> {
    let install_root = prefix
        .parent()
        .ok_or_else(|| format!("invalid python prefix {}", prefix.display()))?;
    fs::create_dir_all(bin_dir)
        .map_err(|err| format!("failed to create {}: {err}", bin_dir.display()))?;

    let installed = discover_installed_pythons(install_root, bin_dir)?;
    let latest = installed
        .first()
        .ok_or_else(|| format!("no installed python stubs found under {}", bin_dir.display()))?;
    let major = latest.version.major;

    write_symlink(
        &bin_dir.join(format!("python{major}")),
        &latest.python_stub,
    )?;
    write_symlink(&bin_dir.join(format!("pip{major}")), &latest.pip_stub)?;
    write_symlink(&bin_dir.join("python"), &bin_dir.join(format!("python{major}")))?;
    write_symlink(&bin_dir.join("pip"), &bin_dir.join(format!("pip{major}")))?;

    Ok(PostInstallOutcome {
        managed_stubs: vec![
            "pip".to_string(),
            format!("pip{major}"),
            "python".to_string(),
            format!("python{major}"),
        ],
    })
}

fn discover_installed_pythons(
    install_root: &Path,
    bin_dir: &Path,
) -> Result<Vec<InstalledPython>, String> {
    let entries = fs::read_dir(install_root)
        .map_err(|err| format!("failed to read {}: {err}", install_root.display()))?;
    let mut installed = Vec::new();

    for entry in entries {
        let entry =
            entry.map_err(|err| format!("failed to read {}: {err}", install_root.display()))?;
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| format!("non-utf8 directory name under {}", install_root.display()))?;
        let Some(version) = parse_formula_version(&name) else {
            continue;
        };

        let python_stub = bin_dir.join(format!("python{}.{}", version.major, version.minor));
        let pip_stub = bin_dir.join(format!("pip{}.{}", version.major, version.minor));
        if !stub_exists(&python_stub) || !stub_exists(&pip_stub) {
            continue;
        }

        installed.push(InstalledPython {
            version,
            python_stub,
            pip_stub,
        });
    }

    installed.sort_by(|left, right| right.version.cmp(&left.version));
    Ok(installed)
}

fn parse_formula_version(formula: &str) -> Option<Version> {
    let version = formula.strip_prefix("python@")?;
    let (major, minor) = version.split_once('.')?;
    if minor.contains('.') {
        return None;
    }
    Some(Version::new(major.parse().ok()?, minor.parse().ok()?, 0))
}

fn stub_exists(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok()
}

fn write_symlink(path: &Path, target: &Path) -> Result<(), String> {
    match fs::symlink_metadata(path) {
        Ok(_) => super::super::remove_path(path)?,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
        Err(err) => return Err(format!("failed to stat {}: {err}", path.display())),
    }

    let relative_target = target.file_name().ok_or_else(|| {
        format!(
            "failed to compute relative target for {} from {}",
            target.display(),
            path.display()
        )
    })?;

    symlink(relative_target, path).map_err(|err| {
        format!(
            "failed to symlink {} to {}: {err}",
            path.display(),
            relative_target.to_string_lossy()
        )
    })
}
