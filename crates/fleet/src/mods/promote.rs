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

use std::path::Path;
use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Review, Vcs, WorkProduct};
use api::Refusal;
use ipc::{ModPromoted, PromoteMod, WireError};

use super::serving::{Named, MOD_NOT_PROMOTABLE};
use super::{examine, Files, Payload, MANIFEST};
use crate::daemon::Fleet;

/// Where a promoted mod lives in the repository.
const PLACE: &str = "packages/mods";

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
                self.pull_attached(&served, session_id.as_str(), &tree.branch, &url).await;
                let base = served.manifest().base().unwrap_or("main");
                // A refused word leaves the pull request on the Job's row, where Fleet's notices still reach it.
                let _ = self.told_by_fleet(session_id.as_str(), &brief(&title, &url, base)).await;
                (Some(url), None)
            }
            Err(why) => (None, Some(why)),
        };
        Ok(ModPromoted { name, session_id, branch: tree.branch, commit, pull_request, pull_request_not_opened })
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
        Payload::Tracker(_) => "tracker",
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
