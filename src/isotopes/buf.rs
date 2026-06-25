#![allow(dead_code)]

use std::path::PathBuf;

pub fn install_is_insecure() -> Result<bool, String> {
    install_insecurity_reasons().map(|reasons| !reasons.is_empty())
}

pub fn install_insecurity_reasons() -> Result<Vec<String>, String> {
    let path = netrc_path()?;
    if path.exists() && buf_netrc_token(&read_to_string(&path)?).is_some() {
        return Ok(vec![format!(
            "Buf registry token is stored in plaintext netrc: {}",
            path.display()
        )]);
    }
    Ok(Vec::new())
}

fn netrc_path() -> Result<PathBuf, String> {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())?;
    Ok(home.join(".netrc"))
}

fn read_to_string(path: &std::path::Path) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|err| format!("failed to read {}: {err}", path.display()))
}

fn buf_netrc_token(contents: &str) -> Option<String> {
    contents.lines().find_map(buf_token_from_machine_line)
}

fn buf_token_from_machine_line(line: &str) -> Option<String> {
    let fields = line.split_whitespace().collect::<Vec<_>>();
    if fields.len() < 6 || fields.first().copied()? != "machine" {
        return None;
    }
    if !matches!(fields.get(1).copied(), Some("buf.build" | "go.buf.build")) {
        return None;
    }
    fields
        .windows(2)
        .find_map(|window| (window[0] == "password" && !window[1].is_empty()).then(|| window[1]))
        .map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_buf_token() {
        assert_eq!(
            buf_netrc_token("machine buf.build login alice password bsr_secret\n").as_deref(),
            Some("bsr_secret")
        );
    }

    #[test]
    fn extracts_legacy_go_buf_token() {
        assert_eq!(
            buf_netrc_token("machine go.buf.build login alice password legacy\n").as_deref(),
            Some("legacy")
        );
    }

    #[test]
    fn ignores_other_machines() {
        assert_eq!(
            buf_netrc_token("machine example.com login alice password secret\n"),
            None
        );
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
    super::radioisotope::findings("buf", install_insecurity_reasons, home)
}
