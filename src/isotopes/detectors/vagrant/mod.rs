#![allow(dead_code)]

use std::path::PathBuf;

pub fn install_is_insecure() -> Result<bool, String> {
    install_insecurity_reasons().map(|reasons| !reasons.is_empty())
}

pub fn install_insecurity_reasons() -> Result<Vec<String>, String> {
    let path = vagrant_token_path()?;
    if path.exists() && token_file_contains_token(&read_to_string(&path)?) {
        return Ok(vec![format!(
            "Vagrant Cloud token file contains a plaintext token: {}",
            path.display()
        )]);
    }
    Ok(Vec::new())
}

fn vagrant_token_path() -> Result<PathBuf, String> {
    Ok(vagrant_home_path()?
        .join("data")
        .join("vagrant_login_token"))
}

fn vagrant_home_path() -> Result<PathBuf, String> {
    if let Some(path) = std::env::var_os("VAGRANT_HOME") {
        return Ok(PathBuf::from(path));
    }

    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())?;
    Ok(home.join(".vagrant.d"))
}

fn read_to_string(path: &std::path::Path) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|err| format!("failed to read {}: {err}", path.display()))
}

fn token_file_contains_token(contents: &str) -> bool {
    !contents.trim().is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_token() {
        assert!(token_file_contains_token("vagrant-cloud-token\n"));
    }

    #[test]
    fn ignores_empty_file() {
        assert!(!token_file_contains_token("\n"));
    }

    #[test]
    fn top_level_install_is_insecure_returns_false_when_default_locations_are_missing() {
        let home = std::env::temp_dir().join(format!(
            "{}-detect-missing-{}",
            module_path!().replace(':', "_"),
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(&home).unwrap();

        let previous_home = std::env::var_os("HOME");
        let previous_vagrant_home = std::env::var_os("VAGRANT_HOME");
        unsafe {
            std::env::set_var("HOME", &home);
            std::env::remove_var("VAGRANT_HOME");
        }

        let result = install_is_insecure().unwrap();

        unsafe {
            match previous_home {
                Some(value) => std::env::set_var("HOME", value),
                None => std::env::remove_var("HOME"),
            }
            match previous_vagrant_home {
                Some(value) => std::env::set_var("VAGRANT_HOME", value),
                None => std::env::remove_var("VAGRANT_HOME"),
            }
        }

        assert!(!result);
        std::fs::remove_dir_all(home).unwrap();
    }

    #[test]
    fn uses_vagrant_home_when_set() {
        let previous_vagrant_home = std::env::var_os("VAGRANT_HOME");
        unsafe { std::env::set_var("VAGRANT_HOME", "/tmp/custom-vagrant-home") };

        assert_eq!(
            vagrant_token_path().unwrap(),
            PathBuf::from("/tmp/custom-vagrant-home/data/vagrant_login_token")
        );

        unsafe {
            match previous_vagrant_home {
                Some(value) => std::env::set_var("VAGRANT_HOME", value),
                None => std::env::remove_var("VAGRANT_HOME"),
            }
        }
    }
}

pub(crate) fn findings(home: &std::path::Path) -> Vec<crate::Finding> {
    super::radioisotope::findings("vagrant", install_insecurity_reasons, home)
}
