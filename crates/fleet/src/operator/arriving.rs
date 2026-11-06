//! What arrives at the Operator, and the live Jobs it is weighed against.

use super::questions::Choice;
use super::ruling::Way;
use super::sources::Filed;

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
        }
    }

    /// Every kind there is.
    pub const EVERY: [Arrived; 12] = [
        Arrived::Start,
        Arrived::Question,
        Arrived::Card,
        Arrived::Stopped,
        Arrived::Red,
        Arrived::Plan,
        Arrived::Report,
        Arrived::Issue,
        Arrived::RedMain,
        Arrived::Review,
        Arrived::Later,
        Arrived::Note,
    ];

    /// What a word from the store names.
    pub fn from_word(word: &str) -> Option<Arrived> {
        Arrived::EVERY.into_iter().find(|arrived| arrived.word() == word)
    }

    /// What a standing order calls this kind of thing.
    pub fn named(self) -> &'static str {
        match self {
            Arrived::Start => "Job starts",
            Arrived::Question => "questions",
            Arrived::Card => "permission cards",
            Arrived::Stopped => "stopped Jobs",
            Arrived::Red => "red pull requests",
            Arrived::Plan => "plans",
            Arrived::Report => "bug reports",
            Arrived::Issue => "new issues",
            Arrived::RedMain => "red mains",
            Arrived::Review => "review requests",
            Arrived::Later => "Later items",
            Arrived::Note => "notes",
        }
    }

    /// A Job gone quiet with work left, which is started anew or raised.
    pub fn is_stall(self) -> bool {
        matches!(self, Arrived::Stopped | Arrived::Red)
    }

    /// A question or a card, which an answer settles as the person's press would.
    pub fn is_asked(self) -> bool {
        matches!(self, Arrived::Question | Arrived::Card)
    }

    /// Work from outside a Job's own run, which [`super::Acting::sourced`] does what is ruled on.
    pub fn is_sourced(self) -> bool {
        matches!(self, Arrived::Report | Arrived::Issue | Arrived::RedMain | Arrived::Review | Arrived::Later | Arrived::Note)
    }

    /// Put to a call for its read even where Fleet's rules leave it one way: the person meets it
    /// with what the Operator made of it.
    pub fn is_read(self) -> bool {
        matches!(self, Arrived::Issue | Arrived::Review)
    }

    /// How the call is told what arrived.
    fn said(self) -> &'static str {
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
    /// The person's own hand, such as a drag in Waiting to start: a row that corrects a ruling.
    Person,
    /// The repository's issue tracker: words from off this machine.
    Tracker,
    /// The forge, for someone else's pull request: words from off this machine.
    Forge,
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
            Source::Person => "person",
            Source::Tracker => "tracker",
            Source::Forge => "forge",
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
            Source::Person => "by the person's own hand".to_string(),
            Source::Tracker => "from the tracker".to_string(),
            Source::Forge => "from the forge".to_string(),
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
    /// The answers an answer may give, for a question or a card.
    pub answers: Vec<Choice>,
    /// Fleet's rule that leaves it only to the person, in a line, where one does.
    pub rule: Option<String>,
    /// The open issues it may be, for a report or a Later item: a link may name only these.
    pub open: Vec<Filed>,
}

impl Arrival {
    /// The item as the call reads it, numbered `at`.
    pub fn shown(&self, at: usize) -> String {
        let mut out = format!("  {at}. {}, {}: \"{}\" in repository {}", self.arrived.said(), self.source.said(), self.subject, self.manifest_id);
        // An issue, a red main and a review request are no Job's.
        if !self.session_id.is_empty() {
            out.push_str(&format!(", Job {}", self.session_id));
        }
        out.push('\n');
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
        if !self.open.is_empty() {
            let open: Vec<String> = self.open.iter().map(|one| format!("{} \"{}\"", one.reference, one.title)).collect();
            out.push_str(&format!("     Open issues it may be: {}\n", open.join("; ")));
        }
        if !self.answers.is_empty() {
            out.push_str("     The answers it may give:");
            for (number, one) in self.answers.iter().enumerate() {
                out.push_str(&format!(" {}. \"{}\"", number + 1, one.label));
                if let Some(earlier) = &one.earlier {
                    out.push_str(&format!(", the person's answer when Job {}, \"{}\", asked the same", earlier.session_id, earlier.title));
                }
                if number + 1 < self.answers.len() {
                    out.push(';');
                }
            }
            out.push('\n');
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
    /// The subagents and background tasks it has working past its turn.
    pub running: Vec<ipc::RunningWork>,
    /// Who started it, as `(id, title)`, nearest first up to the root. Empty for a Job the person
    /// started. An ended starter is named too.
    pub started_by: Vec<(String, String)>,
}

/// A Job's starters, nearest first: `Job <id> "<title>", which Job <id> "<title>" started`, each
/// further one started the one before it.
pub fn started_by_shown(chain: &[(String, String)]) -> String {
    let mut named = chain.iter().map(|(id, title)| format!("Job {id} \"{title}\""));
    let mut out = named.next().unwrap_or_default();
    for one in named {
        out.push_str(&format!(", which {one} started"));
    }
    out
}

impl LiveJob {
    pub fn shown(&self) -> String {
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
        if !self.running.is_empty() {
            let each: Vec<String> = self
                .running
                .iter()
                .map(|one| {
                    let kind = match one.kind {
                        ipc::RunningKind::Subagent => "subagent",
                        ipc::RunningKind::BackgroundTask => "background task",
                    };
                    format!("{kind} \"{}\"", one_line(&one.what))
                })
                .collect();
            out.push_str(&format!("    Still running past its turn: {}\n", each.join(", ")));
        }
        if !self.started_by.is_empty() {
            out.push_str(&format!("    Started by: {}\n", started_by_shown(&self.started_by)));
        }
        out
    }
}

/// A Job waiting to start: in its repository's line for a slot, in the line's order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InLine {
    pub session_id: String,
    pub title: String,
    pub manifest_id: String,
}

/// The Jobs waiting to start that a start may be placed among, as the call reads them.
pub fn line_shown(line: &[InLine]) -> String {
    let mut out = String::from("     Waiting to start in its repository, first served first:");
    for (at, one) in line.iter().enumerate() {
        out.push_str(&format!(" {}. Job {} \"{}\"", at + 1, one.session_id, one.title));
        if at + 1 < line.len() {
            out.push(';');
        }
    }
    out.push('\n');
    out
}

fn one_line(words: &str) -> String {
    words.split_whitespace().collect::<Vec<_>>().join(" ")
}
