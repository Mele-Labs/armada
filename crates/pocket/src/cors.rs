//! Cross-origin calls from the phone app another Mac served.
//!
//! **The signature is the auth; this only lets the browser send it.** An app
//! from Mac A calls Mac B, so B answers a preflight and names A's origin on
//! the answer. Only an `https` origin is named: the app is only ever served
//! over the owner's https tunnel. `/admin/*` is not here and keeps refusing any `Origin`.

use axum::extract::Request;
use axum::http::{header, HeaderValue, Method, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};

const ALLOWED_HEADERS: &str = "content-type, x-pocket-device, x-pocket-time, x-pocket-signature";
const ALLOWED_METHODS: &str = "GET, POST";
/// How long a browser may keep a preflight's answer: ten minutes.
const PREFLIGHT_SECONDS: &str = "600";

fn crossable(request: &Request) -> bool {
    let path = request.uri().path();
    path.starts_with("/api/") || path == "/pair" && matches!(*request.method(), Method::POST | Method::OPTIONS)
}

fn https_origin(request: &Request) -> Option<HeaderValue> {
    request
        .headers()
        .get(header::ORIGIN)
        .filter(|origin| origin.to_str().is_ok_and(|text| text.starts_with("https://")))
        .cloned()
}

pub async fn cors(request: Request, next: Next) -> Response {
    let origin = crossable(&request).then(|| https_origin(&request)).flatten();
    let Some(origin) = origin else {
        return next.run(request).await;
    };
    let mut answer = match *request.method() {
        Method::OPTIONS => {
            let mut preflight = StatusCode::NO_CONTENT.into_response();
            let headers = preflight.headers_mut();
            headers.insert(header::ACCESS_CONTROL_ALLOW_METHODS, HeaderValue::from_static(ALLOWED_METHODS));
            headers.insert(header::ACCESS_CONTROL_ALLOW_HEADERS, HeaderValue::from_static(ALLOWED_HEADERS));
            headers.insert(header::ACCESS_CONTROL_MAX_AGE, HeaderValue::from_static(PREFLIGHT_SECONDS));
            preflight
        }
        _ => next.run(request).await,
    };
    let headers = answer.headers_mut();
    headers.insert(header::ACCESS_CONTROL_ALLOW_ORIGIN, origin);
    headers.append(header::VARY, HeaderValue::from_static("origin"));
    answer
}
