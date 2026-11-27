//! A sketch's scene drawn as Electron's `SketchScene` draws it: boxes, groups, lanes, labels, code
//! and wireframe blocks where the Drone put them, settled apart, joined by routed arrows with their
//! words, the owner's ink over them, and the steps played in turn.
//!
//! Sizes are CSS pixels, as `SketchScene.css` gives them, worked out once per scene with the
//! app's own text measure; the drawing is then scaled to fit, never up, as `fitView` does. A mock
//! node draws the picture of its page (`mock_view`), as tall as its content once the picture has come. A screen node is a wireframe
//! sheet gpui lays out as the browser lays out `ScreenView`, under its title.

mod arrows;
pub(crate) mod export;
mod geometry;
mod glyph;
mod labels;
mod notes;
pub(crate) mod mock_page;
pub(crate) mod mock_view;
mod route;
mod screen;
mod settle;
mod start;
mod tile;

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::time::Instant;

use gpui::{
    canvas, point, px, quad, size, AnyElement, App, AvailableSpace, BorderStyle, Bounds, BoxShadow, ContentMask, Corners, Font, FontWeight, Hsla, IntoElement, PathBuilder,
    PathStyle, Pixels, Point, SharedString, StrokeOptions, Styled, TextAlign, TextRun, TransformationMatrix, Window,
};
use ipc::{Scene, SceneColor, SceneKind, SceneNode, ScreenElement, WirePart};
use lyon::tessellation::{LineCap, LineJoin};

use crate::look::{token, Look};
pub(crate) use notes::{bounds_of, fit_of, nodes, part_at, Mark, Marks, Part};
use arrows::Pattern;
use geometry::{Pt, Rect};
use route::Seg;

/// `--space-8`: what routes arrows round the boxes. The room kept between the boxes themselves is `tile::GAP_X` and `tile::GAP_Y`.
const GAP: f32 = 32.;
/// The node frame's border, `2px solid`.
const BORDER: f32 = 2.;
/// `--text-xs` / `--leading-xs`, the scene's words.
const TEXT: f32 = 13.;
const LEAD: f32 = 18.;
/// The head band's and body's padding: `8px 12px`.
const PAD_Y: f32 = 8.;
const PAD_X: f32 = 12.;
const RADIUS: f32 = 6.;
/// What a step lasts (`--duration-pulse`), and how long its dot travels (three quarters of it).
const STEP_MS: f32 = 1600.;
const DOT_MS: f32 = 1200.;
/// A node that is not in the step playing.
const DIMMED: f32 = 0.35;
/// A flow's dash and gap, `--space-1`; the crawl moves the pattern one period per `--duration-pulse`.
const DASH: f32 = 4.;
/// The chords a rounded corner is cut into for the dashes.
const CORNER_CUTS: usize = 6;
/// The chords a curve and a hop are cut into for the dashes.
const CURVE_CUTS: usize = 24;
const HOP_CUTS: usize = 12;
/// How far a hop over another arrow stands off the line, in scene units.
const HOP: f32 = 6.;
/// An arrowhead's length, and how far a way runs straight into one: two of them, so the head points along its line.
const HEAD: f32 = 6.;
const RUN: f32 = HEAD * 2.;
/// A dotted line's dots: a round dot one unit long, then four of nothing.
const DOT: f32 = 1.;
const DOT_PERIOD: f32 = 5.;

/// How the scene is drawn: shrunk on a thread card, or read in a sheet of its own.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Fit {
    Preview,
    Reading,
    /// Filling the reading column: no air or inset round the canvas, and drawn larger than its own size where the column is wider.
    Column,
}

impl Fit {
    fn padding(self) -> f32 {
        match self {
            Fit::Preview => 0.04,
            Fit::Reading => 0.1,
            Fit::Column => 0.,
        }
    }

    /// The most the fit draws a scene at: its own size, or in the column up to twice it. **A scene holding a mock stays at its
    /// own size**: a mock is a page laid out at its own width, and a chip drawn twice over is not the chip the Job meant.
    pub(crate) fn most_zoom(self, scene: &Scene) -> f32 {
        match self {
            Fit::Preview | Fit::Reading => 1.,
            Fit::Column if scene.nodes.iter().any(|node| node.kind == SceneKind::Mock) => 1.,
            Fit::Column => 2.,
        }
    }

    /// The least a scene is drawn at: so small that only a drawing past fifty screens is held at it, and a reading view never
    /// starts at a corner of one.
    const MIN_ZOOM: f32 = 0.02;

    /// Room kept clear on every side inside the box, in window pixels: `--space-4`, so nothing touches the frame.
    fn inset(self) -> f32 {
        match self {
            Fit::Preview | Fit::Column => 0.,
            Fit::Reading => 16.,
        }
    }
}

/// How far the reader zoomed past the fit, and moved the drawing from the middle, in window pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Lens {
    pub zoom: f32,
    pub pan: (f32, f32),
}

impl Default for Lens {
    fn default() -> Self {
        Lens { zoom: 1., pan: (0., 0.) }
    }
}

/// When a drawing moves: its steps since `steps`, and in a reading view, while no step plays,
/// its flows crawling since `flows`. Both `None` is the resting drawing.
#[derive(Clone, Copy, Default)]
pub(crate) struct Clock {
    pub(crate) steps: Option<Instant>,
    pub(crate) flows: Option<Instant>,
}

impl Clock {
    /// Steps and flows on the one clock, as a card does while the pointer is on it.
    pub(crate) fn both(since: Option<Instant>) -> Self {
        Self { steps: since, flows: since }
    }
}

/// The scene, drawn to fill its parent. `key` names the scene and its revision, so its layout is
/// worked out once.
pub(crate) fn view(key: SharedString, scene: Rc<Scene>, fit: Fit, clock: Clock, lens: Lens) -> impl IntoElement {
    view_changes(key, scene, None, fit, clock, lens)
}

thread_local! {
    /// Each scene read once, by its key: a thread redraws every frame a step plays.
    static READ: RefCell<HashMap<String, Option<Rc<Scene>>>> = RefCell::default();
}

/// The drawing a sketch carries, read whole or not at all, kept by `key`.
pub(crate) fn read(key: &str, raw: &ipc::RawScene) -> Option<Rc<Scene>> {
    if let Some(held) = READ.with(|held| held.borrow().get(key).cloned()) {
        return held;
    }
    let read = raw.scene().map(Rc::new);
    READ.with(|held| held.borrow_mut().insert(key.to_string(), read.clone()));
    read
}

/// A node's height as a place that sizes a drawing without laying it out should take it: a mock's is its title and the
/// content of its page once a picture has come, else the node's own `h`, else `floor`.
pub(crate) fn node_height(node: &SceneNode, look: &Look, floor: f64, cx: &App) -> f64 {
    match mock_view::content_height(node, mock_view::dark(look), cx) {
        Some(content) => f64::from(sheet_top(node) + content),
        None => node.h.unwrap_or(floor),
    }
}

/// Whether the scene has steps to play.
pub(crate) fn plays(scene: &Scene) -> bool {
    scene.nodes.iter().any(|node| node.step.is_some()) || scene.edges.iter().any(|edge| edge.step.is_some())
}

/// What a part of a scene read against an earlier one is to it (`SceneChange`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Change {
    Added,
    Removed,
    Changed,
    Same,
}

impl Change {
    /// `CHANGE_SAID`: the word a reader hears before the part's name.
    fn said(self) -> &'static str {
        match self {
            Change::Added => "Added",
            Change::Removed => "Removed",
            Change::Changed => "Changed",
            Change::Same => "Unchanged",
        }
    }
}

/// Each node's and arrow's change, by id.
#[derive(Debug, Default)]
pub(crate) struct Changes {
    nodes: HashMap<String, Change>,
    edges: HashMap<String, Change>,
}

/// `diffScenes`: `after` read against `before`, by node and arrow id. A part moved or reworded is
/// changed, and what `before` had that `after` lacks is kept where it was, after the rest.
pub(crate) fn diff(before: &Scene, after: &Scene) -> (Scene, Changes) {
    let mut changes = Changes::default();
    for node in &after.nodes {
        let change = match before.nodes.iter().find(|was| was.id == node.id) {
            None => Change::Added,
            Some(was) if was == node => Change::Same,
            Some(_) => Change::Changed,
        };
        changes.nodes.insert(node.id.clone(), change);
    }
    for edge in &after.edges {
        let change = match before.edges.iter().find(|was| was.id == edge.id) {
            None => Change::Added,
            Some(was) if was == edge => Change::Same,
            Some(_) => Change::Changed,
        };
        changes.edges.insert(edge.id.clone(), change);
    }
    let gone_nodes: Vec<_> = before.nodes.iter().filter(|was| !after.nodes.iter().any(|node| node.id == was.id)).cloned().collect();
    let gone_edges: Vec<_> = before.edges.iter().filter(|was| !after.edges.iter().any(|edge| edge.id == was.id)).cloned().collect();
    for node in &gone_nodes {
        changes.nodes.insert(node.id.clone(), Change::Removed);
    }
    for edge in &gone_edges {
        changes.edges.insert(edge.id.clone(), Change::Removed);
    }
    let scene = Scene { nodes: after.nodes.iter().cloned().chain(gone_nodes).collect(), edges: after.edges.iter().cloned().chain(gone_edges).collect(), strokes: after.strokes.clone() };
    (scene, changes)
}

thread_local! {
    /// Each scene read against another once, by its key.
    static DIFFED: RefCell<HashMap<String, (Rc<Scene>, Rc<Changes>)>> = RefCell::default();
}

/// `after` read against `before`, kept by `key`: the scene to draw, holding what was dropped, and
/// what changed in it.
pub(crate) fn against(key: &str, before: &Scene, after: &Scene) -> (Rc<Scene>, Rc<Changes>) {
    if let Some(held) = DIFFED.with(|held| held.borrow().get(key).cloned()) {
        return held;
    }
    let (scene, changes) = diff(before, after);
    let made = (Rc::new(scene), Rc::new(changes));
    DIFFED.with(|held| held.borrow_mut().insert(key.to_string(), made.clone()));
    made
}

/// The scene, drawn to fill its parent, with what changed against an earlier one marked: added
/// green, changed amber, removed red and struck through, the rest stood back.
pub(crate) fn view_changes(key: SharedString, scene: Rc<Scene>, changes: Option<Rc<Changes>>, fit: Fit, clock: Clock, lens: Lens) -> impl IntoElement {
    view_marked(key, scene, changes, fit, clock, lens, Rc::default())
}

/// As [`view_changes`], with the notes' numbers standing on the parts they are about and the parts picked ringed. A reading
/// view keeps where it put each part (`part_at`, `nodes`, `bounds_of`), which a press and the keys ask for.
pub(crate) fn view_marked(key: SharedString, scene: Rc<Scene>, changes: Option<Rc<Changes>>, fit: Fit, clock: Clock, lens: Lens, marks: Rc<Marks>) -> impl IntoElement {
    let held = scene.clone();
    canvas(
        move |bounds, window, cx| prepare(&key, &held, changes, fit, clock, lens, marks, bounds, window, cx),
        move |bounds, mut prepared, window, cx| paint(&scene, &mut prepared, fit, clock, bounds, window, cx),
    )
    .size_full()
}

thread_local! {
    /// Each scene's screen trees, by its key, one per node: `None` where the node is not a screen that reads.
    static TREES: RefCell<HashMap<SharedString, Rc<Vec<Option<Rc<ScreenElement>>>>>> = RefCell::default();
}

fn trees(key: &SharedString, scene: &Scene) -> Rc<Vec<Option<Rc<ScreenElement>>>> {
    if let Some(held) = TREES.with(|held| held.borrow().get(key).cloned()) {
        return held;
    }
    let read = Rc::new(scene.nodes.iter().map(|node| node.screen_tree().map(Rc::new)).collect::<Vec<_>>());
    TREES.with(|held| held.borrow_mut().insert(key.clone(), read.clone()));
    read
}

/// A screen's sheet width: its `w`, or a phone's or a desktop's (`screenWidth`).
fn sheet_width(node: &SceneNode, tree: &ScreenElement) -> f32 {
    node.w.map_or(tree.width() as f32, |w| w as f32)
}

/// How far a screen's sheet sits under the node's top: its title and `--space-2`.
fn sheet_top(node: &SceneNode) -> f32 {
    if node.title.is_some() {
        LEAD + 8.
    } else {
        0.
    }
}

/// What a part is called when somebody asks about it (`partLabel`).
fn part_label(node: &SceneNode) -> &str {
    node.title.as_deref().or_else(|| node.body.as_deref().and_then(|body| body.lines().next())).unwrap_or(&node.id)
}

// ---- layout --------------------------------------------------------------------------------------

/// Where everything stands, in scene units.
struct Laid {
    /// Each node's settled rect, by its index in the scene.
    rects: Vec<Rect>,
    /// Each arrow's way, by its index in the scene, with its words placed. An arrow the router found no way for is `None`.
    ways: Vec<Option<Way>>,
    /// The canvas the drawing stands on: everything it holds, with air round it.
    bounds: Rect,
    /// The same canvas with `tile::EDGE` of air in place of the canvas's own: what a drawing that fills its tile stands on.
    held: Rect,
    /// The "Start" pill of each node the sketch starts at, by node index.
    marks: Vec<Option<Rect>>,
    /// The step's badge and its number, by node index, where the node has a step.
    badges: Vec<Option<(Rect, String)>>,
    /// The outline at each node's left that says what it is, by node index.
    glyphs: Vec<Option<glyph::Mark>>,
    /// The step numbers, sorted and unique: the same number plays together.
    steps: Vec<f64>,
    /// The content height each mock was laid out with, by node index: the layout is worked out again when one is known that was not.
    mocks: Vec<Option<f32>>,
}

struct Way {
    path: Vec<Seg>,
    words: Option<(Rect, Vec<String>)>,
}

/// The app's own text measure, at scene size.
struct Measure<'a> {
    window: &'a Window,
    sans: SharedString,
    mono: SharedString,
}

impl Measure<'_> {
    fn font(&self, weight: f32, mono: bool) -> Font {
        Font { weight: FontWeight(weight), ..gpui::font(if mono { self.mono.clone() } else { self.sans.clone() }) }
    }

    /// The widest of the text's lines, unwrapped.
    fn width(&self, text: &str, at: f32, weight: f32, mono: bool) -> f32 {
        let font = self.font(weight, mono);
        text.split('\n')
            .map(|line| {
                let run = TextRun { len: line.len(), font: font.clone(), ..Default::default() };
                f32::from(self.window.text_system().shape_line(SharedString::from(line.to_string()), px(at), &[run], None).width)
            })
            .fold(0., f32::max)
    }


    /// How many lines the text takes, wrapped at `wrap`.
    fn lines(&self, text: &str, at: f32, weight: f32, mono: bool, wrap: f32) -> usize {
        let run = TextRun { len: text.len(), font: self.font(weight, mono), ..Default::default() };
        self.window
            .text_system()
            .shape_text(SharedString::from(text.to_string()), px(at), &[run], Some(px(wrap)), None)
            .map(|lines| lines.iter().map(|line| line.wrap_boundaries().len() + 1).sum())
            .unwrap_or(1)
    }

    /// The lines the text breaks into at `wrap`, as the reader shows it.
    fn broken(&self, text: &str, at: f32, weight: f32, mono: bool, wrap: Option<f32>) -> Vec<String> {
        let run = TextRun { len: text.len(), font: self.font(weight, mono), ..Default::default() };
        let Ok(shaped) = self.window.text_system().shape_text(SharedString::from(text.to_string()), px(at), &[run], wrap.map(px), None) else {
            return text.split('\n').map(str::to_string).collect();
        };
        let mut lines = Vec::new();
        for line in &shaped {
            let whole: &str = line.text.as_ref();
            let mut start = 0;
            for boundary in line.wrap_boundaries() {
                let Some(glyph) = line.runs().get(boundary.run_ix).and_then(|run| run.glyphs.get(boundary.glyph_ix)) else { continue };
                if let Some(piece) = whole.get(start..glyph.index) {
                    lines.push(piece.trim_end().to_string());
                    start = glyph.index;
                }
            }
            lines.push(whole.get(start..).unwrap_or("").to_string());
        }
        if lines.is_empty() {
            lines.push(String::new());
        }
        lines
    }

    /// `broken`, cut to `most` lines: where text is left over, the last line ends in an ellipsis that fits `wrap`.
    fn clamped(&self, text: &str, at: f32, weight: f32, mono: bool, wrap: f32, most: usize) -> Vec<String> {
        let mut lines = self.broken(text, at, weight, mono, Some(wrap));
        if lines.len() <= most {
            return lines;
        }
        lines.truncate(most);
        if let Some(last) = lines.last_mut() {
            let mut kept: String = last.trim_end().to_string();
            while !kept.is_empty() && self.width(&format!("{kept}\u{2026}"), at, weight, mono) > wrap {
                kept.pop();
            }
            *last = format!("{}\u{2026}", kept.trim_end());
        }
        lines
    }
}

fn ground(node: &SceneNode) -> bool {
    matches!(node.kind, SceneKind::Group | SceneKind::Lane)
}

/// What a tile's size depends on beyond the node: the outline at its left.
#[derive(Clone, Copy)]
struct Facts {
    mark: Option<glyph::Mark>,
}

/// The room at a tile's left for what comes before its words: the outline of what it is, and a registry glyph the node names.
fn lead_width(node: &SceneNode, facts: Facts) -> f32 {
    let one = tile::CHIP + tile::GLYPH_GAP;
    (if facts.mark.is_some() { one } else { 0. }) + if node.icon.is_some() { one } else { 0. }
}

/// The room a block of code's language takes at the top right, and what it keeps from the title.
fn lang_width(node: &SceneNode, measure: &Measure) -> f32 {
    node.lang.as_deref().map_or(0., |lang| tile::GLYPH_GAP + measure.width(lang, tile::LANG_AT, tile::KIND_WEIGHT, true))
}

/// What a tile's body is set in: a card's as a muted line of sans, a block of code's as mono.
fn body_face(node: &SceneNode) -> (f32, f32, bool) {
    if node.kind == SceneKind::Code {
        (tile::CODE_AT, tile::CODE_LEAD, true)
    } else {
        (tile::BODY_AT, tile::BODY_LEAD, false)
    }
}

/// The lines a tile's body breaks into at `wrap`: a card's cut to two, a block of code's whole.
fn body_lines(node: &SceneNode, measure: &Measure, wrap: f32) -> Vec<String> {
    let Some(body) = node.body.as_deref() else { return Vec::new() };
    let (at, _, mono) = body_face(node);
    measure.clamped(body, at, 400., mono, wrap, if node.kind == SceneKind::Code { usize::MAX } else { tile::BODY_LINES })
}

/// A tile's size: wide enough for its outline and its words between the box's floor and ceiling, as tall as the words.
fn tile_size(node: &SceneNode, facts: Facts, measure: &Measure, (floor, ceiling): (f32, f32)) -> (f32, f32) {
    let title = node.title.as_deref().unwrap_or("");
    let title_w = measure.width(title, tile::TITLE_AT, tile::TITLE_WEIGHT, false);
    let (body_at, body_lead, body_mono) = body_face(node);
    let body_w = node.body.as_deref().map_or(0., |body| measure.width(body, body_at, 400., body_mono));
    let (lead_w, lang_w) = (lead_width(node, facts), lang_width(node, measure));
    let content = (2. * tile::PAD_X + lead_w + lang_w + title_w.max(body_w)).clamp(floor, ceiling);
    let w = node.w.map_or(content, |at| content.max(at as f32));
    let wrap = (w - 2. * tile::PAD_X - lead_w - lang_w).max(1.);
    let title_lines = if title.is_empty() { 0 } else { measure.lines(title, tile::TITLE_AT, tile::TITLE_WEIGHT, false, wrap) };
    let lines = body_lines(node, measure, wrap).len();
    let words = title_lines as f32 * tile::TITLE_LEAD + if title_lines > 0 && lines > 0 { tile::BODY_GAP } else { 0. } + lines as f32 * body_lead;
    let h = 2. * tile::PAD_Y + words.max(tile::TITLE_LEAD);
    (w, node.h.map_or(h, |at| h.max(at as f32)))
}

/// The node's size as the browser lays it out: `max-content` between the box's floor and ceiling,
/// `w` and `h` as floors, and the height its words take.
fn size_of(node: &SceneNode, facts: Facts, measure: &Measure, (floor, ceiling): (f32, f32)) -> (f32, f32) {
    let fixed = |w: f32, h: f32| (node.w.map_or(w, |at| at as f32), node.h.map_or(h, |at| at as f32));
    let floored = |w: f32, h: f32| (node.w.map_or(w, |at| w.max(at as f32)), node.h.map_or(h, |at| h.max(at as f32)));
    let clamp = |content: f32| content.clamp(floor, ceiling);
    let title = node.title.as_deref().unwrap_or("");
    let heading = measure.width(title, TEXT, 600., false);
    match node.kind {
        SceneKind::Group | SceneKind::Lane => fixed(0., 0.),
        // At its floor until its picture has come: `laid` gives the node its content's height then.
        SceneKind::Mock => fixed(floor, 160.),
        SceneKind::Screen => fixed(floor, 90.),
        SceneKind::Label => {
            let words = node.title.as_deref().or(node.body.as_deref()).unwrap_or("");
            (measure.width(words, 14., 600., false), LEAD)
        }
        SceneKind::Wire => {
            let part = node.wire.unwrap_or(WirePart::Card);
            let (w, _) = floored(clamp(heading + 2. * PAD_X + 2. * BORDER), 0.);
            let titled = if node.title.is_some() { LEAD + 8. } else { 0. };
            let h = match part {
                WirePart::Card | WirePart::List => 2. * BORDER + 2. * PAD_X + titled + 3. * 8. + 2. * 8.,
                WirePart::Text => 2. * BORDER + 2. * PAD_X + titled + 2. * 8. + 8.,
                WirePart::Button | WirePart::Nav => 2. * BORDER + 2. * PAD_X + LEAD,
                WirePart::Input => 56.,
            };
            floored(w, h)
        }
        SceneKind::Box | SceneKind::Code => tile_size(node, facts, measure, (floor, ceiling)),
    }
}

/// How far a hand line's ink reaches past its points.
const INK_REACH: f32 = 4.;

thread_local! {
    /// The canvas each drawing stood on when it was last drawn, by its key: its width and height in scene units.
    static CANVAS: RefCell<HashMap<SharedString, (f32, f32)>> = RefCell::default();
}

/// Note the canvas a drawing stands on. A box that sized itself from an earlier answer, or from none, draws once more where it changed.
fn keep_canvas(key: &SharedString, bounds: Rect, window: &mut Window) {
    let now = (bounds.w, bounds.h);
    if CANVAS.with(|held| held.borrow_mut().insert(key.clone(), now)) != Some(now) {
        window.request_animation_frame();
    }
}

/// The canvas a drawing stands on, in scene units, once it has been drawn: its shape is what a box sizes itself to.
pub(crate) fn canvas_size(key: &str) -> Option<(f32, f32)> {
    CANVAS.with(|held| held.borrow().get(key).copied())
}

/// The canvas the drawing stands on: every node (at any side of zero), its mark and badge, every arrow's way and its words, and
/// every hand line, with air round it. What the fit shows, so nothing of the drawing is left outside it.
fn canvas_of(rects: &[Rect], marks: &[Option<Rect>], badges: &[Option<(Rect, String)>], ways: &[Option<Way>], scene: &Scene) -> Rect {
    let reach = |way: &Way| {
        let points = route::polyline(&way.path);
        let (xs, ys): (Vec<f32>, Vec<f32>) = points.iter().map(|p| (p.x, p.y)).unzip();
        let least = |all: &[f32]| all.iter().copied().fold(f32::INFINITY, f32::min);
        let most = |all: &[f32]| all.iter().copied().fold(f32::NEG_INFINITY, f32::max);
        (!points.is_empty()).then(|| Rect { x: least(&xs), y: least(&ys), w: most(&xs) - least(&xs), h: most(&ys) - least(&ys) })
    };
    let ink = scene.strokes.iter().flat_map(|stroke| stroke.points.iter()).map(|p| Rect { x: p.x as f32 - INK_REACH, y: p.y as f32 - INK_REACH, w: 2. * INK_REACH, h: 2. * INK_REACH });
    let held = rects
        .iter()
        .copied()
        .chain(marks.iter().flatten().copied())
        .chain(badges.iter().flatten().map(|(rect, _)| *rect))
        .chain(ways.iter().flatten().filter_map(reach))
        .chain(ways.iter().flatten().filter_map(|way| way.words.as_ref().map(|(rect, _)| *rect)))
        .chain(ink)
        .reduce(union)
        .unwrap_or_default();
    Rect { x: held.x - tile::CANVAS_PAD, y: held.y - tile::CANVAS_PAD, w: held.w + 2. * tile::CANVAS_PAD, h: held.h + 2. * tile::CANVAS_PAD }
}

fn lay(scene: &Scene, trees: &[Option<Rc<ScreenElement>>], sheets: &[Option<(f32, f32)>], mocks: Vec<Option<f32>>, look: &Look, window: &Window) -> Laid {
    let measure = Measure {
        window,
        sans: look.font_stack(token::FONT_SANS).into_iter().next().unwrap_or_else(|| "IBM Plex Sans".into()),
        mono: look.font_stack(token::FONT_MONO).into_iter().next().unwrap_or_else(|| "IBM Plex Mono".into()),
    };
    let limits = (f32::from(look.length(token::W_SKETCH_BOX)), f32::from(look.length(token::W_SKETCH_BOX_MAX)));
    let starts = start::nodes(scene);
    let facts: Vec<Facts> = scene.nodes.iter().map(|node| Facts { mark: glyph::mark_of(node, starts.contains(node.id.as_str())) }).collect();
    let sizes: Vec<(f32, f32)> =
        scene.nodes.iter().zip(sheets).zip(&facts).map(|((node, sheet), facts)| sheet.unwrap_or_else(|| size_of(node, *facts, &measure, limits))).collect();
    let order = arrange(scene, &sizes);
    let mut rects: Vec<Rect> = scene.nodes.iter().map(|node| order.iter().find(|(id, _)| *id == node.id).map_or_else(Rect::default, |(_, rect)| *rect)).collect();
    let ways = ways(scene, &order, &rects, HOP);
    let pill_w = measure.width(start::TAB_WORD, tile::PILL_AT, tile::NAME_WEIGHT, false);
    let marks: Vec<Option<Rect>> = scene.nodes.iter().zip(&rects).map(|(node, rect)| starts.contains(node.id.as_str()).then(|| start::pill(*rect, pill_w))).collect();
    let badges: Vec<Option<(Rect, String)>> = scene.nodes.iter().zip(&rects).map(|(node, rect)| tile::badge(node, *rect)).collect();
    let bounds = canvas_of(&rects, &marks, &badges, &ways, scene);
    // Lanes one under another are rows of the canvas, edge to edge: the first stands at its top and the last runs to its foot.
    // Lanes the Drone set side by side are its columns, top to foot, the first from its left edge and the last to its right.
    let lanes: Vec<usize> = scene.nodes.iter().enumerate().filter(|(_, node)| node.kind == SceneKind::Lane).map(|(at, _)| at).collect();
    let columns = lanes.iter().any(|&a| lanes.iter().any(|&b| a != b && rects[b].x >= rects[a].x + rects[a].w));
    stretch_lanes(&mut rects, &lanes, bounds, columns);
    // A step is a number a node, an arrow or an element of a screen carries, and what shares a number plays together.
    let mut steps: Vec<f64> = scene
        .nodes
        .iter()
        .filter_map(|node| node.step)
        .chain(scene.edges.iter().filter_map(|edge| edge.step))
        .chain(trees.iter().flatten().flat_map(|tree| tree.steps()))
        .collect();
    steps.sort_by(f64::total_cmp);
    steps.dedup();
    let glyphs = facts.iter().map(|facts| facts.mark).collect();
    let held = Rect { x: bounds.x + tile::CANVAS_PAD - tile::EDGE, y: bounds.y + tile::CANVAS_PAD - tile::EDGE, w: bounds.w - 2. * (tile::CANVAS_PAD - tile::EDGE), h: bounds.h - 2. * (tile::CANVAS_PAD - tile::EDGE) };
    Laid { rects, ways, bounds, held, marks, badges, glyphs, steps, mocks }
}

/// Lanes stretched to the canvas: rows run its width, the first from its top and the last to its foot; columns run its height,
/// the first from its left and the last to its right.
fn stretch_lanes(rects: &mut [Rect], lanes: &[usize], bounds: Rect, columns: bool) {
    let span = |rect: &Rect| if columns { (rect.x, rect.x + rect.w) } else { (rect.y, rect.y + rect.h) };
    let first = lanes.iter().copied().min_by(|a, b| span(&rects[*a]).0.total_cmp(&span(&rects[*b]).0));
    let last = lanes.iter().copied().max_by(|a, b| span(&rects[*a]).1.total_cmp(&span(&rects[*b]).1));
    for &at in lanes {
        if columns {
            rects[at].y = bounds.y;
            rects[at].h = bounds.h;
        } else {
            rects[at].x = bounds.x;
            rects[at].w = bounds.w;
        }
    }
    if let Some(at) = first {
        if columns {
            rects[at].w += rects[at].x - bounds.x;
            rects[at].x = bounds.x;
        } else {
            rects[at].h += rects[at].y - bounds.y;
            rects[at].y = bounds.y;
        }
    }
    if let Some(at) = last {
        if columns {
            rects[at].w = bounds.x + bounds.w - rects[at].x;
        } else {
            rects[at].h = bounds.y + bounds.h - rects[at].y;
        }
    }
}

/// The boxes settled apart, as `settle` does once every box has its size: solid boxes first, then
/// the grounds, in the order the router breaks its ties by.
fn arrange(scene: &Scene, sizes: &[(f32, f32)]) -> Vec<(String, Rect)> {
    let boxes: Vec<settle::SettleBox> = scene
        .nodes
        .iter()
        .zip(sizes)
        .map(|(node, (w, h))| settle::SettleBox { id: node.id.clone(), rect: Rect { x: node.x as f32, y: node.y as f32, w: *w, h: *h }, ground: ground(node) })
        .collect();
    let metrics = metrics();
    let joins: Vec<settle::Join> = scene
        .edges
        .iter()
        .filter_map(|edge| {
            let label = edge.label.as_deref().filter(|label| !label.trim().is_empty())?;
            let size = labels::label_size(label, metrics);
            Some(settle::Join { from: edge.from.clone(), to: edge.to.clone(), w: size.w, h: size.h })
        })
        .collect();
    let mut settled = settle::settle(&boxes, (tile::GAP_X, tile::GAP_Y), &joins);
    for one in &boxes {
        if !settled.iter().any(|(id, _)| *id == one.id) {
            settled.push((one.id.clone(), one.rect));
        }
    }
    settled
}

fn metrics() -> labels::LabelMetrics {
    labels::LabelMetrics { char_w: tile::LABEL_AT * 0.6, line_h: tile::LABEL_LEAD, pad: 4. }
}

fn ways(scene: &Scene, order: &[(String, Rect)], rects: &[Rect], hop_radius: f32) -> Vec<Option<Way>> {
    let grounds: HashSet<String> = scene.nodes.iter().filter(|node| ground(node)).map(|node| node.id.clone()).collect();
    // Every node is a tile with flat sides now, so an arrow may meet any of them anywhere along a side.
    let pointed: HashSet<String> = HashSet::new();
    let edges: Vec<route::RouteEdge> =
        scene.edges.iter().map(|edge| route::RouteEdge { id: &edge.id, from: &edge.from, to: &edge.to, path: arrows::path(edge), heads: arrows::heads(edge), pattern: arrows::pattern(edge) }).collect();
    let found = route::route_all(order, &edges, GAP, &grounds, &pointed, RUN, GAP / 4.);
    let paths: Vec<Option<Vec<Seg>>> = scene.edges.iter().map(|edge| found.iter().find(|(one, _)| *one == edge.id).map(|(_, way)| way.clone())).collect();
    // Where each head stands, so no arrow's words sit on one.
    let heads: Vec<Rect> = scene
        .edges
        .iter()
        .zip(&paths)
        .filter_map(|(edge, path)| Some((arrows::heads(edge), route::tips(path.as_deref()?)?)))
        .flat_map(|((start, end), tips)| [start.then_some(tips.start), end.then_some(tips.end)])
        .flatten()
        .map(|at| Rect { x: at.x - GAP / 4., y: at.y - GAP / 4., w: GAP / 2., h: GAP / 2. })
        .collect();
    let solid: Vec<Rect> = scene.nodes.iter().zip(rects).filter(|(node, _)| !ground(node)).map(|(_, rect)| *rect).collect();
    let drawn: Vec<labels::LabelEdge> = scene
        .edges
        .iter()
        .zip(&paths)
        .filter_map(|(edge, path)| Some(labels::LabelEdge { id: &edge.id, label: edge.label.as_deref(), path: path.as_deref()? }))
        .collect();
    let places = labels::place_labels(&drawn, &solid, metrics(), &heads);
    // Each arrow hops the ones laid before it, but never within the run a head asks to be straight.
    let keeps: Vec<(f32, f32)> = scene
        .edges
        .iter()
        .map(|edge| {
            let (start, end) = arrows::heads(edge);
            (if start { RUN } else { 0. }, if end { RUN } else { 0. })
        })
        .collect();
    let paths = route::hop_all(&paths, hop_radius, &keeps);
    scene
        .edges
        .iter()
        .zip(paths)
        .map(|(edge, path)| {
            let words = places
                .iter()
                .find(|(id, _)| *id == edge.id)
                .map(|(_, place)| (Rect { x: place.x - place.w / 2., y: place.y - place.h / 2., w: place.w, h: place.h }, place.lines.clone()));
            path.map(|path| Way { path, words })
        })
        .collect()
}

fn union(a: Rect, b: Rect) -> Rect {
    let (x, y) = (a.x.min(b.x), a.y.min(b.y));
    Rect { x, y, w: (a.x + a.w).max(b.x + b.w) - x, h: (a.y + a.h).max(b.y + b.h) - y }
}

thread_local! {
    /// Each scene's layout, by its key and the look it was measured in.
    static LAID: RefCell<HashMap<(SharedString, String), Rc<Laid>>> = RefCell::default();
}

fn laid(key: &SharedString, scene: &Scene, look: &Look, window: &mut Window, cx: &mut App) -> Rc<Laid> {
    let at = (key.clone(), look.id().to_string());
    let mocks = mock_view::sources(key, scene);
    let dark = mock_view::dark(look);
    // Each mock's content height, where a picture of its page has come.
    let heights: Vec<Option<f32>> = scene.nodes.iter().zip(mocks.iter()).map(|(node, mock)| mock_view::height(mock.as_deref()?, node.w? as f32, dark, cx)).collect();
    if let Some(held) = LAID.with(|held| held.borrow().get(&at).cloned()).filter(|held| held.mocks == heights) {
        return held;
    }
    // Each screen's sheet laid out once at full size, as the browser measures the node before React Flow places it.
    let trees = trees(key, scene);
    let sheets: Vec<Option<(f32, f32)>> = scene
        .nodes
        .iter()
        .zip(trees.iter())
        .zip(heights.iter())
        .map(|((node, tree), mock)| {
            if let Some(content) = mock {
                return Some((node.w? as f32, sheet_top(node) + content));
            }
            let tree = tree.as_ref()?;
            let w = sheet_width(node, tree);
            let mut sheet = screen::sheet(tree, None, 1., &screen::Paper::new(look, 1., w, false, window));
            let h = sheet.layout_as_root(size(AvailableSpace::Definite(px(w)), AvailableSpace::MinContent), window, cx).height;
            Some((w, sheet_top(node) + f32::from(h)))
        })
        .collect();
    let made = Rc::new(lay(scene, &trees, &sheets, heights, look, window));
    LAID.with(|held| held.borrow_mut().insert(at, made.clone()));
    made
}

// ---- drawing -------------------------------------------------------------------------------------

/// Scene units to the window: `fitView` on the nodes' bounds, never above 1.
#[derive(Clone, Copy)]
struct View {
    zoom: f32,
    origin: Point<Pixels>,
    centre: Pt,
}

impl View {
    /// The zoom that fits the whole scene inside `bounds`, less the fit's air and inset, never above the fit's most. There is no floor above
    /// the least a scene may be drawn at, so a wide or tall drawing is shrunk whole and never started at its corner.
    fn base(bounds: Bounds<Pixels>, scene: Rect, fit: Fit, most: f32) -> f32 {
        let (w, h) = (f32::from(bounds.size.width), f32::from(bounds.size.height));
        let pad = |side: f32| ((side - side / (1. + fit.padding())) / 2.).floor();
        let across = if scene.w > 0. { (w - 2. * pad(w) - 2. * fit.inset()) / scene.w } else { 1. };
        let down = if scene.h > 0. { (h - 2. * pad(h) - 2. * fit.inset()) / scene.h } else { 1. };
        across.min(down).clamp(Fit::MIN_ZOOM, most)
    }

    fn fit(bounds: Bounds<Pixels>, scene: Rect, fit: Fit, most: f32, lens: Lens) -> View {
        let zoom = Self::base(bounds, scene, fit, most) * lens.zoom;
        let centre = Pt { x: scene.x + scene.w / 2., y: scene.y + scene.h / 2. };
        View { zoom, origin: bounds.center() + point(px(lens.pan.0), px(lens.pan.1)), centre }
    }

    fn at(&self, p: Pt) -> Point<Pixels> {
        self.origin + point(px((p.x - self.centre.x) * self.zoom), px((p.y - self.centre.y) * self.zoom))
    }

    fn len(&self, n: f32) -> Pixels {
        px(n * self.zoom)
    }

    fn rect(&self, r: Rect) -> Bounds<Pixels> {
        Bounds::new(self.at(Pt { x: r.x, y: r.y }), size(self.len(r.w), self.len(r.h)))
    }
}

/// What a step lights: the nodes it names and the nodes its arrows reach; and how far into it the
/// clock is.
struct Playing {
    at: f64,
    into: f32,
}

fn playing(laid: &Laid, since: Option<Instant>) -> Option<Playing> {
    let since = since?;
    if laid.steps.is_empty() {
        return None;
    }
    let ms = since.elapsed().as_secs_f32() * 1000.;
    let index = (ms / STEP_MS) as usize % laid.steps.len();
    Some(Playing { at: laid.steps[index], into: ms % STEP_MS })
}

fn hue(look: &Look, color: Option<SceneColor>, default: SceneColor) -> Hsla {
    look.color(match color.unwrap_or(default) {
        SceneColor::Blue => token::SKETCH_BLUE,
        SceneColor::Teal => token::SKETCH_TEAL,
        SceneColor::Green => token::SKETCH_GREEN,
        SceneColor::Amber => token::SKETCH_AMBER,
        SceneColor::Orange => token::SKETCH_ORANGE,
        SceneColor::Red => token::SKETCH_RED,
        SceneColor::Grey => token::SKETCH_GREY,
    })
}

#[derive(Clone)]
struct Ink<'a> {
    look: &'a Look,
    view: View,
    sans: SharedString,
    mono: SharedString,
    /// A change's hue in place of the part's own, read against an earlier scene.
    over: Option<Hsla>,
    /// The hue of a frame that says something other than a hue does: the accent on the node a sketch starts at.
    frame: Option<Hsla>,
    /// Its words struck through: a part the earlier scene had and this one drops.
    struck: bool,
}

impl<'a> Ink<'a> {
    /// The ink for a part with this change: its status hue, and struck through where it was dropped.
    fn changed(&self, change: Option<Change>) -> Ink<'a> {
        let over = match change {
            Some(Change::Added) => Some(self.look.color(token::STATUS_COMPLETED_SUCCESS)),
            Some(Change::Removed) => Some(self.look.color(token::STATUS_COMPLETED_FAILED)),
            Some(Change::Changed) => Some(self.look.color(token::STATUS_AWAITING_REVIEW)),
            Some(Change::Same) | None => None,
        };
        Ink { over, struck: change == Some(Change::Removed), ..self.clone() }
    }

    /// The ink for the node a sketch starts at: its frame in the accent, and its hue its own. Only a box or a block of code
    /// has a frame to colour; the others carry the pill alone.
    fn started(&self, node: &SceneNode) -> Ink<'a> {
        Ink { frame: tile::is_tile(node).then(|| self.look.color(token::ACCENT)), ..self.clone() }
    }

    /// Words at scene size `at`, wrapped at `wrap` scene units, from `origin`. The height they took, in scene units.
    #[allow(clippy::too_many_arguments)]
    fn words(&self, text: &str, origin: Pt, wrap: Option<f32>, at: f32, weight: f32, mono: bool, color: Hsla, align: TextAlign, window: &mut Window, cx: &mut App) -> f32 {
        self.led(text, origin, wrap, at, LEAD, weight, mono, color, align, window, cx)
    }

    /// `words` with lines `lead` scene units tall.
    #[allow(clippy::too_many_arguments)]
    fn led(&self, text: &str, origin: Pt, wrap: Option<f32>, at: f32, lead: f32, weight: f32, mono: bool, color: Hsla, align: TextAlign, window: &mut Window, cx: &mut App) -> f32 {
        let font = Font { weight: FontWeight(weight), ..gpui::font(if mono { self.mono.clone() } else { self.sans.clone() }) };
        let strikethrough = self.struck.then(|| gpui::StrikethroughStyle { thickness: self.view.len(1.), color: Some(color) });
        let run = TextRun { len: text.len(), font, color, strikethrough, ..Default::default() };
        let lead = self.view.len(lead);
        let width = wrap.map(|wrap| self.view.len(wrap));
        let Ok(lines) = window.text_system().shape_text(SharedString::from(text.to_string()), self.view.len(at), &[run], width, None) else { return 0. };
        let mut down = 0.;
        for line in &lines {
            let top = self.view.at(Pt { x: origin.x, y: origin.y + down });
            let tall = line.size(lead).height;
            let _ = line.paint(top, lead, align, width.map(|width| Bounds::new(top, size(width, tall))), window, cx);
            down += f32::from(tall) / self.view.zoom;
        }
        down
    }

    fn icon(&self, name: &str, at: Pt, size: f32, color: Hsla, window: &mut Window, cx: &mut App) {
        let bounds = self.view.rect(Rect { x: at.x, y: at.y, w: size, h: size });
        let _ = window.paint_svg(bounds, crate::assets::icon_path(name), None, TransformationMatrix::unit(), color, cx);
    }
}

/// What the canvas worked out before painting: where everything stands, the step playing, and each
/// screen's sheet laid out at the view's zoom, by its node's index.
struct Prepared {
    laid: Rc<Laid>,
    view: View,
    changes: Option<Rc<Changes>>,
    now: Option<Playing>,
    /// Each with how far it stands back, which its title shares.
    sheets: Vec<Option<(AnyElement, f32)>>,
    /// Each mock's picture, where one has come, by its node's index: `None` for a node that is not a mock.
    mocks: Vec<Option<mock_view::Shown>>,
    /// Where each part stands, for the notes' numbers, which stand on them.
    placed: Vec<notes::Placed>,
    marks: Rc<Marks>,
}

/// Every node and, inside a mock, every element of its page, where it stands in scene units. Only a reading view holds them.
fn placed_of(scene: &Scene, laid: &Laid, sources: &[Option<Rc<mock_view::Mock>>], dark: bool, cx: &App) -> Vec<notes::Placed> {
    let mut placed = Vec::new();
    for (index, (node, rect)) in scene.nodes.iter().zip(&laid.rects).enumerate() {
        placed.push(notes::Placed { part: Part { id: node.id.clone(), label: part_label(node).to_string() }, rect: *rect, depth: if ground(node) { 0 } else { 1 }, node: true });
        let (Some(Some(mock)), Some(width)) = (sources.get(index), node.w) else { continue };
        let Some(parts) = mock_view::parts(mock, width as f32, dark, cx) else { continue };
        let top = sheet_top(node);
        placed.extend(parts.iter().map(|part| notes::Placed {
            part: Part { id: format!("{}#{}", node.id, part.key), label: part.label.clone() },
            rect: Rect { x: rect.x + part.x, y: rect.y + top + part.y, w: part.w, h: part.h },
            depth: 1 + part.depth,
            node: false,
        }));
    }
    placed
}

/// The nodes the step playing lights: those it names and those its arrows reach.
fn lit<'a>(scene: &'a Scene, now: Option<&Playing>) -> HashSet<&'a str> {
    let Some(now) = now else { return HashSet::new() };
    scene
        .nodes
        .iter()
        .filter(|node| node.step == Some(now.at))
        .map(|node| node.id.as_str())
        .chain(scene.edges.iter().filter(|edge| edge.step == Some(now.at)).map(|edge| edge.to.as_str()))
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn prepare(key: &SharedString, scene: &Scene, changes: Option<Rc<Changes>>, fit: Fit, clock: Clock, lens: Lens, marks: Rc<Marks>, bounds: Bounds<Pixels>, window: &mut Window, cx: &mut App) -> Prepared {
    let look = cx.global::<Look>().clone();
    let laid = laid(key, scene, &look, window, cx);
    let canvas = if fit == Fit::Column { laid.held } else { laid.bounds };
    let most = fit.most_zoom(scene);
    let view = View::fit(bounds, canvas, fit, most, lens);
    keep_canvas(key, canvas, window);
    let now = playing(&laid, clock.steps);
    let lit = lit(scene, now.as_ref());
    let trees = trees(key, scene);
    // Each mock asks for the picture its zoom wants, and draws the nearest it has until that comes.
    let sources = mock_view::sources(key, scene);
    let dark = mock_view::dark(&look);
    let mocks: Vec<Option<mock_view::Shown>> =
        scene.nodes.iter().zip(sources.iter()).map(|(node, mock)| Some(mock_view::Shown { image: mock_view::picture(mock.as_ref()?, node.w? as f32, view.zoom, dark, window, cx) })).collect();
    let mut sheets: Vec<Option<(AnyElement, f32)>> = Vec::new();
    window.with_content_mask(Some(ContentMask { bounds }), |window| {
        for (index, (node, tree)) in scene.nodes.iter().zip(trees.iter()).enumerate() {
            let Some(tree) = tree else {
                sheets.push(None);
                continue;
            };
            let rect = laid.rects[index];
            // A screen that holds the step in one of its elements rings that element and stands the rest of itself back, and does not dim.
            let inner = now.as_ref().map(|now| now.at).filter(|at| !lit.contains(node.id.as_str()) && tree.holds(*at));
            let same = changes.as_ref().and_then(|changes| changes.nodes.get(&node.id)) == Some(&Change::Same);
            let dim = (now.is_some() && inner.is_none() && !lit.contains(node.id.as_str())) || same;
            let w = sheet_width(node, tree);
            let top = sheet_top(node);
            let alpha = if dim { DIMMED } else { 1. };
            let mut sheet = screen::sheet(tree, inner, alpha, &screen::Paper::new(&look, view.zoom, w, fit == Fit::Reading, window));
            let room = size(AvailableSpace::Definite(view.len(w)), AvailableSpace::Definite(view.len(rect.h - top)));
            sheet.prepaint_as_root(view.at(Pt { x: rect.x, y: rect.y + top }), room, window, cx);
            sheets.push(Some((sheet, alpha)));
        }
    });
    let placed = if fit == Fit::Reading { placed_of(scene, &laid, &sources, dark, cx) } else { Vec::new() };
    // A column drawing is kept too, with nothing placed: the review panel zooms one in place and reads its fit from here.
    if fit != Fit::Preview {
        let base = View::base(bounds, canvas, fit, most);
        notes::keep(key, notes::Probe { view, placed: placed.clone(), fit: size(canvas.w * base, canvas.h * base), base });
    }
    Prepared { laid, view, changes, now, sheets, mocks, placed, marks }
}

#[allow(clippy::too_many_arguments)]
fn paint(scene: &Scene, prepared: &mut Prepared, fit: Fit, clock: Clock, bounds: Bounds<Pixels>, window: &mut Window, cx: &mut App) {
    let look = cx.global::<Look>().clone();
    let (laid, view, now) = (prepared.laid.clone(), prepared.view, prepared.now.take());
    let changes_held = prepared.changes.clone();
    let changes = changes_held.as_deref();
    let ink = Ink {
        look: &look,
        view,
        sans: look.font_stack(token::FONT_SANS).into_iter().next().unwrap_or_else(|| "IBM Plex Sans".into()),
        mono: look.font_stack(token::FONT_MONO).into_iter().next().unwrap_or_else(|| "IBM Plex Mono".into()),
        over: None,
        frame: None,
        struck: false,
    };
    let node_change = |node: &SceneNode| changes.and_then(|changes| changes.nodes.get(&node.id).copied());
    // Electron's flow crawl: a reading view's flows move while no step plays; a preview stays still.
    let crawl = match clock.flows {
        Some(since) if fit == Fit::Reading && now.is_none() => Some(since.elapsed().as_secs_f32() * 1000. % STEP_MS / STEP_MS * DASH * 2.),
        _ => None,
    };
    let lit = lit(scene, now.as_ref());
    window.with_content_mask(Some(ContentMask { bounds }), |window| {
        // A drawing that fills its tile has no ground of its own: the tile's surface is its background.
        if fit != Fit::Column {
            paint_canvas(&ink, laid.bounds, window);
        }
        for (node, rect) in scene.nodes.iter().zip(&laid.rects).filter(|(node, _)| ground(node)) {
            // A ground does not stand back for a step, but an unchanged one does against an earlier scene.
            let change = node_change(node);
            paint_ground(&ink.changed(change), node, *rect, laid.bounds, if change == Some(Change::Same) { DIMMED } else { 1. }, window, cx);
        }
        // Every tile's shadow goes down first, so none falls over an arrow where its head meets the tile.
        for (index, (node, rect)) in scene.nodes.iter().zip(&laid.rects).enumerate() {
            let sheet = matches!(prepared.mocks.get(index), Some(Some(_))) || matches!(prepared.sheets.get(index), Some(Some(_)));
            if !tile::is_tile(node) && !sheet {
                continue;
            }
            let dim = (now.is_some() && !lit.contains(node.id.as_str())) || node_change(node) == Some(Change::Same);
            let glow = if sheet { None } else { card_hue(&look, node, laid.marks.get(index).is_some_and(Option::is_some)) };
            paint_shadow(&ink, if sheet { sheet_frame(node, *rect) } else { *rect }, glow, if dim { DIMMED } else { 1. }, window);
        }
        for (edge, way) in scene.edges.iter().zip(&laid.ways) {
            let Some(way) = way else { continue };
            let change = changes.and_then(|changes| changes.edges.get(&edge.id).copied());
            let active = now.as_ref().is_some_and(|now| edge.step == Some(now.at));
            let dim = (now.is_some() && !active) || change == Some(Change::Same);
            let phase = if arrows::crawls(edge) { crawl.unwrap_or(0.) } else { 0. };
            let color = tile::arrow_color(scene, edge);
            paint_edge(&Ink { struck: false, ..ink.changed(change) }, way, color, arrows::pattern(edge), phase, arrows::heads(edge), active, dim, window, cx);
        }
        for (index, (node, rect)) in scene.nodes.iter().zip(&laid.rects).enumerate().filter(|(_, (node, _))| !ground(node)) {
            let change = node_change(node);
            let sheet = prepared.sheets.get_mut(index).and_then(Option::take);
            let lit_here = lit.contains(node.id.as_str());
            let dim = (now.is_some() && !lit_here) || change == Some(Change::Same);
            let alpha = if dim { DIMMED } else { 1. };
            let mark = laid.marks.get(index).copied().flatten();
            if let Some(Some(shown)) = prepared.mocks.get(index) {
                mock_view::paint(&ink.changed(change), node, *rect, shown, alpha, now.is_some() && lit_here, window, cx);
                paint_hairline(&ink, sheet_frame(node, *rect), alpha, window);
                if let Some(mark) = mark {
                    paint_mark(&ink, mark, window, cx);
                }
                continue;
            }
            if let Some((mut sheet, alpha)) = sheet {
                // A screen is a sheet of its own on the ground: its title above it, and the ring on the sheet.
                if let Some(title) = node.title.as_deref() {
                    let muted = look.color(token::FG_MUTED);
                    let at = f32::from(look.length(token::TEXT_SM));
                    ink.changed(change).words(title, Pt { x: rect.x, y: rect.y }, Some(rect.w), at, look.number(token::WEIGHT_HEADING), false, muted.opacity(muted.a * alpha), TextAlign::Left, window, cx);
                }
                sheet.paint(window, cx);
                paint_hairline(&ink, sheet_frame(node, *rect), alpha, window);
                if now.is_some() && lit_here {
                    let top = sheet_top(node);
                    let ring = Rect { x: rect.x - 2., y: rect.y + top - 2., w: rect.w + 4., h: rect.h - top + 4. };
                    let radius = f32::from(look.length(token::RADIUS_SM));
                    window.paint_quad(quad(view.rect(ring), view.len(radius + 2.), gpui::transparent_black(), view.len(2.), look.color(token::EDGE_FOCUS), BorderStyle::Solid));
                }
                if let Some(mark) = mark {
                    paint_mark(&ink, mark, window, cx);
                }
                continue;
            }
            let node_ink = ink.changed(change);
            let outline = laid.glyphs.get(index).copied().flatten();
            paint_node(&if mark.is_some() { node_ink.started(node) } else { node_ink }, node, *rect, outline, laid.badges.get(index).and_then(Option::as_ref), alpha, window, cx);
            if let Some(mark) = mark {
                paint_mark(&ink, mark, window, cx);
            }
            if now.is_some() && !dim {
                let ring = Rect { x: rect.x - 2., y: rect.y - 2., w: rect.w + 4., h: rect.h + 4. };
                window.paint_quad(quad(view.rect(ring), view.len(tile::RADIUS + 2.), gpui::transparent_black(), view.len(2.), look.color(token::EDGE_FOCUS), BorderStyle::Solid));
            }
        }
        for stroke in &scene.strokes {
            paint_ink(&ink, &stroke.points.iter().map(|p| Pt { x: p.x as f32, y: p.y as f32 }).collect::<Vec<_>>(), window);
        }
        notes::paint(&ink, &prepared.placed, &prepared.marks, window, cx);
        // A reading view names each part as Electron's does, its change first, so a walk can ring one.
        if fit == Fit::Reading {
            for (index, (node, rect)) in scene.nodes.iter().zip(&laid.rects).enumerate() {
                let name = match node_change(node) {
                    Some(change) => format!("{}, {}", change.said(), part_label(node)),
                    None => part_label(node).to_string(),
                };
                let name = if laid.marks.get(index).is_some_and(Option::is_some) { format!("{}, {name}", start::TAB_WORD) } else { name };
                crate::labels::note(gpui::Role::Group, name, view.rect(*rect), Some(view.len(tile::RADIUS)), window, cx);
            }
            // An element of a mock's page is named too, so a note aims at it as at a node.
            for one in prepared.placed.iter().filter(|one| !one.node) {
                crate::labels::note(gpui::Role::Group, one.part.label.clone(), view.rect(one.rect), None, window, cx);
            }
        }
        if let Some(now) = &now {
            let t = ease(look.easing(token::EASE).unwrap_or([0.2, 0., 0., 1.]), (now.into / DOT_MS).min(1.));
            for (edge, way) in scene.edges.iter().zip(&laid.ways) {
                let Some(way) = way.as_ref().filter(|_| edge.step == Some(now.at)) else { continue };
                if let Some(dot) = along(&way.path, t) {
                    let r = 8.;
                    window.paint_quad(quad(view.rect(Rect { x: dot.x - r, y: dot.y - r, w: 2. * r, h: 2. * r }), view.len(r), look.color(token::EDGE_FOCUS), px(0.), gpui::transparent_black(), BorderStyle::Solid));
                }
            }
        }
    });
    if now.is_some() || crawl.is_some() {
        window.request_animation_frame();
    }
}

/// A hairline that stays one pixel wide however far the drawing is shrunk.
fn hair(view: View, width: f32) -> Pixels {
    px(width * view.zoom.max(1.))
}

/// Where a mock's picture and a screen's sheet stand in their node: under the title.
fn sheet_frame(node: &SceneNode, rect: Rect) -> Rect {
    let top = sheet_top(node);
    Rect { x: rect.x, y: rect.y + top, w: rect.w, h: rect.h - top }
}

/// `color` a share `by` of the way to white: the outline in its chip reads lighter than the hue it is tinted with.
fn lightened(color: Hsla, by: f32) -> Hsla {
    Hsla { l: color.l + (1. - color.l) * by, ..color }
}

/// `base` lifted a share `by` toward the text colour: the light end of a card's or the canvas's gradient.
fn lifted(look: &Look, base: Hsla, by: f32) -> Hsla {
    base.blend(look.color(token::FG_DEFAULT).opacity(by))
}

/// The hue a card is framed in and glows with: the start's accent, else its own colour. A card with neither has none.
fn card_hue(look: &Look, node: &SceneNode, started: bool) -> Option<Hsla> {
    if started {
        Some(look.color(token::ACCENT))
    } else {
        node.color.map(|color| hue(look, Some(color), SceneColor::Grey))
    }
}

/// The tint of the chip the outline and an icon sit in: the card's hue, amber on a decision, else the text colour, faint.
fn chip_tint(look: &Look, node: &SceneNode, outline: Option<glyph::Mark>, started: bool) -> Hsla {
    let decision = (outline == Some(glyph::Mark::Decision)).then(|| hue(look, Some(SceneColor::Amber), SceneColor::Amber));
    match card_hue(look, node, started).or(decision) {
        Some(color) => color.opacity(tile::CHIP_TINT),
        None => look.color(token::FG_DEFAULT).opacity(tile::CHIP_PLAIN),
    }
}

/// The canvas the drawing stands on: the ground colour, a little lighter toward its top left, in a rounded frame round
/// everything the drawing holds.
fn paint_canvas(ink: &Ink, rect: Rect, window: &mut Window) {
    let (look, view) = (ink.look, ink.view);
    let base = look.color(token::BG_SUNKEN);
    let ground = gpui::linear_gradient(tile::CANVAS_ANGLE, gpui::linear_color_stop(lifted(look, base, tile::CANVAS_LIFT), 0.), gpui::linear_color_stop(base, 1.));
    window.paint_quad(quad(view.rect(rect), view.len(tile::CANVAS_RADIUS), ground, hair(view, tile::HAIRLINE), look.color(token::BORDER_SUBTLE), BorderStyle::Solid));
}

/// The soft drop under a tile, a mock's picture or a screen's sheet, at `alpha` of its strength, and round a card with a
/// hue a faint glow of it.
fn paint_shadow(ink: &Ink, rect: Rect, glow: Option<Hsla>, alpha: f32, window: &mut Window) {
    let view = ink.view;
    let mut shadows: Vec<BoxShadow> = ink
        .look
        .shadows(token::SHADOW_CARD)
        .into_iter()
        .map(|layer| BoxShadow {
            color: layer.color.opacity(layer.color.a * alpha),
            offset: point(layer.offset.x * view.zoom, layer.offset.y * view.zoom),
            blur_radius: layer.blur_radius * view.zoom,
            spread_radius: layer.spread_radius * view.zoom,
            inset: false,
        })
        .collect();
    if let Some(glow) = glow {
        shadows.push(BoxShadow { color: glow.opacity(tile::GLOW_ALPHA * alpha), offset: point(px(0.), px(0.)), blur_radius: view.len(tile::GLOW_BLUR), spread_radius: px(0.), inset: false });
    }
    window.paint_drop_shadows(view.rect(rect), Corners::all(view.len(tile::RADIUS)), &shadows);
}

/// The frame a mock or a screen has: a hairline round its picture, over it, as a tile has.
fn paint_hairline(ink: &Ink, rect: Rect, alpha: f32, window: &mut Window) {
    let (look, view) = (ink.look, ink.view);
    let edge = look.color(token::BORDER_SUBTLE);
    window.paint_quad(quad(view.rect(rect), view.len(tile::RADIUS), gpui::transparent_black(), hair(view, tile::HAIRLINE), edge.opacity(edge.a * alpha), BorderStyle::Solid));
}

/// A group or a lane: a group is a hairline frame round its children on a faint fill and a lane a row of the canvas under a
/// hairline, each named at its top left. A change's hue, where there is one, is the frame's and the name's.
fn paint_ground(ink: &Ink, node: &SceneNode, rect: Rect, canvas: Rect, alpha: f32, window: &mut Window, cx: &mut App) {
    let (look, view) = (ink.look, ink.view);
    let faded = |color: Hsla| color.opacity(color.a * alpha);
    let edge = faded(ink.over.unwrap_or_else(|| look.color(token::BORDER_SUBTLE)));
    if node.kind == SceneKind::Lane {
        // The canvas's own edge is the first lane's top.
        if rect.y > canvas.y + 0.5 {
            let line = Bounds::new(view.at(Pt { x: rect.x, y: rect.y }), size(view.len(rect.w), hair(view, tile::HAIRLINE)));
            window.paint_quad(quad(line, px(0.), edge, px(0.), gpui::transparent_black(), BorderStyle::Solid));
        }
    } else {
        let fill = faded(look.color(token::FG_DEFAULT).opacity(tile::GROUP_FILL));
        window.paint_quad(quad(view.rect(rect), view.len(tile::RADIUS), fill, hair(view, tile::HAIRLINE), edge, BorderStyle::Solid));
    }
    let Some(title) = node.title.as_deref() else { return };
    let name = faded(ink.over.unwrap_or_else(|| look.color(token::FG_MUTED)));
    let at = Pt { x: rect.x + tile::GROUND_PAD, y: rect.y + tile::GROUND_PAD - 2. };
    ink.led(title, at, None, tile::KIND_AT, tile::KIND_AT + 4., tile::NAME_WEIGHT, false, name, TextAlign::Left, window, cx);
}

/// The Start pill: its word on a blue to teal chip, above the top left of the node the sketch starts at. It is drawn at full
/// strength whatever step plays.
fn paint_mark(ink: &Ink, pill: Rect, window: &mut Window, cx: &mut App) {
    let (look, view) = (ink.look, ink.view);
    let fill = gpui::linear_gradient(tile::PILL_ANGLE, gpui::linear_color_stop(look.color(token::SKETCH_BLUE), 0.), gpui::linear_color_stop(look.color(token::SKETCH_TEAL), 1.));
    window.paint_quad(quad(view.rect(pill), view.len(tile::PILL_H / 2.), fill, px(0.), gpui::transparent_black(), BorderStyle::Solid));
    let at = Pt { x: pill.x + tile::PILL_PAD, y: pill.y };
    ink.led(start::TAB_WORD, at, None, tile::PILL_AT, tile::PILL_H, tile::NAME_WEIGHT, false, look.color(token::TEXT_ON_ACCENT), TextAlign::Left, window, cx);
}

/// A step's badge: a dark circle in a light hairline, its number in the text colour.
fn paint_badge(ink: &Ink, rect: Rect, number: &str, alpha: f32, window: &mut Window, cx: &mut App) {
    let (look, view) = (ink.look, ink.view);
    let faded = |color: Hsla| color.opacity(color.a * alpha);
    let ring = faded(look.color(token::FG_DEFAULT).opacity(tile::BADGE_RING));
    window.paint_quad(quad(view.rect(rect), view.len(tile::BADGE / 2.), faded(look.color(token::BG_SUNKEN)), hair(view, tile::HAIRLINE), ring, BorderStyle::Solid));
    ink.led(number, Pt { x: rect.x, y: rect.y }, Some(rect.w), tile::BADGE_AT, tile::BADGE, tile::BADGE_WEIGHT, false, faded(look.color(token::FG_DEFAULT)), TextAlign::Center, window, cx);
}

#[allow(clippy::too_many_arguments)]
fn paint_node(ink: &Ink, node: &SceneNode, rect: Rect, outline: Option<glyph::Mark>, badge: Option<&(Rect, String)>, alpha: f32, window: &mut Window, cx: &mut App) {
    let look = ink.look;
    let faded = |color: Hsla| color.opacity(color.a * alpha);
    match node.kind {
        // A mock is drawn by `mock_view` and a screen as a sheet, before this is reached: their place is kept, nothing drawn.
        SceneKind::Mock | SceneKind::Screen | SceneKind::Group | SceneKind::Lane => {}
        SceneKind::Label => {
            let words = node.title.as_deref().or(node.body.as_deref()).unwrap_or("");
            ink.words(words, Pt { x: rect.x, y: rect.y }, None, 14., 600., false, faded(look.color(token::FG_MUTED)), TextAlign::Left, window, cx);
        }
        SceneKind::Wire => paint_wire(ink, node, rect, alpha, window, cx),
        SceneKind::Box | SceneKind::Code => {
            paint_tile(ink, node, rect, outline, alpha, window, cx);
            if let Some((at, number)) = badge {
                paint_badge(ink, *at, number, alpha, window, cx);
            }
        }
    }
}

/// A card: a raised surface, lighter at its top, in a hairline, the outline of what it is in a tinted chip at its left, its
/// title and, under it, its body as a muted line of sans. A block of code has no outline and keeps its body as mono; its
/// language stands at the top right. A card with a hue (its colour, the start's accent) is framed in it; a change's frame is
/// its hue at full strength.
#[allow(clippy::too_many_arguments)]
fn paint_tile(ink: &Ink, node: &SceneNode, rect: Rect, outline: Option<glyph::Mark>, alpha: f32, window: &mut Window, cx: &mut App) {
    let (look, view) = (ink.look, ink.view);
    let faded = |color: Hsla| color.opacity(color.a * alpha);
    let started = ink.frame.is_some();
    let surface = look.color(token::BG_RAISED);
    let (edge, width) = match (ink.over, card_hue(look, node, started)) {
        (Some(color), _) => (color, tile::FRAME),
        (None, Some(color)) => (color.opacity(tile::FRAME_ALPHA), tile::HAIRLINE),
        (None, None) => (look.color(token::BORDER_SUBTLE), tile::HAIRLINE),
    };
    let fill = gpui::linear_gradient(tile::CARD_ANGLE, gpui::linear_color_stop(faded(lifted(look, surface, tile::CARD_LIFT)), 0.), gpui::linear_color_stop(faded(surface), 1.));
    window.paint_quad(quad(view.rect(rect), view.len(tile::RADIUS), fill, hair(view, width), faded(edge), BorderStyle::Solid));
    let (lead_w, lang_w, lang_text_w, lines) = {
        let measure = Measure { window: &*window, sans: ink.sans.clone(), mono: ink.mono.clone() };
        let (lead_w, lang_w) = (lead_width(node, Facts { mark: outline }), lang_width(node, &measure));
        let lang_text_w = node.lang.as_deref().map_or(0., |lang| measure.width(lang, tile::LANG_AT, tile::KIND_WEIGHT, true));
        let wrap = (rect.w - 2. * tile::PAD_X - lead_w - lang_w).max(1.);
        (lead_w, lang_w, lang_text_w, body_lines(node, &measure, wrap))
    };
    let top = rect.y + tile::PAD_Y;
    let mut x = rect.x + tile::PAD_X;
    let said = faded(ink.over.unwrap_or_else(|| lightened(glyph_hue(look, node, outline, started), tile::LIGHTEN)));
    let tint = faded(chip_tint(look, node, outline, started));
    let chip_y = top + (tile::TITLE_LEAD - tile::CHIP) / 2.;
    let inset = (tile::CHIP - glyph::SIZE) / 2.;
    let chip = |x: f32, window: &mut Window| {
        window.paint_quad(quad(view.rect(Rect { x, y: chip_y, w: tile::CHIP, h: tile::CHIP }), view.len(tile::CHIP_RADIUS), tint, px(0.), gpui::transparent_black(), BorderStyle::Solid));
    };
    if let Some(mark) = outline {
        chip(x, window);
        paint_glyph(ink, mark, Pt { x: x + inset, y: chip_y + inset }, said, window);
        x += tile::CHIP + tile::GLYPH_GAP;
    }
    if let Some(icon) = node.icon.as_deref() {
        chip(x, window);
        let color = faded(lightened(card_hue(look, node, started).unwrap_or_else(|| look.color(token::FG_MUTED)), tile::LIGHTEN));
        ink.icon(icon, Pt { x: x + inset, y: chip_y + inset }, glyph::SIZE, color, window, cx);
        x += tile::CHIP + tile::GLYPH_GAP;
    }
    if let Some(lang) = node.lang.as_deref() {
        ink.led(lang, Pt { x: rect.x + rect.w - tile::PAD_X - lang_text_w, y: top }, None, tile::LANG_AT, tile::TITLE_LEAD, tile::KIND_WEIGHT, true, faded(look.color(token::FG_MUTED)), TextAlign::Left, window, cx);
    }
    let wrap = (rect.w - 2. * tile::PAD_X - lead_w - lang_w).max(1.);
    let mut y = top;
    if let Some(title) = node.title.as_deref().filter(|title| !title.is_empty()) {
        y += ink.led(title, Pt { x, y }, Some(wrap), tile::TITLE_AT, tile::TITLE_LEAD, tile::TITLE_WEIGHT, false, faded(look.color(token::FG_DEFAULT)), TextAlign::Left, window, cx);
        y += tile::BODY_GAP;
    }
    let (body_at, body_lead, body_mono) = body_face(node);
    let color = faded(look.color(if node.kind == SceneKind::Code { token::FG_DEFAULT } else { token::FG_MUTED }));
    for line in lines {
        ink.led(&line, Pt { x, y }, None, body_at, body_lead, 400., body_mono, color, TextAlign::Left, window, cx);
        y += body_lead;
    }
}

/// The hue of a node's outline: the accent on the node the sketch starts at, amber on a decision, else the node's own colour
/// where it names one, else muted.
fn glyph_hue(look: &Look, node: &SceneNode, outline: Option<glyph::Mark>, started: bool) -> Hsla {
    if started {
        look.color(token::ACCENT)
    } else if outline == Some(glyph::Mark::Decision) {
        hue(look, Some(SceneColor::Amber), SceneColor::Amber)
    } else if let Some(color) = node.color {
        hue(look, Some(color), SceneColor::Grey)
    } else {
        look.color(token::FG_MUTED)
    }
}

/// A node's outline, stroked in `color` with its square's top left at `at`.
fn paint_glyph(ink: &Ink, mark: glyph::Mark, at: Pt, color: Hsla, window: &mut Window) {
    let view = ink.view;
    let width = view.len(glyph::STROKE);
    let round = StrokeOptions::default().with_line_width(f32::from(width)).with_line_cap(LineCap::Round).with_line_join(LineJoin::Round);
    let mut line = PathBuilder::stroke(width).with_style(PathStyle::Stroke(round));
    let to = |x: f32, y: f32| view.at(Pt { x: at.x + x, y: at.y + y });
    for cmd in glyph::outline(mark) {
        match cmd {
            glyph::Cmd::Move(x, y) => line.move_to(to(x, y)),
            glyph::Cmd::Line(x, y) => line.line_to(to(x, y)),
            glyph::Cmd::Cubic(a, b, c, d, e, f) => line.cubic_bezier_to(to(e, f), to(a, b), to(c, d)),
            glyph::Cmd::Close => line.close(),
        }
    }
    if let Ok(path) = line.build() {
        window.paint_path(path, color);
    }
}

/// A wireframe block: what a part of a screen is, drawn as grey bars and never as a control.
fn paint_wire(ink: &Ink, node: &SceneNode, rect: Rect, alpha: f32, window: &mut Window, cx: &mut App) {
    let (look, view) = (ink.look, ink.view);
    let faded = |color: Hsla| color.opacity(color.a * alpha);
    let strong = faded(look.color(token::BORDER_STRONG));
    window.paint_quad(quad(view.rect(rect), view.len(RADIUS), faded(look.color(token::BG_SUNKEN)), view.len(BORDER), strong, BorderStyle::Dashed));
    let inner = Rect { x: rect.x + BORDER, y: rect.y + BORDER, w: rect.w - 2. * BORDER, h: rect.h - 2. * BORDER };
    let muted = faded(look.color(token::FG_MUTED));
    let title = node.title.as_deref();
    match node.wire.unwrap_or(WirePart::Card) {
        part @ (WirePart::Card | WirePart::List | WirePart::Text) => {
            let mut y = inner.y + PAD_X;
            let x = inner.x + PAD_X;
            let w = inner.w - 2. * PAD_X;
            if let Some(title) = title {
                ink.words(title, Pt { x, y }, Some(w), TEXT, 600., false, muted, TextAlign::Left, window, cx);
                y += LEAD + 8.;
            }
            let bars: &[f32] = if part == WirePart::Text { &[1., 1.] } else { &[1., 1., 0.6] };
            for share in bars {
                window.paint_quad(quad(view.rect(Rect { x, y, w: w * share, h: 8. }), view.len(3.), strong, px(0.), gpui::transparent_black(), BorderStyle::Solid));
                y += 16.;
            }
        }
        WirePart::Button => {
            window.paint_quad(quad(view.rect(inner), view.len(RADIUS - BORDER), strong, px(0.), gpui::transparent_black(), BorderStyle::Solid));
            if let Some(title) = title {
                ink.words(title, Pt { x: inner.x + PAD_X, y: inner.y + (inner.h - LEAD) / 2. }, Some(inner.w - 2. * PAD_X), TEXT, 600., false, faded(look.color(token::FG_DEFAULT)), TextAlign::Center, window, cx);
            }
        }
        WirePart::Input => {
            let field = Rect { x: inner.x + 8., y: inner.y + 8., w: inner.w - 16., h: inner.h - 16. };
            window.paint_quad(quad(view.rect(field), px(0.), gpui::transparent_black(), view.len(1.), strong, BorderStyle::Solid));
            if let Some(title) = title {
                ink.words(title, Pt { x: field.x + PAD_X, y: field.y + PAD_Y }, Some(field.w - 2. * PAD_X), TEXT, 600., false, muted, TextAlign::Left, window, cx);
            }
        }
        WirePart::Nav => {
            if let Some(title) = title {
                ink.words(title, Pt { x: inner.x + PAD_X, y: inner.y + (inner.h - LEAD) / 2. }, Some(inner.w - 2. * PAD_X), TEXT, 600., false, muted, TextAlign::Left, window, cx);
            }
        }
    }
}

/// An arrow along its way: 1.5px, round, in its hue, solid, dashed or dotted, a solid head at the ends that carry one, and its
/// words in the same hue on a backing the colour of the canvas. A dashed line carries its dashes' phase: how far along the
/// way they have crawled.
#[allow(clippy::too_many_arguments)]
fn paint_edge(ink: &Ink, way: &Way, color: SceneColor, pattern: Pattern, phase: f32, heads: (bool, bool), active: bool, dim: bool, window: &mut Window, cx: &mut App) {
    let (look, view) = (ink.look, ink.view);
    let base = if active { look.color(token::EDGE_FOCUS) } else { ink.over.unwrap_or_else(|| hue(look, Some(color), SceneColor::Grey)) };
    let stroke = if dim { base.opacity(base.a * DIMMED) } else { base };
    let width = view.len(tile::ARROW);
    let round = StrokeOptions::default().with_line_width(f32::from(width)).with_line_cap(LineCap::Round).with_line_join(LineJoin::Round);
    let mut line = match pattern {
        // A dot is a round cap on a run of no length, and a solid line ends round.
        Pattern::Dotted | Pattern::Solid => PathBuilder::stroke(width).with_style(PathStyle::Stroke(round)),
        Pattern::Dashed => PathBuilder::stroke(width),
    };
    // The line stops inside each head, so its end does not show past the head's point.
    let shown = shortened(&way.path, if heads.0 { HEAD / 2. } else { 0. }, if heads.1 { HEAD / 2. } else { 0. });
    match pattern {
        Pattern::Dashed => dashes(&mut line, &flat(&shown), phase, DASH, DASH * 2., view),
        Pattern::Dotted => dashes(&mut line, &flat(&shown), 0., DOT, DOT_PERIOD, view),
        Pattern::Solid => {
            let mut cursor = Pt::default();
            for seg in &shown {
                match *seg {
                    Seg::Move(p) => line.move_to(view.at(p)),
                    Seg::Line(p) => line.line_to(view.at(p)),
                    Seg::Quad(ctrl, p) => line.curve_to(view.at(p), view.at(ctrl)),
                    Seg::Cubic(c1, c2, p) => line.cubic_bezier_to(view.at(p), view.at(c1), view.at(c2)),
                    Seg::Arc(p, _, sweep) => {
                        for at in route::arc_points(cursor, p, sweep, HOP_CUTS) {
                            line.line_to(view.at(at));
                        }
                    }
                }
                cursor = match *seg {
                    Seg::Move(p) | Seg::Line(p) | Seg::Quad(_, p) | Seg::Cubic(_, _, p) | Seg::Arc(p, _, _) => p,
                };
            }
        }
    }
    if let Ok(path) = line.build() {
        window.paint_path(path, stroke);
    }
    if let Some(tips) = route::tips(&way.path) {
        if heads.1 {
            paint_head(view, tips.end, tips.before_end, stroke, window);
        }
        if heads.0 {
            paint_head(view, tips.start, tips.after_start, stroke, window);
        }
    }
    if let Some((rect, lines)) = &way.words {
        let widest = {
            let measure = Measure { window: &*window, sans: ink.sans.clone(), mono: ink.mono.clone() };
            lines.iter().map(|line| measure.width(line, tile::LABEL_AT, tile::LABEL_WEIGHT, false)).fold(0., f32::max)
        };
        let chip = tile::label_chip(*rect, widest, lines.len());
        let backing = look.color(token::BG_SUNKEN).opacity(tile::LABEL_FILL);
        window.paint_quad(quad(view.rect(chip), view.len(chip.h / 2.), backing, hair(view, tile::HAIRLINE), look.color(token::BORDER_SUBTLE), BorderStyle::Solid));
        let words = if dim { base.opacity(base.a * DIMMED) } else { base };
        for (at, text) in lines.iter().enumerate() {
            let line_at = Pt { x: chip.x, y: chip.y + tile::LABEL_PAD_Y + at as f32 * tile::LABEL_LEAD };
            ink.led(text, line_at, Some(chip.w), tile::LABEL_AT, tile::LABEL_LEAD, tile::LABEL_WEIGHT, false, words, TextAlign::Center, window, cx);
        }
    }
}

/// The way with its ends drawn in by `start` and `end`: where it runs straight there, the end moves along it. A short run
/// is left as it is.
fn shortened(path: &[Seg], start: f32, end: f32) -> Vec<Seg> {
    let mut out = path.to_vec();
    let pull = |from: Pt, to: Pt, by: f32| {
        let long = (to.x - from.x).hypot(to.y - from.y);
        (by > 0. && long > 2. * by).then(|| Pt { x: to.x + (from.x - to.x) * by / long, y: to.y + (from.y - to.y) * by / long })
    };
    if end > 0. && out.len() >= 2 {
        let last = out.len() - 1;
        let before = match out[last - 1] {
            Seg::Move(p) | Seg::Line(p) | Seg::Quad(_, p) | Seg::Cubic(_, _, p) | Seg::Arc(p, _, _) => p,
        };
        if let Seg::Line(to) = out[last] {
            if let Some(moved) = pull(before, to, end) {
                out[last] = Seg::Line(moved);
            }
        }
    }
    if start > 0. && out.len() >= 2 {
        if let (Seg::Move(from), Seg::Line(to)) = (out[0], out[1]) {
            if let Some(moved) = pull(to, from, start) {
                out[0] = Seg::Move(moved);
            }
        }
    }
    out
}

/// A solid triangle `HEAD` long and as wide, its tip on where the way ends and pointing away from `from`.
fn paint_head(view: View, at: Pt, from: Pt, color: Hsla, window: &mut Window) {
    let Some(corners) = route::head_triangle(at, from, HEAD, HEAD / 2.) else { return };
    let mut head = PathBuilder::fill();
    head.add_polygon(&corners.map(|corner| view.at(corner)), true);
    if let Ok(path) = head.build() {
        window.paint_path(path, color);
    }
}

/// The owner's ink: a smooth line through his points, 2px at every zoom, round at its ends.
fn paint_ink(ink: &Ink, points: &[Pt], window: &mut Window) {
    let Some((first, rest)) = points.split_first() else { return };
    let options = StrokeOptions::default().with_line_width(2.).with_line_cap(LineCap::Round).with_line_join(LineJoin::Round);
    let mut line = PathBuilder::stroke(px(2.)).with_style(PathStyle::Stroke(options));
    line.move_to(ink.view.at(*first));
    if rest.is_empty() {
        line.line_to(ink.view.at(*first) + point(px(0.5), px(0.)));
    }
    for (at, p) in rest.iter().enumerate() {
        match rest.get(at + 1) {
            Some(next) => line.curve_to(ink.view.at(Pt { x: (p.x + next.x) / 2., y: (p.y + next.y) / 2. }), ink.view.at(*p)),
            None => line.line_to(ink.view.at(*p)),
        }
    }
    if let Ok(path) = line.build() {
        window.paint_path(path, ink.look.color(token::FG_MUTED));
    }
}

/// The way as a line through points, its rounded corners, curves and hops cut into short chords.
fn flat(path: &[Seg]) -> Vec<Pt> {
    let mut points: Vec<Pt> = Vec::new();
    for seg in path {
        match *seg {
            Seg::Move(p) | Seg::Line(p) => points.push(p),
            Seg::Quad(ctrl, p) => {
                let from = points.last().copied().unwrap_or(ctrl);
                for step in 1..=CORNER_CUTS {
                    let t = step as f32 / CORNER_CUTS as f32;
                    let u = 1. - t;
                    points.push(Pt { x: u * u * from.x + 2. * u * t * ctrl.x + t * t * p.x, y: u * u * from.y + 2. * u * t * ctrl.y + t * t * p.y });
                }
            }
            Seg::Cubic(c1, c2, p) => {
                let from = points.last().copied().unwrap_or(c1);
                for step in 1..=CURVE_CUTS {
                    points.push(route::cubic_at(from, c1, c2, p, step as f32 / CURVE_CUTS as f32));
                }
            }
            Seg::Arc(p, _, sweep) => {
                let from = points.last().copied().unwrap_or(p);
                points.extend(route::arc_points(from, p, sweep, HOP_CUTS));
            }
        }
    }
    points
}

/// `stroke-dasharray: on (period - on)` along the points, `phase` further on than where it starts, as a
/// falling `stroke-dashoffset` moves it.
fn dashes(line: &mut PathBuilder, points: &[Pt], phase: f32, on_len: f32, period: f32, view: View) {
    let mut walked = 0.;
    for pair in points.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        let long = (b.x - a.x).hypot(b.y - a.y);
        if long <= 0. {
            continue;
        }
        let at = |s: f32| Pt { x: a.x + (b.x - a.x) * s / long, y: a.y + (b.y - a.y) * s / long };
        // From one edge of a dash to the next along this piece, drawing the dashes.
        let mut s = 0.;
        while s < long {
            let into = (walked + s - phase).rem_euclid(period);
            let (on, left) = if into < on_len { (true, on_len - into) } else { (false, period - into) };
            let end = (s + left.max(0.01)).min(long);
            if on {
                line.move_to(view.at(at(s)));
                line.line_to(view.at(at(end)));
            }
            s = end;
        }
        walked += long;
    }
}

/// The point `t` of the way along it, by length; quadratic corners are read as their chords.
fn along(path: &[Seg], t: f32) -> Option<Pt> {
    let points = route::polyline(path);
    let total: f32 = points.windows(2).map(|pair| (pair[1].x - pair[0].x).hypot(pair[1].y - pair[0].y)).sum();
    let mut left = total * t.clamp(0., 1.);
    for pair in points.windows(2) {
        let long = (pair[1].x - pair[0].x).hypot(pair[1].y - pair[0].y);
        if left <= long && long > 0. {
            let k = left / long;
            return Some(Pt { x: pair[0].x + (pair[1].x - pair[0].x) * k, y: pair[0].y + (pair[1].y - pair[0].y) * k });
        }
        left -= long;
    }
    points.last().copied()
}

/// `cubic-bezier(x1, y1, x2, y2)` at `t`, found by bisection on x.
fn ease([x1, y1, x2, y2]: [f32; 4], t: f32) -> f32 {
    let curve = |a: f32, b: f32, s: f32| 3. * a * s * (1. - s).powi(2) + 3. * b * s.powi(2) * (1. - s) + s.powi(3);
    let (mut lo, mut hi) = (0_f32, 1_f32);
    for _ in 0..24 {
        let mid = (lo + hi) / 2.;
        if curve(x1, x2, mid) < t {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    curve(y1, y2, (lo + hi) / 2.)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scene(text: &str) -> Scene {
        serde_json::from_str(text).expect("a scene")
    }

    fn inside(outer: Rect, inner: Rect) -> bool {
        inner.x >= outer.x - 0.01 && inner.y >= outer.y - 0.01 && inner.x + inner.w <= outer.x + outer.w + 0.01 && inner.y + inner.h <= outer.y + outer.h + 0.01
    }

    #[test]
    fn the_canvas_reaches_a_node_left_of_zero_a_node_above_it_and_a_hand_line_past_both() {
        let rects = [Rect { x: -200., y: 10., w: 100., h: 50. }, Rect { x: 300., y: -90., w: 100., h: 50. }];
        let drawn = scene(r#"{"nodes":[],"edges":[],"strokes":[{"id":"s","points":[{"x":-420,"y":20},{"x":-380,"y":260}]}]}"#);
        let canvas = canvas_of(&rects, &[None, None], &[None, None], &[None, None], &drawn);
        assert!(canvas.x <= -420. - INK_REACH - tile::CANVAS_PAD + 0.01, "the left edge reaches the hand line at -420, not the node at -200: {canvas:?}");
        assert!(canvas.y <= -90. - tile::CANVAS_PAD + 0.01, "the top edge reaches the node above zero");
        assert!(canvas.x + canvas.w >= 400. + tile::CANVAS_PAD - 0.01 && canvas.y + canvas.h >= 260. + INK_REACH + tile::CANVAS_PAD - 0.01);
        assert!(rects.iter().all(|rect| inside(canvas, *rect)));
    }

    #[gpui::test]
    fn the_fit_holds_a_scene_whose_leftmost_node_is_at_x_minus_two_hundred_with_its_arrow_words(cx: &mut gpui::TestAppContext) {
        cx.update(|cx| {
            gpui_component::init(cx);
            crate::look::init(cx);
        });
        let text = r#"{"nodes":[
            {"id":"far","kind":"box","x":-200,"y":-120,"title":"Leftmost"},
            {"id":"mid","kind":"box","x":300,"y":0,"title":"Middle"},
            {"id":"low","kind":"box","x":900,"y":400,"title":"Rightmost and lowest"}],
            "edges":[{"id":"e1","from":"far","to":"mid","label":"a long arrow label that sits between them"},{"id":"e2","from":"mid","to":"low","label":"another label"}]}"#;
        let raw: ipc::RawScene = serde_json::from_str(text).expect("a raw scene");
        let drawn = Rc::new(raw.scene().expect("it reads"));
        let (_, cx) = cx.add_window_view(|_, _| gpui::Empty);
        let (laid, view) = cx.update(|window, cx| {
            let look = cx.global::<Look>().clone();
            let laid = laid(&SharedString::from("far-left"), &drawn, &look, window, cx);
            let bounds = Bounds::new(point(px(0.), px(0.)), size(px(900.), px(600.)));
            (laid.clone(), View::fit(bounds, laid.bounds, Fit::Reading, Fit::Reading.most_zoom(&drawn), Lens::default()))
        });
        let leftmost = laid.rects.iter().map(|rect| rect.x).fold(f32::INFINITY, f32::min);
        assert!(leftmost < 0., "the leftmost node stays left of zero: {leftmost}");
        for rect in &laid.rects {
            assert!(inside(laid.bounds, *rect), "every node is in the canvas: {rect:?} in {:?}", laid.bounds);
        }
        for (rect, _) in laid.ways.iter().flatten().filter_map(|way| way.words.as_ref()) {
            assert!(inside(laid.bounds, *rect), "an arrow's words are in the canvas: {rect:?}");
        }
        // And the canvas is drawn whole inside the view it is fitted to.
        let drawn_at = view.rect(laid.bounds);
        assert!(drawn_at.left() >= px(0.) && drawn_at.top() >= px(0.) && drawn_at.right() <= px(900.) && drawn_at.bottom() <= px(600.), "fitted: {drawn_at:?}");
        let corner = view.rect(*laid.rects.iter().min_by(|a, b| a.x.total_cmp(&b.x)).expect("a node"));
        assert!(corner.left() >= px(0.), "the leftmost node is inside the view: {corner:?}");
    }

    /// The version-check sketch of the mock fleet: a start over a row of three over a row of two, with arrows fanning between rows.
    fn version_checks() -> Rc<Scene> {
        let all: serde_json::Value = ipc::decode("the fixture", include_bytes!("../../fixtures/sketch-shapes-threads.json")).expect("the fixture reads");
        let drawing = all["threads"].as_array().expect("threads").iter().flat_map(|thread| thread["items"].as_array().expect("items").iter()).find(|item| item["id"] == "sk-shapes").expect("the sketch")["drawing"].to_string();
        let raw: ipc::RawScene = ipc::decode("a raw scene", drawing.as_bytes()).expect("a raw scene");
        Rc::new(raw.scene().expect("it reads"))
    }

    fn lay_version_checks(cx: &mut gpui::TestAppContext) -> (Rc<Scene>, Rc<Laid>) {
        cx.update(|cx| {
            gpui_component::init(cx);
            crate::look::init(cx);
        });
        let drawn = version_checks();
        let (_, cx) = cx.add_window_view(|_, _| gpui::Empty);
        let laid = cx.update(|window, cx| {
            let look = cx.global::<Look>().clone();
            laid(&SharedString::from("version-checks"), &drawn, &look, window, cx)
        });
        (drawn, laid)
    }

    /// How far apart two rects stand across x and across y; negative where they overlap that way.
    fn gaps(a: Rect, b: Rect) -> (f32, f32) {
        ((b.x - (a.x + a.w)).max(a.x - (b.x + b.w)), (b.y - (a.y + a.h)).max(a.y - (b.y + b.h)))
    }

    fn corners(way: &Way) -> usize {
        way.path.iter().filter(|seg| matches!(seg, Seg::Quad(..))).count()
    }

    fn rect_of_node(drawn: &Scene, laid: &Laid, id: &str) -> Rect {
        laid.rects[drawn.nodes.iter().position(|node| node.id == id).expect("a node")]
    }

    /// Two lanes side by side, as a Drone placed them for "How a merge crosses the link", each with its own box, and a box
    /// under the first.
    fn lay_side_by_side(cx: &mut gpui::TestAppContext) -> (Rc<Scene>, Rc<Laid>) {
        cx.update(|cx| {
            gpui_component::init(cx);
            crate::look::init(cx);
        });
        let wire = br#"{"nodes":[
            {"id":"a","kind":"lane","x":0,"y":0,"w":420,"h":560,"title":"Mac that keeps"},
            {"id":"b","kind":"lane","x":520,"y":0,"w":420,"h":560,"title":"Mac that gives up"},
            {"id":"one","kind":"box","x":40,"y":80,"title":"Holds the claim","step":1},
            {"id":"two","kind":"box","x":560,"y":80,"title":"Lets it go","step":2},
            {"id":"three","kind":"box","x":60,"y":400,"title":"Merges","body":"The merge runs on the Mac that keeps the claim, after the other let it go."}
        ],"edges":[{"id":"e1","from":"one","to":"two","step":1}]}"#;
        let raw: ipc::RawScene = ipc::decode("a raw scene", wire).expect("a raw scene");
        let drawn = Rc::new(raw.scene().expect("it reads"));
        let (_, cx) = cx.add_window_view(|_, _| gpui::Empty);
        let laid = cx.update(|window, cx| {
            let look = cx.global::<Look>().clone();
            laid(&SharedString::from("side-by-side"), &drawn, &look, window, cx)
        });
        (drawn, laid)
    }

    #[gpui::test]
    fn lanes_set_side_by_side_stand_as_columns_each_holding_its_own_boxes(cx: &mut gpui::TestAppContext) {
        let (drawn, laid) = lay_side_by_side(cx);
        let (a, b) = (rect_of_node(&drawn, &laid, "a"), rect_of_node(&drawn, &laid, "b"));
        assert!(gaps(a, b).0 >= 0., "the lanes stand side by side, not on each other: {a:?} {b:?}");
        assert!(a.h >= 560. && b.h >= 560., "each keeps at least the height it was given: {a:?} {b:?}");
        let centre = |rect: Rect| (rect.x + rect.w / 2., rect.y + rect.h / 2.);
        let holds = |lane: Rect, id: &str| {
            let (x, y) = centre(rect_of_node(&drawn, &laid, id));
            x >= lane.x && x <= lane.x + lane.w && y >= lane.y && y <= lane.y + lane.h
        };
        assert!(holds(a, "one") && holds(a, "three"), "the first lane holds its two boxes");
        assert!(holds(b, "two") && !holds(b, "one"), "the second holds its own box and not the first lane's");
    }

    #[gpui::test]
    fn lanes_set_one_under_another_still_run_the_canvas_width(cx: &mut gpui::TestAppContext) {
        cx.update(|cx| {
            gpui_component::init(cx);
            crate::look::init(cx);
        });
        let wire = br#"{"nodes":[
            {"id":"a","kind":"lane","x":0,"y":0,"w":300,"h":200,"title":"Fleet"},
            {"id":"b","kind":"lane","x":0,"y":260,"w":300,"h":200,"title":"Bridge"},
            {"id":"one","kind":"box","x":40,"y":60,"title":"Reads"},
            {"id":"two","kind":"box","x":400,"y":320,"title":"Draws"}
        ],"edges":[]}"#;
        let raw: ipc::RawScene = ipc::decode("a raw scene", wire).expect("a raw scene");
        let rows = Rc::new(raw.scene().expect("it reads"));
        let (_, cx) = cx.add_window_view(|_, _| gpui::Empty);
        let laid = cx.update(|window, cx| {
            let look = cx.global::<Look>().clone();
            laid(&SharedString::from("rows"), &rows, &look, window, cx)
        });
        for id in ["a", "b"] {
            let lane = rect_of_node(&rows, &laid, id);
            assert!((lane.x - laid.bounds.x).abs() < 0.01 && (lane.w - laid.bounds.w).abs() < 0.01, "{id} runs edge to edge: {lane:?} in {:?}", laid.bounds);
        }
    }

    #[gpui::test]
    fn cards_keep_sixty_four_side_by_side_and_eighty_between_rows_and_the_canvas_keeps_thirty_two(cx: &mut gpui::TestAppContext) {
        let (drawn, laid) = lay_version_checks(cx);
        for (at, a) in laid.rects.iter().enumerate() {
            for b in &laid.rects[at + 1..] {
                let (across, down) = gaps(*a, *b);
                if down < 0. {
                    assert!(across >= tile::GAP_X - 0.01, "side by side: {a:?} and {b:?} are {across} apart");
                } else if across < 0. {
                    assert!(down >= tile::GAP_Y - 0.01, "in rows: {a:?} and {b:?} are {down} apart");
                } else {
                    assert!(across >= tile::GAP_X - 0.01 || down >= tile::GAP_Y - 0.01, "diagonally: {a:?} and {b:?}");
                }
            }
            assert!(a.x - laid.bounds.x >= 32. - 0.01 && a.y - laid.bounds.y >= 32. - 0.01, "the canvas keeps its padding: {a:?} in {:?}", laid.bounds);
            assert!(laid.bounds.x + laid.bounds.w - (a.x + a.w) >= 32. - 0.01 && laid.bounds.y + laid.bounds.h - (a.y + a.h) >= 32. - 0.01);
        }
        // The three checks stand in one row, and so do the two blocking cards: pushing apart never squeezes a row out of line.
        let row = |ids: &[&str]| ids.iter().map(|id| rect_of_node(&drawn, &laid, id).y).collect::<Vec<_>>();
        assert!(row(&["n1", "n2", "n3"]).windows(2).all(|pair| pair[0] == pair[1]), "the row of three is aligned");
        assert!(row(&["off", "on"]).windows(2).all(|pair| pair[0] == pair[1]), "the row of two is aligned");
    }

    #[gpui::test]
    fn every_arrow_leaves_the_side_facing_its_target_with_at_most_two_bends(cx: &mut gpui::TestAppContext) {
        let (drawn, laid) = lay_version_checks(cx);
        for (edge, way) in drawn.edges.iter().zip(&laid.ways) {
            let way = way.as_ref().expect("a way");
            assert!(corners(way) <= 2, "{} has {} bends", edge.id, corners(way));
            // A flow that runs down leaves a card's foot and reaches the next card's top.
            let (from, to) = (rect_of_node(&drawn, &laid, &edge.from), rect_of_node(&drawn, &laid, &edge.to));
            let tips = route::tips(&way.path).expect("tips");
            assert!((tips.start.y - (from.y + from.h)).abs() < 0.2, "{} leaves the foot of its card: {:?} from {from:?}", edge.id, tips.start);
            assert!((tips.end.y - to.y).abs() < 0.2, "{} reaches the top of its card: {:?} to {to:?}", edge.id, tips.end);
        }
    }

    /// The nearest a point stands to a rect: nothing where it is inside.
    fn distance(p: Pt, r: Rect) -> f32 {
        let (dx, dy) = ((r.x - p.x).max(p.x - (r.x + r.w)).max(0.), (r.y - p.y).max(p.y - (r.y + r.h)).max(0.));
        dx.hypot(dy)
    }

    #[gpui::test]
    fn no_run_passes_within_eight_of_a_card_it_does_not_join(cx: &mut gpui::TestAppContext) {
        let (drawn, laid) = lay_version_checks(cx);
        for (edge, way) in drawn.edges.iter().zip(&laid.ways) {
            let points = route::polyline(&way.as_ref().expect("a way").path);
            for (node, rect) in drawn.nodes.iter().zip(&laid.rects) {
                if node.id == edge.from || node.id == edge.to {
                    continue;
                }
                for pair in points.windows(2) {
                    let long = (pair[1].x - pair[0].x).hypot(pair[1].y - pair[0].y);
                    for step in 0..=(long.ceil() as usize) {
                        let k = if long > 0. { step as f32 / long.ceil() } else { 0. };
                        let p = Pt { x: pair[0].x + (pair[1].x - pair[0].x) * k, y: pair[0].y + (pair[1].y - pair[0].y) * k };
                        assert!(distance(p, *rect) >= 8. - 0.01, "{} passes {} at {p:?}", edge.id, node.id);
                    }
                }
            }
        }
    }

    #[gpui::test]
    fn arrows_out_of_one_card_share_a_trunk_and_arrows_into_one_arrive_sixteen_apart(cx: &mut gpui::TestAppContext) {
        let (drawn, laid) = lay_version_checks(cx);
        let tips = |id: &str| {
            let at = drawn.edges.iter().position(|edge| edge.id == id).expect("an edge");
            route::tips(&laid.ways[at].as_ref().expect("a way").path).expect("tips")
        };
        // The three out of the start, and the two out of each of the first two checks, leave from one point.
        for group in [["s1", "s2", "s3"].as_slice(), ["a1", "a2"].as_slice(), ["b1", "b2"].as_slice()] {
            let first = tips(group[0]).start;
            for id in group {
                assert!((tips(id).start.x - first.x).abs() < 0.2 && (tips(id).start.y - first.y).abs() < 0.2, "{id} shares its trunk's start");
            }
        }
        // The two out of the last check differ in line, so they take places of their own.
        assert!((tips("c1").start.x - tips("c2").start.x).abs() >= 16., "a dashed and a dotted arrow out of one card do not share a place");
        // Into one card: at least sixteen apart along its side.
        for target in ["off", "on"] {
            let mut ends: Vec<f32> = drawn.edges.iter().filter(|edge| edge.to == target).map(|edge| tips(&edge.id).end.x).collect();
            ends.sort_by(f32::total_cmp);
            assert!(ends.windows(2).all(|pair| pair[1] - pair[0] >= 16. - 0.01), "arrows into {target} arrive {ends:?}");
        }
    }

    #[gpui::test]
    fn runs_of_different_trunks_keep_twelve_apart(cx: &mut gpui::TestAppContext) {
        let (drawn, laid) = lay_version_checks(cx);
        let trunk = |at: usize| (drawn.edges[at].from.clone(), arrows::pattern(&drawn.edges[at]));
        let runs: Vec<Vec<(Pt, Pt)>> = laid
            .ways
            .iter()
            .map(|way| route::polyline(&way.as_ref().expect("a way").path).windows(2).map(|pair| (pair[0], pair[1])).filter(|(a, b)| a.x == b.x || a.y == b.y).collect())
            .collect();
        for one in 0..runs.len() {
            for other in one + 1..runs.len() {
                if trunk(one) == trunk(other) {
                    continue;
                }
                for (a, b) in &runs[one] {
                    for (c, d) in &runs[other] {
                        let (along, apart) = if a.y == b.y && c.y == d.y {
                            (a.x.max(b.x).min(c.x.max(d.x)) - a.x.min(b.x).max(c.x.min(d.x)), (a.y - c.y).abs())
                        } else if a.x == b.x && c.x == d.x {
                            (a.y.max(b.y).min(c.y.max(d.y)) - a.y.min(b.y).max(c.y.min(d.y)), (a.x - c.x).abs())
                        } else {
                            continue;
                        };
                        // The tenths the path is written in, and a rounded corner's chord, may move a run a hair.
                        assert!(along <= 2. || apart >= 12. - 0.2, "{} and {} run {apart} apart along {along}", drawn.edges[one].id, drawn.edges[other].id);
                    }
                }
            }
        }
    }

    #[gpui::test]
    fn an_arrows_words_stand_on_a_straight_run_never_on_a_bend(cx: &mut gpui::TestAppContext) {
        let (drawn, laid) = lay_version_checks(cx);
        let at = drawn.edges.iter().position(|edge| edge.label.is_some()).expect("a labelled arrow");
        let way = laid.ways[at].as_ref().expect("a way");
        let (rect, _) = way.words.as_ref().expect("its words");
        let centre = Pt { x: rect.x + rect.w / 2., y: rect.y + rect.h / 2. };
        let stands = labels::straights(&way.path).iter().any(|run| {
            let (lo, hi) = if run.across { (run.from.x.min(run.to.x), run.from.x.max(run.to.x)) } else { (run.from.y.min(run.to.y), run.from.y.max(run.to.y)) };
            let (on, along, room) = if run.across { ((centre.y - run.from.y).abs(), centre.x, rect.w) } else { ((centre.x - run.from.x).abs(), centre.y, rect.h) };
            on < 0.6 && along - room / 2. >= lo - 0.01 && along + room / 2. <= hi + 0.01
        });
        assert!(stands, "the words at {centre:?} lie on one straight run of {:?}", way.path);
    }

    #[gpui::test]
    fn an_arrow_does_not_run_down_the_gap_between_two_cards_when_a_way_round_exists(cx: &mut gpui::TestAppContext) {
        cx.update(|cx| {
            gpui_component::init(cx);
            crate::look::init(cx);
        });
        // A card above and one below, both over the gap between two neighbours: a straight drop would squeeze through it.
        let text = r#"{"nodes":[
            {"id":"p","kind":"box","x":0,"y":160,"w":220,"title":"Left"},
            {"id":"q","kind":"box","x":284,"y":160,"w":220,"title":"Right"},
            {"id":"top","kind":"box","x":142,"y":0,"w":220,"title":"Above"},
            {"id":"foot","kind":"box","x":142,"y":340,"w":220,"title":"Below"}],
            "edges":[{"id":"e","from":"top","to":"foot"}]}"#;
        let raw: ipc::RawScene = ipc::decode("a raw scene", text.as_bytes()).expect("a raw scene");
        let drawn = Rc::new(raw.scene().expect("it reads"));
        let (_, cx) = cx.add_window_view(|_, _| gpui::Empty);
        let laid = cx.update(|window, cx| {
            let look = cx.global::<Look>().clone();
            laid(&SharedString::from("gap"), &drawn, &look, window, cx)
        });
        let (p, q) = (rect_of_node(&drawn, &laid, "p"), rect_of_node(&drawn, &laid, "q"));
        let points = route::polyline(&laid.ways[0].as_ref().expect("a way").path);
        for pair in points.windows(2) {
            let down = (pair[0].x - pair[1].x).abs() < 0.01;
            let (x, lo, hi) = (pair[0].x, pair[0].y.min(pair[1].y), pair[0].y.max(pair[1].y));
            let between = x > p.x + p.w && x < q.x;
            assert!(!(down && between && hi - lo > 4. && hi > p.y && lo < p.y + p.h), "the arrow runs down the gap between the cards: {points:?}");
        }
    }

    #[gpui::test]
    fn the_arrows_out_of_the_start_go_out_as_one_trunk_and_hop_nothing(cx: &mut gpui::TestAppContext) {
        let (drawn, laid) = lay_version_checks(cx);
        for id in ["s1", "s2", "s3"] {
            let at = drawn.edges.iter().position(|edge| edge.id == id).expect("an edge");
            let hops = laid.ways[at].as_ref().expect("a way").path.iter().filter(|seg| matches!(seg, Seg::Arc(..))).count();
            assert_eq!(hops, 0, "{id} crosses nothing, and meets its trunk's other arrows where they leave");
        }
    }

    /// The take "Ready" of the status chip question as Fleet holds it: one `mock` node 260 wide.
    const CHIP_TAKE: &str = r#"{"edges":[],"nodes":[{"html":"<div class=\"armada-chip\" data-part=\"chip\">Ready to try</div>","id":"chip-ready","kept":"504e17b72d4e43e5ad85c131e2a969b8","kind":"mock","styles":["packages/components/src/compositions/FactChip/FactChip.css"],"w":260,"x":0,"y":0}]}"#;

    /// Two drawings, each in a column box 1040 wide, as a take's picture is drawn in a wide reading column.
    struct Columns(Rc<Scene>, Rc<Scene>);

    impl gpui::Render for Columns {
        fn render(&mut self, _: &mut Window, _: &mut gpui::Context<Self>) -> impl IntoElement {
            use gpui::ParentElement as _;
            gpui::div()
                .flex()
                .flex_col()
                .size_full()
                .child(gpui::div().w(px(1040.)).h(px(200.)).child(view("chip-take".into(), self.0.clone(), Fit::Column, Clock::default(), Lens::default())))
                .child(gpui::div().w(px(1040.)).h(px(200.)).child(view("box-take".into(), self.1.clone(), Fit::Column, Clock::default(), Lens::default())))
        }
    }

    #[gpui::test]
    fn a_chip_sized_mock_in_a_column_four_times_its_width_is_drawn_at_its_own_size_and_a_box_still_grows(cx: &mut gpui::TestAppContext) {
        cx.update(|cx| {
            gpui_component::init(cx);
            crate::look::init(cx);
        });
        let raw: ipc::RawScene = ipc::decode("a chip take", CHIP_TAKE.as_bytes()).expect("a raw scene");
        let chip = Rc::new(raw.scene().expect("it reads"));
        let boxed: ipc::RawScene = ipc::decode("a box", br#"{"nodes":[{"id":"b","kind":"box","x":0,"y":0,"w":220,"title":"Box"}],"edges":[]}"#).expect("a raw scene");
        let boxed = Rc::new(boxed.scene().expect("it reads"));
        let (_, cx) = cx.add_window_view(|_, _| Columns(chip, boxed));
        cx.simulate_resize(size(px(1040.), px(400.)));
        for _ in 0..2 {
            cx.update(|window, _| window.refresh());
            cx.run_until_parked();
        }
        let (fit, drawn_at) = fit_of("chip-take").expect("the drawing was fitted");
        let (natural, _) = canvas_size("chip-take").expect("its canvas");
        assert!(drawn_at >= 1., "a mock stands at its own size or smaller, never the column's width: {drawn_at}");
        assert!(fit.width <= natural + 0.01, "no wider than its natural size times one: {fit:?} of {natural}");
        let (_, box_at) = fit_of("box-take").expect("the box drawing was fitted");
        assert!(box_at < 1., "a drawing of boxes still grows in the column, up to twice: {box_at}");
    }
}
