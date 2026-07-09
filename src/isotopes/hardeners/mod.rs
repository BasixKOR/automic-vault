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

macro_rules! hardener {
    ($module:ident, $name:literal) => {
        HardenerMetadata {
            name: $name,
            documentation: include_str!(concat!(stringify!($module), ".md")),
            detection: $module::detect(),
        }
    };
}

pub(crate) fn metadata() -> Vec<HardenerMetadata> {
    let mut metadata = vec![
        hardener!(aws_cli, "aws"),
        hardener!(homebrew, "brew"),
        hardener!(gh_cli, "gh"),
        hardener!(sudo, "sudo"),
        hardener!(supabase, "supabase"),
    ];
    metadata.extend(env_wrapper::metadata());
    metadata
}
