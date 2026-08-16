pub(crate) mod aws_cli;
pub(crate) mod aws_release;
pub(crate) mod codex;
pub(crate) mod docker;
pub(crate) mod env_wrapper;
pub(crate) mod gh_cli;
pub(crate) mod homebrew;
pub(crate) mod isotope;
mod migrations;
pub(crate) mod stripe_cli;
pub(crate) mod sudo;
pub(crate) mod supabase;

unsafe extern "C" {
    fn geteuid() -> u32;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PrivilegeMode {
    RootOnly,
    Mixed,
    UserOnly,
}

impl PrivilegeMode {
    pub(crate) fn require_user(self, hardener: &str, test_override: bool) -> Result<(), String> {
        if effective_uid() != 0 || test_override {
            return Ok(());
        }
        match self {
            Self::Mixed => Err(format!(
                "run `av harden {hardener}` without sudo; av will request elevation when needed"
            )),
            Self::UserOnly => Err(format!("`av harden {hardener}` cannot be run as root")),
            Self::RootOnly => Ok(()),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RootOnlyOutcome {
    Previewed,
    Hardened,
}

pub(crate) fn effective_uid() -> u32 {
    crate::test_env_string("AUTOMIC_VAULT_TEST_EUID")
        .and_then(|value| value.parse().ok())
        .unwrap_or_else(|| unsafe { geteuid() })
}

pub(crate) struct HardenerMetadata {
    pub(crate) name: &'static str,
    pub(crate) documentation: &'static str,
    pub(crate) detection: HardenerDetection,
    pub(crate) secret_gate: Option<SecretGateDescriptor>,
}

pub(crate) struct SecretGateDescriptor {
    pub(crate) id: &'static str,
    pub(crate) key_patterns: Vec<String>,
    pub(crate) routes: Vec<SecretGateRoute>,
}

pub(crate) struct SecretGateRoute {
    pub(crate) operation: &'static str,
    pub(crate) script_path: Option<String>,
    pub(crate) target_path: String,
    pub(crate) caller_identifiers: Vec<&'static str>,
    pub(crate) key_patterns: Vec<String>,
    pub(crate) replace_existing_env: bool,
    pub(crate) allow_missing_keys: bool,
}

pub(crate) struct HardenerDetection {
    pub(crate) hardened: bool,
    pub(crate) applicable: bool,
    pub(crate) stub_path: Option<String>,
    pub(crate) target_path: Option<String>,
    pub(crate) commands: Vec<HardenerCommand>,
    pub(crate) diagnostics: Vec<HardenerDiagnostic>,
}

pub(crate) struct HardenerDiagnostic {
    pub(crate) kind: &'static str,
    pub(crate) message: String,
    pub(crate) remediation: String,
    pub(crate) path: Option<String>,
}

#[derive(Clone)]
pub(crate) struct HardenerCommand {
    pub(crate) name: String,
    pub(crate) hardened: bool,
    pub(crate) stub_valid: bool,
    pub(crate) stub_path: Option<String>,
    pub(crate) target_path: String,
    pub(crate) required_paths: Vec<RequiredExecutable>,
    pub(crate) stub_requirements: Option<StubRequirements>,
    pub(crate) injected_keys: Vec<String>,
    pub(crate) assignment_keys: Vec<String>,
    pub(crate) isotope: Option<isotope::Doctor>,
}

#[derive(Clone)]
pub(crate) struct RequiredExecutable {
    pub(crate) name: &'static str,
    pub(crate) path: String,
}

#[derive(Clone)]
pub(crate) struct StubRequirements {
    pub(crate) mode: u32,
    pub(crate) owner: RequiredIdentity,
    pub(crate) group: RequiredIdentity,
}

#[derive(Clone)]
pub(crate) struct RequiredIdentity {
    pub(crate) name: &'static str,
    pub(crate) id: Option<u32>,
}

pub(crate) fn write_secret_gate_notice(stdout: &mut dyn std::io::Write, gate_id: &str) {
    let protection = if gate_id == "brew" {
        "Read & Update"
    } else {
        "Read Only"
    };
    writeln!(
        stdout,
        "\n◇ `{gate_id}` defaults to {protection}, adjust this in the app: `av open --secret-gate {gate_id}`"
    )
    .ok();
}

impl HardenerDetection {
    pub(crate) fn command(
        hardened: bool,
        name: impl Into<String>,
        stub_path: Option<String>,
        target_path: String,
    ) -> Self {
        let applicable = stub_path
            .as_deref()
            .is_some_and(|path| Path::new(path).exists())
            || Path::new(&target_path).exists();
        Self {
            hardened,
            applicable,
            stub_path: stub_path.clone(),
            target_path: Some(target_path.clone()),
            commands: vec![HardenerCommand {
                name: name.into(),
                hardened,
                stub_valid: hardened,
                stub_path,
                target_path,
                required_paths: Vec::new(),
                stub_requirements: None,
                injected_keys: Vec::new(),
                assignment_keys: Vec::new(),
                isotope: None,
            }],
            diagnostics: Vec::new(),
        }
    }

    pub(crate) fn commands(hardened: bool, commands: Vec<HardenerCommand>) -> Self {
        let applicable = commands.iter().any(|command| {
            command
                .stub_path
                .as_deref()
                .is_some_and(|path| Path::new(path).exists())
                || Path::new(&command.target_path).exists()
        });
        let primary = commands.first();
        Self {
            hardened,
            applicable,
            stub_path: primary.and_then(|command| command.stub_path.clone()),
            target_path: primary.map(|command| command.target_path.clone()),
            commands,
            diagnostics: Vec::new(),
        }
    }

    pub(crate) fn configuration(
        hardened: bool,
        applicable: bool,
        target_path: Option<String>,
    ) -> Self {
        Self {
            hardened,
            applicable,
            stub_path: None,
            target_path,
            commands: Vec::new(),
            diagnostics: Vec::new(),
        }
    }
}

use std::os::unix::fs::PermissionsExt;
use std::path::Path;

pub(crate) fn executable(path: &Path) -> bool {
    path.metadata()
        .is_ok_and(|metadata| metadata.is_file() && metadata.permissions().mode() & 0o111 != 0)
}

macro_rules! gated_hardener {
    ($module:ident, $name:literal) => {
        HardenerMetadata {
            name: $name,
            documentation: include_str!(concat!(stringify!($module), ".md")),
            detection: $module::detect(),
            secret_gate: Some($module::secret_gate()),
        }
    };
}

macro_rules! ungated_hardener {
    ($module:ident, $name:literal) => {
        HardenerMetadata {
            name: $name,
            documentation: include_str!(concat!(stringify!($module), ".md")),
            detection: $module::detect(),
            secret_gate: None,
        }
    };
}

pub(crate) fn metadata() -> Vec<HardenerMetadata> {
    let mut metadata = vec![
        gated_hardener!(aws_cli, "aws"),
        ungated_hardener!(codex, "codex"),
        gated_hardener!(docker, "docker"),
        gated_hardener!(homebrew, "brew"),
        gated_hardener!(gh_cli, "gh"),
        gated_hardener!(stripe_cli, "stripe"),
        ungated_hardener!(sudo, "sudo"),
        gated_hardener!(supabase, "supabase"),
    ];
    metadata.extend(env_wrapper::metadata());
    metadata
}

pub(crate) fn secret_gates() -> Vec<SecretGateDescriptor> {
    let mut gates = vec![
        aws_cli::secret_gate(),
        docker::secret_gate(),
        homebrew::secret_gate(),
        gh_cli::secret_gate(),
        stripe_cli::secret_gate(),
        supabase::secret_gate(),
    ];
    gates.extend(env_wrapper::secret_gates());
    gates
}

#[cfg(test)]
mod tests {
    #[test]
    fn secret_gate_notices_match_the_policy_defaults() {
        let mut aws = Vec::new();
        super::write_secret_gate_notice(&mut aws, "aws");
        assert_eq!(
            String::from_utf8(aws).unwrap(),
            "\n◇ `aws` defaults to Read Only, adjust this in the app: `av open --secret-gate aws`\n"
        );

        let mut brew = Vec::new();
        super::write_secret_gate_notice(&mut brew, "brew");
        assert_eq!(
            String::from_utf8(brew).unwrap(),
            "\n◇ `brew` defaults to Read & Update, adjust this in the app: `av open --secret-gate brew`\n"
        );
    }
}
