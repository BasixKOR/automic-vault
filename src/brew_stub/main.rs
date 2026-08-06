use std::ffi::{CStr, CString, OsString};
use std::fs;
use std::io::{self, Read, Write};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt, PermissionsExt};
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

const MARKER: &str = "AUTOMIC_VAULT_BREW_STUB_V13";
const TARGET: &str = "/opt/homebrew/bin/brew";
const PREFIX: &str = "/opt/homebrew";
const APPROVAL_SERVICE: &str = "com.automicvault.av2.approval";
const BREW_USER_UID: &str = "/opt/homebrew/var/automic/user-uid";
const ZSH_COMPLETIONS: &str = "share/zsh/site-functions";
const ZSH_COMPLETION_MIRROR: &str = ".local/share/automic-vault/homebrew/zsh/site-functions";
const MAX_COMPLETION_BYTES: usize = 8 * 1024 * 1024;
const MAX_COMPLETIONS_BYTES: usize = 64 * 1024 * 1024;
const FORBIDDEN_CASK_ARTIFACTS: &str = "app appimage artifact audiounitplugin bashcompletion colorpicker commandwrapper dictionary fishcompletion font generatedscript inputmethod installer internetplugin keyboardlayout manpage mdimporter pkg postflight postflightblock postflightsteps preflight preflightblock preflightsteps prefpane qlplugin screensaver service stageonly suite uninstall uninstallpostflightsteps uninstallpreflightsteps vst3plugin vstplugin zshcompletion";

#[derive(Debug, PartialEq, Eq)]
struct AuthorizationRequest {
    target: String,
    args: Vec<String>,
    cwd: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Caller {
    uid: u32,
    gid: u32,
    home: PathBuf,
    shell: PathBuf,
}

#[derive(Debug, PartialEq, Eq)]
struct CaskMutation {
    command: String,
    names: Vec<String>,
}

fn main() {
    if std::env::args().any(|arg| arg == "--automic-vault-brew-stub-marker") {
        println!("{MARKER}");
        return;
    }

    let args = std::env::args_os().skip(1).collect::<Vec<_>>();
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let mut command = approved_command(args, std::env::vars_os(), &cwd, xpc_authorize)
        .unwrap_or_else(|err| fail(err));
    let caller = caller().unwrap_or_else(|err| fail(err));
    let automic_uid = unsafe { libc::geteuid() };
    let zsh_shellenv = is_zsh_shellenv(command.get_args(), &caller.shell);
    let output = zsh_shellenv.then(|| {
        command
            .output()
            .unwrap_or_else(|err| fail(format!("failed to run {TARGET}: {err}")))
    });
    let status = output.as_ref().map_or_else(
        || {
            command
                .status()
                .unwrap_or_else(|err| fail(format!("failed to run {TARGET}: {err}")))
        },
        |output| output.status,
    );
    if let Some(output) = &output {
        io::stderr().write_all(&output.stderr).ok();
    }
    if !status.success() {
        if let Err(err) = drop_to_caller(&caller).and_then(|()| {
            sync_zsh_completion_mirror(
                Path::new(PREFIX),
                &caller.home,
                &caller.home.join(ZSH_COMPLETION_MIRROR),
                automic_uid,
            )
        }) {
            eprintln!("av-brew-stub: {err}");
        }
        if let Some(output) = output {
            io::stdout().write_all(&output.stdout).ok();
        }
        std::process::exit(status.code().unwrap_or(1));
    }
    drop_to_caller(&caller).unwrap_or_else(|err| fail(err));
    let completion_result = sync_zsh_completion_mirror(
        Path::new(PREFIX),
        &caller.home,
        &caller.home.join(ZSH_COMPLETION_MIRROR),
        automic_uid,
    );
    if zsh_shellenv {
        completion_result.unwrap_or_else(|err| fail(err));
        if let Some(output) = output {
            io::stdout().write_all(&output.stdout).ok();
        }
        io::stdout()
            .write_all(zsh_shellenv_override().as_bytes())
            .ok();
    } else if let Err(err) = completion_result {
        eprintln!("av-brew-stub: failed to refresh zsh completions: {err}");
    }
}

fn fail(message: String) -> ! {
    eprintln!("av-brew-stub: {message}");
    std::process::exit(1);
}

fn approved_command<I, F>(
    args: Vec<OsString>,
    source_env: I,
    cwd: &Path,
    approve: F,
) -> Result<Command, String>
where
    I: IntoIterator<Item = (OsString, OsString)>,
    F: FnOnce(&AuthorizationRequest) -> Result<(), String>,
{
    let source_env = source_env.into_iter().collect::<Vec<_>>();
    let mut request = authorization_request(&args, cwd)?;
    let (args, cask) = governed_args(&request.args)?;
    request.args = args;
    approve(&request)?;
    if let Some(cask) = &cask {
        validate_cask_mutation(cask, cwd)?;
    }
    let mut command = Command::new(TARGET);
    command
        .args(request.args)
        .env_clear()
        .envs(stub_env(source_env));
    if cask.is_some() {
        command.env("HOMEBREW_NO_AUTO_UPDATE", "1");
    }
    unsafe {
        command.pre_exec(drop_to_effective_identity);
    }
    Ok(command)
}

fn drop_to_effective_identity() -> io::Result<()> {
    let uid = unsafe { libc::geteuid() };
    let gid = unsafe { libc::getegid() };
    if unsafe { libc::setregid(gid, gid) } != 0 || unsafe { libc::setreuid(uid, uid) } != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

fn drop_to_caller(caller: &Caller) -> Result<(), String> {
    if unsafe { libc::setregid(caller.gid, caller.gid) } != 0
        || unsafe { libc::setreuid(caller.uid, caller.uid) } != 0
    {
        return Err(format!(
            "failed to drop privileges for zsh completion refresh: {}",
            io::Error::last_os_error()
        ));
    }
    if unsafe { libc::getuid() } != caller.uid
        || unsafe { libc::geteuid() } != caller.uid
        || unsafe { libc::getgid() } != caller.gid
        || unsafe { libc::getegid() } != caller.gid
    {
        return Err("failed to drop all privileges for zsh completion refresh".into());
    }
    Ok(())
}

fn governed_args(args: &[String]) -> Result<(Vec<String>, Option<CaskMutation>), String> {
    let Some((command_index, command)) = mutation_command(args) else {
        return Ok((args.to_vec(), None));
    };
    if command == "bundle" {
        return Err("`brew bundle` is unavailable because Brewfiles may contain casks; run formula commands directly".into());
    }
    let cask = args
        .iter()
        .any(|arg| matches!(arg.as_str(), "--cask" | "--casks"));
    let formula = args
        .iter()
        .any(|arg| matches!(arg.as_str(), "--formula" | "--formulae"));
    if cask && formula {
        return Err("brew command cannot select both formulae and casks".into());
    }
    if cask {
        let names = cask_operands(args, command_index)?;
        if names.is_empty() {
            return Err("CLI-only cask mutations must name each cask explicitly".into());
        }
        return Ok((
            args.to_vec(),
            Some(CaskMutation {
                command: command.to_string(),
                names,
            }),
        ));
    }

    let mut args = args.to_vec();
    if !formula {
        args.insert(command_index + 1, "--formula".into());
    }
    Ok((args, None))
}

fn cask_operands(args: &[String], command_index: usize) -> Result<Vec<String>, String> {
    const ALLOWED_FLAGS: &[&str] = &[
        "--cask",
        "--casks",
        "--debug",
        "--display-times",
        "--dry-run",
        "--force",
        "--greedy",
        "--greedy-auto-updates",
        "--greedy-latest",
        "--quiet",
        "--verbose",
    ];
    for flag in &args[..command_index] {
        if !matches!(flag.as_str(), "--debug" | "--quiet" | "--verbose") {
            return Err(format!(
                "unsupported option `{flag}` for a CLI-only cask mutation"
            ));
        }
    }
    let mut names = Vec::new();
    for arg in &args[command_index + 1..] {
        if arg == "--" {
            continue;
        }
        if arg.starts_with('-') {
            if !ALLOWED_FLAGS.contains(&arg.as_str()) {
                return Err(format!(
                    "unsupported option `{arg}` for a CLI-only cask mutation"
                ));
            }
            continue;
        }
        if !safe_cask_name(arg) {
            return Err(format!("unsupported cask name `{arg}`"));
        }
        names.push(arg.clone());
    }
    Ok(names)
}

fn safe_cask_name(name: &str) -> bool {
    let parts = name.split('/').collect::<Vec<_>>();
    matches!(parts.len(), 1 | 3)
        && parts.iter().all(|part| {
            !part.is_empty()
                && !part.starts_with('.')
                && part.bytes().all(|byte| {
                    byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'+' | b'@')
                })
        })
}

fn validate_cask_mutation(cask: &CaskMutation, cwd: &Path) -> Result<(), String> {
    let installs = matches!(cask.command.as_str(), "install" | "reinstall" | "upgrade");
    let removes = matches!(
        cask.command.as_str(),
        "reinstall" | "upgrade" | "uninstall" | "remove" | "rm"
    );
    for name in &cask.names {
        if installs {
            let info = cask_info(name, cwd)?;
            av::brew_cask_policy::validate_info_cask(name, &info)?;
        }
        if removes {
            let receipt = installed_cask_receipt(name)?;
            av::brew_cask_policy::validate_install_receipt(name, &receipt)?;
        }
    }
    Ok(())
}

fn cask_info(name: &str, cwd: &Path) -> Result<serde_json::Value, String> {
    let output = brew_output(&["info", "--json=v2", "--cask", "--", name], cwd)?;
    let mut info: serde_json::Value = serde_json::from_slice(&output.stdout)
        .map_err(|err| format!("Homebrew returned malformed JSON: {err}"))?;
    let casks = info["casks"]
        .as_array_mut()
        .ok_or_else(|| format!("Homebrew returned malformed cask metadata for `{name}`"))?;
    if casks.len() != 1 {
        return Err(format!(
            "Homebrew returned ambiguous cask metadata for `{name}`"
        ));
    }
    Ok(casks.remove(0))
}

fn brew_output(args: &[&str], cwd: &Path) -> Result<std::process::Output, String> {
    let mut command = Command::new(TARGET);
    command
        .args(args)
        .current_dir(cwd)
        .env_clear()
        .envs(stub_env([]))
        .env("HOMEBREW_NO_AUTO_UPDATE", "1");
    unsafe {
        command.pre_exec(drop_to_effective_identity);
    }
    let output = command
        .output()
        .map_err(|err| format!("failed to inspect cask metadata: {err}"))?;
    if output.status.success() {
        return Ok(output);
    }
    Err(format!(
        "Homebrew cask inspection failed: {}",
        String::from_utf8_lossy(&output.stderr).trim()
    ))
}

fn installed_cask_receipt(name: &str) -> Result<serde_json::Value, String> {
    let token = name.rsplit('/').next().unwrap_or(name);
    let cask = Path::new(PREFIX).join("Caskroom").join(token);
    let metadata = fs::symlink_metadata(&cask)
        .map_err(|err| format!("failed to inspect installed cask `{name}`: {err}"))?;
    if !metadata.file_type().is_dir() {
        return Err(format!(
            "installed cask `{name}` is not a protected directory"
        ));
    }
    let path = cask.join(".metadata/INSTALL_RECEIPT.json");
    let mut file = fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW)
        .open(&path)
        .map_err(|err| format!("failed to read installed cask `{name}` receipt: {err}"))?;
    let metadata = file
        .metadata()
        .map_err(|err| format!("failed to inspect installed cask `{name}` receipt: {err}"))?;
    if !metadata.file_type().is_file()
        || metadata.uid() != unsafe { libc::geteuid() }
        || metadata.mode() & 0o022 != 0
        || metadata.len() > 1024 * 1024
    {
        return Err(format!("installed cask `{name}` receipt is not protected"));
    }
    let mut contents = Vec::with_capacity(metadata.len() as usize);
    file.read_to_end(&mut contents)
        .map_err(|err| format!("failed to read installed cask `{name}` receipt: {err}"))?;
    serde_json::from_slice(&contents)
        .map_err(|err| format!("installed cask `{name}` receipt is malformed: {err}"))
}

fn mutation_command(args: &[String]) -> Option<(usize, &str)> {
    let index = args
        .iter()
        .position(|arg| arg == "--" || !arg.starts_with('-'))?;
    let command = args[index].as_str();
    matches!(
        command,
        "install" | "reinstall" | "upgrade" | "uninstall" | "remove" | "rm" | "bundle"
    )
    .then_some((index, command))
}

fn caller() -> Result<Caller, String> {
    let uid = unsafe { libc::getuid() };
    let gid = unsafe { libc::getgid() };
    let euid = unsafe { libc::geteuid() };
    validate_invoker(uid, euid)?;
    let configured =
        configured_user_uid(Path::new(BREW_USER_UID), euid, unsafe { libc::getegid() })?;
    if uid != configured {
        return Err(
            "brew must be invoked directly by the user configured by `sudo av harden brew`".into(),
        );
    }
    let entry = unsafe { libc::getpwuid(uid) };
    if entry.is_null() {
        return Err(format!("cannot resolve local account for UID {uid}"));
    }
    let entry = unsafe { &*entry };
    if entry.pw_gid != gid {
        return Err("caller's real GID does not match the account primary group".into());
    }
    if entry.pw_name.is_null()
        || unsafe { CStr::from_ptr(entry.pw_name) }
            .to_bytes()
            .is_empty()
    {
        return Err("caller's account name is missing".into());
    }
    if entry.pw_dir.is_null() || entry.pw_shell.is_null() {
        return Err("caller's account home or shell is missing".into());
    }
    let home = PathBuf::from(std::ffi::OsStr::from_bytes(
        unsafe { CStr::from_ptr(entry.pw_dir) }.to_bytes(),
    ));
    let shell = PathBuf::from(std::ffi::OsStr::from_bytes(
        unsafe { CStr::from_ptr(entry.pw_shell) }.to_bytes(),
    ));
    if !home.is_absolute() || !shell.is_absolute() {
        return Err("caller's account home and shell must be absolute paths".into());
    }
    Ok(Caller {
        uid,
        gid,
        home,
        shell,
    })
}

fn validate_invoker(uid: u32, euid: u32) -> Result<(), String> {
    if uid == 0 {
        return Err("brew cannot be invoked as root".into());
    }
    if uid == euid {
        return Err("brew stub is not installed setuid; run `sudo av harden brew`".into());
    }
    Ok(())
}

fn configured_user_uid(path: &Path, owner: u32, group: u32) -> Result<u32, String> {
    let mut file = fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW)
        .open(path)
        .map_err(|err| format!("failed to read configured Homebrew user: {err}"))?;
    let metadata = file
        .metadata()
        .map_err(|err| format!("failed to inspect configured Homebrew user: {err}"))?;
    if !metadata.file_type().is_file()
        || metadata.uid() != owner
        || metadata.gid() != group
        || metadata.mode() & 0o022 != 0
    {
        return Err("configured Homebrew user file is not protected".into());
    }
    let mut configured = String::new();
    file.read_to_string(&mut configured)
        .map_err(|err| format!("failed to read configured Homebrew user: {err}"))?;
    configured
        .trim()
        .parse::<u32>()
        .map_err(|_| "configured Homebrew user UID is invalid".to_string())
}

fn sync_zsh_completion_mirror(
    prefix: &Path,
    home: &Path,
    mirror: &Path,
    trusted_uid: u32,
) -> Result<(), String> {
    let functions = prefix.join(ZSH_COMPLETIONS);
    let canonical_prefix = fs::canonicalize(prefix)
        .map_err(|err| format!("failed to resolve {}: {err}", prefix.display()))?;
    let mut completions = std::collections::BTreeMap::new();
    let mut total = 0usize;
    if functions.exists() {
        let metadata = fs::symlink_metadata(&functions)
            .map_err(|err| format!("failed to inspect {}: {err}", functions.display()))?;
        if !metadata.file_type().is_dir()
            || (metadata.uid() != 0 && metadata.uid() != trusted_uid)
            || metadata.mode() & 0o022 != 0
        {
            return Err(format!(
                "Homebrew zsh completion directory is not protected: {}",
                functions.display()
            ));
        }
        for entry in fs::read_dir(&functions)
            .map_err(|err| format!("failed to read {}: {err}", functions.display()))?
        {
            let entry =
                entry.map_err(|err| format!("failed to read {}: {err}", functions.display()))?;
            let name = entry.file_name();
            if !name.as_encoded_bytes().starts_with(b"_") {
                continue;
            }
            let target = match fs::canonicalize(entry.path()) {
                Ok(target) => target,
                Err(err)
                    if err.kind() == io::ErrorKind::NotFound
                        && entry.file_type().is_ok_and(|kind| kind.is_symlink()) =>
                {
                    continue;
                }
                Err(err) => {
                    return Err(format!(
                        "failed to resolve zsh completion {}: {err}",
                        entry.path().display()
                    ));
                }
            };
            if !target.starts_with(&canonical_prefix) {
                eprintln!(
                    "av-brew-stub: skipping zsh completion {} (resolves to {}) because it resolves outside {}",
                    entry.path().display(),
                    target.display(),
                    prefix.display()
                );
                continue;
            }
            let mut file = fs::OpenOptions::new()
                .read(true)
                .custom_flags(libc::O_NOFOLLOW)
                .open(&target)
                .map_err(|err| format!("failed to open {}: {err}", target.display()))?;
            let metadata = file
                .metadata()
                .map_err(|err| format!("failed to inspect {}: {err}", target.display()))?;
            if !metadata.file_type().is_file()
                || (metadata.uid() != 0 && metadata.uid() != trusted_uid)
                || metadata.mode() & 0o022 != 0
                || metadata.len() > MAX_COMPLETION_BYTES as u64
            {
                return Err(format!(
                    "Homebrew zsh completion is not a protected regular file: {}",
                    target.display()
                ));
            }
            let mut contents = Vec::with_capacity(metadata.len() as usize);
            file.read_to_end(&mut contents)
                .map_err(|err| format!("failed to read {}: {err}", target.display()))?;
            if contents.len() > MAX_COMPLETION_BYTES {
                return Err(format!(
                    "Homebrew zsh completion exceeds the 8 MiB mirror limit: {}",
                    target.display()
                ));
            }
            total = total
                .checked_add(contents.len())
                .filter(|total| *total <= MAX_COMPLETIONS_BYTES)
                .ok_or_else(|| {
                    "Homebrew zsh completions exceed the 64 MiB mirror limit".to_string()
                })?;
            completions.insert(name, contents);
        }
    }

    let parent = prepare_completion_mirror_parent(home, mirror)?;
    if let Ok(metadata) = fs::symlink_metadata(mirror)
        && (!metadata.file_type().is_dir()
            || metadata.uid() != unsafe { libc::geteuid() }
            || metadata.mode() & 0o022 != 0)
    {
        return Err(format!(
            "completion mirror is not protected: {}",
            mirror.display()
        ));
    }
    if completion_mirror_matches(mirror, &completions)? {
        return Ok(());
    }
    let staging = parent.join(format!(
        ".site-functions-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|err| format!("system clock is invalid: {err}"))?
            .as_nanos()
    ));
    fs::create_dir(&staging)
        .map_err(|err| format!("failed to create completion snapshot: {err}"))?;
    fs::set_permissions(&staging, fs::Permissions::from_mode(0o700))
        .map_err(|err| format!("failed to protect completion snapshot: {err}"))?;
    let result = (|| {
        for (name, contents) in &completions {
            let path = staging.join(name);
            fs::write(&path, contents)
                .map_err(|err| format!("failed to write {}: {err}", path.display()))?;
            fs::set_permissions(&path, fs::Permissions::from_mode(0o600))
                .map_err(|err| format!("failed to protect {}: {err}", path.display()))?;
        }
        replace_completion_mirror(&staging, mirror)
    })();
    if result.is_err() {
        let _ = fs::remove_dir_all(&staging);
    }
    result
}

fn prepare_completion_mirror_parent(home: &Path, mirror: &Path) -> Result<PathBuf, String> {
    let parent = mirror
        .parent()
        .ok_or_else(|| format!("completion mirror has no parent: {}", mirror.display()))?;
    let relative = parent.strip_prefix(home).map_err(|_| {
        format!(
            "completion mirror is outside the caller's home: {}",
            mirror.display()
        )
    })?;
    let mut builder = fs::DirBuilder::new();
    builder.recursive(true).mode(0o700);
    builder
        .create(parent)
        .map_err(|err| format!("failed to create {}: {err}", parent.display()))?;

    let uid = unsafe { libc::geteuid() };
    let mut path = home.to_path_buf();
    for component in std::iter::once(None).chain(relative.components().map(Some)) {
        if let Some(component) = component {
            let std::path::Component::Normal(component) = component else {
                return Err(format!(
                    "completion mirror path is invalid: {}",
                    mirror.display()
                ));
            };
            path.push(component);
        }
        let metadata = fs::symlink_metadata(&path)
            .map_err(|err| format!("failed to inspect {}: {err}", path.display()))?;
        if !metadata.file_type().is_dir() || metadata.uid() != uid || metadata.mode() & 0o022 != 0 {
            return Err(format!(
                "completion mirror path is not protected: {}",
                path.display()
            ));
        }
    }
    Ok(parent.to_path_buf())
}

fn completion_mirror_matches(
    mirror: &Path,
    completions: &std::collections::BTreeMap<OsString, Vec<u8>>,
) -> Result<bool, String> {
    let metadata = match fs::symlink_metadata(mirror) {
        Ok(metadata) => metadata,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(false),
        Err(err) => return Err(format!("failed to inspect {}: {err}", mirror.display())),
    };
    if !metadata.file_type().is_dir()
        || metadata.uid() != unsafe { libc::geteuid() }
        || metadata.mode() & 0o022 != 0
    {
        return Ok(false);
    }
    let entries = fs::read_dir(mirror)
        .map_err(|err| format!("failed to read {}: {err}", mirror.display()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|err| format!("failed to read {}: {err}", mirror.display()))?;
    if entries.len() != completions.len() {
        return Ok(false);
    }
    for entry in entries {
        let Some(expected) = completions.get(&entry.file_name()) else {
            return Ok(false);
        };
        let metadata = fs::symlink_metadata(entry.path())
            .map_err(|err| format!("failed to inspect {}: {err}", entry.path().display()))?;
        if !metadata.file_type().is_file()
            || metadata.uid() != unsafe { libc::geteuid() }
            || metadata.mode() & 0o022 != 0
            || fs::read(entry.path()).map_err(|err| format!("failed to read mirror: {err}"))?
                != *expected
        {
            return Ok(false);
        }
    }
    Ok(true)
}

fn replace_completion_mirror(staging: &Path, mirror: &Path) -> Result<(), String> {
    if !mirror.exists() {
        return fs::rename(staging, mirror)
            .map_err(|err| format!("failed to publish completion mirror: {err}"));
    }
    #[cfg(target_os = "macos")]
    {
        let staging_c = CString::new(staging.as_os_str().as_bytes())
            .map_err(|_| "completion snapshot path contains NUL".to_string())?;
        let mirror_c = CString::new(mirror.as_os_str().as_bytes())
            .map_err(|_| "completion mirror path contains NUL".to_string())?;
        if unsafe {
            libc::renameatx_np(
                libc::AT_FDCWD,
                staging_c.as_ptr(),
                libc::AT_FDCWD,
                mirror_c.as_ptr(),
                libc::RENAME_SWAP,
            )
        } != 0
        {
            return Err(format!(
                "failed to publish completion mirror: {}",
                io::Error::last_os_error()
            ));
        }
        fs::remove_dir_all(staging)
            .map_err(|err| format!("failed to remove old completion mirror: {err}"))?;
        return Ok(());
    }
    #[cfg(not(target_os = "macos"))]
    {
        let old = mirror.with_extension("old");
        let _ = fs::remove_dir_all(&old);
        fs::rename(mirror, &old)
            .map_err(|err| format!("failed to retire completion mirror: {err}"))?;
        if let Err(err) = fs::rename(staging, mirror) {
            let _ = fs::rename(&old, mirror);
            return Err(format!("failed to publish completion mirror: {err}"));
        }
        fs::remove_dir_all(old)
            .map_err(|err| format!("failed to remove old completion mirror: {err}"))
    }
}

fn is_zsh_shellenv<'a>(args: impl IntoIterator<Item = &'a std::ffi::OsStr>, shell: &Path) -> bool {
    let args = args.into_iter().collect::<Vec<_>>();
    let Some(command_index) = args
        .iter()
        .position(|arg| arg.as_bytes() == b"--" || !arg.as_bytes().starts_with(b"-"))
    else {
        return false;
    };
    if args[command_index].as_bytes() != b"shellenv" {
        return false;
    }
    args[command_index + 1..]
        .iter()
        .find(|arg| !arg.as_bytes().starts_with(b"-"))
        .map_or_else(
            || {
                shell
                    .file_name()
                    .is_some_and(|shell| shell.as_bytes() == b"zsh")
            },
            |shell| shell.as_bytes() == b"zsh",
        )
}

fn zsh_shellenv_override() -> &'static str {
    "fpath=(\"$HOME/.local/share/automic-vault/homebrew/zsh/site-functions\" ${fpath:#/opt/homebrew/share/zsh/site-functions});\nexport FPATH;\n"
}

fn authorization_request(args: &[OsString], cwd: &Path) -> Result<AuthorizationRequest, String> {
    let args = args
        .iter()
        .map(|arg| {
            arg.to_str()
                .map(str::to_string)
                .ok_or_else(|| "brew arguments must be valid UTF-8".to_string())
        })
        .collect::<Result<Vec<_>, _>>()?;
    let cwd = cwd
        .to_str()
        .ok_or_else(|| "current directory must be valid UTF-8".to_string())?;
    Ok(AuthorizationRequest {
        target: TARGET.to_string(),
        args,
        cwd: cwd.to_string(),
    })
}

#[cfg(target_os = "macos")]
fn xpc_authorize(request: &AuthorizationRequest) -> Result<(), String> {
    use std::os::raw::{c_char, c_int, c_void};

    type XpcObject = *mut c_void;

    unsafe extern "C" {
        static _xpc_type_error: u8;
        static _xpc_error_key_description: *const c_char;

        fn xpc_connection_create_mach_service(
            name: *const c_char,
            targetq: *mut c_void,
            flags: u64,
        ) -> XpcObject;
        fn xpc_connection_activate(connection: XpcObject);
        fn xpc_connection_cancel(connection: XpcObject);
        fn xpc_connection_send_message_with_reply_sync(
            connection: XpcObject,
            message: XpcObject,
        ) -> XpcObject;
        fn xpc_dictionary_create_empty() -> XpcObject;
        fn xpc_dictionary_set_bool(xdict: XpcObject, key: *const c_char, value: bool);
        fn xpc_dictionary_get_bool(xdict: XpcObject, key: *const c_char) -> bool;
        fn xpc_dictionary_set_string(xdict: XpcObject, key: *const c_char, value: *const c_char);
        fn xpc_dictionary_get_string(xdict: XpcObject, key: *const c_char) -> *const c_char;
        fn xpc_dictionary_set_value(xdict: XpcObject, key: *const c_char, value: XpcObject);
        fn xpc_array_create_empty() -> XpcObject;
        fn xpc_array_append_value(xarray: XpcObject, value: XpcObject);
        fn xpc_string_create(string: *const c_char) -> XpcObject;
        fn xpc_get_type(object: XpcObject) -> *const c_void;
        fn xpc_release(object: XpcObject);
        fn xpc_connection_set_peer_code_signing_requirement(
            connection: XpcObject,
            requirement: *const c_char,
        ) -> c_int;
        fn av_xpc_connection_set_empty_event_handler(connection: XpcObject);
    }

    unsafe fn set_string(dict: XpcObject, key: &[u8], value: &str) -> Result<(), String> {
        let value = CString::new(value).map_err(|_| "XPC field contains NUL".to_string())?;
        unsafe { xpc_dictionary_set_string(dict, key.as_ptr().cast(), value.as_ptr()) };
        Ok(())
    }

    unsafe fn string_array(values: &[String]) -> Result<XpcObject, String> {
        let array = unsafe { xpc_array_create_empty() };
        if array.is_null() {
            return Err("failed to create approval XPC array".into());
        }
        for value in values {
            let value =
                CString::new(value.as_str()).map_err(|_| "XPC array contains NUL".to_string())?;
            let string = unsafe { xpc_string_create(value.as_ptr()) };
            unsafe {
                xpc_array_append_value(array, string);
                xpc_release(string);
            }
        }
        Ok(array)
    }

    let service = CString::new(APPROVAL_SERVICE).unwrap();
    let connection =
        unsafe { xpc_connection_create_mach_service(service.as_ptr(), std::ptr::null_mut(), 0) };
    if connection.is_null() {
        return Err("failed to create approval XPC connection".into());
    }

    let menu_requirement = CString::new(av::MENU_HELPER_CODE_SIGNING_REQUIREMENT).unwrap();
    if unsafe {
        xpc_connection_set_peer_code_signing_requirement(connection, menu_requirement.as_ptr())
    } != 0
    {
        unsafe { xpc_release(connection) };
        return Err("failed to configure approval XPC signing requirement".into());
    }

    unsafe {
        av_xpc_connection_set_empty_event_handler(connection);
        xpc_connection_activate(connection);
    }

    let message = unsafe { xpc_dictionary_create_empty() };
    if message.is_null() {
        unsafe {
            xpc_connection_cancel(connection);
            xpc_release(connection);
        }
        return Err("failed to create approval XPC message".into());
    }

    let empty = unsafe { string_array(&[]) }?;
    let args = unsafe { string_array(&request.args) }?;
    unsafe {
        set_string(message, b"op\0", "authorize")?;
        set_string(message, b"target\0", &request.target)?;
        set_string(message, b"cwd\0", &request.cwd)?;
        set_string(message, b"tool\0", "brew")?;
        xpc_dictionary_set_bool(message, b"replace_existing_env\0".as_ptr().cast(), false);
        xpc_dictionary_set_bool(message, b"allow_missing_keys\0".as_ptr().cast(), false);
        xpc_dictionary_set_value(message, b"keys\0".as_ptr().cast(), empty);
        xpc_dictionary_set_value(message, b"args\0".as_ptr().cast(), args);
        xpc_dictionary_set_value(message, b"env_conflicts\0".as_ptr().cast(), empty);
        xpc_release(empty);
        xpc_release(args);
    }

    let reply = unsafe { xpc_connection_send_message_with_reply_sync(connection, message) };
    unsafe {
        xpc_release(message);
        xpc_connection_cancel(connection);
        xpc_release(connection);
    }
    if reply.is_null() {
        return Err("Automic Vault approval did not reply".into());
    }

    let result = unsafe {
        if xpc_get_type(reply) == std::ptr::addr_of!(_xpc_type_error).cast() {
            let error = xpc_dictionary_get_string(reply, _xpc_error_key_description);
            let error = if error.is_null() {
                "approval XPC connection failed".into()
            } else {
                std::ffi::CStr::from_ptr(error)
                    .to_string_lossy()
                    .into_owned()
            };
            if error == "Connection invalid" {
                Err("Automic Vault approval service is not running; open the menu bar app".into())
            } else {
                Err(error)
            }
        } else if xpc_dictionary_get_bool(reply, b"ok\0".as_ptr().cast()) {
            Ok(())
        } else {
            let error = xpc_dictionary_get_string(reply, b"error\0".as_ptr().cast());
            Err(if error.is_null() {
                "brew authorization denied".into()
            } else {
                std::ffi::CStr::from_ptr(error)
                    .to_string_lossy()
                    .into_owned()
            })
        }
    };
    unsafe { xpc_release(reply) };
    result
}

#[cfg(not(target_os = "macos"))]
fn xpc_authorize(_request: &AuthorizationRequest) -> Result<(), String> {
    Err("menu bar approval is only available on macOS".into())
}

fn stub_env<I>(source: I) -> Vec<(OsString, OsString)>
where
    I: IntoIterator<Item = (OsString, OsString)>,
{
    let mut env = vec![
        (
            "PATH".into(),
            "/opt/homebrew/bin:/opt/homebrew/sbin:/usr/bin:/bin:/usr/sbin:/sbin".into(),
        ),
        ("AUTOMIC_VAULT_BREW_STUB".into(), MARKER.into()),
        ("HOME".into(), "/opt/homebrew/var/automic".into()),
        ("USER".into(), "automic".into()),
        ("LOGNAME".into(), "automic".into()),
        ("TMPDIR".into(), "/opt/homebrew/var/automic/tmp".into()),
        (
            "HOMEBREW_CACHE".into(),
            "/opt/homebrew/var/automic/cache".into(),
        ),
        ("HOMEBREW_FORBID_PACKAGES_FROM_PATHS".into(), "1".into()),
        (
            "HOMEBREW_FORBIDDEN_CASK_ARTIFACTS".into(),
            FORBIDDEN_CASK_ARTIFACTS.into(),
        ),
        ("HOMEBREW_FORBIDDEN_OWNER".into(), "Automic Vault".into()),
    ];

    for (key, value) in source {
        let Some(key_str) = key.to_str() else {
            continue;
        };
        if key_str == "TERM"
            || key_str == "LANG"
            || key_str == "NO_COLOR"
            || key_str.starts_with("LC_")
        {
            env.push((key, value));
        }
    }
    env
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn authorization_request_keeps_exact_args_and_cwd() {
        let request = authorization_request(
            &[
                "install".into(),
                "--cask".into(),
                "Visual Studio Code".into(),
            ],
            Path::new("/tmp/a project"),
        )
        .unwrap();

        assert_eq!(request.target, TARGET);
        assert_eq!(request.args, ["install", "--cask", "Visual Studio Code"]);
        assert_eq!(request.cwd, "/tmp/a project");
    }

    #[test]
    fn denial_prevents_command_creation() {
        let result = approved_command(
            vec!["install".into(), "ack".into()],
            [],
            Path::new("/tmp"),
            |_| Err("denied".into()),
        );

        assert_eq!(result.unwrap_err().to_string(), "denied");
    }

    #[test]
    fn approval_sees_the_formula_pinned_command() {
        let command = approved_command(
            vec!["install".into(), "tree".into()],
            [],
            Path::new("/tmp"),
            |request| {
                assert_eq!(request.args, ["install", "--formula", "tree"]);
                Ok(())
            },
        )
        .unwrap();

        assert_eq!(
            command
                .get_args()
                .map(|arg| arg.to_string_lossy().into_owned())
                .collect::<Vec<_>>(),
            ["install", "--formula", "tree"]
        );
    }

    #[test]
    fn child_identity_is_normalized() {
        drop_to_effective_identity().unwrap();
        assert_eq!(unsafe { libc::getuid() }, unsafe { libc::geteuid() });
        assert_eq!(unsafe { libc::getgid() }, unsafe { libc::getegid() });
    }

    #[test]
    fn approved_command_has_sanitized_env() {
        let command = approved_command(
            vec!["info".into(), "ack".into()],
            [
                ("TERM".into(), "xterm-256color".into()),
                ("SECRET".into(), "nope".into()),
            ],
            Path::new("/tmp"),
            |_| Ok(()),
        )
        .unwrap();
        let env = command
            .get_envs()
            .map(|(key, value)| (key.to_owned(), value.unwrap().to_owned()))
            .collect::<Vec<_>>();

        assert!(env.contains(&("HOME".into(), "/opt/homebrew/var/automic".into())));
        assert!(env.contains(&("TERM".into(), "xterm-256color".into())));
        assert!(!env.iter().any(|(key, _)| key == "SECRET"));
    }

    #[test]
    fn stub_env_keeps_only_safe_user_env() {
        let env = stub_env([
            ("TERM".into(), "xterm-256color".into()),
            ("LANG".into(), "en_US.UTF-8".into()),
            ("LC_ALL".into(), "C".into()),
            ("NO_COLOR".into(), "1".into()),
            ("HOMEBREW_PREFIX".into(), "/tmp/bad".into()),
            ("PATH".into(), "/tmp/bad".into()),
        ]);

        assert!(env.contains(&("HOME".into(), "/opt/homebrew/var/automic".into())));
        assert!(env.contains(&("USER".into(), "automic".into())));
        assert!(env.contains(&("LOGNAME".into(), "automic".into())));
        assert!(env.contains(&("TERM".into(), "xterm-256color".into())));
        assert!(env.contains(&("LC_ALL".into(), "C".into())));
        assert!(!env.contains(&("HOMEBREW_PREFIX".into(), "/tmp/bad".into())));
        assert!(!env.contains(&("PATH".into(), "/tmp/bad".into())));
    }

    #[test]
    fn mutations_are_pinned_to_formulae() {
        assert_eq!(
            governed_args(&["install".into(), "tree".into()]).unwrap(),
            (
                vec!["install".into(), "--formula".into(), "tree".into()],
                None
            )
        );
        assert_eq!(
            governed_args(&["upgrade".into(), "--formula".into()]).unwrap(),
            (vec!["upgrade".into(), "--formula".into()], None)
        );
        assert_eq!(
            mutation_command(&[
                "--verbose".into(),
                "install".into(),
                "--cask".into(),
                "firefox".into(),
            ]),
            Some((1, "install"))
        );
        assert_eq!(mutation_command(&["info".into(), "install".into()]), None);
        assert_eq!(mutation_command(&["--".into(), "install".into()]), None);
    }

    #[test]
    fn cli_cask_mutations_are_explicit_and_restricted() {
        assert_eq!(
            governed_args(&["install".into(), "--cask".into(), "codex".into()]).unwrap(),
            (
                vec!["install".into(), "--cask".into(), "codex".into()],
                Some(CaskMutation {
                    command: "install".into(),
                    names: vec!["codex".into()]
                })
            )
        );
        for args in [
            vec!["upgrade".into(), "--cask".into()],
            vec![
                "uninstall".into(),
                "--cask".into(),
                "--zap".into(),
                "codex".into(),
            ],
            vec!["install".into(), "--cask".into(), "./codex.rb".into()],
            vec![
                "install".into(),
                "--cask".into(),
                "--formula".into(),
                "codex".into(),
            ],
        ] {
            assert!(governed_args(&args).is_err());
        }
        assert!(
            governed_args(&["bundle".into()])
                .unwrap_err()
                .contains("Brewfiles may contain casks")
        );
        assert_eq!(
            governed_args(&["list".into(), "--cask".into()]).unwrap(),
            (vec!["list".into(), "--cask".into()], None)
        );
    }

    #[test]
    fn configured_user_must_come_from_a_protected_file() {
        use std::os::unix::fs::PermissionsExt;

        let root = temp_path("user");
        fs::create_dir_all(&root).unwrap();
        let path = root.join("uid");
        fs::write(&path, "501\n").unwrap();
        let metadata = fs::metadata(&path).unwrap();

        assert_eq!(
            configured_user_uid(&path, metadata.uid(), metadata.gid()),
            Ok(501)
        );
        fs::set_permissions(&path, fs::Permissions::from_mode(0o666)).unwrap();
        assert!(configured_user_uid(&path, metadata.uid(), metadata.gid()).is_err());

        let link = root.join("link");
        std::os::unix::fs::symlink(&path, &link).unwrap();
        assert!(configured_user_uid(&link, metadata.uid(), metadata.gid()).is_err());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn invoker_errors_identify_root_and_missing_setuid() {
        assert_eq!(
            validate_invoker(0, 550).unwrap_err(),
            "brew cannot be invoked as root"
        );
        assert_eq!(
            validate_invoker(501, 501).unwrap_err(),
            "brew stub is not installed setuid; run `sudo av harden brew`"
        );
        assert!(validate_invoker(501, 550).is_ok());
    }

    fn temp_path(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("av-brew-stub-{label}-{nanos}"))
    }

    #[test]
    fn zsh_completion_mirror_copies_only_in_prefix_regular_files() {
        use std::os::unix::fs::PermissionsExt;

        let prefix = temp_path("zsh-completion-source");
        let home = temp_path("zsh-completion-home");
        let mirror = home.join(ZSH_COMPLETION_MIRROR);
        let functions = prefix.join(ZSH_COMPLETIONS);
        let target = prefix.join("Cellar/tool/1/share/zsh/site-functions/_tool");
        fs::create_dir(&home).unwrap();
        fs::create_dir_all(&functions).unwrap();
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::write(&target, "#compdef tool").unwrap();
        std::os::unix::fs::symlink(&target, functions.join("_tool")).unwrap();
        fs::write(functions.join("not-loaded"), "ignored").unwrap();

        sync_zsh_completion_mirror(
            &prefix,
            &home,
            &mirror,
            fs::metadata(&target).unwrap().uid(),
        )
        .unwrap();

        let mirrored = fs::symlink_metadata(mirror.join("_tool")).unwrap();
        assert!(mirrored.file_type().is_file());
        assert_eq!(
            fs::read_to_string(mirror.join("_tool")).unwrap(),
            "#compdef tool"
        );
        assert_eq!(mirrored.permissions().mode() & 0o777, 0o600);
        assert!(!mirror.join("not-loaded").exists());

        let unsafe_parent = home.join(".local/share");
        fs::set_permissions(&unsafe_parent, fs::Permissions::from_mode(0o777)).unwrap();
        assert!(
            sync_zsh_completion_mirror(
                &prefix,
                &home,
                &mirror,
                fs::metadata(&target).unwrap().uid(),
            )
            .is_err()
        );
        assert_eq!(
            fs::read_to_string(mirror.join("_tool")).unwrap(),
            "#compdef tool"
        );
        fs::set_permissions(&unsafe_parent, fs::Permissions::from_mode(0o700)).unwrap();

        fs::remove_file(mirror.join("_tool")).unwrap();
        std::os::unix::fs::symlink(&target, mirror.join("_tool")).unwrap();
        sync_zsh_completion_mirror(
            &prefix,
            &home,
            &mirror,
            fs::metadata(&target).unwrap().uid(),
        )
        .unwrap();
        assert!(
            fs::symlink_metadata(mirror.join("_tool"))
                .unwrap()
                .file_type()
                .is_file()
        );

        fs::set_permissions(&target, fs::Permissions::from_mode(0o664)).unwrap();
        assert!(
            sync_zsh_completion_mirror(
                &prefix,
                &home,
                &mirror,
                fs::metadata(&target).unwrap().uid(),
            )
            .is_err()
        );

        fs::remove_dir_all(prefix).unwrap();
        fs::remove_dir_all(home).unwrap();
    }

    #[test]
    fn zsh_completion_mirror_omits_escape_and_publishes_safe_files() {
        let prefix = temp_path("zsh-completion-escape");
        let home = temp_path("zsh-completion-escape-home");
        let mirror = home.join(ZSH_COMPLETION_MIRROR);
        let outside = temp_path("zsh-completion-outside");
        let functions = prefix.join(ZSH_COMPLETIONS);
        fs::create_dir_all(&functions).unwrap();
        fs::create_dir_all(&mirror).unwrap();
        fs::write(mirror.join("_existing"), "safe").unwrap();
        fs::write(functions.join("_protected"), "#compdef protected").unwrap();
        fs::write(&outside, "unsafe").unwrap();
        std::os::unix::fs::symlink(&outside, functions.join("_escape")).unwrap();

        sync_zsh_completion_mirror(
            &prefix,
            &home,
            &mirror,
            fs::metadata(&outside).unwrap().uid(),
        )
        .unwrap();
        assert_eq!(
            fs::read_to_string(mirror.join("_protected")).unwrap(),
            "#compdef protected"
        );
        assert!(!mirror.join("_escape").exists());
        assert!(!mirror.join("_existing").exists());

        fs::remove_dir_all(prefix).unwrap();
        fs::remove_dir_all(home).unwrap();
        fs::remove_file(outside).unwrap();
    }

    #[test]
    fn shellenv_override_uses_only_the_mirror() {
        let output = zsh_shellenv_override();

        assert!(output.contains("$HOME/.local/share/automic-vault/homebrew/zsh/site-functions"));
        assert!(output.contains("${fpath:#/opt/homebrew/share/zsh/site-functions}"));
        assert!(is_zsh_shellenv(
            [std::ffi::OsStr::new("shellenv")],
            Path::new("/bin/zsh")
        ));
        assert!(is_zsh_shellenv(
            [
                std::ffi::OsStr::new("shellenv"),
                std::ffi::OsStr::new("zsh")
            ],
            Path::new("/bin/bash")
        ));
        assert!(!is_zsh_shellenv(
            [
                std::ffi::OsStr::new("shellenv"),
                std::ffi::OsStr::new("fish")
            ],
            Path::new("/bin/zsh")
        ));
    }
}
