//! The In this build sheet: when the owner loads a preview that merges branches several Jobs offered, one note
//! for each Job says what to look at, with steps that take him there, a box that sends his words to that Job
//! (or starts a new one where it has ended), and Good or Needs work, which marks it looked at and tells the Job.
//!
//! What is worked out here is pure, so it is tested: which note is chosen, whether the sheet opens for a build,
//! what a step draws, what Send reads. `view.rs` draws the sheet and holds nothing of its own.

mod view;

use std::collections::HashMap;
use std::path::PathBuf;

use gpui::{App, AppContext, Context, Entity, FocusHandle, Focusable, Global, SharedString, Subscription, Task, Window};
use gpui_component::input::{InputEvent, TextareaState};
use ipc::{BuildNote, BuildNoteFeedbackSent, BuildNotes, NoteVerdict, PreviewStep};
use serde::de::DeserializeOwned;

use crate::fleet::FleetConnection;
use crate::keymap::{self, Act, Slot};
use crate::log::Note;
use crate::look::{token, Token};

/// What a note's box says while empty.
const PLACEHOLDER: &str = "Feedback for this Job";

/// Where a note is kept: Fleet keeps looked at for this (branch, tip), and so does the draft under it.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct NoteKey {
    pub branch: String,
    pub tip: String,
}

impl NoteKey {
    pub fn of(note: &BuildNote) -> NoteKey {
        NoteKey { branch: note.branch.clone(), tip: note.tip.clone() }
    }

    /// The `keep` key of the note's feedback box, so a draft survives closing the sheet and a restart.
    pub fn draft(&self) -> String {
        format!("build-note:{}@{}", self.branch, self.tip)
    }
}

/// The note the keys are on: the one chosen if Fleet still holds it, else the first not looked at, else the first.
pub fn position(notes: &[BuildNote], chosen: Option<&NoteKey>, marked: &Marked) -> Option<usize> {
    chosen
        .and_then(|key| notes.iter().position(|note| NoteKey::of(note) == *key))
        .or_else(|| notes.iter().position(|note| !looked(note, marked)))
        .or(if notes.is_empty() { None } else { Some(0) })
}

/// The note `by` places from `from`, stopping at either end.
pub fn moved(len: usize, from: usize, by: isize) -> usize {
    if len == 0 {
        return 0;
    }
    (from as isize + by).clamp(0, len as isize - 1) as usize
}

/// The verdicts the owner just gave that Fleet has not yet answered for.
pub type Marked = HashMap<NoteKey, NoteVerdict>;

/// Whether a note is looked at: Fleet says so, or the owner just marked it and Fleet has not answered yet.
pub fn looked(note: &BuildNote, marked: &Marked) -> bool {
    note.looked_at || marked.contains_key(&NoteKey::of(note))
}

/// What the owner made of a note: the verdict he just gave, else the one Fleet keeps.
pub fn verdict(note: &BuildNote, marked: &Marked) -> Option<NoteVerdict> {
    marked.get(&NoteKey::of(note)).copied().or(note.verdict)
}

/// Whether the sheet opens by itself: once for each build, and only while some note is not looked at.
/// `last` is the commit it opened for before.
pub fn opens_for(commit: Option<&str>, notes: &[BuildNote], last: Option<&str>) -> bool {
    let Some(commit) = commit.filter(|commit| !commit.is_empty()) else { return false };
    last != Some(commit) && notes.iter().any(|note| !note.looked_at)
}

/// A commit as the header names it.
pub fn short(commit: &str) -> &str {
    commit.get(..7).unwrap_or(commit)
}

/// The act a step's Go runs, and the key that act is drawn with.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Go {
    pub act: &'static str,
    /// The act's key as a keycap reads. Empty where the registry binds it to nothing.
    pub keycap: String,
    /// For an act bound to a row of digits, the digit Go presses.
    pub digit: Option<u8>,
}

/// One step as the sheet draws it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StepLine {
    /// From 1, as the number on its disc and its key.
    pub number: usize,
    pub say: String,
    /// Absent for a step with no act, and for an act the registry does not know: the step has no key and no Go.
    pub go: Option<Go>,
}

/// The steps of a note, each with its Go where its act is in the registry.
pub fn step_lines(steps: &[PreviewStep]) -> Vec<StepLine> {
    steps
        .iter()
        .enumerate()
        .map(|(at, step)| StepLine {
            number: at + 1,
            say: step.say.clone(),
            go: step.act.as_deref().and_then(crate::generated::actions::act).map(|row| Go {
                act: row.id,
                keycap: keymap::shown(row.id),
                digit: match keymap::parse_slots(row.shortcut).into_iter().next() {
                    Some(Slot::Digits { from, .. }) => Some(from),
                    _ => None,
                },
            }),
        })
        .collect()
}

/// The words on the send button: where the note's Job has ended, the words start a new Job.
pub fn send_label(job_ended: bool) -> &'static str {
    if job_ended {
        "Start a Job"
    } else {
        "Send to Job"
    }
}

/// The one line under Start a Job.
pub const STARTS_WITH: &str = "The new Job starts with your feedback, the ended Job's title, its note and its branch.";

/// What the toast says after a send.
pub fn sent_said(title: &str, sent: &BuildNoteFeedbackSent) -> String {
    if sent.started {
        "Started a Job".to_string()
    } else {
        format!("Sent to {title}")
    }
}

/// What a note's row says about it: its glyph, hue and tooltip.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Mark {
    pub glyph: &'static str,
    pub hue: Token,
    pub said: &'static str,
}

/// A note marked before he could give a verdict reads as looked at.
pub fn mark(looked: bool, verdict: Option<NoteVerdict>) -> Mark {
    match (looked, verdict) {
        (_, Some(NoteVerdict::Good)) => Mark { glyph: "circle-check", hue: token::STATUS_COMPLETED_SUCCESS, said: "Good" },
        (_, Some(NoteVerdict::NeedsWork)) => Mark { glyph: "circle-alert", hue: token::STATUS_ESCALATED, said: "Needs work" },
        (true, None) => Mark { glyph: "circle-check", hue: token::STATUS_COMPLETED_SUCCESS, said: "Looked at" },
        (false, None) => Mark { glyph: "circle-dot", hue: token::STATUS_AWAITING_REVIEW, said: "Not looked at" },
    }
}

// -- the commit it last opened for -----------------------------------------------------------------------

/// The build the sheet last opened for on its own, kept in a file so a relaunch on the same build does not open it
/// again. A walk, a mock and a test keep it in memory only.
#[derive(Default)]
struct OpenedFor {
    commit: Option<String>,
    file: Option<PathBuf>,
}

impl Global for OpenedFor {}

/// The file the live app keeps it in, beside the deck's.
pub fn opened_file() -> Option<PathBuf> {
    crate::jobs::deck_file().map(|deck| deck.with_file_name("native-build-notes-opened"))
}

/// Keep it in `file` from here on and take back what it holds. Called once at the live app's start.
pub fn persist(file: Option<PathBuf>, cx: &mut App) {
    let Some(path) = file else { return };
    let commit = std::fs::read_to_string(&path).ok().map(|text| text.trim().to_string()).filter(|commit| !commit.is_empty());
    cx.set_global(OpenedFor { commit, file: Some(path) });
}

fn last_opened(cx: &App) -> Option<String> {
    cx.try_global::<OpenedFor>().and_then(|kept| kept.commit.clone())
}

fn remember_opened(commit: &str, cx: &mut App) {
    let kept = cx.default_global::<OpenedFor>();
    kept.commit = Some(commit.to_string());
    if let Some(path) = kept.file.clone() {
        let text = commit.to_string();
        cx.background_spawn(async move {
            if let Err(why) = std::fs::write(&path, text) {
                Note::warn("the build the notes sheet opened for was not kept").field("why", why.to_string()).emit();
            }
        })
        .detach();
    }
}

// -- the sheet ---------------------------------------------------------------------------------------------

pub struct BuildNotesSheet {
    fleet: Option<Entity<FleetConnection>>,
    held: Option<BuildNotes>,
    open: bool,
    chosen: Option<NoteKey>,
    /// Verdicts the owner gave that Fleet has not yet answered for.
    marked: Marked,
    feedback: Entity<TextareaState>,
    frame: FocusHandle,
    list: FocusHandle,
    send_focus: FocusHandle,
    good_focus: FocusHandle,
    needs_work_focus: FocusHandle,
    sending: bool,
    refused: Option<SharedString>,
    keys_were: Option<FocusHandle>,
    intercept: Option<Subscription>,
    work: Option<Task<()>>,
    _subs: Vec<Subscription>,
}

impl BuildNotesSheet {
    /// On a live Fleet the sheet sends to it; with none (a walk) a press changes nothing but the sheet.
    pub fn new(fleet: Option<Entity<FleetConnection>>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let feedback = cx.new(|cx| TextareaState::new(window, cx).auto_grow(3, 6).placeholder(PLACEHOLDER));
        let typed = cx.subscribe_in(&feedback, window, |this, state, event: &InputEvent, window, cx| match event {
            InputEvent::Change => {
                if let Some(key) = &this.chosen {
                    crate::drafts::keep(&key.draft(), state.read(cx).value().as_ref(), cx);
                }
                this.refused = None;
                cx.notify();
            }
            // ⌘↵ in the box, which takes the key before the sheet's action can.
            InputEvent::PressEnter { secondary: true, .. } => this.send(window, cx),
            _ => {}
        });
        BuildNotesSheet {
            fleet,
            held: None,
            open: false,
            chosen: None,
            marked: Marked::new(),
            feedback,
            frame: cx.focus_handle(),
            list: cx.focus_handle().tab_stop(true),
            send_focus: cx.focus_handle().tab_stop(true),
            good_focus: cx.focus_handle().tab_stop(true),
            needs_work_focus: cx.focus_handle().tab_stop(true),
            sending: false,
            refused: None,
            keys_were: None,
            intercept: None,
            work: None,
            _subs: vec![typed],
        }
    }

    fn notes(&self) -> &[BuildNote] {
        self.held.as_ref().map_or(&[], |held| held.notes.as_slice())
    }

    /// Whether the build holds any note, which is whether the switcher's row opens anything.
    pub fn has_notes(&self) -> bool {
        !self.notes().is_empty()
    }

    fn at(&self) -> Option<usize> {
        position(self.notes(), self.chosen.as_ref(), &self.marked)
    }

    fn note(&self) -> Option<&BuildNote> {
        self.at().and_then(|at| self.notes().get(at))
    }

    /// The notes of the running build, as Fleet last said them. The sheet opens on its own the first time a build
    /// it has not opened for holds a note not looked at.
    pub fn offer(&mut self, notes: BuildNotes, window: &mut Window, cx: &mut Context<Self>) {
        let commit = notes.commit.clone();
        let opens = opens_for(commit.as_deref(), &notes.notes, last_opened(cx).as_deref());
        self.hold(notes, window, cx);
        if opens {
            if let Some(commit) = commit {
                remember_opened(&commit, cx);
            }
            self.show(window, cx);
        }
    }

    fn hold(&mut self, notes: BuildNotes, window: &mut Window, cx: &mut Context<Self>) {
        self.marked.retain(|key, given| notes.notes.iter().any(|note| NoteKey::of(note) == *key && note.verdict != Some(*given)));
        self.held = Some(notes);
        if self.open && !self.has_notes() {
            self.close(window, cx);
            return;
        }
        // The chosen note went from the build, or none was chosen: the keys land on the first to look at.
        let at = self.at();
        let now = at.and_then(|at| self.notes().get(at)).map(NoteKey::of);
        if now != self.chosen {
            self.choose(now, window, cx);
        }
        cx.notify();
    }

    /// Choose a note and show its draft in the box.
    fn choose(&mut self, key: Option<NoteKey>, window: &mut Window, cx: &mut Context<Self>) {
        // The words in the box stay with the note being left, however they got there.
        if let Some(left) = self.chosen.as_ref().filter(|left| Some(*left) != key.as_ref()) {
            crate::drafts::keep(&left.draft(), self.feedback.read(cx).value().as_ref(), cx);
        }
        self.chosen = key;
        self.refused = None;
        let words = self.chosen.as_ref().and_then(|key| crate::drafts::kept(&key.draft(), cx)).unwrap_or_default();
        self.feedback.update(cx, |state, cx| state.set_value(words, window, cx));
        cx.notify();
    }

    fn select(&mut self, at: usize, window: &mut Window, cx: &mut Context<Self>) {
        let key = self.notes().get(at).map(NoteKey::of);
        if key != self.chosen {
            self.choose(key, window, cx);
        }
    }

    /// Open on the note to look at first, the keys in the list of notes.
    pub fn show(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.open || !self.has_notes() {
            return;
        }
        self.keys_were = window.focused(cx);
        self.open = true;
        let key = self.at().and_then(|at| self.notes().get(at)).map(NoteKey::of);
        self.choose(key, window, cx);
        let me = cx.entity().downgrade();
        self.intercept = Some(cx.intercept_keystrokes(move |event, window, cx| {
            let Some(me) = me.upgrade() else { return };
            me.update(cx, |this, cx| {
                if this.open && this.frame.contains_focused(window, cx) {
                    this.intercepted(&event.keystroke, window, cx);
                }
            });
        }));
        let list = self.list.clone();
        window.defer(cx, move |window, cx| window.focus(&list, cx));
        cx.notify();
    }

    /// Open on the note of this (branch, tip), as a card that names an offer asks. A note the build does not hold
    /// opens on the first to look at instead.
    pub fn show_note(&mut self, key: NoteKey, window: &mut Window, cx: &mut Context<Self>) {
        let held = self.notes().iter().any(|note| NoteKey::of(note) == key);
        if held {
            if self.open {
                self.choose(Some(key), window, cx);
                return;
            }
            self.chosen = Some(key);
        }
        self.show(window, cx);
    }

    /// Esc, the scrim or a Go: the sheet goes and the keys go back where they were.
    pub fn close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.open {
            return;
        }
        self.open = false;
        self.intercept = None;
        if let Some(keys) = self.keys_were.take() {
            window.focus(&keys, cx);
        }
        cx.notify();
    }

    /// What the sheet takes before the box can: Esc, and Tab round its own stops.
    fn intercepted(&mut self, key: &gpui::Keystroke, window: &mut Window, cx: &mut Context<Self>) {
        let plain = !key.modifiers.platform && !key.modifiers.control && !key.modifiers.alt;
        match key.key.as_str() {
            "escape" => self.close(window, cx),
            "tab" if plain => {
                let stops = [self.list.clone(), self.feedback.focus_handle(cx), self.send_focus.clone(), self.good_focus.clone(), self.needs_work_focus.clone()];
                let here = stops.iter().position(|stop| stop.contains_focused(window, cx));
                let len = stops.len();
                let next = match (here, key.modifiers.shift) {
                    (Some(at), false) => (at + 1) % len,
                    (Some(at), true) => (at + len - 1) % len,
                    (None, _) => 0,
                };
                window.focus(&stops[next], cx);
            }
            _ => return,
        }
        cx.stop_propagation();
        cx.notify();
    }

    /// The keys: the acts the registry binds while the sheet is up.
    pub(super) fn on_act(&mut self, key: &Act, window: &mut Window, cx: &mut Context<Self>) {
        match key.id {
            "close" => self.close(window, cx),
            // The registry's slots are j, k, ↓, ↑: even slots go down.
            "move_focus" => {
                let from = self.at().unwrap_or(0);
                let to = moved(self.notes().len(), from, if key.slot % 2 == 0 { 1 } else { -1 });
                self.select(to, window, cx);
            }
            "build_note_step" => {
                if let Some(digit) = key.digit {
                    self.go(digit as usize, window, cx);
                }
            }
            "build_note_looked" => self.judge(NoteVerdict::Good, window, cx),
            "build_note_needs_work" => self.judge(NoteVerdict::NeedsWork, window, cx),
            "send_message" => self.send(window, cx),
            // The sheet is a dialog: the keys that start or open something else do nothing while it is up.
            "new_job" | "new_terminal_job" | "dispatch_after" => {}
            _ => cx.propagate(),
        }
    }

    /// Go: the step's act runs on the app, so the sheet goes first and the keys are back where they were.
    pub(super) fn go(&mut self, number: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(go) = self.note().and_then(|note| step_lines(&note.steps).into_iter().find(|line| line.number == number)).and_then(|line| line.go) else {
            return;
        };
        self.close(window, cx);
        window.dispatch_action(Box::new(Act { id: go.act, slot: 0, digit: go.digit }), cx);
    }

    fn port(&self, cx: &App) -> Option<u16> {
        self.fleet.as_ref().and_then(|fleet| fleet.read(cx).held().port())
    }

    /// Good or Needs work: the note reads so in this frame, and Fleet keeps it for this branch at this tip and tells
    /// the Job. Needs work puts the keys in the box, since his words on it are what the Job waits for.
    pub(super) fn judge(&mut self, given: NoteVerdict, window: &mut Window, cx: &mut Context<Self>) {
        let Some(note) = self.note().cloned() else { return };
        if given == NoteVerdict::NeedsWork {
            self.feedback.update(cx, |state, cx| state.focus(window, cx));
        }
        if verdict(&note, &self.marked) == Some(given) {
            return;
        }
        let key = NoteKey::of(&note);
        let Some(port) = self.port(cx) else {
            Note::warn("not sent: no Fleet is connected").field("route", MARK_LOOKED).emit();
            return;
        };
        self.marked.insert(key.clone(), given);
        cx.notify();
        let body = encoded(&ipc::MarkBuildNoteLookedAt { branch: note.branch, tip: note.tip, verdict: Some(given) });
        let task: Task<Result<BuildNotes, String>> = post(port, MARK_LOOKED, body, "BuildNotes", cx);
        cx.spawn_in(window, async move |this, cx| {
            let answer = task.await;
            let _ = this.update_in(cx, |this, window, cx| match answer {
                Ok(notes) => this.hold(notes, window, cx),
                Err(why) => {
                    if this.marked.get(&key) == Some(&given) {
                        this.marked.remove(&key);
                    }
                    Note::warn("Fleet refused the mark").field("route", MARK_LOOKED).field("why", why.as_str()).emit();
                    crate::toasts::tell(why, cx);
                    cx.notify();
                }
            });
        })
        .detach();
    }

    /// Send to Job, or Start a Job where it has ended. A refusal keeps the words and says why.
    pub(super) fn send(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let words = self.feedback.read(cx).value().to_string();
        let Some(note) = self.note().cloned() else { return };
        if self.sending || words.trim().is_empty() {
            return;
        }
        let Some(port) = self.port(cx) else {
            self.refused = Some("Fleet is not connected".into());
            cx.notify();
            return;
        };
        self.sending = true;
        self.refused = None;
        cx.notify();
        let key = NoteKey::of(&note);
        let body = encoded(&ipc::SendBuildNoteFeedback { branch: note.branch.clone(), tip: note.tip.clone(), words });
        let task: Task<Result<BuildNoteFeedbackSent, String>> = post(port, SEND_FEEDBACK, body, "BuildNoteFeedbackSent", cx);
        self.work = Some(cx.spawn_in(window, async move |this, cx| {
            let answer = task.await;
            let _ = this.update_in(cx, |this, window, cx| {
                this.sending = false;
                match answer {
                    Ok(sent) => {
                        crate::drafts::clear(&key.draft(), cx);
                        if this.chosen.as_ref() == Some(&key) {
                            this.feedback.update(cx, |state, cx| state.set_value("", window, cx));
                        }
                        crate::toasts::job_started(sent.session_id.as_str(), sent_said(&note.title, &sent), cx);
                    }
                    Err(why) => {
                        Note::warn("Fleet refused the feedback").field("route", SEND_FEEDBACK).field("why", why.as_str()).emit();
                        if this.chosen.as_ref() == Some(&key) {
                            this.refused = Some(why.into());
                        }
                    }
                }
                cx.notify();
            });
        }));
    }
}

impl Focusable for BuildNotesSheet {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.list.clone()
    }
}

const MARK_LOOKED: &str = "/fleet/build/notes/looked_at";
const SEND_FEEDBACK: &str = "/fleet/build/notes/feedback";

fn encoded(value: &impl serde::Serialize) -> String {
    ipc::encode(value).unwrap_or_default()
}

/// `POST path` off the main thread: the answer as `what` reads it, or Fleet's own sentence for a refusal.
fn post<T: DeserializeOwned + Send + 'static>(port: u16, path: &'static str, body: String, what: &'static str, cx: &App) -> Task<Result<T, String>> {
    cx.background_spawn(async move {
        let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().map_err(|why| why.to_string())?;
        let (status, bytes) = runtime.block_on(crate::transport::post(port, path, body))?;
        if !(200..300).contains(&status) {
            return Err(match ipc::decode::<ipc::WireError>("WireError", &bytes) {
                Ok(error) => error.message,
                Err(_) => format!("Fleet answered {status}"),
            });
        }
        ipc::decode::<T>(what, &bytes).map_err(|why| why.to_string())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn note(branch: &str, looked_at: bool) -> BuildNote {
        ipc::decode::<BuildNote>(
            "BuildNote",
            format!(r#"{{"branch":"{branch}","tip":"t-{branch}","session_id":"s-{branch}","title":"Title {branch}","about":"About","job_ended":false,"looked_at":{looked_at}}}"#).as_bytes(),
        )
        .expect("a note")
    }

    fn key(branch: &str) -> NoteKey {
        NoteKey { branch: branch.into(), tip: format!("t-{branch}") }
    }

    #[test]
    fn the_keys_land_on_the_first_note_not_looked_at_and_on_the_chosen_one_while_it_stands() {
        let notes = vec![note("a", true), note("b", false), note("c", false)];
        let none = Marked::new();
        assert_eq!(position(&notes, None, &none), Some(1));
        assert_eq!(position(&notes, Some(&key("c")), &none), Some(2));
        assert_eq!(position(&notes, Some(&key("gone")), &none), Some(1));
        assert_eq!(position(&[note("a", true)], None, &none), Some(0), "all looked at: the first");
        assert_eq!(position(&[], None, &none), None);
        let marked: Marked = [(key("b"), NoteVerdict::Good)].into();
        assert_eq!(position(&notes, None, &marked), Some(2), "a note just marked is looked at already");
    }

    #[test]
    fn an_arrow_stops_at_either_end_of_the_notes() {
        assert_eq!(moved(3, 0, -1), 0);
        assert_eq!(moved(3, 0, 1), 1);
        assert_eq!(moved(3, 2, 1), 2);
        assert_eq!(moved(0, 0, 1), 0);
    }

    #[test]
    fn the_sheet_opens_once_for_a_build_and_only_while_a_note_is_not_looked_at() {
        let some = vec![note("a", true), note("b", false)];
        assert!(opens_for(Some("abc1234"), &some, None));
        assert!(opens_for(Some("abc1234"), &some, Some("older")));
        assert!(!opens_for(Some("abc1234"), &some, Some("abc1234")), "a relaunch on the same build");
        assert!(!opens_for(Some("abc1234"), &[note("a", true)], None), "everything looked at");
        assert!(!opens_for(Some("abc1234"), &[], None));
        assert!(!opens_for(None, &some, None), "off the preview there is no build commit");
        assert!(!opens_for(Some(""), &some, None));
    }

    #[test]
    fn a_step_with_a_known_act_draws_its_key_and_go_and_an_unknown_act_draws_neither() {
        let steps = vec![
            PreviewStep { say: "Press Projects".into(), act: Some("new_job".into()) },
            PreviewStep { say: "Look at the toast pile".into(), act: None },
            PreviewStep { say: "Press a key nobody has".into(), act: Some("no_such_act_anywhere".into()) },
        ];
        let lines = step_lines(&steps);
        let go = lines[0].go.as_ref().expect("a known act has Go");
        assert_eq!(go.act, "new_job");
        assert_eq!(go.keycap, keymap::shown("new_job"));
        assert!(!go.keycap.is_empty());
        assert_eq!(lines.iter().map(|line| line.number).collect::<Vec<_>>(), [1, 2, 3]);
        assert_eq!(lines[1].go, None);
        assert_eq!(lines[2].go, None, "the registry lacks it: no key and no Go");
        assert_eq!(lines[2].say, "Press a key nobody has");
    }

    #[test]
    fn go_for_an_act_on_a_row_of_digits_presses_the_first_digit() {
        let call = step_lines(&[PreviewStep { say: "Pick".into(), act: Some("call_pick".into()) }]);
        assert_eq!(call[0].go.as_ref().and_then(|go| go.digit), Some(1));
    }

    #[test]
    fn send_reads_start_a_job_where_the_note_s_job_has_ended() {
        assert_eq!(send_label(false), "Send to Job");
        assert_eq!(send_label(true), "Start a Job");
    }

    #[test]
    fn the_toast_names_the_job_sent_to_or_says_a_job_started() {
        let sent = |started| BuildNoteFeedbackSent { session_id: ipc::decode::<ipc::SessionId>("SessionId", br#""s-1""#).expect("id"), started };
        assert_eq!(sent_said("Quiet the clash notices", &sent(false)), "Sent to Quiet the clash notices");
        assert_eq!(sent_said("Quiet the clash notices", &sent(true)), "Started a Job");
    }

    #[test]
    fn a_note_is_marked_by_a_registry_glyph_with_its_state_in_the_tooltip() {
        let said = |looked, given| {
            let one = mark(looked, given);
            (one.glyph, one.said)
        };
        assert_eq!(said(false, None), ("circle-dot", "Not looked at"));
        assert_eq!(said(true, Some(NoteVerdict::Good)), ("circle-check", "Good"));
        assert_eq!(said(true, Some(NoteVerdict::NeedsWork)), ("circle-alert", "Needs work"));
        assert_eq!(said(true, None), ("circle-check", "Looked at"), "marked before a verdict could be given");
        assert_ne!(mark(false, None).hue, mark(true, Some(NoteVerdict::Good)).hue);
        assert_ne!(mark(true, Some(NoteVerdict::Good)).hue, mark(true, Some(NoteVerdict::NeedsWork)).hue);
    }

    #[test]
    fn a_verdict_just_given_stands_over_the_one_fleet_keeps_until_fleet_answers() {
        let mut kept = note("a", true);
        kept.verdict = Some(NoteVerdict::Good);
        assert_eq!(verdict(&kept, &Marked::new()), Some(NoteVerdict::Good));
        let given: Marked = [(key("a"), NoteVerdict::NeedsWork)].into();
        assert_eq!(verdict(&kept, &given), Some(NoteVerdict::NeedsWork));
        assert!(looked(&note("a", false), &given), "a note just judged is looked at");
    }

    #[test]
    fn a_commit_is_named_by_its_first_seven_characters() {
        assert_eq!(short("0123456789abcdef"), "0123456");
        assert_eq!(short("abc"), "abc");
    }

    #[test]
    fn a_draft_is_kept_under_the_note_s_branch_and_tip() {
        assert_eq!(key("a").draft(), "build-note:a@t-a");
        assert_ne!(key("a").draft(), key("b").draft());
    }

    fn sheet(cx: &mut gpui::TestAppContext) -> (Entity<BuildNotesSheet>, &mut gpui::VisualTestContext) {
        cx.update(|cx| {
            gpui_component::init(cx);
            crate::look::init(cx);
            crate::keymap::install(&crate::keymap::Rebinding::default(), cx);
        });
        cx.add_window_view(|window, cx| BuildNotesSheet::new(None, window, cx))
    }

    fn notes(commit: &str) -> BuildNotes {
        let one = |branch: &str, looked_at: bool| {
            format!(
                r#"{{"branch":"{branch}","tip":"t-{branch}","session_id":"s-{branch}","title":"Title {branch}","about":"About","steps":[{{"say":"Press New Job","act":"new_job"}},{{"say":"Press a key nobody has","act":"nothing_here"}}],"job_ended":false,"looked_at":{looked_at}}}"#
            )
        };
        ipc::decode::<BuildNotes>("BuildNotes", format!(r#"{{"commit":"{commit}","notes":[{},{},{}]}}"#, one("a", true), one("b", false), one("c", false)).as_bytes()).expect("notes")
    }

    fn pressed(id: &'static str, slot: u8, digit: Option<u8>) -> Act {
        Act { id, slot, digit }
    }

    #[gpui::test]
    fn the_sheet_opens_on_the_first_note_not_looked_at_and_the_arrows_carry_each_notes_draft(cx: &mut gpui::TestAppContext) {
        let (sheet, cx) = sheet(cx);
        sheet.update_in(cx, |sheet, window, cx| sheet.offer(notes("abc1234def"), window, cx));
        sheet.update_in(cx, |sheet, window, cx| {
            assert!(sheet.open, "a build it has not opened for, with notes not looked at");
            assert_eq!(sheet.at(), Some(1), "the first note not looked at, not the first");
            sheet.feedback.update(cx, |state, cx| state.set_value("on b", window, cx));
        });
        cx.run_until_parked();
        sheet.update_in(cx, |sheet, window, cx| {
            sheet.on_act(&pressed("move_focus", 0, None), window, cx);
            assert_eq!(sheet.at(), Some(2));
            assert_eq!(sheet.feedback.read(cx).value().as_ref(), "", "c has no draft");
            sheet.on_act(&pressed("move_focus", 0, None), window, cx);
            assert_eq!(sheet.at(), Some(2), "it stops at the last note");
            sheet.on_act(&pressed("move_focus", 1, None), window, cx);
            assert_eq!(sheet.at(), Some(1));
            assert_eq!(sheet.feedback.read(cx).value().as_ref(), "on b", "b's draft came back");
        });
    }

    #[gpui::test]
    fn a_card_naming_an_offer_opens_the_sheet_on_that_note_and_one_it_lacks_on_the_first_to_look_at(cx: &mut gpui::TestAppContext) {
        let (sheet, cx) = sheet(cx);
        sheet.update_in(cx, |sheet, window, cx| {
            sheet.hold(notes("abc1234def"), window, cx);
            sheet.show_note(key("c"), window, cx);
            assert!(sheet.open);
            assert_eq!(sheet.at(), Some(2), "the named note, not the first to look at");
            sheet.show_note(key("a"), window, cx);
            assert_eq!(sheet.at(), Some(0), "an open sheet moves to the named note");
            sheet.close(window, cx);
            sheet.chosen = None;
            sheet.show_note(NoteKey { branch: "gone".into(), tip: "t".into() }, window, cx);
            assert_eq!(sheet.at(), Some(1), "a note the build lacks opens on the first to look at");
        });
    }

    #[gpui::test]
    fn the_sheet_opens_once_for_a_build_and_again_for_the_next(cx: &mut gpui::TestAppContext) {
        let (sheet, cx) = sheet(cx);
        sheet.update_in(cx, |sheet, window, cx| {
            sheet.offer(notes("build-one"), window, cx);
            assert!(sheet.open);
            sheet.close(window, cx);
            sheet.offer(notes("build-one"), window, cx);
            assert!(!sheet.open, "the same build does not open it again");
            sheet.show(window, cx);
            assert!(sheet.open, "the switcher's row always opens it while notes exist");
            sheet.close(window, cx);
            sheet.offer(notes("build-two"), window, cx);
            assert!(sheet.open, "another build opens it");
        });
    }

    #[gpui::test]
    fn go_closes_the_sheet_for_a_known_act_and_does_nothing_for_an_unknown_one(cx: &mut gpui::TestAppContext) {
        let (sheet, cx) = sheet(cx);
        sheet.update_in(cx, |sheet, window, cx| {
            sheet.offer(notes("abc1234def"), window, cx);
            sheet.on_act(&pressed("build_note_step", 0, Some(2)), window, cx);
            assert!(sheet.open, "step 2 names an act the registry lacks: no Go");
            sheet.on_act(&pressed("build_note_step", 0, Some(9)), window, cx);
            assert!(sheet.open, "no ninth step");
            sheet.on_act(&pressed("build_note_step", 0, Some(1)), window, cx);
            assert!(!sheet.open, "Go closes the sheet so the act lands on the app");
        });
    }

    #[gpui::test]
    fn the_sheet_draws_while_open_and_nothing_while_shut(cx: &mut gpui::TestAppContext) {
        let (sheet, cx) = sheet(cx);
        cx.run_until_parked();
        sheet.update_in(cx, |sheet, window, cx| sheet.offer(notes("abc1234def"), window, cx));
        cx.run_until_parked();
        sheet.update_in(cx, |sheet, window, cx| sheet.close(window, cx));
        cx.run_until_parked();
    }
}
