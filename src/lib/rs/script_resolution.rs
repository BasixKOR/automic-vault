use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ScriptResolution {
    pub(crate) path: PathBuf,
    pub(crate) interpreter_path: String,
    pub(crate) sha256: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ScriptExecutionScope {
    pub(crate) executable_path: String,
    pub(crate) script_path: Option<String>,
    pub(crate) script_sha256: Option<String>,
}

pub(crate) fn script_execution_scope(
    executable_path: &str,
    resolved_executable_path: &Path,
    file: &File,
    args: &[OsString],
) -> Result<ScriptExecutionScope, String> {
    if let Some(script) = uv_run_script(resolved_executable_path, args)? {
        validate_regular_target(resolved_executable_path, file)?;
        validate_target_root_installation(resolved_executable_path, file)?;
        return Ok(ScriptExecutionScope {
            executable_path: executable_path.to_string(),
            script_path: Some(path_to_display_string(&script.path)?),
            script_sha256: script.sha256,
        });
    }

    if let Some(script) = direct_shebang_script(resolved_executable_path, file)? {
        return Ok(ScriptExecutionScope {
            executable_path: script.interpreter_path,
            script_path: Some(path_to_display_string(&script.path)?),
            script_sha256: script.sha256,
        });
    }

    validate_target_root_installation(resolved_executable_path, file)?;
    let script = interpreter_script(resolved_executable_path, args)?;
    Ok(ScriptExecutionScope {
        executable_path: executable_path.to_string(),
        script_path: script
            .as_ref()
            .map(|script| path_to_display_string(&script.path))
            .transpose()?,
        script_sha256: script.and_then(|script| script.sha256),
    })
}

pub(crate) fn direct_shebang_script(
    script_path: &Path,
    file: &File,
) -> Result<Option<ScriptResolution>, String> {
    let Some(interpreter_path) = shebang_interpreter_path(script_path)? else {
        return Ok(None);
    };
    if executable_file_name(&interpreter_path) == Some("env") {
        return Err("env shebang always-allow is not supported".to_string());
    }

    let interpreter_file = File::open(&interpreter_path).map_err(|err| {
        format!(
            "failed to open shebang interpreter {}: {err}",
            interpreter_path.display()
        )
    })?;
    validate_regular_target(&interpreter_path, &interpreter_file)?;
    validate_target_root_installation(&interpreter_path, &interpreter_file)?;

    let interpreter_path = path_to_display_string(&interpreter_path)?;
    if validate_target_root_installation(script_path, file).is_ok() {
        return Ok(Some(ScriptResolution {
            path: script_path.to_path_buf(),
            interpreter_path,
            sha256: None,
        }));
    }

    Ok(Some(ScriptResolution {
        path: script_path.to_path_buf(),
        interpreter_path,
        sha256: Some(sha256_file(script_path)?),
    }))
}

pub(crate) fn interpreter_script(
    executable_path: &Path,
    args: &[OsString],
) -> Result<Option<ScriptResolution>, String> {
    if executable_file_name(executable_path) == Some("env") {
        return Err("env always-allow is not supported".to_string());
    }
    if executable_file_name(executable_path) == Some("uv") {
        return uv_run_script(executable_path, args);
    }
    if !is_script_interpreter(executable_path) {
        return Ok(None);
    }
    let script_operand = interpreter_script_operand(args)
        .ok_or_else(|| "interpreter always-allow requires a root-owned script file".to_string())?;
    let script_path = resolve_script_operand(script_operand)?;
    let file = File::open(&script_path)
        .map_err(|err| format!("failed to open {}: {err}", script_path.display()))?;
    validate_regular_target(&script_path, &file)?;
    if validate_target_root_installation(&script_path, &file).is_ok() {
        return Ok(Some(ScriptResolution {
            path: script_path,
            interpreter_path: path_to_display_string(executable_path)?,
            sha256: None,
        }));
    }
    let sha256 = sha256_file(&script_path)?;
    Ok(Some(ScriptResolution {
        path: script_path,
        interpreter_path: path_to_display_string(executable_path)?,
        sha256: Some(sha256),
    }))
}

pub(crate) fn script_path_for_display(
    executable_path: &Path,
    scope: Option<&ScriptExecutionScope>,
    args: &[OsString],
) -> Option<PathBuf> {
    if let Some(scope) = scope
        && let Some(script_path) = scope.script_path.as_deref()
    {
        let display_path = PathBuf::from(script_path);
        if display_path == executable_path {
            return Some(display_path);
        }
    }

    interpreter_script_path_for_display(executable_path, args)
}

pub(crate) fn interpreter_script_path_for_display(
    executable_path: &Path,
    args: &[OsString],
) -> Option<PathBuf> {
    if executable_file_name(executable_path) == Some("uv") {
        let script_path = uv_run_script_operand(args)?;
        let path = if script_path.is_absolute() {
            script_path.to_path_buf()
        } else {
            env::current_dir().ok()?.join(script_path)
        };
        return Some(fs::canonicalize(&path).unwrap_or(path));
    }
    if !is_script_interpreter(executable_path)
        || executable_file_name(executable_path) == Some("env")
    {
        return None;
    }
    let script_path = interpreter_script_operand(args)?;
    let path = if script_path.is_absolute() {
        script_path.to_path_buf()
    } else {
        env::current_dir().ok()?.join(script_path)
    };
    Some(fs::canonicalize(&path).unwrap_or(path))
}

pub(crate) fn validate_regular_target(path: &Path, file: &File) -> Result<(), String> {
    let metadata = file
        .metadata()
        .map_err(|err| format!("failed to stat {}: {err}", path.display()))?;
    if !metadata.is_file() {
        return Err("target binary must be a regular file".to_string());
    }
    Ok(())
}

pub(crate) fn validate_target_root_installation(path: &Path, file: &File) -> Result<(), String> {
    let metadata = file
        .metadata()
        .map_err(|err| format!("failed to stat {}: {err}", path.display()))?;
    validate_target_file_metadata(metadata.uid(), metadata.mode())?;
    validate_parent_directories(path)
}

pub(crate) fn validate_root_controlled_path(path: &Path) -> Result<(), String> {
    let file =
        File::open(path).map_err(|err| format!("failed to open {}: {err}", path.display()))?;
    validate_regular_target(path, &file)?;
    validate_target_root_installation(path, &file)
}

pub(crate) fn validate_parent_directories(path: &Path) -> Result<(), String> {
    for directory in path.ancestors().skip(1) {
        let metadata = fs::metadata(directory)
            .map_err(|err| format!("failed to stat {}: {err}", directory.display()))?;
        validate_directory_mode(directory, metadata.mode())?;
    }
    Ok(())
}

pub(crate) fn path_to_display_string(path: &Path) -> Result<String, String> {
    path.to_str()
        .map(str::to_string)
        .ok_or_else(|| "path must be valid UTF-8".to_string())
}

pub(crate) fn executable_file_name(path: &Path) -> Option<&str> {
    path.file_name().and_then(|value| value.to_str())
}

pub(crate) fn is_script_interpreter(path: &Path) -> bool {
    let Some(file_name) = executable_file_name(path) else {
        return false;
    };
    matches!(
        file_name,
        "bash"
            | "dash"
            | "env"
            | "ksh"
            | "node"
            | "osascript"
            | "perl"
            | "python"
            | "python3"
            | "ruby"
            | "sh"
            | "uv"
            | "zsh"
    ) || is_versioned_python_name(file_name)
}

pub(crate) fn interpreter_script_operand(args: &[OsString]) -> Option<&Path> {
    let mut index = 0;
    while index < args.len() {
        let arg = args[index].to_str()?;
        if arg == "--" {
            return args.get(index + 1).map(Path::new);
        }
        if !arg.starts_with('-') || arg == "-" {
            return args.get(index).map(Path::new);
        }
        if interpreter_option_takes_value(arg) {
            index += 2;
        } else {
            index += 1;
        }
    }
    None
}

fn uv_run_script(
    executable_path: &Path,
    args: &[OsString],
) -> Result<Option<ScriptResolution>, String> {
    if executable_file_name(executable_path) != Some("uv") {
        return Ok(None);
    }
    let Some(script_operand) = uv_run_script_operand(args) else {
        return Ok(None);
    };
    let script_path = resolve_script_operand(script_operand)?;
    let file = File::open(&script_path)
        .map_err(|err| format!("failed to open {}: {err}", script_path.display()))?;
    validate_regular_target(&script_path, &file)?;
    if validate_target_root_installation(&script_path, &file).is_ok() {
        return Ok(Some(ScriptResolution {
            path: script_path,
            interpreter_path: path_to_display_string(executable_path)?,
            sha256: None,
        }));
    }
    let sha256 = sha256_file(&script_path)?;
    Ok(Some(ScriptResolution {
        path: script_path,
        interpreter_path: path_to_display_string(executable_path)?,
        sha256: Some(sha256),
    }))
}

fn uv_run_script_operand(args: &[OsString]) -> Option<&Path> {
    let mut index = 0;
    while index < args.len() {
        let arg = args[index].to_str()?;
        if arg == "run" {
            index += 1;
            break;
        }
        if !arg.starts_with('-') {
            return None;
        }
        if arg == "--" {
            return None;
        }
        if uv_global_option_takes_value(arg) {
            index += 2;
        } else if uv_global_flag(arg) {
            index += 1;
        } else {
            return None;
        }
    }
    if index >= args.len() {
        return None;
    }

    while index < args.len() {
        let arg = args[index].to_str()?;
        if arg == "--" {
            return args.get(index + 1).map(Path::new);
        }
        if !arg.starts_with('-') || arg == "-" {
            return args.get(index).map(Path::new);
        }
        if arg == "-m" || arg == "--module" {
            return None;
        }
        if uv_run_option_takes_value(arg) {
            index += 2;
        } else if uv_run_flag(arg) {
            index += 1;
        } else {
            return None;
        }
    }
    None
}

pub(crate) fn resolve_script_operand(path: &Path) -> Result<PathBuf, String> {
    let path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        env::current_dir()
            .map_err(|err| format!("failed to resolve current directory: {err}"))?
            .join(path)
    };
    fs::canonicalize(&path).map_err(|err| format!("failed to resolve {}: {err}", path.display()))
}

pub(crate) fn sha256_file(path: &Path) -> Result<String, String> {
    let mut file =
        File::open(path).map_err(|err| format!("failed to open {}: {err}", path.display()))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 8192];
    loop {
        let count = file
            .read(&mut buffer)
            .map_err(|err| format!("failed to read {}: {err}", path.display()))?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

pub(crate) fn validate_target_file_metadata(uid: u32, mode: u32) -> Result<(), String> {
    if uid != 0 {
        return Err("target binary must be owned by root".to_string());
    }
    if mode & ((libc::S_IWGRP | libc::S_IWOTH) as u32) != 0 {
        return Err("target binary must not be writable by group or others".to_string());
    }
    if mode & 0o111 == 0 {
        return Err("target binary must be executable".to_string());
    }
    Ok(())
}

pub(crate) fn validate_directory_mode(path: &Path, mode: u32) -> Result<(), String> {
    if mode & ((libc::S_IWGRP | libc::S_IWOTH) as u32) != 0 {
        return Err(format!(
            "directory must not be writable by group or others: {}",
            path.display()
        ));
    }
    Ok(())
}

fn shebang_interpreter_path(path: &Path) -> Result<Option<PathBuf>, String> {
    let file = File::open(path).map_err(|err| {
        format!(
            "failed to open {} for shebang inspection: {err}",
            path.display()
        )
    })?;
    let mut reader = BufReader::new(file);
    let mut line = Vec::new();
    reader
        .read_until(b'\n', &mut line)
        .map_err(|err| format!("failed to read shebang from {}: {err}", path.display()))?;
    if !line.starts_with(b"#!") {
        return Ok(None);
    }
    let line = String::from_utf8_lossy(&line[2..]);
    let Some(interpreter) = line.split_whitespace().next() else {
        return Err("script shebang must name an interpreter".to_string());
    };
    let interpreter = Path::new(interpreter);
    if !interpreter.is_absolute() {
        return Err("script shebang interpreter must be absolute".to_string());
    }
    fs::canonicalize(interpreter).map(Some).map_err(|err| {
        format!(
            "failed to resolve shebang interpreter {}: {err}",
            interpreter.display()
        )
    })
}

fn is_versioned_python_name(file_name: &str) -> bool {
    let Some(suffix) = file_name.strip_prefix("python") else {
        return false;
    };
    !suffix.is_empty() && suffix.chars().all(|ch| ch == '.' || ch.is_ascii_digit())
}

pub(crate) fn interpreter_option_takes_value(arg: &str) -> bool {
    matches!(
        arg,
        "-c" | "-m" | "-S" | "-e" | "-I" | "-l" | "-x" | "-C" | "-M" | "-d" | "-r"
    )
}

fn uv_global_option_takes_value(arg: &str) -> bool {
    arg.contains('=')
        || matches!(
            arg,
            "--cache-dir" | "--color" | "--config-file" | "--directory" | "--project"
        )
}

fn uv_global_flag(arg: &str) -> bool {
    matches!(
        arg,
        "-q" | "--quiet"
            | "-v"
            | "--verbose"
            | "--native-tls"
            | "--offline"
            | "--no-cache"
            | "--no-config"
            | "--no-progress"
            | "--version"
    )
}

fn uv_run_option_takes_value(arg: &str) -> bool {
    arg.contains('=')
        || matches!(
            arg,
            "--config-setting"
                | "--config-settings-package"
                | "--default-index"
                | "--directory"
                | "--env-file"
                | "--exclude-newer"
                | "--exclude-newer-package"
                | "--extra"
                | "--find-links"
                | "--from"
                | "--group"
                | "--index"
                | "--index-strategy"
                | "--keyring-provider"
                | "--link-mode"
                | "--no-build-isolation-package"
                | "--no-binary-package"
                | "--no-build-package"
                | "--no-extra"
                | "--no-group"
                | "--only-binary-package"
                | "--only-group"
                | "--prerelease"
                | "--project"
                | "--python"
                | "--python-platform"
                | "--refresh-package"
                | "--resolution"
                | "--upgrade-package"
                | "--with"
                | "--with-editable"
                | "--with-requirements"
        )
}

fn uv_run_flag(arg: &str) -> bool {
    matches!(
        arg,
        "-q" | "--quiet"
            | "-v"
            | "--verbose"
            | "--active"
            | "--all-groups"
            | "--all-extras"
            | "--all-packages"
            | "--compile-bytecode"
            | "--dev"
            | "--exact"
            | "--frozen"
            | "--inexact"
            | "--isolated"
            | "--locked"
            | "--managed-python"
            | "--no-build-isolation"
            | "--no-cache"
            | "--no-config"
            | "--no-dev"
            | "--no-editable"
            | "--no-env-file"
            | "--no-extra"
            | "--no-group"
            | "--no-index"
            | "--no-managed-python"
            | "--no-progress"
            | "--no-sources"
            | "--no-sync"
            | "--offline"
            | "--refresh"
            | "--reinstall"
            | "--system"
            | "--upgrade"
    )
}
