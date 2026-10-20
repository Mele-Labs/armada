//! Requests' claim: **a cookie the owner gave for a host goes out on an agent's request to that
//! host, and nothing the agent reads holds it.**
//!
//! The server is a stand-in that does the worst a server can: it echoes the `Cookie` header it
//! received in its body and sets a cookie back. The cookie store is held in memory. Nothing opens a
//! socket and nothing touches a Keychain.
//!
//! | Proved | Not proved |
//! |---|---|
//! | The held cookie goes out on a request to its host, and `cookie_host` names the host | That `reqwest` sends the header on the wire. `adapters::tests::requests` against a socket on this machine |
//! | The answer the agent reads, headers and body, holds no part of the value, and no `set-cookie` | That the cookie is kept in the Keychain. `adapters::tests::request_cookies`, in a throwaway keychain |
//! | The thread's row holds no part of it, and no `Cookie` header | That the row is written in the Job's thread and the owner's send is told to its agent. `fleet::tests::requesting` |
//! | The words an owner's send tells the Job hold no part of it | That an agent reaches it through its door. `api::tests::served` |
//! | A request to another host, or one carrying its own `Cookie`, sends no held cookie | |

use std::collections::BTreeMap;
use std::sync::Mutex;

use adapter_traits::{
    CookieError, Incoming, Outgoing, RequestCookies, RequestFailure, RequestSender, Secret,
};
use fleet::requesting::{sent, Bounds};
use ipc::{RequestBy, RequestHeader, SendRequest};

const HOST: &str = "api.example.test";
const COOKIE: &str = "session=s3cr3t-0f-the-0wner; theme=dark";
/// The part of the value a server is likeliest to echo on its own.
const SESSION: &str = "s3cr3t-0f-the-0wner";

/// The cookies the owner gave, by host.
#[derive(Default)]
struct Held(Mutex<BTreeMap<String, String>>);

impl RequestCookies for Held {
    fn cookie(&self, host: &str) -> Result<Option<Secret<String>>, CookieError> {
        Ok(self.0.lock().unwrap().get(host).cloned().map(Secret::new))
    }

    fn keep(&self, host: &str, cookie: Secret<String>) -> Result<(), CookieError> {
        self.0.lock().unwrap().insert(host.to_string(), cookie.expose().clone());
        Ok(())
    }

    fn forget(&self, host: &str) -> Result<(), CookieError> {
        self.0.lock().unwrap().remove(host);
        Ok(())
    }

    fn hosts(&self) -> Result<Vec<String>, CookieError> {
        Ok(self.0.lock().unwrap().keys().cloned().collect())
    }
}

/// A server that echoes the `Cookie` it was sent, and sets one back. Keeps what it was sent.
#[derive(Default)]
struct Echo(Mutex<Vec<(String, Option<String>)>>);

impl RequestSender for Echo {
    fn send(&self, request: Outgoing, _most: usize) -> Result<Incoming, RequestFailure> {
        let cookie = request
            .cookie
            .as_ref()
            .map(|cookie| cookie.expose().clone())
            .or_else(|| {
                request
                    .headers
                    .iter()
                    .find(|(name, _)| name.eq_ignore_ascii_case("cookie"))
                    .map(|(_, value)| value.clone())
            });
        self.0.lock().unwrap().push((request.url.clone(), cookie.clone()));
        let said = cookie.unwrap_or_default();
        Ok(Incoming {
            status: 200,
            headers: vec![
                ("content-type".to_string(), "application/json".to_string()),
                ("set-cookie".to_string(), format!("session={SESSION}; Path=/")),
                ("x-echo".to_string(), said.clone()),
            ],
            body: format!(r#"{{"you_sent":"{said}","session":"{SESSION}"}}"#).into_bytes(),
            truncated: false,
        })
    }
}

fn held() -> Held {
    let held = Held::default();
    held.keep(HOST, Secret::new(COOKIE.to_string())).unwrap();
    held
}

fn get(url: &str, headers: Vec<RequestHeader>) -> SendRequest {
    SendRequest {
        session_id: None,
        method: "GET".to_string(),
        url: url.to_string(),
        environment: None,
        headers,
        body: None,
    }
}

/// Every piece of the value a reader could recognise it by.
fn holds_the_cookie(text: &str) -> bool {
    text.contains(COOKIE) || text.contains(SESSION)
}

#[test]
fn the_held_cookie_goes_out_on_an_agents_request_to_its_host() {
    let (cookies, server) = (held(), Echo::default());
    let done = sent(&server, &cookies, RequestBy::Agent, &get("https://api.example.test/v1/me", Vec::new()), &[], &Bounds::shipped())
        .expect("sent");
    let went = server.0.lock().unwrap().clone();
    assert_eq!(went, vec![("https://api.example.test/v1/me".to_string(), Some(COOKIE.to_string()))]);
    assert_eq!(done.answer.cookie_host.as_deref(), Some(HOST));
    assert_eq!(done.answer.status, Some(200));
}

#[test]
fn nothing_the_agent_reads_holds_the_cookie() {
    let (cookies, server) = (held(), Echo::default());
    let done = sent(&server, &cookies, RequestBy::Agent, &get("https://api.example.test/v1/me", Vec::new()), &[], &Bounds::shipped())
        .expect("sent");
    let answer = ipc::encode(&done.answer).expect("encodes");
    assert!(!holds_the_cookie(&answer), "the answer holds the cookie: {answer}");
    assert!(
        !done.answer.headers.iter().any(|header| header.name.eq_ignore_ascii_case("set-cookie")),
        "the agent reads a set-cookie: {:?}",
        done.answer.headers
    );
    let row = ipc::encode(&done.row("r1".to_string(), ipc::Instant::carried("2026-10-19T09:00:00Z")))
        .expect("encodes");
    assert!(!holds_the_cookie(&row), "the row holds the cookie: {row}");
    assert!(!row.to_ascii_lowercase().contains("\"cookie\""), "the row keeps a Cookie header: {row}");
}

#[test]
fn the_words_an_owners_send_tells_the_job_hold_no_cookie() {
    let (cookies, server) = (held(), Echo::default());
    let done = sent(&server, &cookies, RequestBy::Owner, &get("https://api.example.test/v1/me", Vec::new()), &[], &Bounds::shipped())
        .expect("sent");
    let told = done.told();
    assert!(told.starts_with("Owner sent `GET https://api.example.test/v1/me` from Requests. 200 in "), "{told}");
    assert!(!holds_the_cookie(&told), "the Job is told the cookie: {told}");
    let answer = ipc::encode(&done.answer).expect("encodes");
    assert!(!holds_the_cookie(&answer), "the owner's answer holds the cookie: {answer}");
}

#[test]
fn another_host_or_a_cookie_of_its_own_sends_no_held_cookie() {
    let (cookies, server) = (held(), Echo::default());
    let elsewhere = sent(&server, &cookies, RequestBy::Agent, &get("https://other.example.test/", Vec::new()), &[], &Bounds::shipped())
        .expect("sent");
    assert_eq!(elsewhere.answer.cookie_host, None);
    let own = vec![RequestHeader { name: "Cookie".to_string(), value: "mine=1".to_string() }];
    let carried = sent(&server, &cookies, RequestBy::Agent, &get("https://api.example.test/", own), &[], &Bounds::shipped())
        .expect("sent");
    assert_eq!(carried.answer.cookie_host, None);
    let went = server.0.lock().unwrap().clone();
    assert_eq!(went[0].1, None);
    assert_eq!(went[1].1.as_deref(), Some("mine=1"));
}
