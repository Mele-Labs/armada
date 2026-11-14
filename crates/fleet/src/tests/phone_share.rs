//! The share filter on the phone, one test for each route the phone calls: a real Fleet behind a real Phone
//! Gateway, asked by a paired phone. A Job of an unshared project, or of none, is answered as a Job that does not
//! exist and is listed nowhere; the same Job reaches the phone once its project is shared.

use std::sync::Arc;
use std::time::Duration;

use api::Sessions;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use ipc::{Event, SessionId, SetWaitingFor, WaitingInput};
use tower::ServiceExt;

use super::session_host::{rig_placing, Rig};
use super::share::{job_in, project, shared};
use crate::tests::tmp::TempDir;

/// An id no Job has, for the answer an unshared Job must match.
const NOBODY: &str = "01NOSUCHJOB0000000000000000";

fn now() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).expect("after 1970").as_secs() as i64
}

/// A Mac's Fleet on a loopback port, its Gateway in front, and a phone paired with it.
struct Phone {
    rig: Rig,
    gateway: pocket::Gateway,
    device: String,
    key: p256::ecdsa::SigningKey,
    _home: TempDir,
}

impl Phone {
    async fn paired() -> Phone {
        let rig = rig_placing(None);
        let fleet = api::router(api::Served::sharing(Arc::clone(&rig.fleet), ipc::RunId::carried("01RUN"), rig.fleet.events()));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("a loopback port for Fleet");
        let fleet_port = listener.local_addr().expect("bound").port();
        tokio::spawn(async move { axum::serve(listener, fleet).await.ok() });

        let home = TempDir::new();
        let mut store = store::Store::open_migrated_copy(&home.path().join("armada.db")).expect("the Gateway's store");
        let this = store::ThisMac { id: "686f6d65".into(), name: "Home".into(), colour: "teal".into(), minted_at: 1 };
        store.this_mac_or(&this).expect("this Mac");
        let keys = Arc::new(pocket::MemoryMacKeys::default());
        let gateway = pocket::Gateway {
            fleet: Arc::new(move || Ok(fleet_port)),
            assets: None,
            pairing: pocket::Pairing::new(store, Arc::new(now), Arc::new(|| Ok(String::from("http://127.0.0.1:1")))),
            push: pocket::Push::open(home.path(), pocket::PLACEHOLDER_SUBJECT).expect("a push key"),
            way_in: pocket::mac_way_in(home.path().to_path_buf(), 1, None),
            macs: pocket::Macs::new(keys as Arc<dyn pocket::MacKeys>).expect("keys"),
        };
        let key = p256::ecdsa::SigningKey::from_slice(&[7u8; 32]).expect("a key");
        let (code, _, _) = gateway.pairing.start().expect("a code");
        let device = gateway.pairing.claim(&code, "Phone", &pocket::spki_hex(&key).expect("its key")).ok().expect("claimed");
        gateway.pairing.confirm(&code).ok().expect("confirmed");
        Phone { rig, gateway, device, key, _home: home }
    }

    fn request(&self, method: &str, target: &str, body: &str) -> Request<Body> {
        let headers = pocket::sign(&self.key, &self.device, method, target, now(), body.as_bytes()).expect("signed");
        let mut request = Request::builder().method(method).uri(target).header("content-type", "application/json");
        for (name, value) in headers {
            request = request.header(name, value);
        }
        request.body(Body::from(body.to_string())).expect("a request")
    }

    /// What the phone is answered, with `job` written as `<id>` so two Jobs' answers compare whole.
    async fn call(&self, method: &str, target: &str, body: &str, job: &str) -> (StatusCode, String) {
        let answer = pocket::router(self.gateway.clone()).oneshot(self.request(method, target, body)).await.expect("answered");
        let status = answer.status();
        let body = answer.into_body().collect().await.expect("a body").to_bytes();
        (status, String::from_utf8_lossy(&body).replace(job, "<id>"))
    }

    async fn get(&self, target: &str) -> String {
        let (status, body) = self.call("GET", target, "", "\u{0}").await;
        assert_eq!(status, StatusCode::OK, "{target}: {body}");
        body
    }

    /// The repository the rig's Jobs run in, by the name the phone knows it.
    async fn repository(&self) -> String {
        let listed = api::Queries::list_manifests(&*self.rig.fleet).await.expect("listed");
        listed.into_iter().find(|one| one.id == self.rig.manifest).expect("the rig's").repository
    }
}

/// Three Jobs: one in a project not yet shared, one in none, and one in a project shared from the start.
struct Jobs {
    shop: String,
    private: SessionId,
    loose: SessionId,
    open: SessionId,
}

async fn jobs(phone: &Phone) -> Jobs {
    let shop = project(&phone.rig, "Shop").await;
    let garden = project(&phone.rig, "Garden").await;
    shared(&phone.rig, &garden, true).await;
    let private = job_in(&phone.rig, Some(&shop)).await;
    let loose = job_in(&phone.rig, None).await;
    let open = job_in(&phone.rig, Some(&garden)).await;
    Jobs { shop, private, loose, open }
}

fn listed(body: &str, job: &SessionId) -> bool {
    body.contains(&format!("\"id\":\"{}\"", job.as_str()))
}

#[tokio::test]
async fn the_jobs_list_holds_only_jobs_in_a_shared_project() {
    let phone = Phone::paired().await;
    let jobs = jobs(&phone).await;

    let body = phone.get("/api/sessions").await;
    assert!(listed(&body, &jobs.open), "{body}");
    assert!(!listed(&body, &jobs.private) && !listed(&body, &jobs.loose), "{body}");

    shared(&phone.rig, &jobs.shop, true).await;
    assert!(listed(&phone.get("/api/sessions").await, &jobs.private), "shared, it shows");
    shared(&phone.rig, &jobs.shop, false).await;
    assert!(!listed(&phone.get("/api/sessions").await, &jobs.private), "turned off, it goes at once");
}

#[tokio::test]
async fn needs_you_holds_only_jobs_in_a_shared_project() {
    let phone = Phone::paired().await;
    let jobs = jobs(&phone).await;
    for job in [&jobs.private, &jobs.loose, &jobs.open] {
        let item = WaitingInput { approves: false, id: "look".into(), text: "Review the Shop pane".into(), act: None, options: Vec::new(), steps: Vec::new() };
        phone.rig.fleet.set_waiting(None, SetWaitingFor { items: vec![item], session_id: Some(job.clone()) }).await.expect("raised");
    }

    let needs = phone.get("/api/needs").await;
    assert!(listed(&needs, &jobs.open), "{needs}");
    assert!(!listed(&needs, &jobs.private) && !listed(&needs, &jobs.loose), "{needs}");
    shared(&phone.rig, &jobs.shop, true).await;
    assert!(listed(&phone.get("/api/needs").await, &jobs.private));
}

#[tokio::test]
async fn an_unshared_jobs_thread_answers_as_a_job_that_does_not_exist() {
    let phone = Phone::paired().await;
    let jobs = jobs(&phone).await;
    let thread = |job: &str| format!("/api/sessions/one?session_id={job}");

    let missing = phone.call("GET", &thread(NOBODY), "", NOBODY).await;
    assert_ne!(missing.0, StatusCode::OK, "{missing:?}");
    for job in [&jobs.private, &jobs.loose] {
        assert_eq!(phone.call("GET", &thread(job.as_str()), "", job.as_str()).await, missing, "the answer says nothing of it");
    }
    let (status, body) = phone.call("GET", &thread(jobs.open.as_str()), "", jobs.open.as_str()).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    shared(&phone.rig, &jobs.shop, true).await;
    assert_eq!(phone.call("GET", &thread(jobs.private.as_str()), "", "").await.0, StatusCode::OK);
}

#[tokio::test]
async fn an_unshared_jobs_shown_file_answers_as_a_job_that_does_not_exist() {
    let phone = Phone::paired().await;
    let jobs = jobs(&phone).await;
    for route in ["/api/sessions/shown", "/api/sessions/shown/address"] {
        let shown = |job: &str| format!("{route}?session_id={job}&path=shot.png");
        let missing = phone.call("GET", &shown(NOBODY), "", NOBODY).await;
        assert_eq!(phone.call("GET", &shown(jobs.private.as_str()), "", jobs.private.as_str()).await, missing, "{route}");
        assert_eq!(phone.call("GET", &shown(jobs.loose.as_str()), "", jobs.loose.as_str()).await, missing, "{route}");
        // Fleet knows the shared Job, and answers for the file it never showed.
        assert_ne!(phone.call("GET", &shown(jobs.open.as_str()), "", jobs.open.as_str()).await, missing, "{route}");
    }
}

/// Every act the Gateway forwards, each with a body Fleet reads, the Job written `{id}`.
const ACTS: &[(&str, &str)] = &[
    ("ask/answer", r#"{"session_id":"{id}","call":"c1","answer":"refuse"}"#),
    ("waiting/answer", r#"{"session_id":"{id}","item_id":"i1"}"#),
    ("plan/approve", r#"{"session_id":"{id}","version":1}"#),
    ("plan/changes", r#"{"session_id":"{id}","version":1,"marks":[]}"#),
    ("sketch/answer", r#"{"session_id":"{id}","sketch":"s1","drawn":"d1","how":"approved"}"#),
    ("document/answer", r#"{"session_id":"{id}","document":"d1","revision":"r1","how":"approved"}"#),
    ("message", r#"{"session_id":"{id}","text":"Look at the pane"}"#),
    ("begin", r#"{"session_id":"{id}"}"#),
    ("stop", r#"{"session_id":"{id}"}"#),
    ("pause", r#"{"session_id":"{id}"}"#),
    ("resume", r#"{"session_id":"{id}"}"#),
    ("close", r#"{"session_id":"{id}"}"#),
    ("tune", r#"{"session_id":"{id}","mode":"auto"}"#),
    ("run/start", r#"{"session_id":"{id}","command":"true"}"#),
    ("waiting/dismiss", r#"{"session_id":"{id}","item_id":"i1"}"#),
    ("change_waiting", r#"{"waiting_session":"{id}"}"#),
    ("start_waiting", r#"{"session_id":"{id}"}"#),
];

/// A Job in `project` waiting for `on` to end.
async fn waiting_in(rig: &Rig, project: &str, on: &SessionId) -> SessionId {
    let after = ipc::StartAfter { session: on.clone(), condition: ipc::StartCondition::Ends, base: ipc::StartBase::default(), needs: String::new() };
    let start = ipc::StartSession {
        project: Some(project.to_string()),
        manifest_id: rig.manifest.clone(),
        title: Some("Build the pane after it".to_string()),
        model: None,
        effort: None,
        mode: None,
        root: None,
        fork: None,
        request: None,
        after: Some(after),
        held: false,
        project_rows: Vec::new(),
        on_your_words: None,
    };
    api::HostedSessions::start_session(&*rig.fleet, start).await.expect("started, waiting").id
}

#[tokio::test]
async fn an_act_on_an_unshared_job_answers_as_an_act_on_no_job_and_a_shared_one_reaches_it() {
    let phone = Phone::paired().await;
    let jobs = jobs(&phone).await;
    let garden = phone.rig.fleet.project_of_job(jobs.open.as_str()).await.expect("its project");
    for (act, body) in ACTS {
        let route = format!("/api/sessions/{act}");
        let missing = phone.call("POST", &route, &body.replace("{id}", NOBODY), NOBODY).await;
        assert_ne!(missing.0, StatusCode::BAD_REQUEST, "{act}: the body must read, or every Job answers alike: {missing:?}");
        for job in [&jobs.private, &jobs.loose] {
            let unshared = phone.call("POST", &route, &body.replace("{id}", job.as_str()), job.as_str()).await;
            assert_eq!(unshared, missing, "{act}: the answer says nothing of it");
        }
        // A Job of its own, so an act before it (a close, a stop) is not what this one meets. A change to a wait
        // needs a Job that waits: Fleet answers one that does not exactly as it answers no Job.
        let open = match *act {
            "change_waiting" => waiting_in(&phone.rig, &garden, &jobs.open).await,
            _ => job_in(&phone.rig, Some(&garden)).await,
        };
        let reached = phone.call("POST", &route, &body.replace("{id}", open.as_str()), open.as_str()).await;
        assert_ne!(reached, missing, "{act}: a shared Job takes the act");
    }
}

#[tokio::test]
async fn an_answer_to_an_unshared_jobs_question_answers_as_for_no_job() {
    let phone = Phone::paired().await;
    let jobs = jobs(&phone).await;
    let answer = |job: &str| format!(r#"{{"session_id":"{job}","answer":"refuse"}}"#);
    let missing = phone.call("POST", "/api/sessions/answer", &answer(NOBODY), NOBODY).await;
    assert_eq!(missing.0, StatusCode::NOT_FOUND, "{missing:?}");
    assert_eq!(phone.call("POST", "/api/sessions/answer", &answer(jobs.private.as_str()), jobs.private.as_str()).await, missing);
    assert_eq!(phone.call("POST", "/api/sessions/answer", &answer(jobs.loose.as_str()), jobs.loose.as_str()).await, missing);
    assert_ne!(phone.call("POST", "/api/sessions/answer", &answer(jobs.open.as_str()), jobs.open.as_str()).await, missing);
}

#[tokio::test]
async fn a_fork_of_an_unshared_job_answers_as_for_no_job() {
    let phone = Phone::paired().await;
    let jobs = jobs(&phone).await;
    let fork = |job: &str| format!(r#"{{"session_id":"{job}"}}"#);
    let missing = phone.call("POST", "/api/sessions/fork", &fork(NOBODY), NOBODY).await;
    assert_eq!(phone.call("POST", "/api/sessions/fork", &fork(jobs.private.as_str()), jobs.private.as_str()).await, missing);
    assert_eq!(phone.call("POST", "/api/sessions/fork", &fork(jobs.loose.as_str()), jobs.loose.as_str()).await, missing);
    assert_ne!(phone.call("POST", "/api/sessions/fork", &fork(jobs.open.as_str()), jobs.open.as_str()).await, missing, "a shared Job is found");
    assert!(phone.rig.fleet.list_sessions(None, None, None).await.expect("listed").sessions.len() == 3, "nothing was started");
}

#[tokio::test]
async fn the_repositories_are_those_in_a_shared_project_and_a_job_starts_only_there() {
    let phone = Phone::paired().await;
    let shop = project(&phone.rig, "Shop").await;
    let repository = phone.repository().await;
    let quoted = format!("\"{repository}\"");

    assert!(!phone.get("/api/repositories").await.contains(&quoted), "an unshared project's repository is not named");
    let dispatch = |named: &str| format!(r#"{{"text":"Fix the pane","repository":"{named}"}}"#);
    let unknown = phone.call("POST", "/api/sessions", &dispatch("nowhere"), "\u{0}").await;
    assert_eq!(phone.call("POST", "/api/sessions", &dispatch(&repository), "\u{0}").await, unknown.clone(), "as a repository it does not know");
    assert_eq!(phone.call("POST", "/api/sessions/dispatch", &dispatch(&repository), "\u{0}").await.0, unknown.0, "the Operator's too");
    assert!(phone.rig.fleet.list_sessions(None, None, None).await.expect("listed").sessions.is_empty(), "nothing was started");

    shared(&phone.rig, &shop, true).await;
    assert!(phone.get("/api/repositories").await.contains(&quoted));
    let (status, body) = phone.call("POST", "/api/sessions", &dispatch(&repository), "\u{0}").await;
    assert!(status == StatusCode::CREATED || body.contains("The Job started"), "{status} {body}");
    let started = phone.rig.fleet.list_sessions(None, None, None).await.expect("listed").sessions;
    let job = started.first().expect("a Job started").id.clone();
    assert_eq!(phone.rig.fleet.project_of_job(job.as_str()).await.as_deref(), Some(shop.as_str()), "in the shared project");
    assert!(listed(&phone.get("/api/sessions").await, &job), "so the phone sees what it started");
}

#[tokio::test]
async fn a_start_from_the_phone_into_an_unshared_project_is_refused_as_one_that_does_not_exist() {
    let phone = Phone::paired().await;
    let shop = project(&phone.rig, "Shop").await;
    let start = |project: &str| format!(r#"{{"manifest_id":"{}","project":"{project}"}}"#, phone.rig.manifest.as_str());
    let fleet = api::router(api::Served::sharing(Arc::clone(&phone.rig.fleet), ipc::RunId::carried("01RUN"), phone.rig.fleet.events()));
    let as_phone = |body: String| {
        Request::post("/sessions/start").header("content-type", "application/json").header(api::CALLER_HEADER, api::PHONE).body(Body::from(body)).expect("a request")
    };
    let said = |answer: axum::response::Response, project: String| async move {
        let status = answer.status();
        let body = answer.into_body().collect().await.expect("a body").to_bytes();
        (status, String::from_utf8_lossy(&body).replace(&project, "<project>"))
    };
    let missing = said(fleet.clone().oneshot(as_phone(start("no-such-project"))).await.expect("answered"), "no-such-project".into()).await;
    let unshared = said(fleet.clone().oneshot(as_phone(start(&shop))).await.expect("answered"), shop.clone()).await;
    assert_eq!(missing.0, StatusCode::NOT_FOUND, "{missing:?}");
    assert_eq!(unshared, missing, "an unshared project is not named");
}

#[tokio::test]
async fn a_route_the_phone_has_no_ruling_for_is_not_there_for_it() {
    let phone = Phone::paired().await;
    let fleet = api::router(api::Served::sharing(Arc::clone(&phone.rig.fleet), ipc::RunId::carried("01RUN"), phone.rig.fleet.events()));
    let asked = |caller: &'static str| Request::get("/settings").header(api::CALLER_HEADER, caller).body(Body::empty()).expect("a request");
    assert_eq!(fleet.clone().oneshot(asked(api::PHONE)).await.expect("answered").status(), StatusCode::NOT_FOUND);
    assert_eq!(fleet.oneshot(asked(api::BRIDGE)).await.expect("answered").status(), StatusCode::OK, "Bridge reads it as ever");
}

/// The phone's live stream, which the push watcher reads too: an unshared Job's change never reaches it.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_live_stream_carries_only_changes_to_jobs_in_a_shared_project() {
    let phone = Phone::paired().await;
    let jobs = jobs(&phone).await;
    let answer = pocket::router(phone.gateway.clone()).oneshot(phone.request("GET", "/api/live", "")).await.expect("answered");
    assert_eq!(answer.status(), StatusCode::OK);
    let mut body = answer.into_body();
    let mut heard = String::new();
    // The stream is open once its resync arrives.
    while !heard.contains("resync") {
        heard.push_str(&next_frame(&mut body).await);
    }
    let sessions = phone.rig.fleet.list_sessions(None, None, None).await.expect("listed").sessions;
    for job in [&jobs.private, &jobs.loose, &jobs.open] {
        let record = sessions.iter().find(|one| one.id == *job).expect("listed").clone();
        phone.rig.fleet.events().publish(Event::SessionChanged(record));
    }
    while !heard.contains(jobs.open.as_str()) {
        heard.push_str(&next_frame(&mut body).await);
    }
    assert!(!heard.contains(jobs.private.as_str()) && !heard.contains(jobs.loose.as_str()), "{heard}");
}

async fn next_frame(body: &mut Body) -> String {
    let frame = tokio::time::timeout(Duration::from_secs(10), body.frame()).await.expect("the stream stalled");
    frame.expect("the stream is open").expect("a frame").into_data().map(|bytes| String::from_utf8_lossy(&bytes).to_string()).unwrap_or_default()
}
