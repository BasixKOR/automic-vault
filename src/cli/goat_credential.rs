use std::ffi::OsString;
use std::io::{Read, Write};

use ring::digest::{SHA256, digest};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::inject;

const MAX_INPUT_BYTES: u64 = 64 * 1024;
const SECRET_PREFIX: &str = "GOAT_AUTH_SESSION_";

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Secrets {
    password: String,
    access_token: String,
    session_token: String,
}

pub(crate) fn run(args: Vec<OsString>, stdout: &mut dyn Write, stderr: &mut dyn Write) -> i32 {
    let mut stdin = std::io::stdin().lock();
    match run_with_io(&args, &mut stdin, stdout) {
        Ok(()) => 0,
        Err(error) => {
            let _ = writeln!(stderr, "goat-credential: {error}");
            1
        }
    }
}

fn run_with_io(
    args: &[OsString],
    input: &mut dyn Read,
    output: &mut dyn Write,
) -> Result<(), String> {
    let [action, did, pds] = args else {
        return Err("usage: av goat-credential <get|store|forget> <did> <pds>".into());
    };
    let action = action
        .to_str()
        .ok_or_else(|| "credential action must be valid UTF-8".to_string())?;
    let did = normalize_did(
        did.to_str()
            .ok_or_else(|| "goat DID must be valid UTF-8".to_string())?,
    )?;
    let pds = crate::cli::oxide_credential::normalize_host(
        pds.to_str()
            .ok_or_else(|| "goat PDS must be valid UTF-8".to_string())?,
    )?;
    let scope = scope(&did, &pds);
    let account = secret_name(&did, &pds);
    crate::secrets::ensure_goat_helper_ready()?;
    match action {
        "get" => {
            let value = inject::goat_credential(account.clone(), scope)?;
            writeln!(output, "{}", parse_secrets(&value)?)
                .map_err(|error| format!("failed to return goat credential: {error}"))
        }
        "store" => {
            crate::secrets::store_goat_credential(&scope, &parse_secrets(&read_limited(input)?)?)
        }
        "forget" => crate::secrets::delete_goat_credential(&scope, &account),
        _ => Err(format!("unsupported goat credential action: {action}")),
    }
}

pub(crate) fn normalize_did(did: &str) -> Result<String, String> {
    let prefix_len = if did.starts_with("did:plc:") {
        "did:plc:".len()
    } else if did.starts_with("did:web:") {
        "did:web:".len()
    } else {
        0
    };
    if prefix_len == 0
        || did.len() == prefix_len
        || did.len() > 2048
        || !did.is_ascii()
        || did
            .bytes()
            .any(|byte| byte.is_ascii_control() || byte.is_ascii_whitespace())
        || !did
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b".:_%~-".contains(&byte))
    {
        return Err("invalid goat DID".into());
    }
    Ok(did.to_string())
}

pub(crate) fn scope(did: &str, pds: &str) -> String {
    json!({"did": did, "pds": pds}).to_string()
}

pub(crate) fn parse_scope(input: &str) -> Result<(String, String), String> {
    let value: Value = serde_json::from_str(input)
        .map_err(|error| format!("invalid goat credential scope: {error}"))?;
    let object = value
        .as_object()
        .filter(|object| object.len() == 2)
        .ok_or_else(|| "goat scope must contain only `did` and `pds`".to_string())?;
    let did = normalize_did(
        object
            .get("did")
            .and_then(Value::as_str)
            .ok_or_else(|| "goat scope is missing `did`".to_string())?,
    )?;
    let pds = crate::cli::oxide_credential::normalize_host(
        object
            .get("pds")
            .and_then(Value::as_str)
            .ok_or_else(|| "goat scope is missing `pds`".to_string())?,
    )?;
    if input != scope(&did, &pds) {
        return Err("goat credential scope is not canonical".into());
    }
    Ok((did, pds))
}

pub(crate) fn secret_name(did: &str, pds: &str) -> String {
    let hash = digest(&SHA256, format!("{did}\0{pds}").as_bytes());
    let hex = hash
        .as_ref()
        .iter()
        .map(|byte| format!("{byte:02X}"))
        .collect::<String>();
    format!("{SECRET_PREFIX}{hex}")
}

pub(crate) fn parse_secrets(value: &str) -> Result<String, String> {
    if value.len() > MAX_INPUT_BYTES as usize {
        return Err("goat credential exceeds 64 KiB".into());
    }
    let secrets: Secrets =
        serde_json::from_str(value).map_err(|error| format!("invalid goat credential: {error}"))?;
    for field in [
        &secrets.password,
        &secrets.access_token,
        &secrets.session_token,
    ] {
        if field.is_empty() || field == "@av" || field.bytes().any(|byte| byte == 0) {
            return Err(
                "goat credentials must be nonempty, secret values, and contain no NUL".into(),
            );
        }
    }
    serde_json::to_string(&secrets)
        .map_err(|error| format!("failed to encode goat credential: {error}"))
}

fn read_limited(input: &mut dyn Read) -> Result<String, String> {
    let mut bytes = Vec::new();
    input
        .take(MAX_INPUT_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| format!("failed to read goat credential: {error}"))?;
    if bytes.len() as u64 > MAX_INPUT_BYTES {
        return Err("goat credential exceeds 64 KiB".into());
    }
    String::from_utf8(bytes).map_err(|_| "goat credential must be valid UTF-8".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn scope_and_secret_bundle_are_canonical() {
        assert!(normalize_did("did:plc:").is_err());
        assert!(normalize_did("did:web:").is_err());
        let scope = scope("did:plc:abc", "https://pds.example");
        assert_eq!(
            parse_scope(&scope).unwrap(),
            ("did:plc:abc".into(), "https://pds.example".into())
        );
        assert!(
            parse_secrets(
                r#"{"password":"@av","access_token":"access","session_token":"refresh"}"#
            )
            .is_err()
        );
        assert!(parse_scope(r#"{"did":"did:plc:abc","pds":"https://pds.example","x":1}"#).is_err());
        assert_eq!(
            parse_secrets(
                r#"{"session_token":"refresh","password":"pass","access_token":"access"}"#
            )
            .unwrap(),
            r#"{"password":"pass","access_token":"access","session_token":"refresh"}"#
        );
    }

    #[test]
    fn helper_round_trip_uses_test_custody() {
        let _guard = crate::global_test_env_lock().lock().unwrap();
        let root = std::env::temp_dir().join(format!("av-goat-helper-{}", std::process::id()));
        let keychain = root.join("keychain");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        unsafe { std::env::set_var("AUTOMIC_VAULT_TEST_KEYCHAIN_DIR", &keychain) };
        let args = |action: &str| {
            vec![
                action.into(),
                "did:plc:abc".into(),
                "https://pds.example".into(),
            ]
        };
        let value = r#"{"password":"pass","access_token":"access","session_token":"refresh"}"#;
        run_with_io(&args("store"), &mut value.as_bytes(), &mut Vec::new()).unwrap();
        let mut output = Vec::new();
        run_with_io(&args("get"), &mut "".as_bytes(), &mut output).unwrap();
        assert_eq!(String::from_utf8(output).unwrap().trim(), value);
        run_with_io(&args("forget"), &mut "".as_bytes(), &mut Vec::new()).unwrap();
        unsafe { std::env::remove_var("AUTOMIC_VAULT_TEST_KEYCHAIN_DIR") };
        let _ = fs::remove_dir_all(root);
    }
}
