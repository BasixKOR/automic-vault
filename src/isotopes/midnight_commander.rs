#![allow(dead_code)]

use std::path::PathBuf;

pub fn install_is_insecure() -> Result<bool, String> {
    install_insecurity_reasons().map(|reasons| !reasons.is_empty())
}

pub fn install_insecurity_reasons() -> Result<Vec<String>, String> {
    let mut reasons = Vec::new();
    for profile_file in mc_profile_files()? {
        if profile_file.path.exists() && profile_has_secrets(&read_to_string(&profile_file.path)?) {
            reasons.push(format!(
                "Midnight Commander profile file contains VFS credentials: {}",
                profile_file.path.display()
            ));
        }
    }
    Ok(reasons)
}

fn mc_profile_files() -> Result<Vec<ProfileFile>, String> {
    let config_dir = if let Some(path) = std::env::var_os("XDG_CONFIG_HOME") {
        PathBuf::from(path).join("mc")
    } else {
        user_home()?.join(".config/mc")
    };
    Ok(PROFILE_FILES
        .iter()
        .map(|file_name| ProfileFile {
            path: config_dir.join(file_name),
        })
        .collect())
}

struct ProfileFile {
    path: PathBuf,
}

const PROFILE_FILES: &[&str] = &["ini", "hotlist", "panels.ini"];

fn user_home() -> Result<PathBuf, String> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())
}

fn read_to_string(path: &std::path::Path) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|err| format!("failed to read {}: {err}", path.display()))
}

fn profile_has_secrets(contents: &str) -> bool {
    contents.lines().any(line_has_password_setting) || contains_url_password(contents)
}

fn line_has_password_setting(line: &str) -> bool {
    let trimmed = line.trim();
    let Some((key, value)) = trimmed.split_once('=') else {
        return false;
    };
    matches!(key.trim(), "ftpfs_password" | "password")
        && !value.trim().is_empty()
        && value.trim() != "<hidden>"
}

fn contains_url_password(contents: &str) -> bool {
    for scheme in ["://", ":"] {
        let mut rest = contents;
        while let Some(index) = rest.find(scheme) {
            let after_scheme = &rest[index + scheme.len()..];
            let authority_end = after_scheme
                .find(|ch| matches!(ch, '/' | '"' | '\'' | '\n' | '\r' | ' ' | '\t'))
                .unwrap_or(after_scheme.len());
            let authority = &after_scheme[..authority_end];
            if let Some(at_index) = authority.rfind('@') {
                let userinfo = &authority[..at_index];
                if userinfo.contains(':') && !userinfo.ends_with(':') {
                    return true;
                }
            }
            rest = &after_scheme[authority_end..];
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_ftpfs_password_setting() {
        assert!(profile_has_secrets(
            "[Misc]\nftpfs_password=user@example.com\n"
        ));
    }

    #[test]
    fn detects_serialized_vfs_password() {
        assert!(profile_has_secrets(
            "[path-element-0]\nclass-name=ftpfs\nuser=me\npassword=secret\nhost=example.com\n"
        ));
    }

    #[test]
    fn detects_hotlist_url_password() {
        assert!(profile_has_secrets(
            "ENTRY \"remote\" URL \"/ftp://me:secret@example.com/pub\"\n"
        ));
    }

    #[test]
    fn ignores_non_secret_hotlist_entry() {
        assert!(!profile_has_secrets(
            "ENTRY \"remote\" URL \"/ftp://me@example.com/pub\"\n"
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
    super::radioisotope::findings("midnight-commander", install_insecurity_reasons, home)
}
