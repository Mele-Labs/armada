//! What Fleet does when a Session something waits on moves: read the condition against what Fleet
//! observes of it, and start the Session that waited when it holds. `docs/concepts/session.md`, *A
//! Session that waits on another*.
//!
//! **Nothing here trusts the agent.** `ends` is the Session's state, a plan part is the plan's progress
//! as Fleet reads it off the agent's task list and the edits it saw, and a pull request is the ledger's
//! `pr` attachment: pushed when it is held, landed when it is spent or its state says merged. A Session
//! waiting when Fleet starts is read at once, since its condition may have been met while Fleet was down.
//!
//! **The first pull request the Session waited on opens is the wait's** and is kept on it, so a second
//! one never moves a wait that has named #N. A pull request that merged before the wait was made is
//! not the one it waits for.

use std::collections::HashSet;
use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use ipc::{AttachmentState, Event, SessionRecord, WaitPull};
use store::KeptWait;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use crate::daemon::Fleet;

/// What a wait reads against the Session it waits on.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Verdict {
    /// Not yet. `pull` is the pull request it is on, where one has been seen.
    Waiting { pull: Option<WaitPull> },
    /// The condition holds. `said` is why, in a line the new Session's first message carries.
    Met { said: String, pull: Option<WaitPull> },
}

/// Whether the ledger says this pull request merged.
fn merged(pull: &ipc::Attachment) -> bool {
    match pull.detail.get("state").map(String::as_str) {
        Some("merged") => true,
        Some("open" | "draft" | "closed") => false,
        _ => pull.state == AttachmentState::Spent,
    }
}

/// The pull request the wait is on: the one it named, else the newest open one, or one that merged since
/// the wait was made.
fn pull_on<'a>(wait: &KeptWait, on: &'a SessionRecord) -> Option<&'a ipc::Attachment> {
    let held = on.attachments.iter().filter(|one| one.kind == "pr" && one.state != AttachmentState::GivenBack);
    match wait.pull_number.as_deref() {
        Some(number) => held.into_iter().find(|one| one.target == number),
        None => held
            .filter(|one| !(merged(one) && one.changed_at.as_str() <= wait.created_at.as_str()))
            .max_by(|a, b| a.since.cmp(&b.since)),
    }
}

fn pull_wire(pull: &ipc::Attachment) -> Option<WaitPull> {
    Some(WaitPull { number: pull.target.parse().ok()?, branch: pull.detail.get("branch").cloned().unwrap_or_default() })
}

/// A wait read against the Session it waits on, as Fleet now observes it. Pure.
pub(crate) fn judged(wait: &KeptWait, on: &SessionRecord) -> Verdict {
    let title = on.title.clone().unwrap_or_else(|| on.id.as_str().to_string());
    let pull = pull_on(wait, on);
    let seen = pull.and_then(pull_wire);
    match wait.condition.as_str() {
        "group_done" | "task_done" => match on.plan.as_ref().filter(|plan| crate::session_host::after::plan_part_done(plan, wait)) {
            Some(plan) => Verdict::Met { said: format!("{} is done in {title}", crate::session_host::after::plan_part_title(plan, &wait.part).unwrap_or_else(|| wait.part.clone())), pull: seen },
            None => Verdict::Waiting { pull: seen },
        },
        "pr_pushed" => match &seen {
            Some(one) => Verdict::Met { said: format!("pull request #{} is pushed", one.number), pull: seen.clone() },
            None => Verdict::Waiting { pull: None },
        },
        "pr_lands" => match (pull, &seen) {
            (Some(pull), Some(one)) if merged(pull) => Verdict::Met { said: format!("pull request #{} landed", one.number), pull: seen.clone() },
            _ => Verdict::Waiting { pull: seen },
        },
        _ if on.state == ipc::SessionState::Ended => Verdict::Met { said: format!("{title} ended"), pull: seen },
        _ => Verdict::Waiting { pull: seen },
    }
}

/// Start reading the Sessions that others wait on. Production starts it beside the rail's, and a test
/// starts it itself.
pub fn keep_the_waiting<H, V, W>(fleet: Arc<Fleet<H, V, W>>) -> JoinHandle<()>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    // Which Session moved, and the Session it waits on where it is itself a waiting one.
    let (tell, mut heard) = mpsc::unbounded_channel::<(String, Option<String>)>();
    fleet.events().tap(move |event| {
        if let Event::SessionChanged(record) = event {
            let _ = tell.send((record.id.as_str().to_string(), record.waits.as_ref().map(|wait| wait.on.id.as_str().to_string())));
        }
    });
    tokio::spawn(async move {
        let mut watched: HashSet<String> = HashSet::new();
        let open = fleet.store().lock().await.open_waits().unwrap_or_default();
        for wait in open {
            fleet.waiter_told_of_wait(&wait.session_id).await;
            if watched.insert(wait.on_session.clone()) {
                fleet.waits_judged(&wait.on_session).await;
            }
        }
        while let Some((moved, waits_on)) = heard.recv().await {
            if let Some(on) = waits_on {
                fleet.waiter_told_of_wait(&moved).await;
                watched.insert(on.clone());
                fleet.waits_judged(&on).await;
            } else if watched.contains(&moved) && fleet.waits_judged(&moved).await == 0 {
                watched.remove(&moved);
            }
        }
    })
}

impl<H, V, W> Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    /// Read every wait on `on` against it, and start the Sessions whose condition holds. Answers how many
    /// still wait.
    pub(crate) async fn waits_judged(self: &Arc<Self>, on: &str) -> usize {
        let (waits, record, live) = {
            let store = self.store().lock().await;
            let waits = store.waits_on(on).unwrap_or_default();
            if waits.is_empty() {
                return 0;
            }
            let Some(session) = store.session(on).ok().flatten() else { return 0 };
            let Ok(record) = self.ledger_row(&store, &session) else { return 0 };
            let live: Vec<bool> = waits
                .iter()
                .map(|wait| store.session(&wait.session_id).ok().flatten().is_some_and(|one| one.state == store::SessionState::Live))
                .collect();
            (waits, record, live)
        };
        let mut remaining = 0;
        for (mut wait, live) in waits.into_iter().zip(live) {
            // A Session closed while it waited never starts.
            if !live {
                continue;
            }
            let (met, pull) = match judged(&wait, &record) {
                Verdict::Met { said, pull } => (Some(said), pull),
                Verdict::Waiting { pull } => (None, pull),
            };
            if let Some(one) = pull.filter(|one| wait.pull_number.as_deref() != Some(one.number.to_string().as_str())) {
                wait.pull_number = Some(one.number.to_string());
                wait.pull_branch = Some(one.branch);
                let _ = self.store().lock().await.keep_wait(&wait);
                if met.is_none() {
                    let _ = self.published_hosted(&wait.session_id).await;
                }
            }
            match met {
                Some(said) => self.wait_met(wait, said).await,
                None => remaining += 1,
            }
        }
        remaining
    }
}
