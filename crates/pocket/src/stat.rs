//! The PWA's files, served at `/`.

use std::path::{Component, Path, PathBuf};

use axum::extract::{Request, State};
use axum::http::{header, HeaderMap, HeaderValue, Method, StatusCode};
use crate::Gateway;
use axum::response::{IntoResponse, Response};

/// A file under `dir`, or `index.html` for a path with no extension (the app's
/// own screens, such as `/sessions/:id`) and for anything under `/jobs/`, the
/// deep link a push opens, whose last part is a Job id. A missing directory
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
    let file = if file.extension().is_none() || path.starts_with("/jobs/") {
        dir.join("index.html")
    } else {
        file
    };
    // Vite names everything under `assets/` by its content hash, so it never changes.
    // The SPA fallback is `index.html` and so is not under it.
    let cache = if file.starts_with(dir.join("assets")) { IMMUTABLE } else { "no-cache" };
    let accepted = request
        .headers()
        .get(header::ACCEPT_ENCODING)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("");
    let mut varies = false;
    let mut chosen = None;
    for (coding, suffix) in [("br", "br"), ("gzip", "gz")] {
        let copy = beside(&file, suffix);
        if !tokio::fs::try_exists(&copy).await.unwrap_or(false) {
            continue;
        }
        varies = true;
        if chosen.is_none() && accepts(accepted, coding) {
            chosen = Some((coding, copy));
        }
    }
    let (coding, source) = match chosen {
        Some((coding, copy)) => (Some(coding), copy),
        None => (None, file.clone()),
    };
    let Ok(bytes) = tokio::fs::read(&source).await else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let mut headers = HeaderMap::new();
    headers.insert(header::CONTENT_TYPE, HeaderValue::from_static(kind(&file)));
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static(cache));
    if let Some(coding) = coding {
        headers.insert(header::CONTENT_ENCODING, HeaderValue::from_static(coding));
    }
    if varies {
        headers.insert(header::VARY, HeaderValue::from_static("Accept-Encoding"));
    }
    (headers, bytes).into_response()
}

const IMMUTABLE: &str = "public, max-age=31536000, immutable";

/// `file` with `.suffix` added after its own extension.
fn beside(file: &Path, suffix: &str) -> PathBuf {
    let mut name = file.as_os_str().to_os_string();
    name.push(".");
    name.push(suffix);
    PathBuf::from(name)
}

/// Whether an `Accept-Encoding` value lists `coding` with a quality above zero.
fn accepts(accepted: &str, coding: &str) -> bool {
    accepted.split(',').any(|item| {
        let mut parts = item.split(';');
        let named = parts.next().unwrap_or("").trim().eq_ignore_ascii_case(coding);
        let refused = parts.any(|param| {
            param.trim().strip_prefix("q=").and_then(|q| q.trim().parse::<f32>().ok()) == Some(0.0)
        });
        named && !refused
    })
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
