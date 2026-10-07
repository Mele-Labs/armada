//! The sessions Fleet hosts for Bridge. Since 23.49.
//! `docs/concepts/session.md`, *A session Fleet hosts*.
//!
//! **Beside Helm and changing nothing about it.** A hosted session is a ledger
//! row with `origin` `bridge`, and a live process, a thread and a lease taken
//! on its first write besides. `serving` is every command, `hearing` turns what
//! a process says into rows and turns, `gate` is the lease, `asking` is the
//! permission door and `rows` writes the thread.

mod asking;
mod forking;
mod gate;
mod hearing;
mod piloting;
mod places;
mod process;
mod rows;
mod serving;
mod terminal;

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use ipc::{HelmCallInFlight, SessionTurn};

use crate::helm::{Asks, HelmAskHold};

pub use process::{Heard, Process, ProcessHost, Processes, Sink, Start};
pub use terminal::Terminals;

/// How long a session's process may sit with no turn before Fleet ends it, and
/// the next message resumes it. **`settings.session-quiet-timeout`**.
pub const SHIPPED_QUIET_TIMEOUT: Duration = Duration::from_secs(10 * 60);

/// Everything Fleet hosts of sessions: the host that starts a process, what is
/// waiting on a person, and each session's runtime state.
pub struct Hosts {
    processes: Arc<dyn Processes>,
    quiet: Duration,
    asks: Asks,
    runtime: Mutex<HashMap<String, Arc<Runtime>>>,
    minted: AtomicU64,
    sweeping: AtomicBool,
    /// The commands the last process to start said it had, for a session whose
    /// own has not started. Since 23.51.
    commands: Mutex<Vec<String>>,
    /// What Fleet holds for sessions run in a terminal. Since 23.53.
    terminals: Terminals,
}

impl Hosts {
    pub fn hosted_by(processes: Arc<dyn Processes>, quiet: Duration, hold: HelmAskHold) -> Hosts {
        Hosts {
            processes,
            quiet,
            asks: Asks::holding_for(hold),
            runtime: Mutex::new(HashMap::new()),
            minted: AtomicU64::new(0),
            sweeping: AtomicBool::new(false),
            commands: Mutex::new(Vec::new()),
            terminals: Terminals::default(),
        }
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
    pub asked: Option<HelmCallInFlight>,
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
}

impl State {
    fn new() -> State {
        State {
            process: None,
            turn: SessionTurn::Idle,
            queued: 0,
            moving: None,
            woken_by: Vec::new(),
            asked: None,
            last_active: Instant::now(),
            restart_after_turn: false,
            directory: String::new(),
            generation: 0,
            commands: Vec::new(),
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
