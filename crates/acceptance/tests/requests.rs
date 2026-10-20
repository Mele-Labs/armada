//! Requests' claim: **a header the owner kept for a host, a cookie or an `Authorization: Bearer`
//! token or any other, goes out on an agent's request to that host, and nothing the agent reads
//! holds it.**
//!
//! The server is a stand-in that does the worst a server can: it echoes the credential it
//! received in its body, the bare token of a `Bearer` among it, and sets a cookie back. The header
//! store is held in memory. Nothing opens a socket and nothing touches a Keychain.
//!
//! | Proved | Not proved |
//! |---|---|
//! | The kept header goes out on a request to its host, and `cookie_host` and `kept_header` name the host and the header | That `reqwest` sends the header on the wire. `adapters::tests::requests` against a socket on this machine |
//! | The answer the agent reads, headers and body, holds no part of the value, the token of a `Bearer` included, and no `set-cookie` | That the header is kept in the Keychain. `adapters::tests::request_cookies`, in a throwaway keychain |
//! | The thread's row holds no part of it, and no header of its name | That the row is written in the Job's thread and the owner's send is told to its agent. `fleet::tests::requesting` |
//! | The words an owner's send tells the Job hold no part of it | That an agent reaches it through its door. `api::tests::served` |
//! | A request to another host, or one carrying a header of the kept name in any case, sends no kept header | |

use std::collections::BTreeMap;
use std::sync::Mutex;

use adapter_traits::{
    CookieError, Incoming, KeptHeader, KeptHost, Outgoing, RequestCookies, RequestFailure,
    RequestSender, Secret,
};
use fleet::requesting::{sent, Bounds};
use ipc::{RequestBy, RequestHeader, SendRequest};

const HOST: &str = "api.example.test";
const COOKIE: &str = "session=s3cr3t-0f-the-0wner; theme=dark";
const BEARER: &str = "Bearer t0ken-0f-the-0wner-1234";
/// The part of an `Authorization` value a server is likeliest to echo on its own.
const TOKEN: &str = "t0ken-0f-the-0wner-1234";
/// The part of the value a server is likeliest to echo on its own.
const SESSION: &str = "s3cr3t-0f-the-0wner";

/// The headers the owner kept, by host: the header's name and its value.
#[derive(Default)]
struct Held(Mutex<BTreeMap<String, (String, String)>>);

impl RequestCookies for Held {
    fn kept(&self, host: &str) -> Result<Option<KeptHeader>, CookieError> {
        Ok(self
            .0
            .lock()
            .unwrap()
            .get(host)
            .map(|(header, value)| KeptHeader { header: header.clone(), value: Secret::new(value.clone()) }))
    }

    fn keep(&self, host: &str, header: &str, value: Secret<String>) -> Result<(), CookieError> {
        self.0.lock().unwrap().insert(host.to_string(), (header.to_string(), value.expose().clone()));
        Ok(())
    }

    fn forget(&self, host: &str) -> Result<(), CookieError> {
        self.0.lock().unwrap().remove(host);
        Ok(())
    }

    fn hosts(&self) -> Result<Vec<KeptHost>, CookieError> {
        Ok(self
            .0
            .lock()
            .unwrap()
            .iter()
            .map(|(host, (header, _))| KeptHost { host: host.clone(), header: header.clone() })
            .collect())
    }
}

/// What a server was sent: the address, and the credential header it carried, name and value.
type Went = (String, Option<(String, String)>);

/// A server that echoes the credential it was sent, whole and its last word alone, and sets a
/// cookie back. Keeps what it was sent.
#[derive(Default)]
struct Echo(Mutex<Vec<Went>>);

impl RequestSender for Echo {
    fn send(&self, request: Outgoing, _most: usize) -> Result<Incoming, RequestFailure> {
        let credential = request
            .kept
            .as_ref()
            .map(|kept| (kept.header.clone(), kept.value.expose().clone()))
            .or_else(|| {
                request
                    .headers
                    .iter()
                    .find(|(name, _)| ["cookie", "authorization"].contains(&name.to_ascii_lowercase().as_str()))
                    .cloned()
            });
        self.0.lock().unwrap().push((request.url.clone(), credential.clone()));
        let said = credential.map(|(_, value)| value).unwrap_or_default();
        let last = said.rsplit(' ').next().unwrap_or_default().to_string();
        Ok(Incoming {
            status: 200,
            headers: vec![
                ("content-type".to_string(), "application/json".to_string()),
                ("set-cookie".to_string(), format!("session={SESSION}; Path=/")),
                ("x-echo".to_string(), said.clone()),
            ],
            body: format!(r#"{{"you_sent":"{said}","last_word":"{last}","session":"{SESSION}"}}"#).into_bytes(),
            truncated: false,
        })
    }
}

fn held() -> Held {
    let held = Held::default();
    held.keep(HOST, "Cookie", Secret::new(COOKIE.to_string())).unwrap();
    held
}

fn held_bearer() -> Held {
    let held = Held::default();
    held.keep(HOST, "Authorization", Secret::new(BEARER.to_string())).unwrap();
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
    let credential = Some(("Cookie".to_string(), COOKIE.to_string()));
    assert_eq!(went, vec![("https://api.example.test/v1/me".to_string(), credential)]);
    assert_eq!(done.answer.cookie_host.as_deref(), Some(HOST));
    assert_eq!(done.answer.kept_header.as_deref(), Some("Cookie"));
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
    assert!(!row.to_ascii_lowercase().contains("\"name\":\"cookie\""), "the row keeps a Cookie header: {row}");
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
    assert_eq!(went[1].1, Some(("Cookie".to_string(), "mine=1".to_string())));
}

/// Every piece of the bearer a reader could recognise it by.
fn holds_the_bearer(text: &str) -> bool {
    text.contains(BEARER) || text.contains(TOKEN)
}

fn bearer_request() -> SendRequest {
    get("https://api.example.test/v1/me", Vec::new())
}

#[test]
fn a_kept_authorization_goes_out_on_an_agents_request_to_its_host() {
    let (kept, server) = (held_bearer(), Echo::default());
    let done = sent(&server, &kept, RequestBy::Agent, &bearer_request(), &[], &Bounds::shipped()).expect("sent");
    let went = server.0.lock().unwrap().clone();
    let credential = Some(("Authorization".to_string(), BEARER.to_string()));
    assert_eq!(went, vec![("https://api.example.test/v1/me".to_string(), credential)]);
    assert_eq!(done.answer.cookie_host.as_deref(), Some(HOST));
    assert_eq!(done.answer.kept_header.as_deref(), Some("Authorization"));
}

#[test]
fn nothing_the_agent_reads_holds_the_kept_authorization() {
    let (kept, server) = (held_bearer(), Echo::default());
    let done = sent(&server, &kept, RequestBy::Agent, &bearer_request(), &[], &Bounds::shipped()).expect("sent");
    let answer = ipc::encode(&done.answer).expect("encodes");
    assert!(answer.contains("last_word"), "the echo came back: {answer}");
    assert!(!holds_the_bearer(&answer), "the answer holds the token: {answer}");
    let row = ipc::encode(&done.row("r1".to_string(), ipc::Instant::carried("2026-10-19T09:00:00Z")))
        .expect("encodes");
    assert!(!holds_the_bearer(&row), "the row holds the token: {row}");
    assert!(!row.to_ascii_lowercase().contains("\"name\":\"authorization\""), "the row keeps an Authorization header: {row}");
    let owner = sent(&server, &kept, RequestBy::Owner, &bearer_request(), &[], &Bounds::shipped()).expect("sent");
    let told = owner.told();
    assert!(told.contains("The kept Authorization header for api.example.test went with it."), "{told}");
    assert!(!holds_the_bearer(&told), "the Job is told the token: {told}");
    assert!(!holds_the_bearer(&ipc::encode(&owner.answer).expect("encodes")), "the owner reads the token");
}

#[test]
fn a_header_of_the_kept_name_in_any_case_sends_no_kept_header() {
    let (kept, server) = (held_bearer(), Echo::default());
    let own = vec![RequestHeader { name: "authorization".to_string(), value: "Bearer mine".to_string() }];
    let done = sent(&server, &kept, RequestBy::Agent, &get("https://api.example.test/", own), &[], &Bounds::shipped())
        .expect("sent");
    assert_eq!(done.answer.cookie_host, None);
    assert_eq!(done.answer.kept_header, None);
    let went = server.0.lock().unwrap().clone();
    assert_eq!(went[0].1, Some(("authorization".to_string(), "Bearer mine".to_string())));
}

/// A server that refuses everyone.
struct Denies;

impl RequestSender for Denies {
    fn send(&self, _request: Outgoing, _most: usize) -> Result<Incoming, RequestFailure> {
        Ok(Incoming { status: 401, headers: Vec::new(), body: Vec::new(), truncated: false })
    }
}

#[test]
fn a_401_from_a_host_holding_no_header_says_the_owner_can_add_one() {
    let note = |kept: &Held, headers: Vec<RequestHeader>| {
        sent(&Denies, kept, RequestBy::Agent, &get("https://api.example.test/", headers), &[], &Bounds::shipped())
            .expect("sent")
            .answer
            .note
    };
    assert_eq!(
        note(&Held::default(), Vec::new()).as_deref(),
        Some("api.example.test holds no kept header. The owner can add a header for it in Requests.")
    );
    assert_eq!(note(&held_bearer(), Vec::new()), None, "a header went");
    let own = vec![RequestHeader { name: "Authorization".to_string(), value: "Bearer mine".to_string() }];
    assert_eq!(note(&Held::default(), own), None, "the request carried its own");
}

#[test]
fn only_a_cookie_is_hidden_piece_by_piece_and_any_other_header_whole_with_its_token() {
    use fleet::requesting::hidden;
    assert_eq!(hidden("a s3cr3t-0f-the-0wner b", "Cookie", COOKIE), "a [cookie] b");
    assert_eq!(hidden("t0ken-0f-the-0wner-1234 and Bearer t0ken-0f-the-0wner-1234", "Authorization", BEARER), "[header] and [header]");
    // A value with no scheme is hidden whole, and `dark` of a cookie's pair is left in a body.
    assert_eq!(hidden("key k3y-v4lue-99", "X-Api-Key", "k3y-v4lue-99"), "key [header]");
    assert_eq!(hidden("theme dark", "Cookie", COOKIE), "theme dark");
}
