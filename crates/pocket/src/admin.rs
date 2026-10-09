//! `/admin/*`: for Bridge on loopback, never for the phone and never for a page.
//!
//! Two kinds of request are refused. One carries a header `tailscale serve`
//! adds, so it came over the tailnet. The other carries `Origin`, so a web page
//! open in a browser on this Mac sent it; Fleet refuses `Origin` for the same
//! reason (`api.from_a_page`). Bridge's main process calls with neither.

use axum::extract::Request;
use axum::http::{header, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};

/// The headers `tailscale serve` adds.
pub const FORWARDED: [&str; 5] = [
    "tailscale-user-login",
    "tailscale-user-name",
    "x-forwarded-for",
    "x-forwarded-host",
    "x-forwarded-proto",
];

pub async fn loopback_only(request: Request, next: Next) -> Response {
    let headers = request.headers();
    if headers.contains_key(header::ORIGIN) {
        return (
            StatusCode::FORBIDDEN,
            "A web page sent this request. Pairing is done from Bridge on this Mac, in Settings, Phone.",
        )
            .into_response();
    }
    if FORWARDED.iter().any(|name| headers.contains_key(*name)) {
        return (
            StatusCode::FORBIDDEN,
            "This route is for Bridge on this Mac, not for a phone.",
        )
            .into_response();
    }
    next.run(request).await
}
