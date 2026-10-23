//! A taught tracker over HTTP against a socket on this machine, and the credential: filled into a
//! header where the file says, and never in what a refusal says, over either transport.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread::JoinHandle;

use adapter_traits::Tracker;

use super::scratch::Scratch;
use super::taught_tracker::{script, GUIDE, READ_MAP, SEARCH_MAP};
use crate::taught_tracker::{self, TaughtTracker};

const SECRET: &str = "tok-s3cret/+";

/// Each connection answered with the next of `answers`, and every request as it arrived.
fn serve(answers: Vec<String>) -> (String, JoinHandle<Vec<String>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let handle = std::thread::spawn(move || {
        answers
            .into_iter()
            .map(|answer| {
                let (mut stream, _) = listener.accept().unwrap();
                let request = request_of(&mut stream);
                stream.write_all(answer.as_bytes()).unwrap();
                request
            })
            .collect()
    });
    (url, handle)
}

fn request_of(stream: &mut TcpStream) -> String {
    let (mut bytes, mut chunk) = (Vec::new(), [0u8; 4096]);
    loop {
        let text = String::from_utf8_lossy(&bytes).to_string();
        if let Some(end) = text.find("\r\n\r\n") {
            let length = text[..end]
                .lines()
                .find_map(|line| line.to_ascii_lowercase().strip_prefix("content-length: ").map(|n| n.trim().parse().unwrap_or(0)))
                .unwrap_or(0usize);
            if bytes.len() >= end + 4 + length {
                return text;
            }
        }
        let n = stream.read(&mut chunk).unwrap();
        if n == 0 {
            return String::from_utf8_lossy(&bytes).into_owned();
        }
        bytes.extend_from_slice(&chunk[..n]);
    }
}

fn answer(status: &str, body: &str) -> String {
    format!("HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len())
}

fn http_file(url: &str, secret: &str) -> String {
    format!(
        "{GUIDE}\n[credential]\nargv = [\"{secret}\"]\n\n[search]\ntransport = \"http\"\nmethod = \"POST\"\n\
         url = \"{url}/graphql\"\nbody = '{{\"query\": \"issues\", \"filter\": \"{{query}}\"}}'\nmap = '''{SEARCH_MAP}'''\n\
         [search.headers]\nAuthorization = \"Bearer {{credential}}\"\n\n\
         [read]\ntransport = \"http\"\nurl = \"{url}/issues/{{reference}}?k={{credential}}\"\nmap = '''{READ_MAP}'''\n"
    )
}

fn secret_script(dir: &Scratch) -> String {
    script(dir, "secret", &format!("printf '%s\\n' '{SECRET}'"))
}

#[test]
fn a_taught_tracker_searches_and_reads_over_http_with_its_credential_in_the_header() {
    let dir = Scratch::new();
    let searched = r#"{"data":{"issues":[{"identifier":"LIN-1","title":"Over the wire","url":"https://t.example/LIN-1",
      "state":{"name":"Todo","type":"unstarted"},"priority":"Low","labels":[],"updatedAt":"2026-10-20T10:00:00Z"}]}}"#;
    let one = r#"{"identifier":"LIN 1","title":"Over the wire","url":"u","state":{"type":"started"},"comments":[],"pulls":[]}"#;
    let (url, server) = serve(vec![answer("200 OK", searched), answer("200 OK", one), answer("404 Not Found", "{}")]);
    let tracker = TaughtTracker::over(taught_tracker::read(&http_file(&url, &secret_script(&dir))).expect("it reads"));

    let found = tracker.search_issues("/repo", "title has \"wire\"").expect("searched");
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].title.as_written(), "Over the wire");
    assert_eq!(found[0].fields.iter().map(|one| one.name.as_str()).collect::<Vec<_>>(), ["Status", "Priority"]);
    let read = tracker.read_issue("/repo", "LIN 1").expect("read").expect("an issue");
    assert_eq!(read.reference, "LIN 1");
    assert_eq!(tracker.read_issue("/repo", "LIN-404").expect("a 404 is no issue"), None);

    let requests = server.join().unwrap();
    let lower = requests[0].to_ascii_lowercase();
    assert!(lower.starts_with("post /graphql http/1.1"), "{}", requests[0]);
    assert!(lower.contains(&format!("\r\nauthorization: bearer {}\r\n", SECRET.to_ascii_lowercase())), "{}", requests[0]);
    assert!(requests[0].ends_with(r#"{"query": "issues", "filter": "title has \"wire\""}"#), "{}", requests[0]);
    assert!(requests[1].starts_with("GET /issues/LIN%201?k=tok-s3cret%2F%2B HTTP/1.1"), "{}", requests[1]);
}

#[test]
fn the_credential_never_reaches_what_a_refusal_says() {
    let dir = Scratch::new();
    let (url, server) = serve(vec![answer("401 Unauthorized", &format!("{{\"error\":\"token {SECRET} is revoked\"}}"))]);
    let tracker = TaughtTracker::over(taught_tracker::read(&http_file(&url, &secret_script(&dir))).expect("it reads"));
    let refused = tracker.search_issues("/repo", "x").expect_err("refused");
    server.join().unwrap();
    assert!(!refused.said.contains(SECRET), "{}", refused.said);
    assert!(refused.said.contains("401") && refused.said.contains("[credential]"), "{}", refused.said);

    let echoing = script(&dir, "echoing", "echo \"bad token: $2\" >&2\nexit 1");
    let cli = format!(
        "{GUIDE}\n[credential]\nargv = [\"{}\"]\n\n[search]\ntransport = \"cli\"\nargv = [\"{echoing}\", \"search\", \"{{credential}}\", \"{{query}}\"]\nmap = \".\"\n\n\
         [read]\ntransport = \"cli\"\nargv = [\"{echoing}\", \"{{reference}}\"]\nmap = \".\"\n",
        secret_script(&dir)
    );
    let refused = TaughtTracker::over(taught_tracker::read(&cli).expect("it reads")).search_issues("/repo", "x").expect_err("refused");
    assert_eq!(refused.said, "bad token: [credential]");
}

#[test]
fn a_credential_command_that_fails_is_said_and_nothing_is_asked() {
    let dir = Scratch::new();
    let failing = script(&dir, "no-secret", "echo 'The item could not be found.' >&2\nexit 44");
    let (url, _server) = serve(Vec::new());
    let tracker = TaughtTracker::over(taught_tracker::read(&http_file(&url, &failing)).expect("it reads"));
    let refused = tracker.open_issues("/repo").expect_err("no credential");
    assert_eq!(refused.said, "the credential was not read: The item could not be found.");
}
