use super::*;

pub(crate) fn run_i_vendor(
    config: &Config,
    package_name: String,
    package: vendor::VendorPackage,
    progress_callback: Option<Arc<Mutex<Box<ProgressCallback>>>>,
) -> Result<(), String> {
    let progress = InstallProgress::with_callback(&package_name, progress_callback);
    let result = (|| {
        let plan = InstallPlan::for_i(package_name.clone(), package.name.to_string());
        let previous_stubs = load_stub_manifest(&plan.package_manifest_path())?.stubs;
        let (staged_plan, staging_workspace) = prepare_i_install_plan(&plan)?;
        let install_result = (|| {
            let version = (package.version)()?;
            let vendor_install = VendorInstall { package, version };
            let dependencies = resolve_vendor_dependency_specs(
                vendor_install.package.dependencies,
                config,
                false,
            )?;
            let dependency_state = resolve_dependency_install_state(
                &dependencies.formula_graph,
                &staged_plan.tmp_root,
                Some(&progress),
            )?;
            ensure_plan_parent_dirs(&staged_plan)?;

            let dependency_current = dependencies_are_current(
                &staged_plan,
                &dependency_state.installs,
                &dependencies.vendor_installs,
                config,
            )?;
            let mut dependencies_reinstalled = false;
            if !dependency_current {
                progress.begin_install_phase();
                install_dependency_formulas(
                    config,
                    &staged_plan,
                    &dependency_state.installs,
                    Some(&progress),
                )?;
                install_vendor_dependencies(
                    &staged_plan,
                    &dependencies.formula_graph,
                    &dependencies.vendor_installs,
                    Some(&progress),
                )?;
                dependencies_reinstalled = true;
            }

            if !vendor_root_is_current(
                &staged_plan,
                &vendor_install,
                &dependency_state.installs,
                &config.bottle_tag,
            )? {
                if !dependencies_reinstalled {
                    if dependencies.formula_graph.is_empty()
                        && dependencies.vendor_installs.is_empty()
                    {
                        prepare_vendor_root_area(&staged_plan)?;
                    } else {
                        reinstall_vendor_dependency_tree(
                            config,
                            &staged_plan,
                            &dependency_state.installs,
                            &dependencies.formula_graph,
                            &dependencies.vendor_installs,
                            Some(&progress),
                        )?;
                    }
                }
                install_vendor_root(
                    &staged_plan,
                    &dependencies.formula_graph,
                    &vendor_install,
                    Some(&progress),
                )?;
            }

            activate_install(&staged_plan)?;
            write_package_receipt(
                &plan.root_receipt_path(),
                &PackageReceipt {
                    package_name: package_name.clone(),
                    version: vendor_install.version.to_string(),
                    source: PackageReceiptSource::Vendor {
                        vendor_name: vendor_install.package.name.to_string(),
                    },
                    metadata: PackageMetadata::default(),
                },
            )?;
            sync_vendor_stubs(
                &plan,
                &dependencies.formula_graph,
                &vendor_install.package,
                &previous_stubs,
            )?;
            installed_stub_paths(&plan)
        })();
        if install_result.is_err() {
            preserve_optional_temp_dir_on_failure(staging_workspace);
        }
        install_result
    })();

    match result {
        Ok(paths) => {
            progress.finish_with_paths(&paths);
            Ok(())
        }
        Err(err) => {
            progress.clear();
            Err(err)
        }
    }
}

pub(crate) fn run_i_npm(
    config: &Config,
    package_name: String,
    npm_package: String,
    requested_version: Option<String>,
    _options: InstallOptions,
    progress_callback: Option<Arc<Mutex<Box<ProgressCallback>>>>,
) -> Result<(), String> {
    let progress = InstallProgress::with_callback(&package_name, progress_callback);
    let result = (|| {
        let plan = InstallPlan::for_i_npm(package_name.clone(), package_name.clone(), &npm_package);
        let previous_stubs = load_stub_manifest(&plan.package_manifest_path())?.stubs;
        let (staged_plan, staging_workspace) = prepare_i_install_plan(&plan)?;
        let install_result = (|| {
            let executable = npm_package_executable_name(&npm_package);
            let mut dependency_names = vec!["node".to_string()];
            append_npm_package_homebrew_dependencies(&mut dependency_names, &npm_package);
            let dependency_names = dependency_names
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>();
            let dependencies = resolve_vendor_dependency_specs(&dependency_names, config, false)?;
            let dependency_state = resolve_dependency_install_state(
                &dependencies.formula_graph,
                &staged_plan.tmp_root,
                Some(&progress),
            )?;
            ensure_plan_parent_dirs(&staged_plan)?;

            let dependency_current = dependencies_are_current(
                &staged_plan,
                &dependency_state.installs,
                &dependencies.vendor_installs,
                config,
            )?;
            let mut dependencies_reinstalled = false;
            if !dependency_current {
                progress.begin_install_phase();
                install_dependency_formulas(
                    config,
                    &staged_plan,
                    &dependency_state.installs,
                    Some(&progress),
                )?;
                install_vendor_dependencies(
                    &staged_plan,
                    &dependencies.formula_graph,
                    &dependencies.vendor_installs,
                    Some(&progress),
                )?;
                dependencies_reinstalled = true;
            }

            let version = resolve_installable_npm_version(
                &staged_plan,
                &dependencies.formula_graph,
                &package_name,
                &npm_package,
                requested_version.as_deref(),
                Some(&progress),
            )?;

            if !npm_root_is_current(
                &staged_plan,
                &executable,
                &version,
                &dependency_state.installs,
                &config.bottle_tag,
            )? {
                if !dependencies_reinstalled {
                    if dependencies.formula_graph.is_empty()
                        && dependencies.vendor_installs.is_empty()
                    {
                        prepare_vendor_root_area(&staged_plan)?;
                    } else {
                        reinstall_vendor_dependency_tree(
                            config,
                            &staged_plan,
                            &dependency_state.installs,
                            &dependencies.formula_graph,
                            &dependencies.vendor_installs,
                            Some(&progress),
                        )?;
                    }
                }
                install_npm_root(
                    &staged_plan,
                    &dependencies.formula_graph,
                    &package_name,
                    &npm_package,
                    &version,
                    Some(&progress),
                )?;
            }

            activate_install(&staged_plan)?;
            let metadata = resolve_npm_package_metadata(&npm_package)?;
            write_package_receipt(
                &plan.root_receipt_path(),
                &PackageReceipt {
                    package_name: package_name.clone(),
                    version: version.to_string(),
                    source: PackageReceiptSource::Npm {
                        package_name: npm_package.clone(),
                    },
                    metadata,
                },
            )?;
            sync_declared_stubs(
                &plan,
                &dependencies.formula_graph,
                [executable.as_str()],
                &package_stub_exclusions(&plan.package_name),
                &previous_stubs,
            )?;
            installed_stub_paths(&plan)
        })();
        if install_result.is_err() {
            preserve_optional_temp_dir_on_failure(staging_workspace);
        }
        install_result
    })();

    match result {
        Ok(paths) => {
            progress.finish_with_paths(&paths);
            Ok(())
        }
        Err(err) => {
            progress.clear();
            Err(err)
        }
    }
}

pub(crate) fn run_i_pip(
    config: &Config,
    package_name: String,
    pip_package: String,
    progress_callback: Option<Arc<Mutex<Box<ProgressCallback>>>>,
) -> Result<(), String> {
    let progress = InstallProgress::with_callback(&package_name, progress_callback);
    let result = (|| {
        let plan = InstallPlan::for_i_pip(package_name.clone(), package_name.clone(), &pip_package);
        let previous_stubs = load_stub_manifest(&plan.package_manifest_path())?.stubs;
        let version = resolve_pip_latest_version(&pip_package)?;
        let mut dependency_names = vec![pip_package_python_formula(&pip_package)];
        append_pip_package_homebrew_dependencies(&mut dependency_names, &pip_package);
        let formula_graph = resolve_formula_specs(&dependency_names, config, true)?;
        let dependency_state =
            resolve_dependency_install_state(&formula_graph, &plan.tmp_root, Some(&progress))?;
        ensure_plan_parent_dirs(&plan)?;

        let dependency_current =
            dependencies_are_current(&plan, &dependency_state.installs, &[], config)?;
        let mut dependencies_reinstalled = false;
        if !dependency_current {
            progress.begin_install_phase();
            install_dependency_formulas(
                config,
                &plan,
                &dependency_state.installs,
                Some(&progress),
            )?;
            dependencies_reinstalled = true;
        }

        if !pip_root_is_current(
            &plan,
            &version,
            &dependency_state.installs,
            &config.bottle_tag,
        )? {
            if !dependencies_reinstalled {
                reinstall_vendor_dependency_tree(
                    config,
                    &plan,
                    &dependency_state.installs,
                    &[],
                    &[],
                    Some(&progress),
                )?;
            }
            let entrypoints = install_pip_root(
                &plan,
                &formula_graph,
                &package_name,
                &pip_package,
                &version,
                Some(&progress),
            )?;
            write_root_executable_manifest(&plan.root_executables_manifest_path(), &entrypoints)?;
        }

        activate_install(&plan)?;
        let metadata = resolve_pip_package_metadata(&pip_package)?;
        write_package_receipt(
            &plan.root_receipt_path(),
            &PackageReceipt {
                package_name: package_name.clone(),
                version: version.to_string(),
                source: PackageReceiptSource::Pip {
                    package_name: pip_package.clone(),
                },
                metadata,
            },
        )?;
        let root_executables =
            load_root_executable_manifest(&plan.root_executables_manifest_path())?.stubs;
        sync_declared_stubs(
            &plan,
            &formula_graph,
            &root_executables,
            &package_stub_exclusions(&plan.package_name),
            &previous_stubs,
        )?;
        installed_stub_paths(&plan)
    })();

    match result {
        Ok(paths) => {
            progress.finish_with_paths(&paths);
            Ok(())
        }
        Err(err) => {
            progress.clear();
            Err(err)
        }
    }
}
