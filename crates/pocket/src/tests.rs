use std::sync::Arc;

use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tower::ServiceExt;

use crate::{bind, fleet_client, router, Gateway};

fn gateway() -> Gateway {
    Gateway {
        fleet: Arc::new(|| Err("Fleet is not running".to_string())),
        assets: None,
        pairing: crate::pairing_tests::rig_pairing(),
        push: crate::push_tests::test_push(),
    }
}

async fn status(method: Method, path: &str, headers: &[(&str, &str)]) -> StatusCode {
    let mut request = Request::builder().method(method).uri(path);
    for (name, value) in headers {
        request = request.header(*name, *value);
    }
    router(gateway())
        .oneshot(request.body(Body::empty()).unwrap())
        .await
        .unwrap()
        .status()
}

#[tokio::test]
async fn a_route_not_on_the_list_is_404() {
    for (method, path) in [
        (Method::GET, "/jobs"),
        (Method::GET, "/alerts"),
        (Method::POST, "/api/pilot"),
        (Method::GET, "/api/nothing"),
        (Method::GET, "/admin/nothing"),
        (Method::GET, "/"),
    ] {
        assert_eq!(status(method, path, &[]).await, StatusCode::NOT_FOUND, "{path}");
    }
}

#[tokio::test]
async fn a_listed_api_route_without_a_signature_is_400() {
    assert_eq!(status(Method::GET, "/api/needs", &[]).await, StatusCode::BAD_REQUEST);
    assert_eq!(status(Method::DELETE, "/api/sessions/answer", &[]).await, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn admin_is_refused_when_a_tunnel_forwarded_the_request() {
    for header in crate::admin::FORWARDED {
        assert_eq!(
            status(Method::GET, "/admin/devices", &[(header, "x")]).await,
            StatusCode::FORBIDDEN,
            "{header}"
        );
    }
    assert_eq!(
        status(Method::GET, "/admin/devices", &[]).await,
        StatusCode::OK
    );
}

#[tokio::test]
async fn admin_is_refused_for_any_host_but_loopback() {
    for host in ["mac.tail-1.ts.net", "armada.ngrok-free.app", "mac.example.com:443", "127.0.0.1.example.com"] {
        assert_eq!(status(Method::GET, "/admin/devices", &[("host", host)]).await, StatusCode::FORBIDDEN, "{host}");
    }
    for host in ["127.0.0.1:8443", "localhost:8443", "[::1]:8443", "127.0.0.1"] {
        assert_eq!(status(Method::GET, "/admin/devices", &[("host", host)]).await, StatusCode::OK, "{host}");
    }
}

#[tokio::test]
async fn the_forwarding_headers_do_not_close_the_phone_routes() {
    assert_eq!(
        status(Method::GET, "/api/needs", &[("x-forwarded-for", "100.1.1.1")]).await,
        StatusCode::BAD_REQUEST
    );
}

#[tokio::test]
async fn the_listener_is_on_loopback() {
    let listener = bind(0).await.unwrap();
    assert!(listener.local_addr().unwrap().ip().is_loopback());
}

#[tokio::test]
async fn a_missing_static_directory_is_tolerated() {
    let gateway = Gateway {
        assets: Some("/nonexistent/pocket".into()),
        ..gateway()
    };
    let answer = router(gateway)
        .oneshot(Request::get("/sessions/4").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(answer.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn static_files_are_served_and_cannot_leave_the_directory() {
    let dir = std::env::temp_dir().join(format!("pocket-static-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("index.html"), "<p>hi</p>").unwrap();
    let app = router(Gateway {
        assets: Some(dir.clone()),
        ..gateway()
    });
    for path in ["/", "/sessions/4", "/index.html", "/jobs/https%3A%2F%2Fmac-b.tail.ts.net/s.1", "/jobs/_/s1"] {
        let answer = app
            .clone()
            .oneshot(Request::get(path).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(answer.status(), StatusCode::OK, "{path}");
    }
    let out = app
        .oneshot(Request::get("/../secret.txt").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(out.status(), StatusCode::NOT_FOUND);
    std::fs::remove_dir_all(dir).ok();
}

#[tokio::test]
async fn a_call_to_fleet_carries_no_origin() {
    let fleet = bind(0).await.unwrap();
    let port = fleet.local_addr().unwrap().port();
    let seen = tokio::spawn(async move {
        let (mut socket, _) = fleet.accept().await.unwrap();
        let mut got = vec![0; 4096];
        let n = socket.read(&mut got).await.unwrap();
        socket
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\n[]")
            .await
            .unwrap();
        String::from_utf8_lossy(&got[..n]).to_lowercase()
    });
    let answer = fleet_client::send(port, "GET", "/alerts", None).await.unwrap();
    assert_eq!((answer.status, answer.body.as_slice()), (200, &b"[]"[..]));
    let request = seen.await.unwrap();
    assert!(request.starts_with("get /alerts "), "{request}");
    assert!(!request.contains("origin"), "{request}");
}

#[tokio::test]
async fn status_says_when_fleet_is_not_running() {
    assert_eq!(
        status(Method::GET, "/admin/status", &[]).await,
        StatusCode::SERVICE_UNAVAILABLE
    );
}

#[tokio::test]
async fn admin_is_refused_when_a_web_page_sent_the_request() {
    for (method, path) in [
        (Method::POST, "/admin/pair/confirm"),
        (Method::GET, "/admin/devices"),
        (Method::GET, "/admin/status"),
    ] {
        assert_eq!(
            status(method, path, &[("origin", "https://example.com")]).await,
            StatusCode::FORBIDDEN,
            "{path}"
        );
    }
}

#[tokio::test]
async fn the_pair_page_is_the_apps_own() {
    let dir = std::env::temp_dir().join(format!("pocket-pair-page-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("index.html"), "<html>app</html>").unwrap();
    let mut gateway = gateway();
    gateway.assets = Some(dir);
    let answer = router(gateway)
        .oneshot(Request::builder().method(Method::GET).uri("/pair?code=abc").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(answer.status(), StatusCode::OK);
}

async fn answered(method: Method, path: &str, origin: &str) -> axum::response::Response {
    let request = Request::builder().method(method).uri(path).header("origin", origin);
    router(gateway()).oneshot(request.body(Body::empty()).unwrap()).await.unwrap()
}

fn allowed(answer: &axum::response::Response) -> Option<&str> {
    answer.headers().get("access-control-allow-origin").and_then(|v| v.to_str().ok())
}

#[tokio::test]
async fn another_macs_app_may_call_the_phone_routes_and_pair() {
    let mac_a = "https://mac-a.tail.ts.net";
    for path in ["/api/sessions/one", "/api/needs", "/pair"] {
        let preflight = answered(Method::OPTIONS, path, mac_a).await;
        assert_eq!(preflight.status(), StatusCode::NO_CONTENT, "{path}");
        assert_eq!(allowed(&preflight), Some(mac_a), "{path}");
        let headers = preflight.headers()["access-control-allow-headers"].to_str().unwrap();
        assert_eq!(headers, "content-type, x-pocket-device, x-pocket-time, x-pocket-signature");
    }
    // The real call is still refused on its own terms, and the browser may read why.
    let unsigned = answered(Method::GET, "/api/needs", mac_a).await;
    assert_eq!((unsigned.status(), allowed(&unsigned)), (StatusCode::BAD_REQUEST, Some(mac_a)));
}

#[tokio::test]
async fn cors_names_no_plain_origin_and_never_opens_admin() {
    let plain = answered(Method::OPTIONS, "/api/needs", "http://mac-a.tail.ts.net").await;
    assert_ne!(plain.status(), StatusCode::NO_CONTENT);
    assert_eq!(allowed(&plain), None);
    let admin = answered(Method::GET, "/admin/devices", "https://mac-a.tail.ts.net").await;
    assert_eq!((admin.status(), allowed(&admin)), (StatusCode::FORBIDDEN, None));
    let admin = answered(Method::OPTIONS, "/admin/devices", "https://mac-a.tail.ts.net").await;
    assert_eq!(allowed(&admin), None);
    let page = answered(Method::GET, "/pair", "https://mac-a.tail.ts.net").await;
    assert_eq!(allowed(&page), None);
}

/// A built app with a hashed script in both compressed forms, `sw.js` in neither
/// and `style.css` in gzip only.
fn built_app(name: &str) -> (axum::Router, std::path::PathBuf) {
    let dir = std::env::temp_dir().join(format!("pocket-{name}-{}", std::process::id()));
    std::fs::create_dir_all(dir.join("assets")).unwrap();
    std::fs::write(dir.join("index.html"), "<p>hi</p>").unwrap();
    std::fs::write(dir.join("sw.js"), "self").unwrap();
    std::fs::write(dir.join("manifest.webmanifest"), "{}").unwrap();
    std::fs::write(dir.join("assets/app-1a2b.js"), "plain").unwrap();
    std::fs::write(dir.join("assets/app-1a2b.js.br"), "brotli").unwrap();
    std::fs::write(dir.join("assets/app-1a2b.js.gz"), "gzipped").unwrap();
    std::fs::write(dir.join("assets/style-3c4d.css"), "plain css").unwrap();
    std::fs::write(dir.join("assets/style-3c4d.css.gz"), "gzipped css").unwrap();
    let app = router(Gateway { assets: Some(dir.clone()), ..gateway() });
    (app, dir)
}

async fn fetched(app: &axum::Router, path: &str, encoding: Option<&str>) -> (axum::http::HeaderMap, String) {
    let mut request = Request::get(path);
    if let Some(encoding) = encoding {
        request = request.header("accept-encoding", encoding);
    }
    let answer = app.clone().oneshot(request.body(Body::empty()).unwrap()).await.unwrap();
    assert_eq!(answer.status(), StatusCode::OK, "{path}");
    let headers = answer.headers().clone();
    let body = axum::body::to_bytes(answer.into_body(), 1 << 20).await.unwrap();
    (headers, String::from_utf8(body.to_vec()).unwrap())
}

fn header<'a>(headers: &'a axum::http::HeaderMap, name: &str) -> Option<&'a str> {
    headers.get(name).and_then(|value| value.to_str().ok())
}

#[tokio::test]
async fn a_compressed_copy_is_sent_when_the_client_accepts_it() {
    let (app, dir) = built_app("encoded");
    let script = "/assets/app-1a2b.js";
    let (headers, body) = fetched(&app, script, Some("gzip, deflate, br")).await;
    assert_eq!((body.as_str(), header(&headers, "content-encoding")), ("brotli", Some("br")));
    assert_eq!(header(&headers, "content-type"), Some("text/javascript; charset=utf-8"));
    assert_eq!(header(&headers, "vary"), Some("Accept-Encoding"));
    let (headers, body) = fetched(&app, script, Some("gzip;q=0.8, deflate")).await;
    assert_eq!((body.as_str(), header(&headers, "content-encoding")), ("gzipped", Some("gzip")));
    let (headers, body) = fetched(&app, script, Some("identity")).await;
    assert_eq!((body.as_str(), header(&headers, "content-encoding")), ("plain", None));
    assert_eq!(header(&headers, "vary"), Some("Accept-Encoding"));
    let (headers, body) = fetched(&app, script, Some("br;q=0, gzip")).await;
    assert_eq!((body.as_str(), header(&headers, "content-encoding")), ("gzipped", Some("gzip")));
    // No `.br` beside the stylesheet: the gzip copy answers a client that takes both.
    let (headers, body) = fetched(&app, "/assets/style-3c4d.css", Some("br, gzip")).await;
    assert_eq!((body.as_str(), header(&headers, "content-encoding")), ("gzipped css", Some("gzip")));
    assert_eq!(header(&headers, "content-type"), Some("text/css; charset=utf-8"));
    // A file with no copy is served as it is, with nothing to vary on.
    let (headers, body) = fetched(&app, "/sw.js", Some("br, gzip")).await;
    assert_eq!((body.as_str(), header(&headers, "content-encoding"), header(&headers, "vary")), ("self", None, None));
    std::fs::remove_dir_all(dir).ok();
}

#[tokio::test]
async fn hashed_assets_are_cached_for_good_and_everything_else_is_revalidated() {
    let (app, dir) = built_app("cache");
    for path in ["/assets/app-1a2b.js", "/assets/style-3c4d.css"] {
        let (headers, _) = fetched(&app, path, None).await;
        assert_eq!(header(&headers, "cache-control"), Some("public, max-age=31536000, immutable"), "{path}");
    }
    for path in ["/", "/index.html", "/sessions/4", "/sw.js", "/manifest.webmanifest"] {
        let (headers, _) = fetched(&app, path, None).await;
        assert_eq!(header(&headers, "cache-control"), Some("no-cache"), "{path}");
    }
    std::fs::remove_dir_all(dir).ok();
}
