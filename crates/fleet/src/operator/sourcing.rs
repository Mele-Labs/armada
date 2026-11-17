//! Fleet handing the Operator's session work from outside a Job's own run. **Each arrives once**: a
//! report as it is kept, an issue new since the last read of its tracker, and a note the person left on
//! Bridge once its file is new to the repository's annotations and has waited, open and sent to no
//! Job. Nothing arrives while the Operator is off or its session is not running, and what waited is let go.
//!
//! **Nothing is written to the forge.**

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use adapter_traits::{AgentHarness, Delivery, OpenIssue, Vcs, WorkProduct};
use ipc::annotations::{note_read, NoteRead};

use super::seat::Seat;
use super::{held, Arrival, NewIssue, Note, Report};
use crate::daemon::Fleet;

/// How long an open note sent to no Job is read again each tick, for the person to send it, before
/// it is handed on as it stands.
pub(crate) const NOTE_SENT_WAIT: Duration = Duration::from_secs(10 * 60);

/// What is waiting to be handed.
#[derive(Default)]
pub(crate) struct Sources {
    /// Reports kept since the last tick.
    reports: Mutex<Vec<String>>,
    /// The issues each repository's last read held.
    issues: Mutex<BTreeMap<String, BTreeSet<String>>>,
    /// Issues new since the read before, not yet handed.
    fresh: Mutex<Vec<NewIssue>>,
    /// The note files each repository's last listing held. A repository with no entry has not been
    /// listed since Fleet started, so its first listing only marks what is there.
    notes: Mutex<BTreeMap<String, BTreeSet<String>>>,
    /// The open notes sent to no Job yet, by repository and id, read again each tick.
    watching: Mutex<BTreeMap<(String, String), Watched>>,
    /// The projects no seed describes, so a tick does not look at their repositories again.
    pub(super) unseedable: Mutex<BTreeSet<String>>,
}

/// An open note the person may still send to a Job.
struct Watched {
    since: tokio::time::Instant,
    /// Whether it is handed on if it waits out. A note already there when Fleet started is not.
    hand: bool,
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
        let mut fresh = held(&self.sources.fresh);
        // A child is a part of an issue already placed, such as a plan's task mirrored into beads;
        // one assigned is somebody's.
        let new = issues
            .iter()
            .filter(|one| !was.contains(&one.reference))
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
        NOTE_SENT_WAIT
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
    /// Tell the session of `arrival`, once. Whether it was told or had been told already: **`false` where the
    /// Operator is off or its session is not running**, and the arrival reaches nobody.
    pub(crate) async fn hand_once(self: &Arc<Self>, arrival: Arrival) -> bool {
        self.operator_on() && self.arrival_told(&arrival).await
    }

    /// Each report kept, issue new and note left since the last tick, told to the session once. While it is
    /// off or not running, what waited is let go.
    pub async fn sources_handed(self: &Arc<Self>) {
        let seat = self.operator_seat();
        let reports = std::mem::take(&mut *held(&seat.sources.reports));
        let fresh = std::mem::take(&mut *held(&seat.sources.fresh));
        self.job_asks_handed().await;
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
        for arrival in self.notes_arrived().await {
            self.hand_once(arrival).await;
        }
        self.catalog_handed().await;
    }

    /// What Jobs asked the Operator that nothing has raised yet. **While the session is not running they stand on
    /// the sending Job's Needs you at once**; while it runs each is told to it, once, and stands answered here.
    async fn job_asks_handed(self: &Arc<Self>) {
        let asks = self.store().lock().await.unraised_job_asks().unwrap_or_default();
        for ask in asks {
            let arrival = super::sources::ask_arrival(&ask);
            if self.hand_once(arrival).await {
                let at = self.now().as_str().to_string();
                let _ = self.store().lock().await.answer_job_ask(&ask.id, "told the Operator", &at);
            } else {
                self.job_ask_raised(&ask.id).await;
            }
        }
    }

    /// The notes of each hosted repository, as arrivals. **A note is read again each tick while it is open
    /// and sent to no Job**, for as long as [`NOTE_SENT_WAIT`]: one sent to a Job meanwhile is that Job's,
    /// and one that waits that long, still open and unsent, is an arrival as it stands. A note already
    /// there when Fleet started is watched too, but never handed on unsent.
    async fn notes_arrived(&self) -> Vec<Arrival> {
        let mut arrivals = Vec::new();
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
                    watching.entry((manifest.clone(), id)).or_insert(Watched { since: tokio::time::Instant::now(), hand: !first });
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
            let mut left = Vec::new();
            for ((id, _), read) in ours.into_iter().zip(reads) {
                let key = (manifest.clone(), id);
                let Some((since, hand)) = held(&seat.sources.watching).get(&key).map(|one| (one.since, one.hand)) else { continue };
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
                if read.sent.is_some() {
                    done(&key);
                    continue;
                }
                if since.elapsed() >= wait {
                    done(&key);
                    if hand {
                        left.push(read);
                    }
                }
            }
            arrivals.extend(left.into_iter().map(|read| Note { id: read.id, manifest_id: manifest.clone(), text: read.text, screen: read.screen, component: read.component }.arrival()));
        }
        arrivals
    }

    async fn report_of(&self, id: &str) -> Option<Arrival> {
        let (report, manifest) = {
            let store = self.store().lock().await;
            let report = store.bug_report(id).ok().flatten()?;
            let hosting = store.hosting(&report.session_id).ok().flatten()?;
            (report, hosting.manifest_id)
        };
        Some(Report { id: report.id, session_id: report.session_id, manifest_id: manifest, title: report.title, what: report.what }.arrival())
    }
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
