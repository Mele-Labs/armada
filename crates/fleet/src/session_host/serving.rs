//! `api::HostedSessions`, over a real Fleet: start a session, take a message,
//! answer an ask, tune, close, read. Since 23.49.
//!
//! **Nothing here waits on a process.** A message is written to its input and
//! answered at once; what comes back is the thread's, a row at a time.

use std::sync::Arc;
use std::time::Duration;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::{HostedSessions, Refusal, StoredFile};
use base64::Engine as _;
use ipc::{
    AnswerHelmCall, AnswerSessionAsk, CloseSession, GateAnswer, HostedFacts, ManifestId,
    SendSessionMessage, SentFile, SessionGate, SessionId, SessionMode, SessionRecord, SessionRow,
    SessionThread, SessionTurn, SessionVoice, StartSession, TuneSession, WireError,
};
use store::{AttachmentState, Holder, KeptAttachment, KeptHosting, KeptSession, Store};

use super::address_of;
use super::process::{Heard, Start};
use super::rows::{mention_line, new_id, safe_name, UserLine};
use crate::daemon::Fleet;
use crate::repositories::Served;

/// A session id that names nothing. A 422.
const NO_SUCH_SESSION: &str = "fleet.no_such_session";
/// A message or a tune to a session that was closed. A 409.
const SESSION_CLOSED: &str = "fleet.session_closed";
/// A message with neither words nor a file. A 422.
pub(super) const MESSAGE_EMPTY: &str = "fleet.session_message_empty";
/// An attachment that would not decode, or is too large. A 422.
const ATTACHMENT_REFUSED: &str = "fleet.session_attachment_refused";
/// The agent's process would not start. A 500.
const SESSION_UNSTARTED: &str = "fleet.session_unstarted";
/// An answer naming no ask that is waiting. A 409.
const SESSION_ASK_NOT_WAITING: &str = "fleet.session_ask_not_waiting";

/// The most one attachment may be, decoded.
const MOST_AN_ATTACHMENT: usize = 25 * 1024 * 1024;

pub(crate) fn mode_text(mode: SessionMode) -> &'static str {
    match mode {
        SessionMode::Ask => "ask",
        SessionMode::Auto => "auto",
        SessionMode::AcceptEdits => "accept_edits",
        SessionMode::Plan => "plan",
    }
}

pub(crate) fn mode_of(text: &str) -> SessionMode {
    match text {
        "ask" => SessionMode::Ask,
        "accept_edits" => SessionMode::AcceptEdits,
        "plan" => SessionMode::Plan,
        _ => SessionMode::Auto,
    }
}

/// The first line of a message, as the title of a session that has none.
fn title_of(text: &str) -> Option<String> {
    let line = text.lines().map(str::trim).find(|line| !line.is_empty())?;
    Some(line.chars().take(60).collect())
}

impl<H, V, W> HostedSessions for Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    async fn start_session(&self, mut start: StartSession) -> Result<SessionRecord, Refusal> {
        if let Some(from) = start.pilot.take() {
            return self.start_piloted(start, from).await;
        }
        let served = self.served_named(Some(&start.manifest_id))?;
        let id = new_id();
        let now = self.now().as_str().to_string();
        let title = start
            .title
            .map(|title| title.trim().to_string())
            .filter(|title| !title.is_empty());
        let manifest = served.manifest().id().as_str().to_string();
        {
            let mut store = self.store().lock().await;
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
                    last_seen_at: now,
                    last_turn_at: None,
                    ended_at: None,
                    end_reason: None,
                    figures: Default::default(),
                })
                .map_err(|why| self.ledger_fault(why))?;
            store
                .keep_hosting(&KeptHosting {
                    session_id: id.clone(),
                    manifest_id: manifest,
                    model: start.model.filter(|model| !model.trim().is_empty()),
                    effort: start.effort.filter(|effort| !effort.trim().is_empty()),
                    mode: mode_text(start.mode.unwrap_or_default()).to_string(),
                    ran: false,
                    lease_slot: None,
                    lease_branch: None,
                })
                .map_err(|why| self.ledger_fault(why))?;
        }
        self.published_hosted(&id).await
    }

    async fn send_session_message(
        self: Arc<Self>,
        sent: SendSessionMessage,
    ) -> Result<SessionRecord, Refusal> {
        let id = sent.session_id.as_str().to_string();
        if let Some(session) = self.terminal_session(&id).await? {
            let addressed = self.addressed(&sent.mentions).await?;
            let kept = self.keep_uploads(&id, &sent.attachments)?;
            let mut paths = kept.paths;
            paths.extend(kept.picture_paths);
            return self
                .send_to_terminal(&session, sent, addressed, paths)
                .await;
        }
        let (session, hosting) = self.session_and_hosting(&id).await?;
        if session.state == store::SessionState::Ended {
            return Err(self.closed(&id));
        }
        let text = sent.text.trim().to_string();
        if text.is_empty() && sent.attachments.is_empty() {
            return Err(self.hosted_refusal(
                MESSAGE_EMPTY,
                "a message needs words or something sent with it",
            ));
        }
        let kept = self.keep_uploads(&id, &sent.attachments)?;
        let addressed = self.addressed(&sent.mentions).await?;
        let now = self.instant();
        self.row_put(
            &id,
            SessionRow::Message {
                id: self.row_id(&id),
                at: now,
                from: SessionVoice::You,
                text: text.clone(),
                files: kept.files.clone(),
                tags: sent.mentions.clone(),
            },
        )
        .await;
        self.noted_on_the_ledger(&session, &sent.mentions, &text)
            .await?;

        let mut turn = String::new();
        if !hosting.ran {
            if let Some(preface) = self.handoff_preface(&id).await {
                turn.push_str(&preface);
                turn.push_str("\n\nThe person's first message:\n\n");
            }
        }
        if let Some(line) = mention_line(&addressed) {
            turn.push_str(&line);
            turn.push_str("\n\n");
        }
        turn.push_str(&text);
        for path in &kept.paths {
            turn.push_str(&format!("\n\nAttached file: {path}"));
        }
        let line = ipc::encode(&UserLine::of(turn, kept.pictures))
            .map_err(|why| self.hosted_refusal(SESSION_UNSTARTED, &why.to_string()))?;

        let runtime = self.hosts().of(&id);
        self.ensure_process(&id, &hosting)
            .await
            .map_err(|why| self.hosted_fault(SESSION_UNSTARTED, &why))?;
        {
            let mut state = runtime.state();
            state.queued += 1;
            state.turn = SessionTurn::Working { woken_by: None };
            state.last_active = std::time::Instant::now();
            if let Some(process) = &state.process {
                process.send(line);
            }
        }
        if self.hosts().start_sweeping() {
            let fleet = Arc::clone(&self);
            let every =
                (self.hosts().quiet() / 4).clamp(Duration::from_secs(1), Duration::from_secs(30));
            tokio::spawn(async move {
                loop {
                    tokio::time::sleep(every).await;
                    fleet.sweep_quiet().await;
                }
            });
        }
        self.published_hosted(&id).await
    }

    async fn answer_session_ask(&self, said: AnswerSessionAsk) -> Result<SessionRecord, Refusal> {
        let id = said.session_id.as_str().to_string();
        self.session_and_hosting(&id).await?;
        let held = self
            .hosts()
            .of(&id)
            .state()
            .asked
            .as_ref()
            .is_some_and(|asked| asked.call == said.call);
        if !held {
            return Err(self.ask_not_waiting());
        }
        self.hosts()
            .asks()
            .answer(&AnswerHelmCall {
                call: said.call,
                answer: said.answer,
                note: said.note,
            })
            .map_err(|_| self.ask_not_waiting())?;
        self.published_hosted(&id).await
    }

    async fn tune_session(&self, tuned: TuneSession) -> Result<SessionRecord, Refusal> {
        let id = tuned.session_id.as_str().to_string();
        if let Some(session) = self.terminal_session(&id).await? {
            return self.tune_terminal(&session, tuned).await;
        }
        let (session, mut hosting) = self.session_and_hosting(&id).await?;
        if session.state == store::SessionState::Ended {
            return Err(self.closed(&id));
        }
        hosting.model = tuned.model.filter(|model| !model.trim().is_empty());
        hosting.effort = tuned.effort.filter(|effort| !effort.trim().is_empty());
        hosting.mode = mode_text(tuned.mode).to_string();
        self.store()
            .lock()
            .await
            .keep_hosting(&hosting)
            .map_err(|why| self.ledger_fault(why))?;
        // A process runs on what it was started with, so a running one ends
        // once its turn is over and the next message resumes on the new ones.
        let runtime = self.hosts().of(&id);
        {
            let mut state = runtime.state();
            if state.process.is_some() {
                match state.turn {
                    SessionTurn::Idle => Self::let_go(&mut state),
                    SessionTurn::Working { .. } => state.restart_after_turn = true,
                }
            }
        }
        self.published_hosted(&id).await
    }

    async fn close_session(&self, closed: CloseSession) -> Result<SessionRecord, Refusal> {
        let id = closed.session_id.as_str().to_string();
        let (session, hosting) = self.session_and_hosting(&id).await?;
        if session.state == store::SessionState::Ended {
            return self.published_hosted(&id).await;
        }
        {
            let runtime = self.hosts().of(&id);
            let mut state = runtime.state();
            if let Some(asked) = state.asked.take() {
                self.hosts().asks().withdraw(&asked.call);
            }
            Self::let_go(&mut state);
            state.turn = SessionTurn::Idle;
            state.moving = None;
        }
        if let Some(job) = self.piloted_job_of(&id).await {
            // The worktree is the Job's, handed over: it goes back to it, and
            // is not committed to anyone's branch on the way.
            self.closed_while_piloting(&job).await;
        } else if let (Some(slot), Ok(served)) = (
            hosting.lease_slot,
            self.served_named(Some(&ManifestId::carried(&hosting.manifest_id))),
        ) {
            // Parked: what is in the slot is committed to the session's own
            // branch and nothing is pushed, so closing loses no work.
            let _ = self
                .vcs()
                .park_hosted_slot(&crate::leasing::pool_of(&served), slot, &id);
        }
        let now = self.now().as_str().to_string();
        {
            let mut store = self.store().lock().await;
            let mut row = session;
            row.state = store::SessionState::Ended;
            row.ended_at = Some(now.clone());
            row.end_reason = Some(String::from("closed"));
            row.last_seen_at = now.clone();
            store
                .keep_session(&row)
                .map_err(|why| self.ledger_fault(why))?;
            store
                .give_back(&Holder::session(&id), None, &now)
                .map_err(|why| self.ledger_fault(why))?;
        }
        self.published_hosted(&id).await
    }

    async fn get_session(self: Arc<Self>, id: SessionId) -> Result<SessionThread, Refusal> {
        if let Some(session) = self.terminal_session(id.as_str()).await? {
            let rows = self.terminal_thread(&session).await?;
            let record = {
                let store = self.store().lock().await;
                self.ledger_row(&store, &session)?
            };
            return Ok(SessionThread {
                session: record,
                rows,
            });
        }
        let (session, _) = self.session_and_hosting(id.as_str()).await?;
        let record = {
            let store = self.store().lock().await;
            self.ledger_row(&store, &session)?
        };
        Ok(SessionThread {
            session: record,
            rows: self.rows_of(id.as_str()).await?,
        })
    }

    async fn get_session_file(&self, id: SessionId, file: String) -> Result<StoredFile, Refusal> {
        self.session_and_hosting(id.as_str()).await?;
        let sent = self
            .rows_of(id.as_str())
            .await?
            .into_iter()
            .find_map(|row| match row {
                SessionRow::Message { files, .. } => {
                    files.into_iter().find(|candidate| candidate.id == file)
                }
                _ => None,
            })
            .ok_or_else(|| {
                self.hosted_refusal(ATTACHMENT_REFUSED, "nothing was sent under that id")
            })?;
        let at = std::fs::read_dir(self.uploads_of(id.as_str()))
            .ok()
            .and_then(|entries| {
                entries.flatten().find(|entry| {
                    entry
                        .file_name()
                        .to_string_lossy()
                        .starts_with(&format!("{file}-"))
                })
            })
            .ok_or_else(|| {
                self.hosted_refusal(ATTACHMENT_REFUSED, "that file is no longer kept")
            })?;
        let bytes = std::fs::read(at.path())
            .map_err(|why| self.hosted_fault(ATTACHMENT_REFUSED, &why.to_string()))?;
        Ok(StoredFile {
            media_type: sent.media_type,
            bytes,
        })
    }

    async fn take_held_messages(&self, ask: ipc::TakeHeld) -> Result<ipc::MessagesHeld, Refusal> {
        self.held_for(ask).await
    }

    async fn gate_session_call(&self, gate: SessionGate) -> Result<GateAnswer, Refusal> {
        Ok(self.gated(gate).await)
    }
}

/// What a message carried, as it was kept.
pub(crate) struct Kept {
    pub files: Vec<SentFile>,
    /// `(media type, base64)` for each picture.
    pub pictures: Vec<(String, String)>,
    /// Where each file that is not a picture is kept.
    pub paths: Vec<String>,
    /// Where each picture is kept, for a session that is told paths and not
    /// pictures.
    pub picture_paths: Vec<String>,
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
    pub(crate) fn hosted_refusal(&self, code: &'static str, why: &str) -> Refusal {
        Refusal::Unacceptable(WireError::raised(code, why, self.run_id()))
    }

    pub(crate) fn hosted_fault(&self, code: &'static str, why: &str) -> Refusal {
        Refusal::Fault(WireError::raised(code, why, self.run_id()))
    }

    pub(crate) fn closed(&self, id: &str) -> Refusal {
        Refusal::IllegalMove(WireError::raised(
            SESSION_CLOSED,
            format!("session {id} was closed, and a closed session takes nothing more"),
            self.run_id(),
        ))
    }

    fn ask_not_waiting(&self) -> Refusal {
        Refusal::IllegalMove(WireError::raised(
            SESSION_ASK_NOT_WAITING,
            "nothing is waiting under that call. It was answered already, or the session \
             stopped waiting",
            self.run_id(),
        ))
    }

    /// The ledger's row for `id` where a person runs it in a terminal and Fleet
    /// hosts nothing of it.
    async fn terminal_session(&self, id: &str) -> Result<Option<KeptSession>, Refusal> {
        let store = self.store().lock().await;
        let found = store.session(id).map_err(|why| self.ledger_fault(why))?;
        let hosted = store.hosting(id).map_err(|why| self.ledger_fault(why))?;
        Ok(found.filter(|one| one.origin == "terminal" && hosted.is_none()))
    }

    pub(crate) async fn session_and_hosting(
        &self,
        id: &str,
    ) -> Result<(KeptSession, KeptHosting), Refusal> {
        let store = self.store().lock().await;
        let found = store
            .session(id)
            .and_then(|session| Ok((session, store.hosting(id)?)))
            .map_err(|why| self.ledger_fault(why))?;
        match found {
            (Some(session), Some(hosting)) => Ok((session, hosting)),
            _ => Err(self.hosted_refusal(
                NO_SUCH_SESSION,
                &format!("no session Fleet hosts is named {id}"),
            )),
        }
    }

    /// Where a session's process runs: the slot once it has leased one, the
    /// repository's root before.
    pub(crate) fn directory_of(served: &Served, hosting: &KeptHosting) -> String {
        match hosting.lease_slot {
            Some(slot) => adapter_traits::slot_path(served.root(), slot),
            None => served.root().to_string(),
        }
    }

    pub(crate) fn uploads_of(&self, id: &str) -> std::path::PathBuf {
        std::path::Path::new(&self.host().attachments_dir)
            .join("sessions")
            .join(id)
    }

    /// Store what a message carried, outside every worktree.
    pub(crate) fn keep_uploads(
        &self,
        id: &str,
        uploads: &[ipc::SessionUpload],
    ) -> Result<Kept, Refusal> {
        let mut kept = Kept {
            files: Vec::new(),
            pictures: Vec::new(),
            paths: Vec::new(),
            picture_paths: Vec::new(),
        };
        if uploads.is_empty() {
            return Ok(kept);
        }
        let directory = self.uploads_of(id);
        std::fs::create_dir_all(&directory)
            .map_err(|why| self.hosted_fault(ATTACHMENT_REFUSED, &why.to_string()))?;
        for upload in uploads {
            let bytes = base64::engine::general_purpose::STANDARD
                .decode(upload.data.as_bytes())
                .map_err(|why| {
                    self.hosted_refusal(
                        ATTACHMENT_REFUSED,
                        &format!("`{}` is not base64: {why}", upload.name),
                    )
                })?;
            if bytes.len() > MOST_AN_ATTACHMENT {
                return Err(self.hosted_refusal(
                    ATTACHMENT_REFUSED,
                    &format!(
                        "`{}` is larger than {} MB",
                        upload.name,
                        MOST_AN_ATTACHMENT >> 20
                    ),
                ));
            }
            let file = format!("f{}", self.row_id(id).replace('-', ""));
            let path = directory.join(format!("{file}-{}", safe_name(&upload.name)));
            std::fs::write(&path, &bytes)
                .map_err(|why| self.hosted_fault(ATTACHMENT_REFUSED, &why.to_string()))?;
            if upload.media_type.starts_with("image/") {
                kept.pictures
                    .push((upload.media_type.clone(), upload.data.clone()));
                kept.picture_paths.push(path.to_string_lossy().into_owned());
            } else {
                kept.paths.push(path.to_string_lossy().into_owned());
            }
            kept.files.push(SentFile {
                id: file,
                name: upload.name.clone(),
                media_type: upload.media_type.clone(),
            });
        }
        Ok(kept)
    }

    /// Each session a message names, with the address another session writes
    /// to where Fleet hosts it.
    pub(crate) async fn addressed(
        &self,
        tags: &[ipc::SessionTag],
    ) -> Result<Vec<(ipc::SessionTag, Option<String>)>, Refusal> {
        let store = self.store().lock().await;
        tags.iter()
            .map(|tag| {
                let address = match tag.kind {
                    ipc::TagKind::Session => store
                        .hosting(&tag.id)
                        .map_err(|why| self.ledger_fault(why))?
                        .map(|_| address_of(&tag.id)),
                    _ => None,
                };
                Ok((tag.clone(), address))
            })
            .collect()
    }

    /// The title a first message gives, and a Job the message names held as one
    /// the session is looking at.
    async fn noted_on_the_ledger(
        &self,
        session: &KeptSession,
        tags: &[ipc::SessionTag],
        text: &str,
    ) -> Result<(), Refusal> {
        let now = self.now().as_str().to_string();
        let mut store = self.store().lock().await;
        let mut changed = false;
        for tag in tags.iter().filter(|tag| tag.kind == ipc::TagKind::Job) {
            let mut detail = std::collections::BTreeMap::new();
            detail.insert(String::from("looking"), String::from("true"));
            changed |= store
                .attach(
                    &KeptAttachment {
                        holder: Holder::session(&session.id),
                        kind: String::from("job"),
                        manifest_id: session.manifest_id.clone().unwrap_or_default(),
                        target: tag.id.clone(),
                        state: AttachmentState::Standing,
                        detail,
                        since: now.clone(),
                        changed_at: now.clone(),
                    },
                    false,
                )
                .map_err(|why| self.ledger_fault(why))?;
        }
        if session.title.is_none() {
            if let Some(title) = title_of(text) {
                let mut row = session.clone();
                row.title = Some(title);
                store
                    .keep_session(&row)
                    .map_err(|why| self.ledger_fault(why))?;
                changed = true;
            }
        }
        drop(store);
        if changed {
            self.published_hosted(&session.id).await?;
        }
        Ok(())
    }

    /// What a hosted session carries beyond a terminal's row.
    pub(crate) fn hosted_facts(
        &self,
        store: &Store,
        id: &str,
    ) -> Result<Option<HostedFacts>, store::WriteError> {
        let Some(hosting) = store.hosting(id)? else {
            return Ok(None);
        };
        let runtime = self.hosts().of(id);
        let state = runtime.state();
        Ok(Some(HostedFacts {
            turn: state.turn.clone(),
            asked: state.asked.clone(),
            model: hosting.model,
            effort: hosting.effort,
            mode: mode_of(&hosting.mode),
            running: state.process.is_some(),
            commands: if state.commands.is_empty() {
                self.hosts().commands()
            } else {
                state.commands.clone()
            },
        }))
    }

    /// The row as it stands, published whole, and answered.
    pub(crate) async fn published_hosted(&self, id: &str) -> Result<SessionRecord, Refusal> {
        let record = {
            let store = self.store().lock().await;
            let session = store
                .session(id)
                .map_err(|why| self.ledger_fault(why))?
                .ok_or_else(|| {
                    self.hosted_refusal(NO_SUCH_SESSION, &format!("no session is named {id}"))
                })?;
            self.ledger_row(&store, &session)?
        };
        self.publish(ipc::Event::SessionChanged(record.clone()));
        Ok(record)
    }

    /// Let a process go: it is ended and its late word is not read as the new
    /// one's. The caller holds the state.
    pub(crate) fn let_go(state: &mut super::State) {
        if let Some(process) = state.process.take() {
            process.end();
        }
        state.generation += 1;
        state.queued = 0;
    }

    /// Start the session's process if none is running.
    pub(crate) async fn ensure_process(
        self: &Arc<Self>,
        id: &str,
        hosting: &KeptHosting,
    ) -> Result<(), String> {
        let runtime = self.hosts().of(id);
        if runtime.state().process.is_some() {
            return Ok(());
        }
        let served = self
            .served_named(Some(&ManifestId::carried(&hosting.manifest_id)))
            .map_err(|_| format!("the repository {} is no longer served", hosting.manifest_id))?;
        let directory = Self::directory_of(&served, hosting);
        let start = Start {
            directory: directory.clone(),
            session: id.to_string(),
            resuming: hosting.ran,
            name: address_of(id),
            model: hosting.model.clone(),
            effort: hosting.effort.clone(),
            mode: mode_of(&hosting.mode),
            readable: vec![self.uploads_of(id).to_string_lossy().into_owned()],
        };
        let (sink, heard) = tokio::sync::mpsc::unbounded_channel::<Heard>();
        let process = self.hosts().processes().start(&start, sink)?;
        let generation = {
            let mut state = runtime.state();
            state.process = Some(process);
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
        Ok(())
    }

    /// Everything one process says, in order. **Boxed**, because hearing a
    /// message from one session can start another's process, which listens
    /// again, and a future that contains itself cannot be proved `Send`.
    fn listening(
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
