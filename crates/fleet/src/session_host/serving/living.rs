//! A hosted session's process: its facts, its reattachment after a restart, and the quiet sweep.

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
    pub(crate) fn hosted_facts_of(&self, id: &str, hosting: KeptHosting) -> HostedFacts {
        let runtime = self.hosts().of(id);
        let state = runtime.state();
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

    /// Start the session's process if none is running. A keeper a Fleet before
    /// this one started is reattached to rather than replaced.
    pub(crate) async fn ensure_process(
        self: &Arc<Self>,
        id: &str,
        hosting: &KeptHosting,
    ) -> Result<(), String> {
        self.process_of(id, hosting, true).await.map(|_| ())
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

    /// The session's process, reattached or (where `may_start`) started.
    /// Whether it has one.
    pub(super) async fn process_of(
        self: &Arc<Self>,
        id: &str,
        hosting: &KeptHosting,
        may_start: bool,
    ) -> Result<bool, String> {
        let runtime = self.hosts().of(id);
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
                (process, directory)
            }
            None if !may_start => return Ok(false),
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
                        let first = match (hosting.helm, &served) {
                            (true, Some(served)) => Some(self.helm_note_of(served)),
                            _ => self.successor_note_of(id).await,
                        };
                        let runs = match &served {
                            Some(served) => self.run_note_of(served.root()).await,
                            None => None,
                        };
                        let told: Vec<String> = first.into_iter().chain(runs).collect();
                        (!told.is_empty()).then(|| told.join("\n\n"))
                    },
                    name: address_of(id),
                    // None chosen is the Session default at the time the process starts, not when it was made.
                    model: hosting.model.clone().or_else(|| Some(self.session_model())),
                    effort: hosting.effort.clone(),
                    mode: mode_of(&hosting.mode),
                    readable: vec![self.uploads_of(id).to_string_lossy().into_owned()],
                };
                let process = self.hosts().processes().start(&start, sink)?;
                if hosting.ran {
                    let mut cut = self.cut_off_subagents(id).await;
                    cut.extend(self.cut_off_background(id).await);
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
            state.helm = hosting.helm;
            state.directory = directory;
            state.last_active = std::time::Instant::now();
            state.generation
        };
        if !hosting.ran {
            let mut kept = hosting.clone();
            kept.ran = true;
            let _ = self.store().lock().await.keep_hosting(&kept);
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

    /// End every process that has been quiet for longer than the setting.
    /// **Called on a timer, and by a test**, which would not wait for one.
    pub(crate) async fn sweep_quiet(&self) {
        let quiet = self.hosts().quiet();
        let mut ended = Vec::new();
        for (id, runtime) in self.hosts().all() {
            let mut state = runtime.state();
            if state.reattached_busy
                && matches!(state.turn, SessionTurn::Working { .. })
                && state.last_active.elapsed() >= REATTACH_QUIET
            {
                state.reattached_busy = false;
                state.turn = SessionTurn::Idle;
                ended.push(id.clone());
                continue;
            }
            let idle = matches!(state.turn, SessionTurn::Idle);
            if state.process.is_some()
                && idle
                && state.asked.is_none()
                && state.last_active.elapsed() >= quiet
            {
                Self::let_go(&mut state);
                ended.push(id);
            }
        }
        for id in ended {
            let _ = self.published_hosted(&id).await;
        }
    }
}
