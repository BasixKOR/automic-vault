#![allow(dead_code)]

use std::path::PathBuf;

pub fn install_is_insecure() -> Result<bool, String> {
    install_insecurity_reasons().map(|reasons| !reasons.is_empty())
}

pub fn install_insecurity_reasons() -> Result<Vec<String>, String> {
    let path = hcloud_config_path()?;
    if path.exists() && config_contains_token(&read_to_string(&path)?) {
        return Ok(vec![format!(
            "hcloud config file contains plaintext API tokens: {}",
            path.display()
        )]);
    }
    Ok(Vec::new())
}

fn hcloud_config_path() -> Result<PathBuf, String> {
    if let Some(path) = std::env::var_os("HCLOUD_CONFIG").filter(|value| !value.is_empty()) {
        return Ok(PathBuf::from(path));
    }

    let config_home = if let Some(path) = std::env::var_os("XDG_CONFIG_HOME") {
        PathBuf::from(path)
    } else {
        user_home()?.join(".config")
    };
    Ok(config_home.join("hcloud/cli.toml"))
}

fn user_home() -> Result<PathBuf, String> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())
}

fn read_to_string(path: &std::path::Path) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|err| format!("failed to read {}: {err}", path.display()))
}

fn config_contains_token(contents: &str) -> bool {
    contents.lines().any(line_has_token)
}

fn line_has_token(line: &str) -> bool {
    let line = line.trim();
    if line.is_empty() || line.starts_with('#') {
        return false;
    }
    let Some((key, value)) = line.split_once('=') else {
        return false;
    };
    key.trim() == "token" && !toml_string_value(value).unwrap_or_default().is_empty()
}

fn toml_string_value(value: &str) -> Option<&str> {
    value
        .trim()
        .strip_prefix('"')
        .and_then(|value| value.split_once('"').map(|(value, _)| value))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_context_token() {
        assert!(config_contains_token(
            "active_context = \"prod\"\n\n[[contexts]]\nname = \"prod\"\ntoken = \"hcloud-token\"\n"
        ));
    }

    #[test]
    fn ignores_empty_token() {
        assert!(!config_contains_token("[[contexts]]\ntoken = \"\"\n"));
    }

    #[test]
    fn ignores_comments() {
        assert!(!config_contains_token("# token = \"hcloud-token\"\n"));
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
    super::radioisotope::findings("hcloud", install_insecurity_reasons, home)
}
