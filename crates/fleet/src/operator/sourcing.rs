//! Fleet handing the Operator work from outside a Job's own run, and doing what it rules on it.
//! **Each arrives once**: a report as it is kept, an issue new since the last read of its tracker,
//! a red main as Autopilot's red kind finds it, and a note the person left on
//! Bridge once its file is new to the repository's annotations and has waited, open and sent to no
//! Job. A note he sends to a Job meanwhile is his row in the Log, and then arrives as a note of its
//! own, to be moved or left. Nothing arrives while the Operator is off.
//!
//! **Nothing is written to the forge.** A link is kept on this Mac, and File writes only to the
//! repository's beads, never to an issue the person or anyone else filed.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use adapter_traits::{AgentHarness, Delivery, OpenIssue, Vcs, WorkProduct};
use api::HostedSessions;
use ipc::annotations::{note_read, NoteRead};
use ipc::{ManifestId, Notice, SessionTag, SessionVoice, StartSession, TagKind};

use super::seat::Seat;
use super::suggesting::pressed;
use super::{held, Arrival, Arrived, Filed, Grouping, NewIssue, Note, Outcome, Report, Ruling, Way, NEEDS_YOU_WAIT};
use crate::autopilot::report_brief;
use crate::daemon::Fleet;

/// How long an open note sent to no Job is read again each tick, for the person to send it, before
/// it is handed on as it stands.
pub(crate) const NOTE_SENT_WAIT: Duration = Duration::from_secs(10 * 60);

const OPERATOR: &str = "Operator";

/// What is waiting to be handed, and what a call is out on.
#[derive(Default)]
pub(crate) struct Sources {
    out: Arc<Mutex<BTreeSet<String>>>,
    /// Reports kept since the last tick.
    reports: Mutex<Vec<String>>,
    /// The issues each repository's last read held.
    issues: Mutex<BTreeMap<String, BTreeSet<String>>>,
    /// Issues new since the read before, not yet handed.
    fresh: Mutex<Vec<NewIssue>>,
    /// The issues the Operator filed itself, which are not new to it.
    pub(super) filed: Mutex<BTreeSet<String>>,
    /// The note files each repository's last listing held. A repository with no entry has not been
    /// listed since Fleet started, so its first listing only marks what is there.
    notes: Mutex<BTreeMap<String, BTreeSet<String>>>,
    /// The open notes sent to no Job yet, by repository and id, read again each tick.
    watching: Mutex<BTreeMap<(String, String), Watched>>,
    /// How long they are watched where it is not [`NOTE_SENT_WAIT`], which a test shortens.
    sent_wait: Mutex<Option<Duration>>,
}

/// An open note the person may still send to a Job.
struct Watched {
    since: tokio::time::Instant,
    /// Whether it is handed on if it waits out. A note already there when Fleet started is not.
    hand: bool,
    /// Whether it was new since the last listing, so that its being sent already is news.
    news: bool,
    /// Whether a read of it has been made.
    read: bool,
}

/// A note that gained `sent`: the note, and the Job it went to as `(id, title)`.
pub(super) struct Sent {
    note: Note,
    job: String,
    title: Option<String>,
}

impl Seat {
    /// Report `id` was kept: it is handed on the next tick.
    pub(crate) fn report_noted(&self, id: &str) {
        held(&self.sources.reports).push(id.to_string());
    }

    /// What a fresh read of `manifest`'s tracker held. **The first read since Fleet started only
    /// marks what is there**, so the Operator meets what is new and not the whole backlog.
    pub(crate) fn issues_read(&self, manifest: &str, issues: &[OpenIssue]) {
        let now: BTreeSet<String> = issues.iter().map(|one| one.reference.clone()).collect();
        let Some(was) = held(&self.sources.issues).insert(manifest.to_string(), now) else { return };
        let filed = held(&self.sources.filed).clone();
        let mut fresh = held(&self.sources.fresh);
        // A child is a part of an issue already placed, such as a plan's task mirrored into beads;
        // one assigned is somebody's.
        let new = issues
            .iter()
            .filter(|one| !was.contains(&one.reference) && !filed.contains(&one.reference))
            .filter(|one| !one.closed && one.assignees.is_empty() && !one.reference.contains('.'));
        for one in new {
            fresh.push(NewIssue {
                manifest_id: manifest.to_string(),
                reference: one.reference.clone(),
                title: one.title.as_written().to_string(),
                excerpt: one.excerpt.as_written().to_string(),
                author: one.author.as_written().to_string(),
            });
        }
    }

    /// What `manifest`'s annotation directories hold now, as note ids: those not seen in an earlier
    /// listing, and whether this is the first listing since Fleet started, which marks what is there.
    fn notes_new(&self, manifest: &str, listed: BTreeSet<String>) -> (Vec<String>, bool) {
        let mut notes = held(&self.sources.notes);
        match notes.get_mut(manifest) {
            Some(seen) => (listed.into_iter().filter(|id| seen.insert(id.clone())).collect(), false),
            None => {
                notes.insert(manifest.to_string(), listed.clone());
                (listed.into_iter().collect(), true)
            }
        }
    }

    /// How long a note waits to be sent to a Job.
    fn note_wait(&self) -> Duration {
        held(&self.sources.sent_wait).unwrap_or(NOTE_SENT_WAIT)
    }

    /// Watch notes for a shorter time than [`NOTE_SENT_WAIT`].
    #[cfg(test)]
    pub(crate) fn note_wait_is(&self, wait: Duration) {
        *held(&self.sources.sent_wait) = Some(wait);
    }

    /// Whether `id` is waiting for the Operator or out on a call.
    pub(super) fn is_out(&self, id: &str) -> bool {
        held(&self.sources.out).contains(id)
    }

    /// Rule on `arrival` in the background, unless a call on it is already out. Whether it was handed.
    pub(crate) fn hand_sourced(&self, arrival: Arrival) -> bool {
        let Some(operator) = self.operator() else { return false };
        let out = Arc::clone(&self.sources.out);
        if !held(&out).insert(arrival.id.clone()) {
            return false;
        }
        tokio::spawn(async move {
            let id = arrival.id.clone();
            operator.ruled(arrival, NEEDS_YOU_WAIT).await;
            held(&out).remove(&id);
        });
        true
    }
}

impl<H, V, W> Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    /// Hand `arrival` to the Operator, unless the Log already holds it or it is waiting. Whether it was handed.
    pub(crate) async fn hand_once(&self, arrival: Arrival) -> bool {
        if self.store().lock().await.operator_ruled(&arrival.id).unwrap_or(true) {
            return false;
        }
        self.operator_seat().hand_sourced(arrival)
    }

    /// Each report kept, issue new and note left since the last tick, handed
    /// to the Operator once. While it is off, what waited is let go.
    pub async fn sources_handed(&self) {
        let seat = self.operator_seat();
        let reports = std::mem::take(&mut *held(&seat.sources.reports));
        let fresh = std::mem::take(&mut *held(&seat.sources.fresh));
        if !self.operator_on() {
            return;
        }
        for id in reports {
            if let Some(arrival) = self.report_of(&id).await {
                self.hand_once(arrival).await;
            }
        }
        for issue in fresh {
            let (open, files) = self.open_beside(&issue.manifest_id).await;
            self.hand_once(issue.arrival_among(&open, files)).await;
        }
        let (arrivals, sends) = self.notes_arrived().await;
        for sent in sends {
            self.note_sent_kept(sent).await;
        }
        for arrival in arrivals {
            self.hand_once(arrival).await;
        }
    }

    /// The notes of each hosted repository, as arrivals and as sends. **A note is read again each tick
    /// while it is open and sent to no Job**, for as long as [`NOTE_SENT_WAIT`]: one that gains `sent`
    /// is a send, and one that waits that long, still open and unsent, is an arrival as it stands. A
    /// note already there when Fleet started is watched too, but never handed on unsent.
    async fn notes_arrived(&self) -> (Vec<Arrival>, Vec<Sent>) {
        let (mut arrivals, mut sends) = (Vec::new(), Vec::new());
        let seat = self.operator_seat();
        let wait = seat.note_wait();
        for served in self.repositories().hosted() {
            let manifest = served.manifest().id().as_str().to_string();
            let places = adapters::annotation_places(std::path::Path::new(served.root()));
            let listed = tokio::task::spawn_blocking(move || note_files(&places)).await.unwrap_or_default();
            let ids: BTreeSet<String> = listed.iter().map(|(id, _)| id.clone()).collect();
            let (new, first) = seat.notes_new(&manifest, ids);
            {
                let mut watching = held(&seat.sources.watching);
                for id in new {
                    watching.entry((manifest.clone(), id)).or_insert(Watched { since: tokio::time::Instant::now(), hand: !first, news: !first, read: false });
                }
            }
            let ours: Vec<(String, std::path::PathBuf)> = {
                let watching = held(&seat.sources.watching);
                listed.iter().filter(|(id, _)| watching.contains_key(&(manifest.clone(), id.clone()))).cloned().collect()
            };
            let gone: Vec<(String, String)> = {
                let watching = held(&seat.sources.watching);
                watching.keys().filter(|(of, id)| *of == manifest && !ours.iter().any(|(there, _)| there == id)).cloned().collect()
            };
            for key in gone {
                held(&seat.sources.watching).remove(&key);
            }
            let paths: Vec<std::path::PathBuf> = ours.iter().map(|(_, path)| path.clone()).collect();
            let reads: Vec<Option<NoteRead>> = tokio::task::spawn_blocking(move || paths.iter().map(|path| note_read(&std::fs::read(path).ok()?).ok()).collect()).await.unwrap_or_default();
            let (mut left, mut sent) = (Vec::new(), Vec::new());
            for ((id, _), read) in ours.into_iter().zip(reads) {
                let key = (manifest.clone(), id);
                let Some(watched) = held(&seat.sources.watching).get(&key).map(|one| (one.since, one.hand, one.news, one.read)) else { continue };
                let (since, hand, news, was_read) = watched;
                let done = |key: &(String, String)| {
                    held(&seat.sources.watching).remove(key);
                };
                let Some(read) = read else {
                    done(&key);
                    continue;
                };
                if read.status != "open" {
                    done(&key);
                    continue;
                }
                if let Some(to) = &read.sent {
                    done(&key);
                    if news || was_read {
                        sent.push((to.session_id.clone(), to.title.clone(), read));
                    }
                    continue;
                }
                if let Some(one) = held(&seat.sources.watching).get_mut(&key) {
                    one.read = true;
                }
                if since.elapsed() >= wait {
                    done(&key);
                    if hand {
                        left.push(read);
                    }
                }
            }
            if left.is_empty() && sent.is_empty() {
                continue;
            }
            let open = self.open_of(&manifest).await;
            let files = self.served_named(Some(&ManifestId::carried(&manifest))).ok().is_some_and(|served| self.backlog_of(&served).is_some());
            let note = |read: NoteRead| Note { id: read.id, manifest_id: manifest.clone(), text: read.text, screen: read.screen, component: read.component, open: open.clone(), files };
            arrivals.extend(left.into_iter().map(|read| note(read).arrival()));
            sends.extend(sent.into_iter().map(|(job, title, read)| Sent { note: note(read), job, title }));
        }
        (arrivals, sends)
    }

    /// A note the person sent to a Job: his send is a row of the Log, signed by him, and then the
    /// Operator is handed the note as its own arrival, to move or leave. **Nothing is kept where the
    /// Log already holds the note**, such as one handed on as it stood.
    async fn note_sent_kept(&self, sent: Sent) {
        let unsent = sent.note.arrival();
        if self.store().lock().await.operator_ruled(&unsent.id).unwrap_or(true) {
            return;
        }
        let title = match sent.title {
            Some(title) => title,
            None => self.title_of(&sent.job).await,
        };
        let mut row = pressed(&unsent, Outcome::Sent, "the person sent it from Bridge", self.now());
        row.other = Some(sent.job.clone());
        if self.store().lock().await.add_operator_ruling(&row.kept()).is_err() {
            return;
        }
        self.operator_board_changed();
        self.hand_once(sent.note.sent_arrival(&sent.job, &title)).await;
    }

    /// The issues open in `manifest`'s tracker, and whether it is beads, which Fleet writes to.
    pub(super) async fn open_beside(&self, manifest: &str) -> (Vec<Filed>, bool) {
        match self.files_in(manifest) {
            true => (self.open_of(manifest).await, true),
            false => (Vec::new(), false),
        }
    }

    /// Whether `manifest`'s tracker is beads, the one Fleet writes to.
    pub(super) fn files_in(&self, manifest: &str) -> bool {
        self.served_named(Some(&ManifestId::carried(manifest))).ok().is_some_and(|served| self.backlog_of(&served).is_some())
    }

    /// The issues open in `manifest`'s tracker, as Fleet holds them.
    pub(super) async fn open_of(&self, manifest: &str) -> Vec<Filed> {
        let open = self.open_issues(&ManifestId::carried(manifest)).await.unwrap_or_default();
        open.into_iter()
            .filter(|one| !one.closed)
            .map(|one| Filed { reference: one.reference, title: one.title.as_written().to_string() })
            .collect()
    }

    async fn report_of(&self, id: &str) -> Option<Arrival> {
        let (report, manifest) = {
            let store = self.store().lock().await;
            let report = store.bug_report(id).ok().flatten()?;
            let hosting = store.hosting(&report.session_id).ok().flatten()?;
            (report, hosting.manifest_id)
        };
        let open = self.open_of(&manifest).await;
        Some(Report { id: report.id, session_id: report.session_id, manifest_id: manifest, title: report.title, what: report.what, open }.arrival())
    }

    /// What the Operator ruled for work from outside a Job's own run, and whether Fleet did it. A raise
    /// is its Log row. **What it made of a note** comes back, so a press on a suggestion can say it in
    /// the Log. A red main is Autopilot's own act, which says nothing of how it came out.
    pub(super) async fn operator_sourced(self: &Arc<Self>, arrival: &Arrival, ruling: &Ruling) -> Result<Sourced, String> {
        let (manifest, last) = (arrival.manifest_id.as_str(), arrival.id.rsplit(':').next().unwrap_or_default());
        let mut made = Sourced::default();
        let undone = |what: &str| Err(what.to_string());
        match (arrival.arrived, ruling.way, ruling.other.as_deref(), ruling.issue.as_deref()) {
            (Arrived::Report, Way::Link, _, Some(issue)) => {
                if self.store().lock().await.link_bug_report(last, issue).is_err() {
                    return undone("the report was not linked");
                }
            }
            (Arrived::Report, Way::NewJob, _, _) => self.operator_started_report(last, manifest).await?,
            (Arrived::Report, Way::ExistingJob, Some(into), _) => self.operator_handed_report(last, manifest, into).await?,
            (Arrived::RedMain, Way::NewJob, _, _) => self.operator_fixed_red(manifest, last).await,
            (Arrived::RedMain, Way::ExistingJob, Some(into), _) => self.operator_gave_red(manifest, last, into).await?,
            (Arrived::Start, Way::ExistingJob, Some(into), _) => self.operator_handed_start(arrival, into).await?,
            (Arrived::Note, Way::Link, _, _) => {
                if !self.note_settled_by_operator(manifest, last).await {
                    return undone("the note was not marked done");
                }
            }
            (Arrived::Note, Way::File, _, _) => made.issue = Some(self.operator_filed_note(manifest, last, arrival).await.ok_or("the issue was not filed")?),
            (Arrived::Note, Way::NewJob, _, _) => made.job = Some(self.operator_started_note(manifest, last, arrival).await.ok_or("the Job did not start")?),
            (Arrived::Note, Way::ExistingJob, Some(into), _) => self.operator_told_note(manifest, last, arrival, into).await?,
            (Arrived::Merged, Way::Close, _, Some(issue)) => {
                self.operator_closed_issue(manifest, issue, &format!("Done by #{last}, as the Operator ruled")).await?;
            }
            (Arrived::Issue, Way::Group, _, Some(open)) => {
                let new = super::suggesting::issue_of(arrival);
                if new.is_empty() {
                    return undone("the new issue's id does not read");
                }
                match ruling.grouping {
                    Some(Grouping::Part) => self.operator_put_under(manifest, new, open).await?,
                    Some(Grouping::Duplicate) => self.operator_closed_issue(manifest, new, &format!("Duplicate of {open}, as the Operator ruled")).await?,
                    None => return undone("the group names neither duplicate nor part"),
                }
            }
            (Arrived::Error, Way::File, _, _) => made.issue = Some(self.operator_filed_error(manifest, arrival).await.ok_or("the issue was not filed")?),
            (Arrived::Error, Way::NewJob, _, _) => made.job = Some(self.operator_started_error(manifest, arrival).await.ok_or("the Job did not start")?),
            (Arrived::Error, Way::ExistingJob, Some(into), _) => self.operator_told_error(arrival, into).await?,
            _ => {}
        }
        Ok(made)
    }

    /// A note no open issue covers, filed in the repository's beads with his words, and marked done.
    /// The issue it was filed as.
    async fn operator_filed_note(&self, manifest: &str, note: &str, arrival: &Arrival) -> Option<String> {
        let served = self.served_named(Some(&ManifestId::carried(manifest))).ok()?;
        let backlog = self.backlog_of(&served)?;
        let (root, title) = (served.root().to_string(), arrival.subject.clone());
        let body = format!("{}\n\nFiled by the Operator from a note he left on Bridge.", arrival.words);
        let filed = tokio::task::spawn_blocking(move || backlog.file(&root, &title, &body)).await;
        let Ok(Ok(reference)) = filed else { return None };
        held(&self.operator_seat().sources.filed).insert(reference.clone());
        self.note_settled_by_operator(manifest, note).await;
        self.tracking().forget(manifest);
        Some(reference)
    }

    /// A note the Operator ruled a Job of its own: started on it, briefed with his words, and the
    /// note marked sent to it. The Job's id.
    async fn operator_started_note(self: &Arc<Self>, manifest: &str, note: &str, arrival: &Arrival) -> Option<String> {
        let started = self
            .start_session(StartSession {
                project: None,
                manifest_id: ManifestId::carried(manifest),
                title: Some(arrival.subject.clone()),
                model: None,
                effort: None,
                mode: None,
                root: None,
                fork: None,
                request: None,
                after: None,
                held: false,
                project_rows: Vec::new(),
                on_your_words: None,
            })
            .await;
        let record = started.ok()?;
        let job = record.id.as_str().to_string();
        let voice = SessionVoice::Session { id: OPERATOR.to_lowercase(), title: OPERATOR.to_string() };
        let brief = format!("The person left this note on Bridge. Do what it asks.\n\n{}", arrival.words);
        if self.first_message_from(&job, voice, &brief).await.is_err() {
            let _ = self.closed_and_parked(ipc::CloseSession { session_id: record.id }).await;
            return None;
        }
        let title = self.title_of(&job).await;
        self.note_given_by_operator(manifest, note, &job, &title, arrival.is_sent_note()).await;
        Some(job)
    }

    /// A note the Operator ruled to live Job `into`, whose work is near it: told it, and the note
    /// marked sent to it.
    async fn operator_told_note(self: &Arc<Self>, manifest: &str, note: &str, arrival: &Arrival, into: &str) -> Result<(), String> {
        let text = format!("The Operator handed you a note the person left on Bridge, as your work is near it:\n\n{}\n\nDo it as part of your work, or say why it is not yours.", arrival.words);
        if !matches!(self.told_by_fleet(into, &text, Notice::Operator).await, Ok(true)) {
            return Err("the Job did not take the note".to_string());
        }
        let title = self.title_of(into).await;
        self.note_given_by_operator(manifest, note, into, &title, arrival.is_sent_note()).await;
        Ok(())
    }

    /// A report the Operator ruled a Job of its own: started on it, briefed with it, and holding it.
    async fn operator_started_report(self: &Arc<Self>, id: &str, manifest: &str) -> Result<(), String> {
        let Some(report) = self.store().lock().await.bug_report(id).ok().flatten() else { return Err("the report is not there".to_string()) };
        let started = self
            .start_session(StartSession {
                project: None,
                manifest_id: ManifestId::carried(manifest),
                title: Some(report.title.clone()),
                model: None,
                effort: None,
                mode: None,
                root: None,
                fork: None,
                request: None,
                after: None,
                held: false,
                project_rows: Vec::new(),
                on_your_words: None,
            })
            .await;
        let Ok(record) = started else { return Err("the Job did not start".to_string()) };
        let job = record.id.as_str().to_string();
        let voice = SessionVoice::Session { id: OPERATOR.to_lowercase(), title: OPERATOR.to_string() };
        if self.first_message_from(&job, voice, &report_brief(&report)).await.is_err() {
            let _ = self.closed_and_parked(ipc::CloseSession { session_id: record.id }).await;
            return Err("the Job was not briefed".to_string());
        }
        let tag = SessionTag { kind: TagKind::Report, id: report.id, title: report.title, url: None };
        self.reports_named(&job, manifest, &[tag]).await;
        Ok(())
    }

    /// A report the Operator ruled to live Job `into`, whose work is near it: told it, and holding it.
    async fn operator_handed_report(self: &Arc<Self>, id: &str, manifest: &str, into: &str) -> Result<(), String> {
        let Some(report) = self.store().lock().await.bug_report(id).ok().flatten() else { return Err("the report is not there".to_string()) };
        let text = format!(
            "The Operator handed you a bug report, as your work is near it:\n\n{}\n\nFix it as part of your work, or say why it is not yours.",
            report_brief(&report)
        );
        if !matches!(self.told_by_fleet(into, &text, Notice::Operator).await, Ok(true)) {
            return Err("the Job did not take the report".to_string());
        }
        let tag = SessionTag { kind: TagKind::Report, id: report.id, title: report.title, url: None };
        self.reports_named(into, manifest, &[tag]).await;
        Ok(())
    }
}

/// What a ruling on work from outside a Job made, as the Log row names it.
#[derive(Debug, Default)]
pub(super) struct Sourced {
    /// The issue a note was filed as.
    pub issue: Option<String>,
    /// The Job a note was started as.
    pub job: Option<String>,
}

/// The note files in `places`, as each one's id and path: the first of any id found twice.
fn note_files(places: &[std::path::PathBuf]) -> Vec<(String, std::path::PathBuf)> {
    let mut found: BTreeMap<String, std::path::PathBuf> = BTreeMap::new();
    for dir in places {
        for path in std::fs::read_dir(dir).into_iter().flatten().flatten().map(|entry| entry.path()) {
            let id = path.file_name().and_then(|name| name.to_str()).and_then(|name| name.strip_suffix(".json"));
            if let Some(id) = id.filter(|id| ipc::annotations::is_note_id(id)) {
                found.entry(id.to_string()).or_insert_with(|| path.clone());
            }
        }
    }
    found.into_iter().collect()
}
