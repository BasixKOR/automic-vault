use std::collections::BTreeMap;
use std::ffi::OsString;
use std::io::{Read, Write};

use serde::{Deserialize, Serialize};

use super::inject;

const MAX_INPUT_BYTES: u64 = 256 * 1024;
pub(crate) const SECRET_NAME: &str = "ORDERCLI_FOODORA_SESSION";
pub(crate) const SCOPE: &str = r#"{"provider":"foodora"}"#;

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Credentials {
    access_token: String,
    refresh_token: String,
    client_secret: String,
    pending_mfa_token: String,
    cookies_by_host: Option<BTreeMap<String, String>>,
}

pub(crate) fn run(args: Vec<OsString>, stdout: &mut dyn Write, stderr: &mut dyn Write) -> i32 {
    let mut stdin = std::io::stdin().lock();
    match run_with_io(&args, &mut stdin, stdout) {
        Ok(()) => 0,
        Err(error) => {
            let _ = writeln!(stderr, "ordercli-credential: {error}");
            1
        }
    }
}

fn run_with_io(
    args: &[OsString],
    input: &mut dyn Read,
    output: &mut dyn Write,
) -> Result<(), String> {
    let [action] = args else {
        return Err("usage: av ordercli-credential <get|store|forget>".into());
    };
    let action = action
        .to_str()
        .ok_or_else(|| "credential action must be valid UTF-8".to_string())?;
    crate::secrets::ensure_ordercli_helper_ready()?;
    match action {
        "get" => {
            let value = inject::ordercli_credential(SECRET_NAME.into(), SCOPE.into())?;
            writeln!(output, "{}", parse_credentials(&value)?)
                .map_err(|error| format!("failed to return ordercli credential: {error}"))
        }
        "store" => crate::secrets::store_ordercli_credential(
            SCOPE,
            &parse_credentials(&read_limited(input)?)?,
        ),
        "forget" => crate::secrets::delete_ordercli_credential(SCOPE, SECRET_NAME),
        _ => Err(format!("unsupported ordercli credential action: {action}")),
    }
}

pub(crate) fn parse_scope(input: &str) -> Result<(), String> {
    (input == SCOPE)
        .then_some(())
        .ok_or_else(|| "invalid ordercli credential scope".into())
}

pub(crate) fn parse_credentials(value: &str) -> Result<String, String> {
    if value.len() > MAX_INPUT_BYTES as usize {
        return Err("ordercli credential bundle exceeds 256 KiB".into());
    }
    let credentials: Credentials = serde_json::from_str(value)
        .map_err(|error| format!("invalid ordercli credential: {error}"))?;
    let fields = [
        credentials.access_token.as_str(),
        credentials.refresh_token.as_str(),
        credentials.client_secret.as_str(),
        credentials.pending_mfa_token.as_str(),
    ];
    if fields
        .iter()
        .any(|field| field.bytes().any(|byte| byte == 0))
        || credentials.cookies_by_host.as_ref().is_some_and(|cookies| {
            cookies.len() > 256
                || cookies.iter().any(|(host, cookie)| {
                    host.is_empty()
                        || cookie.is_empty()
                        || host.len() > 2048
                        || cookie.len() > 64 * 1024
                        || host.bytes().any(|byte| byte == 0)
                        || cookie.bytes().any(|byte| byte == 0)
                })
        })
        || fields.iter().all(|field| field.is_empty())
            && credentials
                .cookies_by_host
                .as_ref()
                .is_none_or(BTreeMap::is_empty)
    {
        return Err("invalid ordercli credential bundle".into());
    }
    serde_json::to_string(&credentials)
        .map_err(|error| format!("failed to encode ordercli credential: {error}"))
}

fn read_limited(input: &mut dyn Read) -> Result<String, String> {
    let mut bytes = Vec::new();
    input
        .take(MAX_INPUT_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| format!("failed to read ordercli credential: {error}"))?;
    if bytes.len() as u64 > MAX_INPUT_BYTES {
        return Err("ordercli credential bundle exceeds 256 KiB".into());
    }
    String::from_utf8(bytes).map_err(|_| "ordercli credential must be valid UTF-8".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn bundle_is_exact_and_strict() {
        let value = r#"{"access_token":"access","refresh_token":"refresh","client_secret":"","pending_mfa_token":"","cookies_by_host":{"example.com":"cookie"}}"#;
        assert_eq!(parse_credentials(value).unwrap(), value);
        assert!(parse_credentials(r#"{"access_token":"access"}"#).is_err());
        assert!(parse_scope(SCOPE).is_ok());
    }

    #[test]
    fn helper_round_trip_uses_test_custody() {
        let _guard = crate::global_test_env_lock().lock().unwrap();
        let root = std::env::temp_dir().join(format!("av-ordercli-helper-{}", std::process::id()));
        let keychain = root.join("keychain");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        unsafe { std::env::set_var("AUTOMIC_VAULT_TEST_KEYCHAIN_DIR", &keychain) };
        let args = |action: &str| vec![action.into()];
        let value = r#"{"access_token":"access","refresh_token":"refresh","client_secret":"","pending_mfa_token":"","cookies_by_host":null}"#;
        run_with_io(&args("store"), &mut value.as_bytes(), &mut Vec::new()).unwrap();
        let mut output = Vec::new();
        run_with_io(&args("get"), &mut "".as_bytes(), &mut output).unwrap();
        assert_eq!(String::from_utf8(output).unwrap().trim(), value);
        run_with_io(&args("forget"), &mut "".as_bytes(), &mut Vec::new()).unwrap();
        unsafe { std::env::remove_var("AUTOMIC_VAULT_TEST_KEYCHAIN_DIR") };
        let _ = fs::remove_dir_all(root);
    }
}
