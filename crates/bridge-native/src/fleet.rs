//! The Fleet connection as an entity: the one owner of what Fleet has said.
//!
//! Views read from this entity and none opens a connection of its own. A task drains the link's
//! channel, folds **a whole batch** into [`Held`] and notifies once; the Electron Bridge's slowness
//! was redraw volume, one per event (#3356).

use std::collections::{BTreeMap, BTreeSet};

use gpui::{Context, EventEmitter, Task};

use ipc::{
    CallArrival, Cursor, Event, FleetBuildReport, Instant, JobThread, JobThreadTail, MergeLineView, ModKind, ModSummary, NeedsYou, SessionRecord,
    Projects, RepositorySummary, SettingValue, SettingsList, StandingCalls, StreamMessage, ThreadHead, ThreadItem, WaitingItem,
    WatchList, WatchSummary,
};

use crate::link::{self, LinkState, Signal};
use crate::log::Note;

/// A Job's thread as Fleet last served it, kept current by `job_thread.tail`. Plain data: the folding
/// is Fleet's, and this only replaces what the tail says changed.
#[derive(Clone, Debug, PartialEq)]
pub struct HeldThread {
    pub head: ThreadHead,
    /// Oldest first.
    pub items: Vec<ThreadItem>,
    /// Older items exist: this is the `before` that reads them.
    pub older: Option<String>,
    /// Moves on every change, so a view redraws a thread only when this is not the one it drew.
    pub version: u64,
    /// A change reached further back than a tail carries, or a stream gap lost some: read it again.
    pub stale: bool,
    /// The first item that changed since a view last took this (`Held::take_dirty`), so it
    /// measures again from there and no earlier.
    dirty_from: Option<usize>,
    /// How many items were put before the first since a view last took this (`Held::take_prepended`),
    /// so it keeps the place the person is reading.
    prepended: usize,
}

impl HeldThread {
    pub fn from_read(read: JobThread, version: u64) -> Self {
        HeldThread { head: read.head, items: read.items, older: read.older, version, stale: false, dirty_from: Some(0), prepended: 0 }
    }

    /// The thread changed from item `at` on: a view measures again from there.
    pub fn mark_dirty(&mut self, at: usize) {
        self.dirty_from = Some(self.dirty_from.map_or(at, |held| held.min(at)));
    }

    /// Older items, read by `before`, go in front of those held.
    fn older_read(&mut self, read: JobThread) {
        self.prepended += read.items.len();
        self.items.splice(0..0, read.items);
        self.older = read.older;
        self.dirty_from = Some(0);
        self.version += 1;
    }

    /// `items` replace what is held from the item `from` to the end. A client that does not hold
    /// `from` holds only items after it, so it replaces everything with `items`. No `from` is an append.
    fn tailed(&mut self, tail: JobThreadTail) {
        let at = match tail.from {
            Some(from) => self.items.iter().position(|item| item.id() == from).unwrap_or(0),
            None => self.items.len(),
        };
        self.items.truncate(at);
        self.dirty_from = Some(self.dirty_from.map_or(at, |held| held.min(at)));
        self.items.extend(tail.items);
        self.head = tail.head;
        self.version += 1;
    }
}

/// A Job's Needs you as Fleet grouped it (`GET /sessions/needs_you`), with the `waiting_for` it was
/// read for: when the record's list is not this one, the view is read again.
#[derive(Clone, Debug, PartialEq)]
pub struct HeldNeeds {
    pub view: NeedsYou,
    pub read_for: Vec<WaitingItem>,
}

/// What Fleet has said, folded. Plain data, so a fold is tested without a window.
pub struct Held {
    pub state: LinkState,
    pub jobs: Vec<SessionRecord>,
    pub cursor: Option<Cursor>,
    /// The theme word, once Fleet has been read for it. `None` until then.
    pub theme: Option<String>,
    /// The person's saved key changes, as the JSON Settings wrote them; empty while he has made none.
    /// `None` until Fleet has been read for them.
    pub key_bindings: Option<String>,
    /// The mods on this machine, as Fleet last listed them. `None` until the first list.
    pub mods: Option<Vec<ModSummary>>,
    /// The build Fleet runs, as it last reported. **Held across a drop**, since a restart is the
    /// commonest reason Fleet does not answer. `None` until the first read, and where this Fleet serves none.
    pub build: Option<FleetBuildReport>,
    /// Every call standing, as Fleet last said. `None` until Fleet has answered once, so an empty
    /// first reading cannot be mistaken for a count that stands.
    pub calls: Option<StandingCalls>,
    /// Every repository Fleet serves, the one it was started in first. `None` until Fleet has answered.
    pub repositories: Option<Vec<RepositorySummary>>,
    /// His projects as Fleet last said, whole. `None` until Fleet has answered.
    pub projects: Option<Projects>,
    /// Every served repository's merge line, in Fleet's order, as Fleet last read it. `None` until Fleet has answered.
    pub merge_lines: Option<Vec<MergeLineView>>,
    /// The plan's windows as Fleet last read them, **held across a drop**, as Electron keeps the rings. `None` until Fleet has answered.
    pub plan_usage: Option<ipc::PlanUsage>,
    /// Where the step down stands, as Fleet last said, held across a drop. `None` until Fleet has answered.
    pub downshift: Option<ipc::ModelDownshiftState>,
    /// Every standing write clash, as Fleet last said. `None` until Fleet has answered, and again once the
    /// connection drops, as Electron clears its marks: a clash Fleet cannot vouch for is not drawn.
    pub clashes: Option<ipc::AllWriteClashes>,
    /// What the runtime file said on the last attempt, with its path. `None` until it has been read.
    pub presence: Option<(crate::runtime_file::Presence, String)>,
    /// When the link last went unreachable, when Fleet last sent anything, and when a connection last dropped.
    pub silent_since: Option<std::time::Instant>,
    pub read_at: Option<std::time::Instant>,
    pub dropped: Option<std::time::Instant>,
    /// Whether `features.notifications` is on. On until Fleet's settings say false, as Electron reads it.
    pub notifications: bool,
    /// News since the last time it was taken: a thing that began to wait on him. Told once.
    arrived: Vec<CallArrival>,
    /// The port of the Fleet last connected to, so a command goes where the events come from.
    port: Option<u16>,
    /// The stylesheet Fleet checked for a mod, with the `changed_at` of the row it was checked for.
    sheets: BTreeMap<String, (Option<Instant>, String)>,
    /// The threads a view has read, by Job id. Fleet sends a tail for each of these and no other.
    pub threads: BTreeMap<String, HeldThread>,
    /// The Needs you of each Job a view has read, by Job id.
    pub needs: BTreeMap<String, HeldNeeds>,
    /// Each Job's watches, live and ended, newest first, as `watches.changed` last carried them or
    /// `list_watches` answered when a view first asked. By Job id.
    pub watches: BTreeMap<String, Vec<WatchSummary>>,
    /// The Jobs whose watches a view has asked for since the connection, so each is read once.
    watches_asked: BTreeSet<String>,
    /// The Jobs `watches.changed` has spoken for since the connection: an older read must not land over it.
    watches_spoken: BTreeSet<String>,
    /// Moves on each `operator.page_changed` and each gap in the stream: the Operator's page reads again.
    pub operator_page_moved: u64,
}

impl Held {
    pub(crate) fn new() -> Self {
        Held {
            state: LinkState::NotRunning("not yet read".into()),
            jobs: Vec::new(),
            cursor: None,
            theme: None,
            key_bindings: None,
            mods: None,
            build: None,
            calls: None,
            repositories: None,
            projects: None,
            merge_lines: None,
            plan_usage: None,
            downshift: None,
            clashes: None,
            presence: None,
            silent_since: None,
            read_at: None,
            dropped: None,
            notifications: true,
            arrived: Vec::new(),
            port: None,
            sheets: BTreeMap::new(),
            threads: BTreeMap::new(),
            needs: BTreeMap::new(),
            watches: BTreeMap::new(),
            watches_asked: BTreeSet::new(),
            watches_spoken: BTreeSet::new(),
            operator_page_moved: 0,
        }
    }

    /// Whether the view held for this Job was read for this very list, so asking again would answer the same.
    pub fn needs_current(&self, session: &str, waiting: &[WaitingItem]) -> bool {
        self.needs.get(session).is_some_and(|held| held.read_for == waiting)
    }

    /// A thread read whole replaces the one held.
    pub fn thread_read(&mut self, read: JobThread) {
        let id = read.session_id.as_str().to_string();
        let version = self.threads.get(&id).map_or(1, |held| held.version + 1);
        self.threads.insert(id, HeldThread::from_read(read, version));
    }

    /// The Jobs whose threads must be read again, each once.
    pub fn stale_threads(&self) -> BTreeSet<String> {
        self.threads.iter().filter(|(_, held)| held.stale).map(|(id, _)| id.clone()).collect()
    }

    /// The first item of `session`'s thread that changed since this was last asked, and nothing
    /// changed since after it. A view measures its list from there.
    pub fn take_dirty(&mut self, session: &str) -> Option<usize> {
        self.threads.get_mut(session)?.dirty_from.take()
    }

    /// Older items read for a Job go before the ones held.
    pub fn thread_older(&mut self, read: JobThread) {
        if let Some(held) = self.threads.get_mut(read.session_id.as_str()) {
            held.older_read(read);
        }
    }

    /// How many items went before the first since this was last asked.
    pub fn take_prepended(&mut self, session: &str) -> usize {
        self.threads.get_mut(session).map_or(0, |held| std::mem::take(&mut held.prepended))
    }

    fn all_stale(&mut self) {
        self.threads.values_mut().for_each(|held| held.stale = true);
        self.operator_page_moved += 1;
        // A gap may have lost a `watches.changed`: each Job's watches are read again when a view next asks.
        self.watches_asked.clear();
        self.watches_spoken.clear();
    }

    /// A read of a Job's watches lands unless `watches.changed` has spoken for that Job since it was asked.
    fn watches_read(&mut self, list: WatchList) {
        let id = list.session_id.as_str().to_string();
        if !self.watches_spoken.contains(&id) {
            self.watches.insert(id, list.watches);
        }
    }

    /// The checked stylesheets of the theme mods that are on and valid, by mod name. A sheet checked
    /// for an older version of a mod than the list now shows is not offered.
    pub fn theme_sheets(&self) -> BTreeMap<String, String> {
        let listed = self.mods.as_deref().unwrap_or_default();
        self.sheets
            .iter()
            .filter(|(name, (at, _))| {
                listed.iter().any(|row| {
                    &row.name == *name
                        && row.enabled
                        && row.valid
                        && row.kind.is_none_or(|kind| kind == ModKind::Theme)
                        && &row.changed_at == at
                })
            })
            .map(|(name, (_, css))| (name.clone(), css.clone()))
            .collect()
    }

    /// The port Fleet answers on while it is connected, or `None`: a command asked then has nobody to ask.
    pub fn port(&self) -> Option<u16> {
        match self.state {
            LinkState::Connected => self.port,
            _ => None,
        }
    }

    /// The live link, as the title bar reads it.
    pub fn link(&self) -> crate::titlebar::LiveLink {
        crate::titlebar::LiveLink {
            state: self.state.clone(),
            presence: self.presence.clone(),
            silent_since: self.silent_since,
            read_at: self.read_at,
            dropped: self.dropped,
        }
    }

    /// The Dock count: the calls still to be opened, one for each owner. `None` until Fleet has answered.
    pub fn dock_count(&self) -> Option<usize> {
        self.calls.as_ref().map(|calls| calls.unopened().len())
    }

    /// The news folded since the last take, oldest first.
    pub fn take_arrived(&mut self) -> Vec<CallArrival> {
        std::mem::take(&mut self.arrived)
    }

    fn set_settings(&mut self, list: &SettingsList) {
        let switch = list.settings.iter().find(|one| one.key == "features.notifications").map(|one| &one.value);
        self.notifications = switch != Some(&SettingValue::Bool(false));
    }

    fn listed(&mut self, mods: Vec<ModSummary>) {
        self.sheets.retain(|name, _| mods.iter().any(|row| &row.name == name));
        self.mods = Some(mods);
    }

    /// Fold one batch; the notes say what it held, one for the stream and one for each other fact.
    pub fn fold(&mut self, batch: Vec<Signal>) -> Vec<Note> {
        let mut lines = Vec::new();
        let (mut events, mut resyncs, mut missed) = (0usize, 0usize, 0u64);
        let mut kinds: BTreeMap<String, usize> = BTreeMap::new();
        for signal in batch {
            match signal {
                Signal::State(state) => {
                    if let LinkState::Connecting { port, .. } = &state {
                        self.port = Some(*port);
                    }
                    if state != self.state {
                        lines.push(describe(&state));
                        if !matches!(state, LinkState::Connected) {
                            self.clashes = None;
                        }
                        if matches!(self.state, LinkState::Connected) {
                            self.dropped = Some(std::time::Instant::now());
                        }
                        if matches!(state, LinkState::Unreachable(_)) && !matches!(self.state, LinkState::Unreachable(_)) {
                            self.silent_since = Some(std::time::Instant::now());
                        }
                        self.state = state;
                    }
                }
                Signal::Presence(presence, path) => self.presence = Some((presence, path)),
                Signal::Jobs(jobs) => {
                    lines.push(Note::info("jobs read on connect").field("jobs", jobs.len()));
                    self.jobs = jobs;
                }
                Signal::JobsFailed(why) => lines.push(Note::warn("jobs could not be read").field("why", why)),
                Signal::Preferences(preferences) => {
                    if self.theme.as_deref() != Some(preferences.theme.as_str()) {
                        lines.push(Note::info("theme preference read").field("theme", preferences.theme.as_str()));
                    }
                    self.theme = Some(preferences.theme);
                    self.key_bindings = Some(preferences.key_bindings);
                }
                Signal::PreferencesFailed(why) => lines.push(Note::warn("preferences could not be read").field("why", why)),
                Signal::Mods(list) => {
                    lines.push(Note::info("mods read on connect").field("mods", list.mods.len()));
                    self.listed(list.mods);
                }
                Signal::ModsFailed(why) => lines.push(Note::warn("mods could not be read").field("why", why)),
                Signal::ModChecked { checked, changed_at } => match checked.css {
                    Some(css) if checked.valid => {
                        self.sheets.insert(checked.name, (changed_at, css));
                    }
                    _ => {
                        lines.push(
                            Note::warn("a mod did not pass its checks")
                                .field("mod", checked.name.as_str())
                                .field("problems", checked.problems.join("; ")),
                        );
                        self.sheets.remove(&checked.name);
                    }
                },
                Signal::Calls(calls) => self.calls = Some(calls),
                Signal::CallsFailed(why) => lines.push(Note::warn("calls could not be read").field("why", why)),
                Signal::Settings(list) => self.set_settings(&list),
                Signal::SettingsFailed(why) => lines.push(Note::warn("settings could not be read").field("why", why)),
                Signal::Repositories(list) => self.repositories = Some(list.repositories),
                Signal::RepositoriesFailed(why) => lines.push(Note::warn("repositories could not be read").field("why", why)),
                Signal::Projects(read) => self.projects = Some(read),
                Signal::ProjectsFailed(why) => lines.push(Note::warn("projects could not be read").field("why", why)),
                Signal::MergeLines(view) => self.merge_lines = Some(view.lines),
                Signal::MergeLinesFailed(why) => lines.push(Note::warn("the merge line could not be read").field("why", why)),
                Signal::Clashes(all) => self.clashes = Some(all),
                Signal::ClashesFailed(why) => lines.push(Note::warn("the write clashes could not be read").field("why", why)),
                Signal::PlanUsage(usage) => self.plan_usage = Some(usage),
                Signal::PlanUsageFailed(why) => lines.push(Note::warn("the plan usage could not be read").field("why", why)),
                Signal::Downshift(state) => self.downshift = Some(state),
                Signal::DownshiftFailed(why) => lines.push(Note::warn("the step down could not be read").field("why", why)),
                Signal::Build(report) => self.build = Some(report),
                Signal::BuildFailed(why) => lines.push(Note::warn("the build could not be read").field("why", why)),
                Signal::Stream(StreamMessage::Resync(resync)) => {
                    self.read_at = Some(std::time::Instant::now());
                    resyncs += 1;
                    self.cursor = Some(resync.cursor);
                    self.all_stale();
                }
                Signal::Stream(StreamMessage::Missed(lost)) => {
                    self.read_at = Some(std::time::Instant::now());
                    missed += lost.dropped;
                    self.all_stale();
                }
                Signal::Stream(StreamMessage::Event(delivered)) => {
                    self.read_at = Some(std::time::Instant::now());
                    events += 1;
                    self.cursor = Some(delivered.cursor);
                    *kinds.entry(delivered.event.kind()).or_default() += 1;
                    // A row arrives whole and replaces the one held, never patches it.
                    match delivered.event {
                        Event::OperatorPageChanged(_) => self.operator_page_moved += 1,
                        Event::SessionChanged(record) => {
                            match self.jobs.iter_mut().find(|held| held.id == record.id) {
                                Some(held) => *held = record,
                                None => self.jobs.push(record),
                            }
                        }
                        // The list arrives whole and replaces the one held.
                        Event::ModsChanged(list) => self.listed(list.mods),
                        Event::JobThreadTail(tail) => {
                            if let Some(held) = self.threads.get_mut(tail.session_id.as_str()) {
                                held.tailed(tail);
                            }
                        }
                        // Further back than a tail carries: a client that holds the item reads again.
                        Event::JobThreadChanged(changed) => {
                            if let Some(held) = self.threads.get_mut(changed.session_id.as_str()) {
                                if held.items.iter().any(|item| item.id() == changed.from) {
                                    held.stale = true;
                                }
                            }
                        }
                        // The calls arrive whole; the news rides with them and is told once.
                        Event::CallsChanged(changed) => {
                            self.calls = Some(StandingCalls { calls: changed.calls });
                            self.arrived.extend(changed.arrived);
                        }
                        Event::SettingsChanged(list) => self.set_settings(&list),
                        Event::RepositoriesChanged(list) => self.repositories = Some(list.repositories),
                        // His projects arrive whole and replace the ones held.
                        Event::ProjectsChanged(read) => self.projects = Some(read),
                        // The merge lines arrive whole and replace the ones held.
                        Event::MergeLineViewChanged(view) => self.merge_lines = Some(view.lines),
                        Event::WriteClashesChanged(all) => self.clashes = Some(all),
                        Event::PlanUsageChanged(usage) => self.plan_usage = Some(usage),
                        Event::ModelDownshiftChanged(state) => self.downshift = Some(state),
                        // A Job's watches arrive whole and replace the ones held.
                        Event::WatchesChanged(list) => {
                            let id = list.session_id.as_str().to_string();
                            self.watches_spoken.insert(id.clone());
                            self.watches.insert(id, list.watches);
                        }
                        _ => {}
                    }
                }
            }
        }
        if events + resyncs > 0 || missed > 0 {
            let kinds: Vec<String> = kinds.iter().map(|(kind, n)| format!("{kind} x{n}")).collect();
            let mut note = Note::info("stream batch folded")
                .field("events", events)
                .field("kinds", kinds.join(", "))
                .field("resyncs", resyncs)
                .field("missed", missed)
                .field("jobs_held", self.jobs.len());
            if let Some(at) = self.cursor {
                note = note.field("cursor", at.position());
            }
            lines.push(note);
        }
        lines
    }
}

fn describe(state: &LinkState) -> Note {
    match state {
        LinkState::NotRunning(why) => Note::info("Fleet is not running").field("why", why.as_str()),
        LinkState::Refused(why) => Note::warn("runtime file refused").field("why", why.as_str()),
        LinkState::ProtocolMismatch { fleet, ours } => Note::warn("protocol mismatch, not connecting")
            .field("fleet", fleet.to_string())
            .field("ours", ours.to_string()),
        LinkState::Connecting { port, pid } => {
            Note::info("connecting to Fleet").field("port", u32::from(*port)).field("pid", *pid)
        }
        LinkState::Starting => Note::info("Fleet has not answered yet and is still starting"),
        LinkState::Unreachable(why) => Note::warn("Fleet is unreachable").field("why", why.as_str()),
        LinkState::Connected => Note::info("connected to Fleet"),
    }
}

/// How old the merge line kept on disk may be and still draw on open (`remembered.ts`).
const MERGE_LINES_FRESH: chrono::Duration = chrono::Duration::minutes(5);

fn merge_lines_file() -> Option<std::path::PathBuf> {
    crate::jobs::deck_file().map(|deck| deck.with_file_name("native-merge-lines"))
}

/// The line as Fleet last said it, with when, so the next open can draw it before Fleet answers.
fn keep_merge_lines(lines: &[ipc::MergeLineView]) {
    let Some(path) = merge_lines_file() else { return };
    let kept = ipc::encode(&ipc::MergeLineViews { lines: lines.to_vec() })
        .map_err(|why| format!("{why:?}"))
        .and_then(|json| std::fs::write(&path, format!("{}\n{json}", chrono::Utc::now().to_rfc3339())).map_err(|why| why.to_string()));
    if let Err(why) = kept {
        Note::warn("the merge line was not kept").field("why", why).emit();
    }
}

/// The kept line, where it was kept within [`MERGE_LINES_FRESH`] of `now`.
fn kept_merge_lines(now: chrono::DateTime<chrono::Utc>) -> Option<Vec<ipc::MergeLineView>> {
    let text = std::fs::read_to_string(merge_lines_file()?).ok()?;
    let (at, json) = text.split_once('\n')?;
    let at = chrono::DateTime::parse_from_rfc3339(at).ok()?;
    (now.signed_duration_since(at) <= MERGE_LINES_FRESH).then_some(())?;
    ipc::decode::<ipc::MergeLineViews>("MergeLineViews", json.as_bytes()).ok().map(|read| read.lines)
}

/// What began to wait on him, since the last time this was said. Said once for each batch of events.
pub struct Arrived(pub Vec<CallArrival>);

impl EventEmitter<Arrived> for FleetConnection {}

pub struct FleetConnection {
    held: Held,
    /// The Jobs whose thread is being read now, so one read is not asked twice.
    reading: BTreeSet<String>,
    /// The Jobs whose older items are being read now.
    earlier: BTreeSet<String>,
    /// The Jobs whose last read failed: not asked again until a view asks with `retry`, or the
    /// redraw a failure causes would ask for ever.
    failed: BTreeSet<String>,
    /// The Jobs whose Needs you is being read now, by the `waiting_for` asked for.
    needs_reading: BTreeMap<String, Vec<WaitingItem>>,
    /// The Jobs whose last Needs you read failed, by the `waiting_for` asked for: not asked again until
    /// the list changes, or the redraw a failure causes would ask for ever.
    needs_failed: BTreeMap<String, Vec<WaitingItem>>,
    /// The build read every [`FOLLOW`] after an act, until the report stops saying a change is under way.
    following: Option<Task<()>>,
    /// Kept, or the drain is cancelled with it.
    _drain: Task<()>,
}

/// How often the build is read while a restart, revert, swap or snapshot is under way (`FOLLOW_MS`).
const FOLLOW: std::time::Duration = std::time::Duration::from_secs(2);
/// A change still under way after this is a script that died, and the follow stops (`FOLLOW_FOR_MS`).
const FOLLOW_FOR: std::time::Duration = std::time::Duration::from_secs(20 * 60);

/// What the build switcher asks of Fleet, as Electron's `FleetBuilds` asks it.
#[derive(Clone, Debug, PartialEq)]
pub enum BuildAct {
    Change(ipc::ChangeFleetBuild),
    Revert,
    Swap,
    Keep,
    Pick(ipc::PickFleetBuildJob),
}

impl BuildAct {
    fn route(&self) -> &'static str {
        match self {
            BuildAct::Change(_) => "/fleet/build/change",
            BuildAct::Revert => "/fleet/build/revert",
            BuildAct::Swap => "/fleet/build/swap",
            BuildAct::Keep => "/fleet/build/keep",
            BuildAct::Pick(_) => "/fleet/build/pick",
        }
    }

    fn body(&self) -> Option<Result<String, String>> {
        let encoded = |body: Result<String, ipc::Unencodable>| body.map_err(|why| format!("{why:?}"));
        match self {
            BuildAct::Change(change) => Some(encoded(ipc::encode(change))),
            BuildAct::Pick(pick) => Some(encoded(ipc::encode(pick))),
            BuildAct::Revert | BuildAct::Swap | BuildAct::Keep => None,
        }
    }
}

/// A restart, revert, swap or snapshot is under way, so the build is followed.
fn changing(report: &FleetBuildReport) -> bool {
    report.restarting.is_some() || report.reverting || report.swapping || report.snapshot.as_ref().is_some_and(|kept| kept.state == ipc::SnapshotState::Building)
}

impl FleetConnection {
    /// What Fleet has said, folded. Read it in an `observe` handler; it moves once per batch.
    pub fn held(&self) -> &Held {
        &self.held
    }

    /// See [`Held::take_dirty`]. No redraw: the view that asks is already drawing.
    pub fn take_dirty(&mut self, session: &str) -> Option<usize> {
        self.held.take_dirty(session)
    }

    /// See [`Held::take_prepended`].
    pub fn take_prepended(&mut self, session: &str) -> usize {
        self.held.take_prepended(session)
    }

    /// Whether older items of this Job's thread are being read.
    /// Whether the Job's thread is being read again now, as Electron's `refreshing` reads its `reading` list.
    pub fn reading(&self, session: &str) -> bool {
        self.reading.contains(session)
    }

    pub fn reading_older(&self, session: &str) -> bool {
        self.earlier.contains(session)
    }

    /// Read the items before the first one held (`GET /sessions/thread?before=`) off the main thread and
    /// put them in front. A read already under way is not asked twice; a refusal is noted.
    pub fn read_older(&mut self, session: &str, cx: &mut Context<Self>) {
        let Some(port) = self.held.port() else { return };
        let Some(before) = self.held.threads.get(session).and_then(|held| held.older.clone()) else { return };
        if !self.earlier.insert(session.to_string()) {
            return;
        }
        let id = session.to_string();
        let path = format!("/sessions/thread?session_id={session}&before={before}");
        cx.notify();
        cx.spawn(async move |this, cx| {
            let answer = cx
                .background_executor()
                .spawn(async move {
                    let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().map_err(|why| why.to_string())?;
                    let (status, bytes) = runtime.block_on(crate::transport::get(port, &path))?;
                    if !(200..300).contains(&status) {
                        return Err(format!("Fleet answered {status}: {}", String::from_utf8_lossy(&bytes)));
                    }
                    ipc::decode::<JobThread>("JobThread", &bytes).map_err(|why| why.to_string())
                })
                .await;
            let _ = this.update(cx, |this, cx| {
                this.earlier.remove(&id);
                match answer {
                    Ok(read) => this.held.thread_older(read),
                    Err(why) => Note::warn("older items of a Job's thread could not be read").field("job", id.as_str()).field("why", why).emit(),
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// A revert has been asked and Fleet took it: the report says so until the next read, as Electron's
    /// `FleetBuilds.revert` marks it, so the menu's item goes off at once. The old failure is gone.
    pub fn mark_reverting(&mut self, cx: &mut Context<Self>) {
        if let Some(build) = self.held.build.as_mut() {
            build.reverting = true;
            build.failed = None;
            cx.notify();
        }
        self.follow(false, cx);
    }

    /// Ask Fleet for a build act off the main thread. A change, revert or swap Fleet took is marked on the
    /// report at once and followed; one it refused is said on the report's `failed` line, where a failed
    /// restart is said. A pick or a keep is read again at once, and a refusal of one is noted, as Electron
    /// leaves it to the row pressed.
    pub fn build_act(&mut self, act: BuildAct, cx: &mut Context<Self>) {
        let Some(port) = self.held.port() else {
            Note::warn("not sent: no Fleet is connected").field("route", act.route()).emit();
            return;
        };
        let route = act.route();
        let body = act.body();
        cx.spawn(async move |this, cx| {
            let read = cx
                .background_executor()
                .spawn(async move {
                    let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().map_err(|why| why.to_string())?;
                    match body {
                        Some(body) => runtime.block_on(crate::transport::post(port, route, body?)),
                        None => runtime.block_on(crate::transport::post_bare(port, route)),
                    }
                })
                .await;
            let outcome = crate::revert::outcome_of(read);
            let _ = this.update(cx, |this, cx| {
                let took = outcome == crate::revert::Outcome::Started;
                let marks = matches!(act, BuildAct::Change(_) | BuildAct::Revert | BuildAct::Swap);
                match (took, this.held.build.as_mut()) {
                    (true, Some(build)) if marks => {
                        build.failed = None;
                        match &act {
                            BuildAct::Change(change) => build.restarting = Some(change.build),
                            BuildAct::Revert => build.reverting = true,
                            _ => build.swapping = true,
                        }
                    }
                    (false, Some(build)) if marks => build.failed = Some(crate::revert::why_not(&outcome).to_string()),
                    (false, _) => Note::warn("Fleet refused a build act").field("route", route).field("why", crate::revert::why_not(&outcome)).emit(),
                    _ => {}
                }
                cx.notify();
                if took {
                    this.follow(!marks, cx);
                }
            });
        })
        .detach();
    }

    /// Read the build every [`FOLLOW`] (the first at once where `now`) while a change is under way, or while
    /// Fleet is down in the middle of one, for at most [`FOLLOW_FOR`]. A later act starts it over.
    fn follow(&mut self, now: bool, cx: &mut Context<Self>) {
        let until = std::time::Instant::now() + FOLLOW_FOR;
        self.following = Some(cx.spawn(async move |this, cx| {
            let mut wait = !now;
            loop {
                if wait {
                    cx.background_executor().timer(FOLLOW).await;
                }
                wait = true;
                let Ok(port) = this.read_with(cx, |this, _| this.held.port()) else { return };
                if let Some(port) = port {
                    let read = cx
                        .background_executor()
                        .spawn(async move {
                            let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().map_err(|why| why.to_string())?;
                            let (status, bytes) = runtime.block_on(crate::transport::get(port, "/fleet/build"))?;
                            if !(200..300).contains(&status) {
                                return Err(format!("Fleet answered {status}"));
                            }
                            ipc::decode::<FleetBuildReport>("FleetBuildReport", &bytes).map_err(|why| why.to_string())
                        })
                        .await;
                    // A read that failed leaves the report held, as the minute's watch does.
                    if let Ok(report) = read {
                        let _ = this.update(cx, |this, cx| {
                            if this.held.build.as_ref() != Some(&report) {
                                this.held.build = Some(report);
                                cx.notify();
                            }
                        });
                    }
                }
                let Ok(going) = this.read_with(cx, |this, _| port.is_none() || this.held.build.as_ref().is_some_and(changing)) else { return };
                if !going || std::time::Instant::now() >= until {
                    return;
                }
            }
        }));
    }

    /// Read a Job's thread (`GET /sessions/thread`) off the main thread and hold it. Fleet follows the
    /// Job from this first read and sends its changes as `job_thread.tail`. A read already under
    /// way is not asked again; a refused one is a toast in Fleet's words, the view draws nothing for
    /// it, and it is asked again only with `retry`, which is the person opening the Job.
    /// Whether the last read of this Job's thread was refused, and none has been answered since.
    pub fn thread_refused(&self, session: &str) -> bool {
        self.failed.contains(session)
    }

    pub fn read_thread(&mut self, session: &str, retry: bool, cx: &mut Context<Self>) {
        let Some(port) = self.held.port() else { return };
        if retry {
            self.failed.remove(session);
        } else if self.failed.contains(session) {
            return;
        }
        if !self.reading.insert(session.to_string()) {
            return;
        }
        let id = session.to_string();
        let path = format!("/sessions/thread?session_id={session}");
        cx.spawn(async move |this, cx| {
            let answer = cx
                .background_executor()
                .spawn(async move {
                    let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().map_err(|why| why.to_string())?;
                    let (status, bytes) = runtime.block_on(crate::transport::get(port, &path))?;
                    if !(200..300).contains(&status) {
                        return Err(ipc::decode::<ipc::WireError>("WireError", &bytes).map_or_else(|_| format!("Fleet answered {status}"), |error| error.message));
                    }
                    ipc::decode::<JobThread>("JobThread", &bytes).map_err(|why| why.to_string())
                })
                .await;
            let _ = this.update(cx, |this, cx| {
                this.reading.remove(&id);
                match answer {
                    Ok(read) => this.held.thread_read(read),
                    Err(why) => {
                        this.failed.insert(id.clone());
                        Note::warn("a Job's thread could not be read").field("job", id.as_str()).field("why", why.as_str()).emit();
                        cx.defer(move |cx| crate::toasts::tell(why, cx));
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// Read a Job's Needs you (`GET /sessions/needs_you`) off the main thread and hold it, for the
    /// `waiting_for` the Job's record carries now. Asked again only when that list is not the one held, being
    /// read or failed for: a view calls this each time the record moves and the call is a compare.
    pub fn read_needs(&mut self, session: &str, waiting: Vec<WaitingItem>, cx: &mut Context<Self>) {
        let Some(port) = self.held.port() else { return };
        if self.held.needs_current(session, &waiting)
            || self.needs_reading.get(session) == Some(&waiting)
            || self.needs_failed.get(session) == Some(&waiting)
        {
            return;
        }
        self.needs_reading.insert(session.to_string(), waiting.clone());
        let id = session.to_string();
        let path = format!("/sessions/needs_you?session_id={session}");
        cx.spawn(async move |this, cx| {
            let answer = cx
                .background_executor()
                .spawn(async move {
                    let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().map_err(|why| why.to_string())?;
                    let (status, bytes) = runtime.block_on(crate::transport::get(port, &path))?;
                    if !(200..300).contains(&status) {
                        return Err(format!("Fleet answered {status}: {}", String::from_utf8_lossy(&bytes)));
                    }
                    ipc::decode::<NeedsYou>("NeedsYou", &bytes).map_err(|why| why.to_string())
                })
                .await;
            let _ = this.update(cx, |this, cx| {
                this.needs_reading.remove(&id);
                match answer {
                    Ok(view) => {
                        this.needs_failed.remove(&id);
                        this.held.needs.insert(id, HeldNeeds { view, read_for: waiting });
                    }
                    Err(why) => {
                        Note::warn("a Job's Needs you could not be read").field("job", id.as_str()).field("why", why).emit();
                        this.needs_failed.insert(id, waiting);
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// Read a Job's watches, live and ended (`GET /watches?session_id=&ended=true`), once since the
    /// connection; `watches.changed` keeps them current after. A failed read is asked again next time.
    pub fn read_watches(&mut self, session: &str, cx: &mut Context<Self>) {
        let Some(port) = self.held.port() else { return };
        if !self.held.watches_asked.insert(session.to_string()) {
            return;
        }
        let id = session.to_string();
        let path = format!("/watches?session_id={session}&ended=true");
        cx.spawn(async move |this, cx| {
            let answer = cx
                .background_executor()
                .spawn(async move {
                    let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().map_err(|why| why.to_string())?;
                    let (status, bytes) = runtime.block_on(crate::transport::get(port, &path))?;
                    if !(200..300).contains(&status) {
                        return Err(ipc::decode::<ipc::WireError>("WireError", &bytes).map_or_else(|_| format!("Fleet answered {status}"), |error| error.message));
                    }
                    ipc::decode::<WatchList>("WatchList", &bytes).map_err(|why| why.to_string())
                })
                .await;
            let _ = this.update(cx, |this, cx| {
                match answer {
                    Ok(list) => this.held.watches_read(list),
                    Err(why) => {
                        this.held.watches_asked.remove(&id);
                        Note::warn("a Job's watches could not be read").field("job", id.as_str()).field("why", why).emit();
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    pub fn new(cx: &mut Context<Self>) -> Self {
        let (tx, rx) = async_channel::unbounded();
        let home = std::env::var("HOME").ok();
        Note::info("native Bridge started").field("protocol", ipc::ProtocolId::current().to_string()).emit();
        // Its own thread and runtime: nothing here waits on the network. The thread ends when the
        // drain below is dropped and the channel closes.
        let spawned = std::thread::Builder::new().name("fleet-link".into()).spawn(move || {
            match tokio::runtime::Builder::new_current_thread().enable_all().build() {
                Ok(runtime) => runtime.block_on(link::run(tx, home)),
                Err(why) => Note::error("the Fleet link could not start its runtime").field("why", why.to_string()).emit(),
            }
        });
        if let Err(why) = spawned {
            Note::error("the Fleet link could not start its thread").field("why", why.to_string()).emit();
        }
        let drain = cx.spawn(async move |this, cx| {
            while let Ok(first) = rx.recv().await {
                let mut batch = vec![first];
                while let Ok(more) = rx.try_recv() {
                    batch.push(more);
                }
                let folded = this.update(cx, |this, cx| {
                    let lines_were = this.held.merge_lines.clone();
                    let lines = this.held.fold(batch);
                    if this.held.merge_lines != lines_were {
                        if let Some(read) = &this.held.merge_lines {
                            keep_merge_lines(read);
                        }
                    }
                    // A change that reached too far back, or a gap in the stream: read those again.
                    for id in this.held.stale_threads() {
                        if let Some(held) = this.held.threads.get_mut(&id) {
                            held.stale = false;
                        }
                        this.read_thread(&id, true, cx);
                    }
                    let arrived = this.held.take_arrived();
                    cx.notify();
                    if !arrived.is_empty() {
                        cx.emit(Arrived(arrived));
                    }
                    lines
                });
                match folded {
                    Ok(lines) => lines.iter().for_each(Note::emit),
                    Err(_) => return,
                }
            }
        });
        let mut held = Held::new();
        // The strip draws at once on open from the last line read, as Electron's does, while it is fresh.
        held.merge_lines = kept_merge_lines(chrono::Utc::now());
        FleetConnection {
            held,
            reading: BTreeSet::new(),
            earlier: BTreeSet::new(),
            failed: BTreeSet::new(),
            needs_reading: BTreeMap::new(),
            needs_failed: BTreeMap::new(),
            following: None,
            _drain: drain,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core_model::FieldValue;
    use ipc::{Missed, ModChecked, ModList, Preferences, Resync};

    fn event(cursor: u64) -> Signal {
        let wire = format!(
            r#"{{"message":"event","cursor":{cursor},"event":{{"kind":"repositories.changed","repositories":[]}}}}"#
        );
        Signal::Stream(crate::frames::decode(&wire).expect("an event"))
    }

    fn mods(wire: &str) -> ModList {
        ipc::decode("mods", wire.as_bytes()).expect("a mod list")
    }

    fn checked(wire: &str) -> ModChecked {
        ipc::decode("a mod check", wire.as_bytes()).expect("a mod check")
    }

    #[test]
    fn a_batch_of_events_is_one_note_and_moves_the_cursor() {
        let mut held = Held::new();
        let lines = held.fold(vec![event(1), event(2), event(3)]);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].get("events"), Some(&FieldValue::Int(3)));
        assert_eq!(lines[0].get("kinds"), Some(&FieldValue::Str("repositories.changed x3".into())));
        assert_eq!(held.cursor, Some(Cursor::at(3)));
    }

    #[test]
    fn a_state_that_did_not_change_says_nothing() {
        let mut held = Held::new();
        assert_eq!(held.fold(vec![Signal::State(LinkState::Connected)]).len(), 1);
        assert!(held.fold(vec![Signal::State(LinkState::Connected)]).is_empty());
    }

    #[test]
    fn resync_and_missed_are_counted_in_the_batch_note() {
        let mut held = Held::new();
        let resync = Signal::Stream(StreamMessage::Resync(Resync {
            protocol_id: ipc::ProtocolId::current(),
            cursor: Cursor::at(9),
        }));
        let lost = Signal::Stream(StreamMessage::Missed(Missed { dropped: 4 }));
        let lines = held.fold(vec![lost, resync]);
        assert_eq!(lines[0].get("resyncs"), Some(&FieldValue::Int(1)));
        assert_eq!(lines[0].get("missed"), Some(&FieldValue::Int(4)));
        assert_eq!(lines[0].get("cursor"), Some(&FieldValue::Int(9)));
    }

    #[test]
    fn jobs_on_connect_replace_what_was_held() {
        let mut held = Held::new();
        let lines = held.fold(vec![Signal::Jobs(Vec::new())]);
        assert_eq!(lines[0].msg(), "jobs read on connect");
        assert_eq!(lines[0].get("jobs"), Some(&FieldValue::Int(0)));
    }

    #[test]
    fn the_theme_word_is_held_once_read() {
        let mut held = Held::new();
        assert_eq!(held.theme, None);
        held.fold(vec![Signal::Preferences(Preferences { theme: "light".into(), ..Preferences::default() })]);
        assert_eq!(held.theme.as_deref(), Some("light"));
    }

    #[test]
    fn a_checked_sheet_is_offered_only_for_the_mod_version_it_was_checked_against() {
        let mut held = Held::new();
        let listed = r#"{"mods":[{"name":"dusk","kind":"theme","enabled":true,"valid":true,"changed_at":"2026-11-08T10:00:00.000Z"}]}"#;
        held.fold(vec![
            Signal::Mods(mods(listed)),
            Signal::ModChecked {
                checked: checked(r#"{"name":"dusk","valid":true,"problems":[],"css":"[data-theme=\"dusk\"] { --accent: #C1B4EC; }"}"#),
                changed_at: Some(Instant::carried("2026-11-08T10:00:00.000Z")),
            },
        ]);
        assert!(held.theme_sheets().contains_key("dusk"));

        // The mod is edited: the list moves on and the old sheet is not offered.
        let edited = r#"{"mods":[{"name":"dusk","kind":"theme","enabled":true,"valid":true,"changed_at":"2026-11-08T11:00:00.000Z"}]}"#;
        held.fold(vec![Signal::Mods(mods(edited))]);
        assert!(held.theme_sheets().is_empty());
    }

    #[test]
    fn a_mod_switched_off_or_gone_offers_no_sheet() {
        let mut held = Held::new();
        let on = r#"{"mods":[{"name":"dusk","kind":"theme","enabled":true,"valid":true}]}"#;
        let check = Signal::ModChecked {
            checked: checked(r#"{"name":"dusk","valid":true,"problems":[],"css":"x"}"#),
            changed_at: None,
        };
        held.fold(vec![Signal::Mods(mods(on)), check]);
        assert!(held.theme_sheets().contains_key("dusk"));
        held.fold(vec![Signal::Mods(mods(r#"{"mods":[{"name":"dusk","kind":"theme","enabled":false,"valid":true}]}"#))]);
        assert!(held.theme_sheets().is_empty());
        held.fold(vec![Signal::Mods(mods(r#"{"mods":[]}"#))]);
        assert!(held.sheets.is_empty());
    }

    #[test]
    fn a_mod_that_fails_its_check_loses_its_sheet_and_says_why() {
        let mut held = Held::new();
        held.fold(vec![
            Signal::Mods(mods(r#"{"mods":[{"name":"dusk","kind":"theme","enabled":true,"valid":true}]}"#)),
            Signal::ModChecked { checked: checked(r#"{"name":"dusk","valid":true,"problems":[],"css":"x"}"#), changed_at: None },
        ]);
        let lines = held.fold(vec![Signal::ModChecked {
            checked: checked(r#"{"name":"dusk","valid":false,"problems":["a theme rule outside its block"]}"#),
            changed_at: None,
        }]);
        assert!(held.theme_sheets().is_empty());
        assert_eq!(lines[0].get("mod"), Some(&FieldValue::Str("dusk".into())));
    }

    #[test]
    fn the_build_report_is_held_through_a_drop_and_a_failed_read_leaves_it() {
        let mut held = Held::new();
        let report = ipc::decode::<FleetBuildReport>("build", br#"{"on":"main"}"#).expect("a report");
        held.fold(vec![Signal::Build(report.clone())]);
        let lines = held.fold(vec![Signal::State(LinkState::Unreachable("gone".into())), Signal::BuildFailed("Fleet answered 500".into())]);
        assert_eq!(held.build, Some(report));
        assert_eq!(lines.iter().filter(|line| line.msg() == "the build could not be read").count(), 1);
    }

    fn calls(opened: &[bool]) -> ipc::StandingCalls {
        let rows: Vec<String> = opened
            .iter()
            .enumerate()
            .map(|(at, opened)| {
                format!(
                    r#"{{"key":"session:s{at}:item","owner":"session:s{at}","kind":"job","session_id":"s{at}","title":"T","text":"x","since":"2026-11-09T10:0{at}:00.000Z","tone":"waiting","opened":{opened}}}"#
                )
            })
            .collect();
        ipc::decode("calls", format!(r#"{{"calls":[{}]}}"#, rows.join(",")).as_bytes()).expect("calls")
    }

    fn changed(wire_calls: &str, arrived: &str) -> Signal {
        let wire = format!(
            r#"{{"message":"event","cursor":1,"event":{{"kind":"calls.changed","calls":{wire_calls},"arrived":{arrived}}}}}"#
        );
        Signal::Stream(crate::frames::decode(&wire).expect("an event"))
    }

    #[test]
    fn the_dock_count_waits_for_fleet_and_then_counts_the_unopened_once_for_each_owner() {
        let mut held = Held::new();
        assert_eq!(held.dock_count(), None, "nothing is badged before Fleet has answered");
        held.fold(vec![Signal::Calls(ipc::StandingCalls::default())]);
        assert_eq!(held.dock_count(), Some(0), "an empty answer is a count of none");
        held.fold(vec![Signal::Calls(calls(&[false, true, false]))]);
        assert_eq!(held.dock_count(), Some(2));
    }

    #[test]
    fn calls_changed_replaces_the_calls_and_hands_over_its_news_once() {
        let mut held = Held::new();
        let arrival = r#"{"session_id":"s0","title":"Fix it","text":"Pick a slot","tone":"blocked","kind":"needs"}"#;
        held.fold(vec![changed("[]", &format!("[{arrival}]"))]);
        assert_eq!(held.dock_count(), Some(0));
        let news = held.take_arrived();
        assert_eq!(news.len(), 1);
        assert_eq!(news[0].title, "Fix it");
        assert!(held.take_arrived().is_empty(), "news is told once");
        held.fold(vec![changed("[]", "[]")]);
        assert!(held.take_arrived().is_empty());
    }

    #[test]
    fn the_repositories_are_read_on_connect_and_replaced_when_fleet_says_they_changed() {
        let mut held = Held::new();
        assert_eq!(held.repositories, None, "nothing is listed before Fleet has answered");
        let list: ipc::RepositoryList =
            ipc::decode("repositories", br#"{"repositories":[{"root":"/r/one","records_root":"/x/one"}]}"#).expect("a list");
        held.fold(vec![Signal::Repositories(list)]);
        assert_eq!(held.repositories.as_ref().map(|all| all[0].root.as_str()), Some("/r/one"));
        let wire = r#"{"message":"event","cursor":2,"event":{"kind":"repositories.changed","repositories":[]}}"#;
        held.fold(vec![Signal::Stream(crate::frames::decode(wire).expect("an event"))]);
        assert_eq!(held.repositories, Some(Vec::new()));
    }

    #[test]
    fn the_projects_are_read_on_connect_and_replaced_whole_when_fleet_says_they_changed() {
        let mut held = Held::new();
        assert_eq!(held.projects, None, "nothing is listed before Fleet has answered");
        let one = r#"{"id":"p1","name":"Storefront","rows":[],"members":[],"place":{"place":"this_mac"}}"#;
        let read: ipc::Projects = ipc::decode("projects", format!(r#"{{"projects":[{one}],"loose":[],"left_out":[]}}"#).as_bytes()).expect("projects");
        held.fold(vec![Signal::Projects(read)]);
        assert_eq!(held.projects.as_ref().map(|all| all.projects[0].name.as_str()), Some("Storefront"));
        let wire = r#"{"message":"event","cursor":2,"event":{"kind":"projects.changed","projects":[],"loose":[],"left_out":[]}}"#;
        held.fold(vec![Signal::Stream(crate::frames::decode(wire).expect("an event"))]);
        assert_eq!(held.projects.as_ref().map(|all| all.projects.len()), Some(0));
    }

    #[test]
    fn a_read_of_calls_carries_no_news() {
        let mut held = Held::new();
        held.fold(vec![Signal::Calls(calls(&[false]))]);
        assert!(held.take_arrived().is_empty());
    }

    #[test]
    fn notifications_are_on_until_the_settings_say_false() {
        let list = |value: &str| -> SettingsList {
            let wire = format!(
                r#"{{"path":"/x","refused":null,"settings":[{{"key":"features.notifications","group":"Features","section":"s","title":"t","description":"d","tip":null,"kind":{{"kind":"boolean"}},"default":true,"value":{value},"saved":null,"applies":"live","pending_restart":false,"overridden_by_env":null}}]}}"#
            );
            ipc::decode("settings", wire.as_bytes()).expect("settings")
        };
        let mut held = Held::new();
        assert!(held.notifications, "on before Fleet has answered, as Electron reads it");
        held.fold(vec![Signal::Settings(list("false"))]);
        assert!(!held.notifications);
        held.fold(vec![Signal::Settings(list("true"))]);
        assert!(held.notifications);
    }

    #[test]
    fn a_command_has_a_port_only_while_fleet_is_connected() {
        let mut held = Held::new();
        assert_eq!(held.port(), None);
        held.fold(vec![Signal::State(LinkState::Connecting { port: 4100, pid: 7 })]);
        assert_eq!(held.port(), None, "connecting is not connected");
        held.fold(vec![Signal::State(LinkState::Connected)]);
        assert_eq!(held.port(), Some(4100));
        held.fold(vec![Signal::State(LinkState::Starting)]);
        assert_eq!(held.port(), None);
    }

    fn thread(items: &[&str]) -> JobThread {
        let rows: Vec<String> = items
            .iter()
            .map(|id| format!(r#"{{"kind":"row","row":{{"kind":"message","id":"{id}","at":"2026-11-09T10:00:00Z","from":{{"kind":"you"}},"text":"{id}"}}}}"#))
            .collect();
        let wire = format!(
            r#"{{"session_id":"job-1","head":{{"trail":{{"started":"2026-11-09T09:00:00Z"}}}},"items":[{}]}}"#,
            rows.join(",")
        );
        ipc::decode("JobThread", wire.as_bytes()).expect("a thread")
    }

    fn tail(from: Option<&str>, items: &[&str]) -> Signal {
        let from = from.map_or(String::new(), |from| format!(r#""from":"{from}","#));
        let rows: Vec<String> = items
            .iter()
            .map(|id| format!(r#"{{"kind":"row","row":{{"kind":"message","id":"{id}","at":"2026-11-09T10:00:00Z","from":{{"kind":"agent"}},"text":"{id}"}}}}"#))
            .collect();
        let wire = format!(
            r#"{{"message":"event","cursor":5,"event":{{"kind":"job_thread.tail","session_id":"job-1",{from}"head":{{"trail":{{"started":"2026-11-09T09:00:00Z"}}}},"items":[{}]}}}}"#,
            rows.join(",")
        );
        Signal::Stream(crate::frames::decode(&wire).expect("a tail"))
    }

    fn ids(held: &Held) -> Vec<String> {
        held.threads["job-1"].items.iter().map(|item| item.id().to_string()).collect()
    }

    #[test]
    fn a_tail_without_from_appends_and_one_with_from_replaces_to_the_end() {
        let mut held = Held::new();
        held.thread_read(thread(&["a", "b", "c"]));
        held.fold(vec![tail(None, &["d"])]);
        assert_eq!(ids(&held), ["a", "b", "c", "d"]);
        held.fold(vec![tail(Some("c"), &["c2", "d2", "e"])]);
        assert_eq!(ids(&held), ["a", "b", "c2", "d2", "e"]);
        assert_eq!(held.threads["job-1"].version, 3, "a read and two tails");
    }

    #[test]
    fn a_tail_from_an_item_not_held_replaces_everything() {
        let mut held = Held::new();
        held.thread_read(thread(&["a", "b"]));
        held.fold(vec![tail(Some("zzz"), &["x", "y"])]);
        assert_eq!(ids(&held), ["x", "y"]);
    }

    #[test]
    fn a_tail_for_a_thread_nobody_read_is_dropped() {
        let mut held = Held::new();
        held.fold(vec![tail(None, &["d"])]);
        assert!(held.threads.is_empty());
    }

    #[test]
    fn a_gap_in_the_stream_marks_every_held_thread_to_be_read_again() {
        let mut held = Held::new();
        held.thread_read(thread(&["a"]));
        assert!(held.stale_threads().is_empty());
        held.fold(vec![Signal::Stream(StreamMessage::Missed(ipc::Missed { dropped: 2 }))]);
        assert_eq!(held.stale_threads().into_iter().collect::<Vec<_>>(), ["job-1"]);
        held.thread_read(thread(&["a", "b"]));
        assert!(held.stale_threads().is_empty(), "a whole read clears it");
    }
}
