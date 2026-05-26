use super::*;

use base64::Engine;
use std::collections::BTreeSet;
#[cfg(target_os = "macos")]
use std::ffi::c_char;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};

const DOTENV_SCHEMA_VERSION: u32 = 1;
const DOTENV_KEYCHAIN_SERVICE: &str = "com.automicvault.dotenv";
const DOTENV_APPROVAL_NOTIFICATION: &str = "com.automicvault.dotenv-approval.pending-changed";
const DOTENV_SOCKET_NAME: &str = "dotenv.sock";
const DOTENV_METADATA_PATH: &str = ".config/automic-vault.json";
const DOTENV_ENCRYPTED_PREFIX: &str = "encrypted:";
const DOTENV_APP_BUNDLE_IDENTIFIER: &str = "com.automicvault";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct DotenvProjectMetadata {
    pub schema_version: u32,
    pub project_hash: String,
    pub public_key: String,
    pub managed_files: Vec<String>,
    pub known_secrets: Vec<String>,
    #[serde(default)]
    pub expected_callsites: Vec<DotenvExpectedCallsite>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct DotenvExpectedCallsite {
    pub secret: String,
    pub runtime: String,
    pub mode: String,
    pub normalized_backtrace: Vec<String>,
    pub fingerprint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct DotenvApprovalRequestSnapshot {
    pub id: String,
    pub secret: String,
    pub project_root: String,
    pub project_hash: String,
    pub runtime: String,
    pub mode: String,
    pub pid: u32,
    pub cwd: String,
    pub executable_path: Option<String>,
    pub normalized_backtrace: Vec<String>,
    pub fingerprint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct DotenvApprovalDecision {
    pub id: String,
    pub approved: bool,
    pub always_allow: bool,
    pub reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct DotenvSecretRequest {
    #[serde(rename = "type")]
    kind: String,
    id: String,
    secret: String,
    cwd: String,
    runtime: String,
    pid: u32,
    mode: String,
    #[serde(default)]
    backtrace: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum DotenvDaemonResponse {
    SecretResponse { id: String, value: String },
    Error {
        id: Option<String>,
        code: i32,
        message: String,
    },
}

#[derive(Debug)]
struct DotenvIngestOptions {
    root: PathBuf,
    json: bool,
}

#[derive(Debug)]
struct DotenvInfoOptions {
    root: PathBuf,
    json: bool,
}

#[derive(Debug)]
struct DotenvRevokeOptions {
    root: PathBuf,
    secret: Option<String>,
}

#[derive(Debug)]
struct DotenvServeOptions {
    socket: PathBuf,
}

#[derive(Debug, Serialize)]
struct DotenvIngestSummary {
    project_hash: String,
    managed_files: Vec<String>,
    known_secrets: Vec<String>,
    encrypted_values: usize,
}

#[derive(Debug, Serialize)]
struct DotenvInfoSummary {
    project_hash: String,
    managed_files: Vec<String>,
    known_secrets: Vec<String>,
    expected_callsite_count: usize,
}

trait DotenvKeyStore {
    fn load_private_key(&self, account: &str) -> Result<String, String>;
    fn store_private_key(&self, account: &str, value: &str) -> Result<(), String>;
}

struct DotenvKeychainStore;

pub(crate) fn run_dotenv_entry(program_name: &str, args: env::ArgsOs) -> Result<(), String> {
    dispatch_dotenv(program_name, args, &DotenvKeychainStore)
}

fn dispatch_dotenv<I>(
    program_name: &str,
    mut args: I,
    store: &dyn DotenvKeyStore,
) -> Result<(), String>
where
    I: Iterator<Item = OsString>,
{
    let Some(command) = args.next() else {
        print_dotenv_usage(program_name);
        return Err("missing dotenv subcommand".to_string());
    };

    if is_help_flag(&command) {
        print_dotenv_usage(program_name);
        return Ok(());
    }
    if is_version_flag(&command) {
        println!("{program_name} {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    match command.to_str() {
        Some("ingest") => {
            let options = parse_dotenv_ingest_options(program_name, args)?;
            let summary = ingest_dotenv(&options, store)?;
            print_ingest_summary(&summary, options.json)
        }
        Some("info") => {
            let options = parse_dotenv_info_options(program_name, args)?;
            let summary = dotenv_info(&options)?;
            print_info_summary(&summary, options.json)
        }
        Some("revoke") => {
            let options = parse_dotenv_revoke_options(program_name, args)?;
            revoke_dotenv(&options)
        }
        Some("serve") => {
            let options = parse_dotenv_serve_options(program_name, args)?;
            serve_dotenv(&options, store)
        }
        Some(other) => {
            print_dotenv_usage(program_name);
            Err(format!("unknown dotenv subcommand '{other}'"))
        }
        None => Err("dotenv subcommand must be valid UTF-8".to_string()),
    }
}

fn parse_dotenv_ingest_options<I>(
    program_name: &str,
    args: I,
) -> Result<DotenvIngestOptions, String>
where
    I: Iterator<Item = OsString>,
{
    let mut root = None;
    let mut json = false;
    for arg in args {
        if is_help_flag(&arg) {
            print_dotenv_ingest_usage(program_name);
            return Err(RENDERED_ERROR_PREFIX.to_string());
        }
        match arg.to_str() {
            Some("--json") => json = true,
            Some(value) if value.starts_with('-') => return Err(format!("unknown argument '{value}'")),
            Some(_) if root.is_some() => return Err("dotenv ingest path specified more than once".to_string()),
            Some(value) => root = Some(PathBuf::from(value)),
            None => return Err("dotenv ingest path must be valid UTF-8".to_string()),
        }
    }
    Ok(DotenvIngestOptions {
        root: root.unwrap_or_else(|| PathBuf::from(".")),
        json,
    })
}

fn parse_dotenv_info_options<I>(program_name: &str, args: I) -> Result<DotenvInfoOptions, String>
where
    I: Iterator<Item = OsString>,
{
    let mut root = None;
    let mut json = false;
    for arg in args {
        if is_help_flag(&arg) {
            print_dotenv_info_usage(program_name);
            return Err(RENDERED_ERROR_PREFIX.to_string());
        }
        match arg.to_str() {
            Some("--json") => json = true,
            Some(value) if value.starts_with('-') => return Err(format!("unknown argument '{value}'")),
            Some(_) if root.is_some() => return Err("dotenv info path specified more than once".to_string()),
            Some(value) => root = Some(PathBuf::from(value)),
            None => return Err("dotenv info path must be valid UTF-8".to_string()),
        }
    }
    Ok(DotenvInfoOptions {
        root: root.unwrap_or_else(|| PathBuf::from(".")),
        json,
    })
}

fn parse_dotenv_revoke_options<I>(
    program_name: &str,
    args: I,
) -> Result<DotenvRevokeOptions, String>
where
    I: Iterator<Item = OsString>,
{
    let mut root = PathBuf::from(".");
    let mut secret = None;
    let mut expect_path = false;
    for arg in args {
        if is_help_flag(&arg) {
            print_dotenv_revoke_usage(program_name);
            return Err(RENDERED_ERROR_PREFIX.to_string());
        }
        match arg.to_str() {
            Some("--path") => expect_path = true,
            Some(value) if expect_path => {
                root = PathBuf::from(value);
                expect_path = false;
            }
            Some(value) if value.starts_with('-') => return Err(format!("unknown argument '{value}'")),
            Some(_) if secret.is_some() => return Err("dotenv revoke secret specified more than once".to_string()),
            Some(value) => secret = Some(value.to_string()),
            None => return Err("dotenv revoke argument must be valid UTF-8".to_string()),
        }
    }
    if expect_path {
        return Err("missing value for --path".to_string());
    }
    Ok(DotenvRevokeOptions { root, secret })
}

fn parse_dotenv_serve_options<I>(
    program_name: &str,
    mut args: I,
) -> Result<DotenvServeOptions, String>
where
    I: Iterator<Item = OsString>,
{
    let mut socket = None;
    while let Some(arg) = args.next() {
        if is_help_flag(&arg) {
            print_dotenv_serve_usage(program_name);
            return Err(RENDERED_ERROR_PREFIX.to_string());
        }
        match arg.to_str() {
            Some("--socket") => {
                let value = args
                    .next()
                    .ok_or_else(|| "missing value for --socket".to_string())?;
                socket = Some(PathBuf::from(value));
            }
            Some(value) => return Err(format!("unknown argument '{value}'")),
            None => return Err("dotenv serve argument must be valid UTF-8".to_string()),
        }
    }
    Ok(DotenvServeOptions {
        socket: socket.unwrap_or(default_dotenv_socket_path()?),
    })
}

fn ingest_dotenv(
    options: &DotenvIngestOptions,
    store: &dyn DotenvKeyStore,
) -> Result<DotenvIngestSummary, String> {
    let root = fs::canonicalize(&options.root)
        .map_err(|err| format!("failed to resolve {}: {err}", options.root.display()))?;
    let project_hash = project_hash(&root)?;
    let metadata_path = metadata_path(&root);
    let existing = load_metadata_at(&metadata_path).ok();
    let (private_key, public_key) = match existing {
        Some(metadata) if !metadata.public_key.is_empty() => {
            let account = private_key_account(&metadata.project_hash);
            match store.load_private_key(&account) {
                Ok(private_key) => (private_key, metadata.public_key),
                Err(_) => generate_dotenv_keypair(),
            }
        }
        _ => generate_dotenv_keypair(),
    };

    store.store_private_key(&private_key_account(&project_hash), &private_key)?;

    let files = discover_dotenv_files(&root)?;
    if files.is_empty() {
        return Err(format!("no dotenv files found under {}", root.display()));
    }

    let mut known = BTreeSet::new();
    let mut managed_files = Vec::new();
    let mut encrypted_values = 0;
    for file in &files {
        let outcome = rewrite_dotenv_file(file, &root, &public_key)?;
        encrypted_values += outcome.encrypted_values;
        known.extend(outcome.keys);
        managed_files.push(relative_path(&root, file)?);
    }

    let previous_callsites = load_metadata_at(&metadata_path)
        .map(|metadata| metadata.expected_callsites)
        .unwrap_or_default();
    let metadata = DotenvProjectMetadata {
        schema_version: DOTENV_SCHEMA_VERSION,
        project_hash: project_hash.clone(),
        public_key,
        managed_files: managed_files.clone(),
        known_secrets: known.iter().cloned().collect(),
        expected_callsites: previous_callsites,
    };
    save_metadata_at(&metadata_path, &metadata)?;

    Ok(DotenvIngestSummary {
        project_hash,
        managed_files,
        known_secrets: known.into_iter().collect(),
        encrypted_values,
    })
}

fn dotenv_info(options: &DotenvInfoOptions) -> Result<DotenvInfoSummary, String> {
    let root = fs::canonicalize(&options.root)
        .map_err(|err| format!("failed to resolve {}: {err}", options.root.display()))?;
    let metadata = load_metadata_from_root(&root)?;
    Ok(DotenvInfoSummary {
        project_hash: metadata.project_hash,
        managed_files: metadata.managed_files,
        known_secrets: metadata.known_secrets,
        expected_callsite_count: metadata.expected_callsites.len(),
    })
}

fn revoke_dotenv(options: &DotenvRevokeOptions) -> Result<(), String> {
    let root = fs::canonicalize(&options.root)
        .map_err(|err| format!("failed to resolve {}: {err}", options.root.display()))?;
    let path = metadata_path(&root);
    let mut metadata = load_metadata_at(&path)?;
    let before = metadata.expected_callsites.len();
    if let Some(secret) = &options.secret {
        metadata
            .expected_callsites
            .retain(|callsite| callsite.secret != *secret);
    } else {
        metadata.expected_callsites.clear();
    }
    let removed = before.saturating_sub(metadata.expected_callsites.len());
    save_metadata_at(&path, &metadata)?;
    println!("revoked {removed} dotenv approval{}", if removed == 1 { "" } else { "s" });
    Ok(())
}

fn serve_dotenv(options: &DotenvServeOptions, store: &dyn DotenvKeyStore) -> Result<(), String> {
    if let Some(parent) = options.socket.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("failed to create {}: {err}", parent.display()))?;
    }
    if options.socket.exists() {
        let _ = fs::remove_file(&options.socket);
    }
    let listener = UnixListener::bind(&options.socket)
        .map_err(|err| format!("failed to bind {}: {err}", options.socket.display()))?;
    println!("av dotenv serve: listening on {}", options.socket.display());
    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                let response = handle_dotenv_stream(&mut stream, store);
                let encoded = serde_json::to_string(&response)
                    .unwrap_or_else(|_| r#"{"type":"error","id":null,"code":500,"message":"failed to encode response"}"#.to_string());
                let _ = writeln!(stream, "{encoded}");
            }
            Err(err) => eprintln!("av dotenv serve: accept failed: {err}"),
        }
    }
    Ok(())
}

fn handle_dotenv_stream(
    stream: &mut UnixStream,
    store: &dyn DotenvKeyStore,
) -> DotenvDaemonResponse {
    match handle_dotenv_stream_result(stream, store) {
        Ok(response) => response,
        Err((id, code, message)) => DotenvDaemonResponse::Error { id, code, message },
    }
}

fn handle_dotenv_stream_result(
    stream: &mut UnixStream,
    store: &dyn DotenvKeyStore,
) -> Result<DotenvDaemonResponse, (Option<String>, i32, String)> {
    let mut line = String::new();
    BufReader::new(stream)
        .read_line(&mut line)
        .map_err(|err| (None, 400, format!("failed to read dotenv request: {err}")))?;
    let request: DotenvSecretRequest = serde_json::from_str(line.trim_end())
        .map_err(|err| (None, 400, format!("invalid dotenv request: {err}")))?;
    if request.kind != "secret_request" {
        return Err((Some(request.id), 400, "invalid dotenv request type".to_string()));
    }

    let cwd = PathBuf::from(&request.cwd);
    let project_root = find_metadata_root(&cwd)
        .map_err(|err| (Some(request.id.clone()), 404, err))?;
    let metadata_path = metadata_path(&project_root);
    let mut metadata = load_metadata_at(&metadata_path)
        .map_err(|err| (Some(request.id.clone()), 500, err))?;
    if !metadata.known_secrets.iter().any(|key| key == &request.secret) {
        return Err((
            Some(request.id),
            404,
            format!("dotenv secret {} is not managed by this project", request.secret),
        ));
    }

    let normalized = normalize_backtrace(&request.backtrace, &project_root);
    let fingerprint = callsite_fingerprint(&request.secret, &request.runtime, &normalized);
    let approved = metadata
        .expected_callsites
        .iter()
        .any(|callsite| callsite.fingerprint == fingerprint);
    if !approved {
        let approval = request_dotenv_approval(&request, &project_root, &metadata, &normalized, &fingerprint)
            .map_err(|err| (Some(request.id.clone()), 500, err))?;
        if !approval.approved {
            return Err((
                Some(request.id),
                403,
                approval.reason.unwrap_or_else(|| "secret access denied".to_string()),
            ));
        }
        if approval.always_allow {
            metadata.expected_callsites.push(DotenvExpectedCallsite {
                secret: request.secret.clone(),
                runtime: request.runtime.clone(),
                mode: request.mode.clone(),
                normalized_backtrace: normalized.clone(),
                fingerprint,
            });
            save_metadata_at(&metadata_path, &metadata)
                .map_err(|err| (Some(request.id.clone()), 500, err))?;
        }
    }

    let value = decrypt_managed_secret(&project_root, &metadata, &request.secret, store)
        .map_err(|err| (Some(request.id.clone()), 500, err))?;
    Ok(DotenvDaemonResponse::SecretResponse {
        id: request.id,
        value,
    })
}

fn decrypt_managed_secret(
    root: &Path,
    metadata: &DotenvProjectMetadata,
    secret: &str,
    store: &dyn DotenvKeyStore,
) -> Result<String, String> {
    let private_key = store.load_private_key(&private_key_account(&metadata.project_hash))?;
    for managed in &metadata.managed_files {
        let path = root.join(managed);
        let contents = fs::read_to_string(&path)
            .map_err(|err| format!("failed to read {}: {err}", path.display()))?;
        for line in contents.lines() {
            let Some(assignment) = parse_dotenv_assignment(line) else {
                continue;
            };
            if assignment.key == secret {
                let value = strip_dotenv_quotes(assignment.value.trim());
                if let Some(encrypted) = value.strip_prefix(DOTENV_ENCRYPTED_PREFIX) {
                    return decrypt_dotenv_value(encrypted, &private_key);
                }
                return Ok(value.to_string());
            }
        }
    }
    Err(format!("dotenv secret {secret} was not found in managed files"))
}

fn request_dotenv_approval(
    request: &DotenvSecretRequest,
    project_root: &Path,
    metadata: &DotenvProjectMetadata,
    normalized_backtrace: &[String],
    fingerprint: &str,
) -> Result<DotenvApprovalDecision, String> {
    let snapshot = DotenvApprovalRequestSnapshot {
        id: request.id.clone(),
        secret: request.secret.clone(),
        project_root: project_root.to_string_lossy().into_owned(),
        project_hash: metadata.project_hash.clone(),
        runtime: request.runtime.clone(),
        mode: request.mode.clone(),
        pid: request.pid,
        cwd: request.cwd.clone(),
        executable_path: process_path(request.pid as i32),
        normalized_backtrace: normalized_backtrace.to_vec(),
        fingerprint: fingerprint.to_string(),
    };
    let pending = dotenv_pending_approval_path()?;
    write_json(&pending, &snapshot)?;
    if let Err(err) = ping_dotenv_approval_app() {
        let _ = fs::remove_file(&pending);
        return Err(err);
    }
    let decision_path = dotenv_decision_path(&request.id)?;
    loop {
        if let Ok(contents) = fs::read_to_string(&decision_path) {
            let decision: DotenvApprovalDecision = serde_json::from_str(&contents)
                .map_err(|err| format!("failed to decode dotenv approval decision: {err}"))?;
            let _ = fs::remove_file(&pending);
            let _ = fs::remove_file(&decision_path);
            return Ok(decision);
        }
        thread::sleep(Duration::from_millis(200));
    }
}

#[derive(Debug)]
struct RewriteOutcome {
    keys: Vec<String>,
    encrypted_values: usize,
}

#[derive(Debug)]
struct DotenvAssignment<'a> {
    key: &'a str,
    value: &'a str,
}

fn rewrite_dotenv_file(path: &Path, root: &Path, public_key: &str) -> Result<RewriteOutcome, String> {
    let contents =
        fs::read_to_string(path).map_err(|err| format!("failed to read {}: {err}", path.display()))?;
    let mut keys = Vec::new();
    let mut encrypted_values = 0;
    let mut out = String::new();
    if !contents.contains("DOTENV_PUBLIC_KEY") {
        out.push_str(&prepend_public_key(public_key, &relative_path(root, path)?));
        out.push('\n');
    }
    for line in contents.lines() {
        if let Some(assignment) = parse_dotenv_assignment(line) {
            keys.push(assignment.key.to_string());
            let value = strip_dotenv_quotes(assignment.value.trim());
            if !value.is_empty() && !value.starts_with(DOTENV_ENCRYPTED_PREFIX) {
                let encrypted = encrypt_dotenv_value(value, public_key)?;
                out.push_str(assignment.key);
                out.push_str("=\"");
                out.push_str(&encrypted);
                out.push('"');
                out.push('\n');
                encrypted_values += 1;
                continue;
            }
        }
        out.push_str(line);
        out.push('\n');
    }
    fs::write(path, out).map_err(|err| format!("failed to write {}: {err}", path.display()))?;
    Ok(RewriteOutcome {
        keys,
        encrypted_values,
    })
}

fn parse_dotenv_assignment(line: &str) -> Option<DotenvAssignment<'_>> {
    let trimmed = line.trim_start();
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return None;
    }
    let trimmed = trimmed.strip_prefix("export ").unwrap_or(trimmed);
    let (key, value) = trimmed.split_once('=')?;
    let key = key.trim();
    if !valid_dotenv_key(key) {
        return None;
    }
    Some(DotenvAssignment { key, value })
}

fn valid_dotenv_key(key: &str) -> bool {
    let mut chars = key.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !(first == '_' || first.is_ascii_alphabetic()) {
        return false;
    }
    chars.all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
}

fn strip_dotenv_quotes(value: &str) -> &str {
    if value.len() >= 2
        && ((value.starts_with('"') && value.ends_with('"'))
            || (value.starts_with('\'') && value.ends_with('\'')))
    {
        &value[1..value.len() - 1]
    } else {
        value
    }
}

fn prepend_public_key(public_key: &str, filename: &str) -> String {
    [
        "#/-------------------[DOTENV_PUBLIC_KEY]--------------------/".to_string(),
        "#/            public-key encryption for .env files          /".to_string(),
        "#/       [how it works](https://dotenvx.com/encryption)     /".to_string(),
        "#/----------------------------------------------------------/".to_string(),
        format!("DOTENV_PUBLIC_KEY=\"{public_key}\""),
        String::new(),
        format!("# {filename}"),
    ]
    .join("\n")
}

fn encrypt_dotenv_value(value: &str, public_key: &str) -> Result<String, String> {
    let public = hex::decode(public_key).map_err(|err| format!("invalid dotenv public key: {err}"))?;
    let encrypted = ecies::encrypt(&public, value.as_bytes())
        .map_err(|err| format!("failed to encrypt dotenv value: {err}"))?;
    Ok(format!(
        "{DOTENV_ENCRYPTED_PREFIX}{}",
        base64::engine::general_purpose::STANDARD.encode(encrypted)
    ))
}

fn decrypt_dotenv_value(value: &str, private_key: &str) -> Result<String, String> {
    let private = hex::decode(private_key).map_err(|err| format!("invalid dotenv private key: {err}"))?;
    let encrypted = base64::engine::general_purpose::STANDARD
        .decode(value)
        .map_err(|err| format!("invalid encrypted dotenv value: {err}"))?;
    let decrypted = ecies::decrypt(&private, &encrypted)
        .map_err(|err| format!("failed to decrypt dotenv value: {err}"))?;
    String::from_utf8(decrypted).map_err(|err| format!("dotenv value is not UTF-8: {err}"))
}

fn generate_dotenv_keypair() -> (String, String) {
    let (private, public) = ecies::utils::generate_keypair();
    (
        hex::encode(private.serialize()),
        hex::encode(public.serialize()),
    )
}

fn discover_dotenv_files(root: &Path) -> Result<Vec<PathBuf>, String> {
    let mut files = Vec::new();
    for entry in WalkDir::new(root)
        .into_iter()
        .filter_entry(|entry| !skip_dotenv_entry(entry))
    {
        let entry = entry.map_err(|err| format!("failed to walk dotenv files: {err}"))?;
        if !entry.file_type().is_file() {
            continue;
        }
        let name = entry.file_name().to_string_lossy();
        if is_managed_dotenv_filename(&name) {
            files.push(entry.path().to_path_buf());
        }
    }
    files.sort();
    Ok(files)
}

fn skip_dotenv_entry(entry: &walkdir::DirEntry) -> bool {
    if entry.depth() == 0 {
        return false;
    }
    if !entry.file_type().is_dir() {
        return false;
    }
    matches!(
        entry.file_name().to_string_lossy().as_ref(),
        ".git" | "node_modules" | "target" | "vendor" | ".venv" | "__pycache__"
    )
}

fn is_managed_dotenv_filename(name: &str) -> bool {
    (name == ".env" || name.starts_with(".env."))
        && !matches!(
            name,
            ".env.example" | ".env.sample" | ".env.template" | ".env.keys"
        )
}

fn load_metadata_from_root(root: &Path) -> Result<DotenvProjectMetadata, String> {
    load_metadata_at(&metadata_path(root))
}

fn load_metadata_at(path: &Path) -> Result<DotenvProjectMetadata, String> {
    let contents =
        fs::read_to_string(path).map_err(|err| format!("failed to read {}: {err}", path.display()))?;
    serde_json::from_str(&contents)
        .map_err(|err| format!("failed to decode {}: {err}", path.display()))
}

fn save_metadata_at(path: &Path, metadata: &DotenvProjectMetadata) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("failed to create {}: {err}", parent.display()))?;
    }
    let encoded = serde_json::to_vec_pretty(metadata)
        .map_err(|err| format!("failed to encode dotenv metadata: {err}"))?;
    fs::write(path, encoded).map_err(|err| format!("failed to write {}: {err}", path.display()))
}

fn metadata_path(root: &Path) -> PathBuf {
    root.join(DOTENV_METADATA_PATH)
}

fn find_metadata_root(cwd: &Path) -> Result<PathBuf, String> {
    let start = fs::canonicalize(cwd)
        .map_err(|err| format!("failed to resolve cwd {}: {err}", cwd.display()))?;
    for candidate in start.ancestors() {
        if metadata_path(candidate).exists() {
            return Ok(candidate.to_path_buf());
        }
    }
    Err(format!(
        "no {} found from {} upward",
        DOTENV_METADATA_PATH,
        start.display()
    ))
}

fn project_hash(root: &Path) -> Result<String, String> {
    let canonical = fs::canonicalize(root)
        .map_err(|err| format!("failed to resolve {}: {err}", root.display()))?;
    let digest = Sha256::digest(canonical.to_string_lossy().as_bytes());
    Ok(hex::encode(digest))
}

fn private_key_account(project_hash: &str) -> String {
    format!("av.dotenv.project.{project_hash}.privatekey")
}

fn relative_path(root: &Path, path: &Path) -> Result<String, String> {
    path.strip_prefix(root)
        .map_err(|err| format!("failed to relativize {}: {err}", path.display()))
        .map(|path| path.to_string_lossy().into_owned())
}

fn normalize_backtrace(backtrace: &[String], root: &Path) -> Vec<String> {
    let root_string = root.to_string_lossy();
    backtrace
        .iter()
        .map(|line| line.replace(root_string.as_ref(), "."))
        .filter(|line| !line.trim().is_empty())
        .collect()
}

fn callsite_fingerprint(secret: &str, runtime: &str, normalized_backtrace: &[String]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(secret.as_bytes());
    hasher.update([0]);
    hasher.update(runtime.as_bytes());
    for frame in normalized_backtrace {
        hasher.update([0]);
        hasher.update(frame.as_bytes());
    }
    hex::encode(hasher.finalize())
}

fn default_dotenv_socket_path() -> Result<PathBuf, String> {
    let home = env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())?;
    Ok(home
        .join("Library")
        .join("Application Support")
        .join("Automic Vault")
        .join(DOTENV_SOCKET_NAME))
}

fn dotenv_approval_root() -> Result<PathBuf, String> {
    let home = env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())?;
    Ok(home
        .join("Library")
        .join("Application Support")
        .join("Automic Vault")
        .join("dotenv"))
}

fn dotenv_pending_approval_path() -> Result<PathBuf, String> {
    Ok(dotenv_approval_root()?.join("pending-approval.json"))
}

fn dotenv_decision_path(id: &str) -> Result<PathBuf, String> {
    Ok(dotenv_approval_root()?.join("decisions").join(format!("{id}.json")))
}

fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| format!("invalid path {}", path.display()))?;
    fs::create_dir_all(parent)
        .map_err(|err| format!("failed to create {}: {err}", parent.display()))?;
    let encoded = serde_json::to_vec_pretty(value)
        .map_err(|err| format!("failed to encode JSON: {err}"))?;
    fs::write(path, encoded).map_err(|err| format!("failed to write {}: {err}", path.display()))
}

fn print_ingest_summary(summary: &DotenvIngestSummary, json: bool) -> Result<(), String> {
    if json {
        println!(
            "{}",
            serde_json::to_string(summary)
                .map_err(|err| format!("failed to serialize dotenv ingest summary: {err}"))?
        );
    } else {
        println!("managed dotenv project {}", summary.project_hash);
        println!("encrypted {} value(s)", summary.encrypted_values);
        for file in &summary.managed_files {
            println!("managed {file}");
        }
    }
    Ok(())
}

fn print_info_summary(summary: &DotenvInfoSummary, json: bool) -> Result<(), String> {
    if json {
        println!(
            "{}",
            serde_json::to_string(summary)
                .map_err(|err| format!("failed to serialize dotenv info: {err}"))?
        );
    } else {
        println!("project {}", summary.project_hash);
        println!("managed files:");
        for file in &summary.managed_files {
            println!("  {file}");
        }
        println!("known secrets:");
        for key in &summary.known_secrets {
            println!("  {key}");
        }
        println!("expected callsites: {}", summary.expected_callsite_count);
    }
    Ok(())
}

pub(crate) fn print_dotenv_usage(program_name: &str) {
    println!(
        "\
Usage: {program_name} <subcommand>

Subcommands:
  ingest [path] [--json]       Encrypt dotenv files and initialize metadata.
  info [path] [--json]         Show managed dotenv project information.
  revoke [SECRET] [--path p]   Revoke persistent dotenv callsite approvals.
  serve [--socket path]        Start the local dotenv secret daemon."
    );
}

fn print_dotenv_ingest_usage(program_name: &str) {
    println!("Usage: {program_name} ingest [path] [--json]");
}

fn print_dotenv_info_usage(program_name: &str) {
    println!("Usage: {program_name} info [path] [--json]");
}

fn print_dotenv_revoke_usage(program_name: &str) {
    println!("Usage: {program_name} revoke [SECRET] [--path <path>]");
}

fn print_dotenv_serve_usage(program_name: &str) {
    println!("Usage: {program_name} serve [--socket <path>]");
}

#[cfg(target_os = "macos")]
fn ping_dotenv_approval_app() -> Result<(), String> {
    let status = Command::new("/usr/bin/open")
        .args(["-b", DOTENV_APP_BUNDLE_IDENTIFIER])
        .status()
        .map_err(|err| format!("failed to ping Automic Vault.app: {err}"))?;
    if !status.success() {
        return Err("failed to ping Automic Vault.app for dotenv approval".to_string());
    }
    post_dotenv_distributed_notification(DOTENV_APPROVAL_NOTIFICATION)
}

#[cfg(not(target_os = "macos"))]
fn ping_dotenv_approval_app() -> Result<(), String> {
    Err("dotenv approvals are only available on macOS".to_string())
}

#[cfg(target_os = "macos")]
fn process_path(pid: i32) -> Option<String> {
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
fn process_path(_pid: i32) -> Option<String> {
    None
}

impl DotenvKeyStore for DotenvKeychainStore {
    fn load_private_key(&self, account: &str) -> Result<String, String> {
        keychain_read_dotenv_secret(DOTENV_KEYCHAIN_SERVICE, account)
    }

    fn store_private_key(&self, account: &str, value: &str) -> Result<(), String> {
        keychain_write_dotenv_secret(DOTENV_KEYCHAIN_SERVICE, account, value)
    }
}

#[cfg(target_os = "macos")]
fn keychain_read_dotenv_secret(service: &str, account: &str) -> Result<String, String> {
    unsafe extern "C" {
        fn isotope_copy_generic_password_json(
            service: *const libc::c_char,
            account: *const libc::c_char,
            error: *mut *mut libc::c_char,
        ) -> *mut libc::c_char;
    }

    let service = CString::new(service).map_err(|_| "invalid keychain service name".to_string())?;
    let account = CString::new(account).map_err(|_| "invalid keychain account name".to_string())?;
    let mut error = std::ptr::null_mut();
    let value = unsafe {
        isotope_copy_generic_password_json(service.as_ptr(), account.as_ptr(), &mut error)
    };
    if value.is_null() {
        return Err(unsafe { take_dotenv_bridge_string(error) }
            .unwrap_or_else(|| "keychain lookup failed".to_string()));
    }
    unsafe { take_dotenv_bridge_string(value) }
        .ok_or_else(|| "keychain returned invalid UTF-8".to_string())
}

#[cfg(not(target_os = "macos"))]
fn keychain_read_dotenv_secret(_service: &str, _account: &str) -> Result<String, String> {
    Err("dotenv keychain integration is only available on macOS".to_string())
}

#[cfg(target_os = "macos")]
fn keychain_write_dotenv_secret(service: &str, account: &str, value: &str) -> Result<(), String> {
    unsafe extern "C" {
        fn isotope_store_generic_password_json(
            service: *const libc::c_char,
            account: *const libc::c_char,
            value: *const libc::c_char,
            error: *mut *mut libc::c_char,
        ) -> bool;
    }

    let service = CString::new(service).map_err(|_| "invalid keychain service name".to_string())?;
    let account = CString::new(account).map_err(|_| "invalid keychain account name".to_string())?;
    let value = CString::new(value).map_err(|_| "invalid keychain secret value".to_string())?;
    let mut error = std::ptr::null_mut();
    if unsafe {
        isotope_store_generic_password_json(service.as_ptr(), account.as_ptr(), value.as_ptr(), &mut error)
    } {
        return Ok(());
    }
    Err(unsafe { take_dotenv_bridge_string(error) }.unwrap_or_else(|| "keychain write failed".to_string()))
}

#[cfg(not(target_os = "macos"))]
fn keychain_write_dotenv_secret(_service: &str, _account: &str, _value: &str) -> Result<(), String> {
    Err("dotenv keychain integration is only available on macOS".to_string())
}

#[cfg(target_os = "macos")]
fn post_dotenv_distributed_notification(name: &str) -> Result<(), String> {
    unsafe extern "C" {
        fn isotope_post_distributed_notification(
            name: *const libc::c_char,
            error: *mut *mut libc::c_char,
        ) -> bool;
    }

    let name = CString::new(name).map_err(|_| "invalid distributed notification name".to_string())?;
    let mut error = std::ptr::null_mut();
    if unsafe { isotope_post_distributed_notification(name.as_ptr(), &mut error) } {
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
    let result = unsafe { std::ffi::CStr::from_ptr(value) }
        .to_str()
        .ok()
        .map(str::to_string);
    unsafe { isotope_free_c_string(value) };
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct MemoryStore {
        values: Mutex<HashMap<String, String>>,
    }

    impl DotenvKeyStore for MemoryStore {
        fn load_private_key(&self, account: &str) -> Result<String, String> {
            self.values
                .lock()
                .unwrap()
                .get(account)
                .cloned()
                .ok_or_else(|| "missing key".to_string())
        }

        fn store_private_key(&self, account: &str, value: &str) -> Result<(), String> {
            self.values
                .lock()
                .unwrap()
                .insert(account.to_string(), value.to_string());
            Ok(())
        }
    }

    #[test]
    fn dotenv_crypto_roundtrips() {
        let (private, public) = generate_dotenv_keypair();
        let encrypted = encrypt_dotenv_value("secret-value", &public).unwrap();
        let payload = encrypted.strip_prefix(DOTENV_ENCRYPTED_PREFIX).unwrap();
        assert_eq!(decrypt_dotenv_value(payload, &private).unwrap(), "secret-value");
    }

    #[test]
    fn dotenv_ingest_rewrites_files_and_metadata() {
        let temp = tempfile::tempdir().unwrap();
        fs::write(
            temp.path().join(".env"),
            "OPENAI_API_KEY=sk-test\nEMPTY=\n# comment\nDATABASE_URL='postgres://x'\n",
        )
        .unwrap();
        fs::write(temp.path().join(".env.example"), "SHOULD=stay\n").unwrap();
        let store = MemoryStore::default();
        let summary = ingest_dotenv(
            &DotenvIngestOptions {
                root: temp.path().to_path_buf(),
                json: false,
            },
            &store,
        )
        .unwrap();
        assert_eq!(summary.encrypted_values, 2);
        assert_eq!(summary.known_secrets, vec!["DATABASE_URL", "EMPTY", "OPENAI_API_KEY"]);
        let rewritten = fs::read_to_string(temp.path().join(".env")).unwrap();
        assert!(rewritten.contains("DOTENV_PUBLIC_KEY"));
        assert!(rewritten.contains("OPENAI_API_KEY=\"encrypted:"));
        assert!(rewritten.contains("EMPTY="));
        assert_eq!(
            fs::read_to_string(temp.path().join(".env.example")).unwrap(),
            "SHOULD=stay\n"
        );
        let metadata = load_metadata_from_root(temp.path()).unwrap();
        assert_eq!(metadata.schema_version, DOTENV_SCHEMA_VERSION);
        assert_eq!(metadata.managed_files, vec![".env"]);
    }

    #[test]
    fn dotenv_revoke_removes_matching_callsites() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        let path = metadata_path(root);
        save_metadata_at(
            &path,
            &DotenvProjectMetadata {
                schema_version: DOTENV_SCHEMA_VERSION,
                project_hash: "hash".to_string(),
                public_key: "pub".to_string(),
                managed_files: vec![],
                known_secrets: vec!["A".to_string(), "B".to_string()],
                expected_callsites: vec![
                    DotenvExpectedCallsite {
                        secret: "A".to_string(),
                        runtime: "node".to_string(),
                        mode: "development".to_string(),
                        normalized_backtrace: vec!["a".to_string()],
                        fingerprint: "fa".to_string(),
                    },
                    DotenvExpectedCallsite {
                        secret: "B".to_string(),
                        runtime: "node".to_string(),
                        mode: "development".to_string(),
                        normalized_backtrace: vec!["b".to_string()],
                        fingerprint: "fb".to_string(),
                    },
                ],
            },
        )
        .unwrap();
        revoke_dotenv(&DotenvRevokeOptions {
            root: root.to_path_buf(),
            secret: Some("A".to_string()),
        })
        .unwrap();
        let metadata = load_metadata_at(&path).unwrap();
        assert_eq!(metadata.expected_callsites.len(), 1);
        assert_eq!(metadata.expected_callsites[0].secret, "B");
    }
}
