//! The GitHub client against a socket on this machine that answers HTTP/1.1
//! with keep-alive and counts the connections it accepts.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use adapter_traits::Secret;
use serde::{Deserialize, Serialize};

use crate::github_api::{next_link, ApiFailure, GitHubApi, GitHubRepository, NotGitHub, RateLimit};
use crate::ForgeAddress;

#[derive(Clone, Debug)]
pub(super) struct Seen {
    pub(super) method: String,
    pub(super) target: String,
    pub(super) authorization: Option<String>,
    pub(super) accept: Option<String>,
    pub(super) body: String,
}

pub(super) struct Reply {
    pub(super) status: u16,
    pub(super) headers: Vec<(&'static str, String)>,
    pub(super) body: String,
}

pub(super) fn ok(body: &str) -> Reply {
    Reply { status: 200, headers: Vec::new(), body: body.to_string() }
}

pub(super) struct Server {
    pub(super) url: String,
    connections: Arc<AtomicUsize>,
    seen: Arc<Mutex<Vec<Seen>>>,
}

impl Server {
    pub(super) fn seen(&self) -> Vec<Seen> {
        self.seen.lock().unwrap().clone()
    }
}

/// A server answering every request with `answer`, each connection on a thread
/// of its own and kept open for as many requests as the client sends on it.
pub(super) fn serve(answer: impl Fn(&Seen) -> Reply + Send + Sync + 'static) -> Server {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let connections = Arc::new(AtomicUsize::new(0));
    let seen = Arc::new(Mutex::new(Vec::new()));
    let answer = Arc::new(answer);
    let (counted, kept) = (connections.clone(), seen.clone());
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let mut stream = stream.unwrap();
            counted.fetch_add(1, Ordering::SeqCst);
            let (answer, kept) = (answer.clone(), kept.clone());
            std::thread::spawn(move || {
                while let Some(request) = read_request(&mut stream) {
                    kept.lock().unwrap().push(request.clone());
                    let reply = answer(&request);
                    let mut head = format!("HTTP/1.1 {} X\r\nContent-Length: {}\r\n", reply.status, reply.body.len());
                    for (name, value) in &reply.headers {
                        head.push_str(&format!("{name}: {value}\r\n"));
                    }
                    head.push_str("\r\n");
                    if stream.write_all(head.as_bytes()).is_err() || stream.write_all(reply.body.as_bytes()).is_err() {
                        return;
                    }
                }
            });
        }
    });
    Server { url, connections, seen }
}

/// The next request on a connection, or `None` once the client has closed it.
fn read_request(stream: &mut TcpStream) -> Option<Seen> {
    let mut bytes = Vec::new();
    let mut chunk = [0u8; 1024];
    let split = loop {
        let n = stream.read(&mut chunk).ok()?;
        if n == 0 {
            return None;
        }
        bytes.extend_from_slice(&chunk[..n]);
        if let Some(at) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
            break at + 4;
        }
    };
    let head = String::from_utf8(bytes[..split].to_vec()).unwrap();
    let mut lines = head.split("\r\n");
    let mut first = lines.next().unwrap().split(' ');
    let (method, target) = (first.next().unwrap().to_string(), first.next().unwrap().to_string());
    let headers: Vec<(String, String)> =
        lines.filter_map(|l| l.split_once(": ")).map(|(n, v)| (n.to_ascii_lowercase(), v.to_string())).collect();
    let header = |name: &str| headers.iter().find(|(n, _)| n == name).map(|(_, v)| v.clone());
    let wanted: usize = header("content-length").map_or(0, |v| v.parse().unwrap());
    while bytes.len() < split + wanted {
        let n = stream.read(&mut chunk).ok()?;
        if n == 0 {
            return None;
        }
        bytes.extend_from_slice(&chunk[..n]);
    }
    Some(Seen {
        method,
        target,
        authorization: header("authorization"),
        accept: header("accept"),
        body: String::from_utf8(bytes[split..split + wanted].to_vec()).unwrap(),
    })
}

/// A client on `url` whose token source hands out `tokens` in turn, the last
/// one for ever, and counts how often it was asked.
pub(super) fn api(url: &str, tokens: &[&'static str]) -> (GitHubApi, Arc<AtomicUsize>) {
    let reads = Arc::new(AtomicUsize::new(0));
    let (counted, tokens) = (reads.clone(), tokens.to_vec());
    let api = GitHubApi::against(url, move || {
        let n = counted.fetch_add(1, Ordering::SeqCst);
        Ok(Secret::new(tokens[n.min(tokens.len() - 1)].to_string()))
    });
    (api, reads)
}

#[test]
fn the_next_page_is_the_link_marked_next() {
    let link = r#"<https://api.github.com/repositories/1/pulls?page=2>; rel="next", <https://api.github.com/repositories/1/pulls?page=9>; rel="last""#;
    assert_eq!(next_link(link).as_deref(), Some("https://api.github.com/repositories/1/pulls?page=2"));
    let last_page = r#"<https://api.github.com/repositories/1/pulls?page=1>; rel="first", <https://api.github.com/repositories/1/pulls?page=8>; rel="prev""#;
    assert_eq!(next_link(last_page), None);
}

#[test]
fn paging_follows_next_links_on_this_api_and_no_link_elsewhere() {
    // The first page's link names the server's own address, as GitHub's names its own.
    let base = Arc::new(OnceLock::<String>::new());
    let known = base.clone();
    let server = serve(move |seen| match seen.target.as_str() {
        "/repos/o/r/pulls" => Reply {
            status: 200,
            headers: vec![("Link", format!(r#"<{}/repos/o/r/pulls?page=2>; rel="next""#, known.get().unwrap()))],
            body: "[1]".to_string(),
        },
        "/repos/o/r/pulls?page=2" => Reply {
            status: 200,
            headers: vec![("Link", r#"<https://elsewhere.example/steal?page=3>; rel="next""#.to_string())],
            body: "[2]".to_string(),
        },
        _ => Reply { status: 404, headers: Vec::new(), body: String::new() },
    });
    base.set(server.url.clone()).unwrap();
    let (api, _) = api(&server.url, &["t"]);
    let paged = api.get_paged("/repos/o/r/pulls", 10).unwrap();
    let rows: Vec<Vec<u8>> = paged.pages.iter().map(|page| page.decode::<Vec<u8>>("a page").unwrap()).collect();
    assert_eq!(rows, vec![vec![1], vec![2]]);
    assert!(!paged.cut, "a link off this API is not more of the list");
    let targets: Vec<String> = server.seen().into_iter().map(|seen| seen.target).collect();
    assert_eq!(targets, vec!["/repos/o/r/pulls", "/repos/o/r/pulls?page=2"]);
}

#[test]
fn a_list_longer_than_the_callers_cap_says_it_was_cut() {
    let base = Arc::new(OnceLock::<String>::new());
    let known = base.clone();
    let server = serve(move |seen| {
        let page: u32 = seen.target.rsplit_once("page=").map_or(1, |(_, n)| n.parse().unwrap());
        Reply {
            status: 200,
            headers: vec![("Link", format!(r#"<{}/repos/o/r/pulls?page={}>; rel="next""#, known.get().unwrap(), page + 1))],
            body: format!("[{page}]"),
        }
    });
    base.set(server.url.clone()).unwrap();
    let (api, _) = api(&server.url, &["t"]);
    let paged = api.get_paged("/repos/o/r/pulls", 3).unwrap();
    assert_eq!(paged.pages.len(), 3);
    assert!(paged.cut);
    assert_eq!(server.seen().len(), 3, "the page past the cap is not read");
}

#[test]
fn github_com_is_this_adapter_and_another_host_is_not() {
    let here = ForgeAddress::of_remote("git@github.com:Mele-Labs/armada.git").unwrap();
    let repository = GitHubRepository::of(here).unwrap();
    assert_eq!(repository.path(), "/repos/mele-labs/armada");

    let elsewhere = ForgeAddress::of_remote("https://git.example.com/team/tool.git").unwrap();
    match GitHubRepository::of(elsewhere) {
        Err(NotGitHub::Elsewhere(address)) => assert_eq!(address.host(), "git.example.com"),
        other => panic!("another host was taken as GitHub: {other:?}"),
    }
}

#[derive(Serialize)]
struct Number {
    number: u64,
}

#[derive(Debug, Deserialize, PartialEq)]
struct Viewer {
    login: String,
}

#[test]
fn graphql_errors_are_a_failure_even_with_a_200() {
    let server = serve(|seen| {
        if seen.body.contains("broken") {
            ok(r#"{"data":null,"errors":[{"type":"NOT_FOUND","message":"Could not resolve to a PullRequest."}]}"#)
        } else {
            ok(r#"{"data":{"login":"octo"}}"#)
        }
    });
    let (api, _) = api(&server.url, &["t"]);

    let found: Viewer = api.graphql("query { fine }", &Number { number: 7 }).unwrap();
    assert_eq!(found, Viewer { login: "octo".to_string() });
    let refused = api.graphql::<Viewer, _>("query { broken }", &Number { number: 7 });
    assert_eq!(refused.unwrap_err(), ApiFailure::GraphQl(vec!["Could not resolve to a PullRequest.".to_string()]));

    let sent = &server.seen()[0];
    assert_eq!((sent.method.as_str(), sent.target.as_str()), ("POST", "/graphql"));
    assert_eq!(sent.body, r#"{"query":"query { fine }","variables":{"number":7}}"#);
}

#[test]
fn a_401_reads_the_token_again_and_retries_once() {
    let server = serve(|seen| match seen.authorization.as_deref() {
        Some("Bearer old") => Reply { status: 401, headers: Vec::new(), body: r#"{"message":"Bad credentials"}"#.to_string() },
        _ => ok("{}"),
    });
    let (api, reads) = api(&server.url, &["old", "new"]);
    let answer = api.get("/user").unwrap();
    assert_eq!(answer.status, 200);
    let sent: Vec<Option<String>> = server.seen().into_iter().map(|seen| seen.authorization).collect();
    assert_eq!(sent, vec![Some("Bearer old".to_string()), Some("Bearer new".to_string())]);
    assert_eq!(reads.load(Ordering::SeqCst), 2);

    // The new token is kept: the next call reads none.
    api.get("/user").unwrap();
    assert_eq!(reads.load(Ordering::SeqCst), 2);
}

#[test]
fn a_second_401_is_refused_without_a_third_try() {
    let server = serve(|_| Reply { status: 401, headers: Vec::new(), body: r#"{"message":"Bad credentials"}"#.to_string() });
    let (api, reads) = api(&server.url, &["old", "new"]);
    let refused = api.get("/user").unwrap_err();
    assert_eq!(refused, ApiFailure::Refused { status: 401, said: "Bad credentials".to_string(), limit: None });
    assert_eq!(server.seen().len(), 2);
    assert_eq!(reads.load(Ordering::SeqCst), 2);
}

#[test]
fn a_rate_limited_answer_says_how_long_to_wait() {
    let server = serve(|_| Reply {
        status: 429,
        headers: vec![
            ("x-ratelimit-remaining", "0".to_string()),
            ("x-ratelimit-reset", "1700000000".to_string()),
            ("retry-after", "60".to_string()),
        ],
        body: r#"{"message":"API rate limit exceeded"}"#.to_string(),
    });
    let (api, _) = api(&server.url, &["t"]);
    let refused = api.get("/repos/o/r").unwrap_err();
    assert_eq!(
        refused,
        ApiFailure::Refused {
            status: 429,
            said: "API rate limit exceeded".to_string(),
            limit: Some(RateLimit { remaining: Some(0), reset_at: Some(1_700_000_000), retry_after: Some(60) }),
        }
    );
    assert_eq!(server.seen().len(), 1, "a rate limit is not retried here");
}

#[test]
fn threads_sharing_one_client_reuse_its_connections() {
    const THREADS: usize = 6;
    const EACH: usize = 5;
    let server = serve(|_| ok("{}"));
    let (api, reads) = api(&server.url, &["t"]);
    let threads: Vec<_> = (0..THREADS)
        .map(|_| {
            let api = api.clone();
            std::thread::spawn(move || {
                for _ in 0..EACH {
                    api.get("/rate_limit").unwrap();
                }
            })
        })
        .collect();
    for thread in threads {
        thread.join().unwrap();
    }
    let connections = server.connections.load(Ordering::SeqCst);
    assert_eq!(server.seen().len(), THREADS * EACH);
    assert!(connections <= THREADS, "{connections} connections for {} requests", THREADS * EACH);
    assert_eq!(reads.load(Ordering::SeqCst), 1, "the token is read once for every thread");
}
