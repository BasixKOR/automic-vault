use std::ffi::OsString;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use ring::digest::{SHA256, digest};
use serde_json::{Value, json};

use super::inject;

const HELPER_STUB: &str = "#!/usr/local/bin/av terraform-credential\n";
const MAX_INPUT_BYTES: u64 = 64 * 1024;
const MAX_HOSTNAME_BYTES: usize = 253;
const SECRET_PREFIX: &str = "TERRAFORM_HOST_CREDENTIAL_";

pub(crate) fn run(mut args: Vec<OsString>, stdout: &mut dyn Write, stderr: &mut dyn Write) -> i32 {
    let mut stdin = std::io::stdin().lock();
    match run_with_io(&mut args, &mut stdin, stdout) {
        Ok(()) => 0,
        Err(error) => {
            let _ = writeln!(stderr, "terraform-credentials-av: {error}");
            1
        }
    }
}

fn run_with_io(
    args: &mut Vec<OsString>,
    input: &mut dyn Read,
    output: &mut dyn Write,
) -> Result<(), String> {
    if !args.first().is_some_and(is_helper_stub_arg) {
        return Err(
            "refusing invocation without the installed Automic Vault helper launcher".into(),
        );
    }
    args.remove(0);
    let [action, hostname] = args.as_slice() else {
        return Err("usage: terraform-credentials-av <get|store|forget> <hostname>".into());
    };
    let action = action
        .to_str()
        .ok_or_else(|| "credential-helper action must be valid UTF-8".to_string())?;
    let hostname = hostname
        .to_str()
        .ok_or_else(|| "credential-helper hostname must be valid UTF-8".to_string())?;
    let hostname = normalize_hostname(hostname)?;
    crate::secrets::ensure_terraform_helper_ready()?;
    match action {
        "get" => {
            let key = secret_name(&hostname);
            let stored = match inject::terraform_credential(key.clone(), hostname.clone()) {
                Ok(value) => value,
                Err(error) if error == format!("failed to load secret {key}: -25300") => {
                    return writeln!(output, "{{}}")
                        .map_err(|error| format!("failed to return empty credentials: {error}"));
                }
                Err(error) => return Err(error),
            };
            let token = parse_token(&stored)?;
            writeln!(output, "{}", json!({"token": token}))
                .map_err(|error| format!("failed to return credentials: {error}"))
        }
        "store" => {
            let token = parse_token(&read_limited(input)?)?;
            crate::secrets::store_terraform_credential(
                &hostname,
                &json!({"token": token}).to_string(),
            )
        }
        "forget" => crate::secrets::delete_terraform_credential(&hostname),
        _ => Err(format!("unsupported credential-helper action: {action}")),
    }
}

pub(crate) fn parse_token(input: &str) -> Result<String, String> {
    let value: Value = serde_json::from_str(input)
        .map_err(|error| format!("invalid Terraform credential JSON: {error}"))?;
    let object = value
        .as_object()
        .ok_or_else(|| "Terraform credential must be a JSON object".to_string())?;
    if object.len() != 1 || !object.contains_key("token") {
        return Err("Terraform credential must contain only `token`".into());
    }
    object["token"]
        .as_str()
        .filter(|token| !token.is_empty() && !token.as_bytes().contains(&0))
        .map(str::to_string)
        .ok_or_else(|| "Terraform credential token must be a non-empty string without NUL".into())
}

pub(crate) fn normalize_hostname(hostname: &str) -> Result<String, String> {
    if hostname.is_empty()
        || hostname.len() > MAX_HOSTNAME_BYTES
        || !hostname.is_ascii()
        || hostname.starts_with('.')
        || hostname.ends_with('.')
    {
        return Err("invalid Terraform service hostname".into());
    }
    let hostname = hostname.to_ascii_lowercase();
    if hostname.split('.').any(|label| {
        label.is_empty()
            || label.len() > 63
            || label.starts_with('-')
            || label.ends_with('-')
            || !label
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
    }) {
        return Err("invalid Terraform service hostname".into());
    }
    Ok(hostname)
}

pub(crate) fn secret_name(hostname: &str) -> String {
    let hash = digest(&SHA256, hostname.as_bytes());
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
        .map_err(|error| format!("failed to read credential-helper input: {error}"))?;
    if bytes.len() as u64 > MAX_INPUT_BYTES {
        return Err(format!(
            "credential-helper input exceeds {MAX_INPUT_BYTES} bytes"
        ));
    }
    String::from_utf8(bytes).map_err(|_| "credential-helper input must be valid UTF-8".into())
}

fn is_helper_stub_arg(arg: &OsString) -> bool {
    let path = PathBuf::from(arg);
    path == helper_path()
        && std::fs::symlink_metadata(&path).is_ok_and(|metadata| metadata.file_type().is_file())
        && std::fs::read_to_string(path).is_ok_and(|contents| contents == HELPER_STUB)
}

pub(crate) fn helper_path() -> PathBuf {
    if let Some(path) = crate::test_env_var("AUTOMIC_VAULT_TEST_TERRAFORM_HELPER_PATH") {
        return path.into();
    }
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_default()
        .join(".terraform.d/plugins/terraform-credentials-av")
}

pub(crate) fn helper_stub_valid(path: &Path) -> bool {
    std::fs::read_to_string(path).is_ok_and(|contents| contents == HELPER_STUB)
        && std::fs::symlink_metadata(path).is_ok_and(|metadata| metadata.file_type().is_file())
}

pub(crate) const fn helper_stub() -> &'static str {
    HELPER_STUB
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn protocol_round_trip_is_hostname_bound_and_rejects_unknown_fields() {
        assert_eq!(
            normalize_hostname("App.Terraform.IO").unwrap(),
            "app.terraform.io"
        );
        assert_ne!(secret_name("app.terraform.io"), secret_name("example.com"));
        assert_eq!(parse_token(r#"{"token":"secret"}"#).unwrap(), "secret");
        assert!(parse_token(r#"{"token":"secret","future":"value"}"#).is_err());
        assert!(normalize_hostname("example.com:443").is_err());
    }

    #[test]
    fn helper_store_get_forget_and_missing_get_use_test_secret_custody() {
        let _guard = crate::global_test_env_lock().lock().unwrap();
        let root = std::env::temp_dir().join(format!("av-terraform-helper-{}", std::process::id()));
        let helper = root.join("terraform-credentials-av");
        let keychain = root.join("keychain");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        fs::write(&helper, HELPER_STUB).unwrap();
        unsafe {
            std::env::set_var("AUTOMIC_VAULT_TEST_TERRAFORM_HELPER_PATH", &helper);
            std::env::set_var("AUTOMIC_VAULT_TEST_KEYCHAIN_DIR", &keychain);
        }
        let invoke = |action: &str| {
            vec![
                helper.clone().into_os_string(),
                action.into(),
                "App.Terraform.IO".into(),
            ]
        };
        run_with_io(
            &mut invoke("store"),
            &mut r#"{"token":"secret"}"#.as_bytes(),
            &mut Vec::new(),
        )
        .unwrap();
        let mut output = Vec::new();
        run_with_io(&mut invoke("get"), &mut "".as_bytes(), &mut output).unwrap();
        assert_eq!(
            serde_json::from_slice::<Value>(&output).unwrap(),
            json!({"token":"secret"})
        );
        run_with_io(&mut invoke("forget"), &mut "".as_bytes(), &mut Vec::new()).unwrap();
        output.clear();
        run_with_io(&mut invoke("get"), &mut "".as_bytes(), &mut output).unwrap();
        assert_eq!(output, b"{}\n");
        unsafe {
            std::env::remove_var("AUTOMIC_VAULT_TEST_TERRAFORM_HELPER_PATH");
            std::env::remove_var("AUTOMIC_VAULT_TEST_KEYCHAIN_DIR");
        }
        let _ = fs::remove_dir_all(root);
    }
}
