#![allow(dead_code)]

use std::path::PathBuf;

pub fn install_is_insecure() -> Result<bool, String> {
    install_insecurity_reasons().map(|reasons| !reasons.is_empty())
}

pub fn install_insecurity_reasons() -> Result<Vec<String>, String> {
    let path = talosconfig_path()?;
    if path.exists() && talosconfig_has_secrets(&read_to_string(&path)?) {
        return Ok(vec![format!(
            "talosctl config contains client credentials: {}",
            path.display()
        )]);
    }
    Ok(Vec::new())
}

fn talosconfig_path() -> Result<PathBuf, String> {
    if let Some(path) = std::env::var_os("TALOSCONFIG") {
        return Ok(PathBuf::from(path));
    }
    let talos_home = if let Some(path) = std::env::var_os("TALOS_HOME") {
        PathBuf::from(path)
    } else {
        user_home()?.join(".talos")
    };
    Ok(talos_home.join("config"))
}

fn user_home() -> Result<PathBuf, String> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())
}

fn read_to_string(path: &std::path::Path) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|err| format!("failed to read {}: {err}", path.display()))
}

fn talosconfig_has_secrets(contents: &str) -> bool {
    contents.lines().any(line_has_secret)
}

fn line_has_secret(line: &str) -> bool {
    let trimmed = line.trim();
    let Some((key, value)) = trimmed.split_once(':') else {
        return false;
    };
    let key = key.trim();
    let value = value.trim().trim_matches('"').trim_matches('\'');
    matches!(key, "key" | "crt" | "ca" | "username" | "password")
        && !value.is_empty()
        && !value.eq_ignore_ascii_case("null")
        && value != "[]"
        && value != "{}"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_client_key_material() {
        assert!(talosconfig_has_secrets(
            "contexts:\n  prod:\n    crt: LS0t\n    key: LS0t\n"
        ));
    }

    #[test]
    fn detects_basic_auth_passwords() {
        assert!(talosconfig_has_secrets(
            "auth:\n  basic:\n    password: secret\n"
        ));
    }

    #[test]
    fn ignores_empty_values() {
        assert!(!talosconfig_has_secrets(
            "contexts:\n  prod:\n    key: \"\"\n"
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
    super::radioisotope::findings("talosctl", install_insecurity_reasons, home)
}
