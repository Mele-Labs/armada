//! Which Session works a slot, a branch or a pull request. **The lease in the pool answers**, and
//! a ledger row is a claim believed only while the pool agrees with it: a terminal Session's
//! directory is a guess about where it works, and a slot it left stays in that directory for
//! whoever leases it next. `docs/concepts/session.md`, *Whose slot it is*.
//!
//! Every reader of "who holds this" asks here: the owner of a pull request, the holder of a need,
//! the Sessions told a build, the Sessions the write-clash check sets against each other.

use std::path::PathBuf;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use adapters::leasing::Holder as Lease;
use store::{AttachmentState, Holder, HolderKind, KeptAttachment, KeptSession, SessionState, Store, WriteError};

use crate::daemon::Fleet;
use crate::pull_requesting::PR;
use crate::repositories::Served;
use crate::sessioning::{placed, Placed};

/// What the pool says of a Session's claim to a slot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Claim {
    /// The lease is written as this Session's.
    Holds,
    /// Held, and the lease names no live Session: a person's own terminal, a lease from before
    /// leases named theirs, or one taken for a Session that has since ended (`/clear`).
    Unnamed,
    /// Free, or held for another Session or a Job.
    Not,
}

fn live(store: &Store, id: &str) -> bool {
    store
        .session(id)
        .ok()
        .flatten()
        .is_some_and(|one| one.state != SessionState::Ended)
}

/// The pool's word on `session` and slot `slot` of the repository at `root`. `handed` is the Job
/// a piloted Session was handed the slot by, whose lease it is.
pub(crate) fn claim_on(
    store: &Store,
    root: &str,
    slot: u32,
    session: &str,
    handed: Option<&str>,
) -> Claim {
    let at = adapter_traits::slot_path(root.trim_end_matches('/'), slot);
    let at = std::path::Path::new(&at);
    match adapters::leasing::holder_of(at) {
        Some(Lease::Job(id)) if id == session || handed == Some(id.as_str()) => Claim::Holds,
        Some(held) if held.alive() && !matches!(held, Lease::Job(_)) => {
            match adapters::leasing::session_of(at) {
                Some(named) if named == session => Claim::Holds,
                Some(named) if live(store, &named) => Claim::Not,
                _ => Claim::Unnamed,
            }
        }
        _ => Claim::Not,
    }
}

/// A slot a Session stands in that the pool, the worktree and the ledger agree on.
pub(crate) struct Stand {
    pub(crate) path: PathBuf,
    pub(crate) claim: Claim,
}

impl Stand {
    /// Whether `branch` is what is checked out there.
    pub(crate) fn on(&self, branch: &str) -> bool {
        adapters::leasing::checked_out(&self.path).as_deref() == Some(branch)
    }
}

/// Where `session` stands in the pool of `root`, off the rows it `held`: its standing `slot` row
/// of this repository, a lease the pool does not deny, and the worktree there.
pub(crate) fn stand_of(
    store: &Store,
    manifest: &str,
    root: &str,
    held: &[KeptAttachment],
    session: &str,
) -> Option<Stand> {
    let row = held
        .iter()
        .find(|one| one.kind == "slot" && one.state == AttachmentState::Standing && one.manifest_id == manifest)?;
    let slot: u32 = row.target.parse().ok()?;
    let handed = row.detail.get("handed").and_then(|said| said.strip_prefix("job "));
    let claim = claim_on(store, root, slot, session, handed);
    let path = PathBuf::from(adapter_traits::slot_path(root.trim_end_matches('/'), slot));
    (claim != Claim::Not && path.is_dir()).then_some(Stand { path, claim })
}

/// The live Sessions the pool confirms are working `branch`: a standing `branch` row, a standing
/// `slot` row, a lease that is theirs, and the branch checked out there. Those whose lease is
/// written as theirs, else those whose lease names nobody.
fn confirmed_on(store: &Store, manifest: &str, root: &str, branch: &str) -> Vec<String> {
    let (mut holds, mut unnamed) = (Vec::new(), Vec::new());
    for row in store.attachments_at("branch", branch, Some(manifest)).unwrap_or_default() {
        if row.holder.kind != HolderKind::Session
            || row.state != AttachmentState::Standing
            || !live(store, &row.holder.id)
        {
            continue;
        }
        let held = store.attachments_of(&row.holder).unwrap_or_default();
        match stand_of(store, manifest, root, &held, &row.holder.id) {
            Some(stand) if stand.on(branch) && stand.claim == Claim::Holds => holds.push(row.holder.id),
            Some(stand) if stand.on(branch) => unnamed.push(row.holder.id),
            _ => {}
        }
    }
    if holds.is_empty() {
        unnamed
    } else {
        holds
    }
}

/// Whether a branch `session` is credited with is one the pool allows it: a Session that stands
/// in no slot has nothing to contradict it, and one that does stands there on that branch.
pub(crate) fn pool_allows_branch(store: &Store, manifest: &str, root: &str, session: &str, branch: &str) -> bool {
    let held = store.attachments_of(&Holder::session(session)).unwrap_or_default();
    match stand_of(store, manifest, root, &held, session) {
        Some(stand) => stand.on(branch),
        None => !held.iter().any(|one| {
            one.kind == "slot" && one.state == AttachmentState::Standing && one.manifest_id == manifest
        }),
    }
}

/// The live Session whose row of `kind` at `target` is oldest. The only fallback: it is the
/// Session that took the thing first, which a later, stale credit is not.
fn earliest(store: &Store, manifest: &str, kind: &str, target: &str, spent: bool) -> Option<String> {
    store
        .attachments_at(kind, target, Some(manifest))
        .unwrap_or_default()
        .into_iter()
        .filter(|row| {
            row.holder.kind == HolderKind::Session
                && (row.state == AttachmentState::Standing
                    || (spent && row.state == AttachmentState::Spent))
                && live(store, &row.holder.id)
        })
        .min_by(|one, other| one.since.cmp(&other.since))
        .map(|row| row.holder.id)
}

/// Who works `branch`: the Sessions the pool confirms, else the one that stood on it first.
pub(crate) fn sessions_on_branch(store: &Store, manifest: &str, root: &str, branch: &str) -> Vec<String> {
    let confirmed = confirmed_on(store, manifest, root, branch);
    if !confirmed.is_empty() {
        return confirmed;
    }
    earliest(store, manifest, "branch", branch, false).into_iter().collect()
}

/// Who owns pull request `number` of `branch`, among Sessions. **The one the pool confirms**
/// working the branch, and among them the one whose `pr` row holds the number; where the pool
/// confirms nobody, the Session whose `pr` row is oldest, else the one that stood on the branch
/// first. `spent` also counts rows used up by the merge.
pub(crate) fn sessions_on_pull(
    store: &Store,
    manifest: &str,
    root: &str,
    number: u64,
    branch: &str,
    spent: bool,
) -> Vec<String> {
    let confirmed = confirmed_on(store, manifest, root, branch);
    if !confirmed.is_empty() {
        let holding: Vec<String> = store
            .attachments_at(PR, &number.to_string(), Some(manifest))
            .unwrap_or_default()
            .into_iter()
            .filter(|row| row.state == AttachmentState::Standing)
            .map(|row| row.holder.id)
            .filter(|id| confirmed.contains(id))
            .collect();
        return if holding.is_empty() { confirmed } else { holding };
    }
    earliest(store, manifest, PR, &number.to_string(), spent)
        .or_else(|| earliest(store, manifest, "branch", branch, spent))
        .into_iter()
        .collect()
}

/// Whether the pool backs a branch or pull request credited to `session` from where it stands:
/// a Session outside the pool's slots has nothing to contradict it, and one inside a slot is
/// believed only if the lease there is not another's.
pub(crate) fn pool_backs(store: &Store, session: &KeptSession, roots: &[(String, String)]) -> bool {
    let Placed { manifest: Some(manifest), slot: Some(slot) } = placed(&session.cwd, roots) else {
        return true;
    };
    roots
        .iter()
        .find(|(id, _)| *id == manifest)
        .is_none_or(|(_, root)| claim_on(store, root, slot, &session.id, None) != Claim::Not)
}

/// Take the slot `placed` names for `holder`, or give back the one it was in where it stands in
/// none. **The pool is asked first**: a directory is where a harness
/// says it is, and the lease says who works there. `true` where the ledger changed.
pub(crate) fn stand_in_slot(
    store: &mut Store,
    holder: &Holder,
    placed: &Placed,
    roots: &[(String, String)],
    now: &str,
) -> Result<bool, WriteError> {
    let root = placed
        .manifest
        .as_ref()
        .and_then(|manifest| roots.iter().find(|(id, _)| id == manifest))
        .map(|(_, root)| root.trim_end_matches('/'));
    let (Some(manifest), Some(slot), Some(root)) = (&placed.manifest, placed.slot, root) else {
        return store.give_back(holder, Some("slot"), now);
    };
    // A slot the lease is another's is not credited, and nothing is given back for the move: what
    // the Session still holds is the pool's to say (`Fleet::slot_rows_followed`).
    if claim_on(store, root, slot, &holder.id, None) == Claim::Not {
        return Ok(false);
    }
    let row = |kind: &str, target: String| KeptAttachment {
        holder: holder.clone(),
        kind: kind.into(),
        manifest_id: manifest.clone(),
        target,
        state: AttachmentState::Standing,
        detail: Default::default(),
        since: now.into(),
        changed_at: now.into(),
    };
    let mut changed = store.attach(&row("slot", slot.to_string()), true)?;
    // What is checked out in the slot is the branch worked in it, which a pull request from it
    // is found by.
    let at = adapter_traits::slot_path(root, slot);
    let at = std::path::Path::new(&at);
    if let Some(branch) = adapters::leasing::checked_out(at).or_else(|| adapters::leasing::branch_of(at)) {
        changed |= store.attach(&row("branch", branch), true)?;
    }
    Ok(changed)
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
    /// Give back the slot, and the branch credited through it, of every live Session the pool no
    /// longer shows as holding it. **Run on the pull sweep**, so a release by any road (the agent's
    /// own, Bridge's, a lease taken back from a gone holder) is settled for terminal and hosted
    /// Sessions alike, and not at a hosted Session's next gate call. A `pr` row stays: the Session
    /// that opened a pull request still owns it after it leaves the slot.
    pub(crate) async fn slot_rows_followed(&self, served: &Served) {
        let manifest = served.manifest().id().as_str().to_string();
        let root = served.root().to_string();
        let now = self.now().as_str().to_string();
        let mut hosted = Vec::new();
        let mut changed = Vec::new();
        {
            let mut store = self.store().lock().await;
            for row in store.standing_of_kind("slot", &manifest).unwrap_or_default() {
                let Ok(slot) = row.target.parse::<u32>() else { continue };
                let id = row.holder.id.clone();
                if row.holder.kind != HolderKind::Session {
                    continue;
                }
                let Some(session) = store.session(&id).ok().flatten().filter(|one| one.state != SessionState::Ended) else {
                    continue;
                };
                let handed = row.detail.get("handed").and_then(|said| said.strip_prefix("job "));
                if claim_on(&store, &root, slot, &id, handed) != Claim::Not {
                    continue;
                }
                if session.origin == "bridge" {
                    hosted.push(id);
                    continue;
                }
                let gave = [store.give_back(&row.holder, Some("slot"), &now), store.give_back(&row.holder, Some("branch"), &now)];
                if gave.iter().any(|one| matches!(one, Ok(true))) {
                    if let Ok(record) = self.ledger_row(&store, &session) {
                        changed.push(record);
                    }
                }
            }
        }
        for id in hosted {
            let hosting = self.store().lock().await.hosting(&id).ok().flatten();
            if let Some(hosting) = hosting {
                self.follow_the_pool(&id, served, hosting).await;
            }
        }
        for record in changed {
            self.publish(ipc::Event::SessionChanged(record));
        }
    }
}
