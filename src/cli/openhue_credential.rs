use std::ffi::OsString;
use std::io::{Read, Write};

use serde::{Deserialize, Serialize};

use super::inject;

const MAX_KEY_BYTES: u64 = 64 * 1024;
pub(crate) const SECRET_NAME: &str = "OPENHUE_APPLICATION_KEY";

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Scope {
    bridge: String,
}

pub(crate) fn run(args: Vec<OsString>, stdout: &mut dyn Write, stderr: &mut dyn Write) -> i32 {
    let mut stdin = std::io::stdin().lock();
    match run_with_io(&args, &mut stdin, stdout) {
        Ok(()) => 0,
        Err(error) => {
            let _ = writeln!(stderr, "openhue-credential: {error}");
            1
        }
    }
}

fn run_with_io(
    args: &[OsString],
    input: &mut dyn Read,
    output: &mut dyn Write,
) -> Result<(), String> {
    let [action, bridge] = args else {
        return Err("usage: av openhue-credential <get|store> BRIDGE".into());
    };
    let action = action
        .to_str()
        .ok_or_else(|| "credential action must be valid UTF-8".to_string())?;
    let bridge = bridge
        .to_str()
        .ok_or_else(|| "bridge scope must be valid UTF-8".to_string())?;
    let scope = scope(bridge)?;
    crate::secrets::ensure_openhue_helper_ready()?;
    match action {
        "get" => {
            let value = inject::openhue_credential(SECRET_NAME.into(), scope)?;
            writeln!(output, "{}", validate_key(&value)?)
                .map_err(|error| format!("failed to return OpenHue credential: {error}"))
        }
        "store" => {
            crate::secrets::store_openhue_credential(&scope, &validate_key(&read_limited(input)?)?)
        }
        _ => Err(format!("unsupported OpenHue credential action: {action}")),
    }
}

pub(crate) fn scope(bridge: &str) -> Result<String, String> {
    validate_bridge(bridge)?;
    serde_json::to_string(&Scope {
        bridge: bridge.into(),
    })
    .map_err(|error| format!("failed to encode OpenHue bridge scope: {error}"))
}

pub(crate) fn parse_scope(input: &str) -> Result<String, String> {
    let parsed: Scope = serde_json::from_str(input)
        .map_err(|error| format!("invalid OpenHue bridge scope: {error}"))?;
    validate_bridge(&parsed.bridge)?;
    (scope(&parsed.bridge)? == input)
        .then_some(parsed.bridge)
        .ok_or_else(|| "OpenHue bridge scope is not canonical".into())
}

pub(crate) fn validate_bridge(bridge: &str) -> Result<(), String> {
    if bridge.is_empty() || bridge.len() > 255 || bridge.bytes().any(|byte| byte == 0) {
        return Err("invalid OpenHue bridge scope".into());
    }
    Ok(())
}

pub(crate) fn validate_key(value: &str) -> Result<String, String> {
    if value.is_empty()
        || value == "@av"
        || value.len() > MAX_KEY_BYTES as usize
        || value.bytes().any(|byte| byte == 0)
    {
        return Err("invalid OpenHue application key".into());
    }
    Ok(value.to_string())
}

fn read_limited(input: &mut dyn Read) -> Result<String, String> {
    let mut bytes = Vec::new();
    input
        .take(MAX_KEY_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| format!("failed to read OpenHue application key: {error}"))?;
    if bytes.len() as u64 > MAX_KEY_BYTES {
        return Err("OpenHue application key exceeds 64 KiB".into());
    }
    String::from_utf8(bytes).map_err(|_| "OpenHue application key must be valid UTF-8".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn scope_and_key_are_strict() {
        assert_eq!(scope("192.0.2.10").unwrap(), r#"{"bridge":"192.0.2.10"}"#);
        assert_eq!(
            parse_scope(r#"{"bridge":"192.0.2.10"}"#).unwrap(),
            "192.0.2.10"
        );
        assert!(parse_scope(r#"{"bridge":"192.0.2.10","future":true}"#).is_err());
        assert!(validate_key("application-key").is_ok());
        assert!(validate_key("@av").is_err());
    }

    #[test]
    fn helper_round_trip_uses_test_custody() {
        let _guard = crate::global_test_env_lock().lock().unwrap();
        let root = std::env::temp_dir().join(format!("av-openhue-helper-{}", std::process::id()));
        let keychain = root.join("keychain");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        unsafe { std::env::set_var("AUTOMIC_VAULT_TEST_KEYCHAIN_DIR", &keychain) };
        let args = |action: &str| vec![action.into(), "192.0.2.10".into()];
        run_with_io(
            &args("store"),
            &mut "application-key".as_bytes(),
            &mut Vec::new(),
        )
        .unwrap();
        let mut output = Vec::new();
        run_with_io(&args("get"), &mut "".as_bytes(), &mut output).unwrap();
        assert_eq!(String::from_utf8(output).unwrap().trim(), "application-key");
        unsafe { std::env::remove_var("AUTOMIC_VAULT_TEST_KEYCHAIN_DIR") };
        let _ = fs::remove_dir_all(root);
    }
}
