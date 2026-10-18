//! What Fleet decides about an Autopilot, deterministically: whether it has room, which cap it
//! has reached, and what Pause and Stop may do. **Pure**, so each is a case a test names.
//!
//! > **Rule.** Each cap stops the next pick and never a running Session.
//! > Why: a Session's cost arrives as it works, so Autopilot can overshoot a cap by as many
//! > Sessions as it runs at once.
//!
//! **A reached cap pauses, and Resume waits for it to be raised.** The owner's *fix the gaps*,
//! 12 Oct 2026: a cap that ended the run made a person start it again to carry on.

use std::fmt;

use core_model::Timestamp;

/// One of its Sessions, as much as room and Stop read of it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Standing {
    /// Not ended.
    pub live: bool,
    /// It holds something for the person: a question, a permission, a plan to approve, a done.
    pub waiting_on_a_person: bool,
    /// Its agent is at work: a turn is running, or a subagent of it is. A Session that has replied
    /// or gone idle is not.
    pub working: bool,
    /// Its agent's process has started at some point.
    pub ever_ran: bool,
}

impl Standing {
    /// **Live, at work, and not waiting on a person.** One with a call standing holds none, and
    /// neither does one that has replied or gone idle: the owner's *doesn't block the queue*, and
    /// an Autopilot Session rarely ends on its own. An ended one holds none either. One whose
    /// agent has not yet run is about to work, so it holds one.
    pub fn holds_a_place(&self) -> bool {
        self.live && (self.working || !self.ever_ran) && !self.waiting_on_a_person
    }

    /// Picked and not started: live, and its agent has never run. Stop withdraws these.
    pub fn withdrawn_at_stop(&self) -> bool {
        self.live && !self.ever_ran
    }
}

/// How many more Sessions it may start now: `at_once` less those holding a place.
pub fn room(at_once: u32, standing: &[Standing]) -> u32 {
    let holding = standing.iter().filter(|one| one.holds_a_place()).count();
    at_once.saturating_sub(u32::try_from(holding).unwrap_or(u32::MAX))
}

/// The three caps, each optional.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Caps {
    pub until: Option<Timestamp>,
    pub spend_cap_micros: Option<u64>,
    pub session_cap: Option<u32>,
}

/// What its Sessions have spent and how many it started, which the caps are held to.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Spent {
    pub micros: u64,
    pub started: u32,
}

/// Which cap stopped it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cap {
    Until,
    Spend,
    Sessions,
}

impl Cap {
    pub fn as_stored(self) -> &'static str {
        match self {
            Cap::Until => "until",
            Cap::Spend => "spend",
            Cap::Sessions => "sessions",
        }
    }

    pub fn from_stored(word: &str) -> Option<Cap> {
        [Cap::Until, Cap::Spend, Cap::Sessions]
            .into_iter()
            .find(|cap| cap.as_stored() == word)
            .or((word == "jobs").then_some(Cap::Sessions))
    }
}

impl fmt::Display for Cap {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        out.write_str(match self {
            Cap::Until => "the time to stop has passed",
            Cap::Spend => "the spend cap is reached",
            Cap::Sessions => "the Job cap is reached",
        })
    }
}

impl Caps {
    /// The cap reached, where one is: the time first, then the money, then the count, so a
    /// morning that arrives with the money spent says the morning.
    pub fn first_reached(&self, now: &Timestamp, spent: &Spent) -> Option<Cap> {
        let past = |until: &Timestamp| match (now.epoch_millis(), until.epoch_millis()) {
            (Some(now), Some(until)) => now >= until,
            // An instant that does not read is not a cap anything could keep.
            _ => true,
        };
        if self.until.as_ref().is_some_and(past) {
            return Some(Cap::Until);
        }
        if self.spend_cap_micros.is_some_and(|cap| spent.micros >= cap) {
            return Some(Cap::Spend);
        }
        if self.session_cap.is_some_and(|cap| spent.started >= cap) {
            return Some(Cap::Sessions);
        }
        None
    }
}

/// Settings a person confirmed. **Only [`Settings::confirmed`] makes one**, and a start takes
/// nothing else, so a sentence's reading never starts anything by itself.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Settings {
    instruction: String,
    at_once: u32,
    caps: Caps,
}

/// Why settings were refused at start.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Unsettable {
    NoInstruction,
    NoRoom,
    AlreadyOver(Cap),
}

impl fmt::Display for Unsettable {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        out.write_str(match self {
            Unsettable::NoInstruction => "Autopilot needs an instruction to pick by",
            Unsettable::NoRoom => "Autopilot needs room for at least one Job at once",
            Unsettable::AlreadyOver(Cap::Until) => "the time to stop has already passed",
            Unsettable::AlreadyOver(Cap::Spend) => "a spend cap of nothing would start nothing",
            Unsettable::AlreadyOver(Cap::Sessions) => "a Job cap of none would start nothing",
        })
    }
}

impl Settings {
    pub fn confirmed(
        instruction: &str,
        at_once: u32,
        caps: Caps,
        now: &Timestamp,
    ) -> Result<Settings, Unsettable> {
        let instruction = instruction.trim();
        if instruction.is_empty() {
            return Err(Unsettable::NoInstruction);
        }
        if at_once == 0 {
            return Err(Unsettable::NoRoom);
        }
        if let Some(cap) = caps.first_reached(now, &Spent::default()) {
            return Err(Unsettable::AlreadyOver(cap));
        }
        Ok(Settings {
            instruction: instruction.to_string(),
            at_once,
            caps,
        })
    }

    pub fn instruction(&self) -> &str {
        &self.instruction
    }

    pub fn at_once(&self) -> u32 {
        self.at_once
    }

    pub fn caps(&self) -> &Caps {
        &self.caps
    }
}

/// Whether it picks. `Stopped` is the end.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    Running,
    Paused,
    Stopped,
}

/// An act the state does not admit: pausing what is not running, resuming what is not paused,
/// stopping what is stopped.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NotNow {
    pub from: State,
}

/// Why a Resume was refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NotResumed {
    NotNow(NotNow),
    /// A cap is still reached: resuming would pick once and pause again.
    Capped(Cap),
}

impl State {
    pub fn picks(self) -> bool {
        self == State::Running
    }

    pub fn paused(self) -> Result<State, NotNow> {
        match self {
            State::Running => Ok(State::Paused),
            from => Err(NotNow { from }),
        }
    }

    pub fn resumed(self) -> Result<State, NotNow> {
        match self {
            State::Paused => Ok(State::Running),
            from => Err(NotNow { from }),
        }
    }

    /// Resumed, where no cap is reached now. **Any cap, not only the one that paused it**: a
    /// Resume past another cap would pause again at the next pass, having said nothing.
    pub fn resumed_under(
        self,
        caps: &Caps,
        now: &Timestamp,
        spent: &Spent,
    ) -> Result<State, NotResumed> {
        let to = self.resumed().map_err(NotResumed::NotNow)?;
        match caps.first_reached(now, spent) {
            Some(cap) => Err(NotResumed::Capped(cap)),
            None => Ok(to),
        }
    }

    pub fn stopped(self) -> Result<State, NotNow> {
        match self {
            State::Running | State::Paused => Ok(State::Stopped),
            from => Err(NotNow { from }),
        }
    }

    pub fn as_stored(self) -> &'static str {
        match self {
            State::Running => "running",
            State::Paused => "paused",
            State::Stopped => "stopped",
        }
    }

    pub fn from_stored(word: &str) -> Option<State> {
        [State::Running, State::Paused, State::Stopped]
            .into_iter()
            .find(|state| state.as_stored() == word)
    }
}

/// What became of a pick, by the word the store keeps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PickState {
    Started,
    SetAside,
    Withdrawn,
    Dismissed,
}

impl PickState {
    pub(crate) fn as_stored(self) -> &'static str {
        match self {
            PickState::Started => "started",
            PickState::SetAside => "set_aside",
            PickState::Withdrawn => "withdrawn",
            PickState::Dismissed => "dismissed",
        }
    }

    pub(crate) fn from_stored(word: &str) -> Option<PickState> {
        [
            PickState::Started,
            PickState::SetAside,
            PickState::Withdrawn,
            PickState::Dismissed,
        ]
        .into_iter()
        .find(|state| state.as_stored() == word)
    }
}
