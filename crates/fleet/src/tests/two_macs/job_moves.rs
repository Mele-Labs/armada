//! A running Job moved from Work to Home: it finishes its turn, its branch is pushed, Work's slot is freed, and
//! Home resumes the same conversation under the same id, read across in pieces. A Job in an unshared project
//! cannot move, and one that moved into a copy stops crossing back once its project is unshared.

use std::sync::Arc;

use adapter_traits::Vcs;
use api::{HostedSessions, OtherMacs, Refusal};
use ipc::{Event, JobThreadRead, MoveJobToMac, SessionId, SessionState};
use testkit::{Delivered, Delivering, FakeVcs};

use super::{from, job_in, paired, project, shared, until, OneMac, Ticking};
use crate::tests::session_host::denied;

/// More than one piece of the link's reads, so the conversation crosses in several.
fn a_long_conversation() -> Vec<u8> {
    let line = format!("{{\"type\":\"user\",\"message\":\"{}\"}}\n", "x".repeat(1000));
    line.repeat(700).into_bytes()
}

/// Job `id`'s conversation kept where the CLI keeps one for slot 1, under `home`.
fn conversation_kept(home: &str, id: &SessionId, bytes: &[u8]) {
    adapters::terminal_thread::keep_conversation(home, id.as_str(), "/work/slot-1", bytes).expect("the conversation");
}

/// Mac `other` as `mac`'s Fleet knows it. On a Mac the Gateway and Fleet share one store, so Fleet reads the
/// names the Gateway paired; here each has its own, so the pairing is copied across.
async fn known_to(mac: &OneMac, other: &OneMac, name: &str) {
    let paired = store::PairedMac {
        id: other.id.clone(),
        name: name.into(),
        colour: "teal".into(),
        address: "http://127.0.0.1".into(),
        public_key: Some(vec![1]),
        build: None,
        paired_at: 1,
        state: store::MacState::Linked,
        back_code: None,
        last_heard_at: None,
        asleep_since: None,
        why: None,
        wire: None,
        built_at: None,
        source: None,
        pin: None,
    };
    mac.rig.fleet.store().lock().await.keep_mac(&paired).expect("paired");
}

fn move_to(job: &SessionId, mac: Option<&str>) -> MoveJobToMac {
    MoveJobToMac { session_id: job.clone(), mac: mac.map(str::to_string) }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_running_job_finishes_its_turn_and_home_resumes_its_conversation_in_a_fresh_slot() {
    let clock = Arc::new(Ticking::from_nine());
    let (home, work) = (OneMac::new("Home", "01HMAC", &clock).await, OneMac::new("Work", "01WMAC", &clock).await);
    let shop = project(&work.rig, "Shop").await;
    shared(&work.rig, &shop, true).await;
    let job = job_in(&work.rig, Some(&shop)).await;
    // A turn running, and slot 1 leased by its first write.
    work.rig.send(&job, "change a file").await;
    work.rig.stand_in.init(0);
    let path = format!("{}/src/lib.rs", work.rig.root);
    assert!(denied(&work.rig.gate(&job, "Edit", &format!(r#"{{"file_path":"{path}"}}"#)).await));
    let conversation = a_long_conversation();
    conversation_kept(&work.rig.fleet.host().home, &job, &conversation);
    // He shares on both Macs, so each knows the other.
    let garden = project(&home.rig, "Garden").await;
    shared(&home.rig, &garden, true).await;
    paired(&home, &work).await;
    let on_home = job.clone();
    until("Home lists Work's shared Job", || async { from(&home.sessions().await.1, &on_home).is_some() }).await;
    until("Home holds Work's project", || async { home.projects().await.1.projects.iter().any(|one| one.id == shop) }).await;
    until("Work holds Home's project", || async { work.projects().await.1.projects.iter().any(|one| one.id == garden) }).await;

    known_to(&work, &home, "Home").await;
    let moving = Arc::clone(&work.rig.fleet).move_job_to_mac(move_to(&job, Some(&home.id))).await.expect("a move begun");
    assert_eq!(moving.hosted.as_ref().and_then(|one| one.moving_to_mac.clone()), Some(home.id.clone()));
    let again = Arc::clone(&work.rig.fleet).move_job_to_mac(move_to(&job, Some(&home.id))).await.expect_err("moving already");
    assert_eq!(again.error().message, "This Job is already moving to Home.", "named by the Mac's name, never its id");
    // The turn is still running, so nothing is handed over yet.
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    assert!(home.rig.stand_in.starts().is_empty(), "Home started the Job before Work's turn ended");

    // The write was held for slot 1, so the turn carries on in a process there, and ends there.
    work.rig.stand_in.finishes(0, "Waiting for the slot.");
    until("the turn carries on in slot 1", || async { work.rig.stand_in.starts().len() == 2 }).await;
    assert!(home.rig.stand_in.starts().is_empty(), "Home started the Job before Work's turn ended");
    work.rig.stand_in.init(1);
    work.rig.stand_in.finishes(1, "Done with the file.");

    let on_home = job.clone();
    until("Home holds the Job itself", || async { from(&home.sessions().await.1, &on_home) == Some(None) }).await;
    let on_home = job.clone();
    until("Home resumes the Job's conversation", || async {
        home.rig.stand_in.starts().iter().any(|start| start.session == on_home.as_str() && start.resuming)
    })
    .await;
    let kept = adapters::terminal_thread::find(&home.rig.fleet.host().home, job.as_str()).expect("Home keeps the conversation");
    assert_eq!(std::fs::read(kept).expect("read"), conversation, "the conversation arrived whole");

    // Work pushed the branch, ended its own copy and gave its slot back, and lists Home's.
    let pushed = work.rig.fleet.vcs().delivered();
    assert!(pushed.iter().any(|one| matches!(one, Delivered::Pushed { .. })), "the branch was not pushed: {pushed:?}");
    let on_work = job.clone();
    until("Work lists the Job as Home's", || async { from(&work.sessions().await.1, &on_work) == Some(Some(home.id.clone())) }).await;
    let left = work.rig.fleet.store().lock().await.session(job.as_str()).expect("read").expect("Work's row");
    assert_eq!(left.state, store::SessionState::Ended);
    assert_eq!(left.end_reason.as_deref(), Some("moved_to_mac"));
    let pool = crate::leasing::pool_of(&work.rig.fleet.repositories().first().expect("a repository"));
    assert_ne!(work.rig.fleet.vcs().slot_standing(&pool, 1, job.as_str()), adapter_traits::SlotStanding::Held, "Work's slot is still held");
    let (_, listed) = home.sessions().await;
    let row = listed.sessions.iter().find(|one| one.id == job).expect("Home's row");
    assert_eq!(row.state, SessionState::Live);

    // **Unshared, the moved Job stops crossing back**: Work turns Shop off, so Home drops its copy, and the Job,
    // still running on Home in a project Home no longer holds, is neither listed, streamed nor read to Work.
    shared(&work.rig, &shop, false).await;
    until("Home drops its copy of Shop", || async { !home.projects().await.1.projects.iter().any(|one| one.id == shop) }).await;
    assert!(home.rig.fleet.share_sessions().await.expect("listed").sessions.iter().all(|one| one.id != job), "Home still shares it");
    let record = Arc::clone(&home.rig.fleet).get_session(job.clone()).await.expect("Home's Job").session;
    assert!(home.rig.fleet.share_event(Event::SessionChanged(record)).await.is_none(), "a live change of it still crosses");
    let read = JobThreadRead { session_id: job.clone(), before: None, limit: None, upto: None };
    let refused = Arc::clone(&home.rig.fleet).share_job_thread(read).await.expect_err("its thread is refused");
    assert!(matches!(refused, Refusal::NotFound(_)), "not refused as no Job: {refused:?}");
    let on_work = job.clone();
    until("Work no longer lists the Job as Home's", || async { from(&work.sessions().await.1, &on_work) != Some(Some(home.id.clone())) }).await;
}

/// **A move that cannot push loses nothing**: the Job stays on Work, resumed and told why, and Home starts nothing.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_move_whose_branch_cannot_be_pushed_leaves_the_job_running_where_it_was() {
    let clock = Arc::new(Ticking::from_nine());
    let home = OneMac::new("Home", "01HMAC", &clock).await;
    let work = OneMac::fitted("Work", "01WMAC", &clock, |fittings| {
        let vcs = std::mem::replace(&mut fittings.vcs, FakeVcs::new());
        fittings.vcs = vcs.delivering(Delivering { push: adapter_traits::Pushed::NoRemote, ..Delivering::default() });
    })
    .await;
    let shop = project(&work.rig, "Shop").await;
    shared(&work.rig, &shop, true).await;
    let job = job_in(&work.rig, Some(&shop)).await;
    work.rig.send(&job, "change a file").await;
    work.rig.stand_in.init(0);
    let path = format!("{}/src/lib.rs", work.rig.root);
    assert!(denied(&work.rig.gate(&job, "Edit", &format!(r#"{{"file_path":"{path}"}}"#)).await));
    work.rig.stand_in.finishes(0, "Waiting for the slot.");
    until("the turn carries on in slot 1", || async { work.rig.stand_in.starts().len() == 2 }).await;
    let garden = project(&home.rig, "Garden").await;
    shared(&home.rig, &garden, true).await;
    paired(&home, &work).await;
    until("Work holds Home's project", || async { work.projects().await.1.projects.iter().any(|one| one.id == garden) }).await;

    Arc::clone(&work.rig.fleet).move_job_to_mac(move_to(&job, Some(&home.id))).await.expect("a move begun");
    work.rig.stand_in.init(1);
    work.rig.stand_in.finishes(1, "Done with the file.");

    let fleet = Arc::clone(&work.rig.fleet);
    let left = job.clone();
    until("the Job is resumed on Work", || async {
        let record = fleet.published_hosted(left.as_str()).await.expect("Work's row");
        let hosted = record.hosted.expect("hosted");
        record.state == SessionState::Live && hosted.paused.is_none() && hosted.moving_to_mac.is_none()
    })
    .await;
    assert!(home.rig.stand_in.starts().is_empty(), "Home started a Job that did not move");
    let on_home = job.clone();
    until("Home lists the Job as Work's", || async { from(&home.sessions().await.1, &on_home).flatten() == Some(work.id.clone()) }).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_job_in_an_unshared_project_cannot_move_and_is_answered_as_one_that_is_not_there() {
    let clock = Arc::new(Ticking::from_nine());
    let (home, work) = (OneMac::new("Home", "01HMAC", &clock).await, OneMac::new("Work", "01WMAC", &clock).await);
    let (shop, taxes) = (project(&work.rig, "Shop").await, project(&work.rig, "Taxes").await);
    shared(&work.rig, &shop, true).await;
    let shown = job_in(&work.rig, Some(&shop)).await;
    let private = job_in(&work.rig, Some(&taxes)).await;
    paired(&home, &work).await;
    let on_home = shown.clone();
    until("Home lists Work's shared Job", || async { from(&home.sessions().await.1, &on_home).is_some() }).await;

    let refused = Arc::clone(&work.rig.fleet).move_job_to_mac(move_to(&private, Some(&home.id))).await.expect_err("refused");
    let Refusal::NotFound(why) = refused else { panic!("not a 404: {refused:?}") };
    assert_eq!(why.code, "fleet.no_such_session");
    let row = work.rig.fleet.store().lock().await.session(private.as_str()).expect("read").expect("the private Job");
    assert_eq!(row.state, store::SessionState::Live, "the refused move changed the Job");
    assert!(work.rig.fleet.hosts().of(private.as_str()).state().to_mac.is_none());
}
