//! A sketch's scene drawn as Electron's `SketchScene` draws it: boxes, groups, lanes, labels, code
//! and wireframe blocks where the Drone put them, settled apart, joined by routed arrows with their
//! words, the owner's ink over them, and the steps played in turn.
//!
//! Sizes are CSS pixels, as `SketchScene.css` gives them, worked out once per scene with the
//! app's own text measure; the drawing is then scaled to fit, never up, as `fitView` does. A mock
//! node draws nothing until the app can draw a page (plan task #20), and a screen node comes with
//! its own walk: both keep their place, so arrows still reach them.

mod geometry;
mod labels;
mod route;
mod settle;

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::time::Instant;

use gpui::{
    canvas, point, px, quad, size, App, BorderStyle, Bounds, ContentMask, Corners, Edges, Font, FontWeight, Hsla, IntoElement, PathBuilder,
    PathStyle, Pixels, Point, SharedString, StrokeOptions, Styled, TextAlign, TextRun, TransformationMatrix, Window,
};
use ipc::{Scene, SceneColor, SceneKind, SceneNode, SceneShape, WirePart};
use lyon::tessellation::{LineCap, LineJoin};

use crate::look::{token, Look};
use geometry::{Pt, Rect};
use route::Seg;

/// `--space-8`: what settles boxes apart and routes arrows round them.
const GAP: f32 = 32.;
/// The node frame's border, `2px solid`.
const BORDER: f32 = 2.;
/// `--text-xs` / `--leading-xs`, the scene's words.
const TEXT: f32 = 13.;
const LEAD: f32 = 18.;
/// The head band's and body's padding: `8px 12px`.
const PAD_Y: f32 = 8.;
const PAD_X: f32 = 12.;
const ICON: f32 = 12.;
const RADIUS: f32 = 6.;
/// What a step lasts (`--duration-pulse`), and how long its dot travels (three quarters of it).
const STEP_MS: f32 = 1600.;
const DOT_MS: f32 = 1200.;
/// A node that is not in the step playing.
const DIMMED: f32 = 0.35;
/// A flow's dash and gap, `--space-2`; the crawl moves the pattern one period per `--duration-pulse`.
const DASH: f32 = 8.;
/// The chords a rounded corner is cut into for the dashes.
const CORNER_CUTS: usize = 6;

/// How the scene is drawn: shrunk on a thread card, or read in a sheet of its own.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Fit {
    Preview,
    Reading,
}

impl Fit {
    fn padding(self) -> f32 {
        match self {
            Fit::Preview => 0.04,
            Fit::Reading => 0.1,
        }
    }

    fn min_zoom(self) -> f32 {
        match self {
            Fit::Preview => 0.02,
            Fit::Reading => 0.2,
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
    canvas(|_, _, _| (), move |bounds, _, window, cx| paint(&key, &scene, fit, clock, lens, bounds, window, cx)).size_full()
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

/// Whether the scene has steps to play.
pub(crate) fn plays(scene: &Scene) -> bool {
    scene.nodes.iter().any(|node| node.step.is_some()) || scene.edges.iter().any(|edge| edge.step.is_some())
}

// ---- layout --------------------------------------------------------------------------------------

/// Where everything stands, in scene units.
struct Laid {
    /// Each node's settled rect, by its index in the scene.
    rects: Vec<Rect>,
    /// Each arrow's way, by its index in the scene, with its words placed. An arrow the router found no way for is `None`.
    ways: Vec<Option<Way>>,
    bounds: Rect,
    /// The step numbers, sorted and unique: the same number plays together.
    steps: Vec<f64>,
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
}

fn ground(node: &SceneNode) -> bool {
    matches!(node.kind, SceneKind::Group | SceneKind::Lane)
}

/// A box drawn in an outline of its own rather than the frame: no head band, its words centred.
fn shaped(node: &SceneNode) -> Option<SceneShape> {
    (node.kind == SceneKind::Box).then_some(node.shape).flatten().filter(|shape| *shape != SceneShape::Box)
}

/// `.shaped`'s padding for a shape: inline, then top and bottom.
fn shaped_padding(shape: SceneShape, w: f32) -> (f32, f32, f32) {
    match shape {
        SceneShape::Pill => (24., 12., 12.),
        SceneShape::Diamond => (w * 0.25, 12., 16.),
        SceneShape::Hexagon => (32., 12., 12.),
        SceneShape::Cylinder => (16., 24., 12.),
        _ => (16., 12., 12.),
    }
}

/// The node's size as the browser lays it out: `max-content` between the box's floor and ceiling,
/// `w` and `h` as floors, and the height its words take.
fn size_of(node: &SceneNode, measure: &Measure, (floor, ceiling): (f32, f32)) -> (f32, f32) {
    let fixed = |w: f32, h: f32| (node.w.map_or(w, |at| at as f32), node.h.map_or(h, |at| at as f32));
    let floored = |w: f32, h: f32| (node.w.map_or(w, |at| w.max(at as f32)), node.h.map_or(h, |at| h.max(at as f32)));
    let clamp = |content: f32| content.clamp(floor, ceiling);
    let title = node.title.as_deref().unwrap_or("");
    let heading = measure.width(title, TEXT, 600., false);
    match node.kind {
        SceneKind::Group | SceneKind::Lane => fixed(0., 0.),
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
        SceneKind::Box if shaped(node).is_some() => {
            let shape = shaped(node).unwrap_or(SceneShape::Box);
            let named = node.icon.is_some() || node.title.is_some();
            let name = if node.icon.is_some() { ICON + 8. } else { 0. } + heading;
            let body = node.body.as_deref().map_or(0., |body| measure.width(body, TEXT, 400., false));
            let w = match shape {
                SceneShape::Circle => floor * 0.75,
                SceneShape::Diamond => floor * 1.25,
                _ => {
                    let (inline, _, _) = shaped_padding(shape, 0.);
                    floored(clamp(name.max(body) + 2. * inline + 2. * BORDER), 0.).0
                }
            };
            let (inline, top, bottom) = shaped_padding(shape, w);
            let wrap = (w - 2. * BORDER - 2. * inline).max(1.);
            let lines = node.body.as_deref().map_or(0, |body| measure.lines(body, TEXT, 400., false, wrap));
            let content = if named { LEAD } else { 0. } + if named && lines > 0 { 4. } else { 0. } + lines as f32 * LEAD;
            match shape {
                SceneShape::Circle => (w, w),
                SceneShape::Diamond => (w, w * 3. / 5.),
                _ => floored(w, 2. * BORDER + top + bottom + content),
            }
        }
        SceneKind::Box | SceneKind::Code => {
            let mono = node.kind == SceneKind::Code;
            let head = node.title.is_some() || node.icon.is_some();
            let lang = node.lang.as_deref().map_or(0., |lang| 8. + measure.width(lang, TEXT, 600., false));
            let band = if node.icon.is_some() { ICON + 8. } else { 0. } + heading + lang;
            let body = node.body.as_deref().map_or(0., |body| measure.width(body, TEXT, 400., mono));
            let (w, _) = floored(clamp(band.max(body) + 2. * PAD_X + 2. * BORDER), 0.);
            let wrap = (w - 2. * PAD_X - 2. * BORDER).max(1.);
            let band_h = if head { 2. * PAD_Y + LEAD * measure.lines(title, TEXT, 600., false, (wrap - (band - heading)).max(1.)) as f32 } else { 0. };
            let body_h = node.body.as_deref().map_or(0., |body| 2. * PAD_Y + LEAD * measure.lines(body, TEXT, 400., mono, wrap) as f32);
            floored(w, 2. * BORDER + band_h + body_h)
        }
    }
}

fn lay(scene: &Scene, look: &Look, window: &Window) -> Laid {
    let measure = Measure {
        window,
        sans: look.font_stack(token::FONT_SANS).into_iter().next().unwrap_or_else(|| "IBM Plex Sans".into()),
        mono: look.font_stack(token::FONT_MONO).into_iter().next().unwrap_or_else(|| "IBM Plex Mono".into()),
    };
    let limits = (f32::from(look.length(token::W_SKETCH_BOX)), f32::from(look.length(token::W_SKETCH_BOX_MAX)));
    let sizes: Vec<(f32, f32)> = scene.nodes.iter().map(|node| size_of(node, &measure, limits)).collect();
    let order = arrange(scene, &sizes);
    let rects: Vec<Rect> = scene.nodes.iter().map(|node| order.iter().find(|(id, _)| *id == node.id).map_or_else(Rect::default, |(_, rect)| *rect)).collect();
    let ways = ways(scene, &order, &rects);
    let bounds = rects.iter().copied().reduce(union).unwrap_or_default();
    let mut steps: Vec<f64> = scene.nodes.iter().filter_map(|node| node.step).chain(scene.edges.iter().filter_map(|edge| edge.step)).collect();
    steps.sort_by(f64::total_cmp);
    steps.dedup();
    Laid { rects, ways, bounds, steps }
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
    let mut settled = settle::settle(&boxes, GAP, &joins);
    for one in &boxes {
        if !settled.iter().any(|(id, _)| *id == one.id) {
            settled.push((one.id.clone(), one.rect));
        }
    }
    settled
}

fn metrics() -> labels::LabelMetrics {
    labels::LabelMetrics { char_w: TEXT * 0.56, line_h: LEAD, pad: 4. }
}

fn ways(scene: &Scene, order: &[(String, Rect)], rects: &[Rect]) -> Vec<Option<Way>> {
    let grounds: HashSet<String> = scene.nodes.iter().filter(|node| ground(node)).map(|node| node.id.clone()).collect();
    let pointed: HashSet<String> =
        scene.nodes.iter().filter(|node| node.shape.is_some_and(|shape| !matches!(shape, SceneShape::Box | SceneShape::Cylinder))).map(|node| node.id.clone()).collect();
    let edges: Vec<route::RouteEdge> = scene.edges.iter().map(|edge| route::RouteEdge { id: &edge.id, from: &edge.from, to: &edge.to }).collect();
    let found = route::route_all(order, &edges, GAP, &grounds, &pointed);
    let way_of = |id: &str| found.iter().find(|(one, _)| one == id).map(|(_, way)| way);
    let paths: Vec<Option<Vec<Seg>>> = scene.edges.iter().map(|edge| way_of(&edge.id).map(|way| route::rounded_path(way, GAP / 4.))).collect();
    let heads: Vec<Rect> = found.iter().filter_map(|(_, way)| way.last()).map(|end| Rect { x: end.x - GAP / 4., y: end.y - GAP / 4., w: GAP / 2., h: GAP / 2. }).collect();
    let solid: Vec<Rect> = scene.nodes.iter().zip(rects).filter(|(node, _)| !ground(node)).map(|(_, rect)| *rect).collect();
    let drawn: Vec<labels::LabelEdge> = scene
        .edges
        .iter()
        .zip(&paths)
        .filter_map(|(edge, path)| Some(labels::LabelEdge { id: &edge.id, label: edge.label.as_deref(), path: path.as_deref()? }))
        .collect();
    let places = labels::place_labels(&drawn, &solid, metrics(), &heads);
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

fn laid(key: &SharedString, scene: &Scene, look: &Look, window: &Window) -> Rc<Laid> {
    let at = (key.clone(), look.id().to_string());
    if let Some(held) = LAID.with(|held| held.borrow().get(&at).cloned()) {
        return held;
    }
    let made = Rc::new(lay(scene, look, window));
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
    fn fit(bounds: Bounds<Pixels>, scene: Rect, fit: Fit, lens: Lens) -> View {
        let (w, h) = (f32::from(bounds.size.width), f32::from(bounds.size.height));
        let pad = |side: f32| ((side - side / (1. + fit.padding())) / 2.).floor();
        let across = if scene.w > 0. { (w - 2. * pad(w)) / scene.w } else { 1. };
        let down = if scene.h > 0. { (h - 2. * pad(h)) / scene.h } else { 1. };
        let zoom = across.min(down).clamp(fit.min_zoom(), 1.) * lens.zoom;
        View { zoom, origin: bounds.center() + point(px(lens.pan.0), px(lens.pan.1)), centre: Pt { x: scene.x + scene.w / 2., y: scene.y + scene.h / 2. } }
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

/// `color-mix(hue 14%, --bg-raised)`.
fn tint(look: &Look, hue: Hsla) -> Hsla {
    look.color(token::BG_RAISED).blend(hue.opacity(look.percent(token::ROW_TINT_RECENT)))
}

struct Ink<'a> {
    look: &'a Look,
    view: View,
    sans: SharedString,
    mono: SharedString,
}

impl Ink<'_> {
    /// Words at scene size `at`, wrapped at `wrap` scene units, from `origin`. The height they took, in scene units.
    #[allow(clippy::too_many_arguments)]
    fn words(&self, text: &str, origin: Pt, wrap: Option<f32>, at: f32, weight: f32, mono: bool, color: Hsla, align: TextAlign, window: &mut Window, cx: &mut App) -> f32 {
        let font = Font { weight: FontWeight(weight), ..gpui::font(if mono { self.mono.clone() } else { self.sans.clone() }) };
        let run = TextRun { len: text.len(), font, color, ..Default::default() };
        let lead = self.view.len(LEAD);
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

    fn icon(&self, name: &str, at: Pt, color: Hsla, window: &mut Window, cx: &mut App) {
        let bounds = self.view.rect(Rect { x: at.x, y: at.y, w: ICON, h: ICON });
        let _ = window.paint_svg(bounds, crate::assets::icon_path(name), None, TransformationMatrix::unit(), color, cx);
    }
}

#[allow(clippy::too_many_arguments)]
fn paint(key: &SharedString, scene: &Scene, fit: Fit, clock: Clock, lens: Lens, bounds: Bounds<Pixels>, window: &mut Window, cx: &mut App) {
    let look = cx.global::<Look>().clone();
    let laid = laid(key, scene, &look, window);
    let view = View::fit(bounds, laid.bounds, fit, lens);
    let ink = Ink {
        look: &look,
        view,
        sans: look.font_stack(token::FONT_SANS).into_iter().next().unwrap_or_else(|| "IBM Plex Sans".into()),
        mono: look.font_stack(token::FONT_MONO).into_iter().next().unwrap_or_else(|| "IBM Plex Mono".into()),
    };
    let now = playing(&laid, clock.steps);
    // Electron's flow crawl: a reading view's flows move while no step plays; a preview stays still.
    let crawl = match clock.flows {
        Some(since) if fit == Fit::Reading && now.is_none() => Some(since.elapsed().as_secs_f32() * 1000. % STEP_MS / STEP_MS * DASH * 2.),
        _ => None,
    };
    let lit: HashSet<&str> = match &now {
        Some(now) => scene
            .nodes
            .iter()
            .filter(|node| node.step == Some(now.at))
            .map(|node| node.id.as_str())
            .chain(scene.edges.iter().filter(|edge| edge.step == Some(now.at)).map(|edge| edge.to.as_str()))
            .collect(),
        None => HashSet::new(),
    };
    window.with_content_mask(Some(ContentMask { bounds }), |window| {
        for (node, rect) in scene.nodes.iter().zip(&laid.rects).filter(|(node, _)| ground(node)) {
            paint_ground(&ink, node, *rect, window, cx);
        }
        for (edge, way) in scene.edges.iter().zip(&laid.ways) {
            let Some(way) = way else { continue };
            let active = now.as_ref().is_some_and(|now| edge.step == Some(now.at));
            let dim = now.is_some() && !active;
            let flow = (edge.flow != Some(false)).then_some(crawl.unwrap_or(0.));
            paint_edge(&ink, way, edge.color, flow, active, dim, window, cx);
        }
        for (node, rect) in scene.nodes.iter().zip(&laid.rects).filter(|(node, _)| !ground(node)) {
            let dim = now.is_some() && !lit.contains(node.id.as_str());
            paint_node(&ink, node, *rect, if dim { DIMMED } else { 1. }, window, cx);
            if now.is_some() && !dim {
                let radius = match shaped(node) {
                    Some(SceneShape::Pill) => rect.h / 2.,
                    Some(SceneShape::Circle) => rect.w / 2.,
                    _ => RADIUS,
                };
                let ring = Rect { x: rect.x - 2., y: rect.y - 2., w: rect.w + 4., h: rect.h + 4. };
                window.paint_quad(quad(view.rect(ring), view.len(radius + 2.), gpui::transparent_black(), view.len(2.), look.color(token::EDGE_FOCUS), BorderStyle::Solid));
            }
        }
        for stroke in &scene.strokes {
            paint_ink(&ink, &stroke.points.iter().map(|p| Pt { x: p.x as f32, y: p.y as f32 }).collect::<Vec<_>>(), window);
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

/// A group or a lane: a dashed outline in its hue with its title in a band at the top left.
fn paint_ground(ink: &Ink, node: &SceneNode, rect: Rect, window: &mut Window, cx: &mut App) {
    let (look, view) = (ink.look, ink.view);
    let hue = hue(look, node.color, SceneColor::Blue);
    let lane = node.kind == SceneKind::Lane;
    let edges = if lane { Edges { top: view.len(BORDER), bottom: view.len(BORDER), left: px(0.), right: px(0.) } } else { Edges::all(view.len(BORDER)) };
    let radius = if lane { px(0.) } else { view.len(RADIUS) };
    window.paint_quad(quad(view.rect(rect), radius, gpui::transparent_black(), edges, hue, BorderStyle::Dashed));
    let Some(title) = node.title.as_deref() else { return };
    let measure = Measure { window, sans: ink.sans.clone(), mono: ink.mono.clone() };
    let w = measure.width(title, TEXT, 600., false) + 2. * PAD_X;
    let inset = if lane { 0. } else { BORDER };
    let band = Rect { x: rect.x + inset, y: rect.y + BORDER, w, h: LEAD + 2. * PAD_Y };
    let corners = Corners { top_left: if lane { px(0.) } else { view.len(RADIUS - BORDER) }, top_right: px(0.), bottom_right: view.len(RADIUS), bottom_left: px(0.) };
    window.paint_quad(quad(view.rect(band), corners, hue, px(0.), gpui::transparent_black(), BorderStyle::Solid));
    ink.words(title, Pt { x: band.x + PAD_X, y: band.y + PAD_Y }, None, TEXT, 600., false, look.color(token::FG_INVERSE), TextAlign::Left, window, cx);
}

fn paint_node(ink: &Ink, node: &SceneNode, rect: Rect, alpha: f32, window: &mut Window, cx: &mut App) {
    let (look, view) = (ink.look, ink.view);
    let faded = |color: Hsla| color.opacity(color.a * alpha);
    match node.kind {
        // A mock comes with the app's own page view (#20), a screen with its own walk: their place is kept, nothing drawn.
        SceneKind::Mock | SceneKind::Screen | SceneKind::Group | SceneKind::Lane => {}
        SceneKind::Label => {
            let words = node.title.as_deref().or(node.body.as_deref()).unwrap_or("");
            ink.words(words, Pt { x: rect.x, y: rect.y }, None, 14., 600., false, faded(look.color(token::FG_MUTED)), TextAlign::Left, window, cx);
        }
        SceneKind::Wire => paint_wire(ink, node, rect, alpha, window, cx),
        SceneKind::Box if shaped(node).is_some() => paint_shaped(ink, node, rect, alpha, window, cx),
        SceneKind::Box | SceneKind::Code => {
            let mono = node.kind == SceneKind::Code;
            let hue = hue(look, node.color, if mono { SceneColor::Grey } else { SceneColor::Blue });
            window.paint_quad(quad(view.rect(rect), view.len(RADIUS), faded(tint(look, hue)), view.len(BORDER), faded(hue), BorderStyle::Solid));
            let inner = Rect { x: rect.x + BORDER, y: rect.y + BORDER, w: rect.w - 2. * BORDER, h: rect.h - 2. * BORDER };
            let mut top = inner.y;
            if node.title.is_some() || node.icon.is_some() {
                let measure = Measure { window, sans: ink.sans.clone(), mono: ink.mono.clone() };
                let lang_w = node.lang.as_deref().map_or(0., |lang| measure.width(lang, TEXT, 600., false));
                let icon_w = if node.icon.is_some() { ICON + 8. } else { 0. };
                let wrap = (inner.w - 2. * PAD_X - icon_w - if lang_w > 0. { lang_w + 8. } else { 0. }).max(1.);
                let lines = measure.lines(node.title.as_deref().unwrap_or(""), TEXT, 600., false, wrap);
                let band = Rect { x: inner.x, y: inner.y, w: inner.w, h: 2. * PAD_Y + LEAD * lines as f32 };
                let corners = Corners { top_left: view.len(RADIUS - BORDER), top_right: view.len(RADIUS - BORDER), bottom_right: px(0.), bottom_left: px(0.) };
                window.paint_quad(quad(view.rect(band), corners, faded(hue), px(0.), gpui::transparent_black(), BorderStyle::Solid));
                let fg = faded(look.color(token::FG_INVERSE));
                let mut x = band.x + PAD_X;
                if let Some(icon) = node.icon.as_deref() {
                    ink.icon(icon, Pt { x, y: band.y + PAD_Y + (LEAD - ICON) / 2. }, fg, window, cx);
                    x += icon_w;
                }
                if let Some(title) = node.title.as_deref() {
                    ink.words(title, Pt { x, y: band.y + PAD_Y }, Some(wrap), TEXT, 600., false, fg, TextAlign::Left, window, cx);
                }
                if let Some(lang) = node.lang.as_deref() {
                    ink.words(lang, Pt { x: band.x + band.w - PAD_X - lang_w, y: band.y + PAD_Y }, None, TEXT, 600., false, fg.opacity(fg.a * 0.8), TextAlign::Left, window, cx);
                }
                top += band.h;
            }
            if let Some(body) = node.body.as_deref() {
                ink.words(body, Pt { x: inner.x + PAD_X, y: top + PAD_Y }, Some(inner.w - 2. * PAD_X), TEXT, 400., mono, faded(look.color(token::FG_DEFAULT)), TextAlign::Left, window, cx);
            }
        }
    }
}

/// A box in an outline of its own: a pill or a circle are the frame's own corners; a diamond, a
/// hexagon and a cylinder are drawn in a hundred-unit square stretched over the box, 2px inside it.
fn paint_shaped(ink: &Ink, node: &SceneNode, rect: Rect, alpha: f32, window: &mut Window, cx: &mut App) {
    let (look, view) = (ink.look, ink.view);
    let faded = |color: Hsla| color.opacity(color.a * alpha);
    let shape = shaped(node).unwrap_or(SceneShape::Box);
    let hue = faded(hue(look, node.color, SceneColor::Blue));
    let fill = faded(tint(look, hue.opacity(1.)));
    match shape {
        SceneShape::Pill | SceneShape::Circle => {
            let radius = if shape == SceneShape::Pill { rect.h / 2. } else { rect.w / 2. };
            window.paint_quad(quad(view.rect(rect), view.len(radius), fill, view.len(BORDER), hue, BorderStyle::Solid));
        }
        _ => {
            let inner = Rect { x: rect.x + BORDER, y: rect.y + BORDER, w: rect.w - 2. * BORDER, h: rect.h - 2. * BORDER };
            let at = |x: f32, y: f32| view.at(Pt { x: inner.x + inner.w * x / 100., y: inner.y + inner.h * y / 100. });
            let outline = |builder: &mut PathBuilder| match shape {
                SceneShape::Diamond => builder.add_polygon(&[at(50., 0.), at(100., 50.), at(50., 100.), at(0., 50.)], true),
                SceneShape::Hexagon => builder.add_polygon(&[at(12., 0.), at(88., 0.), at(100., 50.), at(88., 100.), at(12., 100.), at(0., 50.)], true),
                _ => {
                    let radii = point(view.len(inner.w / 2.), view.len(inner.h * 0.1));
                    builder.move_to(at(0., 10.));
                    builder.arc_to(radii, px(0.), false, true, at(100., 10.));
                    builder.line_to(at(100., 90.));
                    builder.arc_to(radii, px(0.), false, true, at(0., 90.));
                    builder.close();
                }
            };
            let mut body = PathBuilder::fill();
            outline(&mut body);
            if let Ok(path) = body.build() {
                window.paint_path(path, fill);
            }
            let mut edge = PathBuilder::stroke(px(BORDER));
            outline(&mut edge);
            if shape == SceneShape::Cylinder {
                let radii = point(view.len(inner.w / 2.), view.len(inner.h * 0.1));
                edge.move_to(at(0., 10.));
                edge.arc_to(radii, px(0.), false, false, at(100., 10.));
            }
            if let Ok(path) = edge.build() {
                window.paint_path(path, hue);
            }
        }
    }
    let (inline, top, bottom) = shaped_padding(shape, rect.w);
    let wrap = (rect.w - 2. * BORDER - 2. * inline).max(1.);
    let measure = Measure { window, sans: ink.sans.clone(), mono: ink.mono.clone() };
    let named = node.icon.is_some() || node.title.is_some();
    let lines = node.body.as_deref().map_or(0, |body| measure.lines(body, TEXT, 400., false, wrap));
    let content = if named { LEAD } else { 0. } + if named && lines > 0 { 4. } else { 0. } + lines as f32 * LEAD;
    let title_w = node.title.as_deref().map_or(0., |title| measure.width(title, TEXT, 600., false));
    // The column stands in the middle of the space inside the padding.
    let room = rect.h - 2. * BORDER - top - bottom;
    let mut y = rect.y + BORDER + top + (room - content).max(0.) / 2.;
    let fg = faded(look.color(token::FG_DEFAULT));
    let middle = rect.x + rect.w / 2.;
    if named {
        let name_w = if node.icon.is_some() { ICON + 8. } else { 0. } + title_w;
        let mut x = middle - name_w / 2.;
        if let Some(icon) = node.icon.as_deref() {
            ink.icon(icon, Pt { x, y: y + (LEAD - ICON) / 2. }, fg, window, cx);
            x += ICON + 8.;
        }
        if let Some(title) = node.title.as_deref() {
            ink.words(title, Pt { x, y }, None, TEXT, 600., false, fg, TextAlign::Left, window, cx);
        }
        y += LEAD + if lines > 0 { 4. } else { 0. };
    }
    if let Some(body) = node.body.as_deref() {
        ink.words(body, Pt { x: middle - wrap / 2., y }, Some(wrap), TEXT, 400., false, fg, TextAlign::Center, window, cx);
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

/// An arrow along its way: 2px in its hue, dashed where it is a flow, a filled head at its end, and its words on a sunken backing.
/// A flow carries its dashes' phase: how far along the way they have crawled.
#[allow(clippy::too_many_arguments)]
fn paint_edge(ink: &Ink, way: &Way, color: Option<SceneColor>, flow: Option<f32>, active: bool, dim: bool, window: &mut Window, cx: &mut App) {
    let (look, view) = (ink.look, ink.view);
    let base = if active { look.color(token::EDGE_FOCUS) } else { color.map_or(look.color(token::FG_MUTED), |color| hue(look, Some(color), SceneColor::Grey)) };
    let stroke = if dim { base.opacity(base.a * DIMMED) } else { base };
    let mut line = PathBuilder::stroke(view.len(2.));
    let mut last = Pt::default();
    let mut before = Pt::default();
    for seg in &way.path {
        match *seg {
            Seg::Move(p) => {
                last = p;
                before = p;
            }
            Seg::Line(p) => {
                before = last;
                last = p;
            }
            Seg::Quad(ctrl, p) => {
                before = ctrl;
                last = p;
            }
        }
    }
    match flow {
        Some(phase) => dashes(&mut line, &flat(&way.path), phase, view),
        None => {
            for seg in &way.path {
                match *seg {
                    Seg::Move(p) => line.move_to(view.at(p)),
                    Seg::Line(p) => line.line_to(view.at(p)),
                    Seg::Quad(ctrl, p) => line.curve_to(view.at(p), view.at(ctrl)),
                }
            }
        }
    }
    if let Ok(path) = line.build() {
        window.paint_path(path, stroke);
    }
    // The marker's 10-unit triangle drawn 16px long, its point a tenth past the way's end.
    let (dx, dy) = (last.x - before.x, last.y - before.y);
    let long = dx.hypot(dy);
    if long > 0. {
        let (ux, uy) = (dx / long, dy / long);
        let tip = Pt { x: last.x + ux * 1.6, y: last.y + uy * 1.6 };
        let back = Pt { x: tip.x - ux * 16., y: tip.y - uy * 16. };
        let mut head = PathBuilder::fill();
        head.add_polygon(&[view.at(tip), view.at(Pt { x: back.x - uy * 8., y: back.y + ux * 8. }), view.at(Pt { x: back.x + uy * 8., y: back.y - ux * 8. })], true);
        if let Ok(path) = head.build() {
            window.paint_path(path, stroke);
        }
    }
    if let Some((rect, lines)) = &way.words {
        window.paint_quad(quad(view.rect(*rect), view.len(3.), look.color(token::BG_SUNKEN), px(0.), gpui::transparent_black(), BorderStyle::Solid));
        let muted = look.color(token::FG_MUTED);
        let muted = if dim { muted.opacity(muted.a * DIMMED) } else { muted };
        for (at, text) in lines.iter().enumerate() {
            ink.words(text, Pt { x: rect.x, y: rect.y + 4. + at as f32 * LEAD }, Some(rect.w), TEXT, 400., false, muted, TextAlign::Center, window, cx);
        }
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

/// The way as a line through points, its rounded corners cut into short chords.
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
        }
    }
    points
}

/// `stroke-dasharray: DASH DASH` along the points, `phase` further on than where it starts, as a
/// falling `stroke-dashoffset` moves it.
fn dashes(line: &mut PathBuilder, points: &[Pt], phase: f32, view: View) {
    let period = DASH * 2.;
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
            let (on, left) = if into < DASH { (true, DASH - into) } else { (false, period - into) };
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
    let points: Vec<Pt> = path.iter().map(|seg| match *seg {
        Seg::Move(p) | Seg::Line(p) | Seg::Quad(_, p) => p,
    }).collect();
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
