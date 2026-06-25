#![allow(dead_code)]

use std::path::{Path, PathBuf};

pub fn install_is_insecure() -> Result<bool, String> {
    install_insecurity_reasons().map(|reasons| !reasons.is_empty())
}

pub fn install_insecurity_reasons() -> Result<Vec<String>, String> {
    let mut reasons = Vec::new();
    for path in upload_config_paths()? {
        if path.exists() && upload_config_api_key(&read_to_string(&path)?).is_some() {
            reasons.push(format!(
                "LuaRocks upload config contains a plaintext API key: {}",
                path.display()
            ));
        }
    }
    Ok(reasons)
}

fn upload_config_paths() -> Result<Vec<PathBuf>, String> {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())?;
    let mut paths = Vec::new();

    for (key, value) in std::env::vars_os() {
        let Some(key) = key.to_str() else {
            continue;
        };
        if key == "LUAROCKS_CONFIG" || key.starts_with("LUAROCKS_CONFIG_") {
            if !value.is_empty() {
                paths.push(upload_config_path_for_user_config(Path::new(&value)));
            }
        }
    }

    if let Some(xdg_config_home) =
        std::env::var_os("XDG_CONFIG_HOME").filter(|value| !value.is_empty())
    {
        paths.push(PathBuf::from(xdg_config_home).join("luarocks/upload_config.lua"));
    } else {
        paths.push(home.join(".config/luarocks/upload_config.lua"));
    }
    paths.push(home.join(".luarocks/upload_config.lua"));

    Ok(dedupe_paths(paths))
}

fn upload_config_path_for_user_config(path: &Path) -> PathBuf {
    path.parent()
        .map(|parent| parent.join("upload_config.lua"))
        .unwrap_or_else(|| PathBuf::from("upload_config.lua"))
}

fn dedupe_paths(paths: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut deduped = Vec::new();
    for path in paths {
        if !deduped.iter().any(|existing| existing == &path) {
            deduped.push(path);
        }
    }
    deduped
}

fn read_to_string(path: &Path) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|err| format!("failed to read {}: {err}", path.display()))
}

fn upload_config_api_key(contents: &str) -> Option<String> {
    contents
        .lines()
        .find_map(|line| parse_key_assignment(line).map(|assignment| assignment.value))
}

#[derive(Debug, PartialEq, Eq)]
struct KeyAssignment {
    value_start: usize,
    value_end: usize,
    value: String,
}

fn parse_key_assignment(line: &str) -> Option<KeyAssignment> {
    let trimmed = line.trim_start();
    if trimmed.starts_with("--") {
        return None;
    }

    let equals = line.find('=')?;
    let key_side = line[..equals].trim_end();
    if !key_side_names_key(key_side) {
        return None;
    }

    let value_prefix = &line[equals + 1..];
    let whitespace = value_prefix.len() - value_prefix.trim_start().len();
    let quote_start = equals + 1 + whitespace;
    let quote = line.as_bytes().get(quote_start).copied()?;
    if quote != b'\'' && quote != b'"' {
        return None;
    }

    let mut escaped = false;
    for (offset, byte) in line[quote_start + 1..].bytes().enumerate() {
        if escaped {
            escaped = false;
            continue;
        }
        if byte == b'\\' {
            escaped = true;
            continue;
        }
        if byte == quote {
            let value_start = quote_start;
            let value_end = quote_start + 1 + offset + 1;
            let value = line[quote_start + 1..quote_start + 1 + offset].to_string();
            if value.is_empty() {
                return None;
            }
            return Some(KeyAssignment {
                value_start,
                value_end,
                value,
            });
        }
    }
    None
}

fn key_side_names_key(key_side: &str) -> bool {
    let key_side = key_side.trim_end();
    if key_side.ends_with("[\"key\"]") || key_side.ends_with("['key']") {
        return true;
    }
    if !key_side.ends_with("key") {
        return false;
    }
    key_side
        .chars()
        .rev()
        .nth(3)
        .map(|previous| !matches!(previous, '_' | '-' | 'a'..='z' | 'A'..='Z' | '0'..='9'))
        .unwrap_or(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_luarocks_upload_key() {
        assert_eq!(
            upload_config_api_key("return {\n   key = \"lr_secret\",\n}\n"),
            Some("lr_secret".to_string())
        );
    }

    #[test]
    fn ignores_comments_empty_values_and_other_keys() {
        assert_eq!(
            upload_config_api_key(
                "-- key = \"secret\"\nserver = \"https://luarocks.org\"\nkey = \"\"\n"
            ),
            None
        );
    }

    #[test]
    fn parses_inline_key_assignment() {
        let assignment = parse_key_assignment("return { key = 'secret', server = 'x' }").unwrap();

        assert_eq!(assignment.value, "secret");
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
    super::radioisotope::findings("luarocks", install_insecurity_reasons, home)
}
