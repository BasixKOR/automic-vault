use super::*;

pub(crate) fn list_installed_package_refs() -> Result<Vec<InstalledPackageRef>, String> {
    installed_package_refs(&opt_pkg_root())
}

pub(crate) fn load_installed_package_receipt(
    package_name: &str,
    install_root: &Path,
) -> Result<PackageReceipt, String> {
    load_or_resolve_package_receipt(package_name, install_root)
}
