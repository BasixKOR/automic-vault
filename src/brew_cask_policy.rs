use serde_json::Value;
use std::collections::BTreeSet;
use std::path::{Component, Path};

const BREW_BIN: &str = "/opt/homebrew/bin";

pub fn validate_info_cask(name: &str, cask: &Value) -> Result<(), String> {
    if cask.get("tap").and_then(Value::as_str) != Some("homebrew/cask") {
        return Err(format!(
            "CLI-only cask `{name}` must come from homebrew/cask"
        ));
    }
    if cask
        .get("depends_on")
        .and_then(|value| value.get("cask"))
        .is_some_and(nonempty)
    {
        return Err(format!(
            "CLI-only cask `{name}` cannot depend on another cask"
        ));
    }
    validate_artifacts(
        name,
        cask.get("artifacts")
            .ok_or_else(|| format!("Homebrew returned no artifacts for `{name}`"))?,
        false,
    )
}

pub fn validate_install_receipt(name: &str, receipt: &Value) -> Result<(), String> {
    if receipt
        .get("source")
        .and_then(|value| value.get("tap"))
        .and_then(Value::as_str)
        != Some("homebrew/cask")
    {
        return Err(format!(
            "installed CLI-only cask `{name}` did not come from homebrew/cask"
        ));
    }
    validate_artifacts(
        name,
        receipt
            .get("uninstall_artifacts")
            .ok_or_else(|| format!("installed cask `{name}` has no artifact receipt"))?,
        true,
    )
}

fn validate_artifacts(name: &str, value: &Value, receipt: bool) -> Result<(), String> {
    let artifacts = value
        .as_array()
        .ok_or_else(|| format!("Homebrew returned malformed artifacts for `{name}`"))?;
    let mut binaries = BTreeSet::new();
    let mut completions = Vec::new();

    for artifact in artifacts {
        let object = artifact
            .as_object()
            .ok_or_else(|| format!("Homebrew returned malformed artifact for `{name}`"))?;
        let kinds = object
            .keys()
            .filter(|key| key.as_str() != "target")
            .collect::<Vec<_>>();
        if kinds.len() != 1 {
            return Err(format!("Homebrew returned ambiguous artifact for `{name}`"));
        }
        match kinds[0].as_str() {
            "binary" => {
                let args = artifact_args(name, object.get("binary"), "binary")?;
                let source = args[0]
                    .as_str()
                    .filter(|source| safe_relative_path(source))
                    .ok_or_else(|| format!("cask `{name}` has an unsafe binary source"))?;
                binaries.insert(source.to_string());
                validate_binary_target(name, object, args, receipt)?;
            }
            "generate_completions_from_executable" => {
                let args = artifact_args(
                    name,
                    object.get("generate_completions_from_executable"),
                    "generated completion",
                )?;
                let source = args[0]
                    .as_str()
                    .filter(|source| safe_relative_path(source))
                    .ok_or_else(|| format!("cask `{name}` has an unsafe completion executable"))?;
                completions.push(source.to_string());
                if object.len() != 1 {
                    return Err(format!("cask `{name}` has malformed generated completions"));
                }
            }
            "zap" => {
                if object.len() != 1 || !object["zap"].is_array() {
                    return Err(format!("cask `{name}` has malformed zap metadata"));
                }
            }
            kind => {
                return Err(format!(
                    "cask `{name}` is not CLI-only: unsupported `{kind}` artifact"
                ));
            }
        }
    }

    if binaries.is_empty() {
        return Err(format!("cask `{name}` has no binary artifact"));
    }
    if let Some(source) = completions
        .iter()
        .find(|source| !binaries.contains(source.as_str()))
    {
        return Err(format!(
            "cask `{name}` generates completions from undeclared binary `{source}`"
        ));
    }
    Ok(())
}

fn artifact_args<'a>(
    name: &str,
    value: Option<&'a Value>,
    kind: &str,
) -> Result<&'a [Value], String> {
    value
        .and_then(Value::as_array)
        .filter(|args| !args.is_empty())
        .map(Vec::as_slice)
        .ok_or_else(|| format!("Homebrew returned malformed {kind} artifact for `{name}`"))
}

fn validate_binary_target(
    name: &str,
    object: &serde_json::Map<String, Value>,
    args: &[Value],
    receipt: bool,
) -> Result<(), String> {
    let target = if receipt {
        if object.len() != 1 || args.len() > 2 {
            return Err(format!(
                "installed cask `{name}` has malformed binary metadata"
            ));
        }
        match args.get(1) {
            Some(value) => Some(
                value
                    .as_object()
                    .filter(|target| target.len() == 1)
                    .and_then(|target| target.get("target"))
                    .and_then(Value::as_str)
                    .ok_or_else(|| {
                        format!("installed cask `{name}` has malformed binary target")
                    })?,
            ),
            None => None,
        }
    } else {
        if object.len() != 2 || args.len() != 1 {
            return Err(format!("cask `{name}` has malformed binary metadata"));
        }
        Some(
            object
                .get("target")
                .and_then(Value::as_str)
                .ok_or_else(|| format!("cask `{name}` has no binary target"))?,
        )
    };

    if target.is_some_and(|target| !safe_binary_target(target)) {
        return Err(format!(
            "cask `{name}` binary target must be directly inside {BREW_BIN}"
        ));
    }
    Ok(())
}

fn safe_relative_path(value: &str) -> bool {
    !value.is_empty()
        && !value.starts_with(['$', '~'])
        && Path::new(value).is_relative()
        && Path::new(value)
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
}

fn safe_binary_target(value: &str) -> bool {
    let path = Path::new(value);
    if path.is_absolute() {
        return path.parent() == Some(Path::new(BREW_BIN)) && path.file_name().is_some();
    }
    safe_relative_path(value) && path.components().count() == 1
}

fn nonempty(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Array(values) => !values.is_empty(),
        Value::Object(values) => !values.is_empty(),
        Value::String(value) => !value.is_empty(),
        _ => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn accepts_codex_style_cli_cask() {
        let cask = json!({
            "tap": "homebrew/cask",
            "artifacts": [
                {"binary": ["bin/codex"], "target": "/opt/homebrew/bin/codex"},
                {"generate_completions_from_executable": ["bin/codex", "completion"]},
                {"zap": [{"rmdir": "~/.codex"}]}
            ],
            "depends_on": {}
        });
        assert_eq!(validate_info_cask("codex", &cask), Ok(()));
    }

    #[test]
    fn rejects_external_artifacts_sources_targets_and_dependencies() {
        for cask in [
            json!({"tap": "homebrew/cask", "artifacts": [{"app": ["Foo.app"], "target": "/Applications/Foo.app"}]}),
            json!({"tap": "homebrew/cask", "artifacts": [{"binary": ["../foo"], "target": "/opt/homebrew/bin/foo"}]}),
            json!({"tap": "homebrew/cask", "artifacts": [{"binary": ["$APPDIR/foo"], "target": "/opt/homebrew/bin/foo"}]}),
            json!({"tap": "homebrew/cask", "artifacts": [{"binary": ["foo"], "target": "/usr/local/bin/foo"}]}),
            json!({"tap": "homebrew/cask", "artifacts": [{"binary": ["foo"], "target": "/opt/homebrew/bin/foo"}], "depends_on": {"cask": ["bar"]}}),
            json!({"tap": "other/tap", "artifacts": [{"binary": ["foo"], "target": "/opt/homebrew/bin/foo"}]}),
        ] {
            assert!(validate_info_cask("foo", &cask).is_err());
        }
    }

    #[test]
    fn validates_installed_receipt_fail_closed() {
        let receipt = json!({"source": {"tap": "homebrew/cask"}, "uninstall_artifacts": [
            {"binary": ["bin/codex"]},
            {"generate_completions_from_executable": ["bin/codex", "completion"]},
            {"zap": [{"rmdir": "~/.codex"}]}
        ]});
        assert_eq!(validate_install_receipt("codex", &receipt), Ok(()));
        assert!(
            validate_install_receipt(
                "app",
            &json!({"source": {"tap": "homebrew/cask"}, "uninstall_artifacts": [{"app": ["App.app"]}]})
            )
            .is_err()
        );
        assert!(validate_install_receipt("missing", &json!({})).is_err());
    }
}
