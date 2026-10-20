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
use ipc::{ManifestId, SessionTag, TagKind};
use store::{AttachmentState, Holder, KeptAttachment, Store, WriteError};

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
        let repository = self
            .served_named(Some(&ManifestId::carried(&manifest)))
            .ok()
            .and_then(|served| adapters::remote_of(Path::new(served.root())));
        if let Some(repository) = repository {
            for number in adapters::issues_linked_in(text, &repository) {
                named.entry(number).or_default();
            }
        }
        self.issues_held(session, &manifest, named).await;
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
}
