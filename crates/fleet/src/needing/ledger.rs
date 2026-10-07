//! A need as a row of the session ledger, and the order it is served in.
//! `docs/capabilities/needs.md`.
//!
//! **Plain functions over the store**, so that a Job's declaration, a session's
//! `armada need` and the conversion of what a clone already held all write one
//! row the same way, and the tests can read the order without a Fleet.
//!
//! **A need is not exclusive**: the ledger's exclusive attach would give back
//! this holder's other needs, which is right for a slot and wrong here. **The
//! order is `since`, then the holder's id**, which is what
//! `Store::standing_in_order` returns and nothing here re-sorts.

use std::collections::BTreeMap;

use adapters::needs::clean_path;
use store::{AttachmentState, Holder, HolderKind, KeptAttachment, Store, WriteError};

/// The ledger's word for a need.
pub(crate) const NEED: &str = "need";

/// The prefix of the id a need held by a branch alone carries: no Job and no
/// session stands on the branch, so the branch is the identity.
const BRANCH: &str = "branch:";

/// One need, read off the ledger.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Need {
    pub holder: Holder,
    /// Who holds it as a person would say it: the branch it was declared from,
    /// else the session's title, else the holder's id.
    pub held_by: String,
    pub path: String,
    /// What is needed there, as the declarer said it.
    pub what: String,
    /// What the holder took, once it has said.
    pub took: Option<String>,
    pub since: String,
}

impl Need {
    /// `who: what`, and what it took where it said.
    pub fn describe(&self) -> String {
        match &self.took {
            Some(took) => format!("{}: {}, took {took}", self.held_by, self.what),
            None => format!("{}: {}, took nothing yet", self.held_by, self.what),
        }
    }

    fn of(row: &KeptAttachment, store: &Store) -> Need {
        let held_by = row
            .detail
            .get("branch")
            .cloned()
            .or_else(|| match row.holder.kind {
                HolderKind::Session => store
                    .session(&row.holder.id)
                    .ok()
                    .flatten()
                    .and_then(|session| session.title),
                HolderKind::Job => None,
            })
            .unwrap_or_else(|| row.holder.id.clone());
        Need {
            holder: row.holder.clone(),
            held_by,
            path: row.target.clone(),
            what: row.detail.get("what").cloned().unwrap_or_default(),
            took: row.detail.get("took").cloned(),
            since: row.since.clone(),
        }
    }
}

/// What declaring came to.
#[derive(Debug)]
pub(crate) struct Declared {
    pub mine: Need,
    /// The holder had declared this path already; nothing was recorded.
    pub already: bool,
    /// The standing needs on the same path that were declared first, in order.
    pub ahead: Vec<Need>,
}

/// The holder of a need that belongs to a branch no Job or session stands on.
pub(crate) fn branch_holder(branch: &str) -> Holder {
    Holder::session(format!("{BRANCH}{branch}"))
}

/// The branch a holder is, where it is nothing but a branch.
pub(crate) fn branch_of(holder: &Holder) -> Option<&str> {
    match holder.kind {
        HolderKind::Session => holder.id.strip_prefix(BRANCH),
        HolderKind::Job => None,
    }
}

/// Who holds `branch` in this repository: the Job standing on it, else a session
/// standing on it, else the branch itself. **The same rule for a Job's own
/// declaration and a terminal's `armada need`**, so both land in one order.
pub(crate) fn holder_of_branch(
    store: &Store,
    manifest: &str,
    branch: &str,
) -> Result<Holder, WriteError> {
    let standing: Vec<Holder> = store
        .attachments_at("branch", branch, Some(manifest))?
        .into_iter()
        .filter(|row| row.state == AttachmentState::Standing)
        .map(|row| row.holder)
        .collect();
    let by_kind = |kind| standing.iter().find(|holder| holder.kind == kind).cloned();
    Ok(by_kind(HolderKind::Job)
        .or_else(|| by_kind(HolderKind::Session))
        .unwrap_or_else(|| branch_holder(branch)))
}

/// Every standing need of one repository, in the order they are served, narrowed
/// to `path` where it is named.
pub(crate) fn standing(
    store: &Store,
    manifest: &str,
    path: Option<&str>,
) -> Result<Vec<Need>, WriteError> {
    Ok(store
        .standing_in_order(NEED, manifest, path)?
        .iter()
        .map(|row| Need::of(row, store))
        .collect())
}

/// The needs in `all` on `mine`'s path that come before it. `all` is in order.
pub(crate) fn ahead_of(all: &[Need], mine: &Need) -> Vec<Need> {
    all.iter()
        .take_while(|other| !(other.holder == mine.holder && other.path == mine.path))
        .filter(|other| other.path == mine.path)
        .cloned()
        .collect()
}

/// What `holder` waits behind: each of its needs with the ones ahead of it.
pub(crate) fn behind(all: &[Need], holder: &Holder) -> Vec<(Need, Vec<Need>)> {
    all.iter()
        .filter(|need| &need.holder == holder)
        .map(|need| (need.clone(), ahead_of(all, need)))
        .filter(|(_, ahead)| !ahead.is_empty())
        .collect()
}

/// What a held Job says it waits behind, one clause per path.
pub(crate) fn waiting_text(behind: &[(Need, Vec<Need>)]) -> String {
    let clauses: Vec<String> = behind
        .iter()
        .map(|(mine, ahead)| {
            let who: Vec<String> = ahead.iter().map(Need::describe).collect();
            format!("{} on {} ({})", mine.what, mine.path, who.join("; "))
        })
        .collect();
    format!("waiting behind {}", clauses.join("; "))
}

fn row(
    holder: &Holder,
    manifest: &str,
    path: &str,
    detail: BTreeMap<String, String>,
    at: &str,
) -> KeptAttachment {
    KeptAttachment {
        holder: holder.clone(),
        kind: NEED.into(),
        manifest_id: manifest.into(),
        target: path.into(),
        state: AttachmentState::Standing,
        detail,
        since: at.into(),
        changed_at: at.into(),
    }
}

/// Record `holder`'s need on `path`, declared from `branch`. **The same holder
/// and path again records nothing.**
pub(crate) fn declare(
    store: &mut Store,
    holder: &Holder,
    manifest: &str,
    branch: &str,
    path: &str,
    what: &str,
    at: &str,
) -> Result<Declared, WriteError> {
    let path = clean_path(path);
    let held = standing(store, manifest, Some(&path))?;
    let already = held.iter().any(|need| &need.holder == holder);
    if !already {
        let detail = BTreeMap::from([
            ("what".to_string(), what.to_string()),
            ("branch".to_string(), branch.to_string()),
        ]);
        store.attach(&row(holder, manifest, &path, detail, at), false)?;
    }
    let held = standing(store, manifest, Some(&path))?;
    // Present, since it was just written or already stood: the fallback is the
    // row as it would read, so a store that dropped it is not a second path.
    let mine = held
        .iter()
        .find(|need| &need.holder == holder)
        .cloned()
        .unwrap_or_else(|| Need {
            holder: holder.clone(),
            held_by: branch.to_string(),
            path: path.clone(),
            what: what.to_string(),
            took: None,
            since: at.to_string(),
        });
    let ahead = ahead_of(&held, &mine);
    Ok(Declared {
        mine,
        already,
        ahead,
    })
}

/// Record what `holder` took on `path`. `None` where it holds no standing need
/// there.
pub(crate) fn took(
    store: &mut Store,
    holder: &Holder,
    manifest: &str,
    path: &str,
    value: &str,
    at: &str,
) -> Result<Option<Need>, WriteError> {
    let path = clean_path(path);
    let held = store.attachments_of(holder)?.into_iter().find(|row| {
        row.kind == NEED
            && row.manifest_id == manifest
            && row.target == path
            && row.state == AttachmentState::Standing
    });
    let Some(mut held) = held else {
        return Ok(None);
    };
    held.detail.insert("took".into(), value.into());
    held.changed_at = at.into();
    store.attach(&held, false)?;
    Ok(Some(Need::of(&held, store)))
}

/// Give `holder`'s need on `path` back. `false` where it had none standing.
pub(crate) fn release(
    store: &mut Store,
    holder: &Holder,
    manifest: &str,
    path: &str,
    at: &str,
) -> Result<bool, WriteError> {
    store.settle(
        holder,
        NEED,
        manifest,
        &clean_path(path),
        AttachmentState::GivenBack,
        at,
    )
}

/// Everything `holder` still held is spent, where its work landed, or given
/// back, where it did not. **One move**, so nothing here asks which.
pub(crate) fn ended(
    store: &mut Store,
    holder: &Holder,
    state: AttachmentState,
    at: &str,
) -> Result<bool, WriteError> {
    store.settle_held(holder, Some(NEED), state, at)
}

/// Give back every need held by a branch alone whose branch `exists` no longer
/// finds, so nobody waits behind a branch that was deleted. A Job's and a
/// session's needs are given back where they end, not here.
pub(crate) fn gone_branches_given_back(
    store: &mut Store,
    manifest: &str,
    exists: impl Fn(&str) -> bool,
    at: &str,
) -> Result<(), WriteError> {
    for need in standing(store, manifest, None)? {
        let Some(branch) = branch_of(&need.holder) else {
            continue;
        };
        if !exists(branch) {
            store.settle(
                &need.holder,
                NEED,
                manifest,
                &need.path,
                AttachmentState::GivenBack,
                at,
            )?;
        }
    }
    Ok(())
}
