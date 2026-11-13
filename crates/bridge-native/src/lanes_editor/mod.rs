//! A workflow's lanes with a draft over them: the canvas, the edits made on it kept as a draft with ⌘Z and a history
//! that undoes each one, and Publish, which writes the draft where it applies. One piece, mounted wherever lanes
//! are edited: Settings › Workflows today, a Job's Workflow tab and Now next.
//!
//! **Mounting it.** The host builds `cx.new(|cx| LanesEditor::new(subject, wire, window, cx))` with a [`Subject`]
//! (the workflow, the level its lanes are read at, where its definition is kept) and an [`EditorWire`] it answers
//! Fleet's reads and saves with, and draws the entity as a child. It adds [`LanesEditor::stops`] to its Tab stops
//! before the canvas, passes keys it did not take to [`LanesEditor::key`] while no text field holds them, and
//! subscribes to [`EditorEvent`]. An agent's change comes in through [`LanesEditor::agent_edit`]; a person's edit
//! through [`LanesEditor::edit`].
//!
//! **Nothing is written until Publish.** An edit lands in the [`Draft`]; the canvas shows the draft's lanes, with
//! what differs marked as a changed node is.

mod bar;
mod draft;
mod hooks;
mod patch;
mod wire;

use std::rc::Rc;

use gpui::{div, prelude::*, px, AnyElement, Context, EventEmitter, FocusHandle, Keystroke, Role, SharedString, Subscription, Task, Window};
use ipc::{JobLanesView, SaveWorkflow, WorkflowScope};

pub use draft::{Author, Change, Draft, Edit, Target};
pub use wire::{EditorWire, Refused, Subject};

use crate::labels::labelled;
use crate::lanes_canvas::{lanes_canvas, LanesHost, LanesState};
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

/// A lane's least height (`LANE_MIN` in `lanes_canvas`, `--lane-min`), so every lane stands in the canvas.
const LANE_MIN: f32 = 128.;

/// Fleet's word where what a draft was made from is not what is saved now.
const CHANGED_SINCE: &str = "Changed since you started; Discard and edit again";

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
        let mut editor = LanesEditor { wire, subject, id, read: Read::Loading, saved: None, draft, shown: None, state, history_open: false, refused: None, busy: false, keys, undos: Vec::new(), loading: None, publishing: None, _subs: subs };
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
            let (lanes, definition) = cx.update(|app| (wire.lanes(&subject, app), wire.definition(&subject, app)));
            let (lanes, definition) = (lanes.await, definition.await);
            let _ = this.update(cx, |this, cx| this.loaded(lanes, definition.map(|read| read.definition).ok(), cx));
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

    /// The lanes the canvas draws: the read ones with the draft's steps over them while there are edits.
    fn refresh(&mut self, cx: &mut Context<Self>) {
        self.shown = match &self.read {
            Read::Read(lanes) if !self.draft.is_empty() && !self.draft.base().is_empty() => Some(patch::patched(lanes, self.draft.base(), &self.draft.drafted())),
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
        self.draft.set_target(self.subject.own_target());
        self.refused = None;
        self.refresh(cx);
        if was_here {
            window.focus(&self.state.frame, cx);
        }
        cx.notify();
    }

    /// Writes the draft where it applies, if what it was made from is still what is saved: a copy at the place the
    /// person chose for a workflow that is Armada's, or the copy itself where it is one.
    pub fn publish(&mut self, cx: &mut Context<Self>) {
        let Some(target) = self.draft.target().cloned().filter(|_| !self.busy && !self.draft.is_empty()) else { return };
        self.busy = true;
        self.refused = None;
        cx.notify();
        let (wire, subject) = (self.wire.clone(), self.subject.clone());
        let (base, drafted) = (self.draft.base().to_string(), self.draft.drafted());
        self.publishing = Some(cx.spawn(async move |this, cx| {
            let answer: Result<(), Refused> = async {
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
                cx.update(|app| wire.save_workflow(&manifest, save, app)).await.map(|_| ())
            }
            .await;
            let _ = this.update(cx, |this, cx| this.published(target, answer, cx));
        }));
    }

    fn published(&mut self, target: Target, answer: Result<(), Refused>, cx: &mut Context<Self>) {
        self.busy = false;
        match answer {
            Err(why) => self.refused = Some(why.said),
            Ok(()) => {
                self.draft = Draft::new(String::new(), self.subject.own_target());
                self.history_open = false;
                self.shown = None;
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
    /// the keys; Esc shuts the history. `true` where one was taken. A host calls it only while no text field has them.
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
                let tall = (look.length(token::SPACE_12) * 11.).max(px(lanes.lanes.len() as f32 * LANE_MIN) + look.length(token::BORDER_WIDTH) * 2.);
                // An Esc the lanes did not take, with nothing picked, is the host's: it shuts what holds the editor.
                let shut = cx.listener(|_, event: &gpui::KeyDownEvent, _, cx| {
                    if event.keystroke.key == "escape" {
                        cx.emit(EditorEvent::Escaped);
                        cx.stop_propagation();
                    }
                });
                div().flex_none().h(tall).min_w_0().flex().flex_col().on_key_down(shut).child(lanes_canvas(self, &self.id, lanes, &look, cx)).into_any_element()
            }
        };
        div().flex().flex_col().gap(look.length(token::SPACE_3)).min_w_0().children(self.render_bar(&look, window, cx)).child(body)
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use gpui::{Entity, TestAppContext, VisualTestContext};
    use ipc::{WorkflowDefinition, WorkflowId, WorkflowSaved};

    use super::*;

    const BASE: &str = r#"{"version":1,"workflow_id":"prototype","name":"prototype","steps":[{"id":"plan","label":"Plan"},{"id":"build","label":"Build"},{"id":"land","label":"Land"}]}"#;
    const REVIEWED: &str = r#"{"version":1,"workflow_id":"prototype","name":"prototype","steps":[{"id":"plan","label":"Plan"},{"id":"build","label":"Build"},{"id":"review","label":"Review"},{"id":"land","label":"Land"}]}"#;

    /// What Fleet holds for the editor, and what it was asked to save.
    struct Fleet {
        definition: RefCell<String>,
        saved: RefCell<Vec<(String, SaveWorkflow)>>,
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
            self.0.saved.borrow_mut().push((manifest.to_string(), save));
            Task::ready(Ok(WorkflowSaved { workflow_id: WorkflowId::carried("prototype"), scope, file: "prototype.yml".into(), replaced: false, runs_from: None }))
        }

        fn save_piece(&self, _: ipc::SaveLifecyclePiece, _: &mut gpui::App) -> Task<Result<ipc::LifecyclePieceSaved, Refused>> {
            Task::ready(Err(Refused::said("not here")))
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
        let fleet = Rc::new(Fleet { definition: RefCell::new(BASE.into()), saved: RefCell::default() });
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
        assert_eq!(saved[0].1.definition, REVIEWED);
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
