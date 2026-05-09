use super::*;

pub(crate) const INFO_WIDTH: usize = 64;
pub(crate) const INFO_INNER_WIDTH: usize = INFO_WIDTH - 2;
pub(crate) const INFO_LABEL_WIDTH: usize = 14;

pub(crate) fn load_config() -> Result<Config, String> {
    let bottle_tag = current_bottle_tag()?;
    Ok(Config { bottle_tag })
}

impl PackageStatus {
    pub(crate) fn is_outdated(&self) -> bool {
        self.installed_version != self.latest_version
    }
}

pub(crate) fn compare_package_names_for_search_order(
    left: &str,
    right: &str,
) -> std::cmp::Ordering {
    package_search_order_name(left)
        .cmp(package_search_order_name(right))
        .then_with(|| left.cmp(right))
}

fn package_search_order_name(package_name: &str) -> &str {
    for prefix in [
        BREW_PACKAGE_PREFIX,
        CASK_PACKAGE_PREFIX,
        ISOTOPE_PACKAGE_PREFIX,
        "av:",
        "npm:",
        "pip:",
    ] {
        if let Some(name) = package_name.strip_prefix(prefix) {
            return package_scope_order_name(name);
        }
    }
    package_scope_order_name(package_name)
}

fn package_scope_order_name(package_name: &str) -> &str {
    if let Some(scoped_name) = package_name.strip_prefix('@')
        && let Some((_, name)) = scoped_name.split_once('/')
    {
        return name;
    }
    package_name
}

pub(crate) fn resolve_package_statuses(
    config: &Config,
    selection: &PackageSelection,
) -> Result<Vec<PackageStatus>, String> {
    match selection {
        PackageSelection::AllInstalled => resolve_scanned_package_statuses(
            installed_package_refs(&opt_pkg_root())?,
            |package| {
                resolve_package_status_at(config, &package.package_name, &package.install_root)
            },
            |message| eprintln!("{message}"),
        ),
        PackageSelection::Requested(packages) => {
            let mut package_names = Vec::with_capacity(packages.len());
            for package in packages {
                package_names.push(requested_install_package_name(package)?);
            }
            package_names.sort();
            package_names.dedup();

            let mut statuses = Vec::with_capacity(package_names.len());
            for package_name in package_names {
                statuses.push(resolve_package_status(config, &package_name)?);
            }
            Ok(statuses)
        }
    }
}

pub(crate) fn resolve_installed_package_records(
    selection: &PackageSelection,
) -> Result<Vec<InstalledPackageRecord>, String> {
    match selection {
        PackageSelection::AllInstalled => resolve_scanned_package_records(
            installed_package_refs(&opt_pkg_root())?,
            |package| {
                resolve_installed_package_record_at(&package.package_name, &package.install_root)
            },
            |message| eprintln!("{message}"),
        ),
        PackageSelection::Requested(packages) => {
            let mut package_names = Vec::with_capacity(packages.len());
            for package in packages {
                package_names.push(requested_install_package_name(package)?);
            }
            package_names.sort();
            package_names.dedup();

            let mut records = Vec::with_capacity(package_names.len());
            for package_name in package_names {
                records.push(resolve_installed_package_record(&package_name)?);
            }
            Ok(records)
        }
    }
}

pub(crate) fn resolve_outdated_package_statuses(
    config: &Config,
    selection: &PackageSelection,
) -> Result<Vec<PackageStatus>, String> {
    Ok(filter_outdated_package_statuses(resolve_package_statuses(
        config, selection,
    )?))
}

pub(crate) fn filter_outdated_package_statuses(statuses: Vec<PackageStatus>) -> Vec<PackageStatus> {
    statuses
        .into_iter()
        .filter(PackageStatus::is_outdated)
        .collect()
}

pub(crate) fn resolve_scanned_package_records<Resolve, Warn>(
    mut packages: Vec<InstalledPackageRef>,
    mut resolve: Resolve,
    mut warn: Warn,
) -> Result<Vec<InstalledPackageRecord>, String>
where
    Resolve: FnMut(&InstalledPackageRef) -> Result<InstalledPackageRecord, String>,
    Warn: FnMut(String),
{
    packages.sort_by(|left, right| {
        compare_package_names_for_search_order(&left.package_name, &right.package_name)
    });
    packages.dedup_by(|left, right| left.package_name == right.package_name);

    let mut records = Vec::with_capacity(packages.len());
    for package in packages {
        match resolve(&package) {
            Ok(record) => records.push(record),
            Err(err) => warn(format!(
                "warning: skipping {}: {err}",
                package.install_root.display()
            )),
        }
    }
    Ok(records)
}

pub(crate) fn resolve_scanned_package_statuses<Resolve, Warn>(
    mut packages: Vec<InstalledPackageRef>,
    mut resolve: Resolve,
    mut warn: Warn,
) -> Result<Vec<PackageStatus>, String>
where
    Resolve: FnMut(&InstalledPackageRef) -> Result<PackageStatus, String>,
    Warn: FnMut(String),
{
    packages.sort_by(|left, right| {
        compare_package_names_for_search_order(&left.package_name, &right.package_name)
    });
    packages.dedup_by(|left, right| left.package_name == right.package_name);

    let mut statuses = Vec::with_capacity(packages.len());
    for package in packages {
        match resolve(&package) {
            Ok(status) => statuses.push(status),
            Err(err) => warn(format!(
                "warning: skipping {}: {err}",
                package.install_root.display()
            )),
        }
    }
    Ok(statuses)
}

pub(crate) fn resolve_installed_package_record(
    package_name: &str,
) -> Result<InstalledPackageRecord, String> {
    let install_root = package_install_root(&opt_pkg_root(), package_name)?;
    resolve_installed_package_record_at(package_name, &install_root)
}

pub(crate) fn resolve_package_status(
    config: &Config,
    package_name: &str,
) -> Result<PackageStatus, String> {
    let install_root = package_install_root(&opt_pkg_root(), package_name)?;
    resolve_package_status_at(config, package_name, &install_root)
}

pub(crate) fn resolve_installed_package_record_at(
    package_name: &str,
    install_root: &Path,
) -> Result<InstalledPackageRecord, String> {
    let metadata = fs::symlink_metadata(install_root).map_err(|err| match err.kind() {
        std::io::ErrorKind::NotFound => format!("package {package_name} is not installed"),
        _ => format!("failed to stat {}: {err}", install_root.display()),
    })?;
    if !metadata.is_dir() {
        return Err(format!(
            "installed package root {} is not a directory",
            install_root.display()
        ));
    }

    let receipt = load_or_resolve_package_receipt(package_name, install_root)?;
    Ok(InstalledPackageRecord {
        package_name: receipt.package_name,
        source: receipt.source,
        installed_version: receipt.version,
    })
}

pub(crate) fn resolve_package_status_at(
    config: &Config,
    package_name: &str,
    install_root: &Path,
) -> Result<PackageStatus, String> {
    let metadata = fs::symlink_metadata(install_root).map_err(|err| match err.kind() {
        std::io::ErrorKind::NotFound => format!("package {package_name} is not installed"),
        _ => format!("failed to stat {}: {err}", install_root.display()),
    })?;
    if !metadata.is_dir() {
        return Err(format!(
            "installed package root {} is not a directory",
            install_root.display()
        ));
    }

    let record = resolve_installed_package_record_at(package_name, install_root)?;
    let latest_version = resolve_latest_version_for_source(config, &record.source)?;

    Ok(PackageStatus {
        package_name: record.package_name,
        source: record.source,
        installed_version: record.installed_version,
        latest_version,
    })
}

pub(crate) fn requested_package_name(package: &RequestedPackage) -> String {
    match package {
        RequestedPackage::Auto(package_name)
        | RequestedPackage::HomebrewFormula(package_name)
        | RequestedPackage::HomebrewCask(package_name) => package_name.clone(),
        RequestedPackage::Isotope(package_name) => {
            format!("{ISOTOPE_PACKAGE_PREFIX}{package_name}")
        }
        RequestedPackage::Alias { target, .. } => target.display_name(),
        RequestedPackage::NpmPackage { package, .. } => npm_package_display_name(package),
        RequestedPackage::PipPackage(package_name) => pip_package_display_name(package_name),
    }
}

pub(crate) fn requested_install_package_name(package: &RequestedPackage) -> Result<String, String> {
    match package {
        RequestedPackage::Auto(package_name) => {
            if vendor::get(package_name).is_some() {
                return Ok(package_name.clone());
            }
            if let Some(provider_name) = embedded_provider_install_package_name(package_name)? {
                return Ok(provider_name);
            }
            let formula = formula_install_package_name(package_name)?;
            if formula != *package_name {
                return Ok(formula);
            }
            Ok(package_name.clone())
        }
        RequestedPackage::Alias { target, .. } => match target {
            PackageAliasTarget::HomebrewFormula(formula) => formula_install_package_name(formula),
            PackageAliasTarget::HomebrewCask(cask) => Ok(cask.clone()),
            PackageAliasTarget::NpmPackage(package_name) => {
                Ok(npm_package_display_name(package_name))
            }
            PackageAliasTarget::PipPackage(package_name) => {
                Ok(pip_package_display_name(package_name))
            }
        },
        RequestedPackage::HomebrewFormula(formula) => formula_install_package_name(formula),
        RequestedPackage::HomebrewCask(cask) => Ok(cask.clone()),
        RequestedPackage::Isotope(package_name) => {
            Ok(format!("{ISOTOPE_PACKAGE_PREFIX}{package_name}"))
        }
        RequestedPackage::NpmPackage { package, .. } => Ok(npm_package_display_name(package)),
        RequestedPackage::PipPackage(package_name) => Ok(pip_package_display_name(package_name)),
    }
}

pub(crate) fn requested_package_from_status(status: &PackageStatus) -> RequestedPackage {
    match &status.source {
        PackageReceiptSource::Formula { root_formula } if status.package_name == *root_formula => {
            RequestedPackage::HomebrewFormula(root_formula.clone())
        }
        PackageReceiptSource::Cask { cask_name } if status.package_name == *cask_name => {
            RequestedPackage::HomebrewCask(cask_name.clone())
        }
        PackageReceiptSource::Isotope { isotope_name } => {
            RequestedPackage::Isotope(isotope_name.clone())
        }
        PackageReceiptSource::Npm { package_name } => RequestedPackage::NpmPackage {
            package: package_name.clone(),
            version: None,
        },
        PackageReceiptSource::Pip { package_name } => {
            RequestedPackage::PipPackage(package_name.clone())
        }
        _ => RequestedPackage::Auto(status.package_name.clone()),
    }
}

pub(crate) fn resolve_package_info(
    config: &Config,
    requested: &RequestedPackage,
) -> Result<PackageInfo, String> {
    let package_name = requested_install_package_name(requested)?;
    let install_root = package_info_install_root(requested, &package_name)?;
    let metadata = match fs::symlink_metadata(&install_root) {
        Ok(metadata) => Some(metadata),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => None,
        Err(err) => return Err(format!("failed to stat {}: {err}", install_root.display())),
    };

    if let Some(metadata) = metadata {
        if !metadata.is_dir() {
            return Err(format!(
                "installed package root {} is not a directory",
                install_root.display()
            ));
        }
        return resolve_installed_package_info(config, requested, package_name, install_root);
    }

    Ok(resolve_uninstalled_package_info(
        config,
        requested,
        package_name,
        install_root,
    ))
}

fn package_info_install_root(
    requested: &RequestedPackage,
    package_name: &str,
) -> Result<PathBuf, String> {
    if let RequestedPackage::Isotope(isotope_name) = requested {
        if let Ok(record) = isotope_package_data(isotope_name) {
            if let Some(modified_package) = isotope_modified_package_name(record)? {
                let modified_root = package_install_root(&opt_pkg_root(), &modified_package)?;
                if let Ok(Some(receipt)) = load_package_receipt(&modified_root.join(ROOT_RECEIPT)) {
                    if receipt.package_name == package_name {
                        return Ok(modified_root);
                    }
                }
            }
        }
    }
    package_install_root(&opt_pkg_root(), package_name)
}

pub(crate) fn resolve_package_search_results(
    _config: &Config,
    query: &str,
) -> Result<Vec<PackageSearchResult>, String> {
    let lowered_query = query.trim().to_ascii_lowercase();
    if lowered_query.is_empty() {
        return Ok(Vec::new());
    }

    let mut results = formula_index_entries()?
        .iter()
        .filter(|entry| formula_index_entry_matches(entry, &lowered_query))
        .map(formula_family_search_result)
        .collect::<Vec<_>>();
    let db = crate::cli::load_db()?;
    crate::cli::ensure_db_schema(&db)?;
    results.extend(
        db.casks
            .iter()
            .filter(|(name, metadata)| {
                name.to_ascii_lowercase().contains(&lowered_query)
                    || metadata
                        .aliases
                        .iter()
                        .any(|alias| alias.to_ascii_lowercase().contains(&lowered_query))
                    || metadata
                        .summary
                        .to_ascii_lowercase()
                        .contains(&lowered_query)
            })
            .map(|(name, metadata)| PackageSearchResult {
                package_name: name.clone(),
                source: PackageReceiptSource::Cask {
                    cask_name: name.clone(),
                },
                summary: string_or_none(&metadata.summary),
                latest_version: Some(metadata.version.clone()),
                homepage: string_or_none(&metadata.homepage),
                dependencies: metadata.dependencies.clone(),
                rank: metadata
                    .popularity
                    .as_ref()
                    .map(|popularity| popularity.rank),
                last_updated_at: metadata.last_updated_at.clone(),
            }),
    );
    results.extend(
        db.npms
            .iter()
            .filter(|(name, metadata)| npm_entry_matches(name, metadata, &lowered_query))
            .map(|(name, metadata)| npm_search_result(name, metadata)),
    );
    results.extend(
        vendor::PACKAGES
            .iter()
            .copied()
            .filter(|entry| vendor_entry_matches(entry, &lowered_query))
            .map(vendor_search_result),
    );
    results.sort_by(|left, right| left.package_name.cmp(&right.package_name));
    results.dedup_by(|left, right| left.package_name == right.package_name);
    Ok(results)
}

pub(crate) fn resolve_available_package_results(
    _config: &Config,
) -> Result<Vec<PackageSearchResult>, String> {
    let mut results = formula_index_entries()?
        .iter()
        .map(|entry| PackageSearchResult {
            package_name: entry.name.clone(),
            source: PackageReceiptSource::Formula {
                root_formula: entry.name.clone(),
            },
            summary: string_or_none(&entry.summary),
            latest_version: None,
            homepage: None,
            dependencies: Vec::new(),
            rank: entry.popularity.as_ref().map(|popularity| popularity.rank),
            last_updated_at: entry.last_updated_at.clone(),
        })
        .collect::<Vec<_>>();
    let db = crate::cli::load_db()?;
    crate::cli::ensure_db_schema(&db)?;
    results.extend(
        db.casks
            .into_iter()
            .map(|(name, metadata)| PackageSearchResult {
                package_name: name.clone(),
                source: PackageReceiptSource::Cask {
                    cask_name: name.clone(),
                },
                summary: string_or_none(&metadata.summary),
                latest_version: Some(metadata.version),
                homepage: string_or_none(&metadata.homepage),
                dependencies: metadata.dependencies,
                rank: metadata.popularity.map(|popularity| popularity.rank),
                last_updated_at: metadata.last_updated_at,
            }),
    );
    results.extend(
        db.npms
            .into_iter()
            .map(|(name, metadata)| npm_search_result(&name, &metadata)),
    );
    results.extend(vendor::PACKAGES.iter().copied().map(vendor_search_result));
    results.sort_by(|left, right| left.package_name.cmp(&right.package_name));
    results.dedup_by(|left, right| left.package_name == right.package_name);
    results.sort_by(|left, right| match (left.rank, right.rank) {
        (Some(left_rank), Some(right_rank)) => left_rank
            .cmp(&right_rank)
            .then_with(|| left.package_name.cmp(&right.package_name)),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => left.package_name.cmp(&right.package_name),
    });
    Ok(results)
}

fn vendor_entry_matches(entry: &vendor::VendorEntry, lowered_query: &str) -> bool {
    entry.name.to_ascii_lowercase().contains(lowered_query)
        || format!("av:{}", entry.name)
            .to_ascii_lowercase()
            .contains(lowered_query)
        || (entry.executables)()
            .iter()
            .any(|executable| executable.to_ascii_lowercase().contains(lowered_query))
}

fn npm_entry_matches(
    package_name: &str,
    metadata: &EmbeddedNpmMetadata,
    lowered_query: &str,
) -> bool {
    package_name.to_ascii_lowercase().contains(lowered_query)
        || format!("npm:{package_name}")
            .to_ascii_lowercase()
            .contains(lowered_query)
        || metadata
            .executable
            .to_ascii_lowercase()
            .contains(lowered_query)
        || metadata
            .summary
            .to_ascii_lowercase()
            .contains(lowered_query)
}

fn npm_search_result(package_name: &str, metadata: &EmbeddedNpmMetadata) -> PackageSearchResult {
    let source = PackageReceiptSource::Npm {
        package_name: package_name.to_string(),
    };
    PackageSearchResult {
        package_name: package_source_qualified_name(&source),
        source,
        summary: string_or_none(&metadata.summary),
        latest_version: Some(metadata.version.clone()),
        homepage: string_or_none(&metadata.homepage),
        dependencies: Vec::new(),
        rank: metadata
            .popularity
            .as_ref()
            .map(|popularity| popularity.rank),
        last_updated_at: metadata.last_updated_at.clone(),
    }
}

fn vendor_search_result(entry: &vendor::VendorEntry) -> PackageSearchResult {
    let source = PackageReceiptSource::Vendor {
        vendor_name: entry.name.to_string(),
    };
    PackageSearchResult {
        package_name: package_source_qualified_name(&source),
        source,
        summary: None,
        latest_version: None,
        homepage: None,
        dependencies: entry
            .dependencies
            .map(|dependencies| {
                dependencies()
                    .iter()
                    .map(|dependency| dependency.to_string())
                    .collect()
            })
            .unwrap_or_default(),
        rank: None,
        last_updated_at: None,
    }
}

pub(crate) fn resolve_pulse_package_results(
    _config: &Config,
) -> Result<Vec<PackageSearchResult>, String> {
    let mut results = formula_index_entries()?
        .iter()
        .filter_map(|entry| {
            entry
                .last_updated_at
                .as_ref()
                .map(|last_updated_at| PackageSearchResult {
                    package_name: entry.name.clone(),
                    source: PackageReceiptSource::Formula {
                        root_formula: entry.name.clone(),
                    },
                    summary: string_or_none(&entry.summary),
                    latest_version: None,
                    homepage: None,
                    dependencies: Vec::new(),
                    rank: entry.popularity.as_ref().map(|popularity| popularity.rank),
                    last_updated_at: Some(last_updated_at.clone()),
                })
        })
        .collect::<Vec<_>>();
    let db = crate::cli::load_db()?;
    crate::cli::ensure_db_schema(&db)?;
    results.extend(db.casks.into_iter().filter_map(|(name, metadata)| {
        metadata
            .last_updated_at
            .clone()
            .map(|last_updated_at| PackageSearchResult {
                package_name: name.clone(),
                source: PackageReceiptSource::Cask { cask_name: name },
                summary: string_or_none(&metadata.summary),
                latest_version: Some(metadata.version),
                homepage: string_or_none(&metadata.homepage),
                dependencies: metadata.dependencies,
                rank: metadata.popularity.map(|popularity| popularity.rank),
                last_updated_at: Some(last_updated_at),
            })
    }));
    results.sort_by(|left, right| left.package_name.cmp(&right.package_name));
    results.dedup_by(|left, right| left.package_name == right.package_name);
    results.sort_by(|left, right| {
        match (
            left.last_updated_at
                .as_deref()
                .and_then(parse_embedded_package_timestamp),
            right
                .last_updated_at
                .as_deref()
                .and_then(parse_embedded_package_timestamp),
        ) {
            (Some(left_time), Some(right_time)) => right_time
                .cmp(&left_time)
                .then_with(|| left.package_name.cmp(&right.package_name)),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => left.package_name.cmp(&right.package_name),
        }
    });
    Ok(results)
}

fn parse_embedded_package_timestamp(value: &str) -> Option<OffsetDateTime> {
    OffsetDateTime::parse(value, &Rfc3339).ok()
}

pub(crate) fn formula_index_entry_matches(entry: &FormulaIndexEntry, query: &str) -> bool {
    entry.name.to_ascii_lowercase().contains(query)
        || entry
            .aliases
            .iter()
            .any(|alias| alias.to_ascii_lowercase().contains(query))
        || entry
            .oldnames
            .iter()
            .any(|oldname| oldname.to_ascii_lowercase().contains(query))
}

fn formula_search_result(entry: &FormulaIndexEntry, package_name: &str) -> PackageSearchResult {
    PackageSearchResult {
        package_name: package_name.to_string(),
        source: PackageReceiptSource::Formula {
            root_formula: entry.name.clone(),
        },
        summary: string_or_none(&entry.summary),
        latest_version: None,
        homepage: None,
        dependencies: Vec::new(),
        rank: entry.popularity.as_ref().map(|popularity| popularity.rank),
        last_updated_at: entry.last_updated_at.clone(),
    }
}

pub(crate) fn formula_family_search_result(entry: &FormulaIndexEntry) -> PackageSearchResult {
    let package_name = formula_versioned_base(&entry.name)
        .map(str::to_string)
        .or_else(|| {
            entry
                .aliases
                .iter()
                .find_map(|alias| formula_versioned_base(alias).map(str::to_string))
        })
        .unwrap_or_else(|| entry.name.clone());
    formula_search_result(entry, &package_name)
}

#[cfg(test)]
pub(crate) fn suppress_unversioned_formulae_with_versioned_search_results(
    results: &mut Vec<PackageSearchResult>,
) {
    let versioned_formula_bases = results
        .iter()
        .filter(|result| matches!(result.source, PackageReceiptSource::Formula { .. }))
        .filter_map(|result| formula_versioned_base(&result.package_name))
        .map(|base| base.to_ascii_lowercase())
        .collect::<HashSet<_>>();

    if versioned_formula_bases.is_empty() {
        return;
    }

    results.retain(|result| match &result.source {
        PackageReceiptSource::Formula { .. } => {
            formula_versioned_base(&result.package_name).is_some()
                || !versioned_formula_bases.contains(&result.package_name.to_ascii_lowercase())
        }
        _ => true,
    });
}

pub(crate) fn formula_versioned_base(formula: &str) -> Option<&str> {
    let (base, version) = formula.rsplit_once('@')?;
    if base.is_empty() || version.is_empty() || !version.chars().any(|ch| ch.is_ascii_digit()) {
        return None;
    }
    Some(base)
}

fn formula_family_base(formula: &str) -> String {
    formula_versioned_base(formula)
        .unwrap_or(formula)
        .to_string()
}

fn formula_version_alias(base: &str, version: &str) -> Option<String> {
    let major = version.split(['.', '_']).next()?;
    if major.is_empty() || !major.chars().all(|ch| ch.is_ascii_digit()) {
        return None;
    }
    Some(format!("{base}@{major}"))
}

fn parsed_stable_version(value: &str) -> Option<(u64, u64, u64)> {
    let stable = value.split('_').next().unwrap_or(value);
    let mut parts = stable.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next().unwrap_or("0").parse().ok()?;
    let patch = parts.next().unwrap_or("0").parse().ok()?;
    Some((major, minor, patch))
}

fn version_is_recommendable(version: &str) -> bool {
    let Some((_major, minor, patch)) = parsed_stable_version(version) else {
        return false;
    };
    minor > 1 || (minor == 1 && patch >= 1)
}

fn compare_version_strings(left: &str, right: &str) -> std::cmp::Ordering {
    parsed_stable_version(left)
        .cmp(&parsed_stable_version(right))
        .then_with(|| left.cmp(right))
}

pub(crate) fn formula_display_alias(
    entry: &FormulaIndexEntry,
    base: &str,
    version: &str,
) -> Option<String> {
    if formula_versioned_base(&entry.name) == Some(base) {
        return Some(entry.name.clone());
    }
    entry
        .aliases
        .iter()
        .find(|alias| formula_versioned_base(alias) == Some(base))
        .cloned()
        .or_else(|| formula_version_alias(base, version))
}

pub(crate) fn latest_formula_display_alias(
    entry: &FormulaIndexEntry,
    base: &str,
    version: &str,
) -> Option<String> {
    if formula_versioned_base(&entry.name) == Some(base) {
        return Some(entry.name.clone());
    }
    formula_version_alias(base, version).or_else(|| formula_display_alias(entry, base, version))
}

fn formula_family_entries(root_formula: &str) -> Result<Vec<FormulaIndexEntry>, String> {
    let base = formula_family_base(root_formula);
    let entries = formula_index_entries()?
        .iter()
        .filter(|entry| {
            entry.name == base
                || formula_versioned_base(&entry.name) == Some(base.as_str())
                || entry
                    .aliases
                    .iter()
                    .chain(entry.oldnames.iter())
                    .any(|alias| {
                        alias == &base || formula_versioned_base(alias) == Some(base.as_str())
                    })
        })
        .cloned()
        .collect::<Vec<_>>();
    Ok(entries)
}

fn formula_version_options(root_formula: &str) -> Result<Vec<FormulaVersionOption>, String> {
    let base = formula_family_base(root_formula);
    let mut entries = formula_family_entries(root_formula)?;
    if entries.len() <= 1
        && entries.first().is_none_or(|entry| {
            entry
                .aliases
                .iter()
                .all(|alias| formula_versioned_base(alias).is_none())
        })
    {
        return Ok(Vec::new());
    }

    entries.sort_by(|left, right| left.name.cmp(&right.name));
    entries.dedup_by(|left, right| left.name == right.name);

    let mut candidates = Vec::new();
    for entry in entries {
        if let Ok(info) = fetch_formula_info(&entry.name) {
            let version = formula_version_string(&info);
            let alias_name = formula_display_alias(&entry, &base, &version);
            candidates.push((entry, version, alias_name));
        }
    }
    if candidates.is_empty() {
        return Ok(Vec::new());
    }

    candidates.sort_by(|left, right| compare_version_strings(&left.1, &right.1));
    let latest_formula = candidates
        .last()
        .map(|(entry, _, _)| entry.name.clone())
        .unwrap_or_else(|| root_formula.to_string());
    let recommended_formula = candidates
        .iter()
        .rev()
        .find(|(_, version, _)| version_is_recommendable(version))
        .map(|(entry, _, _)| entry.name.clone());

    let latest_package_name = if candidates.iter().any(|(entry, _, _)| entry.name == base) {
        base.clone()
    } else {
        latest_formula.clone()
    };
    let latest_version = candidates
        .iter()
        .find(|(entry, _, _)| entry.name == latest_formula)
        .map(|(_, version, _)| version.clone());
    let latest_alias = candidates
        .iter()
        .find(|(entry, _, _)| entry.name == latest_formula)
        .and_then(|(entry, version, _)| latest_formula_display_alias(entry, &base, version));

    let mut options = Vec::new();
    options.push(build_formula_version_option(
        "@latest".to_string(),
        latest_alias,
        latest_package_name.clone(),
        latest_formula.clone(),
        latest_version,
        true,
        recommended_formula
            .as_deref()
            .is_some_and(|formula| formula == latest_formula),
    )?);

    for (entry, version, alias_name) in candidates {
        if entry.name == latest_formula && entry.name == latest_package_name {
            continue;
        }
        let display_name = alias_name.clone().unwrap_or_else(|| entry.name.clone());
        options.push(build_formula_version_option(
            display_name,
            alias_name,
            entry.name.clone(),
            entry.name.clone(),
            Some(version),
            false,
            recommended_formula
                .as_deref()
                .is_some_and(|formula| formula == entry.name),
        )?);
    }
    Ok(options)
}

fn build_formula_version_option(
    display_name: String,
    alias_name: Option<String>,
    package_name: String,
    root_formula: String,
    version: Option<String>,
    is_latest: bool,
    is_recommended: bool,
) -> Result<FormulaVersionOption, String> {
    let install_root = package_install_root(&opt_pkg_root(), &package_name)?;
    let installed = install_root.is_dir();
    let stub_active = installed && package_stubs_are_active(&install_root, &package_name)?;
    let install_package_name = format!("{BREW_PACKAGE_PREFIX}{package_name}");
    let supports_side_by_side_stubs = package_name.starts_with("python@");
    Ok(FormulaVersionOption {
        display_name,
        alias_name,
        package_name,
        install_package_name,
        root_formula,
        version,
        install_root,
        installed,
        stub_active,
        is_latest,
        is_recommended,
        supports_side_by_side_stubs,
    })
}

fn package_stubs_are_active(install_root: &Path, package_name: &str) -> Result<bool, String> {
    let manifest = load_stub_manifest(&install_root.join(STUB_MANIFEST))?;
    if manifest.stubs.is_empty() {
        return Ok(false);
    }
    for stub in manifest.stubs {
        let stub_path = managed_bin_root().join(stub);
        if stub_belongs_to_package(&stub_path, package_name)? {
            return Ok(true);
        }
    }
    Ok(false)
}

pub(crate) fn resolve_installed_package_info(
    config: &Config,
    requested: &RequestedPackage,
    package_name: String,
    install_root: PathBuf,
) -> Result<PackageInfo, String> {
    let mut info = PackageInfo {
        package_name,
        qualified_name: String::new(),
        install_root,
        installed: true,
        source: None,
        source_error: None,
        aliases: Vec::new(),
        aliases_error: None,
        installed_version: None,
        latest_version: None,
        latest_version_error: None,
        executable_paths: Vec::new(),
        executable_paths_error: None,
        popularity: None,
        last_updated_at: None,
        homebrew_info: None,
        homebrew_info_error: None,
        npm_homepage: None,
        npm_package_info_error: None,
        security_state: None,
        version_options: Vec::new(),
    };

    match load_package_receipt(&info.install_root.join(ROOT_RECEIPT)) {
        Ok(Some(receipt)) => {
            info.package_name = receipt.package_name;
            info.source = Some(receipt.source);
            info.installed_version = Some(receipt.version);
        }
        Ok(None) => info.source_error = Some("missing package metadata".to_string()),
        Err(err) => info.source_error = Some(err),
    }

    if info.source.is_none() {
        info.source = explicit_requested_package_source(requested);
    }
    match installed_stub_paths_at(&info.install_root) {
        Ok(paths) => info.executable_paths = paths,
        Err(err) => info.executable_paths_error = Some(err),
    }
    populate_package_info_identity(&mut info);
    populate_package_info_metadata(config, &mut info);
    populate_formula_version_options(&mut info);
    info.security_state = package_security_state(&info);
    Ok(info)
}

pub(crate) fn resolve_uninstalled_package_info(
    config: &Config,
    requested: &RequestedPackage,
    package_name: String,
    install_root: PathBuf,
) -> PackageInfo {
    let mut info = PackageInfo {
        package_name,
        qualified_name: String::new(),
        install_root,
        installed: false,
        source: None,
        source_error: None,
        aliases: Vec::new(),
        aliases_error: None,
        installed_version: None,
        latest_version: None,
        latest_version_error: None,
        executable_paths: Vec::new(),
        executable_paths_error: None,
        popularity: None,
        last_updated_at: None,
        homebrew_info: None,
        homebrew_info_error: None,
        npm_homepage: None,
        npm_package_info_error: None,
        security_state: None,
        version_options: Vec::new(),
    };

    match infer_requested_package_source(requested) {
        Ok(source) => info.source = Some(source),
        Err(err) => info.source_error = Some(err),
    }
    if let Some(PackageReceiptSource::Formula { root_formula }) = info.source.as_ref() {
        match predicted_homebrew_executables(root_formula) {
            Ok(paths) => info.executable_paths = paths,
            Err(err) => info.executable_paths_error = Some(err),
        }
    }
    populate_package_info_identity(&mut info);
    populate_package_info_metadata(config, &mut info);
    populate_formula_version_options(&mut info);
    info.security_state = package_security_state(&info);
    info
}

fn populate_formula_version_options(info: &mut PackageInfo) {
    let Some(PackageReceiptSource::Formula { root_formula }) = info.source.as_ref() else {
        return;
    };
    if let Ok(options) = formula_version_options(root_formula) {
        info.version_options = options;
    }
}

pub(crate) fn predicted_homebrew_executables(formula: &str) -> Result<Vec<String>, String> {
    let db = crate::cli::load_db()?;
    crate::cli::ensure_db_schema(&db)?;
    let canonical = canonical_formula_name(formula)?;
    Ok(homebrew_executables_from_db(&canonical, &db))
}

pub(crate) fn homebrew_executables_from_db(formula: &str, db: &Db) -> Vec<String> {
    let mut executables = db
        .entries
        .iter()
        .filter_map(|(executable, provider)| (provider == formula).then_some(executable.clone()))
        .collect::<Vec<_>>();
    executables.sort();
    executables.dedup();
    executables
}

pub(crate) fn populate_package_info_identity(info: &mut PackageInfo) {
    if let Some(source) = info.source.as_ref() {
        info.qualified_name = package_source_qualified_name(source);
        let (aliases, alias_error) = resolve_aliases_for_source(source);
        info.aliases = aliases;
        info.aliases_error = alias_error;
    } else {
        info.qualified_name = info.package_name.clone();
    }
}

pub(crate) fn populate_package_info_metadata(config: &Config, info: &mut PackageInfo) {
    let Some(source) = info.source.as_ref() else {
        return;
    };

    match source {
        PackageReceiptSource::Formula { root_formula } => match fetch_formula_info(root_formula) {
            Ok(formula_info) => {
                info.homebrew_info = Some(homebrew_package_info_from_formula_info(
                    root_formula,
                    &formula_info,
                ));
                if let Ok(db) = crate::cli::load_db() {
                    if crate::cli::ensure_db_schema(&db).is_ok() {
                        let canonical = canonical_formula_name(root_formula)
                            .unwrap_or_else(|_| root_formula.clone());
                        info.popularity = db
                            .formulas
                            .get(&canonical)
                            .and_then(|metadata| metadata.popularity.clone());
                        info.last_updated_at = db
                            .formulas
                            .get(&canonical)
                            .and_then(|metadata| metadata.last_updated_at.clone());
                    }
                }
                if info.last_updated_at.is_none() {
                    let canonical = canonical_formula_name(root_formula)
                        .unwrap_or_else(|_| root_formula.clone());
                    info.last_updated_at =
                        resolve_formula_last_updated_at(&canonical).ok().flatten();
                }
                match ensure_formula_has_bottle(root_formula, &formula_info, &config.bottle_tag) {
                    Ok(()) => info.latest_version = Some(formula_version_string(&formula_info)),
                    Err(err) => info.latest_version_error = Some(err),
                }
            }
            Err(err) => {
                info.latest_version_error = Some(err.clone());
                info.homebrew_info_error = Some(err);
            }
        },
        PackageReceiptSource::Cask { cask_name } => match embedded_cask(cask_name) {
            Ok(cask_info) => {
                info.homebrew_info = Some(HomebrewPackageInfo {
                    formula: cask_name.clone(),
                    description: string_or_none(&cask_info.summary),
                    homepage: string_or_none(&cask_info.homepage),
                    license: None,
                    dependencies: cask_info.dependencies.clone(),
                });
                info.popularity = cask_info.popularity.clone();
                info.last_updated_at = cask_info.last_updated_at.clone();
                if info.last_updated_at.is_none() {
                    info.last_updated_at = resolve_cask_last_updated_at(cask_name).ok().flatten();
                }
                info.latest_version = Some(cask_info.version);
            }
            Err(err) => {
                info.latest_version_error = Some(err.clone());
                info.homebrew_info_error = Some(err);
            }
        },
        PackageReceiptSource::Isotope { isotope_name } => {
            match isotope_package_data(isotope_name) {
                Ok(isotope) => {
                    info.last_updated_at = isotope.published_at.clone();
                    match isotope_modified_package_name(isotope) {
                        Ok(Some(formula)) => match fetch_formula_info(&formula) {
                            Ok(formula_info) => {
                                info.homebrew_info = Some(homebrew_package_info_from_formula_info(
                                    &formula,
                                    &formula_info,
                                ));
                                match ensure_formula_has_bottle(
                                    &formula,
                                    &formula_info,
                                    &config.bottle_tag,
                                ) {
                                    Ok(()) => {
                                        info.latest_version =
                                            Some(formula_version_string(&formula_info))
                                    }
                                    Err(err) => {
                                        info.latest_version = Some(isotope.version.clone());
                                        info.latest_version_error = Some(err);
                                    }
                                }
                            }
                            Err(err) => {
                                info.latest_version = Some(isotope.version.clone());
                                info.latest_version_error = Some(err.clone());
                                info.homebrew_info_error = Some(err);
                                info.homebrew_info =
                                    Some(isotope_homebrew_info(isotope_name, isotope));
                            }
                        },
                        _ => {
                            info.latest_version = Some(isotope.version.clone());
                            info.homebrew_info = Some(isotope_homebrew_info(isotope_name, isotope));
                        }
                    }
                }
                Err(err) => {
                    info.latest_version_error = Some(err.clone());
                    info.homebrew_info_error = Some(err);
                }
            }
        }
        PackageReceiptSource::Npm { package_name } => {
            match resolve_latest_version_for_source(config, source) {
                Ok(latest_version) => info.latest_version = Some(latest_version),
                Err(err) => info.latest_version_error = Some(err),
            }
            match resolve_npm_homepage(package_name) {
                Ok(homepage) => info.npm_homepage = homepage,
                Err(err) => info.npm_package_info_error = Some(err),
            }
        }
        _ => match resolve_latest_version_for_source(config, source) {
            Ok(latest_version) => info.latest_version = Some(latest_version),
            Err(err) => info.latest_version_error = Some(err),
        },
    }
}

fn resolve_formula_last_updated_at(formula: &str) -> Result<Option<String>, String> {
    let path = format!(
        "Formula/{}/{}.rb",
        formula.chars().next().unwrap_or('f'),
        formula
    );
    resolve_homebrew_repo_last_updated_at("Homebrew/homebrew-core", &path)
}

fn resolve_cask_last_updated_at(cask: &str) -> Result<Option<String>, String> {
    let path = format!("Casks/{}/{}.rb", cask.chars().next().unwrap_or('c'), cask);
    resolve_homebrew_repo_last_updated_at("Homebrew/homebrew-cask", &path)
}

fn resolve_homebrew_repo_last_updated_at(repo: &str, path: &str) -> Result<Option<String>, String> {
    let encoded_path = path.replace('/', "%2F");
    let url = format!("https://api.github.com/repos/{repo}/commits?path={encoded_path}&per_page=1");
    let commits = fetch_optional_json::<Vec<GitHubCommitListEntry>, _>(&url, || {
        format!("failed to fetch commit metadata for {path}")
    })?;
    Ok(commits.and_then(|commits| {
        commits.into_iter().next().and_then(|entry| {
            entry
                .commit
                .committer
                .map(|identity| identity.date)
                .or_else(|| entry.commit.author.map(|identity| identity.date))
        })
    }))
}

pub(crate) fn explicit_requested_package_source(
    requested: &RequestedPackage,
) -> Option<PackageReceiptSource> {
    match requested {
        RequestedPackage::HomebrewFormula(formula) => Some(PackageReceiptSource::Formula {
            root_formula: formula.clone(),
        }),
        RequestedPackage::HomebrewCask(cask) => Some(PackageReceiptSource::Cask {
            cask_name: cask.clone(),
        }),
        RequestedPackage::Isotope(isotope) => Some(PackageReceiptSource::Isotope {
            isotope_name: isotope.clone(),
        }),
        RequestedPackage::Alias { target, .. } => match target {
            PackageAliasTarget::HomebrewFormula(formula) => Some(PackageReceiptSource::Formula {
                root_formula: formula.clone(),
            }),
            PackageAliasTarget::HomebrewCask(cask) => Some(PackageReceiptSource::Cask {
                cask_name: cask.clone(),
            }),
            PackageAliasTarget::NpmPackage(package_name) => Some(PackageReceiptSource::Npm {
                package_name: package_name.clone(),
            }),
            PackageAliasTarget::PipPackage(package_name) => Some(PackageReceiptSource::Pip {
                package_name: package_name.clone(),
            }),
        },
        RequestedPackage::NpmPackage { package, .. } => Some(PackageReceiptSource::Npm {
            package_name: package.clone(),
        }),
        RequestedPackage::PipPackage(package_name) => Some(PackageReceiptSource::Pip {
            package_name: package_name.clone(),
        }),
        RequestedPackage::Auto(_) => None,
    }
}

pub(crate) fn infer_requested_package_source(
    requested: &RequestedPackage,
) -> Result<PackageReceiptSource, String> {
    if let Some(source) = explicit_requested_package_source(requested) {
        return Ok(source);
    }

    let RequestedPackage::Auto(package_name) = requested else {
        unreachable!("qualified and aliased packages are handled above")
    };
    if let Some(package) = vendor::get(package_name) {
        return Ok(PackageReceiptSource::Vendor {
            vendor_name: package.name.to_string(),
        });
    }

    Ok(match resolve_i_root_package(package_name)? {
        EmbeddedPackage::Formula(root_formula) => PackageReceiptSource::Formula { root_formula },
        EmbeddedPackage::Cask(cask_name) => PackageReceiptSource::Cask { cask_name },
        EmbeddedPackage::NpmPackage(package_name) => PackageReceiptSource::Npm { package_name },
    })
}

pub(crate) fn resolve_latest_version_for_source(
    config: &Config,
    source: &PackageReceiptSource,
) -> Result<String, String> {
    match source {
        PackageReceiptSource::Formula { root_formula } => {
            resolve_formula_latest_version(config, root_formula)
        }
        PackageReceiptSource::Cask { cask_name } => resolve_cask_latest_version(cask_name),
        PackageReceiptSource::Isotope { isotope_name } => {
            let isotope = isotope_package_data(isotope_name)?;
            if let Some(formula) = isotope_modified_package_name(isotope)? {
                return resolve_formula_latest_version(config, &formula);
            }
            Ok(isotope.version.clone())
        }
        PackageReceiptSource::Vendor { vendor_name } => resolve_vendor_latest_version(vendor_name),
        PackageReceiptSource::Npm { package_name } => resolve_npm_latest_version(package_name),
        PackageReceiptSource::Pip { package_name } => resolve_pip_latest_version(package_name),
    }
}

pub(crate) fn package_source_qualified_name(source: &PackageReceiptSource) -> String {
    match source {
        PackageReceiptSource::Formula { root_formula } => crate::brew::qualified_name(root_formula),
        PackageReceiptSource::Cask { cask_name } => crate::cask::qualified_name(cask_name),
        PackageReceiptSource::Isotope { isotope_name } => {
            format!("{ISOTOPE_PACKAGE_PREFIX}{isotope_name}")
        }
        PackageReceiptSource::Vendor { vendor_name } => format!("av:{vendor_name}"),
        PackageReceiptSource::Npm { package_name } => npm_package_display_name(package_name),
        PackageReceiptSource::Pip { package_name } => pip_package_display_name(package_name),
    }
}

pub(crate) fn resolve_aliases_for_source(
    source: &PackageReceiptSource,
) -> (Vec<String>, Option<String>) {
    let mut aliases = our_aliases_for_source(source);
    let mut alias_error = None;

    if let PackageReceiptSource::Formula { root_formula } = source {
        match homebrew_aliases_for_formula(root_formula) {
            Ok(mut brew_aliases) => aliases.append(&mut brew_aliases),
            Err(err) => alias_error = Some(err),
        }
    } else if let PackageReceiptSource::Cask { cask_name } = source {
        if let Ok(cask) = embedded_cask(cask_name) {
            aliases.extend(cask.aliases.iter().cloned());
        }
    }

    aliases.sort();
    aliases.dedup();
    (aliases, alias_error)
}

pub(crate) fn our_aliases_for_source(source: &PackageReceiptSource) -> Vec<String> {
    let qualified_name = package_source_qualified_name(source);
    let mut aliases = embedded_package_aliases()
        .iter()
        .filter_map(|(alias, target)| {
            (target.display_name() == qualified_name).then_some(alias.clone())
        })
        .collect::<Vec<_>>();
    aliases.sort();
    aliases
}

pub(crate) fn homebrew_aliases_for_formula(formula: &str) -> Result<Vec<String>, String> {
    let mut aliases = formula_alias_index()?
        .iter()
        .filter_map(|(alias, canonical)| (canonical == formula).then_some(alias.clone()))
        .collect::<Vec<_>>();
    aliases.sort();
    Ok(aliases)
}

pub(crate) fn homebrew_package_info_from_formula_info(
    formula: &str,
    info: &FormulaInfo,
) -> HomebrewPackageInfo {
    HomebrewPackageInfo {
        formula: formula.to_string(),
        description: string_or_none(&info.desc),
        homepage: string_or_none(&info.homepage),
        license: info
            .license
            .clone()
            .and_then(|value| string_or_none(&value)),
        dependencies: info.dependencies.clone(),
    }
}

pub(crate) fn isotope_homebrew_info(
    isotope_name: &str,
    isotope: &IsotopePackageData,
) -> HomebrewPackageInfo {
    HomebrewPackageInfo {
        formula: isotope_name.to_string(),
        description: isotope
            .modifies
            .as_deref()
            .map(|modifies| format!("Radioisotope modifying {modifies}"))
            .or_else(|| {
                isotope
                    .replaces
                    .as_deref()
                    .map(|replaces| format!("Isotope mirror replacing {replaces}"))
            }),
        homepage: isotope.release_url.clone(),
        license: None,
        dependencies: Vec::new(),
    }
}

pub(crate) fn formula_package_metadata(formula: &str) -> Result<PackageMetadata, String> {
    let info = fetch_formula_info(formula)?;
    Ok(PackageMetadata {
        description: string_or_none(&info.desc),
        homepage: string_or_none(&info.homepage),
    })
}

pub(crate) fn string_or_none(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

pub(crate) fn formula_index_entries() -> Result<&'static Vec<FormulaIndexEntry>, String> {
    FORMULA_INDEX
        .get_or_init(build_formula_index)
        .as_ref()
        .map_err(|err| err.clone())
}

pub(crate) fn format_package_info(info: &PackageInfo) -> String {
    let installed_value = if info.installed {
        info.install_root.display().to_string()
    } else {
        "no".to_string()
    };
    let mut lines = vec![plain_box_top()];
    for (index, line) in wrap_text(&info.qualified_name, INFO_WIDTH - 6)
        .into_iter()
        .enumerate()
    {
        if index == 0 {
            lines.push(format!("   📦 {line}"));
        } else {
            lines.push(format!("     {line}"));
        }
    }
    lines.push(plain_box_bottom());
    lines.push(String::new());

    push_single_line_field(
        &mut lines,
        "Version",
        &format_version_value(info),
        format_version_status(info).as_deref(),
    );
    push_single_line_field(&mut lines, "Installed", &installed_value, None);
    push_wrapped_field(
        &mut lines,
        "Source",
        &format_source_field(info.source.as_ref()),
    );
    if !info.aliases.is_empty() {
        push_wrapped_field(&mut lines, "Aliases", &info.aliases.join(", "));
    }

    let mut metadata_lines = Vec::new();
    if let Some(homebrew_info) = info.homebrew_info.as_ref() {
        if let Some(description) = homebrew_info.description.as_deref() {
            push_wrapped_field(&mut metadata_lines, "Description", description);
        }
        if let Some(homepage) = homebrew_info.homepage.as_deref() {
            push_wrapped_field(&mut metadata_lines, "Homepage", homepage);
        }
        if let Some(license) = homebrew_info.license.as_deref() {
            push_wrapped_field(&mut metadata_lines, "License", license);
        }
        push_wrapped_field(
            &mut metadata_lines,
            "Formula Page",
            &homebrew_formula_page_url(&homebrew_info.formula),
        );
    } else if let Some(PackageReceiptSource::Formula { root_formula }) = info.source.as_ref() {
        push_wrapped_field(
            &mut metadata_lines,
            "Formula Page",
            &homebrew_formula_page_url(root_formula),
        );
        if let Some(err) = info.homebrew_info_error.as_deref() {
            push_wrapped_field(
                &mut metadata_lines,
                "Homebrew Info",
                &format!("unavailable ({err})"),
            );
        }
    }
    if let Some(PackageReceiptSource::Npm { .. }) = info.source.as_ref() {
        if let Some(homepage) = info.npm_homepage.as_deref() {
            push_wrapped_field(&mut metadata_lines, "Homepage", homepage);
        } else if let Some(err) = info.npm_package_info_error.as_deref() {
            push_wrapped_field(
                &mut metadata_lines,
                "Homepage",
                &format!("unavailable ({err})"),
            );
        }
    }

    if !metadata_lines.is_empty() {
        lines.push(String::new());
        lines.extend(metadata_lines);
    }

    if let Some(homebrew_info) = info.homebrew_info.as_ref() {
        if !homebrew_info.dependencies.is_empty() {
            lines.push(String::new());
            lines.push(section_top("Dependencies"));
            for line in wrap_tokens(&homebrew_info.dependencies, 2, 3) {
                lines.push(line);
            }
            lines.push(section_bottom());
        }
    }

    if !info.executable_paths.is_empty() || info.executable_paths_error.is_some() {
        lines.push(String::new());
        lines.push(section_top("Executables"));
        if let Some(err) = info.executable_paths_error.as_deref() {
            for line in wrap_text(&format!("unavailable ({err})"), INFO_INNER_WIDTH - 2) {
                lines.push(format!("  {line}"));
            }
        } else {
            for executable in &info.executable_paths {
                for line in wrap_text(executable, INFO_INNER_WIDTH - 2) {
                    lines.push(format!("  {line}"));
                }
            }
        }
        lines.push(section_bottom());
    }

    lines.join("\n")
}

pub(crate) fn plain_box_top() -> String {
    format!("╭{}╮", "─".repeat(INFO_INNER_WIDTH))
}

pub(crate) fn plain_box_bottom() -> String {
    format!("╰{}╯", "─".repeat(INFO_INNER_WIDTH))
}

pub(crate) fn section_top(title: &str) -> String {
    let prefix = format!("╭─ {title} ");
    let fill = "─".repeat(INFO_WIDTH - prefix.chars().count() - 1);
    format!("{prefix}{fill}╮")
}

pub(crate) fn section_bottom() -> String {
    format!("╰{}╯", "─".repeat(INFO_INNER_WIDTH))
}

pub(crate) fn push_single_line_field(
    lines: &mut Vec<String>,
    label: &str,
    value: &str,
    suffix: Option<&str>,
) {
    let mut line = format!("  {label:<INFO_LABEL_WIDTH$}{value}");
    if let Some(suffix) = suffix {
        line.push_str("  ");
        line.push_str(suffix);
    }
    lines.push(line);
}

pub(crate) fn push_wrapped_field(lines: &mut Vec<String>, label: &str, value: &str) {
    let wrapped = wrap_text(value, INFO_WIDTH - 2 - INFO_LABEL_WIDTH - 2);
    let mut iter = wrapped.into_iter();
    if let Some(first) = iter.next() {
        lines.push(format!("  {label:<INFO_LABEL_WIDTH$}{first}"));
        for line in iter {
            lines.push(format!("  {:<INFO_LABEL_WIDTH$}{line}", ""));
        }
    } else {
        lines.push(format!("  {label:<INFO_LABEL_WIDTH$}"));
    }
}

pub(crate) fn wrap_text(value: &str, width: usize) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    for paragraph in value.lines() {
        if paragraph.is_empty() {
            if lines.is_empty() || !lines.last().unwrap().is_empty() {
                lines.push(String::new());
            }
            continue;
        }
        let mut current = String::new();
        for word in paragraph.split_whitespace() {
            let chunks = split_text_hard(word, width);
            for chunk in chunks {
                let next_len = if current.is_empty() {
                    chunk.chars().count()
                } else {
                    current.chars().count() + 1 + chunk.chars().count()
                };
                if !current.is_empty() && next_len > width {
                    lines.push(current);
                    current = chunk;
                } else {
                    if !current.is_empty() {
                        current.push(' ');
                    }
                    current.push_str(&chunk);
                }
            }
        }
        if !current.is_empty() {
            lines.push(current);
        }
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

pub(crate) fn split_text_hard(value: &str, width: usize) -> Vec<String> {
    if value.chars().count() <= width {
        return vec![value.to_string()];
    }

    let mut chunks = Vec::new();
    let mut current = String::new();
    for ch in value.chars() {
        if current.chars().count() == width {
            chunks.push(current);
            current = String::new();
        }
        current.push(ch);
    }
    if !current.is_empty() {
        chunks.push(current);
    }
    chunks
}

pub(crate) fn wrap_tokens(tokens: &[String], indent: usize, gap: usize) -> Vec<String> {
    let indent_str = " ".repeat(indent);
    let gap_str = " ".repeat(gap);
    let mut lines = Vec::new();
    let mut current = indent_str.clone();
    for token in tokens {
        let candidate = if current.trim().is_empty() {
            format!("{indent_str}{token}")
        } else {
            format!("{current}{gap_str}{token}")
        };
        if current != indent_str && candidate.chars().count() > INFO_WIDTH {
            lines.push(current);
            current = format!("{indent_str}{token}");
        } else if current == indent_str {
            current.push_str(token);
        } else {
            current.push_str(&gap_str);
            current.push_str(token);
        }
    }
    if current != indent_str {
        lines.push(current);
    }
    lines
}

pub(crate) fn format_source_field(source: Option<&PackageReceiptSource>) -> String {
    match source {
        Some(PackageReceiptSource::Formula { .. }) => "Homebrew".to_string(),
        Some(PackageReceiptSource::Cask { .. }) => "Homebrew Cask".to_string(),
        Some(PackageReceiptSource::Isotope { .. }) => "Isotope".to_string(),
        Some(PackageReceiptSource::Vendor { .. }) => "Subs".to_string(),
        Some(PackageReceiptSource::Npm { .. }) => "npm".to_string(),
        Some(PackageReceiptSource::Pip { .. }) => "PyPI".to_string(),
        None => "Unknown".to_string(),
    }
}

pub(crate) fn format_version_value(info: &PackageInfo) -> String {
    if let Some(installed_version) = info.installed_version.as_deref() {
        installed_version.to_string()
    } else if let Some(latest_version) = info.latest_version.as_deref() {
        latest_version.to_string()
    } else {
        "unknown".to_string()
    }
}

pub(crate) fn format_version_status(info: &PackageInfo) -> Option<String> {
    if !info.installed {
        return None;
    }
    match (&info.installed_version, &info.latest_version) {
        (Some(installed_version), Some(latest_version)) if installed_version == latest_version => {
            Some("✔ up to date".to_string())
        }
        (Some(_), Some(latest_version)) => Some(format!("update available ({latest_version})")),
        (_, Some(_)) => None,
        (_, None) => info
            .latest_version_error
            .as_ref()
            .map(|err| format!("latest unknown ({err})")),
    }
}

pub(crate) fn homebrew_formula_page_url(formula: &str) -> String {
    format!("https://formulae.brew.sh/formula/{formula}")
}

pub(crate) fn installed_stub_paths_at(install_root: &Path) -> Result<Vec<String>, String> {
    let mut paths = load_stub_manifest(&install_root.join(STUB_MANIFEST))?
        .stubs
        .into_iter()
        .map(|stub| managed_bin_root().join(stub).display().to_string())
        .collect::<Vec<_>>();
    paths.sort();
    Ok(paths)
}

#[cfg(test)]
pub(crate) fn installed_package_names(opt_root: &Path) -> Result<Vec<String>, String> {
    Ok(installed_package_refs(opt_root)?
        .into_iter()
        .map(|package| package.package_name)
        .collect())
}

pub(crate) fn installed_package_refs(opt_root: &Path) -> Result<Vec<InstalledPackageRef>, String> {
    let entries = match fs::read_dir(opt_root) {
        Ok(entries) => entries,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(err) => return Err(format!("failed to read {}: {err}", opt_root.display())),
    };

    let mut packages = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|err| format!("failed to read {}: {err}", opt_root.display()))?;
        let path = entry.path();
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| format!("non-utf8 directory name under {}", opt_root.display()))?;
        if name.starts_with('.') || !path.is_dir() {
            continue;
        }
        if name == "homebrew" {
            continue;
        }
        if name == "npm" {
            packages.extend(installed_npm_package_refs(&path)?);
            continue;
        }
        if name == "pip" {
            packages.extend(installed_pip_package_refs(&path)?);
            continue;
        }
        if name == ISOTOPE_INSTALL_ROOT_DIR {
            packages.extend(installed_isotope_package_refs(&path)?);
            continue;
        }
        packages.push(InstalledPackageRef {
            package_name: name,
            install_root: path,
        });
    }
    Ok(packages)
}

pub(crate) fn installed_npm_package_refs(
    npm_root: &Path,
) -> Result<Vec<InstalledPackageRef>, String> {
    let entries = match fs::read_dir(npm_root) {
        Ok(entries) => entries,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(err) => return Err(format!("failed to read {}: {err}", npm_root.display())),
    };

    let mut packages = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|err| format!("failed to read {}: {err}", npm_root.display()))?;
        let path = entry.path();
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| format!("non-utf8 directory name under {}", npm_root.display()))?;
        if name.starts_with('.') || !path.is_dir() {
            continue;
        }
        if name.starts_with('@') {
            let scope_entries = fs::read_dir(&path)
                .map_err(|err| format!("failed to read {}: {err}", path.display()))?;
            for scope_entry in scope_entries {
                let scope_entry = scope_entry
                    .map_err(|err| format!("failed to read {}: {err}", path.display()))?;
                let scoped_path = scope_entry.path();
                let scoped_name = scope_entry
                    .file_name()
                    .into_string()
                    .map_err(|_| format!("non-utf8 directory name under {}", path.display()))?;
                if scoped_name.starts_with('.') || !scoped_path.is_dir() {
                    continue;
                }
                let package = format!("{name}/{scoped_name}");
                packages.push(InstalledPackageRef {
                    package_name: match load_package_receipt(&scoped_path.join(ROOT_RECEIPT)) {
                        Ok(Some(receipt)) => receipt.package_name,
                        Ok(None) | Err(_) => npm_package_display_name(&package),
                    },
                    install_root: scoped_path,
                });
            }
            continue;
        }
        packages.push(InstalledPackageRef {
            package_name: match load_package_receipt(&path.join(ROOT_RECEIPT)) {
                Ok(Some(receipt)) => receipt.package_name,
                Ok(None) | Err(_) => npm_package_display_name(&name),
            },
            install_root: path,
        });
    }
    Ok(packages)
}

pub(crate) fn installed_isotope_package_refs(
    isotope_root: &Path,
) -> Result<Vec<InstalledPackageRef>, String> {
    let entries = match fs::read_dir(isotope_root) {
        Ok(entries) => entries,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(err) => return Err(format!("failed to read {}: {err}", isotope_root.display())),
    };

    let mut packages = Vec::new();
    for entry in entries {
        let entry =
            entry.map_err(|err| format!("failed to read {}: {err}", isotope_root.display()))?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| format!("non-utf8 directory name under {}", isotope_root.display()))?;
        packages.push(InstalledPackageRef {
            package_name: format!("{ISOTOPE_PACKAGE_PREFIX}{name}"),
            install_root: path,
        });
    }
    Ok(packages)
}

pub(crate) fn installed_pip_package_refs(
    pip_root: &Path,
) -> Result<Vec<InstalledPackageRef>, String> {
    let entries = match fs::read_dir(pip_root) {
        Ok(entries) => entries,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(err) => return Err(format!("failed to read {}: {err}", pip_root.display())),
    };

    let mut packages = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|err| format!("failed to read {}: {err}", pip_root.display()))?;
        let path = entry.path();
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| format!("non-utf8 directory name under {}", pip_root.display()))?;
        if name.starts_with('.') || !path.is_dir() {
            continue;
        }
        packages.push(InstalledPackageRef {
            package_name: match load_package_receipt(&path.join(ROOT_RECEIPT)) {
                Ok(Some(receipt)) => receipt.package_name,
                Ok(None) | Err(_) => pip_package_display_name(&name),
            },
            install_root: path,
        });
    }
    Ok(packages)
}

pub(crate) fn load_or_resolve_package_receipt(
    package_name: &str,
    install_root: &Path,
) -> Result<PackageReceipt, String> {
    load_package_receipt(&install_root.join(ROOT_RECEIPT))?
        .ok_or_else(|| format!("package {package_name} is installed but missing package metadata"))
}

pub(crate) fn load_package_receipt(path: &Path) -> Result<Option<PackageReceipt>, String> {
    let data = match fs::read(path) {
        Ok(data) => data,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(format!("failed to read {}: {err}", path.display())),
    };
    let receipt = serde_json::from_slice(&data)
        .map_err(|err| format!("failed to parse {}: {err}", path.display()))?;
    Ok(Some(receipt))
}

pub(crate) fn write_package_receipt(path: &Path, receipt: &PackageReceipt) -> Result<(), String> {
    let data = serde_json::to_vec_pretty(receipt)
        .map_err(|err| format!("failed to serialize package receipt: {err}"))?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("failed to create {}: {err}", parent.display()))?;
    }
    fs::write(path, data).map_err(|err| format!("failed to write {}: {err}", path.display()))
}

pub(crate) fn resolve_formula_latest_version(
    config: &Config,
    formula: &str,
) -> Result<String, String> {
    let info = fetch_formula_info(formula)?;
    ensure_formula_has_bottle(formula, &info, &config.bottle_tag)?;
    Ok(formula_version_string(&info))
}

pub(crate) fn resolve_cask_latest_version(cask: &str) -> Result<String, String> {
    Ok(embedded_cask(cask)?.version.clone())
}

pub(crate) fn resolve_vendor_latest_version(package_name: &str) -> Result<String, String> {
    let package = vendor::get(package_name)
        .ok_or_else(|| format!("vendor package {package_name} is not registered"))?;
    (package.version)().map(|version| version.to_string())
}

pub(crate) fn resolve_npm_package_version(package_name: &str) -> Result<semver::Version, String> {
    let version = vendor::npm_latest_tag(package_name)?;
    vendor::parse_semver(&version, package_name)
}

pub(crate) fn resolve_npm_latest_version(package_name: &str) -> Result<String, String> {
    resolve_npm_package_version(package_name).map(|version| version.to_string())
}

pub(crate) fn resolve_npm_homepage(package_name: &str) -> Result<Option<String>, String> {
    Ok(resolve_npm_package_metadata(package_name)?.homepage)
}

pub(crate) fn resolve_npm_package_metadata(package_name: &str) -> Result<PackageMetadata, String> {
    let url = format!(
        "{}/{}",
        config::npm_registry_root(),
        urlencoding::encode(package_name)
    );
    let response: NpmPackageMetadata = fetch_json(&url, || {
        format!("failed to fetch npm metadata for {package_name}")
    })?;
    Ok(PackageMetadata {
        description: response
            .description
            .and_then(|value| string_or_none(&value)),
        homepage: response.homepage.and_then(|value| string_or_none(&value)),
    })
}

pub(crate) fn resolve_pip_latest_version(package_name: &str) -> Result<String, String> {
    let response = fetch_pypi_package_info(package_name)?;
    if response.info.version.is_empty() {
        return Err(format!(
            "failed to resolve latest PyPI version for {package_name}"
        ));
    }
    Ok(response.info.version)
}

pub(crate) fn resolve_pip_package_metadata(package_name: &str) -> Result<PackageMetadata, String> {
    let response = fetch_pypi_package_info(package_name)?;
    Ok(PackageMetadata {
        description: string_or_none(&response.info.summary),
        homepage: string_or_none(&response.info.home_page),
    })
}

fn fetch_pypi_package_info(package_name: &str) -> Result<PypiPackageInfoResponse, String> {
    let normalized = normalize_pip_package_name(package_name);
    let url = format!("{}/{}/json", pypi_root(), urlencoding::encode(&normalized));
    fetch_json(&url, || {
        format!("failed to fetch PyPI metadata for {package_name}")
    })
}

#[cfg(test)]
pub(crate) fn extract_semver_from_text(text: &str) -> Option<semver::Version> {
    for token in text.split_whitespace() {
        let token = token.trim_matches(|ch: char| {
            !ch.is_ascii_alphanumeric() && !matches!(ch, '.' | '-' | '+' | '_')
        });
        let token = token.strip_prefix('v').unwrap_or(token);
        if token.is_empty() {
            continue;
        }
        if let Ok(version) = semver::Version::parse(token) {
            return Some(version);
        }
    }
    None
}
