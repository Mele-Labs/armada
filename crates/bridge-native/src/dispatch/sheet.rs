//! The dispatch sheet: one line to describe the work, a drawer under it for the repository, the
//! model and the act (`DP/DispatchPanel.tsx`, `DP/dispatching.tsx`).
//!
//! It draws a [`Face`] and holds only what a hand has changed: the words, the pick, the
//! sentence's cycles. A press answers in the same frame (the act turns pending, a chip fills) and
//! tells the app through [`Asked`]; the app answers with [`DispatchSheet::answered`] or
//! [`DispatchSheet::refused`], which puts the sheet back with its words kept. This file is its
//! state and keys; `view.rs` draws it.

use std::borrow::Cow;
use std::cell::Cell;
use std::path::PathBuf;
use std::rc::Rc;

use gpui::{
    point, AppContext, Anchor, App, Bounds, ClipboardItem, Context, DismissEvent, Entity, EventEmitter, FocusHandle, Focusable, KeyDownEvent, Pixels,
    SharedString, Subscription, Window,
};
use gpui_component::input::{InputEvent, Position, TextareaState};
use ipc::SessionTag;

use super::face::{Face, Mark, Offered, Part, Reading, Row, Trigger, Wait};
use super::story::{self, FeatureChoice, Keyed, Kind, Story};
use crate::dropdown::{Dropdown, Entry, Picked};
use crate::job_view::composer::{attached, clipped_keyed, pick_files, read_files, Attached};
use crate::job_view::offer::Pick;
use crate::job_view::sketch_pad::{PadEvent, SketchPad};
use crate::job_view::talk::{self, Hears, Talk};
use crate::sketch::DrawnSketch;
use crate::voice::{Engine, Source};
use crate::keymap::Act;
use crate::look::{token, ActiveLook};
use crate::titlebar::id_of;

/// Where the request's words are kept between openings, and the model and effort picked.
pub const KEEP_REQUEST: &str = "dispatch:request";
const KEEP_TUNING: &str = "dispatch:session-tuning";

/// What the sheet asks of the app. The wiring sends these to Fleet; a walk answers them by scene.
#[derive(Clone, Debug, PartialEq)]
pub enum Asked {
    /// The sheet shut. The draft stays.
    Closed,
    /// The words rested: which repository do they name.
    ReadRepository(String),
    /// A sentence needs the feature catalog of this project, by Fleet's id. The answer is [`DispatchSheet::set_features`].
    ReadFeatures(String),
    /// A Job is tagged: read the wait the words name.
    ReadWait { job: String, words: String },
    /// `project` is Fleet's id for the project of the row picked; none starts it in no project.
    /// `workflow` is the id of the workflow the sentence's kind starts on, empty for a talk Job; none for plain words.
    Start { words: String, repository: String, project: Option<String>, workflow: Option<String>, model: Option<String>, effort: Option<String>, held: bool },
    Queue { words: String, repository: String, project: Option<String>, model: Option<String>, effort: Option<String> },
    /// New Side Job, from the card under the sheet: what ⌘T starts, in the row the picker named if it did. The sheet has shut.
    JobHere(Option<PickedRow>),
}

impl EventEmitter<Asked> for DispatchSheet {}

/// A row of the picker as a start takes it: the repository, and the project it is for, by Fleet's id.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PickedRow {
    pub repository: String,
    pub project: Option<String>,
}

/// The `@` or `#` being typed, and the rows it offers.
pub(super) struct Menu {
    pub trigger: Trigger,
    /// Where the trigger stands in the words, in bytes.
    pub from: usize,
    pub query: String,
    pub rows: Vec<Row>,
    pub active: usize,
}

/// A tool picked from the +: it goes as a `tool` tag with the first message, and its chip is a stop that removes it.
pub(super) struct HeldTool {
    pub tag: SessionTag,
    pub key: FocusHandle,
}

/// A state of the box that ⌘Z comes back to: the words where no sentence stands, or the sentence, and the tools held.
#[derive(Clone)]
pub(super) struct Shape {
    plain: Option<String>,
    story: Option<Story>,
    tools: Vec<SessionTag>,
    preset: Vec<String>,
}

/// The most steps ⌘Z goes back.
const HISTORY: usize = 200;

/// The popover a trigger in the drawer opens.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Pop {
    Model,
    Effort,
    Repository,
}

pub(super) struct Popup {
    pub dropdown: Entity<Dropdown>,
    _picked: Subscription,
    _dismissed: Subscription,
}

#[derive(Default)]
pub(super) struct Anchors {
    pub model: Rc<Cell<Option<Bounds<Pixels>>>>,
    pub effort: Rc<Cell<Option<Bounds<Pixels>>>>,
    pub repository: Rc<Cell<Option<Bounds<Pixels>>>>,
}

pub struct DispatchSheet {
    pub face: Face,
    pub(super) open: bool,
    pub(super) request: Entity<TextareaState>,
    _typed: Subscription,
    /// Files attached to the request: they go with its first message.
    pub(super) files: Vec<Attached>,
    /// Sketches drawn on the request's own pad, going with its first message as pictures.
    pub(super) drawn: Vec<DrawnSketch>,
    next_key: u64,
    pub(super) pad: Entity<SketchPad>,
    _pad: Subscription,
    pub(super) talk: Talk,
    /// The voice Talk hears with, as the Job's box has it, and whether Fleet is live or the walks' mock is.
    voice: Engine,
    live: bool,
    /// The act is out: Start is pending.
    pub(super) sending: bool,
    /// The act out is Draft: the button reads Drafting, and a start waiting on the repository stays a draft.
    pub(super) drafting: bool,
    pub(super) queueing: bool,
    /// A Job was started with no repository named; the row takes the keys until one is picked.
    pub(super) asking: bool,
    /// The act is waiting for the repository read to answer.
    waiting: bool,
    pub(super) menu: Option<Menu>,
    /// The sentence being written, once an opening is taken. The request's own box is put away while it stands.
    pub(super) story: Option<Story>,
    /// The opening marked in the list under the caret, among the ones the typed words begin.
    pub(super) opening: usize,
    /// The feature catalog of one project, by Fleet's id, as last answered.
    catalog: Option<(String, Vec<FeatureChoice>)>,
    /// The project whose catalog was asked for and has not answered.
    features_asked: Option<String>,
    /// The tools the sentence's kind put in, which go again where it turns to plain words.
    preset: Vec<String>,
    /// The states ⌘Z goes back to, newest last, and the ones it came from, for ⌘Y.
    past: Vec<Shape>,
    future: Vec<Shape>,
    /// The blank a run of letters is going into: the run is one step, so a letter in it adds none.
    run: Option<usize>,
    /// The request's box holds what ⌘Z or Esc put in it and nothing was typed since: its ⌘Z and ⌘Y are the sentence's.
    untouched: bool,
    pub(super) story_key: FocusHandle,
    pub(super) list_scroll: gpui::ScrollHandle,
    /// The tools the + lists are open, with the active row.
    pub(super) tools_open: Option<usize>,
    /// The tools picked, in the order picked, one of each.
    pub(super) tools: Vec<HeldTool>,
    /// The Job menu on the sentence, with its active row.
    pub(super) wait_menu: Option<usize>,
    /// The Job the words tag, by id and title.
    pub(super) tagged: Option<(SharedString, SharedString)>,
    pub(super) wait: Option<Wait>,
    pub(super) model: Option<SharedString>,
    pub(super) effort: Option<SharedString>,
    pub(super) popup: Option<Popup>,
    pub(super) at: Anchors,
    keys_were: Option<FocusHandle>,
    /// The keys the request would take for itself (↑ ↓ ↵) are read before it sees them while the sheet is open.
    intercept: Option<Subscription>,
    pub(super) frame: FocusHandle,
    pub(super) row: FocusHandle,
    pub(super) parts: [FocusHandle; 4],
    /// Where the card under the sheet starts a Job, as the title bar names it: none draws no card.
    pub(super) here: Option<SharedString>,
    pub(super) here_key: FocusHandle,
    pub(super) model_key: FocusHandle,
    pub(super) effort_key: FocusHandle,
    pub(super) queue_key: FocusHandle,
    pub(super) draft_key: FocusHandle,
    pub(super) act_key: FocusHandle,
    pub(super) tools_key: FocusHandle,
    pub(super) attach_key: FocusHandle,
    pub(super) sketch_key: FocusHandle,
    pub(super) talk_key: FocusHandle,
}

impl Focusable for DispatchSheet {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.words_focus(cx)
    }
}

impl DispatchSheet {
    pub fn new(face: Face, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let request = cx.new(|cx| TextareaState::new(window, cx).auto_grow(5, 5).placeholder(PLACEHOLDER));
        let typed = cx.subscribe_in(&request, window, |this, request, event: &InputEvent, window, cx| match event {
            InputEvent::Change => this.typed(request, window, cx),
            _ => {}
        });
        let pad = cx.new(SketchPad::new);
        let told = cx.subscribe_in(&pad, window, |this, _, event: &PadEvent, window, cx| {
            match event {
                PadEvent::Attached(drawn) => this.drawn.push(drawn.clone()),
                PadEvent::Closed => {
                    let focus = this.words_focus(cx);
                    window.focus(&focus, cx);
                }
            }
            cx.notify();
        });
        DispatchSheet {
            wait: face.wait.clone(),
            face,
            open: false,
            request,
            _typed: typed,
            files: Vec::new(),
            drawn: Vec::new(),
            next_key: 0,
            pad,
            _pad: told,
            talk: Talk::default(),
            voice: Engine::Apple,
            live: false,
            sending: false,
            drafting: false,
            queueing: false,
            asking: false,
            waiting: false,
            menu: None,
            story: None,
            opening: 0,
            catalog: None,
            features_asked: None,
            preset: Vec::new(),
            past: Vec::new(),
            future: Vec::new(),
            run: None,
            untouched: false,
            story_key: cx.focus_handle(),
            list_scroll: gpui::ScrollHandle::new(),
            tools_open: None,
            tools: Vec::new(),
            wait_menu: None,
            tagged: None,
            model: None,
            effort: None,
            popup: None,
            at: Anchors::default(),
            keys_were: None,
            intercept: None,
            frame: cx.focus_handle(),
            row: cx.focus_handle(),
            parts: [cx.focus_handle(), cx.focus_handle(), cx.focus_handle(), cx.focus_handle()],
            here: None,
            here_key: cx.focus_handle(),
            model_key: cx.focus_handle(),
            effort_key: cx.focus_handle(),
            queue_key: cx.focus_handle(),
            draft_key: cx.focus_handle(),
            act_key: cx.focus_handle(),
            tools_key: cx.focus_handle(),
            attach_key: cx.focus_handle(),
            sketch_key: cx.focus_handle(),
            talk_key: cx.focus_handle(),
        }
    }

    /// The voice Talk hears with, and whether a live Fleet is behind the sheet; on the walks' mock, Attach adds samples.
    pub fn set_voice(&mut self, voice: Engine, live: bool) {
        self.voice = voice;
        self.live = live;
    }

    /// The project or repository the card under the sheet starts a Job in.
    pub fn set_here(&mut self, here: Option<SharedString>, cx: &mut Context<Self>) {
        self.here = here;
        cx.notify();
    }

    /// The repository the picker names, chosen by the words or by hand. Where the rail is scoped it names the title bar's, not one picked.
    pub(super) fn named_repository(&self) -> Option<String> {
        if self.face.reading == Reading::Scoped {
            return None;
        }
        self.face.chosen().map(|chip| chip.name.to_string())
    }

    /// The row the picker names, as a repository and the project it is for, where the rail is not scoped.
    pub(super) fn named_row(&self) -> Option<PickedRow> {
        if self.face.reading == Reading::Scoped {
            return None;
        }
        self.face.chosen().map(|chip| PickedRow { repository: chip.name.to_string(), project: chip.project.as_ref().map(|one| one.id.to_string()) })
    }

    /// Where the card under the sheet, and ⌘T over it, start a Job: the repository the picker names, else where the title bar is.
    pub(super) fn here_in(&self) -> Option<SharedString> {
        let here = self.here.as_ref()?;
        Some(self.named_repository().map_or_else(|| here.clone(), SharedString::from))
    }

    /// The card under the sheet: the sheet shuts, and the Job ⌘T starts is asked for.
    pub(super) fn job_here(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.here.is_none() {
            return;
        }
        let picked = self.named_row();
        self.close(window, cx);
        cx.emit(Asked::JobHere(picked));
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    // -- opening and closing ----------------------------------------------------------------

    /// Open on the draft as it stands: the words kept, a Job. `add` is typed after them: the `@` that
    /// opens its menu, or the key that began the next draft.
    pub fn show(&mut self, add: &str, window: &mut Window, cx: &mut Context<Self>) {
        if self.open {
            return;
        }
        self.keys_were = window.focused(cx);
        self.open = true;
        self.past.clear();
        self.future.clear();
        self.run = None;
        self.untouched = false;
        let me = cx.entity().downgrade();
        self.intercept = Some(cx.intercept_keystrokes(move |event, window, cx| {
            let Some(me) = me.upgrade() else { return };
            me.update(cx, |this, cx| {
                if this.open && this.frame.contains_focused(window, cx) {
                    let down = KeyDownEvent { keystroke: event.keystroke.clone(), is_held: false, prefer_character_input: false };
                    this.on_key(&down, window, cx);
                }
            });
        }));
        self.asking = false;
        self.face.fresh();
        let words = crate::drafts::kept(KEEP_REQUEST, cx).unwrap_or_default();
        let tuning = crate::drafts::kept(KEEP_TUNING, cx).unwrap_or_default();
        let mut parts = tuning.split('\n');
        self.model = parts.next().filter(|one| !one.is_empty()).map(|one| one.to_string().into());
        self.effort = parts.next().filter(|one| !one.is_empty()).map(|one| one.to_string().into());
        let words = format!("{words}{add}");
        self.request.update(cx, |state, cx| {
            state.set_placeholder(PLACEHOLDER, window, cx);
            state.set_value(words.clone(), window, cx);
            end_of(state, &words, window, cx);
        });
        let focus = self.request.focus_handle(cx);
        window.defer(cx, move |window, cx| window.focus(&focus, cx));
        cx.notify();
    }

    /// Esc, or a press on the scrim: the sheet goes and the draft stays, the keys back where they were.
    pub fn close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.open {
            return;
        }
        self.open = false;
        self.intercept = None;
        self.flatten(window, cx);
        self.menu = None;
        self.tools_open = None;
        self.wait_menu = None;
        self.popup = None;
        self.asking = false;
        self.talk.leave();
        if let Some(keys) = self.keys_were.take() {
            window.focus(&keys, cx);
        }
        cx.emit(Asked::Closed);
        cx.notify();
    }

    /// The Job the words tag, as the screen's mock Fleet reads the wait on it.
    pub fn tagged(&self) -> Option<Offered> {
        self.tagged.as_ref().map(|(id, title)| Offered { id: id.clone(), title: title.clone() })
    }

    /// The request's words: the sentence as composed while one is being written, else the box's own.
    pub(super) fn words(&self, cx: &App) -> String {
        match &self.story {
            Some(story) => story.words(),
            None => self.request.read(cx).value().to_string(),
        }
    }

    /// The words a start sends: the sentence with the page of the feature it names, else the box's own.
    fn brief(&self, cx: &App) -> String {
        match &self.story {
            Some(story) => story.brief(&choices(&self.catalog, &self.face, self.project().as_deref())),
            None => self.words(cx),
        }
    }

    pub(super) fn worded(&self, cx: &App) -> bool {
        match &self.story {
            // The tools a kind presets are no words: nothing starts until a blank holds some.
            Some(story) => story.has_words() || !self.files.is_empty() || !self.drawn.is_empty(),
            None => !self.words(cx).trim().is_empty() || self.holds(),
        }
    }

    /// What has the keys when the words do: the sentence while one is written, else the request's box.
    pub(super) fn words_focus(&self, cx: &App) -> FocusHandle {
        if self.story.is_some() {
            self.story_key.clone()
        } else {
            self.request.focus_handle(cx)
        }
    }

    /// The request holds something besides words: a file, a sketch, or a tool.
    pub(super) fn holds(&self) -> bool {
        !self.files.is_empty() || !self.drawn.is_empty() || !self.tools.is_empty()
    }

    /// A Job that waits starts with no first message, so nothing can go with it: Attach and Draw sketch
    /// are put away while a wait is tagged, and paste and drop leave files where they are.
    pub(super) fn takes_files(&self) -> bool {
        self.wait.is_none()
    }

    /// What goes with the request's first message: its files and its sketches.
    pub(super) fn held(&self) -> (Vec<Attached>, Vec<DrawnSketch>) {
        (self.files.clone(), self.drawn.clone())
    }

    /// The tools picked, as the tags the first message carries.
    pub(super) fn tools_held(&self) -> Vec<SessionTag> {
        self.tools.iter().map(|one| one.tag.clone()).collect()
    }

    // -- the words ----------------------------------------------------------------------------

    fn typed(&mut self, request: &Entity<TextareaState>, _window: &mut Window, cx: &mut Context<Self>) {
        let words = request.read(cx).value().to_string();
        crate::drafts::keep(KEEP_REQUEST, &words, cx);
        // Every change clears the refusal and any pending act.
        self.face.refused = None;
        self.asking = false;
        self.sending = false;
        // A start waiting on the repository read stays what was pressed.
        self.drafting = self.drafting && self.waiting;
        let cursor = request.read(cx).cursor().min(words.len());
        self.tools_open = None;
        self.opening = 0;
        self.untouched = false;
        self.menu = self.mention(&words, cursor).filter(|menu| !menu.rows.is_empty());
        self.reread(&words, cx);
        cx.notify();
    }

    // -- the sentence -------------------------------------------------------------------------------

    /// The kinds whose opening the words begin: the ghost and its list stand while there are any.
    pub(super) fn openings(&self, cx: &App) -> Vec<usize> {
        if self.story.is_some() {
            return Vec::new();
        }
        story::openings(&self.words(cx))
    }

    /// An opening taken, by Tab or →, or a press on its row: the box becomes the sentence, with the caret in its first
    /// blank and the kind's tools held.
    pub(super) fn take_opening(&mut self, kind: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(chosen) = super::story_kinds::kinds().get(kind) else { return };
        let before = self.shape_now(cx);
        self.menu = None;
        self.tools_open = None;
        self.preset.clear();
        for id in &chosen.tools {
            let Some(tool) = ipc::ask_tools::ask_tool(id) else { continue };
            if !self.tools.iter().any(|one| one.tag.id == tool.id) {
                let tag = SessionTag { kind: ipc::TagKind::Tool, id: tool.id.to_string(), title: tool.name.to_string(), url: None };
                self.tools.push(HeldTool { tag, key: cx.focus_handle() });
                self.preset.push(tool.id.to_string());
            }
        }
        self.request.update(cx, |state, cx| state.set_value("", window, cx));
        self.story = Some(Story::new(kind));
        self.remember(before);
        self.features_asked = None;
        let focus = self.story_key.clone();
        window.focus(&focus, cx);
        self.story_moved(cx);
        self.want_features(cx);
        cx.notify();
    }

    /// The sentence's words went into the request's box, which has the sentence's place again. The tools its kind put in go.
    fn flatten(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(story) = self.story.take() else { return };
        let words = match story.words() {
            // Nothing typed into a blank: the opening stays, as it was typed.
            words if words.is_empty() => format!("{} ", story.kind().opening),
            words => words,
        };
        let preset = std::mem::take(&mut self.preset);
        self.tools.retain(|one| !preset.contains(&one.tag.id));
        self.request.update(cx, |state, cx| {
            state.set_value(words.clone(), window, cx);
            end_of(state, &words, window, cx);
        });
    }

    /// Esc, or ⌫ in the first blank, or any door that wants the request's box: the sentence is plain words and the keys are in the box.
    pub(super) fn plain_words(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.story.is_none() {
            return;
        }
        let before = self.shape_now(cx);
        self.flatten(window, cx);
        self.remember(before);
        let focus = self.request.focus_handle(cx);
        window.focus(&focus, cx);
        // Setting the value says nothing to the field's watchers: the draft and the reads follow here.
        let request = self.request.clone();
        self.typed(&request, window, cx);
        self.untouched = true;
    }

    // -- undo ----------------------------------------------------------------------------------------

    /// The box as it stands, for ⌘Z to come back to.
    fn shape_now(&self, cx: &App) -> Shape {
        Shape {
            plain: self.story.is_none().then(|| self.request.read(cx).value().to_string()),
            story: self.story.clone(),
            tools: self.tools.iter().map(|one| one.tag.clone()).collect(),
            preset: self.preset.clone(),
        }
    }

    /// A change is made: the state before it is a step, and what ⌘Y had is gone.
    fn remember(&mut self, before: Shape) {
        self.past.push(before);
        if self.past.len() > HISTORY {
            self.past.remove(0);
        }
        self.future.clear();
        self.run = None;
    }

    /// A state brought back: the sentence with its blank and tools, or the words the box held.
    fn restore(&mut self, shape: Shape, window: &mut Window, cx: &mut Context<Self>) {
        self.tools = shape.tools.into_iter().map(|tag| HeldTool { tag, key: cx.focus_handle() }).collect();
        self.preset = shape.preset;
        self.run = None;
        match shape.story {
            Some(story) => {
                self.story = Some(story);
                let focus = self.story_key.clone();
                window.focus(&focus, cx);
                self.story_moved(cx);
                self.want_features(cx);
            }
            None => {
                self.story = None;
                let words = shape.plain.unwrap_or_default();
                self.request.update(cx, |state, cx| {
                    state.set_value(words.clone(), window, cx);
                    end_of(state, &words, window, cx);
                });
                let focus = self.request.focus_handle(cx);
                window.focus(&focus, cx);
                let request = self.request.clone();
                self.typed(&request, window, cx);
                self.untouched = true;
            }
        }
        cx.notify();
    }

    /// ⌘Z: the last step is taken back. False where there is none.
    pub(super) fn undo(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let Some(back) = self.past.pop() else { return false };
        let now = self.shape_now(cx);
        self.future.push(now);
        self.restore(back, window, cx);
        true
    }

    /// ⌘Y or ⇧⌘Z: the step ⌘Z took back is made again. False where there is none.
    pub(super) fn redo(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let Some(again) = self.future.pop() else { return false };
        let now = self.shape_now(cx);
        self.past.push(now);
        self.restore(again, window, cx);
        true
    }

    /// ⌘Z, ⌘Y and ⇧⌘Z while the sentence has the keys, or while the request's box holds what the sentence left in it. True where taken.
    fn history_key(&mut self, key: &str, shift: bool, in_request: bool, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let redo = key == "y" || shift;
        let sentence = self.story.is_some() && self.story_key.is_focused(window);
        let left = in_request && self.untouched && if redo { !self.future.is_empty() } else { !self.past.is_empty() };
        if !(sentence || left) {
            return false;
        }
        // The key is the sentence's whether or not it has a step to go to: no other ⌘Z answers while it is in a blank.
        let _ = if redo { self.redo(window, cx) } else { self.undo(window, cx) };
        true
    }

    /// A blank's words changed: what `typed` does for the box, for the sentence.
    fn story_moved(&mut self, cx: &mut Context<Self>) {
        let words = self.words(cx);
        crate::drafts::keep(KEEP_REQUEST, &words, cx);
        self.face.refused = None;
        self.asking = false;
        self.sending = false;
        self.drafting = self.drafting && self.waiting;
        self.tools_open = None;
        self.reread(&words, cx);
        cx.notify();
    }

    /// What the sentence's feature blanks list: the features of the project the picker names, and where it names none, his projects.
    pub(super) fn features(&self) -> Cow<'_, [FeatureChoice]> {
        let project = self.project();
        choices(&self.catalog, &self.face, project.as_deref())
    }

    /// A project taken from the feature blank: its first row is chosen, as a chip pressed chooses it, and its features are asked for.
    fn pick_project(&mut self, id: &str, cx: &mut Context<Self>) {
        let row = self.face.chips.iter().chain(&self.face.more).find(|chip| chip.project.as_ref().is_some_and(|one| one.id == id)).map(|chip| chip.id());
        if let Some(row) = row {
            self.face.pick(&row);
        }
        self.want_features(cx);
    }

    /// Ask for the catalog of the project the picker names, once, where a sentence stands and it is not held.
    fn want_features(&mut self, cx: &mut Context<Self>) {
        let Some(project) = self.project().filter(|_| self.story.is_some()) else { return };
        if self.catalog.as_ref().is_some_and(|(held, _)| *held == project) || self.features_asked.as_ref() == Some(&project) {
            return;
        }
        self.features_asked = Some(project.clone());
        cx.emit(Asked::ReadFeatures(project));
    }

    /// The app's answer to [`Asked::ReadFeatures`]: the project's features, or none where Fleet could not say.
    pub fn set_features(&mut self, project: String, features: Vec<FeatureChoice>, cx: &mut Context<Self>) {
        if self.features_asked.as_ref() == Some(&project) {
            self.features_asked = None;
        }
        self.catalog = Some((project, features));
        cx.notify();
    }

    /// The keys while a blank has them. True where the sentence took the key.
    fn story_keys(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let before = self.shape_now(cx);
        let project = self.project();
        let catalog = choices(&self.catalog, &self.face, project.as_deref());
        let Some(story) = self.story.as_mut() else { return false };
        let mods = &event.keystroke.modifiers;
        let at = story.at;
        // Letters typed into one blank, and a letter taken back, are one step however many go in.
        let key = event.keystroke.key.as_str();
        let letters = matches!(story.piece(), super::story_kinds::Piece::Blank(_) | super::story_kinds::Piece::Feature(_))
            && !matches!(key, "tab" | "enter")
            && (key == "backspace" || event.keystroke.key_char.is_some());
        let keyed = story.key(event.keystroke.key.as_str(), mods.shift, mods.platform, mods.control || mods.alt, event.keystroke.key_char.as_deref(), &catalog);
        if story.list_open(&catalog) {
            self.list_scroll.scroll_to_item(story.row);
        }
        match keyed {
            Keyed::Ignored => return false,
            Keyed::Taken => {
                // Moving on from a blank ends its run; moving the mark in a list does not.
                if !matches!(key, "up" | "down") {
                    self.run = None;
                }
            }
            Keyed::Changed => {
                if !(letters && self.run == Some(at)) {
                    self.remember(before);
                }
                self.run = letters.then_some(at);
                self.story_moved(cx);
            }
            Keyed::Project(id) => {
                self.remember(before);
                self.run = None;
                self.pick_project(&id, cx);
                self.story_moved(cx);
            }
            Keyed::Start => self.act(window, cx),
            Keyed::Plain | Keyed::Opening => self.plain_words(window, cx),
            Keyed::Leave { back } => {
                self.run = None;
                self.step_focus(back, window, cx)
            }
            Keyed::Paste => {
                let text = cx.read_from_clipboard().and_then(|item| item.text()).unwrap_or_default();
                let line = text.lines().next().unwrap_or_default().trim();
                if !line.is_empty() {
                    self.remember(before);
                    if let Some(story) = self.story.as_mut() {
                        story.typed(line);
                    }
                    self.story_moved(cx);
                }
            }
        }
        cx.notify();
        true
    }

    /// A blank pressed: it has the keys.
    pub(super) fn story_go(&mut self, at: usize, window: &mut Window, cx: &mut Context<Self>) {
        self.run = None;
        if let Some(story) = self.story.as_mut() {
            story.go_to(at);
        }
        let focus = self.story_key.clone();
        window.focus(&focus, cx);
        cx.notify();
    }

    /// A row of the feature list pressed: its feature is in the blank, and the keys go on.
    pub(super) fn story_row(&mut self, row: usize, window: &mut Window, cx: &mut Context<Self>) {
        let before = self.shape_now(cx);
        self.remember(before);
        let project = self.project();
        let catalog = choices(&self.catalog, &self.face, project.as_deref());
        let chose = self.story.as_mut().and_then(|story| story.take_row(row, &catalog));
        drop(catalog);
        if let Some(id) = chose {
            self.pick_project(&id, cx);
        }
        let focus = self.story_key.clone();
        window.focus(&focus, cx);
        self.story_moved(cx);
    }

    /// The choice pressed: the other word stands, and it has the keys.
    pub(super) fn story_choose(&mut self, at: usize, window: &mut Window, cx: &mut Context<Self>) {
        let before = self.shape_now(cx);
        self.remember(before);
        if let Some(story) = self.story.as_mut() {
            story.go_to(at);
            story.choice = 1 - story.choice;
        }
        let focus = self.story_key.clone();
        window.focus(&focus, cx);
        self.story_moved(cx);
    }

    /// The words rested: the repository is read again, and the Job the words tag is looked for.
    fn reread(&mut self, words: &str, cx: &mut Context<Self>) {
        self.tagged = self
            .face
            .mentions
            .jobs
            .iter()
            .filter(|job| words.contains(&format!("@{}", job.title)))
            .max_by_key(|job| job.title.len())
            .map(|job| (job.id.clone(), job.title.clone()));
        if words.trim().is_empty() {
            self.wait = None;
            return;
        }
        // A repository picked by hand, or the one the window is scoped to, stands until the sheet is opened again.
        if !matches!(self.face.reading, Reading::Picked | Reading::Scoped) {
            self.face.reading = Reading::Detecting;
            cx.emit(Asked::ReadRepository(words.to_string()));
        }
        match &self.tagged {
            Some((job, _)) => cx.emit(Asked::ReadWait { job: job.to_string(), words: words.to_string() }),
            None => self.wait = None,
        }
    }

    /// The `@` or `#` being typed before the caret (`DP/SessionMenu.tsx`, `useAtMenu`).
    fn mention(&self, words: &str, cursor: usize) -> Option<Menu> {
        let before = words.get(..cursor)?;
        let mut found = None;
        for (at, ch) in before.char_indices().rev() {
            if ch == '\n' {
                break;
            }
            if ch == '#' || ch == '@' {
                if before[..at].chars().last().map_or(true, char::is_whitespace) {
                    found = Some((at, ch));
                }
                break;
            }
        }
        let (at, ch) = found?;
        let query = before[at + 1..].to_string();
        let trigger = if ch == '@' { Trigger::At } else { Trigger::Hash };
        let spaced = query.chars().any(char::is_whitespace);
        if trigger == Trigger::Hash && spaced {
            return None;
        }
        // `@` narrows while some Job title still starts with what was typed, whitespace included.
        if trigger == Trigger::At && spaced {
            let lower = query.to_lowercase();
            if !self.face.mentions.jobs.iter().any(|job| job.title.to_lowercase().starts_with(&lower)) {
                return None;
            }
        }
        let rows = self.face.mentions.rows(trigger, &query);
        Some(Menu { trigger, from: at, query, rows, active: 0 })
    }

    pub(super) fn choose_row(&mut self, at: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(menu) = self.menu.take() else { return };
        let Some(row) = menu.rows.get(at).cloned() else { return };
        let words = self.words(cx);
        let cursor = self.request.read(cx).cursor().min(words.len());
        let next = format!("{}{}{}", &words[..menu.from], row.insert, &words[cursor..]);
        let end = menu.from + row.insert.len();
        self.request.update(cx, |state, cx| {
            state.set_value(next.clone(), window, cx);
            end_of(state, &next[..end], window, cx);
        });
        // Setting the value says nothing to the field's watchers: the draft, the read and the tag follow here.
        let request = self.request.clone();
        self.typed(&request, window, cx);
    }

    /// A repository chosen for the sheet from elsewhere: its first row, a project's before none.
    pub(super) fn pick_repository(&mut self, name: &str, cx: &mut Context<Self>) {
        if let Some(id) = self.face.chips.iter().chain(&self.face.more).find(|chip| chip.name == name).map(|chip| chip.id()) {
            self.face.pick(&id);
        }
        cx.notify();
    }

    // -- the act ----------------------------------------------------------------------------------

    pub(super) fn repository(&self) -> Option<String> {
        self.face.chosen().map(|chip| chip.name.to_string())
    }

    /// Fleet's id for the project of the row chosen; none where it is a repository in no project.
    pub(super) fn project(&self) -> Option<String> {
        self.face.chosen().and_then(|chip| chip.project.as_ref()).map(|project| project.id.to_string())
    }

    /// Start, from the button or ↵: it runs at once, pending in this frame.
    pub fn act(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.begin(false, window, cx);
    }

    /// Draft: the same start, held on its start card until Start is pressed there. A wait has no card.
    pub(super) fn draft(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.wait.is_some() {
            return;
        }
        self.begin(true, window, cx);
    }

    fn begin(&mut self, held: bool, window: &mut Window, cx: &mut Context<Self>) {
        if !self.face.connected || self.sending || self.queueing || !self.worded(cx) {
            return;
        }
        if self.face.reading == Reading::Detecting && self.repository().is_none() {
            // Nothing starts without a repository: a read still out is waited for.
            self.sending = true;
            self.drafting = held;
            self.waiting = true;
            cx.notify();
            return;
        }
        let Some(repository) = self.repository() else {
            self.ask(window, cx);
            return;
        };
        self.asking = false;
        self.sending = true;
        self.drafting = held;
        let words = self.brief(cx);
        let workflow = self.story.as_ref().map(|story| match story.kind().workflow {
            "none" => String::new(),
            id => id.to_string(),
        });
        self.face.refused = None;
        cx.emit(Asked::Start {
            words,
            repository,
            project: self.project(),
            workflow,
            model: self.model.as_ref().map(|one| one.to_string()),
            effort: self.effort.as_ref().map(|one| one.to_string()),
            held,
        });
        cx.notify();
    }

    /// The words and the repository of a start out for Fleet's answer, for a walk's scene to answer with.
    pub(super) fn started(&self, cx: &App) -> Option<(String, String)> {
        self.sending.then(|| self.repository()).flatten().map(|repository| (self.words(cx), repository))
    }

    /// No repository stands: the picker takes the call hue and its list opens until one is picked.
    fn ask(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.asking = true;
        let row = self.row.clone();
        window.focus(&row, cx);
        self.open_repositories(window, cx);
        cx.notify();
    }

    /// Queue for later: the idea is saved here and the sheet goes.
    pub(super) fn queue(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // A queued idea keeps words only, so nothing attached is dropped by queueing it.
        if !self.face.connected || self.sending || self.queueing || !self.face.queue || self.holds() || self.words(cx).trim().is_empty() {
            return;
        }
        let Some(repository) = self.repository() else {
            self.ask(window, cx);
            return;
        };
        self.queueing = true;
        cx.emit(Asked::Queue {
            words: self.words(cx),
            repository,
            project: self.project(),
            model: self.model.as_ref().map(|one| one.to_string()),
            effort: self.effort.as_ref().map(|one| one.to_string()),
        });
        cx.notify();
    }

    /// Fleet took it: the words go, the sheet clears and shuts.
    pub fn answered(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.sending = false;
        self.drafting = false;
        self.queueing = false;
        self.waiting = false;
        crate::drafts::clear(KEEP_REQUEST, cx);
        self.past.clear();
        self.future.clear();
        self.files.clear();
        self.drawn.clear();
        self.tools.clear();
        self.preset.clear();
        self.story = None;
        self.wait = None;
        self.tagged = None;
        self.request.update(cx, |state, cx| state.set_value("", window, cx));
        self.close(window, cx);
    }

    /// Fleet refused, in its own words: the sheet is as it was, words and pick kept, and the row says why.
    pub fn refused(&mut self, why: SharedString, cx: &mut Context<Self>) {
        self.face.refused = Some(why);
        self.sending = false;
        self.drafting = false;
        self.queueing = false;
        self.waiting = false;
        cx.notify();
    }

    /// The app changed the face under the sheet: a read answered, a wait arrived. A start that was
    /// waiting on the repository goes on.
    pub fn set_face(&mut self, face: Face, window: &mut Window, cx: &mut Context<Self>) {
        let hand = (self.face.reading == Reading::Picked).then(|| self.face.chosen().map(|chip| chip.id())).flatten();
        let refused = self.face.refused.clone();
        self.face = face;
        if let Some(id) = hand {
            self.face.pick(&id);
        }
        if self.face.refused.is_none() {
            self.face.refused = refused.filter(|_| false);
        } else {
            self.sending = false;
            self.drafting = false;
        }
        if let Some(wait) = self.face.wait.clone() {
            self.wait = Some(wait);
            self.tools_open = None;
        }
        if self.waiting && self.face.reading != Reading::Detecting {
            self.waiting = false;
            self.sending = false;
            let held = self.drafting;
            self.begin(held, window, cx);
        }
        self.want_features(cx);
        cx.notify();
    }

    /// What Fleet read of the brief's tag: the sentence over the request, or none where the tag names no wait.
    pub fn set_wait(&mut self, wait: Option<Wait>, cx: &mut Context<Self>) {
        if wait.is_some() {
            self.tools_open = None;
        }
        self.wait = wait;
        cx.notify();
    }

    /// A pasted snippet kept as a file.
    pub fn attach(&mut self, name: &str, cx: &mut Context<Self>) {
        self.add_file(name.to_string(), "text/plain".into(), Vec::new(), cx);
    }

    // -- what goes with the request ----------------------------------------------------------------

    /// Attach opens the Mac's own picker. On the walks' mock there is none to answer, so it adds samples, as a Job's box does.
    pub(super) fn attach_files(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.takes_files() {
            return;
        }
        if !self.live {
            self.add_file("sample.rs".into(), "text/plain".into(), b"fn main() {}\n".to_vec(), cx);
            if let Some(picture) = crate::job_view::pictures::mock_picture("Pasted file.png") {
                self.add_file("Pasted file.png".into(), "image/png".into(), picture, cx);
            }
            return;
        }
        pick_files(window, cx, |this, paths, cx| this.take_paths(paths, cx));
    }

    /// Files named by a drop or a paste are read off the main thread and attached.
    pub(super) fn take_paths(&mut self, paths: Vec<PathBuf>, cx: &mut Context<Self>) {
        if !self.takes_files() {
            return;
        }
        read_files(paths, cx, |this, name, media_type, bytes, cx| this.add_file(name, media_type, bytes, cx));
    }

    fn add_file(&mut self, name: String, media_type: String, bytes: Vec<u8>, cx: &mut Context<Self>) {
        self.next_key += 1;
        self.files.push(attached(self.next_key, name, media_type, bytes));
        cx.notify();
    }

    pub(super) fn remove_file(&mut self, key: u64, window: &mut Window, cx: &mut Context<Self>) {
        self.files.retain(|one| one.key != key);
        // The caret goes back to the request where the chip was.
        let focus = self.words_focus(cx);
        window.focus(&focus, cx);
        cx.notify();
    }

    pub(super) fn remove_drawn(&mut self, id: &str, cx: &mut Context<Self>) {
        self.drawn.retain(|one| one.id != id);
        cx.notify();
    }

    /// A paste with a picture or files on the clipboard attaches them; a long text becomes a file.
    pub(super) fn paste(&mut self, item: &ClipboardItem, cx: &mut Context<Self>) -> bool {
        if !self.takes_files() {
            return false;
        }
        let taken = clipped_keyed(item, &mut self.next_key);
        for (name, media_type, bytes) in taken.files.iter().cloned() {
            self.add_file(name, media_type, bytes, cx);
        }
        let any = taken.taken();
        if !taken.paths.is_empty() {
            self.take_paths(taken.paths, cx);
        }
        any
    }

    /// Draw sketch: the sheet opens on the request's own pad, as it was left.
    pub(super) fn open_sketch(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.takes_files() {
            return;
        }
        self.pad.update(cx, |pad, cx| pad.open(window, cx));
        cx.notify();
    }

    /// The + opens Armada's tools under the words, the first marked, with the keys in the request so the list takes
    /// them; pressed again it shuts them. A wait takes no message, so nothing opens while one is tagged.
    pub(super) fn open_tools(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.takes_files() {
            return;
        }
        self.tools_open = if self.tools_open.is_some() { None } else { Some(0) };
        self.menu = None;
        self.wait_menu = None;
        let focus = self.words_focus(cx);
        window.focus(&focus, cx);
        cx.notify();
    }

    /// The tool at `at` in the list is held, once however often it is picked, and the keys stay in the request.
    pub(super) fn pick_tool(&mut self, at: usize, window: &mut Window, cx: &mut Context<Self>) {
        self.tools_open = None;
        if let Some(Pick::Tag { kind, id, title, url }) = crate::job_view::ask_tools::rows().into_iter().nth(at).map(|row| row.pick) {
            if !self.tools.iter().any(|one| one.tag.id == id) {
                let before = self.shape_now(cx);
                self.tools.push(HeldTool { tag: SessionTag { kind, id, title, url }, key: cx.focus_handle() });
                if self.story.is_some() {
                    self.remember(before);
                }
            }
        }
        let focus = self.words_focus(cx);
        window.focus(&focus, cx);
        cx.notify();
    }

    /// A tool's chip removed: it does not go, and the caret goes back to the request.
    pub(super) fn remove_tool(&mut self, id: &str, window: &mut Window, cx: &mut Context<Self>) {
        let before = self.shape_now(cx);
        let held = self.tools.len();
        self.tools.retain(|one| one.tag.id != id);
        if self.story.is_some() && self.tools.len() != held {
            self.remember(before);
        }
        let focus = self.words_focus(cx);
        window.focus(&focus, cx);
        cx.notify();
    }

    // -- the repository row -------------------------------------------------------------------------

    /// A row picked by hand, by its [`super::face::Chip::id`].
    pub(super) fn pick(&mut self, id: SharedString, window: &mut Window, cx: &mut Context<Self>) {
        let was_asking = self.asking;
        self.face.pick(&id);
        self.face.refused = None;
        self.asking = false;
        self.want_features(cx);
        if was_asking {
            // The act was waiting on a repository: it goes at once.
            self.act(window, cx);
        } else {
            let focus = self.words_focus(cx);
            window.focus(&focus, cx);
        }
        cx.notify();
    }

    // -- keys ----------------------------------------------------------------------------------------

    pub(super) fn on_act(&mut self, key: &Act, window: &mut Window, cx: &mut Context<Self>) {
        match key.id {
            // The sheet is a dialog, and the key does nothing while one is up.
            "new_job" | "send_message" => {}
            "dispatch_after" => {
                self.plain_words(window, cx);
                self.request.update(cx, |state, cx| state.insert("@", window, cx))
            }
            "dispatch_repository" => {
                let Some(digit) = key.digit else { return };
                let Some(chip) = self.face.chips.iter().find(|chip| chip.key == Some(digit)).cloned() else { return };
                let none_chosen = self.repository().is_none();
                self.face.pick(&chip.id());
                self.asking = false;
                self.want_features(cx);
                // With none chosen, the key also sends.
                if none_chosen && self.worded(cx) {
                    self.act(window, cx);
                }
                cx.notify();
            }
            "queue_session" => self.queue(window, cx),
            "draft_session" => self.draft(window, cx),
            _ => cx.propagate(),
        }
    }

    pub(super) fn on_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let key = event.keystroke.key.as_str();
        let mods = &event.keystroke.modifiers;
        let plain = !mods.platform && !mods.control && !mods.alt;
        let in_request = self.request.focus_handle(cx).is_focused(window);
        if in_request && talk::key_down(self, event, window, cx) {
            cx.stop_propagation();
            return;
        }
        if plain_cmd(mods) && matches!(key, "z" | "y") && self.history_key(key, mods.shift, in_request, window, cx) {
            cx.stop_propagation();
            return;
        }
        // The tools, as the `@` menu: arrows wrap, ↵ or ⇥ pick, Esc closes them with the keys in the request.
        if let Some(active) = self.tools_open {
            let len = crate::job_view::ask_tools::rows().len().max(1);
            match key {
                "down" | "up" => {
                    self.tools_open = Some(if key == "down" { (active + 1) % len } else { (active + len - 1) % len });
                    if len > 1 {
                        crate::ui_sounds::moved();
                    }
                }
                "enter" | "tab" if plain => self.pick_tool(active, window, cx),
                "escape" => {
                    self.tools_open = None;
                    let focus = self.words_focus(cx);
                    window.focus(&focus, cx);
                }
                _ => return,
            }
            cx.stop_propagation();
            cx.notify();
            return;
        }
        // The `@` menu: arrows wrap, ↵ or ⇥ choose, Esc closes it and nothing else.
        if let Some(menu) = &mut self.menu {
            let len = menu.rows.len().max(1);
            match key {
                "down" | "up" => {
                    let was = menu.active;
                    menu.active = if key == "down" { (menu.active + 1) % len } else { (menu.active + len - 1) % len };
                    if menu.active != was {
                        crate::ui_sounds::moved();
                    }
                }
                "enter" | "tab" if plain => {
                    let at = menu.active;
                    self.choose_row(at, window, cx);
                }
                "escape" => self.menu = None,
                _ => return,
            }
            cx.stop_propagation();
            cx.notify();
            return;
        }
        // A sentence being written takes the keys while a blank has them.
        if self.story.is_some() && self.story_key.is_focused(window) && self.story_keys(event, window, cx) {
            cx.stop_propagation();
            return;
        }
        // The words begin an opening: ↑ ↓ choose among the ones they begin, ⇥ or → at the end takes the one marked.
        if in_request && plain && !mods.shift {
            let begun = self.openings(cx);
            if !begun.is_empty() {
                let marked = self.opening % begun.len();
                let at_end = self.request.read(cx).cursor() >= self.words(cx).len();
                match key {
                    "tab" => self.take_opening(begun[marked], window, cx),
                    "right" if at_end => self.take_opening(begun[marked], window, cx),
                    "down" | "up" if begun.len() > 1 => {
                        self.opening = if key == "down" { (marked + 1) % begun.len() } else { (marked + begun.len() - 1) % begun.len() };
                        crate::ui_sounds::moved();
                    }
                    _ => return,
                }
                cx.stop_propagation();
                cx.notify();
                return;
            }
        }
        match key {
            "escape" => {
                if talk::put_away(self, cx) || self.wait_menu.take().is_some() || self.popup.take().is_some() {
                    // Only the menu closes.
                } else if self.asking {
                    self.asking = false;
                    let focus = self.words_focus(cx);
                    window.focus(&focus, cx);
                } else {
                    self.close(window, cx);
                }
            }
            // Tab goes round the sheet and the card under it, and never out to the window behind.
            "tab" if plain && self.popup.is_none() => self.step_focus(mods.shift, window, cx),
            "enter" if plain && !mods.shift && in_request => self.act(window, cx),
            "enter" | "space" | "down" if plain && self.row.is_focused(window) => self.open_repositories(window, cx),
            _ => return,
        }
        cx.stop_propagation();
        cx.notify();
    }

    // -- the order Tab takes ---------------------------------------------------------------------------

    /// Whether Queue for later can be pressed: the stop is left out while it cannot.
    pub(super) fn queue_live(&self, cx: &App) -> bool {
        self.face.connected && !self.sending && !self.queueing && !self.words(cx).trim().is_empty()
    }

    /// Whether Draft and Start can be pressed.
    pub(super) fn act_live(&self, cx: &App) -> bool {
        self.face.connected && !self.queueing && self.worded(cx) && !self.sending
    }

    /// Every stop Tab visits, in order: the wait sentence's parts when there is one, the tools held, the request,
    /// its +, Attach, Draw sketch and Talk, the repository, Model, Effort, Queue for later, Draft, Start, then
    /// the card under the sheet. A control that is off, or has no card to draw, has no stop.
    pub(super) fn stops(&self, cx: &App) -> Vec<FocusHandle> {
        let mut stops = Vec::new();
        if let Some(wait) = &self.wait {
            let parts = if wait.needs.is_some() { 4 } else { 3 };
            stops.extend(self.parts[..parts].iter().cloned());
        }
        // The sentence stands where the request's box does, and its tools come after it, as its footer draws them.
        if self.story.is_some() {
            stops.push(self.story_key.clone());
            stops.extend(self.tools.iter().map(|one| one.key.clone()));
        } else {
            stops.extend(self.tools.iter().map(|one| one.key.clone()));
            stops.push(self.request.focus_handle(cx));
        }
        if self.takes_files() {
            stops.extend([self.tools_key.clone(), self.attach_key.clone(), self.sketch_key.clone()]);
        }
        stops.push(self.talk_key.clone());
        if self.face.connected {
            stops.extend([self.row.clone(), self.model_key.clone(), self.effort_key.clone()]);
        }
        let act = self.act_live(cx);
        if self.face.queue && !self.holds() && self.queue_live(cx) {
            stops.push(self.queue_key.clone());
        }
        if self.wait.is_none() && act {
            stops.push(self.draft_key.clone());
        }
        if act {
            stops.push(self.act_key.clone());
        }
        if self.here_in().is_some() {
            stops.push(self.here_key.clone());
        }
        stops
    }

    /// Tab, or Shift-Tab with `back`, from whichever stop has the keys.
    pub(super) fn step_focus(&mut self, back: bool, window: &mut Window, cx: &mut Context<Self>) {
        let stops = self.stops(cx);
        let now = stops.iter().position(|stop| stop.is_focused(window));
        self.wait_menu = None;
        window.focus(&stops[next_stop(stops.len(), now, back)], cx);
    }

    // -- popovers ----------------------------------------------------------------------------------------

    fn open_pop(&mut self, kind: Pop, from: Bounds<Pixels>, name: &'static str, entries: Vec<Entry>, window: &mut Window, cx: &mut Context<Self>) {
        let gap = cx.look().length(token::SPACE_1);
        let at = point(from.origin.x, from.origin.y + from.size.height + gap);
        let dropdown = cx.new(|cx| Dropdown::new(name, entries, at, Anchor::TopLeft, false, cx));
        let picked = cx.subscribe_in(&dropdown, window, move |this, _, picked: &Picked, window, cx| {
            let Picked::Entry { id, .. } = picked;
            this.popup = None;
            this.picked(kind, id, window, cx);
        });
        let dismissed = cx.subscribe_in(&dropdown, window, |this, _, _: &DismissEvent, window, cx| {
            this.popup = None;
            let focus = this.words_focus(cx);
            window.focus(&focus, cx);
            cx.notify();
        });
        let keys = dropdown.read(cx).focus_handle(cx);
        window.defer(cx, move |window, cx| window.focus(&keys, cx));
        self.popup = Some(Popup { dropdown, _picked: picked, _dismissed: dismissed });
        cx.notify();
    }

    fn picked(&mut self, kind: Pop, id: &str, window: &mut Window, cx: &mut Context<Self>) {
        let focus = self.words_focus(cx);
        window.focus(&focus, cx);
        match kind {
            Pop::Model => self.model = id.strip_prefix("model:").filter(|one| !one.is_empty()).map(|one| one.to_string().into()),
            Pop::Effort => self.effort = id.strip_prefix("effort:").filter(|one| !one.is_empty()).map(|one| one.to_string().into()),
            Pop::Repository => {
                if let Some(row) = id.strip_prefix("repo:") {
                    self.pick(row.to_string().into(), window, cx);
                }
                return;
            }
        }
        let kept = format!("{}\n{}", self.model.as_deref().unwrap_or(""), self.effort.as_deref().unwrap_or(""));
        crate::drafts::keep(KEEP_TUNING, &kept, cx);
        cx.notify();
    }

    pub(super) fn open_model(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(from) = self.at.model.get() else { return };
        let mut entries = vec![Entry::Choice { id: "model:", label: "Auto".into(), on: self.model.is_none() }];
        for model in &self.face.tuning.models {
            entries.push(Entry::Choice { id: id_of(format!("model:{model}")), label: model.clone(), on: self.model.as_ref() == Some(model) });
        }
        self.open_pop(Pop::Model, from, "Model", entries, window, cx);
    }

    pub(super) fn open_effort(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(from) = self.at.effort.get() else { return };
        let mut entries = vec![Entry::Choice { id: "effort:", label: "Auto".into(), on: self.effort.is_none() }];
        for effort in &self.face.tuning.efforts {
            entries.push(Entry::Choice { id: id_of(format!("effort:{effort}")), label: effort.clone(), on: self.effort.as_ref() == Some(effort) });
        }
        self.open_pop(Pop::Effort, from, "Effort", entries, window, cx);
    }

    /// The picker's list: his projects, each with its repository on a line under the name, then under No project
    /// every repository Fleet serves. The first nine carry their ⌥ keys, and a search goes over them once
    /// there are more than nine.
    pub(super) fn open_repositories(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(from) = self.at.repository.get() else { return };
        let mut entries = Vec::new();
        if !self.face.more.is_empty() {
            entries.push(Entry::Search { label: "Find a project", to_rule: false });
        }
        let mut in_project = false;
        for chip in self.face.chips.iter().chain(&self.face.more) {
            if chip.project.is_none() && in_project {
                entries.push(Entry::Label("No project"));
            }
            in_project = chip.project.is_some();
            let row = crate::dropdown::Row::new(id_of(format!("repo:{}", chip.id())), chip.label().clone()).ticked(chip.mark == Mark::Chosen);
            let row = match &chip.project {
                Some(_) => row.icon("layers").under(chip.name.clone()),
                None => row.icon("git-branch"),
            };
            let row = match chip.key.and_then(|n| crate::keymap::shown_digit("dispatch_repository", n)) {
                Some(key) => row.key(key),
                None => row,
            };
            entries.push(Entry::Row(row));
        }
        self.open_pop(Pop::Repository, from, "Project", entries, window, cx);
    }

    // -- the wait sentence ------------------------------------------------------------------------------

    pub(super) fn cycle(&mut self, part: Part, back: bool, cx: &mut Context<Self>) {
        if let Some(wait) = &mut self.wait {
            wait.cycled(part, back);
        }
        cx.notify();
    }

    pub(super) fn wait_on(&mut self, at: usize, cx: &mut Context<Self>) {
        self.wait_menu = None;
        let id = self.wait.as_ref().and_then(|wait| wait.others.get(at)).map(|one| one.id.clone());
        if let (Some(id), Some(wait)) = (id, self.wait.as_mut()) {
            wait.waits_on(&id);
        }
        cx.notify();
    }

    pub(super) fn part_keys(&mut self, part: Part, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let key = event.keystroke.key.as_str();
        if key == "escape" {
            if self.wait_menu.take().is_none() {
                let focus = self.words_focus(cx);
                window.focus(&focus, cx);
            }
            cx.stop_propagation();
            cx.notify();
            return;
        }
        if part == Part::On {
            let others = self.wait.as_ref().map_or(0, |wait| wait.others.len()).max(1);
            match (key, self.wait_menu) {
                ("down" | "up" | "space" | "enter", None) => self.wait_menu = Some(0),
                ("down", Some(at)) => {
                    self.wait_menu = Some((at + 1) % others);
                    if others > 1 {
                        crate::ui_sounds::moved();
                    }
                }
                ("up", Some(at)) => {
                    self.wait_menu = Some((at + others - 1) % others);
                    if others > 1 {
                        crate::ui_sounds::moved();
                    }
                }
                ("space" | "enter", Some(at)) => self.wait_on(at, cx),
                _ => return,
            }
        } else {
            match key {
                "down" | "space" | "enter" => self.cycle(part, false, cx),
                "up" => self.cycle(part, true, cx),
                _ => return,
            }
        }
        cx.stop_propagation();
        cx.notify();
    }
}

impl Hears for DispatchSheet {
    fn talk(&mut self) -> &mut Talk {
        &mut self.talk
    }

    fn talk_now(&self) -> &Talk {
        &self.talk
    }

    fn talk_box(&self) -> Entity<TextareaState> {
        self.request.clone()
    }

    fn talk_source(&mut self, _cx: &mut Context<Self>) -> Option<(Engine, Source)> {
        Some((self.voice, talk::source_for(self.voice, !self.live, Default::default)))
    }

    /// Heard words join the request; nothing is started by them.
    fn heard_words(&mut self, words: Vec<String>, window: &mut Window, cx: &mut Context<Self>) {
        self.plain_words(window, cx);
        let next = talk::joined_all(self.words(cx), &words);
        self.request.update(cx, |state, cx| {
            state.set_value(next.clone(), window, cx);
            end_of(state, &next, window, cx);
        });
        // Setting the value says nothing to the field's watchers: the draft and the reads follow here.
        let request = self.request.clone();
        self.typed(&request, window, cx);
    }
}

/// The stop after `now` among `len`, wrapping at both ends. Keys on nothing in the order start at its first stop, or its last going back.
fn next_stop(len: usize, now: Option<usize>, back: bool) -> usize {
    match (now, back) {
        (Some(at), false) => (at + 1) % len,
        (Some(at), true) => (at + len - 1) % len,
        (None, false) => 0,
        (None, true) => len - 1,
    }
}

/// ⌘ held, and no ⌥ or ⌃: a chord of the history.
fn plain_cmd(mods: &gpui::Modifiers) -> bool {
    mods.platform && !mods.alt && !mods.control
}

/// What a feature blank lists: the catalog held for `project`, or none where it is held for another or not yet. With no project
/// chosen, his projects, one row each, in the order the picker lists them.
fn choices<'a>(held: &'a Option<(String, Vec<FeatureChoice>)>, face: &Face, project: Option<&str>) -> Cow<'a, [FeatureChoice]> {
    let Some(project) = project else {
        let mut rows: Vec<FeatureChoice> = Vec::new();
        for one in face.chips.iter().chain(&face.more).filter_map(|chip| chip.project.as_ref()) {
            if !rows.iter().any(|row| row.slug == one.id) {
                rows.push(FeatureChoice { title: one.name.clone(), area: SharedString::default(), slug: one.id.clone(), kind: Kind::Project });
            }
        }
        return Cow::Owned(rows);
    };
    match held {
        Some((one, features)) if one == project => Cow::Borrowed(features),
        _ => Cow::Borrowed(&[]),
    }
}

const PLACEHOLDER: &str = "Describe the work, or paste a link to a ticket.";

/// The caret at the end of `text`, which `set_value` leaves at the start.
fn end_of(state: &mut TextareaState, text: &str, window: &mut Window, cx: &mut Context<TextareaState>) {
    let line = text.matches('\n').count() as u32;
    let last = text.rsplit('\n').next().unwrap_or("");
    state.set_cursor_position(Position::new(line, last.encode_utf16().count() as u32), window, cx);
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{TestAppContext, VisualTestContext};

    fn open<'a>(face: Face, here: Option<&'static str>, cx: &'a mut TestAppContext) -> (Entity<DispatchSheet>, &'a mut VisualTestContext) {
        cx.update(|cx| {
            gpui_component::init(cx);
            crate::look::init(cx);
            crate::keymap::install(&crate::keymap::Rebinding::default(), cx);
        });
        let (sheet, cx) = cx.add_window_view(|window, cx| DispatchSheet::new(face, window, cx));
        sheet.update_in(cx, |sheet, window, cx| {
            sheet.set_here(here.map(|name| name.to_string().into()), cx);
            sheet.request.update(cx, |state, cx| state.set_value("Fix the flaky login test", window, cx));
            let request = sheet.request.focus_handle(cx);
            window.focus(&request, cx);
        });
        (sheet, cx)
    }

    /// The stops as the names this module gives its handles, in the order Tab visits them.
    fn named(sheet: &DispatchSheet, cx: &App) -> Vec<&'static str> {
        sheet
            .stops(cx)
            .iter()
            .map(|stop| {
                let is = |handle: &FocusHandle| handle == stop;
                if is(&sheet.request.focus_handle(cx)) {
                    "request"
                } else if is(&sheet.row) {
                    "repository"
                } else if is(&sheet.model_key) {
                    "model"
                } else if is(&sheet.effort_key) {
                    "effort"
                } else if is(&sheet.queue_key) {
                    "queue"
                } else if is(&sheet.draft_key) {
                    "draft"
                } else if is(&sheet.act_key) {
                    "start"
                } else if is(&sheet.here_key) {
                    "card"
                } else if is(&sheet.tools_key) {
                    "tools"
                } else if sheet.tools.iter().any(|one| is(&one.key)) {
                    "tool"
                } else if is(&sheet.attach_key) {
                    "attach"
                } else if is(&sheet.sketch_key) {
                    "sketch"
                } else if is(&sheet.talk_key) {
                    "talk"
                } else if sheet.parts.iter().any(is) {
                    "part"
                } else {
                    "unknown"
                }
            })
            .collect()
    }

    /// Where the keys are, by name.
    fn at(sheet: &DispatchSheet, window: &Window, cx: &App) -> &'static str {
        let now = sheet.stops(cx).iter().position(|stop| stop.is_focused(window)).expect("the keys are on a stop");
        named(sheet, cx)[now]
    }

    #[test]
    fn the_next_stop_wraps_at_both_ends() {
        assert_eq!(next_stop(3, Some(0), false), 1);
        assert_eq!(next_stop(3, Some(2), false), 0);
        assert_eq!(next_stop(3, Some(0), true), 2);
        assert_eq!(next_stop(3, Some(1), true), 0);
        assert_eq!(next_stop(3, None, false), 0);
        assert_eq!(next_stop(3, None, true), 2);
    }

    #[gpui::test]
    fn tab_goes_round_the_sheet_and_shift_tab_goes_back_the_same_way(cx: &mut TestAppContext) {
        let (sheet, cx) = open(crate::dispatch::scenes::rest(&[]), Some("sample-api"), cx);
        let order = ["request", "tools", "attach", "sketch", "talk", "repository", "model", "effort", "queue", "draft", "start", "card"];
        sheet.update_in(cx, |sheet, window, cx| {
            assert_eq!(named(sheet, cx), order);
            assert_eq!(at(sheet, window, cx), "request");
            for next in order.iter().skip(1).chain(Some(&"request")) {
                sheet.step_focus(false, window, cx);
                assert_eq!(at(sheet, window, cx), *next, "Tab");
            }
            // Back from the request: the last stop, then each one before it, to the request again.
            for next in order.iter().rev() {
                sheet.step_focus(true, window, cx);
                assert_eq!(at(sheet, window, cx), *next, "Shift-Tab");
            }
        });
    }

    #[gpui::test]
    fn a_stop_that_cannot_act_is_left_out(cx: &mut TestAppContext) {
        let (sheet, cx) = open(crate::dispatch::scenes::rest(&[]), None, cx);
        sheet.update_in(cx, |sheet, window, cx| {
            assert_eq!(named(sheet, cx), ["request", "tools", "attach", "sketch", "talk", "repository", "model", "effort", "queue", "draft", "start"], "no card where the title bar names nowhere");
            sheet.request.update(cx, |state, cx| state.set_value("", window, cx));
            assert_eq!(named(sheet, cx), ["request", "tools", "attach", "sketch", "talk", "repository", "model", "effort"], "nothing to start or queue");
            sheet.face.connected = false;
            assert_eq!(named(sheet, cx), ["request", "tools", "attach", "sketch", "talk"], "Fleet unreachable: only the request and its own tools");
        });
    }

    #[gpui::test]
    fn start_sends_the_project_of_the_row_picked(cx: &mut TestAppContext) {
        let (sheet, cx) = open(crate::dispatch::scenes::rest(&[]), None, cx);
        let asked = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let heard = std::rc::Rc::clone(&asked);
        let _sub = cx.update(|_, cx| {
            cx.subscribe(&sheet, move |_, asked: &Asked, _| {
                if let Asked::Start { repository, project, .. } = asked {
                    heard.borrow_mut().push((repository.clone(), project.clone()));
                }
            })
        });
        sheet.update_in(cx, |sheet, window, cx| {
            // ⌥3 is Storefront in sample-api; ⌥7 is sample-docs, a folder in no project.
            for digit in [3, 7] {
                let row = sheet.face.chips.iter().find(|chip| chip.key == Some(digit)).expect("a keyed row").id();
                sheet.face.pick(&row);
                sheet.act(window, cx);
                sheet.sending = false;
            }
        });
        assert_eq!(*asked.borrow(), [("sample-api".to_string(), Some("storefront".to_string())), ("sample-docs".to_string(), None)]);
    }

    #[gpui::test]
    fn the_card_and_command_t_take_the_row_picked_with_its_project(cx: &mut TestAppContext) {
        let (sheet, cx) = open(crate::dispatch::scenes::rest(&[]), Some("sample-api"), cx);
        sheet.update_in(cx, |sheet, _, _| {
            assert_eq!(sheet.named_row(), None, "nothing picked: the title bar says where");
            let row = sheet.face.chips.iter().find(|chip| chip.key == Some(3)).expect("Storefront in sample-api").id();
            sheet.face.pick(&row);
            assert_eq!(sheet.named_row(), Some(PickedRow { repository: "sample-api".into(), project: Some("storefront".into()) }));
            let row = sheet.face.chips.iter().find(|chip| chip.key == Some(7)).expect("sample-docs in no project").id();
            sheet.face.pick(&row);
            assert_eq!(sheet.named_row(), Some(PickedRow { repository: "sample-docs".into(), project: None }));
        });
    }

    #[gpui::test]
    fn the_wait_sentence_stands_before_the_request(cx: &mut TestAppContext) {
        let face = crate::dispatch::scenes::waiting(crate::dispatch::scenes::rest(&[]), None);
        let (sheet, cx) = open(face, Some("sample-api"), cx);
        sheet.update_in(cx, |sheet, window, cx| {
            assert_eq!(named(sheet, cx), ["part", "part", "part", "part", "request", "talk", "repository", "model", "effort", "queue", "start", "card"], "a wait has no Draft, +, Attach or Draw sketch");
            sheet.step_focus(true, window, cx);
            assert_eq!(at(sheet, window, cx), "part", "Shift-Tab from the request enters the sentence");
            assert!(sheet.parts[3].is_focused(window));
        });
    }

    #[gpui::test]
    fn the_tools_button_is_no_stop_while_a_wait_is_tagged(cx: &mut TestAppContext) {
        let face = crate::dispatch::scenes::waiting(crate::dispatch::scenes::rest(&[]), None);
        let (sheet, cx) = open(face, Some("sample-api"), cx);
        sheet.update_in(cx, |sheet, window, cx| {
            assert!(!named(sheet, cx).contains(&"tools"), "a Job that waits starts with no message");
            sheet.open_tools(window, cx);
            assert_eq!(sheet.tools_open, None, "the list does not open");
            sheet.set_wait(None, cx);
            assert_eq!(&named(sheet, cx)[..2], ["request", "tools"], "with no wait the + follows the request");
        });
    }

    fn held_tools(sheet: &DispatchSheet) -> Vec<String> {
        sheet.tools_held().into_iter().map(|tag| tag.id).collect()
    }

    #[gpui::test]
    fn a_removed_tool_chip_is_not_sent(cx: &mut TestAppContext) {
        let (sheet, cx) = open(crate::dispatch::scenes::rest(&[]), None, cx);
        sheet.update_in(cx, |sheet, window, cx| {
            // Sketch is the first row, Takes the fourth.
            sheet.open_tools(window, cx);
            sheet.pick_tool(0, window, cx);
            sheet.open_tools(window, cx);
            sheet.pick_tool(3, window, cx);
            assert_eq!(held_tools(sheet), ["sketch", "takes"]);
            assert_eq!(&named(sheet, cx)[..3], ["tool", "tool", "request"], "each chip is a stop over the request");
            assert!(!named(sheet, cx).contains(&"queue"), "Queue for later keeps nothing but words");
            sheet.remove_tool("sketch", window, cx);
            assert_eq!(held_tools(sheet), ["takes"]);
            assert!(sheet.request.focus_handle(cx).is_focused(window), "the caret goes back to the request");
        });
    }

    fn names(sheet: &DispatchSheet) -> Vec<String> {
        sheet.files.iter().map(|one| one.name.clone()).collect()
    }

    fn drawn(id: &str) -> crate::sketch::DrawnSketch {
        crate::sketch::DrawnSketch { id: id.into(), title: "Sketch".into(), drawing: Default::default() }
    }

    #[gpui::test]
    fn attach_adds_a_chip_and_remove_drops_it(cx: &mut TestAppContext) {
        let (sheet, cx) = open(crate::dispatch::scenes::rest(&[]), None, cx);
        sheet.update_in(cx, |sheet, window, cx| {
            // Mock data has no picker to answer, so Attach adds the samples a Job's box adds.
            sheet.attach_files(window, cx);
            assert_eq!(names(sheet), ["sample.rs", "Pasted file.png"]);
            let key = sheet.files[0].key;
            sheet.remove_file(key, window, cx);
            assert_eq!(names(sheet), ["Pasted file.png"]);
            assert!(sheet.request.focus_handle(cx).is_focused(window), "the caret goes back to the request");
        });
    }

    #[gpui::test]
    fn a_wait_takes_no_files_or_sketches(cx: &mut TestAppContext) {
        let face = crate::dispatch::scenes::waiting(crate::dispatch::scenes::rest(&[]), None);
        let (sheet, cx) = open(face, Some("sample-api"), cx);
        sheet.update_in(cx, |sheet, window, cx| {
            assert!(!sheet.takes_files(), "a Job that waits starts with no message");
            sheet.attach_files(window, cx);
            let picture = gpui::Image::from_bytes(gpui::ImageFormat::Png, vec![1, 2, 3]);
            assert!(!sheet.paste(&ClipboardItem::new_image(&picture), cx), "the paste is left to the words");
            sheet.take_paths(vec![std::path::PathBuf::from("/tmp/dropped.txt")], cx);
            assert!(names(sheet).is_empty(), "nothing attaches while a wait is tagged");
            sheet.open_sketch(window, cx);
            assert!(!sheet.pad.read(cx).is_open(), "Draw sketch does not open");
        });
    }

    #[gpui::test]
    fn a_pasted_picture_attaches(cx: &mut TestAppContext) {
        let (sheet, cx) = open(crate::dispatch::scenes::rest(&[]), None, cx);
        sheet.update_in(cx, |sheet, _, cx| {
            let picture = gpui::Image::from_bytes(gpui::ImageFormat::Png, vec![1, 2, 3]);
            assert!(sheet.paste(&ClipboardItem::new_image(&picture), cx), "the paste is taken, not typed");
            assert_eq!(names(sheet), ["Pasted file.png"]);
            assert!(!sheet.paste(&ClipboardItem::new_string("a few words".into()), cx), "short words are typed");
            assert_eq!(sheet.files.len(), 1);
        });
    }

    #[gpui::test]
    fn dropped_files_attach(cx: &mut TestAppContext) {
        let (sheet, cx) = open(crate::dispatch::scenes::rest(&[]), None, cx);
        let dir = std::env::temp_dir().join(format!("dispatch-drop-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("a folder");
        let file = dir.join("notes.md");
        std::fs::write(&file, "# Notes").expect("a file");
        sheet.update(cx, |sheet, cx| sheet.take_paths(vec![file.clone()], cx));
        cx.run_until_parked();
        sheet.update(cx, |sheet, _| {
            assert_eq!(names(sheet), ["notes.md"]);
            assert_eq!(sheet.files[0].bytes.as_slice(), b"# Notes");
        });
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[gpui::test]
    fn a_sketch_drawn_on_the_request_is_held_and_hides_queue(cx: &mut TestAppContext) {
        let (sheet, cx) = open(crate::dispatch::scenes::rest(&[]), None, cx);
        sheet.update_in(cx, |sheet, window, cx| {
            assert!(named(sheet, cx).contains(&"queue"));
            sheet.open_sketch(window, cx);
            assert!(sheet.pad.read(cx).is_open(), "Draw sketch opens the request's own pad");
        });
        let pad = sheet.read_with(cx, |sheet, _| sheet.pad.clone());
        pad.update(cx, |_, cx| cx.emit(PadEvent::Attached(drawn("d1"))));
        sheet.update_in(cx, |sheet, _, cx| {
            let (files, sketches) = sheet.held();
            assert!(files.is_empty());
            assert_eq!(sketches.iter().map(|one| one.id.as_str()).collect::<Vec<_>>(), ["d1"]);
            assert!(!named(sheet, cx).contains(&"queue"), "Queue for later keeps nothing but words");
            sheet.remove_drawn("d1", cx);
            assert!(sheet.held().1.is_empty());
            assert!(named(sheet, cx).contains(&"queue"));
        });
    }

    #[gpui::test]
    fn files_and_sketches_leave_with_the_answer(cx: &mut TestAppContext) {
        let (sheet, cx) = open(crate::dispatch::scenes::rest(&[]), None, cx);
        sheet.update_in(cx, |sheet, window, cx| {
            sheet.attach("pasted-snippet.txt", cx);
            sheet.drawn.push(drawn("d1"));
            sheet.answered(window, cx);
            assert!(!sheet.holds(), "a Job started takes them; the next sheet starts empty");
        });
    }

    // -- the sentence ---------------------------------------------------------------------------------

    /// One chord or key, as the sheet's own handler hears it.
    fn press(sheet: &mut DispatchSheet, key: &'static str, window: &mut Window, cx: &mut Context<DispatchSheet>) {
        let keystroke = gpui::Keystroke::parse(key).expect("a key");
        sheet.on_key(&KeyDownEvent { keystroke, is_held: false, prefer_character_input: false }, window, cx);
    }

    /// Letters typed one key at a time.
    fn write(sheet: &mut DispatchSheet, text: &str, window: &mut Window, cx: &mut Context<DispatchSheet>) {
        for letter in text.chars() {
            let keystroke = gpui::Keystroke { modifiers: gpui::Modifiers::none(), key: letter.to_string(), key_char: Some(letter.to_string()) };
            sheet.on_key(&KeyDownEvent { keystroke, is_held: false, prefer_character_input: false }, window, cx);
        }
    }

    /// The box holds "When" with the keys in it, and Tab takes the opening it begins.
    fn take_bug(sheet: &mut DispatchSheet, window: &mut Window, cx: &mut Context<DispatchSheet>) {
        sheet.request.update(cx, |state, cx| state.set_value("When", window, cx));
        let request = sheet.request.focus_handle(cx);
        window.focus(&request, cx);
        press(sheet, "tab", window, cx);
    }

    fn blank_words(sheet: &DispatchSheet) -> Vec<String> {
        sheet.story.as_ref().expect("a sentence stands").values.clone()
    }

    #[gpui::test]
    fn typing_when_then_tab_takes_the_bug_sentence_with_the_caret_in_the_first_blank_and_its_tools_held(cx: &mut TestAppContext) {
        let (sheet, cx) = open(crate::dispatch::scenes::rest(&[]), None, cx);
        sheet.update_in(cx, |sheet, window, cx| {
            sheet.request.update(cx, |state, cx| state.set_value("When", window, cx));
            assert!(sheet.tools_held().is_empty());
            press(sheet, "tab", window, cx);
            let story = sheet.story.as_ref().expect("Tab took the opening");
            assert_eq!(story.kind().id, "bug");
            assert_eq!(story.piece(), &super::super::story_kinds::Piece::Blank("do what"), "the caret is in the first blank");
            assert!(sheet.story_key.is_focused(window), "the sentence has the keys");
            assert_eq!(sheet.request.read(cx).value().to_string(), "", "the typed opening is the sentence now");
            assert_eq!(held_tools(sheet), ["terminal", "browser"], "the kind's preset is held");
            assert_eq!(sheet.preset, ["terminal", "browser"]);
        });
    }

    #[gpui::test]
    fn backspace_in_an_empty_first_blank_returns_plain_words_with_the_opening_kept_and_the_preset_gone(cx: &mut TestAppContext) {
        let (sheet, cx) = open(crate::dispatch::scenes::rest(&[]), None, cx);
        sheet.update_in(cx, |sheet, window, cx| {
            take_bug(sheet, window, cx);
            assert_eq!(held_tools(sheet), ["terminal", "browser"]);
            press(sheet, "backspace", window, cx);
            assert!(sheet.story.is_none(), "plain words again");
            assert_eq!(sheet.request.read(cx).value().to_string(), "When I ", "the opening is kept");
            assert!(sheet.request.focus_handle(cx).is_focused(window), "the keys are in the box");
            assert!(held_tools(sheet).is_empty(), "the tools its kind put in go");
        });
    }

    #[gpui::test]
    fn esc_turns_the_sentence_into_its_words_as_plain_text(cx: &mut TestAppContext) {
        let (sheet, cx) = open(crate::dispatch::scenes::rest(&[]), None, cx);
        sheet.update_in(cx, |sheet, window, cx| {
            take_bug(sheet, window, cx);
            write(sheet, "click", window, cx);
            assert_eq!(blank_words(sheet)[0], "click");
            press(sheet, "escape", window, cx);
            assert!(sheet.story.is_none());
            assert_eq!(sheet.request.read(cx).value().to_string(), "When I click");
            assert!(sheet.request.focus_handle(cx).is_focused(window));
        });
    }

    #[gpui::test]
    fn a_run_of_letters_in_one_blank_is_one_step_command_z_takes_it_back_and_command_y_or_shift_command_z_brings_it_back(cx: &mut TestAppContext) {
        let (sheet, cx) = open(crate::dispatch::scenes::rest(&[]), None, cx);
        sheet.update_in(cx, |sheet, window, cx| {
            take_bug(sheet, window, cx);
            write(sheet, "click", window, cx);
            assert_eq!(blank_words(sheet)[0], "click");
            press(sheet, "cmd-z", window, cx);
            assert_eq!(blank_words(sheet)[0], "", "five letters, one step");
            assert!(sheet.story.is_some(), "the sentence is still written");
            press(sheet, "cmd-y", window, cx);
            assert_eq!(blank_words(sheet)[0], "click", "⌘Y");
            press(sheet, "cmd-z", window, cx);
            assert_eq!(blank_words(sheet)[0], "");
            press(sheet, "cmd-shift-z", window, cx);
            assert_eq!(blank_words(sheet)[0], "click", "⇧⌘Z");
        });
    }

    #[gpui::test]
    fn a_new_change_after_an_undo_drops_the_redo(cx: &mut TestAppContext) {
        let (sheet, cx) = open(crate::dispatch::scenes::rest(&[]), None, cx);
        sheet.update_in(cx, |sheet, window, cx| {
            take_bug(sheet, window, cx);
            write(sheet, "click", window, cx);
            press(sheet, "cmd-z", window, cx);
            assert_eq!(blank_words(sheet)[0], "");
            write(sheet, "tap", window, cx);
            press(sheet, "cmd-y", window, cx);
            assert_eq!(blank_words(sheet)[0], "tap", "what ⌘Z took back is gone");
            press(sheet, "cmd-shift-z", window, cx);
            assert_eq!(blank_words(sheet)[0], "tap");
        });
    }

    #[gpui::test]
    fn command_z_after_esc_brings_the_sentence_back(cx: &mut TestAppContext) {
        let (sheet, cx) = open(crate::dispatch::scenes::rest(&[]), None, cx);
        sheet.update_in(cx, |sheet, window, cx| {
            take_bug(sheet, window, cx);
            write(sheet, "click", window, cx);
            press(sheet, "escape", window, cx);
            assert!(sheet.story.is_none());
            assert!(held_tools(sheet).is_empty());
            press(sheet, "cmd-z", window, cx);
            assert_eq!(blank_words(sheet)[0], "click", "the sentence is back with what was typed");
            assert!(sheet.story_key.is_focused(window), "and has the keys");
            assert_eq!(held_tools(sheet), ["terminal", "browser"], "with its tools");
            assert_eq!(sheet.words(cx), "When I click");
        });
    }

    #[gpui::test]
    fn command_z_past_the_taken_opening_returns_plain_words_with_the_typed_opening(cx: &mut TestAppContext) {
        let (sheet, cx) = open(crate::dispatch::scenes::rest(&[]), None, cx);
        sheet.update_in(cx, |sheet, window, cx| {
            take_bug(sheet, window, cx);
            assert!(sheet.story.is_some());
            press(sheet, "cmd-z", window, cx);
            assert!(sheet.story.is_none(), "plain mode");
            assert_eq!(sheet.request.read(cx).value().to_string(), "When", "the opening as it was typed");
            assert!(sheet.request.focus_handle(cx).is_focused(window));
            assert!(held_tools(sheet).is_empty(), "the preset goes with the sentence");
        });
    }

    /// Every `Asked::Start` the sheet emits, as (words, workflow).
    fn starts_heard(sheet: &Entity<DispatchSheet>, cx: &mut VisualTestContext) -> (std::rc::Rc<std::cell::RefCell<Vec<(String, Option<String>)>>>, Subscription) {
        let asked = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let heard = std::rc::Rc::clone(&asked);
        let sub = cx.update(|_, cx| {
            cx.subscribe(sheet, move |_, asked: &Asked, _| {
                if let Asked::Start { words, workflow, .. } = asked {
                    heard.borrow_mut().push((words.clone(), workflow.clone()));
                }
            })
        });
        (asked, sub)
    }

    /// The sheet starts the sentence of `opening` with `first` in its first blank, in the row keyed ⌥3.
    fn start_sentence(sheet: &mut DispatchSheet, opening: &str, first: &str, window: &mut Window, cx: &mut Context<DispatchSheet>) {
        sheet.request.update(cx, |state, cx| state.set_value(opening, window, cx));
        let request = sheet.request.focus_handle(cx);
        window.focus(&request, cx);
        press(sheet, "tab", window, cx);
        write(sheet, first, window, cx);
        let row = sheet.face.chips.iter().find(|chip| chip.key == Some(3)).expect("a keyed row").id();
        sheet.face.pick(&row);
    }

    #[gpui::test]
    fn with_no_project_the_feature_blank_lists_his_projects_and_taking_one_chooses_it_and_asks_for_its_features(cx: &mut TestAppContext) {
        let (sheet, cx) = open(crate::dispatch::scenes::rest(&[]), None, cx);
        let asked = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let heard = std::rc::Rc::clone(&asked);
        let _sub = cx.update(|_, cx| {
            cx.subscribe(&sheet, move |_, asked: &Asked, _| {
                if let Asked::ReadFeatures(project) = asked {
                    heard.borrow_mut().push(project.clone());
                }
            })
        });
        sheet.update_in(cx, |sheet, window, cx| {
            take_bug(sheet, window, cx);
            assert_eq!(sheet.project(), None, "no project is chosen");
            press(sheet, "tab", window, cx);
            let story = sheet.story.as_ref().expect("a sentence stands");
            assert!(matches!(story.piece(), super::super::story_kinds::Piece::Feature(_)), "the feature blank has the keys");
            let rows = story.list(&sheet.features()).expect("the keys are on a feature blank");
            let titles: Vec<_> = rows.iter().map(|row| row.title.to_string()).collect();
            assert!(titles.contains(&"Storefront".to_string()), "his projects are the choices: {titles:?}");
            assert!(rows.iter().all(|row| row.project.is_some()), "and nothing but projects");

            write(sheet, "store", window, cx);
            press(sheet, "enter", window, cx);
            assert_eq!(sheet.project(), Some("storefront".to_string()), "taking the row chose the project, as its chip would");
            let story = sheet.story.as_ref().expect("a sentence stands");
            assert_eq!(blank_words(sheet)[2], "", "the project is no feature: the blank is left to list its features");
            assert!(matches!(story.piece(), super::super::story_kinds::Piece::Feature(_)), "and keeps the keys");
        });
        assert_eq!(*asked.borrow(), ["storefront"], "his projects are known here, so only the project's features are asked of Fleet");
    }

    #[gpui::test]
    fn a_project_row_pressed_chooses_the_project_the_same_way(cx: &mut TestAppContext) {
        let (sheet, cx) = open(crate::dispatch::scenes::rest(&[]), None, cx);
        sheet.update_in(cx, |sheet, window, cx| {
            take_bug(sheet, window, cx);
            press(sheet, "tab", window, cx);
            sheet.story_row(0, window, cx);
            assert!(sheet.project().is_some(), "the first project is chosen");
            assert_eq!(blank_words(sheet)[2], "");
            assert_eq!(sheet.features().len(), 0, "its features are not in yet");
        });
    }

    #[gpui::test]
    fn start_carries_the_workflow_of_the_kind_and_none_for_plain_words(cx: &mut TestAppContext) {
        let (sheet, cx) = open(crate::dispatch::scenes::rest(&[]), None, cx);
        let (heard, _sub) = starts_heard(&sheet, cx);
        sheet.update_in(cx, |sheet, window, cx| {
            start_sentence(sheet, "When", "click Pay", window, cx);
            sheet.act(window, cx);
            sheet.sending = false;
            sheet.plain_words(window, cx);
            start_sentence(sheet, "What", "Armada sang", window, cx);
            assert_eq!(sheet.story.as_ref().expect("a sentence").kind().id, "idea");
            sheet.act(window, cx);
            sheet.sending = false;
            sheet.plain_words(window, cx);
            sheet.act(window, cx);
        });
        let heard = heard.borrow();
        assert_eq!(heard.iter().map(|(_, workflow)| workflow.clone()).collect::<Vec<_>>(), [Some("bug".to_string()), Some(String::new()), None]);
    }

    #[gpui::test]
    fn a_feature_of_the_library_adds_its_page_as_the_last_line_and_typed_words_add_none(cx: &mut TestAppContext) {
        let (sheet, cx) = open(crate::dispatch::scenes::rest(&[]), None, cx);
        let (heard, _sub) = starts_heard(&sheet, cx);
        sheet.update_in(cx, |sheet, window, cx| {
            start_sentence(sheet, "When", "click Pay", window, cx);
            let one = |title: &str, slug: &str| FeatureChoice { title: title.to_string().into(), area: "Payments".into(), slug: slug.to_string().into(), kind: Kind::Feature };
            sheet.catalog = Some(("storefront".to_string(), vec![one("Checkout", "checkout-flow"), one("Billing", "billing")]));
            sheet.story.as_mut().expect("a sentence").values[2] = "Checkout".into();
            sheet.act(window, cx);
            sheet.sending = false;
            sheet.story.as_mut().expect("a sentence").values[2] = "the pay button".into();
            sheet.act(window, cx);
        });
        let heard = heard.borrow();
        assert_eq!(heard[0].0, "When I click Pay in Checkout\nFeature: checkout-flow");
        assert_eq!(heard[1].0, "When I click Pay in the pay button", "words typed into the blank name no feature");
    }
}
