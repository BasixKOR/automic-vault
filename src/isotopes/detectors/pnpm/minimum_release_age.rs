pub(crate) fn findings(home: &std::path::Path) -> Vec<crate::Finding> {
    crate::isotopes::detectors::js_release_age::policy_findings(
        "pnpm-minimum-release-age",
        home,
        super::pnpm_config_paths(home),
        crate::isotopes::detectors::js_release_age::pnpm_minutes,
        "Set `minimum-release-age=1440` in the reported pnpm config file.",
    )
}
