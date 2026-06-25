#![allow(dead_code)]

use std::path::PathBuf;

pub fn install_is_insecure() -> Result<bool, String> {
    install_insecurity_reasons().map(|reasons| !reasons.is_empty())
}

pub fn install_insecurity_reasons() -> Result<Vec<String>, String> {
    let path = mysql_defaults_path()?;
    if path.exists() && mysql_defaults_contains_password(&read_to_string(&path)?) {
        return Ok(vec![format!(
            "MySQL option file contains plaintext passwords: {}",
            path.display()
        )]);
    }
    Ok(Vec::new())
}

fn mysql_defaults_path() -> Result<PathBuf, String> {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())?;
    Ok(home.join(".my.cnf"))
}

fn read_to_string(path: &std::path::Path) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|err| format!("failed to read {}: {err}", path.display()))
}

fn mysql_defaults_contains_password(contents: &str) -> bool {
    contents.lines().any(line_has_password_value)
}

fn line_has_password_value(line: &str) -> bool {
    let line = line.trim();
    if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
        return false;
    }
    let Some((key, value)) = line.split_once('=') else {
        return false;
    };
    key.trim().eq_ignore_ascii_case("password") && !value.trim().is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_password_option() {
        assert!(mysql_defaults_contains_password(
            "[client]\nuser = deploy\npassword = secret\n"
        ));
    }

    #[test]
    fn ignores_password_prompt_flag() {
        assert!(!mysql_defaults_contains_password("[client]\npassword\n"));
    }

    #[test]
    fn ignores_comments() {
        assert!(!mysql_defaults_contains_password("# password = secret\n"));
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
    super::radioisotope::findings("mysql@8.4", install_insecurity_reasons, home)
}
