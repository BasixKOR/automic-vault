use std::ffi::{OsStr, OsString};
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

const TOKEN_ENV: &str = "AUTOMIC_VAULT_CREDENTIAL_HELPER_TOKEN";

unsafe extern "C" {
    fn getuid() -> u32;
}

pub(crate) fn run(
    tool: &OsStr,
    target: &OsStr,
    args: impl Iterator<Item = OsString>,
) -> Result<(), String> {
    let tool = tool
        .to_str()
        .ok_or_else(|| "stub tool must be valid UTF-8".to_string())?;
    let original = PathBuf::from(target);
    if !original.is_absolute() {
        return Err("stub target must be absolute".to_string());
    }
    let nonce = broker_request(&format!("mint {tool} {}\n", original.display()))?;

    let mut command = Command::new(&original);
    command.args(args);
    command.env(TOKEN_ENV, nonce);
    Err(format!(
        "failed to exec {}: {}",
        original.display(),
        command.exec()
    ))
}

pub(crate) fn broker_request(message: &str) -> Result<String, String> {
    let mut stream = UnixStream::connect(socket_path())
        .map_err(|_| "Automic Vault is not running; open the menu bar app".to_string())?;
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .map_err(|err| format!("failed to configure broker socket: {err}"))?;
    stream
        .write_all(message.as_bytes())
        .map_err(|err| format!("failed to write broker request: {err}"))?;
    stream
        .shutdown(std::net::Shutdown::Write)
        .map_err(|err| format!("failed to finish broker request: {err}"))?;

    let mut response = String::new();
    stream
        .read_to_string(&mut response)
        .map_err(|err| format!("failed to read broker response: {err}"))?;
    let response = response.trim();
    if let Some(value) = response.strip_prefix("ok ") {
        Ok(value.to_string())
    } else if let Some(value) = response.strip_prefix("err ") {
        Err(value.to_string())
    } else {
        Err("invalid broker response".to_string())
    }
}

pub(crate) fn socket_path() -> String {
    format!(
        "/tmp/com.automicvault.av2.credential-helper.{}.sock",
        unsafe { getuid() }
    )
}
