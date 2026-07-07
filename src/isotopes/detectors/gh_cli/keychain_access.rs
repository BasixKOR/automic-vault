pub fn install_insecurity_reasons() -> Result<Vec<String>, String> {
    super::reasons_matching("GitHub CLI keychain item allows non-interactive extraction")
}

pub(crate) fn findings(home: &std::path::Path) -> Vec<crate::Finding> {
    crate::isotopes::detectors::radioisotope::findings(
        "gh-cli-keychain-access",
        install_insecurity_reasons,
        home,
    )
}
