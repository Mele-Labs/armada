//! Requests' claim: **a secret variable `{{token}}` fills in on an agent's send, so the server is
//! sent its value, and nothing the agent reads holds it: not the answer, not the thread's row, not
//! the words an owner's send tells the Job, not an error. A commit or a package of the
//! environment carries the secret's name and never its value.**
//!
//! The server is a stand-in that does the worst a server can: it echoes the credential it was
//! sent, in a header and in its body, and the address it was sent to. The Keychain is held in
//! memory. Nothing opens a socket and nothing touches a Keychain.
//!
//! | Proved | Not proved |
//! |---|---|
//! | A `{{token}}` in a header and the address is sent as the Keychain's value | That `reqwest` sends it on the wire. `adapters::tests::requests` |
//! | The answer, its headers, body and address, holds the value nowhere and reads `{{token}}` | That the value is kept in the Keychain. `adapters::tests::request_secrets`, in a throwaway keychain |
//! | The thread's row and the words an owner's send tells the Job hold it nowhere | That the row is written in the Job's thread. `fleet::tests::requesting` |
//! | An error a sender words with the address in it holds it nowhere | That an agent reaches the send through its door. `api::tests::served` |
//! | A `{{name}}` with nothing to fill it is sent as written and named in `note` | |
//! | An environment saved with a secret's value keeps the value for the Keychain alone, and the file committed or packaged from it carries `secret = true` and the name | That the commit lands on a branch. `fleet::tests::committing` |

use std::collections::BTreeMap;
use std::sync::Mutex;

use adapter_traits::{
    Incoming, Outgoing, RequestFailure, RequestSecrets, RequestSender, Secret, SecretError,
    SecretVariable,
};
use fleet::requesting::{copied, environment_written, sent, written, Bounds, Filling, SecretChange};
use ipc::{
    RequestBy, RequestEnvironment, RequestEnvironmentWrite, RequestHeader, RequestToSend,
    RequestVariable, RequestsFile, SendRequest, VariableWrite,
};

/// The repository's key, as the Keychain's account starts.
const REPOSITORY: &str = "acme--shop";
const ENVIRONMENT: &str = "staging";
/// The secret's value: what the server must be sent and the agent must never read.
const TOKEN: &str = "t0ken-0f-the-0wner-1234";

/// The Keychain, in memory: a value by account.
#[derive(Default)]
struct Held(Mutex<BTreeMap<String, String>>);

fn account(variable: &SecretVariable) -> String {
    format!("{}/{}/{}", variable.repository, variable.environment, variable.name)
}

impl RequestSecrets for Held {
    fn read(&self, variable: &SecretVariable) -> Result<Option<Secret<String>>, SecretError> {
        Ok(self.0.lock().unwrap().get(&account(variable)).map(|value| Secret::new(value.clone())))
    }

    fn is_kept(&self, variable: &SecretVariable) -> Result<bool, SecretError> {
        Ok(self.0.lock().unwrap().contains_key(&account(variable)))
    }

    fn keep(&self, variable: &SecretVariable, value: Secret<String>) -> Result<(), SecretError> {
        self.0.lock().unwrap().insert(account(variable), value.expose().clone());
        Ok(())
    }

    fn forget(&self, variable: &SecretVariable) -> Result<(), SecretError> {
        self.0.lock().unwrap().remove(&account(variable));
        Ok(())
    }
}

/// What a server was sent: the address and its `Authorization`.
type Went = (String, Option<String>);

/// A server that echoes the `Authorization` and the address it was sent, and sets a cookie back.
#[derive(Default)]
struct Echo(Mutex<Vec<Went>>);

impl RequestSender for Echo {
    fn send(&self, request: Outgoing, _most: usize) -> Result<Incoming, RequestFailure> {
        let authorization = request
            .headers
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case("authorization"))
            .map(|(_, value)| value.clone());
        self.0.lock().unwrap().push((request.url.clone(), authorization.clone()));
        let said = authorization.unwrap_or_default();
        Ok(Incoming {
            status: 200,
            headers: vec![
                ("content-type".to_string(), "application/json".to_string()),
                ("set-cookie".to_string(), format!("echo={TOKEN}; Path=/")),
                ("x-echo".to_string(), said.clone()),
            ],
            body: format!(r#"{{"you_sent":"{said}","at":"{}"}}"#, request.url).into_bytes(),
            truncated: false,
        })
    }
}

/// A server nothing reaches, whose failure words the address it was sent to. Keeps the address.
#[derive(Default)]
struct Unreachable(Mutex<Vec<String>>);

impl RequestSender for Unreachable {
    fn send(&self, request: Outgoing, _most: usize) -> Result<Incoming, RequestFailure> {
        self.0.lock().unwrap().push(request.url.clone());
        Err(RequestFailure::Unreachable(format!("{} could not be reached", request.url)))
    }
}

/// Whether `server` was sent the token, so a test that finds it nowhere is not passing on a
/// value that never went.
fn went_with_the_token(server: &Echo) -> bool {
    server.0.lock().unwrap().iter().any(|(url, authorization)| {
        url.contains(TOKEN) && authorization.as_deref().is_some_and(|value| value.contains(TOKEN))
    })
}

/// The person's environment, saved with the secret's value as Bridge sends it: the value goes to
/// the Keychain, and the environment kept in the file holds the name alone.
fn saved(keychain: &Held) -> RequestEnvironment {
    let write = RequestEnvironmentWrite {
        name: ENVIRONMENT.to_string(),
        variables: vec![
            VariableWrite::Plain { name: "host".to_string(), value: "api.example.test".to_string() },
            VariableWrite::Secret { name: "token".to_string(), value: Some(TOKEN.to_string()) },
        ],
    };
    let (environment, changes) = environment_written(write, None);
    for change in changes {
        match change {
            SecretChange::Keep { name, value } => {
                let variable = SecretVariable {
                    repository: REPOSITORY.to_string(),
                    environment: ENVIRONMENT.to_string(),
                    name,
                };
                keychain.keep(&variable, value).unwrap();
            }
            SecretChange::Forget { .. } => panic!("nothing was kept before"),
        }
    }
    environment
}

fn filling(environment: RequestEnvironment) -> Filling {
    Filling { repository: Some(REPOSITORY.to_string()), collections: Vec::new(), environments: vec![environment] }
}

/// The agent's own request: the token in a header and in the address.
fn send() -> SendRequest {
    SendRequest {
        session_id: None,
        repository: None,
        request: RequestToSend::Own {
            method: "GET".to_string(),
            url: "https://{{host}}/v1/me?key={{token}}".to_string(),
            collection: None,
        },
        environment: Some(ENVIRONMENT.to_string()),
        headers: vec![RequestHeader { name: "Authorization".to_string(), value: "Bearer {{token}}".to_string() }],
        body: None,
    }
}

fn holds_the_token(text: &str) -> bool {
    text.contains(TOKEN)
}

#[test]
fn a_secret_fills_in_on_an_agents_send_and_the_server_is_sent_its_value() {
    let keychain = Held::default();
    let filling = filling(saved(&keychain));
    let server = Echo::default();
    let done = sent(&server, &keychain, RequestBy::Agent, &send(), &filling, &Bounds::shipped()).expect("sent");
    let went = server.0.lock().unwrap().clone();
    assert_eq!(
        went,
        vec![(format!("https://api.example.test/v1/me?key={TOKEN}"), Some(format!("Bearer {TOKEN}")))]
    );
    assert_eq!(done.answer.status, Some(200));
    assert_eq!(done.answer.url, "https://api.example.test/v1/me?key={{token}}");
    assert_eq!(done.answer.note, None, "everything was filled");
}

#[test]
fn nothing_the_agent_reads_holds_the_secret() {
    let keychain = Held::default();
    let filling = filling(saved(&keychain));
    let server = Echo::default();
    let done = sent(&server, &keychain, RequestBy::Agent, &send(), &filling, &Bounds::shipped()).expect("sent");
    assert!(went_with_the_token(&server), "the server was sent the token, and echoed it");
    let answer = ipc::encode(&done.answer).expect("encodes");
    assert!(answer.contains("you_sent"), "the echo came back: {answer}");
    assert!(!holds_the_token(&answer), "the answer holds the token: {answer}");
    assert!(answer.contains("Bearer {{token}}"), "the echo reads the name: {answer}");
    let row = ipc::encode(&done.row("r1".to_string(), ipc::Instant::carried("2026-10-21T09:00:00Z"))).expect("encodes");
    assert!(!holds_the_token(&row), "the row holds the token: {row}");
    assert!(row.contains("Bearer {{token}}"), "the row keeps the header as written: {row}");
}

#[test]
fn the_words_an_owners_send_tells_the_job_hold_no_secret() {
    let keychain = Held::default();
    let filling = filling(saved(&keychain));
    let server = Echo::default();
    let done = sent(&server, &keychain, RequestBy::Owner, &send(), &filling, &Bounds::shipped()).expect("sent");
    assert!(went_with_the_token(&server), "the server was sent the token, and echoed it");
    let told = done.told();
    assert!(
        told.starts_with("Owner sent `GET https://api.example.test/v1/me?key={{token}}` from Requests. 200 in "),
        "{told}"
    );
    assert!(!holds_the_token(&told), "the Job is told the token: {told}");
    assert!(!holds_the_token(&ipc::encode(&done.answer).expect("encodes")), "the owner reads the token");
}

#[test]
fn an_error_naming_the_address_holds_no_secret() {
    let keychain = Held::default();
    let filling = filling(saved(&keychain));
    let server = Unreachable::default();
    let done = sent(&server, &keychain, RequestBy::Agent, &send(), &filling, &Bounds::shipped()).expect("sent");
    assert!(server.0.lock().unwrap()[0].contains(TOKEN), "the address went filled");
    let error = done.answer.error.clone().expect("nothing answered");
    assert!(!holds_the_token(&error), "the error holds the token: {error}");
    assert!(error.contains("{{token}}"), "{error}");
    let row = ipc::encode(&done.row("r1".to_string(), ipc::Instant::carried("2026-10-21T09:00:00Z"))).expect("encodes");
    assert!(!holds_the_token(&row), "the row holds the token: {row}");
}

#[test]
fn a_name_nothing_fills_is_sent_as_written_and_named() {
    // The environment names the token, and this Keychain never kept it.
    let (server, unkept) = (Echo::default(), Held::default());
    let done = sent(&server, &unkept, RequestBy::Agent, &send(), &filling(saved(&Held::default())), &Bounds::shipped())
        .expect("sent");
    let went = server.0.lock().unwrap().clone();
    assert_eq!(went[0].1.as_deref(), Some("Bearer {{token}}"));
    assert_eq!(done.answer.note.as_deref(), Some("{{token}} was not filled."));
}

#[test]
fn a_commit_or_package_of_the_environment_carries_the_secrets_name_and_never_its_value() {
    let keychain = Held::default();
    let environment = saved(&keychain);
    assert_eq!(
        environment.variables,
        vec![
            RequestVariable::Plain { name: "host".to_string(), value: "api.example.test".to_string() },
            RequestVariable::Secret { name: "token".to_string() },
        ]
    );
    let own = RequestsFile { environments: vec![environment], ..RequestsFile::default() };
    let committed = copied(RequestsFile::default(), &own, &[], &[ENVIRONMENT.to_string()], false).expect("copied");
    let text = written(&committed).expect("written");
    assert!(!holds_the_token(&text), "the committed file holds the token: {text}");
    assert!(text.contains("name = \"token\"") && text.contains("secret = true"), "{text}");
}
