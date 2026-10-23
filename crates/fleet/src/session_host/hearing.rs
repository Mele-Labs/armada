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
use crate::build_keeping::build_heard;
use crate::build_record::Observed;
use crate::session_links::LinkedBy;
use crate::timeline::{Doer, FileAct, Seen};

/// How long after a turn ends a line Fleet wrote may take to begin the next one. A process takes a
/// queued line at once, so a longer wait is a line it folded into the turn that just ended.
const NEXT_TURN_WITHIN: std::time::Duration = if cfg!(test) {
    std::time::Duration::from_millis(300)
} else {
    std::time::Duration::from_secs(10)
};

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
                let (working, complaint, lost) = {
                    let mut state = runtime.state();
                    let gone = state.process.take();
                    let complaint = gone.as_ref().map(|p| p.complaint()).unwrap_or_default();
                    // Its socket is taken away, so a replacement is not found
                    // at it: the keeper of a gone agent would say so again.
                    if let Some(gone) = &gone {
                        gone.end();
                    }
                    state.queued = 0;
                    state.moving = None;
                    let working = matches!(state.turn, SessionTurn::Working { .. });
                    state.turn = SessionTurn::Idle;
                    // Lines it never took are not lost with it: once.
                    let lost = match state.retried {
                        true => Vec::new(),
                        false => std::mem::take(&mut state.unstarted),
                    };
                    state.unstarted.clear();
                    state.retried = !lost.is_empty();
                    // Dropped, so whoever waits hears the process ended rather than waiting it out.
                    state.controls.clear();
                    (working, complaint, lost)
                };
                if working {
                    self.kept_restart(
                        id,
                        "ended",
                        Some("the job's process ended while a turn ran"),
                    )
                    .await;
                }
                // A death that said something is shown even between turns.
                if working || !complaint.is_empty() {
                    self.fleet_note(id, format!("The job's process ended{complaint}"))
                        .await;
                }
                if !lost.is_empty() {
                    self.send_again(id, &runtime, lost).await;
                }
                let _ = self.published_hosted(id).await;
            }
            Heard::Answered(answer) => {
                // None waits on a line answered after its wait ran out, or replayed after a reattach.
                let waiter = runtime.state().controls.remove(&answer.request_id);
                if let Some(waiter) = waiter {
                    let _ = waiter.send(answer.outcome);
                }
            }
            Heard::Attached { busy } => {
                {
                    let mut state = runtime.state();
                    state.last_active = std::time::Instant::now();
                    if busy {
                        state.turn = SessionTurn::Working { woken_by: None };
                        state.reattached_busy = true;
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
                {
                    let mut state = runtime.state();
                    state.last_active = std::time::Instant::now();
                    state.reattached_busy = false;
                }
                for event in events {
                    self.hear(id, &runtime, event).await;
                }
            }
            Heard::Ran(records) => {
                let came_back = records.iter().any(|record| matches!(record, adapters::session_runs::Record::Done { .. }));
                self.runs_heard(&runtime, records);
                // A command that came back is kept now, so a Job started before this turn ends is
                // told it; the place is written when the turn ends.
                if came_back {
                    let (fleet, id, runtime) = (Arc::clone(self), id.to_string(), Arc::clone(&runtime));
                    tokio::spawn(async move { fleet.runs_kept(&id, &runtime).await });
                }
            }
            Heard::Spawned(calls) => {
                let mut state = runtime.state();
                for (call, kind) in calls {
                    state.spawns.told(&call, &kind);
                }
            }
            Heard::Subagent { events, call } => {
                {
                    let mut state = runtime.state();
                    state.last_active = std::time::Instant::now();
                    state.reattached_busy = false;
                }
                // A subagent line that names no call is still a subagent's, not the agent's own.
                let call = call.unwrap_or_default();
                for event in &events {
                    self.work_heard(id, event, Some(&call)).await;
                }
            }
        }
    }

    /// Start a replacement for a process that was gone before it took these
    /// lines, which resumes the conversation, and write them to it.
    pub(crate) async fn send_again(
        self: &Arc<Self>,
        id: &str,
        runtime: &Arc<Runtime>,
        lines: Vec<String>,
    ) {
        let started = match self.session_and_hosting(id).await {
            Ok((_, hosting)) => self.ensure_process(id, &hosting).await,
            Err(_) => Err(String::from("the job could not be read")),
        };
        match started {
            Ok(()) => {
                let mut state = runtime.state();
                state.turn = SessionTurn::Working { woken_by: None };
                state.last_active = std::time::Instant::now();
                for line in lines {
                    state.send_turn(line);
                }
            }
            Err(why) => {
                self.fleet_note(id, format!("The job's process would not start again: {why}"))
                    .await;
            }
        }
    }

    /// A tool call of the session's, or of a subagent in it: entered on the timeline and heard by the
    /// build record, whichever tool made it.
    async fn work_heard(self: &Arc<Self>, id: &str, event: &DroneEvent, subagent: Option<&str>) {
        self.touch_heard(id, event, subagent).await;
        self.background_heard(id, event).await;
        // A terminal session's own mod tells these; a hosted one is heard here.
        if let Some(done) = crate::timeline::tool_of(&**self.harness(), event) {
            let (doer, cwd) = match self.store().lock().await.session(id) {
                Ok(Some(session)) => (Doer::of_session(&session), Some(session.cwd)),
                _ => (Doer::session(id, None, None), None),
            };
            let edited = matches!(done, Seen::File(FileAct::Edit, _));
            match done {
                Seen::File(act, path) => {
                    let file = self.file_named(cwd.as_deref(), &path);
                    self.timeline().touched(doer, act, 1, Some(file));
                }
                Seen::Ran(command) => self.timeline().ran(doer, command),
                Seen::Called(reach) => self.timeline().called(doer, reach),
            }
            let heard = if edited { Observed::Edited } else { Observed::Worked };
            let now = self.now().as_str().to_string();
            let shown = build_heard(&mut *self.store().lock().await, id, &heard, &now).unwrap_or(false);
            if shown {
                let _ = self.published_hosted(id).await;
            }
        }
    }

    /// The files a tool call changed, kept for the Field and the plan: who made the edit is the
    /// subagent whose call the line sat under, else the Session's own agent. Throttled as a call is.
    async fn touch_heard(self: &Arc<Self>, id: &str, event: &DroneEvent, subagent: Option<&str>) {
        let touched = crate::file_touching::touches_of(&**self.harness(), event);
        if touched.is_empty() {
            return;
        }
        let by = subagent.map(|call| {
            let kind = self.hosts().of(id).state().spawns.kind_of(call).unwrap_or("subagent").to_string();
            (kind, call.to_string())
        });
        let now = self.now().as_str().to_string();
        let home = self.host().home.clone();
        let shown = {
            let mut store = self.store().lock().await;
            let Ok(Some(session)) = store.session(id) else { return };
            let worktree = self.worktree_of(&session.cwd);
            let mut shown = false;
            for (path, act) in touched {
                let Some(path) = crate::file_touching::rooted(&worktree, &session.cwd, &home, &path) else { continue };
                shown |= crate::plan_keeping::touch_kept(&mut store, id, by.clone(), &path, act, &now).unwrap_or(false);
            }
            shown
        };
        if shown {
            let step = self.hosts().of(id).state().activity.changed(self.hosts().activity_every());
            self.publish_step(id.to_string(), step);
        }
    }

    async fn hear(self: &Arc<Self>, id: &str, runtime: &Arc<Runtime>, event: DroneEvent) {
        self.work_heard(id, &event, None).await;
        match event {
            DroneEvent::Started { model, .. } => {
                self.limit_cleared(id).await;
                let began = self.now().as_str().to_string();
                let noticed = {
                    let mut state = runtime.state();
                    let noticed = state.fleet_told > 0;
                    state.fleet_told = state.fleet_told.saturating_sub(1);
                    noticed
                };
                // A turn begun again is work the agent had said was over. Fleet's own notice
                // is not: the agent has nothing new to do, and does not call `all_done` again.
                if !noticed {
                    self.done_cleared(id).await;
                    // And it ends the reply the turn before left.
                    self.turn_end_cleared(id).await;
                }
                {
                    let mut state = runtime.state();
                    // A turn Fleet's own notice began is a new turn all the same.
                    state.activity.turn_began();
                    state.turn_began = Some(began);
                    state.said = None;
                    state.resolved_model = (!model.is_empty()).then_some(model);
                    state.last_active = std::time::Instant::now();
                    if state.queued > 0 {
                        state.queued -= 1;
                        if !state.unstarted.is_empty() {
                            state.unstarted.remove(0);
                        }
                        state.retried = false;
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
                by: by @ (Speaker::Drone | Speaker::Harness),
            } => {
                // What is said between the lease and the move is the agent
                // standing down, and the person is not shown it.
                let text = adapters::terminal_thread::agent_said(&text);
                if runtime.state().moving.is_some() || text.trim().is_empty() {
                    return;
                }
                // The harness's words are shown but are not the agent's reply, so a turn
                // ending on them asks the person nothing.
                if by == Speaker::Drone {
                    runtime.state().said = Some(text.clone());
                    self.said_seen(id, &text);
                    self.links_seen(id, &text, LinkedBy::Agent).await;
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
                        notice: None,
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
        let mut named = (None, None);
        let mut cwd = None;
        {
            let mut store = self.store().lock().await;
            if let Ok(Some(mut session)) = store.session(id) {
                named = (session.title.clone(), session.manifest_id.clone());
                cwd = Some(session.cwd.clone());
                session.last_turn_at = Some(now.clone());
                session.last_seen_at = now.clone();
                let before = session.figures.cost_micros.unwrap_or(0);
                session.figures.cost_micros = Some(before.max(cost_micros));
                let _ = store.keep_session(&session);
            }
        }
        // Setup's session may have written its root's armada.yml this turn: load it, as Write does.
        if let (None, Some(cwd)) = (&named.1, &cwd) {
            if let Some(repository) = self.repositories().at(cwd) {
                self.set_up_after_write(&repository);
            }
        }
        let (moving, restart, began, said) = {
            let mut state = runtime.state();
            state.last_active = std::time::Instant::now();
            (state.moving.take(), state.restart_after_turn, state.turn_began.take(), state.said.take())
        };
        // What the Job ran this far is remembered, off the turn so the board is not held for a file.
        {
            let (fleet, id, runtime) = (Arc::clone(self), id.to_string(), Arc::clone(runtime));
            tokio::spawn(async move { fleet.runs_learnt(&id, &runtime).await });
        }
        // How the turn ended, read off what the agent said and never off a tool it called. A turn held
        // for a slot and resumed in it is one turn to the person, so that move's end is not entered.
        if moving.is_none() {
            self.turn_end_kept(id, &now, said.as_deref()).await;
        }
        // A done or a walk the reply claims without its call. Read here and sent once the turn is
        // settled below, since a restart after the turn lets the process go with what was written to it.
        let claim = match &moving {
            None => self.unbacked_claim(id, &now, said.as_deref()).await,
            Some(_) => None,
        };
        // **Once, where the turn ends**, never on the rows between. A turn held for a slot and
        // resumed in it is one turn to the person, so that move's end is not entered.
        if moving.is_none() {
            self.session_turn(id, named, began.as_deref(), &now);
        }
        if let Some(moving) = moving {
            self.moved_into(id, moving).await;
            return;
        }
        let late = {
            let mut state = runtime.state();
            state.restart_after_turn = false;
            state.reattached_busy = false;
            if restart {
                Self::let_go(&mut state);
            }
            state.turn = if state.queued > 0 {
                SessionTurn::Working { woken_by: None }
            } else {
                SessionTurn::Idle
            };
            // A slot granted to a queued session while this turn was ending.
            match state.turn {
                SessionTurn::Idle => state.moving.take(),
                _ => None,
            }
        };
        if let Some(late) = late {
            self.moved_into(id, late).await;
            return;
        }
        if let Some(claim) = claim {
            self.claim_reminded(id, claim).await;
        }
        let _ = self.published_hosted(id).await;
        if runtime.state().queued > 0 {
            self.next_turn_awaited(id, runtime);
        } else {
            let (fleet, id) = (Arc::clone(self), id.to_string());
            tokio::spawn(async move { fleet.slots_reclaimed(&id).await });
        }
    }

    /// A turn ended with lines Fleet wrote still uncounted by a start. The process folds lines that
    /// reach it together into one turn, so Fleet's count can sit above its turns for good, and the
    /// session would read as working with nothing running. **If no turn has begun within
    /// [`NEXT_TURN_WITHIN`], the count is wrong and the session is idle.**
    fn next_turn_awaited(self: &Arc<Self>, id: &str, runtime: &Arc<Runtime>) {
        let (fleet, id, runtime) = (Arc::clone(self), id.to_string(), Arc::clone(runtime));
        tokio::spawn(async move {
            tokio::time::sleep(NEXT_TURN_WITHIN).await;
            let missed = {
                let mut state = runtime.state();
                let missed = state.queued > 0
                    && state.turn_began.is_none()
                    && matches!(state.turn, SessionTurn::Working { .. });
                if missed {
                    state.queued = 0;
                    state.unstarted.clear();
                    state.turn = SessionTurn::Idle;
                }
                missed
            };
            if missed {
                let _ = fleet.published_hosted(&id).await;
            }
        });
    }

    /// The turn that held a write is over: end the process and resume the
    /// session with the slot as its directory, then tell it where it is.
    /// **What was written to the old process and not started goes to the new one after**, as Fleet's
    /// notices told while the write was held: `let_go` clears them with the process.
    pub(super) async fn moved_into(self: &Arc<Self>, id: &str, moving: Move) {
        let runtime = self.hosts().of(id);
        let unstarted = {
            let mut state = runtime.state();
            let unstarted = std::mem::take(&mut state.unstarted);
            Self::let_go(&mut state);
            unstarted
        };
        let Ok((_, hosting)) = self.session_and_hosting(id).await else {
            return;
        };
        let arrived = format!(
            "You are now in {}, a worktree leased for you on branch {}. Carry on with what you \
             were doing, including the write that was held.",
            moving.directory, moving.branch
        );
        let started = self.ensure_process(id, &hosting).await;
        let arrived = super::cut_off::told(&mut runtime.state(), arrived);
        let line = ipc::encode(&UserLine::of(arrived, Vec::new()));
        match (started, line) {
            (Ok(()), Ok(line)) => {
                let mut state = runtime.state();
                state.turn = SessionTurn::Working { woken_by: None };
                state.send_turn(line);
                for line in unstarted {
                    state.send_turn(line);
                }
            }
            (started, _) => {
                runtime.state().turn = SessionTurn::Idle;
                self.fleet_note(
                    id,
                    format!(
                        "Could not move the job into its slot: {}",
                        started.err().unwrap_or_default()
                    ),
                )
                .await;
            }
        }
        let _ = self.published_hosted(id).await;
    }

    /// One session's `SendMessage` to another that Fleet hosts: the receiving
    /// thread draws it as that session's, the timeline enters it unless Helm sent
    /// it, and a receiver whose process was ended is resumed with it.
    async fn delivered(self: &Arc<Self>, from: &str, to: &str, message: &str) {
        let (target, title, named, hosting, from_helm) = {
            let store = self.store().lock().await;
            let Ok(hostings) = store.hostings() else {
                return;
            };
            let from_helm = hostings.iter().any(|one| one.session_id == from && one.helm);
            let Some(hosting) = hostings
                .into_iter()
                .find(|hosting| address_of(&hosting.session_id) == to)
            else {
                return;
            };
            let sender = store.session(from).ok().flatten();
            let named = sender.map(|session| (session.title, session.manifest_id)).unwrap_or_default();
            let title = named.0.clone().unwrap_or_else(|| address_of(from));
            (hosting.session_id.clone(), title, named, hosting, from_helm)
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
                notice: None,
            },
        )
        .await;
        if !from_helm {
            self.session_messaged(from, named, &target, message);
        }
        let runtime = self.hosts().of(&target);
        runtime.state().woken_by.push(SessionVoiceNamed {
            id: from.to_string(),
            title: title.clone(),
        });
        if runtime.state().process.is_some() {
            return;
        }
        let said = format!(
            "A message from job \"{title}\" (address {}): {message}",
            address_of(from)
        );
        if self.ensure_process(&target, &hosting).await.is_err() {
            return;
        }
        let said = super::cut_off::told(&mut runtime.state(), said);
        if let Ok(line) = ipc::encode(&UserLine::of(said, Vec::new())) {
            if let Some(process) = &runtime.state().process {
                process.send(line);
            }
        }
    }
}
