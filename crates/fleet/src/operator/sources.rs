//! The arrivals from outside a Job's own run: a bug report, a new issue, a red main, a review
//! request, a Later item handed over, a note the person left on Bridge and an error Bridge showed
//! him. **Words from off this machine are data**: an issue's body and someone else's review request
//! are offered only a raise, so nothing they say can start a Job or answer for the person, and the
//! call can name only a way it was offered. A note is the person's own words, so it is offered every
//! way, and Fleet keeps what is ruled on it as a suggestion until he presses Do it. An error is
//! Fleet's own build's words, offered every way as a report is, and goes by the role's position.

use super::arriving::{Arrival, Arrived, Source};
use super::candidates::open_near;
use super::ruling::Way;

/// An issue open in a repository's tracker.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Filed {
    pub reference: String,
    pub title: String,
}

/// A bug report an agent made with `report_bug`, kept on this Mac.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Report {
    pub id: String,
    /// The Job that reported it.
    pub session_id: String,
    pub manifest_id: String,
    pub title: String,
    pub what: String,
    /// The repository's open issues, which the report is read against.
    pub open: Vec<Filed>,
}

impl Report {
    /// Linked to an open issue it may be, handed to a live Job near it, started as a Job of its
    /// own or raised. Nothing is written to the issue.
    pub fn arrival(&self) -> Arrival {
        let open = open_near(&format!("{} {}", self.title, self.what), &self.open);
        let mut ways = Vec::new();
        if !open.is_empty() {
            ways.push(Way::Link);
        }
        ways.extend([Way::ExistingJob, Way::NewJob, Way::Raise]);
        Arrival {
            id: format!("report:{}", self.id),
            arrived: Arrived::Report,
            source: Source::Agent,
            session_id: self.session_id.clone(),
            manifest_id: self.manifest_id.clone(),
            subject: self.title.clone(),
            words: self.what.clone(),
            paths: Vec::new(),
            issues: Vec::new(),
            group: None,
            ways,
            answers: Vec::new(),
            rule: None,
            open,
            ties: Vec::new(),
        }
    }
}

/// An issue new to a repository's tracker since Fleet last read it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NewIssue {
    pub manifest_id: String,
    pub reference: String,
    pub title: String,
    /// The start of its body, as the tracker gave it.
    pub excerpt: String,
    pub author: String,
}

impl NewIssue {
    /// Read and raised: **while he is here a new issue starts nothing**, and Autopilot still works
    /// the backlog when it is on.
    pub fn arrival(&self) -> Arrival {
        Arrival {
            id: format!("issue:{}:{}", self.manifest_id, self.reference),
            arrived: Arrived::Issue,
            source: Source::Tracker,
            session_id: String::new(),
            manifest_id: self.manifest_id.clone(),
            subject: self.title.clone(),
            words: format!("{} by {}: {}", self.reference, self.author, self.excerpt),
            paths: Vec::new(),
            issues: vec![self.reference.clone()],
            group: None,
            ways: vec![Way::Raise],
            answers: Vec::new(),
            rule: None,
            open: Vec::new(),
            ties: Vec::new(),
        }
    }
}

/// A repository's main gone red, as Autopilot's red kind finds it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RedMain {
    pub manifest_id: String,
    /// The first red commit.
    pub commit: String,
    /// What Fleet knows of the red: the failing checks and the merges that came before it.
    pub seen: String,
    /// The Job already fixing it, where one holds it.
    pub fixing: Option<String>,
}

impl RedMain {
    /// Fixed by a new Job, by a live Job near it, or raised. **Nothing arrives where a Job is
    /// already fixing it**, so a second one is never started.
    pub fn arrival(&self) -> Option<Arrival> {
        if self.fixing.is_some() {
            return None;
        }
        Some(Arrival {
            id: format!("red_main:{}:{}", self.manifest_id, self.commit),
            arrived: Arrived::RedMain,
            source: Source::Fleet,
            session_id: String::new(),
            manifest_id: self.manifest_id.clone(),
            subject: format!("Main is red at {}", self.commit.get(..7).unwrap_or(&self.commit)),
            words: self.seen.clone(),
            paths: Vec::new(),
            issues: Vec::new(),
            group: None,
            ways: vec![Way::ExistingJob, Way::NewJob, Way::Raise],
            answers: Vec::new(),
            rule: None,
            open: Vec::new(),
            ties: Vec::new(),
        })
    }
}

/// A pull request someone asked the person to review.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReviewRequest {
    pub manifest_id: String,
    pub number: u64,
    pub title: String,
    pub author: String,
    pub url: String,
}

impl ReviewRequest {
    /// Raised with a read. **An agent's review stays on his press.**
    pub fn arrival(&self) -> Arrival {
        Arrival {
            id: format!("review:{}:{}", self.manifest_id, self.number),
            arrived: Arrived::Review,
            source: Source::Forge,
            session_id: String::new(),
            manifest_id: self.manifest_id.clone(),
            subject: self.title.clone(),
            words: format!("Pull request #{} by {} asks for the person's review: {}", self.number, self.author, self.url),
            paths: Vec::new(),
            issues: Vec::new(),
            group: None,
            ways: vec![Way::Raise],
            answers: Vec::new(),
            rule: None,
            open: Vec::new(),
            ties: Vec::new(),
        }
    }
}

/// A Later item a Job handed over to be settled.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Handed {
    pub session_id: String,
    pub manifest_id: String,
    /// The Job's title.
    pub title: String,
    /// `later-<n>`.
    pub later_id: String,
    pub sentence: String,
    /// The repository's open issues, checked first.
    pub open: Vec<Filed>,
    /// Whether the repository's tracker is beads, the one Fleet files in.
    pub files: bool,
}

impl Handed {
    /// Linked to an open issue it may be, filed where the repository's beads take it, or raised.
    /// **Filed with no call where no open issue shares its words**: that is what handing it over asked.
    pub fn arrival(&self) -> Arrival {
        let open = open_near(&self.sentence, &self.open);
        let mut ways = Vec::new();
        if !open.is_empty() {
            ways.push(Way::Link);
        }
        if self.files {
            ways.push(Way::File);
        }
        if ways != [Way::File] {
            ways.push(Way::Raise);
        }
        let rule = (ways == [Way::File]).then(|| "no open issue shares its words, so it is filed".to_string());
        Arrival {
            id: format!("later:{}:{}", self.session_id, self.later_id),
            arrived: Arrived::Later,
            source: Source::Agent,
            session_id: self.session_id.clone(),
            manifest_id: self.manifest_id.clone(),
            subject: self.sentence.clone(),
            words: format!("Job \"{}\" kept it for later, and handed it over to be settled", self.title),
            paths: Vec::new(),
            issues: Vec::new(),
            group: None,
            ways,
            answers: Vec::new(),
            rule,
            open,
            ties: Vec::new(),
        }
    }
}

/// The most of a first line a subject keeps.
const SUBJECT_CHARS: usize = 120;

/// The first line of `text` that says something, cut at [`SUBJECT_CHARS`].
fn subject_of(text: &str) -> String {
    let first = text.lines().map(str::trim).find(|line| !line.is_empty()).unwrap_or_default();
    match first.char_indices().nth(SUBJECT_CHARS) {
        Some((at, _)) => format!("{}…", first[..at].trim_end()),
        None => first.to_string(),
    }
}

/// A note the person left on Bridge, as its file under the repository's annotations holds it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Note {
    /// The note's id, which is also its file name less `.json`.
    pub id: String,
    pub manifest_id: String,
    pub text: String,
    pub screen: Option<String>,
    pub component: Option<String>,
    /// The repository's open issues, which the note is read against.
    pub open: Vec<Filed>,
    /// Whether the repository's tracker is beads, the one Fleet files in.
    pub files: bool,
}

impl Note {
    /// Linked to an open issue it may be, filed where the repository's beads take it, handed to a
    /// live Job near it, started as a Job of its own or raised.
    pub fn arrival(&self) -> Arrival {
        let open = open_near(&self.text, &self.open);
        let mut ways = Vec::new();
        if !open.is_empty() {
            ways.push(Way::Link);
        }
        if self.files {
            ways.push(Way::File);
        }
        ways.extend([Way::ExistingJob, Way::NewJob, Way::Raise]);
        let subject = subject_of(&self.text);
        Arrival {
            id: format!("note:{}:{}", self.manifest_id, self.id),
            arrived: Arrived::Note,
            source: Source::Person,
            session_id: String::new(),
            manifest_id: self.manifest_id.clone(),
            subject,
            words: format!("{}\n\n{}", self.text.trim(), self.left()),
            paths: Vec::new(),
            issues: Vec::new(),
            group: None,
            ways,
            answers: Vec::new(),
            rule: None,
            open,
            ties: Vec::new(),
        }
    }

    /// Where he left it.
    fn left(&self) -> String {
        match (self.screen.as_deref(), self.component.as_deref()) {
            (Some(screen), Some(component)) => format!("Left on the {screen} screen, on {component}"),
            (Some(screen), None) => format!("Left on the {screen} screen"),
            (None, Some(component)) => format!("Left on {component}"),
            (None, None) => "Left on Bridge".to_string(),
        }
    }
}

/// An error Bridge showed the person, as the card carried it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BridgeError {
    /// `error:<fingerprint>:<local day>`, which is also its arrival's id.
    pub id: String,
    pub manifest_id: String,
    pub code: String,
    pub message: String,
    pub fields: Vec<(String, String)>,
    pub chain: Vec<String>,
    pub logs: Vec<String>,
    pub commit: Option<String>,
    /// The Job the card was raised in, as `(id, title)`.
    pub job: Option<(String, String)>,
    /// When the card was taken.
    pub at: String,
    /// The repository's open issues, which the error is read against.
    pub open: Vec<Filed>,
    /// Whether the repository's tracker is beads, the one Fleet files in.
    pub files: bool,
}

impl BridgeError {
    /// Linked to an open issue it may be, filed where the repository's beads take it, handed to a
    /// live Job near it, started as a Job of its own or raised. **It names no Job as its own**, so
    /// the Job it was raised in is weighed with the rest: its title is in the words.
    pub fn arrival(&self) -> Arrival {
        let words = self.words();
        let subject = subject_of(&self.message);
        let open = open_near(&format!("{subject} {words}"), &self.open);
        let mut ways = Vec::new();
        if !open.is_empty() {
            ways.push(Way::Link);
        }
        if self.files {
            ways.push(Way::File);
        }
        ways.extend([Way::ExistingJob, Way::NewJob, Way::Raise]);
        Arrival {
            id: self.id.clone(),
            arrived: Arrived::Error,
            source: Source::Bridge,
            session_id: String::new(),
            manifest_id: self.manifest_id.clone(),
            subject,
            words,
            paths: Vec::new(),
            issues: Vec::new(),
            group: None,
            ways,
            answers: Vec::new(),
            rule: None,
            open,
            ties: Vec::new(),
        }
    }

    /// Everything the card carried, one fact to a line.
    fn words(&self) -> String {
        let mut out = format!("Code: {}\nMessage: {}", self.code, self.message.trim());
        if !self.fields.is_empty() {
            out.push_str("\nFields:");
            for (key, value) in &self.fields {
                out.push_str(&format!("\n  {key}: {value}"));
            }
        }
        if !self.chain.is_empty() {
            out.push_str("\nCause chain, outermost first:");
            for (at, cause) in self.chain.iter().enumerate() {
                out.push_str(&format!("\n  {}. {cause}", at + 1));
            }
        }
        if !self.logs.is_empty() {
            out.push_str(&format!("\nLogs: {}", self.logs.join(", ")));
        }
        if let Some(commit) = &self.commit {
            out.push_str(&format!("\nCommit: {commit}"));
        }
        if let Some((id, title)) = &self.job {
            out.push_str(&format!("\nRaised in Job {id} \"{title}\""));
        }
        out.push_str(&format!("\nTime: {}", self.at));
        out
    }
}
