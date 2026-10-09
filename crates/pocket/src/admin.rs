//! `/admin/*`: for Bridge on loopback, never for the phone.
//!
//! `tailscale serve` adds headers to every request it forwards. A request that
//! carries any of them came over the tailnet, and is refused. Bridge reaches
//! the Gateway directly and carries none.

use axum::extract::Request;
use axum::http::StatusCode;
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
    if FORWARDED
        .iter()
        .any(|name| request.headers().contains_key(*name))
    {
        return (
            StatusCode::FORBIDDEN,
            "This route is for Bridge on this Mac, not for a phone.",
        )
            .into_response();
    }
    next.run(request).await
}
