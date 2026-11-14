//! Telling the Jobs that read a wire type when a pull request changes it, so no agent has to relay
//! it. `docs/concepts/fleet.md`, *Telling the Jobs that read a wire type*.
//!
//! **A pull request that touches `crates/ipc/src` is read from the forge's patches** (`wire_types`),
//! once it is in the merge queue and again once it has merged. Every other live hosted Job is a
//! reader when its branch changes a file under the wire's consumers and its worktree names a
//! changed type there. **Told once per (pull request, cause, recipient)**, kept in the store as
//! `pull_notices` keeps its own. A Job told at the queue is told in one line at the merge.

use std::path::PathBuf;
use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, FollowedPull, Vcs, WorkProduct};

use crate::clashing::{base_of, changed_since_base, Who};
use crate::daemon::Fleet;
use crate::pull_owners::Owner;
use crate::repositories::Served;
use crate::wire_types::{changed_types, TypeChange};

/// Where the wire is read. A branch that changes nothing here has no reason to hear of it.
const READERS: [&str; 3] = ["crates/bridge-native", "apps/pocket", "packages/protocol"];

/// How many places a type is read are named.
const PLACES: usize = 3;

/// How many pull requests' changed types are held in memory, so a reading costs the forge one ask.
const HELD: usize = 16;

const QUEUED: &str = "wire_queued";
const MERGED: &str = "wire_merged";

/// One changed type the Job's worktree names, and where.
struct Read<'a> {
    change: &'a TypeChange,
    at: Vec<String>,
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
    /// A pull request in the merge queue, told to the Jobs that read what it changes.
    pub(crate) async fn wire_told(self: &Arc<Self>, served: &Served, pull: &FollowedPull) {
        let held = format!("{}@{}", pull.number, pull.head.as_deref().unwrap_or_default());
        self.wire_told_of(served, pull.number, pull.branch.as_written(), &held, false).await;
    }

    /// A pull request that merged, told to the Jobs the queue did not reach and, in a line, to
    /// those it did.
    pub(crate) async fn wire_told_of_merge(self: &Arc<Self>, served: &Served, number: u64, branch: &str) {
        self.wire_told_of(served, number, branch, &format!("{number}@merged"), true).await;
    }

    async fn wire_told_of(self: &Arc<Self>, served: &Served, number: u64, branch: &str, held: &str, merged: bool) {
        let root = served.root().to_string();
        let manifest = served.manifest().id().as_str().to_string();
        let mut owners: Vec<Owner> = self.owners_of(served, number, branch, merged).await;
        owners.extend(self.holders_told_of(served, number, branch, merged).await);
        let mut waiting: Vec<(String, bool)> = Vec::new();
        {
            let store = self.store().lock().await;
            let live = store::SessionSearch { manifest_id: None, state: Some(store::SessionState::Live), text: None };
            for one in store.find_sessions(&live).unwrap_or_default() {
                let recipient = Owner::Session(one.id.clone());
                let hosted_here = store.hosting(&one.id).ok().flatten().is_some_and(|hosting| hosting.manifest_id == manifest);
                if owners.contains(&recipient) || !hosted_here {
                    continue;
                }
                let kept = |cause: &str| store.pull_notice_kept(&root, number, "", cause, &recipient.key()).unwrap_or(true);
                let (queued, after) = (kept(QUEUED), kept(MERGED));
                if (merged && !after) || (!merged && !queued) {
                    waiting.push((one.id, queued));
                }
            }
        }
        if waiting.is_empty() {
            return;
        }
        let base = base_of(served);
        let mut types: Option<Vec<TypeChange>> = None;
        for (id, told_at_the_queue) in waiting {
            let text = if merged && told_at_the_queue {
                format!("#{number} merged: its wire change is on {base}.")
            } else {
                if types.is_none() {
                    types = Some(self.wire_types_of(served, number, held).await);
                }
                let types = types.as_deref().unwrap_or_default();
                if types.is_empty() {
                    return;
                }
                let Some(standing) = self.writing_in(&Who::Session(id.clone())).await.filter(|one| one.manifest == manifest) else {
                    continue;
                };
                let (worktrees, looked, since) = (standing.worktrees, types.to_vec(), base.clone());
                let reads = tokio::task::spawn_blocking(move || readers_of(&worktrees, &since, &looked))
                    .await
                    .unwrap_or_default();
                let reads: Vec<Read<'_>> = reads
                    .into_iter()
                    .filter_map(|(name, at)| Some(Read { change: types.iter().find(|one| one.name == name)?, at }))
                    .collect();
                if reads.is_empty() {
                    continue;
                }
                said(number, &base, merged, &reads)
            };
            let told = if merged && told_at_the_queue {
                self.told_news_by_fleet(&id, &text, ipc::Notice::WireChanged).await
            } else {
                self.told_by_fleet(&id, &text, ipc::Notice::WireChanged).await
            };
            if matches!(told, Ok(true)) {
                let now = self.now();
                let cause = if merged { MERGED } else { QUEUED };
                let _ = self.store().lock().await.keep_pull_notice(&root, number, "", cause, &Owner::Session(id).key(), &now);
            }
        }
    }

    /// The wire types a pull request changes, asked of the forge once for each commit. **A forge that
    /// will not answer leaves nothing held** and the next reading asks again.
    async fn wire_types_of(&self, served: &Served, number: u64, held: &str) -> Vec<TypeChange> {
        let key = format!("{}#{held}", served.root());
        if let Some(types) = self.sweeping().lock().await.wire_types.get(&key) {
            return types.clone();
        }
        let ask = number.to_string();
        let Some(files) = self.forge_asked(served.root(), move |vcs: &V, root: &str| vcs.pull_request_patches(root, &ask)).await else {
            return Vec::new();
        };
        let types = changed_types(&files);
        let mut sweep = self.sweeping().lock().await;
        if sweep.wire_types.len() >= HELD {
            sweep.wire_types.clear();
        }
        sweep.wire_types.insert(key, types.clone());
        types
    }
}

/// For each changed type the worktrees name, where: when one of them changes a file the wire is
/// read from, since a branch that does not has nothing to adjust.
fn readers_of(worktrees: &[PathBuf], base: &str, types: &[TypeChange]) -> Vec<(String, Vec<String>)> {
    let reading: Vec<&PathBuf> = worktrees
        .iter()
        .filter(|worktree| {
            changed_since_base(worktree, base)
                .is_ok_and(|changed| changed.iter().any(|path| READERS.iter().any(|dir| path.starts_with(&format!("{dir}/")))))
        })
        .collect();
    let mut reads = Vec::new();
    for change in types {
        let mut at: Vec<String> = Vec::new();
        for worktree in &reading {
            for place in adapters::clash::named_in(worktree, &change.name, &READERS, PLACES).unwrap_or_default() {
                if !at.contains(&place) {
                    at.push(place);
                }
            }
        }
        at.truncate(PLACES);
        if !at.is_empty() {
            reads.push((change.name.clone(), at));
        }
    }
    reads
}

/// What a Job is told: the pull request, how many types it reads, and for each what changed and
/// where the branch reads it.
fn said(number: u64, base: &str, merged: bool, reads: &[Read<'_>]) -> String {
    let count = reads.len();
    let types = if count == 1 { "type" } else { "types" };
    let head = if merged { format!("Wire change merged into {base}") } else { "Wire change in the merge queue".to_string() };
    let mut out = format!("{head}: #{number}. It changes {count} {types} your branch reads.");
    for read in reads {
        out.push_str(&format!("\n{}: {}. Read at {}", read.change.name, read.change.said(), read.at.join(", ")));
    }
    out
}
