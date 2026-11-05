//! Pocket's claim for a video: **a paired phone asks once, signed, for an address, and a `<video>` then
//! streams the file from the Mac's disk in ranges through a ticket that expires, is bound to the phone
//! that asked, and names no path of its own.**

use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
use std::sync::Arc;

use axum::body::{to_bytes, Body};
use axum::extract::RawQuery;
use axum::http::{HeaderMap, Method, Request, StatusCode};
use axum::routing::get;
use axum::Router;
use serde::Deserialize;
use tower::ServiceExt;

use crate::pairing_tests::call;
use crate::thread_tests::{phone_on, send, Phone};

static FILES: AtomicU32 = AtomicU32::new(0);

/// Long enough for several 64 KiB chunks.
const SIZE: usize = 150_000;

const ASK: &str = "/api/sessions/shown/address?session_id=h1&path=video";

#[derive(Deserialize)]
struct Address {
    address: String,
    media_type: String,
}

/// A file whose byte at `i` is `i % 251`, so any slice of it says where it was cut from.
fn video(len: usize) -> (std::path::PathBuf, Vec<u8>) {
    let dir = std::env::temp_dir().join(format!("pocket-shown-{}-{}", std::process::id(), FILES.fetch_add(1, Ordering::Relaxed)));
    std::fs::create_dir_all(&dir).unwrap();
    let bytes: Vec<u8> = (0..len).map(|at| (at % 251) as u8).collect();
    let path = dir.join("demo.mp4");
    std::fs::write(&path, &bytes).unwrap();
    (path, bytes)
}

/// A Fleet that shows `path` as an mp4 for `path=video`, refuses `path=missing`, and answers text and a
/// picture, which are not videos, for `path=text` and `path=picture`. It counts the asks it hears.
async fn fleet_showing(path: &std::path::Path, asked: Arc<AtomicUsize>) -> u16 {
    let on_disk = path.to_str().unwrap().to_string();
    let app = Router::new().route(
        "/sessions/shown",
        get(move |RawQuery(query): RawQuery| {
            let on_disk = on_disk.clone();
            let asked = asked.clone();
            async move {
                asked.fetch_add(1, Ordering::Relaxed);
                let json = [("content-type", "application/json")];
                match query.as_deref() {
                    Some("session_id=h1&path=video") => {
                        let body = ipc::encode(&ipc::ShownVideo { path: on_disk, media_type: "video/mp4".into(), bytes: SIZE as u64 }).unwrap();
                        (StatusCode::OK, json, body.into_bytes())
                    }
                    Some("session_id=h1&path=text") => (StatusCode::OK, json, br#"{"path":"/r/a.md","text":"words"}"#.to_vec()),
                    Some("session_id=h1&path=picture") => (StatusCode::OK, [("content-type", "image/png")], vec![0x89, b'P']),
                    _ => (StatusCode::NOT_FOUND, json, br#"{"code":"fleet.shown_file_not_shown","message":"job h1 did not show it"}"#.to_vec()),
                }
            }
        }),
    );
    let listener = crate::bind(0).await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move { axum::serve(listener, app).await });
    port
}

async fn phone_with(path: &std::path::Path) -> (Phone, Arc<AtomicUsize>) {
    let asked = Arc::new(AtomicUsize::new(0));
    (phone_on(fleet_showing(path, asked.clone()).await).await, asked)
}

/// The address the phone is given for the video, asked for as a paired phone does.
async fn minted(phone: &Phone) -> String {
    let (status, body) = send(phone, Method::GET, ASK, "").await;
    assert_eq!(status, StatusCode::OK, "{body}");
    ipc::decode::<Address>("an address", body.as_bytes()).unwrap().address
}

/// What a `<video>` sends: no signature, no other header than the range.
async fn fetch(phone: &Phone, method: Method, address: &str, range: Option<&str>) -> (StatusCode, HeaderMap, Vec<u8>) {
    let mut request = Request::builder().method(method).uri(address);
    if let Some(range) = range {
        request = request.header("range", range);
    }
    let answer = phone.rig.app.clone().oneshot(request.body(Body::empty()).unwrap()).await.unwrap();
    let (status, headers) = (answer.status(), answer.headers().clone());
    (status, headers, to_bytes(answer.into_body(), 1 << 22).await.unwrap().to_vec())
}

async fn get_ticket(phone: &Phone, address: &str, range: Option<&str>) -> (StatusCode, HeaderMap, Vec<u8>) {
    fetch(phone, Method::GET, address, range).await
}

fn header<'a>(headers: &'a HeaderMap, name: &str) -> &'a str {
    headers.get(name).and_then(|value| value.to_str().ok()).unwrap_or("")
}

#[tokio::test]
async fn the_address_route_refuses_an_unsigned_request() {
    let (path, _) = video(10);
    let (phone, asked) = phone_with(&path).await;
    let (status, _) = call(&phone.rig.app, Method::GET, ASK, &[], "").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(asked.load(Ordering::Relaxed), 0, "Fleet was not asked on behalf of a caller that did not sign");
}

#[tokio::test]
async fn the_address_route_passes_fleets_refusal_through() {
    let (path, _) = video(10);
    let (phone, _) = phone_with(&path).await;
    let (status, body) = send(&phone, Method::GET, "/api/sessions/shown/address?session_id=h1&path=missing", "").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body, r#"{"code":"fleet.shown_file_not_shown","message":"job h1 did not show it"}"#, "Fleet's own words");
}

#[tokio::test]
async fn the_address_route_answers_422_for_a_file_that_is_not_a_video() {
    let (path, _) = video(10);
    let (phone, _) = phone_with(&path).await;
    for what in ["text", "picture"] {
        let ask = format!("/api/sessions/shown/address?session_id=h1&path={what}");
        let (status, body) = send(&phone, Method::GET, &ask, "").await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{what}");
        assert_eq!(body, "Armada did not answer with a video for this file.", "{what}");
    }
}

#[tokio::test]
async fn the_address_route_mints_a_ticket_for_a_video() {
    let (path, _) = video(10);
    let (phone, _) = phone_with(&path).await;
    let (status, body) = send(&phone, Method::GET, ASK, "").await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let first: Address = ipc::decode("an address", body.as_bytes()).unwrap();
    assert_eq!(first.media_type, "video/mp4");
    let name = first.address.strip_prefix("/shown/").expect("an address on /shown/");
    assert_eq!(name.len(), 43, "32 random bytes, unpadded base64url: {name}");
    assert!(name.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_'), "{name}");
    assert!(!first.address.contains(path.to_str().unwrap()), "the address names no path");
    assert_ne!(minted(&phone).await, first.address, "each ask is a ticket of its own");
}

#[tokio::test]
async fn a_ticket_streams_the_whole_file_with_its_length() {
    let (path, bytes) = video(SIZE);
    let (phone, _) = phone_with(&path).await;
    let address = minted(&phone).await;
    let (status, headers, body) = get_ticket(&phone, &address, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(header(&headers, "content-length"), SIZE.to_string());
    assert_eq!(header(&headers, "content-type"), "video/mp4");
    assert_eq!(header(&headers, "accept-ranges"), "bytes");
    assert!(body == bytes, "{} bytes arrived of {SIZE}, and the same ones", body.len());
}

#[tokio::test]
async fn a_ticket_serves_closed_open_ended_and_suffix_ranges() {
    let (path, bytes) = video(SIZE);
    let (phone, _) = phone_with(&path).await;
    let address = minted(&phone).await;
    let last = SIZE - 1;
    // The cuts: one inside a chunk, one across several, an open end, a suffix, and an end past the file.
    let asked = [
        ("bytes=10-19", 10, 19),
        ("bytes=60000-140000", 60000, 140000),
        ("bytes=149990-", 149990, last),
        ("bytes=-5", SIZE - 5, last),
        ("bytes=149990-999999", 149990, last),
    ];
    for (range, first, end) in asked {
        let (status, headers, body) = get_ticket(&phone, &address, Some(range)).await;
        assert_eq!(status, StatusCode::PARTIAL_CONTENT, "{range}");
        assert_eq!(header(&headers, "content-range"), format!("bytes {first}-{end}/{SIZE}"), "{range}");
        assert_eq!(header(&headers, "content-length"), (end - first + 1).to_string(), "{range}");
        assert!(body == bytes[first..=end], "{range}: {} bytes, not those of the file", body.len());
    }
}

#[tokio::test]
async fn a_range_past_the_end_is_416_with_the_size() {
    let (path, _) = video(SIZE);
    let (phone, _) = phone_with(&path).await;
    let address = minted(&phone).await;
    for range in ["bytes=150000-", "bytes=200000-300000", "bytes=-0"] {
        let (status, headers, body) = get_ticket(&phone, &address, Some(range)).await;
        assert_eq!(status, StatusCode::RANGE_NOT_SATISFIABLE, "{range}");
        assert_eq!(header(&headers, "content-range"), format!("bytes */{SIZE}"), "{range}");
        assert!(body.is_empty(), "{range}");
    }
}

#[tokio::test]
async fn an_empty_file_is_200_whole_and_416_for_any_range() {
    let (path, _) = video(0);
    let (phone, _) = phone_with(&path).await;
    let address = minted(&phone).await;
    let (status, headers, body) = get_ticket(&phone, &address, None).await;
    assert_eq!((status, header(&headers, "content-length"), body.len()), (StatusCode::OK, "0", 0));
    for range in ["bytes=0-", "bytes=0-0", "bytes=-5"] {
        let (status, headers, _) = get_ticket(&phone, &address, Some(range)).await;
        assert_eq!(status, StatusCode::RANGE_NOT_SATISFIABLE, "{range}");
        assert_eq!(header(&headers, "content-range"), "bytes */0", "{range}");
    }
}

#[tokio::test]
async fn several_ranges_serve_the_whole_file() {
    let (path, bytes) = video(SIZE);
    let (phone, _) = phone_with(&path).await;
    let address = minted(&phone).await;
    let (status, headers, body) = get_ticket(&phone, &address, Some("bytes=0-4,10-14")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(header(&headers, "content-range"), "", "a whole file has no range");
    assert!(body == bytes);
}

#[tokio::test]
async fn a_ticket_answers_with_no_store_and_no_referrer() {
    let (path, _) = video(SIZE);
    let (phone, _) = phone_with(&path).await;
    let address = minted(&phone).await;
    for range in [None, Some("bytes=0-9")] {
        let (_, headers, _) = get_ticket(&phone, &address, range).await;
        assert_eq!(header(&headers, "cache-control"), "no-store", "{range:?}");
        assert_eq!(header(&headers, "referrer-policy"), "no-referrer", "{range:?}");
        assert_eq!(header(&headers, "x-content-type-options"), "nosniff", "{range:?}");
    }
}

#[tokio::test]
async fn a_ticket_expires_after_fifteen_minutes() {
    let (path, _) = video(10);
    let (phone, _) = phone_with(&path).await;
    let address = minted(&phone).await;
    let made = phone.rig.now.load(Ordering::Relaxed);

    phone.rig.now.store(made + 15 * 60 - 1, Ordering::Relaxed);
    assert_eq!(get_ticket(&phone, &address, None).await.0, StatusCode::OK, "a second before it lapses");
    phone.rig.now.store(made + 15 * 60, Ordering::Relaxed);
    assert_eq!(get_ticket(&phone, &address, None).await.0, StatusCode::NOT_FOUND, "at fifteen minutes");
}

#[tokio::test]
async fn the_oldest_ticket_is_dropped_past_sixty_four() {
    let (path, _) = video(10);
    let (phone, _) = phone_with(&path).await;
    let mut addresses = Vec::new();
    for _ in 0..64 {
        addresses.push(minted(&phone).await);
    }
    assert_eq!(get_ticket(&phone, &addresses[0], None).await.0, StatusCode::OK, "sixty-four are live");

    addresses.push(minted(&phone).await);
    assert_eq!(get_ticket(&phone, &addresses[0], None).await.0, StatusCode::NOT_FOUND, "the oldest made room");
    for address in &addresses[1..] {
        assert_eq!(get_ticket(&phone, address, None).await.0, StatusCode::OK, "{address}");
    }
}

#[tokio::test]
async fn an_unpaired_device_ends_its_ticket() {
    let (path, _) = video(10);
    let (phone, _) = phone_with(&path).await;
    let address = minted(&phone).await;
    assert_eq!(get_ticket(&phone, &address, None).await.0, StatusCode::OK);

    let (status, _) = call(&phone.rig.app, Method::DELETE, &format!("/admin/devices/{}", phone.device), &[], "").await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert_eq!(get_ticket(&phone, &address, None).await.0, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn a_file_gone_since_minting_is_404() {
    let (path, _) = video(10);
    let (phone, _) = phone_with(&path).await;
    let address = minted(&phone).await;
    std::fs::remove_file(&path).unwrap();
    assert_eq!(get_ticket(&phone, &address, None).await.0, StatusCode::NOT_FOUND);

    // A folder where the file was is not a file to stream either.
    let address = minted(&phone).await;
    std::fs::create_dir(&path).unwrap();
    assert_eq!(get_ticket(&phone, &address, None).await.0, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn an_unknown_ticket_is_404() {
    let (path, _) = video(10);
    let (phone, _) = phone_with(&path).await;
    minted(&phone).await;
    for address in ["/shown/nope", &format!("/shown/{}", "A".repeat(43))] {
        assert_eq!(get_ticket(&phone, address, None).await.0, StatusCode::NOT_FOUND, "{address}");
    }
}

/// Unsigned by design: a `<video>` element can send no custom header, so the ticket stands in for the
/// signature, and the address route beside it still refuses the same bare request.
#[tokio::test]
async fn a_ticket_needs_no_signature() {
    let (path, bytes) = video(10);
    let (phone, _) = phone_with(&path).await;
    let address = minted(&phone).await;
    let (status, _, body) = get_ticket(&phone, &address, None).await;
    assert_eq!((status, body), (StatusCode::OK, bytes));
    assert_eq!(call(&phone.rig.app, Method::GET, ASK, &[], "").await.0, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn head_answers_the_headers_without_a_body() {
    let (path, _) = video(SIZE);
    let (phone, _) = phone_with(&path).await;
    let address = minted(&phone).await;
    let (status, headers, body) = fetch(&phone, Method::HEAD, &address, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(header(&headers, "content-length"), SIZE.to_string());
    assert_eq!(header(&headers, "content-type"), "video/mp4");
    assert!(body.is_empty(), "{} body bytes on a HEAD", body.len());
}
