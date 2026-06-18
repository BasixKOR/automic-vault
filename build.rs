fn main() {
    println!("cargo:rustc-check-cfg=cfg(coverage)");
    println!("cargo:rerun-if-env-changed=NUKE_BUILD_ID");
    println!("cargo:rerun-if-env-changed=AV_DOTENV_KEYCHAIN_ACCESS_GROUP");
    if packaged_combined_db_enabled() {
        prepare_packaged_combined_db();
    }
    let build_id = build_id();
    println!("cargo:rustc-env=NUKE_BUILD_ID={build_id}");
    println!(
        "cargo:rustc-env=NUKE_CODESIGN_IDENTITY={}",
        codesign_identity().unwrap_or_default()
    );
    println!(
        "cargo:rustc-env=AV_DOTENV_KEYCHAIN_ACCESS_GROUP={}",
        dotenv_keychain_access_group()
    );
    generate_isotope_integrations();

    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("macos") {
        return;
    }

    println!("cargo:rerun-if-changed=src/helper/xpc_bridge.m");
    println!("cargo:rerun-if-changed=src/lib/rs/isotope_keychain.m");
    println!("cargo:rerun-if-changed=src/helper/Info.plist");
    println!("cargo:rerun-if-changed=src/helper/launchd.plist");
    println!("cargo:rerun-if-changed=.env");
    println!("cargo:rerun-if-env-changed=APPLE_TEAM_ID");
    println!("cargo:rerun-if-env-changed=AV_DOTENV_KEYCHAIN_ACCESS_GROUP");
    println!("cargo:rerun-if-env-changed=CODESIGN_IDENTITY");
    println!("cargo:rerun-if-env-changed=MIN_MACOS_VERSION");
    println!("cargo:rerun-if-env-changed=NUKE_PROTOCOL_VERSION");
    println!("cargo:rerun-if-env-changed=TEAM_COMMON_NAME");
    println!("cargo:rerun-if-env-changed=TEAM_IDENTIFIER");
    println!("cargo:rerun-if-env-changed=NUKE_HELPER_VERSION");

    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let info_plist = helper_info_plist(&manifest_dir);
    let launchd_plist = format!("{manifest_dir}/src/helper/launchd.plist");

    cc::Build::new()
        .file("src/helper/xpc_bridge.m")
        .flag("-fobjc-arc")
        .compile("nuke-helper-xpc");
    cc::Build::new()
        .file("src/lib/rs/isotope_keychain.m")
        .flag("-fobjc-arc")
        .compile("isotope-keychain");

    println!(
        "cargo:rustc-link-arg-bin=nuke-helper=-Wl,-sectcreate,__TEXT,__info_plist,{info_plist}"
    );
    println!(
        "cargo:rustc-link-arg-bin=nuke-helper=-Wl,-sectcreate,__TEXT,__launchd_plist,{launchd_plist}"
    );

    for framework in ["Foundation", "ServiceManagement", "Security"] {
        println!("cargo:rustc-link-lib=framework={framework}");
    }
}

fn packaged_combined_db_enabled() -> bool {
    std::env::var_os("CARGO_FEATURE_PACKAGED_DB").is_some()
}

fn dotenv_keychain_access_group() -> String {
    if let Some(group) = non_empty_build_env_var("AV_DOTENV_KEYCHAIN_ACCESS_GROUP") {
        return group;
    }
    let team_id = non_empty_build_env_var("APPLE_TEAM_ID")
        .and_then(valid_team_identifier)
        .or_else(|| non_empty_build_env_var("TEAM_IDENTIFIER").and_then(valid_team_identifier))
        .or_else(|| {
            codesign_identity().and_then(|identity| team_identifier_from_identity(&identity))
        })
        .or_else(default_team_identifier_from_keychain)
        .unwrap_or_else(|| {
            panic!(
                "Unable to determine Apple team identifier for AV_DOTENV_KEYCHAIN_ACCESS_GROUP; set AV_DOTENV_KEYCHAIN_ACCESS_GROUP, APPLE_TEAM_ID, TEAM_IDENTIFIER, or configure a signing identity"
            )
        });
    format!("{team_id}.com.automicvault.dotenv")
}

fn valid_team_identifier(value: String) -> Option<String> {
    if value
        .chars()
        .all(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit())
        && !value.is_empty()
    {
        Some(value)
    } else {
        None
    }
}

fn team_identifier_from_identity(identity: &str) -> Option<String> {
    let close = identity.rfind(')')?;
    let open = identity[..close].rfind('(')?;
    let candidate = &identity[open + 1..close];
    if candidate
        .chars()
        .all(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit())
        && !candidate.is_empty()
    {
        Some(candidate.to_string())
    } else {
        None
    }
}

fn default_team_identifier_from_keychain() -> Option<String> {
    let output = std::process::Command::new("security")
        .args(["find-identity", "-v", "-p", "codesigning"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .find(|line| line.contains("\"Developer ID Application:"))
        .and_then(team_identifier_from_identity)
}

fn prepare_packaged_combined_db() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let repo_root = std::path::Path::new(&manifest_dir);
    println!("cargo:rerun-if-env-changed=AV_COMBINED_DB_PATH");
    let source = path_env_or_default(
        "AV_COMBINED_DB_PATH",
        repo_root.join("../av.db/cache/automic-vault/combined.json"),
    );
    println!("cargo:rerun-if-changed={}", source.display());

    if !source.is_file() {
        panic!(
            "packaged release builds require a combined package database at {}. Generate it in ../av.db or set AV_COMBINED_DB_PATH.",
            source.display()
        );
    }

    let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR not set");
    let output_path = std::path::Path::new(&out_dir).join("combined.json");
    if let Err(err) = std::fs::copy(&source, &output_path) {
        panic!(
            "failed to prepare packaged combined database from {}: {err}",
            source.display()
        );
    }
}

fn build_env_var(key: &str) -> Option<String> {
    std::env::var(key)
        .ok()
        .or_else(|| env_file_value(key))
        .map(unquote_env_value)
}

fn codesign_identity() -> Option<String> {
    non_empty_build_env_var("CODESIGN_IDENTITY")
        .map(normalize_codesign_identity)
        .or_else(|| {
            let common_name = non_empty_build_env_var("TEAM_COMMON_NAME")?;
            let team_identifier = non_empty_build_env_var("TEAM_IDENTIFIER")?;
            Some(normalize_codesign_identity(format!(
                "{common_name} ({team_identifier})"
            )))
        })
}

fn normalize_codesign_identity(identity: String) -> String {
    if identity == "-" || identity.contains(':') {
        return identity;
    }
    format!("Developer ID Application: {identity}")
}

fn non_empty_build_env_var(key: &str) -> Option<String> {
    build_env_var(key).filter(|value| !value.is_empty())
}

fn unquote_env_value(value: String) -> String {
    let bytes = value.as_bytes();
    if bytes.len() >= 2
        && ((bytes[0] == b'"' && bytes[bytes.len() - 1] == b'"')
            || (bytes[0] == b'\'' && bytes[bytes.len() - 1] == b'\''))
    {
        value[1..value.len() - 1].to_string()
    } else {
        value
    }
}

fn env_file_value(wanted_key: &str) -> Option<String> {
    if !matches!(
        wanted_key,
        "MIN_MACOS_VERSION" | "NUKE_PROTOCOL_VERSION" | "NUKE_HELPER_VERSION"
    ) {
        return None;
    }

    let Ok(contents) = std::fs::read_to_string(".env") else {
        return None;
    };

    for line in contents.lines() {
        let line = line.trim_end_matches('\r');
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        if key == wanted_key {
            return Some(value.to_string());
        }
    }

    None
}

fn generate_isotope_integrations() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let repo_root = std::path::Path::new(&manifest_dir);
    println!("cargo:rerun-if-env-changed=AUTOMIC_VAULT_REPO_CACHE");
    println!("cargo:rerun-if-env-changed=AUTOMIC_VAULT_RADIOISOTOPES_REPO");
    println!("cargo:rerun-if-env-changed=AUTOMIC_VAULT_INCLUDE_ISOTOPE_TESTS");
    println!("cargo:rerun-if-env-changed=CARGO_CFG_COVERAGE");
    let default_isotope_root = absolute_path(repo_root.join("../isotopes"));
    let default_radioisotope_root = absolute_path(repo_root.join("../radioisotopes"));
    let isotope_root =
        path_env_or_default("AUTOMIC_VAULT_REPO_CACHE", default_isotope_root.clone());
    let radioisotope_root = path_env_or_default(
        "AUTOMIC_VAULT_RADIOISOTOPES_REPO",
        default_radioisotope_root.clone(),
    );
    let isotope_roots = [isotope_root.clone(), radioisotope_root.clone()];
    let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR not set");
    let generated_sources_dir = std::path::Path::new(&out_dir).join("isotope-generated");
    let output_path = std::path::Path::new(&out_dir).join("isotope_integrations.rs");
    let include_isotope_tests = include_isotope_tests_for_coverage();
    let coverage_path_aliases = if include_isotope_tests {
        coverage_path_aliases(
            repo_root,
            &isotope_root,
            &default_isotope_root,
            &radioisotope_root,
            &default_radioisotope_root,
        )
    } else {
        Vec::new()
    };
    let generated_radioisotope_root =
        coverage_include_root(&radioisotope_root, &coverage_path_aliases);
    println!(
        "cargo:rustc-env=AUTOMIC_VAULT_GENERATED_RADIOISOTOPES_REPO={}",
        generated_radioisotope_root.display()
    );

    let mut entries = Vec::new();
    for root in isotope_roots {
        println!("cargo:rerun-if-changed={}", root.display());
        collect_isotope_integrations(&root, &mut entries);
    }

    entries.sort_by(|left, right| left.isotope_name.cmp(&right.isotope_name));
    entries.dedup_by(|left, right| left.isotope_name == right.isotope_name);
    let mut output = String::from(concat!(
        "#[cfg(not(target_os = \"macos\"))]\n",
        "compile_error!(\"radioisotope av-inject shebang wrappers are macOS-only for now\");\n\n",
        "pub(crate) struct IsotopeIntegration {\n",
        "  pub(crate) name: &'static str,\n",
        "  pub(crate) detect: Option<fn() -> Result<bool, String>>,\n",
        "  pub(crate) detect_reasons: Option<fn() -> Result<Vec<String>, String>>,\n",
        "  pub(crate) migrate: Option<fn() -> Result<(), String>>,\n",
        "  pub(crate) post_install: Option<fn() -> Result<(), String>>,\n",
        "  pub(crate) post_install_for_formula: Option<fn(&str) -> Result<(), String>>,\n",
        "  pub(crate) has_detect: bool,\n",
        "  pub(crate) has_migration: bool,\n",
        "  pub(crate) has_install_remediation: bool,\n",
        "  pub(crate) credential_helper_name: Option<&'static str>,\n",
        "  pub(crate) credential_helper: Option<for<'a> fn(crate::isotope::CredentialHelperInvocation<'a>) -> Result<(), String>>,\n",
        "}\n\n",
    ));

    for entry in &entries {
        output.push_str(&format!("mod {} {{\n", entry.module_name));
        if let Some(path) = &entry.detect_path {
            let include_path = isotope_include_path(
                include_isotope_tests,
                &generated_sources_dir,
                &entry.module_name,
                "detect",
                path,
                &coverage_path_aliases,
            );
            output.push_str(&format!(
                "  #[allow(clippy::all, dead_code, unused_parens, unused_variables)] pub(crate) mod detect {{ include!(r#\"{}\"#); }}\n",
                include_path.display()
            ));
        }
        if let Some(path) = &entry.migrate_path {
            let include_path = isotope_include_path(
                include_isotope_tests,
                &generated_sources_dir,
                &entry.module_name,
                "migrate",
                path,
                &coverage_path_aliases,
            );
            output.push_str(&format!(
                "  #[allow(clippy::all, dead_code, unused_parens, unused_variables)] pub(crate) mod migrate {{ include!(r#\"{}\"#); }}\n",
                include_path.display()
            ));
        }
        if let Some(path) = &entry.post_install_path {
            let include_path = isotope_include_path(
                include_isotope_tests,
                &generated_sources_dir,
                &entry.module_name,
                "post_install",
                path,
                &coverage_path_aliases,
            );
            output.push_str(&format!(
                "  #[allow(clippy::all, dead_code, unused_parens, unused_variables)] pub(crate) mod post_install {{ include!(r#\"{}\"#); }}\n",
                include_path.display()
            ));
        }
        if let Some(path) = &entry.credential_helper_path {
            let include_path = isotope_include_path(
                include_isotope_tests,
                &generated_sources_dir,
                &entry.module_name,
                "credential_helper",
                path,
                &coverage_path_aliases,
            );
            output.push_str(&format!(
                "  #[allow(clippy::all, dead_code, unused_parens, unused_variables)] pub(crate) mod credential_helper {{ include!(r#\"{}\"#); }}\n",
                include_path.display()
            ));
        }
        output.push_str("}\n\n");
    }

    output.push_str("pub(crate) static INTEGRATIONS: &[IsotopeIntegration] = &[\n");
    for entry in &entries {
        let detect = if entry.detect_path.is_some() {
            format!("Some({}::detect::install_is_insecure)", entry.module_name)
        } else {
            "None".to_string()
        };
        let detect_reasons = if entry.has_detect_reasons {
            format!(
                "Some({}::detect::install_insecurity_reasons)",
                entry.module_name
            )
        } else {
            "None".to_string()
        };
        let migrate = if entry.migrate_path.is_some() {
            format!("Some({}::migrate::migrate_credentials)", entry.module_name)
        } else {
            "None".to_string()
        };
        let post_install = if entry.post_install_path.is_some() {
            format!("Some({}::post_install::post_install)", entry.module_name)
        } else {
            "None".to_string()
        };
        let post_install_for_formula = if entry.has_post_install_for_formula {
            format!(
                "Some({}::post_install::post_install_for_formula)",
                entry.module_name
            )
        } else {
            "None".to_string()
        };
        let credential_helper_name = if entry.credential_helper_path.is_some() {
            format!("Some({}::credential_helper::NAME)", entry.module_name)
        } else {
            "None".to_string()
        };
        let credential_helper = if entry.credential_helper_path.is_some() {
            format!(
                "Some({}::credential_helper::credential_helper)",
                entry.module_name
            )
        } else {
            "None".to_string()
        };
        output.push_str(&format!(
            concat!(
                "  IsotopeIntegration {{ name: {:?}, detect: {}, detect_reasons: {}, ",
                "migrate: {}, post_install: {}, post_install_for_formula: {}, ",
                "has_detect: {}, has_migration: {}, ",
                "has_install_remediation: {}, credential_helper_name: {}, credential_helper: {} }},\n"
            ),
            entry.isotope_name,
            detect,
            detect_reasons,
            migrate,
            post_install,
            post_install_for_formula,
            entry.detect_path.is_some(),
            entry.migrate_path.is_some(),
            entry.post_install_path.is_some(),
            credential_helper_name,
            credential_helper
        ));
    }
    output.push_str("];\n");

    write_if_changed(&output_path, &output)
        .unwrap_or_else(|err| panic!("failed to write {}: {err}", output_path.display()));
}

fn path_env_or_default(key: &str, default: std::path::PathBuf) -> std::path::PathBuf {
    let path = std::env::var_os(key)
        .filter(|value| !value.is_empty())
        .map(std::path::PathBuf::from)
        .unwrap_or(default);
    absolute_path(path)
}

fn absolute_path(path: std::path::PathBuf) -> std::path::PathBuf {
    if path.is_absolute() {
        return path;
    }

    std::env::current_dir()
        .unwrap_or_else(|err| panic!("failed to resolve current directory: {err}"))
        .join(path)
}

fn include_isotope_tests_for_coverage() -> bool {
    // cargo-llvm-cov sets cfg(coverage). The explicit env var keeps CI and
    // automation runs readable while preserving the cargo-llvm-cov default.
    std::env::var_os("CARGO_CFG_COVERAGE").is_some()
        || env_flag("AUTOMIC_VAULT_INCLUDE_ISOTOPE_TESTS")
}

fn env_flag(key: &str) -> bool {
    std::env::var_os(key).is_some_and(|value| {
        let value = value.to_string_lossy();
        !value.is_empty() && value != "0" && value != "false"
    })
}

struct CoveragePathAlias {
    source_root: std::path::PathBuf,
    include_root: std::path::PathBuf,
}

fn coverage_path_aliases(
    repo_root: &std::path::Path,
    isotope_root: &std::path::Path,
    default_isotope_root: &std::path::Path,
    radioisotope_root: &std::path::Path,
    default_radioisotope_root: &std::path::Path,
) -> Vec<CoveragePathAlias> {
    [
        coverage_path_alias(
            isotope_root,
            default_isotope_root,
            repo_root.join("data/isotopes"),
        ),
        coverage_path_alias(
            radioisotope_root,
            default_radioisotope_root,
            repo_root.join("data/radioisotopes/checkout"),
        ),
    ]
    .into_iter()
    .flatten()
    .collect()
}

fn coverage_path_alias(
    source_root: &std::path::Path,
    default_root: &std::path::Path,
    include_root: std::path::PathBuf,
) -> Option<CoveragePathAlias> {
    if source_root != default_root {
        return None;
    }
    ensure_coverage_include_alias(source_root, &include_root);
    Some(CoveragePathAlias {
        source_root: source_root.to_path_buf(),
        include_root,
    })
}

fn ensure_coverage_include_alias(source_root: &std::path::Path, include_root: &std::path::Path) {
    match std::fs::symlink_metadata(include_root) {
        Ok(metadata) => {
            validate_coverage_include_alias(source_root, include_root, &metadata);
            return;
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
        Err(err) => panic!(
            "failed to inspect coverage include path {}: {err}",
            include_root.display()
        ),
    }
    let Some(parent) = include_root.parent() else {
        return;
    };
    std::fs::create_dir_all(parent)
        .unwrap_or_else(|err| panic!("failed to create {}: {err}", parent.display()));
    #[cfg(unix)]
    std::os::unix::fs::symlink(source_root, include_root).unwrap_or_else(|err| {
        panic!(
            "failed to link coverage include path {} to {}: {err}",
            include_root.display(),
            source_root.display()
        )
    });
    #[cfg(not(unix))]
    panic!(
        "coverage include path aliasing requires Unix symlinks: {} -> {}",
        include_root.display(),
        source_root.display()
    );
}

fn validate_coverage_include_alias(
    source_root: &std::path::Path,
    include_root: &std::path::Path,
    metadata: &std::fs::Metadata,
) {
    if !metadata.file_type().is_symlink() {
        panic!(
            "coverage include path {} already exists but is not a symlink to {}",
            include_root.display(),
            source_root.display()
        );
    }

    let expected = canonical_coverage_alias_path(source_root, "coverage source root");
    let actual = canonical_coverage_alias_path(include_root, "coverage include path");
    if actual != expected {
        let target = std::fs::read_link(include_root)
            .map(|path| path.display().to_string())
            .unwrap_or_else(|err| format!("<unreadable: {err}>"));
        panic!(
            "coverage include path {} points to {} (resolved to {}) but expected {}",
            include_root.display(),
            target,
            actual.display(),
            expected.display()
        );
    }
}

fn canonical_coverage_alias_path(path: &std::path::Path, description: &str) -> std::path::PathBuf {
    std::fs::canonicalize(path)
        .unwrap_or_else(|err| panic!("failed to resolve {description} {}: {err}", path.display()))
}

fn coverage_include_path(
    source_path: &std::path::Path,
    aliases: &[CoveragePathAlias],
) -> Option<std::path::PathBuf> {
    aliases.iter().find_map(|alias| {
        source_path
            .strip_prefix(&alias.source_root)
            .ok()
            .map(|relative| alias.include_root.join(relative))
    })
}

fn coverage_include_root<'a>(
    source_root: &'a std::path::Path,
    aliases: &'a [CoveragePathAlias],
) -> &'a std::path::Path {
    aliases
        .iter()
        .find(|alias| alias.source_root == source_root)
        .map(|alias| alias.include_root.as_path())
        .unwrap_or(source_root)
}

fn isotope_include_path(
    include_isotope_tests: bool,
    generated_sources_dir: &std::path::Path,
    module_name: &str,
    suffix: &str,
    source_path: &std::path::Path,
    coverage_path_aliases: &[CoveragePathAlias],
) -> std::path::PathBuf {
    if include_isotope_tests {
        return coverage_include_path(source_path, coverage_path_aliases)
            .unwrap_or_else(|| source_path.to_path_buf());
    }

    sanitized_isotope_source(generated_sources_dir, module_name, suffix, source_path)
}

fn sanitized_isotope_source(
    generated_sources_dir: &std::path::Path,
    module_name: &str,
    suffix: &str,
    source_path: &std::path::Path,
) -> std::path::PathBuf {
    let contents = std::fs::read_to_string(source_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", source_path.display()));
    let output_path = generated_sources_dir.join(format!("{module_name}-{suffix}.rs"));
    let sanitized = strip_cfg_test_modules(&contents);
    write_if_changed(&output_path, &sanitized)
        .unwrap_or_else(|err| panic!("failed to write {}: {err}", output_path.display()));
    output_path
}

fn strip_cfg_test_modules(contents: &str) -> String {
    let mut output = String::new();
    let mut lines = contents.lines();

    while let Some(line) = lines.next() {
        if line.trim() == "#[cfg(test)]" {
            let Some(next_line) = lines.next() else {
                output.push_str(line);
                output.push('\n');
                break;
            };
            if next_line.trim_start().starts_with("mod tests {") {
                let mut depth = brace_delta(next_line);
                while depth > 0 {
                    let Some(module_line) = lines.next() else {
                        break;
                    };
                    depth += brace_delta(module_line);
                }
                continue;
            }
            output.push_str(line);
            output.push('\n');
            output.push_str(next_line);
            output.push('\n');
            continue;
        }

        output.push_str(line);
        output.push('\n');
    }

    output
}

fn brace_delta(line: &str) -> i32 {
    let opens = line.chars().filter(|&ch| ch == '{').count() as i32;
    let closes = line.chars().filter(|&ch| ch == '}').count() as i32;
    opens - closes
}

fn collect_isotope_integrations(
    root: &std::path::Path,
    entries: &mut Vec<IsotopeIntegrationInput>,
) {
    let Ok(children) = std::fs::read_dir(root) else {
        return;
    };
    for child in children.flatten() {
        let repo_dir = child.path();
        if !repo_dir.is_dir() {
            continue;
        }
        let Some(dir_name) = repo_dir.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if dir_name.starts_with('.') {
            continue;
        }
        println!("cargo:rerun-if-changed={}", repo_dir.display());
        let manifest_path = repo_dir.join("automic-vault.yml");
        let Ok(manifest) = std::fs::read_to_string(&manifest_path) else {
            continue;
        };
        println!("cargo:rerun-if-changed={}", manifest_path.display());
        let Some(isotope_name) = isotope_name_from_manifest(&manifest) else {
            continue;
        };

        let detect_path = repo_dir.join("detect.rs");
        let migrate_path = repo_dir.join("migrate.rs");
        let post_install_path = repo_dir.join("post-install.rs");
        let credential_helper_path = repo_dir.join("credential-helper.rs");
        if !detect_path.exists()
            && !migrate_path.exists()
            && !post_install_path.exists()
            && !credential_helper_path.exists()
        {
            continue;
        }

        if detect_path.exists() {
            println!("cargo:rerun-if-changed={}", detect_path.display());
        }
        if migrate_path.exists() {
            println!("cargo:rerun-if-changed={}", migrate_path.display());
        }
        if post_install_path.exists() {
            println!("cargo:rerun-if-changed={}", post_install_path.display());
        }
        if credential_helper_path.exists() {
            println!(
                "cargo:rerun-if-changed={}",
                credential_helper_path.display()
            );
        }
        let has_detect_reasons = detect_path
            .exists()
            .then(|| std::fs::read_to_string(&detect_path).unwrap_or_default())
            .is_some_and(|contents| contents.contains("pub fn install_insecurity_reasons"));
        let has_post_install_for_formula = post_install_path
            .exists()
            .then(|| std::fs::read_to_string(&post_install_path).unwrap_or_default())
            .is_some_and(|contents| contents.contains("pub fn post_install_for_formula"));

        entries.push(IsotopeIntegrationInput {
            module_name: rust_module_name(&isotope_name),
            isotope_name,
            detect_path: detect_path.exists().then_some(detect_path),
            has_detect_reasons,
            migrate_path: migrate_path.exists().then_some(migrate_path),
            post_install_path: post_install_path.exists().then_some(post_install_path),
            has_post_install_for_formula,
            credential_helper_path: credential_helper_path
                .exists()
                .then_some(credential_helper_path),
        });
    }
}

struct IsotopeIntegrationInput {
    module_name: String,
    isotope_name: String,
    detect_path: Option<std::path::PathBuf>,
    has_detect_reasons: bool,
    migrate_path: Option<std::path::PathBuf>,
    post_install_path: Option<std::path::PathBuf>,
    has_post_install_for_formula: bool,
    credential_helper_path: Option<std::path::PathBuf>,
}

fn isotope_name_from_manifest(manifest: &str) -> Option<String> {
    let lines = manifest.lines().collect::<Vec<_>>();
    let mut index = 0;
    while index < lines.len() {
        let line = lines[index];
        if line.trim() == "name:" {
            index += 1;
            while index < lines.len() {
                let value = lines[index].trim();
                if value.is_empty() {
                    index += 1;
                    continue;
                }
                return value.strip_prefix("isotope:").map(str::to_string);
            }
        }
        if let Some(value) = line.trim().strip_prefix("name:") {
            return value.trim().strip_prefix("isotope:").map(str::to_string);
        }
        index += 1;
    }
    None
}

fn rust_module_name(value: &str) -> String {
    let mut output = String::from("isotope_");
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() {
            output.push(ch.to_ascii_lowercase());
        } else {
            output.push('_');
        }
    }
    output
}

fn build_id() -> String {
    if let Ok(value) = std::env::var("NUKE_BUILD_ID")
        && !value.is_empty()
    {
        return value;
    }

    track_git_head();
    git_build_id()
}

fn track_git_head() {
    println!("cargo:rerun-if-changed=.git/HEAD");
    if let Ok(head_path) = std::fs::read_to_string(".git/HEAD")
        && let Some(reference) = head_path.strip_prefix("ref: ")
    {
        println!("cargo:rerun-if-changed=.git/{}", reference.trim());
    }
}

fn git_build_id() -> String {
    let output = std::process::Command::new("git")
        .args(["rev-parse", "--short=12", "HEAD"])
        .output();
    match output {
        Ok(output) if output.status.success() => {
            let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if value.is_empty() {
                env!("CARGO_PKG_VERSION").to_string()
            } else {
                value
            }
        }
        _ => env!("CARGO_PKG_VERSION").to_string(),
    }
}

fn helper_info_plist(manifest_dir: &str) -> String {
    let template_path = format!("{manifest_dir}/src/helper/Info.plist");
    let template = std::fs::read_to_string(&template_path)
        .unwrap_or_else(|err| panic!("failed to read {template_path}: {err}"));
    let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR not set");
    let output_path = format!("{out_dir}/nuke-helper-Info.plist");
    let authorized_client_requirement = authorized_client_requirement();
    let helper_version = build_env_var("NUKE_HELPER_VERSION").expect("NUKE_HELPER_VERSION not set");

    let rendered = template
        .replace("@AUTHORIZED_CLIENT@", &authorized_client_requirement)
        .replace("@HELPER_VERSION@", &helper_version);

    write_if_changed(std::path::Path::new(&output_path), &rendered)
        .unwrap_or_else(|err| panic!("failed to write {output_path}: {err}"));

    output_path
}

fn write_if_changed(path: &std::path::Path, contents: &str) -> std::io::Result<()> {
    if std::fs::read_to_string(path).is_ok_and(|existing| existing == contents) {
        return Ok(());
    }

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    std::fs::write(path, contents)
}

fn authorized_client_requirement() -> String {
    let team_id = std::env::var("APPLE_TEAM_ID")
        .ok()
        .filter(|value| !value.is_empty())
        .or_else(team_id_from_codesign_identity);

    match team_id {
        Some(team_id) => {
            format!(
                "identifier \"com.automicvault\" and anchor apple generic and certificate leaf[subject.OU] = \"{team_id}\""
            )
        }
        None => String::from("identifier \"com.automicvault\" and anchor apple generic"),
    }
}

fn team_id_from_codesign_identity() -> Option<String> {
    let identity = codesign_identity()?;
    let open_paren = identity.rfind('(')?;
    let close_paren = identity.rfind(')')?;
    if close_paren <= open_paren + 1 {
        return None;
    }
    let team_id = identity[open_paren + 1..close_paren].trim();
    if team_id.is_empty() || !team_id.chars().all(|ch| ch.is_ascii_alphanumeric()) {
        return None;
    }
    Some(team_id.to_string())
}
