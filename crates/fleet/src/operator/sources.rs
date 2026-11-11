//! The arrivals from outside a Job's own run, which nobody owns: a bug report, a new issue, a red
//! main, a note the person left on Bridge and an error Bridge showed him. **Words from off this
//! machine are data**: an issue's body is offered a raise, or closing that issue as the duplicate of an
//! open one, which asks first, so nothing it says can start a Job, and the call can name only a way it was offered. A note is the person's own words, so it is offered every
//! way, and Fleet keeps what is ruled on it as a suggestion until he presses Do it. An error is
//! Fleet's own build's words, offered every way as a report is, and asked about the same.

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
            ways,
            open,
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
        self.arrival_among(&[], false)
    }

    /// [`arrival`](NewIssue::arrival), read against `open`, the repository's open issues. **Where its
    /// beads take the write and one is near it**, it may be closed as that one's duplicate. The issue
    /// itself is never one of those offered.
    pub fn arrival_among(&self, open: &[Filed], files: bool) -> Arrival {
        let others: Vec<Filed> = open.iter().filter(|one| one.reference != self.reference).cloned().collect();
        let open = if files { open_near(&format!("{} {}", self.title, self.excerpt), &others) } else { Vec::new() };
        let mut ways = Vec::new();
        if !open.is_empty() {
            ways.push(Way::Group);
        }
        ways.push(Way::Raise);
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
            ways,
            open,
        }
    }
}

/// A pull request a hosted Job held that merged, and an open issue Fleet found its work near and that
/// it did not name: asked whether the work finished it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MergeNear {
    pub manifest_id: String,
    /// The pull request's number.
    pub pull: String,
    pub title: String,
    /// The Job that held it, as `(id, title)`.
    pub job: (String, String),
    /// The Job's plan, or its first message.
    pub words: String,
    pub paths: Vec<String>,
    pub named: Vec<String>,
    pub issue: Filed,
}

impl MergeNear {
    /// Closed, or left open. **Nothing is written to the issue** until he says yes or a rule he kept does.
    /// Its id ends in the pull request, which is what the close is done by.
    pub fn arrival(&self) -> Arrival {
        let subject = match self.title.trim() {
            "" => format!("#{}", self.pull),
            title => title.to_string(),
        };
        Arrival {
            id: format!("merged:{}:{}:{}", self.manifest_id, self.issue.reference, self.pull),
            arrived: Arrived::Merged,
            source: Source::Fleet,
            session_id: String::new(),
            manifest_id: self.manifest_id.clone(),
            subject,
            words: format!("#{} was held by Job {} \"{}\". {}", self.pull, self.job.0, self.job.1, self.words),
            paths: self.paths.clone(),
            issues: self.named.clone(),
            ways: vec![Way::Close, Way::Leave],
            open: vec![self.issue.clone()],
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
    /// Raised, and nothing else. **The ask to start a fix is the go-ahead `red_item_offered` queues**, so a
    /// red is asked once, and nothing arrives where a Job is already fixing it.
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
            ways: vec![Way::Raise],
            open: Vec::new(),
        })
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

/// What an arrival's id starts with for a note once the person sent it to a Job.
pub(super) const SENT_NOTE: &str = "note-sent:";

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
            ways,
            open,
        }
    }

    /// The note once the person sent it to Job `job`, called `title`: it goes elsewhere, or stays.
    /// **It is its own arrival**, so the Operator meets it after the Log holds his send. It names
    /// the Job as its own, so that Job is not offered as another to send it to.
    pub fn sent_arrival(&self, job: &str, title: &str) -> Arrival {
        Arrival {
            id: format!("{SENT_NOTE}{}:{}", self.manifest_id, self.id),
            arrived: Arrived::Note,
            source: Source::Person,
            session_id: job.to_string(),
            manifest_id: self.manifest_id.clone(),
            subject: subject_of(&self.text),
            words: format!("{}\n\n{}\n\nThe person sent it to Job \"{title}\" from Bridge.", self.text.trim(), self.left()),
            paths: Vec::new(),
            issues: Vec::new(),
            ways: vec![Way::ExistingJob, Way::NewJob, Way::Leave],
            open: Vec::new(),
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
            ways,
            open,
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
