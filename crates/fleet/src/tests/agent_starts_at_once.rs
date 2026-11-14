//! The claim: **a Job an agent starts follows the `agent_starts` word.** `at_once` begins it with the brief as its
//! first turn, from the caller; `ask_me` holds it on its start card as it always was. A Job that waits on another
//! follows the word when its wait is met. `docs/concepts/session.md`, *A Session an agent started*.

use std::sync::Arc;

use api::Sessions;
use ipc::{SessionRow, SessionVoice};

use super::agent_starts::{begins, caller, rig};
use super::session_host::eventually;

const BRIEF: &str = "Read crates/reader and say why it drops lines.";

#[tokio::test]
async fn at_once_runs_the_job_with_the_brief_as_the_callers_first_turn_and_raises_no_start_card() {
    let rig = rig(None);
    rig.agents_start("at_once");
    let agent = rig.an_agent().await;
    let processes = rig.stand_in.starts().len();

    let made = Arc::clone(&rig.fleet).start_another_session(caller(), begins(&rig, "Check the reader", BRIEF)).await.expect("started");

    assert!(!made.held, "it is not held");
    let record = rig.record(&made.session_id).await;
    assert!(record.hosted.as_ref().and_then(|hosted| hosted.start.as_ref()).is_none(), "no start card");
    assert!(record.waiting_for.iter().all(|item| !item.id.starts_with("unsent:")), "nothing waits for him: {:?}", record.waiting_for);
    eventually(|| async { rig.stand_in.starts().len() > processes }).await;
    let rows = rig.rows(&made.session_id).await;
    let first = rows.iter().find_map(|row| match row {
        SessionRow::Message { from, text, .. } => Some((from.clone(), text.clone())),
        _ => None,
    });
    let Some((SessionVoice::Session { id, .. }, text)) = first else { panic!("the brief is a Job's word: {rows:?}") };
    assert_eq!((id.as_str(), text.as_str()), (agent.as_str(), BRIEF));
    let caller_record = rig.record(&agent).await;
    assert!(caller_record.waiting_for.iter().all(|item| item.id != format!("start:{}", made.session_id.as_str())), "the caller waits on no press");
}

#[tokio::test]
async fn ask_me_holds_the_job_on_its_start_card_and_runs_nothing() {
    let rig = rig(None);
    rig.agents_start("ask_me");
    rig.an_agent().await;
    let processes = rig.stand_in.starts().len();

    let made = Arc::clone(&rig.fleet).start_another_session(caller(), begins(&rig, "Check the reader", BRIEF)).await.expect("held");

    assert!(made.held);
    assert_eq!(rig.card_message(&made.session_id).await.as_deref(), Some(BRIEF), "the brief is on the card");
    assert_eq!(rig.stand_in.starts().len(), processes, "nothing was started");
    assert!(rig.rows(&made.session_id).await.is_empty(), "no turn, no row");
}
