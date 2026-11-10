//! The claim: **a tool call's own `description` is on its row**, so a Bridge can draw the phrase the
//! agent wrote instead of the raw command, and a call that wrote none carries none.
//!
//! Driven by stream lines sent as the reader sends them, and read off the thread.

use adapter_traits::AgentHarness;
use adapters::HeadlessAgent;
use ipc::{SessionId, SessionRow};

use super::session_host::{eventually, rig_placing, Rig};
use super::session_plan::running;
use crate::session_host::Heard;

const DESCRIBED: &str = r#"{"type":"assistant","message":{"content":[{"type":"tool_use","id":"toolu_60","name":"Bash","input":{"command":"git show --stat HEAD","description":"  Showing files touched by the latest non-merge commit  "}}]}}"#;
const BARE: &str = r#"{"type":"assistant","message":{"content":[{"type":"tool_use","id":"toolu_61","name":"Bash","input":{"command":"ls","description":"   "}}]}}"#;

/// The thread's `Bash` rows as `(text, description)`.
async fn bash_rows(rig: &Rig, id: &SessionId) -> Vec<(String, Option<String>)> {
    rig.rows(id)
        .await
        .into_iter()
        .filter_map(|row| match row {
            SessionRow::Tool { text, description, .. } if text.starts_with("Bash") => Some((text, description)),
            _ => None,
        })
        .collect()
}

#[tokio::test]
async fn a_bash_call_with_a_description_carries_it_trimmed_beside_its_command() {
    let rig = rig_placing(None);
    let id = running(&rig).await;
    rig.stand_in.hears(0, Heard::Events(HeadlessAgent::at("agent").read(DESCRIBED)));
    eventually(|| async { bash_rows(&rig, &id).await.len() == 1 }).await;

    let rows = bash_rows(&rig, &id).await;
    let want = Some("Showing files touched by the latest non-merge commit".to_string());
    assert_eq!(rows, [("Bash git show --stat HEAD".to_string(), want)]);
}

#[tokio::test]
async fn a_bash_call_with_no_description_carries_none() {
    let rig = rig_placing(None);
    let id = running(&rig).await;
    rig.stand_in.hears(0, Heard::Events(HeadlessAgent::at("agent").read(BARE)));
    eventually(|| async { bash_rows(&rig, &id).await.len() == 1 }).await;

    let rows = bash_rows(&rig, &id).await;
    assert_eq!(rows, [("Bash ls".to_string(), None)], "a blank description is none");
}
