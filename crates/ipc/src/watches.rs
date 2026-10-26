//! Watches on the wire: a command Fleet runs on a cadence for one Job, the agent's weighing of a
//! failed reading, and the line a watch's command prints.
//! `docs/design/fleet-watches-ci-and-deploys-for-a-job.md`.
//!
//! **DTOs, never `core_model::Watch`.** The domain type holds every part ever reported and the
//! last reading whole; what reaches Bridge and the agent is chosen field by field in Fleet.
//!
//! **The command's line is read here**, by [`read_watch_line`], because untyped JSON is read in
//! `ipc` and `store` only. It is not the wire to Bridge. [`watch_line`] writes it, for the readers
//! Armada itself ships, so what they print is what Fleet reads.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::ids::Instant;
use crate::SessionId;

/// What a reading or one part of it says.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WatchReadingState {
    Waiting,
    Running,
    Passed,
    Failed,
}

/// Where a watch stands. The last three are ends no reading says.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WatchState {
    Waiting,
    Running,
    Passed,
    Failed,
    GaveUp,
    Broken,
    Stopped,
}

/// One named part of a run, and what it says.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WatchPart {
    pub name: String,
    pub state: WatchReadingState,
}

/// A part the agent weighed as not its change's doing, with its sentence.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WatchSetAside {
    pub part: String,
    pub sentence: String,
}

/// One reading a watch took: what its command's line said, or why it gave none.
///
/// **In `WatchSummary::readings`, back-to-back readings that read the same are one**: `at` is the
/// first, `last_at` the last and `count` how many. Read is the same state, `finished`, `says`,
/// `link` and parts; broken is the same `why`, and `output` is the last one's. The store keeps
/// each; only the answer collapses them.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum WatchReading {
    Read {
        at: Instant,
        state: WatchReadingState,
        finished: bool,
        says: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        link: Option<String>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        parts: Vec<WatchPart>,
        /// How many readings in a row this stands for. Absent for one.
        #[serde(default = "one", skip_serializing_if = "is_one")]
        count: u32,
        /// The last of them, where it stands for more than one.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        last_at: Option<Instant>,
    },
    /// It exited non-zero, ran out of time, or its last line was not one. `why` is said to a
    /// person; `output` is the end of what it printed, where it printed anything.
    Broken {
        at: Instant,
        why: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        output: Option<String>,
        #[serde(default = "one", skip_serializing_if = "is_one")]
        count: u32,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        last_at: Option<Instant>,
    },
}

fn one() -> u32 {
    1
}

#[allow(clippy::trivially_copy_pass_by_ref)]
fn is_one(count: &u32) -> bool {
    *count == 1
}

impl WatchReading {
    /// One reading as taken, standing for itself alone.
    pub fn read(
        at: Instant,
        state: WatchReadingState,
        finished: bool,
        says: String,
        link: Option<String>,
        parts: Vec<WatchPart>,
    ) -> WatchReading {
        WatchReading::Read { at, state, finished, says, link, parts, count: 1, last_at: None }
    }

    /// One broken run as taken, standing for itself alone.
    pub fn broken(at: Instant, why: String, output: Option<String>) -> WatchReading {
        WatchReading::Broken { at, why, output, count: 1, last_at: None }
    }

    /// `next` folded into `self` where the two read the same, else `next` back.
    fn folded(&mut self, next: WatchReading) -> Option<WatchReading> {
        match (self, next) {
            (
                WatchReading::Read { state, finished, says, link, parts, count, last_at, .. },
                WatchReading::Read { at, state: s, finished: f, says: w, link: l, parts: p, .. },
            ) if (*state, *finished, &*says, &*link, &*parts) == (s, f, &w, &l, &p) => {
                *count += 1;
                *last_at = Some(at);
                None
            }
            (
                WatchReading::Broken { why, output, count, last_at, .. },
                WatchReading::Broken { at, why: y, output: o, .. },
            ) if *why == y => {
                *count += 1;
                *last_at = Some(at);
                *output = o;
                None
            }
            (_, next) => Some(next),
        }
    }
}

/// `readings`, oldest first, with each run of back-to-back readings that read the same as one.
pub fn collapsed(readings: impl IntoIterator<Item = WatchReading>) -> Vec<WatchReading> {
    let mut runs: Vec<WatchReading> = Vec::new();
    for reading in readings {
        let unfolded = match runs.last_mut() {
            Some(run) => run.folded(reading),
            None => Some(reading),
        };
        runs.extend(unfolded);
    }
    runs
}

/// The agent's weighing of a failed reading.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WatchVerdict {
    /// The change caused it, and `sentence` says how. The watch ends `failed`.
    Ours { sentence: String },
    /// Not the change's doing: `parts` stop counting toward the watch's state.
    NotOurs { sentence: String, parts: Vec<String> },
}

/// Which way the agent weighed a failed reading.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WatchVerdictWord {
    Ours,
    NotOurs,
}

/// A weighing the agent gave, as kept, with its sentence; `not_ours` with the parts it set aside.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WatchVerdictGiven {
    pub at: Instant,
    pub verdict: WatchVerdictWord,
    pub sentence: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub parts: Vec<String>,
}

/// One watch, as `list_watches` answers it and every act on one answers.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WatchSummary {
    /// Fleet's id for it, which `weigh_watch` and `stop_watch` name.
    pub id: String,
    pub session_id: SessionId,
    pub name: String,
    /// Whole, every value filled, so the person sees what Fleet runs for the Job.
    pub command: Vec<String>,
    /// The recipe it started from. Absent where the agent gave its own command.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recipe: Option<String>,
    pub every_seconds: u32,
    pub gives_up_after_seconds: u32,
    /// The pull request it follows, by number.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub follows: Option<u64>,
    /// The Requests environment whose variables its command is given. Never a value.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub environment: Option<String>,
    pub state: WatchState,
    /// False once it has ended, whichever way.
    pub live: bool,
    pub started_at: Instant,
    /// The newest reading, read or broken, as taken: never collapsed, so its `at` is its own.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last: Option<WatchReading>,
    /// The readings it kept, the newest 50, oldest first, a run that reads the same as one.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub readings: Vec<WatchReading>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub set_aside: Vec<WatchSetAside>,
    /// The agent's weighings, in the order given.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub verdicts: Vec<WatchVerdictGiven>,
    /// What a terminal watch waits for in its Job's terminal, in place of a command.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub in_terminal: Option<String>,
    /// `in_terminal` is read as a regular expression.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub regex: bool,
    /// Where the environment its command is given came from, while Fleet holds it. Absent for a
    /// terminal watch, one that has ended, and one not run since Fleet started.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub login_shell: Option<LoginShell>,
}

/// Where a watch's environment came from: the person's login shell, read when it started, or
/// Fleet's own where that read failed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum LoginShell {
    Read,
    /// Fleet's own environment, and why the login shell's was not read.
    FellBack { why: String },
}

/// `list_watches`, and the `watches.changed` event: a Job's watches, live and ended, the newest
/// first.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WatchList {
    pub session_id: SessionId,
    pub watches: Vec<WatchSummary>,
}

/// What a start runs: a recipe with its values, the agent's own command, or nothing at all, for a
/// watch on the Job's terminal.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WatchStarts {
    Recipe {
        recipe: String,
        /// Each fills one whole `{name}` item of the recipe's command, never read by a shell.
        values: BTreeMap<String, String>,
    },
    Command {
        command: Vec<String>,
        /// A span, `60s`, `2m` or `2h`.
        every: String,
        /// A span. Absent gives up after the machine's `timeouts.watchGiveUpMinutes`.
        gives_up_after: Option<String>,
    },
    /// Passes once output arriving in the Job's terminal after the start matches `text`.
    Terminal {
        text: String,
        /// `text` is a regular expression, not words to find.
        regex: bool,
        /// A span, as a command's.
        gives_up_after: Option<String>,
    },
}

/// `start_watch`. `POST /watches/start`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "StartAsWritten", into = "StartAsWritten")]
pub struct StartWatch {
    /// The Job it watches for. Read only where the connection places none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<SessionId>,
    /// What the person reads: "QA deploy".
    pub name: String,
    pub starts: WatchStarts,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub follows: Option<u64>,
    /// Parts set aside from the start, each with its sentence, so a redeploy's watch keeps what
    /// the last one learned.
    pub set_aside: Vec<WatchSetAside>,
}

/// A start as spelled: `recipe` with `values`, or `command` with `every`, flat.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StartAsWritten {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    session_id: Option<SessionId>,
    name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    recipe: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    values: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    command: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    every: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    gives_up_after: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    in_terminal: Option<String>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    regex: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    follows: Option<u64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    set_aside: Vec<WatchSetAside>,
}

impl TryFrom<StartAsWritten> for StartWatch {
    type Error = &'static str;

    fn try_from(written: StartAsWritten) -> Result<StartWatch, &'static str> {
        if written.regex && written.in_terminal.is_none() {
            return Err("`regex` reads `in_terminal` as a pattern: give `in_terminal`");
        }
        if let Some(text) = written.in_terminal {
            if written.recipe.is_some() || written.command.is_some() || written.every.is_some() || !written.values.is_empty() {
                return Err("`in_terminal` waits on the Job's terminal: leave out `recipe`, `values`, `command` and `every`");
            }
            return Ok(StartWatch {
                session_id: written.session_id,
                name: written.name,
                starts: WatchStarts::Terminal { text, regex: written.regex, gives_up_after: written.gives_up_after },
                follows: written.follows,
                set_aside: written.set_aside,
            });
        }
        let starts = match (written.recipe, written.command, written.every, written.gives_up_after) {
            (Some(recipe), None, None, None) => WatchStarts::Recipe { recipe, values: written.values },
            (Some(_), _, _, _) => {
                return Err("`recipe` names the command, how often and when to give up: leave out \
                            `command`, `every` and `gives_up_after`")
            }
            (None, Some(_), _, _) | (None, _, Some(_), _) if !written.values.is_empty() => {
                return Err("`values` fill a recipe's command: give your own command whole")
            }
            (None, Some(command), Some(every), gives_up_after) => {
                WatchStarts::Command { command, every, gives_up_after }
            }
            (None, _, _, _) => return Err("name a `recipe`, give `command` and `every`, or give `in_terminal`"),
        };
        Ok(StartWatch {
            session_id: written.session_id,
            name: written.name,
            starts,
            follows: written.follows,
            set_aside: written.set_aside,
        })
    }
}

impl From<StartWatch> for StartAsWritten {
    fn from(start: StartWatch) -> StartAsWritten {
        let (mut in_terminal, mut regex) = (None, false);
        let (recipe, values, command, every, gives_up_after) = match start.starts {
            WatchStarts::Recipe { recipe, values } => (Some(recipe), values, None, None, None),
            WatchStarts::Command { command, every, gives_up_after } => {
                (None, BTreeMap::new(), Some(command), Some(every), gives_up_after)
            }
            WatchStarts::Terminal { text, regex: pattern, gives_up_after } => {
                (in_terminal, regex) = (Some(text), pattern);
                (None, BTreeMap::new(), None, None, gives_up_after)
            }
        };
        StartAsWritten {
            session_id: start.session_id,
            name: start.name,
            recipe,
            values,
            command,
            every,
            gives_up_after,
            in_terminal,
            regex,
            follows: start.follows,
            set_aside: start.set_aside,
        }
    }
}

/// `weigh_watch`. `POST /watches/weigh`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "WeighAsWritten", into = "WeighAsWritten")]
pub struct WeighWatch {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<SessionId>,
    /// The watch's id.
    pub watch: String,
    pub verdict: WatchVerdict,
}

/// A weighing as spelled: `verdict` `ours` with `sentence`, or `not_ours` with `sentence` and
/// `parts`, flat.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct WeighAsWritten {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    session_id: Option<SessionId>,
    watch: String,
    verdict: WatchVerdictWord,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    sentence: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    parts: Vec<String>,
}

impl TryFrom<WeighAsWritten> for WeighWatch {
    type Error = &'static str;

    fn try_from(written: WeighAsWritten) -> Result<WeighWatch, &'static str> {
        let verdict = match (written.verdict, written.sentence) {
            (WatchVerdictWord::Ours, Some(sentence)) if written.parts.is_empty() => WatchVerdict::Ours { sentence },
            (WatchVerdictWord::Ours, Some(_)) => {
                return Err("`ours` ends the watch: it sets nothing aside, so leave out `parts`")
            }
            (WatchVerdictWord::Ours, None) => return Err("`ours` carries a `sentence` saying how the change caused it"),
            (WatchVerdictWord::NotOurs, Some(sentence)) => WatchVerdict::NotOurs { sentence, parts: written.parts },
            (WatchVerdictWord::NotOurs, None) => {
                return Err("`not_ours` carries a `sentence` saying why, and the `parts` to set aside")
            }
        };
        Ok(WeighWatch { session_id: written.session_id, watch: written.watch, verdict })
    }
}

impl From<WeighWatch> for WeighAsWritten {
    fn from(weigh: WeighWatch) -> WeighAsWritten {
        let (verdict, sentence, parts) = match weigh.verdict {
            WatchVerdict::Ours { sentence } => (WatchVerdictWord::Ours, Some(sentence), Vec::new()),
            WatchVerdict::NotOurs { sentence, parts } => (WatchVerdictWord::NotOurs, Some(sentence), parts),
        };
        WeighAsWritten { session_id: weigh.session_id, watch: weigh.watch, verdict, sentence, parts }
    }
}

/// `stop_watch`. `POST /watches/stop`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StopWatch {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<SessionId>,
    pub watch: String,
}

/// `delete_watch`. `POST /watches/delete`. An ended watch only.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeleteWatch {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<SessionId>,
    pub watch: String,
}

/// The line as a command prints it. Unknown fields are ignored, so a script may carry its own.
#[derive(Deserialize)]
struct Line {
    state: String,
    finished: bool,
    says: String,
    #[serde(default)]
    link: Option<String>,
    #[serde(default)]
    parts: Vec<LinePart>,
}

#[derive(Deserialize)]
struct LinePart {
    name: String,
    state: String,
}

#[derive(Serialize)]
struct LineOut<'a> {
    state: &'a str,
    finished: bool,
    says: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    link: Option<&'a str>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    parts: Vec<PartOut<'a>>,
}

#[derive(Serialize)]
struct PartOut<'a> {
    name: &'a str,
    state: &'a str,
}

/// `reading` as the one line a watch's command prints, which [`read_watch_line`] reads back whole.
pub fn watch_line(reading: &core_model::Reading) -> Result<String, crate::Unencodable> {
    crate::encode(&LineOut {
        state: reading.state.as_wire(),
        finished: reading.finished,
        says: &reading.says,
        link: reading.link.as_deref(),
        parts: reading.parts.iter().map(|part| PartOut { name: &part.name, state: part.state.as_wire() }).collect(),
    })
}

const STATES: &str = "`waiting`, `running`, `passed` or `failed`";

/// A command's whole standard output, read for its last line. **Anything but one line Fleet
/// understands is a broken reading**, and `why` says which rule it broke.
pub fn read_watch_line(stdout: &str) -> Result<core_model::Reading, core_model::BrokenReading> {
    let broken = |why: String| core_model::BrokenReading { why };
    let last = stdout
        .lines()
        .map(str::trim)
        .rfind(|line| !line.is_empty())
        .ok_or_else(|| broken(String::from("It printed nothing.")))?;
    let line: Line = crate::decode("watch line", last.as_bytes())
        .map_err(|why| broken(format!("Its last line is {why}.")))?;
    let state = |word: &str, of: &str| {
        core_model::ReadingState::from_wire(word)
            .ok_or_else(|| broken(format!("{of} is `{word}`, where it is one of {STATES}.")))
    };
    let overall = state(&line.state, "`state`")?;
    if line.says.trim().is_empty() {
        return Err(broken(String::from("`says` is blank, where it is one sentence.")));
    }
    let mut parts = Vec::with_capacity(line.parts.len());
    for (n, part) in line.parts.into_iter().enumerate() {
        if part.name.trim().is_empty() {
            return Err(broken(format!("`parts[{n}].name` is blank.")));
        }
        let read = state(&part.state, &format!("`parts[{n}].state`"))?;
        parts.push(core_model::Part { name: part.name, state: read });
    }
    Ok(core_model::Reading {
        state: overall,
        finished: line.finished,
        says: line.says,
        link: line.link.filter(|link| !link.trim().is_empty()),
        parts,
    })
}

impl From<core_model::ReadingState> for WatchReadingState {
    fn from(state: core_model::ReadingState) -> WatchReadingState {
        match state {
            core_model::ReadingState::Waiting => WatchReadingState::Waiting,
            core_model::ReadingState::Running => WatchReadingState::Running,
            core_model::ReadingState::Passed => WatchReadingState::Passed,
            core_model::ReadingState::Failed => WatchReadingState::Failed,
        }
    }
}

impl From<core_model::WatchState> for WatchState {
    fn from(state: core_model::WatchState) -> WatchState {
        match state {
            core_model::WatchState::Waiting => WatchState::Waiting,
            core_model::WatchState::Running => WatchState::Running,
            core_model::WatchState::Passed => WatchState::Passed,
            core_model::WatchState::Failed => WatchState::Failed,
            core_model::WatchState::GaveUp => WatchState::GaveUp,
            core_model::WatchState::Broken => WatchState::Broken,
            core_model::WatchState::Stopped => WatchState::Stopped,
        }
    }
}
