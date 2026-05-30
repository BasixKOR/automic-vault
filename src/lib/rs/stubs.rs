use super::*;

pub(crate) fn remove_package_stubs_from_bin(
    opt_root: &Path,
    package_name: &str,
    bin_dir: &Path,
) -> Result<(), String> {
    let install_root = package_install_root(opt_root, package_name)?;
    let manifest = load_stub_manifest(&install_root.join(STUB_MANIFEST))?;
    let shared_stubs = collect_shared_stubs(opt_root, package_name)?;

    for stub in manifest.stubs {
        if shared_stubs.contains(&stub) {
            let path = bin_dir.join(&stub);
            if !stub_belongs_to_package(&path, package_name)? {
                continue;
            }
        }
        let path = bin_dir.join(&stub);
        if path.exists() || fs::symlink_metadata(&path).is_ok() {
            remove_path(&path)?;
        }
    }

    Ok(())
}

pub(crate) fn remove_existing_package_install(
    opt_root: &Path,
    package_name: &str,
    bin_dir: &Path,
) -> Result<(), String> {
    let install_root = package_install_root(opt_root, package_name)?;
    match fs::symlink_metadata(&install_root) {
        Ok(_) => {
            remove_package_stubs_from_bin(opt_root, package_name, bin_dir)?;
            remove_path(&install_root)?;
            if package_name.starts_with("npm:@") {
                remove_empty_parent_dirs(&install_root, &opt_root.join("npm"))?;
            }
            refresh_post_uninstall_stubs(opt_root, bin_dir)
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(format!("failed to stat {}: {err}", install_root.display())),
    }
}

pub(crate) fn remove_empty_parent_dirs(path: &Path, stop_at: &Path) -> Result<(), String> {
    let mut current = path.parent();
    while let Some(dir) = current {
        if dir == stop_at {
            break;
        }
        match fs::remove_dir(dir) {
            Ok(()) => current = dir.parent(),
            Err(err) if err.kind() == std::io::ErrorKind::DirectoryNotEmpty => break,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => current = dir.parent(),
            Err(err) => return Err(format!("failed to remove {}: {err}", dir.display())),
        }
    }
    Ok(())
}

pub(crate) fn collect_shared_stubs(
    opt_root: &Path,
    excluded_package: &str,
) -> Result<HashSet<String>, String> {
    let mut stubs = HashSet::new();
    for package in installed_package_refs(opt_root)? {
        if package.package_name == excluded_package {
            continue;
        }
        for stub in load_stub_manifest(&package.install_root.join(STUB_MANIFEST))?.stubs {
            stubs.insert(stub);
        }
    }

    Ok(stubs)
}

pub(crate) fn refresh_post_uninstall_stubs(opt_root: &Path, bin_dir: &Path) -> Result<(), String> {
    match fs::symlink_metadata(bin_dir) {
        Ok(_) => {}
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(err) => return Err(format!("failed to stat {}: {err}", bin_dir.display())),
    }

    for (formula, prefix) in find_supported_post_install_prefixes(opt_root)? {
        post_install_hooks::run(&formula, &prefix, bin_dir)?;
    }

    Ok(())
}

pub(crate) fn find_supported_post_install_prefixes(
    opt_root: &Path,
) -> Result<Vec<(String, PathBuf)>, String> {
    let mut prefixes = Vec::new();
    for package in installed_package_refs(opt_root)? {
        if let Some(formula) = installed_post_install_formula(&package.install_root)? {
            prefixes.push((formula, package.install_root.clone()));
        }
    }

    Ok(prefixes)
}

pub(crate) fn installed_post_install_formula(
    install_root: &Path,
) -> Result<Option<String>, String> {
    if let Some(receipt) = load_package_receipt(&install_root.join(ROOT_RECEIPT))? {
        let PackageReceiptSource::Formula { root_formula } = receipt.source else {
            return Ok(None);
        };
        if post_install_hooks::supports(&root_formula) {
            return Ok(Some(root_formula));
        }
        return Ok(None);
    }

    Ok(None)
}

pub(crate) fn sync_stubs(
    plan: &InstallPlan,
    graph: &[FormulaSpec],
    previous_stubs: &[String],
) -> Result<(), String> {
    if plan.mode != Mode::I {
        return Ok(());
    }

    let bin_root = managed_bin_root();
    fs::create_dir_all(&bin_root)
        .map_err(|err| format!("failed to create {}: {err}", bin_root.display()))?;
    let current = if plan.mode == Mode::I {
        let manifest = load_root_executable_manifest(&plan.root_executables_manifest_path())?;
        if manifest.stubs.is_empty() {
            collect_root_executables(&plan.install_root)?
        } else {
            collect_declared_root_executables(
                &plan.install_root,
                manifest.stubs.iter().map(String::as_str),
            )?
        }
    } else {
        collect_root_executables(&plan.stable_root)?
    };
    let mut excluded_stubs = formula_stub_exclusions(&plan.root_formula);
    excluded_stubs.extend(imagemagick_stub_exclusions(plan, &current));
    let current = filter_stub_executables(current, &excluded_stubs);
    for stub in previous_stubs {
        if !current.iter().any(|(name, _)| name == stub) {
            let path = bin_root.join(stub);
            if path.exists() || fs::symlink_metadata(&path).is_ok() {
                remove_path(&path)?;
            }
        }
    }

    let env_entries = build_exec_path_entries(plan, graph);
    for (name, actual_path) in &current {
        let stub_path = bin_root.join(name);
        write_stub(plan, &stub_path, actual_path, &env_entries)?;
    }

    write_stub_manifest(
        &plan.package_manifest_path(),
        &StubManifest {
            stubs: current.into_iter().map(|(name, _)| name).collect(),
        },
    )
}

pub(crate) fn sync_vendor_stubs(
    plan: &InstallPlan,
    graph: &[FormulaSpec],
    package: &vendor::VendorPackage,
    previous_stubs: &[String],
) -> Result<(), String> {
    sync_declared_stubs(
        plan,
        graph,
        package.executables.iter().copied(),
        &vendor_stub_exclusions(package),
        previous_stubs,
    )
}

pub(crate) fn sync_declared_stubs<I, S>(
    plan: &InstallPlan,
    graph: &[FormulaSpec],
    executables: I,
    excluded_stubs: &HashSet<String>,
    previous_stubs: &[String],
) -> Result<(), String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    if plan.mode != Mode::I {
        return Ok(());
    }

    let bin_root = managed_bin_root();
    fs::create_dir_all(&bin_root)
        .map_err(|err| format!("failed to create {}: {err}", bin_root.display()))?;
    let current = filter_stub_executables(
        collect_declared_root_executables(&plan.install_root, executables)?,
        excluded_stubs,
    );
    for stub in previous_stubs {
        if !current.iter().any(|(name, _)| name == stub) {
            let path = bin_root.join(stub);
            if path.exists() || fs::symlink_metadata(&path).is_ok() {
                remove_path(&path)?;
            }
        }
    }

    let env_entries = build_exec_path_entries(plan, graph);
    for (name, actual_path) in &current {
        let stub_path = bin_root.join(name);
        write_stub(plan, &stub_path, actual_path, &env_entries)?;
    }

    write_stub_manifest(
        &plan.package_manifest_path(),
        &StubManifest {
            stubs: current.into_iter().map(|(name, _)| name).collect(),
        },
    )
}

pub(crate) fn run_package_post_install(
    plan: &InstallPlan,
    installs: &[InstalledFormula],
    bin_dir: &Path,
) -> Result<(), String> {
    let mut formulas = Vec::new();
    if plan.mode == Mode::I && post_install_hooks::supports(&plan.root_formula) {
        push_unique_string(&mut formulas, plan.root_formula.clone());
    }
    for install in installs {
        if post_install_hooks::supports_dependency(&install.spec.name) {
            push_unique_string(&mut formulas, install.spec.name.clone());
        }
    }
    if formulas.is_empty() {
        return Ok(());
    }

    let mut manifest = load_stub_manifest(&plan.package_manifest_path())?;
    let mut manifest_changed = false;
    for formula in formulas {
        let outcome = post_install_hooks::run(&formula, &plan.install_root, bin_dir)?;
        for stub in outcome.managed_stubs {
            push_unique_string(&mut manifest.stubs, stub);
            manifest_changed = true;
        }
    }
    if !manifest_changed {
        return Ok(());
    }

    manifest.stubs.sort();
    write_stub_manifest(&plan.package_manifest_path(), &manifest)
}

pub(crate) fn collect_root_executables(root: &Path) -> Result<Vec<(String, PathBuf)>, String> {
    let mut execs = Vec::new();
    for dir in [root.join("bin"), root.join("sbin")] {
        let entries = match fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => continue,
            Err(err) => return Err(format!("failed to read {}: {err}", dir.display())),
        };
        for entry in entries {
            let entry = entry.map_err(|err| format!("failed to read {}: {err}", dir.display()))?;
            let path = entry.path();
            if !is_executable(&path) {
                continue;
            }
            let name = entry
                .file_name()
                .to_str()
                .ok_or_else(|| format!("non-utf8 executable name under {}", dir.display()))?
                .to_string();
            execs.push((name, path));
        }
    }
    execs.sort_by(|left, right| left.0.cmp(&right.0));
    Ok(execs)
}

pub(crate) fn collect_declared_root_executables(
    root: &Path,
    executables: impl IntoIterator<Item = impl AsRef<str>>,
) -> Result<Vec<(String, PathBuf)>, String> {
    let mut execs = Vec::new();
    for executable in executables {
        let executable = executable.as_ref();
        let path = ["bin", "sbin"]
            .into_iter()
            .map(|dir| root.join(dir).join(executable))
            .find(|candidate| is_executable(candidate))
            .ok_or_else(|| format!("expected executable {executable} under {}", root.display()))?;
        execs.push((executable.to_string(), path));
    }
    execs.sort_by(|left, right| left.0.cmp(&right.0));
    Ok(execs)
}

pub(crate) fn filter_stub_executables(
    executables: Vec<(String, PathBuf)>,
    excluded_stubs: &HashSet<String>,
) -> Vec<(String, PathBuf)> {
    executables
        .into_iter()
        .filter(|(name, _)| !excluded_stubs.contains(name))
        .collect()
}

pub(crate) fn declared_root_executables_exist(
    root: &Path,
    executables: impl IntoIterator<Item = impl AsRef<str>>,
) -> bool {
    executables.into_iter().all(|executable| {
        let executable = executable.as_ref();
        ["bin", "sbin"]
            .into_iter()
            .map(|dir| root.join(dir).join(executable))
            .any(|candidate| is_executable(&candidate))
    })
}

pub(crate) fn load_stub_manifest(path: &Path) -> Result<StubManifest, String> {
    let data = match fs::read(path) {
        Ok(data) => data,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return Ok(StubManifest { stubs: Vec::new() });
        }
        Err(err) => return Err(format!("failed to read {}: {err}", path.display())),
    };
    serde_json::from_slice(&data)
        .map_err(|err| format!("failed to parse {}: {err}", path.display()))
}

pub(crate) fn installed_stub_paths(plan: &InstallPlan) -> Result<Vec<String>, String> {
    let mut paths = load_stub_manifest(&plan.package_manifest_path())?
        .stubs
        .into_iter()
        .map(|stub| managed_bin_root().join(stub).display().to_string())
        .collect::<Vec<_>>();
    paths.sort();
    Ok(paths)
}

pub(crate) fn write_stub_manifest(path: &Path, manifest: &StubManifest) -> Result<(), String> {
    let data = serde_json::to_vec_pretty(manifest)
        .map_err(|err| format!("failed to serialize stub manifest: {err}"))?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("failed to create {}: {err}", parent.display()))?;
    }
    fs::write(path, data).map_err(|err| format!("failed to write {}: {err}", path.display()))
}

pub(crate) fn load_root_executable_manifest(path: &Path) -> Result<StubManifest, String> {
    load_stub_manifest(path)
}

pub(crate) fn write_root_executable_manifest(
    path: &Path,
    executables: &[String],
) -> Result<(), String> {
    write_stub_manifest(
        path,
        &StubManifest {
            stubs: executables.to_vec(),
        },
    )
}

pub(crate) fn write_stub(
    plan: &InstallPlan,
    stub_path: &Path,
    actual_path: &Path,
    env_entries: &[PathBuf],
) -> Result<(), String> {
    let path_prefix = if env_entries.is_empty() {
        "\"$PATH\"".to_string()
    } else {
        let joined = env::join_paths(env_entries).map_err(|err| {
            format!(
                "failed to join PATH for stub {}: {err}",
                stub_path.display()
            )
        })?;
        format!(
            "\"{}:$PATH\"",
            shell_double_quote_escape(joined.to_string_lossy().as_ref())
        )
    };

    let script = format!(
        "{}PATH={path_prefix}\nexport PATH\nexec {} \"$@\"\n",
        stub_script_prelude(&format!("{STUB_HEADER} {}", plan.package_name)),
        shell_quote(actual_path.to_string_lossy().as_ref()),
    );
    fs::write(stub_path, script)
        .map_err(|err| format!("failed to write {}: {err}", stub_path.display()))?;
    let mut permissions = fs::metadata(stub_path)
        .map_err(|err| format!("failed to stat {}: {err}", stub_path.display()))?
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(stub_path, permissions)
        .map_err(|err| format!("failed to chmod {}: {err}", stub_path.display()))
}

pub(crate) fn stub_belongs_to_package(
    stub_path: &Path,
    package_name: &str,
) -> Result<bool, String> {
    let data = match fs::read_to_string(stub_path) {
        Ok(data) => data,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(err) => return Err(format!("failed to read {}: {err}", stub_path.display())),
    };
    Ok(data
        .lines()
        .any(|line| line == format!("{STUB_HEADER} {package_name}")))
}

pub(crate) fn write_venv_stub(
    plan: &InstallPlan,
    stub_path: &Path,
    actual_path: &Path,
    venv_root: &Path,
) -> Result<(), String> {
    let venv_bin = venv_root.join("bin");
    let script = format!(
        "{}VIRTUAL_ENV={}\nexport VIRTUAL_ENV\nunset PYTHONHOME\nPATH=\"{}:$PATH\"\nexport PATH\nexec {} \"$@\"\n",
        stub_script_prelude(&format!("{STUB_HEADER} {}", plan.package_name)),
        shell_quote(venv_root.to_string_lossy().as_ref()),
        shell_double_quote_escape(venv_bin.to_string_lossy().as_ref()),
        shell_quote(actual_path.to_string_lossy().as_ref()),
    );
    fs::write(stub_path, script)
        .map_err(|err| format!("failed to write {}: {err}", stub_path.display()))?;
    let mut permissions = fs::metadata(stub_path)
        .map_err(|err| format!("failed to stat {}: {err}", stub_path.display()))?
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(stub_path, permissions)
        .map_err(|err| format!("failed to chmod {}: {err}", stub_path.display()))
}

pub(crate) fn stub_script_prelude(header_line: &str) -> String {
    format!("#!/bin/sh\n{header_line}\n")
}

pub(crate) fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}

pub(crate) fn shell_double_quote_escape(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for ch in value.chars() {
        if matches!(ch, '\\' | '"' | '$' | '`') {
            escaped.push('\\');
        }
        escaped.push(ch);
    }
    escaped
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_executable(path: &Path) {
        fs::write(path, "#!/bin/sh\n").unwrap();
        let mut permissions = fs::metadata(path).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(path, permissions).unwrap();
    }

    #[test]
    fn remove_empty_parent_dirs_stops_at_boundary() {
        let temp = TempDir::new().unwrap();
        let stop_at = temp.path().join("npm");
        let nested = stop_at.join("@scope/pkg");
        fs::create_dir_all(&nested).unwrap();
        fs::remove_dir(&nested).unwrap();

        remove_empty_parent_dirs(&nested, &stop_at).unwrap();

        assert!(stop_at.exists());
        assert!(!stop_at.join("@scope").exists());
    }

    #[test]
    fn collect_declared_root_executables_prefers_existing_bin_and_sbin_entries() {
        let temp = TempDir::new().unwrap();
        let root = temp.path();
        fs::create_dir_all(root.join("bin")).unwrap();
        fs::create_dir_all(root.join("sbin")).unwrap();
        write_executable(&root.join("bin/alpha"));
        write_executable(&root.join("sbin/beta"));

        let execs = collect_declared_root_executables(root, ["beta", "alpha"]).unwrap();

        assert_eq!(
            execs,
            vec![
                ("alpha".to_string(), root.join("bin/alpha")),
                ("beta".to_string(), root.join("sbin/beta")),
            ]
        );
    }

    #[test]
    fn collect_root_executables_filters_non_executables_and_sorts_names() {
        let temp = TempDir::new().unwrap();
        let root = temp.path();
        fs::create_dir_all(root.join("bin")).unwrap();
        fs::create_dir_all(root.join("sbin")).unwrap();
        write_executable(&root.join("bin/zeta"));
        write_executable(&root.join("sbin/alpha"));
        fs::write(root.join("bin/readme.txt"), "plain text").unwrap();

        let execs = collect_root_executables(root).unwrap();

        assert_eq!(
            execs,
            vec![
                ("alpha".to_string(), root.join("sbin/alpha")),
                ("zeta".to_string(), root.join("bin/zeta")),
            ]
        );
    }

    #[test]
    fn shell_helpers_escape_single_and_double_quoted_contexts() {
        assert_eq!(shell_quote("a'b"), "'a'\"'\"'b'");
        assert_eq!(shell_double_quote_escape("a\"$`b\\c"), "a\\\"\\$\\`b\\\\c");
        assert_eq!(
            stub_script_prelude("# generated by av package"),
            "#!/bin/sh\n# generated by av package\n"
        );
    }

    #[test]
    fn write_stub_reports_unjoinable_path_entries() {
        let temp = TempDir::new().unwrap();
        let plan = InstallPlan {
            mode: Mode::I,
            package_name: "bad-path".to_string(),
            root_formula: "bad-path".to_string(),
            stable_root: temp.path().join("stable"),
            install_root: temp.path().join("install"),
            tmp_root: temp.path().join("tmp"),
        };
        let error = write_stub(
            &plan,
            &temp.path().join("stub"),
            &temp.path().join("bin/tool"),
            &[PathBuf::from("bad:path")],
        )
        .unwrap_err();

        assert!(error.contains("failed to join PATH"));
    }

    #[test]
    fn load_stub_manifest_defaults_missing_file_and_reports_parse_errors() {
        let temp = TempDir::new().unwrap();
        let missing = temp.path().join("missing.json");
        assert!(load_stub_manifest(&missing).unwrap().stubs.is_empty());

        let invalid = temp.path().join("invalid.json");
        fs::write(&invalid, "{not-json").unwrap();
        let error = load_stub_manifest(&invalid).unwrap_err();
        assert!(error.contains("failed to parse"));
        assert!(error.contains("invalid.json"));
    }

    #[test]
    fn filter_and_declared_exist_cover_inclusion_and_missing_paths() {
        let temp = TempDir::new().unwrap();
        let root = temp.path();
        fs::create_dir_all(root.join("bin")).unwrap();
        write_executable(&root.join("bin/keep"));

        let filtered = filter_stub_executables(
            vec![
                ("drop".to_string(), root.join("bin/drop")),
                ("keep".to_string(), root.join("bin/keep")),
            ],
            &HashSet::from(["drop".to_string()]),
        );
        assert_eq!(filtered, vec![("keep".to_string(), root.join("bin/keep"))]);
        assert!(declared_root_executables_exist(root, ["keep"]));
        assert!(!declared_root_executables_exist(root, ["keep", "missing"]));
    }

    #[test]
    fn stub_belongs_to_package_matches_generated_header() {
        let temp = TempDir::new().unwrap();
        let stub_path = temp.path().join("stub");
        fs::write(
            &stub_path,
            format!(
                "{}PATH=\"$PATH\"\n",
                stub_script_prelude(&format!("{STUB_HEADER} isotope:aws"))
            ),
        )
        .unwrap();

        assert!(stub_belongs_to_package(&stub_path, "isotope:aws").unwrap());
        assert!(!stub_belongs_to_package(&stub_path, "isotope:gh").unwrap());
        assert!(!stub_belongs_to_package(&temp.path().join("missing"), "isotope:aws").unwrap());
    }
}
