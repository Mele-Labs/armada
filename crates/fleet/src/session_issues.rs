//! The issues a Session works on, as `issue` rows on the ledger, so Bridge can list them with the
//! Session's pull requests under each. `docs/concepts/session/what-is-kept.md`.
//!
//! **Named by the person, never guessed from prose**: an issue tag, or a link into the Session's
//! own repository, in a message of the person's or the request the Session was dispatched with. A
//! bare `#123` is a number in a sentence.
//!
//! **One row a Session and issue, and what it already says stands.** A second naming takes it up
//! again where it was given back and fills only what it lacked: the forge's reading, which the pull
//! watch writes, is never overwritten by a tag's address.

use std::collections::BTreeMap;
use std::path::Path;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use adapters::ForgeAddress;
use ipc::{ManifestId, SessionTag, TagKind};
use store::{AttachmentState, Holder, HolderKind, KeptAttachment, Store, WriteError};

use crate::daemon::Fleet;

/// The ledger's kind for an issue a Session works on. `target` is its number.
pub(crate) const ISSUE: &str = "issue";

/// Stand `holder`'s row for issue `number`, with `known` where the row says nothing yet. `true`
/// where anything changed.
pub(crate) fn issue_held(
    store: &mut Store,
    holder: &Holder,
    manifest_id: &str,
    number: u64,
    known: &BTreeMap<String, String>,
    now: &str,
) -> Result<bool, WriteError> {
    let target = number.to_string();
    let mut detail = known.clone();
    if let Some(row) = store
        .attachments_at(ISSUE, &target, Some(manifest_id))?
        .into_iter()
        .find(|row| &row.holder == holder)
    {
        detail.extend(row.detail);
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
        let mut named: BTreeMap<u64, BTreeMap<String, String>> = BTreeMap::new();
        for tag in tags.iter().filter(|tag| tag.kind == TagKind::Issue) {
            // A tracker that is not the forge names its issues by key, which is no number here.
            let Ok(number) = tag.id.trim().trim_start_matches('#').parse::<u64>() else {
                continue;
            };
            let known = named.entry(number).or_default();
            if let Some(url) = tag.url.as_ref().filter(|url| !url.trim().is_empty()) {
                known.insert("address".to_string(), url.clone());
            }
        }
        if let Some(repository) = self.own_repository(&manifest) {
            for number in adapters::issues_linked_in(text, &repository) {
                named.entry(number).or_default();
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
        issue_held(store, holder, manifest_id, number, &BTreeMap::from([("address".to_string(), address)]), now)
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
        named: BTreeMap<u64, BTreeMap<String, String>>,
    ) {
        if named.is_empty() {
            return;
        }
        let now = self.clock().now().as_str().to_string();
        let holder = Holder::session(session);
        let record = {
            let mut store = self.store().lock().await;
            let mut changed = false;
            for (number, known) in &named {
                changed |= issue_held(&mut store, &holder, manifest_id, *number, known, &now).unwrap_or(false);
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
    /// and its address as the forge has them now. The pull watch's own pass, as
    /// [`pull_ledger_kept_current`](Self::pull_ledger_kept_current) is for pull requests, and at the
    /// same utility priority. **A closed issue stands**: the row says closed and is still shown.
    /// Issues the forge will not answer for stay as they were.
    pub(crate) async fn issue_ledger_kept_current(&self, manifest_id: &ManifestId, root: &str) {
        let mut numbers: Vec<u64> = {
            let store = self.store().lock().await;
            let Ok(rows) = store.standing_of_kind(ISSUE, manifest_id.as_str()) else {
                return;
            };
            rows.iter()
                .filter(|row| row.holder.kind == HolderKind::Session)
                .filter_map(|row| row.target.parse().ok())
                .collect()
        };
        numbers.sort_unstable();
        numbers.dedup();
        let Ok(served) = self.served_named(Some(manifest_id)) else {
            return;
        };
        for number in numbers {
            let (tracker, root) = (self.tracker_of(&served), root.to_string());
            let read = tokio::task::spawn_blocking(move || tracker.read_issue(&root, &number.to_string())).await;
            let Ok(Ok(Some(issue))) = read else {
                continue;
            };
            let fresh = [
                ("title", issue.title.as_written()),
                ("state", if issue.closed { "closed" } else { "open" }),
                ("address", issue.url.as_written()),
            ];
            self.issue_follows(manifest_id.as_str(), number, &fresh).await;
        }
    }

    /// Every Session's standing row for issue `number` takes `fresh` over what it said, in one write.
    async fn issue_follows(&self, manifest_id: &str, number: u64, fresh: &[(&str, &str)]) {
        let now = self.clock().now().as_str().to_string();
        let mut changed = Vec::new();
        {
            let mut store = self.store().lock().await;
            let Ok(rows) = store.attachments_at(ISSUE, &number.to_string(), Some(manifest_id)) else {
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
