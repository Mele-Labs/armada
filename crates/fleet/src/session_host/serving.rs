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

mod batching;
mod conditions;
mod living;
mod sending;
mod starting;
mod telling;

pub(crate) use batching::NoticeBatches;

/// A session id that names nothing. A 422.
pub(crate) const NO_SUCH_SESSION: &str = "fleet.no_such_session";
/// A message or a tune to a session that was closed. A 409.
const SESSION_CLOSED: &str = "fleet.session_closed";
/// The ledger rows that say a session holds a slot in the pool.
pub(super) const SLOT_KINDS: [&str; 2] = ["slot", "branch"];
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
    /// Fleet's message, tagged with the notice it is, so a person can mute that notice.
    Noticed(ipc::Notice),
    /// A row of the caller's own, in place of a message: the words are still what the agent is sent.
    Row(SessionRow),
}

/// What Fleet marks a hosted session as at its start. **No `start_session` call can**: the mark is
/// Fleet's to give.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Marked {
    Plain,
    /// A repository's Helm (`helm_session.rs`).
    Helm,
    /// A Mod Job, started by `start_mod_session`.
    Mod,
    /// A Mod Job where Armada's own repository is not served: it runs in the mods folder, with no
    /// Manifest, and so never leases a slot or cuts a branch.
    ModsFolder,
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
        self.started_hosted(start, Marked::Plain).await
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

    async fn send_session_notes(self: Arc<Self>, notes: ipc::SendSessionNotes) -> Result<ipc::NotesSent, Refusal> {
        self.notes_sent(notes).await
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

    async fn confirm_waiting(
        self: Arc<Self>,
        confirm: ipc::ConfirmWaiting,
    ) -> Result<SessionRecord, Refusal> {
        self.confirm_waited(confirm).await
    }

    async fn get_plan_usage(&self) -> Result<ipc::PlanUsage, Refusal> {
        let held = self.hosts().plan_usage();
        if !held.windows.is_empty() {
            return Ok(held);
        }
        Ok(self.plan_usage_restored().await)
    }

    async fn get_model_downshift(&self) -> Result<ipc::ModelDownshiftState, Refusal> {
        self.model_downshift_state().await
    }

    async fn get_agent_sign_in(&self) -> Result<ipc::AgentSignIn, Refusal> {
        Ok(self.agent_sign_in().now())
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
                &format!("job {id} is not a terminal job, and only a terminal runs a slash command"),
            ));
        };
        self.run_terminal_command(&session, &run.command).await
    }

    async fn close_session(self: Arc<Self>, closed: CloseSession) -> Result<SessionRecord, Refusal> {
        let (ended, owed) = self.session_ended(closed).await?;
        if let Some((id, hosting)) = owed {
            // The park commits a tree and ends what runs in it: the reply does not wait for it.
            tokio::spawn(async move { self.parked_after_close(&id, hosting).await });
        }
        Ok(ended)
    }

    async fn retry_handoff(self: Arc<Self>, retry: ipc::RetryHandoff) -> Result<SessionRecord, Refusal> {
        self.handoff_retried(retry).await
    }

    async fn move_session(self: Arc<Self>, moved: MoveSession) -> Result<SessionRecord, Refusal> {
        self.moved_session(moved).await
    }

    /// Ends the turn that is running: the process goes, as it does after a quiet spell, and the
    /// next message resumes the session where it stood. The row stays live and the slot is kept.
    async fn stop_session(&self, stopped: StopSession) -> Result<SessionRecord, Refusal> {
        let id = stopped.session_id.as_str().to_string();
        if self.terminal_session(&id).await?.is_some() {
            return Err(self.hosted_refusal(
                SESSION_COMMAND_REFUSED,
                &format!("job {id} runs in a terminal, and only a hosted job's turn is stopped from here"),
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

    async fn set_session_auto_land(&self, asked: ipc::SetSessionAutoLand) -> Result<SessionRecord, Refusal> {
        self.session_auto_land_set(asked).await
    }

    async fn get_session_mcp(&self, id: SessionId) -> Result<ipc::SessionMcp, Refusal> {
        let servers = self.session_mcp(id.as_str()).await?;
        Ok(ipc::SessionMcp { servers: super::mcp_servers::on_wire(servers) })
    }

    async fn act_on_session_mcp(&self, asked: ipc::ActOnSessionMcp) -> Result<ipc::SessionMcpActed, Refusal> {
        let act = super::mcp_servers::McpAct::from(asked.act);
        let acted = self.session_mcp_acted(asked.session_id.as_str(), &asked.server, act).await?;
        Ok(ipc::SessionMcpActed {
            servers: super::mcp_servers::on_wire(acted.servers),
            sign_in: acted.sign_in.map(|sign_in| ipc::McpSignIn { url: sign_in.url, callback_expected: sign_in.callback_expected }),
        })
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

    async fn begin_session(self: Arc<Self>, begin: ipc::BeginSession) -> Result<SessionRecord, Refusal> {
        self.session_begun(begin).await
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
                Err(self.hosted_refusal(ATTACHMENT_REFUSED, "that job has no such subagent"))
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
