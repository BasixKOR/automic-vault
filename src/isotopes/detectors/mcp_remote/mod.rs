#![allow(dead_code)]

use std::path::{Path, PathBuf};

pub fn install_is_insecure() -> Result<bool, String> {
    install_insecurity_reasons().map(|reasons| !reasons.is_empty())
}

pub fn install_insecurity_reasons() -> Result<Vec<String>, String> {
    let auth_dir = mcp_remote_auth_dir()?;
    if !auth_dir.exists() {
        return Ok(Vec::new());
    }

    let mut reasons = Vec::new();
    for path in credential_files(&auth_dir)? {
        reasons.push(format!(
            "mcp-remote auth file contains plaintext OAuth credentials: {}",
            path.display()
        ));
    }
    Ok(reasons)
}

fn mcp_remote_auth_dir() -> Result<PathBuf, String> {
    if let Some(path) = std::env::var_os("MCP_REMOTE_CONFIG_DIR").filter(|value| !value.is_empty())
    {
        return Ok(PathBuf::from(path));
    }
    Ok(user_home()?.join(".mcp-auth"))
}

fn user_home() -> Result<PathBuf, String> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())
}

fn credential_files(root: &Path) -> Result<Vec<PathBuf>, String> {
    let mut files = Vec::new();
    collect_credential_files(root, root, &mut files)?;
    Ok(files)
}

fn collect_credential_files(
    root: &Path,
    path: &Path,
    files: &mut Vec<PathBuf>,
) -> Result<(), String> {
    for entry in std::fs::read_dir(path)
        .map_err(|err| format!("failed to read {}: {err}", path.display()))?
    {
        let entry =
            entry.map_err(|err| format!("failed to read {} entry: {err}", path.display()))?;
        let path = entry.path();
        let metadata = entry
            .metadata()
            .map_err(|err| format!("failed to stat {}: {err}", path.display()))?;
        if metadata.is_dir() {
            collect_credential_files(root, &path, files)?;
        } else if metadata.is_file() && auth_file_contains_credentials(&path)? {
            files.push(path.strip_prefix(root).unwrap_or(&path).to_path_buf());
        }
    }
    Ok(())
}

fn auth_file_contains_credentials(path: &Path) -> Result<bool, String> {
    let contents = std::fs::read_to_string(path)
        .map_err(|err| format!("failed to read {}: {err}", path.display()))?;
    Ok(contains_json_secret_key(&contents))
}

fn contains_json_secret_key(contents: &str) -> bool {
    [
        "\"access_token\"",
        "\"refresh_token\"",
        "\"id_token\"",
        "\"client_secret\"",
    ]
    .iter()
    .any(|needle| contents.contains(needle))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_oauth_token_json() {
        assert!(contains_json_secret_key(
            r#"{"access_token":"fake-access","refresh_token":"fake-refresh"}"#
        ));
        assert!(contains_json_secret_key(
            r#"{"client_secret":"fake-client-secret"}"#
        ));
        assert!(!contains_json_secret_key(
            r#"{"issuer":"https://example.test"}"#
        ));
    }

    #[test]
    fn finds_nested_credential_files() {
        let temp = test_dir("mcp-remote-detect");
        let nested = temp.join("mcp-remote-0.1.38");
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::write(
            nested.join("server_tokens.json"),
            r#"{"access_token":"fake"}"#,
        )
        .unwrap();
        std::fs::write(
            nested.join("metadata.json"),
            r#"{"issuer":"https://example.test"}"#,
        )
        .unwrap();

        assert_eq!(
            credential_files(&temp).unwrap(),
            vec![PathBuf::from("mcp-remote-0.1.38/server_tokens.json")]
        );
        std::fs::remove_dir_all(temp).unwrap();
    }

    #[test]
    fn top_level_install_is_insecure_returns_false_when_default_dir_is_missing() {
        let home = test_dir("mcp-remote-missing");
        let previous_home = std::env::var_os("HOME");
        let previous_config = std::env::var_os("MCP_REMOTE_CONFIG_DIR");
        unsafe {
            std::env::set_var("HOME", &home);
            std::env::remove_var("MCP_REMOTE_CONFIG_DIR");
        }

        let result = install_is_insecure().unwrap();

        unsafe {
            match previous_home {
                Some(value) => std::env::set_var("HOME", value),
                None => std::env::remove_var("HOME"),
            }
            match previous_config {
                Some(value) => std::env::set_var("MCP_REMOTE_CONFIG_DIR", value),
                None => std::env::remove_var("MCP_REMOTE_CONFIG_DIR"),
            }
        }
        assert!(!result);
        std::fs::remove_dir_all(home).unwrap();
    }

    fn test_dir(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!("{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).unwrap();
        path
    }
}

pub(crate) fn findings(home: &std::path::Path) -> Vec<crate::Finding> {
    super::radioisotope::findings("mcp-remote", install_insecurity_reasons, home)
}
