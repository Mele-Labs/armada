//! The Fleet connection as an entity: the one owner of what Fleet has said.
//!
//! Views read from this entity and none opens a connection of its own. A task drains the link's
//! channel, folds **a whole batch** into [`Held`] and notifies once; the Electron Bridge's slowness
//! was redraw volume, one per event (#3356).

use std::collections::BTreeMap;

use gpui::{Context, Task};

use ipc::{Cursor, Event, Instant, ModKind, ModSummary, SessionRecord, StreamMessage};

use crate::link::{self, LinkState, Signal};
use crate::log::Note;

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
    /// The stylesheet Fleet checked for a mod, with the `changed_at` of the row it was checked for.
    sheets: BTreeMap<String, (Option<Instant>, String)>,
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
            sheets: BTreeMap::new(),
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
                Signal::Stream(StreamMessage::Resync(resync)) => {
                    resyncs += 1;
                    self.cursor = Some(resync.cursor);
                }
                Signal::Stream(StreamMessage::Missed(lost)) => missed += lost.dropped,
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
    /// Kept, or the drain is cancelled with it.
    _drain: Task<()>,
}

impl FleetConnection {
    /// What Fleet has said, folded. Read it in an `observe` handler; it moves once per batch.
    pub fn held(&self) -> &Held {
        &self.held
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
                    cx.notify();
                    lines
                });
                match folded {
                    Ok(lines) => lines.iter().for_each(Note::emit),
                    Err(_) => return,
                }
            }
        });
        FleetConnection { held: Held::new(), _drain: drain }
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
}
