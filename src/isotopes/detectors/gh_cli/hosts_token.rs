pub fn install_insecurity_reasons() -> Result<Vec<String>, String> {
    super::reasons_matching("GitHub CLI hosts file contains plaintext OAuth tokens")
}

pub(crate) fn findings(home: &std::path::Path) -> Vec<crate::Finding> {
    crate::isotopes::detectors::radioisotope::findings(
        "gh-cli-hosts-token",
        install_insecurity_reasons,
        home,
    )
}
