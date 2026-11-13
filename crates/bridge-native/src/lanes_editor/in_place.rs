//! What is picked on the canvas, opened where it stands: a step's Name and Words as fields under its node, a
//! state's Name where its lifecycle was read (else to read), a + in each gap between steps and an empty box at the
//! workflow's end that add a step there, picked. Each field he changed is one edit of the draft, put to Fleet as his. The
//! fields stand over the canvas in window points, under the picked node as the canvas drew it last (`picked_bounds`).

use std::cell::Cell;
use std::rc::Rc;

use gpui::{
    canvas, div, point, prelude::*, px, svg, AnyElement, ScrollHandle, Bounds, Context, Div, Entity, Focusable, FontWeight, Keystroke, MouseButton, Pixels, Role,
    SharedString, Subscription, Window,
};
use gpui_component::input::{Input, InputEvent, InputState, Textarea, TextareaState};
use ipc::{JobLanesView, LaneKind, LaneNodeKind};

use super::draft::Change;
use super::LanesEditor;
use crate::assets::icon_path;
use crate::labels::labelled;
use crate::lanes_canvas::{Pick, StepGap};
use crate::look::{token, Look};
use crate::settings::definition::{steps_of, words_of};
use crate::settings::view::{mono, ring_colour, tip};

/// Classic's name for a step the + adds, and the id it is made from (`addAfter`).
pub const NEW_STEP: &str = "New step";

/// The heads' width over the left of the canvas (`GEO.head`, `layout.ts`): a + under them is not drawn.
const HEADS: f32 = 144.;
/// A node is six gaps wide (`GEO`: a 168 node in a 196 column), for a workflow with one gap to measure by.
const NODE_PER_GAP: f32 = 6.;
/// The most lines the Words grow to before they scroll, so the fields have a height to place by.
const WORDS_ROWS: usize = 4;

/// The lanes with where the workflow ends moved a column past its last step. The canvas keeps its pans inside what it
/// lays out, so the column is room it pans to; nothing stands in it, and the empty box that adds a step at the end is
/// drawn there, a column on from the last step.
pub(super) fn with_end_room(mut view: JobLanesView) -> JobLanesView {
    let Some(lane) = view.lanes.iter_mut().find(|lane| lane.kind == LaneKind::Workflow) else { return view };
    let Some(col) = lane.nodes.iter().filter(|one| one.kind == LaneNodeKind::Step).map(|one| one.col + 1).max() else { return view };
    for one in lane.nodes.iter_mut().filter(|one| one.kind == LaneNodeKind::End) {
        one.col = col;
    }
    view
}

/// The node opened in place, and his fields where it is a step of the definition.
pub(super) struct InPlace {
    pub(super) node: String,
    /// What it is named: the group the fields stand in is read as this.
    pub(super) label: String,
    pub(super) fields: Option<Fields>,
    /// He has the keys in the fields, or had them: an edit landing under them leaves them open.
    pub(super) keyed: bool,
}

/// What the fields open in place edit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Editing {
    /// A step of the definition: its Name and Words.
    Step(String),
    /// A state of a lifecycle that was read: its Name.
    State { lifecycle: String, state: String },
}

/// What the panel holds, for its height.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Shape {
    NameAndWords,
    Name,
    Read,
}

impl Shape {
    fn of(one: &InPlace) -> Shape {
        match &one.fields {
            Some(fields) if fields.words.is_some() => Shape::NameAndWords,
            Some(_) => Shape::Name,
            None => Shape::Read,
        }
    }
}

pub(super) struct Fields {
    pub(super) what: Editing,
    pub(super) name: Entity<InputState>,
    /// A step's Words; a state has none.
    pub(super) words: Option<Entity<TextareaState>>,
    was_name: String,
    was_words: String,
    /// The lines the Words grow to, fewer where the frame has no room for them all.
    rows: Cell<usize>,
    _subs: Vec<Subscription>,
}

/// `base`, or `base_2`, `base_3`, until no step holds it (`uniqueId`).
/// Where the lanes begin in the frame, past the rail and the heads that stand over its left: a + under them is not drawn.
pub(super) fn lanes_edge(frame: Bounds<Pixels>, look: &Look) -> Pixels {
    let rail = look.length(token::BORDER_WIDTH) * 3. + look.length(token::SPACE_3) * 2. + look.length(token::SPACE_1) * 2. + look.length(token::H_CONTROL_SM);
    frame.left() + rail + px(HEADS)
}

pub fn unique_id(label: &str, taken: &[String]) -> String {
    let slug: String = label.to_lowercase().chars().map(|one| if one.is_ascii_alphanumeric() { one } else { '_' }).collect();
    let slug = slug.split('_').filter(|part| !part.is_empty()).collect::<Vec<_>>().join("_");
    let base = if slug.is_empty() { "new".to_string() } else { slug };
    let mut id = base.clone();
    let mut n = 2;
    while taken.contains(&id) {
        id = format!("{base}_{n}");
        n += 1;
    }
    id
}

/// The step a workflow lane's node stands for.
fn step_of(node: &str) -> Option<&str> {
    node.strip_prefix("workflow/")
}

/// A node's width, from the gaps either side of the steps.
fn node_width(gaps: &[StepGap]) -> Option<Pixels> {
    match gaps {
        [.., a, b] => Some(b.bounds.left() - a.bounds.left() - b.bounds.size.width),
        [one] => Some(one.bounds.size.width * NODE_PER_GAP),
        [] => None,
    }
}

/// Where a step's node was drawn, from the gaps either side of it.
fn node_bounds(node: &str, gaps: &[StepGap]) -> Option<Bounds<Pixels>> {
    let wide = node_width(gaps)?;
    let into = gaps.iter().find(|gap| gap.before == node);
    let out = gaps.iter().find(|gap| gap.after == node);
    let left = into.map(|gap| gap.bounds.right()).or_else(|| out.map(|gap| gap.bounds.left() - wide))?;
    let gap = into.or(out)?;
    Some(Bounds { origin: point(left, gap.bounds.top()), size: gpui::size(wide, gap.bounds.size.height) })
}

/// The empty box a column on from the workflow's last step, and that step.
fn end_box(gaps: &[StepGap]) -> Option<(String, Bounds<Pixels>)> {
    let last = gaps.iter().max_by(|a, b| f32::from(a.bounds.left()).total_cmp(&f32::from(b.bounds.left())))?;
    let step = node_bounds(&last.before, gaps)?;
    let on = step.size.width + last.bounds.size.width;
    Some((last.before.clone(), Bounds { origin: point(step.left() + on, step.top()), size: step.size }))
}

impl LanesEditor {
    /// The definition as the draft has it: what a field opens on and what a new step is added to.
    pub(super) fn text(&self) -> String {
        if self.text.is_empty() {
            self.saved.clone().unwrap_or_default()
        } else {
            self.text.clone()
        }
    }

    fn label_of(&self, node: &str, cx: &gpui::App) -> Option<String> {
        use crate::lanes_canvas::LanesHost;
        self.lanes_of(&self.id, cx)?.lanes.iter().flat_map(|lane| lane.nodes.iter()).find(|one| one.id == node).map(|one| one.label.clone())
    }

    /// Whether the keys are in a field opened in place.
    pub fn typing(&self, window: &Window, cx: &gpui::App) -> bool {
        self.typing_words(window, cx) || self.in_place.as_ref().and_then(|one| one.fields.as_ref()).is_some_and(|fields| fields.name.read(cx).focus_handle(cx).is_focused(window) || fields.words.as_ref().is_some_and(|words| words.read(cx).focus_handle(cx).is_focused(window)))
    }

    /// The picked node, where it is a step of the workflow's lane.
    fn picked_step(&self) -> Option<String> {
        match &self.picked {
            Some(Pick::Node(node)) if step_of(node).is_some() && !node.ends_with(":start") && !node.ends_with(":end") => Some(node.clone()),
            _ => None,
        }
    }

    /// Opens `node` where it stands: a step of the definition with its fields, anything else to read. With `keyed`
    /// the keys go into its Name. What was open before lands first, as a blur lands it.
    pub(super) fn open_at(&mut self, node: &str, keyed: bool, window: &mut Window, cx: &mut Context<Self>) {
        if self.in_place.as_ref().is_some_and(|one| one.node != node) {
            self.settle(window, cx);
        }
        let found = {
            use crate::lanes_canvas::LanesHost;
            self.lanes_of(&self.id, cx).and_then(|lanes| lanes.lanes.iter().find_map(|lane| lane.nodes.iter().find(|one| one.id == node).map(|one| (lane.kind, one.kind, one.label.clone()))))
        };
        let Some((lane, kind, label)) = found.filter(|(_, kind, _)| !matches!(kind, LaneNodeKind::Start | LaneNodeKind::End)) else {
            self.in_place = None;
            cx.notify();
            return;
        };
        if self.in_place.as_ref().is_some_and(|one| one.node == node) {
            if keyed {
                self.key_into_name(window, cx);
            }
            return;
        }
        let what = self.editing(node, lane, kind);
        let fields = what.map(|what| {
            let text = self.text();
            let (name, words) = match &what {
                Editing::Step(step) => (steps_of(&text).into_iter().find(|(id, _)| id == step).map(|(_, name)| name).unwrap_or_default(), Some(words_of(&text, step).unwrap_or_default())),
                Editing::State { .. } => (label.clone(), None),
            };
            let name_box = cx.new(|cx| InputState::new(window, cx));
            name_box.update(cx, |state, cx| state.set_value(name.clone(), window, cx));
            let mut subs = vec![cx.subscribe_in(&name_box, window, |this, _, event: &InputEvent, window, cx| this.field_said(event, window, cx))];
            let words_box = words.as_ref().map(|words| {
                let words_box = cx.new(|cx| TextareaState::new(window, cx).auto_grow(2, WORDS_ROWS));
                words_box.update(cx, |state, cx| state.set_value(words.clone(), window, cx));
                subs.push(cx.subscribe_in(&words_box, window, |this, _, event: &InputEvent, window, cx| this.field_said(event, window, cx)));
                words_box
            });
            Fields { what, name: name_box, words: words_box, was_name: name, was_words: words.unwrap_or_default(), rows: Cell::new(WORDS_ROWS), _subs: subs }
        });
        let label = fields.as_ref().map_or(label, |fields| fields.was_name.clone());
        self.in_place = Some(InPlace { node: node.to_string(), label, fields, keyed: false });
        self.reveal.set(true);
        if keyed {
            self.key_into_name(window, cx);
        }
        cx.notify();
    }

    /// What a node's fields edit: a step the definition holds, or a state of a lifecycle that was read. Anything else
    /// opens to read.
    fn editing(&self, node: &str, lane: LaneKind, kind: LaneNodeKind) -> Option<Editing> {
        match (lane, kind) {
            (LaneKind::Workflow, LaneNodeKind::Step) => {
                let step = step_of(node)?;
                steps_of(&self.text()).iter().any(|(id, _)| id == step).then(|| Editing::Step(step.to_string()))
            }
            (LaneKind::Lifecycle, _) => {
                let (lifecycle, state) = node.split_once('/')?;
                let running = self.lifecycles.iter().find(|one| one.lifecycle.id == lifecycle && one.kept.overridden_by.is_none())?;
                running.lifecycle.states.iter().any(|one| one.id == state).then(|| Editing::State { lifecycle: lifecycle.to_string(), state: state.to_string() })
            }
            _ => None,
        }
    }

    /// The keys into the Name, all of it selected so what is typed replaces it.
    fn key_into_name(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(one) = self.in_place.as_mut() else { return };
        let Some(fields) = &one.fields else { return };
        one.keyed = true;
        fields.name.update(cx, |state, cx| {
            state.focus(window, cx);
            state.select_all(window, cx);
        });
        cx.notify();
    }

    fn field_said(&mut self, event: &InputEvent, window: &mut Window, cx: &mut Context<Self>) {
        match event {
            InputEvent::Focus => {
                if let Some(one) = self.in_place.as_mut() {
                    one.keyed = true;
                }
            }
            // The keys left both fields for somewhere else: what he typed lands.
            InputEvent::Blur if !self.typing(window, cx) => self.settle(window, cx),
            _ => {}
        }
    }

    /// What is open in place, closed, each field he changed landed as one edit of his.
    pub(super) fn settle(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        let Some(InPlace { fields: Some(fields), .. }) = self.in_place.take() else { return };
        let name = fields.name.read(cx).value().trim().to_string();
        let renamed = (!name.is_empty() && name != fields.was_name).then(|| format!("Renamed {} to {name}", fields.was_name));
        match &fields.what {
            Editing::Step(step) => {
                let mut named = fields.was_name.clone();
                if let Some(said) = renamed {
                    self.edit(said, Change::StepLabel { step: step.clone(), label: name.clone() }, cx);
                    named = name;
                }
                let words = fields.words.as_ref().map(|words| words.read(cx).value().to_string()).unwrap_or_default();
                if words != fields.was_words {
                    self.edit(format!("Changed the words of {named}"), Change::StepWords { step: step.clone(), words }, cx);
                }
            }
            Editing::State { lifecycle, state } => {
                if let Some(said) = renamed {
                    self.edit(said, Change::StateLabel { lifecycle: lifecycle.clone(), state: state.clone(), label: name }, cx);
                }
            }
        }
        cx.notify();
    }

    /// Enter: what he changed lands, and the keys go back to the canvas.
    pub(super) fn commit_in_place(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.settle(window, cx);
        self.in_place = None;
        self.give_back(window, cx);
    }

    /// Esc: closed with no change, and the keys back to the canvas.
    pub(super) fn close_in_place(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.in_place = None;
        self.give_back(window, cx);
    }

    /// The keys to the canvas once the key that closed the fields is through: gpui still sends a key a host took to
    /// the key-down listeners on its way, and the canvas's own would take it again.
    pub(super) fn give_back(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        cx.defer_in(window, |this, window, cx| window.focus(&this.state.frame, cx));
        cx.notify();
    }

    /// A step added just after `after`, or after the workflow's last where it is none, with its fields open and the
    /// keys in its Name.
    pub(super) fn add_after(&mut self, after: Option<String>, window: &mut Window, cx: &mut Context<Self>) {
        self.settle(window, cx);
        let text = self.text();
        let steps = steps_of(&text);
        let Some((after, named)) = after.and_then(|after| steps.iter().find(|(id, _)| *id == after).cloned()).or_else(|| steps.last().cloned()) else { return };
        let taken: Vec<String> = steps.iter().map(|(id, _)| id.clone()).collect();
        let id = unique_id(NEW_STEP, &taken);
        self.edit(format!("Added {NEW_STEP} after {named}"), Change::AddStep { after, id: id.clone(), label: NEW_STEP.into() }, cx);
        // The new step is picked, as a press on it would: its fields stand under it, and A adds after it next.
        let node = format!("workflow/{id}");
        self.state.pick(&self.id, Some(Pick::Node(node.clone())));
        self.picked = Some(Pick::Node(node.clone()));
        self.armed = false;
        self.open_at(&node, true, window, cx);
    }

    /// The box at the lane's end: a step after the lane's last where the definition holds it, else after the
    /// definition's last.
    pub(super) fn add_at_end(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let last = end_box(&self.state.step_gaps(&self.id)).map(|(last, _)| last);
        self.add_after(last.as_deref().and_then(step_of).map(String::from), window, cx);
    }

    /// The fields' own keys while they hold them: Tab between Name and Words, Enter lands them, Esc closes them.
    /// Shift-Enter is the Words' new line.
    pub fn field_key(&mut self, key: &Keystroke, window: &mut Window, cx: &mut Context<Self>) -> bool {
        if self.typing_words(window, cx) {
            return self.words_key(key, window, cx);
        }
        let mods = &key.modifiers;
        if mods.platform || mods.control || mods.alt {
            return false;
        }
        match key.key.as_str() {
            "tab" => {
                let Some(fields) = self.in_place.as_ref().and_then(|one| one.fields.as_ref()) else { return false };
                // A state's Name is its only field: Tab stays in it.
                let Some(words) = fields.words.clone() else { return true };
                let name = fields.name.clone();
                if name.read(cx).focus_handle(cx).is_focused(window) {
                    words.update(cx, |state, cx| state.focus(window, cx));
                } else {
                    name.update(cx, |state, cx| state.focus(window, cx));
                }
                cx.notify();
                true
            }
            "enter" if !mods.shift => {
                self.commit_in_place(window, cx);
                true
            }
            "escape" => {
                self.close_in_place(window, cx);
                true
            }
            _ => false,
        }
    }

    /// The canvas's keys the editor takes first: Enter on the step just picked puts the keys in its fields; a key
    /// that moves off it leaves Enter to the canvas.
    pub(super) fn canvas_key(&mut self, key: &Keystroke, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let mods = &key.modifiers;
        if mods.platform || mods.control || mods.alt || mods.shift {
            return false;
        }
        match key.key.as_str() {
            "enter" if self.armed => self.open_picked(window, cx),
            "up" | "down" | "left" | "right" | "[" | "]" => {
                self.armed = false;
                false
            }
            _ => false,
        }
    }

    /// E: the picked step's or state's fields, with the keys in its Name. `false` where it has none to edit.
    pub(super) fn open_picked(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        if let Some(Pick::Rule(rule)) = self.picked.clone() {
            self.open_rule(&rule, true, window, cx);
            return self.sentence.is_some();
        }
        let Some(Pick::Node(node)) = self.picked.clone() else { return false };
        let found = {
            use crate::lanes_canvas::LanesHost;
            self.lanes_of(&self.id, cx).and_then(|lanes| lanes.lanes.iter().find_map(|lane| lane.nodes.iter().find(|one| one.id == node).map(|one| (lane.kind, one.kind))))
        };
        if !found.is_some_and(|(lane, kind)| self.editing(&node, lane, kind).is_some()) {
            return false;
        }
        self.open_at(&node, true, window, cx);
        true
    }

    /// A: a step added after the picked one.
    pub(super) fn add_after_picked(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let Some(node) = self.picked_step() else { return false };
        self.add_after(step_of(&node).map(String::from), window, cx);
        true
    }

    // ---- drawn --------------------------------------------------------------------------------

    /// What stands over the canvas: a + in each gap it shows, the box at the workflow's end, and what is open in
    /// place. `frame` is where the canvas was drawn last, in window points.
    pub(super) fn render_in_place(&self, frame: Bounds<Pixels>, look: &Look, cx: &Context<Self>) -> Vec<AnyElement> {
        let mut over = Vec::new();
        if frame.size.width <= px(0.) {
            return over;
        }
        let gaps = self.state.step_gaps(&self.id);
        let edge = lanes_edge(frame, look);
        // What stands over the lanes is cut where the canvas cuts its nodes, at the heads' edge and the frame's.
        let shown = |bounds: &Bounds<Pixels>| bounds.right() > edge && bounds.left() < frame.right() && bounds.bottom() > frame.top() && bounds.top() < frame.bottom();
        let lanes = Bounds { origin: point(edge, frame.top()), size: gpui::size((frame.right() - edge).max(px(0.)), frame.size.height) };
        let mut cut = Vec::new();
        for gap in gaps.iter().filter(|gap| shown(&gap.bounds)) {
            let named = self.label_of(&gap.after, cx).unwrap_or_default();
            cut.push(self.plus(look, lanes, gap.bounds, &gap.after, step_of(&gap.after).map(String::from), named, cx));
        }
        if let Some((last, bounds)) = end_box(&gaps).filter(|(_, bounds)| shown(bounds)) {
            let named = self.label_of(&last, cx).unwrap_or_default();
            cut.push(self.end_box(look, lanes, bounds, named, cx));
        }
        over.push(div().absolute().left(edge - frame.left()).top_0().right_0().bottom_0().overflow_hidden().children(cut).into_any_element());
        over.extend(self.render_fields(frame, look));
        over
    }

    #[allow(clippy::too_many_arguments)]
    fn plus(&self, look: &Look, frame: Bounds<Pixels>, gap: Bounds<Pixels>, node: &str, after: Option<String>, named: String, cx: &Context<Self>) -> AnyElement {
        let wide = look.length(token::H_CONTROL_SM);
        let size = if gap.size.width < wide { gap.size.width } else { wide };
        let at = gap.center();
        let said = format!("Add after {named}");
        let button = div()
            .id(SharedString::from(format!("lanes-add-after:{node}")))
            .absolute()
            .left(at.x - frame.left() - size / 2.)
            .top(at.y - frame.top() - size / 2.)
            .size(size)
            .flex()
            .items_center()
            .justify_center()
            .rounded_full()
            .border(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_STRONG))
            .bg(look.color(token::BG_RAISED))
            .cursor_pointer()
            .hover(|style| style.bg(look.color(token::BG_HOVER)).border_color(look.color(token::ACCENT)))
            .occlude()
            .tooltip(tip(said.clone()))
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_click(cx.listener(move |this, _, window, cx| this.add_after(after.clone(), window, cx)))
            .child(svg().path(icon_path("plus")).size(look.length(token::SPACE_3)).text_color(look.color(token::FG_MUTED)));
        labelled(button, Role::Button, said).into_any_element()
    }

    fn end_box(&self, look: &Look, frame: Bounds<Pixels>, bounds: Bounds<Pixels>, named: String, cx: &Context<Self>) -> AnyElement {
        let said = format!("Add after {named}");
        // The keys are on this box once → went past the workflow lane's last step.
        use crate::lanes_canvas::LanesHost;
        let keyed = self.state.at_end(&self.id).is_some_and(|lane| {
            self.lanes_of(&self.id, cx).is_some_and(|lanes| lanes.lanes.iter().any(|one| one.id == lane && one.kind == LaneKind::Workflow))
        });
        let button = div()
            .id("lanes-add-at-end")
            .absolute()
            .left(bounds.left() - frame.left())
            .top(bounds.top() - frame.top())
            .w(bounds.size.width)
            .h(bounds.size.height)
            .flex()
            .items_center()
            .justify_center()
            .rounded(look.length(token::RADIUS_MD))
            .border(look.length(token::BORDER_WIDTH))
            .when(!keyed, |button| button.border_dashed())
            .border_color(look.color(token::BORDER_STRONG))
            .when(keyed, |button| button.border(look.length(token::FOCUS_RING_WIDTH)).border_color(ring_colour(look)))
            .bg(look.color(token::BG_SUNKEN))
            .cursor_pointer()
            .hover(|style| style.bg(look.color(token::BG_HOVER)).border_color(look.color(token::ACCENT)))
            .occlude()
            .tooltip(tip(said.clone()))
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_click(cx.listener(|this, _, window, cx| this.add_at_end(window, cx)))
            .child(svg().path(icon_path("plus")).size(look.length(token::SPACE_4)).text_color(look.color(token::FG_SUBTLE)));
        labelled(button, Role::Button, said).into_any_element()
    }

    /// The panel's height with the Words this many lines tall: padding and border, a label over each field, the
    /// Name's line and the Words. A state's Name is one label and its field; a name to read, one label and one line.
    fn panel_tall(look: &Look, shape: Shape, rows: usize) -> Pixels {
        let label = look.length(token::LEADING_2XS) + look.length(token::SPACE_1);
        let shell = look.length(token::SPACE_3) * 2. + look.length(token::BORDER_WIDTH) * 2.;
        match shape {
            Shape::NameAndWords => shell + look.length(token::SPACE_3) + label * 2. + look.length(token::SPACE_8) + look.length(token::LEADING_SM) * rows as f32 + look.length(token::SPACE_4),
            Shape::Name => shell + label + look.length(token::SPACE_8),
            Shape::Read => shell + label + look.length(token::LEADING_SM),
        }
    }

    /// The picked node the open fields stand under, as the canvas drew it last.
    fn open_node(&self) -> Option<Bounds<Pixels>> {
        let one = self.in_place.as_ref()?;
        match &self.picked {
            Some(Pick::Node(picked)) if *picked == one.node => self.state.picked_bounds(&self.id),
            _ => None,
        }
    }

    /// Where the panel stands, and how many lines its Words get. It stays inside the part of the canvas's frame that
    /// is drawn: under the node where it fits, else over it, else beside it on the side with more room; where none
    /// fits it is cut to the frame's height, the Words shortened to what that leaves and the Name kept whole.
    pub(super) fn place(&self, look: &Look) -> Option<(Bounds<Pixels>, usize)> {
        let one = self.in_place.as_ref()?;
        let node = self.open_node()?;
        let seen = self.seen.get();
        let gap = look.length(token::SPACE_2);
        let area = Bounds { origin: point(seen.left() + gap, seen.top() + gap), size: gpui::size(seen.size.width - gap * 2., seen.size.height - gap * 2.) };
        let shape = Shape::of(one);
        let wide = look.length(token::SPACE_12) * 6.;
        let wide = if wide > area.size.width { area.size.width.max(px(0.)) } else { wide };
        let full = Self::panel_tall(look, shape, WORDS_ROWS);
        let clamp = |at: Pixels, low: Pixels, high: Pixels| if at > high { high } else { at }.max(low);
        let x = clamp(node.left(), area.left(), area.right() - wide);
        let below = area.bottom() - (node.bottom() + gap);
        let above = (node.top() - gap) - area.top();
        let at = |x: Pixels, y: Pixels, tall: Pixels| Bounds { origin: point(x, y), size: gpui::size(wide, tall) };
        if below >= full {
            return Some((at(x, node.bottom() + gap, full), WORDS_ROWS));
        }
        if above >= full {
            return Some((at(x, node.top() - gap - full, full), WORDS_ROWS));
        }
        let right = area.right() - (node.right() + gap);
        let left = (node.left() - gap) - area.left();
        if right.max(left) >= wide && area.size.height >= full {
            let x = if right >= left { node.right() + gap } else { node.left() - gap - wide };
            return Some((at(x, clamp(node.top(), area.top(), area.bottom() - full), full), WORDS_ROWS));
        }
        let rows = if shape == Shape::NameAndWords {
            let spare = area.size.height - Self::panel_tall(look, shape, 0);
            ((spare / look.length(token::LEADING_SM)).floor() as usize).clamp(1, WORDS_ROWS)
        } else {
            WORDS_ROWS
        };
        let tall = Self::panel_tall(look, shape, rows);
        let y = if below >= above { node.bottom() + gap } else { node.top() - gap - tall };
        Some((at(x, clamp(y, area.top(), area.bottom() - tall), tall), rows))
    }

    /// The page the editor scrolls in: fields that open are scrolled into view in it. The host's one line.
    pub fn scrolls_in(&mut self, page: ScrollHandle) {
        self.page = Some(page);
    }

    /// What the page shows when fields open, relative to the canvas's frame: the node, and the panel at its whole
    /// height under it.
    pub(super) fn reveal_target(&self, look: &Look) -> Option<Bounds<Pixels>> {
        if self.sentence.is_some() {
            return self.sentence_reveal(look);
        }
        let one = self.in_place.as_ref()?;
        let node = self.open_node()?;
        let frame = self.frame.get();
        let gap = look.length(token::SPACE_2);
        let tall = node.size.height + (gap * 2. + Self::panel_tall(look, Shape::of(one), WORDS_ROWS));
        Some(Bounds { origin: point(node.left() - frame.left(), node.top() - frame.top()), size: gpui::size(node.size.width, tall) })
    }

    /// The Words shortened to the lines `place` leaves them, before the panel is drawn at that height.
    pub(super) fn fit_words(&self, look: &Look, cx: &mut Context<Self>) {
        let Some((_, rows)) = self.place(look) else { return };
        let Some(fields) = self.in_place.as_ref().and_then(|one| one.fields.as_ref()) else { return };
        let Some(words) = &fields.words else { return };
        if fields.rows.replace(rows) != rows {
            words.update(cx, |state, cx| state.set_auto_grow(rows.min(2), rows, cx));
        }
    }

    /// The fields where `place` puts them, in a layer cut to the canvas's frame: nothing outside it is covered.
    fn render_fields(&self, frame: Bounds<Pixels>, look: &Look) -> Option<AnyElement> {
        let one = self.in_place.as_ref()?;
        let (at, _) = self.place(look)?;
        let line = look.length(token::LEADING_SM);
        let rows: Vec<AnyElement> = match &one.fields {
            Some(fields) => std::iter::once(row(look, "Name", labelled(div().id("lanes-in-place-name").key_context(crate::keymap::TEXT_BOX).w_full().text_size(look.length(token::TEXT_SM)).line_height(line).child(Input::new(&fields.name)), Role::TextInput, "Name").into_any_element()).flex_none().into_any_element())
                .chain(fields.words.as_ref().map(|words| {
                    row(look, "Words", labelled(div().id("lanes-in-place-words").key_context(crate::keymap::TEXT_BOX).w_full().flex_1().min_h_0().text_size(look.length(token::TEXT_SM)).line_height(line).child(Textarea::new(words)), Role::TextInput, "Words").into_any_element()).flex_1().min_h_0().into_any_element()
                }))
                .collect(),
            None => vec![row(
                look,
                "Name",
                labelled(div().id("lanes-in-place-read").text_size(look.length(token::TEXT_SM)).line_height(line).text_color(look.color(token::FG_DEFAULT)).child(SharedString::from(one.label.clone())), Role::Label, one.label.clone()).into_any_element(),
            )
            .into_any_element()],
        };
        let panel = div()
            .id("lanes-in-place")
            .absolute()
            .left(at.left() - frame.left())
            .top(at.top() - frame.top())
            .w(at.size.width)
            .max_h(at.size.height)
            .overflow_hidden()
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_3))
            .p(look.length(token::SPACE_3))
            .rounded(look.length(token::RADIUS_LG))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_DEFAULT))
            .bg(look.color(token::BG_OVERLAY))
            .shadow(look.shadows(token::SHADOW_OVERLAY))
            .occlude()
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .children(rows);
        Some(div().absolute().top_0().left_0().size_full().overflow_hidden().child(labelled(panel, Role::Group, one.label.clone())).into_any_element())
    }
}

/// A field under its label, as Classic's step card sets a fact (`armada-wm-step__facts`).
fn row(look: &Look, said: &'static str, value: AnyElement) -> Div {
    let label = div()
        .font_family(mono(look))
        .text_size(look.length(token::TEXT_2XS))
        .line_height(look.length(token::LEADING_2XS))
        .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
        .text_color(look.color(token::FG_SUBTLE))
        .child(said);
    div().flex().flex_col().gap(look.length(token::SPACE_1)).min_w_0().child(label).child(value)
}

/// Records where the canvas's frame was drawn, for what stands over it, and the part of it the page let be seen.
/// While fields are open (`watch`), a change in that part draws again, so the fields follow a scroll of the page.
/// `reveal` is a place relative to the frame that the page is scrolled the least to show whole, or its top where it is
/// taller than the page.
pub(super) fn measure(at: Rc<Cell<Bounds<Pixels>>>, seen: Rc<Cell<Bounds<Pixels>>>, reveal: Option<(Bounds<Pixels>, ScrollHandle)>, watch: bool) -> impl IntoElement {
    canvas(
        move |bounds, window, _| {
            at.set(bounds);
            let shown = bounds.intersect(&window.content_mask().bounds);
            if seen.replace(shown) != shown && watch {
                window.refresh();
            }
            let Some((place, page)) = reveal else { return };
            let place = Bounds { origin: point(bounds.left() + place.left(), bounds.top() + place.top()), size: place.size };
            let view = page.bounds();
            let mut by = Pixels::ZERO;
            if place.bottom() > view.bottom() {
                by = place.bottom() - view.bottom();
            }
            if place.top() - by < view.top() {
                by = place.top() - view.top();
            }
            if by == Pixels::ZERO {
                return;
            }
            let offset = page.offset();
            let y = (offset.y - by).max(-page.max_offset().y).min(Pixels::ZERO);
            page.set_offset(point(offset.x, y));
            window.refresh();
        },
        |_, _, _, _| {},
    )
    .absolute()
    .top_0()
    .left_0()
    .size_full()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_new_steps_id_is_its_name_slugged_and_numbered_past_the_ones_taken() {
        assert_eq!(unique_id(NEW_STEP, &[]), "new_step");
        assert_eq!(unique_id(NEW_STEP, &["new_step".into(), "new_step_2".into()]), "new_step_3");
        assert_eq!(unique_id("  ", &[]), "new");
    }
}
