//! What the Operator is handed when a hosted Job ends or a pull request one held merges, and what its
//! end settles. **Every hosted Job, whoever started it**: the words carry what the call needs to
//! propose the next step, which Fleet already holds. `hearing` detects them and reads the facts.

use core_model::Actor;

use super::arriving::{Arrival, Arrived, Source};
use super::ruling::{Outcome, PassedOn, Ruled, Way};
use super::sources::Filed;
use super::{held, Logged, Operator};

/// A Job an end or a merge is tied to, and how.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tied {
    pub session_id: String,
    pub title: String,
    pub how: String,
}

/// What an end or a merge may unblock, as Fleet holds it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Unblocks {
    /// Jobs waiting to start on it.
    pub waiting: Vec<Tied>,
    /// Jobs it started that are still live.
    pub started: Vec<Tied>,
    /// Its Later items not yet settled.
    pub later: Vec<String>,
    /// Open issues sharing its words.
    pub open: Vec<Filed>,
    /// Needs you items on the Mac that name it or wait on a restart, each as one line.
    pub needs: Vec<String>,
}

impl Unblocks {
    /// The sentences the call reads, those with nothing left out.
    fn said(&self) -> Vec<String> {
        let jobs = |jobs: &[Tied]| jobs.iter().map(|one| format!("Job {} \"{}\" {}", one.session_id, one.title, one.how)).collect::<Vec<_>>().join("; ");
        let mut out = Vec::new();
        if !self.waiting.is_empty() {
            out.push(format!("Waiting to start on it: {}.", jobs(&self.waiting)));
        }
        if !self.started.is_empty() {
            out.push(format!("Jobs it started, still live: {}.", jobs(&self.started)));
        }
        if !self.later.is_empty() {
            out.push(format!("Its Later items not yet settled: {}.", self.later.iter().map(|one| format!("\"{one}\"")).collect::<Vec<_>>().join("; ")));
        }
        if !self.open.is_empty() {
            out.push(format!("Open issues near it: {}.", self.open.iter().map(|one| format!("{} \"{}\"", one.reference, one.title)).collect::<Vec<_>>().join("; ")));
        }
        if !self.needs.is_empty() {
            out.push(format!("Needs you items that name it or wait on a restart: {}.", self.needs.join("; ")));
        }
        out
    }

    /// The Jobs the item is tied to, as `(id, how)`, for the live ones among them to be named.
    fn ties(&self, starter: Option<&Tied>) -> Vec<(String, String)> {
        let how = |one: &Tied| (one.session_id.clone(), one.how.clone());
        self.waiting.iter().chain(&self.started).chain(starter).map(how).collect()
    }
}

/// A pull request a Job held, as its ledger row says.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pulled {
    pub number: String,
    pub title: String,
    /// `merged`, `open`, `draft` or `closed`.
    pub state: String,
}

/// A hosted Job that ended.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ended {
    pub session_id: String,
    pub manifest_id: String,
    pub title: String,
    /// What its agent said it achieved with `all_done`, where it said.
    pub summary: Option<String>,
    pub reason: Option<String>,
    pub pulls: Vec<Pulled>,
    /// The Job that started it, where one did.
    pub starter: Option<Tied>,
    pub unblocks: Unblocks,
    /// Its plan's paths and the files it wrote, and the issues it held, so a live Job on the same
    /// ground is shown with it.
    pub paths: Vec<String>,
    pub issues: Vec<String>,
    /// Whether the repository's tracker is beads, the one Fleet files in.
    pub files: bool,
}

/// What may be done with an end or a merge, in the order the call is shown them.
fn ways(open: &[Filed], files: bool) -> Vec<Way> {
    let mut ways = Vec::new();
    if !open.is_empty() {
        ways.push(Way::Link);
    }
    if files {
        ways.push(Way::File);
    }
    ways.extend([Way::ExistingJob, Way::NewJob, Way::Raise, Way::Noted]);
    ways
}

impl Ended {
    /// Told to a live Job it ties to, started as the next piece, filed, linked, raised or noted.
    pub fn arrival(&self) -> Arrival {
        let mut words = format!("Job \"{}\" ended.", self.title);
        if let Some(summary) = self.summary.as_deref().filter(|one| !one.trim().is_empty()) {
            words.push_str(&format!(" What it said it achieved: {}", summary.trim()));
            if !summary.trim_end().ends_with(['.', '!', '?']) {
                words.push('.');
            }
        }
        if let Some(reason) = self.reason.as_deref().filter(|one| !one.trim().is_empty()) {
            words.push_str(&format!(" Why it ended: {}.", reason.trim()));
        }
        if !self.pulls.is_empty() {
            let each: Vec<String> = self.pulls.iter().map(|one| format!("#{} \"{}\" ({})", one.number, one.title, one.state)).collect();
            words.push_str(&format!(" Its pull requests: {}.", each.join("; ")));
        }
        if let Some(starter) = &self.starter {
            words.push_str(&format!(" Started by Job {} \"{}\".", starter.session_id, starter.title));
        }
        for said in self.unblocks.said() {
            words.push(' ');
            words.push_str(&said);
        }
        Arrival {
            id: format!("ended:{}", self.session_id),
            arrived: Arrived::Ended,
            source: Source::Fleet,
            session_id: self.session_id.clone(),
            manifest_id: self.manifest_id.clone(),
            subject: self.title.clone(),
            words,
            paths: self.paths.clone(),
            issues: self.issues.clone(),
            group: None,
            ways: ways(&self.unblocks.open, self.files),
            answers: Vec::new(),
            rule: None,
            open: self.unblocks.open.clone(),
            ties: self.unblocks.ties(self.starter.as_ref()),
        }
    }
}

/// A pull request a hosted Job held that merged.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Merged {
    pub session_id: String,
    pub manifest_id: String,
    /// The Job that held it.
    pub job: String,
    pub number: String,
    pub title: String,
    pub unblocks: Unblocks,
    pub starter: Option<Tied>,
    pub paths: Vec<String>,
    pub issues: Vec<String>,
    pub files: bool,
}

impl Merged {
    pub fn arrival(&self) -> Arrival {
        let mut words = format!("Pull request #{} \"{}\" merged. Job \"{}\" held it.", self.number, self.title, self.job);
        if let Some(starter) = &self.starter {
            words.push_str(&format!(" That Job was started by Job {} \"{}\".", starter.session_id, starter.title));
        }
        for said in self.unblocks.said() {
            words.push(' ');
            words.push_str(&said);
        }
        Arrival {
            id: format!("merged:{}:{}", self.session_id, self.number),
            arrived: Arrived::Merged,
            source: Source::Fleet,
            session_id: self.session_id.clone(),
            manifest_id: self.manifest_id.clone(),
            subject: self.title.clone(),
            words,
            paths: self.paths.clone(),
            issues: self.issues.clone(),
            group: None,
            ways: ways(&self.unblocks.open, self.files),
            answers: Vec::new(),
            rule: None,
            open: self.unblocks.open.clone(),
            ties: self.unblocks.ties(self.starter.as_ref()),
        }
    }
}

impl Operator {
    /// `session` ended: what waits in the queue on its own run, or is out on a call, is Done as
    /// Ended. **A call out on it goes on without it**: the ruling that comes back is dropped, and
    /// whoever waited on the ruling is told it passed on.
    pub(crate) async fn settled_by_end(&self, session: &str) {
        let gone: Vec<super::Held> = {
            let mut guard = held(&self.queue);
            let queue = &mut *guard;
            let mut gone = Vec::new();
            for from in [&mut queue.pending, &mut queue.out] {
                let (ended, rest): (Vec<_>, Vec<_>) = std::mem::take(from).into_iter().partition(|one| one.arrival.session_id == session && one.arrival.arrived.ends_with_job());
                *from = rest;
                gone.extend(ended);
            }
            gone
        };
        if gone.is_empty() {
            return;
        }
        for one in gone {
            let arrival = &one.arrival;
            self.desk
                .keep(Logged {
                    arrival: arrival.id.clone(),
                    arrived: arrival.arrived,
                    source: arrival.source.clone(),
                    session_id: arrival.session_id.clone(),
                    manifest_id: arrival.manifest_id.clone(),
                    subject: arrival.subject.clone(),
                    outcome: Outcome::Ended,
                    other: None,
                    shared: Vec::new(),
                    because: "the Job ended while this waited".to_string(),
                    call: None,
                    by: Actor::Operator,
                    handed: one.handed.clone(),
                    at: self.desk.now(),
                    place: None,
                    first: None,
                    refused: Vec::new(),
                    answer: None,
                    issue: None,
                })
                .await;
            let _ = one.tell.send(Ruled::PassedOn(PassedOn::Settled("the Job ended before the ruling came".to_string())));
        }
        self.desk.board_changed();
    }
}
