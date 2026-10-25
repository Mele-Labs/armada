//! The agents' account signed out, for the whole Fleet: a Job's failed turn signs it out, and the model
//! answering anywhere, or the sign-in Fleet runs on Bridge's press exiting clean, signs it back in and
//! wakes the Jobs it stopped.
//! `docs/concepts/session/a-job-whose-sign-in-expired.md`.

use std::sync::Arc;
use std::time::Duration;

use adapter_traits::{DroneEvent, Speaker};
use api::{HostedSessions, Next, Subscription};
use ipc::{AgentSignIn, Event, SessionId, SessionRow};

use crate::session_host::signed_out::SignInRun;
use crate::tests::session_host::{eventually, rig_placing, Rig};

/// A hosted Job whose first turn has started, and the index of its process.
async fn a_working_job(rig: &Rig) -> (SessionId, usize) {
    let id = rig.start().await;
    rig.send(&id, "fix the ledger").await;
    eventually(|| async { rig.stand_in.process_of(id.as_str()).is_some() }).await;
    let process = rig.stand_in.starts().iter().rposition(|one| one.session == id.as_str()).unwrap();
    rig.stand_in.init(process);
    (id, process)
}

/// The Job's agent tried a turn and the harness's sign-in had run out, as it says it.
fn signs_out(rig: &Rig, process: usize) {
    rig.stand_in.says(
        process,
        vec![
            DroneEvent::Said {
                text: "Failed to authenticate: OAuth session expired and could not be refreshed".into(),
                by: Speaker::Harness,
            },
            DroneEvent::SignedOut,
        ],
    );
}

/// Whether Fleet holds `id` as one the expiry stopped.
fn marked(rig: &Rig, id: &SessionId) -> bool {
    rig.fleet.hosts().of(id.as_str()).state().signed_out
}

async fn state(rig: &Rig) -> AgentSignIn {
    rig.fleet.get_agent_sign_in().await.unwrap()
}

/// The words Fleet sent a hosted Job under `word`.
async fn noticed(rig: &Rig, id: &SessionId, word: &str) -> Vec<String> {
    rig.rows(id)
        .await
        .into_iter()
        .filter_map(|row| match row {
            SessionRow::Message { notice: Some(said), text, .. } if said == word => Some(text),
            _ => None,
        })
        .collect()
}

async fn next_state(watching: &mut Subscription) -> AgentSignIn {
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            match watching.next().await {
                Some(Next::Send(delivered)) => {
                    if let Event::AgentSignInChanged(state) = delivered.event {
                        return state;
                    }
                }
                other => panic!("the stream ended or dropped: {other:?}"),
            }
        }
    })
    .await
    .expect("an agent_sign_in.changed arrives")
}

/// **Every Job shares the account**, so one Job signed out is the Fleet signed out: Bridge is sent the
/// state whole, and no Job carries an item of its own for it.
#[tokio::test]
async fn a_job_signing_out_signs_the_fleet_out() {
    let rig = rig_placing(None);
    let (id, process) = a_working_job(&rig).await;
    let (other, _) = a_working_job(&rig).await;
    let mut watching = rig.fleet.events().subscribe();
    signs_out(&rig, process);

    let published = next_state(&mut watching).await;
    assert!(published.signed_out_since.is_some(), "{published:?}");
    assert!(!published.signing_in);
    assert_eq!(state(&rig).await, published, "what is read is what was sent");
    assert!(marked(&rig, &id));
    assert!(!marked(&rig, &other), "a Job that never failed is not one the expiry stopped");
    for one in [&id, &other] {
        let record = Arc::clone(&rig.fleet).get_session(one.clone()).await.unwrap().session;
        assert!(!record.waiting_for.iter().any(|item| item.id.starts_with("signin:")), "{:?}", record.waiting_for);
    }
}

/// **The harness says `init` before a turn that fails as well**, so a turn starting is no sign-in. A
/// failed turn after, in the same Job or another, keeps when the Fleet signed out.
#[tokio::test]
async fn a_repeat_failure_keeps_when_the_fleet_signed_out() {
    let rig = rig_placing(None);
    let (id, process) = a_working_job(&rig).await;
    signs_out(&rig, process);
    eventually(|| async { state(&rig).await.signed_out_since.is_some() }).await;
    let first = state(&rig).await;

    rig.send(&id, "where are we").await;
    rig.stand_in.init(process);
    signs_out(&rig, process);
    let (other, second) = a_working_job(&rig).await;
    signs_out(&rig, second);
    eventually(|| async { marked(&rig, &other) }).await;
    assert_eq!(state(&rig).await, first);
    assert!(marked(&rig, &id));
}

/// The model answering in any Job is the account working again, however that came about: he may have
/// signed in at a terminal. Every mark is cleared and each other Job the expiry stopped is woken.
#[tokio::test]
async fn an_answer_anywhere_signs_the_fleet_back_in_and_wakes_the_rest() {
    let rig = rig_placing(None);
    let (first, one) = a_working_job(&rig).await;
    let (second, two) = a_working_job(&rig).await;
    let (answering, three) = a_working_job(&rig).await;
    signs_out(&rig, one);
    signs_out(&rig, two);
    eventually(|| async { marked(&rig, &first) && marked(&rig, &second) }).await;

    rig.stand_in.says(three, vec![DroneEvent::Said { text: "On it.".into(), by: Speaker::Drone }]);
    eventually(|| async { state(&rig).await == AgentSignIn::default() }).await;
    for id in [&first, &second] {
        eventually(|| async { noticed(&rig, id, "sign_in_wake").await == ["Continue"] }).await;
        assert!(!marked(&rig, id));
    }
    assert!(noticed(&rig, &answering, "sign_in_wake").await.is_empty(), "the Job that answered is working");
}

/// A sign-in that ends at once with `code`, in place of the browser. `/bin/sh` is named whole, since
/// the run is given the fixture's `PATH`.
fn planted(rig: &Rig, script: &str) {
    rig.fleet.agent_sign_in().plant(SignInRun {
        program: "/bin/sh".into(),
        args: vec!["-c".into(), script.into()],
        directory: rig.root.clone(),
        path: "/usr/bin:/bin".into(),
    });
}

/// **One sign-in is every Job's**, since they share the account: each Job the expiry stopped is woken,
/// and a Job that never signed out is left alone.
#[tokio::test]
async fn a_clean_sign_in_wakes_every_stopped_job() {
    let rig = rig_placing(None);
    planted(&rig, "exit 0");
    let (first, one) = a_working_job(&rig).await;
    let (second, two) = a_working_job(&rig).await;
    let (untouched, _) = a_working_job(&rig).await;
    signs_out(&rig, one);
    signs_out(&rig, two);
    eventually(|| async { marked(&rig, &first) && marked(&rig, &second) }).await;

    let pressed = Arc::clone(&rig.fleet).start_agent_sign_in().await.unwrap();
    assert!(pressed.signing_in, "{pressed:?}");
    eventually(|| async { state(&rig).await == AgentSignIn::default() }).await;
    for id in [&first, &second] {
        eventually(|| async { noticed(&rig, id, "sign_in_wake").await == ["Continue"] }).await;
        assert!(!marked(&rig, id));
    }
    assert!(noticed(&rig, &untouched, "sign_in_wake").await.is_empty(), "a Job that never signed out is left alone");
}

/// A sign-in that fails, or that he gave up on in the browser, leaves the Fleet signed out as it was,
/// and Bridge may offer it again.
#[tokio::test]
async fn a_failed_sign_in_leaves_the_fleet_signed_out() {
    let rig = rig_placing(None);
    planted(&rig, "exit 1");
    let (id, process) = a_working_job(&rig).await;
    signs_out(&rig, process);
    eventually(|| async { state(&rig).await.signed_out_since.is_some() }).await;
    let before = state(&rig).await;

    assert!(Arc::clone(&rig.fleet).start_agent_sign_in().await.unwrap().signing_in);
    eventually(|| async { !state(&rig).await.signing_in }).await;
    assert_eq!(state(&rig).await, before);
    assert!(marked(&rig, &id));
    assert!(noticed(&rig, &id, "sign_in_wake").await.is_empty());
}

/// **One sign-in at a time**: a second press while the first waits on the browser starts nothing. The
/// run holds its input open, so one that reads it waits rather than ending.
#[tokio::test]
async fn a_second_press_while_signing_in_starts_nothing() {
    let rig = rig_placing(None);
    let started = std::path::Path::new(&rig.root).join("sign-ins");
    planted(&rig, &format!("echo started >> '{}'; read waiting", started.display()));
    let runs = || std::fs::read_to_string(&started).map(|text| text.lines().count()).unwrap_or(0);

    assert!(Arc::clone(&rig.fleet).start_agent_sign_in().await.unwrap().signing_in);
    eventually(|| async { runs() == 1 }).await;
    let again = Arc::clone(&rig.fleet).start_agent_sign_in().await.unwrap();
    assert!(again.signing_in, "{again:?}");
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert_eq!(runs(), 1, "the second press started a second sign-in");
    assert!(state(&rig).await.signing_in, "the first is still waiting");
}
