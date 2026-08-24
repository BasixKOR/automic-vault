use std::collections::BTreeMap;
use std::ffi::OsString;
use std::io::{Read, Write};

use serde::{Deserialize, Serialize};

use super::inject;

const MAX_INPUT_BYTES: u64 = 1024 * 1024;
pub(crate) const SECRET_NAME: &str = "UAA_OAUTH_TOKENS";
pub(crate) const SCOPE: &str = r#"{"store":"contexts"}"#;

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Credentials {
    targets: BTreeMap<String, BTreeMap<String, Token>>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Token {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    access_token: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    refresh_token: String,
}

pub(crate) fn run(args: Vec<OsString>, stdout: &mut dyn Write, stderr: &mut dyn Write) -> i32 {
    let mut stdin = std::io::stdin().lock();
    match run_with_io(&args, &mut stdin, stdout) {
        Ok(()) => 0,
        Err(error) => {
            let _ = writeln!(stderr, "uaa-credential: {error}");
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
        return Err("usage: av uaa-credential <get|store|forget>".into());
    };
    let action = action
        .to_str()
        .ok_or_else(|| "credential action must be valid UTF-8".to_string())?;
    crate::secrets::ensure_uaa_helper_ready()?;
    match action {
        "get" => {
            let value = inject::uaa_credential(SECRET_NAME.into(), SCOPE.into())?;
            writeln!(output, "{}", parse_credentials(&value)?)
                .map_err(|error| format!("failed to return UAA credential: {error}"))
        }
        "store" => {
            crate::secrets::store_uaa_credential(SCOPE, &parse_credentials(&read_limited(input)?)?)
        }
        "forget" => crate::secrets::delete_uaa_credential(SCOPE, SECRET_NAME),
        _ => Err(format!("unsupported UAA credential action: {action}")),
    }
}

pub(crate) fn parse_scope(input: &str) -> Result<(), String> {
    (input == SCOPE)
        .then_some(())
        .ok_or_else(|| "invalid UAA credential scope".into())
}

pub(crate) fn parse_credentials(value: &str) -> Result<String, String> {
    if value.len() > MAX_INPUT_BYTES as usize {
        return Err("UAA credential bundle exceeds 1 MiB".into());
    }
    let credentials: Credentials =
        serde_json::from_str(value).map_err(|error| format!("invalid UAA credential: {error}"))?;
    if credentials.targets.is_empty()
        || credentials.targets.len() > 128
        || credentials.targets.iter().any(|(target, contexts)| {
            invalid_key(target)
                || contexts.is_empty()
                || contexts.len() > 256
                || contexts.iter().any(|(context, token)| {
                    invalid_key(context)
                        || invalid_secret(&token.access_token)
                        || invalid_secret(&token.refresh_token)
                        || token.access_token.is_empty() && token.refresh_token.is_empty()
                })
        })
    {
        return Err("invalid UAA credential bundle".into());
    }
    serde_json::to_string(&credentials)
        .map_err(|error| format!("failed to encode UAA credential: {error}"))
}

fn invalid_key(value: &str) -> bool {
    value.is_empty() || value.len() > 4096 || value.bytes().any(|byte| byte == 0)
}

fn invalid_secret(value: &str) -> bool {
    value == "@av" || value.len() > 512 * 1024 || value.bytes().any(|byte| byte == 0)
}

fn read_limited(input: &mut dyn Read) -> Result<String, String> {
    let mut bytes = Vec::new();
    input
        .take(MAX_INPUT_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| format!("failed to read UAA credential: {error}"))?;
    if bytes.len() as u64 > MAX_INPUT_BYTES {
        return Err("UAA credential bundle exceeds 1 MiB".into());
    }
    String::from_utf8(bytes).map_err(|_| "UAA credential must be valid UTF-8".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn bundle_is_exact_and_strict() {
        let value = r#"{"targets":{"url:https://uaa.example":{"client:admin user: grant_type:client_credentials":{"access_token":"access"}}}}"#;
        assert_eq!(parse_credentials(value).unwrap(), value);
        assert!(parse_credentials(r#"{"targets":{}}"#).is_err());
        assert!(
            parse_credentials(r#"{"targets":{"target":{"context":{"access_token":"@av"}}}}"#)
                .is_err()
        );
        assert!(parse_scope(SCOPE).is_ok());
    }

    #[test]
    fn helper_round_trip_uses_test_custody() {
        let _guard = crate::global_test_env_lock().lock().unwrap();
        let root = std::env::temp_dir().join(format!("av-uaa-helper-{}", std::process::id()));
        let keychain = root.join("keychain");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        unsafe { std::env::set_var("AUTOMIC_VAULT_TEST_KEYCHAIN_DIR", &keychain) };
        let args = |action: &str| vec![action.into()];
        let value = r#"{"targets":{"url:https://uaa.example":{"context":{"access_token":"access","refresh_token":"refresh"}}}}"#;
        run_with_io(&args("store"), &mut value.as_bytes(), &mut Vec::new()).unwrap();
        let mut output = Vec::new();
        run_with_io(&args("get"), &mut "".as_bytes(), &mut output).unwrap();
        assert_eq!(String::from_utf8(output).unwrap().trim(), value);
        run_with_io(&args("forget"), &mut "".as_bytes(), &mut Vec::new()).unwrap();
        unsafe { std::env::remove_var("AUTOMIC_VAULT_TEST_KEYCHAIN_DIR") };
        let _ = fs::remove_dir_all(root);
    }
}
