//! `/admin/*`: for Bridge on loopback, never for the phone and never for a page.
//!
//! Three kinds of request are refused. One carries a header a tunnel adds, or
//! names a host other than loopback, so it came through whatever tunnel the
//! owner points at the Gateway. One carries `Origin`, so a web page open in a
//! browser on this Mac sent it; Fleet refuses `Origin` for the same reason
//! (`api.from_a_page`). Bridge's main process calls `127.0.0.1` with neither.

use axum::extract::Request;
use axum::http::{header, HeaderMap, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};

/// The headers `tailscale serve`, Tailscale Funnel, ngrok and `cloudflared` add.
pub const FORWARDED: [&str; 10] = [
    "tailscale-user-login",
    "tailscale-user-name",
    "tailscale-funnel-request",
    "x-forwarded-for",
    "x-forwarded-host",
    "x-forwarded-proto",
    "forwarded",
    "x-real-ip",
    "cf-connecting-ip",
    "cf-ray",
];

/// A tunnel passes the phone's `Host`, the public name, through; Bridge names loopback.
fn names_another_host(headers: &HeaderMap) -> bool {
    let Some(host) = headers.get(header::HOST) else { return false };
    let host = host.to_str().unwrap_or("");
    let name = match host.rsplit_once(':') {
        Some((name, port)) if !name.ends_with(':') && port.bytes().all(|b| b.is_ascii_digit()) => name,
        _ => host,
    };
    !matches!(name.to_ascii_lowercase().as_str(), "127.0.0.1" | "localhost" | "[::1]")
}

pub async fn loopback_only(request: Request, next: Next) -> Response {
    let headers = request.headers();
    if headers.contains_key(header::ORIGIN) {
        return (
            StatusCode::FORBIDDEN,
            "A web page sent this request. Pairing is done from Bridge on this Mac, in Settings, Phone.",
        )
            .into_response();
    }
    if FORWARDED.iter().any(|name| headers.contains_key(*name)) || names_another_host(headers) {
        return (
            StatusCode::FORBIDDEN,
            "This route is for Bridge on this Mac, not for a phone.",
        )
            .into_response();
    }
    next.run(request).await
}
