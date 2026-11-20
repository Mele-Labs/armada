//! A Job's row and thread across his Macs, on loopback: Home opens a Job that Work holds with `/sessions/one`, which asks
//! Work for the rows, and a window of them is cut on Work so that only it crosses.

use ipc::{SessionRow, SessionThread, WireError};

use super::acting::{pair_over_a_shared_job, Pair};
use super::OneMac;

async fn one(mac: &OneMac, path: &str) -> (axum::http::StatusCode, String) {
    use tower::ServiceExt;
    let request = axum::http::Request::get(path).header("x-armada-caller", "bridge").body(axum::body::Body::empty()).expect("a request");
    let answer = mac.fleet.clone().oneshot(request).await.expect("answered");
    let status = answer.status();
    (status, String::from_utf8_lossy(&axum::body::to_bytes(answer.into_body(), 1 << 24).await.expect("a body")).to_string())
}

/// `path` read from Work by Home, as its Fleet reads it: through the link.
async fn crossing(home: &OneMac, work: &OneMac, path: &str) -> (u16, Vec<u8>) {
    pocket::read_other_mac(&home.gateway, &work.id, path).await.expect("Work answered")
}

fn thread_of(body: &str) -> SessionThread {
    ipc::decode("a Job's thread", body.as_bytes()).expect("a thread")
}

fn ids(rows: &[SessionRow]) -> Vec<&str> {
    rows.iter().map(SessionRow::id).collect()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_job_another_mac_holds_opens_with_its_rows_and_its_mac() {
    let Pair { home, work, job } = pair_over_a_shared_job(false).await;
    work.rig.fleet.fleet_note(job.as_str(), "Work has begun".into()).await;

    let (status, body) = one(&home, &format!("/sessions/one?session_id={}", job.as_str())).await;
    assert_eq!(status, axum::http::StatusCode::OK, "{body}");
    let opened = thread_of(&body);
    let at_work = work.rig.rows(&job).await;
    assert!(!at_work.is_empty());
    assert_eq!(opened.rows, at_work, "the rows are Work's");
    assert_eq!(opened.session.id, job);
    assert_eq!(opened.session.mac.as_deref(), Some(work.id.as_str()), "as its row in Home's list says");
    assert!(home.rig.fleet.store().lock().await.session_rows(job.as_str()).expect("reads").is_empty(), "Home keeps none of it");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn the_newest_rows_of_a_job_another_mac_holds_open_and_the_ones_before_follow() {
    let Pair { home, work, job } = pair_over_a_shared_job(false).await;
    for note in 0..6 {
        work.rig.fleet.fleet_note(job.as_str(), format!("Note {note}")).await;
    }
    let at_work = work.rig.rows(&job).await;
    let n = at_work.len();
    assert!(n >= 6);
    let path = |window: &str| format!("/sessions/one?session_id={}&{window}", job.as_str());

    let (status, body) = one(&home, &path("last=2")).await;
    assert_eq!(status, axum::http::StatusCode::OK, "{body}");
    let newest = thread_of(&body);
    assert_eq!((ids(&newest.rows), newest.older), (ids(&at_work[n - 2..]), true), "the newest two, with more before them");
    assert_eq!(newest.session.mac.as_deref(), Some(work.id.as_str()));

    let (_, body) = one(&home, &path(&format!("last=2&before={}", ids(&newest.rows)[0]))).await;
    let earlier = thread_of(&body);
    assert_eq!((ids(&earlier.rows), earlier.older), (ids(&at_work[n - 4..n - 2]), true), "the two before those");

    let (_, body) = one(&home, &path(&format!("last={}", n + 5))).await;
    let all = thread_of(&body);
    assert_eq!((all.rows, all.older), (at_work, false), "a window past the start is the thread, with nothing before it");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn work_cuts_a_window_before_it_crosses_and_refuses_a_job_it_does_not_share_as_for_no_job() {
    let Pair { home, work, job } = pair_over_a_shared_job(false).await;
    for note in 0..6 {
        work.rig.fleet.fleet_note(job.as_str(), format!("Note {note}")).await;
    }
    let whole = work.rig.rows(&job).await.len();
    let (status, bytes) = crossing(&home, &work, &format!("/other_mac/session?session_id={}&last=2", job.as_str())).await;
    assert_eq!(status, 200, "{}", String::from_utf8_lossy(&bytes));
    let crossed = thread_of(&String::from_utf8_lossy(&bytes));
    let (_, everything) = crossing(&home, &work, &format!("/other_mac/session?session_id={}", job.as_str())).await;
    assert_eq!((crossed.rows.len(), whole > 2, bytes.len() < everything.len()), (2, true, true), "the window is smaller on the link");

    // A Job of Work's in no project is not shared, and is refused as a Job that does not exist. A second project
    // holds the repository, so the Job is in neither.
    super::super::share::project(&work.rig, "Yard").await;
    let private = super::super::share::job_in(&work.rig, None).await;
    let (status, bytes) = crossing(&home, &work, &format!("/other_mac/session?session_id={}&last=2", private.as_str())).await;
    let said: WireError = ipc::decode("Work's refusal", &bytes).expect("a refusal");
    assert_eq!((status, said.code.as_str(), said.message.as_str()), (404, "fleet.no_such_session", format!("No Job has the id {}.", private.as_str()).as_str()));
}
