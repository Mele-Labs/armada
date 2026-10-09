//! What a hosted session's process says, turned into rows and turns, and what
//! one session says to another. Since 23.49.
//!
//! **A turn that nobody on the person's side started is a wake.** Fleet counts
//! the messages it wrote; a turn that opens with none outstanding was started
//! by another session's `SendMessage`, and the thread says whose.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, DroneEvent, Speaker, Vcs, WorkProduct};
use ipc::{SessionRow, SessionTurn, SessionVoice, SessionVoiceNamed};

use super::process::Heard;
use super::rows::UserLine;
use super::{address_of, Move, Runtime};
use crate::daemon::Fleet;

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
    /// One thing a process said. `generation` is the process's own: what a
    /// process let go says afterwards is not the session's.
    pub(crate) async fn heard(self: &Arc<Self>, id: &str, generation: u64, said: Heard) {
        let runtime = self.hosts().of(id);
        if runtime.state().generation != generation {
            return;
        }
        match said {
            Heard::Gone => {
                let (working, complaint) = {
                    let mut state = runtime.state();
                    let complaint = state
                        .process
                        .take()
                        .map(|process| process.complaint())
                        .unwrap_or_default();
                    state.queued = 0;
                    state.moving = None;
                    let working = matches!(state.turn, SessionTurn::Working { .. });
                    state.turn = SessionTurn::Idle;
                    (working, complaint)
                };
                // A death that said something is shown even between turns.
                if working || !complaint.is_empty() {
                    self.row_put(
                        id,
                        SessionRow::Tool {
                            id: self.row_id(id),
                            at: self.instant(),
                            text: format!("the session's process ended{complaint}"),
                        },
                    )
                    .await;
                }
                let _ = self.published_hosted(id).await;
            }
            Heard::Attached { busy } => {
                {
                    let mut state = runtime.state();
                    state.last_active = std::time::Instant::now();
                    if busy {
                        state.turn = SessionTurn::Working { woken_by: None };
                    }
                }
                let _ = self.published_hosted(id).await;
            }
            Heard::Commands(names) => {
                self.hosts().heard_commands(names.clone());
                runtime.state().commands = names;
                let _ = self.published_hosted(id).await;
            }
            Heard::Events(events) => {
                runtime.state().last_active = std::time::Instant::now();
                for event in events {
                    self.hear(id, &runtime, event).await;
                }
            }
        }
    }

    async fn hear(self: &Arc<Self>, id: &str, runtime: &Arc<Runtime>, event: DroneEvent) {
        match event {
            DroneEvent::Started { .. } => {
                {
                    let mut state = runtime.state();
                    state.last_active = std::time::Instant::now();
                    if state.queued > 0 {
                        state.queued -= 1;
                        state.turn = SessionTurn::Working { woken_by: None };
                    } else {
                        let by = (!state.woken_by.is_empty()).then(|| state.woken_by.remove(0));
                        state.turn = SessionTurn::Working { woken_by: by };
                    }
                }
                let _ = self.published_hosted(id).await;
            }
            DroneEvent::Said {
                text,
                by: Speaker::Drone,
            } => {
                // What is said between the lease and the move is the agent
                // standing down, and the person is not shown it.
                if runtime.state().moving.is_some() || text.trim().is_empty() {
                    return;
                }
                self.row_put(
                    id,
                    SessionRow::Message {
                        id: self.row_id(id),
                        at: self.instant(),
                        from: SessionVoice::Agent,
                        text,
                        files: Vec::new(),
                        tags: Vec::new(),
                    },
                )
                .await;
            }
            DroneEvent::Called { tool, detail, .. } => {
                let shown = detail.whole().unwrap_or(detail.shown()).to_string();
                if let Some((to, message)) = adapters::sent_message(&tool, &shown) {
                    self.delivered(id, to, message).await;
                }
                let home = std::env::var("HOME").unwrap_or_default();
                if let Some(path) = adapters::written_document(&tool, &shown, &home) {
                    self.artifact_made(id, path, "file").await;
                }
                if let Some(path) = adapters::viewed_image(&tool, &shown, &home) {
                    self.artifact_made(id, path, "image").await;
                }
                let text = if shown.is_empty() {
                    tool
                } else {
                    format!("{tool} {shown}")
                };
                self.row_put(
                    id,
                    SessionRow::Tool {
                        id: self.row_id(id),
                        at: self.instant(),
                        text,
                    },
                )
                .await;
            }
            DroneEvent::Ended { cost_micros, .. } => {
                self.turn_ended(id, runtime, cost_micros).await;
            }
            _ => {}
        }
    }

    async fn turn_ended(self: &Arc<Self>, id: &str, runtime: &Arc<Runtime>, cost_micros: u64) {
        let now = self.now().as_str().to_string();
        {
            let mut store = self.store().lock().await;
            if let Ok(Some(mut session)) = store.session(id) {
                session.last_turn_at = Some(now.clone());
                session.last_seen_at = now;
                let before = session.figures.cost_micros.unwrap_or(0);
                session.figures.cost_micros = Some(before.max(cost_micros));
                let _ = store.keep_session(&session);
            }
        }
        let (moving, restart) = {
            let mut state = runtime.state();
            state.last_active = std::time::Instant::now();
            (state.moving.take(), state.restart_after_turn)
        };
        if let Some(moving) = moving {
            self.moved_into(id, moving).await;
            return;
        }
        {
            let mut state = runtime.state();
            state.restart_after_turn = false;
            if restart {
                Self::let_go(&mut state);
            }
            state.turn = if state.queued > 0 {
                SessionTurn::Working { woken_by: None }
            } else {
                SessionTurn::Idle
            };
        }
        let _ = self.published_hosted(id).await;
    }

    /// The turn that held a write is over: end the process and resume the
    /// session with the slot as its directory, then tell it where it is.
    async fn moved_into(self: &Arc<Self>, id: &str, moving: Move) {
        let runtime = self.hosts().of(id);
        Self::let_go(&mut runtime.state());
        let Ok((_, hosting)) = self.session_and_hosting(id).await else {
            return;
        };
        let arrived = format!(
            "You are now in {}, a worktree leased for you on branch {}. Carry on with what you \
             were doing, including the write that was held.",
            moving.directory, moving.branch
        );
        let started = self.ensure_process(id, &hosting).await;
        let line = ipc::encode(&UserLine::of(arrived, Vec::new()));
        match (started, line) {
            (Ok(()), Ok(line)) => {
                let mut state = runtime.state();
                state.queued += 1;
                state.turn = SessionTurn::Working { woken_by: None };
                if let Some(process) = &state.process {
                    process.send(line);
                }
            }
            (started, _) => {
                runtime.state().turn = SessionTurn::Idle;
                self.row_put(
                    id,
                    SessionRow::Tool {
                        id: self.row_id(id),
                        at: self.instant(),
                        text: format!(
                            "could not move the session into its slot: {}",
                            started.err().unwrap_or_default()
                        ),
                    },
                )
                .await;
            }
        }
        let _ = self.published_hosted(id).await;
    }

    /// One session's `SendMessage` to another that Fleet hosts: the receiving
    /// thread draws it as that session's, and a receiver whose process was
    /// ended is resumed with it.
    async fn delivered(self: &Arc<Self>, from: &str, to: &str, message: &str) {
        let (target, title, hosting) = {
            let store = self.store().lock().await;
            let Ok(hostings) = store.hostings() else {
                return;
            };
            let Some(hosting) = hostings
                .into_iter()
                .find(|hosting| address_of(&hosting.session_id) == to)
            else {
                return;
            };
            let sender = store.session(from).ok().flatten();
            let title = sender
                .and_then(|session| session.title)
                .unwrap_or_else(|| address_of(from));
            (hosting.session_id.clone(), title, hosting)
        };
        if target == from {
            return;
        }
        self.row_put(
            &target,
            SessionRow::Message {
                id: self.row_id(&target),
                at: self.instant(),
                from: SessionVoice::Session {
                    id: from.to_string(),
                    title: title.clone(),
                },
                text: message.to_string(),
                files: Vec::new(),
                tags: Vec::new(),
            },
        )
        .await;
        let runtime = self.hosts().of(&target);
        runtime.state().woken_by.push(SessionVoiceNamed {
            id: from.to_string(),
            title: title.clone(),
        });
        if runtime.state().process.is_some() {
            return;
        }
        let said = format!(
            "A message from session \"{title}\" (address {}): {message}",
            address_of(from)
        );
        if self.ensure_process(&target, &hosting).await.is_err() {
            return;
        }
        if let Ok(line) = ipc::encode(&UserLine::of(said, Vec::new())) {
            if let Some(process) = &runtime.state().process {
                process.send(line);
            }
        }
    }
}
