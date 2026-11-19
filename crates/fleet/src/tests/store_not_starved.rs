//! A pass over every Job's row lets the other routes at the store between rows, rather than holding
//! its one lock for the whole pass.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

use api::{HostedSessions, Next, Sessions};
use ipc::{SessionFact, SessionId, SessionReport};
use testkit::{FakeHarness, FakeVcs, FakeWorkProduct};

use crate::daemon::Fleet;
use crate::tests::daemon::fitted_with;
use crate::tests::tmp::TempDir;

const ROWS: usize = 400;

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn another_route_gets_the_store_while_the_job_list_is_being_read() {
    let home = TempDir::new();
    let fleet: Arc<Fleet<FakeHarness, FakeVcs, FakeWorkProduct>> =
        Arc::new(Fleet::assembled(fitted_with(&home, FakeWorkProduct::changed(&[]), FakeHarness::that_listens())));
    for one in 0..ROWS {
        fleet
            .report_session(SessionReport {
                harness: "a_harness".into(),
                session_id: SessionId::carried(format!("s{one}")),
                fact: SessionFact::Started {
                    cwd: "/nowhere".into(),
                    title: Some(format!("job {one}")),
                    origin: ipc::SessionOrigin::Terminal,
                    mod_version: None,
                },
            })
            .await
            .unwrap();
    }

    let listing = Arc::new(AtomicBool::new(true));
    let longest_wait = Arc::new(AtomicU64::new(0));
    let other = {
        let (fleet, listing, longest_wait) = (Arc::clone(&fleet), Arc::clone(&listing), Arc::clone(&longest_wait));
        tokio::spawn(async move {
            while listing.load(Ordering::SeqCst) {
                let asked = Instant::now();
                drop(fleet.store().lock().await);
                longest_wait.fetch_max(asked.elapsed().as_micros() as u64, Ordering::SeqCst);
                tokio::task::yield_now().await;
            }
        })
    };
    // Let the other route spin before the pass starts, so it is asking when the pass begins.
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    longest_wait.store(0, Ordering::SeqCst);
    let began = Instant::now();
    let listed = fleet.list_sessions(None, None, None).await.expect("listed");
    let pass = began.elapsed().as_micros() as u64;
    let waited = longest_wait.load(Ordering::SeqCst);
    listing.store(false, Ordering::SeqCst);
    other.await.unwrap();

    assert_eq!(listed.sessions.len(), ROWS);
    assert!(waited * 4 < pass, "another route waited {waited}us for the store in a pass of {pass}us");
}
