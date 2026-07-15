mod cli;
mod isotopes;
mod secrets;

pub use cli::{run, run_terminal};

pub const MENU_HELPER_CODE_SIGNING_REQUIREMENT: &str = r#"anchor apple generic and certificate leaf[subject.OU] = ZU76A67LGU and identifier "com.automicvault""#;

pub(crate) fn bash_shell_secret_insecurity_reasons() -> Result<Vec<String>, String> {
    cli::bash_shell_secret_insecurity_reasons()
}

pub(crate) fn zsh_shell_secret_insecurity_reasons() -> Result<Vec<String>, String> {
    cli::zsh_shell_secret_insecurity_reasons()
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
    line: Option<usize>,
}
