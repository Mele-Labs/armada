//! A Job's shell that ended and was replaced, by an exit or by a Fleet restart: the agent's next
//! typing opens the new one and types nothing until the agent has read it.

use std::sync::Arc;

use api::{Refusal, Sessions};
use ipc::ReadTerminal;
use testkit::FakeWorkProduct;

use crate::tests::daemon::a_fleet;
use crate::tests::session_running::{rig, Rig};
use crate::tests::session_terminal::{ends, typing};

fn reading(rig: &Rig) -> ReadTerminal {
    ReadTerminal { session_id: Some(rig.session.clone()), ..Default::default() }
}

/// The refusal's words.
fn said(refused: Result<ipc::TerminalTyped, Refusal>) -> String {
    match refused {
        Err(Refusal::Unacceptable(error)) => error.message.clone(),
        other => panic!("refused as unacceptable: {other:?}"),
    }
}

#[tokio::test]
async fn typing_into_a_shell_that_replaced_an_ended_one_is_refused_until_it_is_read() {
    let rig = rig().await;
    let first = Arc::clone(&rig.fleet)
        .type_in_terminal(None, typing(&rig, "echo one", true, Some(5)))
        .await
        .expect("typed")
        .run_id;
    ends(&rig, &first).await;

    let refused = said(Arc::clone(&rig.fleet).type_in_terminal(None, typing(&rig, "echo two-$((1 + 1))", true, Some(5))).await);
    let second = Arc::clone(&rig.fleet).open_job_shell(rig.session.as_str(), None, None).await.unwrap();
    assert_ne!(second, first);
    assert!(refused.contains(&first) && refused.contains(&second), "{refused}");
    assert!(refused.starts_with("nothing was typed"), "{refused}");

    let read = rig.fleet.read_terminal(None, reading(&rig)).await.expect("read");
    assert_eq!(read.run_id, second);
    let typed = Arc::clone(&rig.fleet).type_in_terminal(None, typing(&rig, "echo two-$((1 + 1))", true, Some(5))).await;
    let held = rig.observes(&second).await.history;
    ends(&rig, &second).await;
    let typed = typed.expect("typed once read");
    assert!(typed.printed.lines().any(|line| line.trim() == "two-2"), "{typed:?}");
    assert_eq!(held.matches("two-2").count(), 1, "the refused typing wrote nothing: {held:?}");
}

#[tokio::test]
async fn a_fleet_restart_neither_reuses_a_shell_id_nor_forgets_the_agent_had_a_shell() {
    let rig = rig().await;
    let before = Arc::clone(&rig.fleet)
        .type_in_terminal(None, typing(&rig, "echo before", true, Some(5)))
        .await
        .expect("typed")
        .run_id;
    ends(&rig, &before).await;

    let restarted = Arc::new(a_fleet(&rig.home, FakeWorkProduct::changed(&[])));
    let refused = Arc::clone(&restarted).type_in_terminal(None, typing(&rig, "puts 1", true, Some(5))).await;
    let after = Arc::clone(&restarted).open_job_shell(rig.session.as_str(), None, None).await.expect("opened");
    restarted.stop_session_run(ipc::StopSessionRun { run_id: after.clone() }).await.expect("in flight");
    assert_ne!(after, before, "a run id is never taken twice");
    let refused = said(refused);
    assert!(refused.contains(&before) && refused.contains(&after), "{refused}");
}
