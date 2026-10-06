mod api;
use axum::{
    Json, Router,
    body::Body,
    extract::Request,
    extract::{DefaultBodyLimit, Path, Query, State},
    http::{HeaderMap, StatusCode, header},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use homer_core::runtime::AppHandle;
use http_body_util::BodyExt;
use serde::Deserialize;
use serde_json::{Value, json};
use std::{path::PathBuf, sync::Arc};
use tokio::io::{AsyncSeekExt, AsyncWriteExt};
use tower_http::services::{ServeDir, ServeFile};
#[derive(Clone)]
struct Server {
    app: AppHandle,
    token: Arc<String>,
    limit: u64,
}
fn authorized(headers: &HeaderMap, token: &str) -> bool {
    let bearer = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "));
    let cookie = headers
        .get(header::COOKIE)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| {
            s.split(';')
                .find_map(|p| p.trim().strip_prefix("homer_session="))
        });
    let candidate = bearer.or(cookie).unwrap_or("");
    let a = candidate.as_bytes();
    let b = token.as_bytes();
    let mut difference = a.len() ^ b.len();
    for i in 0..b.len() {
        difference |= (a.get(i).copied().unwrap_or(0) ^ b[i]) as usize;
    }
    difference == 0
}
fn same_origin(headers: &HeaderMap) -> bool {
    match headers.get(header::ORIGIN).and_then(|v| v.to_str().ok()) {
        None => true,
        Some(origin) => {
            let host = headers
                .get(header::HOST)
                .and_then(|v| v.to_str().ok())
                .unwrap_or("");
            origin == format!("http://{host}") || origin == format!("https://{host}")
        }
    }
}
async fn guard(State(server): State<Server>, request: Request, next: Next) -> Response {
    if !same_origin(request.headers()) {
        return (StatusCode::FORBIDDEN, "Use this server's own browser page.").into_response();
    }
    if !authorized(request.headers(), &server.token) {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({"message":"Sign in to Homer Studio."})),
        )
            .into_response();
    }
    next.run(request).await
}
async fn session(
    State(server): State<Server>,
    headers: HeaderMap,
    Json(input): Json<Value>,
) -> Response {
    if !same_origin(&headers) {
        return StatusCode::FORBIDDEN.into_response();
    }
    let supplied = input.get("token").and_then(Value::as_str).unwrap_or("");
    let mut auth = HeaderMap::new();
    if let Ok(h) = format!("Bearer {supplied}").parse() {
        auth.insert(header::AUTHORIZATION, h);
    }
    if !authorized(&auth, &server.token) {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    let secure = if std::env::var("HOMER_SECURE_COOKIE").as_deref() == Ok("1") {
        "; Secure"
    } else {
        ""
    };
    (
        [(
            header::SET_COOKIE,
            format!(
                "homer_session={}; HttpOnly; SameSite=Strict; Path=/; Max-Age=86400{secure}",
                server.token
            ),
        )],
        Json(json!({"ready":true})),
    )
        .into_response()
}
fn safe_path(app: &AppHandle, value: &str) -> bool {
    let p = std::path::Path::new(value);
    if !p.is_absolute() {
        return false;
    }
    let mut ancestor = p;
    while !ancestor.exists() {
        let Some(parent) = ancestor.parent() else {
            return false;
        };
        ancestor = parent;
    }
    let Ok(real) = ancestor.canonicalize() else {
        return false;
    };
    !p.components()
        .any(|c| matches!(c, std::path::Component::ParentDir))
        && [&app.data, &app.resources]
            .iter()
            .any(|root| root.canonicalize().is_ok_and(|r| real.starts_with(r)))
}
fn validate_paths(app: &AppHandle, args: &Value) -> bool {
    if let Value::Object(fields) = args {
        for (key, value) in fields {
            if matches!(
                key.as_str(),
                "path"
                    | "sourcePath"
                    | "rootPath"
                    | "parentPath"
                    | "destination"
                    | "destinationDir"
                    | "outputPath"
            ) {
                if let Some(path) = value.as_str() {
                    if !safe_path(app, path) {
                        return false;
                    }
                }
            }
        }
    }
    true
}
async fn command(
    State(server): State<Server>,
    Path(name): Path<String>,
    Json(args): Json<Value>,
) -> Response {
    if name == "desktop_info" {
        return Json(json!({"platform":std::env::consts::OS,"architecture":std::env::consts::ARCH,"runtime":"Homer Server"})).into_response();
    }
    if !validate_paths(&server.app, &args) {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"code":"UNSAFE_PATH","message":"Choose a file uploaded to this server."})),
        )
            .into_response();
    }
    let handle = tokio::runtime::Handle::current();
    let result = tokio::task::spawn_blocking(move || {
        handle.block_on(api::dispatch(server.app, &name, args))
    })
    .await;
    match result {
        Ok(Ok(mut value)) => {
            media_urls(&mut value);
            Json(value).into_response()
        }
        Ok(Err(e)) => (StatusCode::BAD_REQUEST, Json(e)).into_response(),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "The operation stopped unexpectedly.",
        )
            .into_response(),
    }
}
fn media_urls(v: &mut Value) {
    match v {
        Value::String(s) => {
            if let Some(id) = s
                .strip_prefix("audio://localhost/")
                .or_else(|| s.strip_prefix("http://audio.localhost/"))
            {
                *s = format!("/api/media/{id}")
            }
        }
        Value::Array(items) => {
            for i in items {
                media_urls(i)
            }
        }
        Value::Object(fields) => {
            for i in fields.values_mut() {
                media_urls(i)
            }
        }
        _ => (),
    }
}
#[derive(Deserialize)]
struct FileName {
    name: String,
}
fn filename(name: &str) -> String {
    std::path::Path::new(name)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("audio.webm")
        .chars()
        .filter(|c| c.is_alphanumeric() || "._- ".contains(*c))
        .take(180)
        .collect()
}
async fn upload(
    State(server): State<Server>,
    Query(input): Query<FileName>,
    mut body: Body,
) -> Response {
    let name = filename(&input.name);
    if name.is_empty() {
        return StatusCode::BAD_REQUEST.into_response();
    }
    let path = server
        .app
        .data
        .join("uploads")
        .join(format!("{}-{name}", uuid::Uuid::new_v4()));
    let Ok(mut file) = tokio::fs::File::create(&path).await else {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    };
    let mut total = 0u64;
    let mut failed = false;
    while let Some(frame) = body.frame().await {
        match frame {
            Ok(frame) => {
                if let Ok(data) = frame.into_data() {
                    total += data.len() as u64;
                    if total > server.limit || file.write_all(&data).await.is_err() {
                        failed = true;
                        break;
                    }
                }
            }
            Err(_) => {
                failed = true;
                break;
            }
        }
    }
    if failed || total == 0 {
        drop(file);
        let _ = tokio::fs::remove_file(path).await;
        return (
            StatusCode::BAD_REQUEST,
            "Upload stopped or exceeds the server's file limit.",
        )
            .into_response();
    }
    if file.sync_all().await.is_err() {
        drop(file);
        let _ = tokio::fs::remove_file(path).await;
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }
    Json(json!({"path":path.to_string_lossy()})).into_response()
}
// Each short recorder chunk is appended under an exclusive process-wide lock.
// Files stay on disk; neither browser nor server retains the full recording.
static RECORDING_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
async fn recording_start(State(server): State<Server>, Json(input): Json<FileName>) -> Response {
    let name = filename(&input.name);
    if !matches!(
        std::path::Path::new(&name)
            .extension()
            .and_then(|s| s.to_str()),
        Some("webm" | "m4a")
    ) {
        return StatusCode::BAD_REQUEST.into_response();
    }
    let id = uuid::Uuid::new_v4().to_string();
    let path = server.app.data.join("uploads").join(format!("{id}-{name}"));
    if tokio::fs::File::create(&path).await.is_err() {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }
    Json(json!({"path":path.to_string_lossy()})).into_response()
}
async fn recording_append(
    State(server): State<Server>,
    Query(input): Query<Download>,
    body: axum::body::Bytes,
) -> Response {
    let _guard = RECORDING_LOCK.lock().await;
    let path = PathBuf::from(&input.path);
    if !safe_path(&server.app, &input.path)
        || path.parent() != Some(server.app.data.join("uploads").as_path())
        || body.is_empty()
        || body.len() > 8 * 1024 * 1024
    {
        return StatusCode::BAD_REQUEST.into_response();
    }
    let Ok(meta) = tokio::fs::metadata(&path).await else {
        return StatusCode::NOT_FOUND.into_response();
    };
    if meta.len() + body.len() as u64 > server.limit {
        return StatusCode::PAYLOAD_TOO_LARGE.into_response();
    }
    let Ok(mut file) = tokio::fs::OpenOptions::new().append(true).open(path).await else {
        return StatusCode::NOT_FOUND.into_response();
    };
    if file.write_all(&body).await.is_err() || file.sync_all().await.is_err() {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }
    StatusCode::NO_CONTENT.into_response()
}
async fn destination(State(server): State<Server>, Json(input): Json<Value>) -> Response {
    let name = filename(
        input
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or("export.m4a"),
    );
    let dir = server
        .app
        .data
        .join("downloads")
        .join(uuid::Uuid::new_v4().to_string());
    if tokio::fs::create_dir(&dir).await.is_err() {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }
    Json(json!({"path":dir.join(name).to_string_lossy()})).into_response()
}
async fn projects(State(server): State<Server>) -> Response {
    let root = server.app.data.join("projects");
    let mut entries = Vec::new();
    if let Ok(mut directories) = tokio::fs::read_dir(root).await {
        while let Ok(Some(entry)) = directories.next_entry().await {
            if entries.len() >= 500 {
                break;
            }
            let path = entry.path();
            if path.join("project.json").is_file()
                && safe_path(&server.app, &path.to_string_lossy())
            {
                entries.push(json!({"name":entry.file_name().to_string_lossy(),"path":path.to_string_lossy()}));
            }
        }
    }
    Json(entries).into_response()
}
async fn download(State(server): State<Server>, Query(input): Query<Download>) -> Response {
    if !safe_path(&server.app, &input.path) {
        return StatusCode::FORBIDDEN.into_response();
    }
    let path = PathBuf::from(input.path);
    let Ok(file) = tokio::fs::File::open(&path).await else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let name = filename(
        path.file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("export"),
    );
    (
        [
            (header::CONTENT_TYPE, "application/octet-stream".to_string()),
            (
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{name}\""),
            ),
        ],
        Body::from_stream(tokio_util::io::ReaderStream::new(file)),
    )
        .into_response()
}
#[derive(Deserialize)]
struct Download {
    path: String,
}
async fn media(
    State(server): State<Server>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> Response {
    let path = server
        .app
        .shared
        .audio_assets
        .read()
        .ok()
        .and_then(|r| r.get(&id).cloned());
    let Some(path) = path else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let Ok(mut file) = tokio::fs::File::open(&path).await else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let Ok(meta) = file.metadata().await else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let total = meta.len();
    let mime = match path.extension().and_then(|s| s.to_str()) {
        Some("wav") => "audio/wav",
        Some("mp4") => "video/mp4",
        _ => "audio/mp4",
    };
    let mut response = Response::builder()
        .header(header::CONTENT_TYPE, mime)
        .header(header::ACCEPT_RANGES, "bytes");
    if let Some(range) = headers.get(header::RANGE).and_then(|v| v.to_str().ok()) {
        let Some((start, end)) = parse_range(range, total) else {
            return Response::builder()
                .status(416)
                .header(header::CONTENT_RANGE, format!("bytes */{total}"))
                .body(Body::empty())
                .unwrap();
        };
        if file.seek(std::io::SeekFrom::Start(start)).await.is_err() {
            return StatusCode::NOT_FOUND.into_response();
        }
        response = response
            .status(206)
            .header(
                header::CONTENT_RANGE,
                format!("bytes {start}-{end}/{total}"),
            )
            .header(header::CONTENT_LENGTH, end - start + 1);
        use tokio::io::AsyncReadExt;
        return response
            .body(Body::from_stream(tokio_util::io::ReaderStream::new(
                file.take(end - start + 1),
            )))
            .unwrap();
    }
    response
        .header(header::CONTENT_LENGTH, total)
        .body(Body::from_stream(tokio_util::io::ReaderStream::new(file)))
        .unwrap()
}
fn parse_range(range: &str, total: u64) -> Option<(u64, u64)> {
    if total == 0 {
        return None;
    }
    let text = range.strip_prefix("bytes=")?;
    if text.contains(',') {
        return None;
    }
    let (a, b) = text.split_once('-')?;
    if a.is_empty() {
        let n = b.parse::<u64>().ok()?;
        return (n > 0).then(|| (total.saturating_sub(n), total - 1));
    }
    let start = a.parse::<u64>().ok()?;
    let end = if b.is_empty() {
        total - 1
    } else {
        b.parse::<u64>().ok()?.min(total - 1)
    };
    (start < total && start <= end).then_some((start, end))
}
#[tokio::main]
async fn main() {
    let token = std::env::var("HOMER_TOKEN")
        .expect("Set HOMER_TOKEN to a private access token of at least 24 characters.");
    assert!(
        token.len() >= 24 && !token.contains(|c: char| !c.is_ascii_alphanumeric()),
        "HOMER_TOKEN must contain at least 24 ASCII letters/digits."
    );
    let data =
        PathBuf::from(std::env::var("HOMER_DATA_DIR").unwrap_or_else(|_| "./homer-data".into()));
    std::fs::create_dir_all(&data).unwrap();
    let data = data.canonicalize().unwrap();
    for dir in ["uploads", "downloads", "projects"] {
        std::fs::create_dir_all(data.join(dir)).unwrap()
    }
    let resources = PathBuf::from(
        std::env::var("HOMER_RESOURCES_DIR").unwrap_or_else(|_| "./server/resources".into()),
    );
    let resources = resources
        .canonicalize()
        .expect("Set HOMER_RESOURCES_DIR to the packaged workers/sfx directory.");
    let app = AppHandle::new(data, resources);
    let server = Server {
        app,
        token: Arc::new(token),
        limit: std::env::var("HOMER_UPLOAD_LIMIT_BYTES")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(10 * 1024 * 1024 * 1024),
    };
    let web = PathBuf::from(std::env::var("HOMER_WEB_DIR").unwrap_or_else(|_| "./dist".into()));
    let protected = Router::new()
        .route("/command/{name}", post(command))
        .route("/upload", post(upload))
        .route("/destination", post(destination))
        .route("/projects", get(projects))
        .route("/recording", post(recording_start).put(recording_append))
        .route("/download", get(download))
        .route("/media/{id}", get(media))
        .layer(middleware::from_fn_with_state(server.clone(), guard));
    let router = Router::new()
        .route("/api/session", post(session))
        .nest("/api", protected)
        .fallback_service(ServeDir::new(&web).fallback(ServeFile::new(web.join("index.html"))))
        .layer(DefaultBodyLimit::max(64 * 1024 * 1024))
        .with_state(server);
    let bind = std::env::var("HOMER_BIND").unwrap_or_else(|_| "127.0.0.1:8080".into());
    let listener = tokio::net::TcpListener::bind(&bind)
        .await
        .expect("Cannot bind Homer Studio server");
    eprintln!("Homer Studio listening on {bind}");
    axum::serve(listener, router).await.unwrap();
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn authentication_and_origin() {
        let mut h = HeaderMap::new();
        assert!(!authorized(&h, "testtoken"));
        h.insert(header::AUTHORIZATION, "Bearer testtoken".parse().unwrap());
        assert!(authorized(&h, "testtoken"));
        assert!(!authorized(&h, "different"));
        h.insert(header::HOST, "localhost:8080".parse().unwrap());
        h.insert(header::ORIGIN, "https://evil.example".parse().unwrap());
        assert!(!same_origin(&h));
        h.insert(header::ORIGIN, "http://localhost:8080".parse().unwrap());
        assert!(same_origin(&h));
    }
    #[test]
    fn range_and_filenames() {
        assert_eq!(parse_range("bytes=-3", 10), Some((7, 9)));
        assert_eq!(parse_range("bytes=9-", 10), Some((9, 9)));
        assert_eq!(parse_range("bytes=10-", 10), None);
        assert_eq!(filename("../../audio.wav"), "audio.wav");
    }
}
