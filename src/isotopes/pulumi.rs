#![allow(dead_code)]

use std::path::{Path, PathBuf};

pub fn install_is_insecure() -> Result<bool, String> {
    install_insecurity_reasons().map(|reasons| !reasons.is_empty())
}

pub fn install_insecurity_reasons() -> Result<Vec<String>, String> {
    let path = pulumi_credentials_path()?;
    if path.exists() && pulumi_credentials_contains_access_token(&read_to_string(&path)?) {
        return Ok(vec![format!(
            "Pulumi credentials file contains plaintext access tokens: {}",
            path.display()
        )]);
    }
    Ok(Vec::new())
}

fn pulumi_credentials_path() -> Result<PathBuf, String> {
    if let Some(dir) = std::env::var_os("PULUMI_CREDENTIALS_PATH").filter(|value| !value.is_empty())
    {
        return Ok(PathBuf::from(dir).join("credentials.json"));
    }
    if let Some(dir) = std::env::var_os("PULUMI_HOME").filter(|value| !value.is_empty()) {
        return Ok(PathBuf::from(dir).join("credentials.json"));
    }

    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())?;
    Ok(home.join(".pulumi/credentials.json"))
}

fn read_to_string(path: &Path) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|err| format!("failed to read {}: {err}", path.display()))
}

fn pulumi_credentials_contains_access_token(contents: &str) -> bool {
    let Some(access_tokens_offset) = contents.find("\"accessTokens\"") else {
        return false;
    };
    let Some(open_brace_offset) = contents[access_tokens_offset..].find('{') else {
        return false;
    };
    let start = access_tokens_offset + open_brace_offset + 1;
    let Some(end) = matching_object_end(&contents[start - 1..]) else {
        return false;
    };
    let object = &contents[start..start - 1 + end];
    object_contains_non_empty_string_value(object)
}

fn matching_object_end(object: &str) -> Option<usize> {
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    for (index, byte) in object.bytes().enumerate() {
        if escaped {
            escaped = false;
            continue;
        }
        if byte == b'\\' && in_string {
            escaped = true;
            continue;
        }
        if byte == b'"' {
            in_string = !in_string;
            continue;
        }
        if in_string {
            continue;
        }
        match byte {
            b'{' => depth += 1,
            b'}' => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some(index);
                }
            }
            _ => {}
        }
    }
    None
}

fn object_contains_non_empty_string_value(object: &str) -> bool {
    let mut index = 0usize;
    while index < object.len() {
        index = skip_json_space_and_commas(object, index);
        let Some((_, after_key)) = parse_json_string(object, index) else {
            return false;
        };
        index = skip_json_space(object, after_key);
        if object.as_bytes().get(index) != Some(&b':') {
            return false;
        }
        index = skip_json_space(object, index + 1);
        let Some((value, after_value)) = parse_json_string(object, index) else {
            return false;
        };
        if !value.is_empty() {
            return true;
        }
        index = after_value;
    }
    false
}

fn skip_json_space_and_commas(value: &str, mut index: usize) -> usize {
    while let Some(byte) = value.as_bytes().get(index) {
        if !matches!(byte, b' ' | b'\n' | b'\r' | b'\t' | b',') {
            break;
        }
        index += 1;
    }
    index
}

fn skip_json_space(value: &str, mut index: usize) -> usize {
    while let Some(byte) = value.as_bytes().get(index) {
        if !matches!(byte, b' ' | b'\n' | b'\r' | b'\t') {
            break;
        }
        index += 1;
    }
    index
}

fn parse_json_string(value: &str, start: usize) -> Option<(String, usize)> {
    if value.as_bytes().get(start) != Some(&b'"') {
        return None;
    }
    let mut escaped = false;
    let mut result = String::new();
    for (offset, byte) in value[start + 1..].bytes().enumerate() {
        if escaped {
            result.push(byte as char);
            escaped = false;
            continue;
        }
        if byte == b'\\' {
            escaped = true;
            continue;
        }
        if byte == b'"' {
            return Some((result, start + 1 + offset + 1));
        }
        result.push(byte as char);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_access_token() {
        assert!(pulumi_credentials_contains_access_token(
            r#"{"current":"https://api.pulumi.com","accessTokens":{"https://api.pulumi.com":"pul-secret"}}"#
        ));
    }

    #[test]
    fn ignores_empty_access_tokens() {
        assert!(!pulumi_credentials_contains_access_token(
            r#"{"accessTokens":{}}"#
        ));
        assert!(!pulumi_credentials_contains_access_token(
            r#"{"accessTokens":{"https://api.pulumi.com":""}}"#
        ));
    }

    #[test]
    fn top_level_install_is_insecure_returns_false_when_default_locations_are_missing() {
        let home = std::env::temp_dir().join(format!(
            "{}-detect-missing-{}",
            module_path!().replace(':', "_"),
            std::process::id()
        ));
        let xdg = home.join("xdg");
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(&xdg).unwrap();

        let previous_home = std::env::var_os("HOME");
        let previous_xdg = std::env::var_os("XDG_CONFIG_HOME");
        unsafe {
            std::env::set_var("HOME", &home);
            std::env::set_var("XDG_CONFIG_HOME", &xdg);
        }

        let result = install_is_insecure().unwrap();

        unsafe {
            match previous_home {
                Some(value) => std::env::set_var("HOME", value),
                None => std::env::remove_var("HOME"),
            }
            match previous_xdg {
                Some(value) => std::env::set_var("XDG_CONFIG_HOME", value),
                None => std::env::remove_var("XDG_CONFIG_HOME"),
            }
        }

        assert!(!result);
        std::fs::remove_dir_all(home).unwrap();
    }
}

pub(crate) fn findings(home: &std::path::Path) -> Vec<crate::Finding> {
    super::radioisotope::findings("pulumi", install_insecurity_reasons, home)
}
