//! A Job's terminal across his Macs, on loopback. Work opens a terminal of a shared Job Home holds, types a command
//! and sees what it printed, and the shell runs on Home. A private Job's terminal, a run that is not the Job's, and
//! a call that is not signed reach nothing; a Mac that sleeps ends the terminal, and nothing typed meanwhile runs
//! when it wakes.

use std::panic::AssertUnwindSafe;
use std::sync::Arc;
use std::time::Duration;

use api::{Refusal, SessionRunEvent, Sessions};
use axum::http::{Method, StatusCode};
use futures_util::FutureExt;
use ipc::{OpenTerminal, SessionId, TerminalFromMac, WriteSessionRun};
use tokio::sync::broadcast;

use super::{job_in, paired, project, shared, until, OneMac, Ticking};

fn open(job: &SessionId) -> OpenTerminal {
    OpenTerminal { session_id: job.clone(), terminal: None, new: false, cols: None, rows: None }
}

/// What `live` says from now until `said` holds of all of it, for at most ten seconds.
async fn heard(live: &mut broadcast::Receiver<SessionRunEvent>, said: impl Fn(&str) -> bool) -> String {
    let mut all = String::new();
    let _ = tokio::time::timeout(Duration::from_secs(10), async {
        while let Ok(event) = live.recv().await {
            if let SessionRunEvent::Output(text) = event {
                all.push_str(&text);
                if said(&all) {
                    return;
                }
            }
        }
    })
    .await;
    all
}

/// What run `run_id` printed on `mac`, as its own Fleet holds it.
async fn printed_on(mac: &OneMac, run_id: &str) -> String {
    mac.rig.fleet.watch_session_run(run_id).map(|observed| observed.history).unwrap_or_default()
}

/// `asked`, sent from `from`'s Gateway to `to`'s as the link sends it: the status and words.
async fn sent(from: &OneMac, to: &OneMac, asked: &TerminalFromMac) -> (u16, String) {
    let body = ipc::encode(asked).expect("encodes").into_bytes();
    let (status, bytes) = pocket::send_other_mac(&from.gateway, &to.id, "/other_mac/terminal", body).await.expect("sent");
    (status, String::from_utf8_lossy(&bytes).to_string())
}

/// Stop a shell on `mac` and wait for it to go: a runtime that shuts down waits on a shell still open.
async fn ends(mac: &OneMac, run_id: &str) {
    let _ = mac.rig.fleet.stop_session_run(ipc::StopSessionRun { run_id: run_id.into() }).await;
    until("the shell ended", || async { mac.rig.fleet.watch_session_run(run_id).is_ok_and(|observed| observed.live.is_err()) }).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn work_types_into_a_terminal_of_homes_shared_job_and_sees_what_it_printed() {
    let clock = Arc::new(Ticking::from_nine());
    let (mut home, work) = (OneMac::new("Home", "01HMAC", &clock).await, OneMac::new("Work", "01WMAC", &clock).await);
    let (open_project, private) = (project(&home.rig, "Shop").await, project(&home.rig, "Taxes").await);
    shared(&home.rig, &open_project, true).await;
    let (s, p) = (job_in(&home.rig, Some(&open_project)).await, job_in(&home.rig, Some(&private)).await);
    paired(&home, &work).await;
    until("S reaches Work's Jobs list", || async { super::from(&work.sessions().await.1, &s).is_some() }).await;

    // A runtime that shuts down waits on a shell still open, so every shell ends before a failure is said.
    let outcome = AssertUnwindSafe(across(&mut home, &work, &s, &p)).catch_unwind().await;
    for job in [&s, &p] {
        for (_, run_id) in home.rig.fleet.session_runs().shells_open(job.as_str()) {
            ends(&home, &run_id).await;
        }
    }
    if let Err(failed) = outcome {
        std::panic::resume_unwind(failed);
    }
}

async fn across(home: &mut OneMac, work: &OneMac, s: &SessionId, p: &SessionId) {
    // 1. Work opens S's terminal, as Bridge does at home: the shell runs on Home, in S's own terminal.
    let opened = Arc::clone(&work.rig.fleet).open_terminal(open(s)).await.expect("opened on Home");
    assert_eq!(opened.terminal.as_deref(), Some("t1"));
    let on_home = home.rig.fleet.session_runs().shells_open(s.as_str());
    assert_eq!(on_home, [(1, opened.run_id.clone())], "the shell is Home's");
    let listed = work.rig.fleet.list_terminals(None, ipc::ListTerminals { session_id: Some(s.clone()) }).await.expect("listed");
    assert_eq!(listed.terminals.iter().map(|one| one.run_id.clone()).collect::<Vec<_>>(), [opened.run_id.clone()]);

    // 2. He types a command on Work and sees its output there: the sum is printed only once the shell ran it.
    let mut live = work.rig.fleet.observe_session_run(opened.run_id.clone()).await.expect("followed").live.expect("running");
    let typed = WriteSessionRun { run_id: opened.run_id.clone(), data: "echo across-$((40+2))\r".into() };
    work.rig.fleet.write_session_run(typed).await.expect("typed on Home");
    let came = heard(&mut live, |all| all.contains("across-42")).await;
    assert!(came.contains("across-42"), "Work saw: {came:?}");

    // 3. A private Job's terminal is answered as a Job that does not exist, and nothing opens on Home.
    let (status, said) = sent(work, home, &TerminalFromMac::Open(open(p))).await;
    assert_eq!(status, 404, "{said}");
    assert!(said.contains("fleet.no_such_session"), "{said}");
    assert!(home.rig.fleet.session_runs().shells_open(p.as_str()).is_empty(), "nothing opened for P");

    // 4. A run that is not one of S's terminals is answered alike, and nothing is typed into it.
    let private_run = Arc::clone(&home.rig.fleet).open_terminal(open(p)).await.expect("Home's own").run_id;
    let into_p = TerminalFromMac::Write { session_id: s.clone(), run_id: private_run.clone(), data: "echo leaked-$((1+1))\r".into() };
    let (status, said) = sent(work, home, &into_p).await;
    assert_eq!(status, 404, "{said}");

    // 5. Typing that is not signed runs nothing.
    let unsigned = TerminalFromMac::Write { session_id: s.clone(), run_id: opened.run_id.clone(), data: "echo unsigned-$((2+3))\r".into() };
    let (status, said) = home.admin(Method::POST, "/api/mac/terminal", ipc::encode(&unsigned).expect("encodes")).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{said}");
    assert!(said.contains("not signed"), "{said}");
    tokio::time::sleep(Duration::from_millis(500)).await;
    assert!(!printed_on(home, &opened.run_id).await.contains("unsigned-5"), "an unsigned call typed");
    assert!(!printed_on(home, &private_run).await.contains("leaked-2"), "a run not S's was typed into");

    // 6. Home sleeps: the terminal on Work ends, and a keystroke meanwhile is refused, never kept for later.
    home.stop().await;
    let ended = tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            match live.recv().await {
                Ok(SessionRunEvent::Exited(end)) => return Some(end),
                Ok(_) => continue,
                Err(_) => return None,
            }
        }
    })
    .await;
    assert!(matches!(ended, Ok(Some(end)) if end.code.is_none()), "Work's terminal ended: {ended:?}");
    let asleep = WriteSessionRun { run_id: opened.run_id.clone(), data: "echo asleep-$((3+4))\r".into() };
    assert!(matches!(work.rig.fleet.write_session_run(asleep).await, Err(Refusal::Unacceptable(_) | Refusal::Fault(_))));
    home.restart();
    until("Home is linked again", || async { work.mac(&home.id).await.is_some_and(|mac| mac.link == Some(ipc::MacLink::Linked)) }).await;
    tokio::time::sleep(Duration::from_millis(500)).await;
    assert!(!printed_on(home, &opened.run_id).await.contains("asleep-7"), "a keystroke typed on wake");
}
