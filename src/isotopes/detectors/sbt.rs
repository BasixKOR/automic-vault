#![allow(dead_code)]

use std::path::PathBuf;

pub fn install_is_insecure() -> Result<bool, String> {
    install_insecurity_reasons().map(|reasons| !reasons.is_empty())
}

pub fn install_insecurity_reasons() -> Result<Vec<String>, String> {
    let path = sbt_credentials_path()?;
    if path.exists() && sbt_credentials_contains_password(&read_to_string(&path)?) {
        return Ok(vec![format!(
            "sbt credentials file contains plaintext passwords: {}",
            path.display()
        )]);
    }
    Ok(Vec::new())
}

fn sbt_credentials_path() -> Result<PathBuf, String> {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())?;
    Ok(home.join(".sbt/.credentials"))
}

fn read_to_string(path: &std::path::Path) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|err| format!("failed to read {}: {err}", path.display()))
}

fn sbt_credentials_contains_password(contents: &str) -> bool {
    contents.lines().any(line_has_password_value)
}

fn line_has_password_value(line: &str) -> bool {
    let line = line.trim();
    if line.is_empty() || line.starts_with('#') || line.starts_with('!') {
        return false;
    }
    let Some((key, value)) = line.split_once('=') else {
        return false;
    };
    matches!(
        key.trim().to_ascii_lowercase().as_str(),
        "password" | "passwd" | "pass" | "pwd"
    ) && !value.trim().is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_password_property() {
        assert!(sbt_credentials_contains_password(
            "realm=Repo\nhost=repo.example.com\nuser=me\npassword=secret\n"
        ));
    }

    #[test]
    fn ignores_comments_and_empty_passwords() {
        assert!(!sbt_credentials_contains_password(
            "# password=secret\npassword=\n"
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
    super::radioisotope::findings("sbt", install_insecurity_reasons, home)
}
