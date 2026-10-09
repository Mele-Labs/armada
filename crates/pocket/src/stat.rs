//! The PWA's files, served at `/`.

use std::path::{Component, Path, PathBuf};

use axum::extract::{Request, State};
use axum::http::{header, Method, StatusCode};
use crate::Gateway;
use axum::response::{IntoResponse, Response};

/// A file under `dir`, or `index.html` for a path with no extension (the app's
/// own screens, such as `/jobs/:id`, which a push opens). A missing directory
/// is 404, not an error: the app may not be built yet.
pub async fn serve(State(gateway): State<Gateway>, request: Request) -> Response {
    let Some(dir) = gateway.assets else {
        return StatusCode::NOT_FOUND.into_response();
    };
    if request.method() != Method::GET && request.method() != Method::HEAD {
        return StatusCode::NOT_FOUND.into_response();
    }
    let path = request.uri().path();
    if path.starts_with("/api/") || path.starts_with("/admin/") {
        return StatusCode::NOT_FOUND.into_response();
    }
    let Some(file) = inside(&dir, path) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let file = if file.extension().is_none() {
        dir.join("index.html")
    } else {
        file
    };
    match tokio::fs::read(&file).await {
        Ok(bytes) => ([(header::CONTENT_TYPE, kind(&file))], bytes).into_response(),
        Err(_) => StatusCode::NOT_FOUND.into_response(),
    }
}

/// `path` under `dir`, or `None` if it would leave it.
fn inside(dir: &Path, path: &str) -> Option<PathBuf> {
    let mut at = dir.to_path_buf();
    for part in Path::new(path.trim_start_matches('/')).components() {
        match part {
            Component::Normal(name) => at.push(name),
            _ => return None,
        }
    }
    Some(at)
}

fn kind(file: &Path) -> &'static str {
    match file.extension().and_then(|e| e.to_str()) {
        Some("html") => "text/html; charset=utf-8",
        Some("js" | "mjs") => "text/javascript; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("json" | "webmanifest") => "application/json",
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        Some("ico") => "image/x-icon",
        _ => "application/octet-stream",
    }
}
