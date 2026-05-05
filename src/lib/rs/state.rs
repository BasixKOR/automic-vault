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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_wrappers_list_refs_and_load_receipts() {
        let package_name = "coverage-state";
        let opt_root = opt_pkg_root();
        let install_root = opt_root.join(package_name);
        if fs::symlink_metadata(&install_root).is_ok() {
            remove_path(&install_root).unwrap();
        }
        fs::create_dir_all(&install_root).unwrap();
        write_package_receipt(
            &install_root.join(ROOT_RECEIPT),
            &PackageReceipt {
                package_name: package_name.to_string(),
                version: "1.2.3".to_string(),
                source: PackageReceiptSource::Formula {
                    root_formula: package_name.to_string(),
                },
                metadata: PackageMetadata::default(),
            },
        )
        .unwrap();

        assert!(
            list_installed_package_refs()
                .unwrap()
                .iter()
                .any(|package| package.package_name == package_name)
        );
        assert_eq!(
            load_installed_package_receipt(package_name, &install_root)
                .unwrap()
                .version,
            "1.2.3"
        );

        remove_path(&install_root).unwrap();
    }
}
