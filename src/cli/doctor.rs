use std::ffi::OsStr;
use std::fs;
use std::io::Write;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};

use crate::isotopes::hardeners::{
    self, HardenerCommand, HardenerMetadata, RequiredExecutable, StubRequirements, executable,
};

use super::scan::Style;

pub(crate) struct DoctorResult {
    name: &'static str,
    commands: Vec<String>,
    issues: Vec<DoctorIssue>,
}

struct DoctorIssue {
    kind: &'static str,
    command: Option<String>,
    message: String,
    remediation: String,
    stub_path: Option<String>,
    target_path: Option<String>,
    resolved_path: Option<String>,
}

pub(crate) fn run<W: Write>(
    stdout: &mut W,
    selector: Option<&str>,
    json: bool,
    style: Style,
) -> Result<i32, String> {
    let results = diagnose(
        hardeners::metadata(),
        selector,
        &std::env::var_os("PATH").unwrap_or_default(),
    )?;
    let issue_count = results
        .iter()
        .map(|result| result.issues.len())
        .sum::<usize>();
    if json {
        print_json(stdout, &results);
    } else {
        print_human(stdout, &results, issue_count, style);
    }
    Ok(if issue_count == 0 { 0 } else { 1 })
}

fn diagnose(
    hardeners: Vec<HardenerMetadata>,
    selector: Option<&str>,
    path: &OsStr,
) -> Result<Vec<DoctorResult>, String> {
    if let Some(selector) = selector {
        let (hardener, command) =
            select(hardeners, selector).ok_or_else(|| format!("unknown command `{selector}`"))?;
        let checked = hardener
            .detection
            .commands
            .iter()
            .any(|candidate| command.as_deref().is_none_or(|name| candidate.name == name))
            || command.is_none() && !hardener.detection.diagnostics.is_empty();
        if !checked {
            return Err(format!(
                "`{selector}` has no Doctor-owned checks; use `av scan` for exposure findings"
            ));
        }
        return Ok(vec![diagnose_one(hardener, command.as_deref(), path)]);
    }

    Ok(hardeners
        .into_iter()
        .filter(|hardener| {
            hardener.detection.applicable
                && (!hardener.detection.commands.is_empty()
                    || !hardener.detection.diagnostics.is_empty())
        })
        .map(|hardener| diagnose_one(hardener, None, path))
        .collect())
}

fn has_stub_checks(command: &HardenerCommand) -> bool {
    command
        .stub_path
        .as_deref()
        .is_some_and(|stub| Path::new(stub) != Path::new(&command.target_path))
}

fn select(
    hardeners: Vec<HardenerMetadata>,
    selector: &str,
) -> Option<(HardenerMetadata, Option<String>)> {
    let canonical = match selector {
        "gh-cli" => "gh",
        "homebrew" => "brew",
        "supabase-cli" => "supabase",
        selector => selector,
    };
    if let Some(index) = hardeners
        .iter()
        .position(|hardener| hardener.name == canonical)
    {
        return Some((hardeners.into_iter().nth(index).unwrap(), None));
    }
    let (index, command) = hardeners.iter().enumerate().find_map(|(index, hardener)| {
        hardener
            .detection
            .commands
            .iter()
            .find(|command| command.name == selector)
            .map(|command| (index, command.name.clone()))
    })?;
    Some((hardeners.into_iter().nth(index).unwrap(), Some(command)))
}

fn diagnose_one(
    hardener: HardenerMetadata,
    command_filter: Option<&str>,
    path: &OsStr,
) -> DoctorResult {
    let commands = hardener
        .detection
        .commands
        .iter()
        .filter(|command| command_filter.is_none_or(|filter| command.name == filter))
        .collect::<Vec<_>>();
    let mut issues = hardener
        .detection
        .diagnostics
        .iter()
        .map(|diagnostic| DoctorIssue {
            kind: diagnostic.kind,
            command: None,
            message: diagnostic.message.clone(),
            remediation: diagnostic.remediation.clone(),
            stub_path: None,
            target_path: diagnostic.path.clone(),
            resolved_path: None,
        })
        .collect::<Vec<_>>();
    issues.extend(
        commands
            .iter()
            .flat_map(|command| diagnose_command(hardener.name, command, path)),
    );

    DoctorResult {
        name: hardener.name,
        commands: commands
            .iter()
            .map(|command| command.name.clone())
            .collect(),
        issues,
    }
}

fn diagnose_command(hardener: &str, command: &HardenerCommand, path: &OsStr) -> Vec<DoctorIssue> {
    let mut issues = Vec::new();
    if !executable(Path::new(&command.target_path)) {
        issues.push(target_issue(hardener, command));
    }
    issues.extend(
        command
            .required_paths
            .iter()
            .filter(|required| !executable(Path::new(&required.path)))
            .map(|required| dependency_issue(hardener, command, required)),
    );
    if has_stub_checks(command) {
        let stub_issues = stub_issues(hardener, command);
        let stub_is_healthy = stub_issues.is_empty();
        issues.extend(stub_issues);
        if stub_is_healthy {
            issues.extend(path_issue(command, path));
        }
    }
    issues
}

fn target_issue(hardener: &str, command: &HardenerCommand) -> DoctorIssue {
    let target = &command.target_path;
    DoctorIssue {
        kind: "target_unavailable",
        command: Some(command.name.clone()),
        message: format!(
            "{} target is missing or not executable: {}",
            command.name, target
        ),
        remediation: format!(
            "Install `{}` at {target}; the {hardener} hardener cannot wrap a missing target. If it is installed elsewhere, make {target} point to that executable, then rerun `av doctor {}`.",
            command.name, command.name
        ),
        stub_path: command.stub_path.clone(),
        target_path: Some(target.clone()),
        resolved_path: None,
    }
}

fn dependency_issue(
    hardener: &str,
    command: &HardenerCommand,
    required: &RequiredExecutable,
) -> DoctorIssue {
    DoctorIssue {
        kind: "dependency_unavailable",
        command: Some(command.name.clone()),
        message: format!(
            "{} hardening requires {} to be an executable file at {}",
            command.name, required.name, required.path
        ),
        remediation: format!(
            "Install or restore {} at {}, then rerun `av doctor {}`. If it is installed elsewhere, replace that path with a root-owned symlink to the executable.",
            required.name, required.path, hardener
        ),
        stub_path: command.stub_path.clone(),
        target_path: Some(required.path.clone()),
        resolved_path: None,
    }
}

fn stub_issues(hardener: &str, command: &HardenerCommand) -> Vec<DoctorIssue> {
    let Some(stub) = command.stub_path.as_deref() else {
        return Vec::new();
    };
    let mut issues = command
        .stub_requirements
        .iter()
        .flat_map(|requirements| identity_issues(hardener, command, stub, requirements))
        .collect::<Vec<_>>();
    let metadata = match fs::symlink_metadata(stub) {
        Ok(metadata) => metadata,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            issues.push(DoctorIssue {
                kind: "stub_missing",
                command: Some(command.name.clone()),
                message: format!(
                    "{} hardening is bypassed because its launcher is missing: {stub}",
                    command.name
                ),
                remediation: format!(
                    "Run `sudo av harden {hardener}` to recreate it. Manual repair: {}. Then rerun `av doctor {}`.",
                    manual_stub_repair(hardener, command, stub),
                    command.name
                ),
                stub_path: Some(stub.to_string()),
                target_path: Some(command.target_path.clone()),
                resolved_path: None,
            });
            return issues;
        }
        Err(err) => {
            issues.push(DoctorIssue {
                kind: "stub_unreadable",
                command: Some(command.name.clone()),
                message: format!("cannot inspect hardened launcher {stub}: {err}"),
                remediation: format!(
                    "Ensure every parent directory permits metadata access and that {stub} is readable, then rerun `av doctor {}`.",
                    command.name
                ),
                stub_path: Some(stub.to_string()),
                target_path: Some(command.target_path.clone()),
                resolved_path: None,
            });
            return issues;
        }
    };
    if !metadata.file_type().is_file() {
        let actual = if metadata.file_type().is_symlink() {
            "a symbolic link"
        } else if metadata.file_type().is_dir() {
            "a directory"
        } else {
            "a non-regular file"
        };
        issues.push(DoctorIssue {
            kind: "stub_wrong_type",
            command: Some(command.name.clone()),
            message: format!(
                "hardened launcher {stub} is {actual}; expected a regular file"
            ),
            remediation: format!(
                "Remove {stub} after reviewing it, then run `sudo av harden {hardener}`. Manual repair: install the documented launcher directly at {stub}; do not use a symlink."
            ),
            stub_path: Some(stub.to_string()),
            target_path: Some(command.target_path.clone()),
            resolved_path: None,
        });
        return issues;
    }

    let actual_mode = metadata.permissions().mode() & 0o7777;
    if !executable(Path::new(stub)) {
        issues.push(DoctorIssue {
            kind: "stub_not_executable",
            command: Some(command.name.clone()),
            message: format!(
                "hardened launcher {stub} is not executable (mode {actual_mode:#06o})"
            ),
            remediation: format!(
                "Set the expected mode with `sudo chmod {mode:04o} {stub}`, then rerun `av doctor {}`.",
                command.name,
                mode = command
                    .stub_requirements
                    .as_ref()
                    .map_or(0o755, |requirements| requirements.mode),
            ),
            stub_path: Some(stub.to_string()),
            target_path: Some(command.target_path.clone()),
            resolved_path: None,
        });
    } else if let Some(requirements) = &command.stub_requirements
        && actual_mode != requirements.mode
    {
        issues.push(DoctorIssue {
            kind: "stub_mode_mismatch",
            command: Some(command.name.clone()),
            message: format!(
                "hardened launcher {stub} has mode {actual_mode:#06o}; expected {:#06o}",
                requirements.mode
            ),
            remediation: format!(
                "Run `sudo chmod {mode:04o} {stub}`, then rerun `av doctor {}`.",
                command.name,
                mode = requirements.mode
            ),
            stub_path: Some(stub.to_string()),
            target_path: Some(command.target_path.clone()),
            resolved_path: None,
        });
    }
    if let Some(requirements) = &command.stub_requirements {
        let owner_mismatch = requirements
            .owner
            .id
            .is_some_and(|expected| metadata.uid() != expected);
        let group_mismatch = requirements
            .group
            .id
            .is_some_and(|expected| metadata.gid() != expected);
        if owner_mismatch || group_mismatch {
            issues.push(DoctorIssue {
                kind: "stub_owner_mismatch",
                command: Some(command.name.clone()),
                message: format!(
                    "hardened launcher {stub} is owned by uid {} and gid {}; expected {} ({}) and {} ({})",
                    metadata.uid(),
                    metadata.gid(),
                    requirements.owner.name,
                    requirements.owner.id.map_or_else(|| "missing".into(), |id| id.to_string()),
                    requirements.group.name,
                    requirements.group.id.map_or_else(|| "missing".into(), |id| id.to_string()),
                ),
                remediation: format!(
                    "Run `sudo chown {}:{} {stub}`, then rerun `av doctor {}`.",
                    requirements.owner.name, requirements.group.name, command.name
                ),
                stub_path: Some(stub.to_string()),
                target_path: Some(command.target_path.clone()),
                resolved_path: None,
            });
        }
    }
    if !command.stub_valid {
        issues.push(DoctorIssue {
            kind: "stub_content_invalid",
            command: Some(command.name.clone()),
            message: format!(
                "launcher {stub} does not contain the expected {hardener} hardening implementation"
            ),
            remediation: format!(
                "Run `sudo av harden {hardener}` to replace it. Manual repair: {}",
                manual_stub_repair(hardener, command, stub)
            ),
            stub_path: Some(stub.to_string()),
            target_path: Some(command.target_path.clone()),
            resolved_path: None,
        });
    }
    issues
}

fn identity_issues(
    hardener: &str,
    command: &HardenerCommand,
    stub: &str,
    requirements: &StubRequirements,
) -> Vec<DoctorIssue> {
    [
        ("user", &requirements.owner),
        ("group", &requirements.group),
    ]
    .into_iter()
    .filter(|(_, identity)| identity.id.is_none())
    .map(|(kind, identity)| DoctorIssue {
        kind: "required_identity_missing",
        command: Some(command.name.clone()),
        message: format!(
            "{hardener} hardening requires local {kind} `{}`, but it cannot be resolved",
            identity.name
        ),
        remediation: format!(
            "Run `sudo av harden {hardener}` to recreate the required account metadata. Manual repair: {}",
            manual_identity_repair(hardener, kind, identity.name, stub)
        ),
        stub_path: Some(stub.to_string()),
        target_path: Some(command.target_path.clone()),
        resolved_path: None,
    })
    .collect()
}

fn manual_identity_repair(hardener: &str, kind: &str, name: &str, stub: &str) -> String {
    match (hardener, kind, name) {
        ("brew", "group", "vault") => format!(
            "choose an unused GID from 550–599, then run `sudo dscl . -create /Groups/vault`, `sudo dscl . -create /Groups/vault RealName 'Automic Vault'`, and `sudo dscl . -create /Groups/vault PrimaryGroupID <gid>`; finally run `sudo chown automic:vault {stub}`."
        ),
        ("brew", "user", "automic") => format!(
            "create the `vault` group first, choose an unused UID from 550–599, then create `automic` with `sudo dscl . -create /Users/automic`, setting RealName to `Automic Vault Homebrew`, UserShell to `/usr/bin/false`, NFSHomeDirectory to `/opt/homebrew/var/automic`, UniqueID to the chosen UID, PrimaryGroupID to the vault GID, and Password to `*`; finally run `sudo chown automic:vault {stub}`."
        ),
        _ => format!(
            "create the documented `{name}` {kind}, set the owner of {stub} accordingly, and rerun `av doctor {hardener}`."
        ),
    }
}

fn manual_stub_repair(hardener: &str, command: &HardenerCommand, stub: &str) -> String {
    if hardener == "brew" {
        return format!(
            "copy the matching `av-brew-stub` binary from `/Applications/Automic Vault.app/Contents/MacOS/av-brew-stub` to {stub} with `sudo install -o automic -g vault -m 6755`, after creating the `automic` user and `vault` group"
        );
    }
    if hardener == "aws" {
        return format!(
            "install the exact `src/isotopes/hardeners/aws` launcher from this Automic Vault release at {stub}, preserve its `/opt/homebrew/bin/aws-vault` and `/opt/homebrew/bin/aws` paths, then run `sudo chown root:wheel {stub} && sudo chmod 0755 {stub}`"
        );
    }
    let keys = command
        .injected_keys
        .iter()
        .map(|key| format!("+{key}"))
        .collect::<Vec<_>>()
        .join(" ");
    let assignments = if command.assignment_keys.is_empty() {
        String::new()
    } else {
        format!(
            "; before exec, split each newline-delimited value in {} into `NAME=value` entries and export them",
            command.assignment_keys.join(", ")
        )
    };
    let ownership = command.stub_requirements.as_ref().map_or_else(
        || "set it executable".to_string(),
        |requirements| {
            format!(
                "run `sudo chown {}:{} {stub} && sudo chmod {:04o} {stub}`",
                requirements.owner.name, requirements.group.name, requirements.mode
            )
        },
    );
    format!(
        "create a regular shell script at {stub} with shebang `#!/usr/local/bin/av inject --allow-missing-keys {keys} /bin/sh` that ends with `exec {} \"$@\"`{assignments}; {ownership}",
        command.target_path
    )
}

fn path_issue(command: &HardenerCommand, path: &OsStr) -> Option<DoctorIssue> {
    let stub = command.stub_path.as_deref()?;
    if same_path(Path::new(stub), Path::new(&command.target_path)) {
        return None;
    }
    let resolved = resolve(&command.name, path);
    if resolved
        .as_deref()
        .is_some_and(|resolved| same_path(Path::new(stub), resolved))
    {
        return None;
    }
    let resolved_path = resolved.map(|path| path.display().to_string());
    let message = match &resolved_path {
        Some(resolved) => format!(
            "{} resolves to {resolved} before the hardened stub {stub}",
            command.name
        ),
        None => format!(
            "{} is not available through PATH; expected {stub}",
            command.name
        ),
    };
    Some(DoctorIssue {
        kind: "stub_not_first_on_path",
        command: Some(command.name.clone()),
        message,
        remediation: format!(
            "Put {stub_dir} before {target_dir} in PATH, then start a new shell. For example: `export PATH=\"{stub_dir}:$PATH\"`.",
            stub_dir = Path::new(stub)
                .parent()
                .unwrap_or_else(|| Path::new(stub))
                .display(),
            target_dir = Path::new(&command.target_path)
                .parent()
                .unwrap_or_else(|| Path::new(&command.target_path))
                .display(),
        ),
        stub_path: Some(stub.to_string()),
        target_path: Some(command.target_path.clone()),
        resolved_path,
    })
}

fn resolve(command: &str, path: &OsStr) -> Option<PathBuf> {
    std::env::split_paths(path)
        .map(|directory| directory.join(command))
        .find(|candidate| executable(candidate))
}

fn same_path(left: &Path, right: &Path) -> bool {
    left == right
        || left
            .canonicalize()
            .ok()
            .zip(right.canonicalize().ok())
            .is_some_and(|(left, right)| left == right)
}

fn print_json(stdout: &mut dyn Write, results: &[DoctorResult]) {
    let report = serde_json::json!({
        "results": results.iter().map(|result| serde_json::json!({
            "name": result.name,
            "commands": result.commands,
            "issues": result.issues.iter().map(|issue| serde_json::json!({
                "kind": issue.kind,
                "command": issue.command,
                "message": issue.message,
                "remediation": issue.remediation,
                "stub_path": issue.stub_path,
                "target_path": issue.target_path,
                "resolved_path": issue.resolved_path,
            })).collect::<Vec<_>>(),
        })).collect::<Vec<_>>(),
    });
    let _ = writeln!(stdout, "{report}");
}

fn print_human(stdout: &mut dyn Write, results: &[DoctorResult], issue_count: usize, style: Style) {
    let _ = writeln!(stdout, "╭─ doctor");
    let _ = writeln!(stdout, "│");
    if results.is_empty() {
        let _ = writeln!(
            stdout,
            "╰─ {}",
            style.paint("32", "No applicable hardeners found")
        );
        return;
    }
    for result in results {
        if result.issues.is_empty() {
            let _ = writeln!(
                stdout,
                "├─ {} {}",
                result.name,
                style.paint("32", "healthy ✔︎")
            );
        } else {
            let _ = writeln!(stdout, "├─ {}", result.name);
            for issue in &result.issues {
                super::scan::write_wrapped_with_continuation(
                    stdout,
                    "│  ├─ ",
                    "│  │  ",
                    &issue.message,
                    style,
                    Some("33"),
                );
                super::scan::write_wrapped_with_continuation(
                    stdout,
                    "│  ╰─ ",
                    "│     ",
                    &issue.remediation,
                    style,
                    None,
                );
            }
        }
    }
    let summary = if issue_count == 0 {
        style.paint("32", "No problems found")
    } else if issue_count == 1 {
        style.paint("33", "1 issue requires attention")
    } else {
        style.paint("33", format!("{issue_count} issues require attention"))
    };
    let _ = writeln!(stdout, "│");
    let _ = writeln!(stdout, "╰─ {summary}");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::isotopes::hardeners::{
        HardenerDetection, HardenerMetadata, RequiredIdentity, StubRequirements,
    };
    use std::fs;
    use std::os::unix::fs::{MetadataExt, PermissionsExt, symlink};
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn explicit_doctor_reports_missing_target_and_stub() {
        let hardeners = vec![hardener(
            "node",
            false,
            command("npm", false, "/missing/stub", "/missing/npm"),
        )];
        assert!(
            diagnose(hardeners, None, OsStr::new(""))
                .unwrap()
                .is_empty()
        );

        let results = diagnose(
            vec![hardener(
                "node",
                false,
                command("npm", false, "/missing/stub", "/missing/npm"),
            )],
            Some("npm"),
            OsStr::new(""),
        )
        .unwrap();
        assert_eq!(
            results[0]
                .issues
                .iter()
                .map(|issue| issue.kind)
                .collect::<Vec<_>>(),
            ["target_unavailable", "stub_missing"]
        );
    }

    #[test]
    fn aggregate_reports_installed_but_broken_hardening() {
        let dir = temp_dir("nonexecutable");
        let target = dir.join("npm");
        fs::write(&target, "not executable").unwrap();
        let hardeners = vec![hardener(
            "node",
            false,
            command(
                "npm",
                false,
                dir.join("stub").to_str().unwrap(),
                target.to_str().unwrap(),
            ),
        )];
        let results = diagnose(hardeners, None, OsStr::new("")).unwrap();
        assert_eq!(
            results[0]
                .issues
                .iter()
                .map(|issue| issue.kind)
                .collect::<Vec<_>>(),
            ["target_unavailable", "stub_missing"]
        );

        let results = diagnose(
            vec![hardener(
                "node",
                false,
                command(
                    "npm",
                    false,
                    dir.join("stub").to_str().unwrap(),
                    target.to_str().unwrap(),
                ),
            )],
            Some("node"),
            OsStr::new(""),
        )
        .unwrap();

        assert_eq!(results[0].issues[0].kind, "target_unavailable");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn accepts_homebrew_as_an_explicit_alias() {
        let results = diagnose(
            vec![hardener(
                "brew",
                false,
                command("brew", false, "/missing/stub", "/missing/brew"),
            )],
            Some("homebrew"),
            OsStr::new(""),
        )
        .unwrap();

        assert_eq!(results[0].name, "brew");
        assert_eq!(results[0].issues[0].kind, "target_unavailable");
    }

    #[test]
    fn accepts_hardener_aliases_and_limits_executable_selection() {
        let dir = temp_dir("aliases");
        let jf = executable_file(&dir.join("jf"));
        let jfrog = executable_file(&dir.join("jfrog"));
        let jf_target = executable_file(&dir.join("jf-target"));
        let jfrog_target = executable_file(&dir.join("jfrog-target"));
        let hardeners = vec![HardenerMetadata {
            name: "jfrog-cli",
            documentation: "",
            detection: HardenerDetection::commands(
                true,
                vec![
                    command(
                        "jf",
                        true,
                        jf.to_str().unwrap(),
                        jf_target.to_str().unwrap(),
                    ),
                    command(
                        "jfrog",
                        true,
                        jfrog.to_str().unwrap(),
                        jfrog_target.to_str().unwrap(),
                    ),
                ],
            ),
            secret_gate: None,
        }];
        let results = diagnose(hardeners, Some("jf"), dir.as_os_str()).unwrap();
        assert_eq!(results[0].commands, ["jf"]);
        assert!(results[0].issues.is_empty());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn reports_hardening_and_path_precedence() {
        let dir = temp_dir("path");
        let stub_dir = dir.join("stub");
        let target_dir = dir.join("target");
        fs::create_dir_all(&stub_dir).unwrap();
        fs::create_dir_all(&target_dir).unwrap();
        let stub = executable_file(&stub_dir.join("aws"));
        let target = executable_file(&target_dir.join("aws"));

        let unhardened = diagnose(
            vec![hardener(
                "aws",
                false,
                command(
                    "aws",
                    false,
                    stub.to_str().unwrap(),
                    target.to_str().unwrap(),
                ),
            )],
            Some("aws"),
            stub_dir.as_os_str(),
        )
        .unwrap();
        assert_eq!(unhardened[0].issues[0].kind, "stub_content_invalid");

        let shadowed_path = std::env::join_paths([&target_dir, &stub_dir]).unwrap();
        let shadowed = diagnose(
            vec![hardener(
                "aws",
                true,
                command(
                    "aws",
                    true,
                    stub.to_str().unwrap(),
                    target.to_str().unwrap(),
                ),
            )],
            None,
            &shadowed_path,
        )
        .unwrap();
        assert_eq!(shadowed[0].issues[0].kind, "stub_not_first_on_path");
        assert_eq!(
            shadowed[0].issues[0].resolved_path.as_deref(),
            target.to_str()
        );
        assert!(shadowed[0].issues[0].remediation.contains("export PATH="));

        let healthy_path = std::env::join_paths([&stub_dir, &target_dir]).unwrap();
        let healthy = diagnose(
            vec![hardener(
                "aws",
                true,
                command(
                    "aws",
                    true,
                    stub.to_str().unwrap(),
                    target.to_str().unwrap(),
                ),
            )],
            None,
            &healthy_path,
        )
        .unwrap();
        assert!(healthy[0].issues.is_empty());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn reports_each_broken_stub_invariant_precisely() {
        let dir = temp_dir("stub-invariants");
        let stub = executable_file(&dir.join("tool"));
        let target = executable_file(&dir.join("tool-target"));
        fs::set_permissions(&stub, fs::Permissions::from_mode(0o777)).unwrap();
        let metadata = stub.metadata().unwrap();
        let mut command = command(
            "tool",
            false,
            stub.to_str().unwrap(),
            target.to_str().unwrap(),
        );
        command.required_paths.push(RequiredExecutable {
            name: "helper",
            path: dir.join("missing-helper").display().to_string(),
        });
        command.stub_requirements = Some(StubRequirements {
            mode: 0o755,
            owner: RequiredIdentity {
                name: "expected-user",
                id: Some(metadata.uid() + 1),
            },
            group: RequiredIdentity {
                name: "expected-group",
                id: Some(metadata.gid()),
            },
        });

        let results = diagnose(
            vec![hardener("tool", false, command)],
            None,
            dir.as_os_str(),
        )
        .unwrap();
        let issues = &results[0].issues;

        assert_eq!(
            issues.iter().map(|issue| issue.kind).collect::<Vec<_>>(),
            [
                "dependency_unavailable",
                "stub_mode_mismatch",
                "stub_owner_mismatch",
                "stub_content_invalid"
            ]
        );
        assert!(issues[0].message.contains("missing-helper"));
        assert!(issues[1].message.contains("0o0777"));
        assert!(issues[2].remediation.contains("sudo chown"));
        assert!(issues[3].remediation.contains("Manual repair:"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn rejects_symlinked_stubs_even_when_they_resolve_to_an_executable() {
        let dir = temp_dir("symlink-stub");
        let target = executable_file(&dir.join("target"));
        let stub = dir.join("stub");
        symlink(&target, &stub).unwrap();
        let results = diagnose(
            vec![hardener(
                "tool",
                true,
                command(
                    "tool",
                    true,
                    stub.to_str().unwrap(),
                    target.to_str().unwrap(),
                ),
            )],
            None,
            dir.as_os_str(),
        )
        .unwrap();

        assert_eq!(results[0].issues[0].kind, "stub_wrong_type");
        assert!(results[0].issues[0].message.contains("symbolic link"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn target_only_hardening_checks_the_isotope_without_inventing_a_stub() {
        let missing = "/missing/isotope/bin/gh";
        let results = diagnose(
            vec![hardener(
                "gh",
                false,
                command("gh", false, missing, missing),
            )],
            Some("gh"),
            OsStr::new(""),
        )
        .unwrap();

        assert_eq!(results[0].issues.len(), 1);
        assert_eq!(results[0].issues[0].kind, "target_unavailable");
        assert!(results[0].issues[0].message.contains(missing));
    }

    #[test]
    fn configuration_exposures_remain_owned_by_scan() {
        let hardener = HardenerMetadata {
            name: "sudo",
            documentation: "",
            detection: HardenerDetection::configuration(
                false,
                true,
                Some("/etc/pam.d/sudo_local".to_string()),
            ),
            secret_gate: None,
        };
        let error = diagnose(vec![hardener], Some("sudo"), OsStr::new(""))
            .err()
            .unwrap();

        assert_eq!(
            error,
            "`sudo` has no Doctor-owned checks; use `av scan` for exposure findings"
        );
    }

    #[test]
    fn every_hardener_has_an_explicit_doctor_or_scan_boundary() {
        let _guard = crate::global_test_env_lock().lock().unwrap();
        for hardener in hardeners::metadata() {
            if hardener.detection.commands.is_empty() {
                assert_eq!(
                    hardener.name, "sudo",
                    "{} needs Doctor checks or an explicit Scan-owned exemption",
                    hardener.name
                );
                continue;
            }
            for command in &hardener.detection.commands {
                if has_stub_checks(command) {
                    assert!(
                        command.stub_requirements.is_some(),
                        "{}:{} lacks stub mode/ownership requirements",
                        hardener.name,
                        command.name
                    );
                    if hardener.name != "brew" {
                        assert!(
                            command
                                .required_paths
                                .iter()
                                .any(|required| required.name == "Automic Vault CLI"),
                            "{}:{} does not check its av interpreter",
                            hardener.name,
                            command.name
                        );
                    }
                } else {
                    assert!(
                        matches!(hardener.name, "gh" | "supabase"),
                        "{}:{} needs explicit target-only Doctor coverage review",
                        hardener.name,
                        command.name
                    );
                }
            }
        }
    }

    fn hardener(name: &'static str, hardened: bool, command: HardenerCommand) -> HardenerMetadata {
        HardenerMetadata {
            name,
            documentation: "",
            detection: HardenerDetection::commands(hardened, vec![command]),
            secret_gate: None,
        }
    }

    fn command(name: &str, hardened: bool, stub: &str, target: &str) -> HardenerCommand {
        HardenerCommand {
            name: name.to_string(),
            hardened,
            stub_valid: hardened,
            stub_path: Some(stub.to_string()),
            target_path: target.to_string(),
            required_paths: Vec::new(),
            stub_requirements: None,
            injected_keys: Vec::new(),
            assignment_keys: Vec::new(),
        }
    }

    fn executable_file(path: &Path) -> PathBuf {
        fs::write(path, "#!/bin/sh\n").unwrap();
        fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
        path.to_path_buf()
    }

    fn temp_dir(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("av-doctor-{label}-{nanos}"));
        fs::create_dir_all(&path).unwrap();
        path
    }
}
