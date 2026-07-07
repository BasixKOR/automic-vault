pub fn install_insecurity_reasons() -> Result<Vec<String>, String> {
    super::reasons_matching("AWS CLI legacy plugins are configured")
}

pub(crate) fn findings(home: &std::path::Path) -> Vec<crate::Finding> {
    crate::isotopes::detectors::radioisotope::findings(
        "aws-cli-legacy-plugins",
        install_insecurity_reasons,
        home,
    )
}
