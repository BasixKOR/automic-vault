mod cli;
mod credential_helper;
mod harden;
mod inject;
mod isotopes;
mod scan;
mod shell_secrets;
mod stub;

pub use cli::{run, run_terminal};

pub(crate) fn bash_shell_secret_insecurity_reasons() -> Result<Vec<String>, String> {
    shell_secrets::bash_reasons()
}

pub(crate) fn zsh_shell_secret_insecurity_reasons() -> Result<Vec<String>, String> {
    shell_secrets::zsh_reasons()
}

#[cfg(test)]
pub fn global_test_env_lock() -> &'static std::sync::Mutex<()> {
    &cli::ENV_LOCK
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Finding {
    source: &'static str,
    homepage: &'static str,
    severity: &'static str,
    explanation: String,
    solution: String,
    affected: Vec<AffectedFile>,
    docs_url: &'static str,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct AffectedFile {
    path: String,
    line: usize,
}
