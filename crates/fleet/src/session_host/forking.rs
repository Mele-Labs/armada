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
//!
//! **A move is a fork across repositories.** [`Fleet::moved_session`] ends the old session and forks
//! its whole conversation into another served repository; the two `forked_*` rows carry
//! `detail.moved`, and the fork is told [`move_note`] in place of [`fork_note`].

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use std::sync::Arc;

use api::{HostedSessions, Refusal};
use ipc::{CloseSession, ForkFrom, MoveSession, SessionRecord, StartSession, WireError};
use store::{AttachmentState, Holder, KeptAttachment, KeptHosting, KeptSession};

use super::rows::new_id;
use super::serving::{mode_text, NO_SUCH_SESSION};
use crate::daemon::Fleet;

/// A fork of a session that is still running. A 409.
const FORK_LIVE: &str = "fleet.session_fork_live";

/// A move to the repository the session already works in. A 409.
const SESSION_MOVE_SAME: &str = "fleet.session_move_same";
/// A move of a terminal session whose mod is still asking: its process is in the owner's terminal. A 409.
const SESSION_MOVE_LISTENING: &str = "fleet.session_move_listening";
/// A move to a repository Fleet does not serve. A 404.
const NO_SUCH_MANIFEST: &str = "fleet.no_such_manifest";

/// On the old session's ledger, naming the fork.
pub(crate) const FORKED_TO: &str = "forked_to";
/// On the fork's ledger, naming the session it came from.
pub(crate) const FORKED_FROM: &str = "forked_from";
/// On both `forked_*` rows of a move, which a fork has not: the id of the repository on the other
/// side, the one the session came from on `forked_from` and the one it went to on `forked_to`.
pub(crate) const MOVED: &str = "moved";
/// On a move's `forked_from` detail: the old session's branch or branches, comma-separated.
const BRANCH: &str = "branch";
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
        "This job is a fork. It was forked from {title}({old}) at {when}, and that job \
         {state}. Your conversation up to the fork is a copy of its, and nothing it started is \
         yours: its subagents, background tasks and shells, slots, branches, worktrees, \
         windows, pull requests and peer jobs stay its own, whatever your copy of the \
         conversation says about them. A task-notification about its background work \
         (finished, failed, stopped, \"didn't finish before the previous job ended\") \
         belongs to it: do not act on it, relaunch that work, or report it to the owner as \
         yours. Do not write in its slots. Your own work starts with your first write, in a \
         slot leased for you."
    )
}

/// What a moved session's agent is told it is: bare facts, and what stayed behind. `from` and `to`
/// are the repositories' roots, `branch` what the old session's branch or branches were.
pub(crate) fn move_note(
    old_title: Option<&str>,
    old: &str,
    when: &str,
    from: &str,
    to: &str,
    branch: Option<&str>,
) -> String {
    let title = old_title
        .map(|title| title.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|title| !title.is_empty())
        .map(|title| format!("\"{title}\" "))
        .unwrap_or_default();
    let branch = branch.map(|branch| format!("its branch ({branch}), ")).unwrap_or_default();
    format!(
        "This job was moved from repository {from} to repository {to} at {when}. Your \
         conversation up to the move is a copy of {title}({old}), which has ended. You now work \
         in {to}. Its slot, {branch}and the files it had changed stayed in {from}, and none of \
         them are yours. Nothing it started is yours: its subagents, background tasks and \
         shells, windows, pull requests and peer jobs stay its own, whatever your copy of \
         the conversation says about them. A task-notification about its background work \
         (finished, failed, stopped, \"didn't finish before the previous job ended\") \
         belongs to it: do not act on it, relaunch that work, or report it to the owner as \
         yours. Do not write in {from}. Paths in the conversation before the move name {from}. \
         Your own work starts with your first write, in a slot leased for you in {to}."
    )
}

/// What a move carries to the fork it starts.
struct Moved {
    /// The repository the old session was in.
    from: String,
    /// The old session's branch or branches there, comma-separated.
    branch: Option<String>,
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
        let held = store.attachments_of(&Holder::session(id)).ok()?;
        let link = held.into_iter().find(|one| one.kind == FORKED_FROM)?;
        // An old session the ledger has forgotten is one that has ended.
        let old_session = store.session(old).ok().flatten();
        let title = old_session.as_ref().and_then(|one| one.title.as_deref());
        if let Some(from) = link.detail.get(MOVED) {
            // A repository no longer served is named by its id.
            let named = |manifest: &str| {
                self.repositories()
                    .serving(manifest)
                    .map_or_else(|| manifest.to_string(), |served| served.root().to_string())
            };
            return Some(move_note(
                title,
                old,
                &link.since,
                &named(from),
                &named(&link.manifest_id),
                link.detail.get(BRANCH).map(String::as_str),
            ));
        }
        Some(fork_note(
            title,
            old,
            &link.since,
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
        self.forked(start, from, None).await
    }

    /// Ends session `moved.session_id` if it is live and starts its whole conversation again in
    /// `moved.manifest_id`, under the same title. Every refusal comes before anything is ended.
    pub(crate) async fn moved_session(self: Arc<Self>, moved: MoveSession) -> Result<SessionRecord, Refusal> {
        let old_id = moved.session_id.as_str().to_string();
        let to = moved.manifest_id.as_str().to_string();
        let (session, hosting, branches) = {
            let store = self.store().lock().await;
            let session = store
                .session(&old_id)
                .map_err(|why| self.ledger_fault(why))?
                .ok_or_else(|| {
                    Refusal::NotFound(WireError::raised(
                        NO_SUCH_SESSION,
                        format!("no job is named {old_id}"),
                        self.run_id(),
                    ))
                })?;
            let hosting = store.hosting(&old_id).map_err(|why| self.ledger_fault(why))?;
            let held = store
                .attachments_of(&Holder::session(&old_id))
                .map_err(|why| self.ledger_fault(why))?;
            (session, hosting, held)
        };
        if self.repositories().serving(&to).is_none() {
            return Err(Refusal::NotFound(WireError::raised(
                NO_SUCH_MANIFEST,
                format!("no repository is served as {to}: held are `{}`", self.held_manifests()),
                self.run_id(),
            )));
        }
        let from = hosting
            .as_ref()
            .map(|hosting| hosting.manifest_id.clone())
            .or(session.manifest_id.clone());
        if from.as_deref() == Some(to.as_str()) {
            return Err(Refusal::IllegalMove(WireError::raised(
                SESSION_MOVE_SAME,
                format!("job {old_id} already works in {to}"),
                self.run_id(),
            )));
        }
        let live = session.state != store::SessionState::Ended;
        if live && hosting.is_none() && self.hosts().terminals().listening(&old_id) {
            return Err(Refusal::IllegalMove(WireError::raised(
                SESSION_MOVE_LISTENING,
                format!("job {old_id} runs in a terminal, which Fleet cannot move"),
                self.run_id(),
            )));
        }
        // Read before the close gives the slot back.
        let branch = branches
            .iter()
            .filter(|one| {
                one.kind == BRANCH
                    && one.state == AttachmentState::Standing
                    && Some(one.manifest_id.as_str()) == from.as_deref()
            })
            .map(|one| one.target.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        if live && hosting.is_some() {
            Arc::clone(&self).close_session(CloseSession { session_id: moved.session_id.clone() }).await?;
        }
        let start = StartSession {
            manifest_id: moved.manifest_id,
            title: None,
            model: hosting.as_ref().and_then(|hosting| hosting.model.clone()),
            effort: hosting.as_ref().and_then(|hosting| hosting.effort.clone()),
            mode: hosting.as_ref().map(|hosting| super::serving::mode_of(&hosting.mode)),
            root: None,
            fork: None,
            request: None,
            after: None,
        };
        let whole = ForkFrom { session_id: moved.session_id, at: None };
        let moved = Moved {
            from: from.unwrap_or_default(),
            branch: Some(branch).filter(|branch| !branch.is_empty()),
        };
        self.forked(start, whole, Some(moved)).await
    }

    async fn forked(
        &self,
        start: StartSession,
        from: ForkFrom,
        moved: Option<Moved>,
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
                    self.hosted_refusal(NO_SUCH_SESSION, &format!("no job is named {old_id}"))
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
                    format!("job {old_id} is still running, and only an ended one is forked"),
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
                    workflow: None,
                    proposed: String::new(),
                    turn_model: None,
                    turn_effort: None,
                    paused_at: None,
                    paused_resuming: false,
                    auto_land: false,
                    mod_job: false,
                })
                .map_err(|why| self.ledger_fault(why))?;
            // `spent`, not `standing`: the old session ending gives back what
            // it holds, and the link is a fact that stays.
            for (holder, kind, target) in [(&id, FORKED_FROM, &old_id), (&old_id, FORKED_TO, &id)] {
                let mut detail: std::collections::BTreeMap<String, String> = match (&from.at, &cut) {
                    (Some(at), Some(cut)) if kind == FORKED_FROM => {
                        [(AT.to_string(), at.clone()), (CUT.to_string(), cut.clone())].into()
                    }
                    _ => Default::default(),
                };
                if let Some(moved) = &moved {
                    // Each ledger names the repository on the other side.
                    let other = if kind == FORKED_FROM { &moved.from } else { &manifest };
                    detail.insert(MOVED.to_string(), other.clone());
                    if let (FORKED_FROM, Some(branch)) = (kind, &moved.branch) {
                        detail.insert(BRANCH.to_string(), branch.clone());
                    }
                }
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
