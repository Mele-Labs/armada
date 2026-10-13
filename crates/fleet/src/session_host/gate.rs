//! The hook a hosted session runs before a write: **the lease is taken on the
//! first one, and until then nothing can write the main checkout**. Since 23.49.
//!
//! The process's directory is fixed when it starts, so the lease cannot move it
//! in place (spike 24 measured the built-in worktree tool refusing a permission
//! door). The hook leases the slot, denies the write, and the turn ends;
//! `hearing::moved_into` then resumes the session with the slot as its
//! directory. **A line the hook cannot read as a read is a write**, so a shell
//! line it does not know leases a slot rather than touching the checkout.
//!
//! **A full pool queues the session** (`queueing`) rather than telling it to try
//! again: it is denied, told its place and told to stop, and Fleet leases and moves
//! it when a slot frees.

use adapter_traits::{AgentHarness, Delivery, SlotLeased, Vcs, WorkProduct, WorktreeSpec};
use ipc::{GateAnswer, ManifestId, SessionGate, SessionMode, SessionRow};
use store::{AttachmentState, Holder, KeptAttachment};

use super::following::Standing;
use super::places::{normalised, place_of, run_from, worked_in, written_by, Place};
use super::serving::mode_of;
use super::Move;
use crate::daemon::Fleet;
use crate::repositories::Served;

/// What a call does to the checkout, as far as its name and input say.
enum Touch {
    Nothing,
    File(String),
    Shell(String),
}

fn touch(gate: &SessionGate) -> Touch {
    let text = |key: &str| gate.tool_input.get(key).and_then(|said| said.as_str());
    match gate.tool_name.as_str() {
        "Write" | "Edit" | "MultiEdit" | "NotebookEdit" => match text("file_path").or(text("notebook_path")) {
            Some(path) => Touch::File(path.to_string()),
            None => Touch::Nothing,
        },
        "Bash" => match text("command") {
            Some(command) if adapters::reads_only(command) => Touch::Nothing,
            Some(command) => Touch::Shell(command.to_string()),
            None => Touch::Shell(String::new()),
        },
        _ => Touch::Nothing,
    }
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
        // The pool is read here, so the ledger's slot is the pool's before it decides.
        let hosting = self.follow_the_pool(&id, &served, hosting).await;
        let own = hosting.lease_slot;
        // A slot it came by some other way ends its wait.
        if own.is_some() {
            self.hosts().slot_queue().leave(&id);
        }
        let directory = Self::directory_of(&served, &hosting);
        let held = |slot: u32| {
            own == Some(slot)
                || self.held_by_the_tree(&id, &adapter_traits::slot_path(&root, slot))
                    == Standing::Ours
        };
        // Where the call lands, against the main checkout and the pool. A relative path lands
        // where the call stands: in a slot if it stands in one, and otherwise the session's own
        // directory, which is a slot or the checkout.
        let lands = |path: &str| match path.starts_with('/') {
            true => place_of(path, &root),
            false if gate.cwd.starts_with('/') => {
                let at = normalised(&format!("{}/{path}", gate.cwd.trim_end_matches('/')));
                match place_of(&at, &root) {
                    Place::Slot(slot) => Place::Slot(slot),
                    _ => Place::Main,
                }
            }
            false => Place::Main,
        };
        let touching = touch(&gate);
        if matches!(touching, Touch::Nothing) {
            return GateAnswer::pass();
        }
        if let (Some(slot), true) = (own, self.hosts().of(&id).state().moving.is_some()) {
            return GateAnswer::deny(waiting_to_move(slot, &directory));
        }
        let written = match &touching {
            Touch::File(path) => Some(path.clone()),
            _ => None,
        };
        let answer = match touching {
            Touch::Nothing => GateAnswer::pass(),
            Touch::File(path) => match (lands(&path), own) {
                (Place::Elsewhere, _) => GateAnswer::pass(),
                (Place::Slot(slot), _) if held(slot) => GateAnswer::pass(),
                (Place::Slot(slot), _) => GateAnswer::deny(not_held(&path, slot)),
                (Place::Main, None) => self.leased(&id, &served).await,
                (Place::Main, Some(_)) if path.starts_with('/') => GateAnswer::deny(format!(
                    "{path} is in the main checkout. This session writes in its own slot at \
                     {directory}: use the same path under it."
                )),
                // A relative path is read against the session's own directory.
                (Place::Main, Some(_)) => GateAnswer::pass(),
            },
            Touch::Shell(command) => {
                if own.is_none() {
                    return self.leased(&id, &served).await;
                }
                let hit = written_by(&command, &gate.cwd)
                    .into_iter()
                    .find_map(|path| match lands(&path) {
                        Place::Main => Some(format!(
                            "That command writes {path} in the main checkout. This session writes \
                             in its own slot at {directory}: use the same path under it."
                        )),
                        Place::Slot(slot) if !held(slot) => Some(not_held(&path, slot)),
                        _ => None,
                    });
                // A `cd` moves every relative write after it, so the directory it ends in is judged.
                let hit = hit.or_else(|| {
                    worked_in(&command, &gate.cwd)
                        .into_iter()
                        .find_map(|entered| match lands(&entered) {
                            Place::Main => Some(format!(
                                "That command works in {entered}, in the main checkout. This \
                                 session writes in its own slot at {directory}: work there."
                            )),
                            Place::Slot(slot) if !held(slot) => Some(not_held(&entered, slot)),
                            _ => None,
                        })
                });
                // **A working directory persists between calls**, so a line that runs something
                // without going anywhere is judged by where the call stands.
                let hit = hit.or_else(|| {
                    run_from(&command, &gate.cwd).and_then(|here| match lands(&here) {
                        Place::Slot(slot) if !held(slot) => Some(not_held(&here, slot)),
                        _ => None,
                    })
                });
                match hit {
                    Some(why) => GateAnswer::deny(why),
                    None => GateAnswer::pass(),
                }
            }
        };
        // A write let through inside the session's own slot is a file it has written, for the
        // check of who else writes it. `docs/concepts/fleet.md`, *Write clashes*.
        if let (Some(path), Some(_), GateAnswer::Pass {}) = (written, own, &answer) {
            let inside = crate::edit_calls::repository_relative(&path, &directory, &self.host().home);
            if !inside.starts_with('/') {
                let who = crate::clashing::Who::Session(id.clone());
                self.writes().wrote(&who, served.manifest().id().as_str(), [inside], &self.now());
            }
        }
        answer
    }

    /// A first write into the checkout. **Behind everyone already waiting**, so a slot
    /// freed between two of them is not taken by the newer. When the pool is full the
    /// session is queued, and `queueing` leases it a slot when one frees.
    async fn leased(&self, id: &str, served: &Served) -> GateAnswer {
        let queue = self.hosts().slot_queue();
        let manifest = served.manifest().id().as_str();
        let taken = match queue.ahead_of(manifest, id) {
            0 => self.take_lease(id, served).await,
            _ => Taken::Full,
        };
        match taken {
            Taken::Gone => GateAnswer::pass(),
            Taken::Held { slot, directory } => {
                queue.leave(id);
                GateAnswer::deny(waiting_to_move(slot, &directory))
            }
            Taken::Refused(why) => GateAnswer::deny(why),
            Taken::Full => {
                let ahead = queue.join(manifest, id);
                // A slot may have been given back between the lease and the join.
                queue.freed();
                GateAnswer::deny(queued(ahead))
            }
            Taken::Moving(moving) => {
                queue.leave(id);
                GateAnswer::deny(format!(
                    "This session writes only inside a worktree leased for it, and it had none \
                     until this write. Slot {} is leased on branch {} at {}, and Fleet is \
                     moving the session there now. Stop here: make no more tool calls and write \
                     nothing more this turn. You will be told when you are in the slot, and then \
                     repeat this write there.",
                    moving.slot, moving.branch, moving.directory
                ))
            }
        }
    }

    /// Lease a slot for the session and cut its branch. **One at a time**, so
    /// two writes in one turn lease one slot. Shared by the gate and by the grant
    /// that serves a queued session, which is why it answers a verdict and not a
    /// [`GateAnswer`].
    pub(super) async fn take_lease(&self, id: &str, served: &Served) -> Taken {
        let runtime = self.hosts().of(id);
        let _one_at_a_time = runtime.lease.lock().await;
        let Ok((session, mut hosting)) = self.session_and_hosting(id).await else {
            return Taken::Gone;
        };
        if session.state == store::SessionState::Ended {
            return Taken::Gone;
        }
        if let Some(slot) = hosting.lease_slot {
            return Taken::Held {
                slot,
                directory: Self::directory_of(served, &hosting),
            };
        }
        let spec = match WorktreeSpec::for_job(served.root(), &format!("session-{}", &id[..8])) {
            Ok(spec) => spec,
            Err(why) => return Taken::Refused(format!("no slot could be named: {why:?}")),
        };
        let pool = crate::leasing::pool_of(served);
        let (slot, directory, branch) =
            match self.vcs().lease_slot(&pool, &spec, id) {
                Ok(SlotLeased::Took { slot, worktree, .. }) => (
                    slot,
                    worktree.path().to_string(),
                    worktree.branch().to_string(),
                ),
                Ok(SlotLeased::Full) => return Taken::Full,
                Err(why) => {
                    return Taken::Refused(format!(
                        "A slot could not be leased for this write: {why}"
                    ))
                }
            };
        hosting.lease_slot = Some(slot);
        hosting.lease_branch = Some(branch.clone());
        let now = self.now().as_str().to_string();
        let titled = session.title.clone();
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
        self.session_slot(id, titled, Some(hosting.manifest_id.clone()), slot, true);
        let moving = Move {
            slot,
            branch: branch.clone(),
            directory,
        };
        runtime.state().moving = Some(moving.clone());
        self.row_put(
            id,
            SessionRow::Lease {
                id: self.row_id(id),
                at: self.instant(),
                slot,
                branch,
            },
        )
        .await;
        let _ = self.published_hosted(id).await;
        Taken::Moving(moving)
    }
}

/// What taking a session's lease came to.
pub(super) enum Taken {
    /// Leased now, and `Runtime::moving` says the session is to be moved into it.
    Moving(Move),
    /// It held a slot already.
    Held { slot: u32, directory: String },
    /// Every slot is held.
    Full,
    /// The pool would not lease, and why.
    Refused(String),
    /// The session is ended or unreadable, and has no use for a slot.
    Gone,
}

fn not_held(path: &str, slot: u32) -> String {
    format!(
        "{path} is in slot {slot}, which is not leased by this session or by anything it \
         started. Lease a slot for it with `armada worktree lease <branch>`, or write in the \
         slot this session already holds."
    )
}

fn queued(ahead: usize) -> String {
    let place = match ahead {
        0 => String::from("first in the queue"),
        n => format!("{n} ahead of it in the queue"),
    };
    format!(
        "Every worktree slot is in use, so this session cannot write yet. It is {place} for \
         one. Stop here: make no more tool calls and write nothing more this turn. Fleet leases \
         a slot for it when one frees, moves the session there and tells it, and then it repeats \
         this write."
    )
}

fn waiting_to_move(slot: u32, directory: &str) -> String {
    format!(
        "Slot {slot} is leased for this session at {directory}, and Fleet is moving the session \
         there. Make no more writes this turn: you will be told when you are in the slot."
    )
}
