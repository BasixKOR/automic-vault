use std::ffi::OsStr;
use std::fs;
use std::io::Write;
use std::os::unix::fs::{MetadataExt, PermissionsExt, symlink};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const INSTALL_ROOT: &str = "/Applications/Automic Vault";
const COMMAND_ROOT: &str = "/usr/local/bin";
const IDENTIFIER_PREFIX: &str = "com.automicvault.launcher-bundle.";

pub(crate) fn install(
    source: &Path,
    bundle_name: &str,
    command_name: &str,
    generation: &str,
) -> Result<(), String> {
    require_privileged()?;
    validate_names(bundle_name, command_name, generation)?;
    validate_source(source, generation)?;
    let root = install_root();
    let commands = command_root();
    prepare_directory(&root)?;
    prepare_directory(&commands)?;

    let final_path = bundle_path(&root, bundle_name);
    let backup = backup_path(&root, generation);
    let transaction = transaction_path(&root, generation);
    let command = commands.join(command_name);
    let runner = runner_path(&final_path);
    guard_command_available(&command, &runner)?;
    if backup.exists() || transaction.exists() {
        return Err("a Launcher Bundle installation transaction already exists".into());
    }

    let source_metadata = fs::symlink_metadata(source)
        .map_err(|error| format!("cannot inspect {}: {error}", source.display()))?;
    write_transaction(&transaction, source_metadata.uid(), source_metadata.gid())?;
    let had_previous = final_path.exists();
    if had_previous {
        validate_protected_tree(&final_path)?;
        fs::rename(&final_path, &backup).map_err(|error| {
            format!(
                "failed to preserve {} before replacement: {error}",
                final_path.display()
            )
        })?;
    }

    let installed = (|| {
        fs::rename(source, &final_path).map_err(|error| {
            format!(
                "failed to install {} at {}: {error}",
                source.display(),
                final_path.display()
            )
        })?;
        protect_tree(&final_path)?;
        validate_installed_bundle(&final_path, command_name, generation)?;
        if fs::symlink_metadata(&command).is_err() {
            symlink(&runner, &command).map_err(|error| {
                format!(
                    "failed to install Launcher Bundle command {}: {error}",
                    command.display()
                )
            })?;
        }
        guard_command_available(&command, &runner)
    })();
    if let Err(error) = installed {
        let _ = remove_tree(&final_path);
        if had_previous {
            let _ = fs::rename(&backup, &final_path);
        }
        if !had_previous && link_matches(&command, &runner) {
            let _ = fs::remove_file(&command);
        }
        let _ = fs::remove_file(&transaction);
        return Err(error);
    }
    Ok(())
}

pub(crate) fn rollback(
    bundle_name: &str,
    command_name: &str,
    generation: &str,
) -> Result<(), String> {
    require_privileged()?;
    validate_names(bundle_name, command_name, generation)?;
    let root = install_root();
    let final_path = bundle_path(&root, bundle_name);
    let backup = backup_path(&root, generation);
    let transaction = transaction_path(&root, generation);
    read_transaction(&transaction)?;
    if final_path.exists() {
        validate_generation(&final_path, generation)?;
        remove_tree(&final_path)?;
    }
    if backup.exists() {
        fs::rename(&backup, &final_path)
            .map_err(|error| format!("failed to restore {}: {error}", final_path.display()))?;
    } else {
        let command = command_root().join(command_name);
        let runner = runner_path(&final_path);
        if link_matches(&command, &runner) {
            fs::remove_file(&command)
                .map_err(|error| format!("failed to remove {}: {error}", command.display()))?;
        }
    }
    fs::remove_file(&transaction)
        .map_err(|error| format!("failed to remove installation transaction: {error}"))
}

pub(crate) fn finish(bundle_name: &str, generation: &str, trash: &Path) -> Result<(), String> {
    require_privileged()?;
    validate_bundle_name(bundle_name)?;
    validate_generation_text(generation)?;
    let root = install_root();
    let final_path = bundle_path(&root, bundle_name);
    validate_generation(&final_path, generation)?;
    let transaction = transaction_path(&root, generation);
    let (uid, gid) = read_transaction(&transaction)?;
    let backup = backup_path(&root, generation);
    if backup.exists() {
        validate_trash(trash, uid)?;
        let destination = trash.join(format!("{bundle_name} {generation}.app"));
        if destination.exists() {
            return Err(format!(
                "Trash destination already exists: {}",
                destination.display()
            ));
        }
        fs::rename(&backup, &destination)
            .map_err(|error| format!("failed to move the old Launcher Bundle to Trash: {error}"))?;
        chown_tree(&destination, uid, gid)?;
    }
    fs::remove_file(&transaction)
        .map_err(|error| format!("failed to finish installation transaction: {error}"))
}

pub(crate) fn remove(
    bundle_name: &str,
    command_name: &str,
    generation: &str,
    trash: &Path,
) -> Result<(), String> {
    require_privileged()?;
    validate_names(bundle_name, command_name, generation)?;
    let root = install_root();
    let final_path = bundle_path(&root, bundle_name);
    validate_generation(&final_path, generation)?;
    let command = command_root().join(command_name);
    let runner = runner_path(&final_path);
    if fs::symlink_metadata(&command).is_ok() && !link_matches(&command, &runner) {
        return Err(format!(
            "refusing to remove unrelated command {}",
            command.display()
        ));
    }
    let trash_metadata = validate_trash(trash, u32::MAX)?;
    let destination = trash.join(format!("{bundle_name} {generation}.app"));
    if destination.exists() {
        return Err(format!(
            "Trash destination already exists: {}",
            destination.display()
        ));
    }
    fs::rename(&final_path, &destination)
        .map_err(|error| format!("failed to move Launcher Bundle to Trash: {error}"))?;
    if link_matches(&command, &runner) {
        fs::remove_file(&command)
            .map_err(|error| format!("failed to remove {}: {error}", command.display()))?;
    }
    chown_tree(&destination, trash_metadata.uid(), trash_metadata.gid())?;
    Ok(())
}

fn install_root() -> PathBuf {
    crate::test_env_var("AUTOMIC_VAULT_TEST_LAUNCHER_BUNDLE_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(INSTALL_ROOT))
}

fn command_root() -> PathBuf {
    crate::test_env_var("AUTOMIC_VAULT_TEST_LAUNCHER_BUNDLE_BIN")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(COMMAND_ROOT))
}

fn require_privileged() -> Result<(), String> {
    if crate::test_env_var("AUTOMIC_VAULT_TEST_LAUNCHER_BUNDLE_ROOT").is_none()
        && unsafe { libc::geteuid() } != 0
    {
        return Err("Launcher Bundle installation requires administrator authorization".into());
    }
    Ok(())
}

fn required_uid() -> u32 {
    if crate::test_env_var("AUTOMIC_VAULT_TEST_LAUNCHER_BUNDLE_ROOT").is_some() {
        unsafe { libc::geteuid() }
    } else {
        0
    }
}

fn required_gid() -> u32 {
    if crate::test_env_var("AUTOMIC_VAULT_TEST_LAUNCHER_BUNDLE_ROOT").is_some() {
        unsafe { libc::getegid() }
    } else {
        0
    }
}

fn prepare_directory(path: &Path) -> Result<(), String> {
    if !path.exists() {
        fs::create_dir(path)
            .map_err(|error| format!("failed to create {}: {error}", path.display()))?;
        fs::set_permissions(path, fs::Permissions::from_mode(0o755))
            .map_err(|error| format!("failed to protect {}: {error}", path.display()))?;
    }
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| format!("cannot inspect {}: {error}", path.display()))?;
    if !metadata.is_dir()
        || metadata.file_type().is_symlink()
        || metadata.uid() != required_uid()
        || metadata.permissions().mode() & 0o022 != 0
    {
        return Err(format!(
            "refusing unsafe installation directory {}",
            path.display()
        ));
    }
    Ok(())
}

fn validate_source(source: &Path, generation: &str) -> Result<(), String> {
    if source.file_name() != Some(OsStr::new("bundle.app"))
        || source
            .parent()
            .and_then(Path::file_name)
            .and_then(OsStr::to_str)
            != Some(&format!(".creating-{generation}"))
    {
        return Err("refusing unexpected Launcher Bundle staging path".into());
    }
    let metadata = fs::symlink_metadata(source)
        .map_err(|error| format!("cannot inspect {}: {error}", source.display()))?;
    let parent = fs::symlink_metadata(source.parent().unwrap())
        .map_err(|error| format!("cannot inspect Launcher Bundle staging directory: {error}"))?;
    if !metadata.is_dir()
        || metadata.file_type().is_symlink()
        || !parent.is_dir()
        || parent.file_type().is_symlink()
        || metadata.uid() == 0
        || parent.uid() != metadata.uid()
        || parent.permissions().mode() & 0o077 != 0
    {
        return Err("refusing unsafe Launcher Bundle staging directory".into());
    }
    Ok(())
}

fn validate_names(bundle_name: &str, command_name: &str, generation: &str) -> Result<(), String> {
    validate_bundle_name(bundle_name)?;
    if !valid_command_name(command_name) {
        return Err("invalid Launcher Bundle command name".into());
    }
    validate_generation_text(generation)
}

pub(crate) fn valid_command_name(command_name: &str) -> bool {
    !command_name.is_empty()
        && command_name.len() <= 80
        && !command_name.starts_with('-')
        && !matches!(command_name, "." | "..")
        && command_name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"._+-".contains(&byte))
}

fn validate_bundle_name(name: &str) -> Result<(), String> {
    if name.is_empty()
        || name.len() > 80
        || matches!(name, "." | "..")
        || name.contains('/')
        || name.chars().any(char::is_control)
    {
        return Err("invalid Launcher Bundle name".into());
    }
    Ok(())
}

fn validate_generation_text(generation: &str) -> Result<(), String> {
    if generation.len() == 36
        && generation.bytes().enumerate().all(|(index, byte)| {
            if matches!(index, 8 | 13 | 18 | 23) {
                byte == b'-'
            } else {
                byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase()
            }
        })
    {
        Ok(())
    } else {
        Err("invalid Launcher Bundle generation".into())
    }
}

fn validate_installed_bundle(
    app: &Path,
    command_name: &str,
    generation: &str,
) -> Result<(), String> {
    validate_protected_tree(app)?;
    validate_generation(app, generation)?;
    let identifier = plist_value(app, "CFBundleIdentifier")?;
    if !identifier.starts_with(IDENTIFIER_PREFIX) {
        return Err("refusing Launcher Bundle without a reserved identity".into());
    }
    if plist_value(app, "AVLauncherBundleCommandName")? != command_name {
        return Err("Launcher Bundle command metadata changed".into());
    }
    if crate::test_env_var("AUTOMIC_VAULT_TEST_LAUNCHER_BUNDLE_ROOT").is_none() {
        let valid = Command::new("/usr/bin/codesign")
            .args(["--verify", "--deep", "--strict", "--all-architectures"])
            .arg(app)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|status| status.success());
        if !valid {
            return Err(
                "installed Launcher Bundle failed strict code-signature verification".into(),
            );
        }
    }
    Ok(())
}

fn validate_generation(app: &Path, generation: &str) -> Result<(), String> {
    if plist_value(app, "AVLauncherBundleGeneration")? == generation {
        Ok(())
    } else {
        Err("Launcher Bundle generation changed".into())
    }
}

fn plist_value(app: &Path, key: &str) -> Result<String, String> {
    let info = app.join("Contents/Info.plist");
    let output = Command::new("/usr/bin/plutil")
        .args(["-extract", key, "raw", "-o", "-"])
        .arg(&info)
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .map_err(|error| format!("failed to inspect {}: {error}", info.display()))?;
    if !output.status.success() {
        return Err(format!("Launcher Bundle metadata is missing {key}"));
    }
    String::from_utf8(output.stdout)
        .map(|value| value.trim().to_string())
        .map_err(|_| format!("Launcher Bundle metadata {key} is not UTF-8"))
}

fn protect_tree(path: &Path) -> Result<(), String> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| format!("cannot inspect {}: {error}", path.display()))?;
    if metadata.file_type().is_symlink() {
        return Err(format!("refusing symbolic link in {}", path.display()));
    }
    if metadata.is_dir() {
        for entry in fs::read_dir(path)
            .map_err(|error| format!("cannot inspect {}: {error}", path.display()))?
        {
            protect_tree(&entry.map_err(|error| error.to_string())?.path())?;
        }
    } else if !metadata.is_file() {
        return Err(format!("refusing special file in {}", path.display()));
    }
    lchown(path, required_uid(), required_gid())?;
    if metadata.permissions().mode() & 0o022 != 0 {
        let mode = metadata.permissions().mode() & !0o022;
        fs::set_permissions(path, fs::Permissions::from_mode(mode))
            .map_err(|error| format!("failed to protect {}: {error}", path.display()))?;
    }
    Ok(())
}

fn validate_protected_tree(path: &Path) -> Result<(), String> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| format!("cannot inspect {}: {error}", path.display()))?;
    if metadata.file_type().is_symlink()
        || metadata.uid() != required_uid()
        || metadata.permissions().mode() & 0o022 != 0
    {
        return Err(format!(
            "refusing unprotected Launcher Bundle entry {}",
            path.display()
        ));
    }
    if metadata.is_dir() {
        for entry in fs::read_dir(path)
            .map_err(|error| format!("cannot inspect {}: {error}", path.display()))?
        {
            validate_protected_tree(&entry.map_err(|error| error.to_string())?.path())?;
        }
    } else if !metadata.is_file() {
        return Err(format!("refusing special file in {}", path.display()));
    }
    Ok(())
}

fn chown_tree(path: &Path, uid: u32, gid: u32) -> Result<(), String> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| format!("cannot inspect {}: {error}", path.display()))?;
    if metadata.file_type().is_symlink() {
        return Err(format!("refusing symbolic link in {}", path.display()));
    }
    if metadata.is_dir() {
        for entry in fs::read_dir(path)
            .map_err(|error| format!("cannot inspect {}: {error}", path.display()))?
        {
            chown_tree(&entry.map_err(|error| error.to_string())?.path(), uid, gid)?;
        }
    }
    lchown(path, uid, gid)
}

fn lchown(path: &Path, uid: u32, gid: u32) -> Result<(), String> {
    let path = std::ffi::CString::new(path.as_os_str().as_encoded_bytes())
        .map_err(|_| "Launcher Bundle path contains NUL")?;
    if unsafe { libc::lchown(path.as_ptr(), uid, gid) } == 0 {
        Ok(())
    } else {
        Err(format!(
            "failed to change ownership: {}",
            std::io::Error::last_os_error()
        ))
    }
}

fn guard_command_available(command: &Path, runner: &Path) -> Result<(), String> {
    match fs::symlink_metadata(command) {
        Ok(_) if link_matches(command, runner) => Ok(()),
        Ok(_) => Err(format!(
            "refusing to replace unrelated command {}",
            command.display()
        )),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!("cannot inspect {}: {error}", command.display())),
    }
}

fn link_matches(command: &Path, runner: &Path) -> bool {
    fs::symlink_metadata(command).is_ok_and(|metadata| {
        metadata.file_type().is_symlink()
            && metadata.uid() == required_uid()
            && fs::read_link(command).ok().as_deref() == Some(runner)
    })
}

fn validate_trash(trash: &Path, expected_uid: u32) -> Result<fs::Metadata, String> {
    let metadata = fs::symlink_metadata(trash)
        .map_err(|error| format!("cannot inspect Trash directory: {error}"))?;
    if trash.file_name() != Some(OsStr::new(".Trash"))
        || !metadata.is_dir()
        || metadata.file_type().is_symlink()
        || metadata.uid() == 0
        || expected_uid != u32::MAX && metadata.uid() != expected_uid
        || metadata.permissions().mode() & 0o077 != 0
    {
        return Err("refusing unsafe Trash directory".into());
    }
    Ok(metadata)
}

fn write_transaction(path: &Path, uid: u32, gid: u32) -> Result<(), String> {
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| format!("failed to create installation transaction: {error}"))?;
    file.write_all(format!("{uid}:{gid}\n").as_bytes())
        .map_err(|error| format!("failed to write installation transaction: {error}"))?;
    fs::set_permissions(path, fs::Permissions::from_mode(0o600))
        .map_err(|error| format!("failed to protect installation transaction: {error}"))
}

fn read_transaction(path: &Path) -> Result<(u32, u32), String> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| format!("installation transaction is unavailable: {error}"))?;
    if !metadata.is_file()
        || metadata.file_type().is_symlink()
        || metadata.uid() != required_uid()
        || metadata.permissions().mode() & 0o077 != 0
    {
        return Err("refusing unsafe installation transaction".into());
    }
    let text = fs::read_to_string(path)
        .map_err(|error| format!("cannot read installation transaction: {error}"))?;
    let (uid, gid) = text
        .trim()
        .split_once(':')
        .ok_or("invalid installation transaction")?;
    Ok((
        uid.parse().map_err(|_| "invalid installation user")?,
        gid.parse().map_err(|_| "invalid installation group")?,
    ))
}

fn bundle_path(root: &Path, name: &str) -> PathBuf {
    root.join(format!("{name}.app"))
}

fn runner_path(app: &Path) -> PathBuf {
    app.join("Contents/MacOS/launcher")
}

fn backup_path(root: &Path, generation: &str) -> PathBuf {
    root.join(format!(".replaced-{generation}.app"))
}

fn transaction_path(root: &Path, generation: &str) -> PathBuf {
    root.join(format!(".transaction-{generation}"))
}

fn remove_tree(path: &Path) -> Result<(), String> {
    if fs::symlink_metadata(path).is_ok() {
        fs::remove_dir_all(path)
            .map_err(|error| format!("failed to remove {}: {error}", path.display()))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    struct TestInstall {
        base: PathBuf,
        root: PathBuf,
        commands: PathBuf,
        trash: PathBuf,
    }

    impl TestInstall {
        fn new(label: &str) -> Self {
            let base = std::env::temp_dir().join(format!(
                "av-launcher-bundle-{label}-{}-{}",
                std::process::id(),
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            let root = base.join("Applications/Automic Vault");
            let commands = base.join("bin");
            let trash = base.join("home/.Trash");
            fs::create_dir_all(root.parent().unwrap()).unwrap();
            fs::create_dir_all(trash.parent().unwrap()).unwrap();
            fs::create_dir(&trash).unwrap();
            fs::set_permissions(&trash, fs::Permissions::from_mode(0o700)).unwrap();
            unsafe {
                std::env::set_var("AUTOMIC_VAULT_TEST_LAUNCHER_BUNDLE_ROOT", &root);
                std::env::set_var("AUTOMIC_VAULT_TEST_LAUNCHER_BUNDLE_BIN", &commands);
            }
            Self {
                base,
                root,
                commands,
                trash,
            }
        }

        fn stage(&self, generation: &str, command: &str) -> PathBuf {
            let work = self.base.join(format!(".creating-{generation}"));
            let app = work.join("bundle.app");
            let contents = app.join("Contents");
            fs::create_dir_all(contents.join("MacOS")).unwrap();
            fs::create_dir_all(contents.join("Resources")).unwrap();
            fs::set_permissions(&work, fs::Permissions::from_mode(0o700)).unwrap();
            fs::write(contents.join("MacOS/launcher"), "runner").unwrap();
            fs::write(contents.join("Resources/payload"), generation).unwrap();
            fs::write(
                contents.join("Info.plist"),
                format!(
                    r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>CFBundleIdentifier</key><string>{IDENTIFIER_PREFIX}{generation}</string>
<key>AVLauncherBundleGeneration</key><string>{generation}</string>
<key>AVLauncherBundleCommandName</key><string>{command}</string>
</dict></plist>"#
                ),
            )
            .unwrap();
            app
        }
    }

    impl Drop for TestInstall {
        fn drop(&mut self) {
            unsafe {
                std::env::remove_var("AUTOMIC_VAULT_TEST_LAUNCHER_BUNDLE_ROOT");
                std::env::remove_var("AUTOMIC_VAULT_TEST_LAUNCHER_BUNDLE_BIN");
            }
            let _ = fs::remove_dir_all(&self.base);
        }
    }

    #[test]
    fn installs_links_and_removes_a_launcher_bundle() {
        let _guard = super::super::ENV_LOCK.lock().unwrap();
        let test = TestInstall::new("lifecycle");
        let generation = "12345678-1234-1234-1234-123456789abc";
        let source = test.stage(generation, "herdr");

        install(&source, "Herdr", "herdr", generation).unwrap();
        let app = test.root.join("Herdr.app");
        let command = test.commands.join("herdr");
        assert_eq!(fs::read_link(&command).unwrap(), runner_path(&app));
        assert_eq!(
            fs::symlink_metadata(&app).unwrap().permissions().mode() & 0o022,
            0
        );
        finish("Herdr", generation, &test.trash).unwrap();

        remove("Herdr", "herdr", generation, &test.trash).unwrap();
        assert!(fs::symlink_metadata(command).is_err());
        assert!(test.trash.join(format!("Herdr {generation}.app")).is_dir());
    }

    #[test]
    fn replacement_can_roll_back_and_command_collisions_are_refused() {
        let _guard = super::super::ENV_LOCK.lock().unwrap();
        let test = TestInstall::new("rollback");
        let first = "12345678-1234-1234-1234-123456789abc";
        let second = "abcdefab-1234-1234-1234-123456789abc";
        install(&test.stage(first, "herdr"), "Herdr", "herdr", first).unwrap();
        finish("Herdr", first, &test.trash).unwrap();

        install(&test.stage(second, "herdr"), "Herdr", "herdr", second).unwrap();
        rollback("Herdr", "herdr", second).unwrap();
        assert_eq!(
            plist_value(&test.root.join("Herdr.app"), "AVLauncherBundleGeneration").unwrap(),
            first
        );

        let occupied = test.commands.join("other");
        fs::write(&occupied, "unrelated").unwrap();
        let source = test.stage(second, "other");
        assert!(
            install(&source, "Other", "other", second)
                .unwrap_err()
                .contains("unrelated command")
        );
        assert!(source.is_dir());
    }
}
