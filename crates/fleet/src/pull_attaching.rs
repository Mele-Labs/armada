//! A pull request nobody reported is attached to the Session that worked its branch. The pull
//! watch reads every open pull request with its head branch, and the ledger already says which
//! Session held that branch: a hosted Session's lease writes the row, and the mod writes it for
//! a terminal one as its directory moves. `docs/concepts/session.md`, *Acts on a pull request*.
//!
//! **Why Fleet and not the mod alone**: the mod attaches a pull request only when it sees a
//! `gh pr create` URL in its own Bash output, so one made by a compound command, or from another
//! checkout, was never attached.

use adapter_traits::{AgentHarness, Delivery, Vcs, WatchedPull, WorkProduct};
use store::{AttachmentState, Holder, HolderKind, KeptAttachment};

use crate::daemon::Fleet;
use crate::pull_requesting::PR;
use crate::repositories::Served;

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
    /// Attach each watched pull request no Session holds to the Session that held its branch.
    /// **Never a second Session, and never a pull request a Session already holds**, in any
    /// state: a merged one a Session took again is the pull watch's to settle, not this.
    /// The standing branch holder is preferred, then the one that changed it last.
    pub(crate) async fn pulls_attached(&self, served: &Served, watched: &[WatchedPull]) {
        let manifest = served.manifest().id().as_str().to_string();
        let now = self.now().as_str().to_string();
        let mut changed = Vec::new();
        {
            let mut store = self.store().lock().await;
            for pull in watched {
                let number = pull.number.to_string();
                let held = |kind: &str, target: &str| {
                    store
                        .attachments_at(kind, target, Some(&manifest))
                        .unwrap_or_default()
                        .into_iter()
                        .filter(|row| row.holder.kind == HolderKind::Session)
                        .collect::<Vec<_>>()
                };
                if !held(PR, &number).is_empty() {
                    continue;
                }
                let branch = pull.branch.as_written();
                let Some(owner) = held("branch", branch).into_iter().next() else {
                    continue;
                };
                let row = KeptAttachment {
                    holder: Holder::session(&owner.holder.id),
                    kind: PR.into(),
                    manifest_id: manifest.clone(),
                    target: number,
                    state: AttachmentState::Standing,
                    detail: [
                        ("state".to_string(), "open".to_string()),
                        ("branch".to_string(), branch.to_string()),
                        ("address".to_string(), pull.url.as_written().to_string()),
                    ]
                    .into(),
                    since: now.clone(),
                    changed_at: now.clone(),
                };
                if !matches!(store.attach(&row, false), Ok(true)) {
                    continue;
                }
                let Some(session) = store.session(&owner.holder.id).ok().flatten() else {
                    continue;
                };
                if let Ok(record) = self.ledger_row(&store, &session) {
                    changed.push(record);
                }
            }
        }
        for record in changed {
            self.publish(ipc::Event::SessionChanged(record));
        }
    }
}
