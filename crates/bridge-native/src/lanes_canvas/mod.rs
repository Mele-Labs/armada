//! A workflow's lanes as Fleet works them out (`ipc::JobLanesView`), drawn as Classic's `LaneCanvas` draws
//! them (`LaneCanvas.tsx`). A Job's Workflow tab hosts it, and any other view that holds lanes can.
//!
//! **Fleet works the lanes out; this draws them.** The geometry is Classic's `layout.ts`: one canvas unit
//! is one pixel at zoom 1, a column is shared by every lane, and the lanes share the frame's height. The
//! heads stand over the left of the frame and keep each lane named as the canvas pans.
//!
//! The host keeps a [`LanesState`] and implements [`LanesHost`] for what a press means where it is drawn.

mod draw;
mod layout;
mod paint;
mod wears;

use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use gpui::{
    canvas, div, prelude::*, px, AnyElement, App, Bounds, ContentMask, Context, Div, FocusHandle, Hsla, KeyDownEvent, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, Pixels,
    Role, ScrollEvent, SharedString, Stateful, Window,
};
use ipc::{JobLanesView, LaneNodeKind, LaneView};

use crate::job_view::rows::{glyph, tip, Size};
use crate::job_view::sketch_pad::{rail_frame, rail_group};
use crate::labels::labelled;
use crate::look::{token, Look};
use draw::{lane_chip, lane_head, lane_node};
use layout::{nearest, pan_to, place, place_chips};
use paint::PaintPlan;
use wears::{mark_of_node, mark_of_rule};

pub(crate) use layout::clock;
pub(crate) use wears::mix;

/// What a lane's head says when a double press opens the lanes of the Job it stands for.
pub(crate) const OPENS_SAID: &str = "Double-click to open its Workflow";

// `GEO`, `layout.ts`.
const HEAD: f32 = 144.;
const PAD: f32 = 16.;
const COL_W: f32 = 196.;
const NODE_W: f32 = 168.;
const STEP_H: f32 = 68.;
const STATE_H: f32 = 56.;
const SUB_CHARS: f32 = 20.;
const SUB_LINE: f32 = 16.;
pub const LANE_MIN: f32 = 128.;
const CHIP_H: f32 = 28.;
const DOT: f32 = 24.;
const DOT_GAP: f32 = 16.;
const LEAD: f32 = 48.;
/// How far out the canvas goes (`MIN_ZOOM`), and in (React Flow's `maxZoom`).
const MIN_ZOOM: f32 = 0.8;
const MAX_ZOOM: f32 = 2.;
/// React Flow's `zoomIn` and `zoomOut`.
const ZOOM_STEP: f32 = 1.2;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct Rect {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct View {
    x: f32,
    y: f32,
    zoom: f32,
}

/// Where the canvas opens and Fit returns to (`HOME`).
const HOME: View = View { x: 0., y: 0., zoom: 1. };

/// What is picked (`LanePick`): a node or a rule, by id.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Pick {
    Node(String),
    Rule(String),
}

/// A pan asked of the canvas (`Pan`), met once the frame has a size.
#[derive(Clone)]
struct Ask {
    rects: Vec<Rect>,
    follow: bool,
    earliest: bool,
    /// The first rect is a pick and stays in view whatever else is cut.
    keep: bool,
}

/// One Job's canvas as he left it.
struct Seen {
    view: View,
    picked: Option<Pick>,
    /// The node or chip the keys are on: a node's id, or an edge's.
    current: Option<String>,
    /// What each node and chip was drawn as, so a change can be told (`markOfNode`, `markOfRule`).
    drawn: Option<HashMap<String, String>>,
    ask: Option<Ask>,
    /// A drag on the canvas: where it began and where the view stood.
    drag: Option<(gpui::Point<Pixels>, View)>,
    /// Where the pick and the gaps between steps were drawn last frame, in the pane's own points.
    hooks: Hooks,
}

impl Default for Seen {
    fn default() -> Self {
        Self { view: HOME, picked: None, current: None, drawn: None, ask: None, drag: None, hooks: Hooks::default() }
    }
}

#[derive(Clone, Default)]
struct Hooks {
    picked: Option<Rect>,
    gaps: Vec<(String, String, Rect)>,
}

/// The room between two steps of the workflow's lane, where an editor mounts its +.
#[derive(Clone, Debug, PartialEq)]
pub struct StepGap {
    /// The step before the gap and the one after it, by node id.
    pub after: String,
    pub before: String,
    /// In window points, as last drawn.
    pub bounds: Bounds<Pixels>,
}

/// The canvas's state on its host: the frame's keys, and each canvas as he left it, by the id it is drawn under.
pub struct LanesState {
    /// The canvas frame, which holds the keys while a node or chip has them.
    pub frame: FocusHandle,
    /// The frame holds the keys, so the node or chip they are on wears the ring. The host sets it from
    /// the frame's focus and blur.
    pub keyed: Cell<bool>,
    seen: RefCell<HashMap<SharedString, Seen>>,
    /// The pane's size last frame, heads included.
    size: Rc<Cell<(f32, f32)>>,
    /// The pane's top left in the window last frame.
    origin: Rc<Cell<gpui::Point<Pixels>>>,
}

impl LanesState {
    pub fn new(cx: &mut App) -> Self {
        Self { frame: cx.focus_handle(), keyed: Cell::new(false), seen: RefCell::default(), size: Rc::new(Cell::new((0., 0.))), origin: Rc::default() }
    }

    fn in_window(&self, rect: Rect) -> Bounds<Pixels> {
        let origin = self.origin.get();
        Bounds { origin: gpui::point(origin.x + px(rect.x), origin.y + px(rect.y)), size: gpui::size(px(rect.w), px(rect.h)) }
    }

    /// Where the picked node or rule chip of the canvas drawn under `id` stands in the window, as last drawn.
    pub fn picked_bounds(&self, id: &SharedString) -> Option<Bounds<Pixels>> {
        let picked = self.seen.borrow().get(id)?.hooks.picked?;
        Some(self.in_window(picked))
    }

    /// Each gap between two steps of the workflow's lane, in order, as last drawn: room for an editor's +.
    pub fn step_gaps(&self, id: &SharedString) -> Vec<StepGap> {
        let seen = self.seen.borrow();
        let Some(seen) = seen.get(id) else { return Vec::new() };
        seen.hooks.gaps.iter().map(|(after, before, rect)| StepGap { after: after.clone(), before: before.clone(), bounds: self.in_window(*rect) }).collect()
    }

    /// Pick a node or rule of the canvas drawn under `id`, or let go (`None`), from outside the canvas,
    /// as a press would; a picked node takes the keys too. The host repaints.
    pub fn pick(&self, id: &SharedString, pick: Option<Pick>) {
        let mut seen = self.seen.borrow_mut();
        let seen = seen.entry(id.clone()).or_default();
        if let Some(Pick::Node(node)) = &pick {
            seen.current = Some(node.clone());
        }
        seen.picked = pick;
    }
}

/// A view that draws the canvas. What a press means is the host's; the defaults do nothing.
pub trait LanesHost: Sized + 'static {
    /// The canvas's state, kept on the host.
    fn lanes_state(&self) -> &LanesState;

    /// The lanes drawn under `id`, which the keys read when they move.
    fn lanes_of<'a>(&'a self, id: &SharedString, cx: &'a App) -> Option<&'a JobLanesView>;

    /// A node or rule was picked, or let go (`None`), by a press or a key.
    fn lanes_picked(&mut self, _id: &SharedString, _pick: Option<Pick>, _window: &mut Window, _cx: &mut Context<Self>) {}

    /// The keys or a press reached the canvas: where the host scrolls, it brings the canvas into view.
    fn lanes_reveal(&self, _window: &mut Window, _cx: &mut Context<Self>) {}

    /// Whether a lane's head opens the lanes of the Job it stands for, so its tooltip says so.
    fn head_opens(&self, _lanes: &JobLanesView, _lane: &LaneView, _cx: &App) -> bool {
        false
    }

    /// A lane's head words, given what presses and keys the host has for them.
    fn head_words(&self, _id: &SharedString, _lane: &LaneView, _opens: bool, words: Stateful<Div>, _look: &Look, _cx: &Context<Self>) -> AnyElement {
        words.into_any_element()
    }

    /// Whether a pan may go one empty column past the last node, where the host draws a box to add a step.
    fn lanes_reach_past_end(&self) -> bool {
        false
    }
}

/// Each lane's band and each node's box (`Placement`).
struct Placement {
    bands: Vec<Rect>,
    items: HashMap<String, Rect>,
    lane_h: f32,
    width: f32,
    height: f32,
}

impl Placement {
    /// How far right a pan may go: the lanes, and with `past_end` one empty column more.
    fn reach(&self, past_end: bool) -> f32 {
        self.width + if past_end { COL_W } else { 0. }
    }
}

/// A node's hue, frame and ground, and its words' colour and weight, as its states set them in Classic's order.
struct Wears {
    hue: Hsla,
    frame: Hsla,
    ground: Hsla,
    ink: Hsla,
    heavy: bool,
    /// The inset frame `here` and `picked` add.
    inset: Option<Hsla>,
    dashed: bool,
}

/// When the lanes change, what changed is brought into view; as they open, where the Job stands (`LaneCanvas`'s follow).
fn lanes_follow(seen: &mut Seen, lanes: &JobLanesView, placement: &Placement, chips: &HashMap<String, Rect>) {
    let mut now = HashMap::new();
    for node in lanes.lanes.iter().flat_map(|lane| lane.nodes.iter()) {
        now.insert(node.id.clone(), mark_of_node(node));
    }
    for edge in &lanes.rules {
        now.insert(edge.id.clone(), mark_of_rule(edge));
    }
    let before = seen.drawn.replace(now.clone());
    if before.as_ref() == Some(&now) {
        return;
    }
    // A step the Job skips is marked where it stands, but nothing happens there: the view goes to what does.
    let standing: HashSet<&str> = lanes.lanes.iter().flat_map(|lane| lane.nodes.iter()).filter(|node| !node.skipped && (node.lit || node.here || !node.waiting.is_empty() || node.changed.is_some())).map(|node| node.id.as_str()).collect();
    let mut ids: Vec<String> = Vec::new();
    let add = |id: &str, ids: &mut Vec<String>| {
        if !ids.iter().any(|one| one == id) {
            ids.push(id.to_string());
        }
    };
    for edge in &lanes.rules {
        let fresh = match &before {
            None => edge.proposed || standing.contains(edge.from.as_str()) || standing.contains(edge.to.as_str()),
            Some(before) => before.get(&edge.id) != now.get(&edge.id),
        };
        if fresh {
            for one in [&edge.id, &edge.from, &edge.to] {
                add(one, &mut ids);
            }
        }
    }
    for lane in &lanes.lanes {
        let prefix = format!("{}/", lane.id);
        let lane_was = before.as_ref().is_some_and(|before| before.keys().any(|key| key.starts_with(&prefix)));
        for node in lane.nodes.iter().filter(|node| !node.skipped) {
            let fresh = match &before {
                None => standing.contains(node.id.as_str()),
                Some(before) => match before.get(&node.id) {
                    None => lane_was || standing.contains(node.id.as_str()),
                    Some(was) => Some(was) != now.get(&node.id),
                },
            };
            if fresh {
                add(&node.id, &mut ids);
            }
        }
    }
    if ids.is_empty() {
        return;
    }
    // What was added to the Job's copy is shown where it stands; what else changed puts the earliest of it at the left.
    let copied: HashSet<&str> = lanes
        .lanes
        .iter()
        .flat_map(|lane| lane.nodes.iter())
        .filter(|node| node.changed.is_some())
        .map(|node| node.id.as_str())
        .chain(lanes.rules.iter().filter(|edge| edge.changed.is_some()).flat_map(|edge| [edge.id.as_str(), edge.from.as_str(), edge.to.as_str()]))
        .collect();
    let follow = before.is_none() || ids.iter().any(|id| !copied.contains(id.as_str()));
    let rects = rects_of(&ids, placement, chips);
    if !rects.is_empty() {
        seen.ask = Some(Ask { rects, follow, earliest: before.is_none(), keep: false });
    }
}

/// The rects of nodes and chips by id, chips as their boxes.
fn rects_of(ids: &[String], placement: &Placement, chips: &HashMap<String, Rect>) -> Vec<Rect> {
    ids.iter().filter_map(|id| placement.items.get(id).copied().or_else(|| chips.get(id).map(|chip| Rect { x: chip.x - chip.w / 2., y: chip.y - chip.h / 2., w: chip.w, h: chip.h }))).collect()
}

/// A Job's canvas: the rail, then the lanes with their heads over the left (`LaneCanvas`).
/// The pick's rect and the gaps between the workflow's steps, moved and scaled as the view draws them.
fn hooks_of(lanes: &JobLanesView, placement: &Placement, chips: &HashMap<String, Rect>, picked: Option<&Pick>, view: View) -> Hooks {
    let shown = |rect: &Rect| Rect { x: view.x + rect.x * view.zoom, y: view.y + rect.y * view.zoom, w: rect.w * view.zoom, h: rect.h * view.zoom };
    let picked = match picked {
        Some(Pick::Node(node)) => placement.items.get(node).map(shown),
        Some(Pick::Rule(rule)) => lanes.rules.iter().find(|edge| &edge.rule == rule).and_then(|edge| chips.get(&edge.id)).map(shown),
        None => None,
    };
    let mut steps: Vec<(&str, &Rect)> = lanes
        .lanes
        .iter()
        .filter(|lane| lane.kind == ipc::LaneKind::Workflow)
        .flat_map(|lane| lane.nodes.iter())
        .filter(|node| !matches!(node.kind, LaneNodeKind::Start | LaneNodeKind::End))
        .filter_map(|node| Some((node.id.as_str(), placement.items.get(&node.id)?)))
        .collect();
    steps.sort_by(|a, b| a.1.x.total_cmp(&b.1.x));
    let gaps = steps
        .windows(2)
        .map(|pair| {
            let (a, b) = (pair[0].1, pair[1].1);
            let x = a.x + a.w;
            let gap = Rect { x, y: a.y.min(b.y), w: (b.x - x).max(0.), h: a.h.max(b.h) };
            (pair[0].0.to_string(), pair[1].0.to_string(), shown(&gap))
        })
        .collect();
    Hooks { picked, gaps }
}

pub fn lanes_canvas<H: LanesHost>(host: &H, id: &SharedString, lanes: &JobLanesView, look: &Look, cx: &Context<H>) -> AnyElement {
    let state = host.lanes_state();
    let (frame_w, frame_h) = state.size.get();
    let placement = place(&lanes.lanes, frame_h);
    let chips = place_chips(&lanes.rules, &placement);
    let ends: HashSet<String> = lanes.lanes.iter().flat_map(|lane| lane.nodes.iter()).filter(|node| matches!(node.kind, LaneNodeKind::Start | LaneNodeKind::End)).map(|node| node.id.clone()).collect();
    let mut all = state.seen.borrow_mut();
    let seen = all.entry(id.clone()).or_default();
    lanes_follow(seen, lanes, &placement, &chips);
    if let Some(ask) = seen.ask.clone().filter(|_| frame_w > 0. && frame_h > 0.) {
        if let Some(next) = pan_to(&ask.rects, seen.view, (frame_w, frame_h), (placement.reach(host.lanes_reach_past_end()), placement.height), ask.follow, ask.earliest, ask.keep) {
            seen.view = next;
        }
        seen.ask = None;
    }
    let view = seen.view;
    let picked = seen.picked.clone();
    seen.hooks = hooks_of(lanes, &placement, &chips, picked.as_ref(), view);
    let first = lanes.lanes.first().and_then(|lane| lane.nodes.iter().find(|node| !ends.contains(&node.id))).map(|node| node.id.clone());
    let current = seen.current.clone().filter(|one| placement.items.contains_key(one) || chips.contains_key(one)).or(first);
    drop(all);
    let keyed = state.keyed.get();

    let z = view.zoom;
    let at = move |x: f32, y: f32| (view.x + x * z, view.y + y * z);

    // Bands, moves and rules, under the nodes: painted in the pane, cut at the heads' edge.
    let size = Rc::clone(&state.size);
    let origin = Rc::clone(&state.origin);
    let painted = PaintPlan::of(lanes, &placement, picked.as_ref(), look);
    let paint_layer = canvas(
        move |bounds, window, _| {
            origin.set(bounds.origin);
            let now = (f32::from(bounds.size.width), f32::from(bounds.size.height));
            if size.get() != now {
                size.set(now);
                window.refresh();
            }
            bounds
        },
        move |_, bounds: Bounds<Pixels>, window, _| {
            let mut clip = bounds;
            clip.origin.x += px(HEAD);
            clip.size.width = (clip.size.width - px(HEAD)).max(px(0.));
            window.with_content_mask(Some(ContentMask { bounds: clip }), |window| painted.paint(bounds.origin, view, window));
        },
    )
    .absolute()
    .size_full();

    let nodes: Vec<AnyElement> = lanes
        .lanes
        .iter()
        .flat_map(|lane| lane.nodes.iter().map(move |node| (lane, node)))
        .filter_map(|(lane, node)| {
            let rect = placement.items.get(&node.id)?;
            let (x, y) = at(rect.x, rect.y);
            let mine = keyed && current.as_deref() == Some(node.id.as_str());
            let picked = picked.as_ref() == Some(&Pick::Node(node.id.clone()));
            Some(lane_node(id, lane, node, (x - HEAD, y), *rect, z, mine, picked, look, cx))
        })
        .collect();
    let chip_views: Vec<AnyElement> = lanes
        .rules
        .iter()
        .filter_map(|edge| {
            let chip = chips.get(&edge.id)?;
            let (x, y) = at(chip.x, chip.y);
            let mine = keyed && current.as_deref() == Some(edge.id.as_str());
            let picked = picked.as_ref() == Some(&Pick::Rule(edge.rule.clone()));
            let target = lanes.lanes.iter().find(|lane| edge.to.split('/').next() == Some(lane.id.as_str())).map_or(edge.to.clone(), |lane| lane.title.clone());
            Some(lane_chip(id, edge, &target, (x - HEAD, y), chip.w * z, z, mine, picked, look, cx))
        })
        .collect();
    let heads: Vec<AnyElement> = lanes.lanes.iter().zip(&placement.bands).map(|(lane, band)| lane_head(host, id, lanes, lane, view.y + band.y * z, band.h * z, look, cx)).collect();

    let layer = div().absolute().top_0().bottom_0().right_0().left(px(HEAD)).overflow_hidden().children(nodes).children(chip_views);
    let heads = div().absolute().top_0().left_0().bottom_0().w(px(HEAD)).overflow_hidden().children(heads);

    let (pressed, moved, lifted, wheel) = (id.clone(), id.clone(), id.clone(), id.clone());
    let keys_for = id.clone();
    let pane = div()
        .id("lanes-pane")
        .track_focus(&state.frame)
        .relative()
        .flex_1()
        .min_w_0()
        .h_full()
        .overflow_hidden()
        .on_mouse_down(MouseButton::Left, cx.listener(move |this, event: &MouseDownEvent, window, cx| {
            window.focus(&this.lanes_state().frame, cx);
            if let Some(seen) = this.lanes_state().seen.borrow_mut().get_mut(&pressed) {
                seen.drag = Some((event.position, seen.view));
            }
        }))
        .on_mouse_move(cx.listener(move |this, event: &MouseMoveEvent, _, cx| {
            let mut all = this.lanes_state().seen.borrow_mut();
            let Some(seen) = all.get_mut(&moved) else { return };
            let Some((from, was)) = seen.drag else { return };
            if !event.dragging() {
                seen.drag = None;
                return;
            }
            seen.view = View { x: was.x + f32::from(event.position.x - from.x), y: was.y + f32::from(event.position.y - from.y), zoom: was.zoom };
            cx.notify();
        }))
        .on_mouse_up(MouseButton::Left, cx.listener(move |this, _: &MouseUpEvent, _, _| {
            if let Some(seen) = this.lanes_state().seen.borrow_mut().get_mut(&lifted) {
                seen.drag = None;
            }
        }))
        .on_scroll(cx.listener(move |this, event: &ScrollEvent, window, cx| {
            // `panOnScroll`: the wheel moves the canvas, the zoom stays.
            let delta = event.delta.pixel_delta(window.line_height());
            if let Some(seen) = this.lanes_state().seen.borrow_mut().get_mut(&wheel) {
                seen.view.x += f32::from(delta.x);
                seen.view.y += f32::from(delta.y);
            }
            cx.stop_propagation();
            cx.notify();
        }))
        .on_key_down(cx.listener(move |this, event: &KeyDownEvent, window, cx| {
            if this.lanes_state().frame.is_focused(window) && lanes_key(this, &keys_for, event, window, cx) {
                cx.stop_propagation();
            }
        }))
        .child(paint_layer)
        .child(layer)
        .child(heads);
    let rail = div()
        .flex_none()
        .flex()
        .flex_col()
        .gap(look.length(token::SPACE_3))
        .p(look.length(token::SPACE_3))
        .child(rail_group(look, "How you are looking at this", vec![lanes_rail(id, "out", None, "\u{2212}", "Zoom out", look, cx), lanes_rail(id, "in", None, "+", "Zoom in", look, cx), lanes_rail(id, "fit", Some("maximize"), "", "Fit", look, cx)]));
    // The frame is never taller than the body shows, so a pan to a node lands it where it can be seen.
    // Classic's keeps a floor of seven space-12s (`docs/concepts/native-bridge.md`, departures).
    labelled(
            div()
                .id(SharedString::from(format!("lanes-frame:{id}")))
                .flex_1()
                .min_h_0()
                .flex()
                .rounded(look.length(token::RADIUS_LG))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::BORDER_DEFAULT))
                .bg(look.color(token::BG_SUNKEN))
                .overflow_hidden()
                .child(rail)
                .child(pane),
            Role::Group,
            "Workflow",
        )
        .into_any_element()
}

/// One of the rail's view acts: Zoom out, Zoom in, Fit (`ViewGroup`).
#[allow(clippy::too_many_arguments)]
fn lanes_rail<H: LanesHost>(id: &SharedString, act: &'static str, icon: Option<&'static str>, sign: &'static str, said: &'static str, look: &Look, cx: &Context<H>) -> AnyElement {
    let owner = id.clone();
    let button = rail_frame(look, act, false, false).tooltip(tip(said));
    let button = match icon {
        Some(icon) => button.child(glyph(look, icon, Size::Small, look.color(token::FG_MUTED))),
        None => button.text_size(look.length(token::TEXT_SM)).text_color(look.color(token::FG_MUTED)).child(sign),
    };
    labelled(
        button.on_click(cx.listener(move |this, _, _, cx| {
            let (w, h) = this.lanes_state().size.get();
            if let Some(seen) = this.lanes_state().seen.borrow_mut().get_mut(&owner) {
                seen.view = match act {
                    "fit" => HOME,
                    _ => {
                        let to = (seen.view.zoom * if act == "in" { ZOOM_STEP } else { 1. / ZOOM_STEP }).clamp(MIN_ZOOM, MAX_ZOOM);
                        // About the middle of the frame, as React Flow's `zoomIn` and `zoomOut` are.
                        let (mx, my) = (w / 2., h / 2.);
                        let k = to / seen.view.zoom;
                        View { x: mx - (mx - seen.view.x) * k, y: my - (my - seen.view.y) * k, zoom: to }
                    }
                };
            }
            cx.notify();
        })),
        Role::Button,
        said,
    )
    .into_any_element()
}

/// The canvas's keys (`keys`): arrows go to the nearest node, `]` and `[` round the rules picking each,
/// Enter picks or lets go, and Esc lets go of what is picked.
fn lanes_key<H: LanesHost>(this: &mut H, id: &SharedString, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<H>) -> bool {
    let keys = &event.keystroke;
    if keys.modifiers.alt || keys.modifiers.control || keys.modifiers.platform {
        return false;
    }
    // Where the body scrolls, the canvas the keys act on comes into view first.
    this.lanes_reveal(window, cx);
    let Some(lanes) = this.lanes_of(id, cx).cloned() else { return false };
    let (_, h) = this.lanes_state().size.get();
    let placement = place(&lanes.lanes, h);
    let chips = place_chips(&lanes.rules, &placement);
    let ends: HashSet<String> = lanes.lanes.iter().flat_map(|lane| lane.nodes.iter()).filter(|node| matches!(node.kind, LaneNodeKind::Start | LaneNodeKind::End)).map(|node| node.id.clone()).collect();
    let first = lanes.lanes.first().and_then(|lane| lane.nodes.iter().find(|node| !ends.contains(&node.id))).map(|node| node.id.clone());
    let mut all = this.lanes_state().seen.borrow_mut();
    let seen = all.entry(id.clone()).or_default();
    let current = seen.current.clone().filter(|one| placement.items.contains_key(one) || chips.contains_key(one)).or(first);
    let mut picked = None;
    let done = match keys.key.as_str() {
        "escape" if seen.picked.is_some() => {
            seen.picked = None;
            true
        }
        "enter" | "space" => match current {
            Some(current) => {
                let pick = match lanes.rules.iter().find(|edge| edge.id == current) {
                    Some(edge) => Pick::Rule(edge.rule.clone()),
                    None => Pick::Node(current.clone()),
                };
                seen.picked = if seen.picked.as_ref() == Some(&pick) { None } else { Some(pick) };
                picked = seen.picked.clone();
                true
            }
            None => false,
        },
        "]" | "[" => {
            let mut rules: Vec<String> = Vec::new();
            for edge in lanes.rules.iter().filter(|edge| !edge.quiet) {
                if !rules.contains(&edge.rule) {
                    rules.push(edge.rule.clone());
                }
            }
            if rules.is_empty() {
                false
            } else {
                let on = current.as_deref().and_then(|current| lanes.rules.iter().find(|edge| edge.id == current)).map(|edge| edge.rule.clone()).or(match &seen.picked {
                    Some(Pick::Rule(rule)) => Some(rule.clone()),
                    _ => None,
                });
                let back = keys.key == "[";
                let next = match on.and_then(|on| rules.iter().position(|one| *one == on)) {
                    None if back => rules.len() - 1,
                    None => 0,
                    Some(at) => (at + if back { rules.len() - 1 } else { 1 }) % rules.len(),
                };
                let rule = rules[next].clone();
                if let Some(edge) = lanes.rules.iter().find(|edge| edge.rule == rule && !edge.quiet) {
                    seen.current = Some(edge.id.clone());
                    seen.ask = Some(Ask { rects: rects_of(&[edge.id.clone(), edge.from.clone(), edge.to.clone()], &placement, &chips), follow: false, earliest: false, keep: true });
                }
                seen.picked = Some(Pick::Rule(rule));
                picked = seen.picked.clone();
                true
            }
        }
        "left" | "right" | "up" | "down" => match current.filter(|one| placement.items.contains_key(one)) {
            Some(from) => {
                if let Some(to) = nearest(&placement.items, &from, &keys.key, &ends) {
                    seen.ask = placement.items.get(&to).map(|rect| Ask { rects: vec![*rect], follow: false, earliest: false, keep: true });
                    seen.current = Some(to);
                }
                true
            }
            None => false,
        },
        _ => false,
    };
    drop(all);
    if done {
        this.lanes_picked(id, picked, window, cx);
        cx.notify();
    }
    done
}

/// A press on a node or chip: the keys go to the canvas there, and what was pressed is picked or let go.
fn lanes_press<H: LanesHost>(this: &mut H, id: &SharedString, current: String, pick: Pick, window: &mut Window, cx: &mut Context<H>) {
    window.focus(&this.lanes_state().frame, cx);
    this.lanes_reveal(window, cx);
    let picked = this.lanes_state().seen.borrow_mut().get_mut(id).and_then(|seen| {
        seen.current = Some(current);
        seen.picked = if seen.picked.as_ref() == Some(&pick) { None } else { Some(pick) };
        seen.picked.clone()
    });
    this.lanes_picked(id, picked, window, cx);
    cx.notify();

}

#[cfg(test)]
mod tests {
    use ipc::{LaneAutonomy, LaneEdge, LaneOutcome, LanePending};

    use super::layout::{chip_size, pan_to};
    use super::{Rect, View, HEAD, PAD};
    use super::wears::{chip_mark, mark_of_rule};

    fn edge() -> LaneEdge {
        LaneEdge {
            id: "ship:a>b".into(),
            rule: "ship".into(),
            from: "a/x".into(),
            to: "b/y".into(),
            label: "Ship it".into(),
            waits: None,
            fired_at: None,
            autonomy: LaneAutonomy::Acts,
            status: LaneOutcome::Acted,
            did: Vec::new(),
            changed: None,
            quiet: false,
            differs: Vec::new(),
            clash: None,
            proposed: false,
            pending: None,
        }
    }

    #[test]
    fn a_rule_set_otherwise_widens_its_chip_and_its_marks_are_seen() {
        let plain = edge();
        let differs = LaneEdge { differs: vec!["Web".into()], ..edge() };
        assert_eq!(chip_size(&differs).0, chip_size(&plain).0 + 18.);
        // A quiet chip is its mark alone, whatever it differs in.
        assert_eq!(chip_size(&LaneEdge { quiet: true, ..differs.clone() }), chip_size(&LaneEdge { quiet: true, ..edge() }));

        let mark = mark_of_rule(&plain);
        for changed in [differs, LaneEdge { clash: Some("Web and Mobile differ".into()), ..edge() }, LaneEdge { proposed: true, ..edge() }, LaneEdge { pending: Some(LanePending::Keep), ..edge() }] {
            assert_ne!(mark_of_rule(&changed), mark, "{changed:?}");
        }

        // A wait's clock comes before a draft's mark, and both before how it ended.
        assert_eq!(chip_mark(&plain), Some(("check", "Acted")));
        assert_eq!(chip_mark(&LaneEdge { proposed: true, ..edge() }), Some(("list-plus", "Proposed")));
        assert_eq!(chip_mark(&LaneEdge { proposed: true, pending: Some(LanePending::Removal), ..edge() }), Some(("clock", "Removed from the repository when its pull request merges")));
    }

    #[test]
    fn a_pick_too_wide_with_its_ends_stays_in_view_clear_of_the_heads() {
        // A rule chip between its two nodes, in a frame too narrow for all three (Settings at 820).
        let (from, chip, to) = (Rect { x: 100., y: 40., w: 168., h: 68. }, Rect { x: 300., y: 60., w: 200., h: 28. }, Rect { x: 520., y: 40., w: 168., h: 68. });
        let view = View { x: 0., y: 0., zoom: 1. };
        let next = pan_to(&[chip, from, to], view, (405., 600.), (1000., 600.), false, false, true).unwrap_or(view);
        assert!(chip.x + next.x >= HEAD + PAD - 0.5, "{next:?}");
        assert!(chip.x + chip.w + next.x <= 405. - PAD + 0.5, "{next:?}");
        // Without the pick, the leftmost go first and the chip is left under the heads.
        let loose = pan_to(&[chip, from, to], view, (405., 600.), (1000., 600.), false, false, false).unwrap_or(view);
        assert!(chip.x + loose.x < HEAD + PAD, "{loose:?}");
    }

    #[test]
    fn the_pick_and_each_gap_between_steps_are_given_where_the_view_draws_them() {
        let lanes = ipc::decode::<ipc::JobLanesView>("JobLanesView", include_bytes!("../../fixtures/job-lanes.json")).unwrap();
        let placement = super::place(&lanes.lanes, 600.);
        let chips = super::place_chips(&lanes.rules, &placement);
        let steps: Vec<&ipc::LaneNode> = lanes.lanes.iter().filter(|lane| lane.kind == ipc::LaneKind::Workflow).flat_map(|lane| lane.nodes.iter()).filter(|node| !matches!(node.kind, ipc::LaneNodeKind::Start | ipc::LaneNodeKind::End)).collect();
        let view = View { x: -40., y: 10., zoom: 0.8 };
        let picked = super::Pick::Node(steps[1].id.clone());
        let hooks = super::hooks_of(&lanes, &placement, &chips, Some(&picked), view);

        let at = placement.items[&steps[1].id];
        assert_eq!(hooks.picked, Some(Rect { x: -40. + at.x * 0.8, y: 10. + at.y * 0.8, w: at.w * 0.8, h: at.h * 0.8 }));
        // One gap between each two steps, in order, wide enough for a + at the smallest zoom.
        assert_eq!(hooks.gaps.len(), steps.len() - 1);
        for ((after, before, gap), pair) in hooks.gaps.iter().zip(steps.windows(2)) {
            assert_eq!((after, before), (&pair[0].id, &pair[1].id));
            assert!(gap.w >= 22., "{gap:?}");
        }
        assert_eq!(super::hooks_of(&lanes, &placement, &chips, None, view).picked, None);
    }

    #[test]
    fn a_pan_reaches_the_empty_column_past_the_last_node() {
        let lanes = ipc::decode::<ipc::JobLanesView>("JobLanesView", include_bytes!("../../fixtures/job-lanes.json")).unwrap();
        let placement = super::place(&lanes.lanes, 600.);
        let last = lanes.lanes.iter().flat_map(|lane| lane.nodes.iter()).map(|node| node.col).max().unwrap_or(0) as f32;
        // Settings' add box, one column past the last node.
        let empty = Rect { x: HEAD + PAD + super::LEAD + (last + 1.) * super::COL_W, y: 40., w: super::NODE_W, h: 68. };
        let frame = (600., 600.);
        let view = View { x: 0., y: 0., zoom: 1. };
        let next = pan_to(&[empty], view, frame, (placement.reach(true), placement.height), false, false, false).unwrap_or(view);
        assert!(empty.x + empty.w + next.x <= frame.0 - PAD + 0.5, "{next:?}");
        assert!(empty.x + next.x >= HEAD + PAD - 0.5, "{next:?}");
        // A Job's lanes, which have no such box, stop at the last node as before.
        assert_eq!(placement.reach(false), placement.width);
        let short = pan_to(&[empty], view, frame, (placement.reach(false), placement.height), false, false, false).unwrap_or(view);
        assert!(empty.x + empty.w + short.x > frame.0 - PAD + 0.5, "{short:?}");
    }
}
