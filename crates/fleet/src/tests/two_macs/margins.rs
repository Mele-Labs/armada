//! A page of a Job another Mac holds, as the native Bridge reads it, and its margin: Home asks Work for the page
//! sealed for a host that draws the kit itself and for the page's threads, and his comment there runs on Work as an act.

use std::sync::Arc;

use api::Margins;
use ipc::{CommentMargin, GetMarginThreads, MarginAnchor, MarginOn, MarginState, MarginThreads};

use super::acting::{acts_run, encoded, pair_over_a_shared_job, post, Pair};
use super::artifacts::{get, page_kept};

fn on_wide(job: &ipc::SessionId, text: &str) -> CommentMargin {
    CommentMargin {
        session_id: job.clone(),
        document: "layouts.html".into(),
        on: MarginOn::Passage { anchor: MarginAnchor { quote: "Wide".into(), block: None, section: None } },
        text: text.into(),
        hold: true,
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn the_hosted_page_of_a_job_another_mac_holds_is_read_from_that_mac() {
    let Pair { home, work, job } = pair_over_a_shared_job(false).await;
    let path = page_kept(&work, &job).await;

    let encoded_path: String = ipc::door::encoded(&path);
    let (status, headers, page) = get(&home, &format!("/sessions/artifacts/page?session_id={}&artifact={encoded_path}&host_kit=true", job.as_str())).await;
    assert_eq!(status, axum::http::StatusCode::OK, "{page}");
    let policy = headers.get("content-security-policy").and_then(|one| one.to_str().ok()).expect("the policy rides as the header");
    assert!(policy.contains("script-src 'none'"), "sealed for a host that draws the kit: {policy}");
    assert!(page.contains("<h1>Layouts</h1>") && page.contains("armada-page-record") && !page.contains("nonce="), "{page}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn the_margin_of_a_page_another_mac_holds_is_read_from_that_mac() {
    let Pair { home, work, job } = pair_over_a_shared_job(false).await;
    page_kept(&work, &job).await;
    Arc::clone(&work.rig.fleet).comment_margin(on_wide(&job, "Too loose?")).await.expect("held on Work");

    let (status, _, body) = get(&home, &format!("/sessions/margin?session_id={}&document=layouts.html", job.as_str())).await;
    assert_eq!(status, axum::http::StatusCode::OK, "{body}");
    let threads: MarginThreads = ipc::decode("the threads", body.as_bytes()).expect("threads");
    let at_work = work.rig.fleet.get_margin_threads(GetMarginThreads { session_id: job.clone(), document: "layouts.html".into() }).await.expect("Work's");
    assert_eq!(threads, at_work, "what Work holds is what Bridge is given");
    assert_eq!(threads.threads.len(), 1);

    let (status, _, body) = get(&home, &format!("/sessions/margin?session_id={}&document=nothing.html", job.as_str())).await;
    assert_eq!(status, axum::http::StatusCode::UNPROCESSABLE_ENTITY, "Work's refusal, as it said it: {body}");
    assert!(body.contains("fleet.margin_no_page"), "{body}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn his_margin_comment_on_a_job_another_mac_holds_runs_on_that_mac() {
    let Pair { home, work, job } = pair_over_a_shared_job(false).await;
    page_kept(&work, &job).await;

    let (status, body) = post(&home, "/sessions/margin/comment", encoded(&on_wide(&job, "Too loose?"))).await;
    assert_eq!(status, axum::http::StatusCode::OK, "{body}");
    let answered: MarginThreads = ipc::decode("the threads", body.as_bytes()).expect("threads");
    assert_eq!(answered.threads.iter().map(|one| one.state).collect::<Vec<_>>(), [MarginState::Held]);
    let at_work = work.rig.fleet.get_margin_threads(GetMarginThreads { session_id: job.clone(), document: "layouts.html".into() }).await.expect("Work's");
    assert_eq!(answered, at_work, "kept where the Job is");
    assert_eq!(acts_run(&work).await.iter().map(|one| one.what.as_str()).collect::<Vec<_>>(), ["margin_comment"]);
    assert!(acts_run(&home).await.is_empty());
}
