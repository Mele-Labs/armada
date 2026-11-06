//! What the call may name for an item, and how its answer is read.
//!
//! **A ruling is refused rather than repaired.** A way the item was not offered, another Job it was
//! not shown as near, or a way between two Jobs that names no shared paths passes the item on untouched,
//! the way an id Fleet did not offer is refused elsewhere. **A place or a first that names a Job not
//! offered is refused alone**: the way stands, the start goes last, and nothing is held. A link to
//! an issue the item was not offered is refused whole.

use std::fmt;
use std::time::Duration;

use super::arriving::{Arrival, InLine};
use super::candidates::Near;
use super::questions::Choice;

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
    /// A question or a permission card is settled as the person's press would.
    Answer,
    /// A report or a Later item is one of the open issues it was offered, and is linked to it.
    Link,
    /// A Later item is filed as a new issue in the repository's beads.
    File,
    /// An end or a merge is read and nothing follows: it goes to Done.
    Noted,
    /// A note the person sent to a Job stays where he sent it. Nothing follows and nothing is in Done.
    Leave,
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
            Way::Answer => "answer",
            Way::Link => "link",
            Way::File => "file",
            Way::Noted => "noted",
            Way::Leave => "leave",
        }
    }

    /// The Log's outcome for this way kept as a proposal.
    pub(super) fn proposed_word(self) -> &'static str {
        match self {
            Way::NewJob => "proposed_new_job",
            Way::ExistingJob => "proposed_existing_job",
            Way::Merge => "proposed_merge",
            Way::KeepInTouch => "proposed_keep_in_touch",
            Way::Raise => "proposed_raise",
            Way::Answer => "proposed_answer",
            Way::Link => "proposed_link",
            Way::File => "proposed_file",
            Way::Noted => "proposed_noted",
            Way::Leave => "proposed_leave",
        }
    }

    /// The way a word from the store names.
    pub(super) fn from_word(word: &str) -> Option<Way> {
        [Way::NewJob, Way::ExistingJob, Way::Merge, Way::KeepInTouch, Way::Raise, Way::Answer, Way::Link, Way::File, Way::Noted, Way::Leave].into_iter().find(|way| way.word() == word)
    }

    pub(super) fn word_shown(self) -> &'static str {
        match self {
            Way::NewJob => "new job",
            Way::ExistingJob => "existing job",
            Way::Merge => "merge",
            Way::KeepInTouch => "keep in touch",
            Way::Raise => "raise",
            Way::Answer => "answer",
            Way::Link => "link to an open issue",
            Way::File => "file a new issue",
            Way::Noted => "noted, nothing follows",
            Way::Leave => "leave it with the Job he sent it to",
        }
    }

    /// Whether the way is between this item's Job and another, so must name both.
    pub fn names_another(self) -> bool {
        matches!(self, Way::ExistingJob | Way::Merge | Way::KeepInTouch)
    }
}

/// Where a start goes in Waiting to start, among the Jobs Fleet offered.
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

    /// As the call writes it: `ahead of <id>`, `behind <id>` or `last`.
    fn read(said: &str) -> Option<Place> {
        let said = said.trim();
        if said.eq_ignore_ascii_case("last") {
            return Some(Place::Last);
        }
        let (how, id) = said.split_once(char::is_whitespace)?;
        let id = id.trim();
        match how.to_ascii_lowercase().as_str() {
            "ahead" => Some(Place::Ahead(id.strip_prefix("of ").unwrap_or(id).trim().to_string())),
            "behind" => Some(Place::Behind(id.to_string())),
            _ => None,
        }
        .filter(|place| place.against().is_some_and(|id| !id.is_empty()))
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
    /// Where a start goes in Waiting to start, where Jobs wait there.
    pub place: Option<Place>,
    /// Of two kept in touch, the one that builds first: the other is held until it pushes.
    pub first: Option<String>,
    /// Each part Fleet refused while the rest stood, in a line.
    pub refused: Vec<String>,
    /// The answer an answer gives, one of those the item was offered.
    pub answer: Option<Choice>,
    /// The open issue a link names, one of those the item was offered.
    pub issue: Option<String>,
    /// The next piece of work in a line, for a new job or a new issue ruled on an end or a merge.
    pub what: Option<String>,
}

impl Ruling {
    /// A ruling of `way` alone, naming no other Job.
    pub fn alone(way: Way, because: impl Into<String>) -> Ruling {
        Ruling { way, other: None, shared: Vec::new(), because: because.into(), place: None, first: None, refused: Vec::new(), answer: None, issue: None, what: None }
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
    /// The day's calls reached the ceiling, so no call was made.
    Ceiling(u32),
    /// An answer that could no longer be given, such as to a question the person answered first.
    Settled(String),
    /// Every way it could go is Read-only, so no call was made. **Not in Done.**
    ReadOnly,
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
            PassedOn::Ceiling(calls) => write!(out, "today's ceiling of {calls} calls was reached, so no call was made"),
            PassedOn::Settled(why) => write!(out, "{why}"),
            PassedOn::ReadOnly => write!(out, "every way it could go is Read-only, so no call was made"),
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
    /// A Job held until another pushed starts as it would have: that other ended without pushing.
    HoldLifted,
    /// A Job ruled to wait for another's push was already working, or waiting on something else, so nothing was held.
    NotHeld,
    /// The person answered a question himself, which a second Job asking the same is offered.
    Answered,
    /// The person pressed Skip on what the Operator would have done.
    Skipped,
    /// The Job an item waited on ended, so the item is Done unruled.
    Ended,
    /// The person sent a note to a Job himself, from Bridge.
    Sent,
    /// Its role is Read-only, so no call was made or none was acted on. **Not in Done.**
    ReadOnly,
    /// The ruling was kept to wait on a press, or is the press a start's own card already is. **Not in
    /// Done**: the press is the row.
    Proposed(Way),
}

impl Outcome {
    pub fn word(self) -> &'static str {
        match self {
            Outcome::Ruled(way) => way.word(),
            Outcome::PassedOn => "passed_on",
            Outcome::Corrected => "corrected",
            Outcome::HoldLifted => "hold_lifted",
            Outcome::NotHeld => "not_held",
            Outcome::Answered => "answered",
            Outcome::Skipped => "skipped",
            Outcome::Ended => "ended",
            Outcome::Sent => "sent",
            Outcome::ReadOnly => "read_only",
            Outcome::Proposed(way) => way.proposed_word(),
        }
    }
}

/// Read one answer to a call that held `items`, one ruling an item in their order, each held to
/// the Jobs `near` it and the Jobs waiting to start in its `line`. An item the answer has no block
/// for, or a block that names what it was not offered, passes on.
pub(super) fn read_rulings(answer: &str, items: &[Arrival], near: &[Vec<Near>], line: &[Vec<InLine>]) -> Vec<Ruled> {
    let blocks = blocks(answer);
    items
        .iter()
        .enumerate()
        .map(|(at, item)| {
            let block = blocks.iter().find(|block| said(block, "item").and_then(|n| n.parse::<usize>().ok()) == Some(at + 1));
            match block {
                Some(block) => read_one(block, item, &near[at])
                    .map(|ruling| placed(ruling, block, &line[at]))
                    .map(Ruled::Ruled)
                    .unwrap_or_else(Ruled::PassedOn),
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
    if way == Way::Answer {
        return answered(block, item, because);
    }
    if way == Way::Link {
        let issue = said(block, "issue").filter(|issue| !none(issue)).ok_or_else(|| PassedOn::Refused("link named no issue".into()))?;
        if !item.open.iter().any(|one| one.reference == issue) {
            return Err(PassedOn::Refused(format!("{issue} is not an open issue it was offered")));
        }
        return Ok(Ruling { issue: Some(issue), ..Ruling::alone(way, because) });
    }
    if item.arrived.is_ending() && matches!(way, Way::NewJob | Way::File) {
        let what = said(block, "what").filter(|what| !none(what)).ok_or_else(|| PassedOn::Refused(format!("{} named no what", way.word_shown())))?;
        return Ok(Ruling { what: Some(what), ..Ruling::alone(way, because) });
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
    // A Job tied to an end, such as one waiting on it, needs no shared ground to be told.
    let tied = near.iter().any(|one| one.job.session_id == other && one.tie.is_some());
    if shared.is_empty() && !tied {
        return Err(PassedOn::Refused(format!("{} named nothing the two Jobs share", way.word_shown())));
    }
    let mut ruling = Ruling { other: Some(other.clone()), shared, ..Ruling::alone(way, because) };
    if way == Way::KeepInTouch {
        if let Some(first) = said(block, "first").filter(|id| !none(id)) {
            match first == item.session_id || first == other {
                true => ruling.first = Some(first),
                false => ruling.refused.push(format!("{first} builds first, which is neither of the two, so nothing is held")),
            }
        }
    }
    Ok(ruling)
}

/// An answer names one of the answers the item was offered, by its number. **One offered alone is
/// the answer** where the call named none.
fn answered(block: &[(String, String)], item: &Arrival, because: String) -> Result<Ruling, PassedOn> {
    let named = said(block, "answer").filter(|answer| !none(answer));
    let choice = match named {
        None if item.answers.len() == 1 => item.answers[0].clone(),
        None => return Err(PassedOn::Refused("answer named no answer".into())),
        Some(named) => named
            .parse::<usize>()
            .ok()
            .and_then(|number| number.checked_sub(1))
            .and_then(|at| item.answers.get(at).cloned())
            .ok_or_else(|| PassedOn::Refused(format!("answer {named} was not one it was offered")))?,
    };
    let other = choice.earlier.as_ref().map(|earlier| earlier.session_id.clone());
    Ok(Ruling { other, answer: Some(choice), ..Ruling::alone(Way::Answer, because) })
}

/// The ruling with its place in Waiting to start, where Jobs wait there: last where the call named
/// none, and last, refused, where it named a Job Fleet did not offer.
fn placed(mut ruling: Ruling, block: &[(String, String)], line: &[InLine]) -> Ruling {
    if line.is_empty() {
        return ruling;
    }
    let Some(said) = said(block, "place").filter(|place| !none(place)) else {
        return ruling;
    };
    ruling.place = Some(match Place::read(&said) {
        Some(place) if place.against().is_none_or(|id| line.iter().any(|one| one.session_id == id)) => place,
        Some(place) => {
            ruling.refused.push(format!("{place}, a Job not waiting to start there, so it goes last"));
            Place::Last
        }
        None => {
            ruling.refused.push(format!("the place \"{said}\" could not be read, so it goes last"));
            Place::Last
        }
    });
    ruling
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
