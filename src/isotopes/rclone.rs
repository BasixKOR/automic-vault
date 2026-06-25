#![allow(dead_code)]

use std::path::PathBuf;

pub fn install_is_insecure() -> Result<bool, String> {
    install_insecurity_reasons().map(|reasons| !reasons.is_empty())
}

pub fn install_insecurity_reasons() -> Result<Vec<String>, String> {
    for path in candidate_config_paths()? {
        if path.exists() && rclone_config_contains_secret(&read_to_string(&path)?) {
            return Ok(vec![format!(
                "rclone config file contains stored credentials: {}",
                path.display()
            )]);
        }
    }
    Ok(Vec::new())
}

fn candidate_config_paths() -> Result<Vec<PathBuf>, String> {
    if let Some(path) = std::env::var_os("RCLONE_CONFIG").filter(|value| !value.is_empty()) {
        return Ok(vec![PathBuf::from(path)]);
    }

    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())?;
    let mut paths = Vec::new();
    if let Some(config) = std::env::var_os("XDG_CONFIG_HOME").filter(|value| !value.is_empty()) {
        paths.push(PathBuf::from(config).join("rclone/rclone.conf"));
    }
    paths.push(home.join(".config/rclone/rclone.conf"));
    paths.push(home.join(".rclone.conf"));
    Ok(paths)
}

fn read_to_string(path: &std::path::Path) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|err| format!("failed to read {}: {err}", path.display()))
}

fn rclone_config_contains_secret(contents: &str) -> bool {
    contents.lines().any(line_has_secret_value)
}

fn line_has_secret_value(line: &str) -> bool {
    let line = line.trim();
    if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
        return false;
    }
    let Some((key, value)) = line.split_once('=') else {
        return false;
    };
    let key = key.trim().to_ascii_lowercase();
    let value = value.trim();
    !value.is_empty()
        && (key == "token"
            || key == "pass"
            || key == "password"
            || key.ends_with("_password")
            || key == "client_secret"
            || key.ends_with("_client_secret")
            || key == "access_token"
            || key == "refresh_token")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_token() {
        assert!(rclone_config_contains_secret(
            "[remote]\ntoken = {\"access_token\":\"x\"}\n"
        ));
    }

    #[test]
    fn detects_obscured_password() {
        assert!(rclone_config_contains_secret("[remote]\npass = obscured\n"));
    }

    #[test]
    fn ignores_comments() {
        assert!(!rclone_config_contains_secret("# token = secret\n"));
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
    super::radioisotope::findings("rclone", install_insecurity_reasons, home)
}
