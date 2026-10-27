//! Promote: a mod put into a repository by a Job of its own, as Commit to repository puts a piece
//! (`crate::committing`, the owner's 21 Oct 2026 decision): the Job's own slot and branch, the
//! mod's files committed alone under `packages/mods/<name>/`, the branch pushed, a pull request
//! opened for it, and the Job told to see that pull request merged. Once merged, the mod ships
//! built in (`super::built_in`).
//!
//! **Every refusal is decided before the Job is made**, so a refused promote leaves no Job behind,
//! and the owner's checkout is read and never written. **A pull request that does not open loses
//! nothing**: the Job keeps the commit on its branch and the answer says why. The Job is then told
//! nothing, since it has no pull request to watch.
//!
//! **The Job keeps which mod it promoted**, as a `mod` row on its ledger beside its pull request, so
//! `list_mods` carries that pull request on the mod's row while it is open and a second promote is
//! refused, however often Bridge is reloaded.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Review, Vcs, WorkProduct};
use api::Refusal;
use ipc::{ModPromoted, ModPullRequest, PromoteMod, SessionId, WireError};
use store::{AttachmentState, Holder, KeptAttachment};

use super::serving::{Named, MOD_NOT_PROMOTABLE};
use super::{examine, Files, Payload, MANIFEST};
use crate::daemon::Fleet;

/// Where a promoted mod lives in the repository.
const PLACE: &str = "packages/mods";
/// The ledger row a promote's Job keeps: its target is the mod's name, its `title` what the promote titled.
const MOD: &str = "mod";

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
    pub(super) async fn mod_promoted(self: &Arc<Self>, promote: PromoteMod) -> Result<ModPromoted, Refusal> {
        let name = promote.name;
        if let Named::BuiltIn(_) = self.mod_named(&name)? {
            return Err(self.cannot_promote(&name, format!("`{name}` is built in, so it is in the repository already")));
        }
        if let Some(open) = self.mods_in_review().await.remove(&name) {
            return Err(self.cannot_promote(&name, format!("`{name}` is in a pull request that is still open, {}", open.address)));
        }
        let dir = self.mods_dir();
        let asked = name.clone();
        let found = tokio::task::spawn_blocking(move || examine(&dir, &asked))
            .await
            .map_err(|why| self.mod_fault(format!("the mod could not be read: {why}")))?;
        let Some(files) = found.files else {
            return Err(self.cannot_promote(
                &name,
                format!("`{name}` is not valid: {}", found.problems.join("; ")),
            ));
        };
        let served = self.served_named(promote.manifest_id.as_ref())?;
        let root = Path::new(served.root());
        if !root.join("packages").is_dir() {
            return Err(self.cannot_promote(
                &name,
                String::from("this repository has no packages/ folder to put a mod in"),
            ));
        }
        let (written, title) = written(&name, &files);
        let held = |(at, text): &(String, String)| std::fs::read_to_string(root.join(at)).is_ok_and(|on| &on == text);
        if written.iter().all(held) {
            return Err(self.cannot_promote(&name, String::from("this repository already holds the mod exactly as it is")));
        }
        let (session_id, tree) = match self.job_of_its_own(&served, title.clone()).await? {
            Ok(job) => job,
            Err(why) => return Err(self.cannot_promote(&name, format!("no Job could be made for this promote. {why}"))),
        };
        let paths: Vec<&str> = written.iter().map(|(at, _)| at.as_str()).collect();
        let commit = self.committed_in(&tree, &written, &title).await.map_err(|why| self.mod_fault(why))?.ok_or_else(|| {
            self.mod_fault(format!("git found nothing to commit for {} on {}", paths.join(", "), tree.branch))
        })?;
        let body = format!("{title}.\n\nMade by Promote in Mods, as a Job of its own: {}.", paths.join(", "));
        let (pull_request, pull_request_not_opened) = match self.delivered_for_review(&served, &tree, Review::assembled(title.clone(), body)).await {
            Ok(url) => {
                self.promote_kept(&served, session_id.as_str(), &name, &title).await;
                self.pull_attached(&served, session_id.as_str(), &tree.branch, &url).await;
                let base = served.manifest().base().unwrap_or("main");
                // A refused word leaves the pull request on the Job's row, where Fleet's notices still reach it.
                let _ = self.told_by_fleet(session_id.as_str(), &brief(&title, &url, base), ipc::Notice::PrHanded).await;
                (Some(url), None)
            }
            Err(why) => (None, Some(why)),
        };
        // Every window's list carries the pull request now, not at the next rescan.
        let _ = self.mods_announced().await;
        Ok(ModPromoted { name, session_id, title, branch: tree.branch, commit, pull_request, pull_request_not_opened })
    }

    /// The `mod` row on the Job's ledger. **Held, never raised**: the pull request is open either way.
    async fn promote_kept(&self, served: &crate::repositories::Served, id: &str, name: &str, title: &str) {
        let now = self.now().as_str().to_string();
        let kept = KeptAttachment {
            holder: Holder::session(id),
            kind: MOD.into(),
            manifest_id: served.manifest().id().as_str().to_string(),
            target: name.to_string(),
            state: AttachmentState::Standing,
            detail: [("title".to_string(), title.to_string())].into(),
            since: now.clone(),
            changed_at: now,
        };
        let _ = self.store().lock().await.attach(&kept, false);
    }

    /// Each mod a promote's Job holds an open pull request for, by name. A ledger that will not read
    /// leaves a mod out, since nothing here may fail the list.
    pub(super) async fn mods_in_review(&self) -> BTreeMap<String, ModPullRequest> {
        let served = self.repositories().served();
        let store = self.store().lock().await;
        let mut open = BTreeMap::new();
        for one in &served {
            let manifest = one.manifest().id().as_str();
            for promote in store.standing_of_kind(MOD, manifest).unwrap_or_default() {
                let held = store.attachments_of(&promote.holder).unwrap_or_default();
                let Some(pull) = held.iter().find(|row| {
                    row.kind == crate::pull_requesting::PR
                        && row.manifest_id == manifest
                        && row.state == AttachmentState::Standing
                        && matches!(row.detail.get("state").map(String::as_str), Some("open" | "draft"))
                }) else {
                    continue;
                };
                let (Ok(number), Some(address)) = (pull.target.parse::<u64>(), pull.detail.get("address")) else { continue };
                open.entry(promote.target.clone()).or_insert_with(|| ModPullRequest {
                    address: address.clone(),
                    number,
                    title: promote.detail.get("title").cloned().unwrap_or_default(),
                    session_id: SessionId::carried(promote.holder.id.as_str()),
                });
            }
        }
        open
    }

    fn cannot_promote(&self, name: &str, said: String) -> Refusal {
        Refusal::IllegalMove(
            WireError::raised(MOD_NOT_PROMOTABLE, said, self.run_id())
                .with_field("name", ipc::WireValue::Str(name.to_string())),
        )
    }
}

/// The mod's two files under `packages/mods/<name>/`, **from the bytes that were checked**, not
/// read again, and the title its commit, Job and pull request carry.
fn written(name: &str, files: &Files) -> (Vec<(String, String)>, String) {
    let kind = match files.payload {
        Payload::Css(_) => "theme",
        Payload::Layout(_) => "layout",
        Payload::Words(_) => "words",
        Payload::Fleet(_) => "fleet",
        Payload::Tweak(_) => "tweak",
        Payload::Tracker(_) => "tracker",
        Payload::Commands(_) => "commands",
    };
    let written = vec![
        (format!("{PLACE}/{name}/{MANIFEST}"), files.manifest.clone()),
        (format!("{PLACE}/{name}/{}", files.payload.file()), files.payload.text().to_string()),
    ];
    (written, format!("Add the {name} {kind} mod"))
}

/// What the Job is told once its pull request is open: what it holds, what Fleet tells it, and
/// when it is done.
fn brief(title: &str, url: &str, base: &str) -> String {
    format!(
        "This Job holds {url}, the pull request titled \"{title}\". Fleet tells you when its ci fails, when it \
         conflicts with {base}, and when it merges. Fix what goes red, keep the branch current with {base}, and \
         finish when it merges."
    )
}
