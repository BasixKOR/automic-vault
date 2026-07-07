#![allow(dead_code)]

use std::path::PathBuf;

pub fn install_is_insecure() -> Result<bool, String> {
    install_insecurity_reasons().map(|reasons| !reasons.is_empty())
}

pub fn install_insecurity_reasons() -> Result<Vec<String>, String> {
    let path = netrc_path()?;
    if path.exists() && !heroku_netrc_tokens(&read_to_string(&path)?).is_empty() {
        return Ok(vec![format!(
            "Heroku API token is stored in plaintext netrc: {}",
            path.display()
        )]);
    }
    Ok(Vec::new())
}

fn netrc_path() -> Result<PathBuf, String> {
    if let Some(path) = std::env::var_os("NETRC").map(PathBuf::from) {
        return Ok(path);
    }
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())?;
    Ok(home.join(".netrc"))
}

fn read_to_string(path: &std::path::Path) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|err| format!("failed to read {}: {err}", path.display()))
}

fn heroku_netrc_tokens(contents: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut current_machine: Option<&str> = None;
    let tokens = netrc_tokens(contents);
    let mut index = 0;
    while index < tokens.len() {
        match tokens[index].as_str() {
            "machine" => {
                current_machine = tokens.get(index + 1).map(String::as_str);
                index += 2;
            }
            "default" => {
                current_machine = Some("default");
                index += 1;
            }
            "password" => {
                if matches!(current_machine, Some("api.heroku.com" | "git.heroku.com")) {
                    if let Some(token) = tokens.get(index + 1) {
                        found.push(token.clone());
                    }
                }
                index += 2;
            }
            _ => index += 1,
        }
    }
    found
}

fn netrc_tokens(contents: &str) -> Vec<String> {
    contents
        .lines()
        .flat_map(|line| line.split('#').next().unwrap_or("").split_whitespace())
        .map(str::to_string)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_single_line_heroku_tokens() {
        assert_eq!(
            heroku_netrc_tokens("machine api.heroku.com login user password hk_secret"),
            vec!["hk_secret".to_string()]
        );
    }

    #[test]
    fn extracts_multiline_heroku_tokens() {
        assert_eq!(
            heroku_netrc_tokens("machine git.heroku.com\n  login user\n  password hk_secret\n"),
            vec!["hk_secret".to_string()]
        );
    }

    #[test]
    fn ignores_other_machines() {
        assert!(heroku_netrc_tokens("machine example.com login user password secret").is_empty());
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
    super::radioisotope::findings("heroku", install_insecurity_reasons, home)
}
