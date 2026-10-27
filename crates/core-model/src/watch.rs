//! A Watch: a command Fleet runs on a cadence for one Job, until what it follows is finished.
//! `docs/design/fleet-watches-ci-and-deploys-for-a-job.md`.
//!
//! **The line is the whole contract with a provider.** A command's last line on standard output
//! is one JSON object, read into a [`Reading`] by `ipc::read_watch_line`, because untyped JSON is
//! read in `ipc` and `store` only. Everything after the read is here, and pure.
//!
//! **`finished` is never read from `state`.** Spike 36 found a run reads `failed` the same mid-run
//! and at its end, so a watch with a failure set aside could not otherwise tell it was over.

use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

/// What a line says of the thing it follows, or of one part of it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ReadingState {
    Waiting,
    Running,
    Passed,
    Failed,
}

impl ReadingState {
    pub const ALL: &'static [ReadingState] = &[
        ReadingState::Waiting,
        ReadingState::Running,
        ReadingState::Passed,
        ReadingState::Failed,
    ];

    /// The word a line spells it with.
    pub fn as_wire(self) -> &'static str {
        match self {
            ReadingState::Waiting => "waiting",
            ReadingState::Running => "running",
            ReadingState::Passed => "passed",
            ReadingState::Failed => "failed",
        }
    }

    pub fn from_wire(word: &str) -> Option<ReadingState> {
        ReadingState::ALL.iter().copied().find(|one| one.as_wire() == word)
    }
}

/// Where a watch stands: what its readings come to, or one of the three ends no reading says.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WatchState {
    Waiting,
    Running,
    Passed,
    Failed,
    /// Every counted part passed, and a part set aside as not ours failed. Never `Passed`: what
    /// it followed did not ship.
    NotOurs,
    /// It ran past its give-up time with nothing finished.
    GaveUp,
    /// Its command broke too many readings in a row.
    Broken,
    /// The person or the agent stopped it, or its Job ended.
    Stopped,
}

impl WatchState {
    pub const ALL: &'static [WatchState] = &[
        WatchState::Waiting,
        WatchState::Running,
        WatchState::Passed,
        WatchState::Failed,
        WatchState::NotOurs,
        WatchState::GaveUp,
        WatchState::Broken,
        WatchState::Stopped,
    ];

    /// The word the wire and the store spell it with.
    pub fn as_wire(self) -> &'static str {
        match self {
            WatchState::Waiting => "waiting",
            WatchState::Running => "running",
            WatchState::Passed => "passed",
            WatchState::Failed => "failed",
            WatchState::NotOurs => "not_ours",
            WatchState::GaveUp => "gave_up",
            WatchState::Broken => "broken",
            WatchState::Stopped => "stopped",
        }
    }

    pub fn from_wire(word: &str) -> Option<WatchState> {
        WatchState::ALL.iter().copied().find(|one| one.as_wire() == word)
    }
}

impl From<ReadingState> for WatchState {
    fn from(state: ReadingState) -> WatchState {
        match state {
            ReadingState::Waiting => WatchState::Waiting,
            ReadingState::Running => WatchState::Running,
            ReadingState::Passed => WatchState::Passed,
            ReadingState::Failed => WatchState::Failed,
        }
    }
}

/// One named part of a run, read on its own because the run's overall state misleads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Part {
    pub name: String,
    pub state: ReadingState,
    /// Where to look at this part, where the line gave one.
    pub link: Option<String>,
    /// The exact lines it failed on, where the line gave them.
    pub error: Option<String>,
    /// The end of its own log, where the line gave it.
    pub tail: Option<String>,
}

impl Part {
    /// A part with only its name and state.
    pub fn new(name: String, state: ReadingState) -> Part {
        Part { name, state, link: None, error: None, tail: None }
    }
}

/// One line a command printed, read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reading {
    pub state: ReadingState,
    /// True once what the watch follows will change no more.
    pub finished: bool,
    /// One sentence: what Bridge draws on the row and what a notice quotes.
    pub says: String,
    pub link: Option<String>,
    pub parts: Vec<Part>,
}

/// A run that gave no reading: it exited non-zero, ran out of time, or its last line was not one.
/// `why` is said to a person; the run's output is the runner's to keep.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrokenReading {
    pub why: String,
}

/// A part the agent weighed as not its change's doing, and its sentence saying why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetAside {
    pub part: String,
    pub sentence: String,
}

/// The agent's weighing of a failed reading.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    /// The change caused it, and `sentence` says how. The watch ends `failed`, and a redeploy is a
    /// new watch.
    Ours { sentence: String },
    /// Not the change's doing. `parts` stop counting toward the watch's state, and while one still
    /// reads `failed` the watch can end `not_ours` and never `passed`.
    NotOurs { sentence: String, parts: Vec<String> },
}

/// What `reading` comes to once the parts in `set_aside` stop counting.
///
/// With parts, any counted `failed` is `failed`, any `waiting` or `running` is `running`, and all
/// `passed` is `passed`. **Every part set aside is all passed** here: [`watch_state_of`] is what tells
/// that from a pass. Without parts, the line's own `state` stands.
pub fn counted_state(reading: &Reading, set_aside: &[SetAside]) -> ReadingState {
    if reading.parts.is_empty() {
        return reading.state;
    }
    let counted = || {
        reading
            .parts
            .iter()
            .filter(|part| !set_aside.iter().any(|aside| aside.part == part.name))
    };
    if counted().any(|part| part.state == ReadingState::Failed) {
        ReadingState::Failed
    } else if counted().any(|part| matches!(part.state, ReadingState::Waiting | ReadingState::Running)) {
        ReadingState::Running
    } else {
        ReadingState::Passed
    }
}

/// The parts set aside whose state in `reading` is `failed`, in the order the reading names them.
/// A set-aside part that passed on a retry is not among them.
pub fn set_aside_failing<'a>(reading: &'a Reading, set_aside: &[SetAside]) -> Vec<&'a str> {
    reading
        .parts
        .iter()
        .filter(|part| part.state == ReadingState::Failed)
        .filter(|part| set_aside.iter().any(|aside| aside.part == part.name))
        .map(|part| part.name.as_str())
        .collect()
}

/// Where `reading` puts a watch: [`counted_state`], except that a pass while a part set aside
/// still fails is `not_ours`, so a run that did not ship never reads green.
pub fn watch_state_of(reading: &Reading, set_aside: &[SetAside]) -> WatchState {
    match counted_state(reading, set_aside) {
        ReadingState::Passed if !set_aside_failing(reading, set_aside).is_empty() => WatchState::NotOurs,
        counted => counted.into(),
    }
}

/// Whether `reading` ends a watch `passed`: only `passed` once counted, nothing set aside failing,
/// and `finished`.
pub fn ends_passed(reading: &Reading, set_aside: &[SetAside]) -> bool {
    reading.finished && watch_state_of(reading, set_aside) == WatchState::Passed
}

/// Whether `reading` ends a watch by itself: `finished`, and `passed` or `not_ours`. A failure
/// waits for the agent's verdict.
pub fn ends_on(reading: &Reading, set_aside: &[SetAside]) -> bool {
    reading.finished && matches!(watch_state_of(reading, set_aside), WatchState::Passed | WatchState::NotOurs)
}

/// Why a weighing was not taken.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NotWeighed {
    /// The watch has ended, so there is nothing left to weigh.
    Ended,
    /// `ours` ends a failed watch, and this one has not failed.
    NotFailed,
    /// `not_ours` names no part, so it would set nothing aside.
    NoParts,
    /// The verdict carries no sentence saying why.
    NoSentence,
    /// A part no reading of this watch has carried.
    Unreported(String),
}

/// One watch, as the domain holds it. **No clock and no runner**: when it is read again, when it
/// gives up and how many broken readings stop it are Fleet's.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Watch {
    pub name: String,
    /// Whole argument items, every value already filled. Never read by a shell.
    pub command: Vec<String>,
    pub every_seconds: u32,
    pub gives_up_after_seconds: u32,
    /// The pull request it follows, by number, where it follows one.
    pub follows: Option<u64>,
    pub set_aside: Vec<SetAside>,
    /// Every part name a reading has carried, so a weighing can be held to them.
    pub reported: Vec<String>,
    pub last: Option<Reading>,
    pub state: WatchState,
    pub live: bool,
}

impl Watch {
    pub fn new(name: String, command: Vec<String>, every_seconds: u32, gives_up_after_seconds: u32) -> Watch {
        Watch {
            name,
            command,
            every_seconds,
            gives_up_after_seconds,
            follows: None,
            set_aside: Vec::new(),
            reported: Vec::new(),
            last: None,
            state: WatchState::Waiting,
            live: true,
        }
    }

    /// Take one reading. **An ended watch takes none**, so a run that finishes after a Stop
    /// changes nothing.
    pub fn read(&mut self, reading: Reading) -> WatchState {
        if !self.live {
            return self.state;
        }
        for part in &reading.parts {
            if !self.reported.contains(&part.name) {
                self.reported.push(part.name.clone());
            }
        }
        self.last = Some(reading);
        self.recount();
        self.state
    }

    /// Weigh the last failed reading, each verdict with its sentence. `ours` ends the watch `failed`; `not_ours` sets its parts
    /// aside, replacing a sentence already kept for one, and the watch is counted again.
    pub fn weigh(&mut self, verdict: Verdict) -> Result<WatchState, NotWeighed> {
        if !self.live {
            return Err(NotWeighed::Ended);
        }
        match verdict {
            Verdict::Ours { sentence } if sentence.trim().is_empty() => Err(NotWeighed::NoSentence),
            Verdict::Ours { .. } if self.state == WatchState::Failed => {
                self.live = false;
                Ok(self.state)
            }
            Verdict::Ours { .. } => Err(NotWeighed::NotFailed),
            Verdict::NotOurs { sentence, parts } => {
                if sentence.trim().is_empty() {
                    return Err(NotWeighed::NoSentence);
                }
                if parts.is_empty() {
                    return Err(NotWeighed::NoParts);
                }
                if let Some(unknown) = parts.iter().find(|part| !self.reported.contains(part)) {
                    return Err(NotWeighed::Unreported(unknown.clone()));
                }
                for part in parts {
                    self.set_aside.retain(|aside| aside.part != part);
                    self.set_aside.push(SetAside { part, sentence: sentence.clone() });
                }
                self.recount();
                Ok(self.state)
            }
        }
    }

    /// End it without a verdict: `GaveUp`, `Broken` or `Stopped`. A watch already ended keeps
    /// the end it had.
    pub fn end(&mut self, how: WatchState) {
        if self.live {
            self.state = how;
            self.live = false;
        }
    }

    fn recount(&mut self) {
        let Some(last) = &self.last else { return };
        self.state = watch_state_of(last, &self.set_aside);
        if ends_on(last, &self.set_aside) {
            self.live = false;
        }
    }
}

/// A span as a recipe writes it, `30s`, `2m` or `2h`, in seconds. `None` for anything else,
/// zero included.
pub fn span_seconds(written: &str) -> Option<u32> {
    let written = written.trim();
    let unit = written.chars().last()?;
    let per = match unit {
        's' => 1,
        'm' => 60,
        'h' => 60 * 60,
        _ => return None,
    };
    let count = &written[..written.len() - 1];
    if count.is_empty() || !count.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    let seconds = count.parse::<u32>().ok()?.checked_mul(per)?;
    (seconds > 0).then_some(seconds)
}

/// The value an argument item stands for: `{sha}` is `sha`. **A value fills one whole item**,
/// so `--sha={sha}` is no slot.
pub fn value_slot(item: &str) -> Option<&str> {
    let name = item.strip_prefix('{')?.strip_suffix('}')?;
    let named = !name.is_empty()
        && name
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_');
    named.then_some(name)
}

/// Why a command was not filled.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NotFilled {
    /// The command names a value nobody gave.
    Missing(String),
    /// A value the command does not name.
    Unknown(String),
    /// A value that is blank, or holds a character no argument can carry.
    Unusable(String),
}

/// `command` with each `{name}` item replaced whole by its value. Every value given must be
/// named, and every name given a value.
pub fn fill(command: &[String], values: &BTreeMap<String, String>) -> Result<Vec<String>, NotFilled> {
    if let Some(unknown) = values
        .keys()
        .find(|name| !command.iter().any(|item| value_slot(item) == Some(name.as_str())))
    {
        return Err(NotFilled::Unknown(unknown.clone()));
    }
    command
        .iter()
        .map(|item| match value_slot(item) {
            None => Ok(item.clone()),
            Some(name) => match values.get(name) {
                None => Err(NotFilled::Missing(name.to_string())),
                Some(value) if value.trim().is_empty() || value.contains('\0') => {
                    Err(NotFilled::Unusable(name.to_string()))
                }
                Some(value) => Ok(value.clone()),
            },
        })
        .collect()
}

#[cfg(test)]
mod tests;
