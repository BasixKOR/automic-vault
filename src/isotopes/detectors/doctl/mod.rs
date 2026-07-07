#![allow(dead_code)]

use std::path::{Path, PathBuf};

pub fn install_is_insecure() -> Result<bool, String> {
    install_insecurity_reasons().map(|reasons| !reasons.is_empty())
}

pub fn install_insecurity_reasons() -> Result<Vec<String>, String> {
    let path = doctl_config_path()?;
    if path.exists() && doctl_config_contains_token(&read_to_string(&path)?) {
        return Ok(vec![format!(
            "doctl config contains plaintext DigitalOcean tokens: {}",
            path.display()
        )]);
    }
    Ok(Vec::new())
}

fn doctl_config_path() -> Result<PathBuf, String> {
    if let Some(path) = std::env::var_os("DIGITALOCEAN_CONFIG").filter(|value| !value.is_empty()) {
        return Ok(PathBuf::from(path));
    }

    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())?;
    if cfg!(target_os = "macos") {
        Ok(home.join("Library/Application Support/doctl/config.yaml"))
    } else if let Some(xdg_config_home) =
        std::env::var_os("XDG_CONFIG_HOME").filter(|value| !value.is_empty())
    {
        Ok(PathBuf::from(xdg_config_home).join("doctl/config.yaml"))
    } else {
        Ok(home.join(".config/doctl/config.yaml"))
    }
}

fn read_to_string(path: &Path) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|err| format!("failed to read {}: {err}", path.display()))
}

fn doctl_config_contains_token(contents: &str) -> bool {
    let mut in_auth_contexts = false;

    for line in contents.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let indent = line.len() - line.trim_start().len();
        if indent == 0 {
            in_auth_contexts = trimmed == "auth-contexts:";
            if line_has_non_empty_yaml_value(trimmed, "access-token") {
                return true;
            }
            continue;
        }
        if in_auth_contexts
            && yaml_value_after_colon(trimmed).is_some_and(|value| !value.is_empty())
        {
            return true;
        }
    }

    false
}

fn line_has_non_empty_yaml_value(line: &str, key: &str) -> bool {
    line.strip_prefix(key)
        .and_then(|rest| rest.strip_prefix(':'))
        .map(str::trim)
        .map(unquote_yaml_scalar)
        .is_some_and(|value| !value.is_empty())
}

fn yaml_value_after_colon(line: &str) -> Option<&str> {
    line.split_once(':')
        .map(|(_, value)| unquote_yaml_scalar(value.trim()))
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
    fn detects_default_access_token() {
        assert!(doctl_config_contains_token("access-token: do_secret\n"));
    }

    #[test]
    fn detects_named_auth_context_token() {
        assert!(doctl_config_contains_token(
            "access-token: \"\"\nauth-contexts:\n  team: do_team_secret\n"
        ));
    }

    #[test]
    fn ignores_empty_tokens() {
        assert!(!doctl_config_contains_token(
            "access-token: \"\"\nauth-contexts:\n  team: ''\n"
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
    super::radioisotope::findings("doctl", install_insecurity_reasons, home)
}
