//! The web links a Session's person and agent write, as `link` rows on the ledger, so Bridge can
//! list them under the Session's artifacts. `docs/concepts/session/what-is-kept.md`.
//!
//! **Spent, not standing**: the Session ending gives back what it holds, and a link it was given
//! or wrote is a fact that stays, as a fork's link is. **One row a Session and address, and the
//! voice that wrote it first stands**: a second naming would otherwise change the row's `by`.

use std::collections::BTreeMap;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use store::{AttachmentState, Holder, KeptAttachment};

use crate::daemon::Fleet;

/// The ledger's kind for a web link. `target` is the address.
pub(crate) const LINK: &str = "link";

/// Whose words a link was written in.
#[derive(Clone, Copy)]
pub(crate) enum LinkedBy {
    You,
    Agent,
}

impl LinkedBy {
    fn as_text(self) -> &'static str {
        match self {
            LinkedBy::You => "you",
            LinkedBy::Agent => "agent",
        }
    }
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
    /// The web links in `text`, held by `session` as written by `by`.
    ///
    /// **Held, never raised**, as `issues_named` is: the words are already said, and a row that
    /// would not write is no reason to refuse them.
    pub(crate) async fn links_seen(&self, session: &str, text: &str, by: LinkedBy) {
        let Some(kept) = self.store().lock().await.session(session).ok().flatten() else {
            return;
        };
        let manifest = kept.manifest_id.clone().unwrap_or_default();
        let repository = kept.manifest_id.as_deref().and_then(|manifest| self.own_repository(manifest));
        let links = adapters::links_in(text, repository.as_ref());
        if links.is_empty() {
            return;
        }
        let now = self.clock().now().as_str().to_string();
        let holder = Holder::session(session);
        let record = {
            let mut store = self.store().lock().await;
            let mut changed = false;
            for link in links {
                let held = store
                    .attachments_at(LINK, &link, Some(&manifest))
                    .map(|rows| rows.iter().any(|row| row.holder == holder))
                    .unwrap_or(true);
                if held {
                    continue;
                }
                changed |= store
                    .attach(
                        &KeptAttachment {
                            holder: holder.clone(),
                            kind: LINK.into(),
                            manifest_id: manifest.clone(),
                            target: link,
                            state: AttachmentState::Spent,
                            detail: BTreeMap::from([("by".to_string(), by.as_text().to_string())]),
                            since: now.clone(),
                            changed_at: now.clone(),
                        },
                        false,
                    )
                    .unwrap_or(false);
            }
            if !changed {
                return;
            }
            self.ledger_row(&store, &kept).ok()
        };
        if let Some(record) = record {
            self.publish(ipc::Event::SessionChanged(record));
        }
    }
}
