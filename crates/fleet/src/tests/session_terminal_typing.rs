//! What the agent's typing does to a line that already holds text, and what it is told when the
//! shell never showed its words. A real Fleet, a real shell and a real pseudo-terminal.

use std::sync::Arc;

use api::Sessions;

use crate::tests::session_running::{rig, Rig};
use crate::tests::session_terminal::{ends, shortly, typing};

/// A terminal's reply to a colour query, as it lands on the input line.
const LEFT_ON_THE_LINE: &str = "11;rgb:1313/1111/1a1a";

/// The Job's shell, once it has run a command: only then is what is typed read by its line editor.
async fn ready(rig: &Rig) -> String {
    let shell = Arc::clone(&rig.fleet).open_job_shell(rig.session.as_str(), None, None).await.unwrap();
    rig.types(&shell, "echo $((40 + 2))\r").await.expect("in flight");
    if !shortly(|| async { rig.observes(&shell).await.history.contains("42") }).await {
        ends(rig, &shell).await;
        panic!("the shell never ran a command");
    }
    shell
}

#[tokio::test]
async fn text_left_on_the_line_is_cleared_before_the_agents_command_runs() {
    let rig = rig().await;
    let shell = ready(&rig).await;
    rig.types(&shell, LEFT_ON_THE_LINE).await.expect("in flight");

    let typed = Arc::clone(&rig.fleet)
        .type_in_terminal(None, typing(&rig, "echo cleared", true, Some(5)))
        .await
        .expect("typed");
    ends(&rig, &shell).await;
    assert!(typed.printed.lines().any(|line| line.trim() == "cleared"), "{:?}", typed.printed);
}

#[tokio::test]
async fn clear_false_types_after_what_is_on_the_line() {
    let rig = rig().await;
    let shell = ready(&rig).await;
    rig.types(&shell, LEFT_ON_THE_LINE).await.expect("in flight");

    let kept = ipc::TypeInTerminal { clear: false, ..typing(&rig, "echo cleared", true, Some(5)) };
    let typed = Arc::clone(&rig.fleet).type_in_terminal(None, kept).await.expect("typed");
    ends(&rig, &shell).await;
    assert!(typed.printed.contains("not found"), "what was left ran with it: {:?}", typed.printed);
    assert!(!typed.printed.lines().any(|line| line.trim() == "cleared"), "{:?}", typed.printed);
}

#[tokio::test]
async fn text_typed_at_a_silent_prompt_answers_echoed_false() {
    let rig = rig().await;
    let asking = Arc::clone(&rig.fleet)
        .type_in_terminal(None, typing(&rig, "read -s answer; echo got-${#answer}", true, Some(5)))
        .await
        .expect("typed");
    assert!(asking.echoed, "{asking:?}");

    let hidden = Arc::clone(&rig.fleet)
        .type_in_terminal(None, typing(&rig, "hunter22", true, Some(5)))
        .await
        .expect("typed");
    let held = rig.observes(&hidden.run_id).await.history;
    ends(&rig, &hidden.run_id).await;
    assert!(hidden.printed.contains("got-8"), "the read took it: {:?}\nheld: {held:?}", hidden.printed);
    assert!(!hidden.echoed, "{hidden:?}");
}

#[tokio::test]
async fn a_long_line_the_shell_wraps_still_reads_as_echoed() {
    let rig = rig().await;
    let long = format!("echo {}", "wrapped-".repeat(20));
    let typed = Arc::clone(&rig.fleet).type_in_terminal(None, typing(&rig, &long, true, Some(5))).await.expect("typed");
    ends(&rig, &typed.run_id).await;
    assert!(typed.echoed, "{typed:?}");
}

#[tokio::test]
async fn ctrl_c_is_typed_bare_and_counts_as_echoed() {
    let rig = rig().await;
    let serving = Arc::clone(&rig.fleet)
        .type_in_terminal(None, typing(&rig, "sleep 30", true, Some(2)))
        .await
        .expect("typed");
    let stopped = Arc::clone(&rig.fleet)
        .type_in_terminal(None, typing(&rig, "\u{3}", false, Some(3)))
        .await
        .expect("typed");
    let back = Arc::clone(&rig.fleet)
        .type_in_terminal(None, typing(&rig, "echo back", true, Some(5)))
        .await
        .expect("typed");
    ends(&rig, &serving.run_id).await;
    assert!(stopped.echoed, "{stopped:?}");
    assert!(back.printed.lines().any(|line| line.trim() == "back"), "sleep was interrupted: {back:?}");
}
