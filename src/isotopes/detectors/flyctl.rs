#![allow(dead_code)]

use std::path::PathBuf;

pub fn install_is_insecure() -> Result<bool, String> {
    install_insecurity_reasons().map(|reasons| !reasons.is_empty())
}

pub fn install_insecurity_reasons() -> Result<Vec<String>, String> {
    let path = fly_config_path()?;
    if path.exists() && fly_config_access_token(&read_to_string(&path)?).is_some() {
        return Ok(vec![format!(
            "flyctl config file contains a plaintext access token: {}",
            path.display()
        )]);
    }
    Ok(Vec::new())
}

fn fly_config_path() -> Result<PathBuf, String> {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())?;
    Ok(home.join(".fly/config.yml"))
}

fn read_to_string(path: &std::path::Path) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|err| format!("failed to read {}: {err}", path.display()))
}

fn fly_config_access_token(contents: &str) -> Option<String> {
    contents.lines().find_map(access_token_from_line)
}

fn access_token_from_line(line: &str) -> Option<String> {
    let line = line.trim_start();
    let value = line.strip_prefix("access_token:")?.trim();
    non_empty_yaml_scalar(value)
}

fn non_empty_yaml_scalar(value: &str) -> Option<String> {
    let value = value.trim_matches('"').trim_matches('\'');
    (!value.is_empty()).then(|| value.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_access_token() {
        assert_eq!(
            fly_config_access_token("access_token: FlyV1 secret\n").as_deref(),
            Some("FlyV1 secret")
        );
    }

    #[test]
    fn ignores_empty_access_token() {
        assert_eq!(fly_config_access_token("access_token: ''\n"), None);
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
    super::radioisotope::findings("flyctl", install_insecurity_reasons, home)
}
