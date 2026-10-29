//! Fleet's offer to refresh the preview, on the Needs you list of the Job holding a branch the
//! preview left out once that branch moves. **Fleet never starts the restart**: the owner presses
//! Refresh preview, or Skip.
//!
//! **Kept once a (Job, branch, tip, build)** in the store, so a later sweep or a Fleet restart raises
//! nothing again; Skip is the dismissed-ids list's and tells the agent nothing. A build change drops
//! the old build's offers, refreshed or not.

use std::collections::BTreeSet;
use std::time::SystemTime;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use ipc::{BuildSource, Instant, WaitingAct, WaitingActKind, WaitingItem, WaitingSource};
use store::KeptPreviewOffer;

use crate::building::{commit_in, left_out_moved, restarting_in, source_in};
use crate::daemon::Fleet;

/// What Refresh preview runs: the Fleet panel's own switch, without adopt.
const REFRESH: &str = "scripts/preview --restart";

/// The newest offer of each branch the person has not skipped, as items. An older tip of a branch
/// is not shown beside a newer one: the newer refresh takes both.
pub(crate) fn offer_items(offers: &[KeptPreviewOffer], dismissed: &[String]) -> Vec<WaitingItem> {
    let newest = offers.iter().enumerate().filter(|(n, one)| offers.get(n + 1).is_none_or(|next| next.branch != one.branch));
    newest.map(|(_, one)| offer_item(one)).filter(|item| !dismissed.contains(&item.id)).collect()
}

fn offer_item(offer: &KeptPreviewOffer) -> WaitingItem {
    WaitingItem {
        id: format!("preview:{}@{}", offer.branch, offer.tip),
        text: offer.text.clone(),
        since: Instant::carried(&offer.raised_at),
        source: WaitingSource::PreviewOffer,
        act: Some(WaitingAct { kind: WaitingActKind::Run, target: REFRESH.to_string() }),
        options: Vec::new(),
        steps: Vec::new(),
        place: None,
        concern: None,
        cites: Vec::new(),
        confirmed: false,
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
    /// Offer a refresh to each live Job holding a branch the preview left out that has moved, and drop
    /// the offers of a build no longer running. On the sweep; **nothing here fails the turn**.
    pub(crate) async fn preview_offered_when_left_out_moved(&self) {
        let support = crate::runtime::support_dir(&self.host().home);
        let now = SystemTime::now();
        // Mid-restart the commit on file is not yet the build, so nothing is dropped or offered.
        if restarting_in(&support, now) {
            return;
        }
        let running = matches!(source_in(&support), Ok(BuildSource::Preview)).then(|| commit_in(&support)).flatten();
        let dropped = self.store().lock().await.drop_preview_offers_not_for(running.as_deref()).unwrap_or_default();
        for session in dropped {
            let _ = self.published_hosted(&session).await;
        }
        let Some(checkout) = self.build_checkout() else {
            return;
        };
        let root = checkout.to_string_lossy().into_owned();
        let moved = {
            let (support, root) = (support.clone(), root.clone());
            tokio::task::spawn_blocking(move || left_out_moved(&support, now, |branch| adapters::branch_tip(&root, branch)))
                .await
                .unwrap_or_default()
        };
        if moved.is_empty() {
            return;
        }
        let Some(served) = self.repositories().served().into_iter().find(|one| one.root() == root) else {
            return;
        };
        let at = self.now().as_str().to_string();
        let mut raised = BTreeSet::new();
        {
            let mut store = self.store().lock().await;
            // A moved branch nobody live holds has no Job to ask; the Fleet panel marks it moved.
            let held = crate::build_notices::branches_held(&store, &served);
            for one in &moved {
                for (session, _) in held.iter().filter(|(_, branches)| branches.contains(&one.branch)) {
                    let offer = KeptPreviewOffer {
                        session_id: session.clone(),
                        branch: one.branch.clone(),
                        tip: one.tip.clone(),
                        build: one.build.clone(),
                        text: format!("{} moved since the preview left it out: {}", one.branch, one.why),
                        raised_at: at.clone(),
                    };
                    if store.keep_preview_offer(&offer).unwrap_or(false) {
                        raised.insert(session.clone());
                    }
                }
            }
        }
        for session in raised {
            let _ = self.published_hosted(&session).await;
        }
    }
}
