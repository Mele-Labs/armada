//! The request sender against a socket on this machine: each header goes out as it is named, a
//! filled secret's among them, a redirect is the answer and not followed, the body is bounded, and a refused
//! connection or a server that never answers is a failure a person can read.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread::JoinHandle;
use std::time::Duration;

use adapter_traits::{Outgoing, RequestFailure, RequestSender};

use crate::HttpRequests;

/// One connection answered with `answer`, and the request's head as it arrived.
fn serve_once(answer: String) -> (String, JoinHandle<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let handle = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let head = head_of(&mut stream);
        stream.write_all(answer.as_bytes()).unwrap();
        head
    });
    (url, handle)
}

fn head_of(stream: &mut TcpStream) -> String {
    let (mut bytes, mut chunk) = (Vec::new(), [0u8; 1024]);
    while !bytes.windows(4).any(|four| four == b"\r\n\r\n") {
        let n = stream.read(&mut chunk).unwrap();
        if n == 0 {
            break;
        }
        bytes.extend_from_slice(&chunk[..n]);
    }
    String::from_utf8_lossy(&bytes).into_owned()
}

fn answer(status: &str, headers: &str, body: &str) -> String {
    format!("HTTP/1.1 {status}\r\n{headers}Content-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len())
}

fn get(url: &str, credential: Option<(&str, &str)>) -> Outgoing {
    let mut headers = vec![("Accept".to_string(), "application/json".to_string())];
    headers.extend(credential.map(|(name, value)| (name.to_string(), value.to_string())));
    Outgoing { method: "GET".to_string(), url: url.to_string(), headers, body: None }
}

fn sender() -> HttpRequests {
    HttpRequests::within(Duration::from_secs(5))
}

#[test]
fn an_authorization_goes_out_under_its_own_name() {
    let (url, server) = serve_once(answer("200 OK", "", "{}"));
    sender().send(get(&url, Some(("Authorization", "Bearer abc123"))), 1024).expect("answered");
    let head = server.join().unwrap().to_ascii_lowercase();
    assert!(head.contains("\r\nauthorization: bearer abc123\r\n"), "{head}");
    assert!(!head.contains("\r\ncookie:"), "{head}");
}

#[test]
fn a_cookie_goes_out_as_the_cookie_header_and_the_answer_comes_back() {
    let (url, server) = serve_once(answer("200 OK", "X-Seen: yes\r\n", "{\"ok\":true}"));
    let came = sender().send(get(&format!("{url}/v1/me"), Some(("Cookie", "session=abc123"))), 1024).expect("answered");
    let head = server.join().unwrap().to_ascii_lowercase();
    assert!(head.starts_with("get /v1/me http/1.1"), "{head}");
    assert!(head.contains("\r\ncookie: session=abc123\r\n"), "{head}");
    assert!(head.contains("\r\naccept: application/json\r\n"), "{head}");
    assert_eq!(came.status, 200);
    assert_eq!(came.body, b"{\"ok\":true}");
    assert!(!came.truncated);
    assert!(came.headers.iter().any(|(name, value)| name == "x-seen" && value == "yes"));
}

#[test]
fn a_redirect_is_the_answer_and_is_not_followed() {
    let (url, server) = serve_once(answer("302 Found", "Location: http://elsewhere.example.test/\r\n", ""));
    let came = sender().send(get(&url, Some(("Cookie", "session=abc123"))), 1024).expect("answered");
    server.join().unwrap();
    assert_eq!(came.status, 302);
}

#[test]
fn a_body_past_the_bound_is_cut_and_says_so() {
    let (url, server) = serve_once(answer("200 OK", "", &"x".repeat(100)));
    let came = sender().send(get(&url, None), 10).expect("answered");
    server.join().unwrap();
    assert_eq!(came.body.len(), 10);
    assert!(came.truncated);
}

#[test]
fn a_refused_connection_and_a_silent_server_are_failures_a_person_can_read() {
    let closed = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", closed.local_addr().unwrap());
    drop(closed);
    let refused = sender().send(get(&url, None), 10).unwrap_err();
    assert!(matches!(refused, RequestFailure::Unreachable(_)), "{refused:?}");
    assert!(!refused.said().is_empty());

    let silent = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", silent.local_addr().unwrap());
    let quick = HttpRequests::within(Duration::from_millis(300));
    let timed = quick.send(get(&url, None), 10).unwrap_err();
    drop(silent);
    assert_eq!(timed, RequestFailure::TimedOut);
}
