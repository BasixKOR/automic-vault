use std::path::{Path, PathBuf};

use crate::{AffectedFile, Finding};

pub(crate) const ONE_DAY_SECONDS: u64 = 24 * 60 * 60;

pub(crate) fn npm_days(contents: &str) -> Option<u64> {
    numeric_setting(contents, &["min-release-age"]).map(|days| days * 24 * 60 * 60)
}

pub(crate) fn pnpm_minutes(contents: &str) -> Option<u64> {
    numeric_setting(contents, &["minimumReleaseAge", "minimum-release-age"])
        .map(|minutes| minutes * 60)
}

pub(crate) fn bun_seconds(contents: &str) -> Option<u64> {
    numeric_setting(contents, &["minimumReleaseAge"])
}

pub(crate) fn yarn_duration(contents: &str) -> Option<u64> {
    setting(contents, &["npmMinimalAgeGate"]).and_then(duration_seconds)
}

pub(crate) fn policy_findings(
    source: &'static str,
    home: &Path,
    paths: impl IntoIterator<Item = PathBuf>,
    configured_seconds: fn(&str) -> Option<u64>,
    solution: &'static str,
) -> Vec<Finding> {
    paths
        .into_iter()
        .filter(|path| path.exists())
        .filter_map(|path| {
            let contents = std::fs::read_to_string(&path).ok()?;
            let reason = match configured_seconds(&contents) {
                Some(seconds) if seconds < ONE_DAY_SECONDS => {
                    "config sets npm package minimum release age below 24 hours"
                }
                _ => return None,
            };
            Some(finding(source, home, path, reason, solution))
        })
        .collect()
}

fn finding(
    source: &'static str,
    home: &Path,
    path: PathBuf,
    reason: &str,
    solution: &'static str,
) -> Finding {
    let docs_url = super::radioisotope::docs_url(source);
    Finding {
        source,
        homepage: docs_url,
        severity: "medium",
        explanation: format!("{source} {reason}: {}", display_path(home, &path)),
        solution: solution.to_string(),
        affected: vec![AffectedFile {
            path: display_path(home, &path),
            line: None,
        }],
        docs_url,
    }
}

fn display_path(home: &Path, path: &Path) -> String {
    path.strip_prefix(home)
        .ok()
        .map(|path| format!("~/{}", path.display()))
        .unwrap_or_else(|| path.display().to_string())
}

fn numeric_setting(contents: &str, keys: &[&str]) -> Option<u64> {
    setting(contents, keys).and_then(|value| value.parse().ok())
}

fn setting<'a>(contents: &'a str, keys: &[&str]) -> Option<&'a str> {
    contents.lines().find_map(|line| {
        let line = line.split('#').next()?.trim();
        if line.is_empty() || line.starts_with(';') {
            return None;
        }
        let (key, value) = line.split_once('=').or_else(|| line.split_once(':'))?;
        let key = key.trim();
        keys.iter()
            .any(|expected| key == *expected)
            .then(|| unquote(value.trim()))
    })
}

fn duration_seconds(value: &str) -> Option<u64> {
    let digits = value
        .chars()
        .take_while(|character| character.is_ascii_digit())
        .collect::<String>();
    let number: u64 = digits.parse().ok()?;
    let unit = value[digits.len()..].trim();
    match unit {
        "" | "m" => Some(number * 60),
        "s" => Some(number),
        "h" => Some(number * 60 * 60),
        "d" => Some(number * 24 * 60 * 60),
        "w" => Some(number * 7 * 24 * 60 * 60),
        _ => None,
    }
}

fn unquote(value: &str) -> &str {
    value
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
        .or_else(|| {
            value
                .strip_prefix('\'')
                .and_then(|value| value.strip_suffix('\''))
        })
        .unwrap_or(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn parses_release_age_units() {
        assert_eq!(npm_days("min-release-age=1\n"), Some(ONE_DAY_SECONDS));
        assert_eq!(
            pnpm_minutes("minimumReleaseAge: 1440\n"),
            Some(ONE_DAY_SECONDS)
        );
        assert_eq!(
            pnpm_minutes("minimum-release-age=1440\n"),
            Some(ONE_DAY_SECONDS)
        );
        assert_eq!(
            bun_seconds("[install]\nminimumReleaseAge = 86400 # seconds\n"),
            Some(ONE_DAY_SECONDS)
        );
        assert_eq!(
            yarn_duration(r#"npmMinimalAgeGate: "1d""#),
            Some(ONE_DAY_SECONDS)
        );
    }

    #[test]
    fn reports_existing_configs_below_policy() {
        let home = temp_home();
        let low = home.join(".npmrc");
        let ok = home.join(".npmrc-ok");
        let unset = home.join(".npmrc-unset");
        std::fs::write(&low, "min-release-age=0\n").unwrap();
        std::fs::write(&ok, "min-release-age=1\n").unwrap();
        std::fs::write(&unset, "registry=https://registry.npmjs.org/\n").unwrap();

        let findings = policy_findings(
            "npm",
            &home,
            [low, ok, unset],
            npm_days,
            "Set `min-release-age=1` in the reported npm config file.",
        );

        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].severity, "medium");
        assert!(findings[0].explanation.contains("below 24 hours"));
        std::fs::remove_dir_all(home).unwrap();
    }

    fn temp_home() -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path =
            std::env::temp_dir().join(format!("js-release-age-{}-{nanos}", std::process::id()));
        std::fs::create_dir_all(&path).unwrap();
        path
    }
}
