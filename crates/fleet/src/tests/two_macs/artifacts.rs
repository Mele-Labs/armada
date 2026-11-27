//! A Job's artifacts across his Macs, on loopback: Home lists and opens the page of a Job that Work holds by asking Work,
//! and his press, vote, archive and restore on it run on Work, once for each id, as acts.

use std::collections::BTreeMap;

use ipc::{ArtifactForm, ArtifactRef, CastPageVote, JobArtifact, PressPageAction, SessionId, VoteCast, VoteResult};
use store::{AttachmentState, Holder, KeptAttachment};

use super::acting::{acts_run, asleep, encoded, pair_over_a_shared_job, post, Pair};
use super::{until, OneMac};

const PAGE: &str = "<html><head><title>Layouts</title></head><body><h1>Layouts</h1>\
    <armada-action id=\"discuss-1\" act=\"message\" label=\"Discuss\" item=\"row-1\"></armada-action>\
    <armada-vote id=\"layout\" form=\"pick\" question=\"Which layout?\"><armada-take id=\"dense\" title=\"Dense\"><p>Tight</p></armada-take>\
    <armada-take id=\"roomy\" title=\"Roomy\"><p>Wide</p></armada-take></armada-vote></body></html>";

/// Work keeps `PAGE` as a page of its Job, revision 1, and answers with the page's path.
pub(super) async fn page_kept(work: &OneMac, job: &SessionId) -> String {
    let file = std::path::Path::new(&work.rig.root).join(".armada/documents/layouts.html");
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    std::fs::write(&file, PAGE).unwrap();
    let path = file.to_string_lossy().into_owned();
    let now = work.rig.fleet.now().as_str().to_string();
    let detail = [("form", "document".to_string()), ("title", "Layouts".to_string()), ("revisions", ipc::encode(&vec![path.clone()]).unwrap())]
        .into_iter()
        .map(|(key, value)| (key.to_string(), value))
        .collect();
    let held = KeptAttachment {
        holder: Holder::session(job.as_str()),
        kind: "artifact".into(),
        manifest_id: work.rig.manifest.as_str().to_string(),
        target: path.clone(),
        state: AttachmentState::Standing,
        detail,
        since: now.clone(),
        changed_at: now,
    };
    work.rig.fleet.store().lock().await.attach(&held, false).expect("kept");
    path
}

pub(super) async fn get(mac: &OneMac, path: &str) -> (axum::http::StatusCode, axum::http::HeaderMap, String) {
    use tower::ServiceExt;
    let request = axum::http::Request::get(path).header("x-armada-caller", "bridge").body(axum::body::Body::empty()).expect("a request");
    let answer = mac.fleet.clone().oneshot(request).await.expect("answered");
    let (status, headers) = (answer.status(), answer.headers().clone());
    let body = axum::body::to_bytes(answer.into_body(), 1 << 24).await.expect("a body");
    (status, headers, String::from_utf8_lossy(&body).to_string())
}

fn listed(body: &str) -> Vec<JobArtifact> {
    ipc::decode("the artifacts", body.as_bytes()).expect("a list")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn the_artifacts_and_page_of_a_job_another_mac_holds_are_read_from_that_mac() {
    let Pair { home, work, job } = pair_over_a_shared_job(false).await;
    let path = page_kept(&work, &job).await;
    assert!(home.rig.fleet.store().lock().await.attachments_of(&Holder::session(job.as_str())).unwrap().is_empty(), "Home keeps none of it");

    let (status, _, body) = get(&home, &format!("/sessions/artifacts?session_id={}", job.as_str())).await;
    assert_eq!(status, axum::http::StatusCode::OK, "{body}");
    let artifacts = listed(&body);
    assert_eq!(artifacts.iter().map(|one| (one.id.as_str(), one.form)).collect::<Vec<_>>(), [(path.as_str(), ArtifactForm::Page)]);

    let encoded_path: String = ipc::door::encoded(&path);
    let (status, headers, page) = get(&home, &format!("/sessions/artifacts/page?session_id={}&artifact={encoded_path}&revision=1", job.as_str())).await;
    assert_eq!(status, axum::http::StatusCode::OK, "{page}");
    assert_eq!(headers.get("content-type").and_then(|one| one.to_str().ok()), Some("text/html; charset=utf-8"));
    let policy = headers.get("content-security-policy").and_then(|one| one.to_str().ok()).expect("the policy rides as the header");
    assert!(page.contains(policy), "and is the page's own");
    assert!(page.contains("<h1>Layouts</h1>") && page.contains("armada-page-record"), "{page}");

    let (status, _, body) = get(&home, &format!("/sessions/artifacts/page?session_id={}&artifact=nothing.html", job.as_str())).await;
    assert_eq!(status, axum::http::StatusCode::UNPROCESSABLE_ENTITY, "Work's refusal, as it said it: {body}");
    assert!(body.contains("fleet.artifact_not_held"), "{body}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_vote_a_press_an_archive_and_a_restore_on_a_job_another_mac_holds_run_on_that_mac() {
    let Pair { home, work, job } = pair_over_a_shared_job(false).await;
    let path = page_kept(&work, &job).await;
    let heard = home.every_event().await;
    let before = heard.lock().expect("not poisoned").len();

    let cast = CastPageVote {
        session_id: job.clone(),
        artifact: path.clone(),
        revision: 1,
        vote: "layout".into(),
        result: VoteResult::Pick { take: "roomy".into(), notes: BTreeMap::new() },
    };
    let (status, body) = post(&home, "/sessions/artifacts/vote", encoded(&cast)).await;
    assert_eq!(status, axum::http::StatusCode::OK, "{body}");
    // What Work publishes for the vote reaches Home's own listeners, which is what draws the Artifacts tab again.
    let (heard, job_id) = (&heard, job.as_str().to_string());
    until("Home hears the Job change", || async {
        heard.lock().expect("not poisoned").iter().skip(before).any(|event| event.contains(&job_id) && event.contains("session.changed"))
    })
    .await;
    let voted: VoteCast = ipc::decode("the cast", body.as_bytes()).expect("a cast");
    assert_eq!(voted.result, cast.result);
    let at_work = work.rig.fleet.job_artifacts(job.as_str()).await.expect("Work's list");
    assert_eq!(at_work.iter().find(|one| one.id == path).expect("the page").votes, [voted], "the vote is kept where the Job is");

    let press = PressPageAction { session_id: job.clone(), artifact: path.clone(), revision: 1, action: "discuss-1".into(), words: None };
    let (status, body) = post(&home, "/sessions/artifacts/press", encoded(&press)).await;
    assert_eq!(status, axum::http::StatusCode::OK, "{body}");
    let at_work = work.rig.fleet.job_artifacts(job.as_str()).await.expect("Work's list");
    assert_eq!(at_work.iter().find(|one| one.id == path).expect("the page").presses.len(), 1);

    let seen = ipc::MarkArtifactSeen { session_id: job.clone(), artifact: path.clone(), revision: 1 };
    let (status, body) = post(&home, "/sessions/artifacts/seen", encoded(&seen)).await;
    assert_eq!(status, axum::http::StatusCode::OK, "{body}");
    assert_eq!(ipc::decode::<ipc::ArtifactSeen>("seen", body.as_bytes()).expect("seen").seen, 1);

    let target = ArtifactRef { artifact: path.clone(), session_id: Some(job.clone()) };
    let (status, body) = post(&home, "/sessions/artifacts/archive", encoded(&target)).await;
    assert_eq!(status, axum::http::StatusCode::OK, "{body}");
    let archived: JobArtifact = ipc::decode("an artifact", body.as_bytes()).expect("an artifact");
    assert_eq!(archived.archived.map(|one| one.by), Some(ipc::ArtifactBy::Owner), "his, as pressed on Home");
    let (status, body) = post(&home, "/sessions/artifacts/restore", encoded(&target)).await;
    assert_eq!(status, axum::http::StatusCode::OK, "{body}");
    assert!(ipc::decode::<JobArtifact>("an artifact", body.as_bytes()).expect("an artifact").archived.is_none());

    let ran = acts_run(&work).await;
    let kinds: Vec<&str> = ran.iter().map(|one| one.what.as_str()).collect();
    assert_eq!(kinds, ["artifact_vote", "artifact_press", "artifact_seen", "artifact_archive", "artifact_restore"]);
    assert!(ran.iter().all(|one| one.mac == home.id && one.via == "mac"), "{ran:?}");
    assert!(acts_run(&home).await.is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_vote_on_a_job_whose_mac_is_asleep_waits_and_goes_when_it_wakes() {
    let Pair { home, work, job } = pair_over_a_shared_job(false).await;
    let mut work = work;
    let path = page_kept(&work, &job).await;
    work.stop().await;
    asleep(&home, &work).await;

    let cast = CastPageVote {
        session_id: job.clone(),
        artifact: path.clone(),
        revision: 1,
        vote: "layout".into(),
        result: VoteResult::Pick { take: "dense".into(), notes: BTreeMap::new() },
    };
    let (status, body) = post(&home, "/sessions/artifacts/vote", encoded(&cast)).await;
    assert_eq!(status, axum::http::StatusCode::ACCEPTED, "{body}");
    let queued: ipc::Queued = ipc::decode("a queued act", body.as_bytes()).expect("the Queued");
    assert_eq!((queued.act.name(), queued.mac.as_str()), ("artifact_vote", work.id.as_str()));

    work.restart();
    let (work_ref, held) = (&work, job.clone());
    until("the vote went to Work", || async {
        let kept = work_ref.rig.fleet.job_artifacts(held.as_str()).await.expect("Work's list");
        kept.iter().any(|one| one.votes.len() == 1)
    })
    .await;
    // Work keeps the run after the vote itself and before it answers, and Home lets the act go only on that answer.
    until("and left Home's queue", || async { super::acting::queued(&home).await.queued.is_empty() }).await;
    assert_eq!(acts_run(&work).await.iter().map(|one| one.what.as_str()).collect::<Vec<_>>(), ["artifact_vote"]);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn his_answer_to_a_part_of_a_job_another_mac_holds_is_taken_on_that_mac() {
    use api::HostedSessions;
    let Pair { home, work, job } = pair_over_a_shared_job(false).await;
    let part = format!(r#"{{"session_id":"{}","title":"Cache design","question":"Keep the cache small?","part":{{"id":"notes","kind":"words","text":"Keep it small."}}}}"#, job.as_str());
    work.rig.fleet.add_part(None, ipc::decode("a part", part.as_bytes()).expect("a part")).await.expect("added");

    // Home reads the artifact through the link and answers the part it reads there.
    let (status, _, body) = get(&home, &format!("/sessions/artifacts?session_id={}", job.as_str())).await;
    assert_eq!(status, axum::http::StatusCode::OK, "{body}");
    let artifact = listed(&body).into_iter().find(|one| one.id == "sketch:Cache design").expect("the artifact");
    let notes = artifact.parts.iter().find(|one| one.id == "notes").expect("the words part");
    let answer = ipc::AnswerPart {
        session_id: job.clone(),
        artifact: artifact.id.clone(),
        part: "notes".into(),
        drawn: notes.drawn.clone(),
        how: ipc::PartHow::Approved,
        take: None,
        words: None,
        notes: BTreeMap::new(),
        extras: Default::default(),
    };
    let (status, body) = post(&home, "/sessions/artifacts/part/answer", encoded(&answer)).await;
    assert_eq!(status, axum::http::StatusCode::OK, "{body}");
    let after = work.rig.fleet.job_artifacts(job.as_str()).await.expect("Work's list");
    let kept = after.iter().find(|one| one.id == artifact.id).expect("the artifact");
    assert_eq!(kept.parts.iter().find(|one| one.id == "notes").map(|one| one.state), Some(ipc::PartState::Approved), "kept where the Job is");
    assert_eq!(acts_run(&work).await.iter().map(|one| one.what.as_str()).collect::<Vec<_>>(), ["artifact_part_answer"]);
    assert!(acts_run(&home).await.is_empty());
}
