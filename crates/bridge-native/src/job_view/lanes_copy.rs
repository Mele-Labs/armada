//! The Workflow tab around the canvas: Add to this Job over it and Changed on this Job under it, as
//! Classic's `lanes-copy.tsx` draws them, and a Job's own Workflow opened from its lane on the parent's
//! (`LanesBody`, `Into` in `shared-panel/lanes.tsx`).
//!
//! **A change applies to this Job at once**; Keep waits on Approve, so it is not drawn as done. A refusal
//! is Fleet's own sentence, drawn where it was asked.

use std::cell::RefCell;
use std::rc::Rc;
use std::collections::{HashMap, HashSet};

use gpui::{canvas, div, prelude::*, AnyElement, Bounds, Context, Entity, FocusHandle, Focusable, FontWeight, KeyDownEvent, Pixels, Role, ScrollHandle, SharedString, Stateful, Subscription, Window};
use gpui_component::input::{Enter, Input, InputEvent, InputState, Textarea, TextareaState};
use ipc::{ChangeJobCopy, ChangeSessionSteps, CopyPieceKind, JobLanesView, KeepJobPiece, KeepJobWorkflow, LaneChange, StepChange};

use super::head::{below_left, drawn};
use super::rows::{glyph, mono, tip, Size};
use super::JobView;
use crate::dropdown::Entry;
use crate::labels::labelled;
use crate::look::{token, Look};

const ADD_SAID: &str = "Add to this Job";
const KEEP_SAID: &str = "Keep for every Job";
const KEPT_SAID: &str = "Proposed for every Job";
/// The line a head that opens a Job's Workflow adds to its tooltip.
pub(super) const OPENS_SAID: &str = "Double-click to open its Workflow";

#[derive(Clone, Copy, PartialEq, Eq)]
enum Adding {
    Step,
    Lifecycle,
    Rule,
}

/// The form Add to this Job opened, on the Job it was opened on.
struct Form {
    job: SharedString,
    adding: Adding,
    /// The step's label; a lifecycle or a rule has none.
    label: Option<Entity<InputState>>,
    /// The step's instructions, or the lifecycle's or rule's whole file.
    text: Entity<TextareaState>,
    /// The step the new one goes after, or none for first.
    after: Option<String>,
    /// The After picker's keys, so Tab reaches it and the menu gives them back.
    after_keys: FocusHandle,
    pending: bool,
    refused: Option<SharedString>,
    _subs: Vec<Subscription>,
}

/// What a Job's Changed list has asked of Fleet (`JobChanges`).
#[derive(Default)]
struct Keeps {
    /// The workflow rows the Job's workflow was proposed with; a change made after offers Keep again.
    kept: Option<String>,
    pending: bool,
    refused: Option<SharedString>,
    /// The rows whose piece was proposed, and the one being proposed.
    rows: HashSet<String>,
    row_pending: Option<String>,
}

/// The tab's state around the canvas.
pub(super) struct LanesCopy {
    form: Option<Form>,
    keeps: HashMap<SharedString, Keeps>,
    /// The Jobs opened over a Job's lanes, outermost first, each with the lane on its parent that opened it.
    into: HashMap<SharedString, Vec<(SharedString, String)>>,
    /// The way back, which has the keys when a Job's own Workflow opens.
    back: FocusHandle,
    /// Each head that opens a Workflow or a ticket's menu, by Job and lane, so Back gives it the keys again.
    heads: RefCell<HashMap<String, FocusHandle>>,
    /// The Workflow body's scroll while a form or the changes stand under the canvas, as Classic's pane scrolls.
    scroll: ScrollHandle,
    /// Where each of the form's fields was last drawn, so the one with the keys is scrolled into view.
    fields_at: Rc<RefCell<HashMap<&'static str, Bounds<Pixels>>>>,
}

gpui::actions!(lanes_form, [AddNow]);

/// ⌘↵ adds from anywhere in the form: bound in the form's own context, it is met before a walk's bar.
pub(super) fn bind(cx: &mut gpui::App) {
    cx.bind_keys([gpui::KeyBinding::new("cmd-enter", AddNow, Some(FORM_KEYS))]);
}

const FORM_KEYS: &str = "lanes-form";
/// The Workflow tab's body where it scrolls, named so a walk can wheel it.
const BODY_SAID: &str = "This Job's Workflow";

impl LanesCopy {
    pub fn new(cx: &mut gpui::App) -> Self {
        Self { form: None, keeps: HashMap::new(), into: HashMap::new(), back: cx.focus_handle(), heads: RefCell::default(), scroll: ScrollHandle::new(), fields_at: Rc::default() }
    }
}

/// The Job a lane of a Job's own lifecycle stands for: the id after the kind, decoded (`instanceKey`).
pub(super) fn instance_key(id: &str) -> String {
    let rest = id.split_once(':').map_or(id, |(_, rest)| rest);
    let bytes = rest.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut at = 0;
    while at < bytes.len() {
        if bytes[at] == b'%' && at + 2 < bytes.len() {
            if let Some(byte) = rest.get(at + 1..at + 3).and_then(|hex| u8::from_str_radix(hex, 16).ok()) {
                out.push(byte);
                at += 3;
                continue;
            }
        }
        out.push(bytes[at]);
        at += 1;
    }
    String::from_utf8(out).unwrap_or_else(|_| rest.to_string())
}

/// The words a change is listed under: the step, the rule or the lifecycle it changed (`changedName`).
fn changed_name(lanes: &JobLanesView, change: &LaneChange) -> String {
    let copy = &lanes.job.copy;
    match change.lane.as_str() {
        "workflow" => copy.steps.iter().find(|one| one.id == change.item).map_or_else(|| change.item.clone(), |one| one.label.clone()),
        "rule" => change.name.clone().unwrap_or_else(|| change.item.clone()),
        lane => {
            let lifecycle = copy.lifecycles.iter().find(|one| one.id == lane);
            let state = lifecycle.and_then(|one| one.states.iter().find(|state| state.id == change.item)).map(|state| state.label.clone());
            let name = lifecycle.map_or_else(|| lane.to_string(), |one| one.label.clone());
            state.map_or(name.clone(), |state| format!("{name}: {state}"))
        }
    }
}

/// A changed rule or lifecycle kept as one piece; a workflow row or a dropped piece has none (`pieceOf`).
fn piece_of(change: &LaneChange) -> Option<(CopyPieceKind, String)> {
    if change.lane == "workflow" || change.dropped {
        return None;
    }
    Some(if change.lane == "rule" { (CopyPieceKind::Rule, change.item.clone()) } else { (CopyPieceKind::Lifecycle, change.lane.clone()) })
}

/// A small button, `size="sm"`: primary or ghost, held while pending.
fn small_button(look: &Look, id: impl Into<gpui::ElementId>, said: &str, lead: Option<&'static str>, primary: bool, on: bool, pending: bool) -> Stateful<gpui::Div> {
    let base = div()
        .id(id)
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .gap(look.length(token::SPACE_1))
        .h(look.length(token::H_CONTROL_SM))
        .px(look.length(token::SPACE_3))
        .rounded(look.length(token::RADIUS_MD))
        .border(look.length(token::BORDER_WIDTH))
        .text_size(look.length(token::TEXT_XS))
        .line_height(look.length(token::LEADING_XS))
        .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)));
    let ink = if primary { look.color(token::TEXT_ON_ACCENT) } else { look.color(token::FG_MUTED) };
    let styled = if primary {
        base.bg(look.color(token::ACCENT)).border_color(look.color(token::ACCENT)).text_color(ink).when(on, |button| button.hover(|style| style.bg(look.color(token::ACCENT_HOVER))))
    } else {
        base.border_color(gpui::transparent_black()).text_color(ink).when(on, |button| button.hover(|style| style.bg(look.color(token::BG_HOVER)).text_color(look.color(token::FG_DEFAULT))))
    };
    styled
        .when(on, |button| button.cursor_pointer())
        .when(!on && !pending, |button| button.opacity(0.4))
        .when_some(if pending { Some("loader-circle") } else { lead }, |button, icon| button.child(glyph(look, icon, Size::Small, ink)))
        .child(SharedString::from(said.to_string()))
}

/// Fleet's sentence where it refused, under what asked (`sv-lanes__refused`).
fn refused_line(look: &Look, id: &str, why: &SharedString, inset: bool) -> AnyElement {
    labelled(
        div()
            .id(SharedString::from(format!("lanes-refused:{id}")))
            .when(inset, |line| line.px(look.length(token::SPACE_3)).py(look.length(token::SPACE_1)))
            .text_size(look.length(token::TEXT_XS))
            .line_height(look.length(token::LEADING_XS))
            .text_color(look.color(token::STATUS_COMPLETED_FAILED))
            .child(why.clone()),
        Role::Alert,
        why.clone(),
    )
    .into_any_element()
}

/// A field's words over it.
fn field_label(look: &Look, said: &str) -> gpui::Div {
    div().text_size(look.length(token::TEXT_XS)).line_height(look.length(token::LEADING_XS)).font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM))).text_color(look.color(token::FG_MUTED)).child(SharedString::from(said.to_string()))
}

/// The frame a text box stands in.
fn box_frame(look: &Look, id: &'static str) -> Stateful<gpui::Div> {
    div()
        .id(id)
        .key_context(crate::keymap::TEXT_BOX)
        .min_w_0()
        .px(look.length(token::SPACE_3))
        .py(look.length(token::SPACE_1))
        .rounded(look.length(token::RADIUS_MD))
        .border(look.length(token::BORDER_WIDTH))
        .border_color(look.color(token::BORDER_DEFAULT))
        .bg(look.color(token::BG_BASE))
        .text_size(look.length(token::TEXT_SM))
        .text_color(look.color(token::FG_DEFAULT))
}

impl JobView {
    /// The Workflow tab's body: a Job's own lanes, or the Job opened over them under a Back and its title.
    /// Nothing for a Job with no lanes, as Classic's `LanesBody` draws nothing.
    pub(super) fn pane_lanes(&self, root: &SharedString, look: &Look, cx: &Context<Self>) -> Option<AnyElement> {
        self.record(root, cx)?.lanes.as_ref()?;
        let trail = self.lanes.copy.into.get(root).cloned().unwrap_or_default();
        let shown = trail.last().map_or_else(|| root.clone(), |(job, _)| job.clone());
        let own = self.own_lanes(&shown, look, cx);
        let body = match trail.len() {
            0 => own?,
            depth => {
                let parent = if depth >= 2 { trail[depth - 2].0.clone() } else { root.clone() };
                let title_of = |job: &SharedString| self.record(job, cx).and_then(|one| one.title.clone()).unwrap_or_else(|| "Workflow".into());
                let back_said = format!("Back to {}", title_of(&parent));
                let way = labelled(
                    small_button(look, "lanes-back", &back_said, Some("chevron-left"), false, true, false)
                        .track_focus(&self.lanes.copy.back)
                        .focus_visible(|style| style.border_color(look.color(token::ACCENT)))
                        .tooltip(tip(back_said.clone()))
                        .on_click(cx.listener({
                            let root = root.clone();
                            move |this, _, window, cx| this.lanes_back(&root, window, cx)
                        }))
                        .on_key_down(cx.listener({
                            let root = root.clone();
                            move |this, event: &KeyDownEvent, window, cx| {
                                if matches!(event.keystroke.key.as_str(), "enter" | "space") && !event.keystroke.modifiers.modified() {
                                    cx.stop_propagation();
                                    this.lanes_back(&root, window, cx);
                                }
                            }
                        })),
                    Role::Button,
                    back_said,
                );
                let title = labelled(
                    div()
                        .id("lanes-into-title")
                        .flex_1()
                        .min_w_0()
                        .truncate()
                        .text_size(look.length(token::TEXT_SM))
                        .line_height(look.length(token::LEADING_SM))
                        .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                        .text_color(look.color(token::FG_DEFAULT))
                        .child(SharedString::from(title_of(&shown))),
                    Role::Heading,
                    title_of(&shown),
                );
                div()
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .flex_col()
                    .gap(look.length(token::SPACE_3))
                    .child(div().flex_none().flex().items_center().gap(look.length(token::SPACE_2)).min_w_0().child(way).child(title))
                    .children(own)
                    .into_any_element()
            }
        };
        let keys_root = root.clone();
        Some(
            div()
                .id("lanes-body")
                .flex_1()
                .min_h_0()
                .flex()
                .flex_col()
                .p(look.length(token::SPACE_3))
                // While a Job's own Workflow is open, Esc is Back, after the canvas and the form have had it.
                .on_key_down(cx.listener(move |this, event: &KeyDownEvent, window, cx| {
                    if event.keystroke.key == "escape" && !event.keystroke.modifiers.modified() && this.pop.is_none() && this.lanes.copy.into.get(&keys_root).is_some_and(|trail| !trail.is_empty()) {
                        cx.stop_propagation();
                        this.lanes_back(&keys_root, window, cx);
                    }
                }))
                .child(body)
                .into_any_element(),
        )
    }

    /// One Job's lanes: Add to this Job, the canvas, then what was changed on it (`OwnLanes`).
    fn own_lanes(&self, id: &SharedString, look: &Look, cx: &Context<Self>) -> Option<AnyElement> {
        let lanes = self.record(id, cx)?.lanes.as_ref()?;
        let canvas = self.lanes_canvas(id, look, cx)?;
        let add = self.add_to_job(id, lanes, look, cx);
        let changes = self.job_changes(id, lanes, look, cx);
        let form_open = self.lanes.copy.form.as_ref().is_some_and(|form| form.job == *id);
        if !form_open && changes.is_none() {
            // Nothing under the canvas: the frame fits the pane, so a pan to a node lands where it is seen.
            return Some(
                div()
                    .flex_1()
                    .min_h_0()
                    .overflow_hidden()
                    .flex()
                    .flex_col()
                    .gap(look.length(token::SPACE_3))
                    .child(div().flex_none().child(add))
                    .child(div().flex_1().min_h_0().flex().flex_col().child(canvas))
                    .into_any_element(),
            );
        }
        // A form or the changes under the canvas: the body scrolls as one, the canvas keeping Classic's floor
        // of seven space-12s (`.sv-lanes__body > .armada-lane-canvas`), but never taller than the body shows,
        // less the Add row, so once the keys bring it into view the node they reach is in sight and Add to this
        // Job stays above it. The canvas pans on the wheel, as Classic's `panOnScroll`; the Add row takes it.
        let floor = look.length(token::SPACE_12) * 7.;
        let gap = look.length(token::SPACE_3);
        let shown = self.lanes.copy.scroll.bounds().size.height;
        let add_tall = self.lanes.copy.fields_at.borrow().get("add").map_or(gpui::px(0.), |at| at.size.height);
        let room = shown - add_tall - gap;
        let tall = if room > gpui::px(0.) { floor.min(room) } else { floor };
        Some(labelled(
            div()
                .id("lanes-scroll")
                .flex_1()
                .min_h_0()
                .overflow_y_scroll()
                .track_scroll(&self.lanes.copy.scroll)
                .flex()
                .flex_col()
                .gap(gap)
                .child(self.form_field("add", div().flex_none().child(add)))
                .child(self.form_field("canvas", div().flex_none().h(tall).flex().flex_col().child(canvas)))
                .children(changes.map(|changes| div().flex_none().child(changes))),
            Role::Region,
            BODY_SAID,
        )
        .into_any_element())
    }

    /// A Job's own Workflow opened over its parent's lanes, with the keys on Back (`setInto`).
    pub(super) fn lanes_into(&mut self, parent: &SharedString, lane: &str, window: &mut Window, cx: &mut Context<Self>) {
        let Some(root) = self.shown.clone() else { return };
        let child: SharedString = instance_key(lane).into();
        if self.record(&child, cx).and_then(|one| one.lanes.as_ref()).is_none() {
            return;
        }
        let trail = self.lanes.copy.into.entry(root).or_default();
        // A press on a head of a Job that is already deeper in is not a way in.
        if trail.last().map_or(false, |(job, _)| job != parent) && trail.iter().all(|(job, _)| job != parent) {
            return;
        }
        trail.push((child, format!("{parent}/{lane}")));
        let back = self.lanes.copy.back.clone();
        window.defer(cx, move |window, cx| window.focus(&back, cx));
        cx.notify();
    }

    /// Back to the lanes under, with the keys on the head that opened them.
    fn lanes_back(&mut self, root: &SharedString, window: &mut Window, cx: &mut Context<Self>) {
        let Some((_, head)) = self.lanes.copy.into.get_mut(root).and_then(Vec::pop) else { return };
        if let Some(handle) = self.lanes.copy.heads.borrow().get(&head).cloned() {
            window.defer(cx, move |window, cx| window.focus(&handle, cx));
        }
        cx.notify();
    }

    /// A head's own keys, made once for each Job and lane.
    pub(super) fn head_focus(&self, job: &str, lane: &str, cx: &gpui::App) -> FocusHandle {
        self.lanes.copy.heads.borrow_mut().entry(format!("{job}/{lane}")).or_insert_with(|| cx.focus_handle()).clone()
    }

    /// Whether a lane stands for a Job this one started that has lanes of its own (`opens`).
    pub(super) fn lane_opens(&self, lanes: &JobLanesView, lane: &str, cx: &gpui::App) -> bool {
        let Some(held) = lanes.job.instances.iter().find(|one| one.id == lane) else { return false };
        lanes.job.copy.lifecycles.iter().any(|one| one.id == held.lifecycle && one.kind == ipc::LaneLifecycleKind::Job) && self.record(&instance_key(&held.id), cx).is_some_and(|one| one.lanes.is_some())
    }

    /// Teach from a ticket's lane head: its title and its note, and the ticket it stands for.
    pub(super) fn teach_ticket_lane(&mut self, title: &str, note: Option<&str>, lane: &str, window: &mut Window, cx: &mut Context<Self>) {
        let text = note.map_or_else(|| title.to_string(), |note| format!("{title} {note}"));
        self.teach_open(super::menus::Teach { kind: "ticket", text, at: None, ids: vec![instance_key(lane)] }, window, cx);
    }

    /// The menu a ticket's lane head opens: its name, then Teach.
    pub(super) fn open_ticket_head_menu(&mut self, title: String, note: Option<String>, lane: String, at: gpui::Point<gpui::Pixels>, from_keys: bool, window: &mut Window, cx: &mut Context<Self>) {
        let heading = format!("Ticket {title}");
        let entries = vec![Entry::Label(crate::titlebar::id_of(heading.clone())), Entry::Act { id: "teach", label: "Teach".into(), icon: Some("graduation-cap"), danger: false }];
        let back = window.focused(cx);
        self.open_pop(
            heading,
            entries,
            at,
            from_keys,
            back,
            move |this, picked, window, cx| {
                if picked == "teach" {
                    this.teach_ticket_lane(&title, note.as_deref(), &lane, window, cx);
                }
            },
            window,
            cx,
        );
    }

    // ---- Add to this Job ----------------------------------------------------------------------------

    /// The menu button and the form it opens (`AddToJob`).
    fn add_to_job(&self, id: &SharedString, lanes: &JobLanesView, look: &Look, cx: &Context<Self>) -> AnyElement {
        let has_steps = !lanes.job.copy.steps.is_empty();
        let job = id.clone();
        let trigger = labelled(
            small_button(look, "lanes-add", ADD_SAID, Some("square-plus"), false, true, false)
                .border_color(look.color(token::BORDER_DEFAULT))
                .bg(look.color(token::BG_RAISED))
                .text_color(look.color(token::FG_DEFAULT))
                .child(glyph(look, "chevron-down", Size::Small, look.color(token::FG_SUBTLE)))
                .on_click(cx.listener(move |this, _, window, cx| {
                    let at = drawn(window, cx, Role::Button, ADD_SAID).map_or_else(|| window.mouse_position(), below_left);
                    let mut entries = Vec::new();
                    if has_steps {
                        entries.push(Entry::Act { id: "step", label: "Step".into(), icon: None, danger: false });
                    }
                    entries.push(Entry::Act { id: "lifecycle", label: "Lifecycle".into(), icon: None, danger: false });
                    entries.push(Entry::Act { id: "rule", label: "Rule".into(), icon: None, danger: false });
                    let job = job.clone();
                    let back = window.focused(cx);
                    this.open_button_menu(
                        ADD_SAID,
                        entries,
                        at,
                        back,
                        move |this, picked, window, cx| {
                            let adding = match picked {
                                "step" => Adding::Step,
                                "lifecycle" => Adding::Lifecycle,
                                _ => Adding::Rule,
                            };
                            this.open_add_form(&job, adding, window, cx);
                        },
                        window,
                        cx,
                    );
                })),
            Role::Button,
            ADD_SAID,
        );
        let form = self.lanes.copy.form.as_ref().filter(|form| form.job == *id).map(|form| self.add_form(form, lanes, look, cx));
        div().flex_none().flex().flex_col().gap(look.length(token::SPACE_2)).min_w_0().child(div().flex().items_center().gap(look.length(token::SPACE_2)).child(trigger)).children(form).into_any_element()
    }

    fn open_add_form(&mut self, job: &SharedString, adding: Adding, window: &mut Window, cx: &mut Context<Self>) {
        let Some(lanes) = self.record(job, cx).and_then(|one| one.lanes.clone()) else { return };
        let label = (adding == Adding::Step).then(|| cx.new(|cx| InputState::new(window, cx)));
        let text = cx.new(|cx| TextareaState::new(window, cx).auto_grow(if adding == Adding::Step { 3 } else { 8 }, if adding == Adding::Step { 3 } else { 8 }));
        let mut subs = Vec::new();
        for input in label.iter() {
            subs.push(cx.subscribe_in(input, window, |_, _, event: &InputEvent, _, cx| {
                if let InputEvent::Change = event {
                    cx.notify();
                }
            }));
        }
        subs.push(cx.subscribe_in(&text, window, |_, _, event: &InputEvent, _, cx| {
            if let InputEvent::Change = event {
                cx.notify();
            }
        }));
        // The field given the keys comes into view, wherever the body was scrolled.
        for input in label.iter() {
            subs.push(cx.on_focus_in(&input.focus_handle(cx), window, |this, window, cx| this.reveal_field("label", window, cx)));
        }
        subs.push(cx.on_focus_in(&text.focus_handle(cx), window, |this, window, cx| this.reveal_field("text", window, cx)));
        let after_keys = cx.focus_handle();
        subs.push(cx.on_focus_in(&after_keys, window, |this, window, cx| this.reveal_field("after", window, cx)));
        // After the step the Job is at, or the last one where it is past them all.
        let steps = &lanes.job.copy.steps;
        let after = (!steps.is_empty()).then(|| steps[(lanes.at.max(0) as usize).min(steps.len() - 1)].id.clone());
        match &label {
            Some(input) => input.update(cx, |input, cx| input.focus(window, cx)),
            None => text.update(cx, |input, cx| input.focus(window, cx)),
        }
        self.lanes.copy.form = Some(Form { job: job.clone(), adding, label, text, after, after_keys, pending: false, refused: None, _subs: subs });
        // A form opens at the body's top, its first field with the keys in view.
        self.lanes.copy.scroll.set_offset(gpui::Point::default());
        self.lanes.copy.fields_at.borrow_mut().clear();
        cx.notify();
    }

    /// Scroll the Workflow body the least that brings a field of the form into view, where it was last drawn.
    pub(super) fn reveal_field(&self, field: &'static str, window: &mut Window, cx: &mut gpui::App) {
        let scroll = self.lanes.copy.scroll.clone();
        let fields_at = self.lanes.copy.fields_at.clone();
        window.defer(cx, move |window, _| {
            let Some(at) = fields_at.borrow().get(field).copied() else { return };
            let view = scroll.bounds();
            let mut offset = scroll.offset();
            if at.top() < view.top() {
                offset.y += view.top() - at.top();
            } else if at.bottom() > view.bottom() {
                offset.y -= (at.bottom() - view.bottom()).min(at.top() - view.top());
            } else {
                return;
            }
            offset.y = offset.y.min(gpui::px(0.)).max(-scroll.max_offset().y);
            scroll.set_offset(offset);
            window.refresh();
        });
    }

    fn close_add_form(&mut self, cx: &mut Context<Self>) {
        self.lanes.copy.form = None;
        // Back at the head of the body, where Add to this Job opened it.
        self.lanes.copy.scroll.set_offset(gpui::Point::default());
        cx.notify();
    }

    /// Add, from the button or ⌘↵ in any field: pending while Fleet has not answered, the refusal kept in
    /// the form where it said no, and the form put away where it took it (`useAdding`).
    fn add_now(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(form) = self.lanes.copy.form.as_mut() else { return };
        if form.pending {
            return;
        }
        let label = form.label.as_ref().map(|input| input.read(cx).value().to_string()).unwrap_or_default();
        let text = form.text.read(cx).value().to_string();
        let blank = match form.adding {
            Adding::Step => label.trim().is_empty(),
            _ => text.trim().is_empty(),
        };
        if blank {
            return;
        }
        form.pending = true;
        form.refused = None;
        let session_id = Self::session_id(&form.job);
        let (route, body, mock) = match form.adding {
            Adding::Step => (
                "/sessions/workflow/steps",
                ipc::encode(&ChangeSessionSteps {
                    session_id,
                    change: StepChange::Add { after: form.after.clone(), id: None, label, about: None, instructions: (!text.trim().is_empty()).then_some(text), ends_on: Vec::new() },
                }),
                None,
            ),
            piece => {
                let (kind, said) = if piece == Adding::Rule { (CopyPieceKind::Rule, "rule") } else { (CopyPieceKind::Lifecycle, "lifecycle") };
                // Mock data: Fleet's loader turns the file down, so the walk shows a refusal in the form.
                let mock = format!("(job copy)/{said}s/new.yml is not YAML: did not find expected key at line 3 column 1");
                ("/sessions/copy", ipc::encode(&ChangeJobCopy { session_id, kind, id: String::new(), text: Some(text) }), Some(mock))
            }
        };
        cx.notify();
        self.post_answered(route, body, mock, window, cx, |this, said, window, cx| {
            let Some(form) = this.lanes.copy.form.as_mut() else { return };
            form.pending = false;
            match said {
                None => this.lanes.copy.form = None,
                Some(why) => {
                    form.refused = Some(why);
                    // The refusal stands where it said no, with Add beside it.
                    this.reveal_field("acts", window, cx);
                }
            }
            cx.notify();
        });
    }

    /// POST and say what Fleet answered: `None` where it took it, its sentence where it did not. With no
    /// Fleet, as on mock data, the answer is `mock` a moment later.
    fn post_answered(
        &mut self,
        route: &'static str,
        body: Result<String, ipc::Unencodable>,
        mock: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
        answered: impl FnOnce(&mut JobView, Option<SharedString>, &mut Window, &mut Context<JobView>) + 'static,
    ) {
        if self.fleet.is_none() {
            cx.spawn_in(window, async move |this, cx| {
                cx.background_executor().timer(std::time::Duration::from_millis(600)).await;
                let _ = this.update_in(cx, |this, window, cx| answered(this, mock.map(SharedString::from), window, cx));
            })
            .detach();
            return;
        }
        self.post(route.into(), body, window, cx, Box::new(move |this, answer, window, cx| answered(this, answer.err().map(SharedString::from), window, cx)));
    }

    /// After's menu: First, then each step, the one standing ticked; the keys go back to After.
    fn open_after_menu(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(form) = self.lanes.copy.form.as_ref() else { return };
        let Some(lanes) = self.record(&form.job, cx).and_then(|one| one.lanes.clone()) else { return };
        let choices: Vec<(String, String)> = lanes.job.copy.steps.iter().map(|one| (one.id.clone(), one.label.clone())).collect();
        let now = form.after.clone();
        let keys = form.after_keys.clone();
        window.focus(&keys, cx);
        let at = drawn(window, cx, Role::ComboBox, "After").map_or_else(|| window.mouse_position(), below_left);
        let mut entries = vec![Entry::Choice { id: "", label: "First".into(), on: now.is_none() }];
        // A menu entry's id is static; the step is found again by its place.
        const PLACES: [&str; 16] = ["0", "1", "2", "3", "4", "5", "6", "7", "8", "9", "10", "11", "12", "13", "14", "15"];
        for (at, (id, label)) in choices.iter().enumerate().take(PLACES.len()) {
            entries.push(Entry::Choice { id: PLACES[at], label: label.clone().into(), on: now.as_deref() == Some(id.as_str()) });
        }
        self.open_button_menu(
            "After",
            entries,
            at,
            Some(keys),
            move |this, picked, _, cx| {
                if let Some(form) = this.lanes.copy.form.as_mut() {
                    form.after = picked.parse::<usize>().ok().and_then(|at| choices.get(at)).map(|(id, _)| id.clone());
                }
                cx.notify();
            },
            window,
            cx,
        );
    }

    /// A field of the form, its bounds kept each frame so the keys' field can be scrolled into view.
    fn form_field(&self, field: &'static str, body: gpui::Div) -> AnyElement {
        let fields_at = self.lanes.copy.fields_at.clone();
        body.relative()
            .child(canvas(move |bounds, _, _| { fields_at.borrow_mut().insert(field, bounds); }, |_, _, _, _| {}).absolute().size_full())
            .into_any_element()
    }

    /// The form (`StepForm`, `PieceForm`): Esc puts it away, ⌘↵ adds from any field.
    fn add_form(&self, form: &Form, lanes: &JobLanesView, look: &Look, cx: &Context<Self>) -> AnyElement {
        let (named, file) = match form.adding {
            Adding::Step => ("Add a step to this Job", None),
            Adding::Lifecycle => ("Add a lifecycle to this Job", Some("Lifecycle file")),
            Adding::Rule => ("Add a rule to this Job", Some("Rule file")),
        };
        let blank = match &form.label {
            Some(label) => label.read(cx).value().trim().is_empty(),
            None => form.text.read(cx).value().trim().is_empty(),
        };
        let mut fields: Vec<AnyElement> = Vec::new();
        if let Some(label) = &form.label {
            fields.push(self.form_field(
                "label",
                div()
                    .flex()
                    .flex_col()
                    .gap(look.length(token::SPACE_1))
                    .child(field_label(look, "Label"))
                    .child(labelled(box_frame(look, "lanes-form-label").flex().items_center().h(look.length(token::H_CONTROL)).py_0().child(Input::new(label).appearance(false).cleanable(false)), Role::TextInput, "Label")),
            ));
        }
        let area_said = file.unwrap_or("Instructions");
        fields.push(self.form_field(
            "text",
            div()
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_1))
                .child(field_label(look, area_said))
                .child(labelled(
                    box_frame(look, "lanes-form-text").when(file.is_some(), |frame| frame.font_family(mono(look)).text_size(look.length(token::TEXT_XS)).line_height(look.length(token::LEADING_XS))).child(Textarea::new(&form.text).appearance(false)),
                    Role::TextInput,
                    area_said,
                )),
        ));
        if form.adding == Adding::Step {
            let steps = &lanes.job.copy.steps;
            let shown = form.after.as_ref().and_then(|after| steps.iter().find(|one| &one.id == after)).map_or_else(|| "First".to_string(), |one| one.label.clone());
            let picker = labelled(
                box_frame(look, "lanes-form-after")
                    .flex()
                    .items_center()
                    .justify_between()
                    .h(look.length(token::H_CONTROL))
                    .py_0()
                    .cursor_pointer()
                    .hover(|style| style.border_color(look.color(token::BORDER_STRONG)))
                    .child(SharedString::from(shown.clone()))
                    .child(glyph(look, "chevron-down", Size::Small, look.color(token::FG_SUBTLE)))
                    .track_focus(&form.after_keys)
                    .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                        if matches!(event.keystroke.key.as_str(), "enter" | "space" | "down") && !event.keystroke.modifiers.modified() {
                            cx.stop_propagation();
                            this.open_after_menu(window, cx);
                        }
                    }))
                    .on_click(cx.listener(|this, _, window, cx| this.open_after_menu(window, cx))),
                Role::ComboBox,
                "After",
            );
            fields.push(self.form_field("after", div().flex().flex_col().gap(look.length(token::SPACE_1)).child(field_label(look, "After")).child(picker)));
        }
        let refused = form.refused.as_ref().map(|why| refused_line(look, "form", why, false));
        let add = labelled(
            small_button(look, "lanes-form-add", "Add", None, true, !blank && !form.pending, form.pending).on_click(cx.listener(move |this, _, window, cx| this.add_now(window, cx))),
            Role::Button,
            "Add",
        );
        let cancel = labelled(small_button(look, "lanes-form-cancel", "Cancel", None, false, true, false).on_click(cx.listener(|this, _, _, cx| this.close_add_form(cx))), Role::Button, "Cancel");
        labelled(
            div()
                .id("lanes-form")
                .key_context(FORM_KEYS)
                .on_action(cx.listener(|this, _: &AddNow, window, cx| this.add_now(window, cx)))
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_2))
                .p(look.length(token::SPACE_3))
                .rounded(look.length(token::RADIUS_LG))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::BORDER_DEFAULT))
                .bg(look.color(token::BG_SUNKEN))
                // ⌘↵ in a field adds here: a one-line box passes it on, and a walk's bar would take it as Approve.
                .capture_action(cx.listener(|this, action: &Enter, window, cx| {
                    if action.secondary {
                        cx.stop_propagation();
                        this.add_now(window, cx);
                    }
                }))
                .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                    let keys = &event.keystroke;
                    if keys.key == "escape" && this.pop.is_none() {
                        cx.stop_propagation();
                        this.close_add_form(cx);
                    }
                }))
                .children(fields)
                .child(self.form_field("acts", div().flex().flex_col().gap(look.length(token::SPACE_2)).children(refused).child(div().flex().items_center().gap(look.length(token::SPACE_2)).child(add).child(cancel)))),
            Role::Group,
            named,
        )
        .into_any_element()
    }

    // ---- Changed on this Job ------------------------------------------------------------------------

    /// What was changed on this Job, a row for each, under the canvas (`JobChanges`). **Keep for every Job is
    /// in the head, once, and only where a step of the workflow changed**; a changed rule or lifecycle has its
    /// own Keep on its row, and a dropped one has none.
    fn job_changes(&self, id: &SharedString, lanes: &JobLanesView, look: &Look, cx: &Context<Self>) -> Option<AnyElement> {
        let changes = &lanes.job.changes;
        if changes.is_empty() {
            return None;
        }
        let workflow: Vec<&LaneChange> = changes.iter().filter(|one| one.lane == "workflow").collect();
        let signature = workflow.iter().map(|one| one.id.as_str()).collect::<Vec<_>>().join("\n");
        let none = Keeps::default();
        let keeps = self.lanes.copy.keeps.get(id).unwrap_or(&none);
        let ink = look.color(token::FG_MUTED);
        let keep_head = (!workflow.is_empty()).then(|| {
            if keeps.kept.as_deref() == Some(signature.as_str()) {
                labelled(div().id("lanes-kept").flex().items_center().h(look.length(token::H_CONTROL_SM)).tooltip(tip(KEPT_SAID)).child(glyph(look, "check", Size::Small, look.color(token::STEP_ADVANCED))), Role::Image, KEPT_SAID).into_any_element()
            } else {
                let job = id.clone();
                let signature = signature.clone();
                labelled(
                    icon_button(look, "lanes-keep", if keeps.pending { "loader-circle" } else { "book-marked" }, ink, !keeps.pending).tooltip(tip(KEEP_SAID)).on_click(cx.listener(move |this, _, window, cx| this.keep_workflow(&job, signature.clone(), window, cx))),
                    Role::Button,
                    KEEP_SAID,
                )
                .into_any_element()
            }
        });
        let head = div()
            .flex_none()
            .flex()
            .items_center()
            .justify_between()
            .px(look.length(token::SPACE_3))
            .when(keep_head.is_none(), |head| head.py(look.length(token::SPACE_1)))
            .border_b(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_DEFAULT))
            .bg(look.color(token::BG_RAISED))
            .font_family(mono(look))
            .text_size(look.length(token::TEXT_XS))
            .line_height(look.length(token::LEADING_XS))
            .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
            .text_color(look.color(token::FG_DEFAULT))
            .child("Changed on this Job")
            .children(keep_head);
        let refused = keeps.refused.as_ref().map(|why| refused_line(look, "keep", why, true));
        let rows: Vec<AnyElement> = changes
            .iter()
            .enumerate()
            .map(|(at, change)| {
                let name = changed_name(lanes, change);
                let keep = piece_of(change).map(|(kind, piece)| {
                    let said = format!("{KEEP_SAID}: {name}");
                    if keeps.rows.contains(&change.id) {
                        let kept = format!("{KEPT_SAID}: {name}");
                        labelled(div().id(SharedString::from(format!("lanes-kept:{}", change.id))).size(look.length(token::H_CONTROL_SM)).flex().items_center().justify_center().tooltip(tip(KEPT_SAID)).child(glyph(look, "check", Size::Small, look.color(token::STEP_ADVANCED))), Role::Image, kept).into_any_element()
                    } else {
                        let pending = keeps.row_pending.as_deref() == Some(change.id.as_str());
                        let (job, row) = (id.clone(), change.id.clone());
                        labelled(
                            icon_button(look, SharedString::from(format!("lanes-keep:{}", change.id)), if pending { "loader-circle" } else { "book-marked" }, ink, keeps.row_pending.is_none())
                                .tooltip(tip(KEEP_SAID))
                                .on_click(cx.listener(move |this, _, window, cx| this.keep_piece(&job, row.clone(), kind, piece.clone(), window, cx))),
                            Role::Button,
                            said,
                        )
                        .into_any_element()
                    }
                });
                labelled(
                    div()
                        .id(SharedString::from(format!("lanes-change:{}", change.id)))
                        .flex()
                        .items_center()
                        .gap(look.length(token::SPACE_2))
                        .min_h(look.length(token::H_CONTROL_SM))
                        .pl(look.length(token::SPACE_3))
                        .pr(look.length(token::SPACE_2))
                        .when(at > 0, |row| row.border_t(look.length(token::BORDER_WIDTH)).border_color(look.color(token::BORDER_SUBTLE)))
                        .child(div().flex_none().w(look.length(token::SPACE_4)).flex().items_center().child(glyph(look, "git-compare", Size::Small, look.color(token::STATUS_AWAITING_REVIEW))))
                        .child(div().flex_none().text_size(look.length(token::TEXT_XS)).line_height(look.length(token::LEADING_XS)).font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM))).text_color(look.color(token::FG_DEFAULT)).child(SharedString::from(name.clone())))
                        .child(div().flex_1().min_w_0().text_size(look.length(token::TEXT_XS)).line_height(look.length(token::LEADING_XS)).text_color(look.color(token::FG_MUTED)).child(SharedString::from(change.text.clone())))
                        // The Keep column is kept on every row, so the words end in one place.
                        .child(div().flex_none().size(look.length(token::H_CONTROL_SM)).children(keep)),
                    Role::ListItem,
                    format!("{name}: {}", change.text),
                )
                .into_any_element()
            })
            .collect();
        Some(
            labelled(
                div()
                    .id("lanes-changes")
                    .min_h_0()
                    .flex()
                    .flex_col()
                    .max_h(look.length(token::SPACE_12) * 6.)
                    .overflow_hidden()
                    .rounded(look.length(token::RADIUS_LG))
                    .border(look.length(token::BORDER_WIDTH))
                    .border_color(look.color(token::BORDER_DEFAULT))
                    .bg(look.color(token::BG_SUNKEN))
                    .child(head)
                    .children(refused)
                    .child(labelled(div().id("lanes-changes-rows").flex_1().flex().flex_col().min_h_0().overflow_y_scroll().children(rows), Role::List, "Changed on this Job")),
                Role::Region,
                "Changed on this Job",
            )
            .into_any_element(),
        )
    }

    fn keep_workflow(&mut self, job: &SharedString, signature: String, window: &mut Window, cx: &mut Context<Self>) {
        let keeps = self.lanes.copy.keeps.entry(job.clone()).or_default();
        if keeps.pending {
            return;
        }
        keeps.pending = true;
        keeps.refused = None;
        cx.notify();
        let body = ipc::encode(&KeepJobWorkflow { session_id: Self::session_id(job), source: None });
        let job = job.clone();
        self.post_answered("/sessions/copy/keep", body, None, window, cx, move |this, said, _, cx| {
            let keeps = this.lanes.copy.keeps.entry(job).or_default();
            keeps.pending = false;
            match said {
                None => keeps.kept = Some(signature),
                Some(why) => keeps.refused = Some(why),
            }
            cx.notify();
        });
    }

    fn keep_piece(&mut self, job: &SharedString, row: String, kind: CopyPieceKind, piece: String, window: &mut Window, cx: &mut Context<Self>) {
        let keeps = self.lanes.copy.keeps.entry(job.clone()).or_default();
        if keeps.row_pending.is_some() {
            return;
        }
        keeps.row_pending = Some(row.clone());
        keeps.refused = None;
        cx.notify();
        let body = ipc::encode(&KeepJobPiece { session_id: Self::session_id(job), kind, id: piece });
        let job = job.clone();
        self.post_answered("/sessions/copy/keep_piece", body, None, window, cx, move |this, said, _, cx| {
            let keeps = this.lanes.copy.keeps.entry(job).or_default();
            keeps.row_pending = None;
            match said {
                None => {
                    keeps.rows.insert(row);
                }
                Some(why) => keeps.refused = Some(why),
            }
            cx.notify();
        });
    }
}

/// A ghost icon button, `size="sm" iconOnly`.
fn icon_button(look: &Look, id: impl Into<gpui::ElementId>, icon: &'static str, ink: gpui::Hsla, on: bool) -> Stateful<gpui::Div> {
    div()
        .id(id)
        .flex_none()
        .size(look.length(token::H_CONTROL_SM))
        .flex()
        .items_center()
        .justify_center()
        .rounded(look.length(token::RADIUS_MD))
        .child(glyph(look, icon, Size::Small, ink))
        .when(on, |button| button.cursor_pointer().hover(|style| style.bg(look.color(token::BG_HOVER))))
}

#[cfg(test)]
mod tests {
    use super::instance_key;

    #[test]
    fn instance_key_drops_the_kind_and_decodes() {
        assert_eq!(instance_key("job:mock-10"), "mock-10");
        assert_eq!(instance_key("job:a%2Fb"), "a/b");
        assert_eq!(instance_key("plain"), "plain");
    }
}
