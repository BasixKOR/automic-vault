#![allow(dead_code)]

use std::path::{Path, PathBuf};

pub fn install_is_insecure() -> Result<bool, String> {
    install_insecurity_reasons().map(|reasons| !reasons.is_empty())
}

pub fn install_insecurity_reasons() -> Result<Vec<String>, String> {
    let path = checkmarx_config_path()?;
    if path.exists() && checkmarx_config_contains_secret(&read_to_string(&path)?) {
        return Ok(vec![format!(
            "Checkmarx AST config contains plaintext credentials: {}",
            path.display()
        )]);
    }
    Ok(Vec::new())
}

fn checkmarx_config_path() -> Result<PathBuf, String> {
    if let Some(path) = std::env::var_os("CX_CONFIG_FILE_PATH").filter(|value| !value.is_empty()) {
        return Ok(PathBuf::from(path));
    }

    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())?;
    Ok(home.join(".checkmarx/checkmarxcli.yaml"))
}

fn read_to_string(path: &Path) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|err| format!("failed to read {}: {err}", path.display()))
}

fn checkmarx_config_contains_secret(contents: &str) -> bool {
    contents.lines().any(|line| {
        parse_secret_line(line)
            .map(|(_, value)| !value.is_empty())
            .unwrap_or(false)
    })
}

fn parse_secret_line(line: &str) -> Option<(&str, String)> {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return None;
    }

    let (key, value) = trimmed.split_once(':')?;
    let key = key.trim();
    if key != "cx_apikey" && key != "cx_client_secret" {
        return None;
    }

    Some((key, unquote_yaml_scalar(value.trim()).to_string()))
}

fn unquote_yaml_scalar(value: &str) -> &str {
    if value.len() >= 2 {
        let bytes = value.as_bytes();
        if (bytes[0] == b'"' && bytes[value.len() - 1] == b'"')
            || (bytes[0] == b'\'' && bytes[value.len() - 1] == b'\'')
        {
            return &value[1..value.len() - 1];
        }
    }
    value
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_api_key_and_client_secret() {
        assert!(checkmarx_config_contains_secret(
            "cx_base_uri: https://example.invalid\ncx_apikey: ast_secret\n"
        ));
        assert!(checkmarx_config_contains_secret(
            "cx_client_id: client\ncx_client_secret: oauth_secret\n"
        ));
    }

    #[test]
    fn ignores_empty_secret_fields() {
        assert!(!checkmarx_config_contains_secret(
            "cx_apikey: \"\"\ncx_client_secret: ''\n"
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
    super::radioisotope::findings("ast-cli", install_insecurity_reasons, home)
}
