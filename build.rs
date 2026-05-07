fn main() {
    println!("cargo:rustc-check-cfg=cfg(coverage)");
    println!("cargo:rerun-if-changed=.git/HEAD");
    if let Ok(head_path) = std::fs::read_to_string(".git/HEAD") {
        if let Some(reference) = head_path.strip_prefix("ref: ") {
            println!("cargo:rerun-if-changed=.git/{}", reference.trim());
        }
    }
    println!("cargo:rustc-env=NUKE_BUILD_ID={}", build_id());
    println!(
        "cargo:rustc-env=NUKE_CODESIGN_IDENTITY={}",
        build_env_var("CODESIGN_IDENTITY").unwrap_or_default()
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
    println!("cargo:rerun-if-env-changed=CODESIGN_IDENTITY");
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

fn build_env_var(key: &str) -> Option<String> {
    std::env::var(key).ok().or_else(|| env_file_value(key))
}

fn env_file_value(wanted_key: &str) -> Option<String> {
    let Ok(contents) = std::fs::read_to_string(".env") else {
        return None;
    };

    for line in contents.lines() {
        let line = line.trim_end_matches('\r');
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        if key != wanted_key || !is_env_key(key) {
            continue;
        }

        return Some(value.to_string());
    }

    None
}

fn is_env_key(key: &str) -> bool {
    let mut chars = key.chars();
    match chars.next() {
        Some(first) if first == '_' || first.is_ascii_alphabetic() => {}
        _ => return false,
    }

    chars.all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
}

fn generate_isotope_integrations() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let repo_root = std::path::Path::new(&manifest_dir);
    let isotope_roots = [
        repo_root.join("data/isotopes"),
        repo_root.join("data/radioisotopes"),
    ];
    let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR not set");
    let output_path = std::path::Path::new(&out_dir).join("isotope_integrations.rs");

    let mut entries = Vec::new();
    for root in isotope_roots {
        println!("cargo:rerun-if-changed={}", root.display());
        collect_isotope_integrations(&root, &mut entries);
    }

    entries.sort_by(|left, right| left.isotope_name.cmp(&right.isotope_name));
    entries.dedup_by(|left, right| left.isotope_name == right.isotope_name);
    let mut output = String::from(concat!(
        "pub(crate) struct IsotopeIntegration {\n",
        "  pub(crate) name: &'static str,\n",
        "  pub(crate) detect: Option<fn() -> Result<bool, String>>,\n",
        "  pub(crate) migrate: Option<fn() -> Result<(), String>>,\n",
        "  pub(crate) post_install: Option<fn() -> Result<(), String>>,\n",
        "}\n\n",
    ));

    for entry in &entries {
        output.push_str(&format!("mod {} {{\n", entry.module_name));
        if let Some(path) = &entry.detect_path {
            output.push_str(&format!(
                "  #[allow(dead_code)] pub(crate) mod detect {{ include!(r#\"{}\"#); }}\n",
                path.display()
            ));
        }
        if let Some(path) = &entry.migrate_path {
            output.push_str(&format!(
                "  #[allow(dead_code)] pub(crate) mod migrate {{ include!(r#\"{}\"#); }}\n",
                path.display()
            ));
        }
        if let Some(path) = &entry.post_install_path {
            output.push_str(&format!(
                "  #[allow(dead_code)] pub(crate) mod post_install {{ include!(r#\"{}\"#); }}\n",
                path.display()
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
        output.push_str(&format!(
            concat!(
                "  IsotopeIntegration {{ name: {:?}, detect: {}, migrate: {}, ",
                "post_install: {} }},\n"
            ),
            entry.isotope_name, detect, migrate, post_install
        ));
    }
    output.push_str("];\n");

    write_if_changed(&output_path, &output)
        .unwrap_or_else(|err| panic!("failed to write {}: {err}", output_path.display()));
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
        if !detect_path.exists() && !migrate_path.exists() && !post_install_path.exists() {
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
        entries.push(IsotopeIntegrationInput {
            module_name: rust_module_name(&isotope_name),
            isotope_name,
            detect_path: detect_path.exists().then_some(detect_path),
            migrate_path: migrate_path.exists().then_some(migrate_path),
            post_install_path: post_install_path.exists().then_some(post_install_path),
        });
    }
}

struct IsotopeIntegrationInput {
    module_name: String,
    isotope_name: String,
    detect_path: Option<std::path::PathBuf>,
    migrate_path: Option<std::path::PathBuf>,
    post_install_path: Option<std::path::PathBuf>,
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
    let helper_version =
        build_env_var("NUKE_HELPER_VERSION").expect("NUKE_HELPER_VERSION not set; add it to .env");

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
    let identity = std::env::var("CODESIGN_IDENTITY").ok()?;
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
