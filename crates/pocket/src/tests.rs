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
async fn a_listed_api_route_without_a_signature_is_401() {
    assert_eq!(status(Method::GET, "/api/needs", &[]).await, StatusCode::UNAUTHORIZED);
    assert_eq!(status(Method::DELETE, "/api/jobs/1", &[]).await, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn admin_is_refused_when_tailscale_serve_forwarded_the_request() {
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
async fn the_forwarding_headers_do_not_close_the_phone_routes() {
    assert_eq!(
        status(Method::GET, "/api/needs", &[("x-forwarded-for", "100.1.1.1")]).await,
        StatusCode::UNAUTHORIZED
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
        .oneshot(Request::get("/jobs/4").body(Body::empty()).unwrap())
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
    for path in ["/", "/jobs/4", "/index.html"] {
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
