use rusqlite::{Connection, OpenFlags, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::env;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread;

const DEFAULT_BIND_ADDR: &str = "127.0.0.1:3004";
const DEFAULT_DB_PATH: &str = "/var/lib/automic-vault-web/pkg.sqlite";
const DEFAULT_ORIGIN_HEADER: &str = "x-automic-vault-origin";
const HTML_CACHE_CONTROL: &str = "public, max-age=86400, s-maxage=86400";
const DEFAULT_SEARCH_LIMIT: usize = 8;
const MAX_SEARCH_LIMIT: usize = 50;

fn main() {
    if let Err(err) = run() {
        eprintln!("av-web: {err}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let state = Arc::new(AppState::from_env());
    assert_database_ready(&state.db_path)?;
    let listener = TcpListener::bind(&state.bind_addr)
        .map_err(|err| format!("failed to bind {}: {err}", state.bind_addr))?;
    eprintln!("av-web listening on {}", state.bind_addr);

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let state = Arc::clone(&state);
                thread::spawn(move || {
                    if let Err(err) = handle_connection(stream, &state) {
                        eprintln!("request failed: {err}");
                    }
                });
            }
            Err(err) => eprintln!("accept failed: {err}"),
        }
    }
    Ok(())
}

fn assert_database_ready(path: &Path) -> Result<(), String> {
    let connection = open_database(path)?;
    connection
        .query_row(
            "SELECT value FROM metadata WHERE key = 'schema'",
            [],
            |_row| Ok(()),
        )
        .map_err(|err| format!("database {} is not ready: {err}", path.display()))
}

#[derive(Debug)]
struct AppState {
    bind_addr: String,
    db_path: PathBuf,
    origin_header: String,
    origin_secret: Option<String>,
}

impl AppState {
    fn from_env() -> Self {
        Self {
            bind_addr: env::var("AV_WEB_BIND_ADDR")
                .unwrap_or_else(|_| DEFAULT_BIND_ADDR.to_string()),
            db_path: env::var_os("AV_WEB_DB_PATH")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from(DEFAULT_DB_PATH)),
            origin_header: env::var("AV_WEB_ORIGIN_HEADER")
                .unwrap_or_else(|_| DEFAULT_ORIGIN_HEADER.to_string())
                .to_ascii_lowercase(),
            origin_secret: env::var("AV_WEB_ORIGIN_SECRET")
                .ok()
                .filter(|value| !value.is_empty()),
        }
    }
}

fn handle_connection(mut stream: TcpStream, state: &AppState) -> Result<(), String> {
    let request = match read_request(&stream)? {
        Some(request) => request,
        None => return Ok(()),
    };

    if request.path == "/healthz" {
        return write_response(
            &mut stream,
            &request.method,
            200,
            "OK",
            vec![
                ("content-type", "text/plain; charset=utf-8".to_string()),
                ("cache-control", "no-store".to_string()),
            ],
            b"ok\n".to_vec(),
        );
    }

    if !origin_request_authorized(
        &request.path,
        &request.headers,
        &state.origin_header,
        state.origin_secret.as_deref(),
    ) {
        return write_response(
            &mut stream,
            &request.method,
            403,
            "Forbidden",
            vec![
                ("content-type", "text/plain; charset=utf-8".to_string()),
                ("cache-control", "no-store".to_string()),
            ],
            b"forbidden\n".to_vec(),
        );
    }

    if request.method != "GET" && request.method != "HEAD" {
        return write_response(
            &mut stream,
            &request.method,
            405,
            "Method Not Allowed",
            vec![
                ("content-type", "text/plain; charset=utf-8".to_string()),
                ("allow", "GET, HEAD".to_string()),
                ("cache-control", "no-store".to_string()),
            ],
            b"method not allowed\n".to_vec(),
        );
    }

    if let Some(location) = slash_redirect_location(&request.path, request.query.as_deref()) {
        return write_response(
            &mut stream,
            &request.method,
            301,
            "Moved Permanently",
            vec![
                ("location", location),
                ("cache-control", HTML_CACHE_CONTROL.to_string()),
            ],
            Vec::new(),
        );
    }

    if is_search_path(&request.path) {
        let query = parse_query(request.query.as_deref().unwrap_or(""));
        let body = search_response_json(&state.db_path, &request.path, &query)?;
        return write_response(
            &mut stream,
            &request.method,
            200,
            "OK",
            vec![
                (
                    "content-type",
                    "application/json; charset=utf-8".to_string(),
                ),
                ("cache-control", HTML_CACHE_CONTROL.to_string()),
            ],
            body,
        );
    }

    if let Some(response) = response_for_path(&state.db_path, &request.path)? {
        return write_response(
            &mut stream,
            &request.method,
            200,
            "OK",
            response.headers(),
            response.body,
        );
    }

    if let Some(response) = dynamic_response_for_path(&state.db_path, &request.path)? {
        return write_response(
            &mut stream,
            &request.method,
            200,
            "OK",
            response.headers(),
            response.body,
        );
    }

    write_response(
        &mut stream,
        &request.method,
        404,
        "Not Found",
        vec![
            ("content-type", "text/plain; charset=utf-8".to_string()),
            ("cache-control", HTML_CACHE_CONTROL.to_string()),
        ],
        b"not found\n".to_vec(),
    )
}

#[derive(Debug, PartialEq, Eq)]
struct Request {
    method: String,
    path: String,
    query: Option<String>,
    headers: BTreeMap<String, String>,
}

fn read_request(stream: &TcpStream) -> Result<Option<Request>, String> {
    let mut reader = BufReader::new(
        stream
            .try_clone()
            .map_err(|err| format!("failed to clone stream: {err}"))?,
    );
    let mut line = String::new();
    if reader
        .read_line(&mut line)
        .map_err(|err| format!("failed to read request line: {err}"))?
        == 0
    {
        return Ok(None);
    }
    let parts = line.split_whitespace().collect::<Vec<_>>();
    if parts.len() < 2 {
        return Err("invalid request line".to_string());
    }
    let method = parts[0].to_ascii_uppercase();
    let (path, query) = split_target(parts[1])?;
    let mut headers = BTreeMap::new();
    loop {
        line.clear();
        let bytes = reader
            .read_line(&mut line)
            .map_err(|err| format!("failed to read header: {err}"))?;
        if bytes == 0 || line == "\n" || line == "\r\n" {
            break;
        }
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        headers.insert(
            name.trim().to_ascii_lowercase(),
            value.trim().trim_end_matches('\r').to_string(),
        );
    }
    Ok(Some(Request {
        method,
        path,
        query,
        headers,
    }))
}

fn split_target(target: &str) -> Result<(String, Option<String>), String> {
    let target = target.split('#').next().unwrap_or(target);
    let (path, query) = target
        .split_once('?')
        .map(|(path, query)| (path, Some(query.to_string())))
        .unwrap_or((target, None));
    let decoded = urlencoding::decode(path)
        .map_err(|err| format!("invalid request path encoding: {err}"))?
        .into_owned();
    let path = if decoded.starts_with('/') {
        decoded
    } else {
        format!("/{decoded}")
    };
    Ok((path, query))
}

fn write_response(
    stream: &mut TcpStream,
    method: &str,
    status: u16,
    reason: &str,
    mut headers: Vec<(&'static str, String)>,
    body: Vec<u8>,
) -> Result<(), String> {
    headers.push(("content-length", body.len().to_string()));
    let mut response = format!("HTTP/1.1 {status} {reason}\r\n");
    for (name, value) in headers {
        response.push_str(name);
        response.push_str(": ");
        response.push_str(&value);
        response.push_str("\r\n");
    }
    response.push_str("\r\n");
    stream
        .write_all(response.as_bytes())
        .map_err(|err| format!("failed to write response headers: {err}"))?;
    if method != "HEAD" {
        stream
            .write_all(&body)
            .map_err(|err| format!("failed to write response body: {err}"))?;
    }
    stream
        .flush()
        .map_err(|err| format!("failed to flush response: {err}"))
}

fn origin_request_authorized(
    path: &str,
    headers: &BTreeMap<String, String>,
    header_name: &str,
    expected_secret: Option<&str>,
) -> bool {
    if path == "/healthz" {
        return true;
    }
    let Some(expected_secret) = expected_secret else {
        return true;
    };
    headers
        .get(&header_name.to_ascii_lowercase())
        .is_some_and(|value| constant_time_eq(value.as_bytes(), expected_secret.as_bytes()))
}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    let mut diff = left.len() ^ right.len();
    for index in 0..left.len().max(right.len()) {
        let left_byte = left.get(index).copied().unwrap_or(0);
        let right_byte = right.get(index).copied().unwrap_or(0);
        diff |= usize::from(left_byte ^ right_byte);
    }
    diff == 0
}

fn slash_redirect_location(path: &str, query: Option<&str>) -> Option<String> {
    let base = match path {
        "/pkg" | "/de/pkg" | "/fr/pkg" | "/ja/pkg" | "/zh-hans/pkg" => {
            format!("{path}/")
        }
        _ => return None,
    };
    Some(match query {
        Some(query) if !query.is_empty() => format!("{base}?{query}"),
        _ => base,
    })
}

fn is_search_path(path: &str) -> bool {
    path == "/pkg/search.json"
        || path == "/de/pkg/search.json"
        || path == "/fr/pkg/search.json"
        || path == "/ja/pkg/search.json"
        || path == "/zh-hans/pkg/search.json"
}

fn normalize_response_path(path: &str) -> String {
    if path.ends_with('/') {
        return format!("{path}index.html");
    }
    if path == "/pkg" || path.ends_with("/pkg") {
        return format!("{path}/index.html");
    }
    if path_has_extension(path) {
        return path.to_string();
    }
    format!("{path}/index.html")
}

fn path_has_extension(path: &str) -> bool {
    path.rsplit('/')
        .next()
        .is_some_and(|segment| segment.contains('.'))
}

#[derive(Debug, PartialEq, Eq)]
struct StoredResponse {
    content_type: String,
    body: Vec<u8>,
    etag: String,
    last_modified: String,
    cache_control: String,
}

impl StoredResponse {
    fn headers(&self) -> Vec<(&'static str, String)> {
        vec![
            ("content-type", self.content_type.clone()),
            ("cache-control", self.cache_control.clone()),
            ("etag", self.etag.clone()),
            ("last-modified", self.last_modified.clone()),
        ]
    }
}

fn open_database(path: &Path) -> Result<Connection, String> {
    Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY
            | OpenFlags::SQLITE_OPEN_URI
            | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|err| format!("failed to open {}: {err}", path.display()))
}

fn response_for_path(db_path: &Path, path: &str) -> Result<Option<StoredResponse>, String> {
    let path = normalize_response_path(path);
    let connection = open_database(db_path)?;
    connection
        .query_row(
            "SELECT content_type, body, etag, last_modified, cache_control FROM responses WHERE path = ?1",
            params![path],
            |row| {
                Ok(StoredResponse {
                    content_type: row.get(0)?,
                    body: row.get(1)?,
                    etag: row.get(2)?,
                    last_modified: row.get(3)?,
                    cache_control: row.get(4)?,
                })
            },
        )
        .optional()
        .map_err(|err| format!("failed to query response: {err}"))
}

#[derive(Debug, Clone)]
struct Locale {
    code: &'static str,
    prefix: &'static str,
    hreflang: &'static str,
}

const LOCALES: &[Locale] = &[
    Locale {
        code: "en",
        prefix: "",
        hreflang: "en",
    },
    Locale {
        code: "de",
        prefix: "/de",
        hreflang: "de",
    },
    Locale {
        code: "fr",
        prefix: "/fr",
        hreflang: "fr",
    },
    Locale {
        code: "ja",
        prefix: "/ja",
        hreflang: "ja",
    },
    Locale {
        code: "zh-Hans",
        prefix: "/zh-hans",
        hreflang: "zh-Hans",
    },
];

const SITE_ORIGIN: &str = "https://www.automicvault.com";
const PROVIDERS: &[&str] = &["brew", "cask", "npm", "pip"];

#[derive(Debug, Clone, Default, Deserialize)]
struct PackageData {
    #[serde(default)]
    aliases: Vec<String>,
    #[serde(default)]
    binaries: Vec<String>,
    #[serde(default)]
    classifiers: Vec<String>,
    #[serde(default)]
    executables: Vec<String>,
    #[serde(default)]
    hubs: Vec<PackageHubData>,
    #[serde(default)]
    keywords: Vec<String>,
    #[serde(default)]
    related: Vec<String>,
    #[serde(default)]
    security: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct PackageHubData {
    #[serde(default)]
    slug: String,
    #[serde(default)]
    label: String,
    #[serde(default)]
    reason: String,
}

#[derive(Debug, Clone)]
struct PackageRow {
    path: String,
    provider: String,
    slug: String,
    package_key: String,
    name: String,
    display_name: String,
    summary: String,
    provider_label: String,
    package_manager_url: String,
    install_command: String,
    native_install_command: String,
    version: String,
    category: String,
    license: String,
    homepage: String,
    repository: String,
    rank: Option<u32>,
    last_updated_at: String,
    indexable: bool,
    data: PackageData,
}

#[derive(Debug, Clone)]
struct HubRow {
    path: String,
    slug: String,
    title: String,
    description: String,
    group: String,
}

fn dynamic_response_for_path(db_path: &Path, path: &str) -> Result<Option<StoredResponse>, String> {
    let (locale, canonical_path) = canonical_pkg_route(path);
    if !canonical_path.starts_with("/pkg/") && canonical_path != "/pkg/index.html" {
        return Ok(None);
    }
    let connection = open_database(db_path)?;
    let body_and_type = if canonical_path == "/pkg/index.html" {
        Some((
            render_index_page(&connection, locale)?,
            "text/html; charset=utf-8",
        ))
    } else if canonical_path == "/pkg/sitemap.xml" {
        Some((
            render_sitemap_index(&connection)?,
            "application/xml; charset=utf-8",
        ))
    } else if canonical_path == "/pkg/sitemap-hubs.xml" {
        Some((
            render_hub_sitemap(&connection)?,
            "application/xml; charset=utf-8",
        ))
    } else if let Some(provider) = sitemap_provider(&canonical_path) {
        Some((
            render_provider_sitemap(&connection, provider)?,
            "application/xml; charset=utf-8",
        ))
    } else if let Some((provider, slug, markdown)) = package_route(&canonical_path) {
        let Some(package) = package_by_provider_slug(&connection, provider, slug)? else {
            return Ok(None);
        };
        if markdown {
            if !package.indexable {
                return Ok(None);
            }
            Some((
                render_package_markdown(&package, locale),
                "text/markdown; charset=utf-8",
            ))
        } else {
            Some((
                render_package_page(&package, locale),
                "text/html; charset=utf-8",
            ))
        }
    } else if let Some(slug) = hub_route(&canonical_path) {
        let Some(hub) = hub_by_slug(&connection, slug)? else {
            return Ok(None);
        };
        Some((
            render_hub_page(&connection, &hub, locale)?,
            "text/html; charset=utf-8",
        ))
    } else {
        None
    };

    let Some((body, content_type)) = body_and_type else {
        return Ok(None);
    };
    Ok(Some(dynamic_stored_response(
        &connection,
        &canonical_path,
        content_type,
        body,
    )?))
}

fn canonical_pkg_route(path: &str) -> (&'static Locale, String) {
    let mut locale = &LOCALES[0];
    let mut canonical = path.to_string();
    for candidate in LOCALES.iter().skip(1) {
        if path == format!("{}/pkg", candidate.prefix) {
            locale = candidate;
            canonical = "/pkg".to_string();
            break;
        }
        if let Some(rest) = path.strip_prefix(&format!("{}/pkg/", candidate.prefix)) {
            locale = candidate;
            canonical = format!("/pkg/{rest}");
            break;
        }
    }
    if canonical == "/pkg" {
        canonical = "/pkg/index.html".to_string();
    } else if canonical.ends_with('/') {
        canonical.push_str("index.html");
    } else if !path_has_extension(&canonical) {
        canonical.push_str("/index.html");
    }
    (locale, canonical)
}

fn package_route(path: &str) -> Option<(&str, &str, bool)> {
    let rest = path.strip_prefix("/pkg/")?;
    let parts = rest.split('/').collect::<Vec<_>>();
    match parts.as_slice() {
        [provider, slug, "index.html"] if PROVIDERS.contains(provider) => {
            Some((provider, slug, false))
        }
        [provider, slug, "index.md"] if PROVIDERS.contains(provider) => {
            Some((provider, slug, true))
        }
        _ => None,
    }
}

fn hub_route(path: &str) -> Option<&str> {
    let rest = path.strip_prefix("/pkg/")?;
    let parts = rest.split('/').collect::<Vec<_>>();
    match parts.as_slice() {
        [slug, "index.html"] if !PROVIDERS.contains(slug) => Some(slug),
        _ => None,
    }
}

fn sitemap_provider(path: &str) -> Option<&str> {
    let provider = path.strip_prefix("/pkg/sitemap-")?.strip_suffix(".xml")?;
    PROVIDERS.contains(&provider).then_some(provider)
}

fn dynamic_stored_response(
    connection: &Connection,
    path: &str,
    content_type: &str,
    body: String,
) -> Result<StoredResponse, String> {
    let source_hash = metadata_string(connection, "source_hash")?.unwrap_or_default();
    let last_modified = metadata_string(connection, "last_modified")?
        .unwrap_or_else(|| "Wed, 03 Jun 2026 00:00:00 GMT".to_string());
    Ok(StoredResponse {
        content_type: content_type.to_string(),
        body: body.into_bytes(),
        etag: format!("\"{}:{}\"", source_hash, path),
        last_modified,
        cache_control: HTML_CACHE_CONTROL.to_string(),
    })
}

fn metadata_string(connection: &Connection, key: &str) -> Result<Option<String>, String> {
    let value = connection
        .query_row(
            "SELECT value FROM metadata WHERE key = ?1",
            params![key],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|err| format!("failed to read metadata {key}: {err}"))?;
    Ok(value.map(|value| serde_json::from_str::<String>(&value).unwrap_or(value)))
}

fn package_by_provider_slug(
    connection: &Connection,
    provider: &str,
    slug: &str,
) -> Result<Option<PackageRow>, String> {
    connection
        .query_row(
            "SELECT path, provider, slug, package_key, name, display_name, summary,
                    provider_label, package_manager_url, install_command, native_install_command,
                    version, category, license, homepage, repository, rank, last_updated_at,
                    indexable, data_json
             FROM packages
             WHERE provider = ?1 AND slug = ?2",
            params![provider, slug],
            package_from_row,
        )
        .optional()
        .map_err(|err| format!("failed to query package {provider}/{slug}: {err}"))
}

fn package_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<PackageRow> {
    let data_json: String = row.get(19)?;
    let data = serde_json::from_str(&data_json).unwrap_or_default();
    Ok(PackageRow {
        path: row.get(0)?,
        provider: row.get(1)?,
        slug: row.get(2)?,
        package_key: row.get(3)?,
        name: row.get(4)?,
        display_name: row.get(5)?,
        summary: row.get(6)?,
        provider_label: row.get(7)?,
        package_manager_url: row.get(8)?,
        install_command: row.get(9)?,
        native_install_command: row.get(10)?,
        version: row.get(11)?,
        category: row.get(12)?,
        license: row.get(13)?,
        homepage: row.get(14)?,
        repository: row.get(15)?,
        rank: row.get(16)?,
        last_updated_at: row.get(17)?,
        indexable: row.get::<_, i64>(18)? != 0,
        data,
    })
}

fn hub_by_slug(connection: &Connection, slug: &str) -> Result<Option<HubRow>, String> {
    connection
        .query_row(
            "SELECT path, slug, title, description, group_name FROM hubs WHERE slug = ?1",
            params![slug],
            |row| {
                Ok(HubRow {
                    path: row.get(0)?,
                    slug: row.get(1)?,
                    title: row.get(2)?,
                    description: row.get(3)?,
                    group: row.get(4)?,
                })
            },
        )
        .optional()
        .map_err(|err| format!("failed to query hub {slug}: {err}"))
}

fn top_packages(connection: &Connection, limit: usize) -> Result<Vec<PackageRow>, String> {
    let mut statement = connection
        .prepare(
            "SELECT path, provider, slug, package_key, name, display_name, summary,
                    provider_label, package_manager_url, install_command, native_install_command,
                    version, category, license, homepage, repository, rank, last_updated_at,
                    indexable, data_json
             FROM packages
             ORDER BY rank IS NULL, rank, display_name
             LIMIT ?1",
        )
        .map_err(|err| format!("failed to prepare top packages query: {err}"))?;
    collect_packages(statement.query_map(params![limit as i64], package_from_row))
}

fn packages_for_hub(
    connection: &Connection,
    slug: &str,
) -> Result<Vec<(PackageRow, String)>, String> {
    let mut statement = connection
        .prepare(
            "SELECT p.path, p.provider, p.slug, p.package_key, p.name, p.display_name, p.summary,
                    p.provider_label, p.package_manager_url, p.install_command, p.native_install_command,
                    p.version, p.category, p.license, p.homepage, p.repository, p.rank,
                    p.last_updated_at, p.indexable, p.data_json, hp.reason
             FROM hub_packages hp
             JOIN packages p ON p.package_key = hp.package_key
             WHERE hp.hub_slug = ?1
             ORDER BY hp.position",
        )
        .map_err(|err| format!("failed to prepare hub package query: {err}"))?;
    let rows = statement
        .query_map(params![slug], |row| {
            Ok((package_from_row(row)?, row.get::<_, String>(20)?))
        })
        .map_err(|err| format!("failed to query hub packages: {err}"))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|err| format!("failed to read hub packages: {err}"))
}

fn hubs(connection: &Connection) -> Result<Vec<HubRow>, String> {
    let mut statement = connection
        .prepare("SELECT path, slug, title, description, group_name FROM hubs ORDER BY group_name, title")
        .map_err(|err| format!("failed to prepare hubs query: {err}"))?;
    let rows = statement
        .query_map([], |row| {
            Ok(HubRow {
                path: row.get(0)?,
                slug: row.get(1)?,
                title: row.get(2)?,
                description: row.get(3)?,
                group: row.get(4)?,
            })
        })
        .map_err(|err| format!("failed to query hubs: {err}"))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|err| format!("failed to read hubs: {err}"))
}

fn collect_packages<F>(
    rows: rusqlite::Result<rusqlite::MappedRows<'_, F>>,
) -> Result<Vec<PackageRow>, String>
where
    F: FnMut(&rusqlite::Row<'_>) -> rusqlite::Result<PackageRow>,
{
    rows.map_err(|err| format!("failed to query packages: {err}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|err| format!("failed to read packages: {err}"))
}

fn render_index_page(connection: &Connection, locale: &Locale) -> Result<String, String> {
    let hubs = hubs(connection)?;
    let packages = top_packages(connection, 240)?;
    let search_endpoint = locale_path("/pkg/search.json", locale);
    let mut body = String::new();
    body.push_str(r#"<section class="hero"><p class="eyebrow">Package security catalog</p><h1>Package security catalog</h1><p>Curated Automic Vault package pages with install, executable, and security signals.</p></section>"#);
    body.push_str(&format!(
        r#"<section class="search-card"><h2>Find package coverage</h2><div id="pkg-search" class="pkg-search" data-av-package-search data-locale="{}" data-search-endpoint="{}" data-placeholder="Search awscli, gh, .env, npm publish"></div></section><script src="{}"></script>"#,
        html_escape(locale.code),
        html_escape(&search_endpoint),
        html_escape(&locale_path("/pkg/search.js", locale))
    ));
    body.push_str(
        r#"<section><h2>Package groups with security signals</h2><div class="hub-groups">"#,
    );
    for hub in hubs {
        body.push_str(&format!(
            r#"<a class="hub-card" href="{}"><span>{}</span><small>{}</small></a>"#,
            html_escape(&locale_path(&hub.path, locale)),
            html_escape(&hub.title),
            html_escape(&hub.description)
        ));
    }
    body.push_str("</div></section><section><h2>Popular packages</h2><div class=\"package-list\">");
    for package in packages {
        body.push_str(&package_row(&package, locale, None));
    }
    body.push_str("</div></section>");
    Ok(layout("Package security catalog", "/pkg/", locale, &body))
}

fn render_hub_page(
    connection: &Connection,
    hub: &HubRow,
    locale: &Locale,
) -> Result<String, String> {
    let packages = packages_for_hub(connection, &hub.slug)?;
    let mut body = String::new();
    body.push_str(&format!(
        r#"<nav class="breadcrumbs"><a href="{}">Packages</a></nav><section class="hero"><p class="eyebrow">{}</p><h1>{}</h1><p>{}</p></section><section><h2>Packages</h2><div class="package-list">"#,
        html_escape(&locale_path("/pkg/", locale)),
        html_escape(&hub.group),
        html_escape(&hub.title),
        html_escape(&hub.description)
    ));
    for (package, reason) in packages {
        body.push_str(&package_row(&package, locale, Some(&reason)));
    }
    body.push_str("</div></section>");
    Ok(layout(&hub.title, &hub.path, locale, &body))
}

fn render_package_page(package: &PackageRow, locale: &Locale) -> String {
    let title = format!("{} package security", package.display_name);
    let mut body = String::new();
    body.push_str(&format!(
        r#"<nav class="breadcrumbs"><a href="{}">Packages</a></nav><section class="hero"><p class="eyebrow">{} package</p><h1>{}</h1><p>{}</p></section>"#,
        html_escape(&locale_path("/pkg/", locale)),
        html_escape(&package.provider_label),
        html_escape(&package.display_name),
        html_escape(&package.summary)
    ));
    body.push_str("<section><h2>Install</h2>");
    body.push_str(&code_block(&package.install_command));
    if !package.native_install_command.is_empty()
        && package.native_install_command != package.install_command
    {
        body.push_str("<h3>Package manager install</h3>");
        body.push_str(&code_block(&package.native_install_command));
    }
    body.push_str("</section><section><h2>Package facts</h2><dl class=\"facts\">");
    fact(&mut body, "Name", &package.name);
    fact(&mut body, "Package key", &package.package_key);
    fact(&mut body, "Provider", &package.provider);
    fact(&mut body, "Slug", &package.slug);
    fact(&mut body, "Manager", &package.provider_label);
    fact(&mut body, "Version", &package.version);
    fact(&mut body, "Category", &package.category);
    fact(&mut body, "License", &package.license);
    if let Some(rank) = package.rank {
        fact(&mut body, "Popularity rank", &rank.to_string());
    }
    fact_link(&mut body, "Homepage", &package.homepage);
    fact_link(&mut body, "Repository", &package.repository);
    fact_link(
        &mut body,
        "Package manager page",
        &package.package_manager_url,
    );
    body.push_str("</dl></section>");
    list_section(&mut body, "Executables", &package.data.executables);
    list_section(&mut body, "Aliases", &package.data.aliases);
    list_section(&mut body, "Binaries", &package.data.binaries);
    list_section(&mut body, "Security signals", &package.data.security);
    list_section(&mut body, "Classifiers", &package.data.classifiers);
    if !package.data.hubs.is_empty() {
        body.push_str("<section><h2>Package groups</h2><ul>");
        for hub in &package.data.hubs {
            let label = if hub.label.is_empty() {
                &hub.slug
            } else {
                &hub.label
            };
            body.push_str(&format!(
                r#"<li><a href="{}">{}</a>{}</li>"#,
                html_escape(&locale_path(&format!("/pkg/{}/", hub.slug), locale)),
                html_escape(label),
                if hub.reason.is_empty() {
                    String::new()
                } else {
                    format!(": {}", html_escape(&hub.reason))
                }
            ));
        }
        body.push_str("</ul></section>");
    }
    list_section(&mut body, "Related packages", &package.data.related);
    list_section(&mut body, "Keywords", &package.data.keywords);
    layout(&title, &package.path, locale, &body)
}

fn render_package_markdown(package: &PackageRow, _locale: &Locale) -> String {
    let mut text = format!(
        "# {}\n\n{}\n\n## Install\n\n```sh\n{}\n```\n\n## Package Facts\n\n- Package key: {}\n- Manager: {}\n- Version: {}\n- Category: {}\n- License: {}\n",
        package.display_name,
        package.summary,
        package.install_command,
        package.package_key,
        package.provider_label,
        empty_as_unknown(&package.version),
        empty_as_unknown(&package.category),
        empty_as_unknown(&package.license)
    );
    markdown_list(&mut text, "Executables", &package.data.executables);
    markdown_list(&mut text, "Aliases", &package.data.aliases);
    markdown_list(&mut text, "Security Signals", &package.data.security);
    text
}

fn render_sitemap_index(connection: &Connection) -> Result<String, String> {
    let mut providers = connection
        .prepare("SELECT DISTINCT provider FROM packages WHERE indexable = 1 ORDER BY provider")
        .map_err(|err| format!("failed to prepare sitemap provider query: {err}"))?;
    let provider_rows = providers
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|err| format!("failed to query sitemap providers: {err}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|err| format!("failed to read sitemap providers: {err}"))?;
    let lastmod = sitemap_lastmod(connection)?;
    let mut xml = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<sitemapindex xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n",
    );
    sitemap_entry(&mut xml, "/pkg/sitemap-hubs.xml", &lastmod);
    for provider in provider_rows {
        sitemap_entry(&mut xml, &format!("/pkg/sitemap-{provider}.xml"), &lastmod);
    }
    xml.push_str("</sitemapindex>\n");
    Ok(xml)
}

fn render_hub_sitemap(connection: &Connection) -> Result<String, String> {
    let lastmod = sitemap_lastmod(connection)?;
    let mut urls = vec![sitemap_url("/pkg/", &lastmod)];
    for hub in hubs(connection)? {
        urls.push(sitemap_url(&hub.path, &lastmod));
    }
    Ok(render_urlset(&urls))
}

fn render_provider_sitemap(connection: &Connection, provider: &str) -> Result<String, String> {
    let mut statement = connection
        .prepare(
            "SELECT path, provider, slug, package_key, name, display_name, summary,
                    provider_label, package_manager_url, install_command, native_install_command,
                    version, category, license, homepage, repository, rank, last_updated_at,
                    indexable, data_json
             FROM packages
             WHERE provider = ?1 AND indexable = 1
             ORDER BY slug",
        )
        .map_err(|err| format!("failed to prepare provider sitemap query: {err}"))?;
    let packages = collect_packages(statement.query_map(params![provider], package_from_row))?;
    let fallback_lastmod = sitemap_lastmod(connection)?;
    let urls = packages
        .iter()
        .map(|package| {
            sitemap_url(
                &package.path,
                non_empty(&package.last_updated_at, &fallback_lastmod),
            )
        })
        .collect::<Vec<_>>();
    Ok(render_urlset(&urls))
}

fn sitemap_lastmod(connection: &Connection) -> Result<String, String> {
    Ok(metadata_string(connection, "generated_at")?
        .and_then(|value| value.split('T').next().map(str::to_string))
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "2026-06-03".to_string()))
}

fn sitemap_entry(xml: &mut String, path: &str, lastmod: &str) {
    xml.push_str(&format!(
        "  <sitemap>\n    <loc>{}{}</loc>\n    <lastmod>{}</lastmod>\n  </sitemap>\n",
        SITE_ORIGIN,
        html_escape(path),
        html_escape(lastmod)
    ));
}

fn sitemap_url(path: &str, lastmod: &str) -> String {
    let mut lines = vec![
        "  <url>".to_string(),
        format!("    <loc>{}{}</loc>", SITE_ORIGIN, html_escape(path)),
        format!("    <lastmod>{}</lastmod>", html_escape(lastmod)),
    ];
    for locale in LOCALES {
        lines.push(format!(
            "    <xhtml:link rel=\"alternate\" hreflang=\"{}\" href=\"{}{}\" />",
            html_escape(locale.hreflang),
            SITE_ORIGIN,
            html_escape(&locale_path(path, locale))
        ));
    }
    lines.push(format!(
        "    <xhtml:link rel=\"alternate\" hreflang=\"x-default\" href=\"{}{}\" />",
        SITE_ORIGIN,
        html_escape(path)
    ));
    lines.push("  </url>".to_string());
    lines.join("\n")
}

fn render_urlset(urls: &[String]) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\" xmlns:xhtml=\"http://www.w3.org/1999/xhtml\">\n{}\n</urlset>\n",
        urls.join("\n")
    )
}

fn layout(title: &str, canonical_path: &str, locale: &Locale, body: &str) -> String {
    let canonical = format!("{SITE_ORIGIN}{}", locale_path(canonical_path, locale));
    let home_path = if locale.prefix.is_empty() {
        "/".to_string()
    } else {
        format!("{}/", locale.prefix)
    };
    format!(
        "<!doctype html><html lang=\"{}\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width, initial-scale=1\"><title>{} | Automic Vault</title><link rel=\"canonical\" href=\"{}\"><link rel=\"stylesheet\" href=\"{}\"></head><body><header class=\"masthead\"><a class=\"brand\" href=\"{}\">Automic Vault</a><nav><a href=\"{}\">Packages</a></nav></header><main>{}</main></body></html>\n",
        html_escape(locale.code),
        html_escape(title),
        html_escape(&canonical),
        html_escape(&locale_path("/pkg/styles.css", locale)),
        html_escape(&home_path),
        html_escape(&locale_path("/pkg/", locale)),
        body
    )
}

fn package_row(package: &PackageRow, locale: &Locale, detail: Option<&str>) -> String {
    let detail = detail.unwrap_or(&package.summary);
    format!(
        r#"<a class="package-row" href="{}"><span>{}</span><small>{} / {}</small></a>"#,
        html_escape(&locale_path(&package.path, locale)),
        html_escape(&package.display_name),
        html_escape(&package.provider_label),
        html_escape(detail)
    )
}

fn locale_path(path: &str, locale: &Locale) -> String {
    if locale.prefix.is_empty() {
        path.to_string()
    } else {
        format!("{}{}", locale.prefix, path)
    }
}

fn html_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn code_block(value: &str) -> String {
    format!("<pre><code>{}</code></pre>", html_escape(value))
}

fn fact(body: &mut String, label: &str, value: &str) {
    if !value.is_empty() {
        body.push_str(&format!(
            "<dt>{}</dt><dd>{}</dd>",
            html_escape(label),
            html_escape(value)
        ));
    }
}

fn fact_link(body: &mut String, label: &str, value: &str) {
    if !value.is_empty() {
        body.push_str(&format!(
            r#"<dt>{}</dt><dd><a href="{}">{}</a></dd>"#,
            html_escape(label),
            html_escape(value),
            html_escape(value)
        ));
    }
}

fn list_section(body: &mut String, title: &str, items: &[String]) {
    if items.is_empty() {
        return;
    }
    body.push_str(&format!("<section><h2>{}</h2><ul>", html_escape(title)));
    for item in items.iter().take(80) {
        body.push_str(&format!("<li>{}</li>", html_escape(item)));
    }
    body.push_str("</ul></section>");
}

fn markdown_list(text: &mut String, title: &str, items: &[String]) {
    if items.is_empty() {
        return;
    }
    text.push_str(&format!("\n## {title}\n\n"));
    for item in items {
        text.push_str(&format!("- {item}\n"));
    }
}

fn empty_as_unknown(value: &str) -> &str {
    if value.is_empty() { "unknown" } else { value }
}

fn non_empty<'a>(value: &'a str, fallback: &'a str) -> &'a str {
    if value.is_empty() { fallback } else { value }
}

fn parse_query(query: &str) -> BTreeMap<String, String> {
    let mut values = BTreeMap::new();
    for pair in query.split('&') {
        if pair.is_empty() {
            continue;
        }
        let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
        let key = decode_query_component(key);
        if key.is_empty() {
            continue;
        }
        values.insert(key, decode_query_component(value));
    }
    values
}

fn decode_query_component(value: &str) -> String {
    let value = value.replace('+', " ");
    urlencoding::decode(&value)
        .map(|value| value.into_owned())
        .unwrap_or(value)
}

fn locale_for_search(path: &str, query: &BTreeMap<String, String>) -> String {
    if let Some(locale) = query.get("locale").filter(|value| !value.is_empty()) {
        return locale.to_string();
    }
    match path {
        "/de/pkg/search.json" => "de",
        "/fr/pkg/search.json" => "fr",
        "/ja/pkg/search.json" => "ja",
        "/zh-hans/pkg/search.json" => "zh-Hans",
        _ => "en",
    }
    .to_string()
}

fn search_response_json(
    db_path: &Path,
    path: &str,
    query: &BTreeMap<String, String>,
) -> Result<Vec<u8>, String> {
    let search_query = query.get("q").map(String::as_str).unwrap_or("").trim();
    let offset = query
        .get("offset")
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(0);
    let limit = query
        .get("limit")
        .and_then(|value| value.parse::<usize>().ok())
        .map(|value| value.min(MAX_SEARCH_LIMIT))
        .unwrap_or(DEFAULT_SEARCH_LIMIT);
    let locale = locale_for_search(path, query);
    let page = search_documents(db_path, search_query, &locale, offset, limit)?;
    serde_json::to_vec(&page).map_err(|err| format!("failed to encode search response: {err}"))
}

#[derive(Debug, Serialize, PartialEq, Eq)]
struct SearchPage {
    query: String,
    locale: String,
    results: Vec<SearchResult>,
    #[serde(rename = "totalCount")]
    total_count: usize,
    #[serde(rename = "nextOffset", skip_serializing_if = "Option::is_none")]
    next_offset: Option<usize>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
struct SearchResult {
    title: String,
    url: String,
    summary: String,
    provider: String,
    #[serde(rename = "packageKey")]
    package_key: String,
    rank: Option<u32>,
    #[serde(skip)]
    search_text: String,
}

fn search_documents(
    db_path: &Path,
    query: &str,
    locale: &str,
    offset: usize,
    limit: usize,
) -> Result<SearchPage, String> {
    let normalized_query = query.trim().to_ascii_lowercase();
    if normalized_query.is_empty() {
        return Ok(SearchPage {
            query: query.to_string(),
            locale: locale.to_string(),
            results: Vec::new(),
            total_count: 0,
            next_offset: None,
        });
    }

    let like_pattern = format!("%{}%", escape_like(&normalized_query));
    let connection = open_database(db_path)?;
    let mut statement = connection
        .prepare(
            "SELECT path, title, summary, provider, package_key, rank, search_text
             FROM search_documents
             WHERE locale = ?1 AND lower(search_text) LIKE ?2 ESCAPE '\\'",
        )
        .map_err(|err| format!("failed to prepare search: {err}"))?;
    let rows = statement
        .query_map(params![locale, like_pattern], |row| {
            Ok(SearchResult {
                url: row.get(0)?,
                title: row.get(1)?,
                summary: row.get(2)?,
                provider: row.get(3)?,
                package_key: row.get(4)?,
                rank: row.get(5)?,
                search_text: row.get(6)?,
            })
        })
        .map_err(|err| format!("failed to search packages: {err}"))?;
    let mut results = rows
        .collect::<Result<Vec<_>, _>>()
        .map_err(|err| format!("failed to read search rows: {err}"))?;
    results.retain(|result| result.search_rank(&normalized_query).is_some());
    results.sort_by(|left, right| {
        left.search_sort_key(&normalized_query)
            .cmp(&right.search_sort_key(&normalized_query))
    });
    let total_count = results.len();
    let next_offset_value = offset.saturating_add(limit);
    let next_offset = (next_offset_value < total_count).then_some(next_offset_value);
    let results = results.into_iter().skip(offset).take(limit).collect();
    Ok(SearchPage {
        query: query.to_string(),
        locale: locale.to_string(),
        results,
        total_count,
        next_offset,
    })
}

fn escape_like(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

impl SearchResult {
    fn search_sort_key(&self, query: &str) -> (u8, usize, u32, String) {
        (
            self.search_rank(query).unwrap_or(u8::MAX),
            self.match_distance(query),
            self.rank.unwrap_or(u32::MAX),
            self.title.to_ascii_lowercase(),
        )
    }

    fn search_rank(&self, query: &str) -> Option<u8> {
        let title = self.title.to_ascii_lowercase();
        let key = self.package_key.to_ascii_lowercase();
        let url_name = self
            .url
            .trim_end_matches('/')
            .rsplit('/')
            .next()
            .unwrap_or("")
            .to_ascii_lowercase();
        let candidates = [title.as_str(), key.as_str(), url_name.as_str()];
        if candidates.iter().any(|candidate| *candidate == query) {
            return Some(0);
        }
        if candidates
            .iter()
            .any(|candidate| candidate.starts_with(query))
        {
            return Some(1);
        }
        if candidates.iter().any(|candidate| candidate.contains(query)) {
            return Some(2);
        }
        self.search_text
            .to_ascii_lowercase()
            .contains(query)
            .then_some(3)
    }

    fn match_distance(&self, query: &str) -> usize {
        [
            self.title.to_ascii_lowercase(),
            self.package_key.to_ascii_lowercase(),
            self.url
                .trim_end_matches('/')
                .rsplit('/')
                .next()
                .unwrap_or("")
                .to_ascii_lowercase(),
            self.search_text.to_ascii_lowercase(),
        ]
        .into_iter()
        .filter_map(|candidate| {
            if candidate == query {
                Some(0)
            } else if candidate.starts_with(query) {
                Some(candidate.len().saturating_sub(query.len()))
            } else {
                candidate
                    .find(query)
                    .map(|index| candidate.len().saturating_sub(query.len()) + index)
            }
        })
        .min()
        .unwrap_or(usize::MAX)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::params;
    use tempfile::NamedTempFile;

    fn test_database() -> NamedTempFile {
        let file = NamedTempFile::new().expect("temp database");
        let connection = Connection::open(file.path()).expect("open temp database");
        connection
            .execute_batch(
                "
                CREATE TABLE metadata(key TEXT PRIMARY KEY, value TEXT NOT NULL);
                INSERT INTO metadata(key, value) VALUES('schema', '1');
                CREATE TABLE responses(
                    path TEXT PRIMARY KEY,
                    content_type TEXT NOT NULL,
                    body BLOB NOT NULL,
                    etag TEXT NOT NULL,
                    last_modified TEXT NOT NULL,
                    cache_control TEXT NOT NULL
                );
                CREATE TABLE search_documents(
                    path TEXT PRIMARY KEY,
                    locale TEXT NOT NULL,
                    title TEXT NOT NULL,
                    summary TEXT NOT NULL,
                    provider TEXT NOT NULL,
                    package_key TEXT NOT NULL,
                    rank INTEGER,
                    search_text TEXT NOT NULL
                );
                CREATE TABLE packages(
                    path TEXT PRIMARY KEY,
                    provider TEXT NOT NULL,
                    slug TEXT NOT NULL,
                    package_key TEXT NOT NULL UNIQUE,
                    name TEXT NOT NULL,
                    display_name TEXT NOT NULL,
                    summary TEXT NOT NULL,
                    provider_label TEXT NOT NULL,
                    package_manager_url TEXT NOT NULL,
                    install_command TEXT NOT NULL,
                    native_install_command TEXT NOT NULL,
                    version TEXT NOT NULL,
                    category TEXT NOT NULL,
                    license TEXT NOT NULL,
                    homepage TEXT NOT NULL,
                    repository TEXT NOT NULL,
                    rank INTEGER,
                    last_updated_at TEXT NOT NULL,
                    indexable INTEGER NOT NULL,
                    data_json TEXT NOT NULL,
                    search_text TEXT NOT NULL
                );
                CREATE TABLE hubs(
                    path TEXT PRIMARY KEY,
                    slug TEXT NOT NULL UNIQUE,
                    title TEXT NOT NULL,
                    description TEXT NOT NULL,
                    group_name TEXT NOT NULL,
                    data_json TEXT NOT NULL
                );
                CREATE TABLE hub_packages(
                    hub_slug TEXT NOT NULL,
                    package_key TEXT NOT NULL,
                    position INTEGER NOT NULL,
                    reason TEXT NOT NULL,
                    PRIMARY KEY(hub_slug, package_key)
                );
                ",
            )
            .expect("create schema");
        connection
            .execute(
                "INSERT INTO responses(path, content_type, body, etag, last_modified, cache_control)
                 VALUES(?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    "/pkg/brew/awscli/index.html",
                    "text/html; charset=utf-8",
                    b"<html>awscli</html>".to_vec(),
                    "\"abc\"",
                    "Tue, 02 Jun 2026 19:54:51 GMT",
                    HTML_CACHE_CONTROL
                ],
            )
            .expect("insert html");
        connection
            .execute(
                "INSERT INTO responses(path, content_type, body, etag, last_modified, cache_control)
                 VALUES(?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    "/pkg/brew/awscli/index.md",
                    "text/markdown; charset=utf-8",
                    b"# awscli\n".to_vec(),
                    "\"def\"",
                    "Tue, 02 Jun 2026 19:54:51 GMT",
                    HTML_CACHE_CONTROL
                ],
            )
            .expect("insert markdown");
        connection
            .execute(
                "INSERT INTO packages(path, provider, slug, package_key, name, display_name, summary,
                 provider_label, package_manager_url, install_command, native_install_command,
                 version, category, license, homepage, repository, rank, last_updated_at, indexable,
                 data_json, search_text)
                 VALUES(?1, 'brew', 'awscli', 'brew:awscli', 'awscli', 'awscli',
                 'AWS command line interface.', 'Homebrew', 'https://brew.example/awscli',
                 'av install awscli', 'brew install awscli', '2.0.0', 'developer-tools',
                 'Apache-2.0', 'https://aws.amazon.com/cli/', '', 2, '2026-06-02', 1,
                 ?2, 'awscli brew:awscli aws cloud cli')",
                params![
                    "/pkg/brew/awscli/",
                    r#"{"aliases":["aws"],"executables":["aws"],"security":["approval gate"],"keywords":["cloud"]}"#
                ],
            )
            .expect("insert package");
        connection
            .execute(
                "INSERT INTO hubs(path, slug, title, description, group_name, data_json)
                 VALUES('/pkg/cloud/', 'cloud', 'Cloud', 'Cloud tools', 'topical', '{}')",
                [],
            )
            .expect("insert hub");
        connection
            .execute(
                "INSERT INTO hub_packages(hub_slug, package_key, position, reason)
                 VALUES('cloud', 'brew:awscli', 1, 'Cloud CLI')",
                [],
            )
            .expect("insert hub package");
        for (path, title, summary, provider, key, rank, text) in [
            (
                "/pkg/brew/awscli/",
                "awscli",
                "AWS command line interface.",
                "brew",
                "brew:awscli",
                Some(2_u32),
                "awscli brew:awscli aws command line interface cloud",
            ),
            (
                "/pkg/brew/aws-cdk/",
                "aws-cdk",
                "AWS CDK.",
                "brew",
                "brew:aws-cdk",
                Some(50_u32),
                "aws-cdk brew:aws-cdk aws infrastructure",
            ),
            (
                "/pkg/brew/cloudflared/",
                "cloudflared",
                "Tunnel client for AWS adjacent workflows.",
                "brew",
                "brew:cloudflared",
                Some(1_u32),
                "cloudflared tunnel client aws adjacent workflows",
            ),
        ] {
            connection
                .execute(
                    "INSERT INTO search_documents(path, locale, title, summary, provider, package_key, rank, search_text)
                     VALUES(?1, 'en', ?2, ?3, ?4, ?5, ?6, ?7)",
                    params![path, title, summary, provider, key, rank, text],
                )
                .expect("insert search row");
        }
        drop(connection);
        file
    }

    #[test]
    fn route_lookup_handles_trailing_slash_and_index_html() {
        let db = test_database();
        let slash = response_for_path(db.path(), "/pkg/brew/awscli/")
            .expect("query")
            .expect("slash response");
        let index = response_for_path(db.path(), "/pkg/brew/awscli/index.html")
            .expect("query")
            .expect("index response");

        assert_eq!(slash.body, index.body);
        assert_eq!(slash.content_type, "text/html; charset=utf-8");
    }

    #[test]
    fn markdown_route_returns_markdown_content_type() {
        let db = test_database();
        let response = dynamic_response_for_path(db.path(), "/pkg/brew/awscli/index.md")
            .expect("query")
            .expect("markdown response");

        assert_eq!(response.content_type, "text/markdown; charset=utf-8");
        assert!(
            String::from_utf8(response.body)
                .expect("utf-8 markdown")
                .contains("## Install")
        );
    }

    #[test]
    fn dynamic_routes_render_package_hub_and_sitemap() {
        let db = test_database();
        let package = dynamic_response_for_path(db.path(), "/pkg/brew/awscli/")
            .expect("query")
            .expect("package response");
        let hub = dynamic_response_for_path(db.path(), "/pkg/cloud/")
            .expect("query")
            .expect("hub response");
        let sitemap = dynamic_response_for_path(db.path(), "/pkg/sitemap-brew.xml")
            .expect("query")
            .expect("sitemap response");

        assert!(
            String::from_utf8(package.body)
                .expect("package html")
                .contains("AWS command line interface")
        );
        assert!(
            String::from_utf8(hub.body)
                .expect("hub html")
                .contains("Cloud CLI")
        );
        assert!(
            String::from_utf8(sitemap.body)
                .expect("sitemap xml")
                .contains("hreflang=\"zh-Hans\"")
        );
    }

    #[test]
    fn missing_route_returns_none() {
        let db = test_database();

        assert!(
            response_for_path(db.path(), "/pkg/brew/nope/")
                .expect("query")
                .is_none()
        );
    }

    #[test]
    fn search_prefers_exact_and_prefix_before_summary_matches() {
        let db = test_database();
        let results = search_documents(db.path(), "aws", "en", 0, 10).expect("search");

        assert_eq!(
            results
                .results
                .iter()
                .map(|result| result.title.as_str())
                .collect::<Vec<_>>(),
            vec!["awscli", "aws-cdk", "cloudflared"]
        );
    }

    #[test]
    fn origin_token_rejects_missing_and_invalid_headers() {
        let headers = BTreeMap::new();
        assert!(!origin_request_authorized(
            "/pkg/",
            &headers,
            "x-test-origin",
            Some("secret")
        ));
        assert!(origin_request_authorized(
            "/healthz",
            &headers,
            "x-test-origin",
            Some("secret")
        ));

        let mut headers = BTreeMap::new();
        headers.insert("x-test-origin".to_string(), "wrong".to_string());
        assert!(!origin_request_authorized(
            "/pkg/",
            &headers,
            "x-test-origin",
            Some("secret")
        ));
        headers.insert("x-test-origin".to_string(), "secret".to_string());
        assert!(origin_request_authorized(
            "/pkg/",
            &headers,
            "x-test-origin",
            Some("secret")
        ));
    }
}
