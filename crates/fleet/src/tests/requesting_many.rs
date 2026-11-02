//! `body_file` and `send_requests`: a body read only from the Job's own places, a template expanded
//! over files and values, the most in flight, the stop after a run of failures, and the secret's
//! value in no line and no saved file.

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use adapter_traits::{Incoming, Outgoing, RequestFailure, RequestSender, Secret, SecretVariable};
use api::{Refusal, Requests};
use ipc::{
    RequestBy, RequestEnvironment, RequestEnvironmentWrite, RequestHeader, RequestTemplate, RequestToSend,
    RequestVariable, RequestVary, SaveRequestEnvironment, SendRequest, SendRequests, SessionId, VariableWrite,
};

use super::requesting::Held;
use super::session_host::{rig_fitted, Rig};
use crate::requesting::batching::{expanded, run};
use crate::requesting::{Bounds, Filling};
use crate::tests::tmp::TempDir;

const TOKEN: &str = "t0ken-0f-the-0wner-1234";

/// A server that keeps every request it was sent, answers each with the status `status` names
/// for its turn and echoes the `Authorization` it carried, so a secret has somewhere to leak.
/// `pause` holds each request, so ones in flight together overlap.
struct Watching {
    seen: Mutex<Vec<Outgoing>>,
    turns: AtomicUsize,
    now: AtomicUsize,
    most: AtomicUsize,
    pause: Duration,
    status: Box<dyn Fn(usize) -> u16 + Send + Sync>,
}

impl Watching {
    fn answering(status: impl Fn(usize) -> u16 + Send + Sync + 'static, pause: Duration) -> Arc<Watching> {
        Arc::new(Watching {
            seen: Mutex::new(Vec::new()),
            turns: AtomicUsize::new(0),
            now: AtomicUsize::new(0),
            most: AtomicUsize::new(0),
            pause,
            status: Box::new(status),
        })
    }

    fn ok() -> Arc<Watching> {
        Watching::answering(|_| 200, Duration::ZERO)
    }
}

impl RequestSender for Watching {
    fn send(&self, request: Outgoing, _most: usize) -> Result<Incoming, RequestFailure> {
        let turn = self.turns.fetch_add(1, Ordering::SeqCst);
        let in_flight = self.now.fetch_add(1, Ordering::SeqCst) + 1;
        self.most.fetch_max(in_flight, Ordering::SeqCst);
        std::thread::sleep(self.pause);
        self.now.fetch_sub(1, Ordering::SeqCst);
        let said = request.headers.iter().find(|(name, _)| name.eq_ignore_ascii_case("authorization")).map(|(_, value)| value.clone());
        let body = format!("you sent {}", said.unwrap_or_default());
        self.seen.lock().unwrap().push(request);
        Ok(Incoming {
            status: (self.status)(turn),
            headers: vec![("content-type".to_string(), "application/json".to_string())],
            body: body.into_bytes(),
            truncated: false,
        })
    }
}

/// A rig whose Fleet sends through `sender`, and whose person keeps `staging` with a `token` secret.
async fn rig_sending_through(sender: &Arc<Watching>) -> Rig {
    let sender = Arc::clone(sender);
    let rig = rig_fitted(move |fit| fit.requests.sender = sender);
    let environment = RequestEnvironmentWrite {
        name: "staging".to_string(),
        variables: vec![
            VariableWrite::Plain { name: "base".to_string(), value: "https://staging.example.test".to_string() },
            VariableWrite::Secret { name: "token".to_string(), value: Some(TOKEN.to_string()) },
        ],
    };
    let save = SaveRequestEnvironment { repository: Some(rig.root.clone()), session_id: None, environment };
    rig.fleet.save_request_environment(None, save).await.expect("kept");
    rig
}

fn posting(body_file: &std::path::Path, job: &SessionId) -> SendRequest {
    SendRequest {
        session_id: Some(job.clone()),
        repository: None,
        request: RequestToSend::Own { method: "POST".to_string(), url: "{{base}}/v1/items".to_string(), collection: None },
        environment: Some("staging".to_string()),
        headers: Vec::new(),
        body: None,
        body_file: Some(body_file.to_string_lossy().into_owned()),
        body_inline: true,
        pick: None,
    }
}

async fn sent_from(rig: &Rig, send: SendRequest) -> Result<ipc::RequestSent, Refusal> {
    Arc::clone(&rig.fleet).send_request(None, RequestBy::Agent, send).await
}

fn refused(answer: &Result<ipc::RequestSent, Refusal>) -> bool {
    matches!(answer, Err(Refusal::Unacceptable(_)))
}

// `body_file`.

#[tokio::test]
async fn a_body_file_in_the_jobs_scratch_folder_is_sent_as_the_body() {
    let sender = Watching::ok();
    let rig = rig_sending_through(&sender).await;
    let id = rig.start().await;
    let file = rig.fleet.scratch_of(id.as_str()).join("one.json");
    std::fs::write(&file, "{\"n\":1}").unwrap();
    let answer = sent_from(&rig, posting(&file, &id)).await.expect("answered");
    assert_eq!(answer.status, Some(200));
    assert_eq!(sender.seen.lock().unwrap()[0].body.as_deref(), Some("{\"n\":1}"));
}

#[tokio::test]
async fn a_body_file_in_the_jobs_slot_is_read_and_one_beside_it_is_not() {
    let sender = Watching::ok();
    let rig = rig_sending_through(&sender).await;
    let id = rig.start().await;
    let mut hosting = rig.fleet.store().lock().await.hosting(id.as_str()).expect("read").expect("hosted");
    hosting.lease_slot = Some(1);
    rig.fleet.store().lock().await.keep_hosting(&hosting).expect("kept");
    let slot = PathBuf::from(adapter_traits::slot_path(&rig.root, 1));
    std::fs::create_dir_all(&slot).unwrap();
    let inside = slot.join("body.json");
    std::fs::write(&inside, "in the slot").unwrap();
    sent_from(&rig, posting(&inside, &id)).await.expect("answered");
    assert_eq!(sender.seen.lock().unwrap()[0].body.as_deref(), Some("in the slot"));

    let beside = slot.with_file_name(format!("{}-beside.json", slot.file_name().unwrap().to_string_lossy()));
    std::fs::write(&beside, "next door").unwrap();
    assert!(refused(&sent_from(&rig, posting(&beside, &id)).await), "a sibling of the slot is not the slot");
    std::fs::remove_file(beside).unwrap();
}

#[tokio::test]
async fn a_body_file_outside_the_jobs_places_is_refused_and_nothing_is_sent() {
    let sender = Watching::ok();
    let rig = rig_sending_through(&sender).await;
    let id = rig.start().await;
    let scratch = rig.fleet.scratch_of(id.as_str());
    let elsewhere = TempDir::new();
    let outside = elsewhere.path().join("secret.txt");
    std::fs::write(&outside, "not the Job's").unwrap();
    std::os::unix::fs::symlink(&outside, scratch.join("link.json")).unwrap();
    std::fs::write(scratch.parent().unwrap().join("other.json"), "a sibling folder").unwrap();
    let spelled_out = scratch.join("..").join("other.json");
    for file in [outside, scratch.join("link.json"), spelled_out, PathBuf::from("relative.json"), scratch.join("gone.json")] {
        let answer = sent_from(&rig, posting(&file, &id)).await;
        assert!(refused(&answer), "{}: {answer:?}", file.display());
    }
    assert!(sender.seen.lock().unwrap().is_empty(), "nothing went");
    let mut both = posting(&scratch.join("both.json"), &id);
    both.body = Some("{}".to_string());
    assert!(ipc::decode::<SendRequest>("a send", ipc::encode(&both).unwrap().as_bytes()).is_err(), "body and body_file are one or the other");
}

#[tokio::test]
async fn a_body_file_over_the_cap_is_refused() {
    let sender = Watching::ok();
    let rig = rig_sending_through(&sender).await;
    let id = rig.start().await;
    let file = rig.fleet.scratch_of(id.as_str()).join("big.json");
    std::fs::write(&file, "x".repeat(crate::requesting::REQUEST_BODY_CAP + 1)).unwrap();
    assert!(refused(&sent_from(&rig, posting(&file, &id)).await));
    std::fs::write(&file, "x".repeat(crate::requesting::REQUEST_BODY_CAP)).unwrap();
    sent_from(&rig, posting(&file, &id)).await.expect("exactly the cap is sent");
}

// `send_requests`.

fn batch(files: &[PathBuf], values: &[String]) -> SendRequests {
    SendRequests {
        session_id: None,
        repository: None,
        environment: Some("staging".to_string()),
        template: RequestTemplate {
            method: "POST".to_string(),
            url: "{{base}}/v1/{{region}}/items".to_string(),
            headers: vec![
                RequestHeader { name: "Authorization".to_string(), value: "Bearer {{token}}".to_string() },
                RequestHeader { name: "X-Region".to_string(), value: "{{ region }}".to_string() },
            ],
            collection: None,
        },
        body_files: files.iter().map(|file| file.to_string_lossy().into_owned()).collect(),
        vary: Some(RequestVary { name: "region".to_string(), values: values.to_vec() }),
        concurrency: None,
        stop_after: None,
    }
}

fn regions(count: usize) -> Vec<String> {
    (0..count).map(|n| format!("r{n:02}")).collect()
}

#[tokio::test]
async fn twenty_files_and_twenty_values_are_four_hundred_requests_and_no_secret_is_read_back() {
    let sender = Watching::ok();
    let rig = rig_sending_through(&sender).await;
    let id = rig.start().await;
    let scratch = rig.fleet.scratch_of(id.as_str());
    let files: Vec<PathBuf> = (0..20)
        .map(|n| {
            let file = scratch.join(format!("doc{n:02}.json"));
            std::fs::write(&file, format!("{{\"doc\":{n},\"region\":\"{{{{region}}}}\",\"key\":\"{{{{token}}}}\"}}")).unwrap();
            file
        })
        .collect();
    let mut send = batch(&files, &regions(20));
    send.session_id = Some(id.clone());
    send.concurrency = Some(8);
    let answer = rig.fleet.send_requests(None, send).await.expect("answered");

    assert_eq!(answer.lines.len(), 400);
    assert!(answer.lines.iter().all(|line| line.status == Some(200) && !line.skipped), "{:?}", answer.lines.iter().find(|l| l.status != Some(200)));
    assert_eq!(answer.lines[0].name, "doc00-r00");
    assert_eq!(answer.lines[399].name, "doc19-r19");
    assert_eq!(answer.note, None);
    let seen = sender.seen.lock().unwrap();
    assert_eq!(seen.len(), 400);
    let one = seen.iter().find(|request| request.url == "https://staging.example.test/v1/r07/items" && request.body.as_deref().is_some_and(|b| b.contains("\"doc\":13"))).expect("doc13 for r07");
    assert_eq!(one.body.as_deref(), Some(format!("{{\"doc\":13,\"region\":\"r07\",\"key\":\"{TOKEN}\"}}").as_str()), "the value went first, then the environment filled the secret");
    assert!(one.headers.contains(&("X-Region".to_string(), "r07".to_string())));
    assert!(one.headers.contains(&("Authorization".to_string(), format!("Bearer {TOKEN}"))));

    assert!(!ipc::encode(&answer).unwrap().contains(TOKEN), "no line holds the secret");
    let folder = PathBuf::from(&answer.folder);
    assert_eq!(folder, scratch.join("requests"));
    let mut saved = 0;
    for line in &answer.lines {
        let text = std::fs::read_to_string(folder.join(line.saved.as_deref().expect("saved"))).unwrap();
        assert_eq!(text, "you sent Bearer {{token}}", "{}", line.name);
        saved += 1;
    }
    assert_eq!(saved, 400);
    assert_eq!(std::fs::read_dir(&folder).unwrap().count(), 400, "one file each, none overwritten");
}

#[tokio::test]
async fn a_batch_is_refused_whole_before_anything_is_sent() {
    let sender = Watching::ok();
    let rig = rig_sending_through(&sender).await;
    let id = rig.start().await;
    let scratch = rig.fleet.scratch_of(id.as_str());
    let good = scratch.join("good.json");
    std::fs::write(&good, "{}").unwrap();
    let elsewhere = TempDir::new();
    let outside = elsewhere.path().join("bad.json");
    std::fs::write(&outside, "{}").unwrap();

    let mut too_many = batch(&[good.clone()], &regions(1001));
    too_many.session_id = Some(id.clone());
    let mut outside_the_places = batch(&[good.clone(), outside], &regions(2));
    outside_the_places.session_id = Some(id.clone());
    let mut neither = batch(&[], &[]);
    neither.vary = None;
    neither.session_id = Some(id.clone());
    for send in [too_many, outside_the_places, neither] {
        let answer = rig.fleet.send_requests(None, send).await;
        assert!(matches!(answer, Err(Refusal::Unacceptable(_))), "{answer:?}");
    }
    let unplaced = rig.fleet.send_requests(None, batch(&[good], &regions(1))).await;
    assert!(matches!(unplaced, Err(Refusal::Unacceptable(_))), "no Job is named: {unplaced:?}");
    assert!(sender.seen.lock().unwrap().is_empty());
}

// The expansion and the run, as functions of values.

fn filling() -> Filling {
    Filling {
        repository: Some("acme--shop".to_string()),
        collections: Vec::new(),
        environments: vec![RequestEnvironment {
            name: "staging".to_string(),
            variables: vec![
                RequestVariable::Plain { name: "base".to_string(), value: "https://staging.example.test".to_string() },
                RequestVariable::Secret { name: "token".to_string() },
            ],
        }],
    }
}

fn keychain() -> Held {
    let held = Held::default();
    let variable = SecretVariable { repository: "acme--shop".into(), environment: "staging".into(), name: "token".into() };
    adapter_traits::RequestSecrets::keep(&held, &variable, Secret::new(TOKEN.to_string())).unwrap();
    held
}

fn files(count: usize) -> Vec<(String, String)> {
    (0..count).map(|n| (format!("f{n}"), format!("body {n} {{{{region}}}} {{{{ token }}}}"))).collect()
}

#[test]
fn the_vary_name_is_replaced_and_every_other_name_is_left_for_the_environment() {
    let send = batch(&[], &["eu".to_string(), "us".to_string()]);
    let items = expanded(&send, &files(2)).expect("expanded");
    let names: Vec<&str> = items.iter().map(|item| item.name.as_str()).collect();
    assert_eq!(names, ["f0-eu", "f0-us", "f1-eu", "f1-us"]);
    let RequestToSend::Own { url, .. } = &items[1].send.request else { panic!("own") };
    assert_eq!(url, "{{base}}/v1/us/items");
    assert_eq!(items[1].send.headers[0].value, "Bearer {{token}}");
    assert_eq!(items[1].send.headers[1].value, "us");
    assert_eq!(items[3].send.body.as_deref(), Some("body 1 us {{ token }}"));
}

#[test]
fn names_are_file_names_and_a_repeat_is_told_apart() {
    let mut send = batch(&[], &["a/b".to_string(), "a_b".to_string(), "x".to_string()]);
    send.vary.as_mut().unwrap().values.push("x".to_string());
    let items = expanded(&send, &[("doc".to_string(), String::new())]).expect("expanded");
    let names: Vec<&str> = items.iter().map(|item| item.name.as_str()).collect();
    assert_eq!(names, ["doc-a_b", "doc-a_b~2", "doc-x", "doc-x~2"]);
}

#[test]
fn most_in_flight_is_the_concurrency_and_never_more_than_ten() {
    for (asked, allowed) in [(Some(3), 3), (None, 5), (Some(50), 10)] {
        let sender = Watching::answering(|_| 200, Duration::from_millis(15));
        let mut send = batch(&[], &regions(40));
        send.concurrency = asked;
        let items = expanded(&send, &[]).expect("expanded");
        let folder = TempDir::new();
        let done = run(&items, &*sender, &keychain(), &filling(), &Bounds::shipped(), &send, folder.path()).expect("ran");
        assert_eq!(done.lines.len(), 40);
        let most = sender.most.load(Ordering::SeqCst);
        assert!(most <= allowed, "{asked:?}: {most} were in flight, and {allowed} is the most");
        assert!(most > 1, "{asked:?}: they did overlap, or the cap is not tested");
    }
}

#[test]
fn a_run_of_failures_stops_the_batch_and_a_success_resets_the_count() {
    // One at a time, so the order is the turn: a pair of failures, a 2xx, then three in a row.
    let statuses = [500, 404, 200, 500, 502, 200, 500, 500, 503, 200, 200, 200];
    let sender = Watching::answering(move |turn| statuses[turn], Duration::ZERO);
    let mut send = batch(&[], &regions(12));
    send.concurrency = Some(1);
    send.stop_after = Some(3);
    let items = expanded(&send, &[]).expect("expanded");
    let folder = TempDir::new();
    let done = run(&items, &*sender, &keychain(), &filling(), &Bounds::shipped(), &send, folder.path()).expect("ran");
    assert_eq!(sender.seen.lock().unwrap().len(), 9, "stopped on the third in a row, at turn nine");
    let skipped: Vec<&str> = done.lines.iter().filter(|line| line.skipped).map(|line| line.name.as_str()).collect();
    assert_eq!(skipped, ["r09", "r10", "r11"]);
    assert!(done.lines[..9].iter().all(|line| line.status.is_some() && !line.skipped));
    assert_eq!(done.note.as_deref(), Some("Stopped after 3 answers in a row that were a 4xx, a 5xx or an error: 3 of 12 were not sent."));
}

#[test]
fn a_run_that_never_fails_in_a_row_sends_everything() {
    let sender = Watching::answering(|turn| if turn % 3 == 2 { 200 } else { 500 }, Duration::ZERO);
    let mut send = batch(&[], &regions(12));
    send.concurrency = Some(1);
    send.stop_after = Some(3);
    let items = expanded(&send, &[]).expect("expanded");
    let folder = TempDir::new();
    let done = run(&items, &*sender, &keychain(), &filling(), &Bounds::shipped(), &send, folder.path()).expect("ran");
    assert_eq!(sender.seen.lock().unwrap().len(), 12);
    assert_eq!(done.note, None);
}

#[test]
fn an_error_is_a_failure_and_its_words_never_hold_the_secret() {
    struct Failing;
    impl RequestSender for Failing {
        fn send(&self, _request: Outgoing, _most: usize) -> Result<Incoming, RequestFailure> {
            Err(RequestFailure::Unreachable(format!("connection refused by {TOKEN}")))
        }
    }
    let mut send = batch(&[], &regions(10));
    send.concurrency = Some(1);
    let items = expanded(&send, &[]).expect("expanded");
    let folder = TempDir::new();
    let done = run(&items, &Failing, &keychain(), &filling(), &Bounds::shipped(), &send, folder.path()).expect("ran");
    assert_eq!(done.lines.iter().filter(|line| !line.skipped).count(), 5, "five errors in a row is the default stop");
    assert!(done.lines[0].error.is_some() && done.lines[0].status.is_none());
    assert!(!ipc::encode(&done).unwrap().contains(TOKEN), "{done:?}");
}
