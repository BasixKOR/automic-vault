#![allow(dead_code)]

use std::path::PathBuf;

pub fn install_is_insecure() -> Result<bool, String> {
    install_insecurity_reasons().map(|reasons| !reasons.is_empty())
}

pub fn install_insecurity_reasons() -> Result<Vec<String>, String> {
    let mut reasons = Vec::new();
    for config in railway_configs()? {
        if config.path.exists() && config_has_secrets(&read_to_string(&config.path)?) {
            reasons.push(format!(
                "Railway CLI auth state is stored in plaintext config: {}",
                config.path.display()
            ));
        }
    }
    Ok(reasons)
}

fn railway_configs() -> Result<Vec<RailwayConfigFile>, String> {
    let railway_dir = user_home()?.join(".railway");
    Ok(vec![
        RailwayConfigFile {
            path: railway_dir.join("config.json"),
        },
        RailwayConfigFile {
            path: railway_dir.join("config-staging.json"),
        },
        RailwayConfigFile {
            path: railway_dir.join("config-dev.json"),
        },
    ])
}

struct RailwayConfigFile {
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
    const KEYS: [&str; 5] = [
        "token",
        "accessToken",
        "refreshToken",
        "access_token",
        "refresh_token",
    ];
    let Ok(config) = serde_json::from_str::<serde_json::Value>(contents) else {
        return KEYS
            .iter()
            .any(|key| contents.contains(&format!(r#""{key}""#)));
    };
    config
        .get("user")
        .and_then(serde_json::Value::as_object)
        .is_some_and(|user| {
            KEYS.iter().any(|key| {
                user.get(*key)
                    .and_then(serde_json::Value::as_str)
                    .is_some_and(|value| !value.is_empty() && value != "@av")
            })
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_legacy_and_oauth_tokens() {
        assert!(config_has_secrets(r#"{"user":{"token":"rw_legacy"}}"#));
        assert!(config_has_secrets(
            r#"{"user":{"accessToken":"access","refreshToken":"refresh"}}"#
        ));
        assert!(config_has_secrets(
            r#"{"user":{"access_token":"access","refresh_token":"refresh"}}"#
        ));
    }

    #[test]
    fn ignores_linked_project_config_without_auth() {
        assert!(!config_has_secrets(
            r#"{"projects":{"/tmp/app":{"project":"p","environment":"e"}},"user":{}}"#
        ));
    }

    #[test]
    fn ignores_isotope_markers_and_metadata() {
        assert!(!config_has_secrets(
            r#"{"user":{"accessToken":"@av","refreshToken":"@av","tokenExpiresAt":42}}"#
        ));
        assert!(!config_has_secrets(r#"{"token":"project-id"}"#));
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
    super::radioisotope::findings("railway", install_insecurity_reasons, home)
}
