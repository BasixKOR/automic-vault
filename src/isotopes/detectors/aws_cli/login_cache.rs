pub fn install_insecurity_reasons() -> Result<Vec<String>, String> {
    super::reasons_matching("AWS login cache contains cached access credentials")
}

pub(crate) fn findings(home: &std::path::Path) -> Vec<crate::Finding> {
    crate::isotopes::detectors::radioisotope::findings(
        "aws-cli-login-cache",
        install_insecurity_reasons,
        home,
    )
}
