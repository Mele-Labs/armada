//! The Workflow tab: a Job's lanes as Fleet works them out (`SessionRecord::lanes`), drawn as Classic's
//! `LaneCanvas` draws them in the Job's Workflow tab (`shared-panel/lanes.tsx`, `LaneCanvas.tsx`).
//!
//! **Fleet works the lanes out; this draws them.** The geometry is Classic's `layout.ts`: one canvas unit
//! is one pixel at zoom 1, a column is shared by every lane, and the lanes share the frame's height. The
//! heads stand over the left of the frame and keep each lane named as the canvas pans.
//!
//! A Job with no lanes draws nothing in the tab.

use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use gpui::{
    canvas, div, point, prelude::*, px, AnyElement, Bounds, ContentMask, Context, FocusHandle, FontWeight, Hsla, KeyDownEvent, MouseButton, MouseDownEvent, MouseMoveEvent,
    MouseUpEvent, PathBuilder, Pixels, Role, Rgba, ScrollEvent, SharedString, Window,
};
use ipc::{JobLanesView, LaneAutonomy, LaneEdge, LaneKind, LaneNode, LaneNodeKind, LaneOutcome, LaneView};

use super::rows::{glyph, mono, Size};
use super::sketch_pad::{rail_frame, rail_group};
use super::JobView;
use crate::labels::labelled;
use crate::look::{token, Look};

// `GEO`, `layout.ts`.
const HEAD: f32 = 144.;
const PAD: f32 = 16.;
const COL_W: f32 = 196.;
const NODE_W: f32 = 168.;
const STEP_H: f32 = 68.;
const STATE_H: f32 = 56.;
const SUB_CHARS: f32 = 20.;
const SUB_LINE: f32 = 16.;
const LANE_MIN: f32 = 128.;
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
pub(super) enum Pick {
    Node(String),
    Rule(String),
}

/// A pan asked of the canvas (`Pan`), met once the frame has a size.
#[derive(Clone)]
struct Ask {
    rects: Vec<Rect>,
    follow: bool,
    earliest: bool,
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
}

impl Default for Seen {
    fn default() -> Self {
        Self { view: HOME, picked: None, current: None, drawn: None, ask: None, drag: None }
    }
}

/// The Workflow tab's state on the Job view.
pub(super) struct Lanes {
    /// The canvas frame, which holds the keys while a node or chip has them.
    pub frame: FocusHandle,
    /// The frame holds the keys, so the node or chip they are on wears the ring.
    pub keyed: Cell<bool>,
    seen: RefCell<HashMap<SharedString, Seen>>,
    /// The pane's size last frame, heads included.
    size: Rc<Cell<(f32, f32)>>,
}

impl Lanes {
    pub fn new(cx: &mut gpui::App) -> Self {
        Self { frame: cx.focus_handle(), keyed: Cell::new(false), seen: RefCell::default(), size: Rc::new(Cell::new((0., 0.))) }
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

/// The lanes fill `height`, and stand at the least the lane height tall each when it is short (`place`).
fn place(lanes: &[LaneView], height: f32) -> Placement {
    let last = lanes.iter().flat_map(|lane| lane.nodes.iter()).map(|node| node.col).max().unwrap_or(0) as f32;
    let origin = HEAD + PAD + LEAD;
    let tail = if lanes.iter().flat_map(|lane| lane.nodes.iter()).any(|node| node.kind == LaneNodeKind::End) { DOT_GAP + DOT } else { 0. };
    let width = origin + PAD + last * COL_W + NODE_W + tail;
    let lane_h = LANE_MIN.max((height / lanes.len().max(1) as f32).floor());
    let mut bands = Vec::new();
    let mut items = HashMap::new();
    for (at, lane) in lanes.iter().enumerate() {
        let y = at as f32 * lane_h;
        // A step that names what it waits on is as tall as its words need.
        let lines = if lane.kind == LaneKind::Workflow {
            lane.nodes.iter().map(|node| node.waiting.iter().map(|said| (said.chars().count() as f32 / SUB_CHARS).ceil()).sum::<f32>() - 2.).fold(0_f32, f32::max)
        } else {
            0.
        };
        let h = if lane.kind == LaneKind::Workflow { STEP_H } else { STATE_H } + lines * SUB_LINE;
        bands.push(Rect { x: 0., y, w: width, h: lane_h });
        for node in &lane.nodes {
            let x = origin + node.col as f32 * COL_W;
            let rect = match node.kind {
                LaneNodeKind::Start => Rect { x: x - DOT_GAP - DOT, y: y + (lane_h - DOT) / 2., w: DOT, h: DOT },
                LaneNodeKind::End => Rect { x: x + NODE_W + DOT_GAP, y: y + (lane_h - DOT) / 2., w: DOT, h: DOT },
                _ => Rect { x, y: y + (lane_h - h) / 2., w: NODE_W, h },
            };
            items.insert(node.id.clone(), rect);
        }
    }
    Placement { bands, items, lane_h, width, height: lanes.len() as f32 * lane_h }
}

/// A time of day, 24-hour, as the chips read it (`clock`).
fn clock(at: &ipc::Instant) -> String {
    chrono::DateTime::parse_from_rfc3339(at.as_str()).map(|at| at.with_timezone(&chrono::Local).format("%H:%M").to_string()).unwrap_or_default()
}

/// The quiet words beside a chip's name (`RuleView.meta`): what it waits for, else when it fired.
fn meta(edge: &LaneEdge) -> Option<String> {
    edge.waits.clone().or_else(|| edge.fired_at.as_ref().map(clock))
}

/// The chip's size as its words will draw it (`chipSize`).
fn chip_size(edge: &LaneEdge) -> (f32, f32) {
    if edge.quiet {
        return (CHIP_H, CHIP_H);
    }
    let words = edge.label.chars().count() as f32 * 6.6 + meta(edge).map_or(0., |meta| 12. + meta.chars().count() as f32 * 7.9);
    ((words + 44.).ceil(), CHIP_H)
}

/// The shape of React Flow's bezier between a bottom and a top handle, both from the one parameter.
fn rise(t: f32) -> f32 {
    1.5 * t * (1. - t) + t * t * t
}
fn slide(t: f32) -> f32 {
    3. * t * t - 2. * t * t * t
}

/// Where along its curve an edge crosses the height `y` (`crossing`).
fn crossing(from: (f32, f32), to: (f32, f32), y: f32) -> f32 {
    let along = (y - from.1) / (to.1 - from.1);
    let (mut low, mut high) = (0_f32, 1_f32);
    for _ in 0..24 {
        let mid = (low + high) / 2.;
        if rise(mid) < along {
            low = mid;
        } else {
            high = mid;
        }
    }
    from.0 + (to.0 - from.0) * slide((low + high) / 2.)
}

fn touches(a: &Rect, b: &Rect, margin: f32) -> bool {
    a.x < b.x + b.w + margin && b.x < a.x + a.w + margin && a.y < b.y + b.h + margin && b.y < a.y + a.h + margin
}

/// Where each rule's chip sits, by its edge's id, as its centre and size (`placeChips`). On the edge, in the
/// gutter between two lanes; the ones with words are placed first, so a chip with only its mark moves.
fn place_chips(edges: &[LaneEdge], placement: &Placement) -> HashMap<String, Rect> {
    let mut out = HashMap::new();
    let mut taken: Vec<Rect> = Vec::new();
    let ordered = edges.iter().filter(|edge| !edge.quiet).chain(edges.iter().filter(|edge| edge.quiet));
    for edge in ordered {
        let (Some(from), Some(to)) = (placement.items.get(&edge.from), placement.items.get(&edge.to)) else { continue };
        let down = to.y > from.y;
        let start = (from.x + from.w / 2., if down { from.y + from.h } else { from.y });
        let end = (to.x + to.w / 2., if down { to.y } else { to.y + to.h });
        let lanes = ((from.y / placement.lane_h).floor() as i32, (to.y / placement.lane_h).floor() as i32);
        let lines: Vec<f32> = if lanes.0 == lanes.1 {
            vec![lanes.0 as f32 * placement.lane_h]
        } else if down {
            (lanes.0 + 1..=lanes.1).map(|at| at as f32 * placement.lane_h).collect()
        } else {
            (lanes.1 + 1..=lanes.0).rev().map(|at| at as f32 * placement.lane_h).collect()
        };
        let (w, h) = chip_size(edge);
        let fit = |x: f32, y: f32| -> Rect {
            let left = (x - w / 2.).max(HEAD + PAD / 2.).min((HEAD + PAD / 2.).max(placement.width - w - PAD / 2.));
            Rect { x: left, y: y - h / 2., w, h }
        };
        let mut chosen = None;
        for &y in &lines {
            let sits = fit(if lanes.0 == lanes.1 { (start.0 + end.0) / 2. } else { crossing(start, end, y) }, y);
            if !taken.iter().any(|one| touches(one, &sits, 4.)) {
                chosen = Some(sits);
                break;
            }
        }
        let chosen = chosen.unwrap_or_else(|| {
            // Every gutter on its way is taken where it crosses: slide along the first until nothing is there.
            let first = lines[0];
            let mut sits = fit(crossing(start, end, first), first);
            for _ in 0..12 {
                let Some(hit) = taken.iter().find(|one| touches(one, &sits, 4.)) else { break };
                sits.x = hit.x + hit.w + 6.;
            }
            sits
        });
        taken.push(chosen);
        out.insert(edge.id.clone(), Rect { x: chosen.x + chosen.w / 2., y: chosen.y + chosen.h / 2., w: chosen.w, h: chosen.h });
    }
    out
}

/// The view that brings `rects` into the frame, or `None` where they already are (`panTo`). The heads stand
/// over the left of the frame, so what is brought in clears them.
fn pan_to(rects: &[Rect], view: View, frame: (f32, f32), content: (f32, f32), follow: bool, earliest: bool) -> Option<View> {
    if rects.is_empty() || frame.0 == 0. || frame.1 == 0. {
        return None;
    }
    let zoom = view.zoom;
    let (left, right, gap) = (HEAD + PAD, PAD, PAD / 2.);
    let usable = frame.0 - left - right;
    let mut sorted = rects.to_vec();
    sorted.sort_by(|a, b| a.x.total_cmp(&b.x));
    let bound = |set: &[Rect]| {
        let x = set.iter().map(|one| one.x).fold(f32::INFINITY, f32::min);
        let y = set.iter().map(|one| one.y).fold(f32::INFINITY, f32::min);
        Rect { x, y, w: set.iter().map(|one| one.x + one.w).fold(f32::NEG_INFINITY, f32::max) - x, h: set.iter().map(|one| one.y + one.h).fold(f32::NEG_INFINITY, f32::max) - y }
    };
    let mut kept: &[Rect] = &sorted;
    while kept.len() > 1 && bound(kept).w * zoom > usable {
        kept = if earliest { &kept[..kept.len() - 1] } else { &kept[1..] };
    }
    let at = bound(kept);
    let mut x = view.x;
    let from = at.x * zoom + view.x;
    let to = (at.x + at.w) * zoom + view.x;
    if from < left || to > frame.0 - right {
        x = if follow || from < left || at.w * zoom > usable { left - at.x * zoom } else { frame.0 - right - (at.x + at.w) * zoom };
    }
    let mut y = view.y;
    let top = at.y * zoom + view.y;
    let bottom = (at.y + at.h) * zoom + view.y;
    if top < gap || bottom > frame.1 - gap {
        y = if top < gap || at.h * zoom > frame.1 - gap * 2. { gap - at.y * zoom } else { frame.1 - gap - (at.y + at.h) * zoom };
    }
    // Never past the lanes: nothing is left of the first column or right of the last.
    x = 0_f32.min(if content.0 * zoom > frame.0 { x.max(frame.0 - content.0 * zoom) } else { 0. });
    y = 0_f32.min(if content.1 * zoom > frame.1 { y.max(frame.1 - content.1 * zoom) } else { 0. });
    ((x - view.x).abs() >= 1. || (y - view.y).abs() >= 1.).then_some(View { x, y, zoom })
}

/// The node an arrow reaches: the nearest whose centre lies that way, one off to the side costing twice
/// what one straight on does (`nearest`). The circles at the workflow's ends are skipped.
fn nearest(items: &HashMap<String, Rect>, from: &str, key: &str, skip: &HashSet<String>) -> Option<String> {
    let here = items.get(from)?;
    let (cx, cy) = (here.x + here.w / 2., here.y + here.h / 2.);
    let mut best: Option<(String, f32)> = None;
    for (id, one) in items {
        if id == from || skip.contains(id) {
            continue;
        }
        let (dx, dy) = (one.x + one.w / 2. - cx, one.y + one.h / 2. - cy);
        let (along, across) = match key {
            "right" => (dx, dy.abs()),
            "left" => (-dx, dy.abs()),
            "down" => (dy, dx.abs()),
            _ => (-dy, dx.abs()),
        };
        if along <= 0. || across > along * 3. {
            continue;
        }
        let cost = along + across * 2.;
        if best.as_ref().is_none_or(|(_, was)| cost < *was) {
            best = Some((id.clone(), cost));
        }
    }
    best.map(|(id, _)| id)
}

/// What a node or a chip is drawn as, so a change to it can be told (`markOfNode`, `markOfRule`).
fn mark_of_node(node: &LaneNode) -> String {
    format!("{}|{}|{}|{:?}|{}|{:?}|{:?}", node.lit, node.here, node.passed, node.waiting, node.skipped, node.ticket.as_ref().map(|one| (&one.label, &one.state_id, &one.state)), node.changed)
}
fn mark_of_rule(edge: &LaneEdge) -> String {
    format!("{:?}|{:?}|{:?}|{:?}|{:?}", edge.status, meta(edge), edge.autonomy, edge.did, edge.changed)
}

/// A node's mark and what its tooltip says of it (`markOf`).
fn mark_of(node: &LaneNode) -> Option<(&'static str, &'static str)> {
    if node.skipped {
        Some(("ban", "Skipped"))
    } else if node.ended {
        Some(("ban", "Ended"))
    } else if !node.waiting.is_empty() {
        Some(("clock", "Waiting"))
    } else if node.lit {
        Some(("circle-dot", "Working"))
    } else if node.here {
        Some(("map-pin", "Now"))
    } else if node.passed {
        Some(("check", "Went through"))
    } else {
        None
    }
}

/// The glyph of a ticket's state on a Job's card (`TICKET_STATE`); a state none names reads as unknown.
fn ticket_state(id: &str) -> &'static str {
    match id {
        "todo" => "circle-dashed",
        "in_progress" => "circle-dot",
        "in_review" => "eye",
        "done" | "closed" => "circle-check",
        _ => "circle-question-mark",
    }
}

/// The mark on a rule's chip (`chipMark`): how it ended on the Job.
fn chip_mark(edge: &LaneEdge) -> Option<(&'static str, &'static str)> {
    match edge.status {
        LaneOutcome::Acted => Some(("check", "Acted")),
        LaneOutcome::Asked => Some(("message-circle-question-mark", "Asked")),
        LaneOutcome::Waiting => Some(("clock", "Waiting")),
        LaneOutcome::Skipped => Some(("ban", "Skipped")),
    }
}

/// `color-mix(in srgb, a p, b)`.
fn mix(a: Hsla, b: Hsla, p: f32) -> Hsla {
    let (a, b) = (a.to_rgb(), b.to_rgb());
    Rgba { r: a.r * p + b.r * (1. - p), g: a.g * p + b.g * (1. - p), b: a.b * p + b.b * (1. - p), a: a.a * p + b.a * (1. - p) }.into()
}

/// The picked frame and ground (`--select-edge`, `--select-ground`, `takes.css`).
fn select_edge(look: &Look) -> Hsla {
    let fg = look.color(token::FG_DEFAULT);
    fg.opacity(fg.a * 0.7)
}
fn select_ground(look: &Look) -> Hsla {
    let tint = look.percent(token::SELECT_TINT);
    mix(look.color(token::ACCENT), look.color(token::BG_BASE), if tint > 0. { tint } else { 0.12 })
}

/// The tint of what a node is (`--kind`), which carries no status.
fn kind_tint(look: &Look, kind: LaneNodeKind) -> Hsla {
    look.color(match kind {
        LaneNodeKind::Watch => token::KIND_GATE,
        LaneNodeKind::Tracker => token::KIND_SETUP,
        LaneNodeKind::Forge => token::KIND_DELIVERY,
        _ => token::KIND_STEP,
    })
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

fn wears(look: &Look, node: &LaneNode, picked: bool) -> Wears {
    let kind = kind_tint(look, node.kind);
    let mut w = Wears {
        hue: look.color(token::FG_SUBTLE),
        frame: look.color(token::BORDER_STRONG),
        ground: mix(kind, look.color(token::BG_RAISED), 0.08),
        ink: look.color(token::FG_DEFAULT),
        heavy: false,
        inset: None,
        dashed: false,
    };
    if node.here {
        w.hue = look.color(token::FG_DEFAULT);
        w.frame = look.color(token::FG_MUTED);
        w.ground = look.color(token::BG_HOVER);
        w.inset = Some(look.color(token::FG_MUTED));
        w.heavy = true;
    }
    if node.passed {
        w.hue = look.color(token::STEP_ADVANCED);
        w.frame = mix(look.color(token::STEP_ADVANCED), look.color(token::BORDER_STRONG), 0.45);
        w.ink = look.color(token::FG_MUTED);
    }
    if node.lit {
        w.hue = look.color(token::STEP_RUNNING);
        w.frame = look.color(token::STEP_RUNNING);
        w.ground = mix(look.color(token::STEP_RUNNING), look.color(token::BG_RAISED), 0.1);
        w.heavy = true;
    }
    if !node.waiting.is_empty() {
        w.hue = look.color(token::STATUS_AWAITING_REVIEW);
        w.frame = look.color(token::STATUS_AWAITING_REVIEW);
        w.ground = mix(look.color(token::STATUS_AWAITING_REVIEW), look.color(token::BG_RAISED), 0.1);
    }
    if node.skipped || node.ended {
        w.hue = look.color(token::FG_SUBTLE);
        w.ink = look.color(token::FG_SUBTLE);
    }
    if node.changed.is_some() {
        w.dashed = true;
        w.frame = look.color(token::STATUS_AWAITING_REVIEW);
    }
    if picked {
        w.frame = select_edge(look);
        w.ground = select_ground(look);
        w.inset = Some(select_edge(look));
        w.heavy = true;
    }
    w
}

/// A rule's hue (`.armada-lane-edge[data-tone]`), and whether its line and diamond are dashed.
fn rule_hue(look: &Look, edge: &LaneEdge, picked: bool) -> (Hsla, bool) {
    let dashed = matches!(edge.status, LaneOutcome::Waiting | LaneOutcome::Skipped);
    let hue = if picked {
        look.color(token::FG_DEFAULT)
    } else if edge.autonomy == LaneAutonomy::Off {
        look.color(token::BORDER_STRONG)
    } else {
        match edge.status {
            LaneOutcome::Acted => look.color(token::STEP_ADVANCED),
            LaneOutcome::Asked => look.color(token::STATUS_AWAITING_REVIEW),
            _ => look.color(token::FG_MUTED),
        }
    };
    (hue, dashed)
}

/// Which sides an edge leaves and enters by (`sidesOf`): across within a lane, a way back under it, and
/// down or up the lanes by the bottom and the top.
#[derive(Clone, Copy)]
enum Side {
    Top,
    Right,
    Bottom,
    Left,
}

fn sides_of(from: &Rect, to: &Rect) -> (Side, Side) {
    if ((from.y + from.h / 2.) - (to.y + to.h / 2.)).abs() < 1. {
        return if to.x >= from.x { (Side::Right, Side::Left) } else { (Side::Bottom, Side::Bottom) };
    }
    if to.y > from.y {
        (Side::Bottom, Side::Top)
    } else {
        (Side::Top, Side::Bottom)
    }
}

fn handle(rect: &Rect, side: Side) -> (f32, f32) {
    match side {
        Side::Top => (rect.x + rect.w / 2., rect.y),
        Side::Right => (rect.x + rect.w, rect.y + rect.h / 2.),
        Side::Bottom => (rect.x + rect.w / 2., rect.y + rect.h),
        Side::Left => (rect.x, rect.y + rect.h / 2.),
    }
}

/// React Flow's bezier between two handles (`getBezierPath` at curvature 0.25): its four points.
fn bezier(from: &Rect, to: &Rect) -> [(f32, f32); 4] {
    let (out, into) = sides_of(from, to);
    let (s, t) = (handle(from, out), handle(to, into));
    let offset = |distance: f32| if distance >= 0. { 0.5 * distance } else { 0.25 * 25. * (-distance).sqrt() };
    let control = |side: Side, a: (f32, f32), b: (f32, f32)| match side {
        Side::Left => (a.0 - offset(a.0 - b.0), a.1),
        Side::Right => (a.0 + offset(b.0 - a.0), a.1),
        Side::Top => (a.0, a.1 - offset(a.1 - b.1)),
        Side::Bottom => (a.0, a.1 + offset(b.1 - a.1)),
    };
    [s, control(out, s, t), control(into, t, s), t]
}

/// A line along a bezier, dashed as `dash` asks (on, off), with a filled arrowhead `head` long at its end.
fn paint_curve(window: &mut Window, at: impl Fn((f32, f32)) -> gpui::Point<Pixels>, curve: [(f32, f32); 4], color: Hsla, width: Pixels, dash: Option<(f32, f32)>, head: f32) {
    let point_at = |t: f32| {
        let u = 1. - t;
        let [a, b, c, d] = curve;
        (u * u * u * a.0 + 3. * u * u * t * b.0 + 3. * u * t * t * c.0 + t * t * t * d.0, u * u * u * a.1 + 3. * u * u * t * b.1 + 3. * u * t * t * c.1 + t * t * t * d.1)
    };
    let points: Vec<(f32, f32)> = (0..=48).map(|step| point_at(step as f32 / 48.)).collect();
    let mut line = PathBuilder::stroke(width);
    match dash {
        None => {
            line.move_to(at(curve[0]));
            line.cubic_bezier_to(at(curve[3]), at(curve[1]), at(curve[2]));
        }
        Some((on, off)) => {
            let period = on + off;
            let mut walked = 0.;
            for pair in points.windows(2) {
                let (a, b) = (pair[0], pair[1]);
                let long = (b.0 - a.0).hypot(b.1 - a.1);
                if long <= 0. {
                    continue;
                }
                let along = |s: f32| (a.0 + (b.0 - a.0) * s / long, a.1 + (b.1 - a.1) * s / long);
                let mut s = 0.;
                while s < long {
                    let into = (walked + s) % period;
                    let (lit, left) = if into < on { (true, on - into) } else { (false, period - into) };
                    let end = (s + left.max(0.01)).min(long);
                    if lit {
                        line.move_to(at(along(s)));
                        line.line_to(at(along(end)));
                    }
                    s = end;
                }
                walked += long;
            }
        }
    }
    if let Ok(path) = line.build() {
        window.paint_path(path, color);
    }
    let tip = curve[3];
    let before = points[points.len() - 3];
    let (dx, dy) = (tip.0 - before.0, tip.1 - before.1);
    let long = dx.hypot(dy).max(0.001);
    let (ux, uy) = (dx / long, dy / long);
    let half = head / 2.;
    let mut arrow = PathBuilder::fill();
    arrow.add_polygon(&[at(tip), at((tip.0 - ux * head - uy * half, tip.1 - uy * head + ux * half)), at((tip.0 - ux * head + uy * half, tip.1 - uy * head - ux * half))], true);
    if let Ok(path) = arrow.build() {
        window.paint_path(path, color);
    }
}

/// A closed polygon, filled and outlined, its outline dashed where asked.
fn paint_polygon(window: &mut Window, points: &[gpui::Point<Pixels>], fill: Hsla, stroke: Hsla, width: Pixels, dash: Option<f32>) {
    let mut body = PathBuilder::fill();
    body.add_polygon(points, true);
    if let Ok(path) = body.build() {
        window.paint_path(path, fill);
    }
    let mut line = PathBuilder::stroke(width);
    let ring: Vec<gpui::Point<Pixels>> = points.iter().copied().chain(points.first().copied()).collect();
    match dash {
        None => {
            line.move_to(ring[0]);
            for one in &ring[1..] {
                line.line_to(*one);
            }
        }
        Some(on) => {
            for pair in ring.windows(2) {
                let (a, b) = (pair[0], pair[1]);
                let long = f32::from(b.x - a.x).hypot(f32::from(b.y - a.y));
                let mut s = 0.;
                while s < long {
                    let end = (s + on).min(long);
                    line.move_to(a + (b - a) * (s / long));
                    line.line_to(a + (b - a) * (end / long));
                    s += on * 2.;
                }
            }
        }
    }
    if let Ok(path) = line.build() {
        window.paint_path(path, stroke);
    }
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
            None => standing.contains(edge.from.as_str()) || standing.contains(edge.to.as_str()),
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
        seen.ask = Some(Ask { rects, follow, earliest: before.is_none() });
    }
}

/// The rects of nodes and chips by id, chips as their boxes.
fn rects_of(ids: &[String], placement: &Placement, chips: &HashMap<String, Rect>) -> Vec<Rect> {
    ids.iter().filter_map(|id| placement.items.get(id).copied().or_else(|| chips.get(id).map(|chip| Rect { x: chip.x - chip.w / 2., y: chip.y - chip.h / 2., w: chip.w, h: chip.h }))).collect()
}

/// The focus ring round the node the keys are on.
fn ring(look: &Look, rect: Rect, at: (f32, f32), z: f32, round: bool) -> gpui::Div {
    let gap = look.length(token::BORDER_WIDTH) * 2.;
    div()
        .absolute()
        .left(px(at.0) - gap)
        .top(px(at.1) - gap)
        .w(px(rect.w * z) + gap * 2.)
        .h(px(rect.h * z) + gap * 2.)
        .border(look.length(token::BORDER_WIDTH) * 2.)
        .border_color(look.color(token::ACCENT))
        .when(round, |ring| ring.rounded_full())
        .when(!round, |ring| ring.rounded(look.length(token::RADIUS_MD) * z + gap))
}

/// A glyph `size` px across at the canvas's zoom.
fn zoomed_glyph(icon: &'static str, z: f32, ink: Hsla, size: f32) -> gpui::Svg {
    gpui::svg().path(crate::assets::icon_path(icon)).flex_none().size(px(size * z)).text_color(ink)
}

/// A lane's head, over the left of the frame at the lane's own height: its title and where its states
/// are read from (`Heads`).
fn lane_head(lane: &LaneView, top: f32, height: f32, look: &Look) -> AnyElement {
    let workflow = lane.kind == LaneKind::Workflow;
    let source = if lane.ended { "Ended".to_string() } else { lane.what.clone().unwrap_or_else(|| lane.source.clone()) };
    let said = lane.note.as_ref().map(|note| format!("{}\n{note}", lane.title));
    let changed = lane.changed.as_ref().map(|changed| {
        div()
            .id(SharedString::from(format!("lane-head-changed:{}", lane.id)))
            .absolute()
            .top(look.length(token::SPACE_1))
            .right(look.length(token::SPACE_1))
            .tooltip(super::rows::tip(format!("Changed on this Job\n{changed}")))
            .child(glyph(look, "git-compare", Size::Small, look.color(token::STATUS_AWAITING_REVIEW)))
    });
    let head = div()
        .id(SharedString::from(format!("lane-head:{}", lane.id)))
        .absolute()
        .left_0()
        .top(px(top))
        .h(px(height))
        .w(px(HEAD))
        .flex()
        .flex_col()
        .justify_center()
        .gap(look.length(token::SPACE_1))
        .px(look.length(token::SPACE_3))
        .py(look.length(token::SPACE_2))
        .border_r(look.length(token::BORDER_WIDTH))
        .border_b(look.length(token::BORDER_WIDTH))
        .border_color(look.color(if workflow { token::BORDER_STRONG } else { token::BORDER_SUBTLE }))
        .bg(look.color(if workflow { token::BG_BASE } else { token::BG_RAISED }))
        .font_family(mono(look))
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .when_some(said, |head, said| head.tooltip(super::rows::tip(said)))
        .child(
            div()
                .min_w_0()
                .truncate()
                .text_size(look.length(token::TEXT_XS))
                .line_height(look.length(token::LEADING_XS))
                .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                .text_color(look.color(if lane.ended { token::FG_SUBTLE } else { token::FG_DEFAULT }))
                .child(SharedString::from(lane.title.clone())),
        )
        .child(div().min_w_0().truncate().text_size(look.length(token::TEXT_2XS)).line_height(look.length(token::LEADING_2XS)).text_color(look.color(token::FG_SUBTLE)).child(SharedString::from(source)))
        .children(changed);
    labelled(head, Role::Group, lane.title.clone()).into_any_element()
}

/// What the pane paints under the nodes: each lane's line, the ways a state is left, the rules' edges and
/// the hexagons a watch is drawn in.
struct PaintPlan {
    bands: Vec<(f32, f32, Hsla)>,
    curves: Vec<([(f32, f32); 4], Hsla, f32, Option<(f32, f32)>, f32)>,
    hexagons: Vec<(Rect, Hsla, Hsla, bool)>,
    hairline: f32,
    space_1: f32,
}

impl PaintPlan {
    fn of(lanes: &JobLanesView, placement: &Placement, picked: Option<&Pick>, look: &Look) -> Self {
        let hairline = f32::from(look.length(token::BORDER_WIDTH));
        let space_1 = f32::from(look.length(token::SPACE_1));
        let space_2 = f32::from(look.length(token::SPACE_2));
        let bands = lanes.lanes.iter().zip(&placement.bands).map(|(lane, band)| (band.y + band.h, band.w, look.color(if lane.kind == LaneKind::Workflow { token::BORDER_STRONG } else { token::BORDER_SUBTLE }))).collect();
        let kinds: HashMap<&str, LaneNodeKind> = lanes.lanes.iter().flat_map(|lane| lane.nodes.iter()).map(|node| (node.id.as_str(), node.kind)).collect();
        let mut curves = Vec::new();
        // A way a state is left within its lane: thin and solid, dashed where it leads to a wait (`MoveEdge`).
        for (from, to) in lanes.lanes.iter().flat_map(|lane| lane.moves.iter()) {
            let (Some(a), Some(b)) = (placement.items.get(from), placement.items.get(to)) else { continue };
            let wait = kinds.get(to.as_str()) == Some(&LaneNodeKind::Watch);
            curves.push((bezier(a, b), look.color(token::BORDER_STRONG), hairline, wait.then_some((space_1, space_1)), 6. * hairline));
        }
        for edge in &lanes.rules {
            let (Some(a), Some(b)) = (placement.items.get(&edge.from), placement.items.get(&edge.to)) else { continue };
            let is_picked = picked == Some(&Pick::Rule(edge.rule.clone()));
            let (hue, dashed) = rule_hue(look, edge, is_picked);
            let width = hairline * if is_picked { 2.5 } else { 1.5 };
            let dash = if edge.autonomy == LaneAutonomy::Off { Some((hairline, space_1)) } else { dashed.then_some((space_2, space_1)) };
            curves.push((bezier(a, b), hue, width, dash, 7. * hairline * 1.5));
        }
        let hexagons = lanes
            .lanes
            .iter()
            .flat_map(|lane| lane.nodes.iter())
            .filter(|node| node.kind == LaneNodeKind::Watch)
            .filter_map(|node| {
                let rect = *placement.items.get(&node.id)?;
                let w = wears(look, node, picked == Some(&Pick::Node(node.id.clone())));
                Some((rect, w.ground, w.frame, w.dashed))
            })
            .collect();
        Self { bands, curves, hexagons, hairline, space_1 }
    }

    fn paint(&self, origin: gpui::Point<Pixels>, view: View, window: &mut Window) {
        let at = |(x, y): (f32, f32)| origin + point(px(view.x + x * view.zoom), px(view.y + y * view.zoom));
        for &(bottom, wide, color) in &self.bands {
            let from = at((0., bottom)) - point(px(0.), px(self.hairline * view.zoom));
            window.paint_quad(gpui::fill(Bounds { origin: from, size: gpui::size(px(wide * view.zoom), px(self.hairline * view.zoom)) }, color));
        }
        for &(curve, color, width, dash, head) in &self.curves {
            paint_curve(window, at, curve, color, px(width * view.zoom), dash.map(|(on, off)| (on * view.zoom, off * view.zoom)), head * view.zoom);
        }
        for &(rect, ground, frame, dashed) in &self.hexagons {
            let p = |fx: f32, fy: f32| at((rect.x + rect.w * fx, rect.y + rect.h * fy));
            let points = [p(0.08, 0.), p(0.92, 0.), p(1., 0.5), p(0.92, 1.), p(0.08, 1.), p(0., 0.5)];
            paint_polygon(window, &points, ground, frame, px(self.hairline), dashed.then_some(self.space_1 * view.zoom));
        }
    }
}

impl JobView {
    /// The Workflow tab's body: the rail, then the lanes with their heads over the left. Nothing for a Job
    /// with no lanes, as Classic's `LanesBody` draws nothing.
    pub(super) fn pane_lanes(&self, id: &SharedString, look: &Look, cx: &Context<Self>) -> Option<AnyElement> {
        let job = self.record(id, cx)?;
        let lanes = job.lanes.as_ref()?;
        let (frame_w, frame_h) = self.lanes.size.get();
        let placement = place(&lanes.lanes, frame_h);
        let chips = place_chips(&lanes.rules, &placement);
        let ends: HashSet<String> = lanes.lanes.iter().flat_map(|lane| lane.nodes.iter()).filter(|node| matches!(node.kind, LaneNodeKind::Start | LaneNodeKind::End)).map(|node| node.id.clone()).collect();
        let mut all = self.lanes.seen.borrow_mut();
        let seen = all.entry(id.clone()).or_default();
        lanes_follow(seen, lanes, &placement, &chips);
        if let Some(ask) = seen.ask.clone().filter(|_| frame_w > 0. && frame_h > 0.) {
            if let Some(next) = pan_to(&ask.rects, seen.view, (frame_w, frame_h), (placement.width, placement.height), ask.follow, ask.earliest) {
                seen.view = next;
            }
            seen.ask = None;
        }
        let view = seen.view;
        let picked = seen.picked.clone();
        let first = lanes.lanes.first().and_then(|lane| lane.nodes.iter().find(|node| !ends.contains(&node.id))).map(|node| node.id.clone());
        let current = seen.current.clone().filter(|one| placement.items.contains_key(one) || chips.contains_key(one)).or(first);
        drop(all);
        let keyed = self.lanes.keyed.get();

        let z = view.zoom;
        let at = move |x: f32, y: f32| (view.x + x * z, view.y + y * z);

        // Bands, moves and rules, under the nodes: painted in the pane, cut at the heads' edge.
        let size = Rc::clone(&self.lanes.size);
        let painted = PaintPlan::of(lanes, &placement, picked.as_ref(), look);
        let paint_layer = canvas(
            move |bounds, window, _| {
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
                Some(self.lane_node(id, lane, node, (x - HEAD, y), *rect, z, mine, picked, look, cx))
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
                Some(self.lane_chip(id, edge, &target, (x - HEAD, y), chip.w * z, z, mine, picked, look, cx))
            })
            .collect();
        let heads: Vec<AnyElement> = lanes.lanes.iter().zip(&placement.bands).map(|(lane, band)| lane_head(lane, view.y + band.y * z, band.h * z, look)).collect();

        let layer = div().absolute().top_0().bottom_0().right_0().left(px(HEAD)).overflow_hidden().children(nodes).children(chip_views);
        let heads = div().absolute().top_0().left_0().bottom_0().w(px(HEAD)).overflow_hidden().children(heads);

        let (pressed, moved, lifted, wheel) = (id.clone(), id.clone(), id.clone(), id.clone());
        let keys_for = id.clone();
        let pane = div()
            .id("lanes-pane")
            .track_focus(&self.lanes.frame)
            .relative()
            .flex_1()
            .min_w_0()
            .h_full()
            .overflow_hidden()
            .on_mouse_down(MouseButton::Left, cx.listener(move |this, event: &MouseDownEvent, window, cx| {
                window.focus(&this.lanes.frame, cx);
                if let Some(seen) = this.lanes.seen.borrow_mut().get_mut(&pressed) {
                    seen.drag = Some((event.position, seen.view));
                }
            }))
            .on_mouse_move(cx.listener(move |this, event: &MouseMoveEvent, _, cx| {
                let mut all = this.lanes.seen.borrow_mut();
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
                if let Some(seen) = this.lanes.seen.borrow_mut().get_mut(&lifted) {
                    seen.drag = None;
                }
            }))
            .on_scroll(cx.listener(move |this, event: &ScrollEvent, window, cx| {
                // `panOnScroll`: the wheel moves the canvas, the zoom stays.
                let delta = event.delta.pixel_delta(window.line_height());
                if let Some(seen) = this.lanes.seen.borrow_mut().get_mut(&wheel) {
                    seen.view.x += f32::from(delta.x);
                    seen.view.y += f32::from(delta.y);
                }
                cx.stop_propagation();
                cx.notify();
            }))
            .on_key_down(cx.listener(move |this, event: &KeyDownEvent, window, cx| {
                if this.lanes.frame.is_focused(window) && this.lanes_key(&keys_for, event, cx) {
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
            .child(rail_group(look, "How you are looking at this", vec![self.lanes_rail(id, "out", None, "\u{2212}", "Zoom out", look, cx), self.lanes_rail(id, "in", None, "+", "Zoom in", look, cx), self.lanes_rail(id, "fit", Some("maximize"), "", "Fit", look, cx)]));
        // The frame fits the pane, so a pan to a node lands it where it can be seen. Classic's keeps a floor
        // of seven space-12s and scrolls the pane under it (`docs/concepts/native-bridge.md`, departures).
        Some(
            div()
                .flex_1()
                .min_h_0()
                .flex()
                .flex_col()
                .p(look.length(token::SPACE_3))
                .child(labelled(
                    div()
                        .id("lanes-frame")
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
                ))
                .into_any_element(),
        )
    }

    /// One of the rail's view acts: Zoom out, Zoom in, Fit (`ViewGroup`).
    #[allow(clippy::too_many_arguments)]
    fn lanes_rail(&self, id: &SharedString, act: &'static str, icon: Option<&'static str>, sign: &'static str, said: &'static str, look: &Look, cx: &Context<Self>) -> AnyElement {
        let owner = id.clone();
        let button = rail_frame(look, act, false, false).tooltip(super::rows::tip(said));
        let button = match icon {
            Some(icon) => button.child(glyph(look, icon, Size::Small, look.color(token::FG_MUTED))),
            None => button.text_size(look.length(token::TEXT_SM)).text_color(look.color(token::FG_MUTED)).child(sign),
        };
        labelled(
            button.on_click(cx.listener(move |this, _, _, cx| {
                let (w, h) = this.lanes.size.get();
                if let Some(seen) = this.lanes.seen.borrow_mut().get_mut(&owner) {
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
    fn lanes_key(&mut self, id: &SharedString, event: &KeyDownEvent, cx: &mut Context<Self>) -> bool {
        let keys = &event.keystroke;
        if keys.modifiers.alt || keys.modifiers.control || keys.modifiers.platform {
            return false;
        }
        let Some(lanes) = self.record(id, cx).and_then(|job| job.lanes.clone()) else { return false };
        let (_, h) = self.lanes.size.get();
        let placement = place(&lanes.lanes, h);
        let chips = place_chips(&lanes.rules, &placement);
        let ends: HashSet<String> = lanes.lanes.iter().flat_map(|lane| lane.nodes.iter()).filter(|node| matches!(node.kind, LaneNodeKind::Start | LaneNodeKind::End)).map(|node| node.id.clone()).collect();
        let first = lanes.lanes.first().and_then(|lane| lane.nodes.iter().find(|node| !ends.contains(&node.id))).map(|node| node.id.clone());
        let mut all = self.lanes.seen.borrow_mut();
        let seen = all.entry(id.clone()).or_default();
        let current = seen.current.clone().filter(|one| placement.items.contains_key(one) || chips.contains_key(one)).or(first);
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
                        seen.ask = Some(Ask { rects: rects_of(&[edge.id.clone(), edge.from.clone(), edge.to.clone()], &placement, &chips), follow: false, earliest: false });
                    }
                    seen.picked = Some(Pick::Rule(rule));
                    true
                }
            }
            "left" | "right" | "up" | "down" => match current.filter(|one| placement.items.contains_key(one)) {
                Some(from) => {
                    if let Some(to) = nearest(&placement.items, &from, &keys.key, &ends) {
                        seen.ask = placement.items.get(&to).map(|rect| Ask { rects: vec![*rect], follow: false, earliest: false });
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
            cx.notify();
        }
        done
    }

    /// A press on a node or chip: the keys go to the canvas there, and what was pressed is picked or let go.
    fn lanes_press(&mut self, id: &SharedString, current: String, pick: Pick, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.lanes.frame, cx);
        if let Some(seen) = self.lanes.seen.borrow_mut().get_mut(id) {
            seen.current = Some(current);
            seen.picked = if seen.picked.as_ref() == Some(&pick) { None } else { Some(pick) };
        }
        cx.notify();
    }

    /// ⌥W: the Workflow tab forward, with the keys on its canvas (`focusKind`).
    pub(super) fn lanes_forward(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.tab_closed("workflow") {
            self.add_tab("workflow", window, cx);
        } else {
            self.zone = super::side::Zone::Workflow;
            cx.notify();
        }
        let frame = self.lanes.frame.clone();
        window.defer(cx, move |window, cx| window.focus(&frame, cx));
    }

    /// One node: its shape, mark, name and what it waits for (`ItemNode`).
    #[allow(clippy::too_many_arguments)]
    fn lane_node(&self, id: &SharedString, lane: &LaneView, node: &LaneNode, at: (f32, f32), rect: Rect, z: f32, mine: bool, picked: bool, look: &Look, cx: &Context<Self>) -> AnyElement {
        let dot = matches!(node.kind, LaneNodeKind::Start | LaneNodeKind::End);
        let ring = mine.then(|| ring(look, rect, at, z, dot));
        let s = |token| look.length(token) * z;
        // Where the workflow starts and ends: a circle with nothing to pick (`dot`).
        if dot {
            let frame = if node.here {
                look.color(token::FG_DEFAULT)
            } else if node.passed {
                look.color(token::STEP_ADVANCED)
            } else {
                look.color(token::FG_SUBTLE)
            };
            let border = look.length(token::BORDER_WIDTH) * 2. * z;
            let circle = div()
                .id(SharedString::from(format!("lane-dot:{}", node.id)))
                .absolute()
                .left(px(at.0))
                .top(px(at.1))
                .size(px(rect.w * z))
                .rounded_full()
                .border(border)
                .border_color(frame)
                .bg(frame)
                .tooltip(super::rows::tip(node.label.clone()))
                .when(node.kind == LaneNodeKind::End, |circle| {
                    // A ring round a dot: the sunken ground inset inside the frame.
                    circle.flex().items_center().justify_center().child(div().size(px(rect.w * z) - border * 2.).rounded_full().border(s(token::SPACE_1)).border_color(look.color(token::BG_SUNKEN)).bg(frame))
                });
            return div().children(ring).child(labelled(circle, Role::Image, node.label.clone())).into_any_element();
        }
        let w = wears(look, node, picked);
        let card = node.kind == LaneNodeKind::Job;
        let pill = matches!(node.kind, LaneNodeKind::Tracker | LaneNodeKind::Forge);
        let hexagon = node.kind == LaneNodeKind::Watch;
        let mark = mark_of(node);
        let name = format!(
            "{}: {}{}{}",
            if lane.kind == LaneKind::Workflow { "Step" } else { lane.title.as_str() },
            node.label,
            node.ticket.as_ref().map_or(String::new(), |ticket| format!(", ticket {}{}", ticket.label, if ticket.state.is_empty() { String::new() } else { format!(", {}", ticket.state) })),
            if node.ended { ", ended" } else { "" }
        );
        let mut lines = vec![node.label.clone()];
        if node.waiting.is_empty() {
            lines.extend(mark.map(|(_, said)| said.to_string()));
        } else {
            lines.extend(node.waiting.iter().cloned());
        }
        if let Some(ticket) = &node.ticket {
            lines.push(match &ticket.title {
                Some(title) => format!("Ticket {}: {title}", ticket.label),
                None => format!("Ticket {}", ticket.label),
            });
            if !ticket.state.is_empty() {
                lines.push(format!("Ticket state: {}", ticket.state));
            }
            lines.push(format!("Held by {}", lane.title));
        }
        let kind = kind_tint(look, node.kind);
        let mark_view = div()
            .flex_none()
            .w(s(token::SPACE_6))
            .flex()
            .items_center()
            .justify_center()
            .children(if card { Some(zoomed_glyph("drone", z, kind, 16.)) } else { mark.map(|(icon, _)| zoomed_glyph(icon, z, w.hue, 16.)) });
        let sub = (!node.waiting.is_empty()).then(|| {
            div()
                .min_w_0()
                .line_clamp(2)
                .text_size(s(token::TEXT_XS))
                .line_height(s(token::LEADING_XS))
                .font_weight(FontWeight(look.number(token::WEIGHT_BODY)))
                .text_color(look.color(token::STATUS_AWAITING_REVIEW))
                .child(SharedString::from(node.waiting.join("\n")))
        });
        let ticket = node.ticket.as_ref().map(|ticket| {
            div()
                .flex()
                .items_center()
                .gap(s(token::SPACE_1))
                .min_w_0()
                .text_size(s(token::TEXT_XS))
                .line_height(s(token::LEADING_XS))
                .text_color(look.color(token::FG_MUTED))
                .child(zoomed_glyph("ticket", z, look.color(token::FG_MUTED), 14.))
                .child(div().min_w_0().truncate().child(SharedString::from(ticket.label.clone())))
                .child(zoomed_glyph(ticket_state(&ticket.state_id), z, look.color(token::FG_MUTED), 14.))
        });
        let words = div()
            .flex_1()
            .min_w_0()
            .flex()
            .flex_col()
            .child(div().min_w_0().when(node.skipped, |label| label.line_through()).child(SharedString::from(node.label.clone())))
            .children(sub)
            .children(ticket);
        let status = card.then(|| mark.map(|(icon, _)| div().flex_none().pl(s(token::SPACE_1)).child(zoomed_glyph(icon, z, w.hue, 14.)))).flatten();
        let inset = w.inset;
        let body = div()
            .id(SharedString::from(format!("lane-node:{}", node.id)))
            .absolute()
            .left(px(at.0))
            .top(px(at.1))
            .w(px(rect.w * z))
            .h(px(rect.h * z))
            .flex()
            .items_center()
            .pt(s(token::SPACE_1))
            .pb(s(token::SPACE_1))
            .pl(if pill { s(token::SPACE_2) } else if hexagon { s(token::SPACE_4) } else { s(token::SPACE_1) })
            .pr(if hexagon { s(token::SPACE_4) } else if pill { s(token::SPACE_3) } else { s(token::SPACE_2) })
            .text_size(s(token::TEXT_SM))
            .line_height(s(token::LEADING_SM))
            .text_color(w.ink)
            .when(w.heavy, |body| body.font_weight(FontWeight(look.number(token::WEIGHT_HEADING))))
            .when(!hexagon, |body| {
                body.bg(w.ground)
                    .border(look.length(token::BORDER_WIDTH) * z)
                    .border_color(w.frame)
                    .when(w.dashed, |body| body.border_dashed())
                    .rounded(if pill { s(token::SPACE_12) * 2. } else if card { s(token::RADIUS_SM) } else { s(token::RADIUS_MD) })
                    .when(card, |body| body.border_l(look.length(token::BORDER_WIDTH) * 3. * z))
            })
            .when_some(inset.filter(|_| !hexagon), |body, inset| {
                body.shadow(vec![gpui::BoxShadow { color: inset, offset: point(px(0.), px(0.)), blur_radius: px(0.), spread_radius: look.length(token::BORDER_WIDTH) * z, inset: true }])
            })
            .cursor_pointer()
            .tooltip(super::rows::tip(lines.join("\n")))
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_click({
                let (owner, node_id) = (id.clone(), node.id.clone());
                cx.listener(move |this, _, window, cx| this.lanes_press(&owner, node_id.clone(), Pick::Node(node_id.clone()), window, cx))
            })
            .child(mark_view)
            .child(words)
            .children(status);
        let body = labelled(body.aria_selected(picked), Role::Button, name);
        let changed = node.changed.as_ref().map(|changed| {
            // A state the Job's own copy changed: the comparison's glyph in a ring on its corner.
            div()
                .id(SharedString::from(format!("lane-changed:{}", node.id)))
                .absolute()
                .left(px(at.0) + px(rect.w * z) - s(token::SPACE_6) + s(token::SPACE_2))
                .top(px(at.1) - s(token::SPACE_2))
                .size(s(token::SPACE_6))
                .flex()
                .items_center()
                .justify_center()
                .rounded_full()
                .border(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::STATUS_AWAITING_REVIEW))
                .bg(look.color(token::BG_RAISED))
                .tooltip(super::rows::tip(format!("Changed on this Job\n{changed}")))
                .child(zoomed_glyph("git-compare", z, look.color(token::STATUS_AWAITING_REVIEW), 14.))
        });
        div().children(ring).child(body).children(changed).into_any_element()
    }

    /// One rule's chip on its edge: a diamond wearing its status, its name and when or what it waits for (`RuleEdge`).
    #[allow(clippy::too_many_arguments)]
    fn lane_chip(&self, id: &SharedString, edge: &LaneEdge, target: &str, centre: (f32, f32), wide: f32, z: f32, mine: bool, picked: bool, look: &Look, cx: &Context<Self>) -> AnyElement {
        let s = |token| look.length(token) * z;
        let (hue, dashed) = rule_hue(look, edge, false);
        let mark = chip_mark(edge);
        let meta = meta(edge);
        let mut lines = vec![edge.label.clone()];
        if edge.did.is_empty() {
            lines.extend(mark.map(|(_, said)| said.to_string()));
        } else {
            lines.extend(edge.did.iter().cloned());
        }
        lines.extend(meta.clone());
        if let Some(changed) = &edge.changed {
            lines.push("Changed on this Job".to_string());
            lines.push(changed.clone());
        }
        let name = if edge.quiet { format!("{}, {target}", edge.label) } else { edge.label.clone() };
        let quiet = edge.quiet;
        let fill = if quiet { look.color(token::BG_RAISED) } else { look.color(token::BG_OVERLAY) };
        let stroke = mix(hue, look.color(token::BORDER_DEFAULT), 0.7);
        let line_w = look.length(token::BORDER_WIDTH) * 1.5;
        let dash = dashed.then(|| f32::from(look.length(token::SPACE_1) * z));
        let diamond = div()
            .relative()
            .flex_none()
            .size(s(token::SPACE_6))
            .flex()
            .items_center()
            .justify_center()
            .child(
                canvas(
                    |_, _, _| {},
                    move |bounds, _, window, _| {
                        let (x, y, w, h) = (bounds.origin.x, bounds.origin.y, bounds.size.width, bounds.size.height);
                        let p = |fx: f32, fy: f32| point(x + w * fx, y + h * fy);
                        paint_polygon(window, &[p(0.5, 1. / 24.), p(23. / 24., 0.5), p(0.5, 23. / 24.), p(1. / 24., 0.5)], fill, stroke, line_w, dash);
                    },
                )
                .absolute()
                .size_full(),
            )
            .children(mark.map(|(icon, _)| zoomed_glyph(icon, z, hue, 12.)));
        let chip = div()
            .id(SharedString::from(format!("lane-chip:{}", edge.id)))
            .flex()
            .items_center()
            .gap(s(token::SPACE_1))
            .h(px(CHIP_H * z))
            .when(quiet, |chip| chip.w(px(CHIP_H * z)).justify_center())
            .when(!quiet, |chip| chip.pl(s(token::SPACE_1)).pr(s(token::SPACE_2)).bg(look.color(token::BG_OVERLAY)))
            .rounded(s(token::RADIUS_SM))
            .text_size(s(token::TEXT_XS))
            .line_height(s(token::LEADING_XS))
            .text_color(if edge.autonomy == LaneAutonomy::Off { look.color(token::FG_SUBTLE) } else { look.color(token::FG_DEFAULT) })
            .whitespace_nowrap()
            .cursor_pointer()
            .when(!quiet, |chip| chip.hover(|style| style.bg(look.color(token::BG_HOVER))))
            .when(picked, |chip| {
                chip.bg(select_ground(look))
                    .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                    .shadow(vec![gpui::BoxShadow { color: select_edge(look), offset: point(px(0.), px(0.)), blur_radius: px(0.), spread_radius: look.length(token::BORDER_WIDTH), inset: true }])
            })
            .when(mine, |chip| chip.border(look.length(token::BORDER_WIDTH) * 2.).border_color(look.color(token::ACCENT)))
            .tooltip(super::rows::tip(lines.join("\n")))
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_click({
                let (owner, edge_id, rule) = (id.clone(), edge.id.clone(), edge.rule.clone());
                cx.listener(move |this, _, window, cx| this.lanes_press(&owner, edge_id.clone(), Pick::Rule(rule.clone()), window, cx))
            })
            .child(diamond)
            .when(!quiet, |chip| chip.child(SharedString::from(edge.label.clone())).children(meta.map(|meta| div().font_family(mono(look)).text_color(look.color(token::FG_SUBTLE)).child(SharedString::from(meta)))))
            .children(edge.changed.as_ref().map(|_| zoomed_glyph("git-compare", z, look.color(token::STATUS_AWAITING_REVIEW), 14.)));
        // Centred on its place on the edge, whatever its words make its width.
        let room = wide.max(CHIP_H * z) * 2.;
        div()
            .absolute()
            .left(px(centre.0 - room / 2.))
            .top(px(centre.1) - px(CHIP_H * z) / 2.)
            .w(px(room))
            .flex()
            .justify_center()
            .child(labelled(chip.aria_selected(picked), Role::Button, name))
            .into_any_element()
    }
}
