use std::ffi::OsString;
use std::io::{Read, Write};

use serde_json::Value;

use super::inject;

const MAX_INPUT_BYTES: u64 = 1024 * 1024;
pub(crate) const SECRET_NAME: &str = "PLUMBER_LOCAL_CONFIG";
pub(crate) const SCOPE: &str = r#"{"store":"local-config"}"#;

pub(crate) fn run(args: Vec<OsString>, stdout: &mut dyn Write, stderr: &mut dyn Write) -> i32 {
    let mut stdin = std::io::stdin().lock();
    match run_with_io(&args, &mut stdin, stdout) {
        Ok(()) => 0,
        Err(error) => {
            let _ = writeln!(stderr, "plumber-credential: {error}");
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
        return Err("usage: av plumber-credential <get|store>".into());
    };
    let action = action
        .to_str()
        .ok_or_else(|| "credential action must be valid UTF-8".to_string())?;
    crate::secrets::ensure_plumber_helper_ready()?;
    match action {
        "get" => {
            let value = inject::plumber_credential(SECRET_NAME.into(), SCOPE.into())?;
            writeln!(output, "{}", parse_config(&value)?)
                .map_err(|error| format!("failed to return Plumber config: {error}"))
        }
        "store" => {
            crate::secrets::store_plumber_credential(SCOPE, &parse_config(&read_limited(input)?)?)
        }
        _ => Err(format!("unsupported Plumber credential action: {action}")),
    }
}

pub(crate) fn parse_scope(input: &str) -> Result<(), String> {
    (input == SCOPE)
        .then_some(())
        .ok_or_else(|| "invalid Plumber config scope".into())
}

pub(crate) fn parse_config(value: &str) -> Result<String, String> {
    if value.is_empty()
        || value.len() > MAX_INPUT_BYTES as usize
        || value.bytes().any(|byte| byte == 0)
    {
        return Err("invalid Plumber config".into());
    }
    let parsed: Value =
        serde_json::from_str(value).map_err(|error| format!("invalid Plumber config: {error}"))?;
    let object = parsed
        .as_object()
        .ok_or_else(|| "Plumber config must be a JSON object".to_string())?;
    if object.len() == 1
        && object.get("automic_vault").and_then(Value::as_str) == Some("plumber-config-v1")
    {
        return Err("refusing to store the Plumber custody marker".into());
    }
    serde_json::to_string(&parsed)
        .map_err(|error| format!("failed to encode Plumber config: {error}"))
}

fn read_limited(input: &mut dyn Read) -> Result<String, String> {
    let mut bytes = Vec::new();
    input
        .take(MAX_INPUT_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| format!("failed to read Plumber config: {error}"))?;
    if bytes.len() as u64 > MAX_INPUT_BYTES {
        return Err("Plumber config exceeds 1 MiB".into());
    }
    String::from_utf8(bytes).map_err(|_| "Plumber config must be valid UTF-8".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn config_is_bounded_json_and_rejects_the_marker() {
        assert_eq!(
            parse_config(r#"{"token":"secret","connections":{}}"#).unwrap(),
            r#"{"connections":{},"token":"secret"}"#
        );
        assert!(parse_config(r#"{"automic_vault":"plumber-config-v1"}"#).is_err());
        assert!(parse_config("[]").is_err());
        assert!(parse_scope(SCOPE).is_ok());
    }

    #[test]
    fn helper_round_trip_uses_test_custody() {
        let _guard = crate::global_test_env_lock().lock().unwrap();
        let root = std::env::temp_dir().join(format!("av-plumber-helper-{}", std::process::id()));
        let keychain = root.join("keychain");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        unsafe { std::env::set_var("AUTOMIC_VAULT_TEST_KEYCHAIN_DIR", &keychain) };
        let args = |action: &str| vec![action.into()];
        let value = r#"{"token":"streamdal-token","connections":{}}"#;
        run_with_io(&args("store"), &mut value.as_bytes(), &mut Vec::new()).unwrap();
        let mut output = Vec::new();
        run_with_io(&args("get"), &mut "".as_bytes(), &mut output).unwrap();
        assert_eq!(
            String::from_utf8(output).unwrap().trim(),
            r#"{"connections":{},"token":"streamdal-token"}"#
        );
        unsafe { std::env::remove_var("AUTOMIC_VAULT_TEST_KEYCHAIN_DIR") };
        let _ = fs::remove_dir_all(root);
    }
}
