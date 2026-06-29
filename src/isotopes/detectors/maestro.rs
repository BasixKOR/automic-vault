#![allow(dead_code)]

use std::path::PathBuf;

pub fn install_is_insecure() -> Result<bool, String> {
    install_insecurity_reasons().map(|reasons| !reasons.is_empty())
}

pub fn install_insecurity_reasons() -> Result<Vec<String>, String> {
    let mut reasons = Vec::new();
    for token_file in maestro_token_files()? {
        if token_file.path.exists() && token_file_has_secret(&read_to_string(&token_file.path)?) {
            reasons.push(format!(
                "{} is stored in plaintext token file: {}",
                token_file.label,
                token_file.path.display()
            ));
        }
    }
    Ok(reasons)
}

fn maestro_token_files() -> Result<Vec<MaestroTokenFile>, String> {
    let dir = user_home()?.join(".mobiledev");
    Ok(vec![
        MaestroTokenFile {
            path: dir.join("authtoken"),
            label: "Maestro Cloud token",
        },
        MaestroTokenFile {
            path: dir.join("openaitoken"),
            label: "Maestro Studio OpenAI token",
        },
    ])
}

struct MaestroTokenFile {
    path: PathBuf,
    label: &'static str,
}

fn user_home() -> Result<PathBuf, String> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())
}

fn read_to_string(path: &std::path::Path) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|err| format!("failed to read {}: {err}", path.display()))
}

fn token_file_has_secret(contents: &str) -> bool {
    !contents.trim().is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_non_empty_token_file() {
        assert!(token_file_has_secret("maestro-token\n"));
        assert!(token_file_has_secret("sk-test"));
    }

    #[test]
    fn ignores_empty_token_file() {
        assert!(!token_file_has_secret(""));
        assert!(!token_file_has_secret("\n  \t"));
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
    super::radioisotope::findings("maestro", install_insecurity_reasons, home)
}
