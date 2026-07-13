pub(crate) mod aws_cli;
pub(crate) mod env_wrapper;
pub(crate) mod gh_cli;
pub(crate) mod homebrew;
pub(crate) mod sudo;
pub(crate) mod supabase;

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
            }],
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
        ungated_hardener!(homebrew, "brew"),
        gated_hardener!(gh_cli, "gh"),
        ungated_hardener!(sudo, "sudo"),
        gated_hardener!(supabase, "supabase"),
    ];
    metadata.extend(env_wrapper::metadata());
    metadata
}
