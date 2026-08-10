mod connector;
mod handler;
mod policy;
mod protocol;
mod transform;

use connector::{DeadlineConnector, http_connector};
use handler::{ProxyFailure, ProxyHandler};
use http_body_util::{BodyExt, combinators::BoxBody};
use http_mitm_proxy::MitmProxy;
use hudsucker::hyper::body::{Bytes, Incoming};
use hudsucker::hyper::service::{HttpService, service_fn};
use hudsucker::hyper::{Method, Request};
use hudsucker::hyper_util::client::legacy::Client;
use hudsucker::hyper_util::rt::{TokioExecutor, TokioIo};
use hudsucker::rcgen::{
    BasicConstraints, CertificateParams, DistinguishedName, DnType, IsCa, Issuer, KeyPair,
    KeyUsagePurpose,
};
use hyper_rustls::HttpsConnectorBuilder;
use policy::CanonicalDestination;
use protocol::{read_bootstrap, send_ready, spawn_broker};
use std::convert::Infallible;
use std::os::fd::FromRawFd;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::sync::watch;
use transform::SecretReference;
use zeroize::Zeroizing;

const CONTROL_ENV: &str = "AV_PROXY_CONTROL";

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("av-proxy-helper: {error}");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), String> {
    hudsucker::rustls::crypto::aws_lc_rs::default_provider()
        .install_default()
        .map_err(|_| "failed to install the proxy TLS crypto provider".to_string())?;
    let (mut control_reader, mut control_writer) = inherited_control_channels()?;
    let bootstrap =
        tokio::time::timeout(Duration::from_secs(5), read_bootstrap(&mut control_reader))
            .await
            .map_err(|_| "bootstrap timed out".to_string())??;
    let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))
        .await
        .map_err(|error| format!("failed to bind loopback proxy: {error}"))?;
    let port = listener
        .local_addr()
        .map_err(|error| format!("failed to inspect loopback proxy: {error}"))?
        .port();
    let (authority, ca_pem) = ephemeral_authority()?;

    send_ready(&mut control_writer, &bootstrap.session_id, port, &ca_pem).await?;
    let (shutdown_sender, mut shutdown_receiver) = watch::channel(false);
    let broker = spawn_broker(
        bootstrap.session_id.clone(),
        control_reader,
        control_writer,
        shutdown_sender,
    );
    let credential = Arc::new(Zeroizing::new(bootstrap.proxy_credential));
    let references = bootstrap
        .references
        .into_iter()
        .map(|(name, reference)| SecretReference { name, reference })
        .collect();
    let handler = ProxyHandler::new(Arc::clone(&credential), references, broker);

    let https = HttpsConnectorBuilder::new()
        .with_webpki_roots()
        .https_or_http()
        .enable_http1()
        .wrap_connector(http_connector());
    let connector = DeadlineConnector::new(https);
    let mut client_builder = Client::builder(TokioExecutor::new());
    client_builder.pool_max_idle_per_host(0);
    let client = client_builder.build(connector);
    let proxy = Arc::new(MitmProxy::new(
        Some(authority),
        Some(http_mitm_proxy::moka::sync::Cache::new(256)),
    ));

    loop {
        tokio::select! {
            changed = shutdown_receiver.changed() => {
                if changed.is_err() || *shutdown_receiver.borrow() {
                    return Ok(());
                }
            }
            accepted = listener.accept() => {
                let (stream, _) = accepted
                    .map_err(|error| format!("failed to accept proxy connection: {error}"))?;
                let connection_proxy = Arc::clone(&proxy);
                let connection_handler = handler.clone();
                let connection_client = client.clone();
                let connection_credential = credential.clone();
                tokio::spawn(async move {
                    serve_connection(
                        stream,
                        connection_proxy,
                        connection_handler,
                        connection_client,
                        connection_credential,
                    )
                    .await;
                });
            }
        }
    }
}

async fn serve_connection<C>(
    stream: tokio::net::TcpStream,
    proxy: Arc<MitmProxy<Issuer<'static, KeyPair>>>,
    handler: ProxyHandler,
    client: Client<C, hudsucker::Body>,
    credential: Arc<Zeroizing<String>>,
) where
    C: hudsucker::hyper_util::client::legacy::connect::Connect + Clone + Send + Sync + 'static,
{
    let authenticated = Arc::new(AtomicBool::new(false));
    let inner_authenticated = Arc::clone(&authenticated);
    let inner = service_fn(move |request: Request<Incoming>| {
        let client = client.clone();
        let mut handler = handler.clone();
        let authenticated = Arc::clone(&inner_authenticated);
        async move {
            if !authenticated.load(Ordering::Acquire) {
                return Ok::<_, Infallible>(ProxyFailure::proxy_auth_required().response());
            }
            let request = request.map(hudsucker::Body::from);
            let transaction = async {
                let request = handler.prepare_request(request).await?;
                let response = client
                    .request(request)
                    .await
                    .map_err(|_| ProxyFailure::bad_gateway("upstream request failed"))?
                    .map(hudsucker::Body::from);
                handler.prepare_response(response).await
            };
            Ok(
                match tokio::time::timeout(Duration::from_secs(30), transaction).await {
                    Ok(Ok(response)) => response,
                    Ok(Err(failure)) => failure.response(),
                    Err(_) => {
                        ProxyFailure::gateway_timeout("proxy transaction timed out").response()
                    }
                },
            )
        }
    });
    let outer = authenticated_service(
        MitmProxy::wrap_service(proxy, inner),
        authenticated,
        credential,
    );
    let _ = hudsucker::hyper::server::conn::http1::Builder::new()
        .preserve_header_case(true)
        .title_case_headers(true)
        .serve_connection(TokioIo::new(stream), outer)
        .with_upgrades()
        .await;
}

fn authenticated_service<M>(
    inner: M,
    authenticated: Arc<AtomicBool>,
    credential: Arc<Zeroizing<String>>,
) -> impl HttpService<
    Incoming,
    ResBody = BoxBody<Bytes, hudsucker::Error>,
    Error = M::Error,
    Future: Send,
> + Send
where
    M: HttpService<Incoming, ResBody = BoxBody<Bytes, hudsucker::Error>> + Send + 'static,
    M::Future: Send + 'static,
    M::Error: 'static,
{
    let inner = Arc::new(tokio::sync::Mutex::new(inner));
    service_fn(move |mut request: Request<Incoming>| {
        let inner = Arc::clone(&inner);
        let authenticated = Arc::clone(&authenticated);
        let credential = credential.clone();
        async move {
            let failure = if request.method() == Method::CONNECT
                && CanonicalDestination::from_connect_uri(request.uri()).is_err()
            {
                Some(ProxyFailure::forbidden(
                    "private or unsupported destination",
                ))
            } else if ProxyHandler::authenticate(&mut request, credential.as_str()).is_err() {
                Some(ProxyFailure::proxy_auth_required())
            } else {
                None
            };
            if let Some(failure) = failure {
                return Ok(failure.response().map(BodyExt::boxed));
            }
            authenticated.store(true, Ordering::Release);
            inner.lock().await.call(request).await
        }
    })
}

fn inherited_control_channels() -> Result<(tokio::fs::File, tokio::fs::File), String> {
    if std::env::var(CONTROL_ENV).as_deref() != Ok("1") {
        return Err("must be launched by Automic Vault".into());
    }
    if unsafe { libc::fcntl(libc::STDIN_FILENO, libc::F_GETFD) } == -1
        || unsafe { libc::fcntl(libc::STDOUT_FILENO, libc::F_GETFD) } == -1
    {
        return Err("invalid control channels".into());
    }
    let input = unsafe { std::fs::File::from_raw_fd(libc::STDIN_FILENO) };
    let output = unsafe { std::fs::File::from_raw_fd(libc::STDOUT_FILENO) };
    Ok((
        tokio::fs::File::from_std(input),
        tokio::fs::File::from_std(output),
    ))
}

fn ephemeral_authority() -> Result<(Issuer<'static, KeyPair>, String), String> {
    let key_pair =
        KeyPair::generate().map_err(|error| format!("failed to generate CA key: {error}"))?;
    let mut params = CertificateParams::default();
    let mut distinguished_name = DistinguishedName::new();
    distinguished_name.push(DnType::CommonName, "Automic Vault Proxy Session");
    distinguished_name.push(DnType::OrganizationName, "Automic Vault");
    params.distinguished_name = distinguished_name;
    params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    params.key_usages = vec![
        KeyUsagePurpose::DigitalSignature,
        KeyUsagePurpose::KeyCertSign,
        KeyUsagePurpose::CrlSign,
    ];
    let certificate = params
        .self_signed(&key_pair)
        .map_err(|error| format!("failed to generate CA certificate: {error}"))?;
    let pem = certificate.pem();
    let issuer = Issuer::new(params, key_pair);
    Ok((issuer, pem))
}
