use rusqlite::{Connection, OpenFlags, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::env;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};
use std::thread;

const DEFAULT_BIND_ADDR: &str = "127.0.0.1:3004";
const DEFAULT_DB_PATH: &str = "/var/lib/automic-vault-web/pkg.sqlite";
const DEFAULT_ORIGIN_HEADER: &str = "x-automic-vault-origin";
const HTML_CACHE_CONTROL: &str = "public, max-age=86400, s-maxage=86400";
const DEFAULT_SEARCH_LIMIT: usize = 8;
const MAX_SEARCH_LIMIT: usize = 50;
const I18N_PKG_TEMPLATES_JSON: &str = include_str!("../../data/www-i18n/pkg/templates.json");
static I18N_PKG_TEMPLATES: OnceLock<Value> = OnceLock::new();

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
    full: Value,
}

#[derive(Debug, Clone)]
struct PackageRow {
    path: String,
    provider: String,
    package_key: String,
    name: String,
    display_name: String,
    summary: String,
    provider_label: String,
    package_manager_url: String,
    install_command: String,
    native_install_command: String,
    version: String,
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
            let generated_at = metadata_string(&connection, "generated_at")?.unwrap_or_default();
            Some((
                render_package_markdown(&package, locale, &generated_at),
                "text/markdown; charset=utf-8",
            ))
        } else {
            let generated_at = metadata_string(&connection, "generated_at")?.unwrap_or_default();
            Some((
                render_package_page(&package, locale, &generated_at),
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
        package_key: row.get(3)?,
        name: row.get(4)?,
        display_name: row.get(5)?,
        summary: row.get(6)?,
        provider_label: row.get(7)?,
        package_manager_url: row.get(8)?,
        install_command: row.get(9)?,
        native_install_command: row.get(10)?,
        version: row.get(11)?,
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

fn all_packages(connection: &Connection) -> Result<Vec<PackageRow>, String> {
    let mut statement = connection
        .prepare(
            "SELECT path, provider, slug, package_key, name, display_name, summary,
                    provider_label, package_manager_url, install_command, native_install_command,
                    version, category, license, homepage, repository, rank, last_updated_at,
                    indexable, data_json
             FROM packages
             ORDER BY rank IS NULL, rank, display_name",
        )
        .map_err(|err| format!("failed to prepare all packages query: {err}"))?;
    collect_packages(statement.query_map([], package_from_row))
}

#[derive(Debug, Clone)]
struct HubSummary {
    hub: HubRow,
    package_count: i64,
}

fn hub_summaries(connection: &Connection) -> Result<Vec<HubSummary>, String> {
    let mut statement = connection
        .prepare(
            "SELECT h.path, h.slug, h.title, h.description, h.group_name, COUNT(hp.package_key)
             FROM hubs h
             LEFT JOIN hub_packages hp ON hp.hub_slug = h.slug
             GROUP BY h.path, h.slug, h.title, h.description, h.group_name
             ORDER BY h.group_name, h.title",
        )
        .map_err(|err| format!("failed to prepare hub summaries query: {err}"))?;
    let rows = statement
        .query_map([], |row| {
            Ok(HubSummary {
                hub: HubRow {
                    path: row.get(0)?,
                    slug: row.get(1)?,
                    title: row.get(2)?,
                    description: row.get(3)?,
                    group: row.get(4)?,
                },
                package_count: row.get(5)?,
            })
        })
        .map_err(|err| format!("failed to query hub summaries: {err}"))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|err| format!("failed to read hub summaries: {err}"))
}

fn metadata_json(connection: &Connection, key: &str) -> Result<Option<Value>, String> {
    let Some(value) = metadata_string(connection, key)? else {
        return Ok(None);
    };
    Ok(serde_json::from_str(&value).ok())
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
    let hub_summaries = hub_summaries(connection)?;
    let packages = all_packages(connection)?;
    let top_packages = packages.iter().take(72).collect::<Vec<_>>();
    let secured = packages
        .iter()
        .filter(|package| package_isotope(package))
        .count();
    let radioisotope_count = metadata_json(connection, "manifest")?
        .and_then(|manifest| value_i64_key(&manifest, "radioisotope_manifest_count"))
        .unwrap_or(secured as i64);
    let gated = packages
        .iter()
        .filter(|package| package_gate(package))
        .count();
    let source_files = metadata_json(connection, "manifest")?
        .and_then(|manifest| value_i64_key(&manifest, "source_file_count"))
        .unwrap_or_default();
    let search_endpoint = locale_path("/pkg/search.json", locale);
    let catalog_title = tx(locale, "packageCatalogTitle", "Package security catalog");
    let mut body = String::new();
    body.push_str(&site_nav(locale));
    body.push_str("<main>");
    body.push_str(&format!(
        r#"<section class="pkg-hero pkg-hero-index" aria-labelledby="pkg-title"><div class="hero-copy"><p class="eyebrow">{}</p><h1 id="pkg-title">{}</h1><p class="lede">{}</p></div><aside class="hero-panel" aria-label="{}">{}{}{}{}</aside></section>"#,
        html_escape(&tx(locale, "catalogEyebrow", "Nucleus package intelligence")),
        html_escape(&catalog_title),
        html_escape(&tx(locale, "catalogPagesCopy", "Generated pages for executable packages Nucleus knows about, with local secret-handling manifests, approval-gate metadata, install popularity, executable aliases, and upstream package facts.")),
        html_escape(&tx(locale, "catalogCounts", "Catalog counts")),
        metric(&tx(locale, "packages", "packages"), &fmt_int(packages.len() as i64)),
        metric(&tx(locale, "radioisotopes", "protected tools"), &fmt_int(radioisotope_count)),
        metric(&tx(locale, "approvalGates", "approval gates"), &fmt_int(gated as i64)),
        metric(&tx(locale, "sourceFiles", "source files"), &fmt_int(source_files)),
    ));
    body.push_str(&format!(
        r#"<section class="pkg-section pkg-search-section" aria-labelledby="pkg-search-title"><div class="search-copy"><p class="section-kicker">{}</p><h2 id="pkg-search-title">{}</h2><p>{}</p></div><div id="pkg-search" class="pkg-search" data-av-package-search data-locale="{}" data-search-endpoint="{}" data-placeholder="{}"></div></section>"#,
        html_escape(&tx(locale, "siteSearch", "site search")),
        html_escape(&tx(locale, "findPackageCoverage", "Find package coverage")),
        html_escape(&tx(locale, "catalogSearchCopy", "Search generated package pages, security guides, documentation, and source-backed metadata from one index.")),
        html_escape(locale.code),
        html_escape(&search_endpoint),
        html_escape(&tx(locale, "searchPlaceholder", "Search awscli, gh, .env, npm publish"))
    ));
    body.push_str(&format!(
        r#"<section class="pkg-section" aria-labelledby="pkg-hubs-title"><p class="section-kicker">{}</p><h2 id="pkg-hubs-title">{}</h2><p>{}</p><div class="hub-groups" aria-label="{}">{}</div></section>"#,
        html_escape(&tx(locale, "catalogHubsKicker", "package hubs")),
        html_escape(&tx(locale, "catalogHubsTitle", "Package groups with security signals")),
        html_escape(&tx(locale, "catalogHubsCopy", "These crawlable hubs group package families that matter for agent security: cloud CLIs, source-control tools, package publishers, MCP tools, and packages with local secret-risk signals.")),
        html_escape(&tx(locale, "catalogHubsAria", "Package category hubs")),
        hub_group_sections(&hub_summaries, locale)
    ));
    body.push_str(&format!(
        r#"<section class="pkg-section split-section"><div><p class="section-kicker">{}</p><h2>{}</h2><p>{}</p></div><div class="package-list" aria-label="{}">"#,
        html_escape(&tx(locale, "catalogPagesKicker", "crawlable catalog")),
        html_escape(&tx(locale, "catalogPagesTitle", "Package pages from local source data")),
        html_escape(&tx(locale, "crawlableCatalog", "Nucleus package metadata, generated package inventories, secret-handling READMEs, migration manifests, and approval-gate seeds are written to static HTML so search and answer engines can find specific tool coverage.")),
        html_escape(&tx(locale, "popularPackages", "Popular packages")),
    ));
    for package in top_packages {
        body.push_str(&index_package_row(package, locale));
    }
    body.push_str("</div></section></main>");
    body.push_str(&site_footer(locale));
    let schema = json!({
        "@context": "https://schema.org",
        "@type": "CollectionPage",
        "name": "Automic Vault package security catalog",
        "url": locale_url("/pkg/", locale),
        "inLanguage": locale.hreflang,
        "isPartOf": {"@type": "WebSite", "name": "Automic Vault", "url": format!("{SITE_ORIGIN}/")},
        "about": tx(locale, "packageCatalogDescription", "Nucleus packages, AI agent package security, approval gates, and secret migration metadata")
    });
    let schema_json = serde_json::to_string_pretty(&schema).unwrap_or_else(|_| "{}".to_string());
    Ok(html_doc(
        locale,
        &format!("{catalog_title} | Automic Vault"),
        &tx(
            locale,
            "packageCatalogDescription",
            "Automic Vault package catalog for executable Nucleus packages, protected-tool secret handling, approval gates, install metadata, and agent security notes.",
        ),
        &locale_url("/pkg/", locale),
        "index,follow",
        "",
        &schema_json,
        &body,
        &format!(
            r#"  <script src="{}"></script>"#,
            html_escape(&locale_path("/pkg/search.js", locale))
        ),
    ))
}

fn render_hub_page(
    connection: &Connection,
    hub: &HubRow,
    locale: &Locale,
) -> Result<String, String> {
    let packages = packages_for_hub(connection, &hub.slug)?;
    let generated_at = metadata_string(connection, "generated_at")?.unwrap_or_default();
    let updated = fmt_date(&generated_at);
    let secured = packages
        .iter()
        .filter(|(package, _reason)| package_isotope(package))
        .count();
    let gated = packages
        .iter()
        .filter(|(package, _reason)| package_gate(package))
        .count();
    let risked = packages
        .iter()
        .filter(|(package, _reason)| package_non_low_geiger(package))
        .count();
    let top = packages.iter().take(72).collect::<Vec<_>>();
    let high_signal = packages
        .iter()
        .take(12)
        .map(|(package, _reason)| package)
        .collect::<Vec<_>>();
    let protected = packages
        .iter()
        .filter(|(package, _reason)| package_isotope(package))
        .take(8)
        .map(|(package, _reason)| package)
        .collect::<Vec<_>>();
    let approval_gated = packages
        .iter()
        .filter(|(package, _reason)| package_gate(package))
        .take(8)
        .map(|(package, _reason)| package)
        .collect::<Vec<_>>();
    let mut spokes = packages
        .iter()
        .map(|(package, _reason)| package)
        .collect::<Vec<_>>();
    spokes.sort_by_key(|package| {
        (
            package.rank.unwrap_or(u32::MAX),
            package.display_name.to_ascii_lowercase(),
        )
    });
    let spokes = spokes.into_iter().take(16).collect::<Vec<_>>();
    let mut body = String::new();
    body.push_str(&site_nav(locale));
    body.push_str("<main>");
    body.push_str(&format!(
        r#"<nav class="breadcrumbs" aria-label="Breadcrumbs"><a href="{}">{}</a><span>/</span><a href="{}">{}</a><span>/</span><span>{}</span></nav><section class="pkg-hero pkg-hero-index" aria-labelledby="hub-title"><div class="hero-copy"><p class="eyebrow">{}</p><h1 id="hub-title">{}</h1><p class="lede">{}</p></div><aside class="hero-panel" aria-label="{}">{}{}{}{}</aside></section>"#,
        html_escape(&locale_path("/", locale)),
        html_escape(&tx(locale, "home", "Home")),
        html_escape(&locale_path("/pkg/", locale)),
        html_escape(&tx(locale, "packages", "Packages")),
        html_escape(&hub.title),
        html_escape(&hub.group),
        html_escape(&hub.title),
        html_escape(&hub.description),
        html_escape(&tx(locale, "hubCounts", "Hub counts")),
        metric(&tx(locale, "packages", "packages"), &fmt_int(packages.len() as i64)),
        metric(&tx(locale, "radioisotopes", "protected tools"), &fmt_int(secured as i64)),
        metric(&tx(locale, "approvalGates", "approval gates"), &fmt_int(gated as i64)),
        metric(&tx(locale, "updated", "updated"), &updated),
    ));
    let hub_description = txf(
        locale,
        "hubDescription",
        "{title} currently includes {count} generated package pages. {secured} have protected-tool coverage, {gated} have approval-gate metadata, and {risked} have non-low Geiger classifier findings. The grouping comes from package metadata, so it can stay current as that metadata changes.",
        &[
            ("title", hub.title.clone()),
            ("count", packages.len().to_string()),
            ("secured", secured.to_string()),
            ("gated", gated.to_string()),
            ("risked", risked.to_string()),
        ],
    );
    body.push_str(&format!(
        r#"<section class="pkg-section split-section"><div><p class="section-kicker">{}</p><h2>{}</h2><p>{}</p></div><div class="detail-stack"><article><h3>{}</h3><p>{}</p></article><article><h3>{}</h3><p>{}</p></article></div></section>"#,
        html_escape(&tx(locale, "packageSummary", "summary")),
        html_escape(&tx(locale, "hubSummaryTitle", "Why this package group is here")),
        html_escape(&hub_description),
        html_escape(&tx(locale, "generatedSource", "Generated source")),
        html_escape(&tx(locale, "generatedSourceCopy", "This hub uses the same local package data as individual package pages: Nucleus package metadata, Homebrew enrichment, Geiger classifier output, secret-handling manifests, and approval-gate seeds where available.")),
        html_escape(&tx(locale, "hubReviewModel", "Review model")),
        html_escape(&tx(locale, "hubReviewCopy", "Use the hub to find command families that need tighter secret injection, approval gates, or manual review before agents run them."))
    ));
    body.push_str(&hub_cluster_block(
        &tx(locale, "hubHighSignalTitle", "High-signal tools"),
        &high_signal,
        locale,
    ));
    body.push_str(&hub_cluster_block(
        &tx(locale, "hubProtectedToolsTitle", "Protected tools"),
        &protected,
        locale,
    ));
    body.push_str(&hub_cluster_block(
        &tx(locale, "hubApprovalGatedTitle", "Approval-gated tools"),
        &approval_gated,
        locale,
    ));
    body.push_str(&hub_related_block(
        &tx(locale, "hubRelatedHubsTitle", "Related hubs"),
        &related_hub_links(hub, &packages, locale),
    ));
    body.push_str(&hub_cluster_block(
        &tx(
            locale,
            "hubRepresentativeSpokesTitle",
            "Representative package spokes",
        ),
        &spokes,
        locale,
    ));
    body.push_str(&format!(
        r#"<section class="pkg-section"><p class="section-kicker">{}</p><h2>{}</h2><div class="table-wrap hub-table"><table><thead><tr><th>{}</th><th>{}</th><th>{}</th><th>{}</th></tr></thead><tbody>"#,
        html_escape(&tx(locale, "packages", "packages")),
        html_escape(&tx(locale, "hubIndexedPagesTitle", "Indexed package pages")),
        html_escape(&tx(locale, "package", "Package")),
        html_escape(&tx(locale, "manager", "Manager")),
        html_escape(&tx(locale, "signals", "Signals")),
        html_escape(&tx(locale, "why", "Why it appears here")),
    ));
    for (package, reason) in top {
        body.push_str(&hub_package_row(package, reason, locale));
    }
    body.push_str("</tbody></table></div></section></main>");
    body.push_str(&site_footer(locale));
    let description = short_text(
        &txf(
            locale,
            "hubSchemaDescription",
            "{description} Browse {count} package pages with install commands, metadata, and Automic Vault security notes.",
            &[
                ("description", hub.description.clone()),
                ("count", packages.len().to_string()),
            ],
        ),
        155,
    );
    let schema = schema_for_hub(
        hub,
        packages.iter().map(|(package, _)| package).collect(),
        &description,
        &updated,
        locale,
    );
    let schema_json = serde_json::to_string_pretty(&schema).unwrap_or_else(|_| "{}".to_string());
    Ok(html_doc(
        locale,
        &format!("{} | Automic Vault package catalog", hub.title),
        &description,
        &locale_url(&hub.path, locale),
        "index,follow",
        "",
        &schema_json,
        &body,
        "",
    ))
}

fn render_package_page(package: &PackageRow, locale: &Locale, generated_at: &str) -> String {
    let install_heading = txf(
        locale,
        "installHeading",
        "Install {name}",
        &[("name", package.display_name.clone())],
    );
    let title = format!("{install_heading} | Automic Vault");
    let description = meta_description(package);
    let updated = first_non_empty(&[
        full_str(package, "lastVerified"),
        package.last_updated_at.clone(),
    ])
    .map(|value| fmt_date(&value))
    .filter(|value| !value.is_empty())
    .unwrap_or_else(|| "2026-06-03".to_string());
    let canonical = locale_url(&package.path, locale);
    let schema = schema_for_package(package, &description, &updated, locale);
    let schema_json = serde_json::to_string_pretty(&schema).unwrap_or_else(|_| "{}".to_string());
    let mut body = String::new();
    body.push_str(&site_nav(locale));
    body.push_str("<main>");
    body.push_str(&format!(
        r#"<nav class="breadcrumbs" aria-label="Breadcrumbs"><a href="{}">{}</a><span>/</span><a href="{}">{}</a><span>/</span><span>{}</span></nav>"#,
        html_escape(&locale_path("/", locale)),
        html_escape(&tx(locale, "home", "Home")),
        html_escape(&locale_path("/pkg/", locale)),
        html_escape(&tx(locale, "packages", "Packages")),
        html_escape(&package.display_name)
    ));
    body.push_str(&format!(
        r##"<section class="pkg-hero" aria-labelledby="pkg-title"><div class="hero-copy"><p class="eyebrow">{}</p><h1 id="pkg-title">{}</h1><p class="lede">{}</p><div class="hero-actions"><a class="button primary" href="#install">{}</a><a class="button secondary" href="#security">{}</a></div></div><aside class="hero-panel" aria-label="{}">{}</aside></section>"##,
        html_escape(&label_for(package, locale)),
        html_escape(&install_heading),
        html_escape(&localized_hero_sentence(package, locale)),
        html_escape(&tx(locale, "installCommand", "Install command")),
        html_escape(&tx(locale, "securityNotes", "Security notes")),
        html_escape(&tx(locale, "heroPanelAria", "Package facts")),
        package_facts(package, locale)
    ));
    body.push_str(&render_install(package, locale));
    body.push_str(&render_overview(package, locale));
    body.push_str(&render_security(package, locale));
    body.push_str(&render_executables(package, locale));
    body.push_str(&render_freshness(package, generated_at, locale));
    body.push_str(&render_install_metadata(package, locale));
    body.push_str(&render_related(package, locale));
    body.push_str(&render_sources(package, locale));
    body.push_str("</main>");
    body.push_str(&site_footer(locale));
    html_doc(
        locale,
        &title,
        &description,
        &canonical,
        if package.indexable {
            "index,follow"
        } else {
            "noindex,follow"
        },
        &format!(
            r#"  <link rel="alternate" type="text/markdown" href="{}index.md">"#,
            html_escape(&canonical)
        ),
        &schema_json,
        &body,
        &copy_script(),
    )
}

fn render_package_markdown(package: &PackageRow, locale: &Locale, generated_at: &str) -> String {
    let mut text = format!(
        "# Install {}\n\n{}\n\n## Install\n\n```sh\n{}\n```\n\n",
        package.display_name,
        hero_sentence(package),
        package.install_command,
    );
    markdown_install_groups(&mut text, package);
    text.push_str("## Package Facts\n\n");
    for (label, value) in [
        ("Package key", package.package_key.clone()),
        ("Package manager", package.provider_label.clone()),
        ("Package manager URL", package.package_manager_url.clone()),
        ("Version", package.version.clone()),
        ("Source summary", package.summary.clone()),
        ("Homepage", package.homepage.clone()),
        ("Repository", package.repository.clone()),
        ("Upstream docs", full_str(package, "upstreamDocs")),
        ("License", package.license.clone()),
        ("Source archive", full_str(package, "sourceArchive")),
        ("Issue tracker", full_str(package, "issueTracker")),
        ("Published", full_str(package, "publishedAt")),
        ("Last verified", full_str(package, "lastVerified")),
        ("Last updated", package.last_updated_at.clone()),
        ("Generated", generated_at.to_string()),
    ] {
        if !value.trim().is_empty() {
            text.push_str(&format!("- **{}:** {}\n", label, markdown_value(&value)));
        }
    }
    markdown_value_list(
        &mut text,
        "Executables",
        &executable_markdown_items(package),
    );
    markdown_value_list(
        &mut text,
        "Dependencies",
        &full_string_array(package, "dependencies"),
    );
    markdown_value_list(
        &mut text,
        "Build Dependencies",
        &full_string_array(package, "buildDependencies"),
    );
    markdown_value_list(
        &mut text,
        "macOS Provided Libraries",
        &full_string_array(package, "usesFromMacos"),
    );
    markdown_value_list(
        &mut text,
        "Install Behavior",
        &markdown_install_behavior_items(package),
    );
    markdown_value_list(
        &mut text,
        "Freshness",
        &markdown_freshness_items(package, generated_at),
    );
    markdown_security_section(&mut text, package, locale);
    markdown_related(&mut text, package, locale);
    markdown_value_list(
        &mut text,
        "Sources",
        &full_string_array(package, "sourceNotes"),
    );
    text
}

fn html_doc(
    locale: &Locale,
    title: &str,
    description: &str,
    canonical: &str,
    robots: &str,
    extra_head: &str,
    schema_json: &str,
    body: &str,
    extra_body: &str,
) -> String {
    format!(
        r#"<!DOCTYPE html>
<html lang="{lang}">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>{title}</title>
  <meta name="description" content="{description}">
  <meta name="robots" content="{robots}">
  <meta property="og:type" content="website">
  <meta property="og:site_name" content="Automic Vault">
  <meta property="og:title" content="{title}">
  <meta property="og:description" content="{description}">
  <meta property="og:url" content="{canonical}">
  <meta property="og:image" content="{origin}/preview.jpg">
  <meta name="twitter:card" content="summary_large_image">
  <meta name="twitter:title" content="{title}">
  <meta name="twitter:description" content="{description}">
  <meta name="twitter:image" content="{origin}/preview.jpg">
  <link rel="canonical" href="{canonical}">
{hreflang}
  <link rel="preconnect" href="https://fonts.googleapis.com">
  <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
  <link href="https://fonts.googleapis.com/css2?family=Geist:wght@400;500;600;700;800&amp;family=Geist+Mono:wght@400;500;600;700&amp;display=swap" rel="stylesheet">
  <link rel="icon" href="/favicon.ico" sizes="16x16 32x32 48x48">
  <link rel="stylesheet" href="{stylesheet}">
  <!-- Google tag (gtag.js) -->
  <script async src="https://www.googletagmanager.com/gtag/js?id=G-Y78QKG1T9Y"></script>
  <script>
    window.dataLayer = window.dataLayer || [];
    function gtag(){{dataLayer.push(arguments);}}
    gtag('js', new Date());
    gtag('config', 'G-Y78QKG1T9Y');
  </script>
{extra_head}
  <script type="application/ld+json">
{schema_json}
  </script>
</head>
<body>
  <div class="site-shell">
    {body}
  </div>
{extra_body}
</body>
</html>
"#,
        lang = html_escape(locale.code),
        title = html_escape(title),
        description = html_escape(description),
        robots = html_escape(robots),
        canonical = html_escape(canonical),
        origin = SITE_ORIGIN,
        hreflang = html_hreflang_links(canonical),
        stylesheet = html_escape(&locale_path("/pkg/styles.css", locale)),
        extra_head = extra_head,
        schema_json = schema_json,
        body = body,
        extra_body = extra_body,
    )
}

fn site_nav(locale: &Locale) -> String {
    format!(
        r#"<header class="masthead"><a class="brand" href="{}" aria-label="Automic Vault home"><img class="brand-mark" src="/assets/icon@2x.webp" alt="Automic Vault" width="54" height="54"><span class="brand-type">Automic Vault</span></a><nav class="nav" aria-label="Main navigation"><a href="{}">Docs</a><a href="{}">Security</a><a href="{}">Packages</a><a href="https://github.com/automic-vault/">GitHub</a></nav></header>"#,
        html_escape(&locale_path("/", locale)),
        html_escape(&locale_path("/docs/", locale)),
        html_escape(&locale_path("/security/", locale)),
        html_escape(&locale_path("/pkg/", locale)),
    )
}

fn site_footer(locale: &Locale) -> String {
    format!(
        r#"<footer class="site-footer"><p>Automic Vault secures Homebrew tools, CLI secrets, and command approval gates locally on your Mac before AI agents use them.</p><div class="footer-links"><a href="{}">Privacy</a><a href="{}">Terms</a><a href="{}">llms.txt</a></div></footer>"#,
        html_escape(&locale_path("/privacy/", locale)),
        html_escape(&locale_path("/terms/", locale)),
        html_escape(&locale_path("/llms.txt", locale)),
    )
}

fn copy_script() -> String {
    r#"  <script>
    document.addEventListener("click", async (event) => {
      const button = event.target.closest("[data-copy]");
      if (!button) return;
      try {
        await navigator.clipboard.writeText(button.getAttribute("data-copy"));
        const previous = button.textContent;
        button.textContent = "Copied";
        button.setAttribute("data-state", "copied");
        window.setTimeout(() => {
          button.textContent = previous;
          button.removeAttribute("data-state");
        }, 1600);
      } catch (_error) {
        button.textContent = "Copy failed";
        button.setAttribute("data-state", "error");
      }
    });
  </script>"#
        .to_string()
}

fn html_hreflang_links(canonical: &str) -> String {
    let Some(path) = canonical.strip_prefix(SITE_ORIGIN) else {
        return String::new();
    };
    let (_, canonical_path) = canonical_pkg_route(path);
    let path = canonical_path
        .strip_suffix("index.html")
        .unwrap_or(&canonical_path);
    let mut lines = Vec::new();
    for locale in LOCALES {
        lines.push(format!(
            r#"  <link rel="alternate" hreflang="{}" href="{}{}">"#,
            html_escape(locale.hreflang),
            SITE_ORIGIN,
            html_escape(&locale_path(path, locale))
        ));
    }
    lines.push(format!(
        r#"  <link rel="alternate" hreflang="x-default" href="{}{}">"#,
        SITE_ORIGIN,
        html_escape(path)
    ));
    lines.join("\n")
}

fn render_install(package: &PackageRow, locale: &Locale) -> String {
    let commands = install_command_entries(package);
    let primary = commands.first().cloned().unwrap_or_else(|| {
        json!({
            "platform": "portable",
            "manager": "Automic Vault",
            "command": package.install_command,
            "kind": "automic_vault",
            "confidence": 1.0,
            "evidence": "deterministic local package key"
        })
    });
    let command =
        value_str_key(&primary, "command").unwrap_or_else(|| package.install_command.clone());
    let notes = full_value(package, "install")
        .and_then(|value| value.get("notes"))
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .take(6)
                .filter_map(value_string)
                .map(|item| format!("<li>{}</li>", html_escape(&item)))
                .collect::<String>()
        })
        .unwrap_or_default();
    let platform_html = render_platform_install_commands(&commands[1..]);
    let manager_link = if package.package_manager_url.is_empty() {
        format!(
            "{} metadata was not linked in local data.",
            html_escape(&package.provider_label)
        )
    } else {
        format!(
            r#"<a href="{}">{}</a>"#,
            html_escape(&package.package_manager_url),
            html_escape(&package.package_manager_url)
        )
    };
    format!(
        r#"<section id="install" class="pkg-section install-section" aria-labelledby="install-title"><div class="install-command-panel"><div><p class="section-kicker">{}</p><h2 id="install-title">{}</h2></div><div class="terminal-block"><div class="terminal-head"><span>{}</span><div class="terminal-actions"><a class="download-av-button" href="/download/" aria-label="{}">{}</a><button class="copy-button" type="button" data-copy="{}" aria-label="{}">{}</button></div></div><pre><code>{}</code></pre></div>{}</div><div class="install-notes-grid"><article><h3>{}</h3><p>{}</p></article><article><h3>{}</h3><ul>{}</ul></article></div></section>"#,
        html_escape(&tx(locale, "install", "install")),
        html_escape(&tx(
            locale,
            "automicVaultInstallHeading",
            "Install with Automic Vault"
        )),
        html_escape(
            &value_str_key(&primary, "manager").unwrap_or_else(|| "Automic Vault".to_string())
        ),
        html_escape(&tx(locale, "downloadAV", "Download AV")),
        html_escape(&tx(locale, "downloadAV", "Download AV")),
        html_escape(&command),
        html_escape(&tx(locale, "copyInstallCommand", "Copy install command")),
        html_escape(&tx(locale, "copy", "Copy")),
        html_escape(&command),
        platform_html,
        html_escape(&tx(
            locale,
            "packageManagerSource",
            "Package manager source"
        )),
        manager_link,
        html_escape(&tx(locale, "platformNotes", "Platform notes")),
        if notes.is_empty() {
            format!(
                "<li>{}</li>",
                html_escape(&tx(
                    locale,
                    "noPlatformNotes",
                    "No package-specific platform notes were present."
                ))
            )
        } else {
            notes
        }
    )
}

fn render_platform_install_commands(commands: &[Value]) -> String {
    let mut sections = Vec::new();
    for (platform, label) in [
        ("macos", "macOS"),
        ("linux", "Linux"),
        ("windows", "Windows"),
        ("portable", "Portable and language managers"),
    ] {
        let rows = commands
            .iter()
            .filter(|item| {
                value_str_key(item, "platform").unwrap_or_else(|| "portable".to_string())
                    == platform
            })
            .map(install_command_row)
            .collect::<String>();
        if !rows.is_empty() {
            sections.push(format!(
                r#"<article><h3>{}</h3><div class="install-command-list">{}</div></article>"#,
                html_escape(label),
                rows
            ));
        }
    }
    if sections.is_empty() {
        String::new()
    } else {
        format!(
            r#"<div class="platform-install-grid" aria-label="Platform install commands">{}</div>"#,
            sections.join("")
        )
    }
}

fn install_command_row(item: &Value) -> String {
    let command = value_str_key(item, "command").unwrap_or_default();
    let label = install_command_manager_label(item);
    let confidence = value_f64_key(item, "confidence").unwrap_or(0.0);
    let confidence_label = if confidence >= 0.9 {
        "verified"
    } else {
        "inferred"
    };
    let source = install_command_source_html(item);
    format!(
        r#"<div class="install-command-row"><div class="install-command-head"><strong class="install-command-eyebrow">{}</strong><span>{} · {:.0}%</span></div><div class="install-command-shell"><code>{}</code><button class="copy-button" type="button" data-copy="{}" aria-label="Copy {} install command">Copy</button></div>{}</div>"#,
        html_escape(&label),
        confidence_label,
        confidence * 100.0,
        html_escape(&command),
        html_escape(&command),
        html_escape(&label),
        source
    )
}

fn install_command_manager_label(item: &Value) -> String {
    let manager = value_str_key(item, "manager").unwrap_or_else(|| "shell".to_string());
    let source_manager = item
        .get("source")
        .and_then(|source| value_str_key(source, "manager"))
        .unwrap_or_default();
    match (if source_manager.is_empty() {
        manager.clone()
    } else {
        source_manager
    })
    .to_ascii_lowercase()
    .as_str()
    {
        "apk" => "Alpine Linux apk".to_string(),
        "apt" => "Debian apt".to_string(),
        "chocolatey" => "Chocolatey".to_string(),
        "dnf" => "Fedora dnf".to_string(),
        "macports" => "MacPorts".to_string(),
        "nix" => "Nix".to_string(),
        "pacman" => "Arch Linux pacman".to_string(),
        "scoop" => "Scoop".to_string(),
        "winget" => "Windows Package Manager".to_string(),
        "zypper" => "openSUSE zypper".to_string(),
        "pip" => "Python pip".to_string(),
        "npm" => "npm".to_string(),
        _ => manager,
    }
}

fn install_command_source_html(item: &Value) -> String {
    if let Some(source) = item.get("source").filter(|value| value.is_object()) {
        let label = value_str_key(source, "source_label").unwrap_or_default();
        let package_name = value_str_key(source, "package_name")
            .or_else(|| value_str_key(source, "package_id"))
            .unwrap_or_default();
        let source_url = value_str_key(source, "source_url").unwrap_or_default();
        let mut pieces = [label, package_name]
            .into_iter()
            .filter(|value| !value.is_empty())
            .map(|value| html_escape(&value))
            .collect::<Vec<_>>();
        if !source_url.is_empty() {
            pieces.push(format!(
                r#"<a href="{}">source: {}</a>"#,
                html_escape(&source_url),
                html_escape(source_host_label(&source_url))
            ));
        }
        return if pieces.is_empty() {
            String::new()
        } else {
            format!(
                r#"<p class="install-command-source">{}</p>"#,
                pieces.join(" · ")
            )
        };
    }
    value_str_key(item, "evidence")
        .filter(|value| !value.is_empty())
        .map(|value| {
            format!(
                r#"<p class="install-command-source">{}</p>"#,
                html_escape(&value)
            )
        })
        .unwrap_or_default()
}

fn render_overview(package: &PackageRow, locale: &Locale) -> String {
    let aliases = full_string_array(package, "aliases");
    let alias_block = if aliases.is_empty() {
        format!(
            "<p>{}</p>",
            html_escape(&tx(
                locale,
                "noAliases",
                "No executable aliases were found in the local package database."
            ))
        )
    } else {
        format!(
            r#"<ul class="chip-list">{}</ul>"#,
            aliases
                .iter()
                .take(32)
                .map(|alias| format!("<li>{}</li>", html_escape(alias)))
                .collect::<String>()
        )
    };
    let homepage = if package.homepage.is_empty() {
        tx(
            locale,
            "homepageMissing",
            "Not present in the local metadata.",
        )
    } else {
        format!(
            r#"<a href="{}">{}</a>"#,
            html_escape(&package.homepage),
            html_escape(&package.homepage)
        )
    };
    format!(
        r#"<section class="pkg-section split-section"><div><p class="section-kicker">{}</p><h2>{}</h2><p>{}</p></div><div class="detail-stack"><article><h3>{}</h3><p>{}</p></article><article><h3>{}</h3>{}</article></div></section>"#,
        html_escape(&tx(locale, "overview", "overview")),
        html_escape(&tx(locale, "packageSummary", "Package summary")),
        html_escape(&package.summary),
        html_escape(&tx(locale, "homepage", "Homepage")),
        homepage,
        html_escape(&tx(locale, "commandsAndAliases", "Commands and aliases")),
        alias_block
    )
}

fn render_security(package: &PackageRow, locale: &Locale) -> String {
    let geiger = render_geiger(package);
    let install_signals = render_install_behavior_signals(package);
    let gate = render_gate(package, locale);
    if let Some(isotope) = full_value(package, "isotope").filter(|value| value.is_object()) {
        let justification = isotope.get("justification").unwrap_or(&Value::Null);
        let title = value_str_key(justification, "title")
            .or_else(|| Some("Protected-tool coverage".to_string()))
            .unwrap();
        let detail = value_str_key(justification, "detail")
            .or_else(|| full_opt_str(package, "isotopeReadme"))
            .unwrap_or_else(|| {
                "Automic Vault has a local secret-handling manifest for this package.".to_string()
            });
        let caveats = isotope
            .get("caveats")
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .take(8)
                    .filter_map(value_string)
                    .map(|item| format!("<li>{}</li>", html_escape(&item)))
                    .collect::<String>()
            })
            .unwrap_or_default();
        let readme = full_opt_str(package, "isotopeReadmeHtml")
            .filter(|value| !value.is_empty())
            .map(|html| {
                format!(
                    r#"<div class="readme-excerpt"><p class="readme-label">{}</p>{}<p class="readme-source">{}: <code>{}</code></p></div>"#,
                    html_escape(&tx(locale, "localReadmeExcerpt", "Local README excerpt")),
                    html,
                    html_escape(&tx(locale, "source", "Source")),
                    html_escape(&full_str(package, "isotopeReadmeSource"))
                )
            })
            .unwrap_or_default();
        return format!(
            r#"<section id="security" class="pkg-section security-section"><div><p class="section-kicker">{}</p><h2>{}</h2><p>{}</p>{}{}{}</div><div class="detail-stack"><article><h3>{}</h3><p>{}</p></article><article><h3>{}</h3><ul>{}</ul></article></div></section>{}"#,
            html_escape(&tx(locale, "radioisotopeKicker", "protected-tool coverage")),
            html_escape(&title),
            html_escape(&detail),
            geiger,
            install_signals,
            readme,
            html_escape(&tx(locale, "coverageSource", "Coverage source")),
            html_escape(&tx(
                locale,
                "sourceExcerpt",
                "Local secret-handling manifest"
            )),
            html_escape(&tx(locale, "caveats", "Caveats")),
            if caveats.is_empty() {
                "<li>No caveats were listed in the local manifest.</li>".to_string()
            } else {
                caveats
            },
            render_gate(package, locale)
        );
    }
    if !gate.is_empty() {
        return gate;
    }
    format!(
        r#"<section id="security" class="pkg-section security-section"><div><p class="section-kicker">{}</p><h2>{}</h2><p>{}</p>{}{}</div><div class="detail-stack"><article><h3>{}</h3><p>{}</p></article></div></section>"#,
        html_escape(&tx(locale, "securityPosture", "security posture")),
        html_escape(&security_heading(package, locale)),
        html_escape(&security_summary(package, locale)),
        geiger,
        install_signals,
        html_escape(&tx(locale, "recommendedReview", "Recommended review")),
        html_escape(&tx(
            locale,
            "recommendedReviewCopy",
            "Before unattended agent use, check whether the tool reads plaintext credentials, writes remote state, publishes artifacts, or shells out to plugins."
        ))
    )
}

fn render_geiger(package: &PackageRow) -> String {
    let Some(geiger) = full_value(package, "geiger").filter(|value| value.is_object()) else {
        return String::new();
    };
    let reasons = value_array(geiger, "reasons")
        .into_iter()
        .take(5)
        .filter_map(value_string)
        .map(|item| format!("<li>{}</li>", html_escape(&item)))
        .collect::<String>();
    let signals = value_array(geiger, "signals")
        .into_iter()
        .take(5)
        .filter_map(value_string)
        .map(|item| format!("<li>{}</li>", html_escape(&item)))
        .collect::<String>();
    format!(
        r#"<div class="signal-grid" aria-label="Geiger classifier signals"><article><h3>Risk classifier</h3><p><strong>{}</strong> risk · {} confidence · {}</p></article><article><h3>Why</h3><ul>{}</ul></article><article><h3>Signals</h3><ul>{}</ul></article></div>"#,
        html_escape(&value_str_key(geiger, "level").unwrap_or_else(|| "unknown".to_string())),
        html_escape(&value_str_key(geiger, "confidence").unwrap_or_else(|| "unknown".to_string())),
        html_escape(
            &value_str_key(geiger, "category").unwrap_or_else(|| "uncategorized".to_string())
        ),
        if reasons.is_empty() {
            "<li>No classifier reasons were present.</li>".to_string()
        } else {
            reasons
        },
        if signals.is_empty() {
            "<li>No classifier signals were present.</li>".to_string()
        } else {
            signals
        },
    )
}

fn render_install_behavior_signals(package: &PackageRow) -> String {
    let Some(behavior) = full_value(package, "installBehavior").filter(|value| value.is_object())
    else {
        return String::new();
    };
    let mut signals = Vec::new();
    if let Some(items) = behavior
        .get("lifecycleScripts")
        .and_then(Value::as_array)
        .filter(|items| !items.is_empty())
    {
        signals.push(format!(
            "npm lifecycle scripts are declared: {}.",
            items
                .iter()
                .take(5)
                .filter_map(value_string)
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    if let Some(value) = behavior.get("postInstallDefined").and_then(Value::as_bool) {
        signals.push(if value {
            if package.provider == "npm" {
                "npm package metadata declares a postinstall script.".to_string()
            } else {
                "Homebrew declares a post-install hook for this formula.".to_string()
            }
        } else if package.provider == "npm" {
            "No npm postinstall script is recorded in package metadata.".to_string()
        } else {
            "No Homebrew post-install hook is recorded in formula metadata.".to_string()
        });
    }
    if behavior.get("prepareDefined").and_then(Value::as_bool) == Some(true) {
        signals.push("npm package metadata declares a prepare script.".to_string());
    }
    if let Some(value) = value_str_key(behavior, "pythonRequires").filter(|value| !value.is_empty())
    {
        signals.push(format!("PyPI metadata requires Python {value}."));
    }
    if let Some(value) = value_i64_key(behavior, "requiresDistCount").filter(|value| *value > 0) {
        signals.push(format!(
            "PyPI metadata lists {value} dependency specifications."
        ));
    }
    if value_str_key(behavior, "service")
        .filter(|value| !value.is_empty())
        .is_some()
    {
        signals.push("Formula metadata declares a service or daemon block.".to_string());
    }
    if let Some(bottle) = full_value(package, "bottle").filter(|value| value.is_object()) {
        if bottle.get("available").and_then(Value::as_bool) == Some(true) {
            let platforms = value_array(bottle, "platforms");
            if platforms.is_empty() {
                signals.push("Homebrew bottle metadata is available.".to_string());
            } else {
                signals.push(format!(
                    "Homebrew bottle metadata is available for {} platform targets.",
                    platforms.len()
                ));
            }
        } else {
            signals.push("No Homebrew bottle metadata was recorded.".to_string());
        }
    }
    let dependencies = full_string_array(package, "dependencies");
    if !dependencies.is_empty() {
        signals.push(format!(
            "Installs with {} runtime dependencies.",
            dependencies.len()
        ));
    }
    let build_dependencies = full_string_array(package, "buildDependencies");
    if !build_dependencies.is_empty() {
        signals.push(format!(
            "Build metadata lists {} build dependencies.",
            build_dependencies.len()
        ));
    }
    if signals.is_empty() {
        String::new()
    } else {
        format!(
            r#"<div class="signal-grid install-signal-grid" aria-label="Install behavior signals"><article><h3>Install behavior</h3><ul>{}</ul></article></div>"#,
            signals
                .into_iter()
                .take(6)
                .map(|item| format!("<li>{}</li>", html_escape(&item)))
                .collect::<String>()
        )
    }
}

fn render_gate(package: &PackageRow, locale: &Locale) -> String {
    let Some(gate) = full_value(package, "approvalGate").filter(|value| value.is_object()) else {
        return String::new();
    };
    let rules = value_array(gate, "rules")
        .into_iter()
        .filter_map(value_string)
        .map(|item| format!("<li>{}</li>", html_escape(&item)))
        .collect::<String>();
    let severities = full_value(package, "approvalGate")
        .map(|gate| value_array(gate, "severities"))
        .unwrap_or_default()
        .into_iter()
        .filter_map(value_string)
        .collect::<Vec<_>>()
        .join(", ");
    let entrypoints = value_array(gate, "entrypoints")
        .into_iter()
        .filter_map(value_string)
        .collect::<Vec<_>>()
        .join(", ");
    let reviewed = value_str_key(gate, "reviewed_at").unwrap_or_default();
    let reviewed_copy = if reviewed.is_empty() {
        String::new()
    } else {
        format!(", reviewed {reviewed}")
    };
    format!(
        r#"<section class="pkg-section split-section gate-section"><div><p class="section-kicker">{}</p><h2>{}</h2><p>{}</p></div><div class="detail-stack"><article><h3>{}</h3><ul>{}</ul></article></div></section>"#,
        html_escape(&tx(locale, "approvalGatesKicker", "approval gates")),
        html_escape(&tx(
            locale,
            "approvalGateHeading",
            "Human review metadata for risky commands"
        )),
        html_escape(&txf(
            locale,
            "approvalGateCopy",
            "The local approval-gate seed includes {count} rules for {name}. Covered entrypoints: {entrypoints}. Severity labels: {severities}. Coverage: {coverage}{reviewed}.",
            &[
                (
                    "count",
                    value_i64_key(gate, "rule_count").unwrap_or(0).to_string()
                ),
                ("name", package.display_name.clone()),
                (
                    "entrypoints",
                    if entrypoints.is_empty() {
                        package.display_name.clone()
                    } else {
                        entrypoints.clone()
                    }
                ),
                (
                    "severities",
                    if severities.is_empty() {
                        "not specified".to_string()
                    } else {
                        severities.clone()
                    }
                ),
                (
                    "coverage",
                    value_str_key(gate, "coverage_status").unwrap_or_else(|| "unknown".to_string())
                ),
                ("reviewed", reviewed_copy.clone()),
            ],
        )),
        html_escape(&tx(locale, "exampleGatedActions", "Example gated actions")),
        if rules.is_empty() {
            format!(
                "<li>{}</li>",
                html_escape(&tx(
                    locale,
                    "noApprovalRules",
                    "No rule descriptions were present."
                ))
            )
        } else {
            rules
        },
    )
}

fn render_executables(package: &PackageRow, locale: &Locale) -> String {
    let mut rows = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for item in value_array(&package.data.full, "executablesDetailed") {
        let name = value_str_key(item, "name")
            .or_else(|| value_str_key(item, "target"))
            .or_else(|| value_str_key(item, "source"))
            .unwrap_or_default();
        if name.is_empty() || !seen.insert(name.clone()) {
            continue;
        }
        rows.push(executable_row(
            &name,
            &value_str_key(item, "kind").unwrap_or_else(|| "executable".to_string()),
            &value_str_key(item, "exposure").unwrap_or_else(|| "global executable".to_string()),
            &value_str_key(item, "note").unwrap_or_default(),
        ));
    }
    for item in value_array(&package.data.full, "binaries") {
        let name = value_str_key(item, "target")
            .or_else(|| value_str_key(item, "source"))
            .unwrap_or_default();
        if !name.is_empty() && seen.insert(name.clone()) {
            rows.push(executable_row(
                &name,
                "binary",
                "Homebrew cask binary",
                &value_str_key(item, "source").unwrap_or_default(),
            ));
        }
    }
    let stub_exclusions = full_value(package, "extra")
        .and_then(|extra| extra.get("stub_exclusions"))
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(value_string)
                .collect::<std::collections::BTreeSet<_>>()
        })
        .unwrap_or_default();
    for alias in full_string_array(package, "aliases") {
        if seen.insert(alias.clone()) {
            rows.push(executable_row(
                &alias,
                "executable",
                if stub_exclusions.contains(&alias) {
                    "Automic Vault stub excluded"
                } else {
                    "indexed executable"
                },
                "Discovered from the local executable index.",
            ));
        }
    }
    format!(
        r#"<section class="pkg-section" aria-labelledby="executables-title"><p class="section-kicker">{}</p><h2 id="executables-title">{}</h2><div class="table-wrap executable-table"><table><thead><tr><th>{}</th><th>{}</th><th>{}</th><th>{}</th></tr></thead><tbody>{}</tbody></table></div></section>"#,
        html_escape(&tx(locale, "executables", "executables")),
        html_escape(&tx(locale, "executablesTitle", "Installed executables")),
        html_escape(&tx(locale, "command", "Command")),
        html_escape(&tx(locale, "kind", "Kind")),
        html_escape(&tx(locale, "exposure", "Exposure")),
        html_escape(&tx(locale, "note", "Note")),
        if rows.is_empty() {
            format!(
                r#"<tr><td colspan="4">{}</td></tr>"#,
                html_escape(&tx(
                    locale,
                    "executableDataMissing",
                    "No executable data was present."
                ))
            )
        } else {
            rows.join("")
        }
    )
}

fn executable_row(name: &str, kind: &str, exposure: &str, note: &str) -> String {
    format!(
        "<tr><td><code>{}</code></td><td>{}</td><td>{}</td><td>{}</td></tr>",
        html_escape(name),
        html_escape(kind),
        html_escape(exposure),
        html_escape(note)
    )
}

fn render_freshness(package: &PackageRow, generated_at: &str, locale: &Locale) -> String {
    let freshness = full_value(package, "versionFreshness").unwrap_or(&Value::Null);
    let manager = freshness.get("packageManager").unwrap_or(&Value::Null);
    let site = freshness.get("siteData").unwrap_or(&Value::Null);
    let upstream = freshness.get("upstream").unwrap_or(&Value::Null);
    let warnings = value_array(freshness, "warnings")
        .into_iter()
        .filter(|item| item.is_object())
        .map(|item| {
            let severity = value_str_key(item, "severity").unwrap_or_else(|| "info".to_string());
            let message = value_str_key(item, "message")
                .or_else(|| value_str_key(item, "kind"))
                .unwrap_or_else(|| "freshness".to_string());
            let evidence = value_str_key(item, "evidence").unwrap_or_default();
            let confidence = value_str_key(item, "confidence").unwrap_or_default();
            format!(
                r#"<li class="freshness-item freshness-{}"><strong>{}</strong><span>{}</span>{}{}</li>"#,
                html_escape(&severity),
                html_escape(&severity),
                html_escape(&message),
                if evidence.is_empty() {
                    String::new()
                } else {
                    format!("<small>{}</small>", link_value(&evidence))
                },
                if confidence.is_empty() {
                    String::new()
                } else {
                    format!("<em>{} confidence</em>", html_escape(&confidence))
                }
            )
        })
        .collect::<String>();
    let warnings = if warnings.is_empty() {
        r#"<li class="freshness-item freshness-info"><strong>ok</strong><span>No freshness warnings were generated.</span></li>"#.to_string()
    } else {
        warnings
    };
    let repository = value_str_key(upstream, "repository").unwrap_or_default();
    format!(
        r#"<section class="pkg-section split-section freshness-section" aria-labelledby="freshness-title" data-pagefind-ignore="all"><div><p class="section-kicker">{}</p><h2 id="freshness-title">{}</h2><p>{}</p></div><div><div class="freshness-metrics"><div><span>{}</span><strong>{}</strong></div><div><span>{}</span><strong>{}</strong></div><div><span>{}</span><strong>{}</strong></div><div><span>{}</span><strong>{}</strong></div><div><span>{}</span><strong>{}</strong></div><div><span>{}</span><strong>{}</strong></div></div>{}<ul class="freshness-list">{}</ul></div></section>"#,
        html_escape(&tx(locale, "freshness", "freshness")),
        html_escape(&tx(locale, "freshnessTitle", "Version and freshness")),
        html_escape(&tx(
            locale,
            "freshnessCopy",
            "These signals separate page generation age, package-manager activity, and upstream release comparison. Version lag is warned only when an evidence URL and comparable versions are present."
        )),
        html_escape(&tx(locale, "pageGenerated", "page generated")),
        html_escape(&fmt_date(generated_at)),
        html_escape(&tx(locale, "managerVersion", "manager version")),
        html_escape(
            &value_str_key(manager, "version")
                .unwrap_or_else(|| empty_as_unknown(&package.version).to_string())
        ),
        html_escape(&tx(locale, "managerUpdated", "manager updated")),
        html_escape(&fmt_date(
            &value_str_key(manager, "updatedAt").unwrap_or_else(|| package.last_updated_at.clone())
        )),
        html_escape(&tx(locale, "localData", "local data")),
        html_escape(&value_str_key(site, "status").unwrap_or_else(|| "unknown".to_string())),
        html_escape(&tx(locale, "upstream", "upstream")),
        html_escape(
            &value_str_key(upstream, "comparison").unwrap_or_else(|| "not available".to_string())
        ),
        html_escape(&tx(locale, "upstreamLatestDetected", "latest detected")),
        html_escape(
            &value_str_key(upstream, "latestVersion").unwrap_or_else(|| "not detected".to_string())
        ),
        if repository.is_empty() {
            String::new()
        } else {
            format!(
                r#"<p class="freshness-repo"><a href="{}">{}</a></p>"#,
                html_escape(&repository),
                html_escape(&repository)
            )
        },
        warnings
    )
}

fn render_install_metadata(package: &PackageRow, locale: &Locale) -> String {
    let mut rows = Vec::new();
    for (label, value) in [
        ("Package key", package.package_key.clone()),
        ("Version", package.version.clone()),
        ("Package manager", package.provider_label.clone()),
        ("Package manager page", package.package_manager_url.clone()),
        ("Homepage", package.homepage.clone()),
        ("Repository", package.repository.clone()),
        ("Upstream docs", full_str(package, "upstreamDocs")),
        ("License", package.license.clone()),
        ("Source archive", full_str(package, "sourceArchive")),
        ("Issue tracker", full_str(package, "issueTracker")),
        ("Last updated", package.last_updated_at.clone()),
        ("Last verified", full_str(package, "lastVerified")),
        ("Published", full_str(package, "publishedAt")),
        ("Pulse", full_str(package, "pulseKind")),
        ("SHA-256", full_str(package, "sha256")),
        ("Download URL", full_str(package, "url")),
    ] {
        if !value.is_empty() {
            rows.push((label.to_string(), value));
        }
    }
    push_joined_row(
        &mut rows,
        "Dependencies",
        &full_string_array(package, "dependencies"),
    );
    push_joined_row(
        &mut rows,
        "Build dependencies",
        &full_string_array(package, "buildDependencies"),
    );
    push_joined_row(
        &mut rows,
        "Uses from macOS",
        &full_string_array(package, "usesFromMacos"),
    );
    if let Some(bottle) = full_value(package, "bottle").filter(|value| value.is_object()) {
        let mut detail = if bottle.get("available").and_then(Value::as_bool) == Some(true) {
            "available".to_string()
        } else {
            "not recorded".to_string()
        };
        let platforms = value_array(bottle, "platforms")
            .into_iter()
            .filter_map(value_string)
            .collect::<Vec<_>>();
        if !platforms.is_empty() {
            detail.push_str(&format!(" ({})", platforms.join(", ")));
        }
        rows.push(("Bottle".to_string(), detail));
    }
    if let Some(behavior) = full_value(package, "installBehavior").filter(|value| value.is_object())
    {
        if let Some(post_install) = behavior.get("postInstallDefined").and_then(Value::as_bool) {
            rows.push((
                if package.provider == "npm" {
                    "npm postinstall"
                } else {
                    "Homebrew post-install"
                }
                .to_string(),
                if post_install {
                    "defined"
                } else {
                    "not defined"
                }
                .to_string(),
            ));
        }
        rows.push((
            "Service".to_string(),
            value_str_key(behavior, "service")
                .filter(|value| !value.is_empty())
                .unwrap_or_else(|| "none declared".to_string()),
        ));
        if let Some(caveats) = value_str_key(behavior, "caveats").filter(|value| !value.is_empty())
        {
            rows.push(("Caveats".to_string(), caveats));
        }
    }
    push_joined_row(
        &mut rows,
        "Keywords",
        &full_string_array(package, "keywords"),
    );
    push_joined_row(
        &mut rows,
        "Classifiers",
        &full_string_array(package, "classifiers"),
    );
    let row_html = rows
        .into_iter()
        .map(|(label, value)| {
            format!(
                "<tr><th>{}</th><td>{}</td></tr>",
                html_escape(&label),
                link_value(&value)
            )
        })
        .collect::<String>();
    format!(
        r#"<section class="pkg-section"><p class="section-kicker">{}</p><h2>{}</h2><div class="table-wrap"><table><tbody>{}</tbody></table></div></section>"#,
        html_escape(&tx(locale, "packageMetadataKicker", "install metadata")),
        html_escape(&tx(locale, "metadataTitle", "Package metadata")),
        if row_html.is_empty() {
            format!(
                "<tr><th>{}</th><td>{}</td></tr>",
                html_escape(&tx(locale, "status", "Status")),
                html_escape(&tx(
                    locale,
                    "metadataEmpty",
                    "No resolver details were present."
                ))
            )
        } else {
            row_html
        }
    )
}

fn render_related(package: &PackageRow, locale: &Locale) -> String {
    let hubs = value_array(&package.data.full, "packageHubs")
        .into_iter()
        .take(4)
        .filter_map(|hub| {
            let slug = value_str_key(hub, "slug")?;
            if slug.is_empty() {
                return None;
            }
            let label = value_str_key(hub, "label").unwrap_or_else(|| slug.clone());
            let reason = value_str_key(hub, "reason").unwrap_or_default();
            Some(format!(
                r#"<li><a href="{}">{}</a>{}</li>"#,
                html_escape(&locale_path(&format!("/pkg/{slug}/"), locale)),
                html_escape(&label),
                if reason.is_empty() {
                    String::new()
                } else {
                    format!("<span>{}</span>", html_escape(&reason))
                }
            ))
        })
        .collect::<Vec<_>>();
    let related = related_links(package, locale, "relatedPackages", 8, false);
    let workflow = related_links(package, locale, "relatedPackages", 6, true);
    let also = related_links(package, locale, "alsoAvailableVia", 4, false);
    let guides = core_security_guides(package, locale);
    let columns = [
        related_article(&tx(locale, "topicalHubs", "Topical hubs"), hubs),
        related_article(&tx(locale, "relatedTools", "Related tools"), related),
        related_article(
            &tx(locale, "sameWorkflow", "Same workflow"),
            [workflow, also].concat(),
        ),
        related_article(
            &tx(locale, "agentSecurityGuides", "Agent security guides"),
            guides,
        ),
    ]
    .into_iter()
    .filter(|value| !value.is_empty())
    .collect::<String>();
    format!(
        r#"<section class="pkg-section split-section related-section" aria-labelledby="related-title"><div><p class="section-kicker">{}</p><h2 id="related-title">{}</h2><p>{}</p></div><div class="related-columns">{}</div></section>"#,
        html_escape(&tx(locale, "packageGraph", "package graph")),
        html_escape(&tx(locale, "internalLinks", "Internal package links")),
        html_escape(&tx(
            locale,
            "packageGraphCopy",
            "Links come from deterministic package relationships, av.db category and tag curation, ecosystem matches, and package hub membership."
        )),
        columns
    )
}

fn render_sources(package: &PackageRow, locale: &Locale) -> String {
    let mut notes = full_string_array(package, "sourceNotes");
    notes.sort();
    notes.dedup();
    if notes.is_empty() {
        notes.push("local package generator".to_string());
    }
    let items = notes
        .into_iter()
        .map(|note| format!("<li>{}</li>", html_escape(&note)))
        .collect::<String>();
    format!(
        r#"<section class="pkg-section split-section sources-section"><div><p class="section-kicker">{}</p><h2>{}</h2><p>{}</p></div><div class="detail-stack"><article><h3>{}</h3><ul>{}</ul></article></div></section>"#,
        html_escape(&tx(locale, "sourceTrail", "source trail")),
        html_escape(&tx(
            locale,
            "generatedFromRepositoryData",
            "Generated from repository data"
        )),
        tx(
            locale,
            "sourcesCopy",
            "This page is generated by <code>av-web</code> from the private package SQLite artifact built by <code>scripts/generate-pkg-sqlite.py</code>."
        ),
        html_escape(&tx(locale, "usedSources", "Used sources")),
        items
    )
}

fn package_isotope(package: &PackageRow) -> bool {
    full_value(package, "isotope")
        .filter(|value| value.is_object())
        .is_some()
}

fn package_gate(package: &PackageRow) -> bool {
    full_value(package, "approvalGate")
        .filter(|value| value.is_object())
        .is_some()
}

fn package_non_low_geiger(package: &PackageRow) -> bool {
    let Some(geiger) = full_value(package, "geiger").filter(|value| value.is_object()) else {
        return false;
    };
    !matches!(
        value_str_key(geiger, "level")
            .unwrap_or_default()
            .to_ascii_lowercase()
            .as_str(),
        "" | "green" | "low" | "unknown"
    )
}

fn label_for(package: &PackageRow, locale: &Locale) -> String {
    let mut labels = vec![package.provider.clone()];
    if package_isotope(package) {
        labels.push(tx(locale, "radioisotopeKicker", "protected-tool coverage"));
    }
    if package_gate(package) {
        labels.push(tx(locale, "approvalGatesKicker", "approval gates"));
    }
    if let Some(rank) = package.rank {
        labels.push(format!("{} {rank}", tx(locale, "rank", "rank")));
    }
    labels.join(" / ")
}

fn hub_group_sections(hubs: &[HubSummary], locale: &Locale) -> String {
    let mut sections = Vec::new();
    for (group, label) in [
        (
            "security",
            tx(locale, "hubSecurityGroupTitle", "Security hubs"),
        ),
        (
            "topical",
            tx(locale, "hubTopicalGroupTitle", "Topical hubs"),
        ),
        (
            "ecosystem",
            tx(locale, "hubEcosystemGroupTitle", "Ecosystem hubs"),
        ),
    ] {
        let cards = hubs
            .iter()
            .filter(|summary| summary.hub.group == group)
            .map(|summary| {
                format!(
                    r#"<a class="hub-card" href="{}"><span>{}</span><strong>{}</strong><small>{}</small></a>"#,
                    html_escape(&locale_path(&summary.hub.path, locale)),
                    html_escape(&summary.hub.title),
                    html_escape(&fmt_int(summary.package_count)),
                    html_escape(&short_text(&summary.hub.description, 96))
                )
            })
            .collect::<String>();
        if !cards.is_empty() {
            sections.push(format!(
                r#"<section class="hub-group"><h3>{}</h3><div class="hub-grid">{}</div></section>"#,
                html_escape(&label),
                cards
            ));
        }
    }
    sections.join("")
}

fn index_package_row(package: &PackageRow, locale: &Locale) -> String {
    format!(
        r#"<a class="package-row" href="{}"><span>{}</span><small>{}</small></a>"#,
        html_escape(&locale_path(&package.path, locale)),
        html_escape(&package.display_name),
        html_escape(&label_for(package, locale))
    )
}

fn hub_cluster_block(title: &str, packages: &[&PackageRow], locale: &Locale) -> String {
    if packages.is_empty() {
        return String::new();
    }
    let cards = packages
        .iter()
        .map(|package| hub_spoke_card(package, locale))
        .collect::<String>();
    format!(
        r#"<section class="pkg-section hub-cluster"><h2>{}</h2><div class="package-list hub-spoke-list">{}</div></section>"#,
        html_escape(title),
        cards
    )
}

fn hub_spoke_card(package: &PackageRow, locale: &Locale) -> String {
    format!(
        r#"<a class="package-row" href="{}"><span>{}</span><small>{}</small></a>"#,
        html_escape(&locale_path(&package.path, locale)),
        html_escape(&package.display_name),
        html_escape(&hub_package_reason(package, locale))
    )
}

fn hub_related_block(title: &str, links: &[String]) -> String {
    if links.is_empty() {
        return String::new();
    }
    format!(
        r#"<section class="pkg-section hub-cluster"><h2>{}</h2><div class="hub-related-list">{}</div></section>"#,
        html_escape(title),
        links.join("")
    )
}

fn related_hub_links(
    hub: &HubRow,
    packages: &[(PackageRow, String)],
    locale: &Locale,
) -> Vec<String> {
    let mut counts: BTreeMap<String, (String, String, i64)> = BTreeMap::new();
    for (package, _reason) in packages {
        for item in value_array(&package.data.full, "packageHubs") {
            let slug = value_str_key(item, "slug").unwrap_or_default();
            if slug.is_empty() || slug == hub.slug {
                continue;
            }
            let label = value_str_key(item, "label").unwrap_or_else(|| slug.replace('-', " "));
            let reason = value_str_key(item, "reason").unwrap_or_default();
            let entry = counts.entry(slug).or_insert((label, reason.clone(), 0));
            if entry.1.is_empty() && !reason.is_empty() {
                entry.1 = reason;
            }
            entry.2 += 1;
        }
    }
    let mut ranked = counts.into_iter().collect::<Vec<_>>();
    ranked.sort_by(|left, right| {
        right.1.2.cmp(&left.1.2).then_with(|| {
            left.1
                .0
                .to_ascii_lowercase()
                .cmp(&right.1.0.to_ascii_lowercase())
        })
    });
    ranked
        .into_iter()
        .take(8)
        .map(|(slug, (label, reason, count))| {
            let fallback_reason = tx(locale, "packageGraph", "package graph");
            let reason = if reason.is_empty() {
                fallback_reason.as_str()
            } else {
                reason.as_str()
            };
            format!(
                r#"<a class="hub-related-card" href="{}"><span>{}</span><small>{}</small><strong>{}</strong></a>"#,
                html_escape(&locale_path(&format!("/pkg/{slug}/"), locale)),
                html_escape(&label),
                html_escape(reason),
                html_escape(&fmt_int(count))
            )
        })
        .collect()
}

fn hub_package_reason(package: &PackageRow, locale: &Locale) -> String {
    if let Some(isotope) = full_value(package, "isotope").filter(|value| value.is_object()) {
        if let Some(title) = isotope
            .get("justification")
            .and_then(|justification| value_str_key(justification, "title"))
        {
            return title;
        }
    }
    if let Some(gate) = full_value(package, "approvalGate").filter(|value| value.is_object()) {
        return txf(
            locale,
            "hubPackageReasonApproval",
            "{count} approval-gate rules are present.",
            &[(
                "count",
                value_i64_key(gate, "rule_count")
                    .map(fmt_int)
                    .unwrap_or_else(|| "Local".to_string()),
            )],
        );
    }
    if let Some(geiger) = full_value(package, "geiger").filter(|value| value.is_object()) {
        if let Some(reason) = value_array(geiger, "reasons")
            .into_iter()
            .find_map(value_string)
        {
            return short_text(&reason, 140);
        }
    }
    if !package.summary.is_empty() {
        return short_text(&package.summary, 140);
    }
    let aliases = full_string_array(package, "aliases");
    if !aliases.is_empty() {
        return format!(
            "{}",
            txf(
                locale,
                "hubPackageReasonAlias",
                "Executable aliases include {aliases}.",
                &[(
                    "aliases",
                    aliases.into_iter().take(4).collect::<Vec<_>>().join(", "),
                )],
            )
        );
    }
    tx(
        locale,
        "hubPackageReasonDefault",
        "Matched package metadata for this hub.",
    )
}

fn hub_package_row(package: &PackageRow, reason: &str, locale: &Locale) -> String {
    let mut signals = Vec::new();
    if package_isotope(package) {
        signals.push(tx(locale, "radioisotopeKicker", "protected-tool coverage"));
    }
    if package_gate(package) {
        signals.push(tx(locale, "approvalGatesKicker", "approval gate"));
    }
    if let Some(geiger) = full_value(package, "geiger").filter(|value| value.is_object()) {
        signals.push(txf(
            locale,
            "riskLevel",
            "{level} risk",
            &[(
                "level",
                value_str_key(geiger, "level").unwrap_or_else(|| "unknown".to_string()),
            )],
        ));
    }
    if !package.version.is_empty() {
        signals.push(format!("v{}", package.version));
    }
    let reason = if reason.trim().is_empty() {
        hub_package_reason(package, locale)
    } else {
        reason.to_string()
    };
    format!(
        r#"<tr><td><a href="{}">{}</a></td><td>{}</td><td>{}</td><td>{}</td></tr>"#,
        html_escape(&package.path),
        html_escape(&package.display_name),
        html_escape(&package.provider_label),
        html_escape(&if signals.is_empty() {
            label_for(package, locale)
        } else {
            signals.join(", ")
        }),
        html_escape(&reason)
    )
}

fn schema_for_hub(
    hub: &HubRow,
    pages: Vec<&PackageRow>,
    description: &str,
    updated: &str,
    locale: &Locale,
) -> Value {
    let url = locale_url(&hub.path, locale);
    let items = pages
        .into_iter()
        .take(72)
        .enumerate()
        .map(|(index, package)| {
            json!({
                "@type": "ListItem",
                "position": index + 1,
                "url": locale_url(&package.path, locale),
                "name": package.display_name,
                "description": hub_package_reason(package, locale)
            })
        })
        .collect::<Vec<_>>();
    json!({
        "@context": "https://schema.org",
        "@graph": [
            {"@type": "WebSite", "@id": format!("{SITE_ORIGIN}/#website"), "name": "Automic Vault", "url": format!("{SITE_ORIGIN}/")},
            {"@type": "Organization", "@id": format!("{SITE_ORIGIN}/#organization"), "name": "Automic Vault", "url": format!("{SITE_ORIGIN}/")},
            {"@type": "Person", "@id": format!("{SITE_ORIGIN}/about/#max-howell"), "name": "Max Howell", "url": format!("{SITE_ORIGIN}/about/")},
            {
                "@type": "CollectionPage",
                "@id": format!("{url}#webpage"),
                "name": hub.title,
                "headline": hub.title,
                "url": url,
                "description": description,
                "inLanguage": locale.hreflang,
                "dateModified": updated,
                "isPartOf": {"@id": format!("{SITE_ORIGIN}/#website")},
                "author": {"@id": format!("{SITE_ORIGIN}/about/#max-howell")},
                "publisher": {"@id": format!("{SITE_ORIGIN}/#organization")},
                "mainEntity": {"@id": format!("{url}#list")}
            },
            {
                "@type": "ItemList",
                "@id": format!("{url}#list"),
                "name": hub.title,
                "itemListElement": items
            }
        ]
    })
}

fn full_value<'a>(package: &'a PackageRow, key: &str) -> Option<&'a Value> {
    package.data.full.get(key).filter(|value| !value.is_null())
}

fn full_str(package: &PackageRow, key: &str) -> String {
    full_opt_str(package, key).unwrap_or_default()
}

fn full_opt_str(package: &PackageRow, key: &str) -> Option<String> {
    full_value(package, key).and_then(value_string)
}

fn full_string_array(package: &PackageRow, key: &str) -> Vec<String> {
    let Some(items) = full_value(package, key).and_then(Value::as_array) else {
        return Vec::new();
    };
    let mut seen = std::collections::BTreeSet::new();
    let mut values = Vec::new();
    for item in items {
        let value = value_string(item)
            .or_else(|| value_str_key(item, "label"))
            .or_else(|| value_str_key(item, "name"))
            .or_else(|| value_str_key(item, "target"))
            .or_else(|| value_str_key(item, "source"))
            .or_else(|| value_str_key(item, "package"))
            .or_else(|| value_str_key(item, "key"))
            .unwrap_or_default();
        let value = value.trim();
        if !value.is_empty() && seen.insert(value.to_string()) {
            values.push(value.to_string());
        }
    }
    values
}

fn value_array<'a>(value: &'a Value, key: &str) -> Vec<&'a Value> {
    value
        .get(key)
        .and_then(Value::as_array)
        .map(|items| items.iter().collect())
        .unwrap_or_default()
}

fn value_string(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => {
            let text = text.trim();
            (!text.is_empty()).then(|| text.to_string())
        }
        Value::Number(number) => Some(number.to_string()),
        Value::Bool(value) => Some(value.to_string()),
        _ => None,
    }
}

fn value_str_key(value: &Value, key: &str) -> Option<String> {
    value.get(key).and_then(value_string)
}

fn value_i64_key(value: &Value, key: &str) -> Option<i64> {
    value.get(key).and_then(|value| {
        value
            .as_i64()
            .or_else(|| value.as_u64().and_then(|value| i64::try_from(value).ok()))
            .or_else(|| value.as_str().and_then(|value| value.parse().ok()))
    })
}

fn value_f64_key(value: &Value, key: &str) -> Option<f64> {
    value.get(key).and_then(|value| {
        value
            .as_f64()
            .or_else(|| value.as_str().and_then(|value| value.parse().ok()))
    })
}

fn first_non_empty(values: &[String]) -> Option<String> {
    values
        .iter()
        .map(|value| value.trim())
        .find(|value| !value.is_empty())
        .map(str::to_string)
}

fn fmt_date(value: &str) -> String {
    let value = value.trim();
    if value.len() >= 10 && value.as_bytes().get(4) == Some(&b'-') {
        value[..10].to_string()
    } else {
        value.to_string()
    }
}

fn fmt_int(value: i64) -> String {
    let mut digits = value.abs().to_string();
    let mut out = String::new();
    while digits.len() > 3 {
        let tail = digits.split_off(digits.len() - 3);
        if out.is_empty() {
            out = tail;
        } else {
            out = format!("{tail},{out}");
        }
    }
    if out.is_empty() {
        out = digits;
    } else if !digits.is_empty() {
        out = format!("{digits},{out}");
    }
    if value < 0 { format!("-{out}") } else { out }
}

fn locale_url(path: &str, locale: &Locale) -> String {
    format!("{SITE_ORIGIN}{}", locale_path(path, locale))
}

fn tx(locale: &Locale, key: &str, default: &str) -> String {
    if locale.code == "en" {
        return default.to_string();
    }
    let templates = I18N_PKG_TEMPLATES
        .get_or_init(|| serde_json::from_str(I18N_PKG_TEMPLATES_JSON).unwrap_or(Value::Null));
    templates
        .get(locale.code)
        .and_then(|items| items.get(key))
        .and_then(Value::as_str)
        .unwrap_or(default)
        .to_string()
}

fn txf(locale: &Locale, key: &str, default: &str, replacements: &[(&str, String)]) -> String {
    let mut value = tx(locale, key, default);
    for (name, replacement) in replacements {
        value = value.replace(&format!("{{{name}}}"), replacement);
    }
    value
}

fn source_host_label(url: &str) -> &str {
    url.trim_start_matches("https://")
        .trim_start_matches("http://")
        .split('/')
        .next()
        .unwrap_or(url)
}

fn sentence_text(value: &str) -> String {
    let mut text = normalize_space(value);
    if text.is_empty() {
        return text;
    }
    if !matches!(text.as_bytes().last(), Some(b'.' | b'!' | b'?')) {
        text.push('.');
    }
    text
}

fn normalize_space(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn short_text(value: &str, limit: usize) -> String {
    let value = normalize_space(value);
    if value.chars().count() <= limit {
        return value;
    }
    let mut text = String::new();
    for word in value.split_whitespace() {
        let extra = if text.is_empty() { 0 } else { 1 };
        if text.chars().count() + word.chars().count() + extra > limit.saturating_sub(1) {
            break;
        }
        if !text.is_empty() {
            text.push(' ');
        }
        text.push_str(word);
    }
    if text.is_empty() {
        value
            .chars()
            .take(limit.saturating_sub(1))
            .collect::<String>()
            + "…"
    } else {
        text + "…"
    }
}

fn hero_sentence(package: &PackageRow) -> String {
    let summary = normalize_space(&package.summary);
    if !summary.is_empty() && !package.install_command.is_empty() {
        let verified = first_non_empty(&[
            full_str(package, "lastVerified"),
            package.last_updated_at.clone(),
        ])
        .map(|value| fmt_date(&value))
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "from local package data".to_string());
        let alternate = alternate_install_sentence(package);
        let alternate = if alternate.is_empty() {
            String::new()
        } else {
            format!(" {alternate}")
        };
        return format!(
            "{} Version {} via {}; verified {}.{}",
            sentence_text(&summary),
            empty_as_unknown(&package.version),
            package.provider_label,
            verified,
            alternate
        );
    }
    if let Some(isotope) = full_value(package, "isotope").filter(|value| value.is_object()) {
        let title = isotope
            .get("justification")
            .and_then(|justification| value_str_key(justification, "title"))
            .unwrap_or_else(|| "secret handling".to_string());
        return format!(
            "Automic Vault tracks {} because {} affects agent-run command-line tools on macOS.",
            package.display_name,
            title.trim_end_matches('.').to_ascii_lowercase()
        );
    }
    if full_value(package, "approvalGate").is_some() {
        return format!(
            "Automic Vault has approval-gate metadata for {}, including high-risk commands and recommended human review points.",
            package.display_name
        );
    }
    if !summary.is_empty() {
        return format!("Nucleus can resolve {}: {}", package.display_name, summary);
    }
    format!(
        "Nucleus package metadata for {}, from local Automic Vault package sources.",
        package.display_name
    )
}

fn localized_hero_sentence(package: &PackageRow, locale: &Locale) -> String {
    if locale.code == "en" {
        hero_sentence(package)
    } else {
        txf(
            locale,
            "heroSentence",
            "View install routes, executables, metadata, and security notes for {name}.",
            &[("name", package.display_name.clone())],
        )
    }
}

fn meta_description(package: &PackageRow) -> String {
    let mut parts = Vec::new();
    if let Some(alternate) = alternate_install_command(package) {
        let manager = value_str_key(&alternate, "manager")
            .unwrap_or_else(|| "another package manager".to_string());
        let command = value_str_key(&alternate, "command").unwrap_or_default();
        parts.push(format!(
            "Install {} with {} or {}: {}.",
            package.display_name, package.provider_label, manager, command
        ));
    } else {
        parts.push(format!(
            "Install {} with {}.",
            package.display_name, package.provider_label
        ));
    }
    if !package.summary.is_empty() {
        parts.push(package.summary.clone());
    }
    if !full_string_array(package, "executablesDetailed").is_empty()
        || !full_string_array(package, "aliases").is_empty()
    {
        parts.push("View executables, metadata, and security notes.".to_string());
    }
    if let Some(isotope) = full_value(package, "isotope").filter(|value| value.is_object()) {
        if let Some(title) = isotope
            .get("justification")
            .and_then(|justification| value_str_key(justification, "title"))
        {
            parts.push(format!("Protected-tool coverage: {title}."));
        }
    }
    if let Some(gate) = full_value(package, "approvalGate").filter(|value| value.is_object()) {
        if let Some(rule_count) = value_i64_key(gate, "rule_count") {
            parts.push(format!("Includes {rule_count} approval-gate rules."));
        }
    }
    short_text(&parts.join(" "), 155)
}

fn alternate_install_command(package: &PackageRow) -> Option<Value> {
    install_command_entries(package).into_iter().find(|item| {
        if value_str_key(item, "kind").as_deref() == Some("automic_vault") {
            return false;
        }
        let command = value_str_key(item, "command").unwrap_or_default();
        native_command_provider(&command)
            .map(|provider| provider != package.provider)
            .unwrap_or(false)
    })
}

fn native_command_provider(command: &str) -> Option<&'static str> {
    let command = command.trim();
    if command.starts_with("brew install --cask ") {
        Some("cask")
    } else if command.starts_with("brew install ") {
        Some("brew")
    } else if command.starts_with("npm install ") || command.starts_with("npm i ") {
        Some("npm")
    } else if command.starts_with("pip install ")
        || command.starts_with("pip3 install ")
        || command.starts_with("python -m pip install ")
        || command.starts_with("python3 -m pip install ")
    {
        Some("pip")
    } else {
        None
    }
}

fn alternate_install_sentence(package: &PackageRow) -> String {
    let Some(alternate) = alternate_install_command(package) else {
        return String::new();
    };
    let manager = value_str_key(&alternate, "manager")
        .unwrap_or_else(|| "another package manager".to_string());
    let command = value_str_key(&alternate, "command").unwrap_or_default();
    if command.is_empty() {
        String::new()
    } else {
        format!("Also installable with {manager}: {command}.")
    }
}

fn install_command_entries(package: &PackageRow) -> Vec<Value> {
    if let Some(items) = full_value(package, "installCommands")
        .and_then(Value::as_array)
        .filter(|items| !items.is_empty())
    {
        return items.iter().cloned().collect();
    }
    let mut entries = vec![json!({
        "platform": "portable",
        "manager": "Automic Vault",
        "command": if package.install_command.is_empty() {
            format!("sudo av install {}", package.package_key)
        } else {
            package.install_command.clone()
        },
        "kind": "automic_vault",
        "confidence": 1.0,
        "evidence": "deterministic local package key"
    })];
    if !package.native_install_command.is_empty() {
        entries.push(json!({
            "platform": if package.provider == "brew" || package.provider == "cask" { "macos" } else { "portable" },
            "manager": package.provider_label,
            "command": package.native_install_command,
            "kind": "package_manager",
            "confidence": 1.0,
            "evidence": "provider-native install command"
        }));
    }
    entries
}

fn package_facts(package: &PackageRow, locale: &Locale) -> String {
    let mut facts = vec![metric(
        &tx(locale, "manager", "manager"),
        &package.provider_label,
    )];
    if !package.version.is_empty() {
        facts.push(metric(&tx(locale, "version", "version"), &package.version));
    }
    if !package.license.is_empty() {
        facts.push(metric(&tx(locale, "license", "license"), &package.license));
    }
    if let Some(geiger) = full_value(package, "geiger").filter(|value| value.is_object()) {
        facts.push(metric(
            &tx(locale, "risk", "risk"),
            &value_str_key(geiger, "level").unwrap_or_else(|| "unknown".to_string()),
        ));
        facts.push(metric(
            &tx(locale, "classifierConfidence", "classifier confidence"),
            &value_str_key(geiger, "confidence").unwrap_or_else(|| "unknown".to_string()),
        ));
    }
    if let Some(rank) = full_value(package, "popularity")
        .and_then(|value| value_i64_key(value, "rank"))
        .or_else(|| package.rank.map(i64::from))
    {
        facts.push(metric(&tx(locale, "rank", "rank"), &fmt_int(rank)));
    }
    if let Some(popularity) = full_value(package, "popularity").filter(|value| value.is_object()) {
        if let Some(installs) = value_i64_key(popularity, "installs_per_365_days") {
            facts.push(metric(
                &tx(locale, "installs365d", "365d installs"),
                &fmt_int(installs),
            ));
        } else if let Some(downloads) = value_i64_key(popularity, "downloads_per_30_days") {
            facts.push(metric(
                &tx(locale, "downloads30d", "30d downloads"),
                &fmt_int(downloads),
            ));
        }
    }
    if full_value(package, "isotope").is_some() {
        facts.push(metric(
            &tx(locale, "radioisotopeKicker", "protected-tool coverage"),
            &tx(locale, "covered", "covered"),
        ));
    }
    if let Some(gate) = full_value(package, "approvalGate").filter(|value| value.is_object()) {
        if let Some(rule_count) = value_i64_key(gate, "rule_count") {
            facts.push(metric(
                &tx(locale, "approvalRules", "approval rules"),
                &fmt_int(rule_count),
            ));
        }
    }
    if let Some(verified) = full_opt_str(package, "lastVerified").filter(|value| !value.is_empty())
    {
        facts.push(metric(
            &tx(locale, "verified", "verified"),
            &fmt_date(&verified),
        ));
    } else if !package.last_updated_at.is_empty() {
        facts.push(metric(
            &tx(locale, "updated", "updated"),
            &fmt_date(&package.last_updated_at),
        ));
    }
    facts.join("")
}

fn metric(label: &str, value: &str) -> String {
    format!(
        r#"<div class="metric"><span>{}</span><strong>{}</strong></div>"#,
        html_escape(label),
        html_escape(value)
    )
}

fn security_heading(package: &PackageRow, locale: &Locale) -> String {
    if let Some(geiger) = full_value(package, "geiger").filter(|value| value.is_object()) {
        return txf(
            locale,
            "riskLevel",
            "Risk level: {level}",
            &[(
                "level",
                value_str_key(geiger, "level").unwrap_or_else(|| "unknown".to_string()),
            )],
        );
    }
    tx(
        locale,
        "radioisotopeMissingHeading",
        "No protected-tool coverage found yet",
    )
}

fn security_summary(package: &PackageRow, locale: &Locale) -> String {
    if let Some(geiger) = full_value(package, "geiger").filter(|value| value.is_object()) {
        let reasons = value_array(geiger, "reasons")
            .into_iter()
            .take(2)
            .filter_map(value_string)
            .map(|reason| sentence_text(reason.trim_end_matches('.')))
            .collect::<Vec<_>>();
        if !reasons.is_empty() {
            return reasons.join(" ");
        }
    }
    txf(
        locale,
        "radioisotopeMissingSummary",
        "No matching local secret-handling manifest was found for {name}. Nucleus package metadata is still published here so future coverage has a stable package URL.",
        &[("name", package.display_name.clone())],
    )
}

fn link_value(value: &str) -> String {
    if value.starts_with("https://") || value.starts_with("http://") {
        format!(
            r#"<a href="{}">{}</a>"#,
            html_escape(value),
            html_escape(value)
        )
    } else {
        html_escape(value)
    }
}

fn push_joined_row(rows: &mut Vec<(String, String)>, label: &str, values: &[String]) {
    if !values.is_empty() {
        rows.push((label.to_string(), values.join(", ")));
    }
}

fn related_links(
    package: &PackageRow,
    locale: &Locale,
    key: &str,
    limit: usize,
    workflow_only: bool,
) -> Vec<String> {
    let workflow_rels = [
        "adjacent_workflow",
        "format_peer",
        "language_runtime_peer",
        "command_surface_peer",
        "security_surface_peer",
        "domain_peer",
    ];
    let mut links = full_value(package, key)
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter(|item| item.is_object())
                .filter_map(|item| {
                    let rel = value_str_key(item, "rel").unwrap_or_default();
                    let is_workflow = workflow_rels.contains(&rel.as_str());
                    if workflow_only != is_workflow && key == "relatedPackages" {
                        return None;
                    }
                    let provider = value_str_key(item, "provider")?;
                    let name = value_str_key(item, "name")
                        .or_else(|| value_str_key(item, "package"))
                        .or_else(|| value_str_key(item, "target"))?;
                    if provider.is_empty()
                        || name.is_empty()
                        || format!("{provider}:{name}") == package.package_key
                    {
                        return None;
                    }
                    let label = value_str_key(item, "label").unwrap_or_else(|| name.clone());
                    let reason = value_str_key(item, "reason").unwrap_or_default();
                    let href =
                        locale_path(&format!("/pkg/{}/{}/", provider, slugify(&name)), locale);
                    Some(format!(
                        r#"<li><a href="{}">{}</a>{}</li>"#,
                        html_escape(&href),
                        html_escape(&label),
                        if reason.is_empty() {
                            String::new()
                        } else {
                            format!("<span>{}</span>", html_escape(&reason))
                        }
                    ))
                })
                .take(limit)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if links.is_empty() && key == "relatedPackages" && !workflow_only && package.provider == "brew"
    {
        links = full_string_array(package, "dependencies")
            .into_iter()
            .take(limit)
            .map(|dependency| {
                let href = locale_path(&format!("/pkg/brew/{}/", slugify(&dependency)), locale);
                format!(
                    r#"<li><a href="{}">{}</a><span>{} dependency.</span></li>"#,
                    html_escape(&href),
                    html_escape(&dependency),
                    html_escape(&package.provider_label)
                )
            })
            .collect();
    }
    links
}

fn related_article(title: &str, items: Vec<String>) -> String {
    let content = items
        .into_iter()
        .filter(|item| !item.is_empty())
        .collect::<String>();
    if content.is_empty() {
        String::new()
    } else {
        format!(
            "<article><h3>{}</h3><ul>{content}</ul></article>",
            html_escape(title)
        )
    }
}

fn core_security_guides(package: &PackageRow, locale: &Locale) -> Vec<String> {
    let mut links = vec![
        (
            locale_path("/secret-scanner-for-ai-agents/", locale),
            "AI agent secret scanner",
            "Find plaintext credentials before an agent run starts.",
        ),
        (
            locale_path("/ai-agent-approval-gates/", locale),
            "AI agent approval gates",
            "Put approvals in front of risky package and tool actions.",
        ),
        (
            locale_path("/docs/#secrets", locale),
            "Secret injection docs",
            "Move supported secrets out of plaintext files and inject them into approved tools.",
        ),
    ];
    let haystack = format!(
        "{} {} {} {} {}",
        package.provider,
        package.name,
        package.display_name,
        package.summary,
        full_string_array(package, "aliases").join(" ")
    )
    .to_ascii_lowercase();
    if package.provider == "brew" || haystack.contains("homebrew") {
        links.push((
            locale_path("/download/", locale),
            "Secure Homebrew tools",
            "Install Vault and scan the tools your Mac already uses.",
        ));
    }
    if haystack.contains("aws") || haystack.contains("cloud") {
        links.push((
            locale_path("/secure-aws-cli-credentials-ai-agents/", locale),
            "Secure AWS CLI credentials",
            "Keep cloud keys out of ambient config files.",
        ));
    }
    if haystack.contains("github")
        || full_string_array(package, "aliases")
            .iter()
            .any(|alias| alias == "gh")
    {
        links.push((
            locale_path("/github-cli-token-security-ai-agents/", locale),
            "GitHub CLI token security",
            "Protect source and release tokens used by local tools.",
        ));
    }
    links
        .into_iter()
        .take(5)
        .map(|(url, label, copy)| {
            format!(
                r#"<li><a href="{}">{}</a><span>{}</span></li>"#,
                html_escape(&url),
                html_escape(label),
                html_escape(copy)
            )
        })
        .collect()
}

fn slugify(value: &str) -> String {
    let mut slug = String::new();
    let mut last_dash = false;
    for character in value.chars().flat_map(char::to_lowercase) {
        if character.is_ascii_alphanumeric() {
            slug.push(character);
            last_dash = false;
        } else if !last_dash && !slug.is_empty() {
            slug.push('-');
            last_dash = true;
        }
    }
    while slug.ends_with('-') {
        slug.pop();
    }
    slug
}

fn schema_for_package(
    package: &PackageRow,
    description: &str,
    updated: &str,
    locale: &Locale,
) -> Value {
    let url = locale_url(&package.path, locale);
    let mut software = json!({
        "@type": "SoftwareApplication",
        "@id": format!("{url}#software"),
        "name": package.display_name,
        "applicationCategory": "DeveloperApplication",
        "operatingSystem": "macOS",
        "url": url,
        "description": description,
        "dateModified": updated,
        "inLanguage": locale.hreflang,
        "isPartOf": {"@id": format!("{SITE_ORIGIN}/#website")}
    });
    if let Some(object) = software.as_object_mut() {
        if !package.homepage.is_empty() {
            object.insert("sameAs".to_string(), json!(package.homepage));
        }
        if !package.version.is_empty() {
            object.insert("softwareVersion".to_string(), json!(package.version));
        }
        if !package.license.is_empty() {
            object.insert("license".to_string(), json!(package.license));
        }
        if !package.repository.is_empty() {
            object.insert("codeRepository".to_string(), json!(package.repository));
        }
        let dependencies = full_string_array(package, "dependencies");
        if !dependencies.is_empty() {
            object.insert(
                "softwareRequirements".to_string(),
                json!(
                    dependencies
                        .into_iter()
                        .take(16)
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            );
        }
    }
    let steps = install_command_entries(package)
        .into_iter()
        .filter_map(|item| {
            let command = value_str_key(&item, "command")?;
            if command.is_empty() {
                return None;
            }
            let manager = value_str_key(&item, "manager").unwrap_or_else(|| "install".to_string());
            Some((manager, command))
        })
        .take(12)
        .enumerate()
        .map(|(index, (manager, command))| {
            json!({
                "@type": "HowToStep",
                "position": index + 1,
                "name": format!("Run {manager} command"),
                "text": command
            })
        })
        .collect::<Vec<_>>();
    json!({
        "@context": "https://schema.org",
        "@graph": [
            {"@type": "WebSite", "@id": format!("{SITE_ORIGIN}/#website"), "name": "Automic Vault", "url": format!("{SITE_ORIGIN}/")},
            {"@type": "Organization", "@id": format!("{SITE_ORIGIN}/#organization"), "name": "Automic Vault", "url": format!("{SITE_ORIGIN}/")},
            {"@type": "Person", "@id": format!("{SITE_ORIGIN}/about/#max-howell"), "name": "Max Howell", "url": format!("{SITE_ORIGIN}/about/")},
            software,
            {
                "@type": "TechArticle",
                "@id": format!("{url}#article"),
                "headline": format!("Install {} with {}", package.display_name, package.provider_label),
                "description": description,
                "dateModified": updated,
                "inLanguage": locale.hreflang,
                "author": {"@id": format!("{SITE_ORIGIN}/about/#max-howell")},
                "reviewedBy": {"@id": format!("{SITE_ORIGIN}/about/#max-howell")},
                "publisher": {"@id": format!("{SITE_ORIGIN}/#organization")},
                "mainEntity": {"@id": format!("{url}#software")}
            },
            {
                "@type": "BreadcrumbList",
                "@id": format!("{url}#breadcrumbs"),
                "itemListElement": [
                    {"@type": "ListItem", "position": 1, "name": "Home", "item": locale_url("/", locale)},
                    {"@type": "ListItem", "position": 2, "name": "Packages", "item": locale_url("/pkg/", locale)},
                    {"@type": "ListItem", "position": 3, "name": package.display_name, "item": url}
                ]
            },
            {
                "@type": "HowTo",
                "@id": format!("{url}#install-howto"),
                "name": format!("Install {}", package.display_name),
                "step": steps
            }
        ]
    })
}

fn markdown_install_groups(text: &mut String, package: &PackageRow) {
    let commands = install_command_entries(package);
    if commands.len() <= 1 {
        return;
    }
    text.push_str("Additional install commands:\n\n");
    for (platform, label) in [
        ("macos", "macOS"),
        ("linux", "Linux"),
        ("windows", "Windows"),
        ("portable", "Portable and language managers"),
    ] {
        let items = commands
            .iter()
            .skip(1)
            .filter(|item| {
                value_str_key(item, "platform").unwrap_or_else(|| "portable".to_string())
                    == platform
            })
            .collect::<Vec<_>>();
        if items.is_empty() {
            continue;
        }
        text.push_str(&format!("### {label}\n\n"));
        for item in items {
            let manager = value_str_key(item, "manager").unwrap_or_else(|| "shell".to_string());
            let command = value_str_key(item, "command").unwrap_or_default();
            if command.is_empty() {
                continue;
            }
            let confidence = value_f64_key(item, "confidence")
                .map(|value| format!("{:.0}%", value * 100.0))
                .unwrap_or_else(|| "unknown confidence".to_string());
            text.push_str(&format!(
                "- {manager} ({confidence}):\n\n```sh\n{command}\n```\n"
            ));
            if let Some(evidence) = value_str_key(item, "evidence") {
                text.push_str(&format!("\n  Evidence: {evidence}\n"));
            }
            text.push('\n');
        }
    }
}

fn markdown_value_list(text: &mut String, title: &str, items: &[String]) {
    if items.is_empty() {
        return;
    }
    text.push_str(&format!("\n## {title}\n\n"));
    for item in items {
        text.push_str(&format!("- {}\n", markdown_value(item)));
    }
}

fn markdown_value(value: &str) -> String {
    if value.starts_with("https://") || value.starts_with("http://") {
        format!("<{value}>")
    } else {
        value.to_string()
    }
}

fn executable_markdown_items(package: &PackageRow) -> Vec<String> {
    let mut items = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for item in value_array(&package.data.full, "executablesDetailed") {
        let name = value_str_key(item, "name")
            .or_else(|| value_str_key(item, "target"))
            .or_else(|| value_str_key(item, "source"))
            .unwrap_or_default();
        if name.is_empty() || !seen.insert(name.clone()) {
            continue;
        }
        let kind = value_str_key(item, "kind").unwrap_or_else(|| "executable".to_string());
        let note = value_str_key(item, "note").unwrap_or_default();
        items.push(if note.is_empty() {
            format!("{name} ({kind})")
        } else {
            format!("{name} ({kind}): {note}")
        });
    }
    for item in value_array(&package.data.full, "binaries") {
        let name = value_str_key(item, "target")
            .or_else(|| value_str_key(item, "source"))
            .unwrap_or_default();
        if !name.is_empty() {
            items.push(format!("{name} (binary)"));
        }
    }
    for alias in full_string_array(package, "aliases") {
        items.push(format!("{alias} (alias)"));
    }
    items
}

fn markdown_install_behavior_items(package: &PackageRow) -> Vec<String> {
    let mut items = Vec::new();
    if let Some(behavior) = full_value(package, "installBehavior").filter(|value| value.is_object())
    {
        if let Some(post_install) = behavior.get("postInstallDefined").and_then(Value::as_bool) {
            items.push(format!(
                "Post-install hook: {}",
                if post_install {
                    "defined"
                } else {
                    "not defined"
                }
            ));
        }
        if let Some(service) = value_str_key(behavior, "service").filter(|value| !value.is_empty())
        {
            items.push(format!("Service: {service}"));
        }
        if let Some(caveats) = value_str_key(behavior, "caveats").filter(|value| !value.is_empty())
        {
            items.push(format!("Caveats: {caveats}"));
        }
        let lifecycle = value_array(behavior, "lifecycleScripts")
            .into_iter()
            .filter_map(value_string)
            .collect::<Vec<_>>();
        if !lifecycle.is_empty() {
            items.push(format!("Lifecycle scripts: {}", lifecycle.join(", ")));
        }
        if let Some(python) =
            value_str_key(behavior, "pythonRequires").filter(|value| !value.is_empty())
        {
            items.push(format!("Python requires: {python}"));
        }
        if let Some(count) = value_i64_key(behavior, "requiresDistCount") {
            items.push(format!("PyPI dependency specs: {count}"));
        }
    }
    if let Some(bottle) = full_value(package, "bottle").filter(|value| value.is_object()) {
        let mut detail = if bottle.get("available").and_then(Value::as_bool) == Some(true) {
            "available".to_string()
        } else {
            "not available".to_string()
        };
        let platforms = value_array(bottle, "platforms")
            .into_iter()
            .filter_map(value_string)
            .take(12)
            .collect::<Vec<_>>();
        if !platforms.is_empty() {
            detail.push_str(&format!(" on {}", platforms.join(", ")));
        }
        items.push(format!("Bottle: {detail}"));
    }
    items
}

fn markdown_freshness_items(package: &PackageRow, generated_at: &str) -> Vec<String> {
    let freshness = full_value(package, "versionFreshness").unwrap_or(&Value::Null);
    let manager = freshness.get("packageManager").unwrap_or(&Value::Null);
    let site = freshness.get("siteData").unwrap_or(&Value::Null);
    let upstream = freshness.get("upstream").unwrap_or(&Value::Null);
    let mut items = vec![
        format!(
            "Page generated: {}",
            non_empty(&fmt_date(generated_at), "unknown")
        ),
        format!(
            "Package-manager version: {}",
            value_str_key(manager, "version")
                .unwrap_or_else(|| empty_as_unknown(&package.version).to_string())
        ),
    ];
    if let Some(updated) = value_str_key(manager, "updatedAt") {
        items.push(format!("Package-manager updated: {}", fmt_date(&updated)));
    }
    if let Some(status) = value_str_key(site, "status") {
        items.push(format!("Local data status: {status}"));
    }
    if let Some(repository) = value_str_key(upstream, "repository") {
        items.push(format!("Upstream repository: {repository}"));
    }
    if let Some(version) = value_str_key(upstream, "latestVersion") {
        items.push(format!(
            "Upstream latest detected: {} ({})",
            version,
            value_str_key(upstream, "comparison").unwrap_or_else(|| "unknown".to_string())
        ));
    }
    for item in value_array(freshness, "warnings").into_iter().take(8) {
        if item.is_object() {
            let severity = value_str_key(item, "severity").unwrap_or_else(|| "info".to_string());
            let message = value_str_key(item, "message").unwrap_or_default();
            if !message.is_empty() {
                items.push(format!("{severity}: {message}"));
            }
        }
    }
    items
}

fn markdown_security_section(text: &mut String, package: &PackageRow, locale: &Locale) {
    text.push_str(&format!(
        "\n## {}\n\n",
        tx(locale, "securityNotes", "Security Notes")
    ));
    text.push_str(&security_summary(package, locale));
    text.push_str("\n\n");
    if let Some(isotope) = full_value(package, "isotope").filter(|value| value.is_object()) {
        if let Some(title) = isotope
            .get("justification")
            .and_then(|justification| value_str_key(justification, "title"))
        {
            text.push_str(&format!("- **Protected-tool coverage:** {title}\n"));
        }
    }
    if let Some(geiger) = full_value(package, "geiger").filter(|value| value.is_object()) {
        text.push_str(&format!(
            "- **Geiger risk:** {} / {}\n",
            value_str_key(geiger, "level").unwrap_or_else(|| "unknown".to_string()),
            value_str_key(geiger, "confidence").unwrap_or_else(|| "unknown".to_string())
        ));
        for item in value_array(geiger, "reasons")
            .into_iter()
            .filter_map(value_string)
        {
            text.push_str(&format!("- {item}\n"));
        }
    }
    if let Some(gate) = full_value(package, "approvalGate").filter(|value| value.is_object()) {
        if let Some(rule_count) = value_i64_key(gate, "rule_count") {
            text.push_str(&format!("- **Approval gate rules:** {rule_count}\n"));
        }
    }
    text.push('\n');
}

fn markdown_related(text: &mut String, package: &PackageRow, locale: &Locale) {
    let mut items = Vec::new();
    if let Some(hubs) = full_value(package, "packageHubs").and_then(Value::as_array) {
        for item in hubs.iter().take(4) {
            let slug = value_str_key(item, "slug").unwrap_or_default();
            if slug.is_empty() {
                continue;
            }
            let label = value_str_key(item, "label").unwrap_or_else(|| slug.clone());
            let reason = value_str_key(item, "reason").unwrap_or_default();
            let url = locale_url(&format!("/pkg/{slug}/"), locale);
            items.push(markdown_link_item(&label, &url, &reason));
        }
    }
    for key in ["relatedPackages", "alsoAvailableVia"] {
        if let Some(values) = full_value(package, key).and_then(Value::as_array) {
            for item in values.iter().take(24) {
                let label = value_str_key(item, "label")
                    .or_else(|| value_str_key(item, "name"))
                    .unwrap_or_default();
                let provider = value_str_key(item, "provider").unwrap_or_default();
                let name = value_str_key(item, "name").unwrap_or_else(|| label.clone());
                let reason = value_str_key(item, "reason").unwrap_or_default();
                if label.is_empty() || provider.is_empty() || name.is_empty() {
                    continue;
                }
                let url = locale_url(&format!("/pkg/{}/{}/", provider, slugify(&name)), locale);
                items.push(markdown_link_item(&label, &url, &reason));
            }
        }
    }
    markdown_value_list(text, "Related Links", &items);
}

fn markdown_link_item(label: &str, url: &str, reason: &str) -> String {
    if reason.is_empty() {
        format!("[{label}]({url})")
    } else {
        format!("[{label}]({url}) - {reason}")
    }
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
