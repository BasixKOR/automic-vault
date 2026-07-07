#![allow(dead_code)]

use std::path::PathBuf;

pub fn install_is_insecure() -> Result<bool, String> {
    install_insecurity_reasons().map(|reasons| !reasons.is_empty())
}

pub fn install_insecurity_reasons() -> Result<Vec<String>, String> {
    let path = qwen_settings_path()?;
    if path.exists() && qwen_settings_contains_env_secret(&read_to_string(&path)?) {
        return Ok(vec![format!(
            "Qwen Code settings contain plaintext API keys: {}",
            path.display()
        )]);
    }
    Ok(Vec::new())
}

fn qwen_settings_path() -> Result<PathBuf, String> {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())?;
    Ok(home.join(".qwen/settings.json"))
}

fn read_to_string(path: &std::path::Path) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|err| format!("failed to read {}: {err}", path.display()))
}

fn qwen_settings_contains_env_secret(contents: &str) -> bool {
    json_object_field(contents, "env").is_some_and(json_object_contains_nonempty_string_value)
}

fn json_object_field<'a>(contents: &'a str, field: &str) -> Option<&'a str> {
    let quoted = format!("\"{field}\"");
    let key_start = contents.find(&quoted)?;
    let after_key = &contents[key_start + quoted.len()..];
    let after_colon = after_key.split_once(':')?.1.trim_start();
    let object_start = after_colon.find('{')?;
    let object = &after_colon[object_start..];
    let mut in_string = false;
    let mut escaped = false;
    let mut depth = 0usize;

    for (index, byte) in object.bytes().enumerate() {
        if in_string {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                in_string = false;
            }
            continue;
        }

        match byte {
            b'"' => in_string = true,
            b'{' => depth += 1,
            b'}' => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some(&object[..=index]);
                }
            }
            _ => {}
        }
    }

    None
}

fn json_object_contains_nonempty_string_value(object: &str) -> bool {
    let mut rest = object;
    while let Some((_, after_colon)) = rest.split_once(':') {
        let value = after_colon.trim_start();
        if let Some(after_quote) = value.strip_prefix('"') {
            if let Some((string_value, after_string)) = after_quote.split_once('"') {
                if !string_value.is_empty() {
                    return true;
                }
                rest = after_string;
                continue;
            }
            return false;
        }
        rest = after_colon.get(1..).unwrap_or_default();
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_api_key_in_env_settings() {
        assert!(qwen_settings_contains_env_secret(
            r#"{"env":{"DASHSCOPE_API_KEY":"sk-test"}}"#
        ));
    }

    #[test]
    fn ignores_env_key_declarations_without_env_values() {
        assert!(!qwen_settings_contains_env_secret(
            r#"{"modelProviders":{"openai":[{"envKey":"DASHSCOPE_API_KEY"}]}}"#
        ));
    }

    #[test]
    fn ignores_empty_env_values() {
        assert!(!qwen_settings_contains_env_secret(
            r#"{"env":{"DASHSCOPE_API_KEY":""}}"#
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
    super::radioisotope::findings("qwen-code", install_insecurity_reasons, home)
}
