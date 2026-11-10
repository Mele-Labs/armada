//! The Fleet connection as an entity: the one owner of what Fleet has said.
//!
//! Views read from this entity and none opens a connection of its own. A task drains the link's
//! channel, folds **a whole batch** into [`Held`] and notifies once; the Electron Bridge's slowness
//! was redraw volume, one per event (#3356).

use std::collections::{BTreeMap, BTreeSet};

use gpui::{Context, Task};

use ipc::{
    Cursor, Event, FleetBuildReport, Instant, JobThread, JobThreadTail, ModKind, ModSummary, SessionRecord, StreamMessage,
    ThreadHead, ThreadItem,
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
}

impl HeldThread {
    pub fn from_read(read: JobThread, version: u64) -> Self {
        HeldThread { head: read.head, items: read.items, older: read.older, version, stale: false, dirty_from: Some(0) }
    }

    /// The thread changed from item `at` on: a view measures again from there.
    pub fn mark_dirty(&mut self, at: usize) {
        self.dirty_from = Some(self.dirty_from.map_or(at, |held| held.min(at)));
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
    /// The port of the Fleet last connected to, so a command goes where the events come from.
    port: Option<u16>,
    /// The stylesheet Fleet checked for a mod, with the `changed_at` of the row it was checked for.
    sheets: BTreeMap<String, (Option<Instant>, String)>,
    /// The threads a view has read, by Job id. Fleet sends a tail for each of these and no other.
    pub threads: BTreeMap<String, HeldThread>,
}

impl Held {
    fn new() -> Self {
        Held {
            state: LinkState::NotRunning("not yet read".into()),
            jobs: Vec::new(),
            cursor: None,
            theme: None,
            key_bindings: None,
            mods: None,
            build: None,
            port: None,
            sheets: BTreeMap::new(),
            threads: BTreeMap::new(),
        }
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

    fn all_stale(&mut self) {
        self.threads.values_mut().for_each(|held| held.stale = true);
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
                        self.state = state;
                    }
                }
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
                Signal::Build(report) => self.build = Some(report),
                Signal::BuildFailed(why) => lines.push(Note::warn("the build could not be read").field("why", why)),
                Signal::Stream(StreamMessage::Resync(resync)) => {
                    resyncs += 1;
                    self.cursor = Some(resync.cursor);
                    self.all_stale();
                }
                Signal::Stream(StreamMessage::Missed(lost)) => {
                    missed += lost.dropped;
                    self.all_stale();
                }
                Signal::Stream(StreamMessage::Event(delivered)) => {
                    events += 1;
                    self.cursor = Some(delivered.cursor);
                    *kinds.entry(delivered.event.kind()).or_default() += 1;
                    // A row arrives whole and replaces the one held, never patches it.
                    match delivered.event {
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

pub struct FleetConnection {
    held: Held,
    /// The Jobs whose thread is being read now, so one read is not asked twice.
    reading: BTreeSet<String>,
    /// The Jobs whose last read failed: not asked again until a view asks with `retry`, or the
    /// redraw a failure causes would ask for ever.
    failed: BTreeSet<String>,
    /// Kept, or the drain is cancelled with it.
    _drain: Task<()>,
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

    /// A revert has been asked and Fleet took it: the report says so until the next read, as Electron's
    /// `FleetBuilds.revert` marks it, so the menu's item goes off at once. The old failure is gone.
    pub fn mark_reverting(&mut self, cx: &mut Context<Self>) {
        if let Some(build) = self.held.build.as_mut() {
            build.reverting = true;
            build.failed = None;
            cx.notify();
        }
    }

    /// Read a Job's thread (`GET /sessions/thread`) off the main thread and hold it. Fleet follows the
    /// Job from this first read and sends its changes as `job_thread.tail`. A read already under
    /// way is not asked again; a failed one is noted and the view keeps its skeleton, and is asked
    /// again only with `retry`, which is the person opening the Job.
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
                        return Err(format!("Fleet answered {status}: {}", String::from_utf8_lossy(&bytes)));
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
                        Note::warn("a Job's thread could not be read").field("job", id.as_str()).field("why", why).emit();
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
                    let lines = this.held.fold(batch);
                    // A change that reached too far back, or a gap in the stream: read those again.
                    for id in this.held.stale_threads() {
                        if let Some(held) = this.held.threads.get_mut(&id) {
                            held.stale = false;
                        }
                        this.read_thread(&id, true, cx);
                    }
                    cx.notify();
                    lines
                });
                match folded {
                    Ok(lines) => lines.iter().for_each(Note::emit),
                    Err(_) => return,
                }
            }
        });
        FleetConnection { held: Held::new(), reading: BTreeSet::new(), failed: BTreeSet::new(), _drain: drain }
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
