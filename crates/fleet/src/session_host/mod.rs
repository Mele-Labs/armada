//! The sessions Fleet hosts for Bridge. Since 23.49.
//! `docs/concepts/session.md`, *A session Fleet hosts*.
//!
//! **Beside Helm and changing nothing about it.** A hosted session is a ledger
//! row with `origin` `bridge`, and a live process, a thread and a lease taken
//! on its first write besides. `serving` is every command, `hearing` turns what
//! a process says into rows and turns, `gate` is the lease, `activity` is what
//! its tool calls show it doing, `asking` is the permission door and `rows`
//! writes the thread.

pub(crate) mod activity;
pub(crate) mod after;
mod agent_starting;
mod annotating;
mod asking;
mod backgrounding;
mod cut_off;
pub(crate) mod disputing;
pub(crate) mod dispute_review;
pub(crate) mod dispute_reading;
pub(crate) mod ending;
pub(crate) mod asking_in_window;
pub(crate) mod showing_file;
pub(crate) mod sketching;
pub(crate) mod following;
mod finishing;
mod fork_point;
mod forking;
mod gate;
mod job_of_its_own;
mod handing;
pub(crate) mod handoff_answers;
pub(crate) mod replacing;
pub(crate) mod linking;
mod hearing;
pub mod keeper;
/// A hosted job's MCP servers, read and acted on through control lines: `get_session_mcp`,
/// `act_on_session_mcp`.
pub(crate) mod mcp_servers;
pub(crate) mod kept;
/// `send_session_note` and `send_session_notes`: pinned notes, with the lines of code each was
/// drawn from.
mod noting;
mod auto_landing;
mod pausing;
mod reach_folders;
pub(crate) mod delivering;
pub(crate) mod planning;
mod places;
mod process;
pub(crate) mod proposing;
pub mod queueing;
pub(crate) mod slot_asking;
pub(crate) mod releasing;
mod rows;
#[cfg(test)]
pub(crate) use rows::mention_line;
/// A judge reading a Session's work, and sending or dismissing what it raised.
pub(crate) mod session_judging;
mod serving;
pub(crate) mod asks;
pub(crate) mod asks_model;
pub(crate) mod clarity;
pub(crate) mod dedup;
pub(crate) mod extracting;
pub(crate) mod tell_refusal;
pub(crate) mod reply_claims;
pub(crate) mod plan_asks;
pub(crate) mod standing;
pub(crate) mod workflow_pick;
mod standing_tick;
mod thread_cache;
pub(crate) use agent_starting::UNSENT_ITEM;
pub(crate) use serving::{mode_of, RunEnd, MESSAGE_EMPTY, NO_SUCH_SESSION};
mod terminal;
pub(crate) mod uploads;
pub(crate) mod waiting;
pub(crate) mod readiness;
pub(crate) mod walking;
pub(crate) mod walk_asks;

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use ipc::{HelmCallInFlight, SessionTurn};

use crate::helm::{Asks, HelmAskHold};

pub use process::{Heard, Process, ProcessHost, Processes, Sink, Start};
pub use terminal::Terminals;
pub(crate) use terminal::PromptOpen;

/// How long a session's process may sit with no turn before Fleet ends it, and
/// the next message resumes it. **`settings.session-quiet-timeout`**.
pub const SHIPPED_QUIET_TIMEOUT: Duration = Duration::from_secs(10 * 60);

/// Everything Fleet hosts of sessions: the host that starts a process, what is
/// waiting on a person, and each session's runtime state.
pub struct Hosts {
    processes: Arc<dyn Processes>,
    quiet: Duration,
    /// How long a running turn may be heard nothing from before the Session reads as stuck.
    stuck_after: Mutex<Duration>,
    /// The Sessions last published as stuck, so a Session that goes quiet, or is heard from again, is
    /// published when it does and not only when something else moves. Memory only.
    stuck: Mutex<std::collections::HashSet<String>>,
    asks: Asks,
    runtime: Mutex<HashMap<String, Arc<Runtime>>>,
    minted: AtomicU64,
    sweeping: AtomicBool,
    /// The commands the last process to start said it had, for a session whose
    /// own has not started. Since 23.51.
    commands: Mutex<Vec<String>>,
    /// What Fleet holds for sessions run in a terminal. Since 23.53.
    terminals: Terminals,
    /// The newest reading of the account's plan windows, from whichever session reported last.
    /// Each fresh one is kept in the store too, and a restarted Fleet takes the kept one back with
    /// its spent windows dropped, until a session reports a newer.
    plan: Mutex<ipc::PlanUsage>,
    /// Sessions refused a slot because the pool was full, and waiting for one.
    slot_queue: Arc<queueing::SlotQueue>,
    /// Held while a repository's Helm Session is looked for and made, so two first messages make one.
    helm_starting: tokio::sync::Mutex<()>,
    /// The most often a session's row goes out for what it is doing, in milliseconds.
    activity_every: AtomicU64,
    /// Waiting items, by session and item, whose answer or removal is being delivered now.
    settling: Arc<Mutex<std::collections::HashSet<(String, String)>>>,
}

/// One waiting item claimed while its answer or removal is delivered, let go when dropped. **Why
/// it exists**: the item comes off the list only after the send, so a second press inside that
/// send found it still open, and the agent was told the same answer once per press.
pub(crate) struct Settling {
    held: Arc<Mutex<std::collections::HashSet<(String, String)>>>,
    key: (String, String),
}

impl Drop for Settling {
    fn drop(&mut self) {
        self.held.lock().expect("held across no panic").remove(&self.key);
    }
}

impl Hosts {
    pub fn hosted_by(processes: Arc<dyn Processes>, quiet: Duration, hold: HelmAskHold) -> Hosts {
        Hosts {
            processes,
            quiet,
            stuck_after: Mutex::new(standing::STUCK_AFTER),
            stuck: Mutex::new(std::collections::HashSet::new()),
            asks: Asks::holding_for(hold),
            runtime: Mutex::new(HashMap::new()),
            minted: AtomicU64::new(0),
            sweeping: AtomicBool::new(false),
            commands: Mutex::new(Vec::new()),
            terminals: Terminals::default(),
            plan: Mutex::new(ipc::PlanUsage::default()),
            slot_queue: Arc::default(),
            helm_starting: tokio::sync::Mutex::new(()),
            activity_every: AtomicU64::new(activity::PUBLISH_EVERY.as_millis() as u64),
            settling: Arc::default(),
        }
    }

    /// Claim a waiting item for one answer or removal. `None` where another is being delivered.
    pub(crate) fn settling(&self, session: &str, item: &str) -> Option<Settling> {
        let key = (session.to_string(), item.to_string());
        let fresh = self.settling.lock().expect("held across no panic").insert(key.clone());
        fresh.then(|| Settling { held: Arc::clone(&self.settling), key })
    }

    pub(crate) fn activity_every(&self) -> Duration {
        Duration::from_millis(self.activity_every.load(Ordering::Relaxed))
    }

    /// A shorter wait than the shipped one, so a test does not sit through it.
    #[cfg(test)]
    pub(crate) fn publish_activity_every(&self, every: Duration) {
        self.activity_every.store(every.as_millis() as u64, Ordering::Relaxed);
    }

    /// Where a test plants the bound a running turn is stuck past, so it need not outlive ten minutes.
    #[cfg(test)]
    pub(crate) fn plant_stuck_after(&self, stuck_after: Duration) {
        *self.stuck_after.lock().expect("held across no panic") = stuck_after;
    }

    pub(crate) fn stuck_after(&self) -> Duration {
        *self.stuck_after.lock().expect("held across no panic")
    }

    /// Every Session with a turn running and how long nothing has been heard from it. A turn held on
    /// a question or a permission is left out: it waits on the person, which is not stuck.
    pub(crate) fn turns_quiet(&self) -> Vec<(String, Duration)> {
        let hosted: Vec<(String, Arc<Runtime>)> = self
            .runtime
            .lock()
            .expect("held across no panic")
            .iter()
            .map(|(id, runtime)| (id.clone(), Arc::clone(runtime)))
            .collect();
        let mut quiet: Vec<(String, Duration)> = hosted
            .into_iter()
            .filter_map(|(id, runtime)| {
                let state = runtime.state();
                (matches!(state.turn, SessionTurn::Working { .. }) && state.asked.is_none())
                    .then(|| (id, state.last_active.elapsed()))
            })
            .collect();
        let asking: Vec<String> = self
            .terminals
            .running_quiet()
            .into_iter()
            .map(|(id, _)| id)
            .filter(|id| self.of(id).state().asked.is_some() || self.terminals.prompt_of(id).is_some())
            .collect();
        quiet.extend(self.terminals.running_quiet().into_iter().filter(|(id, _)| !asking.contains(id)));
        quiet
    }

    /// The Sessions whose stuck standing moved since the last look, given the ones stuck now: those
    /// that went quiet past the bound and those that were heard from again.
    pub(crate) fn stuck_moved(&self, now: std::collections::HashSet<String>) -> Vec<String> {
        let mut was = self.stuck.lock().expect("held across no panic");
        let moved = was.symmetric_difference(&now).cloned().collect();
        *was = now;
        moved
    }

    pub(crate) fn helm_starting(&self) -> &tokio::sync::Mutex<()> {
        &self.helm_starting
    }

    pub(crate) fn slot_queue(&self) -> &Arc<queueing::SlotQueue> {
        &self.slot_queue
    }

    /// A slot was given back, or the pool grew: the sessions waiting for one are served.
    pub(crate) fn slot_freed(&self) {
        self.slot_queue.freed();
    }

    /// The reading held.
    pub(crate) fn plan_usage(&self) -> ipc::PlanUsage {
        self.plan.lock().expect("the reading is not held across a panic").clone()
    }

    /// Take the reading the store kept as the one held, where none is. What is held after, which is a
    /// fresher one where a session reported first.
    pub(crate) fn plan_restored(&self, kept: ipc::PlanUsage) -> ipc::PlanUsage {
        let mut held = self.plan.lock().expect("the reading is not held across a panic");
        if held.windows.is_empty() {
            *held = kept;
        }
        held.clone()
    }

    /// Keep a session's reading as the newest. `Some` of the reading now where it differs from the
    /// one held. An empty report is a session off a subscription: the reading where none was taken,
    /// so Bridge stops waiting for one, and nothing where windows are held.
    pub(crate) fn plan_read(&self, windows: Vec<ipc::PlanWindow>, at: &str) -> Option<ipc::PlanUsage> {
        let mut held = self.plan.lock().expect("the reading is not held across a panic");
        if windows.is_empty() && (!held.windows.is_empty() || held.read_at.is_some()) {
            return None;
        }
        if !windows.is_empty() && held.windows == windows {
            return None;
        }
        *held = ipc::PlanUsage {
            windows,
            read_at: Some(ipc::Instant::carried(at)),
        };
        Some(held.clone())
    }

    pub(crate) fn terminals(&self) -> &Terminals {
        &self.terminals
    }

    /// Remember what a process said it has.
    pub(crate) fn heard_commands(&self, names: Vec<String>) {
        *self
            .commands
            .lock()
            .expect("the commands are not held across a panic") = names;
    }

    /// The commands the last process said it had.
    pub(crate) fn commands(&self) -> Vec<String> {
        self.commands
            .lock()
            .expect("the commands are not held across a panic")
            .clone()
    }

    pub fn quiet(&self) -> Duration {
        self.quiet
    }

    pub(crate) fn asks(&self) -> &Asks {
        &self.asks
    }

    pub(crate) fn processes(&self) -> &Arc<dyn Processes> {
        &self.processes
    }

    /// The runtime state of one session, made on first ask.
    pub(crate) fn of(&self, session: &str) -> Arc<Runtime> {
        let mut held = self
            .runtime
            .lock()
            .expect("the runtime table is not held across a panic");
        Arc::clone(held.entry(session.to_string()).or_insert_with(|| {
            Arc::new(Runtime {
                lease: tokio::sync::Mutex::new(()),
                writing_retro: AtomicBool::new(false),
                state: Mutex::new(State::new()),
            })
        }))
    }

    /// Every session with a runtime, by id.
    pub(crate) fn all(&self) -> Vec<(String, Arc<Runtime>)> {
        self.runtime
            .lock()
            .expect("the runtime table is not held across a panic")
            .iter()
            .map(|(id, runtime)| (id.clone(), Arc::clone(runtime)))
            .collect()
    }

    /// A row id no other row of the session has had.
    pub(crate) fn next_row(&self, session: &str, at: &str) -> String {
        let n = self.minted.fetch_add(1, Ordering::SeqCst);
        format!("{}-{at}-{n}", &session[..session.len().min(8)])
    }

    pub(crate) fn start_sweeping(&self) -> bool {
        !self.sweeping.swap(true, Ordering::SeqCst)
    }
}

/// What another session addresses this one by. **Derived from the id and
/// nothing else**, so it is the same after a restart of Fleet and of the
/// session.
pub fn address_of(session: &str) -> String {
    format!("s-{}", &session[..session.len().min(8)])
}

/// The lease a session is waiting to be moved into.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Move {
    pub slot: u32,
    pub branch: String,
    pub directory: String,
}

/// One session's state that lives only as long as Fleet.
pub(crate) struct Runtime {
    /// Held while a lease is taken, so two writes in one turn lease one slot.
    pub(crate) lease: tokio::sync::Mutex<()>,
    /// Whether this session's retro is being written, so a second press waits.
    pub(crate) writing_retro: AtomicBool,
    state: Mutex<State>,
}

pub(crate) struct State {
    pub process: Option<Arc<dyn Process>>,
    pub turn: SessionTurn,
    /// Messages Fleet wrote whose turn has not started.
    pub queued: u32,
    /// The slot the session was leased, waiting for its turn to end so the
    /// process can be resumed there.
    pub moving: Option<Move>,
    /// Sessions that wrote to this one and whose message has not yet started a
    /// turn.
    pub woken_by: Vec<ipc::SessionVoiceNamed>,
    /// Fleet's own notices whose turn has not started. A turn one of them begins leaves the agent's
    /// `all_done` standing.
    pub fleet_told: u32,
    pub asked: Option<HelmCallInFlight>,
    /// The door the answer to `asked` came through, set by the request that
    /// answers and taken by the ask that was waiting, which is another task.
    pub answered_via: Option<ipc::Via>,
    /// Folders past its slots the owner allowed for this session alone. In memory, never written,
    /// and cleared when the session is closed.
    pub allowed_folders: Vec<String>,
    pub last_active: Instant,
    /// A tune arrived while a turn was running: end the process after it.
    pub restart_after_turn: bool,
    /// Where the process runs, so a lease is told from the main checkout.
    pub directory: String,
    /// Which process of the session is the live one, so the late word of one
    /// that was let go is not read as the new one's.
    pub generation: u64,
    /// What this session's own process said it has. Empty until it starts.
    pub commands: Vec<String>,
    /// The model the process's `init` line said the CLI resolved. Absent until it
    /// starts, and again once the process is let go. Since 23.85.
    pub resolved_model: Option<String>,
    /// The lines written whose turn has not started, kept so a process that
    /// is gone before it took them is replaced and they are sent again.
    pub unstarted: Vec<String>,
    /// Those lines have been sent to a replacement once already.
    pub retried: bool,
    /// The subagents the last restart cut off, told to the agent with its next turn.
    pub cut_off: Vec<String>,
    /// The keeper said busy on reattach and nothing has been heard since, so
    /// a long silence is a turn end that was missed.
    pub reattached_busy: bool,
    /// When the turn running now began, by Fleet's clock, for the timeline's account of how long it
    /// took. Set when the process says a turn started and taken when one ends.
    pub turn_began: Option<String>,
    /// The last thing the agent said to the person in the turn running now, so the turn's end can say
    /// how it ended. Cleared when a turn starts. In memory.
    pub said: Option<String>,
    /// This is a repository's Helm Session: its door is Helm's, which places its calls.
    pub helm: bool,
    /// What its last `get_events_since` was answered, kept for Helm's debug record. In memory.
    pub polled: Option<ipc::EventsSince>,
    /// What its tool calls show it doing. In memory.
    pub activity: activity::Activity,
    /// Whether the proposer is reading the request it was dispatched with, and the turns that
    /// wait for the answer. In memory.
    pub proposal: proposing::ProposalState,
    /// The type each subagent call of the stream asked for, to name who made an edit. In memory.
    pub spawns: crate::file_touching::Spawns,
    /// How many judges are reading the Session's work now. In memory: a Fleet that restarts reads none.
    pub judging: u32,
    /// The shell and watch calls of the stream, to pair with the task each starts. In memory.
    pub background: backgrounding::Calls,
    /// The commands the session ran and what came back, for `crate::run_memory`. In memory.
    pub runs: crate::run_memory::RunLog,
    /// Who waits on each control line written to the process, by its request id. In memory.
    pub controls: HashMap<String, mcp_servers::Waiter>,
}

impl State {
    /// Write one line to the process as a turn of its own.
    pub fn send_turn(&mut self, line: String) {
        self.queued += 1;
        self.reattached_busy = false;
        self.unstarted.push(line.clone());
        if let Some(process) = &self.process {
            process.send(line);
        }
    }

    fn new() -> State {
        State {
            process: None,
            turn: SessionTurn::Idle,
            queued: 0,
            moving: None,
            woken_by: Vec::new(),
            fleet_told: 0,
            asked: None,
            answered_via: None,
            allowed_folders: Vec::new(),
            last_active: Instant::now(),
            restart_after_turn: false,
            directory: String::new(),
            generation: 0,
            commands: Vec::new(),
            resolved_model: None,
            unstarted: Vec::new(),
            retried: false,
            cut_off: Vec::new(),
            reattached_busy: false,
            turn_began: None,
            said: None,
            helm: false,
            polled: None,
            activity: activity::Activity::default(),
            proposal: proposing::ProposalState::default(),
            spawns: crate::file_touching::Spawns::default(),
            judging: 0,
            background: backgrounding::Calls::default(),
            runs: crate::run_memory::RunLog::default(),
            controls: HashMap::new(),
        }
    }
}

impl Runtime {
    pub(crate) fn state(&self) -> std::sync::MutexGuard<'_, State> {
        self.state
            .lock()
            .expect("a session's state is not held across a panic")
    }
}
