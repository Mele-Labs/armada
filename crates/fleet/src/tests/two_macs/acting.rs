//! Acting across his Macs, on loopback: Home acts on Jobs that Work holds, and Work runs each act on its own
//! checkout, once for each id, only for a Job in a shared project. What Work cannot be reached for waits on Home,
//! in order, until Work answers again.

use std::sync::Arc;

use api::HostedSessions;
use ipc::{
    AnswerSessionAsk, ApprovePlan, CallAnswer, JobAct, MacAct, MacLink, Queued, QueuedList, SendSessionMessage, SendSessionMessageBody, SessionId,
    SessionRecord, SessionRow, SessionVoice, StartOn, StartSession, StopSession, TakeBack,
};

use super::{job_in, paired, project, shared, until, OneMac, Ticking};
use crate::mac_acts::JobSent;
use crate::tests::pull_requesting::{facts, forge_ran, passed};
use crate::tests::session_plan::{first, recorded};

const ASK: &str = r#"{"tool_name":"AskUserQuestion","tool_use_id":"toolu_1","input":{"questions":[
  {"question":"Which size?","header":"Size","multiSelect":false,
   "options":[{"label":"S","description":"Small"},{"label":"L","description":"Large"}]}]}}"#;

struct Pair {
    home: OneMac,
    work: OneMac,
    job: SessionId,
}

/// Home and Work paired, and a Job of Work's in a shared project that Home lists. With `pull`, the Job owns pull
/// request 12 of its repository, which Home reads off the row.
async fn pair_over_a_shared_job(pull: bool) -> Pair {
    let clock = Arc::new(Ticking::from_nine());
    let (home, work) = (OneMac::new("Home", "01HMAC", &clock).await, OneMac::new("Work", "01WMAC", &clock).await);
    let shop = project(&work.rig, "Shop").await;
    shared(&work.rig, &shop, true).await;
    let job = job_in(&work.rig, Some(&shop)).await;
    if pull {
        let now = work.rig.fleet.now().as_str().to_string();
        let held = store::KeptAttachment {
            holder: store::Holder::session(job.as_str()),
            kind: "pr".into(),
            manifest_id: work.rig.manifest.as_str().to_string(),
            target: "12".into(),
            state: store::AttachmentState::Standing,
            detail: Default::default(),
            since: now.clone(),
            changed_at: now,
        };
        work.rig.fleet.store().lock().await.attach(&held, false).expect("kept");
    }
    paired(&home, &work).await;
    let (id, wanted) = (job.clone(), if pull { Some("12") } else { None });
    until("Work's Job reaches Home's list", || async {
        let (_, list) = home.sessions().await;
        let Some(row) = list.sessions.iter().find(|one| one.id == id) else { return false };
        row.mac.as_deref() == Some(work.id.as_str()) && wanted.is_none_or(|number| row.attachments.iter().any(|one| one.kind == "pr" && one.target == number))
    })
    .await;
    Pair { home, work, job }
}

async fn post(mac: &OneMac, path: &str, body: String) -> (axum::http::StatusCode, String) {
    use tower::ServiceExt;
    let request = axum::http::Request::post(path)
        .header("content-type", "application/json")
        .header("x-armada-caller", "bridge")
        .body(axum::body::Body::from(body))
        .expect("a request");
    let answer = mac.fleet.clone().oneshot(request).await.expect("answered");
    let status = answer.status();
    let body = axum::body::to_bytes(answer.into_body(), 1 << 24).await.expect("a body");
    (status, String::from_utf8_lossy(&body).to_string())
}

fn encoded<T: serde::Serialize>(value: &T) -> String {
    ipc::encode(value).expect("encodes")
}

fn message(job: &SessionId, text: &str) -> SendSessionMessageBody {
    let message = SendSessionMessage { session_id: job.clone(), text: text.into(), attachments: Vec::new(), mentions: Vec::new(), sketches: Vec::new() };
    SendSessionMessageBody { message, aim: None }
}

/// How many times the person's `text` is in `mac`'s thread of `job`.
async fn said(mac: &OneMac, job: &SessionId, text: &str) -> usize {
    let rows = mac.rig.rows(job).await;
    rows.iter().filter(|row| matches!(row, SessionRow::Message { from: SessionVoice::You, text: said, .. } if said == text)).count()
}

async fn acts_run(mac: &OneMac) -> Vec<store::KeptActRun> {
    mac.rig.fleet.store().lock().await.acts_run().expect("reads")
}

async fn queued(mac: &OneMac) -> QueuedList {
    let (status, body) = {
        use tower::ServiceExt;
        let answer = mac.fleet.clone().oneshot(axum::http::Request::get("/queued").body(axum::body::Body::empty()).expect("a request")).await.expect("answered");
        (answer.status(), axum::body::to_bytes(answer.into_body(), 1 << 20).await.expect("a body"))
    };
    assert_eq!(status, axum::http::StatusCode::OK);
    ipc::decode("the queue", &body).expect("a queue")
}

async fn asleep(home: &OneMac, work: &OneMac) {
    until("Work shows asleep on Home", || async { matches!(home.mac(&work.id).await.and_then(|mac| mac.link), Some(MacLink::Asleep { .. })) }).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn from_one_mac_a_message_an_answer_an_approval_a_merge_and_a_stop_run_on_the_mac_that_holds_the_job() {
    let Pair { home, work, job } = pair_over_a_shared_job(true).await;
    work.rig.fleet.vcs().now_pull_request(Some(facts(adapter_traits::PullRequestStanding::Open)));
    forge_ran(&work.rig.fleet, passed());

    // A message: Work's thread holds it, Home's holds nothing.
    let (status, body) = post(&home, "/sessions/message", encoded(&message(&job, "Use the blue tape"))).await;
    assert_eq!(status, axum::http::StatusCode::ACCEPTED, "{body}");
    assert_eq!(said(&work, &job, "Use the blue tape").await, 1);

    // An ask the Job holds at Work's door, answered from Home.
    let (fleet, session) = (Arc::clone(&work.rig.fleet), job.as_str().to_string());
    let held = tokio::spawn(async move { fleet.session_permission(&session, ipc::decode("an ask", ASK.as_bytes()).expect("an ask")).await });
    let call = {
        let found = std::sync::Mutex::new(None);
        super::super::session_host::eventually(|| async {
            let asked = Arc::clone(&work.rig.fleet).get_session(job.clone()).await.expect("read").session.hosted.expect("hosted").asked;
            *found.lock().unwrap() = asked.clone();
            asked.is_some()
        })
        .await;
        let call = found.lock().unwrap().take().expect("an ask is waiting").call;
        call
    };
    let answer = AnswerSessionAsk {
        session_id: job.clone(),
        call,
        answer: CallAnswer::AllowOnce,
        note: None,
        answers: vec![ipc::QuestionAnswer { question: "Which size?".into(), chosen: vec!["S".into()] }],
        folder: None,
        kind: None,
    };
    let (status, body) = post(&home, "/sessions/ask/answer", encoded(&answer)).await;
    assert_eq!(status, axum::http::StatusCode::OK, "{body}");
    tokio::time::timeout(std::time::Duration::from_secs(5), held).await.expect("the ask was answered on Work").expect("joined").ok();

    // A plan approved.
    recorded(&work.rig, first(&job)).await;
    let (status, body) = post(&home, "/sessions/plan/approve", encoded(&ApprovePlan { session_id: job.clone(), version: 1 })).await;
    assert_eq!(status, axum::http::StatusCode::OK, "{body}");
    let plan = Arc::clone(&work.rig.fleet).get_session(job.clone()).await.expect("read").session.plan.expect("a plan");
    assert!(plan.approved, "{plan:?}");

    // A merge, asked of the forge by Work and never by Home.
    let path = format!("/pull_requests/{}/12/merge", work.rig.manifest.as_str());
    let (status, body) = post(&home, &path, String::new()).await;
    assert_eq!(status, axum::http::StatusCode::OK, "{body}");
    assert_eq!(work.rig.fleet.vcs().times_asked_to_merge(), 1);
    assert_eq!(home.rig.fleet.vcs().times_asked_to_merge(), 0, "nothing is merged from the Mac that does not hold the Job");

    // A stop.
    let (status, body) = post(&home, "/sessions/stop", encoded(&StopSession { session_id: job.clone() })).await;
    assert_eq!(status, axum::http::StatusCode::OK, "{body}");

    // Work's record: each act, from Home, as the door `mac`. Home ran none.
    let ran = acts_run(&work).await;
    let kinds: Vec<&str> = ran.iter().map(|one| one.what.as_str()).collect();
    assert_eq!(kinds, ["message", "ask_answer", "plan_approve", "merge", "stop"]);
    assert!(ran.iter().all(|one| one.mac == home.id && one.via == "mac"), "{ran:?}");
    assert!(acts_run(&home).await.is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_start_naming_another_mac_starts_the_job_there_and_its_row_shows_here_at_once() {
    let clock = Arc::new(Ticking::from_nine());
    let (home, work) = (OneMac::new("Home", "01HMAC", &clock).await, OneMac::new("Work", "01WMAC", &clock).await);
    let shop = project(&work.rig, "Shop").await;
    shared(&work.rig, &shop, true).await;
    paired(&home, &work).await;
    let work_id = work.id.clone();
    until("Work's project reaches Home", || async { home.projects().await.1.projects.iter().any(|one| one.id == shop && one.mac.as_deref() == Some(work_id.as_str())) }).await;

    let start = StartSession {
        project: Some(shop.clone()),
        manifest_id: work.rig.manifest.clone(),
        title: Some("Tape the box".into()),
        model: None,
        effort: None,
        mode: None,
        root: None,
        fork: None,
        request: None,
        after: None,
        held: false,
        project_rows: Vec::new(),
        on_your_words: None,
    };
    let (status, body) = post(&home, "/sessions/start", encoded(&StartOn { mac: Some(work.id.clone()), start })).await;
    assert_eq!(status, axum::http::StatusCode::OK, "{body}");
    let record: SessionRecord = ipc::decode("the Job", body.as_bytes()).expect("a Job");
    assert_eq!(record.mac.as_deref(), Some(work.id.as_str()));

    assert_eq!(super::from(&work.sessions().await.1, &record.id), Some(None), "it runs on Work, as one of Work's own");
    assert_eq!(super::from(&home.sessions().await.1, &record.id), Some(Some(work.id.clone())), "and Home draws it at once with Work's Mac");
}

fn acts_on(job: &str, manifest: &str, project: &str) -> Vec<JobAct> {
    let bodies = [
        format!(r#"{{"kind":"message","body":{{"session_id":"{job}","text":"hi"}}}}"#),
        format!(r#"{{"kind":"ask_answer","body":{{"session_id":"{job}","call":"c1","answer":"allow_once"}}}}"#),
        format!(r#"{{"kind":"waiting_answer","body":{{"session_id":"{job}","item_id":"i1","choice":0}}}}"#),
        format!(r#"{{"kind":"waiting_answer_parts","body":{{"session_id":"{job}","item_id":"i1","parts_picked":[{{"option":0,"part":"a"}}]}}}}"#),
        format!(r#"{{"kind":"plan_approve","body":{{"session_id":"{job}","version":1}}}}"#),
        format!(r#"{{"kind":"plan_changes","body":{{"session_id":"{job}","version":1,"marks":[]}}}}"#),
        format!(r#"{{"kind":"sketch_answer","body":{{"session_id":"{job}","sketch":"s","drawn":"d","how":"approved"}}}}"#),
        format!(r#"{{"kind":"document_answer","body":{{"session_id":"{job}","document":"d","revision":"1","how":"approved"}}}}"#),
        format!(r#"{{"kind":"stop","body":{{"session_id":"{job}"}}}}"#),
        format!(r#"{{"kind":"merge","body":{{"session_id":"{job}","repository":"{manifest}","number":12}}}}"#),
        format!(r#"{{"kind":"start","body":{{"manifest_id":"{manifest}","project":"{project}"}}}}"#),
        format!(r#"{{"kind":"dispatch","body":{{"manifest_id":"{manifest}","project":"{project}"}}}}"#),
    ];
    bodies.iter().map(|body| ipc::decode("an act", body.as_bytes()).expect("an act")).collect()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn an_act_or_a_start_on_what_is_not_shared_is_answered_not_found_as_for_nothing_and_leaves_no_trace() {
    let clock = Arc::new(Ticking::from_nine());
    let (home, work) = (OneMac::new("Home", "01HMAC", &clock).await, OneMac::new("Work", "01WMAC", &clock).await);
    let open = project(&work.rig, "Shop").await;
    let private = project(&work.rig, "Taxes").await;
    shared(&work.rig, &open, true).await;
    let (s, p) = (job_in(&work.rig, Some(&open)).await, job_in(&work.rig, Some(&private)).await);
    let carried = work.every_event().await;
    paired(&home, &work).await;
    let s_id = s.clone();
    until("Work's shared Job reaches Home", || async { super::from(&home.sessions().await.1, &s_id).is_some() }).await;
    let (rows_before, jobs_before) = (work.rig.rows(&p).await.len(), work.sessions().await.1.sessions.len());
    let events_before = carried.lock().expect("not poisoned").len();
    let manifest = work.rig.manifest.as_str().to_string();

    // Each act, on Work's unshared Job and on an id Work never had, answered alike once the id is taken out.
    let (unshared, missing) = (acts_on(p.as_str(), &manifest, &private), acts_on("nobody", &manifest, "nope"));
    for (on_unshared, on_nothing) in unshared.into_iter().zip(missing) {
        let kind = on_unshared.name();
        let answered = |act: JobAct, id: String, project: String| {
            let home = &home;
            let work_id = work.id.clone();
            async move {
                let said = home.rig.fleet.acted_on_other_mac(work_id, act).await.expect("answered, not queued");
                let text = String::from_utf8(said.body).expect("text").replace(&id, "<id>").replace(&project, "<project>");
                (said.status, text)
            }
        };
        let (a, b) = (answered(on_unshared, p.as_str().to_string(), private.clone()).await, answered(on_nothing, "nobody".into(), "nope".into()).await);
        assert_eq!(a.0, 404, "{kind}: {a:?}");
        assert_eq!(a, b, "{kind} is answered as an act on nothing is");
    }

    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    assert!(acts_run(&work).await.is_empty(), "no run is kept");
    assert_eq!(work.rig.rows(&p).await.len(), rows_before, "nothing was written to its thread");
    assert_eq!(work.sessions().await.1.sessions.len(), jobs_before, "and no Job was started");
    let events = carried.lock().expect("not poisoned")[events_before..].join("\n");
    assert!(!events.contains(p.as_str()), "no event on it: {events}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn acts_for_a_sleeping_mac_wait_in_order_can_be_taken_back_and_go_once_each_when_it_wakes() {
    let Pair { home, work, job } = pair_over_a_shared_job(false).await;
    let mut work = work;
    work.stop().await;
    asleep(&home, &work).await;

    let mut waiting = Vec::new();
    for text in ["first", "second", "third"] {
        let (status, body) = post(&home, "/sessions/message", encoded(&message(&job, text))).await;
        assert_eq!(status, axum::http::StatusCode::ACCEPTED, "{body}");
        let kept: Queued = ipc::decode("a queued act", body.as_bytes()).expect("the Queued");
        assert_eq!((kept.mac.as_str(), kept.refused.as_deref()), (work.id.as_str(), None));
        waiting.push(kept.id);
    }
    let listed = queued(&home).await.queued;
    assert_eq!(listed.iter().map(|one| one.id.clone()).collect::<Vec<_>>(), waiting, "in the order he made them");
    assert_eq!(said(&work, &job, "first").await, 0, "nothing has gone");

    let (status, body) = post(&home, "/queued/take_back", encoded(&TakeBack { id: waiting[1].clone() })).await;
    assert_eq!(status, axum::http::StatusCode::OK, "{body}");
    let left: QueuedList = ipc::decode("the queue", body.as_bytes()).expect("a queue");
    assert_eq!(left.queued.iter().map(|one| one.id.clone()).collect::<Vec<_>>(), [waiting[0].clone(), waiting[2].clone()]);
    let (status, _) = post(&home, "/queued/take_back", encoded(&TakeBack { id: waiting[1].clone() })).await;
    assert_eq!(status, axum::http::StatusCode::NOT_FOUND, "taken back already");

    work.restart();
    let (third, work_ref) = (job.clone(), &work);
    until("the rest went to Work", || async { said(work_ref, &third, "first").await == 1 && said(work_ref, &third, "third").await == 1 }).await;
    until("and left Home's queue", || async { queued(&home).await.queued.is_empty() }).await;
    assert_eq!(said(&work, &job, "second").await, 0, "what was taken back never went");

    let ran = acts_run(&work).await;
    assert_eq!(ran.iter().map(|one| one.id.clone()).collect::<Vec<_>>(), [waiting[0].clone(), waiting[2].clone()], "once each, in order");
    let thread: Vec<String> = work
        .rig
        .rows(&job)
        .await
        .into_iter()
        .filter_map(|row| match row {
            SessionRow::Message { from: SessionVoice::You, text, .. } => Some(text),
            _ => None,
        })
        .collect();
    assert_eq!(thread, ["first", "third"]);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn an_act_sent_twice_under_one_id_runs_once_and_is_answered_alike() {
    let Pair { home, work, job } = pair_over_a_shared_job(false).await;
    let sent = MacAct { id: "one-send".into(), act: JobAct::Message(message(&job, "Once only")), at: 1 };

    let mut answers = Vec::new();
    for _ in 0..2 {
        match home.rig.fleet.send_job_act(&work.id, &sent).await {
            Ok(JobSent::Answered(answered)) => answers.push((answered.status, answered.body)),
            _ => panic!("Work did not answer"),
        }
    }

    assert_eq!(answers[0].0, 202);
    assert_eq!(answers[0], answers[1], "the first answer, again");
    assert_eq!(said(&work, &job, "Once only").await, 1);
    assert_eq!(acts_run(&work).await.len(), 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_queued_act_the_mac_refuses_stays_with_its_why_and_the_acts_after_it_still_go() {
    let Pair { home, work, job } = pair_over_a_shared_job(false).await;
    let mut work = work;
    work.stop().await;
    asleep(&home, &work).await;

    // Work has no plan 5 to approve, so it will refuse the first on its turn.
    let (status, body) = post(&home, "/sessions/plan/approve", encoded(&ApprovePlan { session_id: job.clone(), version: 5 })).await;
    assert_eq!(status, axum::http::StatusCode::ACCEPTED, "{body}");
    let refused: Queued = ipc::decode("a queued act", body.as_bytes()).expect("the Queued");
    let (status, _) = post(&home, "/sessions/message", encoded(&message(&job, "After it"))).await;
    assert_eq!(status, axum::http::StatusCode::ACCEPTED);

    work.restart();
    let work_ref = &work;
    let after = job.clone();
    until("the act after it went", || async { said(work_ref, &after, "After it").await == 1 }).await;
    // Work writes the message before Home reads its answer, so Home's queue lets it go a moment later.
    let home_ref = &home;
    until("and left Home's queue", || async { queued(home_ref).await.queued.len() == 1 }).await;
    let left = queued(&home).await.queued;
    assert_eq!(left.len(), 1, "{left:?}");
    assert_eq!(left[0].id, refused.id);
    assert!(left[0].refused.as_deref().is_some_and(|why| !why.is_empty()), "it says why: {left:?}");
    assert_eq!(left[0].act.name(), "plan_approve");

    let (status, body) = post(&home, "/queued/take_back", encoded(&TakeBack { id: refused.id })).await;
    assert_eq!(status, axum::http::StatusCode::OK, "{body}");
    assert!(queued(&home).await.queued.is_empty(), "taking it back is how it goes");
}
