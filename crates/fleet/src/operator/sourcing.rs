//! Fleet handing the Operator work from outside a Job's own run, and doing what it rules on it.
//! **Each arrives once**: a report as it is kept, an issue new since the last read of its tracker,
//! a red main as Autopilot's red kind finds it, a review request read at most every ten minutes
//! beside the read of main, and a Later item as a Job hands it over. Nothing arrives while the
//! Operator is off.
//!
//! **Nothing is written to the forge.** A link is kept on this Mac, and File writes only to the
//! repository's beads, never to an issue the person or anyone else filed.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use adapter_traits::{AgentHarness, Delivery, OpenIssue, ReviewList, ReviewListAsk, Vcs, WorkProduct};
use api::HostedSessions;
use ipc::{ManifestId, Notice, SessionTag, SessionVoice, StartSession, TagKind, MOST_PER_LIST};

use super::seat::Seat;
use super::{held, Arrival, Arrived, Filed, Handed, NewIssue, Report, ReviewRequest, Ruling, Way, NEEDS_YOU_WAIT};
use crate::autopilot::report_brief;
use crate::daemon::Fleet;
use crate::pull_review_lists::row_of;

/// The least time between two reads of the pull requests that ask for the person's review.
pub(crate) const REVIEWS_EVERY: Duration = Duration::from_secs(10 * 60);

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
    filed: Mutex<BTreeSet<String>>,
    reviews_read: Mutex<Option<tokio::time::Instant>>,
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

    /// Whether the review requests are due a read, marking them read where they are.
    fn reviews_due(&self) -> bool {
        let mut last = held(&self.sources.reviews_read);
        if last.is_some_and(|at| at.elapsed() < REVIEWS_EVERY) {
            return false;
        }
        *last = Some(tokio::time::Instant::now());
        true
    }

    /// Rule on `arrival` in the background, unless a call on it is already out.
    fn hand_sourced(&self, arrival: Arrival) {
        let Some(operator) = self.operator() else { return };
        let out = Arc::clone(&self.sources.out);
        if !held(&out).insert(arrival.id.clone()) {
            return;
        }
        tokio::spawn(async move {
            let id = arrival.id.clone();
            operator.ruled(arrival, NEEDS_YOU_WAIT).await;
            held(&out).remove(&id);
        });
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
    /// Hand `arrival` to the Operator, unless the Log already holds it.
    pub(crate) async fn hand_once(&self, arrival: Arrival) {
        if self.store().lock().await.operator_ruled(&arrival.id).unwrap_or(true) {
            return;
        }
        self.operator_seat().hand_sourced(arrival);
    }

    /// Each report kept, issue new and Later item handed over since the last tick, handed to the
    /// Operator once. While it is off, what waited is let go.
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
            self.hand_once(issue.arrival()).await;
        }
        let handed = self.store().lock().await.later_handed().unwrap_or_default();
        for one in handed {
            if let Some(arrival) = self.handed_of(one).await {
                self.hand_once(arrival).await;
            }
        }
    }

    /// The issues open in `manifest`'s tracker, as Fleet holds them.
    async fn open_of(&self, manifest: &str) -> Vec<Filed> {
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

    async fn handed_of(&self, one: store::KeptHanded) -> Option<Arrival> {
        let (title, manifest) = {
            let store = self.store().lock().await;
            let session = store.session(&one.session_id).ok().flatten()?;
            let hosting = store.hosting(&one.session_id).ok().flatten()?;
            (session.title.unwrap_or_else(|| one.session_id.clone()), hosting.manifest_id)
        };
        let served = self.served_named(Some(&ManifestId::carried(&manifest))).ok()?;
        let files = self.backlog_of(&served).is_some();
        let open = self.open_of(&manifest).await;
        Some(Handed { session_id: one.session_id, manifest_id: manifest, title, later_id: one.id, sentence: one.sentence, open, files }.arrival())
    }

    /// The pull requests that ask for the person's review, each handed once: at most every ten
    /// minutes, from the read of main, and nothing while the Operator is off.
    pub(crate) async fn review_requests_handed(&self) {
        if !self.operator_on() || !self.operator_seat().reviews_due() {
            return;
        }
        let Ok(from) = self.served_named(None) else { return };
        let Some(me) = self.mine_in(from.root()).await.login().map(str::to_string) else { return };
        let served = self.named_repositories().await;
        let repositories: Vec<String> = served.iter().map(|it| it.slug.clone()).collect();
        if repositories.is_empty() {
            return;
        }
        let listed = self
            .forge_asked(from.root(), move |vcs: &V, root: &str| {
                vcs.pull_review_list(root, &ReviewListAsk { list: ReviewList::Requested, login: &me, repositories: &repositories, most: MOST_PER_LIST })
            })
            .await;
        for pull in listed.unwrap_or_default() {
            let Some(row) = row_of(pull, &served) else { continue };
            let request = ReviewRequest { manifest_id: row.manifest_id.as_str().to_string(), number: row.number, title: row.title, author: row.author, url: row.address };
            self.hand_once(request.arrival()).await;
        }
    }

    /// What the Operator ruled for work from outside a Job's own run. A raise is its Log row.
    pub(super) async fn operator_sourced(self: &Arc<Self>, arrival: &Arrival, ruling: &Ruling) {
        let (manifest, last) = (arrival.manifest_id.as_str(), arrival.id.rsplit(':').next().unwrap_or_default());
        match (arrival.arrived, ruling.way, ruling.other.as_deref(), ruling.issue.as_deref()) {
            (Arrived::Report, Way::Link, _, Some(issue)) => {
                let _ = self.store().lock().await.link_bug_report(last, issue);
            }
            (Arrived::Report, Way::NewJob, _, _) => self.operator_started_report(last, manifest).await,
            (Arrived::Report, Way::ExistingJob, Some(into), _) => self.operator_handed_report(last, manifest, into).await,
            (Arrived::RedMain, Way::NewJob, _, _) => self.operator_fixed_red(manifest, last).await,
            (Arrived::RedMain, Way::ExistingJob, Some(into), _) => self.operator_gave_red(manifest, last, into).await,
            (Arrived::Later, Way::Link, _, Some(issue)) => {
                let now = self.now();
                let _ = self.store().lock().await.later_handed_filed(&arrival.session_id, last, issue, now.as_str());
            }
            (Arrived::Later, Way::File, _, _) => self.operator_filed(&arrival.session_id, last, manifest, &arrival.subject).await,
            _ => {}
        }
    }

    /// A report the Operator ruled a Job of its own: started on it, briefed with it, and holding it.
    async fn operator_started_report(self: &Arc<Self>, id: &str, manifest: &str) {
        let Some(report) = self.store().lock().await.bug_report(id).ok().flatten() else { return };
        let started = self
            .start_session(StartSession {
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
        let Ok(record) = started else { return };
        let job = record.id.as_str().to_string();
        let voice = SessionVoice::Session { id: OPERATOR.to_lowercase(), title: OPERATOR.to_string() };
        if self.first_message_from(&job, voice, &report_brief(&report)).await.is_err() {
            let _ = self.closed_and_parked(ipc::CloseSession { session_id: record.id }).await;
            return;
        }
        let tag = SessionTag { kind: TagKind::Report, id: report.id, title: report.title, url: None };
        self.reports_named(&job, manifest, &[tag]).await;
    }

    /// A report the Operator ruled to live Job `into`, whose work is near it: told it, and holding it.
    async fn operator_handed_report(self: &Arc<Self>, id: &str, manifest: &str, into: &str) {
        let Some(report) = self.store().lock().await.bug_report(id).ok().flatten() else { return };
        let text = format!(
            "The Operator handed you a bug report, as your work is near it:\n\n{}\n\nFix it as part of your work, or say why it is not yours.",
            report_brief(&report)
        );
        if matches!(self.told_by_fleet(into, &text, Notice::Operator).await, Ok(true)) {
            let tag = SessionTag { kind: TagKind::Report, id: report.id, title: report.title, url: None };
            self.reports_named(into, manifest, &[tag]).await;
        }
    }

    /// A Later item no open issue covers, filed in the repository's beads and settled as filed.
    async fn operator_filed(&self, session: &str, later: &str, manifest: &str, sentence: &str) {
        let Ok(served) = self.served_named(Some(&ManifestId::carried(manifest))) else { return };
        let Some(backlog) = self.backlog_of(&served) else { return };
        let title = self.store().lock().await.session(session).ok().flatten().and_then(|one| one.title).unwrap_or_else(|| session.to_string());
        let body = format!("Kept for later by Job \"{title}\", and filed by the Operator.");
        let (root, sentence) = (served.root().to_string(), sentence.to_string());
        let filed = tokio::task::spawn_blocking(move || backlog.file(&root, &sentence, &body)).await;
        let Ok(Ok(reference)) = filed else { return };
        held(&self.operator_seat().sources.filed).insert(reference.clone());
        let now = self.now();
        let _ = self.store().lock().await.later_handed_filed(session, later, &reference, now.as_str());
        self.tracking().forget(manifest);
    }
}
