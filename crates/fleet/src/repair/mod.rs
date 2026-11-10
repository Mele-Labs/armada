//! A repair: an agent hosted beside a broken Job, with no session row, no place in the Jobs list and
//! no start card. It runs one turn in the Job's own checkout, suited to what broke, and ends by itself.
//! `docs/concepts/session/what-a-job-is-doing.md`.
//!
//! What it is told is [`brief`]; the process and the stop that reaches it are [`hosting`]; starting
//! one and what its end does to the Job is [`running`]. **What Fleet does with its end is read from
//! its first line**, never from the agent saying it is done.

mod brief;
mod hosting;
mod running;

pub(crate) use brief::shipped_texts;
pub(crate) use running::REPAIR_UNHEALTHY;
pub use hosting::{RepairHost, Repairs};
pub(crate) use hosting::{Finish, RepairEnded, REPAIR_LIMIT};

/// What broke, as Fleet saw it. Each kind has its own brief.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RepairBreak {
    /// It stopped twice with nothing asked, its agent alive.
    Quiet,
    /// A turn running with nothing heard for `STUCK_AFTER`.
    Hung,
    /// Its process died and Fleet's one restart is spent.
    Died,
    /// Conflict markers, a half finished merge, or missing dependencies or a binary.
    Checkout,
    /// A pull request's checks failed or it conflicts, and its Job ended or stopped.
    RedPull { number: u64 },
    /// A slot whose holder is gone, left with changes.
    Stranded { slot: u32 },
}

impl RepairBreak {
    /// One of each kind, for what must hold for every brief.
    pub(crate) const ALL: [RepairBreak; 6] = [
        RepairBreak::Quiet,
        RepairBreak::Hung,
        RepairBreak::Died,
        RepairBreak::Checkout,
        RepairBreak::RedPull { number: 0 },
        RepairBreak::Stranded { slot: 0 },
    ];

    /// How the store keeps the kind.
    pub(crate) fn key(self) -> &'static str {
        match self {
            RepairBreak::Quiet => "quiet",
            RepairBreak::Hung => "hung",
            RepairBreak::Died => "died",
            RepairBreak::Checkout => "checkout",
            RepairBreak::RedPull { .. } => "red_pull",
            RepairBreak::Stranded { .. } => "stranded",
        }
    }

    /// What broke, in the words the note and the Now panel use.
    pub(crate) fn words(self) -> String {
        match self {
            RepairBreak::Quiet => "it stopped twice with nothing asked".to_string(),
            RepairBreak::Hung => "a command hung".to_string(),
            RepairBreak::Died => "its process died twice".to_string(),
            RepairBreak::Checkout => "its checkout is broken".to_string(),
            RepairBreak::RedPull { number } => format!("checks failed on #{number}"),
            RepairBreak::Stranded { slot } => format!("slot-{slot} was left with changes"),
        }
    }

    /// What the Job is told when the repair did it, or `None` where the Job is not the one to be woken:
    /// a pull request's branch and a stranded slot have no agent to carry on.
    pub(crate) fn woke_with(self) -> Option<&'static str> {
        let did = match self {
            RepairBreak::Quiet => "worked out why it stopped",
            RepairBreak::Hung => "stopped the command that hung",
            RepairBreak::Died => "fixed what stopped its process",
            RepairBreak::Checkout => "put its checkout right",
            RepairBreak::RedPull { .. } | RepairBreak::Stranded { .. } => return None,
        };
        Some(did)
    }
}

/// What a repair's final message said, by its first line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Report {
    Repaired(String),
    NotRepaired(String),
    /// Neither marker leads the message, or it said nothing.
    Unmarked,
}

/// The first line of `last`, read for the two markers the brief ends with.
pub(crate) fn report_of(last: Option<&str>) -> Report {
    let first = last.and_then(|text| text.lines().next()).unwrap_or("").trim();
    // `NOT REPAIRED:` is read first: it contains the other marker's word.
    if let Some(rest) = first.strip_prefix("NOT REPAIRED:") {
        Report::NotRepaired(rest.trim().to_string())
    } else if let Some(rest) = first.strip_prefix("REPAIRED:") {
        Report::Repaired(rest.trim().to_string())
    } else {
        Report::Unmarked
    }
}

/// What came of a repair, as Fleet keeps and says it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Verdict {
    /// How the store keeps it: `repaired`, `not_repaired`, `unmarked`, `timed_out`, `stopped` or `failed`.
    pub(crate) outcome: &'static str,
    /// The sentence the note carries.
    pub(crate) said: String,
    pub(crate) repaired: bool,
}

/// What a repair's end comes to. **Only a turn that ended itself and began with `REPAIRED:` counts.**
pub(crate) fn verdict_of(ended: &RepairEnded) -> Verdict {
    let not = |outcome, said: String| Verdict { outcome, said, repaired: false };
    match &ended.finish {
        Finish::TimedOut => not("timed_out", format!("it ran past {} minutes", REPAIR_LIMIT.as_secs() / 60)),
        Finish::Stopped => not("stopped", "the job moved on first".to_string()),
        Finish::Failed(why) => not("failed", why.clone()),
        Finish::Ended => match report_of(ended.said.as_deref()) {
            Report::Repaired(said) => Verdict { outcome: "repaired", said, repaired: true },
            Report::NotRepaired(said) => not("not_repaired", said),
            Report::Unmarked => not("unmarked", "it ended without saying whether it repaired anything".to_string()),
        },
    }
}

/// What Fleet's note in the Job's thread begins with when a repair starts, and when it ends. **The
/// Now panel reads a repair under way from these**, a start with no end after it, so they are one
/// definition for the writer and the reader.
pub(crate) const STARTED: &str = "Repair: ";
pub(crate) const REPAIRED: &str = "Repaired: ";
pub(crate) const NOT_REPAIRED: &str = "Not repaired: ";
