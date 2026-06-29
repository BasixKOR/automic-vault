#![allow(dead_code)]

use std::path::PathBuf;

pub fn install_is_insecure() -> Result<bool, String> {
    install_insecurity_reasons().map(|reasons| !reasons.is_empty())
}

pub fn install_insecurity_reasons() -> Result<Vec<String>, String> {
    let path = sentry_config_path()?;
    if path.exists() && sentry_config_contains_token(&read_to_string(&path)?) {
        return Ok(vec![format!(
            "Sentry CLI config contains a plaintext auth token: {}",
            path.display()
        )]);
    }
    Ok(Vec::new())
}

fn sentry_config_path() -> Result<PathBuf, String> {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())?;
    Ok(home.join(".sentryclirc"))
}

fn read_to_string(path: &std::path::Path) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|err| format!("failed to read {}: {err}", path.display()))
}

fn sentry_config_contains_token(contents: &str) -> bool {
    let mut in_auth = false;
    for line in contents.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            in_auth = trimmed == "[auth]";
            continue;
        }
        if in_auth && token_value(trimmed).is_some_and(|value| !value.is_empty()) {
            return true;
        }
    }
    false
}

fn token_value(line: &str) -> Option<&str> {
    line.strip_prefix("token")
        .and_then(|rest| rest.trim_start().strip_prefix('='))
        .map(str::trim)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_auth_token() {
        assert!(sentry_config_contains_token(
            "[auth]\ntoken=sntrys_secret\n"
        ));
    }

    #[test]
    fn ignores_empty_auth_token() {
        assert!(!sentry_config_contains_token("[auth]\ntoken=\n"));
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
    super::radioisotope::findings("sentry-cli", install_insecurity_reasons, home)
}
