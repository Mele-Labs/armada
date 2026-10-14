//! A session started as a copy of an ended or dead one's conversation, or of
//! any one's up to a message. Since 23.69, 23.79. `docs/concepts/session.md`,
//! *A forked session*.
//!
//! **The fork is a new session.** It has its own id and ledger, holds no slot
//! and no branch of the old one's, and takes a slot on its first write like any
//! other start. What it takes from the old one is the conversation, which the
//! agent copies when its first process starts, and the title.
//!
//! **The agent is told so, too.** The copy carries the old session's subagents, background
//! tasks and slots in its words, and the CLI replays their notifications into it, so the fork
//! would take them for its own. Every process of a fork has [`fork_note`] in its system prompt.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{ForkFrom, SessionRecord, StartSession, WireError};
use store::{AttachmentState, Holder, KeptAttachment, KeptHosting, KeptSession};

use super::rows::new_id;
use super::serving::{mode_text, NO_SUCH_SESSION};
use crate::daemon::Fleet;

/// A fork of a session that is still running. A 409.
const FORK_LIVE: &str = "fleet.session_fork_live";

/// On the old session's ledger, naming the fork.
pub(crate) const FORKED_TO: &str = "forked_to";
/// On the fork's ledger, naming the session it came from.
pub(crate) const FORKED_FROM: &str = "forked_from";
/// On `forked_from`'s detail: the thread row the fork ends at, which Bridge cuts the old rows by.
const AT: &str = "at";
/// On `forked_from`'s detail: the transcript line that row is, which the first process is told.
const CUT: &str = "cut";

/// What a fork's agent is told it is: bare facts, and what to do with the old session's work.
/// `old_title` is the title the old session had, `ended` whether it had ended when this was read.
pub(crate) fn fork_note(old_title: Option<&str>, old: &str, when: &str, ended: bool) -> String {
    let title = old_title
        .map(|title| title.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|title| !title.is_empty())
        .map(|title| format!("\"{title}\" "))
        .unwrap_or_default();
    let state = if ended { "has ended" } else { "is live and keeps working" };
    format!(
        "This session is a fork. It was forked from {title}({old}) at {when}, and that session \
         {state}. Your conversation up to the fork is a copy of its, and nothing it started is \
         yours: its subagents, background tasks and shells, slots, branches, worktrees, \
         windows, pull requests and peer sessions stay its own, whatever your copy of the \
         conversation says about them. A task-notification about its background work \
         (finished, failed, stopped, \"didn't finish before the previous session ended\") \
         belongs to it: do not act on it, relaunch that work, or report it to the owner as \
         yours. Do not write in its slots. Your own work starts with your first write, in a \
         slot leased for you."
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
    /// The transcript line a fork's first process ends its copy at, where the fork was taken at a
    /// message. Kept on the ledger, so a Fleet that restarted before that process started has it.
    pub(crate) async fn fork_cut_of(&self, id: &str) -> Option<String> {
        let store = self.store().lock().await;
        let held = store.attachments_of(&Holder::session(id)).ok()?;
        held.into_iter()
            .find(|one| one.kind == FORKED_FROM)
            .and_then(|one| one.detail.get(CUT).cloned())
    }

    /// What every process of fork `id` is told it is, of the session `old` it came from. Read at
    /// each start, so whether the old session is live is as of then.
    pub(crate) async fn fork_note_of(&self, id: &str, old: &str) -> Option<String> {
        let store = self.store().lock().await;
        let when = store
            .attachments_of(&Holder::session(id))
            .ok()?
            .into_iter()
            .find(|one| one.kind == FORKED_FROM)
            .map(|one| one.since)?;
        // An old session the ledger has forgotten is one that has ended.
        let old_session = store.session(old).ok().flatten();
        Some(fork_note(
            old_session.as_ref().and_then(|one| one.title.as_deref()),
            old,
            &when,
            old_session
                .as_ref()
                .is_none_or(|one| one.state == store::SessionState::Ended),
        ))
    }

    pub(crate) async fn start_forked(
        &self,
        start: StartSession,
        from: ForkFrom,
    ) -> Result<SessionRecord, Refusal> {
        let old_id = from.session_id.as_str().to_string();
        let served = self.served_named(Some(&start.manifest_id))?;
        // **Before anything is written**: a message that has no line leaves no session behind.
        let cut = match &from.at {
            Some(at) => Some(self.fork_cut(&old_id, at).await?),
            None => None,
        };
        let manifest = served.manifest().id().as_str().to_string();
        let id = new_id();
        let now = self.now().as_str().to_string();
        {
            let mut store = self.store().lock().await;
            let old = store
                .session(&old_id)
                .map_err(|why| self.ledger_fault(why))?
                .ok_or_else(|| {
                    self.hosted_refusal(NO_SUCH_SESSION, &format!("no session is named {old_id}"))
                })?;
            let hosted = store
                .hosting(&old_id)
                .map_err(|why| self.ledger_fault(why))?
                .is_some();
            // Ended, or a terminal session whose mod stopped asking. A hosted
            // session that is not closed is live whatever its process does.
            let dead = old.state == store::SessionState::Ended
                || (!hosted && !self.hosts().terminals().listening(&old_id));
            // A fork at a message copies what is written and takes nothing from the old session,
            // so it may be live.
            if !dead && cut.is_none() {
                return Err(Refusal::IllegalMove(WireError::raised(
                    FORK_LIVE,
                    format!("session {old_id} is still running, and only an ended one is forked"),
                    self.run_id(),
                )));
            }
            let title = start
                .title
                .map(|title| title.trim().to_string())
                .filter(|title| !title.is_empty())
                .or(old.title.clone());
            store
                .keep_session(&KeptSession {
                    id: id.clone(),
                    harness: String::from(adapters::HOSTED_HARNESS),
                    origin: String::from("bridge"),
                    manifest_id: Some(manifest.clone()),
                    cwd: served.root().to_string(),
                    title,
                    state: store::SessionState::Live,
                    started_at: now.clone(),
                    last_seen_at: now.clone(),
                    last_turn_at: None,
                    ended_at: None,
                    end_reason: None,
                    figures: Default::default(),
                    mod_version: None,
                })
                .map_err(|why| self.ledger_fault(why))?;
            store
                .keep_hosting(&KeptHosting {
                    session_id: id.clone(),
                    manifest_id: manifest.clone(),
                    model: start.model.filter(|model| !model.trim().is_empty()),
                    effort: start.effort.filter(|effort| !effort.trim().is_empty()),
                    mode: mode_text(start.mode.unwrap_or_default()).to_string(),
                    ran: false,
                    lease_slot: None,
                    lease_branch: None,
                    fork_of: Some(old_id.clone()),
                    // A fork of Helm is an ordinary Session: Helm's note is not in the copied conversation.
                    helm: false,
                    setup: false,
                })
                .map_err(|why| self.ledger_fault(why))?;
            // `spent`, not `standing`: the old session ending gives back what
            // it holds, and the link is a fact that stays.
            for (holder, kind, target) in [(&id, FORKED_FROM, &old_id), (&old_id, FORKED_TO, &id)] {
                let detail = match (&from.at, &cut) {
                    (Some(at), Some(cut)) if kind == FORKED_FROM => {
                        [(AT.to_string(), at.clone()), (CUT.to_string(), cut.clone())].into()
                    }
                    _ => Default::default(),
                };
                store
                    .attach(
                        &KeptAttachment {
                            holder: Holder::session(holder),
                            kind: kind.to_string(),
                            manifest_id: manifest.clone(),
                            target: target.clone(),
                            state: AttachmentState::Spent,
                            detail,
                            since: now.clone(),
                            changed_at: now.clone(),
                        },
                        false,
                    )
                    .map_err(|why| self.ledger_fault(why))?;
            }
        }
        let _ = self.published_hosted(&old_id).await;
        self.published_hosted(&id).await
    }
}
