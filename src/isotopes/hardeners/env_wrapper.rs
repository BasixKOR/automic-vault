use std::fs;
use std::io::{self, IsTerminal, Write};
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};

use super::{
    HardenerCommand, HardenerDetection, HardenerMetadata, RequiredExecutable, RequiredIdentity,
    SecretGateDescriptor, SecretGateRoute, StubRequirements,
};

const MARKER: &str = "AUTOMIC_VAULT_ENV_WRAPPER_STUB_V1";
const STUB_DIR: &str = "/usr/local/bin";
const DOCUMENTATION: &str = "# Environment Wrapper\n\nInstalls a small launcher stub that runs the target tool through `av inject --allow-missing-keys` with the migrated isotope keys. This does not migrate existing plaintext credentials; run `av scan` after hardening to find anything still on disk.\n";

unsafe extern "C" {
    fn geteuid() -> u32;
}

pub(crate) fn run_target(
    target: &str,
    stdout: &mut dyn Write,
    yes: bool,
) -> Option<Result<(), String>> {
    Some(run(wrapper(target)?, stdout, yes))
}

pub(crate) fn metadata() -> Vec<HardenerMetadata> {
    WRAPPERS
        .iter()
        .map(|wrapper| HardenerMetadata {
            name: wrapper.name,
            documentation: DOCUMENTATION,
            detection: detect(wrapper),
            secret_gate: Some(secret_gate(wrapper)),
        })
        .collect()
}

fn secret_gate(wrapper: &EnvWrapper) -> SecretGateDescriptor {
    let routes = stubs(wrapper)
        .map(|stub| SecretGateRoute {
            operation: "inject",
            script_path: Some(stub_path(stub.command).display().to_string()),
            target_path: "/bin/sh".to_string(),
            caller_identifiers: vec!["com.automicvault.av"],
            key_patterns: stub.keys.iter().map(|key| (*key).to_string()).collect(),
            replace_existing_env: false,
            allow_missing_keys: true,
        })
        .collect::<Vec<_>>();
    let mut key_patterns = routes
        .iter()
        .flat_map(|route| route.key_patterns.iter().cloned())
        .collect::<Vec<_>>();
    key_patterns.sort();
    key_patterns.dedup();
    SecretGateDescriptor {
        id: wrapper.name,
        key_patterns,
        routes,
    }
}

fn run(wrapper: &EnvWrapper, stdout: &mut dyn Write, yes: bool) -> Result<(), String> {
    if effective_uid() != 0 {
        return Err(format!("run `sudo av harden {}`", wrapper.name));
    }
    for stub in stubs(wrapper) {
        let target = target_path(stub);
        if !target.exists() {
            return Err(format!(
                "{} is not installed at {}",
                wrapper.name,
                target.display()
            ));
        }
        let stub_path = stub_path(stub.command);
        if stub_path.exists() && !is_managed_stub(&stub_path, stub) {
            return Err(format!(
                "{} already exists and is not an Automic Vault env-wrapper stub",
                stub_path.display()
            ));
        }
    }

    writeln!(stdout, "╭─ harden {}", wrapper.name).ok();
    writeln!(stdout, "│").ok();
    for stub in stubs(wrapper) {
        writeln!(stdout, "├─ write {}", stub_path(stub.command).display()).ok();
    }
    writeln!(stdout, "│").ok();
    if !confirm(stdout, yes)? {
        writeln!(stdout, "╰─ cancelled").ok();
        return Ok(());
    }

    for stub in stubs(wrapper) {
        install_stub(stub)?;
    }
    writeln!(stdout, "╰─ hardened {}", wrapper.name).ok();
    Ok(())
}

fn detect(wrapper: &EnvWrapper) -> HardenerDetection {
    let commands = stubs(wrapper)
        .map(|stub| {
            let path = stub_path(stub.command);
            let stub_valid = is_current_stub(&path, stub);
            HardenerCommand {
                name: stub.command.to_string(),
                hardened: stub_valid,
                stub_valid,
                stub_path: Some(path.display().to_string()),
                target_path: target_path(stub).display().to_string(),
                required_paths: if test_stub_dir().is_some() {
                    Vec::new()
                } else {
                    vec![
                        RequiredExecutable {
                            name: "Automic Vault CLI",
                            path: "/usr/local/bin/av".to_string(),
                        },
                        RequiredExecutable {
                            name: "POSIX shell",
                            path: "/bin/sh".to_string(),
                        },
                    ]
                },
                stub_requirements: Some(root_stub_requirements(&path)),
                injected_keys: stub.keys.iter().map(|key| (*key).to_string()).collect(),
                assignment_keys: stub
                    .assignment_keys
                    .iter()
                    .map(|key| (*key).to_string())
                    .collect(),
            }
        })
        .collect::<Vec<_>>();
    let hardened = commands.iter().all(|command| command.hardened);
    HardenerDetection::commands(hardened, commands)
}

fn root_stub_requirements(path: &Path) -> StubRequirements {
    let test_ids = test_stub_dir().and_then(|_| {
        path.parent()
            .and_then(|parent| parent.metadata().ok())
            .map(|metadata| (metadata.uid(), metadata.gid()))
    });
    let (uid, gid) = test_ids.unwrap_or((0, 0));
    StubRequirements {
        mode: 0o755,
        owner: RequiredIdentity {
            name: if test_ids.is_some() {
                "test user"
            } else {
                "root"
            },
            id: Some(uid),
        },
        group: RequiredIdentity {
            name: if test_ids.is_some() {
                "test group"
            } else {
                "wheel"
            },
            id: Some(gid),
        },
    }
}

fn confirm(stdout: &mut dyn Write, yes: bool) -> Result<bool, String> {
    if yes {
        writeln!(stdout, "◇ Continue? yes (--yes)").ok();
        return Ok(true);
    }

    write!(stdout, "◇ Continue? [y/N] ").ok();
    stdout
        .flush()
        .map_err(|err| format!("failed to flush prompt: {err}"))?;
    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .map_err(|err| format!("failed to read confirmation: {err}"))?;
    if !io::stdin().is_terminal() {
        writeln!(stdout).ok();
    }
    Ok(matches!(
        input.trim().to_ascii_lowercase().as_str(),
        "y" | "yes"
    ))
}

fn install_stub(stub: &StubSpec) -> Result<(), String> {
    let path = stub_path(stub.command);
    fs::write(&path, stub_script(stub))
        .map_err(|err| format!("failed to write {}: {err}", path.display()))?;
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755))
        .map_err(|err| format!("failed to chmod {}: {err}", path.display()))
}

fn is_managed_stub(path: &Path, stub: &StubSpec) -> bool {
    fs::read_to_string(path)
        .map(|contents| {
            contents.contains(MARKER)
                && contents.contains(&format!(
                    "original='{}'",
                    shell_single_argument(&target_path(stub))
                ))
        })
        .unwrap_or(false)
}

fn is_current_stub(path: &Path, stub: &StubSpec) -> bool {
    fs::read_to_string(path).is_ok_and(|contents| contents == stub_script(stub))
}

fn stub_script(stub: &StubSpec) -> String {
    let keys = stub
        .keys
        .iter()
        .map(|key| format!(" +{key}"))
        .collect::<String>();
    let mut script = format!(
        "#!/usr/local/bin/av inject --allow-missing-keys{keys} /bin/sh\n\
set -eu\n\
# {MARKER}\n\
original='{}'\n",
        shell_single_argument(&target_path(stub))
    );
    for key in stub.assignment_keys {
        script.push_str(&format!(
            "if [ -n \"${{{key}:-}}\" ]; then\n  old_ifs=\"$IFS\"\n  IFS='\n'\n  for assignment in ${{{key}-}}; do\n    [ -n \"$assignment\" ] || continue\n    export \"$assignment\"\n  done\n  IFS=\"$old_ifs\"\nfi\n"
        ));
    }
    script.push_str("exec \"$original\" \"$@\"\n");
    script
}

fn shell_single_argument(path: &Path) -> String {
    path.to_string_lossy().replace('\'', r#"'\''"#)
}

fn wrapper(name: &str) -> Option<&'static EnvWrapper> {
    WRAPPERS.iter().find(|wrapper| wrapper.name == name)
}

fn target_path(stub: &StubSpec) -> PathBuf {
    test_target_dir()
        .map(|dir| dir.join(stub.command))
        .unwrap_or_else(|| PathBuf::from(stub.target))
}

fn stub_path(command: &str) -> PathBuf {
    test_stub_dir()
        .unwrap_or_else(|| PathBuf::from(STUB_DIR))
        .join(command)
}

fn test_target_dir() -> Option<PathBuf> {
    std::env::var_os("AUTOMIC_VAULT_TEST_ENV_WRAPPER_TARGET_DIR").map(PathBuf::from)
}

fn test_stub_dir() -> Option<PathBuf> {
    std::env::var_os("AUTOMIC_VAULT_TEST_ENV_WRAPPER_STUB_DIR").map(PathBuf::from)
}

fn effective_uid() -> u32 {
    std::env::var("AUTOMIC_VAULT_TEST_EUID")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or_else(|| unsafe { geteuid() })
}

#[derive(Clone, Copy)]
struct EnvWrapper {
    name: &'static str,
    primary: StubSpec,
    extra: &'static [StubSpec],
}

#[derive(Clone, Copy)]
struct StubSpec {
    command: &'static str,
    target: &'static str,
    keys: &'static [&'static str],
    assignment_keys: &'static [&'static str],
}

fn stubs(wrapper: &EnvWrapper) -> impl Iterator<Item = &StubSpec> {
    std::iter::once(&wrapper.primary).chain(wrapper.extra.iter())
}

const JFROG_EXTRA: &[StubSpec] = &[stub(
    "jfrog",
    "/opt/jfrog-cli/bin/jfrog",
    &["JFROG_ENV_ASSIGNMENTS"],
    &["JFROG_ENV_ASSIGNMENTS"],
)];

const WRAPPERS: &[EnvWrapper] = &[
    one(
        "akamai",
        "akamai",
        "/opt/akamai/bin/akamai",
        &["AKAMAI_ENV_ASSIGNMENTS"],
        &["AKAMAI_ENV_ASSIGNMENTS"],
    ),
    one(
        "algolia",
        "algolia",
        "/opt/algolia/bin/algolia",
        &["ALGOLIA_ENV_ASSIGNMENTS"],
        &["ALGOLIA_ENV_ASSIGNMENTS"],
    ),
    one(
        "argocd",
        "argocd",
        "/opt/argocd/bin/argocd",
        &["ARGOCD_AUTH_TOKEN"],
        &[],
    ),
    one(
        "ast-cli",
        "cx",
        "/opt/ast-cli/bin/cx",
        &["CX_APIKEY", "CX_CLIENT_SECRET"],
        &[],
    ),
    one("buf", "buf", "/opt/buf/bin/buf", &["BUF_TOKEN"], &[]),
    one(
        "censys",
        "censys",
        "/opt/censys/bin/censys",
        &["CENSYS_API_ID", "CENSYS_API_SECRET", "CENSYS_ASM_API_KEY"],
        &[],
    ),
    one(
        "checkov",
        "checkov",
        "/opt/checkov/bin/checkov",
        &["BC_API_KEY"],
        &[],
    ),
    one(
        "circleci",
        "circleci",
        "/opt/circleci/bin/circleci",
        &["CIRCLECI_CLI_TOKEN"],
        &[],
    ),
    one("civo", "civo", "/opt/civo/bin/civo", &["CIVO_TOKEN"], &[]),
    one(
        "cloudsmith-cli",
        "cloudsmith",
        "/opt/cloudsmith-cli/bin/cloudsmith",
        &["CLOUDSMITH_API_KEY"],
        &[],
    ),
    one(
        "composer",
        "composer",
        "/opt/composer/bin/composer",
        &["COMPOSER_AUTH"],
        &[],
    ),
    one(
        "doctl",
        "doctl",
        "/opt/doctl/bin/doctl",
        &["DIGITALOCEAN_ACCESS_TOKEN"],
        &[],
    ),
    one(
        "flyctl",
        "flyctl",
        "/opt/flyctl/bin/flyctl",
        &["FLY_ACCESS_TOKEN"],
        &[],
    ),
    one(
        "glab",
        "glab",
        "/opt/glab/bin/glab",
        &["GLAB_ENV_ASSIGNMENTS"],
        &["GLAB_ENV_ASSIGNMENTS"],
    ),
    one(
        "gotify",
        "gotify",
        "/opt/gotify/bin/gotify",
        &["GOTIFY_TOKEN"],
        &[],
    ),
    one(
        "gptcommit",
        "gptcommit",
        "/opt/gptcommit/bin/gptcommit",
        &["GPTCOMMIT__OPENAI__API_KEY"],
        &[],
    ),
    one(
        "grafanactl",
        "grafanactl",
        "/opt/grafanactl/bin/grafanactl",
        &["GRAFANACTL_ENV_ASSIGNMENTS"],
        &["GRAFANACTL_ENV_ASSIGNMENTS"],
    ),
    one(
        "heroku",
        "heroku",
        "/opt/heroku/bin/heroku",
        &["HEROKU_API_KEY"],
        &[],
    ),
    one(
        "hcloud",
        "hcloud",
        "/opt/hcloud/bin/hcloud",
        &["HCLOUD_TOKEN"],
        &[],
    ),
    one(
        "huggingface-cli",
        "hf",
        "/opt/hf/bin/hf",
        &["HF_TOKEN"],
        &[],
    ),
    multi(
        "jfrog-cli",
        stub(
            "jf",
            "/opt/jfrog-cli/bin/jf",
            &["JFROG_ENV_ASSIGNMENTS"],
            &["JFROG_ENV_ASSIGNMENTS"],
        ),
        JFROG_EXTRA,
    ),
    one("k6", "k6", "/opt/k6/bin/k6", &["K6_CLOUD_TOKEN"], &[]),
    one(
        "luarocks",
        "luarocks",
        "/opt/luarocks/bin/luarocks",
        &["LUAROCKS_API_KEY"],
        &[],
    ),
    one(
        "minio-mc",
        "mc",
        "/opt/mc/bin/mc",
        &["MINIO_MC_HOST_ENV"],
        &["MINIO_MC_HOST_ENV"],
    ),
    one(
        "netlify-cli",
        "netlify",
        "/opt/netlify-cli/bin/netlify",
        &["NETLIFY_AUTH_TOKEN"],
        &[],
    ),
    one(
        "node",
        "npm",
        "/opt/node/bin/npm",
        &["NODE_AUTH_TOKEN"],
        &[],
    ),
    one(
        "pnpm",
        "pnpm",
        "/opt/pnpm/bin/pnpm",
        &["NODE_AUTH_TOKEN"],
        &[],
    ),
    one(
        "pulumi",
        "pulumi",
        "/opt/pulumi/bin/pulumi",
        &["PULUMI_ACCESS_TOKEN"],
        &[],
    ),
    one(
        "qwen-code",
        "qwen",
        "/opt/qwen-code/bin/qwen",
        &["QWEN_ENV_ASSIGNMENTS"],
        &["QWEN_ENV_ASSIGNMENTS"],
    ),
    one(
        "runpodctl",
        "runpodctl",
        "/opt/runpodctl/bin/runpodctl",
        &["RUNPOD_API_KEY"],
        &[],
    ),
    one(
        "s3cmd",
        "s3cmd",
        "/opt/s3cmd/bin/s3cmd",
        &["S3CMD_ENV_ASSIGNMENTS"],
        &["S3CMD_ENV_ASSIGNMENTS"],
    ),
    one(
        "sentry-cli",
        "sentry-cli",
        "/opt/getsentry/tools/sentry-cli/bin/sentry-cli",
        &["SENTRY_AUTH_TOKEN"],
        &[],
    ),
    one(
        "snowflake-cli",
        "snow",
        "/opt/snowflake-cli/bin/snow",
        &["SNOWFLAKE_ENV_ASSIGNMENTS"],
        &["SNOWFLAKE_ENV_ASSIGNMENTS"],
    ),
    one(
        "snyk",
        "snyk",
        "/opt/snyk/bin/snyk",
        &["SNYK_ENV_ASSIGNMENTS"],
        &["SNYK_ENV_ASSIGNMENTS"],
    ),
    one(
        "transifex-cli",
        "tx",
        "/opt/transifex-cli/bin/tx",
        &["TRANSIFEX_ENV_ASSIGNMENTS"],
        &["TRANSIFEX_ENV_ASSIGNMENTS"],
    ),
    one(
        "travis",
        "travis",
        "/opt/travis/bin/travis",
        &["TRAVIS_TOKEN"],
        &[],
    ),
    one(
        "twine",
        "twine",
        "/opt/twine/bin/twine",
        &["TWINE_ENV_ASSIGNMENTS"],
        &["TWINE_ENV_ASSIGNMENTS"],
    ),
    one(
        "vagrant",
        "vagrant",
        "/opt/hashicorp/tap/vagrant/bin/vagrant",
        &["VAGRANT_CLOUD_TOKEN"],
        &[],
    ),
    one(
        "vault",
        "vault",
        "/opt/hashicorp/tap/vault/bin/vault",
        &["VAULT_TOKEN"],
        &[],
    ),
    one(
        "virustotal-cli",
        "vt",
        "/opt/virustotal-cli/bin/vt",
        &["VTCLI_APIKEY"],
        &[],
    ),
    one(
        "vultr",
        "vultr-cli",
        "/opt/vultr/bin/vultr-cli",
        &["VULTR_API_KEY"],
        &[],
    ),
    one("wsk", "wsk", "/opt/wsk/bin/wsk", &["WHISK_AUTH"], &[]),
];

const fn one(
    name: &'static str,
    command: &'static str,
    target: &'static str,
    keys: &'static [&'static str],
    assignment_keys: &'static [&'static str],
) -> EnvWrapper {
    EnvWrapper {
        name,
        primary: stub(command, target, keys, assignment_keys),
        extra: &[],
    }
}

const fn multi(name: &'static str, primary: StubSpec, extra: &'static [StubSpec]) -> EnvWrapper {
    EnvWrapper {
        name,
        primary,
        extra,
    }
}

const fn stub(
    command: &'static str,
    target: &'static str,
    keys: &'static [&'static str],
    assignment_keys: &'static [&'static str],
) -> StubSpec {
    StubSpec {
        command,
        target,
        keys,
        assignment_keys,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn installs_simple_env_stub() {
        let _guard = crate::global_test_env_lock().lock().unwrap();
        let dir = temp_dir("env-wrapper-simple");
        let target_dir = dir.join("target");
        let stub_dir = dir.join("stub");
        fs::create_dir_all(&target_dir).unwrap();
        fs::create_dir_all(&stub_dir).unwrap();
        fs::write(target_dir.join("doctl"), "").unwrap();
        unsafe {
            std::env::set_var("AUTOMIC_VAULT_TEST_ENV_WRAPPER_TARGET_DIR", &target_dir);
            std::env::set_var("AUTOMIC_VAULT_TEST_ENV_WRAPPER_STUB_DIR", &stub_dir);
            std::env::set_var("AUTOMIC_VAULT_TEST_EUID", "0");
        }

        run(wrapper("doctl").unwrap(), &mut Vec::new(), true).unwrap();

        unsafe {
            std::env::remove_var("AUTOMIC_VAULT_TEST_ENV_WRAPPER_TARGET_DIR");
            std::env::remove_var("AUTOMIC_VAULT_TEST_ENV_WRAPPER_STUB_DIR");
            std::env::remove_var("AUTOMIC_VAULT_TEST_EUID");
        }
        let script = fs::read_to_string(stub_dir.join("doctl")).unwrap();
        assert!(script.contains(MARKER));
        assert!(script.contains("+DIGITALOCEAN_ACCESS_TOKEN"));
        assert!(script.contains("exec \"$original\" \"$@\""));
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn assignment_keys_are_exported() {
        let script = stub_script(&stub(
            "akamai",
            "/opt/akamai/bin/akamai",
            &["AKAMAI_ENV_ASSIGNMENTS"],
            &["AKAMAI_ENV_ASSIGNMENTS"],
        ));

        assert!(script.contains("+AKAMAI_ENV_ASSIGNMENTS"));
        assert!(script.contains("for assignment in ${AKAMAI_ENV_ASSIGNMENTS-}"));
        assert!(script.contains("export \"$assignment\""));
    }

    #[test]
    fn current_stub_validation_rejects_marker_preserving_edits() {
        let spec = stub("tool", "/opt/tool/bin/tool", &["TOOL_TOKEN"], &[]);
        let path = temp_dir("env-wrapper-exact").join("tool");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, stub_script(&spec)).unwrap();
        assert!(is_current_stub(&path, &spec));

        fs::write(&path, format!("{}\n# modified\n", stub_script(&spec))).unwrap();
        assert!(is_managed_stub(&path, &spec));
        assert!(!is_current_stub(&path, &spec));
        let _ = fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn refuses_non_managed_stub() {
        let _guard = crate::global_test_env_lock().lock().unwrap();
        let dir = temp_dir("env-wrapper-refuse");
        let target_dir = dir.join("target");
        let stub_dir = dir.join("stub");
        fs::create_dir_all(&target_dir).unwrap();
        fs::create_dir_all(&stub_dir).unwrap();
        fs::write(target_dir.join("doctl"), "").unwrap();
        fs::write(stub_dir.join("doctl"), "#!/bin/sh\n").unwrap();
        unsafe {
            std::env::set_var("AUTOMIC_VAULT_TEST_ENV_WRAPPER_TARGET_DIR", &target_dir);
            std::env::set_var("AUTOMIC_VAULT_TEST_ENV_WRAPPER_STUB_DIR", &stub_dir);
            std::env::set_var("AUTOMIC_VAULT_TEST_EUID", "0");
        }

        let err = run(wrapper("doctl").unwrap(), &mut Vec::new(), true).unwrap_err();

        unsafe {
            std::env::remove_var("AUTOMIC_VAULT_TEST_ENV_WRAPPER_TARGET_DIR");
            std::env::remove_var("AUTOMIC_VAULT_TEST_ENV_WRAPPER_STUB_DIR");
            std::env::remove_var("AUTOMIC_VAULT_TEST_EUID");
        }
        assert!(err.contains("is not an Automic Vault env-wrapper stub"));
        fs::remove_dir_all(dir).unwrap();
    }

    fn temp_dir(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("av-{label}-{}-{nanos}", std::process::id()))
    }
}
