//! The hook a hosted session runs before a write: **the lease is taken on the
//! first one, and until then nothing can write the main checkout**. Since 23.49.
//!
//! The process's directory is fixed when it starts, so the lease cannot move it
//! in place (spike 24 measured the built-in worktree tool refusing a permission
//! door). The hook leases the slot, denies the write, and the turn ends;
//! `hearing::moved_into` then resumes the session with the slot as its
//! directory. A session resumed with no slot is leased one before its process
//! starts ([`Fleet::leased_for_resume`]), so no first call moves it. **A line the
//! hook cannot read as a read is a write**, so a shell line it does not know
//! leases a slot rather than touching the checkout.
//!
//! **A full pool queues the session** (`queueing`) rather than telling it to try
//! again: it is denied, told its place and told to stop, and Fleet leases and moves
//! it when a slot frees.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, SlotLeased, Vcs, WorkProduct, WorktreeSpec};
use ipc::{GateAnswer, ManifestId, SessionGate, SessionMode, SessionRow};
use store::{AttachmentState, Holder, KeptAttachment, KeptHosting};

use super::following::Standing;
use super::queueing::Choice;
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

/// Whether a tool can write the checkout: the ones a lease is taken on.
fn can_write(tool: &str) -> bool {
    matches!(tool, "Write" | "Edit" | "MultiEdit" | "NotebookEdit" | "Bash")
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
    pub(crate) async fn gated(self: &Arc<Self>, gate: SessionGate) -> GateAnswer {
        let id = gate.session_id.clone();
        let Ok((session, mut hosting)) = self.session_and_hosting(&id).await else {
            return GateAnswer::pass();
        };
        if session.state == store::SessionState::Ended {
            return GateAnswer::pass();
        }
        // Every call of a hosted session that is open, whatever the gate then decides.
        self.call_seen(&gate);
        // A tool the hook only reaches Fleet on so it can be noted holds nothing, so it does not
        // read the pool or the Manifest.
        if !can_write(&gate.tool_name) {
            return GateAnswer::pass();
        }
        if mode_of(&hosting.mode) == SessionMode::Plan {
            return GateAnswer::pass();
        }
        // Before the write rules: `armada` reads, so a lone check never reaches them.
        if let Some(answer) = in_the_background(&gate, mode_of(&hosting.mode)) {
            return answer;
        }
        let served = match self.served_named(Some(&ManifestId::carried(&hosting.manifest_id))) {
            Ok(served) => served,
            // A Mod Job in the mods folder has no repository, so there is no slot to keep it to.
            Err(_) if hosting.manifest_id.is_empty() && hosting.mod_job => return GateAnswer::pass(),
            // Setup's session, until its root's Manifest is served (the owner, 13 Oct 2026).
            Err(_) if hosting.manifest_id.is_empty() => {
                let root = session.cwd.clone();
                match self.set_up_since(session, &mut hosting).await {
                    Some(served) => served,
                    None => return setting_up(&gate, &root),
                }
            }
            Err(_) => return GateAnswer::pass(),
        };
        let root = served.root().trim_end_matches('/').to_string();
        // The pool is read here, so the ledger's slot is the pool's before it decides.
        let hosting = self.follow_the_pool(&id, &served, hosting).await;
        let own = hosting.lease_slot;
        // A slot it came by some other way ends its wait.
        if own.is_some() {
            self.line_left(&id).await;
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
        // Setup's window stays open once its Manifest is served: an app's armada.yml written a
        // turn later belongs in the checkout, where a slot branch would never bring it.
        if let (true, Touch::File(path)) = (hosting.setup, &touching) {
            if setup_file(&absolute(&gate, path), &root) {
                return GateAnswer::pass();
            }
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
                    "{path} is in the main checkout. This job writes in its own slot at \
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
                            "That command writes {path} in the main checkout. This job writes \
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
                                 job writes in its own slot at {directory}: work there."
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
        // A write let through inside any slot the session holds is a file it has written there,
        // for the check of who else writes it. `docs/concepts/fleet.md`, *Write clashes*.
        if let (Some(path), Some(_), GateAnswer::Pass {}) = (written, own, &answer) {
            let inside = match lands(&path) {
                Place::Slot(slot) if held(slot) => {
                    let at = adapter_traits::slot_path(&root, slot);
                    // The pool then names the session on a lease its subagent took, which a
                    // process holder cannot say once it is gone.
                    if own != Some(slot) {
                        let _ = adapters::leasing::note_session(std::path::Path::new(&at), &id);
                    }
                    let whole = absolute(&gate, &path);
                    Some(crate::clashing::repository_relative(&whole, &at, &self.host().home))
                }
                // A relative path is read against the session's own directory.
                Place::Main if !path.starts_with('/') => Some(path.clone()),
                _ => None,
            };
            if let Some(inside) = inside.filter(|inside| !inside.starts_with('/')) {
                let who = crate::clashing::Who::Session(id.clone());
                self.writes().wrote(&who, served.manifest().id().as_str(), [inside], &self.now());
            }
        }
        answer
    }

    /// Setup's session once its root's Manifest is served: bound to it from here
    /// on, so it leases a slot like any other. `None` while there is none.
    async fn set_up_since(
        &self,
        mut session: store::KeptSession,
        hosting: &mut store::KeptHosting,
    ) -> Option<Served> {
        let manifest = self
            .repositories()
            .at(&session.cwd)?
            .manifest()?
            .id()
            .clone();
        let served = self.repositories().serving(manifest.as_str())?;
        hosting.manifest_id = manifest.as_str().to_string();
        // Only Setup's session reaches here, including one started before the mark was kept.
        hosting.setup = true;
        session.manifest_id = Some(hosting.manifest_id.clone());
        {
            let mut store = self.store().lock().await;
            let _ = store.keep_hosting(hosting);
            let _ = store.keep_session(&session);
        }
        let _ = self.published_hosted(&session.id).await;
        Some(served)
    }

    /// A first write into the checkout. **Behind everyone already waiting**, so a slot
    /// freed between two of them is not taken by the newer. When the pool is full the
    /// session is queued, and `queueing` leases it a slot when one frees.
    async fn leased(&self, id: &str, served: &Served) -> GateAnswer {
        let queue = self.hosts().slot_queue();
        let manifest = served.manifest().id().as_str();
        // A slot added for a command that is asking is its own until it leases it.
        let held_back = queue.reserved(manifest, Some(id), std::time::Instant::now());
        let taken = match queue.ahead_of(manifest, id) {
            0 if !held_back => self.take_lease(id, served).await,
            _ => Taken::Full,
        };
        match taken {
            Taken::Gone => GateAnswer::pass(),
            Taken::Held { slot, directory } => {
                self.line_left(id).await;
                GateAnswer::deny(waiting_to_move(slot, &directory))
            }
            Taken::Refused(why) => GateAnswer::deny(why),
            Taken::Full => {
                let ahead = self.line_joined(manifest, id).await;
                GateAnswer::deny(queued(ahead, queue.chosen(id)))
            }
            Taken::Moving(moving) => {
                self.line_left(id).await;
                GateAnswer::deny(format!(
                    "This job writes only inside a worktree leased for it, and it had none \
                     until this write. Slot {} is leased on branch {} at {}, and Fleet is \
                     moving the job there now. Stop here: make no more writes this turn. Reads \
                     and Fleet's own tools, such as all_done, still work. You will be told when \
                     you are in the slot, and then repeat this write there.",
                    moving.slot, moving.branch, moving.directory
                ))
            }
        }
    }

    /// A session about to be resumed with no slot is given one first, and its process starts in
    /// it. **A resumed session otherwise starts in the main checkout and is moved by whatever its
    /// first tool call is**, which cost it that call and a turn. A full pool, or a line of
    /// sessions already waiting, leaves the hosting as it was and the first write queues as before.
    pub(crate) async fn leased_for_resume(
        &self,
        served: &Served,
        hosting: &KeptHosting,
    ) -> KeptHosting {
        let id = hosting.session_id.as_str();
        let manifest = served.manifest().id().as_str();
        let queue = self.hosts().slot_queue();
        let unleased = hosting.lease_slot.is_none() && !hosting.helm && mode_of(&hosting.mode) != SessionMode::Plan;
        if !unleased {
            return hosting.clone();
        }
        // Behind a line already: the call is raised now, and the first write queues as before.
        if queue.ahead_of(manifest, id) != 0 {
            self.line_joined(manifest, id).await;
            return hosting.clone();
        }
        match self.take_lease(id, served).await {
            // No process runs yet, so there is nothing to move.
            Taken::Moving(_) => self.hosts().of(id).state().moving = None,
            Taken::Held { .. } => {}
            Taken::Full => {
                self.line_joined(manifest, id).await;
                return hosting.clone();
            }
            Taken::Refused(_) | Taken::Gone => return hosting.clone(),
        }
        self.line_left(id).await;
        match self.session_and_hosting(id).await {
            Ok((_, leased)) => leased,
            Err(_) => hosting.clone(),
        }
    }

    /// The session is in the repository's line for a slot, and the person has been asked what to
    /// do about it if this is its first time there. How many are ahead of it.
    pub(super) async fn line_joined(&self, manifest: &str, id: &str) -> usize {
        let queue = self.hosts().slot_queue();
        let (ahead, fresh) = queue.join(manifest, id, &self.instant());
        if fresh {
            let _ = self.published_hosted(id).await;
        }
        // A slot may have been given back between the lease and the join.
        queue.freed();
        ahead
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
        let pool = crate::leasing::pool_of(served);
        // **A paused session is seated on its own branch at the tip**, not on a fresh one.
        let parked_on = hosting.paused_at.is_some().then(|| hosting.lease_branch.clone()).flatten();
        let leased = match &parked_on {
            Some(branch) => self.vcs().lease_existing_slot(&pool, branch, id),
            None => match WorktreeSpec::for_job(served.root(), &format!("session-{}", &id[..8])) {
                Ok(spec) => self.vcs().lease_slot(&pool, &spec, id),
                Err(why) => return Taken::Refused(format!("no slot could be named: {why:?}")),
            },
        };
        let (slot, directory, branch) = match leased {
            Ok(SlotLeased::Took { slot, worktree, .. }) => (
                slot,
                worktree.path().to_string(),
                worktree.branch().to_string(),
            ),
            Ok(SlotLeased::Full) => return Taken::Full,
            Err(why) => {
                return Taken::Refused(format!("A slot could not be leased for this write: {why}"))
            }
        };
        let was_paused = parked_on.is_some();
        hosting.paused_at = None;
        hosting.paused_resuming = false;
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
        // No process runs and none is moved: a resume starts nothing, and a message that
        // resumes it starts its process in this directory.
        if was_paused {
            let _ = self.published_hosted(id).await;
            return Taken::Held { slot, directory };
        }
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

/// The files Setup's session writes in the checkout, by name at any depth, plus
/// `.git/info/exclude` (the owner, 14 Oct 2026: a monorepo has one `armada.yml` per app).
const SETUP_NAMES: [&str; 2] = ["armada.yml", ".gitignore"];

/// A file tool's path, made absolute against where the call stands, `.` and `..` resolved.
fn absolute(gate: &SessionGate, path: &str) -> String {
    match path.starts_with('/') {
        true => normalised(path),
        false => normalised(&format!("{}/{path}", gate.cwd.trim_end_matches('/'))),
    }
}

/// Whether `at`, absolute and normalised, is a setup file of `root`'s own checkout. Nothing
/// under a slot, and nothing else under `.git`.
fn setup_file(at: &str, root: &str) -> bool {
    let root = root.trim_end_matches('/');
    if place_of(at, root) != Place::Main || at == root {
        return false;
    }
    if at == format!("{root}/.git/info/exclude") {
        return true;
    }
    let named = at.rsplit('/').next().unwrap_or_default();
    !at.starts_with(&format!("{root}/.git/")) && SETUP_NAMES.contains(&named)
}

/// Setup's session's call, before `root` has a Manifest. A relative path is read
/// against where the call stands.
fn setting_up(gate: &SessionGate, root: &str) -> GateAnswer {
    let root = root.trim_end_matches('/');
    match touch(gate) {
        Touch::Nothing => GateAnswer::pass(),
        Touch::File(path) if setup_file(&absolute(gate, &path), root) => GateAnswer::pass(),
        Touch::File(_) | Touch::Shell(_) => GateAnswer::deny(format!(
            "Until this repository's armada.yml is served, this job writes only a file named \
             armada.yml or .gitignore anywhere under {root}, and {root}/.git/info/exclude, with \
             the Write or Edit tool. Nothing else is written here."
        )),
    }
}

/// **A check runs in the background, so the person can reach the session while it runs**
/// (#2415). The harness wakes the session when it ends. A subagent's call is left alone: one
/// handed a background result ends its turn and reports before the check has finished. A line
/// that also writes is refused, since backgrounding it would hide whether the write happened.
fn in_the_background(gate: &SessionGate, mode: SessionMode) -> Option<GateAnswer> {
    let command = gate.tool_input.get("command").and_then(|said| said.as_str())?;
    if gate.tool_name != "Bash" || gate.agent_id.is_some() || gate.in_background() {
        return None;
    }
    match adapters::check_line(command) {
        adapters::CheckLine::NoCheck => None,
        adapters::CheckLine::Alone => Some(GateAnswer::rewrite(
            gate.backgrounded(),
            mode == SessionMode::Ask,
            "Fleet runs a check in the background, so the person can reach the job while it runs.",
        )),
        adapters::CheckLine::Chained => Some(GateAnswer::deny(
            "A check runs in the background, so the person can reach this job while it runs. \
             Run the check on a line of its own, then the rest once you are told it ended.",
        )),
    }
}

fn not_held(path: &str, slot: u32) -> String {
    format!(
        "{path} is in slot {slot}, which is not leased by this job or by anything it \
         started. Lease a slot for it with `armada worktree lease <branch>`, or write in the \
         slot this job already holds."
    )
}

fn queued(ahead: usize, chosen: Option<Choice>) -> String {
    let place = match ahead {
        0 => String::from("first in the queue"),
        n => format!("{n} ahead of it in the queue"),
    };
    let owner = match chosen {
        Some(Choice::Waiting) => "The owner chose to wait for a slot to free.",
        Some(Choice::Adding) => "The owner chose to add a slot.",
        _ => "The owner is being asked whether this job waits for a slot to free or Fleet adds one.",
    };
    format!(
        "Every worktree slot is in use, so this job cannot write yet. It is {place} for \
         one. {owner} Stop here: make no more writes this turn. Reads and Fleet's own tools, \
         such as all_done, still work. Fleet leases a slot for it, moves the job there and \
         tells it, and then it repeats this write."
    )
}

fn waiting_to_move(slot: u32, directory: &str) -> String {
    format!(
        "Slot {slot} is leased for this job at {directory}, and Fleet is moving the job \
         there. Make no more writes this turn: you will be told when you are in the slot."
    )
}
