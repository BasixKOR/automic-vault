use super::*;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;
use std::collections::BTreeMap;
use std::io;

#[cfg(target_os = "macos")]
use std::ffi::{CString, c_char, c_int};

const DOTENV_KEYCHAIN_SERVICE: &str = "com.automicvault.dotenv";
const ENCRYPTED_PREFIX: &str = "encrypted:";
const DOTENV_PUBLIC_KEY_PREFIX: &str = "DOTENV_PUBLIC_KEY";
const DOTENV_PRIVATE_KEY_PREFIX: &str = "DOTENV_PRIVATE_KEY";
const DOTENV_USER_APPROVAL_SUBDIR: &str = "dotenv";
const DOTENV_APPROVAL_NOTIFICATION: &str = "com.automicvault.dotenv-approval.pending-changed";
const DOTENV_REMEMBERED_APPROVALS: &str = "remembered-approvals.json";
const AV_DOTENV_FILE_ENV: &str = "AV_DOTENV_FILE";
const AV_DOTENV_DIGEST_ENV: &str = "AV_DOTENV_DIGEST";
const AV_DOTENV_KEYS_ENV: &str = "AV_DOTENV_KEYS";

#[cfg(target_os = "macos")]
const ERR_SEC_ITEM_NOT_FOUND: c_int = -25300;

#[derive(Debug, Clone, PartialEq, Eq)]
enum DotenvCommand {
    Init(DotenvFileOption),
    Set(DotenvSetOptions),
    Encrypt(DotenvEncryptOptions),
    Import(DotenvImportOptions),
    Hook(DotenvShell),
    Export(DotenvExportOptions),
    Run(DotenvRunOptions),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DotenvFileOption {
    file: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DotenvSetOptions {
    file: PathBuf,
    key: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DotenvEncryptOptions {
    file: PathBuf,
    include_keys: Vec<String>,
    exclude_keys: Vec<String>,
    check: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DotenvImportOptions {
    file: PathBuf,
    keys_file: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DotenvExportOptions {
    shell: DotenvShell,
    cwd: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DotenvRunOptions {
    file: PathBuf,
    command: OsString,
    args: Vec<OsString>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DotenvShell {
    Bash,
    Fish,
    Zsh,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum DotenvApprovalMode {
    Export,
    Run,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct DotenvApprovalRequestSnapshot {
    id: String,
    mode: DotenvApprovalMode,
    env_file_path: String,
    project_root: String,
    env_sha256: String,
    public_key_fingerprint: String,
    keys: Vec<String>,
    cwd: String,
    parent_process: DotenvParentProcessSnapshot,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    command: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct DotenvParentProcessSnapshot {
    pid: i32,
    executable_path: Option<String>,
    display_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct DotenvApprovalDecision {
    id: String,
    approved: bool,
    reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct DotenvRememberedApprovalEntry {
    mode: DotenvApprovalMode,
    env_file_path: String,
    project_root: String,
    env_sha256: String,
    public_key_fingerprint: String,
    keys: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
struct DotenvRememberedApprovalStore {
    entries: Vec<DotenvRememberedApprovalEntry>,
}

impl DotenvRememberedApprovalStore {
    fn contains(&self, entry: &DotenvRememberedApprovalEntry) -> bool {
        self.entries.iter().any(|candidate| candidate == entry)
    }

    fn remember(&mut self, entry: DotenvRememberedApprovalEntry) {
        if !self.contains(&entry) {
            self.entries.push(entry);
        }
    }
}

trait DotenvPrivateKeyStore {
    fn load_private_key(&self, public_key: &str) -> Result<String, String>;
    fn store_private_key(&self, public_key: &str, private_key: &str) -> Result<(), String>;
}

struct KeychainDotenvPrivateKeyStore;

#[derive(Debug, Clone, PartialEq, Eq)]
struct DotenvLine {
    raw: String,
    assignment: Option<DotenvAssignment>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DotenvAssignment {
    key: String,
    value: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DotenvDocument {
    path: PathBuf,
    lines: Vec<DotenvLine>,
    had_trailing_newline: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DotenvKeypair {
    public_key_name: String,
    public_key: String,
    private_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DotenvLoadedSecrets {
    env_path: PathBuf,
    project_root: PathBuf,
    env_sha256: String,
    public_key_fingerprint: String,
    values: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DotenvRedactor {
    secrets: Vec<Vec<u8>>,
    pending: Vec<u8>,
    redacted: usize,
    hold_len: usize,
}

pub(crate) fn run_dotenv_entry(
    program_name: &str,
    args: impl Iterator<Item = OsString>,
) -> Result<(), String> {
    dispatch_dotenv(program_name, args, &KeychainDotenvPrivateKeyStore)
}

fn dispatch_dotenv(
    program_name: &str,
    args: impl Iterator<Item = OsString>,
    store: &dyn DotenvPrivateKeyStore,
) -> Result<(), String> {
    let Some(command) = parse_dotenv_command(program_name, args)? else {
        return Ok(());
    };
    match command {
        DotenvCommand::Init(options) => run_dotenv_init(&options, store),
        DotenvCommand::Set(options) => {
            let value = read_dotenv_secret()?;
            run_dotenv_set(&options, &value, store)
        }
        DotenvCommand::Encrypt(options) => run_dotenv_encrypt(&options, store),
        DotenvCommand::Import(options) => run_dotenv_import(&options, store),
        DotenvCommand::Hook(shell) => {
            print_dotenv_hook(program_name, shell);
            Ok(())
        }
        DotenvCommand::Export(options) => run_dotenv_export(&options, store),
        DotenvCommand::Run(options) => run_dotenv_run(&options, store),
    }
}

fn parse_dotenv_command(
    program_name: &str,
    mut args: impl Iterator<Item = OsString>,
) -> Result<Option<DotenvCommand>, String> {
    let Some(first_arg) = args.next() else {
        print_dotenv_usage(program_name);
        return Err("missing dotenv command".to_string());
    };
    if is_help_flag(&first_arg) {
        print_dotenv_usage(program_name);
        return Ok(None);
    }
    if is_version_flag(&first_arg) {
        println!("{program_name} {}", env!("CARGO_PKG_VERSION"));
        return Ok(None);
    }

    let subcommand = first_arg
        .to_str()
        .ok_or_else(|| "dotenv command must be valid UTF-8".to_string())?;
    match subcommand {
        "init" => parse_dotenv_init(program_name, args).map(|value| value.map(DotenvCommand::Init)),
        "set" => parse_dotenv_set(program_name, args).map(|value| value.map(DotenvCommand::Set)),
        "encrypt" => {
            parse_dotenv_encrypt(program_name, args).map(|value| value.map(DotenvCommand::Encrypt))
        }
        "import" => {
            parse_dotenv_import(program_name, args).map(|value| value.map(DotenvCommand::Import))
        }
        "hook" => parse_dotenv_hook(program_name, args).map(|value| value.map(DotenvCommand::Hook)),
        "export" => {
            parse_dotenv_export(program_name, args).map(|value| value.map(DotenvCommand::Export))
        }
        "run" => parse_dotenv_run(program_name, args).map(|value| value.map(DotenvCommand::Run)),
        other => Err(format!("unknown dotenv command '{other}'")),
    }
}

fn parse_dotenv_init(
    program_name: &str,
    args: impl Iterator<Item = OsString>,
) -> Result<Option<DotenvFileOption>, String> {
    parse_file_only_options(program_name, "init", args, print_dotenv_init_usage)
}

fn parse_dotenv_set(
    program_name: &str,
    args: impl Iterator<Item = OsString>,
) -> Result<Option<DotenvSetOptions>, String> {
    let mut file = PathBuf::from(".env");
    let mut key: Option<String> = None;
    let mut args = args.peekable();
    while let Some(arg) = args.next() {
        if is_help_flag(&arg) {
            print_dotenv_set_usage(program_name);
            return Ok(None);
        }
        if is_version_flag(&arg) {
            println!("{program_name} {}", env!("CARGO_PKG_VERSION"));
            return Ok(None);
        }
        if arg == "--file" || arg == "-f" {
            file = next_path_value(&mut args, "--file")?;
            continue;
        }
        if key.is_some() {
            return Err("dotenv set supports one KEY".to_string());
        }
        let value = arg
            .to_str()
            .ok_or_else(|| "dotenv set key must be valid UTF-8".to_string())?;
        validate_dotenv_key_name(value)?;
        key = Some(value.to_string());
    }

    let Some(key) = key else {
        print_dotenv_set_usage(program_name);
        return Err("missing KEY".to_string());
    };
    Ok(Some(DotenvSetOptions { file, key }))
}

fn parse_dotenv_encrypt(
    program_name: &str,
    args: impl Iterator<Item = OsString>,
) -> Result<Option<DotenvEncryptOptions>, String> {
    let mut file = PathBuf::from(".env");
    let mut include_keys = Vec::new();
    let mut exclude_keys = Vec::new();
    let mut check = false;
    let mut args = args.peekable();
    while let Some(arg) = args.next() {
        if is_help_flag(&arg) {
            print_dotenv_encrypt_usage(program_name);
            return Ok(None);
        }
        if is_version_flag(&arg) {
            println!("{program_name} {}", env!("CARGO_PKG_VERSION"));
            return Ok(None);
        }
        if arg == "--file" || arg == "-f" {
            file = next_path_value(&mut args, "--file")?;
            continue;
        }
        if arg == "--check" {
            check = true;
            continue;
        }
        if arg == "--key" || arg == "-k" {
            collect_key_values(&mut args, &mut include_keys, "--key")?;
            continue;
        }
        if arg == "--exclude-key" || arg == "-ek" {
            collect_key_values(&mut args, &mut exclude_keys, "--exclude-key")?;
            continue;
        }
        return Err(format!(
            "unknown dotenv encrypt argument '{}'",
            arg.to_string_lossy()
        ));
    }
    include_keys.sort();
    include_keys.dedup();
    exclude_keys.sort();
    exclude_keys.dedup();
    Ok(Some(DotenvEncryptOptions {
        file,
        include_keys,
        exclude_keys,
        check,
    }))
}

fn parse_dotenv_import(
    program_name: &str,
    args: impl Iterator<Item = OsString>,
) -> Result<Option<DotenvImportOptions>, String> {
    let mut file = PathBuf::from(".env");
    let mut keys_file: Option<PathBuf> = None;
    let mut args = args.peekable();
    while let Some(arg) = args.next() {
        if is_help_flag(&arg) {
            print_dotenv_import_usage(program_name);
            return Ok(None);
        }
        if is_version_flag(&arg) {
            println!("{program_name} {}", env!("CARGO_PKG_VERSION"));
            return Ok(None);
        }
        if arg == "--file" || arg == "-f" {
            file = next_path_value(&mut args, "--file")?;
            continue;
        }
        if arg == "--keys-file" {
            keys_file = Some(next_path_value(&mut args, "--keys-file")?);
            continue;
        }
        return Err(format!(
            "unknown dotenv import argument '{}'",
            arg.to_string_lossy()
        ));
    }
    let keys_file = keys_file.unwrap_or_else(|| {
        file.parent()
            .map(|parent| parent.join(".env.keys"))
            .unwrap_or_else(|| PathBuf::from(".env.keys"))
    });
    Ok(Some(DotenvImportOptions { file, keys_file }))
}

fn parse_dotenv_hook(
    program_name: &str,
    mut args: impl Iterator<Item = OsString>,
) -> Result<Option<DotenvShell>, String> {
    let Some(shell) = args.next() else {
        print_dotenv_hook_usage(program_name);
        return Err("missing shell".to_string());
    };
    if is_help_flag(&shell) {
        print_dotenv_hook_usage(program_name);
        return Ok(None);
    }
    if is_version_flag(&shell) {
        println!("{program_name} {}", env!("CARGO_PKG_VERSION"));
        return Ok(None);
    }
    if args.next().is_some() {
        return Err("dotenv hook supports one shell".to_string());
    }
    let shell = parse_dotenv_shell(&shell)?;
    Ok(Some(shell))
}

fn parse_dotenv_export(
    program_name: &str,
    args: impl Iterator<Item = OsString>,
) -> Result<Option<DotenvExportOptions>, String> {
    let mut shell: Option<DotenvShell> = None;
    let mut cwd: Option<PathBuf> = None;
    let mut args = args.peekable();
    while let Some(arg) = args.next() {
        if is_help_flag(&arg) {
            print_dotenv_export_usage(program_name);
            return Ok(None);
        }
        if is_version_flag(&arg) {
            println!("{program_name} {}", env!("CARGO_PKG_VERSION"));
            return Ok(None);
        }
        if arg == "--shell" {
            let value = args
                .next()
                .ok_or_else(|| "missing value for --shell".to_string())?;
            shell = Some(parse_dotenv_shell(&value)?);
            continue;
        }
        if arg == "--cwd" {
            cwd = Some(next_path_value(&mut args, "--cwd")?);
            continue;
        }
        return Err(format!(
            "unknown dotenv export argument '{}'",
            arg.to_string_lossy()
        ));
    }
    let shell = shell.ok_or_else(|| "missing --shell".to_string())?;
    let cwd =
        cwd.unwrap_or(env::current_dir().map_err(|err| format!("failed to resolve cwd: {err}"))?);
    Ok(Some(DotenvExportOptions { shell, cwd }))
}

fn parse_dotenv_run(
    program_name: &str,
    args: impl Iterator<Item = OsString>,
) -> Result<Option<DotenvRunOptions>, String> {
    let mut file = PathBuf::from(".env");
    let mut positionals = Vec::new();
    let mut args = args.peekable();
    while let Some(arg) = args.next() {
        if positionals.is_empty() {
            if is_help_flag(&arg) {
                print_dotenv_run_usage(program_name);
                return Ok(None);
            }
            if is_version_flag(&arg) {
                println!("{program_name} {}", env!("CARGO_PKG_VERSION"));
                return Ok(None);
            }
            if arg == "--file" || arg == "-f" {
                file = next_path_value(&mut args, "--file")?;
                continue;
            }
            if arg == "--" {
                positionals.extend(args);
                break;
            }
        }
        positionals.push(arg);
    }
    if positionals.is_empty() {
        print_dotenv_run_usage(program_name);
        return Err("missing command".to_string());
    }
    let command = positionals.remove(0);
    Ok(Some(DotenvRunOptions {
        file,
        command,
        args: positionals,
    }))
}

fn parse_file_only_options<I>(
    program_name: &str,
    command_name: &str,
    args: I,
    print_usage: fn(&str),
) -> Result<Option<DotenvFileOption>, String>
where
    I: Iterator<Item = OsString>,
{
    let mut file = PathBuf::from(".env");
    let mut args = args.peekable();
    while let Some(arg) = args.next() {
        if is_help_flag(&arg) {
            print_usage(program_name);
            return Ok(None);
        }
        if is_version_flag(&arg) {
            println!("{program_name} {}", env!("CARGO_PKG_VERSION"));
            return Ok(None);
        }
        if arg == "--file" || arg == "-f" {
            file = next_path_value(&mut args, "--file")?;
            continue;
        }
        return Err(format!(
            "unknown dotenv {command_name} argument '{}'",
            arg.to_string_lossy()
        ));
    }
    Ok(Some(DotenvFileOption { file }))
}

fn next_path_value<I>(args: &mut std::iter::Peekable<I>, flag: &str) -> Result<PathBuf, String>
where
    I: Iterator<Item = OsString>,
{
    args.next()
        .map(PathBuf::from)
        .ok_or_else(|| format!("missing value for {flag}"))
}

fn collect_key_values<I>(
    args: &mut std::iter::Peekable<I>,
    keys: &mut Vec<String>,
    flag: &str,
) -> Result<(), String>
where
    I: Iterator<Item = OsString>,
{
    let mut collected = 0;
    while let Some(next) = args.peek() {
        if next.to_string_lossy().starts_with('-') {
            break;
        }
        let value = args.next().unwrap();
        let key = value
            .to_str()
            .ok_or_else(|| format!("{flag} value must be valid UTF-8"))?;
        validate_dotenv_key_name(key)?;
        keys.push(key.to_string());
        collected += 1;
    }
    if collected == 0 {
        return Err(format!("missing value for {flag}"));
    }
    Ok(())
}

fn parse_dotenv_shell(value: &OsStr) -> Result<DotenvShell, String> {
    match value.to_str() {
        Some("bash") => Ok(DotenvShell::Bash),
        Some("fish") => Ok(DotenvShell::Fish),
        Some("zsh") => Ok(DotenvShell::Zsh),
        Some(other) => Err(format!("unsupported shell '{other}'")),
        None => Err("shell must be valid UTF-8".to_string()),
    }
}

fn run_dotenv_init(
    options: &DotenvFileOption,
    store: &dyn DotenvPrivateKeyStore,
) -> Result<(), String> {
    let mut document = DotenvDocument::load_or_empty(&options.file)?;
    if document.public_key().is_some() {
        return Err(format!(
            "{} already has a DOTENV_PUBLIC_KEY",
            document.path.display()
        ));
    }
    let keypair = generate_dotenv_keypair(&document.path);
    document.ensure_public_key(&keypair.public_key_name, &keypair.public_key);
    document.write()?;
    store.store_private_key(&keypair.public_key, &keypair.private_key)?;
    println!("initialized {}", document.path.display());
    Ok(())
}

fn run_dotenv_set(
    options: &DotenvSetOptions,
    value: &str,
    store: &dyn DotenvPrivateKeyStore,
) -> Result<(), String> {
    let mut document = DotenvDocument::load_or_empty(&options.file)?;
    let public_key = ensure_document_public_key(&mut document, store)?;
    let encrypted = encrypt_dotenv_value(value, &public_key)?;
    document.set_value(&options.key, &encrypted);
    document.write()?;
    println!("set {}", options.key);
    Ok(())
}

fn run_dotenv_encrypt(
    options: &DotenvEncryptOptions,
    store: &dyn DotenvPrivateKeyStore,
) -> Result<(), String> {
    let mut document = DotenvDocument::load_or_empty(&options.file)?;
    let keys = document.encryptable_keys(&options.include_keys, &options.exclude_keys);
    if options.check {
        if keys.is_empty() {
            return Ok(());
        }
        return Err(format!(
            "{} has plaintext dotenv values: {}",
            document.path.display(),
            keys.join(", ")
        ));
    }
    let public_key = ensure_document_public_key(&mut document, store)?;
    if keys.is_empty() {
        document.write()?;
        println!("no plaintext values to encrypt");
        return Ok(());
    }
    for key in &keys {
        let value = document
            .value(key)
            .ok_or_else(|| format!("missing key during encryption: {key}"))?;
        let encrypted = encrypt_dotenv_value(&value, &public_key)?;
        document.set_value(key, &encrypted);
    }
    document.write()?;
    println!("encrypted {}", keys.join(", "));
    Ok(())
}

fn run_dotenv_import(
    options: &DotenvImportOptions,
    store: &dyn DotenvPrivateKeyStore,
) -> Result<(), String> {
    let document = DotenvDocument::load(&options.file)?;
    let (public_key_name, public_key) = document
        .public_key()
        .ok_or_else(|| format!("{} is missing DOTENV_PUBLIC_KEY", document.path.display()))?;
    let private_key_name = private_key_name_for_public_key_name(&public_key_name);
    let keys_document = DotenvDocument::load(&options.keys_file)?;
    let private_key = keys_document.value(&private_key_name).ok_or_else(|| {
        format!(
            "{} is missing {}",
            keys_document.path.display(),
            private_key_name
        )
    })?;
    validate_private_key_list(&private_key)?;
    store.store_private_key(&public_key, &private_key)?;
    println!("imported {}", private_key_name);
    Ok(())
}

fn run_dotenv_export(
    options: &DotenvExportOptions,
    store: &dyn DotenvPrivateKeyStore,
) -> Result<(), String> {
    let previous_keys = previous_dotenv_keys();
    let Some(env_path) = nearest_dotenv_file(&options.cwd) else {
        print_shell_unload(options.shell, &previous_keys);
        return Ok(());
    };

    let env_digest = sha256_file_hex(&env_path)?;
    if env::var(AV_DOTENV_FILE_ENV).ok().as_deref() == env_path.to_str()
        && env::var(AV_DOTENV_DIGEST_ENV).ok().as_deref() == Some(env_digest.as_str())
    {
        return Ok(());
    }

    let loaded = load_dotenv_secrets(
        &env_path,
        DotenvApprovalMode::Export,
        &[],
        store,
        Some(&previous_keys),
    )?;
    print_shell_exports(options.shell, &previous_keys, &loaded);
    Ok(())
}

fn run_dotenv_run(
    options: &DotenvRunOptions,
    store: &dyn DotenvPrivateKeyStore,
) -> Result<(), String> {
    disable_dotenv_core_dumps()?;
    let command_line = std::iter::once(options.command.clone())
        .chain(options.args.clone())
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    let loaded = load_dotenv_secrets(
        &options.file,
        DotenvApprovalMode::Run,
        &command_line,
        store,
        None,
    )?;

    let mut command = Command::new(&options.command);
    command.args(&options.args);
    command.stdout(Stdio::piped());
    command.stderr(Stdio::piped());
    for (key, value) in &loaded.values {
        if env::var_os(key).is_none() {
            command.env(key, value);
        }
    }
    let mut child = command.spawn().map_err(|err| {
        format!(
            "failed to execute {}: {err}",
            options.command.to_string_lossy()
        )
    })?;
    let secrets = loaded
        .values
        .values()
        .filter(|value| !value.is_empty())
        .map(|value| value.as_bytes().to_vec())
        .collect::<Vec<_>>();
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let stdout_secrets = secrets.clone();
    let stdout_thread = stdout.map(|stream| {
        thread::spawn(move || stream_redacted_output(stream, io::stdout(), stdout_secrets))
    });
    let stderr_thread = stderr
        .map(|stream| thread::spawn(move || stream_redacted_output(stream, io::stderr(), secrets)));
    let status = child
        .wait()
        .map_err(|err| format!("failed to wait for dotenv command: {err}"))?;
    let mut redactions = 0;
    if let Some(handle) = stdout_thread {
        redactions += handle
            .join()
            .map_err(|_| "stdout redaction thread panicked".to_string())??;
    }
    if let Some(handle) = stderr_thread {
        redactions += handle
            .join()
            .map_err(|_| "stderr redaction thread panicked".to_string())??;
    }
    if redactions > 0 {
        eprintln!("av dotenv: redacted secret output");
    }
    if status.success() {
        Ok(())
    } else if let Some(code) = status.code() {
        process::exit(code);
    } else {
        Err("dotenv command terminated by signal".to_string())
    }
}

fn ensure_document_public_key(
    document: &mut DotenvDocument,
    store: &dyn DotenvPrivateKeyStore,
) -> Result<String, String> {
    if let Some((_name, public_key)) = document.public_key() {
        return Ok(public_key);
    }
    let keypair = generate_dotenv_keypair(&document.path);
    document.ensure_public_key(&keypair.public_key_name, &keypair.public_key);
    store.store_private_key(&keypair.public_key, &keypair.private_key)?;
    Ok(keypair.public_key)
}

fn load_dotenv_secrets(
    file: &Path,
    mode: DotenvApprovalMode,
    command: &[String],
    store: &dyn DotenvPrivateKeyStore,
    previous_av_keys: Option<&[String]>,
) -> Result<DotenvLoadedSecrets, String> {
    let document = DotenvDocument::load(file)?;
    let (_public_key_name, public_key) = document
        .public_key()
        .ok_or_else(|| format!("{} is missing DOTENV_PUBLIC_KEY", document.path.display()))?;
    let private_key = store.load_private_key(&public_key)?;
    validate_private_key_list(&private_key)?;
    let env_sha256 = sha256_file_hex(&document.path)?;
    let public_key_fingerprint = public_key_fingerprint(&public_key);
    let mut values = BTreeMap::new();
    for line in &document.lines {
        let Some(assignment) = &line.assignment else {
            continue;
        };
        if is_public_key_name(&assignment.key) || !is_valid_dotenv_key_name(&assignment.key) {
            continue;
        }
        if env_key_is_preexisting(&assignment.key, previous_av_keys) {
            continue;
        }
        values.insert(
            assignment.key.clone(),
            decrypt_dotenv_value(&assignment.key, &assignment.value, &private_key)?,
        );
    }
    let keys = values.keys().cloned().collect::<Vec<_>>();
    let project_root = document
        .path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    request_dotenv_approval_if_needed(
        mode,
        &document.path,
        &project_root,
        &env_sha256,
        &public_key_fingerprint,
        &keys,
        command,
    )?;
    Ok(DotenvLoadedSecrets {
        env_path: document.path,
        project_root,
        env_sha256,
        public_key_fingerprint,
        values,
    })
}

fn env_key_is_preexisting(key: &str, previous_av_keys: Option<&[String]>) -> bool {
    if env::var_os(key).is_none() {
        return false;
    }
    previous_av_keys
        .map(|keys| keys.iter().any(|previous| previous == key))
        .unwrap_or(false)
        == false
}

impl DotenvDocument {
    fn load(path: &Path) -> Result<Self, String> {
        let path = resolve_dotenv_path(path)?;
        let contents = fs::read_to_string(&path)
            .map_err(|err| format!("failed to read {}: {err}", path.display()))?;
        Ok(Self::parse(path, &contents))
    }

    fn load_or_empty(path: &Path) -> Result<Self, String> {
        let path = resolve_dotenv_path(path)?;
        match fs::read_to_string(&path) {
            Ok(contents) => Ok(Self::parse(path, &contents)),
            Err(err) if err.kind() == ErrorKind::NotFound => Ok(Self {
                path,
                lines: Vec::new(),
                had_trailing_newline: true,
            }),
            Err(err) => Err(format!("failed to read {}: {err}", path.display())),
        }
    }

    fn parse(path: PathBuf, contents: &str) -> Self {
        let normalized = contents.replace("\r\n", "\n").replace('\r', "\n");
        let had_trailing_newline = normalized.ends_with('\n');
        let mut lines = normalized
            .lines()
            .map(|raw| DotenvLine {
                raw: raw.to_string(),
                assignment: parse_dotenv_assignment(raw),
            })
            .collect::<Vec<_>>();
        if normalized.is_empty() {
            lines.clear();
        }
        Self {
            path,
            lines,
            had_trailing_newline,
        }
    }

    fn write(&self) -> Result<(), String> {
        if let Some(parent) = self.path.parent()
            && !parent.as_os_str().is_empty()
        {
            fs::create_dir_all(parent)
                .map_err(|err| format!("failed to create {}: {err}", parent.display()))?;
        }
        fs::write(&self.path, self.render())
            .map_err(|err| format!("failed to write {}: {err}", self.path.display()))
    }

    fn render(&self) -> String {
        let mut output = self
            .lines
            .iter()
            .map(|line| line.raw.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        if self.had_trailing_newline || !output.is_empty() {
            output.push('\n');
        }
        output
    }

    fn public_key(&self) -> Option<(String, String)> {
        self.lines.iter().find_map(|line| {
            let assignment = line.assignment.as_ref()?;
            if is_public_key_name(&assignment.key) && !assignment.value.is_empty() {
                Some((assignment.key.clone(), assignment.value.clone()))
            } else {
                None
            }
        })
    }

    fn value(&self, key: &str) -> Option<String> {
        self.lines.iter().find_map(|line| {
            let assignment = line.assignment.as_ref()?;
            if assignment.key == key {
                Some(assignment.value.clone())
            } else {
                None
            }
        })
    }

    fn ensure_public_key(&mut self, key: &str, value: &str) {
        let line = format_assignment(key, value);
        if self.lines.is_empty() {
            self.lines.extend(dotenv_header_lines());
            self.lines.push(DotenvLine {
                assignment: parse_dotenv_assignment(&line),
                raw: line,
            });
            return;
        }
        self.lines.insert(
            0,
            DotenvLine {
                assignment: parse_dotenv_assignment(&line),
                raw: line,
            },
        );
        for header in dotenv_header_lines().into_iter().rev() {
            self.lines.insert(0, header);
        }
    }

    fn set_value(&mut self, key: &str, value: &str) {
        let raw = format_assignment(key, value);
        for line in &mut self.lines {
            if line
                .assignment
                .as_ref()
                .is_some_and(|assignment| assignment.key == key)
            {
                line.raw = raw.clone();
                line.assignment = parse_dotenv_assignment(&raw);
                return;
            }
        }
        if !self.lines.is_empty()
            && self
                .lines
                .last()
                .is_some_and(|line| !line.raw.trim().is_empty())
        {
            self.lines.push(DotenvLine {
                raw: String::new(),
                assignment: None,
            });
        }
        self.lines.push(DotenvLine {
            assignment: parse_dotenv_assignment(&raw),
            raw,
        });
    }

    fn encryptable_keys(&self, include_keys: &[String], exclude_keys: &[String]) -> Vec<String> {
        let include = include_keys.iter().collect::<HashSet<_>>();
        let exclude = exclude_keys.iter().collect::<HashSet<_>>();
        let mut keys = Vec::new();
        for line in &self.lines {
            let Some(assignment) = &line.assignment else {
                continue;
            };
            if is_public_key_name(&assignment.key) || !is_valid_dotenv_key_name(&assignment.key) {
                continue;
            }
            if !include.is_empty() && !include.contains(&assignment.key) {
                continue;
            }
            if exclude.contains(&assignment.key) || is_encrypted_value(&assignment.value) {
                continue;
            }
            push_unique_string(&mut keys, assignment.key.clone());
        }
        keys
    }
}

fn resolve_dotenv_path(path: &Path) -> Result<PathBuf, String> {
    if path.exists() {
        fs::canonicalize(path).map_err(|err| format!("failed to resolve {}: {err}", path.display()))
    } else if path.is_absolute() {
        Ok(path.to_path_buf())
    } else {
        Ok(env::current_dir()
            .map_err(|err| format!("failed to resolve current directory: {err}"))?
            .join(path))
    }
}

fn parse_dotenv_assignment(raw: &str) -> Option<DotenvAssignment> {
    let trimmed = raw.trim_start();
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return None;
    }
    let assignment = trimmed.strip_prefix("export ").unwrap_or(trimmed);
    let equals = assignment.find('=');
    let colon = assignment.find(':').filter(|index| {
        assignment[*index + 1..]
            .chars()
            .next()
            .is_some_and(char::is_whitespace)
    });
    let sep = match (equals, colon) {
        (Some(eq), Some(col)) => Some(eq.min(col)),
        (Some(eq), None) => Some(eq),
        (None, Some(col)) => Some(col),
        (None, None) => None,
    }?;
    let key = assignment[..sep].trim();
    if key.is_empty() {
        return None;
    }
    let value_start = if assignment.as_bytes()[sep] == b':' {
        sep + 1
    } else {
        sep + 1
    };
    let value = parse_dotenv_value(&assignment[value_start..]);
    Some(DotenvAssignment {
        key: key.to_string(),
        value,
    })
}

fn parse_dotenv_value(value: &str) -> String {
    let trimmed = value.trim();
    let Some(first) = trimmed.chars().next() else {
        return String::new();
    };
    if matches!(first, '\'' | '"' | '`') {
        return parse_quoted_dotenv_value(trimmed, first);
    }
    trimmed
        .split_once('#')
        .map(|(head, _)| head.trim_end())
        .unwrap_or(trimmed)
        .to_string()
}

fn parse_quoted_dotenv_value(value: &str, quote: char) -> String {
    let mut escaped = false;
    let mut end = None;
    for (index, ch) in value.char_indices().skip(1) {
        if escaped {
            escaped = false;
            continue;
        }
        if ch == '\\' && quote != '\'' {
            escaped = true;
            continue;
        }
        if ch == quote {
            end = Some(index);
            break;
        }
    }
    let inner = match end {
        Some(index) => &value[1..index],
        None => &value[1..],
    };
    if quote == '"' {
        inner
            .replace("\\n", "\n")
            .replace("\\r", "\r")
            .replace("\\t", "\t")
            .replace("\\\"", "\"")
            .replace("\\\\", "\\")
    } else {
        inner.to_string()
    }
}

fn format_assignment(key: &str, value: &str) -> String {
    format!("{key}=\"{}\"", dotenv_double_quote_escape(value))
}

fn dotenv_double_quote_escape(value: &str) -> String {
    let mut escaped = String::new();
    for ch in value.chars() {
        match ch {
            '\\' => escaped.push_str("\\\\"),
            '"' => escaped.push_str("\\\""),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            _ => escaped.push(ch),
        }
    }
    escaped
}

fn dotenv_header_lines() -> Vec<DotenvLine> {
    [
        "# Agents! Hi! You can use these keys by",
        "# running `av dotenv run SCRIPT.ext`",
        "# Your human will be prompted to allow it",
        "# We will monitor output to make sure secrets don't escape!",
        "",
    ]
    .into_iter()
    .map(|raw| DotenvLine {
        raw: raw.to_string(),
        assignment: None,
    })
    .collect()
}

fn generate_dotenv_keypair(path: &Path) -> DotenvKeypair {
    let (private_key, public_key) = ecies::utils::generate_keypair();
    DotenvKeypair {
        public_key_name: public_key_name_for_file(path),
        public_key: encode_hex(&public_key.serialize_compressed()),
        private_key: encode_hex(&private_key.serialize()),
    }
}

fn public_key_name_for_file(path: &Path) -> String {
    let filename = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or(".env")
        .to_ascii_lowercase();
    let filename = filename.strip_suffix(".txt").unwrap_or(&filename);
    if filename == ".env" {
        return DOTENV_PUBLIC_KEY_PREFIX.to_string();
    }
    let parts = filename.split('.').collect::<Vec<_>>();
    let environment = match parts.get(2..) {
        Some([]) | None => filename.replacen(".env", "development", 1),
        Some([one]) => (*one).to_string(),
        Some([one, two]) => format!("{one}_{two}"),
        Some(rest) => rest[..2].join("_"),
    };
    format!(
        "{}_{}",
        DOTENV_PUBLIC_KEY_PREFIX,
        environment.to_ascii_uppercase()
    )
}

fn private_key_name_for_public_key_name(public_key_name: &str) -> String {
    public_key_name.replacen(DOTENV_PUBLIC_KEY_PREFIX, DOTENV_PRIVATE_KEY_PREFIX, 1)
}

fn encrypt_dotenv_value(value: &str, public_key: &str) -> Result<String, String> {
    let public_key = decode_hex(public_key)?;
    let encrypted = ecies::encrypt(&public_key, value.as_bytes())
        .map_err(|err| format!("failed to encrypt dotenv value: {err}"))?;
    Ok(format!("{ENCRYPTED_PREFIX}{}", BASE64.encode(encrypted)))
}

fn decrypt_dotenv_value(key: &str, value: &str, private_keys: &str) -> Result<String, String> {
    if !is_encrypted_value(value) {
        return Ok(value.to_string());
    }
    let encoded = value
        .strip_prefix(ENCRYPTED_PREFIX)
        .expect("checked encrypted prefix");
    let ciphertext = BASE64
        .decode(encoded)
        .map_err(|err| format!("could not decrypt {key}: malformed encrypted data: {err}"))?;
    let mut last_error = None;
    for private_key in private_keys
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        let private_key = match decode_hex(private_key) {
            Ok(value) => value,
            Err(err) => {
                last_error = Some(err);
                continue;
            }
        };
        match ecies::decrypt(&private_key, &ciphertext) {
            Ok(value) => {
                return String::from_utf8(value)
                    .map_err(|_| format!("could not decrypt {key}: plaintext is not UTF-8"));
            }
            Err(err) => last_error = Some(err.to_string()),
        }
    }
    Err(format!(
        "could not decrypt {key}: {}",
        last_error.unwrap_or_else(|| "missing private key".to_string())
    ))
}

fn is_encrypted_value(value: &str) -> bool {
    value.starts_with(ENCRYPTED_PREFIX) && value.len() > ENCRYPTED_PREFIX.len()
}

fn is_public_key_name(key: &str) -> bool {
    key == DOTENV_PUBLIC_KEY_PREFIX
        || key
            .strip_prefix(DOTENV_PUBLIC_KEY_PREFIX)
            .is_some_and(|suffix| suffix.starts_with('_'))
}

fn validate_dotenv_key_name(key: &str) -> Result<(), String> {
    if is_valid_dotenv_key_name(key) {
        Ok(())
    } else {
        Err(format!("invalid dotenv key name: {key}"))
    }
}

fn is_valid_dotenv_key_name(key: &str) -> bool {
    let mut chars = key.chars();
    match chars.next() {
        Some(first) if first == '_' || first.is_ascii_alphabetic() => {}
        _ => return false,
    }
    chars.all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
}

fn validate_private_key_list(value: &str) -> Result<(), String> {
    for key in value
        .split(',')
        .map(str::trim)
        .filter(|key| !key.is_empty())
    {
        let decoded = decode_hex(key)?;
        if decoded.len() != 32 {
            return Err("dotenv private key must be 32 bytes".to_string());
        }
    }
    Ok(())
}

fn public_key_fingerprint(public_key: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(public_key.as_bytes());
    encode_hex(&hasher.finalize())
}

fn keychain_account_for_public_key(public_key: &str) -> String {
    format!("DOTENV_PRIVATE_KEY:{}", public_key_fingerprint(public_key))
}

fn sha256_file_hex(path: &Path) -> Result<String, String> {
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
    Ok(encode_hex(&hasher.finalize()))
}

fn encode_hex(bytes: &[u8]) -> String {
    const TABLE: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(TABLE[(byte >> 4) as usize] as char);
        output.push(TABLE[(byte & 0x0f) as usize] as char);
    }
    output
}

fn decode_hex(value: &str) -> Result<Vec<u8>, String> {
    let value = value.trim();
    let value = value
        .strip_prefix("0x")
        .or_else(|| value.strip_prefix("0X"))
        .unwrap_or(value);
    if value.len() % 2 != 0 {
        return Err("hex value must have an even number of characters".to_string());
    }
    let mut bytes = Vec::with_capacity(value.len() / 2);
    let raw = value.as_bytes();
    for chunk in raw.chunks(2) {
        let high = hex_nibble(chunk[0])?;
        let low = hex_nibble(chunk[1])?;
        bytes.push((high << 4) | low);
    }
    Ok(bytes)
}

fn hex_nibble(byte: u8) -> Result<u8, String> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        b'A'..=b'F' => Ok(byte - b'A' + 10),
        _ => Err("hex value contains non-hex characters".to_string()),
    }
}

fn nearest_dotenv_file(cwd: &Path) -> Option<PathBuf> {
    let mut current = fs::canonicalize(cwd).ok()?;
    loop {
        let candidate = current.join(".env");
        if candidate.is_file() {
            return Some(candidate);
        }
        if !current.pop() {
            return None;
        }
    }
}

fn previous_dotenv_keys() -> Vec<String> {
    env::var(AV_DOTENV_KEYS_ENV)
        .ok()
        .map(|value| {
            value
                .split(':')
                .filter(|key| is_valid_dotenv_key_name(key))
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn print_shell_unload(shell: DotenvShell, previous_keys: &[String]) {
    match shell {
        DotenvShell::Bash | DotenvShell::Zsh => {
            for key in previous_keys {
                println!("unset {key}");
            }
            println!("unset {AV_DOTENV_FILE_ENV}");
            println!("unset {AV_DOTENV_DIGEST_ENV}");
            println!("unset {AV_DOTENV_KEYS_ENV}");
        }
        DotenvShell::Fish => {
            for key in previous_keys {
                println!("set -e {key};");
            }
            println!("set -e {AV_DOTENV_FILE_ENV};");
            println!("set -e {AV_DOTENV_DIGEST_ENV};");
            println!("set -e {AV_DOTENV_KEYS_ENV};");
        }
    }
}

fn print_shell_exports(shell: DotenvShell, previous_keys: &[String], loaded: &DotenvLoadedSecrets) {
    print_shell_unload(shell, previous_keys);
    let keys = loaded.values.keys().cloned().collect::<Vec<_>>();
    match shell {
        DotenvShell::Bash | DotenvShell::Zsh => {
            for (key, value) in &loaded.values {
                println!("export {key}={}", shell_quote(value));
            }
            println!(
                "export {AV_DOTENV_FILE_ENV}={}",
                shell_quote(loaded.env_path.to_string_lossy().as_ref())
            );
            println!(
                "export {AV_DOTENV_DIGEST_ENV}={}",
                shell_quote(&loaded.env_sha256)
            );
            println!(
                "export {AV_DOTENV_KEYS_ENV}={}",
                shell_quote(&keys.join(":"))
            );
        }
        DotenvShell::Fish => {
            for (key, value) in &loaded.values {
                println!("set -gx {key} {};", shell_quote(value));
            }
            println!(
                "set -gx {AV_DOTENV_FILE_ENV} {};",
                shell_quote(loaded.env_path.to_string_lossy().as_ref())
            );
            println!(
                "set -gx {AV_DOTENV_DIGEST_ENV} {};",
                shell_quote(&loaded.env_sha256)
            );
            println!(
                "set -gx {AV_DOTENV_KEYS_ENV} {};",
                shell_quote(&keys.join(":"))
            );
        }
    }
}

fn request_dotenv_approval_if_needed(
    mode: DotenvApprovalMode,
    env_path: &Path,
    project_root: &Path,
    env_sha256: &str,
    public_key_fingerprint: &str,
    keys: &[String],
    command: &[String],
) -> Result<(), String> {
    let entry = DotenvRememberedApprovalEntry {
        mode,
        env_file_path: env_path.to_string_lossy().into_owned(),
        project_root: project_root.to_string_lossy().into_owned(),
        env_sha256: env_sha256.to_string(),
        public_key_fingerprint: public_key_fingerprint.to_string(),
        keys: keys.to_vec(),
    };
    if load_dotenv_remembered_approvals()?.contains(&entry) {
        return Ok(());
    }
    request_dotenv_approval(&entry, command)?;
    remember_dotenv_approval(entry)
}

fn request_dotenv_approval(
    entry: &DotenvRememberedApprovalEntry,
    command: &[String],
) -> Result<(), String> {
    let request_id = format!(
        "{}-{}",
        process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|err| format!("failed to compute request timestamp: {err}"))?
            .as_millis()
    );
    let request = DotenvApprovalRequestSnapshot {
        id: request_id.clone(),
        mode: entry.mode,
        env_file_path: entry.env_file_path.clone(),
        project_root: entry.project_root.clone(),
        env_sha256: entry.env_sha256.clone(),
        public_key_fingerprint: entry.public_key_fingerprint.clone(),
        keys: entry.keys.clone(),
        cwd: env::current_dir()
            .map_err(|err| format!("failed to resolve current directory: {err}"))?
            .to_string_lossy()
            .into_owned(),
        parent_process: dotenv_parent_process_snapshot(),
        command: command.to_vec(),
    };
    let pending_url = dotenv_pending_approval_path()?;
    write_dotenv_json(&pending_url, &request)?;
    if let Err(err) = ping_dotenv_approval_app() {
        let _ = fs::remove_file(&pending_url);
        return Err(err);
    }
    wait_for_dotenv_decision(&request_id)
}

fn wait_for_dotenv_decision(id: &str) -> Result<(), String> {
    let decision_url = dotenv_decision_path(id)?;
    let pending_url = dotenv_pending_approval_path()?;
    loop {
        if let Ok(contents) = fs::read_to_string(&decision_url) {
            let decision: DotenvApprovalDecision = serde_json::from_str(&contents)
                .map_err(|err| format!("failed to decode dotenv approval decision: {err}"))?;
            if decision.id != id {
                return Err("dotenv approval decision id mismatch".to_string());
            }
            let _ = fs::remove_file(&pending_url);
            let _ = fs::remove_file(&decision_url);
            if decision.approved {
                return Ok(());
            }
            return Err(decision
                .reason
                .unwrap_or_else(|| "dotenv approval denied".to_string()));
        }
        thread::sleep(Duration::from_millis(250));
    }
}

fn load_dotenv_remembered_approvals() -> Result<DotenvRememberedApprovalStore, String> {
    let path = dotenv_remembered_approvals_path()?;
    if !path.exists() {
        return Ok(DotenvRememberedApprovalStore::default());
    }
    let contents = fs::read_to_string(&path)
        .map_err(|err| format!("failed to read {}: {err}", path.display()))?;
    serde_json::from_str(&contents)
        .map_err(|err| format!("failed to decode {}: {err}", path.display()))
}

fn remember_dotenv_approval(entry: DotenvRememberedApprovalEntry) -> Result<(), String> {
    let mut store = load_dotenv_remembered_approvals()?;
    store.remember(entry);
    write_dotenv_json(&dotenv_remembered_approvals_path()?, &store)
}

fn dotenv_parent_process_snapshot() -> DotenvParentProcessSnapshot {
    let pid = unsafe { libc::getppid() };
    let executable_path = dotenv_parent_process_path(pid);
    let display_name = executable_path
        .as_deref()
        .and_then(|path| Path::new(path).file_name())
        .and_then(|name| name.to_str())
        .map(str::to_string);
    DotenvParentProcessSnapshot {
        pid,
        executable_path,
        display_name,
    }
}

#[cfg(target_os = "macos")]
fn dotenv_parent_process_path(pid: i32) -> Option<String> {
    let output = Command::new("/bin/ps")
        .args(["-p", &pid.to_string(), "-o", "comm="])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let path = String::from_utf8(output.stdout).ok()?.trim().to_string();
    if path.is_empty() { None } else { Some(path) }
}

#[cfg(not(target_os = "macos"))]
fn dotenv_parent_process_path(_pid: i32) -> Option<String> {
    None
}

fn dotenv_user_approval_root() -> Result<PathBuf, String> {
    let home = env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())?;
    Ok(home
        .join("Library")
        .join("Application Support")
        .join("Automic Vault")
        .join(DOTENV_USER_APPROVAL_SUBDIR))
}

fn dotenv_pending_approval_path() -> Result<PathBuf, String> {
    Ok(dotenv_user_approval_root()?.join("pending-approval.json"))
}

fn dotenv_decision_path(id: &str) -> Result<PathBuf, String> {
    Ok(dotenv_user_approval_root()?
        .join("decisions")
        .join(format!("{id}.json")))
}

fn dotenv_remembered_approvals_path() -> Result<PathBuf, String> {
    Ok(dotenv_user_approval_root()?.join(DOTENV_REMEMBERED_APPROVALS))
}

fn write_dotenv_json<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| format!("invalid dotenv approval path {}", path.display()))?;
    fs::create_dir_all(parent)
        .map_err(|err| format!("failed to create {}: {err}", parent.display()))?;
    let payload = serde_json::to_vec_pretty(value)
        .map_err(|err| format!("failed to encode dotenv approval JSON: {err}"))?;
    fs::write(path, payload).map_err(|err| format!("failed to write {}: {err}", path.display()))
}

#[cfg(target_os = "macos")]
fn ping_dotenv_approval_app() -> Result<(), String> {
    let status = Command::new("/usr/bin/open")
        .args(["-b", GUI_APP_BUNDLE_IDENTIFIER])
        .status()
        .map_err(|err| format!("failed to ping Automic Vault.app: {err}"))?;
    if !status.success() {
        return Err("failed to ping Automic Vault.app for dotenv approval".to_string());
    }
    dotenv_post_distributed_notification(DOTENV_APPROVAL_NOTIFICATION)
}

#[cfg(not(target_os = "macos"))]
fn ping_dotenv_approval_app() -> Result<(), String> {
    Err("dotenv approvals are only available on macOS".to_string())
}

fn stream_redacted_output<R, W>(
    mut reader: R,
    mut writer: W,
    secrets: Vec<Vec<u8>>,
) -> Result<usize, String>
where
    R: Read,
    W: Write,
{
    let mut redactor = DotenvRedactor::new(secrets);
    let mut buffer = [0_u8; 8192];
    loop {
        let count = reader
            .read(&mut buffer)
            .map_err(|err| format!("failed to read process output: {err}"))?;
        if count == 0 {
            break;
        }
        let chunk = redactor.feed(&buffer[..count], false);
        writer
            .write_all(&chunk)
            .and_then(|_| writer.flush())
            .map_err(|err| format!("failed to write redacted output: {err}"))?;
    }
    let chunk = redactor.feed(&[], true);
    writer
        .write_all(&chunk)
        .and_then(|_| writer.flush())
        .map_err(|err| format!("failed to write redacted output: {err}"))?;
    Ok(redactor.redacted)
}

impl DotenvRedactor {
    fn new(mut secrets: Vec<Vec<u8>>) -> Self {
        secrets.retain(|secret| !secret.is_empty());
        secrets.sort_by_key(|secret| std::cmp::Reverse(secret.len()));
        secrets.dedup();
        let hold_len = secrets
            .iter()
            .map(Vec::len)
            .max()
            .unwrap_or(1)
            .saturating_sub(1);
        Self {
            secrets,
            pending: Vec::new(),
            redacted: 0,
            hold_len,
        }
    }

    fn feed(&mut self, chunk: &[u8], final_chunk: bool) -> Vec<u8> {
        self.pending.extend_from_slice(chunk);
        let process_len = if final_chunk {
            self.pending.len()
        } else {
            self.pending.len().saturating_sub(self.hold_len)
        };
        let process = self.pending[..process_len].to_vec();
        self.pending = self.pending[process_len..].to_vec();
        self.redact_bytes(&process)
    }

    fn redact_bytes(&mut self, input: &[u8]) -> Vec<u8> {
        if self.secrets.is_empty() {
            return input.to_vec();
        }
        let mut output = Vec::with_capacity(input.len());
        let mut index = 0;
        while index < input.len() {
            if let Some(secret) = self
                .secrets
                .iter()
                .find(|secret| input[index..].starts_with(secret.as_slice()))
            {
                output.extend_from_slice(b"[REDACTED]");
                index += secret.len();
                self.redacted += 1;
            } else {
                output.push(input[index]);
                index += 1;
            }
        }
        output
    }
}

fn read_dotenv_secret() -> Result<String, String> {
    let mut stdin = io::stdin();
    let mut value = String::new();
    if stdin.is_terminal() {
        eprint!("Secret: ");
        io::stderr()
            .flush()
            .map_err(|err| format!("failed to flush prompt: {err}"))?;
        read_dotenv_secret_line_no_echo(&mut stdin, &mut value)?;
        eprintln!();
    } else {
        stdin
            .read_to_string(&mut value)
            .map_err(|err| format!("failed to read secret from stdin: {err}"))?;
    }
    let value = value.trim().to_string();
    if value.is_empty() {
        return Err("empty dotenv secret value".to_string());
    }
    Ok(value)
}

fn read_dotenv_secret_line_no_echo(
    stdin: &mut io::Stdin,
    value: &mut String,
) -> Result<(), String> {
    let fd = stdin.as_raw_fd();
    let mut termios = std::mem::MaybeUninit::<libc::termios>::uninit();
    if unsafe { libc::tcgetattr(fd, termios.as_mut_ptr()) } != 0 {
        return Err(format!(
            "failed to read terminal settings: {}",
            io::Error::last_os_error()
        ));
    }
    let original = unsafe { termios.assume_init() };
    let mut hidden = original;
    hidden.c_lflag &= !libc::ECHO;
    if unsafe { libc::tcsetattr(fd, libc::TCSANOW, &hidden) } != 0 {
        return Err(format!(
            "failed to disable terminal echo: {}",
            io::Error::last_os_error()
        ));
    }
    let read_result = stdin.read_line(value);
    let restore_result = unsafe { libc::tcsetattr(fd, libc::TCSANOW, &original) };
    if restore_result != 0 {
        return Err(format!(
            "failed to restore terminal echo: {}",
            io::Error::last_os_error()
        ));
    }
    read_result.map_err(|err| format!("failed to read secret: {err}"))?;
    Ok(())
}

fn disable_dotenv_core_dumps() -> Result<(), String> {
    let limit = libc::rlimit {
        rlim_cur: 0,
        rlim_max: 0,
    };
    if unsafe { libc::setrlimit(libc::RLIMIT_CORE, &limit) } == 0 {
        Ok(())
    } else {
        Err(format!(
            "failed to disable core dumps: {}",
            io::Error::last_os_error()
        ))
    }
}

fn print_dotenv_hook(program_name: &str, shell: DotenvShell) {
    match shell {
        DotenvShell::Bash => println!(
            r#"__av_dotenv_hook() {{
  local __av_dotenv
  __av_dotenv="$({program_name} export --shell bash --cwd "$PWD")" || return $?
  eval "$__av_dotenv"
}}
if [[ -n "${{PROMPT_COMMAND:-}}" ]]; then
  PROMPT_COMMAND="__av_dotenv_hook; $PROMPT_COMMAND"
else
  PROMPT_COMMAND="__av_dotenv_hook"
fi
__av_dotenv_hook"#
        ),
        DotenvShell::Zsh => println!(
            r#"__av_dotenv_hook() {{
  local __av_dotenv
  __av_dotenv="$({program_name} export --shell zsh --cwd "$PWD")" || return $?
  eval "$__av_dotenv"
}}
autoload -Uz add-zsh-hook
add-zsh-hook chpwd __av_dotenv_hook
__av_dotenv_hook"#
        ),
        DotenvShell::Fish => println!(
            r#"function __av_dotenv_hook --on-variable PWD
  {program_name} export --shell fish --cwd "$PWD" | source
end
__av_dotenv_hook"#
        ),
    }
}

pub(crate) fn print_dotenv_usage(program_name: &str) {
    println!(
        "\
Usage: {program_name} <init|set|encrypt|import|hook|export|run> [options]

Loads encrypted dotenvx-compatible .env files with Automic Vault approval."
    );
}

fn print_dotenv_init_usage(program_name: &str) {
    println!("Usage: {program_name} init [--file .env]");
}

fn print_dotenv_set_usage(program_name: &str) {
    println!("Usage: {program_name} set [--file .env] KEY");
}

fn print_dotenv_encrypt_usage(program_name: &str) {
    println!(
        "Usage: {program_name} encrypt [--file .env] [--key KEY...] [--exclude-key KEY...] [--check]"
    );
}

fn print_dotenv_import_usage(program_name: &str) {
    println!("Usage: {program_name} import [--file .env] [--keys-file .env.keys]");
}

fn print_dotenv_hook_usage(program_name: &str) {
    println!("Usage: {program_name} hook zsh|bash|fish");
}

fn print_dotenv_export_usage(program_name: &str) {
    println!("Usage: {program_name} export --shell zsh|bash|fish [--cwd <path>]");
}

fn print_dotenv_run_usage(program_name: &str) {
    println!("Usage: {program_name} run [--file .env] [--] <command> [args...]");
}

impl DotenvPrivateKeyStore for KeychainDotenvPrivateKeyStore {
    fn load_private_key(&self, public_key: &str) -> Result<String, String> {
        let account = keychain_account_for_public_key(public_key);
        keychain_read_dotenv_private_key(DOTENV_KEYCHAIN_SERVICE, &account)
    }

    fn store_private_key(&self, public_key: &str, private_key: &str) -> Result<(), String> {
        let account = keychain_account_for_public_key(public_key);
        keychain_write_dotenv_private_key(DOTENV_KEYCHAIN_SERVICE, &account, private_key)
    }
}

#[cfg(target_os = "macos")]
fn keychain_read_dotenv_private_key(service: &str, account: &str) -> Result<String, String> {
    unsafe extern "C" {
        fn isotope_copy_generic_password_json_with_status(
            service_cstr: *const c_char,
            account_cstr: *const c_char,
            error_cstr: *mut *mut c_char,
            status_out: *mut c_int,
        ) -> *mut c_char;
    }
    let service_cstr =
        CString::new(service).map_err(|_| "invalid keychain service name".to_string())?;
    let account_cstr =
        CString::new(account).map_err(|_| "invalid keychain account name".to_string())?;
    let mut error = std::ptr::null_mut();
    let mut status = 0;
    let value = unsafe {
        isotope_copy_generic_password_json_with_status(
            service_cstr.as_ptr(),
            account_cstr.as_ptr(),
            &mut error,
            &mut status,
        )
    };
    if value.is_null() {
        let message = unsafe { take_dotenv_bridge_string(error) }
            .unwrap_or_else(|| "keychain lookup failed".to_string());
        if status == ERR_SEC_ITEM_NOT_FOUND {
            return Err(format!(
                "failed to load dotenv private key: {message}. Run av dotenv import or av dotenv init."
            ));
        }
        return Err(format!("failed to load dotenv private key: {message}"));
    }
    unsafe { take_dotenv_bridge_string(value) }
        .ok_or_else(|| "keychain returned invalid UTF-8".to_string())
}

#[cfg(not(target_os = "macos"))]
fn keychain_read_dotenv_private_key(_service: &str, _account: &str) -> Result<String, String> {
    Err("dotenv keychain integration is only available on macOS".to_string())
}

#[cfg(target_os = "macos")]
fn keychain_write_dotenv_private_key(
    service: &str,
    account: &str,
    value: &str,
) -> Result<(), String> {
    unsafe extern "C" {
        fn isotope_store_generic_password_json(
            service_cstr: *const c_char,
            account_cstr: *const c_char,
            value_cstr: *const c_char,
            error_cstr: *mut *mut c_char,
        ) -> bool;
    }
    let service_cstr =
        CString::new(service).map_err(|_| "invalid keychain service name".to_string())?;
    let account_cstr =
        CString::new(account).map_err(|_| "invalid keychain account name".to_string())?;
    let value_cstr = CString::new(value).map_err(|_| "invalid keychain private key".to_string())?;
    let mut error = std::ptr::null_mut();
    if unsafe {
        isotope_store_generic_password_json(
            service_cstr.as_ptr(),
            account_cstr.as_ptr(),
            value_cstr.as_ptr(),
            &mut error,
        )
    } {
        return Ok(());
    }
    let message = unsafe { take_dotenv_bridge_string(error) }
        .unwrap_or_else(|| "keychain write failed".to_string());
    Err(format!("failed to store dotenv private key: {message}"))
}

#[cfg(not(target_os = "macos"))]
fn keychain_write_dotenv_private_key(
    _service: &str,
    _account: &str,
    _value: &str,
) -> Result<(), String> {
    Err("dotenv keychain integration is only available on macOS".to_string())
}

#[cfg(target_os = "macos")]
fn dotenv_post_distributed_notification(name: &str) -> Result<(), String> {
    unsafe extern "C" {
        fn isotope_post_distributed_notification(
            name_cstr: *const c_char,
            error_cstr: *mut *mut c_char,
        ) -> bool;
    }
    let name_cstr =
        CString::new(name).map_err(|_| "invalid distributed notification name".to_string())?;
    let mut error = std::ptr::null_mut();
    if unsafe { isotope_post_distributed_notification(name_cstr.as_ptr(), &mut error) } {
        return Ok(());
    }
    Err(unsafe { take_dotenv_bridge_string(error) }
        .unwrap_or_else(|| "failed to post dotenv approval notification".to_string()))
}

#[cfg(target_os = "macos")]
unsafe fn take_dotenv_bridge_string(value: *mut c_char) -> Option<String> {
    if value.is_null() {
        return None;
    }
    unsafe extern "C" {
        fn isotope_free_c_string(value: *mut c_char);
    }
    let bytes = unsafe { std::ffi::CStr::from_ptr(value) }
        .to_str()
        .ok()
        .map(str::to_owned);
    unsafe { isotope_free_c_string(value) };
    bytes
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(unix)]
    use std::os::unix::ffi::OsStringExt;
    use std::sync::Mutex;

    #[derive(Default)]
    struct StubDotenvPrivateKeyStore {
        private_keys: Mutex<BTreeMap<String, String>>,
    }

    impl DotenvPrivateKeyStore for StubDotenvPrivateKeyStore {
        fn load_private_key(&self, public_key: &str) -> Result<String, String> {
            self.private_keys
                .lock()
                .unwrap()
                .get(public_key)
                .cloned()
                .ok_or_else(|| "missing private key".to_string())
        }

        fn store_private_key(&self, public_key: &str, private_key: &str) -> Result<(), String> {
            self.private_keys
                .lock()
                .unwrap()
                .insert(public_key.to_string(), private_key.to_string());
            Ok(())
        }
    }

    struct DotenvEnvGuard {
        previous: Vec<(String, Option<OsString>)>,
    }

    impl DotenvEnvGuard {
        fn set(values: &[(&str, &str)]) -> Self {
            let previous = values
                .iter()
                .map(|(key, value)| {
                    let previous = env::var_os(key);
                    unsafe {
                        env::set_var(key, value);
                    }
                    ((*key).to_string(), previous)
                })
                .collect();
            Self { previous }
        }

        fn unset(keys: &[&str]) -> Self {
            let previous = keys
                .iter()
                .map(|key| {
                    let previous = env::var_os(key);
                    unsafe {
                        env::remove_var(key);
                    }
                    ((*key).to_string(), previous)
                })
                .collect();
            Self { previous }
        }
    }

    impl Drop for DotenvEnvGuard {
        fn drop(&mut self) {
            for (key, previous) in self.previous.drain(..).rev() {
                match previous {
                    Some(value) => unsafe {
                        env::set_var(&key, value);
                    },
                    None => unsafe {
                        env::remove_var(&key);
                    },
                }
            }
        }
    }

    fn remembered_entry_for(
        env_path: &Path,
        mode: DotenvApprovalMode,
        public_key: &str,
        keys: &[&str],
    ) -> DotenvRememberedApprovalEntry {
        let env_path = fs::canonicalize(env_path).unwrap();
        DotenvRememberedApprovalEntry {
            mode,
            env_file_path: env_path.to_string_lossy().into_owned(),
            project_root: env_path.parent().unwrap().to_string_lossy().into_owned(),
            env_sha256: sha256_file_hex(&env_path).unwrap(),
            public_key_fingerprint: public_key_fingerprint(public_key),
            keys: keys.iter().map(|key| (*key).to_string()).collect(),
        }
    }

    #[test]
    fn dotenv_parse_handles_comments_quotes_and_public_key() {
        let doc = DotenvDocument::parse(
            PathBuf::from(".env"),
            "DOTENV_PUBLIC_KEY=abc\nFOO=\"bar\\n baz\" # comment\nexport BAR='literal#x'\n",
        );
        assert_eq!(
            doc.public_key(),
            Some(("DOTENV_PUBLIC_KEY".to_string(), "abc".to_string()))
        );
        assert_eq!(doc.value("FOO").unwrap(), "bar\n baz");
        assert_eq!(doc.value("BAR").unwrap(), "literal#x");
    }

    #[test]
    fn dotenv_document_preserves_comments_when_setting() {
        let mut doc =
            DotenvDocument::parse(PathBuf::from(".env"), "# hello\nFOO=old\n\nBAR=keep\n");
        doc.set_value("FOO", "new");
        doc.set_value("BAZ", "space value");
        let rendered = doc.render();
        assert!(rendered.contains("# hello\n"));
        assert!(rendered.contains("FOO=\"new\"\n"));
        assert!(rendered.contains("BAR=keep\n"));
        assert!(rendered.contains("BAZ=\"space value\"\n"));
    }

    #[test]
    fn dotenv_crypto_roundtrips_with_generated_keypair() {
        let keypair = generate_dotenv_keypair(Path::new(".env"));
        let encrypted = encrypt_dotenv_value("secret value", &keypair.public_key).unwrap();
        assert!(encrypted.starts_with(ENCRYPTED_PREFIX));
        let decrypted = decrypt_dotenv_value("FOO", &encrypted, &keypair.private_key).unwrap();
        assert_eq!(decrypted, "secret value");
    }

    #[test]
    fn dotenv_known_eciesjs_fixture_decrypts() {
        let private_key = "e520872701d9ec44dbac2eab85512ad14ad0c42e01de56d7b528abd8524fcb47";
        let encrypted = "encrypted:BHvhiFrrSNTU2wyZKZZyXTJkeE/viMW2B4L40PlAwhMif8P5BPhG1ew9D7pmU3VFAejrrcQhqjiSog/vM8/wIGBHBYpM+0776ulrLQGbSrLtzjMyh0ig0AimnI9YFrctRb2bWkG7bqASerIwV+xvzQ==";
        let decrypted = decrypt_dotenv_value("HELLO", encrypted, private_key).unwrap();
        assert_eq!(decrypted, "hello world\u{1f30d}");
    }

    #[test]
    fn dotenv_redactor_catches_chunk_boundaries() {
        let mut redactor = DotenvRedactor::new(vec![b"secret-token".to_vec()]);
        let mut out = redactor.feed(b"before secret", false);
        out.extend(redactor.feed(b"-token after", true));
        assert_eq!(String::from_utf8(out).unwrap(), "before [REDACTED] after");
        assert_eq!(redactor.redacted, 1);

        let mut redactor = DotenvRedactor::new(vec![b"secret-token".to_vec()]);
        let mut out = redactor.feed(b"secret", false);
        out.extend(redactor.feed(b"-token", true));
        assert_eq!(String::from_utf8(out).unwrap(), "[REDACTED]");
        assert_eq!(redactor.redacted, 1);
    }

    #[test]
    fn dotenv_shell_exports_unset_previous_keys() {
        let loaded = DotenvLoadedSecrets {
            env_path: PathBuf::from("/tmp/project/.env"),
            project_root: PathBuf::from("/tmp/project"),
            env_sha256: "abc".to_string(),
            public_key_fingerprint: "def".to_string(),
            values: BTreeMap::from([("FOO".to_string(), "bar baz".to_string())]),
        };
        assert_eq!(shell_quote("bar baz"), "'bar baz'");
        assert_eq!(loaded.values["FOO"], "bar baz");
    }

    #[test]
    fn dotenv_parse_encrypt_options_collects_multiple_keys() {
        let options = parse_dotenv_encrypt(
            "av dotenv",
            vec![
                OsString::from("--key"),
                OsString::from("FOO"),
                OsString::from("BAR"),
                OsString::from("--exclude-key"),
                OsString::from("BAZ"),
                OsString::from("--check"),
            ]
            .into_iter(),
        )
        .unwrap()
        .unwrap();
        assert_eq!(options.include_keys, vec!["BAR", "FOO"]);
        assert_eq!(options.exclude_keys, vec!["BAZ"]);
        assert!(options.check);
    }

    #[test]
    fn dotenv_init_and_encrypt_use_stub_store() {
        let temp = TempDir::new().unwrap();
        let env_path = temp.path().join(".env");
        fs::write(&env_path, "FOO=bar\n").unwrap();
        let store = StubDotenvPrivateKeyStore::default();
        run_dotenv_encrypt(
            &DotenvEncryptOptions {
                file: env_path.clone(),
                include_keys: Vec::new(),
                exclude_keys: Vec::new(),
                check: false,
            },
            &store,
        )
        .unwrap();
        let output = fs::read_to_string(env_path).unwrap();
        assert!(output.contains("DOTENV_PUBLIC_KEY"));
        assert!(output.contains("FOO=\"encrypted:"));
    }

    #[test]
    fn dotenv_encrypt_provisions_key_without_plaintext() {
        let temp = TempDir::new().unwrap();
        let env_path = temp.path().join(".env");
        fs::write(&env_path, "# comments only\n").unwrap();
        let store = StubDotenvPrivateKeyStore::default();

        run_dotenv_encrypt(
            &DotenvEncryptOptions {
                file: env_path.clone(),
                include_keys: Vec::new(),
                exclude_keys: Vec::new(),
                check: false,
            },
            &store,
        )
        .unwrap();

        let output = fs::read_to_string(env_path).unwrap();
        assert!(output.contains("DOTENV_PUBLIC_KEY"));
        assert!(output.contains("# comments only"));
    }

    #[test]
    #[cfg(unix)]
    #[test]
    fn dotenv_command_parsers_cover_help_version_and_error_edges() {
            parse_dotenv_command("av dotenv", Vec::<OsString>::new().into_iter()).unwrap_err(),
            "missing dotenv command"
        );
        assert!(
            parse_dotenv_command("av dotenv", [OsString::from("--help")].into_iter())
                .unwrap()
                .is_none()
        );
        assert!(
            parse_dotenv_command("av dotenv", [OsString::from("--version")].into_iter())
                .unwrap()
                .is_none()
        );
        assert_eq!(
            parse_dotenv_command("av dotenv", [OsString::from_vec(vec![0xff])].into_iter())
                .unwrap_err(),
            "dotenv command must be valid UTF-8"
        );
        assert_eq!(
            parse_dotenv_command("av dotenv", [OsString::from("bogus")].into_iter()).unwrap_err(),
            "unknown dotenv command 'bogus'"
        );

        assert!(
            parse_dotenv_set("av dotenv", [OsString::from("--help")].into_iter())
                .unwrap()
                .is_none()
        );
        assert!(
            parse_dotenv_set("av dotenv", [OsString::from("--version")].into_iter())
                .unwrap()
                .is_none()
        );
        assert_eq!(
            parse_dotenv_set("av dotenv", Vec::<OsString>::new().into_iter()).unwrap_err(),
            "missing KEY"
        );
        assert_eq!(
            parse_dotenv_set(
                "av dotenv",
                [OsString::from("FOO"), OsString::from("BAR")].into_iter(),
            )
            .unwrap_err(),
            "dotenv set supports one KEY"
        );
        assert_eq!(
            parse_dotenv_set("av dotenv", [OsString::from("1BAD")].into_iter()).unwrap_err(),
            "invalid dotenv key name: 1BAD"
        );
        assert_eq!(
            parse_dotenv_set("av dotenv", [OsString::from_vec(vec![0xff])].into_iter())
                .unwrap_err(),
            "dotenv set key must be valid UTF-8"
        );
        let set = parse_dotenv_set(
            "av dotenv",
            [
                OsString::from("-f"),
                OsString::from("custom.env"),
                OsString::from("FOO"),
            ]
            .into_iter(),
        )
        .unwrap()
        .unwrap();
        assert_eq!(set.file, PathBuf::from("custom.env"));
        assert_eq!(set.key, "FOO");

        assert!(
            parse_dotenv_encrypt("av dotenv", [OsString::from("--help")].into_iter())
                .unwrap()
                .is_none()
        );
        assert!(
            parse_dotenv_encrypt("av dotenv", [OsString::from("--version")].into_iter())
                .unwrap()
                .is_none()
        );
        assert_eq!(
            parse_dotenv_encrypt("av dotenv", [OsString::from("--key")].into_iter()).unwrap_err(),
            "missing value for --key"
        );
        assert_eq!(
            parse_dotenv_encrypt("av dotenv", [OsString::from("--exclude-key")].into_iter())
                .unwrap_err(),
            "missing value for --exclude-key"
        );
        assert_eq!(
            parse_dotenv_encrypt(
                "av dotenv",
                [OsString::from("--key"), OsString::from("BAD-NAME")].into_iter(),
            )
            .unwrap_err(),
            "invalid dotenv key name: BAD-NAME"
        );
        assert_eq!(
            parse_dotenv_encrypt("av dotenv", [OsString::from("--unknown")].into_iter())
                .unwrap_err(),
            "unknown dotenv encrypt argument '--unknown'"
        );
        assert_eq!(
            parse_dotenv_encrypt("av dotenv", [OsString::from("--file")].into_iter()).unwrap_err(),
            "missing value for --file"
        );

        assert!(
            parse_dotenv_import("av dotenv", [OsString::from("--help")].into_iter())
                .unwrap()
                .is_none()
        );
        assert!(
            parse_dotenv_import("av dotenv", [OsString::from("--version")].into_iter())
                .unwrap()
                .is_none()
        );
        assert_eq!(
            parse_dotenv_import("av dotenv", [OsString::from("--unknown")].into_iter())
                .unwrap_err(),
            "unknown dotenv import argument '--unknown'"
        );
        let import = parse_dotenv_import(
            "av dotenv",
            [OsString::from("--file"), OsString::from("dir/.env.prod")].into_iter(),
        )
        .unwrap()
        .unwrap();
        assert_eq!(import.keys_file, PathBuf::from("dir/.env.keys"));
        let import = parse_dotenv_import(
            "av dotenv",
            [
                OsString::from("--file"),
                OsString::from(".env"),
                OsString::from("--keys-file"),
                OsString::from("keys.env"),
            ]
            .into_iter(),
        )
        .unwrap()
        .unwrap();
        assert_eq!(import.keys_file, PathBuf::from("keys.env"));

        assert_eq!(
            parse_dotenv_hook("av dotenv", Vec::<OsString>::new().into_iter()).unwrap_err(),
            "missing shell"
        );
        assert!(
            parse_dotenv_hook("av dotenv", [OsString::from("--help")].into_iter())
                .unwrap()
                .is_none()
        );
        assert!(
            parse_dotenv_hook("av dotenv", [OsString::from("--version")].into_iter())
                .unwrap()
                .is_none()
        );
        assert_eq!(
            parse_dotenv_hook(
                "av dotenv",
                [OsString::from("bash"), OsString::from("extra")].into_iter(),
            )
            .unwrap_err(),
            "dotenv hook supports one shell"
        );
        assert_eq!(
            parse_dotenv_hook("av dotenv", [OsString::from("tcsh")].into_iter()).unwrap_err(),
            "unsupported shell 'tcsh'"
        );
        assert_eq!(
            parse_dotenv_hook("av dotenv", [OsString::from_vec(vec![0xff])].into_iter())
                .unwrap_err(),
            "shell must be valid UTF-8"
        );

        assert!(
            parse_dotenv_export("av dotenv", [OsString::from("--help")].into_iter())
                .unwrap()
                .is_none()
        );
        assert!(
            parse_dotenv_export("av dotenv", [OsString::from("--version")].into_iter())
                .unwrap()
                .is_none()
        );
        assert_eq!(
            parse_dotenv_export("av dotenv", Vec::<OsString>::new().into_iter()).unwrap_err(),
            "missing --shell"
        );
        assert_eq!(
            parse_dotenv_export("av dotenv", [OsString::from("--shell")].into_iter()).unwrap_err(),
            "missing value for --shell"
        );
        assert_eq!(
            parse_dotenv_export("av dotenv", [OsString::from("--unknown")].into_iter())
                .unwrap_err(),
            "unknown dotenv export argument '--unknown'"
        );
        let export = parse_dotenv_export(
            "av dotenv",
            [
                OsString::from("--shell"),
                OsString::from("fish"),
                OsString::from("--cwd"),
                OsString::from("/tmp"),
            ]
            .into_iter(),
        )
        .unwrap()
        .unwrap();
        assert_eq!(export.shell, DotenvShell::Fish);
        assert_eq!(export.cwd, PathBuf::from("/tmp"));

        assert_eq!(
            parse_dotenv_run("av dotenv", Vec::<OsString>::new().into_iter()).unwrap_err(),
            "missing command"
        );
        assert!(
            parse_dotenv_run("av dotenv", [OsString::from("--help")].into_iter())
                .unwrap()
                .is_none()
        );
        assert!(
            parse_dotenv_run("av dotenv", [OsString::from("--version")].into_iter())
                .unwrap()
                .is_none()
        );
        assert_eq!(
            parse_dotenv_run("av dotenv", [OsString::from("--file")].into_iter()).unwrap_err(),
            "missing value for --file"
        );
        let run = parse_dotenv_run(
            "av dotenv",
            [
                OsString::from("-f"),
                OsString::from("custom.env"),
                OsString::from("--"),
                OsString::from("/bin/echo"),
                OsString::from("hello"),
            ]
            .into_iter(),
        )
        .unwrap()
        .unwrap();
        assert_eq!(run.file, PathBuf::from("custom.env"));
        assert_eq!(run.command, OsString::from("/bin/echo"));
        assert_eq!(run.args, vec![OsString::from("hello")]);
    }

    #[test]
    fn dotenv_document_helpers_cover_rendering_selection_and_paths() {
        let temp = TempDir::new().unwrap();
        let missing = temp.path().join("nested/.env");
        let empty = DotenvDocument::load_or_empty(&missing).unwrap();
        assert!(empty.lines.is_empty());
        assert!(empty.had_trailing_newline);
        assert_eq!(empty.path, missing);

        let mut doc = DotenvDocument::parse(
            PathBuf::from(".env.local.txt"),
            "export FOO: value # comment\r\nBAR=`raw value`\rBAZ=\"line\\nnext\"\nNO_SEP\n",
        );
        assert_eq!(doc.value("FOO").unwrap(), "value");
        assert_eq!(doc.value("BAR").unwrap(), "raw value");
        assert_eq!(doc.value("BAZ").unwrap(), "line\nnext");
        assert!(doc.value("NO_SEP").is_none());
        assert!(doc.render().ends_with('\n'));

        doc.ensure_public_key("DOTENV_PUBLIC_KEY_LOCAL", "abc123");
        assert_eq!(
            doc.public_key(),
            Some(("DOTENV_PUBLIC_KEY_LOCAL".to_string(), "abc123".to_string()))
        );
        doc.set_value("QUOTED", "tabs\tand\nlines\"\\");
        assert!(
            doc.render()
                .contains("QUOTED=\"tabs\\tand\\nlines\\\"\\\\\"")
        );

        let selected = doc.encryptable_keys(
            &["FOO".to_string(), "QUOTED".to_string()],
            &["FOO".to_string()],
        );
        assert_eq!(selected, vec!["QUOTED"]);
        doc.set_value("QUOTED", "encrypted:abc");
        assert!(doc.encryptable_keys(&[], &[]).contains(&"FOO".to_string()));
        assert!(
            !doc.encryptable_keys(&[], &[])
                .contains(&"QUOTED".to_string())
        );

        let mut empty_doc = DotenvDocument::parse(PathBuf::from(".env"), "");
        empty_doc.ensure_public_key("DOTENV_PUBLIC_KEY", "public");
        assert!(empty_doc.render().contains("DOTENV_PUBLIC_KEY=\"public\""));

        let write_path = temp.path().join("write/.env");
        let writable = DotenvDocument::parse(write_path.clone(), "FOO=bar");
        writable.write().unwrap();
        let loaded = DotenvDocument::load(&write_path).unwrap();
        assert_eq!(loaded.path, fs::canonicalize(&write_path).unwrap());
        assert_eq!(loaded.value("FOO").unwrap(), "bar");

        assert_eq!(
            resolve_dotenv_path(&temp.path().join("absent.env")).unwrap(),
            temp.path().join("absent.env")
        );
        assert_eq!(
            public_key_name_for_file(Path::new(".env")),
            "DOTENV_PUBLIC_KEY"
        );
        assert_eq!(
            public_key_name_for_file(Path::new(".env.production.local.txt")),
            "DOTENV_PUBLIC_KEY_PRODUCTION_LOCAL"
        );
        assert_eq!(
            private_key_name_for_public_key_name("DOTENV_PUBLIC_KEY_PRODUCTION"),
            "DOTENV_PRIVATE_KEY_PRODUCTION"
        );
    }

    #[test]
    fn dotenv_crypto_helpers_cover_validation_and_decryption_errors() {
        assert_eq!(decode_hex("0x0A").unwrap(), vec![10]);
        assert_eq!(
            decode_hex("abc").unwrap_err(),
            "hex value must have an even number of characters"
        );
        assert_eq!(
            decode_hex("zz").unwrap_err(),
            "hex value contains non-hex characters"
        );
        assert!(validate_private_key_list("").is_ok());
        assert_eq!(
            validate_private_key_list("aa").unwrap_err(),
            "dotenv private key must be 32 bytes"
        );
        assert_eq!(
            validate_private_key_list("not-hex").unwrap_err(),
            "hex value must have an even number of characters"
        );

        assert_eq!(
            decrypt_dotenv_value("PLAIN", "not encrypted", "").unwrap(),
            "not encrypted"
        );
        assert!(
            decrypt_dotenv_value("BAD", "encrypted:not-base64", "")
                .unwrap_err()
                .contains("malformed encrypted data")
        );
        assert_eq!(
            decrypt_dotenv_value("EMPTY", "encrypted:abcd", "").unwrap_err(),
            "could not decrypt EMPTY: missing private key"
        );

        let good = generate_dotenv_keypair(Path::new(".env"));
        let wrong = generate_dotenv_keypair(Path::new(".env"));
        let encrypted = encrypt_dotenv_value("secret", &good.public_key).unwrap();
        assert!(
            decrypt_dotenv_value("FOO", &encrypted, &wrong.private_key)
                .unwrap_err()
                .contains("could not decrypt FOO")
        );
        assert!(public_key_fingerprint(&good.public_key).len() == 64);
        assert!(
            keychain_account_for_public_key(&good.public_key).starts_with("DOTENV_PRIVATE_KEY:")
        );
    }

    #[test]
    fn dotenv_approval_store_paths_and_decisions_cover_json_edges() {
        let _lock = global_test_env_lock().lock().unwrap();
        let temp = TempDir::new().unwrap();
        let home = temp.path().join("home");
        fs::create_dir_all(&home).unwrap();
        let home_str = home.to_str().unwrap();
        let _env = DotenvEnvGuard::set(&[("HOME", home_str)]);

        assert_eq!(
            dotenv_user_approval_root().unwrap(),
            home.join("Library/Application Support/Automic Vault/dotenv")
        );
        assert!(
            load_dotenv_remembered_approvals()
                .unwrap()
                .entries
                .is_empty()
        );

        let entry = DotenvRememberedApprovalEntry {
            mode: DotenvApprovalMode::Export,
            env_file_path: "/tmp/project/.env".to_string(),
            project_root: "/tmp/project".to_string(),
            env_sha256: "sha".to_string(),
            public_key_fingerprint: "fingerprint".to_string(),
            keys: vec!["FOO".to_string()],
        };
        remember_dotenv_approval(entry.clone()).unwrap();
        remember_dotenv_approval(entry.clone()).unwrap();
        let store = load_dotenv_remembered_approvals().unwrap();
        assert_eq!(store.entries, vec![entry.clone()]);

        let pending = dotenv_pending_approval_path().unwrap();
        write_dotenv_json(&pending, &entry).unwrap();
        assert!(pending.is_file());

        let decision_path = dotenv_decision_path("approved").unwrap();
        write_dotenv_json(
            &decision_path,
            &DotenvApprovalDecision {
                id: "approved".to_string(),
                approved: true,
                reason: None,
            },
        )
        .unwrap();
        wait_for_dotenv_decision("approved").unwrap();
        assert!(!pending.exists());
        assert!(!decision_path.exists());

        write_dotenv_json(&dotenv_pending_approval_path().unwrap(), &entry).unwrap();
        write_dotenv_json(
            &dotenv_decision_path("denied").unwrap(),
            &DotenvApprovalDecision {
                id: "denied".to_string(),
                approved: false,
                reason: None,
            },
        )
        .unwrap();
        assert_eq!(
            wait_for_dotenv_decision("denied").unwrap_err(),
            "dotenv approval denied"
        );

        write_dotenv_json(&dotenv_pending_approval_path().unwrap(), &entry).unwrap();
        write_dotenv_json(
            &dotenv_decision_path("mismatch").unwrap(),
            &DotenvApprovalDecision {
                id: "other".to_string(),
                approved: true,
                reason: None,
            },
        )
        .unwrap();
        assert_eq!(
            wait_for_dotenv_decision("mismatch").unwrap_err(),
            "dotenv approval decision id mismatch"
        );

        fs::write(dotenv_remembered_approvals_path().unwrap(), "not json").unwrap();
        assert!(
            load_dotenv_remembered_approvals()
                .unwrap_err()
                .contains("failed to decode")
        );
        drop(_env);
        let _env = DotenvEnvGuard::unset(&["HOME"]);
        assert_eq!(dotenv_user_approval_root().unwrap_err(), "HOME is not set");
    }

    #[cfg(unix)]
    #[test]
    fn dotenv_load_export_and_run_cover_approval_bypass_paths() {
        let _lock = global_test_env_lock().lock().unwrap();
        let temp = TempDir::new().unwrap();
        let home = temp.path().join("home");
        let project = temp.path().join("project");
        fs::create_dir_all(&project).unwrap();
        fs::create_dir_all(project.join("child")).unwrap();
        let home_str = home.to_str().unwrap();
        let _env = DotenvEnvGuard::set(&[("HOME", home_str), (AV_DOTENV_KEYS_ENV, "FOO:BAD-NAME")]);
        let _unset = DotenvEnvGuard::unset(&["FOO", "BAR", "EXTERNAL"]);

        let keypair = generate_dotenv_keypair(Path::new(".env"));
        let encrypted_bar = encrypt_dotenv_value("bar secret", &keypair.public_key).unwrap();
        let env_path = project.join(".env");
        fs::write(
            &env_path,
            format!(
                "DOTENV_PUBLIC_KEY={}\nFOO=plain secret\nBAR={}\nBAD-NAME=skip\n",
                keypair.public_key, encrypted_bar
            ),
        )
        .unwrap();
        let store = StubDotenvPrivateKeyStore::default();
        store
            .store_private_key(&keypair.public_key, &keypair.private_key)
            .unwrap();

        remember_dotenv_approval(remembered_entry_for(
            &env_path,
            DotenvApprovalMode::Export,
            &keypair.public_key,
            &["BAR", "FOO"],
        ))
        .unwrap();
        let loaded = load_dotenv_secrets(
            &env_path,
            DotenvApprovalMode::Export,
            &[],
            &store,
            Some(&["FOO".to_string()]),
        )
        .unwrap();
        assert_eq!(loaded.values["FOO"], "plain secret");
        assert_eq!(loaded.values["BAR"], "bar secret");
        assert_eq!(
            nearest_dotenv_file(&project.join("child")).unwrap(),
            loaded.env_path
        );

        print_shell_unload(DotenvShell::Bash, &["OLD".to_string()]);
        print_shell_unload(DotenvShell::Fish, &["OLD".to_string()]);
        print_shell_exports(DotenvShell::Zsh, &["OLD".to_string()], &loaded);
        print_shell_exports(DotenvShell::Fish, &["OLD".to_string()], &loaded);
        print_dotenv_hook("av dotenv", DotenvShell::Bash);
        print_dotenv_hook("av dotenv", DotenvShell::Zsh);
        print_dotenv_hook("av dotenv", DotenvShell::Fish);

        run_dotenv_export(
            &DotenvExportOptions {
                shell: DotenvShell::Bash,
                cwd: temp.path().join("missing"),
            },
            &store,
        )
        .unwrap();

        let digest = sha256_file_hex(&env_path).unwrap();
        let _current = DotenvEnvGuard::set(&[
            (AV_DOTENV_FILE_ENV, env_path.to_str().unwrap()),
            (AV_DOTENV_DIGEST_ENV, &digest),
        ]);
        run_dotenv_export(
            &DotenvExportOptions {
                shell: DotenvShell::Bash,
                cwd: project.clone(),
            },
            &store,
        )
        .unwrap();
        drop(_current);

        remember_dotenv_approval(remembered_entry_for(
            &env_path,
            DotenvApprovalMode::Run,
            &keypair.public_key,
            &["BAR", "FOO"],
        ))
        .unwrap();
        run_dotenv_run(
            &DotenvRunOptions {
                file: env_path,
                command: OsString::from("/bin/sh"),
                args: vec![
                    OsString::from("-c"),
                    OsString::from("printf '%s\\n' \"$FOO:$BAR\""),
                ],
            },
            &store,
        )
        .unwrap();

        unsafe {
            env::set_var("EXTERNAL", "already set");
        }
        assert!(env_key_is_preexisting("EXTERNAL", None));
        assert!(!env_key_is_preexisting(
            "EXTERNAL",
            Some(&["EXTERNAL".to_string()])
        ));
        assert!(!env_key_is_preexisting("MISSING", None));
        assert_eq!(previous_dotenv_keys(), vec!["FOO".to_string()]);
    }

    #[test]
    fn dotenv_import_set_encrypt_and_store_cover_success_and_errors() {
        let temp = TempDir::new().unwrap();
        let env_path = temp.path().join(".env");
        let keys_path = temp.path().join(".env.keys");
        let keypair = generate_dotenv_keypair(&env_path);
        fs::write(
            &env_path,
            format!(
                "DOTENV_PUBLIC_KEY={}\nFOO=plain\nBAR=encrypted:abc\n",
                keypair.public_key
            ),
        )
        .unwrap();
        fs::write(
            &keys_path,
            format!(
                "{}={}\n",
                private_key_name_for_public_key_name("DOTENV_PUBLIC_KEY"),
                keypair.private_key
            ),
        )
        .unwrap();

        let store = StubDotenvPrivateKeyStore::default();
        run_dotenv_import(
            &DotenvImportOptions {
                file: env_path.clone(),
                keys_file: keys_path.clone(),
            },
            &store,
        )
        .unwrap();
        assert_eq!(
            store.load_private_key(&keypair.public_key).unwrap(),
            keypair.private_key
        );

        run_dotenv_set(
            &DotenvSetOptions {
                file: env_path.clone(),
                key: "NEW_SECRET".to_string(),
            },
            "new value",
            &store,
        )
        .unwrap();
        assert!(
            fs::read_to_string(&env_path)
                .unwrap()
                .contains("NEW_SECRET=\"encrypted:")
        );

        assert!(
            run_dotenv_encrypt(
                &DotenvEncryptOptions {
                    file: env_path.clone(),
                    include_keys: vec!["FOO".to_string()],
                    exclude_keys: Vec::new(),
                    check: true,
                },
                &store,
            )
            .unwrap_err()
            .contains("plaintext dotenv values: FOO")
        );

        run_dotenv_encrypt(
            &DotenvEncryptOptions {
                file: env_path.clone(),
                include_keys: vec!["FOO".to_string()],
                exclude_keys: Vec::new(),
                check: false,
            },
            &store,
        )
        .unwrap();
        assert!(
            fs::read_to_string(&env_path)
                .unwrap()
                .contains("FOO=\"encrypted:")
        );

        run_dotenv_encrypt(
            &DotenvEncryptOptions {
                file: env_path.clone(),
                include_keys: vec!["MISSING".to_string()],
                exclude_keys: Vec::new(),
                check: false,
            },
            &store,
        )
        .unwrap();

        let missing_public = temp.path().join("missing-public.env");
        fs::write(&missing_public, "FOO=bar\n").unwrap();
        assert!(
            run_dotenv_import(
                &DotenvImportOptions {
                    file: missing_public,
                    keys_file: keys_path.clone(),
                },
                &store,
            )
            .unwrap_err()
            .contains("is missing DOTENV_PUBLIC_KEY")
        );

        let missing_private = temp.path().join("missing-private.keys");
        fs::write(&missing_private, "OTHER=value\n").unwrap();
        assert!(
            run_dotenv_import(
                &DotenvImportOptions {
                    file: env_path.clone(),
                    keys_file: missing_private,
                },
                &store,
            )
            .unwrap_err()
            .contains("is missing DOTENV_PRIVATE_KEY")
        );

        let invalid_private = temp.path().join("invalid-private.keys");
        fs::write(&invalid_private, "DOTENV_PRIVATE_KEY=abc\n").unwrap();
        assert_eq!(
            run_dotenv_import(
                &DotenvImportOptions {
                    file: env_path,
                    keys_file: invalid_private,
                },
                &store,
            )
            .unwrap_err(),
            "hex value must have an even number of characters"
        );
    }
}
