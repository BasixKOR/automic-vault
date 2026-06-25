#![allow(dead_code)]

use std::path::PathBuf;

pub fn install_is_insecure() -> Result<bool, String> {
    install_insecurity_reasons().map(|reasons| !reasons.is_empty())
}

pub fn install_insecurity_reasons() -> Result<Vec<String>, String> {
    let auth_file = containers_auth_file()?;
    if auth_file.exists() && auth_file_has_secrets(&read_to_string(&auth_file)?) {
        return Ok(vec![format!(
            "skopeo registry credentials are stored in plaintext auth file: {}",
            auth_file.display()
        )]);
    }
    Ok(Vec::new())
}

fn containers_auth_file() -> Result<PathBuf, String> {
    Ok(user_home()?.join(".config/containers/auth.json"))
}

fn user_home() -> Result<PathBuf, String> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())
}

fn read_to_string(path: &std::path::Path) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|err| format!("failed to read {}: {err}", path.display()))
}

fn auth_file_has_secrets(contents: &str) -> bool {
    (contents.contains("\"auths\"") && contents.contains("\"auth\""))
        || contents.contains("\"identitytoken\"")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_registry_auth_entries() {
        assert!(auth_file_has_secrets(
            r#"{"auths":{"registry.example":{"auth":"dXNlcjpwYXNz"}}}"#
        ));
        assert!(auth_file_has_secrets(
            r#"{"auths":{"registry.example":{"identitytoken":"token"}}}"#
        ));
    }

    #[test]
    fn ignores_auth_file_without_credentials() {
        assert!(!auth_file_has_secrets(r#"{"auths":{}}"#));
        assert!(!auth_file_has_secrets(
            r#"{"credHelpers":{"registry.example":"osxkeychain"}}"#
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
    super::radioisotope::findings("skopeo", install_insecurity_reasons, home)
}
