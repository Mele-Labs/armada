//! The issues a Session works on, as `issue` rows on the ledger, so Bridge can list them with the
//! Session's pull requests under each. `docs/concepts/session/what-is-kept.md`.
//!
//! **Named by the person, never guessed from prose**: an issue tag, or a link into the Session's
//! own repository, in a message of the person's or the request the Session was dispatched with, or
//! one the agent claims (`claim_issue`). A pull request the Session holds names one too: the forge
//! issues it closes, and a key of the tracker's own projects in its title or branch, which is how
//! that tracker links them. A bare `#123` is a number in a sentence. An issue is the
//! forge's number or a tracker's key, as [`IssueReference`] reads it.
//!
//! **One row a Session and issue, and what it already says stands.** A second naming takes it up
//! again where it was given back and fills only what it lacked: the forge's reading, which the pull
//! reading writes, is never overwritten by a tag's address.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use adapters::ForgeAddress;
use api::{Caller, Refusal};
use ipc::{ClaimIssue, IssueClaimed, IssueReference, ManifestId, SessionTag, TagKind, WireError};
use store::{AttachmentState, Holder, HolderKind, KeptAttachment, Store, WriteError};

use crate::backlog::ClaimLost;
use crate::daemon::Fleet;

/// The ledger's kind for an issue a Session works on. `target` is its [`IssueReference`].
pub(crate) const ISSUE: &str = "issue";

const NOT_CLAIMABLE: &str = "fleet.issue_not_claimable";

/// The backlog, synced, names someone else as holding the issue. A 409 naming them.
const HELD_ELSEWHERE: &str = "fleet.issue_held";

/// The detail key naming the pull requests that work an issue, comma-joined ascending.
pub(crate) const PULL_REQUESTS: &str = "pull_requests";

/// The detail key naming who the Job claimed the issue as in a backlog Fleet writes to. A row
/// without it was named, not claimed, and the backlog naming someone else for it is no lost claim.
pub(crate) const CLAIMED_AS: &str = "claimed_as";

/// The detail key saying the Job claimed the issue (`claim_issue`), as against one it filed or was told of.
/// Bridge draws a claimed issue as work in hand and any other as set aside.
pub(crate) const CLAIMED: &str = "claimed";

/// Stand `holder`'s row for `issue`, with `known` where the row says nothing yet. `true` where
/// anything changed. **`pull_requests` only grows**: what `known` names is added to what the row
/// names.
pub(crate) fn issue_held(
    store: &mut Store,
    holder: &Holder,
    manifest_id: &str,
    issue: &IssueReference,
    known: &BTreeMap<String, String>,
    now: &str,
) -> Result<bool, WriteError> {
    let target = issue.as_str().to_string();
    let mut detail = known.clone();
    if let Some(row) = store
        .attachments_at(ISSUE, &target, Some(manifest_id))?
        .into_iter()
        .find(|row| &row.holder == holder)
    {
        let mut numbers = pull_requests_named(row.detail.get(PULL_REQUESTS));
        numbers.extend(pull_requests_named(known.get(PULL_REQUESTS)));
        detail.extend(row.detail);
        // A claim comes after the naming: it is said over a row that was only named, never taken back.
        if known.contains_key(CLAIMED) {
            detail.insert(CLAIMED.to_string(), "true".to_string());
        }
        if !numbers.is_empty() {
            detail.insert(PULL_REQUESTS.to_string(), pull_requests_joined(&numbers));
        }
    }
    store.attach(
        &KeptAttachment {
            holder: holder.clone(),
            kind: ISSUE.into(),
            manifest_id: manifest_id.to_string(),
            target,
            state: AttachmentState::Standing,
            detail,
            since: now.to_string(),
            changed_at: now.to_string(),
        },
        false,
    )
}

/// The pull request numbers a row's `pull_requests` names.
pub(crate) fn pull_requests_named(joined: Option<&String>) -> BTreeSet<u64> {
    joined
        .map(|joined| joined.split(',').filter_map(|one| one.trim().parse().ok()).collect())
        .unwrap_or_default()
}

pub(crate) fn pull_requests_joined<'a>(numbers: impl IntoIterator<Item = &'a u64>) -> String {
    numbers.into_iter().map(u64::to_string).collect::<Vec<_>>().join(",")
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
    /// The issues the person named to `session` in `text` and its `tags`, held by it.
    ///
    /// **Held, never raised**, as the pull request ledger is: the message is the person's, and a row
    /// that would not write is no reason to refuse it.
    pub(crate) async fn issues_named(&self, session: &str, text: &str, tags: &[SessionTag]) {
        let Some(manifest) = self.store().lock().await.session(session).ok().flatten().and_then(|one| one.manifest_id)
        else {
            return;
        };
        let mut named: BTreeMap<IssueReference, BTreeMap<String, String>> = BTreeMap::new();
        for tag in tags.iter().filter(|tag| tag.kind == TagKind::Issue) {
            let Some(issue) = IssueReference::read(&tag.id) else {
                continue;
            };
            let known = named.entry(issue).or_default();
            if let Some(url) = tag.url.as_ref().filter(|url| !url.trim().is_empty()) {
                known.insert("address".to_string(), url.clone());
            }
        }
        // The tracker the Manifest chose says which of its own issues the text links, so a Jira
        // browse link names its key and a forge link into a Jira repository names nothing.
        if let Ok(served) = self.served_named(Some(&ManifestId::carried(&manifest))) {
            for written in self.tracker_of(&served).issues_linked_in(served.root(), text) {
                if let Some(issue) = IssueReference::read(&written) {
                    named.entry(issue).or_default();
                }
            }
        }
        self.issues_held(session, &manifest, named).await;
    }

    /// An issue the mod read off what `gh issue create` printed, held by `holder` **only where its
    /// address is in the Session's own repository**: one filed elsewhere is not this Session's to work.
    pub(crate) fn issue_filed(
        &self,
        store: &mut Store,
        holder: &Holder,
        manifest_id: &str,
        filed: &ipc::AttachmentReport,
        now: &str,
    ) -> Result<bool, WriteError> {
        let address = filed.detail.get("address").cloned().unwrap_or_default();
        let own = filed.target.trim().parse::<u64>().ok().filter(|number| {
            self.own_repository(manifest_id)
                .is_some_and(|repository| adapters::issues_linked_in(&address, &repository).contains(number))
        });
        let Some(number) = own else {
            return Ok(false);
        };
        let known = BTreeMap::from([("address".to_string(), address)]);
        issue_held(store, holder, manifest_id, &IssueReference::number(number), &known, now)
    }

    /// `claim_issue`: the caller, placed as `claim_pull_request` places it, holds the issue it names,
    /// with the pull requests that work it. **Nothing is asked of a tracker Fleet only reads**: the
    /// pull reading reads it. In one Fleet writes to, the issue is claimed there first, and a claim
    /// the synced copy gives to someone else is refused naming them.
    pub(crate) async fn issue_claimed(&self, caller: Option<Caller>, claim: ClaimIssue) -> Result<IssueClaimed, Refusal> {
        let refuse = |why: String| self.hosted_refusal(NOT_CLAIMABLE, &why);
        let issue = IssueReference::read(&claim.issue).ok_or_else(|| {
            refuse(format!(
                "`{}` is neither an issue number such as `7` nor a tracker's key such as `ABC-12`",
                claim.issue.trim()
            ))
        })?;
        let id = caller
            .as_ref()
            .and_then(|caller| self.session_holding(caller))
            .or_else(|| {
                claim.session_id.as_ref().map(|id| id.as_str().trim().to_string()).filter(|id| !id.is_empty())
            })
            .ok_or_else(|| refuse("an issue is claimed by a job, and this call is not one".into()))?;
        let manifest = self
            .store()
            .lock()
            .await
            .session(&id)
            .map_err(|why| self.ledger_fault(why))?
            .and_then(|session| session.manifest_id)
            .ok_or_else(|| refuse(format!("job {id} is in no repository Fleet serves")))?;
        let claimed_as = self.claimed_where_written(&manifest, &issue, &id).await?;
        let mut store = self.store().lock().await;
        let holder = Holder::session(&id);
        let mut known = BTreeMap::from([(CLAIMED.to_string(), "true".to_string())]);
        if let Some(actor) = claimed_as {
            known.insert(CLAIMED_AS.to_string(), actor);
            // Kept past the Job's end, so its pull request's merge still closes the issue.
            let _ = store.keep_job_issue(&id, &manifest, issue.as_str());
        }
        if !claim.pull_requests.is_empty() {
            known.insert(PULL_REQUESTS.to_string(), pull_requests_joined(&claim.pull_requests.iter().copied().collect::<BTreeSet<_>>()));
        }
        let now = self.now().as_str().to_string();
        let changed = issue_held(&mut store, &holder, &manifest, &issue, &known, &now).map_err(|why| self.ledger_fault(why))?;
        let pull_requests = store
            .attachments_at(ISSUE, issue.as_str(), Some(&manifest))
            .map_err(|why| self.ledger_fault(why))?
            .into_iter()
            .find(|row| row.holder == holder)
            .map(|row| pull_requests_named(row.detail.get(PULL_REQUESTS)).into_iter().collect())
            .unwrap_or_default();
        let record = match changed {
            true => store.session(&id).ok().flatten().and_then(|session| self.ledger_row(&store, &session).ok()),
            false => None,
        };
        drop(store);
        if let Some(record) = record {
            self.publish(ipc::Event::SessionChanged(record));
        }
        Ok(IssueClaimed { issue: issue.as_str().to_string(), holder_id: id, pull_requests })
    }

    /// Claim `issue` for Job `id` in the backlog of `manifest`'s repository, where its tracker is one
    /// Fleet writes to, and who it was claimed as. Runs a sync or two, so it holds no lock on the ledger.
    async fn claimed_where_written(&self, manifest: &str, issue: &IssueReference, id: &str) -> Result<Option<String>, Refusal> {
        let served = self.served_named(Some(&ManifestId::carried(manifest)))?;
        let Some(backlog) = self.backlog_of(&served) else {
            return Ok(None);
        };
        let actor = self.claim_actor(id).await;
        match self.claimed_in_backlog(&served, backlog, issue.as_str(), &actor).await {
            Ok(()) => Ok(Some(actor)),
            Err(ClaimLost::Held { holder }) => Err(Refusal::IllegalMove(WireError::raised(
                HELD_ELSEWHERE,
                format!("{issue} is held by {holder}, as the backlog reads after a sync. Pick another issue"),
                self.run_id(),
            ))),
            Err(ClaimLost::Unsettled(why)) => Err(self.hosted_refusal(
                NOT_CLAIMABLE,
                &format!("{issue} was not claimed: {} did not happen: {}. Claiming it again is safe", why.doing, why.said),
            )),
        }
    }

    /// Where the repository `manifest_id` serves lives on its forge: the only one whose issues a
    /// Session in it takes from a link.
    pub(crate) fn own_repository(&self, manifest_id: &str) -> Option<ForgeAddress> {
        let served = self.served_named(Some(&ManifestId::carried(manifest_id))).ok()?;
        adapters::remote_of(Path::new(served.root()))
    }

    /// Stand each issue in `named` on `session`'s ledger, and publish the Session where any moved.
    pub(crate) async fn issues_held(
        &self,
        session: &str,
        manifest_id: &str,
        named: BTreeMap<IssueReference, BTreeMap<String, String>>,
    ) {
        if named.is_empty() {
            return;
        }
        let now = self.clock().now().as_str().to_string();
        let holder = Holder::session(session);
        let record = {
            let mut store = self.store().lock().await;
            let mut changed = false;
            for (issue, known) in &named {
                changed |= issue_held(&mut store, &holder, manifest_id, issue, known, &now).unwrap_or(false);
            }
            if !changed {
                return;
            }
            store
                .session(session)
                .ok()
                .flatten()
                .and_then(|one| self.ledger_row(&store, &one).ok())
        };
        if let Some(record) = record {
            self.publish(ipc::Event::SessionChanged(record));
        }
    }

    /// Every issue a Session holds in this repository, read once and followed: its title, its state
    /// and its address as the Manifest's tracker has them now, a number or a key alike. The pull reading's own pass, as
    /// [`pull_ledger_kept_current`](Self::pull_ledger_kept_current) is for pull requests, and at the
    /// same utility priority. **A closed issue stands**: the row says closed and is still shown.
    /// Issues the forge will not answer for stay as they were.
    pub(crate) async fn issue_ledger_kept_current(&self, manifest_id: &ManifestId, root: &str) {
        let mut issues: Vec<IssueReference> = {
            let store = self.store().lock().await;
            let Ok(rows) = store.standing_of_kind(ISSUE, manifest_id.as_str()) else {
                return;
            };
            rows.iter()
                .filter(|row| row.holder.kind == HolderKind::Session)
                .filter_map(|row| IssueReference::read(&row.target))
                .collect()
        };
        issues.sort_unstable();
        issues.dedup();
        let Ok(served) = self.served_named(Some(manifest_id)) else {
            return;
        };
        for issue in issues {
            let (tracker, root, reference) = (self.tracker_of(&served), root.to_string(), issue.as_str().to_string());
            let read = tokio::task::spawn_blocking(move || tracker.read_issue(&root, &reference)).await;
            let Ok(Ok(Some(read))) = read else {
                continue;
            };
            self.lifecycle_ticket_seen(manifest_id, issue.as_str(), &read).await;
            let fresh = [
                ("title", read.title.as_written()),
                ("state", if read.closed { "closed" } else { "open" }),
                ("address", read.url.as_written()),
            ];
            self.issue_follows(manifest_id.as_str(), &issue, &fresh).await;
        }
    }

    /// Every Session's standing row for `issue` takes `fresh` over what it said, in one write.
    async fn issue_follows(&self, manifest_id: &str, issue: &IssueReference, fresh: &[(&str, &str)]) {
        let now = self.clock().now().as_str().to_string();
        let mut changed = Vec::new();
        {
            let mut store = self.store().lock().await;
            let Ok(rows) = store.attachments_at(ISSUE, issue.as_str(), Some(manifest_id)) else {
                return;
            };
            for row in rows
                .iter()
                .filter(|row| row.holder.kind == HolderKind::Session && row.state == AttachmentState::Standing)
            {
                let mut detail = row.detail.clone();
                for (key, value) in fresh.iter().filter(|(_, value)| !value.trim().is_empty()) {
                    detail.insert(key.to_string(), value.to_string());
                }
                if !matches!(store.follow(row, AttachmentState::Standing, &detail, &now), Ok(true)) {
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
}
