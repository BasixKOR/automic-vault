pub fn install_insecurity_reasons() -> Result<Vec<String>, String> {
    super::reasons_matching("AWS shared credentials file contains plaintext access keys")
}

pub(crate) fn findings(home: &std::path::Path) -> Vec<crate::Finding> {
    crate::isotopes::detectors::radioisotope::findings(
        "aws-cli-credentials-file",
        install_insecurity_reasons,
        home,
    )
}
