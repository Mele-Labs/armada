//! Requests through a real Fleet hosting a stand-in agent, against a server on this machine that
//! echoes the `Cookie` or `Authorization` it was sent and sets a cookie back: the row in the Job's thread, the owner's send
//! told to the agent and the agent's not, the environments file, and the cookie's value nowhere a
//! reader looks.

use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use adapter_traits::{CookieError, KeptHeader, KeptHost, RequestCookies, Secret};
use api::Requests;
use ipc::{
    RequestBy, RequestEnvironment, RequestEnvironmentsFile, RequestEnvironmentsRead, SendRequest,
    SessionId, SessionRow,
};

use super::session_host::{rig_fitted, Rig};
use crate::tests::session_host::eventually;

const COOKIE: &str = "session=s3cr3t-0f-the-0wner";
const SESSION: &str = "s3cr3t-0f-the-0wner";

/// The headers the owner kept, in memory: a header's name and value by host. What every
/// fixture's Fleet holds.
#[derive(Default)]
pub(crate) struct Held(Mutex<BTreeMap<String, (String, String)>>);

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

/// `connections` requests answered with the `Cookie` each carried, echoed, and a cookie set back.
fn echoing(connections: usize) -> (String, JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let handle = std::thread::spawn(move || {
        for _ in 0..connections {
            let (mut stream, _) = listener.accept().unwrap();
            let (mut bytes, mut chunk) = (Vec::new(), [0u8; 1024]);
            while !bytes.windows(4).any(|four| four == b"\r\n\r\n") {
                let n = stream.read(&mut chunk).unwrap();
                if n == 0 {
                    break;
                }
                bytes.extend_from_slice(&chunk[..n]);
            }
            let head = String::from_utf8_lossy(&bytes).into_owned();
            let cookie = head
                .lines()
                .find_map(|line| {
                    let (name, value) = line.split_once(": ")?;
                    ["cookie", "authorization"].contains(&name.to_ascii_lowercase().as_str()).then_some(value)
                })
                .unwrap_or("")
                .to_string();
            let last = cookie.rsplit(' ').next().unwrap_or_default();
            let body = if cookie.contains(' ') && !cookie.contains('=') {
                format!("you sent {cookie}, token {last}")
            } else {
                format!("you sent {cookie}")
            };
            let answer = format!(
                "HTTP/1.1 200 OK\r\nSet-Cookie: session={SESSION}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            stream.write_all(answer.as_bytes()).unwrap();
        }
    });
    (url, handle)
}

const BEARER: &str = "Bearer t0ken-0f-the-0wner-1234";
const TOKEN: &str = "t0ken-0f-the-0wner-1234";

fn rig_holding_bearer() -> Rig {
    rig_fitted(|fittings| {
        let held = Held::default();
        held.keep("127.0.0.1", "Authorization", Secret::new(BEARER.to_string())).unwrap();
        fittings.requests.cookies = Arc::new(held);
    })
}

fn rig_holding() -> Rig {
    rig_fitted(|fittings| {
        let held = Held::default();
        held.keep("127.0.0.1", "Cookie", Secret::new(COOKIE.to_string())).unwrap();
        fittings.requests.cookies = Arc::new(held);
    })
}

fn get(url: &str, session: Option<&SessionId>) -> SendRequest {
    SendRequest {
        session_id: session.cloned(),
        method: "get".to_string(),
        url: url.to_string(),
        environment: None,
        headers: Vec::new(),
        body: None,
    }
}

fn holds_the_cookie(text: &str) -> bool {
    text.contains(SESSION)
}

#[tokio::test]
async fn an_owners_send_keeps_a_row_and_tells_the_agent_and_neither_holds_the_cookie() {
    let rig = rig_holding();
    let id = rig.start().await;
    let (url, server) = echoing(1);
    let answer = Arc::clone(&rig.fleet)
        .send_request(None, RequestBy::Owner, get(&format!("{url}/v1/me"), Some(&id)))
        .await
        .expect("answered");
    server.join().unwrap();
    assert_eq!(answer.status, Some(200));
    assert_eq!(answer.cookie_host.as_deref(), Some("127.0.0.1"));
    assert_eq!(answer.body, "you sent [cookie]", "the server was sent the cookie and the owner reads none of it");
    assert!(answer.headers.iter().any(|header| header.name == "set-cookie"), "the owner's answer keeps set-cookie");
    assert!(!holds_the_cookie(&ipc::encode(&answer).unwrap()));

    eventually(|| async { !rig.stand_in.process(0).sent.lock().unwrap().is_empty() }).await;
    let rows = rig.rows(&id).await;
    let [SessionRow::Requested { by, method, url: sent_to, status, cookie_host, response, .. }] = &rows[..] else {
        panic!("one requested row and no message from Fleet: {rows:?}");
    };
    assert_eq!((*by, method.as_str(), *status), (RequestBy::Owner, "GET", Some(200)));
    assert_eq!(sent_to, &format!("{url}/v1/me"));
    assert_eq!(cookie_host.as_deref(), Some("127.0.0.1"));
    assert_eq!(response, "you sent [cookie]");
    assert!(!holds_the_cookie(&ipc::encode(&rows).unwrap()));

    let told = rig.stand_in.process(0).sent.lock().unwrap().clone();
    assert_eq!(told.len(), 1, "told once: {told:?}");
    assert!(told[0].contains(&format!("Owner sent `GET {url}/v1/me` from Requests.")), "{told:?}");
    assert!(told[0].contains(" 200 in "), "{told:?}");
    assert!(!holds_the_cookie(&told[0]), "{told:?}");
}

#[tokio::test]
async fn an_owners_send_with_a_kept_authorization_keeps_a_row_and_tells_the_agent_and_neither_holds_the_token() {
    let rig = rig_holding_bearer();
    let id = rig.start().await;
    let (url, server) = echoing(1);
    let mut send = get(&format!("{url}/v1/me"), Some(&id));
    send.headers = vec![ipc::RequestHeader { name: "Accept".to_string(), value: "application/json".to_string() }];
    let answer = Arc::clone(&rig.fleet).send_request(None, RequestBy::Owner, send).await.expect("answered");
    server.join().unwrap();
    assert_eq!((answer.cookie_host.as_deref(), answer.kept_header.as_deref()), (Some("127.0.0.1"), Some("Authorization")));
    assert!(answer.body.starts_with("you sent [header]"), "the server was sent the header and the owner reads none of it: {}", answer.body);
    assert!(!ipc::encode(&answer).unwrap().contains(TOKEN));

    eventually(|| async { !rig.stand_in.process(0).sent.lock().unwrap().is_empty() }).await;
    let rows = rig.rows(&id).await;
    let [SessionRow::Requested { headers, cookie_host, kept_header, .. }] = &rows[..] else {
        panic!("one requested row: {rows:?}");
    };
    assert_eq!((cookie_host.as_deref(), kept_header.as_deref()), (Some("127.0.0.1"), Some("Authorization")));
    assert_eq!(headers.len(), 1, "the row keeps Accept and no Authorization: {headers:?}");
    assert!(!ipc::encode(&rows).unwrap().contains(TOKEN));

    let told = rig.stand_in.process(0).sent.lock().unwrap().clone();
    assert!(told[0].contains("The kept Authorization header for 127.0.0.1 went with it."), "{told:?}");
    assert!(!told[0].contains(TOKEN), "{told:?}");
}

#[tokio::test]
async fn an_agents_send_from_its_connection_keeps_a_row_and_tells_it_nothing() {
    let rig = rig_holding();
    let id = rig.start().await;
    rig.send(&id, "hello").await;
    eventually(|| async { rig.stand_in.process(0).sent.lock().unwrap().len() == 1 }).await;
    let caller = api::Caller::at("127.0.0.1:50999".parse().unwrap());
    let (url, server) = echoing(1);
    let answer = Arc::clone(&rig.fleet)
        .send_request(Some(caller), RequestBy::Agent, get(&url, None))
        .await
        .expect("answered");
    server.join().unwrap();
    assert_eq!(answer.body, "you sent [cookie]");
    assert!(!answer.headers.iter().any(|header| header.name.eq_ignore_ascii_case("set-cookie")));
    assert!(!holds_the_cookie(&ipc::encode(&answer).unwrap()));

    let rows = rig.rows(&id).await;
    let requested: Vec<&SessionRow> = rows.iter().filter(|row| matches!(row, SessionRow::Requested { .. })).collect();
    assert!(matches!(requested[..], [SessionRow::Requested { by: RequestBy::Agent, .. }]), "{rows:?}");
    assert!(!holds_the_cookie(&ipc::encode(&rows).unwrap()));
    assert_eq!(rig.stand_in.process(0).sent.lock().unwrap().len(), 1, "the agent holds the answer and is told nothing");
}

#[tokio::test]
async fn a_saved_environment_is_listed_and_fills_base() {
    let rig = rig_holding();
    let (url, server) = echoing(1);
    let file = RequestEnvironmentsFile {
        version: 1,
        environment: vec![RequestEnvironment { name: "local".to_string(), base: url.clone() }],
    };
    let saved = rig.fleet.save_request_environments(file.clone()).await.expect("saved");
    assert_eq!(saved, RequestEnvironmentsRead::Read { file: file.clone() });
    let listed = rig.fleet.list_request_environments(None, None).await.expect("listed");
    assert_eq!(listed.saved, RequestEnvironmentsRead::Read { file });
    assert!(listed.servers.is_empty());

    let mut send = get("{{base}}/health", None);
    send.environment = Some("local".to_string());
    let answer = Arc::clone(&rig.fleet).send_request(None, RequestBy::Owner, send).await.expect("answered");
    server.join().unwrap();
    assert_eq!(answer.url, format!("{url}/health"));
    assert_eq!(answer.status, Some(200));
}

#[tokio::test]
async fn a_save_that_breaks_a_rule_is_refused_and_a_send_that_cannot_go_is_refused() {
    let rig = rig_holding();
    let named = |name: &str, base: &str| RequestEnvironmentsFile {
        version: 1,
        environment: vec![RequestEnvironment { name: name.to_string(), base: base.to_string() }],
    };
    for file in [
        named("server:web", "http://localhost:3000"),
        named("", "http://localhost:3000"),
        named("staging", "ftp://staging.example.test"),
        named("staging", "https://staging.example.test/"),
    ] {
        assert!(
            matches!(rig.fleet.save_request_environments(file.clone()).await, Err(api::Refusal::Unacceptable(_))),
            "{file:?}"
        );
    }
    for send in [
        get("{{base}}/health", None),
        SendRequest { method: "TRACE".to_string(), ..get("http://127.0.0.1:9/", None) },
        get("file:///etc/hosts", None),
    ] {
        assert!(
            matches!(Arc::clone(&rig.fleet).send_request(None, RequestBy::Owner, send.clone()).await, Err(api::Refusal::Unacceptable(_))),
            "{send:?}"
        );
    }
}

#[tokio::test]
async fn a_refused_connection_is_an_answer_with_its_error() {
    let rig = rig_holding();
    let closed = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", closed.local_addr().unwrap());
    drop(closed);
    let answer = Arc::clone(&rig.fleet)
        .send_request(None, RequestBy::Agent, get(&url, None))
        .await
        .expect("an answer, not a refusal");
    assert_eq!(answer.status, None);
    assert!(answer.error.as_deref().is_some_and(|why| !why.is_empty()), "{answer:?}");
    assert!(!holds_the_cookie(&ipc::encode(&answer).unwrap()));
}

#[tokio::test]
async fn the_cookie_acts_answer_hosts_and_never_a_value() {
    let rig = rig_holding();
    let host = |host: &str, header: &str| ipc::RequestHost { host: host.to_string(), header: header.to_string() };
    let hosts = rig.fleet.list_request_hosts().await.expect("listed");
    assert_eq!(hosts.hosts, vec![host("127.0.0.1", "Cookie")]);
    let hosts = rig
        .fleet
        .set_request_cookie(ipc::SetRequestCookie {
            host: "api.example.test".to_string(),
            header: "Authorization".to_string(),
            value: "Bearer abc".to_string(),
        })
        .await
        .expect("kept");
    assert_eq!(hosts.hosts, vec![host("127.0.0.1", "Cookie"), host("api.example.test", "Authorization")]);
    let hosts = rig
        .fleet
        .forget_request_cookie(ipc::ForgetRequestCookie { host: "127.0.0.1".to_string() })
        .await
        .expect("forgotten");
    assert_eq!(hosts.hosts, vec![host("api.example.test", "Authorization")]);
}
