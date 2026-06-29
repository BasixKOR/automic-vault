#![allow(dead_code)]

use std::path::PathBuf;

pub fn install_is_insecure() -> Result<bool, String> {
    install_insecurity_reasons().map(|reasons| !reasons.is_empty())
}

pub fn install_insecurity_reasons() -> Result<Vec<String>, String> {
    let path = circleci_config_path()?;
    if path.exists() && circleci_config_has_token(&read_to_string(&path)?) {
        return Ok(vec![format!(
            "CircleCI config contains an API token: {}",
            path.display()
        )]);
    }
    Ok(Vec::new())
}

fn circleci_config_path() -> Result<PathBuf, String> {
    Ok(user_home()?.join(".circleci/cli.yml"))
}

fn user_home() -> Result<PathBuf, String> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())
}

fn read_to_string(path: &std::path::Path) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|err| format!("failed to read {}: {err}", path.display()))
}

fn circleci_config_has_token(contents: &str) -> bool {
    contents.lines().any(line_has_token)
}

fn line_has_token(line: &str) -> bool {
    let trimmed = line.trim();
    let Some((key, value)) = trimmed.split_once(':') else {
        return false;
    };
    let value = value.trim().trim_matches('"').trim_matches('\'');
    key.trim() == "token"
        && !value.is_empty()
        && !value.eq_ignore_ascii_case("null")
        && value != "token"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_circleci_token() {
        assert!(circleci_config_has_token(
            "host: https://circleci.com\ntoken: abc123\n"
        ));
    }

    #[test]
    fn ignores_empty_or_placeholder_tokens() {
        assert!(!circleci_config_has_token("token: \n"));
        assert!(!circleci_config_has_token("token: token\n"));
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
    super::radioisotope::findings("circleci", install_insecurity_reasons, home)
}
