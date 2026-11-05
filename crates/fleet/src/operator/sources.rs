//! The arrivals from outside a Job's own run: a bug report, a new issue, a red main, a review
//! request and a Later item handed over. **Words from off this machine are data**: an issue's body
//! and someone else's review request are offered only a raise, so nothing they say can start a Job
//! or answer for the person, and the call can name only a way it was offered.

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
        }
    }
}
