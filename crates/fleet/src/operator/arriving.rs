//! What arrives at the Operator's session, and the live Jobs Fleet lists for it.

/// What kind of thing arrived, as the Log names it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Arrived {
    /// A Job started: from a dispatch, another Job, a fork, Fleet or a terminal.
    Start,
    /// An agent asked the person something.
    Question,
    /// An agent asked to use a tool, on a permission card.
    Card,
    /// Fleet marked a Job Stopped.
    Stopped,
    /// A Job ended a turn after Fleet told it its pull request failed, and pushed nothing since.
    Red,
    /// A Job recorded a plan.
    Plan,
    /// An agent reported a bug with `report_bug`.
    Report,
    /// An issue new to the repository's tracker.
    Issue,
    /// The repository's main went red.
    RedMain,
    /// Someone asked the person to review a pull request.
    Review,
    /// A Job handed a Later item over to be settled.
    Later,
    /// A note the person left on Bridge, for a small thing or an idea.
    Note,
    /// A hosted Job ended, whoever started it.
    Ended,
    /// A pull request a hosted Job held merged.
    Merged,
    /// An error Bridge showed the person, which goes first in the order.
    Error,
    /// A Job raised a conflict with another Job's work.
    Conflict,
    /// A Job asked whether a pull request or a branch should land.
    Held,
    /// A Job named a feature the project's library does not hold.
    Feature,
    /// An area of a project's feature library passed about eight features.
    Area,
}

impl Arrived {
    pub fn word(self) -> &'static str {
        match self {
            Arrived::Start => "start",
            Arrived::Question => "question",
            Arrived::Card => "card",
            Arrived::Stopped => "stopped",
            Arrived::Red => "red",
            Arrived::Plan => "plan",
            Arrived::Report => "report",
            Arrived::Issue => "issue",
            Arrived::RedMain => "red_main",
            Arrived::Review => "review",
            Arrived::Later => "later",
            Arrived::Note => "note",
            Arrived::Ended => "ended",
            Arrived::Merged => "merged",
            Arrived::Error => "error",
            Arrived::Conflict => "conflict",
            Arrived::Held => "held",
            Arrived::Feature => "feature",
            Arrived::Area => "area",
        }
    }

    /// How the session is told what arrived.
    pub(super) fn said(self) -> &'static str {
        match self {
            Arrived::Start => "A Job started",
            Arrived::Question => "An agent asked a question",
            Arrived::Card => "An agent asked to use a tool",
            Arrived::Stopped => "A Job stopped with work left",
            Arrived::Red => "A Job left its pull request red",
            Arrived::Plan => "A Job recorded a plan",
            Arrived::Report => "An agent reported a bug",
            Arrived::Issue => "A new issue was filed",
            Arrived::RedMain => "Main went red",
            Arrived::Review => "A review was requested of the person",
            Arrived::Later => "A Job handed over a Later item",
            Arrived::Note => "The person left a note",
            Arrived::Ended => "A Job ended",
            Arrived::Merged => "A pull request merged",
            Arrived::Error => "Bridge showed the person an error",
            Arrived::Conflict => "A Job raised a conflict with another Job",
            Arrived::Held => "A Job asked whether work should land",
            Arrived::Feature => "A Job named a feature the library does not hold",
            Arrived::Area => "An area of the feature library grew past eight features",
        }
    }
}

/// Where it came from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Source {
    /// The person's dispatch, through the proposer. `away` where he made it on his phone or another Mac.
    Dispatch { away: bool },
    /// Another Job's agent, named by its id.
    Job(String),
    /// A fork the person pressed.
    Fork,
    /// One of Fleet's own Jobs: a repair, a retro.
    Fleet,
    /// A Job started in a terminal, which Fleet hears of after it began.
    Terminal,
    /// The Job's own agent.
    Agent,
    /// The person's own hand, such as a drag in Waiting to start: a row that corrects a ruling.
    Person,
    /// The repository's issue tracker: words from off this machine.
    Tracker,
    /// The forge, for someone else's pull request: words from off this machine.
    Forge,
    /// Bridge, for an error it showed the person.
    Bridge,
}

impl Source {
    pub fn word(&self) -> &'static str {
        match self {
            Source::Dispatch { .. } => "dispatch",
            Source::Job(_) => "job",
            Source::Fork => "fork",
            Source::Fleet => "fleet",
            Source::Terminal => "terminal",
            Source::Agent => "agent",
            Source::Person => "person",
            Source::Tracker => "tracker",
            Source::Forge => "forge",
            Source::Bridge => "bridge",
        }
    }

    /// The Job it came from, where another Job started it.
    pub fn session(&self) -> Option<&str> {
        match self {
            Source::Job(id) => Some(id),
            _ => None,
        }
    }
}

/// One arrival, as Fleet hands it to the session.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Arrival {
    /// Names this arrival, as `start:<session>`.
    pub id: String,
    pub arrived: Arrived,
    pub source: Source,
    pub session_id: String,
    pub manifest_id: String,
    /// The project this arrival is for alone, where it belongs to one and to no repository; empty where the repository says which hear it.
    pub project: String,
    /// What arrived, in a line: the Job's title, the question.
    pub subject: String,
    /// Its words: what was dispatched, what was asked.
    pub words: String,
}

impl Arrival {
    /// Work the person dispatched, which no Job holds yet.
    pub fn is_dispatch(&self) -> bool {
        self.id.starts_with(super::dispatching::DISPATCH)
    }

    /// A conflict or a landing question a Job sent.
    pub fn is_job_ask(&self) -> bool {
        matches!(self.arrived, Arrived::Conflict | Arrived::Held)
    }
}

/// A Job that is live, as Fleet lists it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LiveJob {
    pub session_id: String,
    pub title: String,
    pub manifest_id: String,
}

/// A Job waiting to start: in its repository's line for a slot, in the line's order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InLine {
    pub session_id: String,
    pub title: String,
    pub manifest_id: String,
}
