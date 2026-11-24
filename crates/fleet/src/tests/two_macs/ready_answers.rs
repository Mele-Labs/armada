//! His answer on a Ready to try card of a Job another Mac holds, on loopback: the card reads against the build he runs
//! on Home, and his Looks good runs on Work as an act where Home's build holds the branch at its tip, and nowhere where
//! it does not.

use std::fs;

use axum::http::StatusCode;
use ipc::{AnswerReadyToTry, JobThread, NoteVerdict, ReadyAnswer, SessionId, ThreadItem, ThreadReadyToTry};

use super::acting::{acts_run, encoded, pair_over_a_shared_job, post, queued, Pair};
use super::OneMac;

const BRANCH: &str = "armada/try";
const TIP: &str = "a1b2c3d4e5f60718293a4b5c6d7e8f9012345678";

/// Work keeps an offer of `BRANCH` at `TIP` by its Job, which is the Job's one card.
async fn offered(work: &OneMac, job: &SessionId) {
    let at = work.rig.fleet.now().as_str().to_string();
    let mut store = work.rig.fleet.store().lock().await;
    store.keep_preview_offered(job.as_str(), BRANCH, TIP, "Keeps one thing", None, &[], &at).expect("kept");
}

/// Home runs a preview build holding `BRANCH` at `TIP`, as `scripts/restart` and `scripts/preview` record it.
fn home_builds_it(home: &OneMac) {
    let support = crate::runtime::support_dir(&home.rig.fleet.host().home);
    fs::create_dir_all(&support).unwrap();
    let commit = "c".repeat(40);
    fs::write(support.join("restart-commit"), format!("{commit}\n")).unwrap();
    fs::write(support.join("restart-source"), "/Users/user/armada/.armada/preview\n").unwrap();
    fs::write(support.join("preview-held"), format!("{commit}\n{}\n{BRANCH}\t{TIP}\n", "b".repeat(40))).unwrap();
}

/// The card in Home's read of the Job's thread.
async fn card_on_home(home: &OneMac, job: &SessionId) -> ThreadReadyToTry {
    use tower::ServiceExt;
    let request = axum::http::Request::get(format!("/sessions/thread?session_id={}", job.as_str()))
        .header("x-armada-caller", "bridge")
        .body(axum::body::Body::empty())
        .expect("a request");
    let answer = home.fleet.clone().oneshot(request).await.expect("answered");
    assert_eq!(answer.status(), StatusCode::OK);
    let body = axum::body::to_bytes(answer.into_body(), 1 << 24).await.expect("a body");
    let thread: JobThread = ipc::decode("the thread", &body).expect("a thread");
    thread.items.into_iter().find_map(|item| match item {
        ThreadItem::ReadyToTry(card) => Some(card),
        _ => None,
    }).expect("the card")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn his_answer_on_a_card_of_a_job_another_mac_holds_goes_there_only_where_his_build_here_holds_it() {
    let Pair { home, work, job } = pair_over_a_shared_job(false).await;
    offered(&work, &job).await;
    let looks_good = AnswerReadyToTry { session_id: job.clone(), branch: BRANCH.into(), tip: TIP.into(), answer: ReadyAnswer::LooksGood };

    assert!(!card_on_home(&home, &job).await.in_build, "Home runs main");
    let (status, body) = post(&home, "/sessions/ready_to_try/answer", encoded(&looks_good)).await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    assert!(body.contains("fleet.ready_not_in_build"), "{body}");
    assert!(acts_run(&work).await.is_empty() && queued(&home).await.queued.is_empty(), "nothing went, and nothing waits");

    // Work runs main, so only Home's own build can put the card in.
    home_builds_it(&home);
    assert!(card_on_home(&home, &job).await.in_build, "read against the build he runs on Home");
    let (status, body) = post(&home, "/sessions/ready_to_try/answer", encoded(&looks_good)).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let card: ThreadReadyToTry = ipc::decode("the card", body.as_bytes()).expect("a card");
    assert_eq!(card.verdict, Some(NoteVerdict::Good), "Work's card, as it stands after");
    let note = work.rig.fleet.store().lock().await.build_note_of(BRANCH, TIP).expect("reads").expect("the note");
    assert_eq!(note.verdict.as_deref(), Some("good"), "kept where the Job is");

    let ran = acts_run(&work).await;
    assert_eq!(ran.iter().map(|one| one.what.as_str()).collect::<Vec<_>>(), ["ready_answer"]);
    assert!(ran.iter().all(|one| one.mac == home.id && one.via == "mac"), "{ran:?}");
    assert!(acts_run(&home).await.is_empty());
}
