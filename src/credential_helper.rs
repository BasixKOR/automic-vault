use std::ffi::OsStr;
use std::io::Write;
use std::process::Command;

use crate::stub;

const TOKEN_ENV: &str = "AUTOMIC_VAULT_CREDENTIAL_HELPER_TOKEN";
const AWS_ACCESS_KEY_ID: &str = "AWS_ACCESS_KEY_ID";
const AWS_SECRET_ACCESS_KEY: &str = "AWS_SECRET_ACCESS_KEY";

pub(crate) fn run(protocol: &OsStr, stdout: &mut dyn Write) -> Result<(), String> {
    if protocol != "aws" {
        return Err("unknown credential helper protocol".to_string());
    }
    let token = std::env::var(TOKEN_ENV)
        .map_err(|_| "missing AWS credential_process approval token".to_string())?;
    stub::broker_request(&format!("validate aws {token}\n"))?;

    let access_key_id = load_secret(AWS_ACCESS_KEY_ID)?;
    let secret_access_key = load_secret(AWS_SECRET_ACCESS_KEY)?;
    writeln!(
        stdout,
        "{{\"AccessKeyId\":{},\"SecretAccessKey\":{},\"Version\":1}}",
        json_string(&access_key_id),
        json_string(&secret_access_key)
    )
    .map_err(|err| format!("failed to write AWS credential_process response: {err}"))
}

fn load_secret(key: &str) -> Result<String, String> {
    if let Ok(value) = std::env::var(format!("AUTOMIC_VAULT_TEST_{key}")) {
        return Ok(value);
    }
    let output = Command::new("/usr/bin/security")
        .args([
            "find-generic-password",
            "-s",
            "com.automicvault.isotope",
            "-a",
            key,
            "-w",
        ])
        .output()
        .map_err(|err| format!("failed to run security: {err}"))?;
    if !output.status.success() {
        return Err(format!("failed to load isotope key {key}"));
    }
    String::from_utf8(output.stdout)
        .map(|value| value.trim_end_matches('\n').to_string())
        .map_err(|_| format!("isotope key {key} is not valid UTF-8"))
}

fn json_string(value: &str) -> String {
    let mut out = String::from("\"");
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            value if value < ' ' => out.push_str(&format!("\\u{:04x}", value as u32)),
            value => out.push(value),
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_escapes_secret_values() {
        assert_eq!(json_string("a\"b\\c\n"), r#""a\"b\\c\n""#);
    }
}
