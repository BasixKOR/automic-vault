use std::ffi::OsString;
use std::io::{Read, Write};

use ring::digest::{SHA256, digest};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::inject;

const MAX_INPUT_BYTES: u64 = 64 * 1024;
const SECRET_PREFIX: &str = "RAILWAY_AUTH_";

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct Credentials {
    token: Option<String>,
    access_token: Option<String>,
    refresh_token: Option<String>,
}

pub(crate) fn run(args: Vec<OsString>, stdout: &mut dyn Write, stderr: &mut dyn Write) -> i32 {
    let mut stdin = std::io::stdin().lock();
    match run_with_io(&args, &mut stdin, stdout) {
        Ok(()) => 0,
        Err(error) => {
            let _ = writeln!(stderr, "railway-credential: {error}");
            1
        }
    }
}

fn run_with_io(
    args: &[OsString],
    input: &mut dyn Read,
    output: &mut dyn Write,
) -> Result<(), String> {
    let [action, environment, host] = args else {
        return Err("usage: av railway-credential <get|store|forget> <environment> <host>".into());
    };
    let action = action
        .to_str()
        .ok_or_else(|| "credential action must be valid UTF-8".to_string())?;
    let environment = environment
        .to_str()
        .ok_or_else(|| "Railway environment must be valid UTF-8".to_string())?;
    let host = host
        .to_str()
        .ok_or_else(|| "Railway host must be valid UTF-8".to_string())?;
    validate_scope(environment, host)?;
    let scope = scope(environment, host);
    let account = secret_name(environment, host);
    crate::secrets::ensure_railway_helper_ready()?;
    match action {
        "get" => {
            let value = inject::railway_credential(account, scope)?;
            writeln!(output, "{}", parse_credentials(&value)?)
                .map_err(|error| format!("failed to return Railway credential: {error}"))
        }
        "store" => crate::secrets::store_railway_credential(
            &scope,
            &parse_credentials(&read_limited(input)?)?,
        ),
        "forget" => crate::secrets::delete_railway_credential(&scope, &account),
        _ => Err(format!("unsupported Railway credential action: {action}")),
    }
}

pub(crate) fn validate_scope(environment: &str, host: &str) -> Result<(), String> {
    let expected = match environment {
        "production" => "railway.com",
        "staging" => "railway-staging.com",
        "dev" => "railway-develop.com",
        _ => return Err("invalid Railway environment".into()),
    };
    if host != expected {
        return Err("Railway host does not match environment".into());
    }
    Ok(())
}

pub(crate) fn scope(environment: &str, host: &str) -> String {
    json!({"environment": environment, "host": host}).to_string()
}

pub(crate) fn parse_scope(input: &str) -> Result<(String, String), String> {
    let value: Value = serde_json::from_str(input)
        .map_err(|error| format!("invalid Railway credential scope: {error}"))?;
    let object = value
        .as_object()
        .filter(|object| object.len() == 2)
        .ok_or_else(|| "Railway scope must contain only `environment` and `host`".to_string())?;
    let environment = object
        .get("environment")
        .and_then(Value::as_str)
        .ok_or_else(|| "Railway scope is missing `environment`".to_string())?;
    let host = object
        .get("host")
        .and_then(Value::as_str)
        .ok_or_else(|| "Railway scope is missing `host`".to_string())?;
    validate_scope(environment, host)?;
    if input != scope(environment, host) {
        return Err("Railway credential scope is not canonical".into());
    }
    Ok((environment.into(), host.into()))
}

pub(crate) fn secret_name(environment: &str, host: &str) -> String {
    let hash = digest(&SHA256, format!("{environment}\0{host}").as_bytes());
    let hex = hash
        .as_ref()
        .iter()
        .map(|byte| format!("{byte:02X}"))
        .collect::<String>();
    format!("{SECRET_PREFIX}{hex}")
}

pub(crate) fn parse_credentials(value: &str) -> Result<String, String> {
    if value.len() > MAX_INPUT_BYTES as usize {
        return Err("Railway credential exceeds 64 KiB".into());
    }
    let credentials: Credentials = serde_json::from_str(value)
        .map_err(|error| format!("invalid Railway credential: {error}"))?;
    let fields = [
        credentials.token.as_deref(),
        credentials.access_token.as_deref(),
        credentials.refresh_token.as_deref(),
    ];
    if fields
        .iter()
        .flatten()
        .any(|field| field.is_empty() || field.bytes().any(|byte| byte == 0))
        || credentials.token.is_some()
            && (credentials.access_token.is_some() || credentials.refresh_token.is_some())
        || credentials.token.is_none() && credentials.access_token.is_none()
    {
        return Err("invalid Railway credential shape".into());
    }
    serde_json::to_string(&credentials)
        .map_err(|error| format!("failed to encode Railway credential: {error}"))
}

fn read_limited(input: &mut dyn Read) -> Result<String, String> {
    let mut bytes = Vec::new();
    input
        .take(MAX_INPUT_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| format!("failed to read Railway credential: {error}"))?;
    if bytes.len() as u64 > MAX_INPUT_BYTES {
        return Err("Railway credential exceeds 64 KiB".into());
    }
    String::from_utf8(bytes).map_err(|_| "Railway credential must be valid UTF-8".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn scope_and_credentials_are_strict() {
        let scope = scope("production", "railway.com");
        assert_eq!(
            parse_scope(&scope).unwrap(),
            ("production".into(), "railway.com".into())
        );
        assert!(parse_scope(r#"{"environment":"production","host":"evil.example"}"#).is_err());
        assert!(parse_credentials(r#"{"refreshToken":"refresh"}"#).is_err());
        assert_eq!(
            parse_credentials(r#"{"accessToken":"access","refreshToken":"refresh"}"#).unwrap(),
            r#"{"token":null,"accessToken":"access","refreshToken":"refresh"}"#
        );
    }

    #[test]
    fn helper_round_trip_uses_test_custody() {
        let _guard = crate::global_test_env_lock().lock().unwrap();
        let root = std::env::temp_dir().join(format!("av-railway-helper-{}", std::process::id()));
        let keychain = root.join("keychain");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        unsafe { std::env::set_var("AUTOMIC_VAULT_TEST_KEYCHAIN_DIR", &keychain) };
        let args = |action: &str| vec![action.into(), "production".into(), "railway.com".into()];
        let value = r#"{"token":null,"accessToken":"access","refreshToken":"refresh"}"#;
        run_with_io(&args("store"), &mut value.as_bytes(), &mut Vec::new()).unwrap();
        let mut output = Vec::new();
        run_with_io(&args("get"), &mut "".as_bytes(), &mut output).unwrap();
        assert_eq!(String::from_utf8(output).unwrap().trim(), value);
        run_with_io(&args("forget"), &mut "".as_bytes(), &mut Vec::new()).unwrap();
        unsafe { std::env::remove_var("AUTOMIC_VAULT_TEST_KEYCHAIN_DIR") };
        let _ = fs::remove_dir_all(root);
    }
}
