use crate::policy::CanonicalDestination;
use crate::protocol::{AuthorizationMetadata, SecretBroker};
use crate::transform::{
    SecretReference, query_names, referenced_names, sanitize_bytes, substitute_bytes,
    substitute_uri,
};
use base64::Engine;
use http_body_util::{BodyExt, Limited};
use hudsucker::Body;
use hudsucker::hyper::header::{
    CONNECTION, CONTENT_LENGTH, HOST, HeaderName, HeaderValue, PROXY_AUTHENTICATE,
    PROXY_AUTHORIZATION, TE, TRAILER, TRANSFER_ENCODING, UPGRADE,
};
use hudsucker::hyper::{Request, Response, StatusCode, Uri, Version};
use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;
use subtle::ConstantTimeEq;
use zeroize::{Zeroize, Zeroizing};

const MAX_BODY_BYTES: usize = 10 * 1024 * 1024;
const AUTHORIZATION_TIMEOUT: Duration = Duration::from_secs(30);

pub(crate) struct ProxyHandler {
    credential: Arc<Zeroizing<String>>,
    references: Arc<Vec<SecretReference>>,
    broker: SecretBroker,
    response_secrets: BTreeMap<String, String>,
}

impl ProxyHandler {
    pub(crate) fn new(
        credential: Arc<Zeroizing<String>>,
        references: Vec<SecretReference>,
        broker: SecretBroker,
    ) -> Self {
        Self {
            credential,
            references: Arc::new(references),
            broker,
            response_secrets: BTreeMap::new(),
        }
    }

    pub(crate) async fn prepare_request(
        &mut self,
        request: Request<Body>,
    ) -> Result<Request<Body>, ProxyFailure> {
        zeroize_secret_values(&mut self.response_secrets);
        reject_protocol_upgrade(&request)?;
        let destination =
            CanonicalDestination::from_uri(request.uri()).map_err(ProxyFailure::forbidden)?;
        if self.references.iter().any(|reference| {
            request
                .uri()
                .authority()
                .is_some_and(|authority| authority.as_str().contains(&reference.reference))
                || request.headers().get(HOST).is_some_and(|host| {
                    host.as_bytes()
                        .windows(reference.reference.len())
                        .any(|part| part == reference.reference.as_bytes())
                })
        }) {
            return Err(ProxyFailure::bad_request(
                "secret references are only supported in URL paths and queries",
            ));
        }

        let request = hudsucker::decode_request(request).map_err(|_| {
            ProxyFailure::bad_gateway("request content encoding is not inspectable")
        })?;
        let (mut parts, body) = request.into_parts();
        let body = collect_body(body).await?;
        strip_hop_by_hop_headers(&mut parts.headers);
        let path_and_query = parts
            .uri
            .path_and_query()
            .map_or("/", |value| value.as_str())
            .to_string();
        let header_values = parts
            .headers
            .iter()
            .filter(|(name, _)| *name != PROXY_AUTHORIZATION)
            .map(|(_, value)| value.as_bytes().to_vec())
            .collect::<Vec<_>>();
        let names = referenced_names(
            &self.references,
            &path_and_query,
            header_values.into_iter(),
            &body,
        )
        .into_iter()
        .collect::<Vec<_>>();
        parts.headers.remove(PROXY_AUTHORIZATION);

        if names.is_empty() {
            finalize_request_parts(&mut parts, body.len())?;
            return Ok(Request::from_parts(parts, Body::from(body)));
        }

        let metadata = AuthorizationMetadata {
            method: parts.method.as_str().to_string(),
            origin: destination.origin(),
            path: parts.uri.path().chars().take(2048).collect(),
            query_names: query_names(&path_and_query),
            secret_names: names.clone(),
        };
        let mut authorization =
            tokio::time::timeout(AUTHORIZATION_TIMEOUT, self.broker.authorize(metadata))
                .await
                .map_err(|_| ProxyFailure::gateway_timeout("authorization timed out"))?
                .map_err(ProxyFailure::forbidden)?;
        if authorization.secrets.len() != names.len()
            || names
                .iter()
                .any(|name| authorization.secrets.get(name).is_none_or(String::is_empty))
        {
            return Err(ProxyFailure::bad_gateway(
                "authorization returned incomplete secret material",
            ));
        }

        parts.uri = replace_uri_path(
            &parts.uri,
            substitute_uri(&path_and_query, &self.references, &authorization.secrets)
                .map_err(ProxyFailure::bad_gateway)?,
        )?;
        substitute_headers(
            &mut parts.headers,
            &self.references,
            &authorization.secrets,
            false,
        )?;
        let body = substitute_bytes(&body, &self.references, &authorization.secrets)
            .map_err(ProxyFailure::bad_gateway)?;
        finalize_request_parts(&mut parts, body.len())?;
        self.response_secrets = std::mem::take(&mut authorization.secrets);
        Ok(Request::from_parts(parts, Body::from(body)))
    }

    pub(crate) async fn prepare_response(
        &mut self,
        response: Response<Body>,
    ) -> Result<Response<Body>, ProxyFailure> {
        let response = hudsucker::decode_response(response).map_err(|_| {
            ProxyFailure::bad_gateway("response content encoding is not inspectable")
        })?;
        let (mut parts, body) = response.into_parts();
        let body = collect_body(body).await?;
        strip_hop_by_hop_headers(&mut parts.headers);
        if !self.response_secrets.is_empty() {
            substitute_headers(
                &mut parts.headers,
                &self.references,
                &self.response_secrets,
                true,
            )?;
        }
        let body = sanitize_bytes(&body, &self.references, &self.response_secrets);
        normalize_body_headers(&mut parts.headers, body.len())?;
        zeroize_secret_values(&mut self.response_secrets);
        Ok(Response::from_parts(parts, Body::from(body)))
    }

    pub(crate) fn authenticate(
        request: &mut Request<impl Sized>,
        credential: &str,
    ) -> Result<(), ProxyFailure> {
        let mut values = request.headers().get_all(PROXY_AUTHORIZATION).iter();
        let Some(value) = values.next() else {
            return Err(ProxyFailure::proxy_auth_required());
        };
        if values.next().is_some() || !valid_proxy_authorization(value, credential) {
            return Err(ProxyFailure::proxy_auth_required());
        }
        request.headers_mut().remove(PROXY_AUTHORIZATION);
        Ok(())
    }
}

impl Clone for ProxyHandler {
    fn clone(&self) -> Self {
        Self {
            credential: Arc::clone(&self.credential),
            references: Arc::clone(&self.references),
            broker: self.broker.clone(),
            response_secrets: BTreeMap::new(),
        }
    }
}

impl Drop for ProxyHandler {
    fn drop(&mut self) {
        zeroize_secret_values(&mut self.response_secrets);
    }
}

fn valid_proxy_authorization(value: &HeaderValue, credential: &str) -> bool {
    let Ok(value) = value.to_str() else {
        return false;
    };
    let Some(encoded) = value.strip_prefix("Basic ") else {
        return false;
    };
    let Ok(decoded) = base64::engine::general_purpose::STANDARD.decode(encoded) else {
        return false;
    };
    let expected = format!("av:{credential}");
    decoded.len() == expected.len() && bool::from(decoded.ct_eq(expected.as_bytes()))
}

async fn collect_body(body: Body) -> Result<Vec<u8>, ProxyFailure> {
    Limited::new(body, MAX_BODY_BYTES)
        .collect()
        .await
        .map(|body| body.to_bytes().to_vec())
        .map_err(|_| ProxyFailure::payload_too_large())
}

fn replace_uri_path(uri: &Uri, path_and_query: String) -> Result<Uri, ProxyFailure> {
    let mut parts = uri.clone().into_parts();
    parts.path_and_query = Some(
        path_and_query
            .parse()
            .map_err(|_| ProxyFailure::bad_request("secret is not valid in this URL"))?,
    );
    Uri::from_parts(parts).map_err(|_| ProxyFailure::bad_request("secret is not valid in this URL"))
}

fn substitute_headers(
    headers: &mut hudsucker::hyper::HeaderMap,
    references: &[SecretReference],
    secrets: &BTreeMap<String, String>,
    sanitize: bool,
) -> Result<(), ProxyFailure> {
    let names = headers.keys().cloned().collect::<Vec<HeaderName>>();
    for name in names {
        let values = headers.get_all(&name).iter().cloned().collect::<Vec<_>>();
        if values.is_empty() {
            continue;
        }
        headers.remove(&name);
        for value in values {
            let bytes = if sanitize {
                sanitize_bytes(value.as_bytes(), references, secrets)
            } else {
                substitute_bytes(value.as_bytes(), references, secrets)
                    .map_err(ProxyFailure::bad_gateway)?
            };
            let value = HeaderValue::from_bytes(&bytes)
                .map_err(|_| ProxyFailure::bad_gateway("secret is not valid in this header"))?;
            headers.append(name.clone(), value);
        }
    }
    Ok(())
}

fn normalize_body_headers(
    headers: &mut hudsucker::hyper::HeaderMap,
    body_length: usize,
) -> Result<(), ProxyFailure> {
    headers.remove(TRANSFER_ENCODING);
    headers.insert(
        CONTENT_LENGTH,
        HeaderValue::from_str(&body_length.to_string())
            .map_err(|_| ProxyFailure::bad_gateway("body length is invalid"))?,
    );
    Ok(())
}

fn finalize_request_parts(
    parts: &mut hudsucker::hyper::http::request::Parts,
    body_length: usize,
) -> Result<(), ProxyFailure> {
    parts.version = Version::HTTP_11;
    let authority = parts
        .uri
        .authority()
        .ok_or_else(|| ProxyFailure::bad_request("request has no authority"))?;
    parts.headers.insert(
        HOST,
        HeaderValue::from_str(authority.as_str())
            .map_err(|_| ProxyFailure::bad_request("request authority is invalid"))?,
    );
    normalize_body_headers(&mut parts.headers, body_length)
}

fn strip_hop_by_hop_headers(headers: &mut hudsucker::hyper::HeaderMap) {
    let nominated = headers
        .get_all(CONNECTION)
        .iter()
        .flat_map(|value| value.as_bytes().split(|byte| *byte == b','))
        .filter_map(|value| HeaderName::from_bytes(value.trim_ascii()).ok())
        .collect::<Vec<_>>();
    for name in nominated {
        headers.remove(name);
    }
    for name in [
        CONNECTION,
        HeaderName::from_static("keep-alive"),
        TE,
        TRAILER,
        TRANSFER_ENCODING,
        UPGRADE,
        PROXY_AUTHORIZATION,
        HeaderName::from_static("proxy-connection"),
    ] {
        headers.remove(name);
    }
}

fn zeroize_secret_values(secrets: &mut BTreeMap<String, String>) {
    for value in secrets.values_mut() {
        value.zeroize();
    }
    secrets.clear();
}

fn reject_protocol_upgrade(request: &Request<Body>) -> Result<(), ProxyFailure> {
    if request.headers().contains_key(UPGRADE)
        || request.headers().get_all(CONNECTION).iter().any(|value| {
            value
                .as_bytes()
                .split(|byte| *byte == b',')
                .any(|token| token.trim_ascii().eq_ignore_ascii_case(b"upgrade"))
        })
    {
        return Err(ProxyFailure::bad_request(
            "protocol upgrades are not supported",
        ));
    }
    Ok(())
}

pub(crate) struct ProxyFailure {
    status: StatusCode,
    proxy_authenticate: bool,
}

impl ProxyFailure {
    fn bad_request(_reason: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            proxy_authenticate: false,
        }
    }

    pub(crate) fn forbidden(_reason: impl Into<String>) -> Self {
        Self {
            status: StatusCode::FORBIDDEN,
            proxy_authenticate: false,
        }
    }

    fn payload_too_large() -> Self {
        Self {
            status: StatusCode::PAYLOAD_TOO_LARGE,
            proxy_authenticate: false,
        }
    }

    pub(crate) fn bad_gateway(_reason: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_GATEWAY,
            proxy_authenticate: false,
        }
    }

    pub(crate) fn gateway_timeout(_reason: impl Into<String>) -> Self {
        Self {
            status: StatusCode::GATEWAY_TIMEOUT,
            proxy_authenticate: false,
        }
    }

    pub(crate) fn proxy_auth_required() -> Self {
        Self {
            status: StatusCode::PROXY_AUTHENTICATION_REQUIRED,
            proxy_authenticate: true,
        }
    }

    pub(crate) fn response(self) -> Response<Body> {
        let mut response = Response::builder().status(self.status);
        if self.proxy_authenticate {
            response = response.header(PROXY_AUTHENTICATE, "Basic realm=\"Automic Vault\"");
        }
        response.body(Body::empty()).expect("static proxy response")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_proxy_authorization_in_constant_time_shape() {
        let valid = format!(
            "Basic {}",
            base64::engine::general_purpose::STANDARD.encode("av:credential")
        );
        assert!(valid_proxy_authorization(
            &HeaderValue::from_str(&valid).unwrap(),
            "credential"
        ));
        assert!(!valid_proxy_authorization(
            &HeaderValue::from_static("Basic Zm9vOmJhcg=="),
            "credential"
        ));
    }

    #[test]
    fn rejects_upgrade_requests() {
        let request = Request::builder()
            .uri("http://example.com/")
            .header(CONNECTION, "keep-alive, Upgrade")
            .body(Body::empty())
            .unwrap();
        assert!(reject_protocol_upgrade(&request).is_err());
    }
}
