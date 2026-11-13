//! A hosted session's process: its facts, its reattachment after a restart, and the quiet sweep.

use std::path::Path;

use config::settings as keys;

use super::super::token_use;
use super::*;

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
    /// What a hosted session carries beyond a terminal's row.
    pub(crate) fn hosted_facts_of(
        &self,
        id: &str,
        hosting: KeptHosting,
        start: Option<crate::session_host::start_card::StartReads>,
        values: Vec<store::KeptValue>,
    ) -> HostedFacts {
        let runtime = self.hosts().of(id);
        let state = runtime.state();
        let start = start.map(|reads| self.start_wire(&hosting, reads, state.proposal.queued.is_some()));
        HostedFacts {
            turn: state.turn.clone(),
            asked: state.asked.clone(),
            model: hosting.model,
            resolved_model: state.resolved_model.clone(),
            effort: hosting.effort,
            mode: mode_of(&hosting.mode),
            running: state.process.is_some(),
            commands: if state.commands.is_empty() {
                self.hosts().commands()
            } else {
                state.commands.clone()
            },
            paused: hosting.paused_at.as_deref().map(|at| ipc::SessionPaused {
                at: ipc::Instant::carried(at),
                resuming: hosting.paused_resuming,
            }),
            start,
            start_failed: state.start_failed.as_ref().map(|failed| ipc::StartFailed {
                cause: failed.cause.clone(),
                log: failed.log.clone(),
            }),
            project: hosting.project,
            values: crate::values::Frozen::of(values).map(|frozen| frozen.wire()),
        }
    }

    /// The row as it stands, published whole, and answered.
    pub(crate) async fn published_hosted(&self, id: &str) -> Result<SessionRecord, Refusal> {
        let record = {
            let store = self.store().lock().await;
            let session = store
                .session(id)
                .map_err(|why| self.ledger_fault(why))?
                .ok_or_else(|| {
                    self.hosted_refusal(NO_SUCH_SESSION, &format!("no job is named {id}"))
                })?;
            self.ledger_row(&store, &session)?
        };
        self.publish(ipc::Event::SessionChanged(record.clone()));
        Ok(record)
    }

    /// Let a process go: it is ended and its late word is not read as the new
    /// one's. The caller holds the state.
    pub(crate) fn let_go(state: &mut super::super::State) {
        if let Some(process) = state.process.take() {
            process.end();
        }
        state.generation += 1;
        state.resolved_model = None;
        state.queued = 0;
        state.unstarted.clear();
    }

    /// A process runs on what it was started with, so a running one ends once its turn is over
    /// and the next message resumes on what the Job names now.
    pub(crate) fn restarts_on_what_it_names(&self, id: &str) {
        let runtime = self.hosts().of(id);
        let mut state = runtime.state();
        if state.process.is_some() {
            match state.turn {
                SessionTurn::Idle => Self::let_go(&mut state),
                SessionTurn::Working { .. } => state.restart_after_turn = true,
            }
        }
    }

    /// A Job naming a model that is switched off goes back to the Job model, kept so its row says
    /// what it runs on. Whoever named it, a hand-on, a fork or a Job from before the switch, and a
    /// process that outlived a Fleet on it moves too.
    async fn off_a_model_switched_off(&self, id: &str, mut hosting: KeptHosting) -> KeptHosting {
        if hosting.model.as_ref().is_some_and(|model| !self.models_on().contains(model)) {
            hosting.model = None;
            let _ = self.store().lock().await.keep_hosting(&hosting);
            self.restarts_on_what_it_names(id);
        }
        hosting
    }

    /// The models on were `was_on` before settings.json changed. Every open Job on one that is now
    /// off moves as a change in its model picker moves it: when its turn is over.
    pub(crate) async fn models_switched_off(&self, was_on: &[String]) {
        let on = self.models_on();
        if was_on.iter().all(|model| on.contains(model)) {
            return;
        }
        let moved: Vec<String> = {
            let mut store = self.store().lock().await;
            let Ok(hostings) = store.hostings() else { return };
            let mut moved = Vec::new();
            for mut hosting in hostings {
                // A Job naming none runs on what the Job model was when its process started.
                let Some(runs_on) = hosting.model.as_ref().or(hosting.turn_model.as_ref()) else { continue };
                if on.contains(runs_on) {
                    continue;
                }
                let open = store.session(&hosting.session_id).ok().flatten().is_some_and(|session| session.state != store::SessionState::Ended);
                if !open {
                    continue;
                }
                if hosting.model.as_ref().is_some_and(|model| !on.contains(model)) {
                    hosting.model = None;
                    if store.keep_hosting(&hosting).is_err() {
                        continue;
                    }
                }
                moved.push(hosting.session_id);
            }
            moved
        };
        for id in moved {
            self.restarts_on_what_it_names(&id);
            let _ = self.published_hosted(&id).await;
        }
    }

    /// Start the session's process if none is running. A keeper a Fleet before
    /// this one started is reattached to rather than replaced.
    pub(crate) async fn ensure_process(
        self: &Arc<Self>,
        id: &str,
        hosting: &KeptHosting,
    ) -> Result<(), String> {
        let hosting = self.off_a_model_switched_off(id, hosting.clone()).await;
        self.process_of(id, &hosting, true).await.map(|_| ())
    }

    /// Reattach to every hosted session whose keeper still answers, at boot.
    /// Nothing is started. How many were taken back.
    pub async fn reattached_sessions(self: &Arc<Self>) -> usize {
        let hostings = self.store().lock().await.hostings();
        let Ok(hostings) = hostings else { return 0 };
        let mut taken = 0;
        for hosting in hostings {
            if let Ok(true) = self.process_of(&hosting.session_id, &hosting, false).await {
                taken += 1;
                let _ = self.published_hosted(&hosting.session_id).await;
            }
        }
        taken
    }

    /// A process reattached with its move into the slot still standing is the one from before the
    /// move, in the checkout, while its hosting names the slot: a relative write would land in main.
    /// **The move is owed again**, held until its agents end, as it was (`armada-f7s3`).
    async fn move_owed_again(&self, id: &str, hosting: &KeptHosting, directory: &str) {
        let (Some(slot), Some(branch)) = (hosting.lease_slot, hosting.lease_branch.clone()) else {
            return;
        };
        let waits = self.store().lock().await.slot_waits_of(id).unwrap_or_default();
        if waits.iter().any(|wait| wait.phase == "move" && wait.ended_at.is_none()) {
            let directory = directory.to_string();
            self.hosts().of(id).state().moving =
                Some(crate::session_host::Move { slot, branch, directory, held: true });
        }
    }

    /// Every slot wait the last Fleet left standing ended, but the moves owed again: the line is
    /// read back after, and its waits begin again. Called once, after the reattach at boot.
    pub(crate) async fn slot_waits_ended_at_boot(&self) {
        let at = self.now().as_str().to_string();
        let owed: Vec<String> = self
            .hosts()
            .all()
            .into_iter()
            .filter(|(_, runtime)| runtime.state().moving.is_some())
            .map(|(id, _)| id)
            .collect();
        let _ = self.store().lock().await.end_all_slot_waits(&at, &owed);
    }

    /// The session's process, reattached or (where `may_start`) started.
    /// Whether it has one.
    pub(super) async fn process_of(
        self: &Arc<Self>,
        id: &str,
        hosting: &KeptHosting,
        may_start: bool,
    ) -> Result<bool, String> {
        let runtime = self.hosts().of(id);
        // **One start at a time.** The awaits below let a second sender past the check, and its
        // keeper took the first one's socket: the first agent was killed and read as the Job dying.
        let _starting = runtime.starting.lock().await;
        if runtime.state().process.is_some() {
            return Ok(true);
        }
        let served = self.served_named(Some(&ManifestId::carried(&hosting.manifest_id)));
        let (sink, heard) = tokio::sync::mpsc::unbounded_channel::<Heard>();
        let (process, directory) = match self.hosts().processes().reattach(id, sink.clone()) {
            Some(process) => {
                self.kept_restart(
                    id,
                    "reattached",
                    Some("Fleet came back and took the job's process again"),
                )
                .await;
                let directory = match &served {
                    Ok(served) => Self::directory_of(served, hosting),
                    Err(_) => self
                        .unset_up_directory(id, hosting)
                        .await
                        .unwrap_or_default(),
                };
                self.move_owed_again(id, hosting, &directory).await;
                (process, directory)
            }
            None if !may_start => return Ok(false),
            // **The one place a paused Job's agent is kept from starting**, whoever wrote to it:
            // only the person's Resume lifts a pause, and it starts nothing itself.
            None if hosting.paused_at.is_some() => {
                return Err(String::from("the job is paused, and only the person resumes it"));
            }
            None => {
                let (directory, resuming, resumed, served) = match served {
                    Ok(served) => {
                        // **A session that has run is leased its slot before its process starts**, so no
                        // tool call of the first turn is the one that moves it.
                        let resuming = hosting.ran && self.has_a_conversation(id).await;
                        let resumed = match resuming {
                            true => self.leased_for_resume(&served, hosting).await,
                            false => hosting.clone(),
                        };
                        let directory = Self::directory_of(&served, &resumed);
                        (directory, resuming, resumed, Some(served))
                    }
                    // Setup's session has no Manifest and so no slot: it runs in the root it started in.
                    Err(_) => {
                        let directory = self.unset_up_directory(id, hosting).await.ok_or_else(|| {
                            format!("the repository {} is no longer served", hosting.manifest_id)
                        })?;
                        let resuming = hosting.ran && self.has_a_conversation(id).await;
                        (directory, resuming, hosting.clone(), None)
                    }
                };
                let hosting = &resumed;
                self.mod_filled_for_launch().await;
                let (compaction_window, connectors, skills_off) = {
                    let now = self.machine_settings().now();
                    (
                        now.get(keys::COMPACTION_WINDOW),
                        token_use::connectors_on(
                            now.get(keys::CONNECTORS_OFF),
                            &hosting.manifest_id,
                            served.as_ref().is_none_or(|served| served.manifest().connectors()),
                        ),
                        token_use::skills_off(
                            now.get(keys::SKILLS_OFF),
                            &hosting.manifest_id,
                            |name| {
                                served
                                    .as_ref()
                                    .is_some_and(|served| adapters::has_own_skill(Path::new(served.root()), name))
                            },
                        ),
                    )
                };
                let start = Start {
                    directory: directory.clone(),
                    session: id.to_string(),
                    // **Resumed only where there is a conversation to resume.** A process that
                    // started and ended before its first turn wrote none, and `--resume` on it
                    // exits at once ("No conversation found"), so every later message was lost
                    // (s-c271bfe5, 8 Oct 2026). Started fresh under the same id instead.
                    resuming,
                    forking: hosting.fork_of.clone(),
                    fork_note: match &hosting.fork_of {
                        Some(old) => self.fork_note_of(id, old).await,
                        None => None,
                    },
                    fork_cut: match hosting.fork_of {
                        Some(_) => self.fork_cut_of(id).await,
                        None => None,
                    },
                    // Read at each start, so an edit to the note reaches the next process.
                    note: {
                        let first = self.successor_note_of(id).await;
                        let logbook = self.logbook_note_of(served.as_ref().map(|one| one.root())).await;
                        let told: Vec<String> = first.into_iter().chain(logbook).chain(Some(self.ways_note())).collect();
                        Some(told.join("\n\n"))
                    },
                    name: address_of(id),
                    // None chosen is the Session default at the time the process starts, not when it was made.
                    model: hosting.model.clone().or_else(|| Some(self.session_model())),
                    models: self.models_on(),
                    effort: hosting.effort.clone(),
                    mode: mode_of(&hosting.mode),
                    readable: vec![self.uploads_of(id).to_string_lossy().into_owned()],
                    connectors,
                    compaction_window,
                    skills_off,
                };
                let process = self.hosts().processes().start(&start, sink)?;
                if hosting.ran {
                    let mut cut = self.cut_off_subagents(id).await;
                    cut.extend(self.cut_off_background(id).await);
                    self.calls_let_go(id).await;
                    self.kept_restart(id, "resumed", super::super::cut_off::cut_off_said(&cut).as_deref())
                        .await;
                    runtime.state().cut_off = cut;
                }
                (process, directory)
            }
        };
        let generation = {
            let mut state = runtime.state();
            state.process = Some(process);
            state.on_your_words = hosting.on_your_words;
            state.directory = directory;
            state.last_active = std::time::Instant::now();
            state.generation
        };
        if !hosting.ran {
            let mut kept = hosting.clone();
            kept.ran = true;
            let _ = self.store().lock().await.keep_hosting(&kept);
            self.started_kept(id, hosting).await;
        }
        tokio::spawn(Self::listening(
            Arc::clone(self),
            id.to_string(),
            generation,
            heard,
        ));
        if self.hosts().start_sweeping() {
            let fleet = Arc::clone(self);
            let every =
                (self.hosts().quiet() / 4).clamp(Duration::from_secs(1), Duration::from_secs(30));
            tokio::spawn(async move {
                loop {
                    tokio::time::sleep(every).await;
                    fleet.sweep_quiet().await;
                    fleet.held_moves_made().await;
                    fleet.processes_moved_onto_mod().await;
                }
            });
        }
        Ok(true)
    }

    /// Everything one process says, in order. **Boxed**, because hearing a
    /// message from one session can start another's process, which listens
    /// again, and a future that contains itself cannot be proved `Send`.
    pub(super) fn listening(
        fleet: Arc<Self>,
        id: String,
        generation: u64,
        mut heard: tokio::sync::mpsc::UnboundedReceiver<Heard>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>> {
        Box::pin(async move {
            while let Some(said) = heard.recv().await {
                fleet.heard(&id, generation, said).await;
            }
        })
    }

    /// End every process that has been quiet for longer than the setting, unless an agent of it is
    /// still working, then compact the Jobs waiting on the owner whose cache is nearly out. **Called
    /// on a timer, and by a test**, which would not wait for one.
    pub(crate) async fn sweep_quiet(self: &Arc<Self>) {
        let quiet = self.hosts().quiet();
        let is_quiet = |state: &super::super::State| {
            state.process.is_some()
                && matches!(state.turn, SessionTurn::Idle)
                && state.asked.is_none()
                // A move ends this process itself, and tells the Job.
                && state.moving.is_none()
                && state.last_active.elapsed() >= quiet
        };
        let mut ended = Vec::new();
        let mut closed = Vec::new();
        let mut gone_quiet = Vec::new();
        for (id, runtime) in self.hosts().all() {
            let mut state = runtime.state();
            if state.reattached_busy
                && matches!(state.turn, SessionTurn::Working { .. })
                && state.last_active.elapsed() >= REATTACH_QUIET
            {
                state.reattached_busy = false;
                state.turn = SessionTurn::Idle;
                ended.push(id.clone());
                closed.push(id.clone());
                continue;
            }
            if is_quiet(&state) {
                gone_quiet.push((id, Arc::clone(&runtime)));
            }
        }
        for (id, runtime) in gone_quiet {
            if self.work_beyond_the_turn(&id).await {
                continue;
            }
            // Read again under the lock: a turn may have begun while the ledger was read.
            let mut state = runtime.state();
            if is_quiet(&state) {
                Self::let_go(&mut state);
                ended.push(id);
            }
        }
        // After the state locks are dropped: a reattached turn that went quiet is over, not dead.
        for id in closed {
            self.turn_closed(&id).await;
        }
        for id in ended {
            let _ = self.published_hosted(&id).await;
        }
        self.compacted_while_waiting().await;
    }
}
