//! What Fleet does when a Session another Session's agent started needs the owner, ends, or has a
//! pull request merged: it tells the one that started it. `docs/concepts/session.md`, *Linked Sessions*.
//!
//! **The parent is told in two places at once.** Its agent is sent a message that wakes it, and its
//! thread keeps a `linked` row where it happened, which Bridge draws as the press that opens the
//! child. A notice is news that asks the agent for nothing, so it leaves the agent's `all_done`
//! standing. **Each is told once**: `done` and `merged` are kept in the store, so a restart does not
//! tell them again; `needs` is a move from not needing to needing, read against what this loop last
//! saw, and a Session already needing when Fleet starts is not told.
//!
//! Every Session with a `started_by` row is followed, whether or not it was handed a plan group.

use std::collections::HashMap;
use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use ipc::{Event, LinkedEvent, SessionLink, SessionRecord, SessionRow, SessionStanding, WaitingSource};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use crate::daemon::Fleet;

/// The parent of a Session, from its `started_by` row, and the group it was handed.
fn followed_by(record: &SessionRecord) -> Option<(String, Option<String>)> {
    let link = record.attachments.iter().find(|one| one.kind == "started_by")?;
    Some((link.target.clone(), link.detail.get(crate::session_host::linking::GROUP).cloned()))
}

/// Whether the Session waits on the person for something. A first message nobody has sent, a reply
/// and a done are not that: the first is the person's own to send, and the others ask nothing.
fn needs_the_owner(record: &SessionRecord) -> bool {
    let waits = record
        .waiting_for
        .iter()
        .any(|item| !matches!(item.source, WaitingSource::FirstMessage | WaitingSource::Replied | WaitingSource::Done));
    let asked = record.hosted.as_ref().is_some_and(|hosted| hosted.asked.is_some());
    record.state == ipc::SessionState::Live && (waits || asked || record.standing == Some(SessionStanding::Stuck))
}

/// Start telling parents about the Sessions they follow. Production starts it beside the rail's, and a
/// test starts it itself.
pub fn keep_the_linked<H, V, W>(fleet: Arc<Fleet<H, V, W>>) -> JoinHandle<()>
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
    fleet.events().tap(move |event| {
        if let Event::SessionChanged(record) = event {
            if followed_by(record).is_some() {
                let _ = tell.send(record.clone());
            }
        }
    });
    tokio::spawn(async move {
        let mut needing: HashMap<String, bool> = HashMap::new();
        while let Some(record) = heard.recv().await {
            fleet.linked_moved(record, &mut needing).await;
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
    async fn linked_moved(self: &Arc<Self>, record: SessionRecord, needing: &mut HashMap<String, bool>) {
        let Some((parent, group)) = followed_by(&record) else { return };
        let child = record.id.as_str().to_string();
        let title = record.title.clone().unwrap_or_else(|| child.clone());
        let now = needs_the_owner(&record);
        let before = needing.insert(child.clone(), now);
        let said = |what: &str| {
            let group = group.as_ref().map(|one| format!(" Plan group: {one}.")).unwrap_or_default();
            format!("Linked Job \"{title}\" ({child}) {what}.{group}")
        };
        if now && before == Some(false) {
            self.linked_told(&parent, &child, &title, LinkedEvent::Needs, said("needs the owner")).await;
        }
        if record.state == ipc::SessionState::Ended && self.linked_once(&child, "done", "").await {
            self.linked_told(&parent, &child, &title, LinkedEvent::Done, said("ended")).await;
        }
        for pull in record.attachments.iter().filter(|one| one.kind == "pr" && one.state == ipc::AttachmentState::Spent) {
            if self.linked_once(&child, "merged", &pull.target).await {
                self.linked_told(&parent, &child, &title, LinkedEvent::Merged, said(&format!("had its pull request #{} merged", pull.target))).await;
            }
        }
    }

    /// Whether this is the first time `event` under `key` is seen for `child`, and keeps it.
    async fn linked_once(&self, child: &str, event: &str, key: &str) -> bool {
        let now = self.now();
        self.store().lock().await.keep_linked_told(child, event, key, &now).unwrap_or(false)
    }

    async fn linked_told(self: &Arc<Self>, parent: &str, child: &str, title: &str, event: LinkedEvent, text: String) {
        let row = SessionRow::Linked {
            id: self.row_id(parent),
            at: self.instant(),
            session: SessionLink { id: ipc::SessionId::carried(child), title: title.to_string() },
            event,
        };
        let _ = self.told_news_by_fleet_as(parent, &text, row).await;
    }
}
