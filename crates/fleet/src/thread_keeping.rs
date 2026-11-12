//! The threads a client has read, held so a change to one reaches it as `job_thread.tail`.
//!
//! Memory only. A Job is held from its first `get_job_thread` until it ends or 32 other Jobs have been read
//! since. Each change Fleet hears of it (a row, the record, a timeline entry) is applied to the held
//! thread, the thread is folded again, and the first item that differs is what goes out.

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::{Arc, Mutex};

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use ipc::{
    Event, JobThread, JobThreadChanged, JobThreadRead, JobThreadTail, SessionId, SessionRecord, SessionRow,
    SessionState, SessionThread, ThreadHead, ThreadItem, TimelineEvent, TimelineKind, TimelineSubject,
};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use crate::daemon::Fleet;
use crate::thread_view::{cut_after, folded, folded_cut, revision_paths, tail_of, window, Revisions};

/// How many Jobs are held at once: the ones read most recently.
pub(crate) const HELD_MOST: usize = 32;

/// A `before` no item of the thread holds. A 422.
const THREAD_ITEM_UNKNOWN: &str = "fleet.thread_item_unknown";

/// The timeline kinds that make a `change` item.
const CHANGES: [TimelineKind; 3] = [TimelineKind::SessionWorkflow, TimelineKind::SessionModel, TimelineKind::SessionEffort];

/// One thing Fleet heard of a held Job.
pub(crate) enum Change {
    Row(SessionRow),
    Record(Box<SessionRecord>),
    Timeline(TimelineEvent),
}

struct Reading {
    thread: SessionThread,
    timeline: Vec<TimelineEvent>,
    /// The sketch revisions read so far, each once: a kept revision never changes.
    revisions: Revisions,
    head: ThreadHead,
    items: Vec<ThreadItem>,
}

#[derive(Default)]
struct Entry {
    reading: Option<Reading>,
    /// Set while a client's read of the Job is being made: what is heard meanwhile is applied to that read.
    buffer: Option<Vec<Change>>,
}

#[derive(Default)]
struct Entries {
    map: HashMap<String, Entry>,
    /// Oldest read first.
    recent: VecDeque<String>,
}

/// The held threads.
#[derive(Default)]
pub(crate) struct Held {
    /// The ids in `entries`, kept apart so the event tap, which must not wait, never waits on a fold.
    watched: Mutex<HashSet<String>>,
    entries: tokio::sync::Mutex<Entries>,
}

impl Held {
    pub(crate) fn is_watched(&self, id: &str) -> bool {
        self.watched.lock().map(|watched| watched.contains(id)).unwrap_or(false)
    }

    fn mark(&self, id: &str, held: bool) {
        if let Ok(mut watched) = self.watched.lock() {
            if held {
                watched.insert(id.to_string());
            } else {
                watched.remove(id);
            }
        }
    }

    fn forget(&self, entries: &mut Entries, id: &str) {
        entries.map.remove(id);
        entries.recent.retain(|one| one != id);
        self.mark(id, false);
    }

    /// A client's read of `id` is about to be made: what is heard until [`Held::read_through`] is applied to it.
    pub(crate) async fn begin(&self, id: &str) {
        let mut entries = self.entries.lock().await;
        entries.recent.retain(|one| one != id);
        entries.recent.push_back(id.to_string());
        entries.map.entry(id.to_string()).or_default().buffer = Some(Vec::new());
        self.mark(id, true);
        while entries.recent.len() > HELD_MOST {
            if let Some(oldest) = entries.recent.pop_front() {
                entries.map.remove(&oldest);
                self.mark(&oldest, false);
            }
        }
    }

    /// The read failed, so nothing is held for it.
    pub(crate) async fn abandon(&self, id: &str) {
        let mut entries = self.entries.lock().await;
        if entries.map.get(id).is_some_and(|entry| entry.reading.is_none()) {
            self.forget(&mut entries, id);
        } else if let Some(entry) = entries.map.get_mut(id) {
            entry.buffer = None;
        }
    }

    /// The read is made: apply what was heard since [`Held::begin`], hold the thread unless the Job ended, and
    /// answer its head and all its items.
    pub(crate) async fn read_through(
        &self,
        id: &str,
        mut thread: SessionThread,
        mut timeline: Vec<TimelineEvent>,
    ) -> (ThreadHead, Vec<ThreadItem>) {
        let mut entries = self.entries.lock().await;
        let heard = entries.map.get_mut(id).and_then(|entry| entry.buffer.take()).unwrap_or_default();
        for change in heard {
            apply(&mut thread, &mut timeline, change);
        }
        let mut revisions = entries.map.get_mut(id).and_then(|entry| entry.reading.take()).map(|was| was.revisions).unwrap_or_default();
        revisions_read(&thread.session, &mut revisions).await;
        let (head, items) = folded(&thread, &timeline, &revisions);
        if thread.session.state == SessionState::Ended {
            self.forget(&mut entries, id);
        } else if let Some(entry) = entries.map.get_mut(id) {
            entry.reading = Some(Reading { thread, timeline, revisions, head: head.clone(), items: items.clone() });
        }
        (head, items)
    }

    /// Whether the held Job runs in a terminal, whose rows are read off its transcript and not heard.
    pub(crate) async fn in_a_terminal(&self, id: &str) -> bool {
        let entries = self.entries.lock().await;
        let held = entries.map.get(id).and_then(|entry| entry.reading.as_ref());
        held.is_some_and(|reading| reading.thread.session.terminal.is_some())
    }

    /// What Fleet heard of a held Job, and the events that carry it. The Job is let go if it ended.
    pub(crate) async fn changed(&self, id: &str, changes: Vec<Change>) -> Vec<Event> {
        let mut entries = self.entries.lock().await;
        let Some(entry) = entries.map.get_mut(id) else { return Vec::new() };
        if let Some(buffer) = entry.buffer.as_mut() {
            buffer.extend(changes);
            return Vec::new();
        }
        let Some(reading) = entry.reading.as_mut() else { return Vec::new() };
        for change in changes {
            apply(&mut reading.thread, &mut reading.timeline, change);
        }
        revisions_read(&reading.thread.session, &mut reading.revisions).await;
        let events = refolded(reading);
        if reading.thread.session.state == SessionState::Ended {
            self.forget(&mut entries, id);
        }
        events
    }

    /// A terminal Job read again, which is how a change to one is found.
    pub(crate) async fn reread(&self, id: &str, thread: SessionThread, timeline: Vec<TimelineEvent>) -> Vec<Event> {
        let mut entries = self.entries.lock().await;
        let Some(entry) = entries.map.get_mut(id) else { return Vec::new() };
        if entry.buffer.is_some() {
            return Vec::new();
        }
        let Some(reading) = entry.reading.as_mut() else { return Vec::new() };
        reading.thread = thread;
        reading.timeline = timeline;
        revisions_read(&reading.thread.session, &mut reading.revisions).await;
        let events = refolded(reading);
        if reading.thread.session.state == SessionState::Ended {
            self.forget(&mut entries, id);
        }
        events
    }
}

/// Fold the held thread again, keep the result, and say what moved.
fn refolded(reading: &mut Reading) -> Vec<Event> {
    let (head, items) = folded(&reading.thread, &reading.timeline, &reading.revisions);
    let events = events_of(&reading.thread.session.id, (&reading.head, &reading.items), (&head, &items));
    reading.head = head;
    reading.items = items;
    events
}

/// Read the sketch revisions the record names that are not read yet, off the turn loop. A file that will
/// not read or draw is tried again at the next fold, and the sketch offers no revisions until it does.
async fn revisions_read(record: &SessionRecord, revisions: &mut Revisions) {
    let missing = revision_paths(record).into_iter().filter(|path| !revisions.contains_key(path)).collect::<Vec<_>>();
    if missing.is_empty() {
        return;
    }
    let read = tokio::task::spawn_blocking(move || {
        missing
            .into_iter()
            .filter_map(|path| {
                let scene = ipc::scene_of_detail(&std::fs::read_to_string(&path).ok()?)?;
                Some((path, scene))
            })
            .collect::<Vec<_>>()
    })
    .await;
    revisions.extend(read.unwrap_or_default());
}

/// Apply one thing heard to a thread. A row is appended, or replaces the row with its id.
fn apply(thread: &mut SessionThread, timeline: &mut Vec<TimelineEvent>, change: Change) {
    match change {
        Change::Row(row) => match thread.rows.iter().position(|held| held.id() == row.id()) {
            Some(at) => thread.rows[at] = row,
            None => thread.rows.push(row),
        },
        Change::Record(record) => thread.session = *record,
        Change::Timeline(event) => {
            let ours = matches!(&event.subject, TimelineSubject::Session { id } if *id == thread.session.id);
            if ours && CHANGES.contains(&event.kind) && !timeline.iter().any(|held| held.id == event.id) {
                timeline.push(event);
            }
        }
    }
}

/// What moved between two foldings of a thread: the tail from the first item that differs, or only the mark
/// where that is more than [`ipc::THREAD_TAIL_MOST`] items back, and the head where only it changed.
pub(crate) fn events_of(
    session: &SessionId,
    before: (&ThreadHead, &[ThreadItem]),
    after: (&ThreadHead, &[ThreadItem]),
) -> Vec<Event> {
    let Some((from, items)) = tail_of(before.1, after.1) else {
        if before.0 == after.0 {
            return Vec::new();
        }
        return vec![tail(session, None, Vec::new(), after.0)];
    };
    match from {
        Some(from) if items.len() > ipc::THREAD_TAIL_MOST => {
            vec![Event::JobThreadChanged(JobThreadChanged { session_id: session.clone(), from })]
        }
        from => vec![tail(session, from, items, after.0)],
    }
}

fn tail(session: &SessionId, from: Option<String>, items: Vec<ThreadItem>, head: &ThreadHead) -> Event {
    Event::JobThreadTail(JobThreadTail { session_id: session.clone(), from, items, head: head.clone() })
}

/// Start following the threads clients read. Production starts it beside the other keepers, and a test
/// calls [`Fleet::job_thread_changed`] itself.
pub fn keep_the_threads<H, V, W>(fleet: Arc<Fleet<H, V, W>>) -> JoinHandle<()>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    let (tell, mut heard) = mpsc::unbounded_channel::<(String, Change)>();
    let held = Arc::clone(&fleet.hosts().threads);
    fleet.events().tap(move |event| {
        let (id, change) = match event {
            Event::SessionRow(changed) => (changed.session_id.as_str(), Change::Row(changed.row.clone())),
            Event::SessionChanged(record) => (record.id.as_str(), Change::Record(Box::new(record.clone()))),
            Event::TimelineAppended(appended) => match &appended.event.subject {
                TimelineSubject::Session { id } => (id.as_str(), Change::Timeline(appended.event.clone())),
                _ => return,
            },
            _ => return,
        };
        if held.is_watched(id) {
            let _ = tell.send((id.to_string(), change));
        }
    });
    tokio::spawn(async move {
        while let Some(first) = heard.recv().await {
            // What is already waiting is folded once per Job, in the order it was heard.
            let mut batch: Vec<(String, Vec<Change>)> = Vec::new();
            for (id, change) in std::iter::once(first).chain(std::iter::from_fn(|| heard.try_recv().ok())) {
                match batch.iter_mut().find(|(one, _)| *one == id) {
                    Some((_, changes)) => changes.push(change),
                    None => batch.push((id, vec![change])),
                }
            }
            for (id, changes) in batch {
                fleet.job_thread_changed(&id, changes).await;
            }
        }
    })
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
    /// The Job's thread and the timeline entries that make its change items: what [`folded`] takes.
    pub(crate) async fn thread_read(
        self: &Arc<Self>,
        id: &SessionId,
    ) -> Result<(SessionThread, Vec<TimelineEvent>), api::Refusal> {
        let thread = ipc_get_session(self, id).await?;
        let kinds = CHANGES.map(TimelineKind::as_wire);
        let kinds = kinds.iter().map(String::as_str).collect::<Vec<_>>();
        let kept = self.store().lock().await.session_timeline(id.as_str(), &kinds).unwrap_or_default();
        let timeline = kept.iter().filter_map(crate::timeline::wire).collect();
        Ok((thread, timeline))
    }

    /// `get_job_thread`: fold the Job's thread, answer the window asked for, and follow the Job.
    pub(crate) async fn job_thread_served(self: Arc<Self>, read: JobThreadRead) -> Result<JobThread, api::Refusal> {
        if let Some(upto) = read.upto.as_deref() {
            return self.job_thread_cut(&read, upto).await;
        }
        let threads = Arc::clone(&self.hosts().threads);
        let id = read.session_id.clone();
        threads.begin(id.as_str()).await;
        let (thread, timeline) = match self.thread_read(&id).await {
            Ok(read) => read,
            Err(refusal) => {
                threads.abandon(id.as_str()).await;
                return Err(refusal);
            }
        };
        let (head, items) = threads.read_through(id.as_str(), thread, timeline).await;
        match window(&items, read.before.as_deref(), read.limit) {
            Some((items, older)) => Ok(JobThread { session_id: id, head, items, older }),
            None => Err(self.hosted_refusal(THREAD_ITEM_UNKNOWN, "no item of that job's thread holds that id")),
        }
    }

    /// `get_job_thread` with `upto`: the thread cut after that row, then folded, then windowed. **Not held**: the
    /// held reading is the whole live thread `job_thread.tail` follows, and a cut one must neither replace nor
    /// join it. The head is empty: [`folded_cut`].
    async fn job_thread_cut(self: Arc<Self>, read: &JobThreadRead, upto: &str) -> Result<JobThread, api::Refusal> {
        let (mut thread, mut timeline) = self.thread_read(&read.session_id).await?;
        let Some(end) = thread.rows.iter().position(|row| row.id() == upto) else {
            return Err(self.hosted_refusal(THREAD_ITEM_UNKNOWN, "no row of that job's thread holds that id"));
        };
        thread.rows.truncate(end + 1);
        // A change after the cut row is not part of the old thread.
        let cut_at = thread.rows[end].at().clone();
        timeline.retain(|event| event.at <= cut_at);
        let mut revisions = Revisions::new();
        revisions_read(&thread.session, &mut revisions).await;
        let (head, mut items) = folded_cut(&thread, &timeline, &revisions);
        cut_after(&mut items, upto);
        match window(&items, read.before.as_deref(), read.limit) {
            Some((items, older)) => Ok(JobThread { session_id: read.session_id.clone(), head, items, older }),
            None => Err(self.hosted_refusal(THREAD_ITEM_UNKNOWN, "no item of that job's thread holds that id")),
        }
    }

    /// Apply what was heard of a held Job and publish what moved.
    pub(crate) async fn job_thread_changed(self: &Arc<Self>, id: &str, changes: Vec<Change>) {
        let threads = Arc::clone(&self.hosts().threads);
        let reread = threads.in_a_terminal(id).await && changes.iter().any(|one| matches!(one, Change::Record(_)));
        let events = if reread {
            match self.thread_read(&SessionId::carried(id)).await {
                Ok((thread, timeline)) => threads.reread(id, thread, timeline).await,
                Err(_) => Vec::new(),
            }
        } else {
            threads.changed(id, changes).await
        };
        for event in events {
            self.publish(event);
        }
    }
}

/// `get_session`, which hosted and terminal Jobs both answer.
async fn ipc_get_session<H, V, W>(fleet: &Arc<Fleet<H, V, W>>, id: &SessionId) -> Result<SessionThread, api::Refusal>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    api::HostedSessions::get_session(Arc::clone(fleet), id.clone()).await
}
