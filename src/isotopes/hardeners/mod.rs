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
    pub(crate) stub_path: Option<String>,
    pub(crate) target_path: Option<String>,
}

impl HardenerDetection {
    pub(crate) fn hardened(stub_path: Option<String>, target_path: Option<String>) -> Self {
        Self {
            hardened: true,
            stub_path,
            target_path,
        }
    }

    pub(crate) fn missing(target_path: Option<String>) -> Self {
        Self {
            hardened: false,
            stub_path: None,
            target_path,
        }
    }
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
