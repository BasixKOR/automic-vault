use super::*;

use std::collections::BTreeMap;
use std::io;
use std::os::unix::net::UnixStream;
use std::os::unix::process::CommandExt;

pub const VAULT_AGENT_ID_ENV: &str = "VAULT_AGENT_ID";
pub const VAULT_SOCKET_PATH_ENV: &str = "VAULT_SOCKET_PATH";
pub const VAULT_TOOLCHAIN_ROOT_ENV: &str = "VAULT_TOOLCHAIN_ROOT";
pub const VAULT_TRUSTED_INTERNAL_ENV: &str = "VAULT_INTERNAL_EXEC";
pub const VAULT_TRUSTED_INTERNAL_VALUE: &str = "1";
const DEFAULT_VAULT_ROOT_PREFIX: &str = "automic-vault.";
const DEFAULT_VAULT_BIN_DIR: &str = "bin";
const DEFAULT_VAULT_SANDBOX_PROFILE: &str = "vault.sb";
const DEFAULT_VAULT_SOCKET_NAME: &str = "vault.sock";
const DEFAULT_VAULT_APPROVAL_ERROR_CODE: i32 = 403;
const SANDBOX_EXEC_PATH: &str = "/usr/bin/sandbox-exec";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExecutionIntent {
    pub tool: String,
    pub args: Vec<String>,
    pub cwd: String,
    pub env: BTreeMap<String, String>,
    pub agent_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VaultExecutionEnvironment {
    pub toolchain_root: String,
    pub bin_dir: String,
    pub sandbox_profile_path: String,
    pub allowed_executable_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub initial_executable_path: Option<String>,
    pub socket_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VaultToolAlias {
    pub name: String,
    pub source_path: String,
    pub link_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VaultToolchainManifest {
    pub environment: VaultExecutionEnvironment,
    pub aliases: Vec<VaultToolAlias>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VaultApprovalRequest {
    pub id: String,
    pub intent: ExecutionIntent,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VaultApprovalResponse {
    pub id: String,
    pub approved: bool,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VaultContainmentSession {
    pub id: String,
    pub pid: u32,
    pub agent_id: String,
    pub command: String,
    pub args: Vec<String>,
    pub cwd: String,
    pub initial_executable_path: String,
    pub toolchain_root: String,
    pub bin_dir: String,
    pub sandbox_profile_path: String,
    pub socket_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VaultExecChunk {
    pub id: String,
    pub stream: String,
    pub data: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VaultExecCompletion {
    pub id: String,
    pub exit_code: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum VaultClientRequest {
    ContainmentStarted { session: VaultContainmentSession },
    ApprovalRequest { id: String, intent: ExecutionIntent },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum VaultDaemonEvent {
    ApprovalResponse {
        id: String,
        approved: bool,
        reason: Option<String>,
    },
    ExecChunk {
        id: String,
        stream: String,
        data: String,
    },
    ExecComplete {
        id: String,
        exit_code: i32,
    },
    Error {
        id: Option<String>,
        code: i32,
        message: String,
    },
}

pub fn vault_main_entry() {
    let mut args = env::args_os();
    let program = args.next().unwrap_or_else(|| OsString::from("vault"));
    let program_name = Path::new(&program)
        .file_name()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .unwrap_or("vault");
    let result = run_vault_entry(program_name, args);
    if let Err(err) = result {
        eprintln!("{program_name}: {err}");
        process::exit(1);
    }
}

pub(crate) fn run_vault_entry(program_name: &str, args: env::ArgsOs) -> Result<(), String> {
    dispatch_vault(program_name, args)
}

fn dispatch_vault<I>(program_name: &str, mut args: I) -> Result<(), String>
where
    I: Iterator<Item = OsString>,
{
    let Some(first_arg) = args.next() else {
        print_vault_usage(program_name);
        return Err("missing command".to_string());
    };

    if is_help_flag(&first_arg) {
        print_vault_usage(program_name);
        return Ok(());
    }

    if is_version_flag(&first_arg) {
        println!("{program_name} {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    if matches!(first_arg.to_str(), Some("--proxy")) {
        return run_proxy(args);
    }
    if matches!(first_arg.to_str(), Some("internal-exec")) {
        return run_internal_exec(program_name, args);
    }
    if matches!(first_arg.to_str(), Some("toolchain")) {
        return run_toolchain_command(program_name, args);
    }
    if matches!(first_arg.to_str(), Some("sandbox-profile")) {
        return run_sandbox_profile_command(program_name, args);
    }

    run_sandboxed_command(first_arg, args)
}

fn run_proxy<I>(mut args: I) -> Result<(), String>
where
    I: Iterator<Item = OsString>,
{
    let stub_path = args
        .next()
        .ok_or_else(|| "missing proxy stub path".to_string())?;
    let stub_path = PathBuf::from(stub_path);
    let tool = stub_path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| "proxy stub path must end in a valid UTF-8 tool name".to_string())?
        .to_string();
    let intent = capture_proxy_intent(tool, args)?;
    request_vault_execution(intent)
}

fn run_sandboxed_command<I>(command: OsString, args: I) -> Result<(), String>
where
    I: Iterator<Item = OsString>,
{
    let command = command
        .into_string()
        .map_err(|_| "command must be valid UTF-8".to_string())?;
    let command_args = args.collect::<Vec<_>>();
    let command_arg_strings = command_args
        .iter()
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    let initial_executable = resolve_initial_executable(&command)?;
    let vault_binary =
        env::current_exe().map_err(|err| format!("failed to resolve vault executable: {err}"))?;
    let socket_path = resolve_vault_socket_path()?;
    let agent_id = env::var(VAULT_AGENT_ID_ENV)
        .ok()
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| format!("vault-{}", process::id()));
    let manifest = build_vault_toolchain_for_launch(
        &vault_binary,
        &socket_path,
        Some(initial_executable.as_path()),
    )?;
    let cwd =
        env::current_dir().map_err(|err| format!("failed to resolve current directory: {err}"))?;
    let session = VaultContainmentSession {
        id: agent_id.clone(),
        pid: process::id(),
        agent_id: agent_id.clone(),
        command: command.clone(),
        args: command_arg_strings,
        cwd: cwd.to_string_lossy().into_owned(),
        initial_executable_path: initial_executable.to_string_lossy().into_owned(),
        toolchain_root: manifest.environment.toolchain_root.clone(),
        bin_dir: manifest.environment.bin_dir.clone(),
        sandbox_profile_path: manifest.environment.sandbox_profile_path.clone(),
        socket_path: manifest.environment.socket_path.clone(),
    };
    notify_containment_started(&session);

    let mut sandbox = Command::new(SANDBOX_EXEC_PATH);
    sandbox
        .arg("-f")
        .arg(&manifest.environment.sandbox_profile_path)
        .arg(&initial_executable)
        .args(command_args)
        .env("PATH", &manifest.environment.bin_dir)
        .env(VAULT_SOCKET_PATH_ENV, &manifest.environment.socket_path)
        .env(
            VAULT_TOOLCHAIN_ROOT_ENV,
            &manifest.environment.toolchain_root,
        )
        .env(VAULT_AGENT_ID_ENV, agent_id);
    Err(format!("failed to enter sandbox: {}", sandbox.exec()))
}

fn run_internal_exec<I>(program_name: &str, args: I) -> Result<(), String>
where
    I: Iterator<Item = OsString>,
{
    match env::var(VAULT_TRUSTED_INTERNAL_ENV) {
        Ok(value) if value == VAULT_TRUSTED_INTERNAL_VALUE => {}
        _ => {
            return Err("internal-exec is restricted to trusted callers".to_string());
        }
    }

    let mut argv = Vec::new();
    for arg in args {
        argv.push(
            arg.into_string()
                .map_err(|_| "internal-exec arguments must be valid UTF-8".to_string())?,
        );
    }

    if argv.is_empty() {
        print_vault_usage(program_name);
        return Err("missing tool name".to_string());
    }

    let tool = argv.remove(0);
    let cwd =
        env::current_dir().map_err(|err| format!("failed to resolve current directory: {err}"))?;
    let mut command = Command::new(resolve_host_tool_path(&tool));
    command.args(argv);
    command.current_dir(&cwd);
    command.env_clear();
    for (key, value) in filtered_vault_environment() {
        command.env(key, value);
    }

    let status = command
        .status()
        .map_err(|err| format!("failed to execute '{tool}': {err}"))?;
    if status.success() {
        return Ok(());
    }

    Err(match status.code() {
        Some(code) => format!("command exited with status {code}"),
        None => "command terminated by signal".to_string(),
    })
}

fn run_toolchain_command<I>(program_name: &str, args: I) -> Result<(), String>
where
    I: Iterator<Item = OsString>,
{
    let mut socket_path: Option<PathBuf> = None;
    let mut vault_binary: Option<PathBuf> = None;
    let mut json_output = false;
    let mut args = args.peekable();

    while let Some(arg) = args.next() {
        let arg = arg
            .into_string()
            .map_err(|_| "toolchain arguments must be valid UTF-8".to_string())?;
        match arg.as_str() {
            "--help" | "-h" => {
                print_toolchain_usage(program_name);
                return Ok(());
            }
            "--json" => json_output = true,
            "--socket" => {
                let value = args
                    .next()
                    .ok_or_else(|| "missing value for --socket".to_string())?;
                socket_path = Some(PathBuf::from(value));
            }
            "--vault-bin" => {
                let value = args
                    .next()
                    .ok_or_else(|| "missing value for --vault-bin".to_string())?;
                vault_binary = Some(PathBuf::from(value));
            }
            other => return Err(format!("unknown toolchain argument '{other}'")),
        }
    }

    let socket_path = socket_path.unwrap_or(resolve_vault_socket_path()?);
    let vault_binary = match vault_binary {
        Some(path) => path,
        None => env::current_exe()
            .map_err(|err| format!("failed to resolve current executable: {err}"))?,
    };
    let manifest = build_vault_toolchain(&vault_binary, &socket_path)?;
    if json_output {
        println!(
            "{}",
            serde_json::to_string_pretty(&manifest)
                .map_err(|err| format!("failed to encode toolchain manifest: {err}"))?
        );
    } else {
        println!("{}", manifest.environment.toolchain_root);
    }
    Ok(())
}

fn run_sandbox_profile_command<I>(program_name: &str, args: I) -> Result<(), String>
where
    I: Iterator<Item = OsString>,
{
    let mut allowed_path: Option<PathBuf> = None;
    let mut args = args.peekable();
    while let Some(arg) = args.next() {
        let arg = arg
            .into_string()
            .map_err(|_| "sandbox-profile arguments must be valid UTF-8".to_string())?;
        match arg.as_str() {
            "--help" | "-h" => {
                print_sandbox_profile_usage(program_name);
                return Ok(());
            }
            "--allow" => {
                let value = args
                    .next()
                    .ok_or_else(|| "missing value for --allow".to_string())?;
                allowed_path = Some(PathBuf::from(value));
            }
            other => return Err(format!("unknown sandbox-profile argument '{other}'")),
        }
    }

    let allowed_path = match allowed_path {
        Some(path) => path,
        None => env::current_exe()
            .map_err(|err| format!("failed to resolve current executable: {err}"))?,
    };
    let stub_bin_dir = allowed_path
        .parent()
        .ok_or_else(|| "allowed path must have a parent directory".to_string())?;
    print!(
        "{}",
        render_vault_sandbox_profile(&allowed_path, stub_bin_dir, None)
    );
    Ok(())
}

fn request_vault_execution(intent: ExecutionIntent) -> Result<(), String> {
    let request_id = format!(
        "{}-{}",
        process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|err| format!("failed to compute request timestamp: {err}"))?
            .as_millis()
    );
    let request = VaultClientRequest::ApprovalRequest {
        id: request_id.clone(),
        intent,
    };
    let socket_path = resolve_vault_socket_path()?;
    let mut stream = UnixStream::connect(&socket_path)
        .map_err(|err| format!("vaultd unavailable at {}: {err}", socket_path.display()))?;
    let encoded = serde_json::to_string(&request)
        .map_err(|err| format!("failed to encode vault request: {err}"))?;
    stream
        .write_all(encoded.as_bytes())
        .and_then(|_| stream.write_all(b"\n"))
        .and_then(|_| stream.flush())
        .map_err(|err| format!("failed to send vault request: {err}"))?;

    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    let mut approved = false;

    loop {
        line.clear();
        let bytes = reader
            .read_line(&mut line)
            .map_err(|err| format!("failed to read vault response: {err}"))?;
        if bytes == 0 {
            if approved {
                return Ok(());
            }
            return Err("vaultd closed the connection before completion".to_string());
        }

        let trimmed = line.trim_end_matches(['\r', '\n']);
        if trimmed.is_empty() {
            continue;
        }

        let event: VaultDaemonEvent = serde_json::from_str(trimmed)
            .map_err(|err| format!("failed to decode vault response: {err}"))?;
        match event {
            VaultDaemonEvent::ApprovalResponse {
                id,
                approved: response_approved,
                reason,
            } => {
                if id != request_id {
                    return Err("vaultd returned a mismatched approval response".to_string());
                }
                if !response_approved {
                    return Err(reason.unwrap_or_else(|| "command denied".to_string()));
                }
                approved = true;
            }
            VaultDaemonEvent::ExecChunk { id, stream, data } => {
                if id != request_id {
                    return Err("vaultd returned a mismatched execution chunk".to_string());
                }
                match stream.as_str() {
                    "stdout" => {
                        print!("{data}");
                        let _ = io::stdout().flush();
                    }
                    "stderr" => {
                        eprint!("{data}");
                        let _ = io::stderr().flush();
                    }
                    other => return Err(format!("vaultd returned unknown stream '{other}'")),
                }
            }
            VaultDaemonEvent::ExecComplete { id, exit_code } => {
                if id != request_id {
                    return Err("vaultd returned a mismatched completion".to_string());
                }
                if exit_code == 0 {
                    return Ok(());
                }
                process::exit(exit_code);
            }
            VaultDaemonEvent::Error { id, code, message } => {
                if let Some(id) = id
                    && id != request_id
                {
                    return Err("vaultd returned a mismatched error".to_string());
                }
                if code == DEFAULT_VAULT_APPROVAL_ERROR_CODE {
                    return Err(message);
                }
                return Err(format!("vaultd error {code}: {message}"));
            }
        }
    }
}

fn notify_containment_started(session: &VaultContainmentSession) {
    let Ok(socket_path) = resolve_vault_socket_path() else {
        return;
    };
    let Ok(mut stream) = UnixStream::connect(&socket_path) else {
        return;
    };
    let request = VaultClientRequest::ContainmentStarted {
        session: session.clone(),
    };
    let Ok(encoded) = serde_json::to_string(&request) else {
        return;
    };
    let _ = stream
        .write_all(encoded.as_bytes())
        .and_then(|_| stream.write_all(b"\n"))
        .and_then(|_| stream.flush());
}

fn capture_proxy_intent<I>(tool: String, args: I) -> Result<ExecutionIntent, String>
where
    I: Iterator<Item = OsString>,
{
    let mut captured_args = Vec::new();
    for arg in args {
        captured_args.push(
            arg.into_string()
                .map_err(|_| "tool arguments must be valid UTF-8".to_string())?,
        );
    }
    build_execution_intent(tool, captured_args)
}

fn build_execution_intent(tool: String, args: Vec<String>) -> Result<ExecutionIntent, String> {
    if tool.contains('/') {
        return Err("tool name must not contain path separators".to_string());
    }
    let cwd = env::current_dir().map_err(|err| format!("failed to resolve cwd: {err}"))?;
    Ok(ExecutionIntent {
        tool,
        args,
        cwd: cwd.to_string_lossy().into_owned(),
        env: filtered_vault_environment(),
        agent_id: env::var(VAULT_AGENT_ID_ENV)
            .ok()
            .filter(|value| !value.is_empty()),
    })
}

fn filtered_vault_environment() -> BTreeMap<String, String> {
    let mut filtered = BTreeMap::new();
    for (key, value) in env::vars() {
        if matches!(
            key.as_str(),
            "HOME"
                | "LANG"
                | "LC_ALL"
                | "LOGNAME"
                | "PATH"
                | "PWD"
                | "SHELL"
                | "TERM"
                | "TMPDIR"
                | "USER"
                | VAULT_AGENT_ID_ENV
                | VAULT_SOCKET_PATH_ENV
                | VAULT_TOOLCHAIN_ROOT_ENV
        ) {
            filtered.insert(key, value);
        }
    }
    filtered
}

fn resolve_initial_executable(command: &str) -> Result<PathBuf, String> {
    if command.contains('/') {
        let path = PathBuf::from(command);
        if is_executable_file(&path) {
            return Ok(path);
        }
        return Err(format!("command '{command}' is not executable"));
    }

    let Some(paths) = env::var_os("PATH") else {
        return Err("PATH is not set".to_string());
    };
    for root in env::split_paths(&paths) {
        let candidate = root.join(command);
        if is_executable_file(&candidate) {
            return Ok(candidate);
        }
    }
    Err(format!("command '{command}' not found in PATH"))
}

fn resolve_host_tool_path(tool: &str) -> PathBuf {
    if let Some(path) = find_executable_in_paths(
        tool,
        [
            "/usr/local/bin",
            "/opt/homebrew/bin",
            "/usr/bin",
            "/bin",
            "/usr/sbin",
            "/sbin",
        ],
    ) {
        return path;
    }
    PathBuf::from(tool)
}

fn find_executable_in_paths<'a, I>(tool: &str, paths: I) -> Option<PathBuf>
where
    I: IntoIterator<Item = &'a str>,
{
    for path in paths {
        let candidate = Path::new(path).join(tool);
        if is_executable_file(&candidate) {
            return Some(candidate);
        }
    }
    None
}

fn resolve_vault_socket_path() -> Result<PathBuf, String> {
    if let Some(path) = env::var_os(VAULT_SOCKET_PATH_ENV)
        && !path.is_empty()
    {
        return Ok(PathBuf::from(path));
    }
    let home = env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())?;
    Ok(home
        .join("Library")
        .join("Application Support")
        .join("Automic Vault")
        .join(DEFAULT_VAULT_SOCKET_NAME))
}

pub fn print_vault_usage(program_name: &str) {
    println!(
        "\
Usage: {program_name} <command> [args...]
       {program_name} --proxy <stub-path> [args...]
       {program_name} internal-exec <tool> [args...]
       {program_name} toolchain [--socket <path>] [--vault-bin <path>] [--json]
       {program_name} sandbox-profile [--allow <path>]

Sandbox execution:
  Runs the command inside a vaulted sandbox with a synthetic PATH. Generated
  stubs inside that PATH call {program_name} --proxy to request approved host execution."
    );
}

fn print_toolchain_usage(program_name: &str) {
    println!(
        "\
Usage: {program_name} toolchain [--socket <path>] [--vault-bin <path>] [--json]

Creates a temporary interception toolchain rooted in /tmp and prints either
its root path or a JSON manifest."
    );
}

fn print_sandbox_profile_usage(program_name: &str) {
    println!(
        "\
Usage: {program_name} sandbox-profile [--allow <path>]

Prints a sandbox-exec profile that only permits execution of the allowed
vault binary."
    );
}

pub fn build_vault_toolchain(
    vault_binary: &Path,
    socket_path: &Path,
) -> Result<VaultToolchainManifest, String> {
    build_vault_toolchain_for_launch(vault_binary, socket_path, None)
}

pub fn build_vault_toolchain_for_launch(
    vault_binary: &Path,
    socket_path: &Path,
    initial_executable_path: Option<&Path>,
) -> Result<VaultToolchainManifest, String> {
    let root = tempfile::Builder::new()
        .prefix(DEFAULT_VAULT_ROOT_PREFIX)
        .tempdir_in("/tmp")
        .map_err(|err| format!("failed to create vault toolchain root: {err}"))?;
    let root_path = root.keep();
    let bin_dir = root_path.join(DEFAULT_VAULT_BIN_DIR);
    fs::create_dir_all(&bin_dir)
        .map_err(|err| format!("failed to create {}: {err}", bin_dir.display()))?;

    let staged_vault = bin_dir.join("vault");
    fs::copy(vault_binary, &staged_vault).map_err(|err| {
        format!(
            "failed to stage vault binary {} -> {}: {err}",
            vault_binary.display(),
            staged_vault.display()
        )
    })?;
    let mut permissions = fs::metadata(&staged_vault)
        .map_err(|err| format!("failed to read {} metadata: {err}", staged_vault.display()))?
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&staged_vault, permissions).map_err(|err| {
        format!(
            "failed to set executable permissions on {}: {err}",
            staged_vault.display()
        )
    })?;

    let aliases = collect_vault_aliases()
        .into_iter()
        .filter_map(|(name, source_path)| {
            if !source_path.is_file() {
                return None;
            }
            let stub_path = bin_dir.join(&name);
            if write_vault_proxy_stub(&stub_path, &staged_vault).is_err() {
                return None;
            }
            Some(VaultToolAlias {
                name,
                source_path: source_path.to_string_lossy().into_owned(),
                link_path: stub_path.to_string_lossy().into_owned(),
            })
        })
        .collect::<Vec<_>>();

    let sandbox_profile_path = root_path.join(DEFAULT_VAULT_SANDBOX_PROFILE);
    fs::write(
        &sandbox_profile_path,
        render_vault_sandbox_profile(&staged_vault, &bin_dir, initial_executable_path),
    )
    .map_err(|err| {
        format!(
            "failed to write sandbox profile {}: {err}",
            sandbox_profile_path.display()
        )
    })?;

    Ok(VaultToolchainManifest {
        environment: VaultExecutionEnvironment {
            toolchain_root: root_path.to_string_lossy().into_owned(),
            bin_dir: bin_dir.to_string_lossy().into_owned(),
            sandbox_profile_path: sandbox_profile_path.to_string_lossy().into_owned(),
            allowed_executable_path: staged_vault.to_string_lossy().into_owned(),
            initial_executable_path: initial_executable_path
                .map(|path| path.to_string_lossy().into_owned()),
            socket_path: socket_path.to_string_lossy().into_owned(),
        },
        aliases,
    })
}

fn write_vault_proxy_stub(stub_path: &Path, staged_vault: &Path) -> Result<(), String> {
    let stub = format!("#!{} --proxy\n", staged_vault.display());
    fs::write(stub_path, stub)
        .map_err(|err| format!("failed to write proxy stub {}: {err}", stub_path.display()))?;
    let mut permissions = fs::metadata(stub_path)
        .map_err(|err| format!("failed to read {} metadata: {err}", stub_path.display()))?
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(stub_path, permissions)
        .map_err(|err| format!("failed to chmod proxy stub {}: {err}", stub_path.display()))
}

pub fn render_vault_sandbox_profile(
    allowed_executable_path: &Path,
    stub_bin_dir: &Path,
    initial_executable_path: Option<&Path>,
) -> String {
    let mut profile = "(version 1)\n(allow default)".to_string();
    for path in sandbox_path_variants(allowed_executable_path) {
        profile.push_str(&format!(
            "\n(allow process-exec (literal \"{}\"))",
            path.display()
        ));
    }
    for path in sandbox_path_variants(stub_bin_dir) {
        profile.push_str(&format!(
            "\n(allow process-exec (subpath \"{}\"))",
            path.display()
        ));
    }
    for initial_executable_path in initial_executable_paths(initial_executable_path) {
        for path in sandbox_path_variants(&initial_executable_path) {
            profile.push_str(&format!(
                "\n(allow process-exec (literal \"{}\"))",
                path.display()
            ));
        }
    }
    profile.push_str(
        "
(deny process-exec)",
    );
    profile
}

fn sandbox_path_variants(path: &Path) -> Vec<PathBuf> {
    let mut variants = vec![path.to_path_buf()];
    if let Ok(canonical) = fs::canonicalize(path)
        && !variants.contains(&canonical)
    {
        variants.push(canonical);
    }
    if let Ok(stripped) = path.strip_prefix("/tmp") {
        let private_tmp = Path::new("/private/tmp").join(stripped);
        if !variants.contains(&private_tmp) {
            variants.push(private_tmp);
        }
    }
    variants
}

fn initial_executable_paths(initial_executable_path: Option<&Path>) -> Vec<PathBuf> {
    let Some(initial_executable_path) = initial_executable_path else {
        return Vec::new();
    };
    let mut paths = vec![initial_executable_path.to_path_buf()];
    if initial_executable_path == Path::new("/bin/sh") {
        paths.push(PathBuf::from("/bin/bash"));
    }
    paths
}

fn collect_vault_aliases() -> Vec<(String, PathBuf)> {
    let mut aliases = BTreeMap::new();
    let mut roots = Vec::new();
    if let Some(paths) = env::var_os("PATH") {
        roots.extend(env::split_paths(&paths));
    }
    roots.extend(
        [
            managed_bin_root(),
            PathBuf::from("/bin"),
            PathBuf::from("/usr/bin"),
            PathBuf::from("/sbin"),
            PathBuf::from("/usr/sbin"),
            PathBuf::from("/usr/local/bin"),
            PathBuf::from("/opt/homebrew/bin"),
        ]
        .into_iter(),
    );

    for root in roots {
        let Ok(entries) = fs::read_dir(&root) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if !is_executable_file(&path) {
                continue;
            }
            let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
                continue;
            };
            if name == "vault" || name.starts_with('.') {
                continue;
            }
            aliases
                .entry(name.to_string())
                .or_insert_with(|| path.clone());
        }
    }
    aliases.into_iter().collect()
}

fn is_executable_file(path: &Path) -> bool {
    let Ok(metadata) = fs::metadata(path) else {
        return false;
    };
    metadata.is_file() && metadata.permissions().mode() & 0o111 != 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::net::UnixListener;

    struct EnvGuard {
        previous: Vec<(&'static str, Option<OsString>)>,
    }

    impl EnvGuard {
        fn set(values: &[(&'static str, &str)]) -> Self {
            let previous = values
                .iter()
                .map(|(key, value)| {
                    let previous = env::var_os(key);
                    unsafe { env::set_var(key, value) };
                    (*key, previous)
                })
                .collect();
            Self { previous }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            for (key, previous) in self.previous.drain(..).rev() {
                match previous {
                    Some(value) => unsafe { env::set_var(key, value) },
                    None => unsafe { env::remove_var(key) },
                }
            }
        }
    }

    fn write_executable(path: &Path) {
        fs::write(path, b"#!/bin/sh\n").unwrap();
        let mut permissions = fs::metadata(path).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(path, permissions).unwrap();
    }

    #[test]
    fn subs_filtered_environment_keeps_expected_keys_only() {
        let _lock = crate::global_test_env_lock().lock().unwrap();
        let _env = EnvGuard::set(&[
            ("HOME", "/tmp/home"),
            ("PATH", "/tmp/bin"),
            ("SECRET_TOKEN", "nope"),
            (VAULT_AGENT_ID_ENV, "agent-1"),
        ]);

        let filtered = filtered_vault_environment();
        assert_eq!(filtered.get("HOME"), Some(&"/tmp/home".to_string()));
        assert_eq!(filtered.get("PATH"), Some(&"/tmp/bin".to_string()));
        assert_eq!(
            filtered.get(VAULT_AGENT_ID_ENV),
            Some(&"agent-1".to_string())
        );
        assert!(!filtered.contains_key("SECRET_TOKEN"));
    }

    #[test]
    fn subs_sandbox_profile_allows_stubs_vault_and_initial_binary() {
        let profile = render_vault_sandbox_profile(
            Path::new("/tmp/automic-vault.123/bin/vault"),
            Path::new("/tmp/automic-vault.123/bin"),
            Some(Path::new("/usr/local/bin/codex")),
        );
        assert!(profile.contains("(allow default)"));
        assert!(profile.contains("/tmp/automic-vault.123/bin/vault"));
        assert!(profile.contains("/tmp/automic-vault.123/bin"));
        assert!(profile.contains("/private/tmp/automic-vault.123/bin/vault"));
        assert!(profile.contains("/private/tmp/automic-vault.123/bin"));
        assert!(profile.contains("/usr/local/bin/codex"));
        assert!(profile.contains("(deny process-exec)"));
    }

    #[test]
    fn subs_sandbox_profile_allows_bin_sh_bash_variant() {
        let profile = render_vault_sandbox_profile(
            Path::new("/tmp/automic-vault.123/bin/vault"),
            Path::new("/tmp/automic-vault.123/bin"),
            Some(Path::new("/bin/sh")),
        );
        assert!(profile.contains("/bin/sh"));
        assert!(profile.contains("/bin/bash"));
    }

    #[test]
    fn subs_build_toolchain_stages_vault_and_writes_proxy_stubs() {
        let tempdir = tempfile::tempdir().unwrap();
        let vault_binary = tempdir.path().join("vault");
        fs::write(&vault_binary, "#!/bin/sh\n").unwrap();
        let mut permissions = fs::metadata(&vault_binary).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&vault_binary, permissions).unwrap();

        let manifest = build_vault_toolchain(&vault_binary, Path::new("/tmp/vault.sock")).unwrap();
        assert!(Path::new(&manifest.environment.allowed_executable_path).is_file());
        assert!(Path::new(&manifest.environment.sandbox_profile_path).is_file());
        assert!(manifest.aliases.iter().all(|alias| {
            let stub_path = Path::new(&alias.link_path);
            let metadata = stub_path.symlink_metadata().unwrap();
            let content = fs::read_to_string(stub_path).unwrap();
            metadata.is_file() && !metadata.file_type().is_symlink() && content.contains(" --proxy")
        }));
    }

    #[test]
    fn subs_proxy_intent_uses_stub_basename_as_tool() {
        let intent = build_execution_intent(
            Path::new("/tmp/automic-vault.123/bin/git")
                .file_name()
                .unwrap()
                .to_string_lossy()
                .into_owned(),
            vec!["status".to_string()],
        )
        .unwrap();
        assert_eq!(intent.tool, "git");
        assert_eq!(intent.args, vec!["status"]);
    }

    #[test]
    fn subs_vault_dispatch_rejects_invalid_modes_and_args() {
        assert!(dispatch_vault("vault", Vec::<OsString>::new().into_iter()).is_err());
        assert!(
            dispatch_vault("vault", vec![OsString::from("--help")].into_iter()).is_ok()
        );
        assert!(
            dispatch_vault("vault", vec![OsString::from("--version")].into_iter()).is_ok()
        );
        assert!(run_proxy(Vec::<OsString>::new().into_iter())
            .unwrap_err()
            .contains("missing proxy stub path"));
        assert!(run_internal_exec("vault", Vec::<OsString>::new().into_iter())
            .unwrap_err()
            .contains("restricted"));
        assert!(run_toolchain_command(
            "vault",
            vec![OsString::from("--unknown")].into_iter(),
        )
        .unwrap_err()
        .contains("unknown toolchain"));
        assert!(run_sandbox_profile_command(
            "vault",
            vec![OsString::from("--allow")].into_iter(),
        )
        .unwrap_err()
        .contains("missing value"));
    }

    #[test]
    fn subs_vault_resolution_helpers_cover_paths_and_env() {
        let _lock = crate::global_test_env_lock().lock().unwrap();
        let temp = TempDir::new().unwrap();
        let bin = temp.path().join("bin");
        fs::create_dir_all(&bin).unwrap();
        let tool = bin.join("demo-tool");
        write_executable(&tool);
        let socket = temp.path().join("vault.sock");
        let _env = EnvGuard::set(&[
            ("HOME", temp.path().to_str().unwrap()),
            ("PATH", bin.to_str().unwrap()),
            (VAULT_SOCKET_PATH_ENV, socket.to_str().unwrap()),
            (VAULT_AGENT_ID_ENV, "agent-42"),
        ]);

        assert_eq!(resolve_vault_socket_path().unwrap(), socket);
        assert_eq!(resolve_initial_executable("demo-tool").unwrap(), tool);
        assert!(resolve_initial_executable("missing-tool")
            .unwrap_err()
            .contains("not found"));
        assert!(resolve_initial_executable("/tmp/not-executable")
            .unwrap_err()
            .contains("not executable"));
        assert_eq!(
            find_executable_in_paths("demo-tool", [bin.to_str().unwrap()]).unwrap(),
            bin.join("demo-tool")
        );
        assert_eq!(
            resolve_host_tool_path("definitely-not-a-real-tool"),
            PathBuf::from("definitely-not-a-real-tool")
        );

        let intent = capture_proxy_intent(
            "demo-tool".to_string(),
            vec![OsString::from("--flag")].into_iter(),
        )
        .unwrap();
        assert_eq!(intent.agent_id, Some("agent-42".to_string()));
        assert_eq!(intent.args, vec!["--flag"]);
        assert!(build_execution_intent("bad/tool".to_string(), Vec::new())
            .unwrap_err()
            .contains("path separators"));
    }

    #[test]
    fn subs_request_vault_execution_handles_success_and_errors() {
        fn serve_once<F>(socket: &Path, respond: F) -> thread::JoinHandle<()>
        where
            F: FnOnce(VaultClientRequest, &mut UnixStream) + Send + 'static,
        {
            let listener = UnixListener::bind(socket).unwrap();
            thread::spawn(move || {
                let (mut stream, _) = listener.accept().unwrap();
                let mut line = String::new();
                BufReader::new(stream.try_clone().unwrap())
                    .read_line(&mut line)
                    .unwrap();
                let request = serde_json::from_str::<VaultClientRequest>(&line).unwrap();
                respond(request, &mut stream);
            })
        }

        let _lock = crate::global_test_env_lock().lock().unwrap();
        let temp = TempDir::new().unwrap();
        let success_socket = temp.path().join("success.sock");
        let _env = EnvGuard::set(&[(VAULT_SOCKET_PATH_ENV, success_socket.to_str().unwrap())]);
        let success = serve_once(&success_socket, |request, stream| {
            let VaultClientRequest::ApprovalRequest { id, .. } = request else {
                panic!("unexpected request")
            };
            for event in [
                VaultDaemonEvent::ApprovalResponse {
                    id: id.clone(),
                    approved: true,
                    reason: None,
                },
                VaultDaemonEvent::ExecChunk {
                    id: id.clone(),
                    stream: "stdout".to_string(),
                    data: "ok\n".to_string(),
                },
                VaultDaemonEvent::ExecChunk {
                    id: id.clone(),
                    stream: "stderr".to_string(),
                    data: "warn\n".to_string(),
                },
                VaultDaemonEvent::ExecComplete { id, exit_code: 0 },
            ] {
                writeln!(stream, "{}", serde_json::to_string(&event).unwrap()).unwrap();
            }
        });
        request_vault_execution(build_execution_intent("git".to_string(), Vec::new()).unwrap())
            .unwrap();
        success.join().unwrap();

        let denied_socket = temp.path().join("denied.sock");
        let _env = EnvGuard::set(&[(VAULT_SOCKET_PATH_ENV, denied_socket.to_str().unwrap())]);
        let denied = serve_once(&denied_socket, |request, stream| {
            let VaultClientRequest::ApprovalRequest { id, .. } = request else {
                panic!("unexpected request")
            };
            writeln!(
                stream,
                "{}",
                serde_json::to_string(&VaultDaemonEvent::ApprovalResponse {
                    id,
                    approved: false,
                    reason: Some("denied".to_string()),
                })
                .unwrap()
            )
            .unwrap();
        });
        assert_eq!(
            request_vault_execution(build_execution_intent("git".to_string(), Vec::new()).unwrap())
                .unwrap_err(),
            "denied"
        );
        denied.join().unwrap();

        let error_socket = temp.path().join("error.sock");
        let _env = EnvGuard::set(&[(VAULT_SOCKET_PATH_ENV, error_socket.to_str().unwrap())]);
        let error = serve_once(&error_socket, |_request, stream| {
            writeln!(
                stream,
                "{}",
                serde_json::to_string(&VaultDaemonEvent::Error {
                    id: None,
                    code: 500,
                    message: "boom".to_string(),
                })
                .unwrap()
            )
            .unwrap();
        });
        assert!(request_vault_execution(build_execution_intent("git".to_string(), Vec::new()).unwrap())
            .unwrap_err()
            .contains("vaultd error 500"));
        error.join().unwrap();
    }

    #[test]
    fn subs_notify_containment_started_sends_best_effort_event() {
        let _lock = crate::global_test_env_lock().lock().unwrap();
        let temp = TempDir::new().unwrap();
        let socket = temp.path().join("containment.sock");
        let listener = UnixListener::bind(&socket).unwrap();
        let _env = EnvGuard::set(&[(VAULT_SOCKET_PATH_ENV, socket.to_str().unwrap())]);
        let handle = thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let mut line = String::new();
            BufReader::new(stream).read_line(&mut line).unwrap();
            serde_json::from_str::<VaultClientRequest>(&line).unwrap()
        });
        let session = VaultContainmentSession {
            id: "session".to_string(),
            pid: 1,
            agent_id: "agent".to_string(),
            command: "git".to_string(),
            args: vec!["status".to_string()],
            cwd: "/tmp".to_string(),
            initial_executable_path: "/usr/bin/git".to_string(),
            toolchain_root: "/tmp/vault".to_string(),
            bin_dir: "/tmp/vault/bin".to_string(),
            sandbox_profile_path: "/tmp/vault/vault.sb".to_string(),
            socket_path: socket.to_string_lossy().into_owned(),
        };

        notify_containment_started(&session);
        assert_eq!(
            handle.join().unwrap(),
            VaultClientRequest::ContainmentStarted { session }
        );
    }

    #[test]
    fn subs_toolchain_and_sandbox_commands_cover_success_paths() {
        let _lock = crate::global_test_env_lock().lock().unwrap();
        let temp = TempDir::new().unwrap();
        let vault_binary = temp.path().join("vault");
        write_executable(&vault_binary);
        let socket = temp.path().join("vault.sock");
        let _env = EnvGuard::set(&[
            ("HOME", temp.path().to_str().unwrap()),
            (VAULT_SOCKET_PATH_ENV, socket.to_str().unwrap()),
        ]);

        run_toolchain_command(
            "vault",
            vec![
                OsString::from("--socket"),
                OsString::from(socket.as_os_str()),
                OsString::from("--vault-bin"),
                OsString::from(vault_binary.as_os_str()),
                OsString::from("--json"),
            ]
            .into_iter(),
        )
        .unwrap();
        run_sandbox_profile_command(
            "vault",
            vec![
                OsString::from("--allow"),
                OsString::from(vault_binary.as_os_str()),
            ]
            .into_iter(),
        )
        .unwrap();
    }
}
