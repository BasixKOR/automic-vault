use std::path::Path;

use crate::{AffectedFile, Finding};

pub(crate) const NAME: &str = "aws";
const HOMEPAGE: &str = "https://aws.amazon.com/cli/";
const HIGH: &str = "high";
const DOCS_URL: &str = "https://github.com/automic-vault/automic-vault";
const MESSAGE: &str = "AWS profiles store plaintext access keys in ~/.aws/credentials.";
const SOLUTION: &str =
    "Run `av harden aws` to import the keys into Keychain and remove them from ~/.aws/credentials.";

pub(crate) fn findings(home: &Path) -> Vec<Finding> {
    let path = home.join(".aws/credentials");
    let Ok(contents) = std::fs::read_to_string(&path) else {
        return Vec::new();
    };
    let lines = plaintext_key_lines(&contents);
    if lines.is_empty() {
        return Vec::new();
    }
    vec![Finding {
        source: NAME,
        homepage: HOMEPAGE,
        severity: HIGH,
        explanation: MESSAGE.to_string(),
        solution: SOLUTION.to_string(),
        affected: lines
            .into_iter()
            .map(|line| AffectedFile {
                path: path.display().to_string(),
                line,
            })
            .collect(),
        docs_url: DOCS_URL,
    }]
}

fn plaintext_key_lines(contents: &str) -> Vec<usize> {
    let mut in_profile = false;
    let mut lines = Vec::new();
    for (index, line) in contents.lines().enumerate() {
        let trimmed = line.trim();
        if let Some(section) = trimmed
            .strip_prefix('[')
            .and_then(|line| line.strip_suffix(']'))
        {
            in_profile = !section.trim().is_empty();
            continue;
        }
        if in_profile
            && trimmed.split_once('=').is_some_and(|(key, value)| {
                matches!(key.trim(), "aws_access_key_id" | "aws_secret_access_key")
                    && !value.trim().is_empty()
            })
        {
            lines.push(index + 1);
        }
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_plaintext_aws_keys_in_any_profile() {
        assert_eq!(
            plaintext_key_lines(
                "[profile dev]\naws_access_key_id=x\n[default]\naws_access_key_id = AKIA\naws_secret_access_key = secret\n"
            ),
            vec![2, 4, 5]
        );
    }
}
