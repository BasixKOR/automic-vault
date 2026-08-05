use std::io::Write;
use std::path::Path;

use super::HardenerDetection;

const KEY: &str = "cli_auth_credentials_store";

pub(crate) fn run(stdout: &mut dyn Write) -> Result<(), String> {
    let config = crate::isotopes::detectors::codex::config_path()?;
    let auth = crate::isotopes::detectors::codex::auth_path()?;

    writeln!(stdout, "╭─ harden codex").ok();
    writeln!(stdout, "│").ok();
    writeln!(
        stdout,
        "◇ stores future Codex credentials in the macOS Keychain"
    )
    .ok();
    writeln!(stdout, "◇ ChatGPT desktop's Codex surface shares this configuration and may require sign-in again; ordinary non-Codex chats are unaffected").ok();
    writeln!(stdout, "│").ok();

    if keyring_configured(&config)? && !auth.exists() {
        writeln!(stdout, "╰─ already hardened ✔︎").ok();
        return Ok(());
    }

    writeln!(
        stdout,
        "├─ 1. close other Codex CLI, IDE, and ChatGPT desktop Codex sessions"
    )
    .ok();
    writeln!(
        stdout,
        "├─ 2. set `{KEY} = \"keyring\"` in {}",
        config.display()
    )
    .ok();
    writeln!(stdout, "├─ 3. run: `codex login`").ok();
    writeln!(stdout, "├─ 4. confirm: `codex login status`").ok();
    writeln!(
        stdout,
        "╰─ 5. only after confirmation, delete {}",
        auth.display()
    )
    .ok();
    Ok(())
}

pub(crate) fn detect() -> HardenerDetection {
    let Ok(config) = crate::isotopes::detectors::codex::config_path() else {
        return HardenerDetection::configuration(false, false, None);
    };
    let auth = crate::isotopes::detectors::codex::auth_path().ok();
    let plaintext_auth_exists = auth.as_deref().is_some_and(Path::exists);
    let applicable = config.exists() || plaintext_auth_exists;
    HardenerDetection::configuration(
        keyring_configured(&config).unwrap_or(false) && !plaintext_auth_exists,
        applicable,
        Some(config.display().to_string()),
    )
}

fn keyring_configured(path: &Path) -> Result<bool, String> {
    if !path.exists() {
        return Ok(false);
    }
    let contents = std::fs::read_to_string(path)
        .map_err(|err| format!("failed to read {}: {err}", path.display()))?;
    Ok(top_level_value(&contents, KEY) == Some("keyring"))
}

fn top_level_value<'a>(contents: &'a str, wanted: &str) -> Option<&'a str> {
    let mut found = None;
    for line in contents.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            break;
        }
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        if key.trim() == wanted {
            if found.is_some() {
                return None;
            }
            found = Some(toml_string(value.trim())?);
        }
    }
    found
}

fn toml_string(value: &str) -> Option<&str> {
    let quote = value.as_bytes().first().copied()?;
    if !matches!(quote, b'\'' | b'"') {
        return None;
    }
    let end = value[1..].find(char::from(quote))? + 1;
    value[end + 1..]
        .trim_start()
        .starts_with('#')
        .then_some(&value[1..end])
        .or_else(|| (value[end + 1..].trim().is_empty()).then_some(&value[1..end]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognizes_only_a_top_level_keyring_setting() {
        assert_eq!(
            top_level_value(
                "model = \"gpt-5.6\"\ncli_auth_credentials_store = \"keyring\" # secure\n",
                KEY
            ),
            Some("keyring")
        );
        assert_eq!(
            top_level_value("[profile]\ncli_auth_credentials_store = \"keyring\"\n", KEY),
            None
        );
        assert_eq!(
            top_level_value("cli_auth_credentials_store = \"auto\"\n", KEY),
            Some("auto")
        );
        assert_eq!(
            top_level_value(
                "cli_auth_credentials_store = \"keyring\"\ncli_auth_credentials_store = \"file\"\n",
                KEY
            ),
            None
        );
    }
}
