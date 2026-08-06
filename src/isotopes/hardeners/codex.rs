use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

use super::HardenerDetection;

const PRIVILEGE_MODE: super::PrivilegeMode = super::PrivilegeMode::UserOnly;
const KEY: &str = "cli_auth_credentials_store";
const SETTING: &str = "cli_auth_credentials_store = \"keyring\"";
const OVERRIDE: &str = "cli_auth_credentials_store=\"keyring\"";

pub(crate) fn run(stdout: &mut dyn Write, yes: bool) -> Result<(), String> {
    PRIVILEGE_MODE.require_user("codex", false)?;
    let config = crate::isotopes::detectors::codex::config_path()?;
    let auth = crate::isotopes::detectors::codex::auth_path()?;

    writeln!(stdout, "╭─ harden codex").ok();
    writeln!(stdout, "│").ok();
    if keyring_configured(&config)? && !auth.exists() {
        writeln!(stdout, "╰─ already hardened ✔︎").ok();
        return Ok(());
    }

    ensure_chatgpt_is_not_running()?;
    let codex = codex_cli()?;
    let login = Login::read(&auth)?;
    writeln!(stdout, "├─ set `{SETTING}` in {}", config.display()).ok();
    writeln!(stdout, "├─ run `{}`", login.display(&codex)).ok();
    writeln!(stdout, "├─ verify `{} login status`", codex.display()).ok();
    writeln!(
        stdout,
        "├─ delete {} only after verification",
        auth.display()
    )
    .ok();
    writeln!(stdout, "│").ok();
    if !super::gh_cli::confirm(stdout, yes)? {
        writeln!(stdout, "╰─ cancelled").ok();
        return Ok(());
    }
    ensure_chatgpt_is_not_running()?;

    let backup = ConfigBackup::read(&config)?;
    let hardened = hardened_config(backup.contents.as_deref().unwrap_or(""))?;
    write_config(&config, &hardened, backup.mode)?;
    writeln!(stdout, "├─ configured Codex to require the macOS Keychain").ok();

    if let Err(err) = login.run(&codex) {
        return Err(rollback_error(&config, &backup, err));
    }
    writeln!(stdout, "├─ Codex login completed").ok();

    if let Err(err) = run_codex(&codex, &["login", "status"]) {
        return Err(rollback_error(&config, &backup, err));
    }
    writeln!(stdout, "├─ verified Codex login from the Keychain").ok();

    remove_plaintext_auth(&auth)?;
    writeln!(stdout, "╰─ hardened codex").ok();
    Ok(())
}

pub(crate) fn detect() -> HardenerDetection {
    let Ok(config) = crate::isotopes::detectors::codex::config_path() else {
        return HardenerDetection::configuration(false, false, None);
    };
    let auth = crate::isotopes::detectors::codex::auth_path().ok();
    let plaintext_auth_exists = auth.as_deref().is_some_and(Path::exists);
    let applicable = config.exists() || plaintext_auth_exists;
    HardenerDetection::configuration(
        keyring_configured(&config).unwrap_or(false) && !plaintext_auth_exists,
        applicable,
        Some(config.display().to_string()),
    )
}

fn codex_cli() -> Result<PathBuf, String> {
    if let Some(path) = crate::test_env_var("AUTOMIC_VAULT_TEST_CODEX_CLI_PATH") {
        return Ok(path.into());
    }
    crate::cli::doctor::trusted_codex_cli()
}

fn ensure_chatgpt_is_not_running() -> Result<(), String> {
    if chatgpt_is_running()? {
        Err("quit ChatGPT.app before hardening Codex, then try again".into())
    } else {
        Ok(())
    }
}

fn chatgpt_is_running() -> Result<bool, String> {
    if let Some(value) = crate::test_env_string("AUTOMIC_VAULT_TEST_CHATGPT_RUNNING") {
        return Ok(value == "1");
    }
    let status = Command::new("/usr/bin/pgrep")
        .args(["-x", "ChatGPT"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|err| format!("failed to check whether ChatGPT.app is running: {err}"))?;
    match status.code() {
        Some(0) => Ok(true),
        Some(1) => Ok(false),
        _ => Err(format!(
            "could not determine whether ChatGPT.app is running: {status}"
        )),
    }
}

fn run_codex(codex: &Path, args: &[&str]) -> Result<(), String> {
    let status = codex_command(codex)
        .args(args)
        .args(["-c", OVERRIDE])
        .status()
        .map_err(|err| format!("failed to run `codex {}`: {err}", args.join(" ")))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("`codex {}` failed with {status}", args.join(" ")))
    }
}

fn codex_command(codex: &Path) -> Command {
    let mut command = Command::new(codex);
    command
        .env_remove("CODEX_API_KEY")
        .env_remove("CODEX_ACCESS_TOKEN")
        .env_remove("OPENAI_API_KEY");
    command
}

enum Login {
    ChatGpt,
    ApiKey(String),
    AccessToken(String),
}

impl Login {
    fn read(auth: &Path) -> Result<Self, String> {
        match fs::symlink_metadata(auth) {
            Ok(metadata) if metadata.file_type().is_file() => {}
            Ok(_) => {
                return Err(format!(
                    "refusing to migrate non-regular Codex auth file {}",
                    auth.display()
                ));
            }
            Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(Self::ChatGpt),
            Err(err) => return Err(format!("failed to inspect {}: {err}", auth.display())),
        }
        let contents = match fs::read_to_string(auth) {
            Ok(contents) => contents,
            Err(err) => return Err(format!("failed to read {}: {err}", auth.display())),
        };
        let value: serde_json::Value = serde_json::from_str(&contents)
            .map_err(|err| format!("cannot safely migrate malformed {}: {err}", auth.display()))?;
        let object = value
            .as_object()
            .ok_or_else(|| format!("cannot safely migrate non-object {}", auth.display()))?;
        let api_key = nonempty_string(object.get("OPENAI_API_KEY"));
        let access_token = nonempty_string(object.get("personal_access_token"));
        let chatgpt = object
            .get("tokens")
            .and_then(serde_json::Value::as_object)
            .is_some_and(|tokens| {
                ["access_token", "refresh_token", "id_token"]
                    .iter()
                    .any(|key| nonempty_string(tokens.get(*key)).is_some())
            });
        let unsupported = ["bedrock_api_key", "agent_identity"]
            .iter()
            .any(|key| object.get(*key).is_some_and(contains_secret));
        if unsupported
            || usize::from(api_key.is_some())
                + usize::from(access_token.is_some())
                + usize::from(chatgpt)
                > 1
        {
            return Err(format!(
                "{} contains mixed or unsupported Codex credentials; migrate it manually",
                auth.display()
            ));
        }
        Ok(match (api_key, access_token, chatgpt) {
            (Some(secret), None, false) => Self::ApiKey(secret.to_string()),
            (None, Some(secret), false) => Self::AccessToken(secret.to_string()),
            _ => Self::ChatGpt,
        })
    }

    fn display(&self, codex: &Path) -> String {
        let flag = match self {
            Self::ChatGpt => "",
            Self::ApiKey(_) => " --with-api-key",
            Self::AccessToken(_) => " --with-access-token",
        };
        format!("{} login{flag}", codex.display())
    }

    fn run(&self, codex: &Path) -> Result<(), String> {
        let mut command = codex_command(codex);
        command.arg("login");
        let secret = match self {
            Self::ChatGpt => None,
            Self::ApiKey(secret) => {
                command.arg("--with-api-key");
                Some(secret)
            }
            Self::AccessToken(secret) => {
                command.arg("--with-access-token");
                Some(secret)
            }
        };
        command.args(["-c", OVERRIDE]);
        if secret.is_some() {
            command.stdin(Stdio::piped());
        }
        let mut child = command
            .spawn()
            .map_err(|err| format!("failed to run `codex login`: {err}"))?;
        let input_error = secret.and_then(|secret| {
            child.stdin.take().map_or_else(
                || Some("failed to open Codex login input".to_string()),
                |mut stdin| {
                    writeln!(stdin, "{secret}")
                        .err()
                        .map(|err| format!("failed to send credentials to Codex login: {err}"))
                },
            )
        });
        let status = child
            .wait()
            .map_err(|err| format!("failed to wait for `codex login`: {err}"))?;
        if let Some(err) = input_error {
            return Err(err);
        }
        if status.success() {
            Ok(())
        } else {
            Err(format!("`codex login` failed with {status}"))
        }
    }
}

fn nonempty_string(value: Option<&serde_json::Value>) -> Option<&str> {
    value
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.trim().is_empty())
}

fn contains_secret(value: &serde_json::Value) -> bool {
    match value {
        serde_json::Value::String(value) => !value.trim().is_empty(),
        serde_json::Value::Array(values) => values.iter().any(contains_secret),
        serde_json::Value::Object(values) => values.values().any(contains_secret),
        _ => false,
    }
}

struct ConfigBackup {
    contents: Option<String>,
    mode: u32,
}

impl ConfigBackup {
    fn read(path: &Path) -> Result<Self, String> {
        let metadata = match fs::symlink_metadata(path) {
            Ok(metadata) if metadata.file_type().is_file() => Some(metadata),
            Ok(_) => {
                return Err(format!(
                    "refusing to replace non-regular Codex config {}",
                    path.display()
                ));
            }
            Err(err) if err.kind() == io::ErrorKind::NotFound => None,
            Err(err) => return Err(format!("failed to inspect {}: {err}", path.display())),
        };
        let contents = metadata
            .as_ref()
            .map(|_| {
                fs::read_to_string(path)
                    .map_err(|err| format!("failed to read {}: {err}", path.display()))
            })
            .transpose()?;
        Ok(Self {
            contents,
            mode: metadata.map_or(0o600, |metadata| metadata.permissions().mode() & 0o777),
        })
    }

    fn restore(&self, path: &Path) -> Result<(), String> {
        if let Some(contents) = &self.contents {
            write_config(path, contents, self.mode)
        } else {
            match fs::remove_file(path) {
                Ok(()) => Ok(()),
                Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(()),
                Err(err) => Err(format!("failed to remove {}: {err}", path.display())),
            }
        }
    }
}

fn rollback_error(config: &Path, backup: &ConfigBackup, cause: String) -> String {
    match backup.restore(config) {
        Ok(()) => format!("{cause}; restored the original Codex configuration; credentials kept"),
        Err(rollback) => format!(
            "{cause}; credentials kept, but failed to restore the original Codex configuration: {rollback}"
        ),
    }
}

fn write_config(path: &Path, contents: &str, mode: u32) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| format!("Codex config has no parent directory: {}", path.display()))?;
    if !parent.exists() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("failed to create {}: {err}", parent.display()))?;
        fs::set_permissions(parent, fs::Permissions::from_mode(0o700))
            .map_err(|err| format!("failed to protect {}: {err}", parent.display()))?;
    }
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let staging = parent.join(format!(
        ".config.toml.automic-vault.{}.{nonce}.tmp",
        std::process::id()
    ));
    let result = (|| {
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .mode(0o600)
            .open(&staging)
            .map_err(|err| format!("failed to create {}: {err}", staging.display()))?;
        file.write_all(contents.as_bytes())
            .and_then(|()| file.sync_all())
            .map_err(|err| format!("failed to write {}: {err}", staging.display()))?;
        file.set_permissions(fs::Permissions::from_mode(mode))
            .map_err(|err| format!("failed to chmod {}: {err}", staging.display()))?;
        fs::rename(&staging, path)
            .map_err(|err| format!("failed to replace {}: {err}", path.display()))
    })();
    if result.is_err() {
        let _ = fs::remove_file(staging);
    }
    result
}

fn hardened_config(contents: &str) -> Result<String, String> {
    if contents.contains("\"\"\"") || contents.contains("'''") {
        return Err("cannot safely edit a Codex config containing multiline strings".into());
    }
    let mut output = String::with_capacity(contents.len() + SETTING.len() + 1);
    let mut found = false;
    let mut top_level = true;
    for line in contents.split_inclusive('\n') {
        let text = line.strip_suffix('\n').unwrap_or(line);
        let trimmed = text.trim();
        if top_level
            && !trimmed.starts_with('#')
            && trimmed
                .split_once('=')
                .is_some_and(|(key, _)| is_setting_key(key))
        {
            if found {
                return Err(format!("Codex config contains duplicate `{KEY}` settings"));
            }
            found = true;
            output.push_str(SETTING);
            if line.ends_with('\n') {
                output.push('\n');
            }
        } else {
            output.push_str(line);
        }
        if trimmed.starts_with('[') {
            top_level = false;
        }
    }
    if found {
        Ok(output)
    } else {
        Ok(format!("{SETTING}\n{contents}"))
    }
}

fn keyring_configured(path: &Path) -> Result<bool, String> {
    if !path.exists() {
        return Ok(false);
    }
    let contents = fs::read_to_string(path)
        .map_err(|err| format!("failed to read {}: {err}", path.display()))?;
    Ok(top_level_value(&contents, KEY) == Some("keyring"))
}

fn top_level_value<'a>(contents: &'a str, wanted: &str) -> Option<&'a str> {
    let mut found = None;
    for line in contents.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            break;
        }
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        if wanted == KEY && is_setting_key(key) {
            if found.is_some() {
                return None;
            }
            found = Some(toml_string(value.trim())?);
        }
    }
    found
}

fn is_setting_key(key: &str) -> bool {
    matches!(
        key.trim(),
        "cli_auth_credentials_store"
            | "\"cli_auth_credentials_store\""
            | "'cli_auth_credentials_store'"
    )
}

fn toml_string(value: &str) -> Option<&str> {
    let quote = value.as_bytes().first().copied()?;
    if !matches!(quote, b'\'' | b'"') {
        return None;
    }
    let end = value[1..].find(char::from(quote))? + 1;
    value[end + 1..]
        .trim_start()
        .starts_with('#')
        .then_some(&value[1..end])
        .or_else(|| (value[end + 1..].trim().is_empty()).then_some(&value[1..end]))
}

fn remove_plaintext_auth(path: &Path) -> Result<(), String> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(format!("failed to delete {}: {err}", path.display())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::symlink;

    #[test]
    fn rewrites_only_the_top_level_credential_store() {
        assert_eq!(
            top_level_value(
                "model = \"gpt-5.6\"\ncli_auth_credentials_store = \"keyring\" # secure\n",
                KEY
            ),
            Some("keyring")
        );

        assert_eq!(
            hardened_config(
                "model = \"gpt-5.6\"\ncli_auth_credentials_store = \"file\" # old\n[profile]\ncli_auth_credentials_store = \"file\"\n"
            ).unwrap(),
            "model = \"gpt-5.6\"\ncli_auth_credentials_store = \"keyring\"\n[profile]\ncli_auth_credentials_store = \"file\"\n"
        );
        assert_eq!(
            hardened_config("[profile]\nmodel = \"gpt-5.6\"\n").unwrap(),
            "cli_auth_credentials_store = \"keyring\"\n[profile]\nmodel = \"gpt-5.6\"\n"
        );
        assert_eq!(
            hardened_config("\"cli_auth_credentials_store\" = \"file\"\n").unwrap(),
            "cli_auth_credentials_store = \"keyring\"\n"
        );
        assert!(hardened_config(&format!("{SETTING}\n{SETTING}\n")).is_err());
        assert!(hardened_config("notes = \"\"\"\nunchanged\n\"\"\"\n").is_err());
    }

    #[test]
    fn refuses_a_symlinked_auth_file() {
        let dir = std::env::temp_dir().join(format!("av-codex-auth-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("target"), r#"{"OPENAI_API_KEY":"secret"}"#).unwrap();
        symlink(dir.join("target"), dir.join("auth.json")).unwrap();
        assert!(Login::read(&dir.join("auth.json")).is_err());
        fs::remove_dir_all(dir).unwrap();
    }
}
