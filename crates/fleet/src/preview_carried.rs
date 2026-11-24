//! What a refresh of the preview carries forward that is not an offer: the branches the running build holds, at
//! the tips it holds them, and the branches he picked for a build. **A refresh starts from what he is on**, so an
//! offer arriving never drops them. One leaves only where its Job withdrew that tip, where an offer of the branch
//! is ready (its tip is merged instead), or where `scripts/preview` cannot take it, which it names: on main, a
//! conflict, or not building. `docs/concepts/fleet.md`, *Offering a branch for the preview*.

use std::collections::BTreeSet;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use adapters::preview_check;

use crate::daemon::Fleet;
use crate::kept_builds;

/// One branch a refresh carries forward.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Carried {
    pub(crate) branch: String,
    pub(crate) tip: String,
    /// Whether the running build holds it at `tip`, rather than it being picked and not yet built.
    pub(crate) held: bool,
    /// The Job whose note names this (branch, tip), and its `about`; empty where none here does.
    pub(crate) session: String,
    pub(crate) about: String,
}

/// The lines `scripts/preview` reads for `carried`, before the offers, less any branch `offered` names: an offer
/// that is ready brings its own tip.
pub(crate) fn carried_lines(carried: &[Carried], offered: &BTreeSet<String>) -> String {
    carried
        .iter()
        .filter(|one| !offered.contains(&one.branch))
        .map(|one| format!("{}\t{}\t{}\t{}\n", one.branch, one.tip, one.session, one.about.replace(['\t', '\n'], " ")))
        .collect()
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
    /// The running preview's branches in the order it merged them, then the picks it does not hold at the tip this
    /// repository or origin has, less every (branch, tip) a Job withdrew. Off the preview nothing is held.
    pub(crate) async fn preview_carried(&self) -> Vec<Carried> {
        let support = crate::runtime::support_dir(&self.host().home);
        let mut carried: Vec<Carried> = Vec::new();
        if let Some((commit, tips)) = self.preview_build() {
            let order = self.preview_held(&support, &commit).map(|held| held.branches).unwrap_or_default();
            let rest = tips.keys().filter(|branch| !order.contains(branch)).cloned().collect::<Vec<_>>();
            for branch in order.into_iter().chain(rest) {
                if let Some(tip) = tips.get(&branch) {
                    carried.push(Carried { tip: tip.clone(), branch, held: true, session: String::new(), about: String::new() });
                }
            }
        }
        let picks: Vec<String> = kept_builds::picked_in(&support).into_iter().filter(|branch| carried.iter().all(|one| one.branch != *branch)).collect();
        if let (false, Some(root)) = (picks.is_empty(), self.build_checkout()) {
            let found = tokio::task::spawn_blocking(move || {
                picks
                    .into_iter()
                    .filter_map(|branch| {
                        let tip = preview_check::commit_of(&root, &format!("refs/heads/{branch}"))
                            .or_else(|| preview_check::commit_of(&root, &format!("refs/remotes/origin/{branch}")))?;
                        Some((branch, tip))
                    })
                    .collect::<Vec<_>>()
            })
            .await
            .unwrap_or_default();
            carried.extend(found.into_iter().map(|(branch, tip)| Carried { branch, tip, held: false, session: String::new(), about: String::new() }));
        }
        let store = self.store().lock().await;
        let withdrawn = store.preview_withdrawn_all().unwrap_or_default();
        carried.retain(|one| !withdrawn.iter().any(|(branch, tip)| *branch == one.branch && *tip == one.tip));
        for one in carried.iter_mut() {
            if let Some(note) = store.build_note_of(&one.branch, &one.tip).ok().flatten() {
                one.session = note.session_id;
                one.about = note.about;
            }
        }
        carried
    }
}
