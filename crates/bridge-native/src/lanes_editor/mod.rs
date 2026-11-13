//! A workflow's lanes with a draft over them: the canvas, the edits made on it kept as Fleet's draft with ⌘Z and a
//! history that undoes each one, and Publish, which writes the draft where it applies. One piece, mounted wherever
//! lanes are edited: Settings › Workflows today, a Job's Workflow tab and Now next.
//!
//! **Mounting it.** The host builds `cx.new(|cx| LanesEditor::new(subject, wire, window, cx))` with a [`Subject`]
//! (the workflow, the level its lanes are read at, where its definition is kept) and an [`EditorWire`] it answers
//! Fleet's reads and drafts with, and draws the entity as a child. It adds [`LanesEditor::stops`] to its Tab stops
//! before the canvas, passes keys it did not take to [`LanesEditor::key`] while no text field holds them, and
//! subscribes to [`EditorEvent`]. An agent's change comes in through [`LanesEditor::agent_edit`], or, where Fleet
//! already put it in the draft, [`LanesEditor::drafted_by_agent`]; a person's edit through [`LanesEditor::edit`], or
//! on the canvas itself: a picked step opens its fields in place, and a + in a gap adds a step there (`in_place`).
//! While [`LanesEditor::typing`], the host passes its keys to [`LanesEditor::field_key`] first, and counts the
//! fields as text.
//!
//! **The draft is Fleet's.** Each edit works out the new whole text from the current one and puts it
//! (`PUT /workflows/draft`); the answer is the draft. Nothing is written until Publish. A state renamed goes to its
//! lifecycle's piece draft, at the scope the draft applies at, and waits here for the answer where Armada's own
//! workflow has no scope yet.

mod bar;
mod draft;
mod in_place;
mod patch;
mod wire;

use std::cell::Cell;
use std::collections::VecDeque;
use std::rc::Rc;

use gpui::{div, prelude::*, px, AnyElement, Bounds, Context, EventEmitter, FocusHandle, Keystroke, Pixels, Role, SharedString, Subscription, Task, Window};
use ipc::{DraftBy, JobLanesView, LifecycleDefinition, LifecyclePiece, LifecyclePieceKind, PieceDraft, PieceScope, PublishPieceDraft, PublishWorkflowDraft, PutPieceDraft, PutWorkflowDraft, WorkflowDraft, WorkflowId, WorkflowScope};

pub use draft::{Change, Target};
pub use wire::{EditorWire, Refused, Subject};

use crate::labels::labelled;
use crate::lanes_canvas::{lanes_canvas, LanesHost, LanesState, Pick};
use crate::look::{token, ActiveLook};
use crate::settings::definition::with_delta;
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

/// What the list says of a change that carries no words.
const A_CHANGE: &str = "A change";

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
    restart: FocusHandle,
    repository: FocusHandle,
    every: FocusHandle,
    project: FocusHandle,
}

/// A press the bar's keys answer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Press {
    Changes,
    Discard,
    Publish,
    Restart,
    Repository,
    Every,
    Project,
    Undo(usize),
}

/// Where a row of the Changes list comes from: a change of the workflow's draft, a change of one piece's draft (both
/// by their place in its `changes`), or a rename still waiting for where it applies.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Of {
    Workflow(usize),
    Piece(usize, usize),
    Waiting(usize),
}

/// One row of the Changes list.
#[derive(Clone, Debug)]
struct Row {
    id: i64,
    by: DraftBy,
    said: String,
    of: Of,
    /// A change shown after it edits the same words, or what it did does not fit the text now: its Undo is off.
    blocked: bool,
}

/// What is asked of Fleet, in the order the person made it. Each waits for the one before it to be answered.
enum Call {
    /// The workflow's draft text as it is now.
    Put { text: String, said: Option<String> },
    /// A state renamed, in the lifecycle that runs, at `scope`.
    Rename { said: String, lifecycle: String, state: String, label: String, scope: PieceScope },
    /// A piece's draft text as it is now.
    PutPiece { id: String, scope: PieceScope, text: String, said: Option<String> },
    DeletePiece { id: String, scope: PieceScope },
    /// Every draft held is thrown away.
    Discard,
    /// Every draft held is written, at `target`.
    Publish { target: Target },
}

/// What one request to Fleet is.
enum Op {
    Put(PutWorkflowDraft),
    Delete,
    PutPiece(PutPieceDraft),
    DeletePiece(String, PieceScope),
    PublishWorkflow(String, PublishWorkflowDraft),
    PublishPiece(PublishPieceDraft),
}

/// What Fleet's answer to an `Op` leaves.
enum Done {
    Workflow(WorkflowDraft),
    WorkflowGone,
    Piece(PieceDraft),
    PieceGone(String, PieceScope),
}

/// What a `Call` does once all its requests are answered.
enum After {
    Nothing,
    Discarded,
    Published(Target),
}

const STALE: [&str; 2] = ["fleet.workflow_draft_stale", "fleet.piece_draft_stale"];

/// The scope the piece drafts of a draft that applies at `target` are kept at.
fn piece_scope(target: &Target) -> PieceScope {
    match target {
        // A project is a Workspace grown: its lifecycles are scoped within it, by its id.
        Target::Repository(id) | Target::Project(id) => PieceScope::Within { id: id.clone() },
        Target::EveryProject => PieceScope::Everywhere,
    }
}

/// A lifecycle as the text of its piece, as `SaveLifecyclePiece.piece` takes it.
fn piece_text(lifecycle: LifecycleDefinition) -> String {
    serde_json::to_string(&LifecyclePiece::Lifecycle { lifecycle }).unwrap_or_default()
}

fn lifecycle_of(text: &str) -> Option<LifecycleDefinition> {
    match ipc::decode::<LifecyclePiece>("piece", text.as_bytes()) {
        Ok(LifecyclePiece::Lifecycle { lifecycle }) => Some(lifecycle),
        _ => None,
    }
}

/// `text` with the states `change` renamed (those whose name differs from `before`'s) named as `before` names them.
fn piece_undone(current: &str, change: &str, before: &str) -> Option<String> {
    let (mut now, was, then) = (lifecycle_of(current)?, lifecycle_of(change)?, lifecycle_of(before)?);
    for state in &mut now.states {
        let label = |of: &LifecycleDefinition| of.states.iter().find(|one| one.id == state.id).map(|one| one.label.clone());
        if let (Some(changed), Some(earlier)) = (label(&was), label(&then)) {
            if changed != earlier {
                state.label = earlier;
            }
        }
    }
    Some(piece_text(now))
}

pub struct LanesEditor {
    wire: Rc<dyn EditorWire>,
    subject: Subject,
    id: SharedString,
    read: Read,
    /// The definition as last read: what a draft begins from where Fleet holds none.
    saved: Option<String>,
    /// Fleet's draft of the workflow, as it last answered.
    draft: Option<WorkflowDraft>,
    /// Fleet's drafts of the lifecycles a state was renamed in, at the scope the draft applies at.
    pieces: Vec<PieceDraft>,
    /// The text as the editor has it: the draft's, with every edit Fleet has not answered yet.
    text: String,
    /// Renames waiting for where the draft applies.
    waiting: Vec<draft::Waiting>,
    /// Where the draft applies.
    target: Option<Target>,
    /// The Changes list.
    rows: Vec<Row>,
    /// The read lanes with the draft laid over them, while it has edits.
    shown: Option<JobLanesView>,
    state: LanesState,
    history_open: bool,
    /// What Fleet refused with, said on the bar.
    refused: Option<String>,
    /// The refusal was that the workflow changed since the draft began: the bar offers to start over.
    stale: bool,
    busy: bool,
    keys: Keys,
    /// One stop for each change's Undo.
    undos: Vec<FocusHandle>,
    queue: VecDeque<Call>,
    sending: Option<Task<()>>,
    loading: Option<Task<()>>,
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
        let keys = Keys { changes: stop(cx), discard: stop(cx), publish: stop(cx), restart: stop(cx), repository: stop(cx), every: stop(cx), project: stop(cx) };
        let id: SharedString = format!("{}|{}|{}", subject.workflow, subject.level, subject.manifest).into();
        let target = subject.own_target();
        let mut editor = LanesEditor {
            wire,
            subject,
            id,
            read: Read::Loading,
            saved: None,
            draft: None,
            pieces: Vec::new(),
            text: String::new(),
            waiting: Vec::new(),
            target,
            rows: Vec::new(),
            shown: None,
            state,
            history_open: false,
            refused: None,
            stale: false,
            busy: false,
            keys,
            undos: Vec::new(),
            queue: VecDeque::new(),
            sending: None,
            loading: None,
            picked: None,
            armed: false,
            in_place: None,
            frame: Rc::default(),
            seen: Rc::default(),
            page: None,
            reveal: Cell::new(false),
            lifecycles: Vec::new(),
            _subs: subs,
        };
        editor.load(cx);
        editor
    }

    #[allow(dead_code)] // The surface a Job's Workflow tab mounts it through.
    pub fn subject(&self) -> &Subject {
        &self.subject
    }

    /// The project the host is editing now, offered first where an edit asks where it applies.
    pub fn offer_project(&mut self, project: Option<(String, String)>, cx: &mut Context<Self>) {
        if self.subject.project != project {
            self.subject.project = project;
            cx.notify();
        }
    }

    /// Whether anything is drafted, or on its way to Fleet to be.
    pub fn has_edits(&self) -> bool {
        self.draft.is_some() || !self.pieces.is_empty() || !self.waiting.is_empty() || !self.queue.is_empty() || self.sending.is_some()
    }

    /// The text the draft began from, or what is saved where there is no draft.
    fn base(&self) -> &str {
        self.draft.as_ref().map(|draft| draft.base.as_str()).or(self.saved.as_deref()).unwrap_or_default()
    }

    /// The lanes, the definition and the drafts read again.
    pub fn reload(&mut self, cx: &mut Context<Self>) {
        self.load(cx);
    }

    fn load(&mut self, cx: &mut Context<Self>) {
        let (wire, subject) = (self.wire.clone(), self.subject.clone());
        // A piece draft is read at the scope the draft applies at: where Armada's own has none yet, there is none to read.
        let scope = self.target.as_ref().map(piece_scope);
        self.loading = Some(cx.spawn(async move |this, cx| {
            let (lanes, definition, lifecycles, draft) = cx.update(|app| (wire.lanes(&subject, app), wire.definition(&subject, app), wire.lifecycles(&subject.repository, app), wire.workflow_draft(&subject, app)));
            let (lanes, definition, lifecycles, draft) = (lanes.await, definition.await, lifecycles.await, draft.await);
            // Without them a state is read and not renamed.
            let lifecycles = lifecycles.map(|read| read.lifecycles).unwrap_or_default();
            let draft = match draft {
                Ok(draft) => Some(Some(draft)),
                Err(why) if why.is("fleet.workflow_draft_none") => Some(None),
                Err(_) => None,
            };
            let mut pieces = Vec::new();
            for one in lifecycles.iter().filter(|one| one.kept.overridden_by.is_none()) {
                let Some(scope) = &scope else { break };
                if let Ok(piece) = cx.update(|app| wire.piece_draft(LifecyclePieceKind::Lifecycle, &one.lifecycle.id, scope, app)).await {
                    pieces.push(piece);
                }
            }
            let pieces = scope.is_some().then_some(pieces);
            let _ = this.update(cx, |this, cx| {
                this.lifecycles = lifecycles;
                this.loaded(lanes, definition.map(|read| read.definition).ok(), draft, pieces, cx)
            });
        }));
    }

    fn loaded(&mut self, lanes: Result<JobLanesView, Refused>, definition: Option<String>, draft: Option<Option<WorkflowDraft>>, pieces: Option<Vec<PieceDraft>>, cx: &mut Context<Self>) {
        self.read = match lanes {
            Ok(lanes) => Read::Read(lanes),
            Err(why) => Read::Refused(why.said),
        };
        if let Some(text) = definition {
            self.saved = Some(text);
        }
        // What is on its way to Fleet is not read over: the answers will say.
        if self.queue.is_empty() {
            if let Some(draft) = draft {
                self.draft = draft;
            }
            if let Some(pieces) = pieces {
                self.pieces = pieces;
            }
            self.text = self.draft.as_ref().map(|draft| draft.text.clone()).or_else(|| self.saved.clone()).unwrap_or_default();
        }
        self.refresh(cx);
        cx.notify();
    }

    /// The states the draft renames, `(lifecycle, state, name)`: those of each piece draft that the lifecycle that runs
    /// names otherwise, then the renames still waiting.
    fn state_labels(&self) -> Vec<(String, String, String)> {
        let mut labels = Vec::new();
        for piece in &self.pieces {
            let (Some(drafted), Some(running)) = (lifecycle_of(&piece.text), self.running_piece(&piece.id, &piece.scope)) else { continue };
            for state in &drafted.states {
                if running.states.iter().any(|one| one.id == state.id && one.label != state.label) {
                    labels.push((piece.id.clone(), state.id.clone(), state.label.clone()));
                }
            }
        }
        labels.extend(self.waiting.iter().map(|one| (one.lifecycle.clone(), one.state.clone(), one.label.clone())));
        labels
    }

    /// The lifecycle that runs, as it would be kept at `scope`.
    fn running_piece(&self, id: &str, scope: &PieceScope) -> Option<LifecycleDefinition> {
        self.lifecycles.iter().find(|one| one.lifecycle.id == id && one.kept.overridden_by.is_none()).map(|one| LifecycleDefinition { scope: scope.clone(), ..one.lifecycle.clone() })
    }

    /// What a piece draft began from: the piece Kit held, else the lifecycle that runs.
    fn piece_origin(&self, piece: &PieceDraft) -> String {
        if piece.base.is_empty() {
            self.running_piece(&piece.id, &piece.scope).map(piece_text).unwrap_or_default()
        } else {
            piece.base.clone()
        }
    }

    /// The lanes the canvas draws: the read ones with the draft's steps and states over them while there are edits,
    /// and room after the workflow's last step.
    fn refresh(&mut self, cx: &mut Context<Self>) {
        let base = self.base().to_string();
        self.shown = match &self.read {
            Read::Read(lanes) if self.has_edits() && !base.is_empty() => {
                let steps = patch::patched(lanes, &base, &self.text);
                Some(in_place::with_end_room(patch::with_states_renamed(steps, &self.state_labels())))
            }
            Read::Read(lanes) => Some(in_place::with_end_room(lanes.clone())),
            _ => None,
        };
        self.rows = self.build_rows();
        while self.undos.len() < self.rows.len() {
            self.undos.push(cx.focus_handle().tab_stop(true));
        }
        self.undos.truncate(self.rows.len());
        if !self.has_edits() {
            self.history_open = false;
            self.target = self.subject.own_target();
        }
        // Fields he is not in are opened again on what the draft says now.
        if !self.in_place.as_ref().is_some_and(|one| one.keyed) {
            self.in_place = None;
        }
    }

    // ---- the Changes list ----------------------------------------------------------------------

    /// The text that takes back one row: where it is the draft's last, the text before it, put again; else what it did
    /// reversed on top of the text now, which Fleet is told is the undoing of that change. None where that does not fit.
    fn reverse(&self, of: Of) -> Option<(String, Option<String>)> {
        match of {
            Of::Workflow(at) => {
                let draft = self.draft.as_ref()?;
                let (change, previous) = (&draft.changes[at], draft::before(&draft.changes, at, &draft.base));
                if self.text == change.text {
                    return Some((previous.to_string(), None));
                }
                with_delta(&self.text, &change.text, previous).map(|text| (text, Some(draft::undid_summary(change.id))))
            }
            Of::Piece(piece, at) => {
                let piece = self.pieces.get(piece)?;
                let origin = self.piece_origin(piece);
                let (change, previous) = (&piece.changes[at], draft::before(&piece.changes, at, &origin));
                if piece.text == change.text {
                    return Some((previous.to_string(), None));
                }
                piece_undone(&piece.text, &change.text, previous).map(|text| (text, Some(draft::undid_summary(change.id))))
            }
            Of::Waiting(_) => None,
        }
    }

    /// Each change of each draft that is not taken back, in the order Fleet numbered them, then the renames waiting.
    fn build_rows(&self) -> Vec<Row> {
        let said = |change: &ipc::DraftChange| change.summary.clone().unwrap_or_else(|| A_CHANGE.to_string());
        let mut rows = Vec::new();
        if let Some(draft) = &self.draft {
            let shown = draft::shown(&draft.changes, &draft.base);
            for (k, at) in shown.iter().enumerate() {
                let change = &draft.changes[*at];
                let blocked = draft::edited_later(&draft.changes, &shown, k, &draft.base) || self.reverse(Of::Workflow(*at)).is_none();
                rows.push(Row { id: change.id, by: change.by, said: said(change), of: Of::Workflow(*at), blocked });
            }
        }
        for (p, piece) in self.pieces.iter().enumerate() {
            let origin = self.piece_origin(piece);
            let shown = draft::shown(&piece.changes, &origin);
            for (k, at) in shown.iter().enumerate() {
                let change = &piece.changes[*at];
                let blocked = draft::edited_later(&piece.changes, &shown, k, &origin) || self.reverse(Of::Piece(p, *at)).is_none();
                rows.push(Row { id: change.id, by: change.by, said: said(change), of: Of::Piece(p, *at), blocked });
            }
        }
        rows.sort_by_key(|row| row.id);
        rows.extend(self.waiting.iter().enumerate().map(|(n, one)| Row { id: i64::MAX, by: DraftBy::You, said: one.said.clone(), of: Of::Waiting(n), blocked: false }));
        rows
    }

    // ---- the draft ----------------------------------------------------------------------------

    /// One edit: the new whole text worked out from the one now, with the line the history reads for it, and put.
    pub fn edit(&mut self, said: impl Into<String>, change: Change, cx: &mut Context<Self>) {
        let said = said.into();
        self.refused = None;
        self.stale = false;
        if let Change::StateLabel { lifecycle, state, label } = change {
            return self.rename(said, lifecycle, state, label, cx);
        }
        // An agent's change names the text it was drafted from; any other edit waits on the read.
        if self.text.is_empty() {
            match &change {
                Change::Definition { before, .. } => self.text = before.clone(),
                _ => return,
            }
        }
        let Some(text) = change.apply(&self.text) else { return };
        self.put_text(text, Some(said), cx);
    }

    fn put_text(&mut self, text: String, said: Option<String>, cx: &mut Context<Self>) {
        if text == self.text {
            return;
        }
        self.text = text.clone();
        self.enqueue(Call::Put { text, said }, cx);
        self.refresh(cx);
        cx.notify();
    }

    fn rename(&mut self, said: String, lifecycle: String, state: String, label: String, cx: &mut Context<Self>) {
        match self.target.as_ref().map(piece_scope) {
            Some(scope) => self.enqueue(Call::Rename { said, lifecycle, state, label, scope }, cx),
            None => self.waiting.push(draft::Waiting { said, lifecycle, state, label }),
        }
        self.refresh(cx);
        cx.notify();
    }

    /// An agent's change, drafted from `before` as `after`, for what the person asked, where Fleet did not put it in the
    /// draft: it is laid on the text now and put as the person's.
    pub fn agent_edit(&mut self, asked: impl Into<String>, before: String, after: String, cx: &mut Context<Self>) {
        self.edit(asked, Change::Definition { before, after }, cx);
    }

    /// An agent's change Fleet has put in the draft as the agent's: the draft is read again.
    pub fn drafted_by_agent(&mut self, cx: &mut Context<Self>) {
        let (wire, subject) = (self.wire.clone(), self.subject.clone());
        self.loading = Some(cx.spawn(async move |this, cx| {
            let draft = cx.update(|app| wire.workflow_draft(&subject, app)).await;
            let _ = this.update(cx, |this, cx| {
                if let (Ok(draft), true) = (draft, this.queue.is_empty()) {
                    this.text = draft.text.clone();
                    this.draft = Some(draft);
                    this.refresh(cx);
                    cx.notify();
                }
            });
        }));
    }

    /// Where the draft applies, as the person answered the question. The piece drafts held at another scope are thrown
    /// away and put again at this one, and the renames that waited for it are put.
    pub fn apply_at(&mut self, target: Target, cx: &mut Context<Self>) {
        if self.target.as_ref() == Some(&target) {
            return;
        }
        let scope = piece_scope(&target);
        self.target = Some(target);
        for piece in self.pieces.clone().into_iter().filter(|piece| piece.scope != scope) {
            let origin = self.piece_origin(&piece);
            let said = draft::shown(&piece.changes, &origin).last().and_then(|at| piece.changes[*at].summary.clone());
            self.enqueue(Call::DeletePiece { id: piece.id.clone(), scope: piece.scope.clone() }, cx);
            if let Some(mut lifecycle) = lifecycle_of(&piece.text) {
                lifecycle.scope = scope.clone();
                self.enqueue(Call::PutPiece { id: piece.id, scope: scope.clone(), text: piece_text(lifecycle), said }, cx);
            }
        }
        for one in std::mem::take(&mut self.waiting) {
            self.enqueue(Call::Rename { said: one.said, lifecycle: one.lifecycle, state: one.state, label: one.label, scope: scope.clone() }, cx);
        }
        self.refresh(cx);
        cx.notify();
    }

    /// ⌘Z: the last change taken back. `false` where there is none, or it cannot be.
    pub fn undo_last(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let at = self.rows.len();
        at > 0 && self.undo_at(at - 1, window, cx)
    }

    /// One change taken back, wherever it stands in the list.
    pub fn undo_at(&mut self, at: usize, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let Some(row) = self.rows.get(at).filter(|row| !row.blocked).cloned() else { return false };
        let was_here = self.owns_focus(window);
        let emptied = self.rows.len() == 1;
        match row.of {
            Of::Waiting(at) => {
                self.waiting.remove(at);
            }
            Of::Workflow(_) => {
                let Some((text, said)) = self.reverse(row.of) else { return false };
                self.put_text(text, said, cx);
            }
            Of::Piece(piece, _) => {
                let Some((text, said)) = self.reverse(row.of) else { return false };
                let piece = &self.pieces[piece];
                let call = Call::PutPiece { id: piece.id.clone(), scope: piece.scope.clone(), text, said };
                self.enqueue(call, cx);
            }
        }
        self.refused = None;
        self.stale = false;
        self.refresh(cx);
        // The control that was pressed goes with its row: the keys go to the history's head, or to the canvas once
        // nothing is left of the bar.
        if was_here {
            window.focus(if emptied { &self.state.frame } else { &self.keys.changes }, cx);
        }
        cx.notify();
        true
    }

    /// Every draft thrown away, back to what is published, and read again.
    pub fn discard(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let was_here = self.owns_focus(window);
        self.in_place = None;
        self.waiting.clear();
        self.refused = None;
        self.stale = false;
        self.text = self.saved.clone().unwrap_or_default();
        self.enqueue(Call::Discard, cx);
        self.refresh(cx);
        if was_here {
            window.focus(&self.state.frame, cx);
        }
        cx.notify();
    }

    /// Writes the draft where it applies, through Fleet: a copy at the place the person chose for a workflow that is
    /// Armada's, or the workflow itself where it is a copy. Then each lifecycle it renamed a state of, at the same
    /// place. A refusal keeps the draft, and says Fleet's sentence.
    pub fn publish(&mut self, cx: &mut Context<Self>) {
        let Some(target) = self.target.clone().filter(|_| !self.busy && self.has_edits()) else { return };
        self.busy = true;
        self.refused = None;
        self.stale = false;
        self.enqueue(Call::Publish { target }, cx);
        cx.notify();
    }

    // ---- what Fleet is asked ------------------------------------------------------------------

    fn enqueue(&mut self, call: Call, cx: &mut Context<Self>) {
        self.queue.push_back(call);
        self.pump(cx);
    }

    /// One task sends the calls in order, each once the one before is answered, and folds each answer into what is held.
    fn pump(&mut self, cx: &mut Context<Self>) {
        if self.sending.is_some() {
            return;
        }
        let (wire, subject) = (self.wire.clone(), self.subject.clone());
        self.sending = Some(cx.spawn(async move |this, cx| {
            loop {
                let Ok(Some((ops, after))) = this.update(cx, |this, _| this.queue.pop_front().map(|call| this.plan(call))) else { break };
                let mut failed = None;
                for op in ops {
                    match send(&wire, &subject, op, cx).await {
                        Ok(done) => {
                            let _ = this.update(cx, |this, cx| this.done(done, cx));
                        }
                        Err(why) => {
                            failed = Some(why);
                            break;
                        }
                    }
                }
                let _ = this.update(cx, |this, cx| match failed {
                    Some(why) => this.failed(why, cx),
                    None => this.finished(after, cx),
                });
            }
            let _ = this.update(cx, |this, cx| {
                this.sending = None;
                this.refresh(cx);
                cx.notify();
            });
        }));
    }

    /// The requests a call is, worked out from what Fleet holds now: a put of the text the place already holds is a
    /// delete, so no draft is kept that changes nothing.
    fn plan(&mut self, call: Call) -> (Vec<Op>, After) {
        let (source, project) = self.subject.place();
        let workflow = WorkflowId::carried(&self.subject.workflow);
        match call {
            Call::Put { text, said } => {
                let ops = if text == self.base() && self.pieces.is_empty() {
                    self.draft.is_some().then_some(Op::Delete).into_iter().collect()
                } else {
                    vec![Op::Put(PutWorkflowDraft { workflow_id: workflow, source: source.into(), project, text, summary: said })]
                };
                (ops, After::Nothing)
            }
            Call::Rename { said, lifecycle, state, label, scope } => {
                let current = self.pieces.iter().find(|one| one.id == lifecycle && one.scope == scope).map(|one| one.text.clone()).or_else(|| self.running_piece(&lifecycle, &scope).map(piece_text));
                let renamed = current.and_then(|text| lifecycle_of(&text)).and_then(|mut one| {
                    one.states.iter_mut().find(|held| held.id == state)?.label = label;
                    Some(piece_text(one))
                });
                let ops = renamed.and_then(|text| self.piece_op(lifecycle, scope, text, Some(said))).into_iter().collect();
                (ops, After::Nothing)
            }
            Call::PutPiece { id, scope, text, said } => (self.piece_op(id, scope, text, said).into_iter().collect(), After::Nothing),
            Call::DeletePiece { id, scope } => (self.pieces.iter().any(|one| one.id == id && one.scope == scope).then_some(Op::DeletePiece(id, scope)).into_iter().collect(), After::Nothing),
            Call::Discard => {
                let mut ops: Vec<Op> = self.draft.is_some().then_some(Op::Delete).into_iter().collect();
                ops.extend(self.pieces.iter().map(|one| Op::DeletePiece(one.id.clone(), one.scope.clone())));
                (ops, After::Discarded)
            }
            Call::Publish { target } => {
                let (scope, manifest) = match &target {
                    Target::Repository(manifest) => (WorkflowScope::Repository, manifest.clone()),
                    Target::Project(id) => (WorkflowScope::Project { id: id.clone() }, self.subject.repository.clone()),
                    Target::EveryProject => (WorkflowScope::Kit, self.subject.repository.clone()),
                };
                // Only a workflow of Armada's is told where; any other is written in its own place.
                let scope = (source == "armada").then_some(scope);
                let mut ops = Vec::new();
                if self.draft.is_some() {
                    ops.push(Op::PublishWorkflow(manifest, PublishWorkflowDraft { workflow_id: workflow, source: source.into(), project, scope, overwrite: false }));
                }
                ops.extend(self.pieces.iter().map(|one| Op::PublishPiece(PublishPieceDraft { kind: one.kind, id: one.id.clone(), scope: one.scope.clone() })));
                (ops, After::Published(target))
            }
        }
    }

    /// A piece's draft put as `text`, or thrown away where that is what the piece began from.
    fn piece_op(&self, id: String, scope: PieceScope, text: String, said: Option<String>) -> Option<Op> {
        let held = self.pieces.iter().find(|one| one.id == id && one.scope == scope);
        let origin = held.map(|one| self.piece_origin(one)).or_else(|| self.running_piece(&id, &scope).map(piece_text));
        if origin.as_deref() == Some(text.as_str()) {
            return held.map(|_| Op::DeletePiece(id, scope));
        }
        Some(Op::PutPiece(PutPieceDraft { kind: LifecyclePieceKind::Lifecycle, id, scope, text, summary: said }))
    }

    /// An answer folded into what is held. The text is Fleet's once nothing is waiting behind it.
    fn done(&mut self, done: Done, cx: &mut Context<Self>) {
        let idle = self.queue.is_empty();
        match done {
            Done::Workflow(draft) => {
                if idle {
                    self.text = draft.text.clone();
                }
                self.draft = Some(draft);
            }
            Done::WorkflowGone => {
                self.draft = None;
                if idle {
                    self.text = self.saved.clone().unwrap_or_default();
                }
            }
            Done::Piece(piece) => match self.pieces.iter().position(|one| one.kind == piece.kind && one.id == piece.id && one.scope == piece.scope) {
                Some(at) => self.pieces[at] = piece,
                None => self.pieces.push(piece),
            },
            Done::PieceGone(id, scope) => self.pieces.retain(|one| !(one.id == id && one.scope == scope)),
        }
        self.refresh(cx);
        cx.notify();
    }

    fn finished(&mut self, after: After, cx: &mut Context<Self>) {
        self.busy = false;
        match after {
            After::Nothing => {}
            After::Discarded => {
                self.history_open = false;
                self.in_place = None;
                self.waiting.clear();
                self.target = self.subject.own_target();
                self.load(cx);
            }
            After::Published(target) => {
                self.draft = None;
                self.pieces.clear();
                self.waiting.clear();
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

    /// Fleet refused: its sentence is said, what waited behind is let go, and what Fleet holds is read again.
    fn failed(&mut self, why: Refused, cx: &mut Context<Self>) {
        self.busy = false;
        self.stale = STALE.iter().any(|code| why.is(code));
        self.refused = Some(why.said);
        self.queue.clear();
        self.load(cx);
        cx.notify();
    }

    // ---- keys ---------------------------------------------------------------------------------

    /// Whether the bar asks where the draft applies: a workflow that is Armada's, once it has a change.
    fn question_shown(&self) -> bool {
        self.subject.shipped && self.has_edits()
    }

    /// The editor's Tab stops, in the order they are drawn: the answers to where it applies, the bar, the history's
    /// Undo for each change, then the canvas.
    pub fn stops(&self) -> Vec<FocusHandle> {
        let mut stops = Vec::new();
        if self.has_edits() {
            if self.question_shown() {
                if self.subject.project.is_some() {
                    stops.push(self.keys.project.clone());
                }
                stops.extend([self.keys.repository.clone(), self.keys.every.clone()]);
            }
            stops.extend([self.keys.changes.clone(), self.keys.discard.clone(), self.keys.publish.clone()]);
            if self.stale {
                stops.push(self.keys.restart.clone());
            }
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
                self.history_open = !self.history_open && self.has_edits();
                cx.notify();
            }
            Press::Discard | Press::Restart => self.discard(window, cx),
            Press::Publish => self.publish(cx),
            Press::Repository => {
                self.apply_at(Target::Repository(self.subject.repository.clone()), cx);
                window.focus(&self.keys.changes, cx);
            }
            Press::Every => {
                self.apply_at(Target::EveryProject, cx);
                window.focus(&self.keys.changes, cx);
            }
            Press::Project => {
                if let Some((id, _)) = self.subject.project.clone() {
                    self.apply_at(Target::Project(id), cx);
                }
                window.focus(&self.keys.changes, cx);
            }
            Press::Undo(at) => {
                self.undo_at(at, window, cx);
            }
        }
    }

    fn focused_press(&self, window: &Window) -> Option<Press> {
        let keys = &self.keys;
        [(&keys.project, Press::Project), (&keys.repository, Press::Repository), (&keys.every, Press::Every), (&keys.changes, Press::Changes), (&keys.discard, Press::Discard), (&keys.publish, Press::Publish), (&keys.restart, Press::Restart)]
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

/// One request, answered. A delete of a draft Fleet no longer holds is as good as one that took it.
async fn send(wire: &Rc<dyn EditorWire>, subject: &Subject, op: Op, cx: &mut gpui::AsyncApp) -> Result<Done, Refused> {
    match op {
        Op::Put(put) => cx.update(|app| wire.put_workflow_draft(&subject.held_under, put, app)).await.map(Done::Workflow),
        Op::Delete => match cx.update(|app| wire.discard_workflow_draft(subject, app)).await {
            Err(why) if !why.is("fleet.workflow_draft_none") => Err(why),
            _ => Ok(Done::WorkflowGone),
        },
        Op::PutPiece(put) => cx.update(|app| wire.put_piece_draft(put, app)).await.map(Done::Piece),
        Op::DeletePiece(id, scope) => match cx.update(|app| wire.discard_piece_draft(LifecyclePieceKind::Lifecycle, &id, &scope, app)).await {
            Err(why) if !why.is("fleet.piece_draft_none") => Err(why),
            _ => Ok(Done::PieceGone(id, scope)),
        },
        Op::PublishWorkflow(manifest, publish) => cx.update(|app| wire.publish_workflow_draft(&manifest, publish, app)).await.map(|_| Done::WorkflowGone),
        Op::PublishPiece(publish) => {
            let (id, scope) = (publish.id.clone(), publish.scope.clone());
            cx.update(|app| wire.publish_piece_draft(publish, app)).await.map(|_| Done::PieceGone(id, scope))
        }
    }
}

impl LanesHost for LanesEditor {
    fn lanes_state(&self) -> &LanesState {
        &self.state
    }

    /// The canvas pans one empty column past the last step, where the box that adds a step at the end stands.
    fn lanes_reach_past_end(&self) -> bool {
        true
    }

    /// Enter on the end box adds the step the box's press does, and the keys go to it.
    fn lanes_add_at_end(&mut self, _id: &SharedString, _lane: &str, window: &mut Window, cx: &mut Context<Self>) {
        self.add_at_end(window, cx);
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
    use std::cell::{Cell, RefCell};

    use gpui::{Entity, Focusable, TestAppContext, VisualTestContext};
    use ipc::{DraftChange, Instant, WorkflowDefinition, WorkflowDraftDiscarded, WorkflowPublished, WorkflowSaved};

    use super::in_place::NEW_STEP;
    use super::*;

    const BASE: &str = r#"{"version":1,"workflow_id":"prototype","name":"prototype","steps":[{"id":"plan","label":"Plan"},{"id":"build","label":"Build"},{"id":"land","label":"Land"}]}"#;
    const REVIEWED: &str = r#"{"version":1,"workflow_id":"prototype","name":"prototype","steps":[{"id":"plan","label":"Plan"},{"id":"build","label":"Build"},{"id":"review","label":"Review"},{"id":"land","label":"Land"}]}"#;

    /// What Fleet holds for the editor, and what it was asked, in the order it was asked.
    struct Fleet {
        /// What is published.
        definition: RefCell<String>,
        draft: RefCell<Option<WorkflowDraft>>,
        pieces: RefCell<Vec<PieceDraft>>,
        /// Fleet's counter for every change to every draft.
        last: Cell<i64>,
        asked: RefCell<Vec<&'static str>>,
        puts: RefCell<Vec<PutWorkflowDraft>>,
        piece_puts: RefCell<Vec<PutPieceDraft>>,
        publishes: RefCell<Vec<(String, PublishWorkflowDraft)>>,
        piece_publishes: RefCell<Vec<PublishPieceDraft>>,
        /// Fleet's refusal of a publish.
        refuses_publish: RefCell<Option<Refused>>,
    }

    impl Fleet {
        fn change(&self, text: &str, by: DraftBy, summary: Option<String>) -> DraftChange {
            self.last.set(self.last.get() + 1);
            DraftChange { id: self.last.get(), by, at: Instant::carried("2026-11-13T09:00:00Z"), text: text.into(), summary, change: None }
        }

        /// A put as Fleet takes it: it begins the draft from what is published, and a text the draft ends on adds nothing.
        fn put(&self, text: &str, by: DraftBy, summary: Option<String>) -> WorkflowDraft {
            let published = self.definition.borrow().clone();
            let mut held = self.draft.borrow_mut();
            let now = Instant::carried("2026-11-13T09:00:00Z");
            let draft = held.get_or_insert_with(|| WorkflowDraft { workflow_id: WorkflowId::carried("prototype"), source: "armada".into(), project: None, text: published.clone(), base: published, unfit: None, changes: Vec::new(), started_at: now.clone(), updated_at: now });
            if draft.changes.last().map(|last| last.text.as_str()) != Some(text) {
                draft.changes.push(self.change(text, by, summary));
            }
            draft.text = text.into();
            draft.clone()
        }
    }

    fn refused(code: &str, said: &str) -> Refused {
        Refused { key: None, said: said.into(), code: Some(code.into()) }
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
            Task::ready(Ok(WorkflowDefinition { workflow_id: WorkflowId::carried("prototype"), source: "armada".into(), file: "prototype.yml".into(), definition: self.0.definition.borrow().clone(), overridden_by: None, project: None, based_on: None }))
        }

        fn workflow_draft(&self, _: &Subject, _: &mut gpui::App) -> Task<Result<WorkflowDraft, Refused>> {
            Task::ready(self.0.draft.borrow().clone().ok_or_else(|| refused("fleet.workflow_draft_none", "No draft")))
        }

        fn put_workflow_draft(&self, _: &str, put: PutWorkflowDraft, _: &mut gpui::App) -> Task<Result<WorkflowDraft, Refused>> {
            self.0.asked.borrow_mut().push("PUT workflow");
            let draft = self.0.put(&put.text, DraftBy::You, put.summary.clone());
            self.0.puts.borrow_mut().push(put);
            Task::ready(Ok(draft))
        }

        fn discard_workflow_draft(&self, _: &Subject, _: &mut gpui::App) -> Task<Result<WorkflowDraftDiscarded, Refused>> {
            self.0.asked.borrow_mut().push("DELETE workflow");
            *self.0.draft.borrow_mut() = None;
            Task::ready(Ok(WorkflowDraftDiscarded { workflow_id: WorkflowId::carried("prototype"), source: "armada".into(), project: None }))
        }

        fn publish_workflow_draft(&self, manifest: &str, publish: PublishWorkflowDraft, _: &mut gpui::App) -> Task<Result<WorkflowPublished, Refused>> {
            self.0.asked.borrow_mut().push("PUBLISH workflow");
            if let Some(why) = self.0.refuses_publish.borrow().clone() {
                return Task::ready(Err(why));
            }
            let scope = publish.scope.clone().unwrap_or(WorkflowScope::Repository);
            self.0.publishes.borrow_mut().push((manifest.to_string(), publish));
            if let Some(draft) = self.0.draft.borrow_mut().take() {
                *self.0.definition.borrow_mut() = draft.text;
            }
            Task::ready(Ok(WorkflowPublished { saved: WorkflowSaved { workflow_id: WorkflowId::carried("prototype"), scope, file: "prototype.yml".into(), replaced: false, runs_from: None }, version: 1 }))
        }

        fn lifecycles(&self, manifest: &str, _: &mut gpui::App) -> Task<Result<ipc::RepositoryLifecycles, Refused>> {
            Task::ready(Ok(ipc::RepositoryLifecycles { manifest_id: ipc::ManifestId::carried(manifest), lifecycles: applied(), left_out: Vec::new() }))
        }

        fn piece_draft(&self, kind: LifecyclePieceKind, id: &str, scope: &PieceScope, _: &mut gpui::App) -> Task<Result<PieceDraft, Refused>> {
            let found = self.0.pieces.borrow().iter().find(|one| one.kind == kind && one.id == id && one.scope == *scope).cloned();
            Task::ready(found.ok_or_else(|| refused("fleet.piece_draft_none", "No draft")))
        }

        fn put_piece_draft(&self, put: PutPieceDraft, _: &mut gpui::App) -> Task<Result<PieceDraft, Refused>> {
            self.0.asked.borrow_mut().push("PUT piece");
            let now = Instant::carried("2026-11-13T09:00:00Z");
            let mut held = self.0.pieces.borrow_mut();
            let at = held.iter().position(|one| one.id == put.id && one.scope == put.scope).unwrap_or_else(|| {
                held.push(PieceDraft { kind: put.kind, id: put.id.clone(), scope: put.scope.clone(), text: String::new(), base: String::new(), unfit: None, changes: Vec::new(), started_at: now.clone(), updated_at: now });
                held.len() - 1
            });
            if held[at].changes.last().map(|last| last.text.as_str()) != Some(put.text.as_str()) {
                held[at].changes.push(self.0.change(&put.text, DraftBy::You, put.summary.clone()));
            }
            held[at].text = put.text.clone();
            let answer = held[at].clone();
            self.0.piece_puts.borrow_mut().push(put);
            Task::ready(Ok(answer))
        }

        fn discard_piece_draft(&self, kind: LifecyclePieceKind, id: &str, scope: &PieceScope, _: &mut gpui::App) -> Task<Result<ipc::PieceDraftDiscarded, Refused>> {
            self.0.asked.borrow_mut().push("DELETE piece");
            self.0.pieces.borrow_mut().retain(|one| !(one.id == id && one.scope == *scope));
            Task::ready(Ok(ipc::PieceDraftDiscarded { kind, id: id.into(), scope: scope.clone() }))
        }

        fn publish_piece_draft(&self, publish: PublishPieceDraft, _: &mut gpui::App) -> Task<Result<ipc::PiecePublished, Refused>> {
            self.0.asked.borrow_mut().push("PUBLISH piece");
            self.0.pieces.borrow_mut().retain(|one| !(one.id == publish.id && one.scope == publish.scope));
            let saved = ipc::LifecyclePieceSaved { kind: publish.kind, id: publish.id.clone(), scope: publish.scope.clone(), file: "ticket.yml".into(), replaced: true };
            self.0.piece_publishes.borrow_mut().push(publish);
            Task::ready(Ok(ipc::PiecePublished { saved }))
        }

        fn delete_piece(&self, _: ipc::DeleteLifecyclePiece, _: &mut gpui::App) -> Task<Result<ipc::LifecyclePieceDeleted, Refused>> {
            Task::ready(Err(Refused::said("not here")))
        }
    }

    fn open_as<'a>(cx: &'a mut TestAppContext, shipped: bool, source: &'static str) -> (Entity<LanesEditor>, Rc<Fleet>, Rc<RefCell<Vec<EditorEvent>>>, &'a mut VisualTestContext) {
        cx.update(|cx| {
            gpui_component::init(cx);
            crate::look::init(cx);
        });
        let fleet = Rc::new(Fleet {
            definition: RefCell::new(BASE.into()),
            draft: RefCell::default(),
            pieces: RefCell::default(),
            last: Cell::new(0),
            asked: RefCell::default(),
            puts: RefCell::default(),
            piece_puts: RefCell::default(),
            publishes: RefCell::default(),
            piece_publishes: RefCell::default(),
            refuses_publish: RefCell::default(),
        });
        let subject = Subject { workflow: "prototype".into(), level: "default".into(), manifest: "armada".into(), held_under: "armada".into(), source, shipped, repository: "armada".into(), project: None };
        let wire: Rc<dyn EditorWire> = Rc::new(FakeWire(fleet.clone()));
        let (editor, cx) = cx.add_window_view(|window, cx| LanesEditor::new(subject, wire, window, cx));
        let seen = Rc::new(RefCell::new(Vec::new()));
        let heard = seen.clone();
        cx.update(|_, cx| cx.subscribe(&editor, move |_, event: &EditorEvent, _| heard.borrow_mut().push(event.clone())).detach());
        cx.run_until_parked();
        (editor, fleet, seen, cx)
    }

    fn open(cx: &mut TestAppContext, shipped: bool) -> (Entity<LanesEditor>, Rc<Fleet>, Rc<RefCell<Vec<EditorEvent>>>, &mut VisualTestContext) {
        open_as(cx, shipped, "armada")
    }

    fn drawn(editor: &Entity<LanesEditor>, cx: &mut VisualTestContext, node: &str) -> Option<(u32, Option<String>)> {
        editor.read_with(cx, |editor, cx| editor.lanes_of(&"".into(), cx).and_then(|lanes| lanes.lanes.iter().flat_map(|lane| lane.nodes.iter()).find(|one| one.id == node).map(|one| (one.col, one.changed.clone()))))
    }

    /// The Changes list, by who made each change and the line it reads.
    fn history(editor: &Entity<LanesEditor>, cx: &mut VisualTestContext) -> Vec<(DraftBy, String)> {
        cx.run_until_parked();
        editor.read_with(cx, |editor, _| editor.rows.iter().map(|row| (row.by, row.said.clone())).collect())
    }

    fn text(editor: &Entity<LanesEditor>, cx: &mut VisualTestContext) -> String {
        editor.read_with(cx, |editor, _| editor.text.clone())
    }

    fn undo_last(editor: &Entity<LanesEditor>, cx: &mut VisualTestContext) -> bool {
        let undone = cx.update(|window, cx| editor.update(cx, |editor, cx| editor.undo_last(window, cx)));
        cx.run_until_parked();
        undone
    }

    fn undo_at(editor: &Entity<LanesEditor>, cx: &mut VisualTestContext, at: usize) -> bool {
        let undone = cx.update(|window, cx| editor.update(cx, |editor, cx| editor.undo_at(at, window, cx)));
        cx.run_until_parked();
        undone
    }

    fn person(said: &str) -> (DraftBy, String) {
        (DraftBy::You, said.to_string())
    }

    fn rename(step: &str, label: &str) -> Change {
        Change::StepLabel { step: step.into(), label: label.into() }
    }

    fn steps(text: &str) -> Vec<String> {
        crate::settings::definition::steps_of(text).into_iter().map(|(_, label)| label).collect()
    }

    #[gpui::test]
    fn each_edit_is_one_put_of_the_whole_text_and_nothing_is_published(cx: &mut TestAppContext) {
        let (editor, fleet, _, cx) = open(cx, false);
        editor.update(cx, |editor, cx| {
            editor.edit("Rename Build", rename("build", "Make"), cx);
            editor.edit("Words for Plan", Change::StepWords { step: "plan".into(), words: "Plan small.".into() }, cx);
            editor.edit("Added New step after Land", Change::AddStep { after: "land".into(), id: "new_step".into(), label: NEW_STEP.into() }, cx);
        });
        assert_eq!(history(&editor, cx), [person("Rename Build"), person("Words for Plan"), person("Added New step after Land")]);
        let puts = fleet.puts.borrow();
        assert_eq!(puts.len(), 3, "a put for each edit");
        assert_eq!((puts[0].workflow_id.as_str(), puts[0].source.as_str(), puts[0].project.clone(), puts[0].summary.as_deref()), ("prototype", "armada", None, Some("Rename Build")));
        assert_eq!(steps(&puts[0].text), ["Plan", "Make", "Land"]);
        assert_eq!(steps(&puts[2].text), ["Plan", "Make", "Land", NEW_STEP], "each put is the whole text, edits so far in it");
        assert_eq!(crate::settings::definition::words_of(&puts[2].text, "plan").as_deref(), Some("Plan small."));
        assert!(fleet.publishes.borrow().is_empty(), "nothing is written before Publish");
        assert_eq!(fleet.draft.borrow().as_ref().map(|draft| draft.text.clone()), Some(puts[2].text.clone()));
    }

    #[gpui::test]
    fn command_z_puts_the_text_before_the_last_change_and_the_list_shrinks_until_nothing_is_left(cx: &mut TestAppContext) {
        let (editor, fleet, _, cx) = open(cx, false);
        editor.update(cx, |editor, cx| {
            editor.edit("Rename Build", rename("build", "Make"), cx);
            editor.edit("Rename Land", rename("land", "Ship"), cx);
        });
        assert_eq!(history(&editor, cx).len(), 2);
        assert!(undo_last(&editor, cx));
        {
            let puts = fleet.puts.borrow();
            assert_eq!(puts.len(), 3);
            assert_eq!(puts[2].text, puts[0].text, "the text before it is put again");
            assert_eq!(puts[2].summary, None);
        }
        assert_eq!(history(&editor, cx), [person("Rename Build")], "the change put back and the put are not shown");
        assert_eq!(steps(&text(&editor, cx)), ["Plan", "Make", "Land"]);
        // Back to what is published, and no piece draft held: the draft is deleted, not kept empty.
        assert!(undo_last(&editor, cx));
        assert_eq!(fleet.asked.borrow().last(), Some(&"DELETE workflow"));
        assert!(fleet.draft.borrow().is_none());
        assert!(!editor.read_with(cx, |editor, _| editor.has_edits()));
        assert_eq!(text(&editor, cx), BASE);
        assert!(!undo_last(&editor, cx), "nothing left to take back");
    }

    #[gpui::test]
    fn a_change_in_the_middle_is_undone_on_top_of_the_text_now_and_leaves_the_list_as_if_it_never_was(cx: &mut TestAppContext) {
        let (editor, fleet, _, cx) = open(cx, false);
        editor.update(cx, |editor, cx| {
            editor.edit("Rename Build", rename("build", "Make"), cx);
            editor.edit("Words for Plan", Change::StepWords { step: "plan".into(), words: "Plan small.".into() }, cx);
            editor.edit("Rename Land", rename("land", "Ship"), cx);
        });
        assert_eq!(history(&editor, cx).len(), 3);
        assert!(undo_at(&editor, cx, 1));
        {
            let puts = fleet.puts.borrow();
            assert_eq!(puts.len(), 4);
            let undone = fleet.draft.borrow().as_ref().map(|draft| draft.changes[1].id).expect("a draft");
            assert_eq!(puts[3].summary, Some(format!("Undid #{undone}")), "Fleet is told which change it took back");
        }
        assert_eq!(history(&editor, cx), [person("Rename Build"), person("Rename Land")], "both the row and the undoing are left out");
        let now = text(&editor, cx);
        assert_eq!(steps(&now), ["Plan", "Make", "Ship"]);
        assert_eq!(crate::settings::definition::words_of(&now, "plan"), None, "the words are back as they were");
        assert_eq!(drawn(&editor, cx, "workflow/build"), Some((1, Some("Renamed from Build".into()))));
    }

    #[gpui::test]
    fn a_change_the_one_after_it_edits_the_same_words_of_cannot_be_undone_from_the_middle(cx: &mut TestAppContext) {
        let (editor, fleet, _, cx) = open(cx, false);
        let words = |words: &str| Change::StepWords { step: "plan".into(), words: words.into() };
        editor.update(cx, |editor, cx| {
            editor.edit("First words", words("Plan small."), cx);
            editor.edit("Second words", words("Plan smaller."), cx);
            editor.edit("Rename Land", rename("land", "Ship"), cx);
        });
        assert_eq!(history(&editor, cx).len(), 3);
        let blocked: Vec<bool> = editor.read_with(cx, |editor, _| editor.rows.iter().map(|row| row.blocked).collect());
        assert_eq!(blocked, [true, false, false], "the second edits the first's words");
        let puts = fleet.puts.borrow().len();
        assert!(!undo_at(&editor, cx, 0));
        assert_eq!(fleet.puts.borrow().len(), puts, "nothing is put");
        assert!(undo_at(&editor, cx, 1));
        let blocked: Vec<bool> = editor.read_with(cx, |editor, _| editor.rows.iter().map(|row| row.blocked).collect());
        assert_eq!(blocked, [false, false], "with the second gone the first can go");
    }

    #[gpui::test]
    fn an_agents_change_fleet_drafted_is_read_not_put_and_one_it_did_not_is_laid_on_top_and_put(cx: &mut TestAppContext) {
        let (editor, fleet, _, cx) = open(cx, true);
        assert_eq!(drawn(&editor, cx, "workflow/review"), None);
        // Fleet put the change in the draft as the agent's, and the list it polls said so.
        fleet.put(REVIEWED, DraftBy::Agent, Some("Add a Review step after Build".into()));
        editor.update(cx, |editor, cx| editor.drafted_by_agent(cx));
        assert_eq!(history(&editor, cx), [(DraftBy::Agent, "Add a Review step after Build".to_string())]);
        assert_eq!(drawn(&editor, cx, "workflow/review"), Some((2, Some("Added".into()))));
        assert!(fleet.puts.borrow().is_empty(), "the draft is read again, not put");
        editor.read_with(cx, |editor, _| assert_eq!(editor.target, None, "a shipped workflow asks where it applies"));
        // A change that is ready and not in the draft (the draft moved) is laid on the text now and put as his.
        editor.update(cx, |editor, cx| editor.agent_edit("Rename Land to Ship", REVIEWED.into(), REVIEWED.replace("\"Land\"", "\"Ship\""), cx));
        assert_eq!(history(&editor, cx), [(DraftBy::Agent, "Add a Review step after Build".to_string()), person("Rename Land to Ship")]);
        assert_eq!(fleet.puts.borrow().len(), 1);
        assert_eq!(steps(&fleet.puts.borrow()[0].text), ["Plan", "Build", "Review", "Ship"]);
        // Command Z takes the agent's own change back last, as any other.
        assert!(undo_last(&editor, cx));
        assert!(undo_last(&editor, cx));
        assert_eq!(drawn(&editor, cx, "workflow/review"), None);
        assert!(!editor.read_with(cx, |editor, _| editor.has_edits()));
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
        assert_eq!(history(&editor, cx), [person("Renamed Build to Make"), person("Changed the words of Make")]);
        let drafted = text(&editor, cx);
        assert_eq!(crate::settings::definition::steps_of(&drafted)[1], ("build".to_string(), "Make".to_string()));
        assert_eq!(crate::settings::definition::words_of(&drafted, "build").as_deref(), Some("Build it small."));
        assert!(editor.read_with(cx, |editor, _| editor.in_place.is_none()));
        assert!(cx.update(|window, cx| editor.read(cx).canvas_focused(window)), "the keys are back on the canvas");
        assert_eq!(drawn(&editor, cx, "workflow/build"), Some((1, Some("Renamed from Build".into()))));
        assert!(fleet.publishes.borrow().is_empty(), "nothing is written before Publish");
    }

    #[gpui::test]
    fn a_field_left_as_it_was_makes_no_edit_and_esc_drops_what_was_typed(cx: &mut TestAppContext) {
        let (editor, _, _, cx) = open(cx, false);
        cx.update(|window, cx| editor.update(cx, |editor, cx| editor.open_at("workflow/land", true, window, cx)));
        type_in(&editor, cx, None, Some("Land it."));
        assert!(key(&editor, cx, "enter"));
        assert_eq!(history(&editor, cx), [person("Changed the words of Land")]);
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
    fn the_end_boxs_key_adds_one_step_after_the_last_and_picks_it(cx: &mut TestAppContext) {
        let (editor, _, _, cx) = open(cx, false);
        cx.update(|window, cx| editor.update(cx, |editor, cx| editor.lanes_add_at_end(&"".into(), "workflow", window, cx)));
        assert_eq!(history(&editor, cx), [person("Added New step after Land")]);
        assert_eq!(editor.read_with(cx, |editor, _| editor.picked.clone()), Some(Pick::Node("workflow/new_step".into())));
        assert!(cx.update(|window, cx| editor.read(cx).typing(window, cx)), "the keys are in its Name");
    }

    #[gpui::test]
    fn a_plus_adds_a_step_after_the_gaps_first_with_its_name_open_and_both_undo_alone(cx: &mut TestAppContext) {
        let (editor, _, _, cx) = open(cx, false);
        cx.update(|window, cx| editor.update(cx, |editor, cx| editor.add_after(Some("build".into()), window, cx)));
        assert_eq!(history(&editor, cx), [person("Added New step after Build")]);
        assert_eq!(drawn(&editor, cx, "workflow/new_step"), Some((2, Some("Added".into()))));
        editor.read_with(cx, |editor, cx| {
            let one = editor.in_place.as_ref().expect("its fields are open");
            assert_eq!(one.node, "workflow/new_step");
            assert_eq!(one.fields.as_ref().map(|fields| fields.name.read(cx).value().to_string()).as_deref(), Some(NEW_STEP));
        });
        assert!(cx.update(|window, cx| editor.read(cx).typing(window, cx)), "the keys are in its Name");
        type_in(&editor, cx, Some("Review"), None);
        assert!(key(&editor, cx, "enter"));
        assert_eq!(history(&editor, cx)[1], person("Renamed New step to Review"));
        assert!(undo_at(&editor, cx, 1));
        assert_eq!(crate::settings::definition::steps_of(&text(&editor, cx))[2], ("new_step".to_string(), NEW_STEP.to_string()));
        assert!(undo_last(&editor, cx));
        assert_eq!(drawn(&editor, cx, "workflow/new_step"), None);
        // With none named, it goes after the last step.
        cx.update(|window, cx| editor.update(cx, |editor, cx| editor.add_after(None, window, cx)));
        assert_eq!(crate::settings::definition::steps_of(&text(&editor, cx)).last().map(|(id, _)| id.clone()).as_deref(), Some("new_step"));
    }

    fn label_of(editor: &Entity<LanesEditor>, cx: &mut VisualTestContext, node: &str) -> Option<(String, Option<String>)> {
        editor.read_with(cx, |editor, cx| editor.lanes_of(&"".into(), cx).and_then(|lanes| lanes.lanes.iter().flat_map(|lane| lane.nodes.iter()).find(|one| one.id == node).map(|one| (one.label.clone(), one.changed.clone()))))
    }

    #[gpui::test]
    fn a_picked_state_opens_its_name_and_a_rename_is_a_change_of_his_in_its_lifecycles_draft_apart_from_the_workflows(cx: &mut TestAppContext) {
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
        assert_eq!(history(&editor, cx), [person("Renamed In Progress to Working")]);
        assert_eq!(text(&editor, cx), BASE, "the workflow's text is not the lifecycle's");
        assert!(fleet.puts.borrow().is_empty() && fleet.draft.borrow().is_none(), "the workflow has no draft");
        assert_eq!(label_of(&editor, cx, "ticket/in_progress"), Some(("Working".into(), Some("Renamed from In Progress".into()))));
        assert!(undo_last(&editor, cx));
        assert_eq!(label_of(&editor, cx, "ticket/in_progress"), Some(("In Progress".into(), None)), "its Undo takes the name back");
        assert!(fleet.pieces.borrow().is_empty(), "back to what Kit holds, the piece draft is deleted");
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
        assert_eq!(history(&editor, cx), [person("Renamed Plan to Think")], "a pick let go lands what was typed");
        assert!(editor.read_with(cx, |editor, _| editor.in_place.is_none()));
        assert!(fleet.publishes.borrow().is_empty() && fleet.piece_publishes.borrow().is_empty(), "nothing is written before Publish");
    }

    #[gpui::test]
    fn a_step_added_is_picked_and_a_adds_the_next_after_it(cx: &mut TestAppContext) {
        let (editor, _, _, cx) = open(cx, false);
        cx.update(|window, cx| editor.update(cx, |editor, cx| editor.add_after(Some("build".into()), window, cx)));
        assert_eq!(editor.read_with(cx, |editor, _| editor.picked.clone()), Some(Pick::Node("workflow/new_step".into())));
        assert!(key(&editor, cx, "enter"));
        cx.run_until_parked();
        assert!(cx.update(|window, cx| editor.update(cx, |editor, cx| editor.add_after_picked(window, cx))));
        let ids: Vec<String> = crate::settings::definition::steps_of(&text(&editor, cx)).into_iter().map(|(id, _)| id).collect();
        assert_eq!(ids, ["plan", "build", "new_step", "new_step_2", "land"]);
    }

    /// A state renamed and a step renamed.
    fn rename_both(editor: &Entity<LanesEditor>, cx: &mut VisualTestContext) {
        editor.update(cx, |editor, cx| {
            editor.edit("Renamed Build to Make", rename("build", "Make"), cx);
            editor.edit("Renamed In Progress to Working", Change::StateLabel { lifecycle: "ticket".into(), state: "in_progress".into(), label: "Working".into() }, cx);
        });
        cx.run_until_parked();
    }

    #[gpui::test]
    fn a_rename_is_put_as_the_lifecycles_draft_at_the_scope_the_draft_applies_at(cx: &mut TestAppContext) {
        let (editor, fleet, _, cx) = open(cx, false);
        // The question is asked once there is a change; a repository's own copy answers it.
        editor.update(cx, |editor, cx| editor.edit("Renamed Build to Make", rename("build", "Make"), cx));
        editor.update(cx, |editor, cx| editor.apply_at(Target::Repository("armada".into()), cx));
        editor.update(cx, |editor, cx| editor.edit("Renamed In Progress to Working", Change::StateLabel { lifecycle: "ticket".into(), state: "in_progress".into(), label: "Working".into() }, cx));
        cx.run_until_parked();
        {
            let puts = fleet.piece_puts.borrow();
            assert_eq!(puts.len(), 1);
            assert_eq!((puts[0].kind, puts[0].id.as_str(), puts[0].scope.clone(), puts[0].summary.as_deref()), (LifecyclePieceKind::Lifecycle, "ticket", PieceScope::Within { id: "armada".into() }, Some("Renamed In Progress to Working")));
            let lifecycle = lifecycle_of(&puts[0].text).expect("the lifecycle's piece, as a save takes it");
            assert_eq!(lifecycle.states.iter().map(|one| one.label.as_str()).collect::<Vec<_>>(), ["Todo", "Working", "Done"], "the one that runs, renamed");
            assert_eq!(lifecycle.scope, PieceScope::Within { id: "armada".into() });
        }
        assert_eq!(history(&editor, cx), [person("Renamed Build to Make"), person("Renamed In Progress to Working")], "the two drafts share one list, in the order Fleet numbered");
    }

    #[gpui::test]
    fn a_rename_on_armadas_own_waits_for_where_it_applies_then_is_put_and_moves_with_the_answer(cx: &mut TestAppContext) {
        let (editor, fleet, _, cx) = open(cx, true);
        editor.update(cx, |editor, cx| editor.edit("Renamed In Progress to Working", Change::StateLabel { lifecycle: "ticket".into(), state: "in_progress".into(), label: "Working".into() }, cx));
        assert_eq!(history(&editor, cx), [person("Renamed In Progress to Working")], "held here, and listed");
        assert!(fleet.piece_puts.borrow().is_empty(), "no scope to put it at");
        assert_eq!(label_of(&editor, cx, "ticket/in_progress").map(|one| one.0), Some("Working".into()));
        editor.update(cx, |editor, cx| editor.apply_at(Target::Repository("armada".into()), cx));
        cx.run_until_parked();
        assert_eq!(fleet.piece_puts.borrow().iter().map(|put| put.scope.clone()).collect::<Vec<_>>(), [PieceScope::Within { id: "armada".into() }]);
        assert_eq!(history(&editor, cx), [person("Renamed In Progress to Working")]);
        // The answer changed: the draft is thrown away and put again at the new scope.
        editor.update(cx, |editor, cx| editor.apply_at(Target::EveryProject, cx));
        cx.run_until_parked();
        assert_eq!(fleet.asked.borrow()[..], ["PUT piece", "DELETE piece", "PUT piece"]);
        assert_eq!(fleet.pieces.borrow().iter().map(|one| one.scope.clone()).collect::<Vec<_>>(), [PieceScope::Everywhere]);
        assert_eq!(history(&editor, cx), [person("Renamed In Progress to Working")]);
    }

    #[gpui::test]
    fn discard_deletes_the_workflows_draft_and_each_piece_draft_and_reads_again(cx: &mut TestAppContext) {
        let (editor, fleet, _, cx) = open(cx, false);
        rename_both(&editor, cx);
        assert!(fleet.draft.borrow().is_some() && !fleet.pieces.borrow().is_empty());
        cx.update(|window, cx| editor.update(cx, |editor, cx| editor.discard(window, cx)));
        cx.run_until_parked();
        assert_eq!(fleet.asked.borrow()[2..], ["DELETE workflow", "DELETE piece"]);
        assert!(fleet.draft.borrow().is_none() && fleet.pieces.borrow().is_empty());
        assert!(!editor.read_with(cx, |editor, _| editor.has_edits()));
        assert_eq!(text(&editor, cx), BASE);
        assert_eq!(drawn(&editor, cx, "workflow/build"), Some((1, None)));
    }

    #[gpui::test]
    fn publish_writes_the_workflow_draft_then_each_piece_draft_and_says_where_only_for_armadas(cx: &mut TestAppContext) {
        let (editor, fleet, seen, cx) = open(cx, true);
        editor.update(cx, |editor, cx| editor.agent_edit("Add a Review step after Build", BASE.into(), REVIEWED.into(), cx));
        editor.update(cx, |editor, cx| editor.publish(cx));
        cx.run_until_parked();
        assert!(fleet.publishes.borrow().is_empty(), "no place answered");
        editor.update(cx, |editor, cx| editor.apply_at(Target::EveryProject, cx));
        editor.update(cx, |editor, cx| editor.edit("Renamed In Progress to Working", Change::StateLabel { lifecycle: "ticket".into(), state: "in_progress".into(), label: "Working".into() }, cx));
        editor.update(cx, |editor, cx| editor.publish(cx));
        cx.run_until_parked();
        {
            let publishes = fleet.publishes.borrow();
            assert_eq!(publishes.len(), 1);
            let (manifest, publish) = &publishes[0];
            assert_eq!(manifest, "armada");
            assert_eq!((publish.workflow_id.as_str(), publish.source.as_str(), publish.project.clone(), publish.scope.clone(), publish.overwrite), ("prototype", "armada", None, Some(WorkflowScope::Kit), false));
        }
        assert_eq!(fleet.asked.borrow().iter().rev().take(2).rev().copied().collect::<Vec<_>>(), ["PUBLISH workflow", "PUBLISH piece"]);
        assert_eq!(fleet.piece_publishes.borrow().iter().map(|one| (one.id.clone(), one.scope.clone())).collect::<Vec<_>>(), [("ticket".to_string(), PieceScope::Everywhere)]);
        assert_eq!(*seen.borrow(), [EditorEvent::Published { target: Target::EveryProject }]);
        assert!(!editor.read_with(cx, |editor, _| editor.has_edits()), "the draft is let go once it is written");
    }

    #[gpui::test]
    fn a_workflow_that_is_not_armadas_is_published_in_its_own_place_with_no_scope(cx: &mut TestAppContext) {
        let (editor, fleet, _, cx) = open_as(cx, false, "repository");
        editor.update(cx, |editor, cx| editor.edit("Rename Build", rename("build", "Make"), cx));
        editor.update(cx, |editor, cx| editor.publish(cx));
        cx.run_until_parked();
        let publishes = fleet.publishes.borrow();
        assert_eq!((publishes[0].0.as_str(), publishes[0].1.source.as_str(), publishes[0].1.scope.clone()), ("armada", "repository", None));
    }

    #[gpui::test]
    fn a_publish_fleet_refuses_keeps_the_draft_and_says_its_sentence_and_a_stale_one_offers_to_start_over(cx: &mut TestAppContext) {
        let (editor, fleet, seen, cx) = open(cx, false);
        editor.update(cx, |editor, cx| editor.edit("Rename Build", rename("build", "Make"), cx));
        *fleet.refuses_publish.borrow_mut() = Some(refused("fleet.workflow_exists", "Kit already holds a workflow `prototype`"));
        editor.update(cx, |editor, cx| editor.publish(cx));
        cx.run_until_parked();
        editor.read_with(cx, |editor, _| {
            assert_eq!((editor.refused.as_deref(), editor.stale), (Some("Kit already holds a workflow `prototype`"), false), "just the sentence");
            assert!(editor.has_edits(), "the draft stays");
            assert!(!editor.stops().contains(&editor.keys.restart));
        });
        *fleet.refuses_publish.borrow_mut() = Some(refused("fleet.workflow_draft_stale", "This workflow changed since the draft was started"));
        editor.update(cx, |editor, cx| editor.publish(cx));
        cx.run_until_parked();
        editor.read_with(cx, |editor, _| {
            assert_eq!((editor.refused.as_deref(), editor.stale), (Some("This workflow changed since the draft was started"), true));
            assert!(editor.stops().contains(&editor.keys.restart), "Start from the published text is offered");
        });
        assert!(seen.borrow().is_empty());
        cx.update(|window, cx| editor.update(cx, |editor, cx| editor.pressed(Press::Restart, window, cx)));
        cx.run_until_parked();
        assert!(fleet.draft.borrow().is_none(), "it discards");
        editor.read_with(cx, |editor, _| {
            assert!(!editor.has_edits() && !editor.stale && editor.refused.is_none());
            assert_eq!(editor.text, BASE);
        });
    }
}
