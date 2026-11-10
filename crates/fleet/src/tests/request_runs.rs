//! A saved request's runs through a real Fleet: every send of a saved request is kept, the owner's
//! and a Job's agent's, whether or not it answered; the agent's outlives its Job; a request keeps its
//! newest runs and no secret's value is in one; the reads filter, page and answer an agent as an
//! agent's own send is answered. A request's own method and address keeps none.

use std::sync::Arc;

use adapter_traits::{Incoming, Outgoing, RequestFailure, RequestSender};
use api::{HostedSessions, Refusal, Requests, RunsAsked};
use ipc::{
    RequestBy, RequestCollection, RequestEnvironmentWrite, RequestHeader, RequestRunStatus, RequestToSend,
    SaveRequestCollection, SaveRequestEnvironment, SavedRequest, SavedRequestNamed, SendRequest, SessionId, VariableWrite,
};

use super::session_host::{rig_fitted, Rig};

const TOKEN: &str = "t0ken-0f-the-0wner-1234";

/// A server on no socket. It answers with the status the `x-status` header names (200 where none),
/// refuses where `x-fail` is present, and echoes the `Authorization` it was sent, so a secret has
/// somewhere to leak. `x-size` asks for that many bytes of body in place of the echo.
struct Answering;

impl RequestSender for Answering {
    fn send(&self, request: Outgoing, _most: usize) -> Result<Incoming, RequestFailure> {
        let header = |name: &str| request.headers.iter().find(|(one, _)| one.eq_ignore_ascii_case(name)).map(|(_, value)| value.clone());
        if header("x-fail").is_some() {
            return Err(RequestFailure::Unreachable(format!("connection refused for {}", request.url)));
        }
        let body = match header("x-size").and_then(|size| size.parse::<usize>().ok()) {
            Some(size) => "x".repeat(size),
            None => format!("you sent {}", header("authorization").unwrap_or_default()),
        };
        Ok(Incoming {
            status: header("x-status").and_then(|status| status.parse().ok()).unwrap_or(200),
            headers: vec![
                ("content-type".to_string(), "text/plain".to_string()),
                ("set-cookie".to_string(), "session=abc".to_string()),
            ],
            body: body.into_bytes(),
            truncated: false,
        })
    }
}

/// A rig whose person keeps `staging` (a `token` secret and a `base`) and a collection `users`
/// holding the saved request `me`, which sends the token and a JSON body.
async fn rig_saving() -> Rig {
    let rig = rig_fitted(|fit| fit.requests.sender = Arc::new(Answering));
    let environment = RequestEnvironmentWrite {
        name: "staging".to_string(),
        variables: vec![
            VariableWrite::Plain { name: "base".to_string(), value: "https://staging.example.test".to_string() },
            VariableWrite::Secret { name: "token".to_string(), value: Some(TOKEN.to_string()) },
        ],
    };
    let save = SaveRequestEnvironment { repository: Some(rig.root.clone()), session_id: None, environment };
    rig.fleet.save_request_environment(None, save).await.expect("kept");
    let collection = RequestCollection {
        id: "users".to_string(),
        name: "Users".to_string(),
        headers: Vec::new(),
        requests: vec![SavedRequest {
            id: "me".to_string(),
            name: "Me".to_string(),
            method: "POST".to_string(),
            url: "{{base}}/v1/me".to_string(),
            headers: vec![header("Authorization", "Bearer {{token}}")],
            body: Some("{\n  \"who\": \"{{token}}\"\n}".to_string()),
        }],
    };
    let save = SaveRequestCollection { repository: Some(rig.root.clone()), session_id: None, collection };
    rig.fleet.save_request_collection(None, save).await.expect("saved");
    rig
}

fn header(name: &str, value: &str) -> RequestHeader {
    RequestHeader { name: name.to_string(), value: value.to_string() }
}

/// The saved request `Users`/`Me`, named by its names, sent through `staging` with `extra` headers.
fn saved(job: Option<&SessionId>, extra: Vec<RequestHeader>) -> SendRequest {
    SendRequest {
        session_id: job.cloned(),
        repository: None,
        request: RequestToSend::Saved(SavedRequestNamed { collection: "Users".to_string(), request: "Me".to_string() }),
        environment: Some("staging".to_string()),
        headers: extra,
        body: None,
        body_file: None,
        body_inline: true,
        pick: None,
    }
}

fn own(url: &str) -> SendRequest {
    let mut send = saved(None, Vec::new());
    send.request = RequestToSend::Own { method: "GET".to_string(), url: url.to_string(), collection: None };
    send
}

async fn sent_by(rig: &Rig, by: RequestBy, send: SendRequest) {
    let send = SendRequest { repository: Some(rig.root.clone()), ..send };
    Arc::clone(&rig.fleet).send_request(None, by, send).await.expect("answered");
}

fn asking(rig: &Rig) -> RunsAsked {
    RunsAsked {
        repository: Some(rig.root.clone()),
        collection: "users".to_string(),
        request: "me".to_string(),
        ..RunsAsked::default()
    }
}

fn refused(refusal: Refusal) -> (u16, String) {
    (refusal.status(), refusal.error().code.clone())
}

#[tokio::test]
async fn a_send_of_a_saved_request_keeps_a_run_whole_and_no_secret_is_in_it() {
    let rig = rig_saving().await;
    sent_by(&rig, RequestBy::Owner, saved(None, Vec::new())).await;

    let listed = rig.fleet.list_request_runs(None, asking(&rig)).await.expect("listed");
    let [line] = &listed.runs[..] else { panic!("one run: {listed:?}") };
    assert_eq!((line.by, line.status, line.method.as_str()), (RequestBy::Owner, Some(200), "POST"));
    assert_eq!(line.url, "https://staging.example.test/v1/me");
    assert_eq!(line.summary, "{ \"who\": \"{{token}}\" }", "the body's start on one line, the secret by name");
    assert_eq!(listed.next, None);

    let run = rig.fleet.get_request_run(None, RequestBy::Owner, Some(rig.root.clone()), None, line.id.clone()).await.expect("read");
    assert_eq!((run.collection.as_str(), run.request.as_str(), run.environment.as_deref()), ("users", "me", Some("staging")));
    assert_eq!(run.sent.headers, vec![header("Authorization", "Bearer {{token}}")], "the request as sent, the secret by name");
    assert!(run.sent.body.as_deref().is_some_and(|body| body.contains("{{token}}")), "{:?}", run.sent.body);
    assert_eq!(run.answer.body, "you sent Bearer {{token}}");
    assert_eq!(run.answer.content_type.as_deref(), Some("text/plain"));
    assert_eq!(run.answer.bytes, "you sent Bearer t0ken-0f-the-0wner-1234".len() as u64, "what the answer weighed as it came");
    assert!(run.answer.headers.iter().any(|one| one.name == "set-cookie"), "the owner reads what the owner was sent");
    for text in [ipc::encode(&listed).unwrap(), ipc::encode(&run).unwrap()] {
        assert!(!text.contains(TOKEN), "{text}");
    }
}

#[tokio::test]
async fn a_request_of_its_own_and_a_send_refused_before_it_went_keep_no_run() {
    let rig = rig_saving().await;
    sent_by(&rig, RequestBy::Owner, own("https://staging.example.test/v1/other")).await;
    let mut unknown = saved(None, Vec::new());
    unknown.environment = Some("production".to_string());
    let refusal = Arc::clone(&rig.fleet)
        .send_request(None, RequestBy::Owner, SendRequest { repository: Some(rig.root.clone()), ..unknown })
        .await
        .expect_err("refused");
    assert_eq!(refusal.status(), 422);
    assert!(rig.fleet.list_request_runs(None, asking(&rig)).await.expect("listed").runs.is_empty());
}

#[tokio::test]
async fn an_agents_run_outlives_its_job_and_is_named_by_the_job() {
    let rig = rig_saving().await;
    let job = rig.start_titled(Some("Fix the login")).await;
    sent_by(&rig, RequestBy::Agent, saved(Some(&job), Vec::new())).await;

    Arc::clone(&rig.fleet).close_session(ipc::CloseSession { session_id: job.clone() }).await.expect("closed");
    let record = rig.fleet.store().lock().await.session(job.as_str()).unwrap().unwrap();
    assert_eq!(record.state, store::SessionState::Ended, "the Job is over");

    let listed = rig.fleet.list_request_runs(None, asking(&rig)).await.expect("listed");
    let [line] = &listed.runs[..] else { panic!("the run is kept: {listed:?}") };
    assert_eq!(line.by, RequestBy::Agent);
    let sender = line.job.clone().expect("the Job that sent it");
    assert_eq!((sender.id.as_str(), sender.title.as_deref()), (job.as_str(), Some("Fix the login")));
    let run = rig.fleet.get_request_run(None, RequestBy::Owner, Some(rig.root.clone()), None, line.id.clone()).await.expect("read");
    assert_eq!(run.answer.status, Some(200));
    assert!(!run.answer.headers.iter().any(|one| one.name == "set-cookie"), "the agent's own send was never given it");
}

#[tokio::test]
async fn a_request_keeps_its_newest_runs_and_the_oldest_is_the_one_dropped() {
    let rig = rig_saving().await;
    let first = {
        sent_by(&rig, RequestBy::Owner, saved(None, vec![header("x-status", "201")])).await;
        rig.fleet.list_request_runs(None, asking(&rig)).await.expect("listed").runs[0].id.clone()
    };
    for _ in 0..store::RUNS_KEPT {
        sent_by(&rig, RequestBy::Owner, saved(None, Vec::new())).await;
    }
    let mut every = asking(&rig);
    every.limit = Some(1000);
    let listed = rig.fleet.list_request_runs(None, every).await.expect("listed");
    assert_eq!(listed.runs.len(), store::RUNS_KEPT, "no more than the cap, and the page's own cap is 200");
    assert!(listed.runs.iter().all(|line| line.status == Some(200)), "the first, a 201, was the one dropped");
    let gone = rig.fleet.get_request_run(None, RequestBy::Owner, Some(rig.root.clone()), None, first).await.expect_err("gone");
    assert_eq!(refused(gone), (404, "fleet.no_such_request_run".to_string()));
}

#[tokio::test]
async fn the_filters_take_the_sender_and_the_status_class_and_a_page_says_where_the_next_begins() {
    let rig = rig_saving().await;
    let job = rig.start().await;
    sent_by(&rig, RequestBy::Owner, saved(None, Vec::new())).await;
    sent_by(&rig, RequestBy::Agent, saved(Some(&job), vec![header("x-status", "404")])).await;
    sent_by(&rig, RequestBy::Agent, saved(Some(&job), vec![header("x-status", "201")])).await;
    sent_by(&rig, RequestBy::Owner, saved(None, vec![header("x-fail", "yes")])).await;
    sent_by(&rig, RequestBy::Agent, saved(Some(&job), vec![header("x-status", "503")])).await;
    let statuses = |runs: &ipc::RequestRuns| runs.runs.iter().map(|line| line.status).collect::<Vec<_>>();

    let all = rig.fleet.list_request_runs(None, asking(&rig)).await.expect("listed");
    assert_eq!(statuses(&all), [Some(503), None, Some(201), Some(404), Some(200)], "newest first");
    let mut agents = asking(&rig);
    agents.by = Some(RequestBy::Agent);
    assert_eq!(statuses(&rig.fleet.list_request_runs(None, agents.clone()).await.expect("listed")), [Some(503), Some(201), Some(404)]);
    agents.status = Some(RequestRunStatus::ClientError);
    assert_eq!(statuses(&rig.fleet.list_request_runs(None, agents).await.expect("listed")), [Some(404)]);
    let mut failed = asking(&rig);
    failed.status = Some(RequestRunStatus::Failed);
    let failures = rig.fleet.list_request_runs(None, failed).await.expect("listed");
    assert_eq!(statuses(&failures), [None]);
    assert!(failures.runs[0].error.as_deref().is_some_and(|why| why.contains("connection refused")), "{failures:?}");
    let mut owners_successes = asking(&rig);
    owners_successes.by = Some(RequestBy::Owner);
    owners_successes.status = Some(RequestRunStatus::Success);
    assert_eq!(statuses(&rig.fleet.list_request_runs(None, owners_successes).await.expect("listed")), [Some(200)]);

    let mut page = asking(&rig);
    page.limit = Some(2);
    let first = rig.fleet.list_request_runs(None, page.clone()).await.expect("listed");
    assert_eq!(statuses(&first), [Some(503), None]);
    page.before = first.next.clone();
    let second = rig.fleet.list_request_runs(None, page.clone()).await.expect("listed");
    assert_eq!(statuses(&second), [Some(201), Some(404)]);
    page.before = second.next.clone();
    let last = rig.fleet.list_request_runs(None, page).await.expect("listed");
    assert_eq!((statuses(&last), last.next), (vec![Some(200)], None), "the last page names no next");
}

#[tokio::test]
async fn an_agent_reads_a_run_as_its_own_send_is_answered_and_a_bad_id_or_repository_is_refused() {
    let rig = rig_saving().await;
    sent_by(&rig, RequestBy::Owner, saved(None, vec![header("x-size", "40000")])).await;
    let id = rig.fleet.list_request_runs(None, asking(&rig)).await.expect("listed").runs[0].id.clone();
    let read = |by| rig.fleet.get_request_run(None, by, Some(rig.root.clone()), None, id.clone());

    let owner = read(RequestBy::Owner).await.expect("read");
    assert_eq!((owner.answer.body.len(), owner.answer.truncated, owner.answer.bytes), (40000, false, 40000));
    assert!(owner.answer.headers.iter().any(|one| one.name == "set-cookie"));
    let agent = read(RequestBy::Agent).await.expect("read");
    assert_eq!((agent.answer.body.len(), agent.answer.truncated), (32 * 1024, true), "cut where its own answer is cut");
    assert!(!agent.answer.headers.iter().any(|one| one.name == "set-cookie"), "and no set-cookie");

    let other = rig.fleet.get_request_run(None, RequestBy::Owner, Some("/not/served".to_string()), None, id.clone()).await;
    assert!(matches!(other, Err(Refusal::NotFound(_) | Refusal::Unacceptable(_))), "{other:?}");
    let word = rig.fleet.get_request_run(None, RequestBy::Owner, Some(rig.root.clone()), None, "latest".to_string()).await;
    assert_eq!(refused(word.expect_err("refused")), (422, "fleet.request_run_refused".to_string()));
    let none = rig.fleet.get_request_run(None, RequestBy::Owner, Some(rig.root.clone()), None, "99999".to_string()).await;
    assert_eq!(refused(none.expect_err("refused")), (404, "fleet.no_such_request_run".to_string()));
}
