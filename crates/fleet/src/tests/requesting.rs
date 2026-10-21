//! Requests' sends through a real Fleet hosting a stand-in agent, against a server on this machine
//! that echoes the `Authorization` it was sent: a secret filled from the person's environment, the
//! row in the Job's thread, the owner's send told to the agent and the agent's not, and the
//! secret's value nowhere a reader looks. Then the fill itself, as a function of values.

use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use adapter_traits::{
    Incoming, Outgoing, RequestFailure, RequestSecrets, RequestSender, Secret, SecretError, SecretVariable,
};
use api::Requests;
use ipc::{
    RequestBy, RequestCollection, RequestEnvironment, RequestEnvironmentWrite, RequestHeader,
    RequestToSend, RequestVariable, SaveRequestEnvironment, SavedRequest, SavedRequestNamed,
    SendRequest, SessionId, SessionRow, VariableWrite,
};

use super::session_host::{rig_fitted, Rig};
use crate::requesting::{sent, Bounds, Filling, Unsent};
use crate::tests::session_host::eventually;

const TOKEN: &str = "t0ken-0f-the-0wner-1234";

/// The Keychain, in memory: a value by account. What every fixture's Fleet holds. `refusing`
/// makes every keep and forget fail, as a locked Keychain does.
#[derive(Default)]
pub(crate) struct Held {
    pub(crate) values: Mutex<BTreeMap<String, String>>,
    pub(crate) refusing: std::sync::atomic::AtomicBool,
}

fn account(variable: &SecretVariable) -> String {
    format!("{}/{}/{}", variable.repository, variable.environment, variable.name)
}

impl Held {
    fn refused(&self) -> Result<(), SecretError> {
        match self.refusing.load(std::sync::atomic::Ordering::SeqCst) {
            true => Err(SecretError::Unkept(String::from("the keychain is locked"))),
            false => Ok(()),
        }
    }

    /// Every account holding a value.
    pub(crate) fn accounts(&self) -> Vec<String> {
        self.values.lock().unwrap().keys().cloned().collect()
    }
}

impl RequestSecrets for Held {
    fn read(&self, variable: &SecretVariable) -> Result<Option<Secret<String>>, SecretError> {
        Ok(self.values.lock().unwrap().get(&account(variable)).map(|value| Secret::new(value.clone())))
    }

    fn is_kept(&self, variable: &SecretVariable) -> Result<bool, SecretError> {
        Ok(self.values.lock().unwrap().contains_key(&account(variable)))
    }

    fn keep(&self, variable: &SecretVariable, value: Secret<String>) -> Result<(), SecretError> {
        self.refused()?;
        self.values.lock().unwrap().insert(account(variable), value.expose().clone());
        Ok(())
    }

    fn forget(&self, variable: &SecretVariable) -> Result<(), SecretError> {
        self.refused()?;
        self.values.lock().unwrap().remove(&account(variable));
        Ok(())
    }
}

/// `connections` requests answered with the `Authorization` each carried, echoed.
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
            let said = head
                .lines()
                .find_map(|line| {
                    let (name, value) = line.split_once(": ")?;
                    name.eq_ignore_ascii_case("authorization").then_some(value)
                })
                .unwrap_or("")
                .to_string();
            let body = format!("you sent {said}");
            let answer = format!(
                "HTTP/1.1 200 OK\r\nSet-Cookie: echo={said}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            stream.write_all(answer.as_bytes()).unwrap();
        }
    });
    (url, handle)
}

/// A rig whose person keeps `staging` for its repository: `token` secret, `base` plain at `base`.
async fn rig_keeping(base: &str) -> Rig {
    let rig = rig_fitted(|_| {});
    let environment = RequestEnvironmentWrite {
        name: "staging".to_string(),
        variables: vec![
            VariableWrite::Plain { name: "base".to_string(), value: base.to_string() },
            VariableWrite::Secret { name: "token".to_string(), value: Some(TOKEN.to_string()) },
        ],
    };
    let save = SaveRequestEnvironment { repository: Some(rig.root.clone()), session_id: None, environment };
    rig.fleet.save_request_environment(None, save).await.expect("kept");
    rig
}

fn own(method: &str, url: &str, session: Option<&SessionId>) -> SendRequest {
    SendRequest {
        session_id: session.cloned(),
        repository: None,
        request: RequestToSend::Own { method: method.to_string(), url: url.to_string(), collection: None },
        environment: None,
        headers: Vec::new(),
        body: None,
    }
}

/// The agent's or the owner's send to `{{base}}/v1/me` with the token, from `session`.
fn with_the_token(session: Option<&SessionId>) -> SendRequest {
    let mut send = own("get", "{{base}}/v1/me", session);
    send.environment = Some("staging".to_string());
    send.headers = vec![RequestHeader { name: "Authorization".to_string(), value: "Bearer {{token}}".to_string() }];
    send
}

#[tokio::test]
async fn an_owners_send_fills_the_secret_keeps_a_row_and_tells_the_agent_and_none_holds_it() {
    let (url, server) = echoing(1);
    let rig = rig_keeping(&url).await;
    let id = rig.start().await;
    let answer = Arc::clone(&rig.fleet)
        .send_request(None, RequestBy::Owner, with_the_token(Some(&id)))
        .await
        .expect("answered");
    server.join().unwrap();
    assert_eq!(answer.status, Some(200));
    assert_eq!(answer.url, format!("{url}/v1/me"));
    assert_eq!(answer.body, "you sent Bearer {{token}}", "the server was sent the token and the owner reads its name");
    assert!(answer.headers.iter().any(|header| header.name == "set-cookie"), "the owner's answer keeps set-cookie");
    assert!(!ipc::encode(&answer).unwrap().contains(TOKEN));

    eventually(|| async { !rig.stand_in.process(0).sent.lock().unwrap().is_empty() }).await;
    let rows = rig.rows(&id).await;
    let [SessionRow::Requested { by, method, url: sent_to, status, headers, environment, response, .. }] = &rows[..] else {
        panic!("one requested row and no message from Fleet: {rows:?}");
    };
    assert_eq!((*by, method.as_str(), *status), (RequestBy::Owner, "GET", Some(200)));
    assert_eq!(sent_to, &format!("{url}/v1/me"));
    assert_eq!(environment.as_deref(), Some("staging"));
    assert_eq!(headers[0].value, "Bearer {{token}}");
    assert_eq!(response, "you sent Bearer {{token}}");
    assert!(!ipc::encode(&rows).unwrap().contains(TOKEN));

    let told = rig.stand_in.process(0).sent.lock().unwrap().clone();
    assert_eq!(told.len(), 1, "told once: {told:?}");
    assert!(told[0].contains(&format!("Owner sent `GET {url}/v1/me` from Requests.")), "{told:?}");
    assert!(!told[0].contains(TOKEN), "{told:?}");
}

#[tokio::test]
async fn an_agents_send_from_its_connection_keeps_a_row_and_tells_it_nothing() {
    let (url, server) = echoing(1);
    let rig = rig_keeping(&url).await;
    let id = rig.start().await;
    rig.send(&id, "hello").await;
    eventually(|| async { rig.stand_in.process(0).sent.lock().unwrap().len() == 1 }).await;
    let caller = api::Caller::at("127.0.0.1:50999".parse().unwrap());
    let answer = Arc::clone(&rig.fleet)
        .send_request(Some(caller), RequestBy::Agent, with_the_token(None))
        .await
        .expect("answered");
    server.join().unwrap();
    assert_eq!(answer.body, "you sent Bearer {{token}}", "the Job's repository's environment filled it");
    assert!(!answer.headers.iter().any(|header| header.name.eq_ignore_ascii_case("set-cookie")));

    let rows = rig.rows(&id).await;
    let requested: Vec<&SessionRow> = rows.iter().filter(|row| matches!(row, SessionRow::Requested { .. })).collect();
    assert!(matches!(requested[..], [SessionRow::Requested { by: RequestBy::Agent, .. }]), "{rows:?}");
    assert!(!ipc::encode(&rows).unwrap().contains(TOKEN));
    assert_eq!(rig.stand_in.process(0).sent.lock().unwrap().len(), 1, "the agent holds the answer and is told nothing");
}

#[tokio::test]
async fn a_send_that_cannot_go_is_refused_and_a_refused_connection_is_an_answer() {
    let rig = rig_keeping("http://127.0.0.1:9").await;
    let mut unknown = with_the_token(None);
    unknown.repository = Some(rig.root.clone());
    unknown.environment = Some("production".to_string());
    for send in [own("TRACE", "http://127.0.0.1:9/", None), own("GET", "file:///etc/hosts", None), unknown] {
        assert!(
            matches!(Arc::clone(&rig.fleet).send_request(None, RequestBy::Owner, send.clone()).await, Err(api::Refusal::Unacceptable(_))),
            "{send:?}"
        );
    }
    let closed = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", closed.local_addr().unwrap());
    drop(closed);
    let answer = Arc::clone(&rig.fleet).send_request(None, RequestBy::Agent, own("GET", &url, None)).await.expect("an answer");
    assert_eq!(answer.status, None);
    assert!(answer.error.as_deref().is_some_and(|why| !why.is_empty()), "{answer:?}");
}

// The fill, as a function of values.

/// A server that keeps what it was sent and answers 200 with no body.
#[derive(Default)]
struct Kept(Mutex<Vec<(String, Vec<(String, String)>, Option<String>)>>);

impl RequestSender for Kept {
    fn send(&self, request: Outgoing, _most: usize) -> Result<Incoming, RequestFailure> {
        self.0.lock().unwrap().push((request.url, request.headers, request.body));
        Ok(Incoming { status: 200, headers: Vec::new(), body: Vec::new(), truncated: false })
    }
}

fn header(name: &str, value: &str) -> RequestHeader {
    RequestHeader { name: name.to_string(), value: value.to_string() }
}

fn filling() -> Filling {
    Filling {
        repository: Some("acme--shop".to_string()),
        collections: vec![RequestCollection {
            id: "users".to_string(),
            name: "Users".to_string(),
            headers: vec![header("Accept", "application/json"), header("X-Team", "shop")],
            requests: vec![SavedRequest {
                id: "me".to_string(),
                name: "Me".to_string(),
                method: "post".to_string(),
                url: "{{base}}/v1/me".to_string(),
                headers: vec![header("x-team", "web")],
                body: Some("{\"who\":\"{{who}}\"}".to_string()),
            }],
        }],
        environments: vec![RequestEnvironment {
            name: "staging".to_string(),
            variables: vec![
                RequestVariable::Plain { name: "base".to_string(), value: "https://staging.example.test".to_string() },
                RequestVariable::Secret { name: "short".to_string() },
                RequestVariable::Secret { name: "token".to_string() },
            ],
        }],
    }
}

fn keychain() -> Held {
    let held = Held::default();
    for (name, value) in [("token", TOKEN), ("short", "abc")] {
        let variable = SecretVariable { repository: "acme--shop".into(), environment: "staging".into(), name: name.into() };
        held.keep(&variable, Secret::new(value.to_string())).unwrap();
    }
    held
}

fn saved(collection: &str, request: &str) -> SendRequest {
    SendRequest {
        session_id: None,
        repository: None,
        request: RequestToSend::Saved(SavedRequestNamed { collection: collection.to_string(), request: request.to_string() }),
        environment: Some("staging".to_string()),
        headers: Vec::new(),
        body: None,
    }
}

#[test]
fn a_saved_request_goes_as_kept_under_its_collections_headers_and_the_sends_go_over_both() {
    let server = Kept::default();
    let mut send = saved("Users", "me");
    send.headers = vec![header("ACCEPT", "text/plain")];
    let done = sent(&server, &keychain(), RequestBy::Owner, &send, &filling(), &Bounds::shipped()).expect("sent");
    let went = server.0.lock().unwrap().clone();
    let (url, headers, body) = &went[0];
    assert_eq!(url, "https://staging.example.test/v1/me");
    assert_eq!(headers, &vec![("x-team".to_string(), "web".to_string()), ("ACCEPT".to_string(), "text/plain".to_string())]);
    assert_eq!(body.as_deref(), Some("{\"who\":\"{{who}}\"}"), "an unfilled name goes as written");
    assert_eq!(done.answer.note.as_deref(), Some("{{who}} was not filled."));

    let mut replaced = saved("users", "Me");
    replaced.body = Some("{}".to_string());
    sent(&server, &keychain(), RequestBy::Owner, &replaced, &filling(), &Bounds::shipped()).expect("sent");
    assert_eq!(server.0.lock().unwrap()[1].2.as_deref(), Some("{}"), "the send's body replaces the kept one");
}

#[test]
fn an_own_request_takes_its_collections_headers_and_a_short_secret_is_not_hidden() {
    let server = Kept::default();
    let mut send = own("GET", "{{base}}/v1/me?short={{short}}", None);
    send.request = RequestToSend::Own {
        method: "GET".to_string(),
        url: "{{base}}/v1/me?short={{short}}".to_string(),
        collection: Some("users".to_string()),
    };
    send.environment = Some("staging".to_string());
    send.headers = vec![header("Authorization", "Bearer {{ token }}")];
    let done = sent(&server, &keychain(), RequestBy::Agent, &send, &filling(), &Bounds::shipped()).expect("sent");
    let went = server.0.lock().unwrap().clone();
    assert_eq!(went[0].0, "https://staging.example.test/v1/me?short=abc");
    assert!(went[0].1.contains(&("Accept".to_string(), "application/json".to_string())));
    assert!(went[0].1.contains(&("Authorization".to_string(), format!("Bearer {TOKEN}"))));
    assert_eq!(done.answer.url, "https://staging.example.test/v1/me?short=abc", "under six characters, deliberately left");
    let row = ipc::encode(&done.row("r1".into(), ipc::Instant::carried("2026-10-21T09:00:00Z"))).unwrap();
    assert!(row.contains("Bearer {{token}}") && !row.contains(TOKEN), "{row}");
}

#[test]
fn a_refusal_naming_a_filled_address_hides_the_secret() {
    let mut send = own("GET", "{{token}}/v1", None);
    send.environment = Some("staging".to_string());
    let refused = sent(&Kept::default(), &keychain(), RequestBy::Agent, &send, &filling(), &Bounds::shipped()).unwrap_err();
    let Unsent::Refused(why) = refused else { panic!("{refused:?}") };
    assert!(why.contains("{{token}}/v1 is not an http or https address") && !why.contains(TOKEN), "{why}");
    for missing in [saved("users", "nobody"), saved("orders", "me")] {
        assert!(matches!(sent(&Kept::default(), &keychain(), RequestBy::Agent, &missing, &filling(), &Bounds::shipped()), Err(Unsent::Refused(_))));
    }
}
