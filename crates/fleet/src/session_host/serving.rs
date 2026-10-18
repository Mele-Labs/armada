//! `api::HostedSessions`, over a real Fleet: start a session, take a message,
//! answer an ask, tune, close, read. Since 23.49.
//!
//! **Nothing here waits on a process.** A message is written to its input and
//! answered at once; what comes back is the thread's, a row at a time.

use std::sync::Arc;
use std::time::Duration;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::{HostedSessions, Refusal, Sessions, StoredFile};
use ipc::{
    AnswerHelmCall, AnswerSessionAsk, CloseSession, GateAnswer, HostedFacts, ManifestId,
    MoveSession, RunSessionCommand, SendSessionMessage, SessionGate, SessionId, SessionMode, SessionRecord, SessionRow,
    SessionSubagent, SessionThread, SessionTurn, SessionVoice, StartSession, StopSession, TuneSession,
    WireError,
};
use store::{Holder, KeptHosting, KeptSession};

use super::address_of;
use super::process::{Heard, Start};
use super::rows::{mention_line, new_id, UserLine};
use super::uploads::ATTACHMENT_REFUSED;
use crate::daemon::Fleet;
use crate::helm::NotAnswerable;
use crate::repositories::Served;

/// A session id that names nothing. A 422.
pub(crate) const NO_SUCH_SESSION: &str = "fleet.no_such_session";
/// A message or a tune to a session that was closed. A 409.
const SESSION_CLOSED: &str = "fleet.session_closed";
/// The ledger rows that say a session holds a slot in the pool.
const SLOT_KINDS: [&str; 2] = ["slot", "branch"];
/// A command that is not one slash command, or a session that is not a terminal one. A 422.
pub(super) const SESSION_COMMAND_REFUSED: &str = "fleet.session_command_refused";
/// A message with neither words nor a file. A 422.
pub(crate) const MESSAGE_EMPTY: &str = "fleet.session_message_empty";
/// A reattached session the keeper called busy, and that then says nothing for
/// this long, missed its turn's end: it goes to Idle.
const REATTACH_QUIET: Duration = Duration::from_secs(120);

/// The agent's process would not start. A 500.
const SESSION_UNSTARTED: &str = "fleet.session_unstarted";
/// An answer naming no ask that is waiting. A 409.
const SESSION_ASK_NOT_WAITING: &str = "fleet.session_ask_not_waiting";

/// An answer to an ask with questions that leaves one unanswered. A 422.
const SESSION_ANSWER_INCOMPLETE: &str = "fleet.session_answer_incomplete";
/// An "always" answer naming no folder, or one that does not hold the path asked about. A 422.
const SESSION_ANSWER_NOT_A_FOLDER: &str = "fleet.session_answer_not_a_folder";
/// How Fleet signs a word it sends a session: a named voice beside the sessions that write to
/// one, so no new kind of row reaches Bridge.
pub(crate) const FLEET_NAME: &str = "Fleet";

/// How a run ended, as a hosted thread keeps it in a `ran` row.
pub(crate) struct RunEnd {
    pub command: String,
    /// `None` where the run was stopped or a signal ended it.
    pub code: Option<i32>,
    /// The last lines it printed, as plain text.
    pub tail: String,
}

/// What a thread keeps of the words that start a turn.
enum Written {
    Message(SessionVoice),
    Ran(RunEnd),
    /// A row of the caller's own, in place of a message: the words are still what the agent is sent.
    Row(SessionRow),
}

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
pub(super) fn title_of(text: &str) -> Option<String> {
    let line = text.lines().map(str::trim).find(|line| !line.is_empty())?;
    Some(line.chars().take(60).collect())
}

/// A subagent's transcript as the wire has it. `parent` is the Session's resolved model and the
/// window it reported, which the subagent shares when it runs on the same model id.
fn subagent_read(thread: adapters::terminal_thread::Subagent, parent: Option<(String, u64)>) -> SessionSubagent {
    let parent = parent.as_ref().map(|(model, window)| (model.as_str(), *window));
    let model = thread.gauge.model;
    SessionSubagent {
        rows: thread.rows,
        finished: thread.finished,
        report: thread.report,
        context_tokens: thread.gauge.tokens,
        context_window: thread
            .gauge
            .tokens
            .map(|tokens| adapters::terminal_thread::window_for(tokens, model.as_deref(), parent)),
        model,
    }
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
        // Read only through `start_session_reading`, which holds the Fleet a task needs.
        start.request = None;
        if let Some(from) = start.fork.take() {
            return self.start_forked(start, from).await;
        }
        self.started_hosted(start, false).await
    }

    async fn start_session_reading(
        self: Arc<Self>,
        start: StartSession,
    ) -> Result<SessionRecord, Refusal> {
        self.started_reading(start).await
    }

    async fn hand_to_session(self: Arc<Self>, hand: ipc::HandToSession) -> Result<ipc::SessionHeld, Refusal> {
        self.group_handed(hand).await
    }

    async fn read_session_wait(self: Arc<Self>, read: ipc::ReadSessionWait) -> Result<ipc::WaitReading, Refusal> {
        self.wait_read(read).await
    }

    async fn start_waiting_now(self: Arc<Self>, start: ipc::StartWaitingNow) -> Result<SessionRecord, Refusal> {
        self.waiting_started_now(start.session_id.as_str()).await
    }

    async fn send_session_message(
        self: Arc<Self>,
        sent: SendSessionMessage,
    ) -> Result<SessionRecord, Refusal> {
        let terminal = self.terminal_session(sent.session_id.as_str()).await?;
        self.send_known(terminal, sent).await
    }

    async fn send_session_note(self: Arc<Self>, note: ipc::SendSessionNote) -> Result<ipc::NoteSent, Refusal> {
        self.note_sent(note).await
    }

    async fn answer_session_ask(&self, said: AnswerSessionAsk) -> Result<SessionRecord, Refusal> {
        let id = said.session_id.as_str().to_string();
        let terminal = self.terminal_session(&id).await?.is_some();
        if terminal {
            self.terminal_ask_standing(&id).await;
        } else {
            self.session_and_hosting(&id).await?;
        }
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
        let said_answer = said.answer;
        let answer = AnswerHelmCall {
            call: said.call,
            answer: said.answer,
            note: said.note,
            answers: said.answers,
            folder: said.folder,
        };
        self.hosts().of(&id).state().answered_via = api::via();
        let waiting = self
            .hosts()
            .asks()
            .answer(&answer)
            .map_err(|why| match why {
                NotAnswerable::Incomplete => Refusal::Unacceptable(WireError::raised(
                    SESSION_ANSWER_INCOMPLETE,
                    "that ask has questions, and each needs an answer",
                    self.run_id(),
                )),
                NotAnswerable::NotAFolderAbove => Refusal::Unacceptable(WireError::raised(
                    SESSION_ANSWER_NOT_A_FOLDER,
                    "an always answer names the folder it is for: the path asked about or one above it",
                    self.run_id(),
                )),
                _ => self.ask_not_waiting(),
            })?;
        if terminal {
            self.terminal_ask_answered(&id, &waiting, &crate::helm::Said::of(&answer))
                .await?;
        }
        let record = self.published_hosted(&id).await?;
        self.session_answered(&record, crate::timeline::card_answer_says(said_answer));
        Ok(record)
    }

    async fn answer_waiting(
        self: Arc<Self>,
        said: ipc::AnswerWaiting,
    ) -> Result<SessionRecord, Refusal> {
        self.settle_waiting(said).await
    }

    async fn dismiss_waiting(
        self: Arc<Self>,
        dismiss: ipc::DismissWaiting,
    ) -> Result<SessionRecord, Refusal> {
        self.dismiss_waited(dismiss).await
    }

    async fn get_plan_usage(&self) -> Result<ipc::PlanUsage, Refusal> {
        Ok(self.hosts().plan_usage())
    }

    async fn get_model_downshift(&self) -> Result<ipc::ModelDownshiftState, Refusal> {
        self.model_downshift_state().await
    }

    async fn accept_downshift(&self) -> Result<ipc::ModelDownshiftState, Refusal> {
        self.downshift_accepted().await
    }

    async fn restore_downshift(&self) -> Result<ipc::ModelDownshiftState, Refusal> {
        self.downshift_restored().await
    }

    async fn keep_downshift(&self) -> Result<ipc::ModelDownshiftState, Refusal> {
        self.downshift_kept().await
    }

    async fn settle_downshift(&self) -> Result<ipc::ModelDownshiftState, Refusal> {
        self.downshift_settled().await
    }

    async fn get_sleep(&self) -> Result<ipc::SleepState, Refusal> {
        self.sleep_state().await
    }

    async fn set_sleep(self: Arc<Self>, set: ipc::SetSleep) -> Result<ipc::SleepState, Refusal> {
        self.switch_sleep(set.on).await
    }

    async fn review_sleep(self: Arc<Self>, review: ipc::ReviewSleep) -> Result<ipc::SleepState, Refusal> {
        self.review_a_decision(review).await
    }

    async fn override_sleep(
        self: Arc<Self>,
        over: ipc::OverrideSleep,
    ) -> Result<ipc::SleepState, Refusal> {
        self.override_a_decision(over).await
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
        let model = tuned.model.filter(|model| !model.trim().is_empty());
        let effort = tuned.effort.filter(|effort| !effort.trim().is_empty());
        let (model_changed, effort_changed) = (hosting.model != model, hosting.effort != effort);
        hosting.model = model;
        hosting.effort = effort;
        hosting.mode = mode_text(tuned.mode).to_string();
        {
            let mut store = self.store().lock().await;
            store
                .keep_hosting(&hosting)
                .map_err(|why| self.ledger_fault(why))?;
            // What the person changed is theirs now, and a proposal still being read leaves it be.
            if model_changed {
                self.person_changed(&mut store, &id, ipc::ProposedField::Model);
            }
            if effort_changed {
                self.person_changed(&mut store, &id, ipc::ProposedField::Effort);
            }
        }
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

    async fn run_session_command(&self, run: RunSessionCommand) -> Result<SessionRecord, Refusal> {
        let id = run.session_id.as_str().to_string();
        let Some(session) = self.terminal_session(&id).await? else {
            return Err(self.hosted_refusal(
                SESSION_COMMAND_REFUSED,
                &format!("session {id} is not a terminal session, and only a terminal runs a slash command"),
            ));
        };
        self.run_terminal_command(&session, &run.command).await
    }

    async fn close_session(&self, closed: CloseSession) -> Result<SessionRecord, Refusal> {
        let id = closed.session_id.as_str().to_string();
        if let Some(session) = self.terminal_session(&id).await? {
            return self.end_terminal(&session).await;
        }
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
            state.allowed_folders.clear();
        }
        let mut still_held = Vec::new();
        if let Ok(served) = self.served_named(Some(&ManifestId::carried(&hosting.manifest_id))) {
            // Parked: what is in each slot is committed to the branch it is on and
            // nothing is pushed, so closing loses no work.
            still_held = self.park_every_slot_of(&id, &served, hosting.lease_slot).await;
        }
        // Out of the line if it was in it, and its slot, if it had one, is the next one's.
        self.hosts().slot_queue().leave(&id);
        self.hosts().slot_freed();
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
            // A slot the pool still holds for it stays on the ledger as held, or it is
            // stranded with nothing saying so. The boot pass parks it once it can.
            let holder = Holder::session(&id);
            if still_held.is_empty() {
                store.give_back(&holder, None, &now).map_err(|why| self.ledger_fault(why))?;
            } else {
                let held = store.attachments_of(&holder).map_err(|why| self.ledger_fault(why))?;
                for kind in held.iter().map(|one| one.kind.as_str()).filter(|kind| !SLOT_KINDS.contains(kind)) {
                    store.give_back(&holder, Some(kind), &now).map_err(|why| self.ledger_fault(why))?;
                }
            }
        }
        self.session_wound_up(&id).await;
        self.writes().left(&crate::clashing::Who::Session(id.clone()));
        self.published_hosted(&id).await
    }

    async fn move_session(&self, moved: MoveSession) -> Result<SessionRecord, Refusal> {
        self.moved_session(moved).await
    }

    /// Ends the turn that is running: the process goes, as it does after a quiet spell, and the
    /// next message resumes the session where it stood. The row stays live and the slot is kept.
    async fn stop_session(&self, stopped: StopSession) -> Result<SessionRecord, Refusal> {
        let id = stopped.session_id.as_str().to_string();
        if self.terminal_session(&id).await?.is_some() {
            return Err(self.hosted_refusal(
                SESSION_COMMAND_REFUSED,
                &format!("session {id} runs in a terminal, and only a hosted session's turn is stopped from here"),
            ));
        }
        let (session, _) = self.session_and_hosting(&id).await?;
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
            state.restart_after_turn = false;
            state.reattached_busy = false;
            state.woken_by.clear();
        }
        self.published_hosted(&id).await
    }

    async fn pause_session(&self, asked: ipc::PauseSession) -> Result<SessionRecord, Refusal> {
        self.session_paused(asked).await
    }

    async fn resume_session(&self, asked: ipc::ResumeSession) -> Result<SessionRecord, Refusal> {
        self.session_resumed(asked).await
    }

    async fn set_session_workflow(
        self: Arc<Self>,
        set: ipc::SetSessionWorkflow,
    ) -> Result<SessionRecord, Refusal> {
        self.workflow_set(set).await
    }

    async fn release_session_holds(
        &self,
        release: ipc::ReleaseSessionHolds,
    ) -> Result<ipc::SessionHoldsReleased, Refusal> {
        self.session_holds_released(release).await
    }

    async fn get_session(self: Arc<Self>, id: SessionId) -> Result<SessionThread, Refusal> {
        if let Some(session) = self.terminal_session(id.as_str()).await? {
            self.terminal_ask_standing(id.as_str()).await;
            let mut rows = self.terminal_thread(&session).await?;
            // The question its terminal is showing, while Bridge may still answer it.
            rows.extend(self.rows_of(id.as_str()).await?.into_iter().filter(|row| {
                matches!(
                    row,
                    SessionRow::Ask {
                        state: ipc::SessionAskState::Waiting,
                        ..
                    }
                )
            }));
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
        let rows = self.rows_of(id.as_str()).await?;
        Ok(SessionThread {
            session: record,
            rows: self.retro_rows_settled(id.as_str(), rows).await,
        })
    }

    async fn get_session_subagent(
        &self,
        id: SessionId,
        subagent: String,
    ) -> Result<SessionSubagent, Refusal> {
        let home = self.host().home.clone();
        let session = id.as_str().to_string();
        let subagent_id = subagent.clone();
        let threads = self.hosts().terminals().threads();
        let parent = self.parent_window(id.as_str()).await;
        let read = tokio::task::spawn_blocking(move || {
            adapters::terminal_thread::find_subagent(&home, &session, &subagent)
                .map(|file| threads.subagent(&file, &session, &subagent))
        })
        .await
        .map_err(|why| self.hosted_fault(ATTACHMENT_REFUSED, &why.to_string()))?;
        match read {
            Some(Ok(thread)) if thread.finished => {
                // Settled where it was read, so the ledger moves it under All without a timer.
                let _ = self
                    .report_session(ipc::SessionReport {
                        harness: String::from(adapters::HOSTED_HARNESS),
                        session_id: id.clone(),
                        fact: ipc::SessionFact::Settled {
                            attachment: ipc::AttachmentNamed {
                                kind: String::from("subagent"),
                                target: subagent_id,
                            },
                            state: ipc::AttachmentState::Spent,
                        },
                    })
                    .await;
                Ok(subagent_read(thread, parent))
            }
            Some(Ok(thread)) => Ok(subagent_read(thread, parent)),
            Some(Err(why)) => Err(self.hosted_fault(ATTACHMENT_REFUSED, &why.to_string())),
            None => {
                Err(self.hosted_refusal(ATTACHMENT_REFUSED, "that session has no such subagent"))
            }
        }
    }

    async fn get_session_file(&self, id: SessionId, file: String) -> Result<StoredFile, Refusal> {
        self.session_and_hosting(id.as_str()).await?;
        let sent = self
            .rows_of(id.as_str())
            .await?
            .into_iter()
            .find_map(|row| match row {
                SessionRow::Message { files, .. } | SessionRow::Sent { files, .. } => {
                    files.into_iter().find(|candidate| candidate.id == file)
                }
                _ => None,
            })
            .ok_or_else(|| {
                self.hosted_refusal(ATTACHMENT_REFUSED, "nothing was sent under that id")
            })?;
        self.stored_file(&self.uploads_of(id.as_str()), &file, sent.media_type)
    }

    async fn take_held_messages(&self, ask: ipc::TakeHeld) -> Result<ipc::MessagesHeld, Refusal> {
        self.held_for(ask).await
    }

    async fn ask_from_terminal(
        &self,
        ask: ipc::TerminalAsk,
    ) -> Result<ipc::TerminalAsked, Refusal> {
        match ask {
            ipc::TerminalAsk::Asks { session_id, input, tool } => {
                let asking = ipc::AskingToRun {
                    tool_name: tool.unwrap_or_else(|| String::from(ipc::ASKS_A_QUESTION)),
                    input,
                    tool_use_id: None,
                };
                self.terminal_question(&session_id, asking).await
            }
            ipc::TerminalAsk::Wait { session_id, call } => {
                self.terminal_question_wait(&session_id, &call).await
            }
            ipc::TerminalAsk::Settled {
                session_id,
                answered,
            } => self.terminal_question_settled(&session_id, answered).await,
        }
    }

    async fn gate_session_call(self: Arc<Self>, gate: SessionGate) -> Result<GateAnswer, Refusal> {
        Ok(self.gated(gate).await)
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
    /// The model the session runs on, as its process or its mod named it, and the window it last
    /// reported: what a subagent on the same model id is measured against. `None` unless both are known.
    async fn parent_window(&self, id: &str) -> Option<(String, u64)> {
        let window = self.store().lock().await.session(id).ok().flatten()?.figures.context_window?;
        let model = self.hosts().of(id).state().resolved_model.clone();
        let model = model.or_else(|| self.hosts().terminals().facts_of(id).and_then(|facts| facts.model))?;
        Some((model, window))
    }

    /// A plain hosted session's ledger row and its hosting. `helm` marks it as a repository's Helm
    /// (`helm_session.rs`), which no `start_session` call can: the mark is Fleet's to give.
    pub(crate) async fn started_hosted(
        &self,
        mut start: StartSession,
        helm: bool,
    ) -> Result<SessionRecord, Refusal> {
        let rooted = start.root.is_some();
        let (manifest, cwd) = match start.root.take() {
            Some(root) => {
                let repository = self.repository_named(Some(&root))?;
                let manifest = repository
                    .manifest()
                    .map(|one| one.id().as_str().to_string());
                (manifest, repository.root().to_string())
            }
            None => {
                let served = self.served_named(Some(&start.manifest_id))?;
                let manifest = served.manifest().id().as_str().to_string();
                (Some(manifest), served.root().to_string())
            }
        };
        // A root that has a Manifest by now is a plain start on it, not Setup's.
        let setup = rooted && manifest.is_none();
        let id = new_id();
        let now = self.now().as_str().to_string();
        let title = start
            .title
            .map(|title| title.trim().to_string())
            .filter(|title| !title.is_empty());
        // A start with a request is being read: what the person set on it stands over the answer.
        // Marked before the row is published, so the first record already says so.
        if start.request.take().is_some() {
            let set = |value: &Option<String>| value.as_ref().is_some_and(|one| !one.trim().is_empty());
            let mut person = Vec::new();
            for (field, given) in [
                (ipc::ProposedField::Title, title.is_some()),
                (ipc::ProposedField::Model, set(&start.model)),
                (ipc::ProposedField::Effort, set(&start.effort)),
            ] {
                if given {
                    person.push(field);
                }
            }
            self.hosts().of(&id).state().proposal = super::proposing::ProposalState::begin(person);
        }
        {
            let mut store = self.store().lock().await;
            store
                .keep_session(&KeptSession {
                    id: id.clone(),
                    harness: String::from(adapters::HOSTED_HARNESS),
                    origin: String::from("bridge"),
                    manifest_id: manifest.clone(),
                    cwd,
                    title,
                    state: store::SessionState::Live,
                    started_at: now.clone(),
                    last_seen_at: now,
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
                    // Empty is Setup's session, which has no Manifest to name.
                    manifest_id: manifest.unwrap_or_default(),
                    model: start.model.filter(|model| !model.trim().is_empty()),
                    effort: start.effort.filter(|effort| !effort.trim().is_empty()),
                    mode: mode_text(start.mode.unwrap_or_default()).to_string(),
                    ran: false,
                    lease_slot: None,
                    lease_branch: None,
                    fork_of: None,
                    helm,
                    setup,
                    workflow: None,
                    proposed: String::new(),
                    turn_model: None,
                    turn_effort: None,
                    paused_at: None,
                    paused_resuming: false,
                })
                .map_err(|why| self.ledger_fault(why))?;
        }
        self.published_hosted(&id).await
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
    pub(crate) async fn terminal_session(&self, id: &str) -> Result<Option<KeptSession>, Refusal> {
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

    /// Where Setup's session runs, which has no Manifest and so no slot: the
    /// root it was started in. `None` for any other session.
    pub(crate) async fn unset_up_directory(
        &self,
        id: &str,
        hosting: &KeptHosting,
    ) -> Option<String> {
        if !hosting.manifest_id.is_empty() {
            return None;
        }
        let session = self.store().lock().await.session(id).ok().flatten()?;
        Some(session.cwd)
    }

    /// Where a session's process runs: the slot once it has leased one, the
    /// repository's root before.
    pub(crate) fn directory_of(served: &Served, hosting: &KeptHosting) -> String {
        match hosting.lease_slot {
            Some(slot) => adapter_traits::slot_path(served.root(), slot),
            None => served.root().to_string(),
        }
    }

    /// Whether there is a conversation to resume: the agent CLI kept one, or the agent has
    /// left anything in the thread but the person's own messages and Fleet's note that a process
    /// ended. A process that ended before its first turn left nothing else.
    async fn has_a_conversation(&self, id: &str) -> bool {
        if adapters::terminal_thread::find(&self.host().home, id).is_some() {
            return true;
        }
        self.rows_of(id).await.is_ok_and(|rows| {
            rows.iter().any(|row| match row {
                // Fleet's own notes, a process that ended among them, are not the agent's.
                SessionRow::Message { from, .. } => *from == ipc::SessionVoice::Agent,
                // Fleet's word about a run, as its message was.
                SessionRow::Ran { .. } => false,
                _ => true,
            })
        })
    }

    /// Fleet's own word to a session, started as a turn that wakes it. **Drawn as Fleet's** in a
    /// hosted thread, and held for the mod to submit in a terminal one, whose thread Fleet cannot
    /// write to. `Ok(false)` where there is nobody to tell: the session ended, or a terminal one
    /// that is not listening, and a caller asks again later.
    pub(crate) async fn told_by_fleet(
        self: &Arc<Self>,
        session: &str,
        text: &str,
    ) -> Result<bool, Refusal> {
        self.fleet_word(session, text, false, None).await
    }

    /// [`Self::told_by_fleet`] where a hosted thread keeps `row` in place of Fleet's message, so what the
    /// person did is drawn once, as itself.
    pub(crate) async fn told_by_fleet_as(
        self: &Arc<Self>,
        session: &str,
        text: &str,
        row: SessionRow,
    ) -> Result<bool, Refusal> {
        self.fleet_word(session, text, false, Some(Written::Row(row))).await
    }

    /// [`Self::told_by_fleet`] for a run that ended. **A hosted thread keeps a `ran` row** in place
    /// of Fleet's message, and the agent is sent the same words; a terminal Session's thread is
    /// not Fleet's to write, so it holds the words as it holds any other.
    pub(crate) async fn told_of_a_run(
        self: &Arc<Self>,
        session: &str,
        text: &str,
        ran: RunEnd,
    ) -> Result<bool, Refusal> {
        self.fleet_word(session, text, false, Some(Written::Ran(ran))).await
    }

    /// [`Self::told_by_fleet`] for news that asks the agent for nothing: a pull request merged, a
    /// build moved. **The turn it starts leaves the agent's `all_done` standing**, since the agent
    /// has no work to find in it and would not call `all_done` again.
    pub(crate) async fn told_news_by_fleet(
        self: &Arc<Self>,
        session: &str,
        text: &str,
    ) -> Result<bool, Refusal> {
        self.fleet_word(session, text, true, None).await
    }

    /// [`Self::told_news_by_fleet`] where a hosted thread keeps `row` in place of Fleet's message.
    pub(crate) async fn told_news_by_fleet_as(
        self: &Arc<Self>,
        session: &str,
        text: &str,
        row: SessionRow,
    ) -> Result<bool, Refusal> {
        self.fleet_word(session, text, true, Some(Written::Row(row))).await
    }

    /// The first message of a Session that waited on another, written to its thread as the voice of the
    /// Session it waited on and started as a turn.
    pub(crate) async fn first_message_from(
        self: &Arc<Self>,
        session: &str,
        from: SessionVoice,
        text: &str,
    ) -> Result<(), Refusal> {
        let sent = SendSessionMessage {
            session_id: SessionId::carried(session),
            text: text.to_string(),
            attachments: Vec::new(),
            mentions: Vec::new(),
        };
        Arc::clone(self).send_hosted_written(sent, Written::Message(from), None, None).await.map(|_| ())
    }

    async fn fleet_word(
        self: &Arc<Self>,
        session: &str,
        text: &str,
        news: bool,
        kept: Option<Written>,
    ) -> Result<bool, Refusal> {
        if let Some(terminal) = self.terminal_session(session).await? {
            if terminal.state == store::SessionState::Ended {
                return Ok(false);
            }
            let said = format!("{FLEET_NAME}: {text}");
            return Ok(self.hosts().terminals().hold(
                session,
                said,
                super::terminal::LISTENING_FOR,
            ));
        }
        let (held, _) = self.session_and_hosting(session).await?;
        if held.state == store::SessionState::Ended {
            return Ok(false);
        }
        let sent = SendSessionMessage {
            session_id: SessionId::carried(session),
            text: text.to_string(),
            attachments: Vec::new(),
            mentions: Vec::new(),
        };
        let written = match kept {
            Some(kept) => kept,
            None => Written::Message(SessionVoice::Session {
                id: FLEET_NAME.to_lowercase(),
                title: FLEET_NAME.to_string(),
            }),
        };
        if news {
            self.hosts().of(session).state().fleet_told += 1;
        }
        Arc::clone(self)
            .send_hosted_written(sent, written, None, None)
            .await
            .map(|_| true)
    }

    /// A message to a session Fleet hosts, written to its thread as `from` and started as a turn,
    /// with a line of Fleet's own ahead of the words in the turn. **`You` is a person's; Fleet's
    /// own notices come through [`Self::told_by_fleet`].** **The thread's row keeps the words as
    /// typed**, so the line is the agent's and never drawn: Helm's "where they are" is the one
    /// caller that sets it.
    async fn send_hosted_ahead(
        self: Arc<Self>,
        sent: SendSessionMessage,
        from: SessionVoice,
        ahead: Option<String>,
        behind: Option<String>,
    ) -> Result<SessionRecord, Refusal> {
        self.send_hosted_written(sent, Written::Message(from), ahead, behind)
            .await
    }

    /// [`Self::send_hosted_ahead`] with what the thread keeps of the words chosen: a message from
    /// someone, or the `ran` row of a command that ended. `behind` is Fleet's lines after the words,
    /// as a pinned note's are, and is no more the thread's than `ahead` is.
    async fn send_hosted_written(
        self: Arc<Self>,
        sent: SendSessionMessage,
        written: Written,
        ahead: Option<String>,
        behind: Option<String>,
    ) -> Result<SessionRecord, Refusal> {
        let id = sent.session_id.as_str().to_string();
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
        let kept = self.keep_uploads(&self.uploads_of(&id), &id, &sent.attachments)?;
        let addressed = self.addressed(&sent.mentions).await?;
        // A person's first message to a Session that was started without a request is read as a request is.
        // Fleet's words and a Session's brief (`first_message_from`) are not the person's to be read.
        if matches!(written, Written::Message(SessionVoice::You)) {
            self.reading_begun(&session, &hosting, &text);
        }
        let now = self.instant();
        let row = match &written {
            Written::Message(from) => SessionRow::Message {
                id: self.row_id(&id),
                at: now,
                from: from.clone(),
                text: text.clone(),
                files: kept.files.clone(),
                tags: sent.mentions.clone(),
            },
            Written::Row(row) => row.clone(),
            Written::Ran(ran) => SessionRow::Ran {
                id: self.row_id(&id),
                at: now,
                command: ran.command.clone(),
                code: ran.code,
                tail: ran.tail.clone(),
            },
        };
        self.row_put(&id, row).await;
        if matches!(written, Written::Message(SessionVoice::You)) {
            self.noted_on_the_ledger(&session, &text).await?;
            // What the box held may be the first message another Session's agent wrote for him.
            self.first_message_sent(&id).await;
        }

        let held = super::proposing::Held {
            text,
            kept,
            addressed,
            ahead,
            behind,
        };
        // A Session whose request is still being read starts on the answer, and this turn with it.
        let runtime = self.hosts().of(&id);
        let held = {
            let mut state = runtime.state();
            match state.proposal.holds_back() {
                true => {
                    state.proposal.held.push_back(held);
                    None
                }
                false => Some(held),
            }
        };
        if let Some(held) = held {
            self.deliver_turn(&id, held).await?;
        }
        self.published_hosted(&id).await
    }

    /// Start the process if it has none, and write the turn to it.
    pub(crate) async fn deliver_turn(
        self: &Arc<Self>,
        id: &str,
        held: super::proposing::Held,
    ) -> Result<(), Refusal> {
        let super::proposing::Held {
            text,
            kept,
            addressed,
            ahead,
            behind,
        } = held;
        let (session, hosting) = self.session_and_hosting(id).await?;
        // A session closed while its turn was held takes nothing more.
        if session.state == store::SessionState::Ended {
            return Err(self.closed(id));
        }
        let runtime = self.hosts().of(id);
        self.ensure_process(id, &hosting)
            .await
            .map_err(|why| self.hosted_fault(SESSION_UNSTARTED, &why))?;
        let mut turn = String::new();
        if let Some(line) = &ahead {
            turn.push_str(line);
            turn.push_str("\n\n");
        }
        if let Some(line) = self.turn_told(id, &hosting).await {
            turn.push_str(&line);
            turn.push_str("\n\n");
        }
        if let Some(line) = mention_line(&addressed) {
            turn.push_str(&line);
            turn.push_str("\n\n");
        }
        turn.push_str(&text);
        if let Some(lines) = &behind {
            turn.push('\n');
            turn.push_str(lines);
        }
        for path in &kept.paths {
            turn.push_str(&format!("\n\nAttached file: {path}"));
        }
        let turn = super::cut_off::told(&mut runtime.state(), turn);
        let line = ipc::encode(&UserLine::of(turn, kept.pictures))
            .map_err(|why| self.hosted_refusal(SESSION_UNSTARTED, &why.to_string()))?;

        {
            let mut state = runtime.state();
            state.turn = SessionTurn::Working { woken_by: None };
            state.last_active = std::time::Instant::now();
            state.send_turn(line);
        }
        // The process said it was gone before the line was written to it.
        let lost = {
            let mut state = runtime.state();
            match (state.process.is_none(), state.retried) {
                (true, false) if !state.unstarted.is_empty() => {
                    state.retried = true;
                    std::mem::take(&mut state.unstarted)
                }
                _ => Vec::new(),
            }
        };
        if !lost.is_empty() {
            self.send_again(id, &runtime, lost).await;
        }
        Ok(())
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

    /// The title a first message gives.
    async fn noted_on_the_ledger(
        &self,
        session: &KeptSession,
        text: &str,
    ) -> Result<(), Refusal> {
        let mut store = self.store().lock().await;
        let mut changed = false;
        // A title the proposer is about to give is not made from the first message first.
        let reading = self.hosts().of(&session.id).state().proposal.reading.is_some();
        if session.title.is_none() && !reading {
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

    /// A message to a session whose kind the caller has already read: `terminal` is the session
    /// where it is a terminal one.
    pub(crate) async fn send_known(
        self: Arc<Self>,
        terminal: Option<KeptSession>,
        sent: SendSessionMessage,
    ) -> Result<SessionRecord, Refusal> {
        self.send_known_ahead(terminal, sent, None).await
    }

    /// [`Self::send_known`] with the line [`Self::send_hosted_ahead`] puts ahead of the words.
    /// Read only for a hosted session.
    pub(crate) async fn send_known_ahead(
        self: Arc<Self>,
        terminal: Option<KeptSession>,
        sent: SendSessionMessage,
        ahead: Option<String>,
    ) -> Result<SessionRecord, Refusal> {
        self.send_known_added(terminal, sent, ahead, None).await
    }

    /// [`Self::send_known_ahead`] with Fleet's lines after the words as well, for a terminal session
    /// too: a pinned note's.
    pub(crate) async fn send_known_added(
        self: Arc<Self>,
        terminal: Option<KeptSession>,
        sent: SendSessionMessage,
        ahead: Option<String>,
        behind: Option<String>,
    ) -> Result<SessionRecord, Refusal> {
        let id = sent.session_id.as_str().to_string();
        let approved = super::walking::approved_address(&sent.text).map(str::to_string);
        let noted = super::readiness::note_address(&sent.text).map(str::to_string);
        let record = if let Some(session) = terminal {
            let addressed = self.addressed(&sent.mentions).await?;
            let kept = self.keep_uploads(&self.uploads_of(&id), &id, &sent.attachments)?;
            let mut paths = kept.paths;
            paths.extend(kept.picture_paths);
            self.send_to_terminal(&session, sent, addressed, paths, behind)
                .await?
        } else {
            Arc::clone(&self)
                .send_hosted_ahead(sent, SessionVoice::You, ahead, behind)
                .await?
        };
        if let Some(url) = approved {
            self.walk_ended(&id, &url).await;
        }
        if noted.is_some() {
            self.window_noted(&id).await;
        }
        // The person spoke, so the agent's word that nothing was left no longer stands.
        let done = self.done_cleared(&id).await;
        // And so does the reply the last turn left: he has answered it.
        let replied = self.turn_end_cleared(&id).await;
        // And the person has taken the session back from the limit: Fleet does not wake it as well.
        let limited = self.limit_cleared(&id).await;
        if done || replied || limited {
            return self.published_hosted(&id).await;
        }
        Ok(record)
    }

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
    async fn process_of(
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
                    Some("Fleet came back and took the session's process again"),
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
                    note: match (hosting.helm, &served) {
                        (true, Some(served)) => Some(self.helm_note_of(served)),
                        _ => None,
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
                    self.kept_restart(id, "resumed", super::cut_off::cut_off_said(&cut).as_deref())
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
