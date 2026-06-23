use std::{env, fs, io::Write, process::{Command, Stdio}};

pub fn install_is_insecure() -> Result<bool, String> {
    Ok(!install_insecurity_reasons()?.is_empty())
}

pub fn install_insecurity_reasons() -> Result<Vec<String>, String> {
    let mut reasons = Vec::new();
    let home = env::var_os("HOME").unwrap_or_default();
    if fs::read_to_string(std::path::PathBuf::from(&home).join(".git-credentials"))
        .unwrap_or_default()
        .contains("://")
    {
        reasons.push("Git credential store contains plaintext credentials".to_string());
    }
    if env::var_os("AUTOMIC_VAULT_TEST_GIT_CREDENTIAL_FILL_DETECTOR").is_some() {
        let mut child = Command::new("git")
            .args(["credential", "fill"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .map_err(|err| err.to_string())?;
        child.stdin.as_mut().unwrap().write_all(b"protocol=https\nhost=github.com\n\n").map_err(|err| err.to_string())?;
        let output = child.wait_with_output().map_err(|err| err.to_string())?;
        if String::from_utf8_lossy(&output.stdout).contains("password=") {
            reasons.push("git credential fill returned plaintext credentials. Click Learn More.".to_string());
        }
    }
    Ok(reasons)
}
