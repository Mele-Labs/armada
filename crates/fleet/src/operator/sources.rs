//! The arrivals from outside a Job's own run, which nobody owns: a bug report, a new issue, a red
//! main, a note the person left on Bridge and an error Bridge showed him. **Words from off this
//! machine are data**: an issue's body reaches the session as quoted words and never as an instruction.

use super::arriving::{Arrival, Arrived, Source};

/// A bug report an agent made with `report_bug`, kept on this Mac.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Report {
    pub id: String,
    /// The Job that reported it.
    pub session_id: String,
    pub manifest_id: String,
    pub title: String,
    pub what: String,
}

impl Report {
    pub fn arrival(&self) -> Arrival {
        Arrival {
            id: format!("report:{}", self.id),
            arrived: Arrived::Report,
            source: Source::Agent,
            session_id: self.session_id.clone(),
            manifest_id: self.manifest_id.clone(),
            project: String::new(),
            subject: self.title.clone(),
            words: self.what.clone(),
        }
    }
}

/// What a Job asked with `raise_conflict` or `ask_held`, as the Operator is handed it. **Its words are what the
/// Job wrote**, so the call may tell the Job or raise it and does nothing else with them.
pub fn ask_arrival(ask: &store::KeptJobAsk) -> Arrival {
    let job = |title: &str, session: &str| format!("Job {session} \"{title}\" ({})", crate::session_host::address_of(session));
    let (arrived, subject, words) = match (ask.kind.as_str(), ask.pull, ask.branch.as_deref()) {
        ("conflict", _, _) => {
            let other = ask.other_session_id.clone().unwrap_or_default();
            let mut words = format!(
                "{} does: {}\n{} does: {}\nIf both land: {}",
                job(&ask.job_title, &ask.session_id),
                ask.mine,
                job(ask.other_title.as_deref().unwrap_or_default(), &other),
                ask.theirs,
                ask.breaks
            );
            if !ask.wire.is_empty() {
                words.push_str(&format!("\nWire types: {}", ask.wire.join(", ")));
            }
            (Arrived::Conflict, ask.breaks.clone(), words)
        }
        (_, Some(pull), _) => {
            let checks = ask.checks.as_deref().map(|checks| format!(", its checks {checks}")).unwrap_or_default();
            let title = ask.title.as_deref().map(|title| format!(" \"{title}\"")).unwrap_or_default();
            (Arrived::Held, format!("Should #{pull} land?"), format!("{} asks whether #{pull}{title} should land{checks}.", job(&ask.job_title, &ask.session_id)))
        }
        (_, None, branch) => {
            let branch = branch.unwrap_or_default();
            (Arrived::Held, format!("Should {branch} land?"), format!("{} asks whether branch {branch} should land.", job(&ask.job_title, &ask.session_id)))
        }
    };
    Arrival {
        id: match ask.round {
            0 => ask.id.clone(),
            round => format!("{}#{round}", ask.id),
        },
        arrived,
        source: Source::Agent,
        session_id: ask.session_id.clone(),
        manifest_id: ask.manifest_id.clone(),
        project: String::new(),
        subject,
        words,
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
    pub fn arrival(&self) -> Arrival {
        Arrival {
            id: format!("issue:{}:{}", self.manifest_id, self.reference),
            arrived: Arrived::Issue,
            source: Source::Tracker,
            session_id: String::new(),
            manifest_id: self.manifest_id.clone(),
            project: String::new(),
            subject: self.title.clone(),
            words: format!("{} by {}: {}", self.reference, self.author, self.excerpt),
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
    /// The features the library points to from where the note was left, one project to a line. Empty where no project holds a
    /// library.
    pub features: String,
}

impl Note {
    pub fn arrival(&self) -> Arrival {
        Arrival {
            id: format!("note:{}:{}", self.manifest_id, self.id),
            arrived: Arrived::Note,
            source: Source::Person,
            session_id: String::new(),
            manifest_id: self.manifest_id.clone(),
            project: String::new(),
            subject: subject_of(&self.text),
            words: format!("{}\n\n{}{}", self.text.trim(), self.left(), self.features),
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
}

impl BridgeError {
    /// **It names no Job as its own**: the Job it was raised in is in the words.
    pub fn arrival(&self) -> Arrival {
        Arrival {
            id: self.id.clone(),
            arrived: Arrived::Error,
            source: Source::Bridge,
            session_id: String::new(),
            manifest_id: self.manifest_id.clone(),
            project: String::new(),
            subject: subject_of(&self.message),
            words: self.words(),
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
