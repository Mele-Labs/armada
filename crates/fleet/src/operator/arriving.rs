//! What arrives at the Operator, and the live Jobs it is weighed against.

use super::ruling::Way;

/// What kind of thing arrived, as the Log names it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Arrived {
    /// A Job started: from a dispatch, another Job, a fork, Fleet or a terminal.
    Start,
    /// An agent asked the person something.
    Question,
    /// Fleet marked a Job Stopped.
    Stopped,
    /// A Job ended a turn after Fleet told it its pull request failed, and pushed nothing since.
    Red,
    /// A Job recorded a plan.
    Plan,
}

impl Arrived {
    pub fn word(self) -> &'static str {
        match self {
            Arrived::Start => "start",
            Arrived::Question => "question",
            Arrived::Stopped => "stopped",
            Arrived::Red => "red",
            Arrived::Plan => "plan",
        }
    }

    /// A Job gone quiet with work left, which is nudged, started anew or raised.
    pub fn is_stall(self) -> bool {
        matches!(self, Arrived::Stopped | Arrived::Red)
    }

    /// How the call is told what arrived.
    fn said(self) -> &'static str {
        match self {
            Arrived::Start => "A Job started",
            Arrived::Question => "An agent asked a question",
            Arrived::Stopped => "A Job stopped with work left",
            Arrived::Red => "A Job left its pull request red",
            Arrived::Plan => "A Job recorded a plan",
        }
    }
}

/// Where it came from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Source {
    /// The person's dispatch, through the proposer.
    Dispatch,
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
}

impl Source {
    pub fn word(&self) -> &'static str {
        match self {
            Source::Dispatch => "dispatch",
            Source::Job(_) => "job",
            Source::Fork => "fork",
            Source::Fleet => "fleet",
            Source::Terminal => "terminal",
            Source::Agent => "agent",
        }
    }

    /// The Job it came from, where another Job started it.
    pub fn session(&self) -> Option<&str> {
        match self {
            Source::Job(id) => Some(id),
            _ => None,
        }
    }

    fn said(&self) -> String {
        match self {
            Source::Dispatch => "from the person's dispatch".to_string(),
            Source::Job(id) => format!("started by Job {id}"),
            Source::Fork => "a fork the person pressed".to_string(),
            Source::Fleet => "one of Fleet's own".to_string(),
            Source::Terminal => "from a terminal".to_string(),
            Source::Agent => "from its own agent".to_string(),
        }
    }
}

/// One arrival, as Fleet hands it over: the item, the Job it is about, and the ways Fleet's rules
/// leave open for it. **The call can name only one of `ways`**, so a rule Fleet holds is never the
/// call's to weigh.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Arrival {
    /// Names this arrival in the Log, as `start:<session>`.
    pub id: String,
    pub arrived: Arrived,
    pub source: Source,
    pub session_id: String,
    pub manifest_id: String,
    /// What arrived, in a line: the Job's title, the question.
    pub subject: String,
    /// Its words: what was dispatched, what was asked.
    pub words: String,
    /// The paths its plan names or it has written so far.
    pub paths: Vec<String>,
    /// The issues it names or holds.
    pub issues: Vec<String>,
    /// The plan group it is, for a plan recorded: ruled group by group.
    pub group: Option<String>,
    pub ways: Vec<Way>,
}

impl Arrival {
    /// The item as the call reads it, numbered `at`.
    pub(super) fn shown(&self, at: usize) -> String {
        let mut out = format!(
            "  {at}. {}, {}: \"{}\" in repository {}, Job {}\n",
            self.arrived.said(),
            self.source.said(),
            self.subject,
            self.manifest_id,
            self.session_id
        );
        if !self.words.trim().is_empty() {
            out.push_str(&format!("     Its words: {}\n", one_line(&self.words)));
        }
        if !self.paths.is_empty() {
            out.push_str(&format!("     Its paths: {}\n", self.paths.join(", ")));
        }
        if !self.issues.is_empty() {
            out.push_str(&format!("     Its issues: {}\n", self.issues.join(", ")));
        }
        if let Some(group) = &self.group {
            out.push_str(&format!("     It is group {group} of that Job's plan\n"));
        }
        out.push_str("     The ways it is offered:");
        for (number, way) in self.ways.iter().enumerate() {
            out.push_str(&format!(" {}. {}", number + 1, way.word_shown()));
            if number + 1 < self.ways.len() {
                out.push(';');
            }
        }
        out.push('\n');
        out
    }
}

/// A Job that is live as the call is made, as the call is shown it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LiveJob {
    pub session_id: String,
    pub title: String,
    pub manifest_id: String,
    /// Its dispatch words, for a Job with no plan yet: when overlap is cheapest to catch.
    pub words: String,
    /// Its plan's paths and the files it has written.
    pub paths: Vec<String>,
    /// The issues it holds or its plan's groups work.
    pub issues: Vec<String>,
}

impl LiveJob {
    pub(super) fn shown(&self) -> String {
        let mut out = format!("  Job {}: \"{}\" in repository {}\n", self.session_id, self.title, self.manifest_id);
        if !self.words.trim().is_empty() {
            out.push_str(&format!("    Its words: {}\n", one_line(&self.words)));
        }
        if !self.paths.is_empty() {
            out.push_str(&format!("    Its paths: {}\n", self.paths.join(", ")));
        }
        if !self.issues.is_empty() {
            out.push_str(&format!("    Its issues: {}\n", self.issues.join(", ")));
        }
        out
    }
}

fn one_line(words: &str) -> String {
    words.split_whitespace().collect::<Vec<_>>().join(" ")
}
