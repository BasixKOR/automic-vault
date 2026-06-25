#![allow(dead_code)]

use std::path::PathBuf;

pub fn install_is_insecure() -> Result<bool, String> {
    install_insecurity_reasons().map(|reasons| !reasons.is_empty())
}

pub fn install_insecurity_reasons() -> Result<Vec<String>, String> {
    let mut reasons = Vec::new();
    for path in cargo_credentials_paths()? {
        if !path.exists() {
            continue;
        }
        let contents = std::fs::read_to_string(&path)
            .map_err(|err| format!("failed to read {}: {err}", path.display()))?;
        if cargo_credentials_contain_plaintext_token(&contents) {
            reasons.push(format!(
                "Cargo credentials contain a plaintext registry token: {}",
                path.display()
            ));
        }
    }
    Ok(reasons)
}

fn cargo_credentials_paths() -> Result<Vec<PathBuf>, String> {
    if let Some(path) = std::env::var_os("CARGO_HOME").filter(|value| !value.is_empty()) {
        let cargo_home = PathBuf::from(path);
        return Ok(vec![
            cargo_home.join("credentials.toml"),
            cargo_home.join("credentials"),
        ]);
    }

    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())?;
    Ok(vec![
        home.join(".cargo/credentials.toml"),
        home.join(".cargo/credentials"),
    ])
}

fn cargo_credentials_contain_plaintext_token(contents: &str) -> bool {
    let mut in_default_registry = false;

    for line in contents.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        if let Some(table) = parse_table_header(trimmed) {
            in_default_registry = table == "registry";
            continue;
        }

        if !in_default_registry {
            continue;
        }

        let Some((key, value)) = parse_assignment(trimmed) else {
            continue;
        };
        if key == "token" && token_value_is_plaintext(value) {
            return true;
        }
    }

    false
}

fn parse_table_header(line: &str) -> Option<&str> {
    line.strip_prefix('[')
        .and_then(|value| value.strip_suffix(']'))
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn parse_assignment(line: &str) -> Option<(&str, &str)> {
    let (key, value) = line.split_once('=')?;
    Some((key.trim(), value.trim()))
}

fn token_value_is_plaintext(value: &str) -> bool {
    let value = strip_inline_comment(value).trim();
    parse_toml_string(value).is_some_and(|token| !token.is_empty())
}

fn strip_inline_comment(value: &str) -> &str {
    value
        .split_once('#')
        .map(|(value, _)| value)
        .unwrap_or(value)
}

fn parse_toml_string(value: &str) -> Option<&str> {
    value
        .strip_prefix('"')
        .and_then(|value| value.split_once('"').map(|(token, _)| token))
        .or_else(|| {
            value
                .strip_prefix('\'')
                .and_then(|value| value.split_once('\'').map(|(token, _)| token))
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_default_registry_token() {
        let contents = "[registry]\ntoken = \"cargo_secret\"\n";

        assert!(cargo_credentials_contain_plaintext_token(contents));
    }

    #[test]
    fn ignores_custom_registry_token() {
        let contents = "[registries.internal]\ntoken = \"secret\"\n";

        assert!(!cargo_credentials_contain_plaintext_token(contents));
    }

    #[test]
    fn ignores_comments_and_empty_tokens() {
        let contents = "# token = \"secret\"\n[registry]\ntoken = \"\"\n";

        assert!(!cargo_credentials_contain_plaintext_token(contents));
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
    super::radioisotope::findings("rust", install_insecurity_reasons, home)
}
