//! The hook a hosted session runs before a write: **the lease is taken on the
//! first one, and until then nothing can write the main checkout**. Since 23.47.
//!
//! The process's directory is fixed when it starts, so the lease cannot move it
//! in place (spike 24 measured the built-in worktree tool refusing a permission
//! door). The hook leases the slot, denies the write, and the turn ends;
//! `hearing::moved_into` then resumes the session with the slot as its
//! directory. **A line the hook cannot read as a read is a write**, so a shell
//! line it does not know leases a slot rather than touching the checkout.

use adapter_traits::{AgentHarness, Delivery, SlotLeased, Vcs, WorkProduct, WorktreeSpec};
use ipc::{GateAnswer, ManifestId, SessionGate, SessionMode, SessionRow};
use store::{AttachmentState, Holder, KeptAttachment};

use super::serving::mode_of;
use super::Move;
use crate::daemon::Fleet;
use crate::repositories::Served;

/// What a call does to the checkout, as far as its name and input say.
enum Touch {
    Nothing,
    File(String),
    Shell,
}

fn touch(gate: &SessionGate) -> Touch {
    let text = |key: &str| gate.tool_input.get(key).and_then(|said| said.as_str());
    match gate.tool_name.as_str() {
        "Write" | "Edit" | "NotebookEdit" => match text("file_path").or(text("notebook_path")) {
            Some(path) => Touch::File(path.to_string()),
            None => Touch::Nothing,
        },
        "Bash" => match text("command") {
            Some(command) if adapters::reads_only(command) => Touch::Nothing,
            _ => Touch::Shell,
        },
        _ => Touch::Nothing,
    }
}

fn inside(path: &str, directory: &str) -> bool {
    let directory = directory.trim_end_matches('/');
    path == directory || path.starts_with(&format!("{directory}/"))
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
    pub(crate) async fn gated(&self, gate: SessionGate) -> GateAnswer {
        let id = gate.session_id.clone();
        let Ok((session, hosting)) = self.session_and_hosting(&id).await else {
            return GateAnswer::pass();
        };
        if session.state == store::SessionState::Ended
            || mode_of(&hosting.mode) == SessionMode::Plan
        {
            return GateAnswer::pass();
        }
        let Ok(served) = self.served_named(Some(&ManifestId::carried(&hosting.manifest_id))) else {
            return GateAnswer::pass();
        };
        let root = served.root().trim_end_matches('/').to_string();
        let touching = touch(&gate);
        let in_the_repository = |path: &str| !path.starts_with('/') || inside(path, &root);
        match (&touching, hosting.lease_slot) {
            (Touch::Nothing, _) => GateAnswer::pass(),
            (Touch::File(path), None) if !in_the_repository(path) => GateAnswer::pass(),
            (_, None) => self.leased(&id, &served).await,
            (touching, Some(slot)) => {
                let directory = Self::directory_of(&served, &hosting);
                if self.hosts().of(&id).state().moving.is_some() {
                    return GateAnswer::deny(waiting_to_move(slot, &directory));
                }
                match touching {
                    Touch::File(path) if path.starts_with('/') && inside(path, &root) => {
                        if inside(path, &directory) {
                            GateAnswer::pass()
                        } else {
                            GateAnswer::deny(format!(
                                "{path} is in the main checkout. This session writes in its own \
                                 slot at {directory}: use the same path under it."
                            ))
                        }
                    }
                    _ => GateAnswer::pass(),
                }
            }
        }
    }

    /// Lease a slot for the session and cut its branch. **One at a time**, so
    /// two writes in one turn lease one slot.
    async fn leased(&self, id: &str, served: &Served) -> GateAnswer {
        let runtime = self.hosts().of(id);
        let _one_at_a_time = runtime.lease.lock().await;
        let Ok((session, mut hosting)) = self.session_and_hosting(id).await else {
            return GateAnswer::pass();
        };
        if let Some(slot) = hosting.lease_slot {
            return GateAnswer::deny(waiting_to_move(slot, &Self::directory_of(served, &hosting)));
        }
        let spec = match WorktreeSpec::for_job(served.root(), &format!("session-{}", &id[..8])) {
            Ok(spec) => spec,
            Err(why) => return GateAnswer::deny(format!("no slot could be named: {why:?}")),
        };
        let pool = crate::leasing::pool_of(served);
        let (slot, directory, branch) =
            match self.vcs().lease_slot(&pool, &spec, id) {
                Ok(SlotLeased::Took { slot, worktree, .. }) => (
                    slot,
                    worktree.path().to_string(),
                    worktree.branch().to_string(),
                ),
                Ok(SlotLeased::Full) => return GateAnswer::deny(
                    "Every worktree slot is in use, so this session cannot write yet. Tell the \
                     person, and try again once one is free.",
                ),
                Err(why) => {
                    return GateAnswer::deny(format!(
                        "A slot could not be leased for this write: {why}"
                    ))
                }
            };
        hosting.lease_slot = Some(slot);
        hosting.lease_branch = Some(branch.clone());
        let now = self.now().as_str().to_string();
        {
            let mut store = self.store().lock().await;
            let _ = store.keep_hosting(&hosting);
            let manifest = hosting.manifest_id.clone();
            for (kind, target) in [("slot", slot.to_string()), ("branch", branch.clone())] {
                let _ = store.attach(
                    &KeptAttachment {
                        holder: Holder::session(id),
                        kind: kind.into(),
                        manifest_id: manifest.clone(),
                        target,
                        state: AttachmentState::Standing,
                        detail: Default::default(),
                        since: now.clone(),
                        changed_at: now.clone(),
                    },
                    true,
                );
            }
            let mut row = session;
            row.cwd = directory.clone();
            row.last_seen_at = now;
            let _ = store.keep_session(&row);
        }
        runtime.state().moving = Some(Move {
            slot,
            branch: branch.clone(),
            directory: directory.clone(),
        });
        self.row_put(
            id,
            SessionRow::Lease {
                id: self.row_id(id),
                at: self.instant(),
                slot,
                branch: branch.clone(),
            },
        )
        .await;
        let _ = self.published_hosted(id).await;
        GateAnswer::deny(format!(
            "This session writes only inside a worktree leased for it, and it had none until \
             this write. Slot {slot} is leased on branch {branch} at {directory}, and Fleet is \
             moving the session there now. Stop here: make no more tool calls and write nothing \
             more this turn. You will be told when you are in the slot, and then repeat this \
             write there."
        ))
    }
}

fn waiting_to_move(slot: u32, directory: &str) -> String {
    format!(
        "Slot {slot} is leased for this session at {directory}, and Fleet is moving the session \
         there. Make no more writes this turn: you will be told when you are in the slot."
    )
}
