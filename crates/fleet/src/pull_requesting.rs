//! A pull request by repository and number, and the acts a Session takes on
//! one: read it, take it out of draft, merge it, ask the forge to merge it when
//! its checks pass, and draft a Code Review Job against it. Since 23.48.
//! `docs/concepts/session.md`, *Acts on a pull request*.
//!
//! **Nobody's Job.** `merge_pull_request` is a Job's press at its gate and
//! records the landing on the Job. A Session's pull request has no Job to
//! record on, so these go to the forge through the same adapter calls and
//! answer the pull request as the forge shows it afterwards.
//!
//! **Every write is read first.** A refusal Fleet can name before it writes is
//! named, so a merge on a pull request whose checks are running never reaches
//! the forge, and the forge's own words are for what Fleet could not know.
//!
//! **The ledger follows the forge, in one write a row.** Every read and every
//! act, and the pull reading each interval (`pull_ledger_kept_current`), take the
//! pull request's state as the `detail` of every standing `pr` row Sessions
//! hold for it and settle a merged one `spent` and a closed one `given_back`
//! in the same write, so no row stands that says merged.

use std::collections::BTreeMap;

use adapter_traits::{
    AgentHarness, Delivery, FailureWeight, Forge as TheForge, NotMerged, PullRequestStanding as Forge, Vcs,
    WhatTheForgeRan, WorkProduct,
};
use api::{PullRequests, Refusal};
use ipc::{
    ForgeChecks, ManifestId, PullRequestStanding, PullRequestState, WireError,
};
use store::HolderKind;

use crate::daemon::Fleet;
use crate::repositories::Served;
use crate::session_issues::{issue_held, PULL_REQUESTS};
use crate::refusing::{
    MERGE_BRANCH_PROTECTED, MERGE_CHECKS_NOT_PASSED, MERGE_CONFLICTED, MERGE_NOT_OPEN,
    MERGE_NO_TOOL, MERGE_REFUSED,
};

/// The forge would not say what the pull request is: no tool, nobody signed in,
/// no such number, or a state this build has no word for. A 500, because
/// asking again is reasonable.
pub(crate) const UNREADABLE: &str = "fleet.pull_request_unreadable";
/// A draft cannot be merged, or have auto-merge asked of it. A 409.
pub(crate) const IS_A_DRAFT: &str = "fleet.pull_request_is_a_draft";
/// Taking out of draft what is not one. A 409.
pub(crate) const NOT_A_DRAFT: &str = "fleet.pull_request_not_a_draft";
/// The forge would not take it out of draft, in its own sentence. A 409.
pub(crate) const READY_REFUSED: &str = "fleet.ready_refused";
/// The forge would not turn auto-merge on, in its own sentence. A 409.
pub(crate) const AUTO_MERGE_REFUSED: &str = "fleet.auto_merge_refused";
/// A review pressed from a Session the ledger has never heard of. A 422.
pub(crate) const SESSION_UNKNOWN: &str = "fleet.session_unknown";

/// The ledger's kind for a pull request a Session holds, and the kind its owners are read under.
pub(crate) const PR: &str = "pr";

/// What a pull request was named as.
enum Named {
    Number(u64),
    Address { number: u64 },
}

/// A number, or an address ending `/pull/<number>`.
fn named(said: &str) -> Option<Named> {
    let said = said.trim().trim_end_matches('/');
    if let Ok(number) = said.parse() {
        return Some(Named::Number(number));
    }
    let (before, last) = said.rsplit_once('/')?;
    let number = last.parse().ok()?;
    (before.ends_with("/pull") && before.starts_with("http")).then_some(Named::Address { number })
}

/// The number a pull request's address ends in.
pub(crate) fn number_of(address: &str) -> Option<u64> {
    match named(address)? {
        Named::Number(number) | Named::Address { number, .. } => Some(number),
    }
}

fn checks_of(ran: &WhatTheForgeRan) -> Option<ForgeChecks> {
    match ran {
        // **Not a pass**, `WhatTheForgeRan::NothingRan`'s own rule.
        WhatTheForgeRan::NothingRan | WhatTheForgeRan::StillWaiting { .. } => {
            Some(ForgeChecks::Pending)
        }
        WhatTheForgeRan::AllPassed { .. } => Some(ForgeChecks::Passed),
        WhatTheForgeRan::SomeFailed { failed, weight, .. } => Some(ForgeChecks::Failed {
            failing: failed
                .iter()
                .map(|name| name.as_written().to_string())
                .collect(),
            weight: match weight {
                FailureWeight::Required => ipc::FailureWeight::Required,
                FailureWeight::WhileRunning => ipc::FailureWeight::WhileRunning,
                FailureWeight::AfterRequired => ipc::FailureWeight::AfterRequired,
            },
        }),
        WhatTheForgeRan::Unreadable => None,
    }
}

/// Whether a `pr` row says its pull request merged: the forge's `state` in its
/// `detail` first, and `spent` only where the forge has not described it. **A
/// row is spent by a `gh pr` command the mod read, which the forge may not
/// bear out** (#2027, #3042), so spent alone is never a merge it contradicts.
pub(crate) fn pull_merged(detail: &BTreeMap<String, String>, spent: bool) -> bool {
    match detail.get("state").map(String::as_str) {
        Some("merged") => true,
        Some("open" | "draft" | "closed") => false,
        _ => spent,
    }
}

/// What a ledger row's `detail` says of the pull request, in the words the
/// wire uses.
fn detail_of(state: &PullRequestState) -> BTreeMap<String, String> {
    let (checks, failing) = match &state.checks {
        ForgeChecks::Pending => ("pending", String::new()),
        ForgeChecks::Passed => ("passed", String::new()),
        ForgeChecks::Failed { failing, weight } => match weight {
            ipc::FailureWeight::Required => ("failed", failing.join(", ")),
            ipc::FailureWeight::WhileRunning => ("pending", failing.join(", ")),
            ipc::FailureWeight::AfterRequired => ("passed", failing.join(", ")),
        },
    };
    let standing = match state.state {
        PullRequestStanding::Draft => "draft",
        PullRequestStanding::Open => "open",
        PullRequestStanding::Merged => "merged",
        PullRequestStanding::Closed => "closed",
    };
    let mut detail = BTreeMap::from([
        ("state".to_string(), standing.to_string()),
        ("auto_merge".to_string(), state.auto_merge.to_string()),
        ("queued".to_string(), state.queued.to_string()),
        ("checks".to_string(), checks.to_string()),
        ("failing".to_string(), failing),
        ("title".to_string(), state.title.clone()),
        ("branch".to_string(), state.branch.clone()),
        ("address".to_string(), state.address.clone()),
    ]);
    // Left out where there is none, so a row never holds a description of nothing.
    if let Some(body) = &state.body {
        detail.insert("body".to_string(), body.clone());
    }
    // Bridge draws stacked pull requests from it; a forge that named none leaves it off.
    if !state.base.is_empty() {
        detail.insert("base".to_string(), state.base.clone());
    }
    // The same rule: a pull request that closes nothing has no `closes`.
    if !state.closes.is_empty() {
        let closes: Vec<String> = state.closes.iter().map(u64::to_string).collect();
        detail.insert("closes".to_string(), closes.join(","));
    }
    detail
}

/// What one read of a pull request needs from the Manifest, owned so it can go to a thread.
pub(crate) struct PullRequestAsk {
    manifest_id: ManifestId,
    root: String,
    number: u64,
    gate: String,
    base: Option<String>,
}

impl PullRequestAsk {
    fn of(manifest_id: &ManifestId, root: &str, number: u64, served: &Served) -> Self {
        Self {
            manifest_id: manifest_id.clone(),
            root: root.to_string(),
            number,
            gate: served.manifest().ci_gate().to_string(),
            base: served.manifest().base().map(str::to_string),
        }
    }
}

/// The pull request as the forge shows it. **Two asks, and either one silent is silent**: a
/// state with no checks would let a merge be pressed on a guess.
fn read_from_forge<V: TheForge>(
    vcs: &V,
    run: &ipc::RunId,
    ask: &PullRequestAsk,
) -> Result<PullRequestState, Refusal> {
    let PullRequestAsk { manifest_id, root, number, gate, base } = ask;
    let number = *number;
    let named = number.to_string();
    let unreadable = || {
        Refusal::Fault(WireError::raised(
            UNREADABLE,
            format!("the forge would not say what pull request {number} is"),
            run.clone(),
        ))
    };
    let facts = vcs.pull_request_facts(root, &named).ok_or_else(unreadable)?;
    let checks = checks_of(&vcs.under_review(root, &named, gate).checks).ok_or_else(unreadable)?;
    // Only an open pull request waits in the queue, and a forge that will not
    // say leaves it out of the queue: the read stays the pull request's own.
    let queued = facts.standing == Forge::Open
        && base
            .as_deref()
            .and_then(|base| vcs.merge_queue(root, base))
            .is_some_and(|queue| queue.iter().any(|entry| entry.number == number));
    Ok(PullRequestState {
        manifest_id: manifest_id.clone(),
        number,
        state: match facts.standing {
            Forge::Draft => PullRequestStanding::Draft,
            Forge::Open => PullRequestStanding::Open,
            Forge::Merged => PullRequestStanding::Merged,
            Forge::Closed => PullRequestStanding::Closed,
        },
        auto_merge: facts.auto_merge,
        queued,
        checks,
        title: facts.title,
        branch: facts.branch,
        base: facts.base,
        address: facts.url,
        body: facts.body,
        closes: facts.closes,
    })
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
    pub(crate) fn pr_refusal(&self, code: &'static str, message: impl Into<String>) -> WireError {
        WireError::raised(code, message, self.run_id())
    }

    /// The pull request as the forge shows it, or the forge's silence. **Two
    /// asks, and either one silent is silent**: a state with no checks would
    /// let a merge be pressed on a guess.
    pub(crate) fn pull_request_read(
        &self,
        manifest_id: &ManifestId,
        root: &str,
        number: u64,
    ) -> Result<PullRequestState, Refusal> {
        let served = self.served_named(Some(manifest_id))?;
        read_from_forge(
            &**self.vcs(),
            &self.run_id(),
            &PullRequestAsk::of(manifest_id, root, number, &served),
        )
    }

    /// [`pull_request_read`](Self::pull_request_read), asked of the forge off the runtime's
    /// threads: three processes in a row are not for a worker to wait on.
    pub(crate) async fn pull_request_read_off(
        &self,
        manifest_id: &ManifestId,
        root: &str,
        number: u64,
    ) -> Result<PullRequestState, Refusal> {
        let served = self.served_named(Some(manifest_id))?;
        let ask = PullRequestAsk::of(manifest_id, root, number, &served);
        let (vcs, run) = (std::sync::Arc::clone(self.vcs()), self.run_id());
        tokio::task::spawn_blocking(move || read_from_forge(&*vcs, &run, &ask))
            .await
            .expect("the forge process panicked")
    }

    /// Every Session still holding this pull request takes the state as the
    /// `detail` of its `pr` row, and a merged or closed one is settled.
    ///
    /// **Held, never raised**: the forge has already answered and the person
    /// asked about the pull request, not about the ledger, so a row that would
    /// not write is no reason to refuse an answer.
    pub(crate) async fn ledger_follows(&self, state: &PullRequestState) {
        let target = state.number.to_string();
        let detail = detail_of(state);
        let over = match state.state {
            PullRequestStanding::Merged => store::AttachmentState::Spent,
            PullRequestStanding::Closed => store::AttachmentState::GivenBack,
            _ => store::AttachmentState::Standing,
        };
        let now = self.clock().now().as_str().to_string();
        // A key in its title or branch is how the Manifest's tracker itself links an issue to it.
        let named: Vec<ipc::IssueReference> = self
            .served_named(Some(&state.manifest_id))
            .map(|served| self.tracker_of(&served).issues_named_by_pull_request(&state.title, &state.branch))
            .unwrap_or_default()
            .iter()
            .filter_map(|key| ipc::IssueReference::read(key)).collect();
        let working = BTreeMap::from([(PULL_REQUESTS.to_string(), target.clone())]);
        let mut changed = Vec::new();
        {
            let mut store = self.store().lock().await;
            let Ok(rows) = store.attachments_at(PR, &target, Some(state.manifest_id.as_str()))
            else {
                return;
            };
            for row in rows.iter().filter(|row| {
                row.holder.kind == HolderKind::Session
                    && (row.state == store::AttachmentState::Standing
                        || (row.state == store::AttachmentState::Spent
                            && matches!(row.detail.get("state").map(String::as_str), Some("open" | "draft"))))
            }) {
                // **One write for the detail and the state**: a row never reads merged in
                // one and standing in the other.
                let mut moved = matches!(store.follow(row, over, &detail, &now), Ok(true));
                // What it closes is what the Session works on, merged or not.
                for number in &state.closes {
                    let issue = ipc::IssueReference::number(*number);
                    moved |= issue_held(&mut store, &row.holder, &row.manifest_id, &issue, &BTreeMap::new(), &now)
                        .unwrap_or(false);
                }
                for issue in &named {
                    moved |= issue_held(&mut store, &row.holder, &row.manifest_id, issue, &working, &now).unwrap_or(false);
                }
                if !moved {
                    continue;
                }
                let Some(session) = store.session(&row.holder.id).ok().flatten() else {
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

    /// Every pull request a Session still holds in this repository, read once and
    /// followed, **so a terminal Session nobody has opened is as current as one
    /// whose sheet is open**. Rows the forge will not answer for stay as they were.
    pub(crate) async fn pull_ledger_kept_current(&self, manifest_id: &ManifestId, root: &str) {
        let mut numbers: Vec<u64> = {
            let store = self.store().lock().await;
            let Ok(rows) = store.followed_of_kind(PR, manifest_id.as_str()) else {
                return;
            };
            rows.iter()
                .filter(|row| row.holder.kind == HolderKind::Session)
                .filter_map(|row| row.target.parse().ok())
                .collect()
        };
        numbers.sort_unstable();
        numbers.dedup();
        for number in numbers {
            let _ = self.pull_request_followed(manifest_id, root, number).await;
        }
        // After the pull requests, so an issue one of them closed this pass is read in it too.
        self.issue_ledger_kept_current(manifest_id, root).await;
    }

    /// Read, and let the ledger follow. Every act's last step and the query.
    async fn pull_request_followed(
        &self,
        manifest_id: &ManifestId,
        root: &str,
        number: u64,
    ) -> Result<PullRequestState, Refusal> {
        let state = self.pull_request_read(manifest_id, root, number)?;
        self.ledger_follows(&state).await;
        self.lifecycle_pull_seen(manifest_id, root, &state).await;
        Ok(state)
    }

    pub(crate) fn not_merged_refusal(&self, number: u64, why: NotMerged) -> Refusal {
        let said = why.said();
        let raised = |code: &'static str| {
            self.pr_refusal(code, said.clone())
                .with_field("number", ipc::WireValue::Str(number.to_string()))
                .with_field("refused", ipc::WireValue::Str(why.kind().to_string()))
        };
        match &why {
            NotMerged::Protected { .. } => Refusal::IllegalMove(raised(MERGE_BRANCH_PROTECTED)),
            NotMerged::Conflicted { .. } => Refusal::IllegalMove(raised(MERGE_CONFLICTED)),
            NotMerged::ChecksNotPassed { .. } => {
                Refusal::IllegalMove(raised(MERGE_CHECKS_NOT_PASSED))
            }
            NotMerged::NotOpen { .. } => Refusal::IllegalMove(raised(MERGE_NOT_OPEN)),
            NotMerged::NoTool { .. } => Refusal::Fault(raised(MERGE_NO_TOOL)),
            // The three only `merge_by: push` says: none of them is a forge merge's answer.
            NotMerged::BaseMoved { .. }
            | NotMerged::GateFailed { .. }
            | NotMerged::Unchecked { .. }
            | NotMerged::Refused { .. } => Refusal::Fault(raised(MERGE_REFUSED)),
        }
    }

    /// A pull request that is not open, as a refusal. `None` where it is.
    pub(crate) fn not_open_refusal(&self, state: &PullRequestState) -> Option<Refusal> {
        let raised = |code: &'static str, said: &str| {
            Refusal::IllegalMove(
                self.pr_refusal(code, format!("pull request {} {said}", state.number)),
            )
        };
        match state.state {
            PullRequestStanding::Draft => Some(raised(
                IS_A_DRAFT,
                "is a draft, and the forge will not do that to one",
            )),
            PullRequestStanding::Merged => Some(raised(MERGE_NOT_OPEN, "is merged already")),
            PullRequestStanding::Closed => Some(raised(MERGE_NOT_OPEN, "is closed")),
            PullRequestStanding::Open => None,
        }
    }

    /// What a check that has not passed is called to a person.
    pub(crate) fn checks_not_passed(&self, state: &PullRequestState) -> Refusal {
        let said = match &state.checks {
            ForgeChecks::Failed { failing, .. } => {
                format!(
                    "{} failed on pull request {}",
                    failing.join(", "),
                    state.number
                )
            }
            _ => format!(
                "the checks on pull request {} are still running",
                state.number
            ),
        };
        Refusal::IllegalMove(self.pr_refusal(MERGE_CHECKS_NOT_PASSED, said))
    }
}

impl<H, V, W> PullRequests for Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    async fn get_pull_request(
        &self,
        manifest_id: ManifestId,
        number: u64,
    ) -> Result<PullRequestState, Refusal> {
        let served = self.served_named(Some(&manifest_id))?;
        self.pull_request_followed(&manifest_id, served.root(), number)
            .await
    }

    async fn ready_pull_request(
        self: std::sync::Arc<Self>,
        manifest_id: ManifestId,
        number: u64,
    ) -> Result<PullRequestState, Refusal> {
        let acted = self.ready_acted(&manifest_id, number).await?;
        Ok(self.replied_to(acted))
    }

    async fn merge_pull_request_by_number(
        self: std::sync::Arc<Self>,
        manifest_id: ManifestId,
        number: u64,
    ) -> Result<PullRequestState, Refusal> {
        let acted = self.merge_acted(&manifest_id, number).await?;
        Ok(self.replied_to(acted))
    }

    async fn enable_auto_merge(
        self: std::sync::Arc<Self>,
        manifest_id: ManifestId,
        number: u64,
    ) -> Result<PullRequestState, Refusal> {
        let acted = self.auto_merge_acted(&manifest_id, number).await?;
        Ok(self.replied_to(acted))
    }

    async fn refresh_pull_request(
        self: std::sync::Arc<Self>,
        manifest_id: ManifestId,
        number: u64,
    ) -> Result<PullRequestState, Refusal> {
        let served = self.served_named(Some(&manifest_id))?;
        let state = self
            .pull_request_followed(&manifest_id, served.root(), number)
            .await?;
        self.pull_request_refreshed(&served, &state).await;
        Ok(state)
    }
}

#[cfg(test)]
mod named_as {
    use super::{named, Named};

    #[test]
    fn a_number_is_a_number() {
        assert!(matches!(named(" 1843 "), Some(Named::Number(1843))));
    }

    #[test]
    fn anything_else_is_neither() {
        for said in [
            "",
            "#1843",
            "armada",
            "https://forge.invalid/armada/issues/12",
            "pull/12",
        ] {
            assert!(named(said).is_none(), "{said}");
        }
    }
}
