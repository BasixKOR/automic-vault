pub(crate) mod aws_cli;
pub(crate) mod gh_cli;

pub(crate) struct HardenerMetadata {
    pub(crate) name: &'static str,
    pub(crate) documentation: &'static str,
}

macro_rules! hardener {
    ($module:ident, $name:literal) => {
        HardenerMetadata {
            name: $name,
            documentation: include_str!(concat!(stringify!($module), ".md")),
        }
    };
}

const HARDENERS: &[HardenerMetadata] = &[hardener!(aws_cli, "aws"), hardener!(gh_cli, "gh-cli")];

pub(crate) fn metadata() -> &'static [HardenerMetadata] {
    HARDENERS
}
