use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::sync::{mpsc, oneshot, watch};
use zeroize::Zeroize;

const MAX_CONTROL_FRAME: usize = 4 * 1024 * 1024;

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub(crate) struct Bootstrap {
    pub(crate) session_id: String,
    pub(crate) proxy_credential: String,
    pub(crate) target: TargetIdentity,
    pub(crate) references: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub(crate) struct TargetIdentity {
    pub(crate) pid: i32,
    pub(crate) pid_version: i32,
    pub(crate) start_usec: u64,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum AppMessage {
    Bootstrap(Bootstrap),
    Authorization {
        request_id: u64,
        allowed: bool,
        #[serde(default)]
        secrets: BTreeMap<String, String>,
        reason: Option<String>,
    },
    Shutdown,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum HelperMessage<'a> {
    Ready {
        session_id: &'a str,
        port: u16,
        ca_pem: &'a str,
    },
    Authorize {
        request_id: u64,
        session_id: &'a str,
        method: &'a str,
        origin: &'a str,
        path: &'a str,
        query_names: &'a [String],
        secret_names: &'a [String],
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct AuthorizationMetadata {
    pub(crate) method: String,
    pub(crate) origin: String,
    pub(crate) path: String,
    pub(crate) query_names: Vec<String>,
    pub(crate) secret_names: Vec<String>,
}

pub(crate) struct AuthorizationResult {
    pub(crate) secrets: BTreeMap<String, String>,
}

impl Drop for AuthorizationResult {
    fn drop(&mut self) {
        for secret in self.secrets.values_mut() {
            secret.zeroize();
        }
        self.secrets.clear();
    }
}

struct AuthorizationCall {
    metadata: AuthorizationMetadata,
    response: oneshot::Sender<Result<AuthorizationResult, String>>,
}

#[derive(Clone)]
pub(crate) struct SecretBroker {
    calls: mpsc::Sender<AuthorizationCall>,
}

impl SecretBroker {
    pub(crate) async fn authorize(
        &self,
        metadata: AuthorizationMetadata,
    ) -> Result<AuthorizationResult, String> {
        let (response, receiver) = oneshot::channel();
        self.calls
            .send(AuthorizationCall { metadata, response })
            .await
            .map_err(|_| "Automic Vault control channel is closed".to_string())?;
        receiver
            .await
            .map_err(|_| "Automic Vault control channel is closed".to_string())?
    }
}

pub(crate) async fn read_bootstrap<R: AsyncRead + Unpin>(
    reader: &mut R,
) -> Result<Bootstrap, String> {
    match read_frame::<_, AppMessage>(reader).await? {
        AppMessage::Bootstrap(bootstrap) => validate_bootstrap(bootstrap),
        _ => Err("first control message was not a bootstrap".into()),
    }
}

pub(crate) async fn send_ready<W: AsyncWrite + Unpin>(
    writer: &mut W,
    session_id: &str,
    port: u16,
    ca_pem: &str,
) -> Result<(), String> {
    write_frame(
        writer,
        &HelperMessage::Ready {
            session_id,
            port,
            ca_pem,
        },
    )
    .await
}

pub(crate) fn spawn_broker<R, W>(
    session_id: String,
    reader: R,
    writer: W,
    shutdown: watch::Sender<bool>,
) -> SecretBroker
where
    R: AsyncRead + Unpin + Send + 'static,
    W: AsyncWrite + Unpin + Send + 'static,
{
    let (calls, receiver) = mpsc::channel(32);
    tokio::spawn(run_broker(session_id, reader, writer, receiver, shutdown));
    SecretBroker { calls }
}

async fn run_broker<R, W>(
    session_id: String,
    mut reader: R,
    mut writer: W,
    mut calls: mpsc::Receiver<AuthorizationCall>,
    shutdown: watch::Sender<bool>,
) where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    let next_id = Arc::new(AtomicU64::new(1));
    let mut pending = BTreeMap::<u64, oneshot::Sender<Result<AuthorizationResult, String>>>::new();
    loop {
        tokio::select! {
            call = calls.recv() => {
                let Some(call) = call else { break };
                let request_id = next_id.fetch_add(1, Ordering::Relaxed);
                let metadata = &call.metadata;
                let message = HelperMessage::Authorize {
                    request_id,
                    session_id: &session_id,
                    method: &metadata.method,
                    origin: &metadata.origin,
                    path: &metadata.path,
                    query_names: &metadata.query_names,
                    secret_names: &metadata.secret_names,
                };
                if let Err(error) = write_frame(&mut writer, &message).await {
                    let _ = call.response.send(Err(error));
                    break;
                }
                pending.insert(request_id, call.response);
            }
            message = read_frame::<_, AppMessage>(&mut reader) => {
                match message {
                    Ok(AppMessage::Authorization { request_id, allowed, secrets, reason }) => {
                        let Some(response) = pending.remove(&request_id) else { break };
                        let result = if allowed {
                            Ok(AuthorizationResult { secrets })
                        } else {
                            Err(reason.unwrap_or_else(|| "destination access denied".into()))
                        };
                        let _ = response.send(result);
                    }
                    Ok(AppMessage::Shutdown) => break,
                    _ => break,
                }
            }
        }
    }
    for (_, response) in pending {
        let _ = response.send(Err("Automic Vault control channel is closed".into()));
    }
    let _ = shutdown.send(true);
}

fn validate_bootstrap(bootstrap: Bootstrap) -> Result<Bootstrap, String> {
    if bootstrap.session_id.len() < 16 || bootstrap.proxy_credential.len() < 32 {
        return Err("bootstrap contains weak session material".into());
    }
    if bootstrap.target.pid <= 1
        || bootstrap.target.pid_version <= 0
        || bootstrap.target.start_usec == 0
    {
        return Err("bootstrap contains an invalid target identity".into());
    }
    if bootstrap.references.is_empty() || bootstrap.references.len() > 128 {
        return Err("bootstrap contains an invalid number of secret references".into());
    }
    let mut unique = std::collections::BTreeSet::new();
    for (name, reference) in &bootstrap.references {
        if !valid_secret_name(name)
            || reference.len() < 32
            || !reference
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
            || !unique.insert(reference)
        {
            return Err("bootstrap contains an invalid secret reference".into());
        }
    }
    Ok(bootstrap)
}

fn valid_secret_name(name: &str) -> bool {
    let mut bytes = name.bytes();
    bytes
        .next()
        .is_some_and(|byte| byte.is_ascii_alphabetic() || byte == b'_')
        && bytes.all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

async fn read_frame<R: AsyncRead + Unpin, T: DeserializeOwned>(
    reader: &mut R,
) -> Result<T, String> {
    let length = reader
        .read_u32()
        .await
        .map_err(|error| format!("failed to read control frame: {error}"))?
        as usize;
    if length == 0 || length > MAX_CONTROL_FRAME {
        return Err("control frame has an invalid length".into());
    }
    let mut bytes = vec![0; length];
    reader
        .read_exact(&mut bytes)
        .await
        .map_err(|error| format!("failed to read control frame: {error}"))?;
    serde_json::from_slice(&bytes).map_err(|error| format!("invalid control frame: {error}"))
}

async fn write_frame<W: AsyncWrite + Unpin, T: Serialize>(
    writer: &mut W,
    value: &T,
) -> Result<(), String> {
    let bytes = serde_json::to_vec(value)
        .map_err(|error| format!("failed to encode control frame: {error}"))?;
    if bytes.is_empty() || bytes.len() > MAX_CONTROL_FRAME {
        return Err("control frame has an invalid length".into());
    }
    writer
        .write_u32(bytes.len() as u32)
        .await
        .map_err(|error| format!("failed to write control frame: {error}"))?;
    writer
        .write_all(&bytes)
        .await
        .map_err(|error| format!("failed to write control frame: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bootstrap() -> Bootstrap {
        Bootstrap {
            session_id: "session_0123456789".into(),
            proxy_credential: "credential_0123456789012345678901".into(),
            target: TargetIdentity {
                pid: 42,
                pid_version: 7,
                start_usec: 123,
            },
            references: BTreeMap::from([(
                "API_TOKEN".into(),
                "avref_01234567890123456789012345".into(),
            )]),
        }
    }

    #[test]
    fn validates_strong_unique_bootstrap_material() {
        assert_eq!(validate_bootstrap(bootstrap()).unwrap(), bootstrap());
        let mut duplicate = bootstrap();
        duplicate.references.insert(
            "OTHER_TOKEN".into(),
            "avref_01234567890123456789012345".into(),
        );
        assert!(validate_bootstrap(duplicate).is_err());
    }

    #[test]
    fn accepts_base64url_secret_references() {
        let mut value = bootstrap();
        value.references.insert(
            "API_TOKEN".into(),
            "avref_-01234567890123456789012345".into(),
        );
        assert!(validate_bootstrap(value).is_ok());
    }

    #[tokio::test]
    async fn length_delimited_frames_round_trip() {
        let (mut left, mut right) = tokio::io::duplex(4096);
        let expected = bootstrap();
        let write = tokio::spawn(async move {
            write_frame(
                &mut left,
                &HelperMessage::Ready {
                    session_id: &expected.session_id,
                    port: 1234,
                    ca_pem: "certificate",
                },
            )
            .await
        });
        let length = right.read_u32().await.unwrap() as usize;
        let mut bytes = vec![0; length];
        right.read_exact(&mut bytes).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(json["type"], "ready");
        assert_eq!(json["port"], 1234);
        write.await.unwrap().unwrap();
    }

    #[test]
    fn decodes_the_flat_bootstrap_wire_message() {
        let value = serde_json::json!({
            "type": "bootstrap",
            "session_id": "session_0123456789",
            "proxy_credential": "credential_0123456789012345678901",
            "target": { "pid": 42, "pid_version": 7, "start_usec": 123 },
            "references": { "API_TOKEN": "avref_01234567890123456789012345" }
        });
        let message: AppMessage = serde_json::from_value(value).unwrap();
        let AppMessage::Bootstrap(decoded) = message else {
            panic!("expected bootstrap message");
        };
        assert_eq!(decoded, bootstrap());
    }
}
