//! The tap that finds what happened to a Job between two looks and hands it to the Job's rules.
//! **No loop of its own**: it hears the row changes the rail's and the lifecycles' taps hear, and
//! does its work only for a Job whose look changed.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use ipc::{Event, SessionRecord, SessionState};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use super::taking_on::{landed, opened_pull_request};
use crate::daemon::Fleet;

/// What of a row decides whether anything a rule can see moved.
type Look = (Option<u32>, bool, bool, bool);

fn look_of(record: &SessionRecord) -> Look {
    (record.workflow_step, opened_pull_request(&record.attachments), landed(&record.attachments), record.state == SessionState::Ended)
}

/// Start finding what happens to Jobs that have rules. Production starts it beside the rail's and the
/// lifecycles', and a test calls [`Fleet::rules_looked_at`] itself.
pub fn keep_the_rules<H, V, W>(fleet: Arc<Fleet<H, V, W>>) -> JoinHandle<()>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    let (tell, mut heard) = mpsc::unbounded_channel::<SessionRecord>();
    let last: Mutex<HashMap<String, Look>> = Mutex::new(HashMap::new());
    fleet.events().tap(move |event| {
        let Event::SessionChanged(record) = event else { return };
        if record.manifest_id.is_none() {
            return;
        }
        let look = look_of(record);
        let changed = last.lock().map(|mut held| held.insert(record.id.as_str().to_string(), look) != Some(look)).unwrap_or(true);
        if changed {
            let _ = tell.send(record.clone());
        }
    });
    let doing = Arc::clone(&fleet);
    tokio::spawn(async move {
        // What a Fleet that stopped owes: moves its rules did not hear, and acts they planned and nobody did.
        doing.rules_caught_up().await;
        doing.rule_acts_carried_out().await;
        loop {
            doing.rule_acts().woken().await;
            doing.rule_acts_carried_out().await;
        }
    });
    tokio::spawn(async move {
        while let Some(record) = heard.recv().await {
            fleet.rules_looked_at(&record).await;
        }
    })
}
