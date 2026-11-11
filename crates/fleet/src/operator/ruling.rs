//! What the call may name for an item, and how its answer is read.
//!
//! **A ruling is refused rather than repaired.** A way the item was not offered, another Job it was
//! not shown as near, or a way between two Jobs that names no shared paths passes the item on untouched,
//! the way an id Fleet did not offer is refused elsewhere. A link to an issue the item was not
//! offered is refused whole.

use std::fmt;
use std::time::Duration;

use super::arriving::Arrival;
use super::candidates::Near;
use super::Paused;

/// One way an item can go.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Way {
    /// The work goes ahead as its own Job.
    NewJob,
    /// A live Job's plan already covers it: the words go to that Job.
    ExistingJob,
    /// It goes to the person, with the call's read.
    Raise,
    /// A report or an issue is one of the open issues it was offered, and is linked to it.
    Link,
    /// A note or an error is filed as a new issue in the repository's beads.
    File,
    /// An open issue a merged pull request finished without naming it is closed, done by that pull request.
    Close,
    /// A new issue that duplicates an open one is closed as its duplicate, with a link to the one it repeats.
    Group,
    /// A suggestion that repeats one kept before is read and nothing follows: the Log says which it repeats.
    /// It is never offered to a call.
    Noted,
    /// A note the person sent to a Job stays where he sent it. Nothing follows and nothing is in Done.
    Leave,
}

impl Way {
    pub fn word(self) -> &'static str {
        match self {
            Way::NewJob => "new_job",
            Way::ExistingJob => "existing_job",
            Way::Raise => "raise",
            Way::Link => "link",
            Way::File => "file",
            Way::Close => "close",
            Way::Group => "group",
            Way::Noted => "noted",
            Way::Leave => "leave",
        }
    }

    /// The Log's outcome for this way kept as a proposal.
    pub(super) fn proposed_word(self) -> &'static str {
        match self {
            Way::NewJob => "proposed_new_job",
            Way::ExistingJob => "proposed_existing_job",
            Way::Raise => "proposed_raise",
            Way::Link => "proposed_link",
            Way::File => "proposed_file",
            Way::Close => "proposed_close",
            Way::Group => "proposed_group",
            Way::Noted => "proposed_noted",
            Way::Leave => "proposed_leave",
        }
    }

    /// The way a word from the store names.
    pub(super) fn from_word(word: &str) -> Option<Way> {
        [Way::NewJob, Way::ExistingJob, Way::Raise, Way::Link, Way::File, Way::Close, Way::Group, Way::Noted, Way::Leave].into_iter().find(|way| way.word() == word)
    }

    pub(super) fn word_shown(self) -> &'static str {
        match self {
            Way::NewJob => "new job",
            Way::ExistingJob => "existing job",
            Way::Raise => "raise",
            Way::Link => "link to an open issue",
            Way::File => "file a new issue",
            Way::Close => "close the open issue it lists, which the merged work finished",
            Way::Group => "close it as a duplicate of the open issue it lists",
            Way::Noted => "note it",
            Way::Leave => "leave it with the Job he sent it to",
        }
    }

    /// Whether the way is between this item's Job and another, so must name both.
    pub fn names_another(self) -> bool {
        self == Way::ExistingJob
    }
}

/// Where a Job goes in Waiting to start, among the Jobs waiting there.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Place {
    Ahead(String),
    Behind(String),
    Last,
}

impl Place {
    /// As the store keeps it.
    pub fn word(&self) -> String {
        match self {
            Place::Ahead(id) => format!("ahead:{id}"),
            Place::Behind(id) => format!("behind:{id}"),
            Place::Last => "last".to_string(),
        }
    }

    /// The Job it is placed against, where it names one.
    pub fn against(&self) -> Option<&str> {
        match self {
            Place::Ahead(id) | Place::Behind(id) => Some(id),
            Place::Last => None,
        }
    }
}

impl fmt::Display for Place {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Place::Ahead(id) => write!(out, "ahead of {id}"),
            Place::Behind(id) => write!(out, "behind {id}"),
            Place::Last => write!(out, "last"),
        }
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
    /// Each part Fleet refused while the rest stood, in a line.
    pub refused: Vec<String>,
    /// The open issue a link, a close or a group names, one of those the item was offered.
    pub issue: Option<String>,
}

impl Ruling {
    /// A ruling of `way` alone, naming no other Job.
    pub fn alone(way: Way, because: impl Into<String>) -> Ruling {
        Ruling { way, other: None, shared: Vec::new(), because: because.into(), refused: Vec::new(), issue: None }
    }
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
    /// The week's plan is used to the Operator's pause, so no call was made.
    Paused(Paused),
    /// The ruling came back after the Operator was turned off, so nothing was done.
    Off,
    /// The ruling came back, and Fleet could not carry it out.
    Undone(String),
}

impl fmt::Display for PassedOn {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PassedOn::Failed(why) => write!(out, "no ruling: {why}"),
            PassedOn::Late(waited) => write!(out, "no ruling came within {} seconds", waited.as_secs()),
            PassedOn::Unreadable(why) => write!(out, "the answer could not be read: {why}"),
            PassedOn::Refused(why) => write!(out, "the ruling was refused: {why}"),
            PassedOn::Paused(paused) => write!(out, "the week's plan is {}% used, past the Operator's pause at {}%, so no call was made", paused.week, paused.at),
            PassedOn::Off => write!(out, "the Operator was turned off while its call was out, so nothing was done"),
            PassedOn::Undone(why) => write!(out, "it could not be carried out: {why}"),
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
    /// The person moved a Job in Waiting to start, over the Operator's place.
    Corrected,
    /// A hold of a standing order lifted: the Job it waited on moved as the order said.
    HoldLifted,
    /// The person pressed Skip on what the Operator would have done.
    Skipped,
    /// The person sent a note to a Job himself, from Bridge.
    Sent,
    /// The ruling was kept to wait on a press. **Not in Done**: the press is the row.
    Proposed(Way),
}

impl Outcome {
    pub fn word(self) -> &'static str {
        match self {
            Outcome::Ruled(way) => way.word(),
            Outcome::PassedOn => "passed_on",
            Outcome::Corrected => "corrected",
            Outcome::HoldLifted => "hold_lifted",
            Outcome::Skipped => "skipped",
            Outcome::Sent => "sent",
            Outcome::Proposed(way) => way.proposed_word(),
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
                Some(block) => read_one(block, item, &near[at]).map(Ruled::Ruled).unwrap_or_else(Ruled::PassedOn),
                None => Ruled::PassedOn(PassedOn::Unreadable(format!("it gave no ruling for item {}", at + 1))),
            }
        })
        .collect()
}

fn read_one(block: &[(String, String)], item: &Arrival, near: &[Near]) -> Result<Ruling, PassedOn> {
    let because = said(block, "because").filter(|why| !why.is_empty()).ok_or_else(|| PassedOn::Unreadable("it gave no reason".into()))?;
    let named = said(block, "way").ok_or_else(|| PassedOn::Unreadable("it named no way".into()))?;
    let way = named
        .parse::<usize>()
        .ok()
        .and_then(|number| number.checked_sub(1))
        .and_then(|at| item.ways.get(at).copied())
        .ok_or_else(|| PassedOn::Refused(format!("way {named} was not one it was offered")))?;
    if matches!(way, Way::Link | Way::Close | Way::Group) {
        let issue = said(block, "issue").filter(|issue| !none(issue)).ok_or_else(|| PassedOn::Refused(format!("{} named no issue", way.word())))?;
        if !item.open.iter().any(|one| one.reference == issue) {
            return Err(PassedOn::Refused(format!("{issue} is not an open issue it was offered")));
        }
        return Ok(Ruling { issue: Some(issue), ..Ruling::alone(way, because) });
    }
    if !way.names_another() {
        return Ok(Ruling::alone(way, because));
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
    Ok(Ruling { other: Some(other), shared, ..Ruling::alone(way, because) })
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
