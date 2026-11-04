//! What the call may name for an item, and how its answer is read.
//!
//! **A ruling is refused rather than repaired.** A way the item was not offered, another Job it was
//! not shown as near, or a way between two Jobs that names no shared paths passes the item on untouched,
//! the way an id Fleet did not offer is refused elsewhere.

use std::fmt;
use std::time::Duration;

use super::arriving::Arrival;
use super::candidates::Near;

/// One way an item can go.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Way {
    /// The work goes ahead as its own Job.
    NewJob,
    /// A live Job's plan already covers it: the words go to that Job.
    ExistingJob,
    /// Two Jobs are doing the same thing, and become one.
    Merge,
    /// Separate work on shared ground: both carry on, each told of the other.
    KeepInTouch,
    /// It goes to the person, with the call's read.
    Raise,
    /// A Job gone quiet is told its next step, and carries on.
    Nudge,
}

impl Way {
    /// The ways a start is offered, in the order the call is shown them.
    pub const STARTING: &'static [Way] = &[Way::NewJob, Way::ExistingJob, Way::Merge, Way::KeepInTouch];

    pub fn word(self) -> &'static str {
        match self {
            Way::NewJob => "new_job",
            Way::ExistingJob => "existing_job",
            Way::Merge => "merge",
            Way::KeepInTouch => "keep_in_touch",
            Way::Raise => "raise",
            Way::Nudge => "nudge",
        }
    }

    pub(super) fn word_shown(self) -> &'static str {
        match self {
            Way::NewJob => "new job",
            Way::ExistingJob => "existing job",
            Way::Merge => "merge",
            Way::KeepInTouch => "keep in touch",
            Way::Raise => "raise",
            Way::Nudge => "nudge",
        }
    }

    /// Whether the way is between this item's Job and another, so must name both.
    pub fn names_another(self) -> bool {
        matches!(self, Way::ExistingJob | Way::Merge | Way::KeepInTouch)
    }
}

/// What the call ruled for one item.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ruling {
    pub way: Way,
    /// The other Job, for a way between two.
    pub other: Option<String>,
    /// The paths or plan group the two share, for a way between two.
    pub shared: Vec<String>,
    pub because: String,
    /// The next step a nudge names, in the call's words.
    pub step: Option<String>,
}

/// Why an item went on untouched. **Never a ruling**: the item goes on as it does today.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PassedOn {
    /// The call could not be made, or ended badly.
    Failed(String),
    /// No ruling came within the time the item could wait.
    Late(Duration),
    /// The answer could not be read.
    Unreadable(String),
    /// The answer named something Fleet did not offer.
    Refused(String),
    /// The day's calls reached the ceiling, so no call was made.
    Ceiling(u32),
}

impl fmt::Display for PassedOn {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PassedOn::Failed(why) => write!(out, "no ruling: {why}"),
            PassedOn::Late(waited) => write!(out, "no ruling came within {} seconds", waited.as_secs()),
            PassedOn::Unreadable(why) => write!(out, "the answer could not be read: {why}"),
            PassedOn::Refused(why) => write!(out, "the ruling was refused: {why}"),
            PassedOn::Ceiling(calls) => write!(out, "today's ceiling of {calls} calls was reached, so no call was made"),
        }
    }
}

/// What became of one item.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ruled {
    Ruled(Ruling),
    PassedOn(PassedOn),
}

/// The Log's word for what became of an item.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Ruled(Way),
    PassedOn,
}

impl Outcome {
    pub fn word(self) -> &'static str {
        match self {
            Outcome::Ruled(way) => way.word(),
            Outcome::PassedOn => "passed_on",
        }
    }
}

/// Read one answer to a call that held `items`, one ruling an item in their order, each held to
/// the Jobs `near` it. An item the answer has no block for, or a block that names what it was not
/// offered, passes on.
pub(super) fn read_rulings(answer: &str, items: &[Arrival], near: &[Vec<Near>]) -> Vec<Ruled> {
    let blocks = blocks(answer);
    items
        .iter()
        .enumerate()
        .map(|(at, item)| {
            let block = blocks.iter().find(|block| said(block, "item").and_then(|n| n.parse::<usize>().ok()) == Some(at + 1));
            match block {
                Some(block) => read_one(block, item, &near[at]).unwrap_or_else(Ruled::PassedOn),
                None => Ruled::PassedOn(PassedOn::Unreadable(format!("it gave no ruling for item {}", at + 1))),
            }
        })
        .collect()
}

fn read_one(block: &[(String, String)], item: &Arrival, near: &[Near]) -> Result<Ruled, PassedOn> {
    let because = said(block, "because").filter(|why| !why.is_empty()).ok_or_else(|| PassedOn::Unreadable("it gave no reason".into()))?;
    let named = said(block, "way").ok_or_else(|| PassedOn::Unreadable("it named no way".into()))?;
    let way = named
        .parse::<usize>()
        .ok()
        .and_then(|number| number.checked_sub(1))
        .and_then(|at| item.ways.get(at).copied())
        .ok_or_else(|| PassedOn::Refused(format!("way {named} was not one it was offered")))?;
    if way == Way::Nudge {
        let step = said(block, "step").filter(|step| !none(step)).ok_or_else(|| PassedOn::Refused("nudge named no next step".into()))?;
        return Ok(Ruled::Ruled(Ruling { way, other: None, shared: Vec::new(), because, step: Some(step) }));
    }
    if !way.names_another() {
        return Ok(Ruled::Ruled(Ruling { way, other: None, shared: Vec::new(), because, step: None }));
    }
    let other = said(block, "job").filter(|id| !none(id)).ok_or_else(|| PassedOn::Refused(format!("{} named no other Job", way.word_shown())))?;
    if !near.iter().any(|one| one.job.session_id == other) {
        return Err(PassedOn::Refused(format!("{other} is not a live Job it was shown as sharing something with it")));
    }
    let shared: Vec<String> = said(block, "shared")
        .filter(|paths| !none(paths))
        .map(|paths| paths.split(',').map(|one| one.trim().to_string()).filter(|one| !one.is_empty()).collect())
        .unwrap_or_default();
    if shared.is_empty() {
        return Err(PassedOn::Refused(format!("{} named nothing the two Jobs share", way.word_shown())));
    }
    Ok(Ruled::Ruled(Ruling { way, other: Some(other), shared, because, step: None }))
}

/// The answer cut into blocks, each starting at an `item:` line, as key and value pairs.
fn blocks(answer: &str) -> Vec<Vec<(String, String)>> {
    let mut blocks: Vec<Vec<(String, String)>> = Vec::new();
    for line in answer.lines() {
        let Some((key, value)) = line.trim().split_once(':') else { continue };
        let key = key.trim().to_ascii_lowercase();
        let value = value.trim().trim_matches('`').trim().to_string();
        if key == "item" || blocks.is_empty() {
            blocks.push(Vec::new());
        }
        if let Some(block) = blocks.last_mut() {
            block.push((key, value));
        }
    }
    blocks
}

fn said(block: &[(String, String)], key: &str) -> Option<String> {
    block.iter().find(|(named, _)| named == key).map(|(_, value)| value.clone())
}

fn none(value: &str) -> bool {
    value.is_empty() || value.eq_ignore_ascii_case("none")
}
