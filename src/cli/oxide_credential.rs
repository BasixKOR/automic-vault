use std::ffi::OsString;
use std::io::{Read, Write};

use ring::digest::{SHA256, digest};
use serde_json::{Value, json};
use url::Url;

use super::inject;

const MAX_INPUT_BYTES: u64 = 64 * 1024;
const MAX_PROFILE_BYTES: usize = 128;
const SECRET_PREFIX: &str = "OXIDE_PROFILE_TOKEN_";

pub(crate) fn run(args: Vec<OsString>, stdout: &mut dyn Write, stderr: &mut dyn Write) -> i32 {
    let mut stdin = std::io::stdin().lock();
    match run_with_io(&args, &mut stdin, stdout) {
        Ok(()) => 0,
        Err(error) => {
            let _ = writeln!(stderr, "oxide-credential: {error}");
            1
        }
    }
}

fn run_with_io(
    args: &[OsString],
    input: &mut dyn Read,
    output: &mut dyn Write,
) -> Result<(), String> {
    let [action, profile, host] = args else {
        return Err("usage: av oxide-credential <get|store|forget> <profile> <host>".into());
    };
    let action = action
        .to_str()
        .ok_or_else(|| "credential action must be valid UTF-8".to_string())?;
    let profile = normalize_profile(
        profile
            .to_str()
            .ok_or_else(|| "Oxide profile must be valid UTF-8".to_string())?,
    )?;
    let host = normalize_host(
        host.to_str()
            .ok_or_else(|| "Oxide host must be valid UTF-8".to_string())?,
    )?;
    let scope = scope(&profile, &host);
    let account = secret_name(&profile, &host);
    crate::secrets::ensure_oxide_helper_ready()?;
    match action {
        "get" => {
            let token = match inject::oxide_credential(account.clone(), scope) {
                Ok(value) => parse_token(&value)?,
                Err(error) if error == format!("failed to load secret {account}: -25300") => {
                    return Err(format!(
                        "no Oxide credential is stored for profile {profile:?}"
                    ));
                }
                Err(error) => return Err(error),
            };
            writeln!(output, "{token}")
                .map_err(|error| format!("failed to return Oxide credential: {error}"))
        }
        "store" => {
            let token = parse_token(&read_limited(input)?)?;
            crate::secrets::store_oxide_credential(&scope, &token)
        }
        "forget" => crate::secrets::delete_oxide_credential(&scope, &account),
        _ => Err(format!("unsupported Oxide credential action: {action}")),
    }
}

pub(crate) fn normalize_profile(profile: &str) -> Result<String, String> {
    if profile.is_empty()
        || profile.len() > MAX_PROFILE_BYTES
        || profile.trim() != profile
        || !profile.is_ascii()
        || profile.bytes().any(|byte| byte.is_ascii_control())
    {
        return Err("invalid Oxide profile name".into());
    }
    Ok(profile.to_string())
}

pub(crate) fn normalize_host(host: &str) -> Result<String, String> {
    let mut url = Url::parse(host).map_err(|_| "invalid Oxide host URL".to_string())?;
    if !matches!(url.scheme(), "http" | "https")
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || !matches!(url.path(), "" | "/")
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err("Oxide host must contain only an HTTP(S) origin".into());
    }
    if matches!(
        (url.scheme(), url.port()),
        ("https", Some(443)) | ("http", Some(80))
    ) {
        url.set_port(None)
            .map_err(|()| "invalid Oxide host URL".to_string())?;
    }
    Ok(url
        .as_str()
        .strip_suffix('/')
        .unwrap_or(url.as_str())
        .to_string())
}

pub(crate) fn parse_token(token: &str) -> Result<String, String> {
    if token.is_empty()
        || token.len() > MAX_INPUT_BYTES as usize
        || token.bytes().any(|byte| matches!(byte, 0 | b'\n' | b'\r'))
    {
        return Err("Oxide token must be non-empty and contain no NUL or line breaks".into());
    }
    Ok(token.to_string())
}

pub(crate) fn scope(profile: &str, host: &str) -> String {
    json!({"host": host, "profile": profile}).to_string()
}

pub(crate) fn parse_scope(input: &str) -> Result<(String, String), String> {
    let value: Value = serde_json::from_str(input)
        .map_err(|error| format!("invalid Oxide credential scope: {error}"))?;
    let object = value
        .as_object()
        .filter(|object| object.len() == 2)
        .ok_or_else(|| {
            "Oxide credential scope must contain only `profile` and `host`".to_string()
        })?;
    let profile = object
        .get("profile")
        .and_then(Value::as_str)
        .ok_or_else(|| "Oxide credential scope is missing `profile`".to_string())?;
    let host = object
        .get("host")
        .and_then(Value::as_str)
        .ok_or_else(|| "Oxide credential scope is missing `host`".to_string())?;
    let profile = normalize_profile(profile)?;
    let host = normalize_host(host)?;
    if input != scope(&profile, &host) {
        return Err("Oxide credential scope is not canonical".into());
    }
    Ok((profile, host))
}

pub(crate) fn secret_name(profile: &str, host: &str) -> String {
    let hash = digest(&SHA256, format!("{profile}\0{host}").as_bytes());
    let hex = hash
        .as_ref()
        .iter()
        .map(|byte| format!("{byte:02X}"))
        .collect::<String>();
    format!("{SECRET_PREFIX}{hex}")
}

fn read_limited(input: &mut dyn Read) -> Result<String, String> {
    let mut bytes = Vec::new();
    input
        .take(MAX_INPUT_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| format!("failed to read Oxide credential: {error}"))?;
    if bytes.len() as u64 > MAX_INPUT_BYTES {
        return Err(format!("Oxide credential exceeds {MAX_INPUT_BYTES} bytes"));
    }
    String::from_utf8(bytes).map_err(|_| "Oxide credential must be valid UTF-8".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn scope_is_canonical_and_profile_bound() {
        assert_eq!(
            normalize_host("https://OXIDE.example/").unwrap(),
            "https://oxide.example"
        );
        assert_eq!(
            normalize_host("https://oxide.example:443/").unwrap(),
            "https://oxide.example"
        );
        assert_eq!(
            normalize_host("http://oxide.example:80/").unwrap(),
            "http://oxide.example"
        );
        assert_eq!(
            normalize_host("https://oxide.example:8443/").unwrap(),
            "https://oxide.example:8443"
        );
        assert!(normalize_host("https://oxide.example/path").is_err());
        assert_ne!(
            secret_name("prod", "https://oxide.example"),
            secret_name("dev", "https://oxide.example")
        );
        let value = scope("prod", "https://oxide.example");
        assert_eq!(
            parse_scope(&value).unwrap(),
            ("prod".into(), "https://oxide.example".into())
        );
        assert!(
            parse_scope(r#"{"profile":"prod","host":"https://oxide.example","extra":true}"#)
                .is_err()
        );
    }

    #[test]
    fn helper_store_get_and_forget_use_test_secret_custody() {
        let _guard = crate::global_test_env_lock().lock().unwrap();
        let root = std::env::temp_dir().join(format!("av-oxide-helper-{}", std::process::id()));
        let keychain = root.join("keychain");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        unsafe { std::env::set_var("AUTOMIC_VAULT_TEST_KEYCHAIN_DIR", &keychain) };
        let invoke =
            |action: &str| vec![action.into(), "prod".into(), "https://oxide.example".into()];
        run_with_io(&invoke("store"), &mut "token".as_bytes(), &mut Vec::new()).unwrap();
        let mut output = Vec::new();
        run_with_io(&invoke("get"), &mut "".as_bytes(), &mut output).unwrap();
        assert_eq!(output, b"token\n");
        run_with_io(&invoke("forget"), &mut "".as_bytes(), &mut Vec::new()).unwrap();
        assert!(run_with_io(&invoke("get"), &mut "".as_bytes(), &mut Vec::new()).is_err());
        unsafe { std::env::remove_var("AUTOMIC_VAULT_TEST_KEYCHAIN_DIR") };
        let _ = fs::remove_dir_all(root);
    }
}
