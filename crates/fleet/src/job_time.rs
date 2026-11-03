//! Where a Job's time went: every second between its start and its end in exactly one bucket.
//! `docs/concepts/session.md`.
//!
//! **Values in, values out**, as `usage_rules.rs` is. [`fold`] reads one Job's kept times, in
//! milliseconds, and nothing else: no clock and no file. Fleet's part, reading the store and
//! keeping the answer, is `job_timing.rs`.
//!
//! **Where two buckets claim a second, the first in [`Bucket`]'s order takes it.** Work wins over
//! waiting, and a wait the Job is not in (the pull request, a stopped agent) wins over nothing.
//! A claim that reaches back past a restart starts at the restart: the gap is not work.

use std::cmp::Ordering;

/// Where a stretch went, in the order that settles a second two of them claim.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Bucket {
    /// A Check queued or ran.
    Checks,
    /// A subagent ran.
    Subagents,
    /// A tool ran in the main thread.
    Tools,
    /// The main thread waited on the model's answer.
    Model,
    /// Waiting for a slot, or for Fleet to move the Job into it.
    Slot,
    /// Waiting on the owner: a question, a walk, a plan, the start card.
    You,
    /// CI and the merge queue, with nothing else running.
    PullRequest,
    /// The Job's agent was not running, or Fleet was down, and nothing was asked.
    Stopped,
    /// Nothing running and nothing asked.
    Idle,
}

/// Which reading of the fold a row was made by. Raised when the fold changes what it gives for a Job
/// it already split, so the next pass splits those again.
pub const FOLD: i64 = 1;

/// A stretch, in milliseconds since the epoch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Span {
    pub from: i64,
    pub to: i64,
}

/// One turn of the Job's main thread. A subagent's is not one.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Turn {
    /// When the message was complete.
    pub at: i64,
    /// The user line it answered: the prompt, or the last tool result before it.
    pub model_began: Option<i64>,
    /// The first call it made, and the last result of its calls: wall time.
    pub tools_began: Option<i64>,
    pub tools_ended: Option<i64>,
    pub called_tools: bool,
    /// One of its calls was a question to the owner, which returns when he answers: the span from the
    /// first call to the last result is his time.
    pub asked_owner: bool,
}

/// Everything the fold reads of one Job.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Times {
    pub began: i64,
    pub ended: i64,
    pub turns: Vec<Turn>,
    pub checks: Vec<Span>,
    pub subagents: Vec<Span>,
    pub slot: Vec<Span>,
    pub owner: Vec<Span>,
    pub pull_request: Vec<Span>,
    pub stopped: Vec<Span>,
    /// When the agent's process came back, or died: a stretch of work does not run across one.
    pub restarts: Vec<i64>,
}

/// Milliseconds in each bucket. They sum to the Job's whole.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Split {
    pub checks: i64,
    pub subagents: i64,
    pub tools: i64,
    pub model: i64,
    pub slot: i64,
    pub you: i64,
    pub pull_request: i64,
    pub stopped: i64,
    pub idle: i64,
}

impl Split {
    pub fn total(&self) -> i64 {
        self.checks
            + self.subagents
            + self.tools
            + self.model
            + self.slot
            + self.you
            + self.pull_request
            + self.stopped
            + self.idle
    }

    fn add(&mut self, bucket: Bucket, millis: i64) {
        let held = match bucket {
            Bucket::Checks => &mut self.checks,
            Bucket::Subagents => &mut self.subagents,
            Bucket::Tools => &mut self.tools,
            Bucket::Model => &mut self.model,
            Bucket::Slot => &mut self.slot,
            Bucket::You => &mut self.you,
            Bucket::PullRequest => &mut self.pull_request,
            Bucket::Stopped => &mut self.stopped,
            Bucket::Idle => &mut self.idle,
        };
        *held += millis;
    }
}

/// The split, and where it stands on a guess.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Folded {
    pub split: Split,
    /// A turn of the main thread called tools and has no end for them (the Job's last turn is
    /// left out, since a run cut off mid-tool has none), so those tools' time reads as the model's.
    pub tool_times_missing: bool,
    /// A turn has no stamp for when its model began, so its wait is guessed ([`GUESS`]).
    pub model_starts_guessed: bool,
}

/// How far back from a turn's end the model's wait is taken to run when nothing says where it
/// began, as `usage_rules` counts a gap at most 5 minutes.
pub const GUESS: i64 = 5 * 60 * 1000;

/// A claim's rank: a [`Bucket`], plus a guessed model wait, which loses to every other claim and
/// is counted as the model's. Idle claims nothing: it is what no claim reaches.
#[derive(Clone, Copy)]
enum Rank {
    Of(Bucket),
    Guessed,
}

const RANKS: usize = 9;

fn slot_of(rank: Rank) -> usize {
    match rank {
        Rank::Of(bucket) => bucket as usize,
        Rank::Guessed => RANKS - 1,
    }
}

fn bucket_of(slot: usize) -> Bucket {
    use Bucket::*;
    [Checks, Subagents, Tools, Model, Slot, You, PullRequest, Stopped, Model][slot]
}

/// Split one Job's time. Every millisecond between `began` and `ended` is in one bucket, so
/// [`Split::total`] is the whole; a Job that ends before it began has none.
pub fn fold(times: &Times) -> Folded {
    let (began, ended) = (times.began, times.ended);
    let mut folded = Folded::default();
    if ended <= began {
        return folded;
    }
    let mut restarts = times.restarts.clone();
    restarts.sort_unstable();
    let mut claims: Vec<(Span, Rank)> = Vec::new();
    let mut claim = |span: Span, rank: Rank, work: bool| {
        // Work does not run across a restart: it starts where the process came back.
        let from = match work {
            true => {
                let upto = restarts.partition_point(|at| *at < span.to);
                upto.checked_sub(1).map(|last| restarts[last]).filter(|at| *at > span.from).unwrap_or(span.from)
            }
            false => span.from,
        };
        let (from, to) = (from.max(began), span.to.min(ended));
        if from < to {
            claims.push((Span { from, to }, rank));
        }
    };
    for span in &times.checks {
        claim(*span, Rank::Of(Bucket::Checks), true);
    }
    for span in &times.subagents {
        claim(*span, Rank::Of(Bucket::Subagents), true);
    }
    for span in &times.slot {
        claim(*span, Rank::Of(Bucket::Slot), false);
    }
    for span in &times.owner {
        claim(*span, Rank::Of(Bucket::You), false);
    }
    for span in &times.pull_request {
        claim(*span, Rank::Of(Bucket::PullRequest), false);
    }
    for span in &times.stopped {
        claim(*span, Rank::Of(Bucket::Stopped), false);
    }
    let mut turns = times.turns.clone();
    turns.sort_by_key(|turn| turn.at);
    let last = turns.len().saturating_sub(1);
    // The end of the turn before, and the later of that and its tools' end.
    let (mut before, mut anchor) = (began, began);
    for (at, turn) in turns.iter().enumerate() {
        if let (Some(from), Some(to)) = (turn.tools_began, turn.tools_ended) {
            // A question to the owner returns when he answers, so its span is his wait, not a tool's.
            let bucket = if turn.asked_owner { Bucket::You } else { Bucket::Tools };
            claim(Span { from, to }, Rank::Of(bucket), true);
        }
        folded.tool_times_missing |= turn.called_tools && turn.tools_ended.is_none() && at != last;
        match turn.model_began {
            Some(from) => claim(Span { from: from.max(before), to: turn.at }, Rank::Of(Bucket::Model), true),
            None => {
                folded.model_starts_guessed = true;
                claim(Span { from: (turn.at - GUESS).max(anchor), to: turn.at }, Rank::Guessed, true);
            }
        }
        before = before.max(turn.at);
        anchor = before.max(turn.tools_ended.unwrap_or(0));
    }
    folded.split = swept(began, ended, &claims);
    folded
}

/// Walk the claims' edges in order, a count of the claims standing at each rank, and give each
/// stretch between two edges to the first rank with one standing, or to Idle.
fn swept(began: i64, ended: i64, claims: &[(Span, Rank)]) -> Split {
    let mut edges: Vec<(i64, usize, i32)> = Vec::with_capacity(claims.len() * 2);
    for (span, rank) in claims {
        edges.push((span.from, slot_of(*rank), 1));
        edges.push((span.to, slot_of(*rank), -1));
    }
    edges.sort_by(|a, b| match a.0.cmp(&b.0) {
        Ordering::Equal => a.2.cmp(&b.2),
        other => other,
    });
    let mut split = Split::default();
    let mut standing = [0i32; RANKS];
    let mut at = began;
    for (edge, rank, change) in edges {
        if edge > at {
            split.add(standing.iter().position(|held| *held > 0).map_or(Bucket::Idle, bucket_of), edge - at);
            at = edge;
        }
        standing[rank] += change;
    }
    split.add(Bucket::Idle, ended - at);
    split
}

/// The stretches that begin at each of `opens` and end at the first of `closes` after it, or at
/// `ended` where none does. An open inside a stretch already open makes another that ends with it,
/// which the fold's sweep counts once.
pub fn spans_between(opens: &[i64], closes: &[i64], ended: i64) -> Vec<Span> {
    let mut closes = closes.to_vec();
    closes.sort_unstable();
    opens
        .iter()
        .map(|open| {
            let after = closes.partition_point(|close| close <= open);
            Span { from: *open, to: closes.get(after).copied().unwrap_or(ended) }
        })
        .collect()
}

/// The stretches an attachment stood: a version that stands begins one unless it is already in
/// one, any other ends it, and one still standing runs to `ended`. `versions` are `(at, standing)`,
/// oldest first.
pub fn stints(versions: &[(i64, bool)], ended: i64) -> Vec<Span> {
    let mut spans = Vec::new();
    let mut from: Option<i64> = None;
    for (at, standing) in versions {
        match (from, standing) {
            (None, true) => from = Some(*at),
            (Some(began), false) => {
                spans.push(Span { from: began, to: *at });
                from = None;
            }
            _ => {}
        }
    }
    if let Some(began) = from {
        spans.push(Span { from: began, to: ended });
    }
    spans
}
