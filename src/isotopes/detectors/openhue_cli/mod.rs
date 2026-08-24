#![allow(dead_code)]

use std::path::{Path, PathBuf};

pub fn install_is_insecure() -> Result<bool, String> {
    install_insecurity_reasons().map(|reasons| !reasons.is_empty())
}

pub fn install_insecurity_reasons() -> Result<Vec<String>, String> {
    let path = openhue_config_path()?;
    if path.exists() && openhue_application_key(&read_to_string(&path)?).is_some() {
        return Ok(vec![format!(
            "OpenHue config contains a plaintext Hue application key: {}",
            path.display()
        )]);
    }
    Ok(Vec::new())
}

fn openhue_config_path() -> Result<PathBuf, String> {
    if let Some(xdg_config_home) =
        std::env::var_os("XDG_CONFIG_HOME").filter(|value| !value.is_empty())
    {
        return Ok(PathBuf::from(xdg_config_home).join("openhue/config.yaml"));
    }

    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())?;
    Ok(home.join(".openhue/config.yaml"))
}

fn read_to_string(path: &Path) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|err| format!("failed to read {}: {err}", path.display()))
}

fn openhue_application_key(contents: &str) -> Option<String> {
    contents.lines().find_map(parse_key_line)
}

fn parse_key_line(line: &str) -> Option<String> {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return None;
    }
    let (key, value) = trimmed.split_once(':')?;
    if !key.trim().eq_ignore_ascii_case("key") {
        return None;
    }

    let value = unquote_yaml_scalar(value.trim());
    if value.is_empty() || value == "@av" {
        None
    } else {
        Some(value.to_string())
    }
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
    fn detects_hue_application_key() {
        assert_eq!(
            openhue_application_key("Bridge: 192.0.2.10\nKey: hue_secret\n"),
            Some("hue_secret".to_string())
        );
    }

    #[test]
    fn ignores_comments_and_empty_keys() {
        assert_eq!(
            openhue_application_key("# Key: secret\nBridge: 192.0.2.10\nKey: \"\"\n"),
            None
        );
        assert_eq!(
            openhue_application_key("bridge: 192.0.2.10\nkey: '@av'\n"),
            None
        );
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
    super::radioisotope::findings("openhue-cli", install_insecurity_reasons, home)
}
