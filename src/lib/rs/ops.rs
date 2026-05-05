use super::*;

const MAX_HELPER_PACKAGES: usize = 50;
const HELPER_AV_INSTALL_TARGET: &str = "/usr/local/bin/av";
const HELPER_CLI_INSTALL_TARGETS: [(&str, &str); 1] = [("av", "/usr/local/bin/av")];
const ISOTOPE_ALWAYS_ALLOW_PATH: &str =
    "/Library/Application Support/Automic Vault/isotope/always-allow.json";
const DEFAULT_SEARCH_PAGE_SIZE: usize = 100;
const MAX_SEARCH_PAGE_SIZE: usize = 200;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum HelperCommand {
    Install {
        packages: Vec<PackageSpec>,
    },
    Update {
        packages: Vec<PackageSpec>,
    },
    Uninstall {
        packages: Vec<PackageSpec>,
    },
    UpdateAll,
    InstallAv {
        source_path: String,
        caller_path: String,
    },
    InstallIsotopeRoot {
        isotope_name: String,
    },
    ConvertRadioisotope {
        isotope_name: String,
    },
    InstallIsotopeStubs {
        isotope_name: String,
    },
    RememberIsotopeAlwaysAllow {
        executable_path: String,
        script_path: Option<String>,
        keys: Vec<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PackageSpec {
    pub name: String,
    pub version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ProgressEvent {
    Resolving,
    Downloading {
        package: String,
        bytes_per_sec: u64,
        progress: f32,
    },
    Installing {
        package: String,
    },
    Log {
        package: String,
        message: String,
    },
    Completed {
        package: String,
    },
    Error {
        message: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HelperCommandSuccess {
    pub message: String,
    pub processed_packages: Vec<String>,
}

pub type HelperCommandResult = Result<HelperCommandSuccess, String>;

pub fn execute_helper_command<F>(
    command: HelperCommand,
    progress_callback: F,
) -> HelperCommandResult
where
    F: FnMut(ProgressEvent) + Send + 'static,
{
    let progress_callback = Arc::new(Mutex::new(
        Box::new(progress_callback) as Box<ProgressCallback>
    ));
    let result = match command {
        HelperCommand::Install { packages } => {
            install_packages(packages, progress_callback.clone())
        }
        HelperCommand::Update { packages } => update_packages(packages, progress_callback.clone()),
        HelperCommand::Uninstall { packages } => {
            uninstall_packages(packages, progress_callback.clone())
        }
        HelperCommand::UpdateAll => update_all_packages(progress_callback.clone()),
        HelperCommand::InstallAv {
            source_path,
            caller_path,
        } => install_cli_tools(&source_path, &caller_path, progress_callback.clone()),
        HelperCommand::InstallIsotopeRoot { isotope_name } => {
            install_isotope_root_with_helper(&isotope_name, progress_callback.clone())
        }
        HelperCommand::ConvertRadioisotope { isotope_name } => {
            convert_radioisotope_with_helper(&isotope_name, progress_callback.clone())
        }
        HelperCommand::InstallIsotopeStubs { isotope_name } => {
            install_isotope_stubs_with_helper(&isotope_name, progress_callback.clone())
        }
        HelperCommand::RememberIsotopeAlwaysAllow {
            executable_path,
            script_path,
            keys,
        } => remember_isotope_always_allow(&executable_path, script_path.as_deref(), keys),
    };
    if let Err(err) = &result {
        if let Ok(mut callback) = progress_callback.lock() {
            callback(ProgressEvent::Error {
                message: err.clone(),
            });
        }
    }
    result
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct IsotopeAlwaysAllowEntry {
    executable_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    script_path: Option<String>,
    keys: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
struct IsotopeAlwaysAllowStore {
    entries: Vec<IsotopeAlwaysAllowEntry>,
}

pub fn check_for_updates() -> Result<bool, String> {
    let config = load_config()?;
    Ok(!resolve_outdated_package_statuses(&config, &PackageSelection::AllInstalled)?.is_empty())
}

pub(crate) fn list_installed_packages() -> Result<core::ListInstalledResponse, String> {
    let mut packages = state::list_installed_package_refs()?;
    packages.sort_by(|left, right| {
        compare_package_names_for_search_order(&left.package_name, &right.package_name)
    });
    packages.dedup_by(|left, right| left.package_name == right.package_name);

    let mut results = Vec::with_capacity(packages.len());
    for package in packages {
        let receipt =
            state::load_installed_package_receipt(&package.package_name, &package.install_root)?;
        let qualified_name = package_source_qualified_name(&receipt.source);
        let security_state =
            package_security_state_for_identifiers([receipt.package_name.clone(), qualified_name]);
        results.push(core::InstalledPackageSummary {
            name: receipt.package_name,
            source: receipt.source,
            version: receipt.version,
            description: receipt.metadata.description,
            security_state,
        });
    }

    Ok(core::ListInstalledResponse { packages: results })
}

pub(crate) fn list_available_packages(
    offset: usize,
    limit: usize,
) -> Result<core::SearchPackagesResponse, String> {
    let packages = resolve_available_package_results(&Config {
        bottle_tag: String::new(),
    })?;
    let limit = search_page_size(limit);
    let total_count = packages.len();
    let next_offset = packages.get(offset + limit).map(|_| offset + limit);
    let packages = packages
        .into_iter()
        .skip(offset)
        .take(limit)
        .map(search_package_summary)
        .collect();
    Ok(core::SearchPackagesResponse {
        packages,
        total_count,
        next_offset,
    })
}

pub(crate) fn list_pulse_packages(
    offset: usize,
    limit: usize,
) -> Result<core::SearchPackagesResponse, String> {
    let packages = resolve_pulse_package_results(&Config {
        bottle_tag: String::new(),
    })?;
    let limit = search_page_size(limit);
    let total_count = packages.len();
    let next_offset = packages.get(offset + limit).map(|_| offset + limit);
    let packages = packages
        .into_iter()
        .skip(offset)
        .take(limit)
        .map(search_package_summary)
        .collect();
    Ok(core::SearchPackagesResponse {
        packages,
        total_count,
        next_offset,
    })
}

pub(crate) fn search_packages(
    query: &str,
    offset: usize,
    limit: usize,
) -> Result<core::SearchPackagesResponse, String> {
    let packages = resolve_package_search_results(
        &Config {
            bottle_tag: String::new(),
        },
        query,
    )?;
    let limit = search_page_size(limit);
    let total_count = packages.len();
    let next_offset = packages.get(offset + limit).map(|_| offset + limit);
    let packages = packages
        .into_iter()
        .skip(offset)
        .take(limit)
        .map(search_package_summary)
        .collect();
    Ok(core::SearchPackagesResponse {
        packages,
        total_count,
        next_offset,
    })
}

fn search_page_size(limit: usize) -> usize {
    match limit {
        0 => DEFAULT_SEARCH_PAGE_SIZE,
        _ => limit.min(MAX_SEARCH_PAGE_SIZE),
    }
}

fn search_package_summary(package: PackageSearchResult) -> core::SearchPackageSummary {
    core::SearchPackageSummary {
        name: package.package_name,
        source: package.source,
        version: package.latest_version,
        description: package.summary,
    }
}

pub(crate) fn package_info(package: &str) -> Result<PackageInfo, String> {
    let config = load_config()?;
    let requested = cli::parse_package_name(&OsString::from(package))?;
    resolve_package_info(&config, &requested)
}

pub(crate) fn list_outdated_packages() -> Result<core::ListOutdatedResponse, String> {
    let config = load_config()?;
    let packages = resolve_scanned_package_statuses(
        state::list_installed_package_refs()?,
        |package| resolve_package_status_at(&config, &package.package_name, &package.install_root),
        |_| {},
    )?
    .into_iter()
    .filter(PackageStatus::is_outdated)
    .map(|package| core::OutdatedPackageSummary {
        name: package.package_name,
        current_version: package.installed_version,
        latest_version: package.latest_version,
    })
    .collect();

    Ok(core::ListOutdatedResponse { packages })
}

pub(crate) fn system_info() -> core::SystemInfoResponse {
    core::SystemInfoResponse {
        version: env!("CARGO_PKG_VERSION"),
        protocol_version: core::PROTOCOL_VERSION,
        build_id: env!("NUKE_BUILD_ID"),
    }
}

pub(crate) fn isotope_migration_plan(
    isotope_name: &str,
) -> Result<core::IsotopeMigrationPlanResponse, String> {
    let isotope_name = normalized_isotope_name(isotope_name)?;
    let record = isotope_package_data(&isotope_name)?;
    Ok(core::IsotopeMigrationPlanResponse {
        isotope_name,
        replaces_package: isotope_replaced_package_name(record)?,
        modifies_package: isotope_modified_package_name(record)?,
        is_radioisotope: isotope_has_post_install(&record.name),
        has_migration: record.migrate.is_some() || isotope_has_migration(&record.name),
    })
}

pub(crate) fn migrate_isotope(
    isotope_name: &str,
) -> Result<core::IsotopeMigrationPlanResponse, String> {
    let isotope_name = normalized_isotope_name(isotope_name)?;
    let record = isotope_package_data(&isotope_name)?;
    let plan = InstallPlan::for_i_isotope(isotope_qualified_name(&isotope_name), &isotope_name);
    run_isotope_migration(&plan, record, None)?;
    isotope_migration_plan(&isotope_name)
}

fn normalized_isotope_name(value: &str) -> Result<String, String> {
    let name = value.strip_prefix(ISOTOPE_PACKAGE_PREFIX).unwrap_or(value);
    if name.is_empty() {
        return Err("missing isotope name".to_string());
    }
    if name.contains('/') {
        return Err(format!("invalid isotope name: {name}"));
    }
    Ok(name.to_string())
}

fn install_packages(
    packages: Vec<PackageSpec>,
    progress_callback: Arc<Mutex<Box<ProgressCallback>>>,
) -> HelperCommandResult {
    require_root()?;
    let requested = validate_install_specs(packages)?;
    let processed_packages = requested
        .iter()
        .map(requested_package_name)
        .collect::<Vec<_>>();

    let _lock = acquire_package_mutation_lock()?;
    let config = load_config()?;
    for package in requested {
        run_i_package_with_progress(
            &config,
            package,
            InstallOptions {
                allow_reinstall: false,
            },
            Some(progress_callback.clone()),
        )?;
    }

    Ok(HelperCommandSuccess {
        message: "Install complete".to_string(),
        processed_packages,
    })
}

fn uninstall_packages(
    packages: Vec<PackageSpec>,
    progress_callback: Arc<Mutex<Box<ProgressCallback>>>,
) -> HelperCommandResult {
    require_root()?;
    let package_names = validate_uninstall_specs(packages)?;
    let _lock = acquire_package_mutation_lock()?;
    for package_name in &package_names {
        if let Ok(mut callback) = progress_callback.lock() {
            callback(ProgressEvent::Installing {
                package: package_name.clone(),
            });
        }
        ensure_package_installed(&opt_pkg_root(), package_name)?;
        uninstall_package(package_name)?;
        if let Ok(mut callback) = progress_callback.lock() {
            callback(ProgressEvent::Completed {
                package: package_name.clone(),
            });
        }
    }

    Ok(HelperCommandSuccess {
        message: "Uninstall complete".to_string(),
        processed_packages: package_names,
    })
}

fn update_packages(
    packages: Vec<PackageSpec>,
    progress_callback: Arc<Mutex<Box<ProgressCallback>>>,
) -> HelperCommandResult {
    require_root()?;
    let requested = validate_install_specs(packages)?;
    let processed_packages = requested
        .iter()
        .map(requested_package_name)
        .collect::<Vec<_>>();

    let _lock = acquire_package_mutation_lock()?;
    let config = load_config()?;
    for package in requested {
        run_i_package_with_progress(
            &config,
            package,
            InstallOptions {
                allow_reinstall: true,
            },
            Some(progress_callback.clone()),
        )?;
    }

    Ok(HelperCommandSuccess {
        message: "Update complete".to_string(),
        processed_packages,
    })
}

fn update_all_packages(
    progress_callback: Arc<Mutex<Box<ProgressCallback>>>,
) -> HelperCommandResult {
    require_root()?;
    let _lock = acquire_package_mutation_lock()?;
    let config = load_config()?;
    let outdated = resolve_outdated_package_statuses(&config, &PackageSelection::AllInstalled)?;
    let processed_packages = outdated
        .iter()
        .map(|package| package.package_name.clone())
        .collect::<Vec<_>>();

    for package in outdated {
        run_i_package_with_progress(
            &config,
            requested_package_from_status(&package),
            InstallOptions {
                allow_reinstall: true,
            },
            Some(progress_callback.clone()),
        )?;
    }

    Ok(HelperCommandSuccess {
        message: if processed_packages.is_empty() {
            "System already current".to_string()
        } else {
            "Update complete".to_string()
        },
        processed_packages,
    })
}

fn install_isotope_stubs_with_helper(
    isotope_name: &str,
    progress_callback: Arc<Mutex<Box<ProgressCallback>>>,
) -> HelperCommandResult {
    require_root()?;
    let isotope_name = normalized_isotope_name(isotope_name)?;
    let package_name = isotope_qualified_name(&isotope_name);
    let _lock = acquire_package_mutation_lock()?;
    install_isotope_stubs(&isotope_name, Some(progress_callback))?;
    Ok(HelperCommandSuccess {
        message: "Isotope stubs installed".to_string(),
        processed_packages: vec![package_name],
    })
}

fn install_isotope_root_with_helper(
    isotope_name: &str,
    progress_callback: Arc<Mutex<Box<ProgressCallback>>>,
) -> HelperCommandResult {
    require_root()?;
    let isotope_name = normalized_isotope_name(isotope_name)?;
    let package_name = isotope_qualified_name(&isotope_name);
    let _lock = acquire_package_mutation_lock()?;
    let config = load_config()?;
    run_i_isotope_root_only(
        &config,
        package_name.clone(),
        isotope_name,
        Some(progress_callback),
    )?;
    Ok(HelperCommandSuccess {
        message: "Isotope root installed".to_string(),
        processed_packages: vec![package_name],
    })
}

fn convert_radioisotope_with_helper(
    isotope_name: &str,
    progress_callback: Arc<Mutex<Box<ProgressCallback>>>,
) -> HelperCommandResult {
    require_root()?;
    let isotope_name = normalized_isotope_name(isotope_name)?;
    let package_name = isotope_qualified_name(&isotope_name);
    let _lock = acquire_package_mutation_lock()?;
    let config = load_config()?;
    run_i_radioisotope(
        &config,
        package_name.clone(),
        isotope_name,
        false,
        Some(progress_callback),
    )?;
    Ok(HelperCommandSuccess {
        message: "Isotope conversion complete".to_string(),
        processed_packages: vec![package_name],
    })
}

fn install_cli_tools(
    source_path: &str,
    caller_path: &str,
    progress_callback: Arc<Mutex<Box<ProgressCallback>>>,
) -> HelperCommandResult {
    require_root()?;

    let installs = cli_tool_installs_for_source(Path::new(source_path));
    verify_cli_install_signatures(
        &installs
            .iter()
            .map(|(_, source_path, _)| source_path.as_path())
            .collect::<Vec<_>>(),
        Path::new(caller_path),
    )?;
    let mut processed_packages = Vec::with_capacity(installs.len());

    for (tool_name, source_path, target_path) in installs {
        if let Ok(mut callback) = progress_callback.lock() {
            callback(ProgressEvent::Installing {
                package: tool_name.to_string(),
            });
        }

        install_binary_at(&source_path, &target_path, tool_name)?;
        processed_packages.push(tool_name.to_string());

        if let Ok(mut callback) = progress_callback.lock() {
            callback(ProgressEvent::Completed {
                package: tool_name.to_string(),
            });
        }
    }

    Ok(HelperCommandSuccess {
        message: "Automic Vault command line tools installed to /usr/local/bin".to_string(),
        processed_packages,
    })
}

fn cli_tool_installs_for_source(source_path: &Path) -> Vec<(&'static str, PathBuf, PathBuf)> {
    if source_path.is_dir() {
        return HELPER_CLI_INSTALL_TARGETS
            .iter()
            .map(|(tool_name, target_path)| {
                (
                    *tool_name,
                    source_path.join(tool_name),
                    PathBuf::from(target_path),
                )
            })
            .collect();
    }

    vec![(
        PKG_DISPLAY_NAME,
        source_path.to_path_buf(),
        PathBuf::from(HELPER_AV_INSTALL_TARGET),
    )]
}

pub fn verify_helper_codesign_identity() -> Result<(), String> {
    verify_expected_codesign_identity(
        &std::env::current_exe()
            .map_err(|err| format!("failed to resolve helper executable path: {err}"))?,
    )
}

#[cfg(target_os = "macos")]
fn verify_cli_install_signatures(source_paths: &[&Path], caller_path: &Path) -> Result<(), String> {
    if expected_codesign_identity().is_none() {
        return Ok(());
    }
    if source_paths.is_empty() {
        return Err("no staged command line tools to install".to_string());
    }
    if caller_path.as_os_str().is_empty() {
        return Err("unable to identify the GUI app requesting CLI installation".to_string());
    }

    let helper_path = std::env::current_exe()
        .map_err(|err| format!("failed to resolve helper executable path: {err}"))?;
    let helper_signature = code_signature_authorities(&helper_path)?;
    ensure_expected_codesign_identity("helper", &helper_path, &helper_signature)?;

    let caller_signature = code_signature_authorities(caller_path)?;
    if caller_signature != helper_signature {
        return Err(format!(
            "GUI app signature does not match helper signature: {}",
            caller_path.display()
        ));
    }

    for source_path in source_paths {
        let source_signature = code_signature_authorities(source_path)?;
        ensure_expected_codesign_identity("staged av", source_path, &source_signature)?;
        if source_signature != caller_signature {
            return Err(format!(
                "staged av signature does not match GUI app and helper: {}",
                source_path.display()
            ));
        }
    }

    Ok(())
}

#[cfg(not(target_os = "macos"))]
fn verify_cli_install_signatures(
    _source_paths: &[&Path],
    _caller_path: &Path,
) -> Result<(), String> {
    Ok(())
}

#[cfg(target_os = "macos")]
fn verify_expected_codesign_identity(path: &Path) -> Result<(), String> {
    if expected_codesign_identity().is_none() {
        return Ok(());
    }
    let signature = code_signature_authorities(path)?;
    ensure_expected_codesign_identity("helper", path, &signature)
}

#[cfg(not(target_os = "macos"))]
fn verify_expected_codesign_identity(_path: &Path) -> Result<(), String> {
    Ok(())
}

#[cfg(target_os = "macos")]
fn ensure_expected_codesign_identity(
    label: &str,
    path: &Path,
    authorities: &[String],
) -> Result<(), String> {
    let Some(expected) = expected_codesign_identity() else {
        return Ok(());
    };

    match authorities.first() {
        Some(actual) if actual == expected => Ok(()),
        Some(actual) => Err(format!(
            "{label} signature identity mismatch for {}: expected {expected}, got {actual}",
            path.display()
        )),
        None => Err(format!(
            "{label} is not signed with expected identity {expected}: {}",
            path.display()
        )),
    }
}

#[cfg(target_os = "macos")]
fn expected_codesign_identity() -> Option<&'static str> {
    let expected = env!("NUKE_CODESIGN_IDENTITY").trim();
    (!expected.is_empty() && expected != "-").then_some(expected)
}

#[cfg(target_os = "macos")]
fn code_signature_authorities(path: &Path) -> Result<Vec<String>, String> {
    let output = Command::new("/usr/bin/codesign")
        .args(["-dv", "--verbose=4"])
        .arg(path)
        .output()
        .map_err(|err| format!("failed to run codesign for {}: {err}", path.display()))?;
    if !output.status.success() {
        let stderr_lines = String::from_utf8_lossy(&output.stderr)
            .lines()
            .map(str::to_string)
            .collect::<Vec<_>>();
        return Err(format!(
            "failed to inspect code signature for {}{}",
            path.display(),
            format_command_output_suffix(&stderr_lines)
        ));
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    let authorities = stderr
        .lines()
        .filter_map(|line| line.strip_prefix("Authority="))
        .map(str::to_string)
        .collect::<Vec<_>>();
    if authorities.is_empty() {
        return Err(format!(
            "code signature for {} has no signing authority",
            path.display()
        ));
    }
    Ok(authorities)
}

fn install_binary_at(
    source_path: &Path,
    target_path: &Path,
    tool_name: &str,
) -> Result<(), String> {
    let source_metadata = fs::metadata(source_path)
        .map_err(|err| format!("failed to stat {}: {err}", source_path.display()))?;
    if !source_metadata.is_file() {
        return Err(format!(
            "staged {tool_name} binary is not a file: {}",
            source_path.display()
        ));
    }

    let target_dir = target_path.parent().ok_or_else(|| {
        format!(
            "invalid {tool_name} install target {}",
            target_path.display()
        )
    })?;
    fs::create_dir_all(target_dir)
        .map_err(|err| format!("failed to create {}: {err}", target_dir.display()))?;

    let temp_dir = TempDir::new_in(target_dir).map_err(|err| {
        format!(
            "failed to create temp dir in {}: {err}",
            target_dir.display()
        )
    })?;
    let staged_target = temp_dir.path().join(tool_name);
    fs::copy(source_path, &staged_target).map_err(|err| {
        format!(
            "failed to copy {} to {}: {err}",
            source_path.display(),
            staged_target.display()
        )
    })?;

    let mut permissions = fs::metadata(&staged_target)
        .map_err(|err| format!("failed to stat {}: {err}", staged_target.display()))?
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&staged_target, permissions)
        .map_err(|err| format!("failed to chmod {}: {err}", staged_target.display()))?;

    fs::rename(&staged_target, target_path).map_err(|err| {
        format!(
            "failed to install {tool_name} at {}: {err}",
            target_path.display()
        )
    })?;

    Ok(())
}

fn remember_isotope_always_allow(
    executable_path: &str,
    script_path: Option<&str>,
    mut keys: Vec<String>,
) -> HelperCommandResult {
    require_root()?;
    let executable_path = validate_isotope_always_allow_target(executable_path)?;
    let script_path = validate_isotope_always_allow_script(&executable_path, script_path)?;
    validate_isotope_keys(&keys)?;
    keys.sort();
    keys.dedup();

    let path = Path::new(ISOTOPE_ALWAYS_ALLOW_PATH);
    let mut store = load_isotope_always_allow_store(path)?;
    if !store.entries.iter().any(|entry| {
        entry.executable_path == executable_path
            && entry.script_path == script_path
            && entry.keys == keys
    }) {
        store.entries.push(IsotopeAlwaysAllowEntry {
            executable_path: executable_path.clone(),
            script_path: script_path.clone(),
            keys,
        });
        store.entries.sort_by(|left, right| {
            left.executable_path
                .cmp(&right.executable_path)
                .then_with(|| left.script_path.cmp(&right.script_path))
                .then_with(|| left.keys.cmp(&right.keys))
        });
        write_isotope_always_allow_store(path, &store)?;
    }

    Ok(HelperCommandSuccess {
        message: "Isotope always-allow remembered".to_string(),
        processed_packages: Vec::new(),
    })
}

fn validate_isotope_always_allow_target(executable_path: &str) -> Result<String, String> {
    let path = fs::canonicalize(executable_path)
        .map_err(|err| format!("failed to resolve isotope target {executable_path}: {err}"))?;
    let metadata = fs::metadata(&path)
        .map_err(|err| format!("failed to stat isotope target {}: {err}", path.display()))?;
    if !metadata.is_file() {
        return Err("isotope target must be a regular file".to_string());
    }
    if metadata.uid() != 0 {
        return Err("isotope target must be owned by root".to_string());
    }
    if metadata.mode() & ((libc::S_IWGRP | libc::S_IWOTH) as u32) != 0 {
        return Err("isotope target must not be writable by group or others".to_string());
    }
    for directory in path.ancestors().skip(1) {
        let metadata = fs::metadata(directory)
            .map_err(|err| format!("failed to stat {}: {err}", directory.display()))?;
        if metadata.mode() & ((libc::S_IWGRP | libc::S_IWOTH) as u32) != 0 {
            return Err(format!(
                "isotope target directory must not be writable by group or others: {}",
                directory.display()
            ));
        }
    }
    path.to_str()
        .map(str::to_string)
        .ok_or_else(|| "isotope target path must be valid UTF-8".to_string())
}

fn validate_isotope_always_allow_script(
    executable_path: &str,
    script_path: Option<&str>,
) -> Result<Option<String>, String> {
    let is_interpreter = is_isotope_script_interpreter(Path::new(executable_path));
    match (is_interpreter, script_path.filter(|path| !path.is_empty())) {
        (true, Some(script_path)) => validate_isotope_always_allow_target(script_path).map(Some),
        (true, None) => Err("isotope interpreter target requires a script path".to_string()),
        (false, Some(_)) => Err("isotope script path requires an interpreter target".to_string()),
        (false, None) => Ok(None),
    }
}

fn is_isotope_script_interpreter(path: &Path) -> bool {
    let Some(file_name) = path.file_name().and_then(|value| value.to_str()) else {
        return false;
    };
    matches!(
        file_name,
        "bash"
            | "dash"
            | "env"
            | "ksh"
            | "node"
            | "osascript"
            | "perl"
            | "python"
            | "python3"
            | "ruby"
            | "sh"
            | "zsh"
    ) || is_versioned_python_name(file_name)
}

fn is_versioned_python_name(file_name: &str) -> bool {
    let Some(suffix) = file_name.strip_prefix("python") else {
        return false;
    };
    !suffix.is_empty() && suffix.chars().all(|ch| ch == '.' || ch.is_ascii_digit())
}

fn validate_isotope_keys(keys: &[String]) -> Result<(), String> {
    if keys.is_empty() {
        return Err("at least one isotope key is required".to_string());
    }
    for key in keys {
        let mut chars = key.chars();
        let Some(first) = chars.next() else {
            return Err("empty isotope key name".to_string());
        };
        if !(first == '_' || first.is_ascii_alphabetic())
            || chars.any(|ch| !(ch == '_' || ch.is_ascii_alphanumeric()))
        {
            return Err(format!("invalid isotope key name: {key}"));
        }
    }
    Ok(())
}

fn load_isotope_always_allow_store(path: &Path) -> Result<IsotopeAlwaysAllowStore, String> {
    if !path.exists() {
        return Ok(IsotopeAlwaysAllowStore::default());
    }
    let contents = fs::read_to_string(path)
        .map_err(|err| format!("failed to read {}: {err}", path.display()))?;
    serde_json::from_str(&contents)
        .map_err(|err| format!("failed to decode {}: {err}", path.display()))
}

fn write_isotope_always_allow_store(
    path: &Path,
    store: &IsotopeAlwaysAllowStore,
) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| format!("invalid isotope always-allow path {}", path.display()))?;
    fs::create_dir_all(parent)
        .map_err(|err| format!("failed to create {}: {err}", parent.display()))?;
    let temp_dir = TempDir::new_in(parent)
        .map_err(|err| format!("failed to create temp dir in {}: {err}", parent.display()))?;
    let temp_path = temp_dir.path().join("always-allow.json");
    let payload = serde_json::to_vec_pretty(store)
        .map_err(|err| format!("failed to encode isotope always-allow store: {err}"))?;
    fs::write(&temp_path, payload)
        .map_err(|err| format!("failed to write {}: {err}", temp_path.display()))?;
    fs::set_permissions(&temp_path, fs::Permissions::from_mode(0o644))
        .map_err(|err| format!("failed to chmod {}: {err}", temp_path.display()))?;
    fs::rename(&temp_path, path)
        .map_err(|err| format!("failed to install {}: {err}", path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn install_binary_at_copies_binary_and_sets_mode() {
        let temp = TempDir::new().unwrap();
        let source = temp.path().join("source-av");
        let target_dir = temp.path().join("usr/local/bin");
        let target = target_dir.join("av");

        fs::write(&source, "#!/bin/sh\necho av\n").unwrap();
        let mut source_permissions = fs::metadata(&source).unwrap().permissions();
        source_permissions.set_mode(0o700);
        fs::set_permissions(&source, source_permissions).unwrap();

        install_binary_at(&source, &target, "av").unwrap();

        assert_eq!(fs::read_to_string(&target).unwrap(), "#!/bin/sh\necho av\n");
        assert_eq!(
            fs::metadata(&target).unwrap().permissions().mode() & 0o777,
            0o755
        );
    }

    #[test]
    fn install_binary_at_reports_invalid_sources_and_targets() {
        let temp = TempDir::new().unwrap();
        let source_dir = temp.path().join("staged-dir");
        fs::create_dir_all(&source_dir).unwrap();
        assert!(
            install_binary_at(&source_dir, &temp.path().join("av"), "av")
                .unwrap_err()
                .contains("is not a file")
        );

        let missing = temp.path().join("missing-av");
        assert!(
            install_binary_at(&missing, &temp.path().join("av"), "av")
                .unwrap_err()
                .contains("failed to stat")
        );
    }

    #[test]
    fn cli_tool_installs_for_source_expands_staging_directory() {
        let temp = TempDir::new().unwrap();
        let installs = cli_tool_installs_for_source(temp.path());

        assert_eq!(
            installs,
            vec![(
                "av",
                temp.path().join("av"),
                PathBuf::from("/usr/local/bin/av")
            )]
        );

        let file = temp.path().join("av");
        fs::write(&file, b"av").unwrap();
        assert_eq!(
            cli_tool_installs_for_source(&file),
            vec![(
                PKG_DISPLAY_NAME,
                file,
                PathBuf::from(HELPER_AV_INSTALL_TARGET)
            )]
        );
    }

    #[test]
    fn isotope_always_allow_store_uses_script_path_shape() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("always-allow.json");
        let store = IsotopeAlwaysAllowStore {
            entries: vec![IsotopeAlwaysAllowEntry {
                executable_path: "/opt/awscli/bin/python3.14".to_string(),
                script_path: Some("/opt/awscli/bin/aws".to_string()),
                keys: vec![
                    "AWS_ACCESS_KEY_ID".to_string(),
                    "AWS_SECRET_ACCESS_KEY".to_string(),
                ],
            }],
        };

        write_isotope_always_allow_store(&path, &store).unwrap();
        let reloaded = load_isotope_always_allow_store(&path).unwrap();

        assert_eq!(reloaded, store);
        let encoded = fs::read_to_string(path).unwrap();
        assert!(encoded.contains("\"script_path\""));
    }

    #[test]
    fn isotope_interpreter_detection_accepts_versioned_python() {
        assert!(is_isotope_script_interpreter(Path::new(
            "/opt/awscli/bin/python3.14"
        )));
        assert!(is_isotope_script_interpreter(Path::new("/bin/python3")));
        assert!(!is_isotope_script_interpreter(Path::new(
            "/bin/python-config"
        )));
    }

    #[test]
    fn helper_command_errors_emit_progress_error() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let captured = events.clone();
        let result = execute_helper_command(
            HelperCommand::Install {
                packages: Vec::new(),
            },
            move |event| captured.lock().unwrap().push(event),
        );

        assert!(result.is_err());
        assert!(matches!(
            events.lock().unwrap().last(),
            Some(ProgressEvent::Error { .. })
        ));
    }

    #[test]
    fn helper_command_routes_isotope_and_always_allow_errors() {
        for command in [
            HelperCommand::Update {
                packages: vec![PackageSpec {
                    name: "rg".to_string(),
                    version: None,
                }],
            },
            HelperCommand::Uninstall {
                packages: vec![PackageSpec {
                    name: "rg".to_string(),
                    version: None,
                }],
            },
            HelperCommand::UpdateAll,
            HelperCommand::InstallAv {
                source_path: "/tmp/av".to_string(),
                caller_path: "/tmp/Automic Vault.app".to_string(),
            },
            HelperCommand::InstallIsotopeRoot {
                isotope_name: String::new(),
            },
            HelperCommand::ConvertRadioisotope {
                isotope_name: "bad/name".to_string(),
            },
            HelperCommand::InstallIsotopeStubs {
                isotope_name: String::new(),
            },
            HelperCommand::RememberIsotopeAlwaysAllow {
                executable_path: String::new(),
                script_path: None,
                keys: Vec::new(),
            },
        ] {
            let result = execute_helper_command(command, |_| {});
            assert!(result.is_err());
        }
    }

    #[test]
    fn package_search_wrappers_cover_pagination_edges() {
        let empty = search_packages("", 0, 10).unwrap();
        assert_eq!(empty.total_count, 0);
        assert!(empty.packages.is_empty());
        assert_eq!(empty.next_offset, None);

        let default_page = list_available_packages(0, 0).unwrap();
        assert!(default_page.total_count >= default_page.packages.len());
        assert!(default_page.packages.len() <= DEFAULT_SEARCH_PAGE_SIZE);

        let capped_page = list_available_packages(0, usize::MAX).unwrap();
        assert!(capped_page.packages.len() <= MAX_SEARCH_PAGE_SIZE);

        let past_end = search_packages("rg", usize::MAX / 2, 1).unwrap();
        assert!(past_end.packages.is_empty());
        assert_eq!(past_end.next_offset, None);

        let pulse = list_pulse_packages(0, 1).unwrap();
        assert_eq!(pulse.packages.len(), 1);
        assert!(pulse.next_offset.is_some());
    }

    #[test]
    fn validation_helpers_cover_limits_versions_and_isotope_names() {
        assert_eq!(search_page_size(0), DEFAULT_SEARCH_PAGE_SIZE);
        assert_eq!(search_page_size(1), 1);
        assert_eq!(search_page_size(usize::MAX), MAX_SEARCH_PAGE_SIZE);

        assert_eq!(normalized_isotope_name("isotope:gh").unwrap(), "gh");
        assert_eq!(normalized_isotope_name("aws-cli").unwrap(), "aws-cli");
        assert!(normalized_isotope_name("").unwrap_err().contains("missing"));
        assert!(
            normalized_isotope_name("bad/name")
                .unwrap_err()
                .contains("invalid")
        );

        assert_eq!(
            validate_optional_version(Some(" 1.2.3 ")).unwrap(),
            Some("1.2.3".to_string())
        );
        assert!(validate_optional_version(Some(" ")).is_err());
        assert!(validate_optional_version(Some("1.2.3 beta")).is_err());

        assert!(validate_install_specs(Vec::new()).is_err());
        assert!(
            validate_install_specs(vec![PackageSpec {
                name: "npm:openclaw".to_string(),
                version: Some("4.5.6".to_string()),
            }])
            .is_ok()
        );
        assert!(
            validate_install_specs(vec![
                PackageSpec {
                    name: "cask:cursor".to_string(),
                    version: None,
                },
                PackageSpec {
                    name: "isotope:gh".to_string(),
                    version: None,
                },
                PackageSpec {
                    name: "pip:My_Package.Name".to_string(),
                    version: None,
                },
                PackageSpec {
                    name: "rg".to_string(),
                    version: None,
                },
            ])
            .is_ok()
        );
        assert!(
            validate_install_specs(vec![PackageSpec {
                name: "brew:sqlite".to_string(),
                version: Some("3".to_string()),
            }])
            .unwrap_err()
            .contains("does not support explicit version")
        );
        assert!(
            validate_uninstall_specs(vec![PackageSpec {
                name: "npm:openclaw".to_string(),
                version: Some("4.5.6".to_string()),
            }])
            .unwrap_err()
            .contains("cannot specify a version")
        );
        let too_many = (0..=MAX_HELPER_PACKAGES)
            .map(|index| PackageSpec {
                name: format!("pkg-{index}"),
                version: None,
            })
            .collect();
        assert!(
            validate_install_specs(too_many)
                .unwrap_err()
                .contains("at most")
        );
        assert_eq!(
            validate_uninstall_specs(vec![
                PackageSpec {
                    name: "brew:ripgrep".to_string(),
                    version: None,
                },
                PackageSpec {
                    name: "cask:cursor".to_string(),
                    version: None,
                },
                PackageSpec {
                    name: "isotope:gh".to_string(),
                    version: None,
                },
                PackageSpec {
                    name: "pip:My_Package.Name".to_string(),
                    version: None,
                },
            ])
            .unwrap(),
            vec![
                "ripgrep".to_string(),
                "cursor".to_string(),
                "isotope:gh".to_string(),
                "pip:my-package-name".to_string()
            ]
        );
    }

    #[test]
    fn isotope_always_allow_validation_rejects_bad_inputs() {
        assert!(
            validate_isotope_keys(&[])
                .unwrap_err()
                .contains("at least one")
        );
        assert!(
            validate_isotope_keys(&["".to_string()])
                .unwrap_err()
                .contains("empty")
        );
        assert!(
            validate_isotope_keys(&["1BAD".to_string()])
                .unwrap_err()
                .contains("invalid")
        );
        assert!(validate_isotope_keys(&["GOOD_1".to_string()]).is_ok());

        assert!(
            validate_isotope_always_allow_script("/bin/sh", None)
                .unwrap_err()
                .contains("requires a script path")
        );
        assert!(
            validate_isotope_always_allow_script("/bin/echo", Some("/tmp/script"))
                .unwrap_err()
                .contains("requires an interpreter target")
        );
        assert_eq!(
            validate_isotope_always_allow_script("/bin/echo", None).unwrap(),
            None
        );
    }

    #[test]
    fn always_allow_store_reports_decode_errors() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("always-allow.json");
        fs::write(&path, b"{not json").unwrap();

        assert!(
            load_isotope_always_allow_store(&path)
                .unwrap_err()
                .contains("failed to decode")
        );
        assert_eq!(
            load_isotope_always_allow_store(&temp.path().join("missing.json")).unwrap(),
            IsotopeAlwaysAllowStore::default()
        );
    }
}

fn require_root() -> Result<(), String> {
    if is_root() {
        return Ok(());
    }
    Err("must be run as root".to_string())
}

fn validate_install_specs(packages: Vec<PackageSpec>) -> Result<Vec<RequestedPackage>, String> {
    validate_request_count(&packages)?;
    let mut requested = Vec::with_capacity(packages.len());
    for package in packages {
        requested.push(requested_package_from_spec(&package)?);
    }
    Ok(requested)
}

fn validate_uninstall_specs(packages: Vec<PackageSpec>) -> Result<Vec<String>, String> {
    validate_request_count(&packages)?;
    let mut package_names = Vec::with_capacity(packages.len());
    for package in packages {
        if package.version.is_some() {
            return Err(format!(
                "package {} cannot specify a version for uninstall",
                package.name
            ));
        }
        package_names.push(cli::parse_uninstall_package_name(&OsString::from(
            package.name,
        ))?);
    }
    Ok(package_names)
}

fn validate_request_count(packages: &[PackageSpec]) -> Result<(), String> {
    if packages.is_empty() {
        return Err("at least one package is required".to_string());
    }
    if packages.len() > MAX_HELPER_PACKAGES {
        return Err(format!(
            "at most {MAX_HELPER_PACKAGES} packages are allowed per request"
        ));
    }
    Ok(())
}

fn requested_package_from_spec(package: &PackageSpec) -> Result<RequestedPackage, String> {
    let requested = cli::parse_package_name(&OsString::from(package.name.clone()))?;
    match requested {
        RequestedPackage::NpmPackage {
            package: npm_package,
            version: _,
        } => Ok(RequestedPackage::NpmPackage {
            package: npm_package,
            version: validate_optional_version(package.version.as_deref())?,
        }),
        RequestedPackage::Auto(_)
        | RequestedPackage::Alias { .. }
        | RequestedPackage::HomebrewFormula(_)
        | RequestedPackage::HomebrewCask(_)
        | RequestedPackage::Isotope(_)
        | RequestedPackage::PipPackage(_) => {
            if package.version.is_some() {
                return Err(format!(
                    "package {} does not support explicit version selection",
                    package.name
                ));
            }
            Ok(requested)
        }
    }
}

fn validate_optional_version(version: Option<&str>) -> Result<Option<String>, String> {
    let Some(version) = version else {
        return Ok(None);
    };
    let trimmed = version.trim();
    if trimmed.is_empty() {
        return Err("version must not be empty".to_string());
    }
    if trimmed
        .chars()
        .any(|ch| ch.is_control() || ch.is_whitespace())
    {
        return Err("version must not contain whitespace or control characters".to_string());
    }
    Ok(Some(trimmed.to_string()))
}
