use rusqlite::{Connection, OpenFlags, OptionalExtension, params};
use serde::Serialize;
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

    match response_for_path(&state.db_path, &request.path)? {
        Some(response) => write_response(
            &mut stream,
            &request.method,
            200,
            "OK",
            response.headers(),
            response.body,
        ),
        None => write_response(
            &mut stream,
            &request.method,
            404,
            "Not Found",
            vec![
                ("content-type", "text/plain; charset=utf-8".to_string()),
                ("cache-control", HTML_CACHE_CONTROL.to_string()),
            ],
            b"not found\n".to_vec(),
        ),
    }
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
        let response = response_for_path(db.path(), "/pkg/brew/awscli/index.md")
            .expect("query")
            .expect("markdown response");

        assert_eq!(response.content_type, "text/markdown; charset=utf-8");
        assert_eq!(response.body, b"# awscli\n");
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
