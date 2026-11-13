//! A workflow's lanes with a draft over them: the canvas, the edits made on it kept as a draft with ⌘Z and a history
//! that undoes each one, and Publish, which writes the draft where it applies. One piece, mounted wherever lanes
//! are edited: Settings › Workflows today, a Job's Workflow tab and Now next.
//!
//! **Mounting it.** The host builds `cx.new(|cx| LanesEditor::new(subject, wire, window, cx))` with a [`Subject`]
//! (the workflow, the level its lanes are read at, where its definition is kept) and an [`EditorWire`] it answers
//! Fleet's reads and saves with, and draws the entity as a child. It adds [`LanesEditor::stops`] to its Tab stops
//! before the canvas, passes keys it did not take to [`LanesEditor::key`] while no text field holds them, and
//! subscribes to [`EditorEvent`]. An agent's change comes in through [`LanesEditor::agent_edit`]; a person's edit
//! through [`LanesEditor::edit`], or on the canvas itself: a picked step opens its fields in place, and a + in a gap
//! adds a step there (`in_place`). While [`LanesEditor::typing`], the host passes its keys to
//! [`LanesEditor::field_key`] first, and counts the fields as text.
//!
//! **Nothing is written until Publish.** An edit lands in the [`Draft`]; the canvas shows the draft's lanes, with
//! what differs marked as a changed node is.

mod bar;
mod draft;
mod in_place;
mod patch;
mod wire;

use std::cell::Cell;
use std::rc::Rc;

use gpui::{div, prelude::*, px, AnyElement, Bounds, Context, EventEmitter, FocusHandle, Keystroke, Pixels, Role, SharedString, Subscription, Task, Window};
use ipc::{JobLanesView, SaveWorkflow, WorkflowScope};

pub use draft::{Author, Change, Draft, Edit, Target};
pub use wire::{EditorWire, Refused, Subject};

use crate::labels::labelled;
use crate::lanes_canvas::{lanes_canvas, LanesHost, LanesState, Pick};
use crate::look::{token, ActiveLook};
use crate::settings::view::bone;

/// What the editor tells the view that holds it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EditorEvent {
    /// The draft was written at `target`, and the lanes read again.
    Published { target: Target },
    /// An Esc the canvas did not take, with nothing picked: the host shuts what holds it.
    Escaped,
}

impl EventEmitter<EditorEvent> for LanesEditor {}

/// Fleet's word where what a draft was made from is not what is saved now.
const CHANGED_SINCE: &str = "This workflow changed since you started. Discard and edit again.";

enum Read {
    Loading,
    Refused(String),
    Read(JobLanesView),
}

/// The editor's stops: the buttons on the draft bar and the two answers to where it applies.
struct Keys {
    changes: FocusHandle,
    discard: FocusHandle,
    publish: FocusHandle,
    repository: FocusHandle,
    every: FocusHandle,
}

/// A press the bar's keys answer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Press {
    Changes,
    Discard,
    Publish,
    Repository,
    Every,
    Undo(usize),
}

pub struct LanesEditor {
    wire: Rc<dyn EditorWire>,
    subject: Subject,
    id: SharedString,
    read: Read,
    /// The definition as last read: what the draft is made from, and what Publish checks is still saved.
    saved: Option<String>,
    draft: Draft,
    /// The read lanes with the draft laid over them, while it has edits.
    shown: Option<JobLanesView>,
    state: LanesState,
    history_open: bool,
    /// What Publish was refused with, said on the bar.
    refused: Option<String>,
    busy: bool,
    keys: Keys,
    /// One stop for each edit's Undo.
    undos: Vec<FocusHandle>,
    loading: Option<Task<()>>,
    publishing: Option<Task<()>>,
    /// What the canvas has picked, as it last said.
    picked: Option<Pick>,
    /// The pick was the canvas's last act, so Enter on it opens its fields rather than letting it go.
    armed: bool,
    /// The node opened where it stands.
    in_place: Option<in_place::InPlace>,
    /// Where the canvas's frame was drawn last, in window points.
    frame: Rc<Cell<Bounds<Pixels>>>,
    /// The part of that frame the page let be seen: the fields stand inside it.
    seen: Rc<Cell<Bounds<Pixels>>>,
    /// The page the editor scrolls in, from its host: fields that open are scrolled into view in it.
    page: Option<gpui::ScrollHandle>,
    /// Fields opened since the page was last scrolled to them.
    reveal: Cell<bool>,
    /// The lifecycles that apply where the lanes are read, as last read: a state is renamed in the one that runs.
    lifecycles: Vec<ipc::AppliedLifecycle>,
    /// The definition a Publish wrote before a lifecycle it then saved was refused: tried again, it is not written
    /// twice.
    written: Option<String>,
    _subs: Vec<Subscription>,
}

impl LanesEditor {
    pub fn new(subject: Subject, wire: Rc<dyn EditorWire>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let state = LanesState::new(cx);
        // The node or chip the keys are on wears the ring while the canvas holds them.
        let subs = vec![
            cx.on_focus(&state.frame, window, |this, _, cx| {
                this.state.keyed.set(true);
                cx.notify();
            }),
            cx.on_blur(&state.frame, window, |this, _, cx| {
                this.state.keyed.set(false);
                cx.notify();
            }),
        ];
        let stop = |cx: &mut Context<Self>| cx.focus_handle().tab_stop(true);
        let keys = Keys { changes: stop(cx), discard: stop(cx), publish: stop(cx), repository: stop(cx), every: stop(cx) };
        let id: SharedString = format!("{}|{}|{}", subject.workflow, subject.level, subject.manifest).into();
        let draft = Draft::new(String::new(), subject.own_target());
        let mut editor = LanesEditor { wire, subject, id, read: Read::Loading, saved: None, draft, shown: None, state, history_open: false, refused: None, busy: false, keys, undos: Vec::new(), loading: None, publishing: None, picked: None, armed: false, in_place: None, frame: Rc::default(), seen: Rc::default(), page: None, reveal: Cell::new(false), lifecycles: Vec::new(), written: None, _subs: subs };
        editor.load(cx);
        editor
    }

    #[allow(dead_code)] // The surface a Job's Workflow tab mounts it through.
    pub fn subject(&self) -> &Subject {
        &self.subject
    }

    #[allow(dead_code)] // The surface a Job's Workflow tab mounts it through.
    pub fn draft(&self) -> &Draft {
        &self.draft
    }

    pub fn has_edits(&self) -> bool {
        !self.draft.is_empty()
    }

    /// The lanes and the definition read again, the draft kept as it is.
    pub fn reload(&mut self, cx: &mut Context<Self>) {
        self.load(cx);
    }

    fn load(&mut self, cx: &mut Context<Self>) {
        let (wire, subject) = (self.wire.clone(), self.subject.clone());
        self.loading = Some(cx.spawn(async move |this, cx| {
            let (lanes, definition, lifecycles) = cx.update(|app| (wire.lanes(&subject, app), wire.definition(&subject, app), wire.lifecycles(&subject.repository, app)));
            let (lanes, definition, lifecycles) = (lanes.await, definition.await, lifecycles.await);
            let _ = this.update(cx, |this, cx| {
                // Without them a state is read and not renamed.
                this.lifecycles = lifecycles.map(|read| read.lifecycles).unwrap_or_default();
                this.loaded(lanes, definition.map(|read| read.definition).ok(), cx)
            });
        }));
    }

    fn loaded(&mut self, lanes: Result<JobLanesView, Refused>, definition: Option<String>, cx: &mut Context<Self>) {
        self.read = match lanes {
            Ok(lanes) => Read::Read(lanes),
            Err(why) => Read::Refused(why.said),
        };
        if let Some(text) = definition {
            if self.draft.is_empty() {
                self.draft.clear(text.clone());
            }
            self.saved = Some(text);
        }
        self.refresh(cx);
        cx.notify();
    }

    /// The lanes the canvas draws: the read ones with the draft's steps and states over them while there are edits,
    /// and room after the workflow's last step.
    fn refresh(&mut self, cx: &mut Context<Self>) {
        self.shown = match &self.read {
            Read::Read(lanes) if !self.draft.is_empty() && !self.draft.base().is_empty() => {
                let steps = patch::patched(lanes, self.draft.base(), &self.draft.drafted());
                Some(in_place::with_end_room(patch::with_states_renamed(steps, &self.draft.state_labels())))
            }
            Read::Read(lanes) => Some(in_place::with_end_room(lanes.clone())),
            _ => None,
        };
        while self.undos.len() < self.draft.edits().len() {
            self.undos.push(cx.focus_handle().tab_stop(true));
        }
        self.undos.truncate(self.draft.edits().len());
        if self.draft.is_empty() {
            self.history_open = false;
            self.draft.set_target(self.subject.own_target());
        }
        // Fields he is not in are opened again on what the draft says now.
        if !self.in_place.as_ref().is_some_and(|one| one.keyed) {
            self.in_place = None;
        }
    }

    // ---- the draft ----------------------------------------------------------------------------

    /// One edit, on the draft, with the line the history reads for it. Nothing is written.
    pub fn edit(&mut self, author: Author, said: impl Into<String>, change: Change, cx: &mut Context<Self>) {
        if self.draft.base().is_empty() {
            // An agent's change names the text it was drafted from; any other edit waits on the read.
            let base = match &change {
                Change::Definition { before, .. } => Some(before.clone()),
                _ => self.saved.clone(),
            };
            let Some(base) = base else { return };
            self.draft.clear(base);
        }
        self.draft.push(Edit { author, said: said.into(), change });
        self.refused = None;
        self.refresh(cx);
        cx.notify();
    }

    /// An agent's change, drafted from `before` as `after`, for what the person asked.
    pub fn agent_edit(&mut self, asked: impl Into<String>, before: String, after: String, cx: &mut Context<Self>) {
        self.edit(Author::Agent, asked, Change::Definition { before, after }, cx);
    }

    /// Where the draft applies, as the person answered the question.
    pub fn apply_at(&mut self, target: Target, cx: &mut Context<Self>) {
        self.draft.set_target(Some(target));
        cx.notify();
    }

    /// ⌘Z: the last edit dropped. `false` where there is none.
    pub fn undo_last(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let at = self.draft.edits().len();
        at > 0 && self.undo_at(at - 1, window, cx)
    }

    /// One edit dropped, wherever it stands: the rest replay without it.
    pub fn undo_at(&mut self, at: usize, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let was_here = self.owns_focus(window);
        if self.draft.undo_at(at).is_none() {
            return false;
        }
        self.refused = None;
        self.refresh(cx);
        // The control that was pressed goes with its row: the keys go to the history's head, or to the canvas once
        // nothing is left of the bar.
        if was_here {
            window.focus(if self.draft.is_empty() { &self.state.frame } else { &self.keys.changes }, cx);
        }
        cx.notify();
        true
    }

    /// Every edit dropped, back to what is saved.
    pub fn discard(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let was_here = self.owns_focus(window);
        let base = self.saved.clone().unwrap_or_else(|| self.draft.base().to_string());
        self.draft.clear(base);
        self.in_place = None;
        self.written = None;
        self.draft.set_target(self.subject.own_target());
        self.refused = None;
        self.refresh(cx);
        if was_here {
            window.focus(&self.state.frame, cx);
        }
        cx.notify();
    }

    /// Writes the draft where it applies, if what it was made from is still what is saved: a copy at the place the
    /// person chose for a workflow that is Armada's, or the copy itself where it is one. Then each lifecycle it
    /// renamed a state of, at the same place. A refusal keeps the draft.
    pub fn publish(&mut self, cx: &mut Context<Self>) {
        let Some(target) = self.draft.target().cloned().filter(|_| !self.busy && !self.draft.is_empty()) else { return };
        self.busy = true;
        self.refused = None;
        cx.notify();
        let (wire, subject) = (self.wire.clone(), self.subject.clone());
        let (base, drafted) = (self.draft.base().to_string(), self.draft.drafted());
        let pieces = self.pieces(&target);
        let written = self.written.clone();
        self.publishing = Some(cx.spawn(async move |this, cx| {
            let mut wrote = None;
            let answer: Result<(), Refused> = async {
                // A definition no edit changed is not written: the draft renamed states only.
                if drafted != base && written.as_ref() != Some(&drafted) {
                    let now = cx.update(|app| wire.definition(&subject, app)).await?;
                    if now.definition != base {
                        return Err(Refused::said(CHANGED_SINCE));
                    }
                    let (scope, manifest) = match &target {
                        Target::Repository(manifest) => (WorkflowScope::Repository, manifest.clone()),
                        Target::EveryProject => (WorkflowScope::Kit, subject.repository.clone()),
                    };
                    // A copy is new where the workflow is Armada's: a copy that is already there is not replaced unasked.
                    let save = SaveWorkflow { scope, definition: drafted.clone(), overwrite: !subject.shipped };
                    cx.update(|app| wire.save_workflow(&manifest, save, app)).await?;
                    wrote = Some(drafted.clone());
                }
                for save in pieces {
                    cx.update(|app| wire.save_piece(save, app)).await?;
                }
                Ok(())
            }
            .await;
            let _ = this.update(cx, |this, cx| this.published(target, answer, wrote, cx));
        }));
    }

    /// Each lifecycle the draft renamed a state of, as it is saved at `target`: the one that runs, renamed, scoped
    /// there, and replacing the one already kept at that scope.
    fn pieces(&self, target: &Target) -> Vec<ipc::SaveLifecyclePiece> {
        let scope = match target {
            Target::Repository(id) => ipc::PieceScope::Within { id: id.clone() },
            Target::EveryProject => ipc::PieceScope::Everywhere,
        };
        let running: Vec<ipc::LifecycleDefinition> = self.lifecycles.iter().filter(|one| one.kept.overridden_by.is_none()).map(|one| one.lifecycle.clone()).collect();
        self.draft
            .renamed_lifecycles(&running)
            .into_iter()
            .map(|mut lifecycle| {
                let overwrite = self.lifecycles.iter().any(|one| one.lifecycle.id == lifecycle.id && one.lifecycle.scope == scope);
                lifecycle.scope = scope.clone();
                ipc::SaveLifecyclePiece { piece: ipc::LifecyclePiece::Lifecycle { lifecycle }, overwrite }
            })
            .collect()
    }

    fn published(&mut self, target: Target, answer: Result<(), Refused>, wrote: Option<String>, cx: &mut Context<Self>) {
        self.busy = false;
        match answer {
            Err(why) => {
                self.refused = Some(why.said);
                self.written = wrote.or(self.written.take());
            }
            Ok(()) => {
                self.draft = Draft::new(String::new(), self.subject.own_target());
                self.written = None;
                self.history_open = false;
                self.shown = None;
                self.in_place = None;
                self.undos.clear();
                self.load(cx);
                cx.emit(EditorEvent::Published { target });
            }
        }
        cx.notify();
    }

    // ---- keys ---------------------------------------------------------------------------------

    /// Whether the bar asks where the draft applies: a workflow that is Armada's, once it has a change.
    fn question_shown(&self) -> bool {
        self.subject.shipped && !self.draft.is_empty()
    }

    /// The editor's Tab stops, in the order they are drawn: the answers to where it applies, the bar, the history's
    /// Undo for each change, then the canvas.
    pub fn stops(&self) -> Vec<FocusHandle> {
        let mut stops = Vec::new();
        if !self.draft.is_empty() {
            if self.question_shown() {
                stops.extend([self.keys.repository.clone(), self.keys.every.clone()]);
            }
            stops.extend([self.keys.changes.clone(), self.keys.discard.clone(), self.keys.publish.clone()]);
            if self.history_open {
                stops.extend(self.undos.iter().cloned());
            }
        }
        if matches!(self.read, Read::Read(_)) {
            stops.push(self.state.frame.clone());
        }
        stops
    }

    /// The keys to the canvas, where the bar they were on has gone.
    pub fn focus_canvas(&self, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.state.frame, cx);
    }

    /// Whether the canvas holds the keys, so it takes the arrows, Enter, Space and Esc itself.
    pub fn canvas_focused(&self, window: &Window) -> bool {
        self.state.frame.is_focused(window)
    }

    /// Whether the keys are on the bar or the history, and not on the canvas.
    fn owns_focus(&self, window: &Window) -> bool {
        let canvas = self.state.frame.clone();
        self.stops().iter().any(|stop| *stop != canvas && stop.is_focused(window))
    }

    fn pressed(&mut self, press: Press, window: &mut Window, cx: &mut Context<Self>) {
        match press {
            Press::Changes => {
                self.history_open = !self.history_open && !self.draft.is_empty();
                cx.notify();
            }
            Press::Discard => self.discard(window, cx),
            Press::Publish => self.publish(cx),
            Press::Repository => {
                self.apply_at(Target::Repository(self.subject.repository.clone()), cx);
                window.focus(&self.keys.changes, cx);
            }
            Press::Every => {
                self.apply_at(Target::EveryProject, cx);
                window.focus(&self.keys.changes, cx);
            }
            Press::Undo(at) => {
                self.undo_at(at, window, cx);
            }
        }
    }

    fn focused_press(&self, window: &Window) -> Option<Press> {
        let keys = &self.keys;
        [(&keys.repository, Press::Repository), (&keys.every, Press::Every), (&keys.changes, Press::Changes), (&keys.discard, Press::Discard), (&keys.publish, Press::Publish)]
            .into_iter()
            .find(|(stop, _)| stop.is_focused(window))
            .map(|(_, press)| press)
            .or_else(|| self.history_open.then(|| self.undos.iter().position(|stop| stop.is_focused(window)).map(Press::Undo)).flatten())
    }

    /// The editor's keys, for a host that gets them first: ⌘Z drops the last change; Enter and Space press what has
    /// the keys; Esc shuts the history; on the canvas, E opens the picked step's fields and A adds a step after it
    /// (Classic's `E` and `A`). `true` where one was taken. A host calls it only while no text field has them.
    pub fn key(&mut self, key: &Keystroke, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let mods = &key.modifiers;
        let word = key.key.as_str();
        if word == "z" && mods.platform && !mods.shift && !mods.control && !mods.alt {
            return self.undo_last(window, cx);
        }
        if mods.platform || mods.control || mods.alt {
            return false;
        }
        match word {
            "escape" if self.history_open && self.owns_focus(window) => {
                self.history_open = false;
                window.focus(&self.keys.changes, cx);
                cx.notify();
                true
            }
            "e" if !mods.shift && self.canvas_focused(window) => self.open_picked(window, cx),
            "a" if !mods.shift && self.canvas_focused(window) => self.add_after_picked(window, cx),
            "enter" | "space" => match self.focused_press(window) {
                Some(press) => {
                    self.pressed(press, window, cx);
                    true
                }
                None => false,
            },
            _ => false,
        }
    }
}

impl LanesHost for LanesEditor {
    fn lanes_state(&self) -> &LanesState {
        &self.state
    }

    fn lanes_of<'a>(&'a self, _id: &SharedString, _cx: &'a gpui::App) -> Option<&'a JobLanesView> {
        self.shown.as_ref().or(match &self.read {
            Read::Read(lanes) => Some(lanes),
            _ => None,
        })
    }

    /// A pick opens what was picked where it stands; letting go closes it. What was typed lands first.
    fn lanes_picked(&mut self, _id: &SharedString, pick: Option<Pick>, window: &mut Window, cx: &mut Context<Self>) {
        self.picked = pick.clone();
        self.armed = true;
        match pick {
            Some(Pick::Node(node)) => self.open_at(&node, false, window, cx),
            _ => {
                self.settle(window, cx);
                self.in_place = None;
            }
        }
    }
}

impl gpui::Render for LanesEditor {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let look = cx.look().clone();
        let body: AnyElement = match &self.read {
            Read::Loading => bone(&look, px(0.), look.length(token::SPACE_12) * 11.).w_full().into_any_element(),
            Read::Refused(said) => labelled(crate::settings::view::small(&look, said.clone(), look.color(token::STATUS_COMPLETED_FAILED)).id("workflow-lanes-refused"), Role::Alert, said.clone()).into_any_element(),
            Read::Read(read) => {
                let lanes = self.shown.as_ref().unwrap_or(read);
                // Four lanes of the least height fill it; a lane more makes it taller by that much.
                let tall = (look.length(token::SPACE_12) * 11.).max(px(lanes.lanes.len() as f32 * crate::lanes_canvas::LANE_MIN) + look.length(token::BORDER_WIDTH) * 2.);
                // An Esc the lanes did not take, with nothing picked, is the host's: it shuts what holds the editor.
                let shut = cx.listener(|this, event: &gpui::KeyDownEvent, window, cx| {
                    if event.keystroke.key == "escape" && this.canvas_focused(window) {
                        cx.emit(EditorEvent::Escaped);
                        cx.stop_propagation();
                    }
                });
                // Enter on the step just picked goes to its fields before the canvas lets the pick go.
                let fields = cx.listener(|this, event: &gpui::KeyDownEvent, window, cx| {
                    if this.canvas_focused(window) && this.canvas_key(&event.keystroke, window, cx) {
                        cx.stop_propagation();
                    }
                });
                let canvas = lanes_canvas(self, &self.id, lanes, &look, cx);
                // After the canvas has laid out this frame's nodes, so a step just added has its place.
                self.fit_words(&look, cx);
                let reveal = if self.reveal.get() { self.reveal_target(&look) } else { None };
                if reveal.is_some() {
                    self.reveal.set(false);
                }
                let watch = self.in_place.is_some();
                let over = self.render_in_place(self.frame.get(), &look, cx);
                div()
                    .relative()
                    .flex_none()
                    .h(tall)
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .capture_key_down(fields)
                    .on_key_down(shut)
                    .child(in_place::measure(self.frame.clone(), self.seen.clone(), reveal.zip(self.page.clone()), watch))
                    .child(canvas)
                    .children(over)
                    .into_any_element()
            }
        };
        div().flex().flex_col().gap(look.length(token::SPACE_3)).min_w_0().children(self.render_bar(&look, window, cx)).child(body)
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use gpui::{Entity, Focusable, TestAppContext, VisualTestContext};
    use ipc::{WorkflowDefinition, WorkflowId, WorkflowSaved};

    use super::in_place::NEW_STEP;
    use super::*;

    const BASE: &str = r#"{"version":1,"workflow_id":"prototype","name":"prototype","steps":[{"id":"plan","label":"Plan"},{"id":"build","label":"Build"},{"id":"land","label":"Land"}]}"#;
    const REVIEWED: &str = r#"{"version":1,"workflow_id":"prototype","name":"prototype","steps":[{"id":"plan","label":"Plan"},{"id":"build","label":"Build"},{"id":"review","label":"Review"},{"id":"land","label":"Land"}]}"#;

    /// What Fleet holds for the editor, and what it was asked to save, in the order it was asked.
    struct Fleet {
        definition: RefCell<String>,
        saved: RefCell<Vec<(String, SaveWorkflow)>>,
        pieces: RefCell<Vec<ipc::SaveLifecyclePiece>>,
        asked: RefCell<Vec<&'static str>>,
        /// Fleet's sentence for a piece save it refuses.
        refuses_pieces: RefCell<Option<String>>,
    }

    /// The Ticket as Armada carries it, and a copy of it Kit keeps for every repository.
    fn applied() -> Vec<ipc::AppliedLifecycle> {
        let state = |id: &str, label: &str| ipc::LifecycleStateDefinition { id: id.into(), label: label.into(), read: ipc::StateReadFrom::Tracker { words: vec![label.into()] } };
        let ticket = |scope: ipc::PieceScope, source: &str, over: bool| ipc::AppliedLifecycle {
            lifecycle: ipc::LifecycleDefinition { id: "ticket".into(), name: "Ticket".into(), scope, states: vec![state("todo", "Todo"), state("in_progress", "In Progress"), state("done", "Done")], transitions: Vec::new(), ends: vec!["done".into()] },
            kept: ipc::PieceKept {
                rank: ipc::PieceRank { reach: "everywhere".into(), source: source.into(), home: false },
                path: None,
                overridden_by: over.then(|| ipc::PieceRank { reach: "everywhere".into(), source: "kit".into(), home: false }),
                hides: None,
            },
        };
        vec![ticket(ipc::PieceScope::Everywhere, "armada", true), ticket(ipc::PieceScope::Everywhere, "kit", false)]
    }

    struct FakeWire(Rc<Fleet>);

    impl EditorWire for FakeWire {
        fn lanes(&self, _: &Subject, _: &mut gpui::App) -> Task<Result<JobLanesView, Refused>> {
            Task::ready(ipc::decode("JobLanesView", include_bytes!("../settings/mock-lanes.json")).map_err(|why| Refused::said(why.to_string())))
        }

        fn definition(&self, _: &Subject, _: &mut gpui::App) -> Task<Result<ipc::WorkflowDefinition, Refused>> {
            Task::ready(Ok(WorkflowDefinition { workflow_id: WorkflowId::carried("prototype"), source: "armada".into(), file: "prototype.yml".into(), definition: self.0.definition.borrow().clone(), overridden_by: None }))
        }

        fn save_workflow(&self, manifest: &str, save: SaveWorkflow, _: &mut gpui::App) -> Task<Result<ipc::WorkflowSaved, Refused>> {
            let scope = save.scope;
            self.0.asked.borrow_mut().push("definition");
            self.0.saved.borrow_mut().push((manifest.to_string(), save));
            Task::ready(Ok(WorkflowSaved { workflow_id: WorkflowId::carried("prototype"), scope, file: "prototype.yml".into(), replaced: false, runs_from: None }))
        }

        fn lifecycles(&self, manifest: &str, _: &mut gpui::App) -> Task<Result<ipc::RepositoryLifecycles, Refused>> {
            Task::ready(Ok(ipc::RepositoryLifecycles { manifest_id: ipc::ManifestId::carried(manifest), lifecycles: applied(), left_out: Vec::new() }))
        }

        fn save_piece(&self, save: ipc::SaveLifecyclePiece, _: &mut gpui::App) -> Task<Result<ipc::LifecyclePieceSaved, Refused>> {
            self.0.asked.borrow_mut().push("piece");
            if let Some(said) = self.0.refuses_pieces.borrow().clone() {
                return Task::ready(Err(Refused::said(said)));
            }
            let ipc::LifecyclePiece::Lifecycle { lifecycle } = &save.piece else { return Task::ready(Err(Refused::said("a lifecycle"))) };
            let saved = ipc::LifecyclePieceSaved { kind: ipc::LifecyclePieceKind::Lifecycle, id: lifecycle.id.clone(), scope: lifecycle.scope.clone(), file: "ticket.yml".into(), replaced: save.overwrite };
            self.0.pieces.borrow_mut().push(save);
            Task::ready(Ok(saved))
        }

        fn delete_piece(&self, _: ipc::DeleteLifecyclePiece, _: &mut gpui::App) -> Task<Result<ipc::LifecyclePieceDeleted, Refused>> {
            Task::ready(Err(Refused::said("not here")))
        }
    }

    fn open(cx: &mut TestAppContext, shipped: bool) -> (Entity<LanesEditor>, Rc<Fleet>, Rc<RefCell<Vec<EditorEvent>>>, &mut VisualTestContext) {
        cx.update(|cx| {
            gpui_component::init(cx);
            crate::look::init(cx);
        });
        let fleet = Rc::new(Fleet { definition: RefCell::new(BASE.into()), saved: RefCell::default(), pieces: RefCell::default(), asked: RefCell::default(), refuses_pieces: RefCell::default() });
        let subject = Subject { workflow: "prototype".into(), level: "default".into(), manifest: "armada".into(), held_under: "armada".into(), source: "armada", shipped, repository: "armada".into() };
        let wire: Rc<dyn EditorWire> = Rc::new(FakeWire(fleet.clone()));
        let (editor, cx) = cx.add_window_view(|window, cx| LanesEditor::new(subject, wire, window, cx));
        let seen = Rc::new(RefCell::new(Vec::new()));
        let heard = seen.clone();
        cx.update(|_, cx| cx.subscribe(&editor, move |_, event: &EditorEvent, _| heard.borrow_mut().push(event.clone())).detach());
        cx.run_until_parked();
        (editor, fleet, seen, cx)
    }

    fn drawn(editor: &Entity<LanesEditor>, cx: &mut VisualTestContext, node: &str) -> Option<(u32, Option<String>)> {
        editor.read_with(cx, |editor, cx| editor.lanes_of(&"".into(), cx).and_then(|lanes| lanes.lanes.iter().flat_map(|lane| lane.nodes.iter()).find(|one| one.id == node).map(|one| (one.col, one.changed.clone()))))
    }

    #[gpui::test]
    fn an_agents_change_is_drafted_on_the_canvas_and_command_z_takes_it_back(cx: &mut TestAppContext) {
        let (editor, fleet, _, cx) = open(cx, true);
        assert_eq!(drawn(&editor, cx, "workflow/review"), None);
        editor.update(cx, |editor, cx| editor.agent_edit("Add a Review step after Build", BASE.into(), REVIEWED.into(), cx));
        assert_eq!(drawn(&editor, cx, "workflow/review"), Some((2, Some("Added".into()))));
        editor.read_with(cx, |editor, _| {
            assert_eq!(editor.draft().edits().iter().map(|edit| (edit.author, edit.said.as_str())).collect::<Vec<_>>(), [(Author::Agent, "Add a Review step after Build")]);
            assert_eq!(editor.draft().target(), None, "a shipped workflow asks where it applies");
        });
        assert!(cx.update(|window, cx| editor.update(cx, |editor, cx| editor.undo_last(window, cx))));
        assert_eq!(drawn(&editor, cx, "workflow/review"), None);
        assert!(!editor.read_with(cx, |editor, _| editor.has_edits()));
        assert!(!cx.update(|window, cx| editor.update(cx, |editor, cx| editor.undo_last(window, cx))), "nothing left to drop");
        assert!(fleet.saved.borrow().is_empty(), "nothing is written before Publish");
    }

    #[gpui::test]
    fn a_change_in_the_middle_is_undone_by_its_own_row(cx: &mut TestAppContext) {
        let (editor, _, _, cx) = open(cx, false);
        editor.update(cx, |editor, cx| {
            editor.edit(Author::Person, "Rename Build", Change::StepLabel { step: "build".into(), label: "Make".into() }, cx);
            editor.edit(Author::Person, "Words for Plan", Change::StepWords { step: "plan".into(), words: "Plan small.".into() }, cx);
            editor.edit(Author::Person, "Rename Land", Change::StepLabel { step: "land".into(), label: "Ship".into() }, cx);
        });
        assert!(cx.update(|window, cx| editor.update(cx, |editor, cx| editor.undo_at(1, window, cx))));
        editor.read_with(cx, |editor, _| {
            let said: Vec<&str> = editor.draft().edits().iter().map(|edit| edit.said.as_str()).collect();
            assert_eq!(said, ["Rename Build", "Rename Land"]);
            let steps = crate::settings::definition::steps_of(&editor.draft().drafted());
            assert_eq!(steps.iter().map(|(_, label)| label.as_str()).collect::<Vec<_>>(), ["Plan", "Make", "Ship"]);
        });
        assert_eq!(drawn(&editor, cx, "workflow/build"), Some((1, Some("Renamed from Build".into()))));
    }

    /// What is typed into the fields open in place.
    fn type_in(editor: &Entity<LanesEditor>, cx: &mut VisualTestContext, name: Option<&str>, words: Option<&str>) {
        cx.update(|window, cx| {
            let (name_box, words_box) = editor.read_with(cx, |editor, _| {
                let fields = editor.in_place.as_ref().and_then(|one| one.fields.as_ref()).expect("fields open in place");
                (fields.name.clone(), fields.words.clone())
            });
            if let Some(name) = name {
                name_box.update(cx, |state, cx| state.set_value(name.to_string(), window, cx));
            }
            if let Some(words) = words {
                words_box.expect("a step's Words").update(cx, |state, cx| state.set_value(words.to_string(), window, cx));
            }
        });
    }

    fn history(editor: &Entity<LanesEditor>, cx: &mut VisualTestContext) -> Vec<(Author, String)> {
        editor.read_with(cx, |editor, _| editor.draft().edits().iter().map(|edit| (edit.author, edit.said.clone())).collect())
    }

    fn key(editor: &Entity<LanesEditor>, cx: &mut VisualTestContext, key: &'static str) -> bool {
        let stroke = Keystroke::parse(key).expect("a key");
        cx.update(|window, cx| editor.update(cx, |editor, cx| editor.field_key(&stroke, window, cx)))
    }

    #[gpui::test]
    fn each_field_he_changed_lands_as_one_edit_of_his_and_enter_gives_the_keys_back(cx: &mut TestAppContext) {
        let (editor, fleet, _, cx) = open(cx, false);
        cx.update(|window, cx| editor.update(cx, |editor, cx| editor.open_at("workflow/build", true, window, cx)));
        assert!(cx.update(|window, cx| editor.read(cx).typing(window, cx)), "the keys are in its Name");
        type_in(&editor, cx, Some("Make"), None);
        assert!(key(&editor, cx, "tab"));
        assert!(cx.update(|window, cx| editor.read(cx).in_place.as_ref().and_then(|one| one.fields.as_ref()).is_some_and(|fields| fields.words.as_ref().is_some_and(|words| words.read(cx).focus_handle(cx).is_focused(window)))), "Tab goes to its Words");
        type_in(&editor, cx, None, Some("Build it small."));
        assert!(key(&editor, cx, "enter"));
        cx.run_until_parked();
        assert_eq!(history(&editor, cx), [(Author::Person, "Renamed Build to Make".to_string()), (Author::Person, "Changed the words of Make".to_string())]);
        editor.read_with(cx, |editor, _| {
            let text = editor.draft().drafted();
            assert_eq!(crate::settings::definition::steps_of(&text)[1], ("build".to_string(), "Make".to_string()));
            assert_eq!(crate::settings::definition::words_of(&text, "build").as_deref(), Some("Build it small."));
            assert!(editor.in_place.is_none());
        });
        assert!(cx.update(|window, cx| editor.read(cx).canvas_focused(window)), "the keys are back on the canvas");
        assert_eq!(drawn(&editor, cx, "workflow/build"), Some((1, Some("Renamed from Build".into()))));
        assert!(fleet.saved.borrow().is_empty(), "nothing is written before Publish");
    }

    #[gpui::test]
    fn a_field_left_as_it_was_makes_no_edit_and_esc_drops_what_was_typed(cx: &mut TestAppContext) {
        let (editor, _, _, cx) = open(cx, false);
        cx.update(|window, cx| editor.update(cx, |editor, cx| editor.open_at("workflow/land", true, window, cx)));
        type_in(&editor, cx, None, Some("Land it."));
        assert!(key(&editor, cx, "enter"));
        assert_eq!(history(&editor, cx), [(Author::Person, "Changed the words of Land".to_string())]);
        cx.update(|window, cx| editor.update(cx, |editor, cx| editor.open_at("workflow/land", true, window, cx)));
        type_in(&editor, cx, Some("Ship"), Some("Ship it."));
        assert!(key(&editor, cx, "escape"));
        assert_eq!(history(&editor, cx).len(), 1, "Esc closes without a change");
        cx.update(|window, cx| editor.update(cx, |editor, cx| editor.open_at("workflow/land", true, window, cx)));
        type_in(&editor, cx, Some("  "), None);
        assert!(key(&editor, cx, "enter"));
        assert_eq!(history(&editor, cx).len(), 1, "an empty name is not a name");
    }

    #[gpui::test]
    fn a_plus_adds_a_step_after_the_gaps_first_with_its_name_open_and_both_undo_alone(cx: &mut TestAppContext) {
        let (editor, _, _, cx) = open(cx, false);
        cx.update(|window, cx| editor.update(cx, |editor, cx| editor.add_after(Some("build".into()), window, cx)));
        assert_eq!(history(&editor, cx), [(Author::Person, "Added New step after Build".to_string())]);
        assert_eq!(drawn(&editor, cx, "workflow/new_step"), Some((2, Some("Added".into()))));
        editor.read_with(cx, |editor, cx| {
            let one = editor.in_place.as_ref().expect("its fields are open");
            assert_eq!(one.node, "workflow/new_step");
            assert_eq!(one.fields.as_ref().map(|fields| fields.name.read(cx).value().to_string()).as_deref(), Some(NEW_STEP));
        });
        assert!(cx.update(|window, cx| editor.read(cx).typing(window, cx)), "the keys are in its Name");
        type_in(&editor, cx, Some("Review"), None);
        assert!(key(&editor, cx, "enter"));
        assert_eq!(history(&editor, cx)[1], (Author::Person, "Renamed New step to Review".to_string()));
        assert!(cx.update(|window, cx| editor.update(cx, |editor, cx| editor.undo_at(1, window, cx))));
        assert_eq!(crate::settings::definition::steps_of(&editor.read_with(cx, |editor, _| editor.draft().drafted()))[2], ("new_step".to_string(), NEW_STEP.to_string()));
        assert!(cx.update(|window, cx| editor.update(cx, |editor, cx| editor.undo_last(window, cx))));
        assert_eq!(drawn(&editor, cx, "workflow/new_step"), None);
        // With none named, it goes after the last step.
        cx.update(|window, cx| editor.update(cx, |editor, cx| editor.add_after(None, window, cx)));
        assert_eq!(crate::settings::definition::steps_of(&editor.read_with(cx, |editor, _| editor.draft().drafted())).last().map(|(id, _)| id.clone()).as_deref(), Some("new_step"));
    }

    #[gpui::test]
    fn a_picked_state_opens_its_name_and_a_rename_is_one_edit_of_his_held_apart_from_the_definition(cx: &mut TestAppContext) {
        let (editor, fleet, _, cx) = open(cx, false);
        cx.update(|window, cx| editor.update(cx, |editor, cx| editor.lanes_picked(&"".into(), Some(Pick::Node("ticket/in_progress".into())), window, cx)));
        editor.read_with(cx, |editor, cx| {
            let one = editor.in_place.as_ref().expect("open in place");
            let fields = one.fields.as_ref().expect("its Name to edit");
            assert_eq!((fields.name.read(cx).value().to_string().as_str(), fields.words.is_none()), ("In Progress", true), "a state has a Name and no Words");
        });
        cx.update(|window, cx| editor.update(cx, |editor, cx| editor.open_picked(window, cx)));
        type_in(&editor, cx, Some("Working"), None);
        assert!(key(&editor, cx, "tab"), "Tab stays in its only field");
        assert!(key(&editor, cx, "enter"));
        assert_eq!(history(&editor, cx), [(Author::Person, "Renamed In Progress to Working".to_string())]);
        editor.read_with(cx, |editor, _| {
            assert_eq!(editor.draft().drafted(), BASE, "the workflow's text is not the lifecycle's");
            assert_eq!(editor.draft().edits()[0].change, Change::StateLabel { lifecycle: "ticket".into(), state: "in_progress".into(), label: "Working".into() });
        });
        let label = |cx: &mut VisualTestContext| editor.read_with(cx, |editor, cx| editor.lanes_of(&"".into(), cx).and_then(|lanes| lanes.lanes.iter().flat_map(|lane| lane.nodes.iter()).find(|one| one.id == "ticket/in_progress").map(|one| (one.label.clone(), one.changed.clone()))));
        assert_eq!(label(cx), Some(("Working".into(), Some("Renamed from In Progress".into()))));
        assert!(cx.update(|window, cx| editor.update(cx, |editor, cx| editor.undo_last(window, cx))));
        assert_eq!(label(cx), Some(("In Progress".into(), None)), "its Undo takes the name back");
        // A state of a lifecycle that was not read opens to read.
        cx.update(|window, cx| editor.update(cx, |editor, cx| editor.lanes_picked(&"".into(), Some(Pick::Node("watch/running".into())), window, cx)));
        assert!(editor.read_with(cx, |editor, _| editor.in_place.as_ref().is_some_and(|one| one.fields.is_none())));
        // A step the definition does not hold is read, not edited.
        cx.update(|window, cx| editor.update(cx, |editor, cx| editor.lanes_picked(&"".into(), Some(Pick::Node("workflow/walk".into())), window, cx)));
        assert!(editor.read_with(cx, |editor, _| editor.in_place.as_ref().is_some_and(|one| one.fields.is_none())));
        cx.update(|window, cx| editor.update(cx, |editor, cx| editor.lanes_picked(&"".into(), Some(Pick::Node("workflow/plan".into())), window, cx)));
        cx.update(|window, cx| editor.update(cx, |editor, cx| editor.open_picked(window, cx)));
        type_in(&editor, cx, Some("Think"), None);
        cx.update(|window, cx| editor.update(cx, |editor, cx| editor.lanes_picked(&"".into(), None, window, cx)));
        assert_eq!(history(&editor, cx), [(Author::Person, "Renamed Plan to Think".to_string())], "a pick let go lands what was typed");
        assert!(editor.read_with(cx, |editor, _| editor.in_place.is_none()));
        assert!(fleet.pieces.borrow().is_empty(), "nothing is written before Publish");
    }

    #[gpui::test]
    fn a_step_added_is_picked_and_a_adds_the_next_after_it(cx: &mut TestAppContext) {
        let (editor, _, _, cx) = open(cx, false);
        cx.update(|window, cx| editor.update(cx, |editor, cx| editor.add_after(Some("build".into()), window, cx)));
        assert_eq!(editor.read_with(cx, |editor, _| editor.picked.clone()), Some(Pick::Node("workflow/new_step".into())));
        assert!(key(&editor, cx, "enter"));
        cx.run_until_parked();
        assert!(cx.update(|window, cx| editor.update(cx, |editor, cx| editor.add_after_picked(window, cx))));
        let steps = crate::settings::definition::steps_of(&editor.read_with(cx, |editor, _| editor.draft().drafted()));
        assert_eq!(steps.iter().map(|(id, _)| id.as_str()).collect::<Vec<_>>(), ["plan", "build", "new_step", "new_step_2", "land"]);
    }

    /// A state renamed and a step renamed, at `target`.
    fn rename_both(editor: &Entity<LanesEditor>, cx: &mut VisualTestContext, target: Target) {
        editor.update(cx, |editor, cx| {
            editor.edit(Author::Person, "Renamed Build to Make", Change::StepLabel { step: "build".into(), label: "Make".into() }, cx);
            editor.edit(Author::Person, "Renamed In Progress to Working", Change::StateLabel { lifecycle: "ticket".into(), state: "in_progress".into(), label: "Working".into() }, cx);
            editor.apply_at(target, cx);
        });
    }

    #[gpui::test]
    fn publish_writes_the_definition_then_each_renamed_lifecycle_at_the_drafts_place(cx: &mut TestAppContext) {
        let (editor, fleet, seen, cx) = open(cx, false);
        rename_both(&editor, cx, Target::Repository("armada".into()));
        editor.update(cx, |editor, cx| editor.publish(cx));
        cx.run_until_parked();
        assert_eq!(*fleet.asked.borrow(), ["definition", "piece"]);
        let pieces = fleet.pieces.borrow();
        let ipc::LifecyclePiece::Lifecycle { lifecycle } = &pieces[0].piece else { panic!("a lifecycle") };
        assert_eq!(lifecycle.scope, ipc::PieceScope::Within { id: "armada".into() });
        assert_eq!(lifecycle.states.iter().map(|one| one.label.as_str()).collect::<Vec<_>>(), ["Todo", "Working", "Done"], "the one that runs, renamed");
        assert!(!pieces[0].overwrite, "nothing of its id is kept at that scope");
        assert_eq!(*seen.borrow(), [EditorEvent::Published { target: Target::Repository("armada".into()) }]);
    }

    #[gpui::test]
    fn a_rename_for_every_project_replaces_the_one_kept_for_every_repository(cx: &mut TestAppContext) {
        let (editor, fleet, _, cx) = open(cx, false);
        rename_both(&editor, cx, Target::EveryProject);
        editor.update(cx, |editor, cx| editor.publish(cx));
        cx.run_until_parked();
        let pieces = fleet.pieces.borrow();
        let ipc::LifecyclePiece::Lifecycle { lifecycle } = &pieces[0].piece else { panic!("a lifecycle") };
        assert_eq!((lifecycle.scope.clone(), pieces[0].overwrite), (ipc::PieceScope::Everywhere, true), "Kit keeps one for every repository: it is replaced");
    }

    #[gpui::test]
    fn a_lifecycle_refused_keeps_the_draft_with_fleets_sentence_and_the_definition_is_not_written_twice(cx: &mut TestAppContext) {
        let (editor, fleet, seen, cx) = open(cx, false);
        *fleet.refuses_pieces.borrow_mut() = Some("Ticket has no state Done to end on".into());
        rename_both(&editor, cx, Target::EveryProject);
        editor.update(cx, |editor, cx| editor.publish(cx));
        cx.run_until_parked();
        editor.read_with(cx, |editor, _| {
            assert_eq!(editor.refused.as_deref(), Some("Ticket has no state Done to end on"));
            assert_eq!(editor.draft().edits().len(), 2, "the draft stays");
        });
        assert!(seen.borrow().is_empty());
        *fleet.refuses_pieces.borrow_mut() = None;
        *fleet.definition.borrow_mut() = fleet.saved.borrow()[0].1.definition.clone();
        editor.update(cx, |editor, cx| editor.publish(cx));
        cx.run_until_parked();
        assert_eq!(*fleet.asked.borrow(), ["definition", "piece", "piece"], "tried again, only what was refused is sent");
        assert_eq!(seen.borrow().len(), 1);
    }

    #[gpui::test]
    fn publish_waits_on_where_it_applies_then_makes_the_copy_there(cx: &mut TestAppContext) {
        let (editor, fleet, seen, cx) = open(cx, true);
        editor.update(cx, |editor, cx| editor.agent_edit("Add a Review step after Build", BASE.into(), REVIEWED.into(), cx));
        editor.update(cx, |editor, cx| editor.publish(cx));
        cx.run_until_parked();
        assert!(fleet.saved.borrow().is_empty(), "no place answered");
        editor.update(cx, |editor, cx| editor.apply_at(Target::EveryProject, cx));
        editor.update(cx, |editor, cx| editor.publish(cx));
        cx.run_until_parked();
        let saved = fleet.saved.borrow();
        assert_eq!(saved.len(), 1);
        assert_eq!((saved[0].0.as_str(), saved[0].1.scope, saved[0].1.overwrite), ("armada", WorkflowScope::Kit, false), "a copy is new where the workflow is Armada's");
        assert_eq!(crate::settings::definition::tree_of(&saved[0].1.definition), crate::settings::definition::tree_of(REVIEWED));
        assert_eq!(*seen.borrow(), [EditorEvent::Published { target: Target::EveryProject }]);
        assert!(!editor.read_with(cx, |editor, _| editor.has_edits()), "the draft is let go once it is written");
    }

    #[gpui::test]
    fn publish_is_refused_where_what_was_saved_moved_under_the_draft(cx: &mut TestAppContext) {
        let (editor, fleet, seen, cx) = open(cx, true);
        editor.update(cx, |editor, cx| editor.agent_edit("Add a Review step after Build", BASE.into(), REVIEWED.into(), cx));
        editor.update(cx, |editor, cx| editor.apply_at(Target::Repository("armada".into()), cx));
        *fleet.definition.borrow_mut() = BASE.replace("Plan", "Think");
        editor.update(cx, |editor, cx| editor.publish(cx));
        cx.run_until_parked();
        assert!(fleet.saved.borrow().is_empty());
        assert!(seen.borrow().is_empty());
        editor.read_with(cx, |editor, _| {
            assert_eq!(editor.refused.as_deref(), Some(CHANGED_SINCE));
            assert!(editor.has_edits(), "the draft stays for Discard");
        });
    }
}
