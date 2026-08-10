use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SecretReference {
    pub(crate) name: String,
    pub(crate) reference: String,
}

pub(crate) fn referenced_names(
    references: &[SecretReference],
    uri_path_and_query: &str,
    headers: impl Iterator<Item = Vec<u8>> + Clone,
    body: &[u8],
) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    for reference in references {
        let needle = reference.reference.as_bytes();
        if uri_path_and_query
            .as_bytes()
            .windows(needle.len())
            .any(|part| part == needle)
            || headers
                .clone()
                .any(|value| value.windows(needle.len()).any(|part| part == needle))
            || body.windows(needle.len()).any(|part| part == needle)
        {
            names.insert(reference.name.clone());
        }
    }
    names
}

pub(crate) fn substitute_uri(
    path_and_query: &str,
    references: &[SecretReference],
    secrets: &BTreeMap<String, String>,
) -> Result<String, String> {
    let mut value = path_and_query.to_string();
    for reference in ordered_references(references) {
        let Some(secret) = secrets.get(&reference.name) else {
            continue;
        };
        value = value.replace(
            &reference.reference,
            &utf8_percent_encode(secret, NON_ALPHANUMERIC).to_string(),
        );
    }
    Ok(value)
}

pub(crate) fn substitute_bytes(
    value: &[u8],
    references: &[SecretReference],
    secrets: &BTreeMap<String, String>,
) -> Result<Vec<u8>, String> {
    let mut result = value.to_vec();
    for reference in ordered_references(references) {
        let Some(secret) = secrets.get(&reference.name) else {
            continue;
        };
        result = replace_all(&result, reference.reference.as_bytes(), secret.as_bytes());
    }
    Ok(result)
}

pub(crate) fn sanitize_bytes(
    value: &[u8],
    references: &[SecretReference],
    secrets: &BTreeMap<String, String>,
) -> Vec<u8> {
    let mut result = value.to_vec();
    let mut ordered = references.iter().collect::<Vec<_>>();
    ordered.sort_by_key(|reference| {
        std::cmp::Reverse(
            secrets
                .get(&reference.name)
                .map_or(0, |secret| secret.len()),
        )
    });
    for reference in ordered {
        if let Some(secret) = secrets.get(&reference.name)
            && !secret.is_empty()
        {
            result = replace_all(&result, secret.as_bytes(), reference.reference.as_bytes());
        }
    }
    result
}

pub(crate) fn query_names(path_and_query: &str) -> Vec<String> {
    let Some((_, query)) = path_and_query.split_once('?') else {
        return Vec::new();
    };
    query
        .split('&')
        .filter_map(|pair| {
            pair.split_once('=')
                .map_or(Some(pair), |(name, _)| Some(name))
        })
        .filter(|name| !name.is_empty())
        .map(|name| name.chars().take(128).collect())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn ordered_references(references: &[SecretReference]) -> Vec<&SecretReference> {
    let mut ordered = references.iter().collect::<Vec<_>>();
    ordered.sort_by_key(|reference| std::cmp::Reverse(reference.reference.len()));
    ordered
}

fn replace_all(value: &[u8], needle: &[u8], replacement: &[u8]) -> Vec<u8> {
    if needle.is_empty() {
        return value.to_vec();
    }
    let mut result = Vec::with_capacity(value.len());
    let mut cursor = 0;
    while let Some(offset) = value[cursor..]
        .windows(needle.len())
        .position(|part| part == needle)
    {
        let start = cursor + offset;
        result.extend_from_slice(&value[cursor..start]);
        result.extend_from_slice(replacement);
        cursor = start + needle.len();
    }
    result.extend_from_slice(&value[cursor..]);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn references() -> Vec<SecretReference> {
        vec![SecretReference {
            name: "API_TOKEN".into(),
            reference: "avref_012345".into(),
        }]
    }

    fn secrets() -> BTreeMap<String, String> {
        BTreeMap::from([("API_TOKEN".into(), "a token/?&".into())])
    }

    #[test]
    fn substitutes_url_components_without_changing_query_structure() {
        assert_eq!(
            substitute_uri(
                "/v1/avref_012345?token=avref_012345&mode=x",
                &references(),
                &secrets()
            )
            .unwrap(),
            "/v1/a%20token%2F%3F%26?token=a%20token%2F%3F%26&mode=x"
        );
    }

    #[test]
    fn substitutes_request_bytes_and_sanitizes_response_bytes() {
        let request = substitute_bytes(
            b"Bearer avref_012345 / avref_012345",
            &references(),
            &secrets(),
        )
        .unwrap();
        assert_eq!(request, b"Bearer a token/?& / a token/?&");
        assert_eq!(
            sanitize_bytes(&request, &references(), &secrets()),
            b"Bearer avref_012345 / avref_012345"
        );
    }

    #[test]
    fn reports_only_query_names() {
        assert_eq!(
            query_names("/v1?token=secret&empty=&mode=fast"),
            vec!["empty", "mode", "token"]
        );
    }

    #[test]
    fn finds_references_in_each_supported_location() {
        assert_eq!(
            referenced_names(
                &references(),
                "/v1",
                vec![b"Bearer avref_012345".to_vec()].into_iter(),
                b""
            ),
            BTreeSet::from(["API_TOKEN".into()])
        );
    }
}
