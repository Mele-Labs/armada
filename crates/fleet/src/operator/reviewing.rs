//! A pull request nobody has looked at, offered the review tools its project names. A look at a
//! repository's open pull requests that finds one it has not kept puts one item in Your turn:
//! `Run <tools> on #N?`, with Run and Not now. `docs/practices/protocol/projects.md`, *Review on a new pull request*.
//!
//! **Any author, draft or not.** The rules engine fires on a Job's events and a repository's pull
//! request has no Job, so this is Fleet's own item, performed by Fleet on the press as a red main's is.
//! **Asked once**: the store keeps each number seen, so Not now is for good and a restart asks nothing again.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use ipc::{Instant, ManifestId, ReviewRunKind, StartReviewRun, WaitingItem, WaitingOption, WaitingSource};
use store::KeptTurn;

use crate::daemon::Fleet;
use crate::repositories::Served;

/// An item of a new pull request starts with this: `review:<manifest>:<number>`.
pub(super) const REVIEW: &str = "review:";
pub(super) const RUN: &str = "Run";
pub(super) const NOT_NOW: &str = "Not now";

/// The item's id for pull request `number` of `manifest`.
fn id_of(manifest: &str, number: u64) -> String {
    format!("{REVIEW}{manifest}:{number}")
}

/// `review:<manifest>:<number>` as its parts.
pub(super) fn parts_of(id: &str) -> Option<(&str, u64)> {
    let (manifest, number) = id.strip_prefix(REVIEW)?.rsplit_once(':')?;
    Some((manifest, number.parse().ok()?))
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
    /// The tools the projects holding this repository name, each once, in the order named.
    fn review_tools_named(&self, manifest: &core_model::ManifestId) -> Vec<String> {
        let mut named: Vec<String> = Vec::new();
        for project in self.projects_held().running().filter(|project| project.holds(manifest)) {
            for id in &project.values().review_on_new_pr {
                if !named.contains(id) {
                    named.push(id.clone());
                }
            }
        }
        named
    }

    /// Of `named`, the tools the repository defines now.
    fn review_tools_defined(&self, root: &str, named: &[String]) -> Vec<String> {
        let resolved = self.review_tools_resolved(root);
        named
            .iter()
            .filter(|id| resolved.get(id.as_str()).is_some_and(|def| def.kind == config::review_tools::ToolKind::Tool))
            .cloned()
            .collect()
    }

    /// What a look left open in `served`: every number is kept, and one not kept before is offered. **Nothing is
    /// offered while the Operator is off**, as a red main is not, and the numbers are kept all the same so the
    /// switch coming on does not ask of the whole list.
    pub(crate) async fn review_offers_raised(&self, served: &Served, open: &[u64]) {
        let manifest = served.manifest().id().as_str().to_string();
        let fresh = match self.store().lock().await.review_unseen(&manifest, open, &self.now()) {
            Ok(fresh) => fresh,
            Err(why) => return self.operator_warned("the pull requests looked at were not kept", why),
        };
        self.review_offers_withdrawn(&manifest, open).await;
        if fresh.is_empty() || !self.operator_on() {
            return;
        }
        let tools = self.review_tools_defined(served.root(), &self.review_tools_named(served.manifest().id()));
        if tools.is_empty() {
            return;
        }
        for number in fresh {
            self.review_offered(&manifest, number, &tools).await;
        }
    }

    async fn review_offered(&self, manifest: &str, number: u64, tools: &[String]) {
        let item = WaitingItem {
            approves: false,
            id: id_of(manifest, number),
            text: format!("Run {} on #{number}?", tools.join(", ")),
            since: Instant::carried(self.now().as_str()),
            source: WaitingSource::OperatorSuggestion,
            act: None,
            options: [RUN, NOT_NOW].map(|label| WaitingOption { label: label.to_string(), description: None, picture: None }).into(),
            steps: Vec::new(),
            place: None,
            confirmed: false,
            holders: Vec::new(),
            creating_since: None,
            create_refused: None,
            operator_read: None,
            artifact: None,
        };
        let raising = &self.operator_seat().raising;
        let _turn = raising.turn.lock().await;
        let kept = {
            let mut store = self.store().lock().await;
            let row = KeptTurn {
                manifest_id: manifest.to_string(),
                go_ahead: true,
                body: ipc::operator_page::item_kept(&item),
                pick: Some(RUN.to_string()),
                raised_at: self.now().as_str().to_string(),
                item: item.id.clone(),
                ..KeptTurn::default()
            };
            store.add_operator_turn(&row)
        };
        match kept {
            Ok(_) => {
                self.operator_journaled(&format!("Queued: {}", item.text)).await;
                self.operator_page_changed();
            }
            Err(why) => self.operator_warned("a new pull request's item was not kept", why),
        }
    }

    /// An item of a pull request that is no longer open has left Your turn: there is nothing to review.
    async fn review_offers_withdrawn(&self, manifest: &str, open: &[u64]) {
        let present = self.store().lock().await.operator_turn_present().unwrap_or_default();
        let mut left = false;
        for row in present.iter().filter(|row| row.settled_at.is_none()) {
            if parts_of(&row.item).is_some_and(|(of, number)| of == manifest && !open.contains(&number)) {
                left |= self.turn_left(row).await;
            }
        }
        if left {
            self.operator_page_changed();
        }
    }

    /// The press of Run: one review run for each tool the repository still defines. Why nothing started, where none did.
    pub(super) async fn review_run_pressed(self: &Arc<Self>, manifest: &str, number: u64) -> Result<(), String> {
        let served = self.repositories().served().into_iter().find(|served| served.manifest().id().as_str() == manifest).ok_or("that repository is not served now")?;
        let tools = self.review_tools_defined(served.root(), &self.review_tools_named(served.manifest().id()));
        if tools.is_empty() {
            return Err("No review tool is named for this project, or the repository no longer defines one".to_string());
        }
        let mut failed = Vec::new();
        for name in tools {
            let start = StartReviewRun { number, kind: ReviewRunKind::Tool, name: name.clone(), again: false, place: None, finding: None, since: None };
            if let Err(refused) = Arc::clone(self).review_run_started(ManifestId::carried(manifest.to_string()), start).await {
                failed.push(format!("{name}: {}", refused.error().message));
            }
        }
        if failed.is_empty() { Ok(()) } else { Err(failed.join(". ")) }
    }
}
