#![allow(dead_code)]

use std::path::PathBuf;

pub fn install_is_insecure() -> Result<bool, String> {
    install_insecurity_reasons().map(|reasons| !reasons.is_empty())
}

pub fn install_insecurity_reasons() -> Result<Vec<String>, String> {
    let mut reasons = Vec::new();
    for config in gallery_dl_configs()? {
        if config.path.exists() && config_has_secrets(&read_to_string(&config.path)?) {
            reasons.push(format!(
                "gallery-dl config contains credentials: {}",
                config.path.display()
            ));
        }
    }
    Ok(reasons)
}

fn gallery_dl_configs() -> Result<Vec<ConfigFile>, String> {
    let home = user_home()?;
    let config_home = if let Some(path) = std::env::var_os("XDG_CONFIG_HOME") {
        PathBuf::from(path)
    } else {
        home.join(".config")
    };
    Ok(vec![
        ConfigFile {
            path: config_home.join("gallery-dl/config.json"),
        },
        ConfigFile {
            path: home.join(".gallery-dl.conf"),
        },
    ])
}

struct ConfigFile {
    path: PathBuf,
}

fn user_home() -> Result<PathBuf, String> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())
}

fn read_to_string(path: &std::path::Path) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|err| format!("failed to read {}: {err}", path.display()))
}

fn config_has_secrets(contents: &str) -> bool {
    for key in [
        "password",
        "refresh-token",
        "access-token",
        "access-token-secret",
        "client-secret",
        "api-secret",
        "api-key",
        "jwt",
        "csrf",
        "cookies",
    ] {
        if jsonish_key_has_nonempty_value(contents, key) {
            return true;
        }
    }
    false
}

fn jsonish_key_has_nonempty_value(contents: &str, key: &str) -> bool {
    for quote in ['"', '\''] {
        let quoted_key = format!("{quote}{key}{quote}");
        let mut rest = contents;
        while let Some(index) = rest.find(&quoted_key) {
            let after_key = &rest[index + quoted_key.len()..];
            let Some(colon_index) = after_key.find(':') else {
                return false;
            };
            let value = after_key[colon_index + 1..].trim_start();
            if value.starts_with("null")
                || value.starts_with("\"\"")
                || value.starts_with("''")
                || value.starts_with("{}")
                || value.starts_with("[]")
            {
                rest = &after_key[colon_index + 1..];
                continue;
            }
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_passwords() {
        assert!(config_has_secrets(
            r#"{"extractor":{"sankaku":{"username":"me","password":"secret"}}}"#
        ));
    }

    #[test]
    fn detects_oauth_tokens() {
        assert!(config_has_secrets(
            r#"{"extractor":{"deviantart":{"refresh-token":"token"}}}"#
        ));
        assert!(config_has_secrets(
            r#"{"extractor":{"twitter":{"access-token-secret":"secret"}}}"#
        ));
    }

    #[test]
    fn detects_cookie_objects() {
        assert!(config_has_secrets(
            r#"{"extractor":{"twitter":{"cookies":{"auth_token":"secret"}}}}"#
        ));
    }

    #[test]
    fn ignores_empty_secret_values() {
        assert!(!config_has_secrets(
            r#"{"extractor":{"twitter":{"cookies":{},"password":null,"refresh-token":""}}}"#
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
    super::radioisotope::findings("gallery-dl", install_insecurity_reasons, home)
}
