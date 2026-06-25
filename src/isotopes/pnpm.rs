#![allow(dead_code)]

use std::path::PathBuf;

pub fn install_is_insecure() -> Result<bool, String> {
    install_insecurity_reasons().map(|reasons| !reasons.is_empty())
}

pub fn install_insecurity_reasons() -> Result<Vec<String>, String> {
    let npmrc = npm_user_config_path()?;
    if !npmrc.exists() {
        return Ok(Vec::new());
    }

    let contents = std::fs::read_to_string(&npmrc)
        .map_err(|err| format!("failed to read {}: {err}", npmrc.display()))?;
    if npmrc_contains_plaintext_auth_token(&contents) {
        return Ok(vec![format!(
            "npm user config contains a plaintext auth token: {}",
            npmrc.display()
        )]);
    }

    Ok(Vec::new())
}

fn npm_user_config_path() -> Result<PathBuf, String> {
    if let Some(path) = std::env::var_os("NPM_CONFIG_USERCONFIG").filter(|value| !value.is_empty())
    {
        return Ok(PathBuf::from(path));
    }

    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())?;
    Ok(home.join(".npmrc"))
}

fn npmrc_contains_plaintext_auth_token(contents: &str) -> bool {
    contents.lines().any(|line| {
        parse_auth_token_line(line).is_some_and(|(_, value)| auth_token_value_is_plaintext(value))
    })
}

fn parse_auth_token_line(line: &str) -> Option<(&str, &str)> {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with(';') {
        return None;
    }

    let (key, value) = trimmed.split_once('=')?;
    let key = key.trim();
    if !key.ends_with(":_authToken") && key != "_authToken" {
        return None;
    }

    Some((key, value.trim()))
}

fn auth_token_value_is_plaintext(value: &str) -> bool {
    !value.is_empty() && value != "${NODE_AUTH_TOKEN}"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_plaintext_registry_token() {
        assert!(npmrc_contains_plaintext_auth_token(
            "//registry.npmjs.org/:_authToken=npm_secret\n"
        ));
    }

    #[test]
    fn ignores_env_placeholder_token() {
        assert!(!npmrc_contains_plaintext_auth_token(
            "//registry.npmjs.org/:_authToken=${NODE_AUTH_TOKEN}\n"
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
    super::radioisotope::findings("pnpm", install_insecurity_reasons, home)
}
