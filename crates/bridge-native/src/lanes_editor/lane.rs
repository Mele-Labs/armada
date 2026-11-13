//! Add lifecycle: a menu of what a new lane follows (**What does this lane follow?**), and the lane a pick draws under the
//! others, with its usual states and its name box in its head, ready to rename. A new lane is a lifecycle the draft adds: it is
//! kept as a piece draft (`LifecyclePiece::Lifecycle`) like a rule is, with the line that says what changed, so Changes, ⌘Z,
//! Discard and Publish take it as they take any edit.
//!
//! **A Blank lane** has no usual states: its name box is empty and holds the keys, and its press **Add a state** adds one to
//! name in place. It stays out of the draft until it has a name and a state, as a rule's sentence stays out until it has a
//! moment and an act.
//!
//! Keys: ↑ and ↓ in the menu, Enter or Space picks, Esc shuts it and the keys go back to the button. In a new lane, Tab
//! reaches the name box and the press.

use gpui::{anchored, canvas, deferred, div, point, prelude::*, px, svg, Anchor, AnyElement, Bounds, Context, Entity, FocusHandle, Focusable, FontWeight, Keystroke, MouseButton, Pixels, Role, SharedString, Stateful, Subscription, Window};
use gpui_component::input::{InputEvent, Textarea, TextareaState};
use gpui_component::{Sizable, Size};
use ipc::{LaneNodeKind, LifecycleDefinition, LifecycleStateDefinition, LifecycleTransition, PieceScope, StateReadFrom};

use super::in_place::{lanes_edge, node_bounds, unique_id};
use super::LanesEditor;
use crate::assets::icon_path;
use crate::labels::labelled;
use crate::lanes_canvas::mix;
use crate::look::{token, Look};
use crate::settings::view::{bounds_of, ring_colour};

/// What the menu asks, and its name.
pub(super) const ASK: &str = "What does this lane follow?";

/// The name a state added to a blank lane has until it is renamed.
pub(super) const NEW_STATE: &str = "New state";

/// What a blank lane's name box says while it is empty.
/// A state node's height over its width, as the canvas draws one.
const STATE_PER_NODE: f32 = 56. / 168.;
/// The lines a name may take in its box before it scrolls.
const NAME_ROWS: usize = 3;
const NAME_THIS: &str = "Lane name";

/// The source word under the name of a lane that reads its states from nothing the wire names.
const YOUR_OWN: &str = "Your own";

/// What a new lane follows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Kind {
    PullRequest,
    Ticket,
    Watch,
    Job,
    Blank,
}

/// One row of the menu: the kind, its name, where its states come from in quiet words, and its glyph.
pub(super) struct KindRow {
    pub kind: Kind,
    pub name: &'static str,
    pub says: &'static str,
    pub glyph: &'static str,
}

/// The menu's rows, in order: Blank lane is always the last.
pub(super) const KINDS: [KindRow; 5] = [
    KindRow { kind: Kind::PullRequest, name: "Pull request", says: "Draft to Merged, from your forge", glyph: "git-pull-request" },
    KindRow { kind: Kind::Ticket, name: "Ticket", says: "States from your tracker", glyph: "ticket" },
    KindRow { kind: Kind::Watch, name: "Watch", says: "States a watch's command reports", glyph: "activity" },
    KindRow { kind: Kind::Job, name: "Job", says: "Started to Ended, from Fleet's own events", glyph: "bot" },
    KindRow { kind: Kind::Blank, name: "Blank lane", says: "States you name yourself", glyph: "plus" },
];

/// A pull request's usual states, `(id, label)`: `StateReadFrom::Forge`'s `fact`s, as the variant's doc comment lists them. The id
/// is the fact.
const FORGE_STATES: [(&str, &str); 4] = [("draft", "Draft"), ("in_review", "In review"), ("approved", "Approved"), ("merged", "Merged")];

/// A ticket's usual states: the tracker's own words, which a state reads (`StateReadFrom::Tracker`).
const TRACKER_STATES: [(&str, &str); 4] = [("todo", "Todo"), ("in_progress", "In Progress"), ("in_review", "In Review"), ("done", "Done")];

/// A watch's states: `StateReadFrom::Watch`'s `state`s, as the variant's doc comment lists them. The id is the state.
const WATCH_STATES: [(&str, &str); 4] = [("waiting", "Waiting"), ("running", "Running"), ("passed", "Passed"), ("failed", "Failed")];

/// A Job's usual states: `StateReadFrom::Fleet`'s `fact`s, without `step_entered:<step id>`, as the variant's doc comment
/// lists them. The id is the fact.
const FLEET_STATES: [(&str, &str); 4] = [("started", "Started"), ("pull_request_opened", "Pull request opened"), ("landed", "Landed"), ("ended", "Ended")];

/// The lane a kind draws, with its usual states in order, the moves expected between them and the states that end it.
/// A blank lane has none, and no name. It is a Tracker lane whose states carry no words, so every value is unmapped.
pub(super) fn lifecycle_of_kind(kind: Kind, id: &str, scope: PieceScope) -> LifecycleDefinition {
    let state = |id: &str, label: &str, read: StateReadFrom| LifecycleStateDefinition { id: id.into(), label: label.into(), read };
    let states: Vec<LifecycleStateDefinition> = match kind {
        Kind::PullRequest => FORGE_STATES.iter().map(|(fact, label)| state(fact, label, StateReadFrom::Forge { fact: (*fact).into() })).collect(),
        Kind::Ticket => TRACKER_STATES.iter().map(|(id, label)| state(id, label, StateReadFrom::Tracker { words: vec![(*label).into()] })).collect(),
        Kind::Watch => WATCH_STATES.iter().map(|(watched, label)| state(watched, label, StateReadFrom::Watch { state: (*watched).into() })).collect(),
        Kind::Job => FLEET_STATES.iter().map(|(fact, label)| state(fact, label, StateReadFrom::Fleet { fact: (*fact).into() })).collect(),
        Kind::Blank => Vec::new(),
    };
    let pairs: Vec<(String, String)> = match kind {
        // A watch that runs ends in one of two.
        Kind::Watch => vec![("waiting".into(), "running".into()), ("running".into(), "passed".into()), ("running".into(), "failed".into())],
        _ => states.windows(2).map(|pair| (pair[0].id.clone(), pair[1].id.clone())).collect(),
    };
    let ends: Vec<String> = match kind {
        Kind::Watch => vec!["passed".into(), "failed".into()],
        _ => states.last().map(|last| last.id.clone()).into_iter().collect(),
    };
    let name = KINDS.iter().find(|row| row.kind == kind).filter(|_| kind != Kind::Blank).map_or("", |row| row.name);
    LifecycleDefinition { id: id.into(), name: name.into(), scope, states, transitions: pairs.into_iter().map(|(from, to)| LifecycleTransition { from, to }).collect(), ends }
}

/// The lane with one state more, named `NEW_STATE`, after the last and ending it; and that state's id.
pub(super) fn with_state_added(mut lane: LifecycleDefinition) -> (LifecycleDefinition, String) {
    let taken: Vec<String> = lane.states.iter().map(|one| one.id.clone()).collect();
    let id = unique_id(NEW_STATE, &taken);
    if let Some(last) = lane.states.last() {
        lane.transitions.push(LifecycleTransition { from: last.id.clone(), to: id.clone() });
    }
    let read = StateReadFrom::Own;
    lane.states.push(LifecycleStateDefinition { id: id.clone(), label: NEW_STATE.into(), read });
    lane.ends = vec![id.clone()];
    (lane, id)
}

/// The word under a lane's name: where its states are read from.
pub(super) fn source_of(lane: &LifecycleDefinition) -> &'static str {
    match lane.states.first().map(|state| &state.read) {
        Some(StateReadFrom::Forge { .. }) => "Forge",
        Some(StateReadFrom::Watch { .. }) => "Watch",
        Some(StateReadFrom::Fleet { .. }) => "Fleet",
        Some(StateReadFrom::Tracker { words }) if !words.is_empty() => "Tracker",
        _ => YOUR_OWN,
    }
}

/// What the canvas tints a state with.
pub(super) fn node_kind(read: &StateReadFrom) -> LaneNodeKind {
    match read {
        StateReadFrom::Forge { .. } => LaneNodeKind::Forge,
        StateReadFrom::Watch { .. } => LaneNodeKind::Watch,
        StateReadFrom::Fleet { .. } => LaneNodeKind::Job,
        StateReadFrom::Tracker { .. } => LaneNodeKind::Tracker,
        StateReadFrom::Own => LaneNodeKind::Own,
    }
}

/// What a kind is tinted with: the tint its states wear on the canvas.
fn tint_of(look: &Look, kind: Kind) -> gpui::Hsla {
    look.color(match kind {
        Kind::PullRequest => token::KIND_DELIVERY,
        Kind::Ticket => token::KIND_SETUP,
        Kind::Watch => token::KIND_GATE,
        Kind::Job => token::KIND_STEP,
        Kind::Blank => token::FG_MUTED,
    })
}

/// The kind menu while it is open: the row the arrows are on, its own stop, and the button the keys go back to.
pub(super) struct KindMenu {
    at: usize,
    focus: FocusHandle,
    opener: FocusHandle,
}

/// What a lane just added holds in its head: the box its name is typed in, and the press that adds a state.
pub(super) struct NewLane {
    pub id: String,
    pub name: Entity<TextareaState>,
    /// The name box's own stop, kept so the lane's stops can name it without reading the box.
    name_focus: FocusHandle,
    pill: FocusHandle,
    /// The name as it last landed.
    was: String,
    _sub: Subscription,
}

impl NewLane {
    pub(super) fn new(id: String, name: &str, window: &mut Window, cx: &mut Context<LanesEditor>) -> Self {
        // A long name wraps in the box rather than running past it.
        let input = cx.new(|cx| TextareaState::new(window, cx).auto_grow(1, NAME_ROWS).placeholder(NAME_THIS));
        input.update(cx, |state, cx| state.set_value(name.to_string(), window, cx));
        let name_focus = input.read(cx).focus_handle(cx);
        // The keys leaving the box, or Enter in it, land what was typed.
        let sub = cx.subscribe_in(&input, window, |this, _, event: &InputEvent, window, cx| {
            if matches!(event, InputEvent::Blur) {
                this.settle_lane_names(false, window, cx);
            }
        });
        NewLane { id, name: input, name_focus, pill: cx.focus_handle().tab_stop(true), was: name.to_string(), _sub: sub }
    }
}

impl LanesEditor {
    // ---- the lanes the draft adds ----------------------------------------------------------------

    /// The lane the draft adds, or a blank one not yet in it, by its id.
    pub(super) fn lane_def(&self, id: &str) -> Option<LifecycleDefinition> {
        self.added.iter().chain(self.pending_lane.iter()).find(|one| one.id == id).cloned()
    }

    /// Every id a new lane must not take: the lifecycles that run, those the draft adds, and the lanes drawn.
    fn lane_ids(&self) -> Vec<String> {
        let mut ids: Vec<String> = self.lifecycles.iter().map(|one| one.lifecycle.id.clone()).collect();
        ids.extend(self.added.iter().chain(self.pending_lane.iter()).map(|one| one.id.clone()));
        ids.extend(self.pieces.iter().map(|one| one.id.clone()));
        if let super::Read::Read(view) = &self.read {
            ids.extend(view.lanes.iter().map(|lane| lane.id.clone()));
        }
        ids
    }

    /// What Fleet's piece drafts and the edits still waiting say the added lanes are. A piece of a lifecycle that runs, or of a
    /// lane the canvas already draws, renames a state of it and adds none.
    pub(super) fn sync_added(&mut self) {
        let drawn: Vec<&str> = match &self.read {
            super::Read::Read(view) => view.lanes.iter().map(|lane| lane.id.as_str()).collect(),
            _ => Vec::new(),
        };
        let mut now: Vec<LifecycleDefinition> = self
            .pieces
            .iter()
            // A draft Kit holds no lane under is a new lane, whether or not Fleet lists it among those that run.
            .filter(|piece| piece.kind == ipc::LifecyclePieceKind::Lifecycle && (piece.base.is_empty() || (!self.lifecycles.iter().any(|one| one.lifecycle.id == piece.id) && !drawn.contains(&piece.id.as_str()))))
            .filter_map(|piece| super::lifecycle_of(&piece.text))
            .collect();
        for waiting in &self.waiting_lifecycles {
            match now.iter_mut().find(|one| one.id == waiting.lifecycle.id) {
                Some(held) => *held = waiting.lifecycle.clone(),
                None => now.push(waiting.lifecycle.clone()),
            }
        }
        self.added = now;
    }

    /// One lane's edit: the whole lane as it is now, put as its piece draft at the scope the draft applies at, with the line the
    /// history says; where the draft applies is not known yet, it waits. A blank lane that has no name or no state yet is only
    /// held here.
    pub(super) fn edit_lifecycle(&mut self, said: String, lane: LifecycleDefinition, cx: &mut Context<Self>) {
        self.refused = None;
        self.stale = false;
        if self.pending_lane.as_ref().is_some_and(|held| held.id == lane.id) {
            self.pending_lane = Some(lane);
            return self.land_blank(cx);
        }
        match self.target.as_ref().map(super::piece_scope) {
            Some(scope) => {
                let lane = LifecycleDefinition { scope: scope.clone(), ..lane };
                self.hold_added(lane.clone());
                let text = super::piece_text(lane.clone());
                self.enqueue(super::Call::PutPiece { kind: ipc::LifecyclePieceKind::Lifecycle, id: lane.id, scope, text, said: Some(said) }, cx);
            }
            None => {
                self.hold_added(lane.clone());
                self.waiting_lifecycles.push(super::draft::WaitingLifecycle { said, lifecycle: lane });
            }
        }
        self.refresh(cx);
        cx.notify();
    }

    fn hold_added(&mut self, lane: LifecycleDefinition) {
        match self.added.iter_mut().find(|one| one.id == lane.id) {
            Some(held) => *held = lane,
            None => self.added.push(lane),
        }
    }

    /// A blank lane with a name and a state joins the draft, named for its name as a rule is for its act.
    fn land_blank(&mut self, cx: &mut Context<Self>) {
        let Some(lane) = self.pending_lane.clone().filter(|lane| !lane.name.trim().is_empty() && !lane.states.is_empty()) else {
            self.refresh(cx);
            cx.notify();
            return;
        };
        let was = lane.id.clone();
        let mut taken = self.lane_ids();
        taken.retain(|held| *held != was);
        let id = unique_id(&lane.name, &taken);
        for one in self.new_lanes.iter_mut().filter(|one| one.id == was) {
            one.id = id.clone();
        }
        self.pending_lane = None;
        let said = format!("Added the lane {}", lane.name);
        self.edit_lifecycle(said, LifecycleDefinition { id, ..lane }, cx);
    }

    /// A change that took a lane back: its lane is as `text` says, or gone where nothing is left; and a waiting edit taken off, as
    /// the last still waiting says.
    pub(super) fn lane_taken_back(&mut self, id: &str, text: Option<&str>) {
        let now = match text {
            Some(text) => (!text.is_empty()).then(|| super::lifecycle_of(text)).flatten(),
            None => self.waiting_lifecycles.iter().rev().find(|one| one.lifecycle.id == id).map(|one| one.lifecycle.clone()),
        };
        match now {
            Some(lane) => self.hold_added(lane),
            None => self.added.retain(|one| one.id != id),
        }
    }

    /// Every lane the draft added is gone, with what stood in its head.
    pub(super) fn lanes_dropped(&mut self) {
        self.waiting_lifecycles.clear();
        self.added.clear();
        self.pending_lane = None;
        self.new_lanes.clear();
        self.kinds = None;
    }

    // ---- the kind menu --------------------------------------------------------------------------

    /// Add lifecycle: the kinds a lane can follow, in a menu under the button, with the keys in it. Pressed while it is
    /// open, it shuts. `opener` is the button, which Esc gives the keys back to.
    pub fn add_lifecycle(&mut self, opener: FocusHandle, window: &mut Window, cx: &mut Context<Self>) {
        if self.kinds.take().is_some() {
            window.focus(&opener, cx);
            cx.notify();
            return;
        }
        self.settle(window, cx);
        self.in_place = None;
        self.sentence = None;
        let focus = cx.focus_handle();
        window.focus(&focus, cx);
        self.kinds = Some(KindMenu { at: 0, focus, opener });
        cx.notify();
    }

    fn close_kinds(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(menu) = self.kinds.take() {
            window.focus(&menu.opener, cx);
        }
        cx.notify();
    }

    /// The menu's keys while it is open: ↑ and ↓ between rows, Enter or Space picks, Esc shuts it. Any other key is the menu's too,
    /// so none reaches the page behind it; a key with ⌘, ⌃ or ⌥ is not.
    pub(super) fn kind_key(&mut self, key: &Keystroke, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let mods = &key.modifiers;
        let Some(menu) = self.kinds.as_mut() else { return false };
        if mods.platform || mods.control || mods.alt {
            return false;
        }
        let count = KINDS.len();
        match key.key.as_str() {
            "down" => menu.at = (menu.at + 1) % count,
            "up" => menu.at = (menu.at + count - 1) % count,
            "enter" | "space" => {
                let kind = KINDS[menu.at].kind;
                self.pick_kind(kind, window, cx);
                return true;
            }
            "escape" => {
                self.close_kinds(window, cx);
                return true;
            }
            _ => {}
        }
        cx.notify();
        true
    }

    /// A kind picked: a new lane under the others with its usual states, its name box holding the keys with the name selected.
    /// A blank lane has an empty box and no state.
    pub(super) fn pick_kind(&mut self, kind: Kind, window: &mut Window, cx: &mut Context<Self>) {
        self.kinds = None;
        self.settle(window, cx);
        self.in_place = None;
        self.sentence = None;
        self.state.pick(&self.id, None);
        self.picked = None;
        let scope = self.target.as_ref().map(super::piece_scope).unwrap_or(PieceScope::Everywhere);
        let Some(row) = KINDS.iter().find(|row| row.kind == kind) else { return };
        let taken = self.lane_ids();
        let name = if kind == Kind::Blank { "" } else { row.name };
        let id = unique_id(if kind == Kind::Blank { "new lane" } else { row.name }, &taken);
        let lane = lifecycle_of_kind(kind, &id, scope);
        let held = NewLane::new(id.clone(), name, window, cx);
        held.name.update(cx, |state, cx| {
            state.focus(window, cx);
            state.select_all(window, cx);
        });
        self.new_lanes.push(held);
        if kind == Kind::Blank {
            self.pending_lane = Some(lane);
            self.refresh(cx);
        } else {
            self.edit_lifecycle(format!("Added the lane {name}"), lane, cx);
        }
        self.show_lane.set(Some(id));
        self.reveal.set(true);
        cx.notify();
    }

    /// The menu, under the button it was opened from, or over it where the window has no room below; it never runs past the
    /// window's edges.
    pub(super) fn render_kinds(&self, look: &Look, window: &mut Window, cx: &Context<Self>) -> Option<AnyElement> {
        let menu = self.kinds.as_ref()?;
        let Some(button) = bounds_of(window, cx, Role::Button, "Add lifecycle") else {
            // The button is drawn this frame for the first time: the menu follows on the next one.
            window.on_next_frame(|window, _| window.refresh());
            return None;
        };
        let gap = look.length(token::SPACE_1);
        let view = window.viewport_size();
        let wide = (look.length(token::SPACE_12) * 7.).min((view.width - gap * 4.).max(px(0.)));
        let row_tall = look.length(token::SPACE_8) + look.length(token::SPACE_2);
        let head_tall = look.length(token::H_CONTROL_SM);
        let want = row_tall * KINDS.len() as f32 + head_tall + gap * 3. + look.length(token::BORDER_WIDTH) * 3.;
        let below = view.height - button.bottom() - gap * 2.;
        let above = button.top() - gap * 2.;
        let over = below < want && above > below;
        let room = if over { above } else { below }.max(look.length(token::SPACE_12));
        let header = div()
            .flex_none()
            .px(look.length(token::SPACE_2))
            .h(head_tall)
            .flex()
            .items_center()
            .text_size(look.length(token::TEXT_2XS))
            .font_family(crate::settings::view::mono(look))
            .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
            .text_color(look.color(token::FG_SUBTLE))
            .child(ASK);
        let mut rows: Vec<AnyElement> = Vec::new();
        for (n, one) in KINDS.iter().enumerate() {
            if one.kind == Kind::Blank {
                rows.push(div().flex_none().h(look.length(token::BORDER_WIDTH)).my(gap).bg(look.color(token::BORDER_SUBTLE)).into_any_element());
            }
            let tint = tint_of(look, one.kind);
            let chip = div()
                .flex_none()
                .size(look.length(token::SPACE_8))
                .flex()
                .items_center()
                .justify_center()
                .rounded(look.length(token::RADIUS_MD))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(mix(tint, look.color(token::BG_OVERLAY), 0.4))
                .bg(mix(tint, look.color(token::BG_OVERLAY), 0.16))
                .child(svg().path(icon_path(one.glyph)).size(look.length(token::SPACE_4)).text_color(tint));
            let words = div()
                .flex()
                .flex_col()
                .min_w_0()
                .child(div().text_size(look.length(token::TEXT_SM)).line_height(look.length(token::LEADING_SM)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).text_color(look.color(token::FG_DEFAULT)).child(one.name))
                .child(div().min_w_0().truncate().text_size(look.length(token::TEXT_XS)).line_height(look.length(token::LEADING_XS)).text_color(look.color(token::FG_SUBTLE)).child(one.says));
            let row = div()
                .id(SharedString::from(format!("lane-kind:{n}")))
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_3))
                .px(look.length(token::SPACE_2))
                .h(row_tall)
                .rounded(look.length(token::RADIUS_MD))
                .cursor_pointer()
                .when(menu.at == n, |row| row.bg(look.color(token::BG_HOVER)))
                .hover(|style| style.bg(look.color(token::BG_HOVER)))
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .on_click({
                    let kind = one.kind;
                    cx.listener(move |this, _, window, cx| this.pick_kind(kind, window, cx))
                })
                .child(chip)
                .child(words);
            rows.push(labelled(row.aria_selected(menu.at == n), Role::MenuItem, one.name).into_any_element());
        }
        let list = div()
            .id("lane-kinds")
            .track_focus(&menu.focus)
            .occlude()
            .w(wide)
            .max_h(room)
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .p(gap)
            .rounded(look.length(token::RADIUS_LG))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_DEFAULT))
            .bg(look.color(token::BG_OVERLAY))
            .shadow(look.shadows(token::SHADOW_OVERLAY))
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_mouse_down_out(cx.listener(|this, _, _, cx| {
                this.kinds = None;
                cx.notify();
            }))
            .child(header)
            .children(rows);
        let (at, corner) = if over { (point(button.right(), button.top() - gap), Anchor::BottomRight) } else { (point(button.right(), button.bottom() + gap), Anchor::TopRight) };
        Some(deferred(anchored().position(at).anchor(corner).snap_to_window_with_margin(gap).child(labelled(list, Role::Menu, ASK))).with_priority(2).into_any_element())
    }

    // ---- a lane's head ---------------------------------------------------------------------------

    /// The Tab stops of the lanes just added: the name box of each, then the press of a lane that has no states of its own.
    pub(super) fn lane_stops(&self) -> Vec<FocusHandle> {
        let mut stops = Vec::new();
        for one in &self.new_lanes {
            stops.push(one.name_focus.clone());
            if self.lane_def(&one.id).is_some_and(|lane| source_of(&lane) == YOUR_OWN) {
                stops.push(one.pill.clone());
            }
        }
        stops
    }

    /// Whether the keys are in a lane's name box.
    pub(super) fn lane_name_focused(&self, window: &Window) -> bool {
        self.new_lanes.iter().any(|one| one.name_focus.is_focused(window))
    }

    /// What was typed in a name box lands where the keys have left it, or with `done` on Enter: a name changed is one edit of the
    /// lane; one emptied is put back as it was.
    pub(super) fn settle_lane_names(&mut self, done: bool, window: &mut Window, cx: &mut Context<Self>) {
        for at in 0..self.new_lanes.len() {
            let (id, name, was) = {
                let one = &self.new_lanes[at];
                (one.id.clone(), one.name.read(cx).value().replace(['\n', '\r'], " ").trim().to_string(), one.was.clone())
            };
            // Still being typed: the keys are in the box and Enter has not been pressed.
            if name == was || (self.new_lanes[at].name_focus.is_focused(window) && !done) {
                continue;
            }
            let Some(mut lane) = self.lane_def(&id) else { continue };
            if name.is_empty() {
                self.new_lanes[at].name.update(cx, |state, cx| state.set_value(was, window, cx));
                continue;
            }
            let said = if was.is_empty() { format!("Named the lane {name}") } else { format!("Renamed the lane {was} to {name}") };
            self.new_lanes[at].was = name.clone();
            lane.name = name;
            self.edit_lifecycle(said, lane, cx);
        }
    }

    /// A name box not in use shows the lane's name as the draft has it now, which a change taken back may have moved.
    pub(super) fn sync_lane_names(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // A lane the draft held from before (one restored from Fleet) has no box yet: it gets the same one a new lane has.
        for lane in self.added.clone() {
            if !self.new_lanes.iter().any(|one| one.id == lane.id) {
                self.new_lanes.push(NewLane::new(lane.id.clone(), &lane.name, window, cx));
            }
        }
        for at in 0..self.new_lanes.len() {
            let one = &self.new_lanes[at];
            let Some(name) = self.lane_def(&one.id).map(|lane| lane.name).filter(|name| !name.is_empty()) else { continue };
            if one.name_focus.is_focused(window) || one.name.read(cx).value().as_ref() == name.as_str() {
                continue;
            }
            self.new_lanes[at].was = name.clone();
            self.new_lanes[at].name.update(cx, |state, cx| state.set_value(name, window, cx));
        }
    }

    /// The name box's keys: Enter lands the name and goes on to the press that adds a state where there is one, else back to
    /// the canvas; Esc puts back what was there, and drops a blank lane that has nothing; Tab goes on to the press, Shift-Tab back.
    pub(super) fn lane_name_key(&mut self, key: &Keystroke, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let mods = &key.modifiers;
        if mods.platform || mods.control || mods.alt {
            return false;
        }
        let Some(at) = self.new_lanes.iter().position(|one| one.name_focus.is_focused(window)) else { return false };
        let id = self.new_lanes[at].id.clone();
        let blank = self.lane_def(&id).is_some_and(|lane| source_of(&lane) == YOUR_OWN);
        match key.key.as_str() {
            "enter" => {
                self.settle_lane_names(true, window, cx);
                let at = self.new_lanes.iter().position(|one| one.name_focus.is_focused(window)).unwrap_or(at);
                match self.new_lanes.get(at).filter(|_| blank) {
                    Some(one) => window.focus(&one.pill, cx),
                    None => self.give_back(window, cx),
                }
                true
            }
            "escape" => {
                let held = self.new_lanes[at].was.clone();
                if self.pending_lane.as_ref().is_some_and(|lane| lane.id == id) && lane_is_empty(&self.pending_lane) {
                    self.pending_lane = None;
                    self.new_lanes.remove(at);
                    self.refresh(cx);
                } else {
                    self.new_lanes[at].name.update(cx, |state, cx| state.set_value(held, window, cx));
                }
                self.give_back(window, cx);
                true
            }
            "tab" if !mods.shift && blank => {
                window.focus(&self.new_lanes[at].pill, cx);
                true
            }
            "tab" if mods.shift => {
                self.settle_lane_names(true, window, cx);
                self.give_back(window, cx);
                true
            }
            _ => false,
        }
    }

    /// The press's keys: Enter or Space adds a state, Esc gives the keys back to the canvas.
    pub(super) fn lane_key(&mut self, key: &Keystroke, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let mods = &key.modifiers;
        if mods.platform || mods.control || mods.alt {
            return false;
        }
        let Some(id) = self.new_lanes.iter().find(|one| one.pill.is_focused(window)).map(|one| one.id.clone()) else { return false };
        match key.key.as_str() {
            "enter" | "space" => self.add_state(&id, window, cx),
            "escape" => self.give_back(window, cx),
            _ => return false,
        }
        true
    }

    /// Add a state: one more at the end of the lane, its name open where it stands with the keys in it.
    pub(super) fn add_state(&mut self, id: &str, window: &mut Window, cx: &mut Context<Self>) {
        self.settle(window, cx);
        // A name typed lands first, as a blur lands it.
        self.settle_lane_names(true, window, cx);
        let Some(lane) = self.lane_def(id) else { return };
        let named = if lane.name.is_empty() { "the lane".to_string() } else { lane.name.clone() };
        let (lane, state) = with_state_added(lane);
        let said = format!("Added the state {NEW_STATE} to {named}");
        self.edit_lifecycle(said, lane, cx);
        // The lane may have taken its id from its name by now.
        let id = self.new_lanes.iter().find(|one| one.id == id || self.lane_def(&one.id).is_some_and(|lane| lane.states.iter().any(|one| one.id == state))).map_or(id.to_string(), |one| one.id.clone());
        let node = format!("{id}/{state}");
        self.state.pick(&self.id, Some(crate::lanes_canvas::Pick::Node(node.clone())));
        self.picked = Some(crate::lanes_canvas::Pick::Node(node.clone()));
        self.armed = false;
        self.open_at(&node, true, window, cx);
    }

    /// A head the draft added is tinted with the accent, and holds a name box where the lane was just added. Any other head is
    /// the canvas's own words.
    pub(super) fn new_lane_head(&self, lane: &ipc::LaneView, words: Stateful<gpui::Div>, look: &Look) -> AnyElement {
        let Some(def) = self.lane_def(&lane.id) else { return words.into_any_element() };
        let (heads, id) = (self.lane_heads.clone(), lane.id.clone());
        let measured = canvas(
            move |bounds, window, _| {
                if heads.borrow_mut().insert(id.clone(), bounds) != Some(bounds) {
                    window.refresh();
                }
            },
            |_, _, _, _| {},
        )
        .absolute()
        .top_0()
        .left_0()
        .size_full();
        let Some(held) = self.new_lanes.iter().find(|one| one.id == lane.id) else {
            return words.bg(look.color(token::ACCENT_MUTED)).child(measured).into_any_element();
        };
        let source = SharedString::from(source_of(&def));
        let field = labelled(
            div()
                .id(SharedString::from(format!("lane-name:{}", lane.id)))
                .key_context(crate::keymap::TEXT_BOX)
                .w_full()
                .text_size(look.length(token::TEXT_2XS))
                .line_height(look.length(token::LEADING_2XS))
                .child(Textarea::new(&held.name).with_size(Size::Small)),
            Role::TextInput,
            "Lane name",
        );
        div()
            .id(SharedString::from(format!("lane-head-new:{}", lane.id)))
            .relative()
            .size_full()
            .flex()
            .flex_col()
            .justify_center()
            .gap(look.length(token::SPACE_1))
            .py(look.length(token::SPACE_2))
            .bg(look.color(token::ACCENT_MUTED))
            .child(measured)
            .child(div().w_full().px(look.length(token::BORDER_WIDTH) * 2.).child(field))
            .child(div().min_w_0().truncate().px(look.length(token::SPACE_2)).text_size(look.length(token::TEXT_2XS)).line_height(look.length(token::LEADING_2XS)).text_color(look.color(token::FG_MUTED)).child(source))
            .into_any_element()
    }

    /// Add a state, drawn where the lane's next state would stand: the column after its last state, the first while it has none,
    /// across from its head. The columns are the workflow lane's, as the canvas drew them; none is drawn until both are.
    pub(super) fn render_state_pills(&self, frame: Bounds<Pixels>, look: &Look, cx: &Context<Self>) -> Option<AnyElement> {
        if frame.size.width <= px(0.) {
            return None;
        }
        let gaps = self.state.step_gaps(&self.id);
        let (first, gap) = (gaps.first()?, gaps.first()?.bounds.size.width);
        let first = node_bounds(&first.after, &gaps)?;
        let (wide, stride) = (first.size.width, first.size.width + gap);
        let tall = wide * STATE_PER_NODE;
        let edge = lanes_edge(frame, look);
        let heads = self.lane_heads.borrow();
        let pills: Vec<AnyElement> = self
            .new_lanes
            .iter()
            .filter_map(|held| {
                let def = self.lane_def(&held.id).filter(|lane| source_of(lane) == YOUR_OWN)?;
                let head = heads.get(&held.id)?;
                let at = point(first.left() + stride * def.states.len() as f32, head.center().y - tall / 2.);
                let box_ = Bounds { origin: at, size: gpui::size(wide, tall) };
                (box_.right() > edge && box_.left() < frame.right()).then(|| self.state_pill(held, box_, frame, look, cx))
            })
            .collect();
        if pills.is_empty() {
            return None;
        }
        Some(div().absolute().left(edge - frame.left()).top_0().right_0().bottom_0().overflow_hidden().children(pills).into_any_element())
    }

    fn state_pill(&self, held: &NewLane, at: Bounds<Pixels>, frame: Bounds<Pixels>, look: &Look, cx: &Context<Self>) -> AnyElement {
        let lane_id = held.id.clone();
        let ring = ring_colour(look);
        let edge = lanes_edge(frame, look);
        let pill = div()
            .id(SharedString::from(format!("lane-add-state:{lane_id}")))
            .track_focus(&held.pill)
            .absolute()
            .left(at.left() - edge)
            .top(at.top() - frame.top())
            .w(at.size.width)
            .h(at.size.height)
            .flex()
            .items_center()
            .justify_center()
            .gap(look.length(token::SPACE_1))
            .rounded(look.length(token::RADIUS_MD))
            .border(look.length(token::FOCUS_RING_WIDTH))
            .border_dashed()
            .border_color(look.color(token::BORDER_STRONG))
            .text_size(look.length(token::TEXT_XS))
            .line_height(look.length(token::LEADING_XS))
            .text_color(look.color(token::FG_MUTED))
            .whitespace_nowrap()
            .cursor_pointer()
            .hover(|style| style.bg(look.color(token::BG_HOVER)))
            .focus(move |style| style.border_color(ring))
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_click(cx.listener(move |this, _, window, cx| this.add_state(&lane_id, window, cx)))
            .child(svg().flex_none().path(icon_path("plus")).size(look.length(token::SPACE_3)).text_color(look.color(token::FG_MUTED)))
            .child("Add a state");
        labelled(pill, Role::Button, "Add a state").into_any_element()
    }

    /// The lanes the draft added, tinted across the canvas from their heads: a faint accent over the band and the accent's ring
    /// round the whole lane, inset. Where a head has not been drawn yet there is none, and the head's own draw asks for another.
    pub(super) fn render_lane_tints(&self, frame: Bounds<Pixels>, look: &Look) -> Option<AnyElement> {
        if frame.size.width <= px(0.) {
            return None;
        }
        let heads = self.lane_heads.borrow();
        let tints: Vec<AnyElement> = self
            .added
            .iter()
            .chain(self.pending_lane.iter())
            .filter_map(|lane| heads.get(&lane.id).map(|head| (lane.id.clone(), *head)))
            .flat_map(|(id, head)| {
                let (left, top) = (head.left() - frame.left(), head.top() - frame.top());
                let wide = (frame.right() - head.left()).max(px(0.));
                let band = div().id(SharedString::from(format!("lane-tint:{id}"))).absolute().left(left + head.size.width).top(top).w((wide - head.size.width).max(px(0.))).h(head.size.height).bg(look.color(token::ACCENT_FAINT));
                let ring = div().id(SharedString::from(format!("lane-ring:{id}"))).absolute().left(left).top(top).w(wide).h(head.size.height).border(look.length(token::BORDER_WIDTH) * 2.).border_color(look.color(token::ACCENT));
                [band.into_any_element(), ring.into_any_element()]
            })
            .collect();
        if tints.is_empty() {
            return None;
        }
        Some(div().absolute().top_0().left_0().size_full().overflow_hidden().children(tints).into_any_element())
    }

    /// Where the lane just added was drawn, relative to the canvas's frame, for the page to scroll to; none until its head has been.
    pub(super) fn lane_reveal(&self) -> Option<Bounds<Pixels>> {
        let id = self.show_lane.take()?;
        let Some(head) = self.lane_heads.borrow().get(&id).copied() else {
            self.show_lane.set(Some(id));
            return None;
        };
        let frame = self.frame.get();
        Some(Bounds { origin: point(head.left() - frame.left(), head.top() - frame.top()), size: head.size })
    }
}

/// A blank lane with no name and no state: nothing to keep.
fn lane_is_empty(lane: &Option<LifecycleDefinition>) -> bool {
    lane.as_ref().is_some_and(|lane| lane.name.trim().is_empty() && lane.states.is_empty())
}
