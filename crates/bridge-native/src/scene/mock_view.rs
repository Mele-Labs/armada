//! A sketch's `mock` node drawn as the picture of its sealed page (`mock_page`), at the node's width, as tall as
//! its content, under its title. The page is drawn by a web view the app lends through a [`Painter`], one picture
//! at a time and never more than two at once; a node whose picture is not there yet, or could not be made, keeps a
//! plain frame at its floor.
//!
//! **The page is laid out at the node's own width in scene units**, as Classic's frame is, and only the picture's
//! density follows the zoom: its scale is the window's times the scene zoom stepped to a power of 1.25, so a
//! resize or a wheel turn re-renders once per step, not once per frame. The content's height is the same at every
//! scale, so the scene lays out again once, when the first picture of a page arrives.

use std::cell::Cell;
use std::collections::{hash_map::DefaultHasher, HashMap, HashSet, VecDeque};
use std::hash::{Hash, Hasher};
use std::rc::Rc;
use std::sync::Arc;

use gpui::{point, px, quad, size, App, BorderStyle, Bounds, Corners, Global, Hsla, RenderImage, SharedString, Task, TextAlign, Window};
use ipc::{KeptStyles, SceneKind, SceneNode};

pub use crate::web::PagePart;

use super::geometry::{Pt, Rect};
use super::mock_page::{page, under_app_tokens, KeptCache};
use super::{sheet_top, Ink};
use crate::look::{token, Look, Tone};

/// How many pictures are being made at once, across every scene in the app.
const IN_FLIGHT: usize = 2;
/// The step between the scales a page is drawn at.
const STEP: f32 = 1.25;
/// The densest a picture is made: four pixels to a point.
const DENSEST: f32 = 4.;
/// The thinnest the web view makes a picture (`render_html` clamps there too).
const THINNEST: f32 = 0.5;
/// The scales kept for a page: a zoom that moved on leaves the older ones to draw from until the new one is there.
const SCALES_KEPT: usize = 3;
/// The pages kept across the app, the least lately drawn let go past it. Above the most a screen draws at once, or the
/// pages on screen would push each other out and be made again without end.
const PAGES_KEPT: usize = 32;

/// The page's appearance, as the web view's `prefers-color-scheme` answers it. The same two words as `web::render`'s.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Appearance {
    Light,
    Dark,
}

/// What the web view answers: the picture, `width * scale` pixels across, the content's height in points, and the parts
/// a note may name, where the page laid them out.
pub type Picture = (Arc<RenderImage>, f32, Arc<Vec<PagePart>>);

/// Draws `html` `width` points across at `scale` pixels to the point. **Called on the main thread**, where a web view lives.
pub type Painter = Rc<dyn Fn(String, f32, f32, Appearance, &mut App) -> Task<Result<Picture, String>>>;

/// A mock node's own words: what its picture is made from, hashed once.
pub struct Mock {
    hash: u64,
    html: String,
    css: Option<String>,
    kept: Option<String>,
}

/// A mock node's source, or `None` for any other node.
fn source(node: &SceneNode) -> Option<Rc<Mock>> {
    if node.kind != SceneKind::Mock {
        return None;
    }
    let html = node.html.clone()?;
    Some(Rc::new(Mock { hash: hash_of(&html, &node.css, &node.kept), html, css: node.css.clone(), kept: node.kept.clone() }))
}

fn hash_of(html: &str, css: &Option<String>, kept: &Option<String>) -> u64 {
    let mut hasher = DefaultHasher::new();
    (html, css, kept).hash(&mut hasher);
    hasher.finish()
}

/// How tall a mock node's page stands, once a picture of it has come: the content's height at the node's width, in
/// scene units. For a place that sizes a drawing from its nodes without laying it out.
pub(crate) fn content_height(node: &SceneNode, dark: bool, cx: &App) -> Option<f32> {
    if node.kind != SceneKind::Mock {
        return None;
    }
    let key = PageKey { hash: hash_of(node.html.as_deref()?, &node.css, &node.kept), width: (node.w? as f32).round().max(1.) as u32, dark };
    cx.try_global::<Pictures>()?.pages.get(&key).map(|drawn| drawn.height)
}

thread_local! {
    /// Each scene's mock sources by its key, one per node, read once.
    static SOURCES: std::cell::RefCell<HashMap<SharedString, Rc<Vec<Option<Rc<Mock>>>>>> = std::cell::RefCell::default();
}

pub(super) fn sources(key: &SharedString, scene: &ipc::Scene) -> Rc<Vec<Option<Rc<Mock>>>> {
    if let Some(held) = SOURCES.with(|held| held.borrow().get(key).cloned()) {
        return held;
    }
    let read = Rc::new(scene.nodes.iter().map(source).collect::<Vec<_>>());
    SOURCES.with(|held| held.borrow_mut().insert(key.clone(), read.clone()));
    read
}

/// What fixes how a page lays out: the words, the width in scene units and the theme.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PageKey {
    hash: u64,
    width: u32,
    dark: bool,
}

/// One picture: the page and the scale it is made at, in thousandths.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Key {
    page: PageKey,
    scale: u32,
}

fn page_key(mock: &Mock, width: f32, dark: bool) -> PageKey {
    PageKey { hash: mock.hash, width: width.round().max(1.) as u32, dark }
}

/// The scale to make a picture at: the window's, times the zoom stepped to a power of 1.25.
fn scale_for(window_scale: f32, zoom: f32) -> f32 {
    let steps = (zoom.max(0.001).ln() / STEP.ln()).round().clamp(-12., 12.);
    (window_scale * STEP.powf(steps)).clamp(THINNEST, DENSEST)
}

fn key_of(page: PageKey, scale: f32) -> Key {
    Key { page, scale: (scale * 1000.).round() as u32 }
}

/// Which pictures are waiting and which are being made, so a thread of twelve mocks opens two web views and not twelve.
#[derive(Default)]
struct Queue {
    asked: HashSet<Key>,
    waiting: VecDeque<Key>,
    in_flight: usize,
}

impl Queue {
    /// Asks for a picture. Answers false where it is already waiting or being made.
    fn ask(&mut self, key: Key) -> bool {
        if !self.asked.insert(key) {
            return false;
        }
        self.waiting.push_back(key);
        true
    }

    /// The next picture to start, while fewer than `IN_FLIGHT` are being made.
    fn next(&mut self) -> Option<Key> {
        if self.in_flight >= IN_FLIGHT {
            return None;
        }
        let key = self.waiting.pop_front()?;
        self.in_flight += 1;
        Some(key)
    }

    fn done(&mut self, key: Key) {
        self.asked.remove(&key);
        self.in_flight = self.in_flight.saturating_sub(1);
    }
}

/// What has been drawn of a page: its content's height and the pictures, oldest first.
struct Drawn {
    height: f32,
    parts: Arc<Vec<PagePart>>,
    images: Vec<(u32, Arc<RenderImage>)>,
    /// When it was last drawn, on `Pictures::tick`.
    used: Cell<u64>,
}

/// The app's pictures of mock pages, for the run.
struct Pictures {
    painter: Option<Painter>,
    /// Where Fleet answers, for the style sets a node names. `None` reads the mock Fleet's.
    port: Option<Rc<dyn Fn(&App) -> Option<u16>>>,
    kept: Arc<KeptCache>,
    pages: HashMap<PageKey, Drawn>,
    /// Counts draws and arrivals, so the page least lately drawn is the one let go.
    tick: Cell<u64>,
    /// A page the web view could not draw keeps its plain frame; it is asked again only when a painter is set.
    failed: HashSet<PageKey>,
    queue: Queue,
    /// What each key waiting is made from.
    wants: HashMap<Key, Rc<Mock>>,
}

impl Default for Pictures {
    fn default() -> Self {
        Pictures { painter: None, port: None, kept: Arc::default(), pages: HashMap::new(), tick: Cell::new(0), failed: HashSet::new(), queue: Queue::default(), wants: HashMap::new() }
    }
}

impl Global for Pictures {}

/// The web view the app draws pages with. Set once; pages that failed for want of one are asked again.
pub fn set_painter(painter: Painter, cx: &mut App) {
    let pictures = cx.default_global::<Pictures>();
    pictures.painter = Some(painter);
    pictures.failed.clear();
    cx.refresh_windows();
}

/// The live app: pages are drawn with Fleet's style sets, read on the port the connection holds now.
pub fn hold_live(fleet: &gpui::Entity<crate::fleet::FleetConnection>, cx: &mut App) {
    let fleet = fleet.clone();
    cx.default_global::<Pictures>().port = Some(Rc::new(move |cx: &App| fleet.read(cx).held().port()));
    set_painter(
        Rc::new(|html, width, scale, appearance, cx: &mut App| {
            let appearance = match appearance {
                Appearance::Light => crate::web::Appearance::Light,
                Appearance::Dark => crate::web::Appearance::Dark,
            };
            let drawn = crate::web::render_html(html, width, scale, appearance, cx);
            cx.foreground_executor().spawn(async move { drawn.await.map(|rendered| (rendered.image, rendered.height, Arc::new(rendered.parts))) })
        }),
        cx,
    );
}

/// The web view the app draws pages with, for a page drawn outside a scene, as a card's first screen is.
pub(crate) fn painter(cx: &App) -> Option<Painter> {
    cx.try_global::<Pictures>()?.painter.clone()
}

/// How tall a page's content stands at this width, once any picture of it has come.
pub(super) fn height(mock: &Mock, width: f32, dark: bool, cx: &App) -> Option<f32> {
    cx.try_global::<Pictures>()?.pages.get(&page_key(mock, width, dark)).map(|drawn| drawn.height)
}

/// What a page holds that a note may name, once a picture of it has come.
pub(super) fn parts(mock: &Mock, width: f32, dark: bool, cx: &App) -> Option<Arc<Vec<PagePart>>> {
    cx.try_global::<Pictures>()?.pages.get(&page_key(mock, width, dark)).map(|drawn| drawn.parts.clone())
}

/// The picture to draw now, asking for the one the zoom wants. Until it comes, the nearest scale already made.
pub(super) fn picture(mock: &Rc<Mock>, width: f32, zoom: f32, dark: bool, window: &Window, cx: &mut App) -> Option<Arc<RenderImage>> {
    let key = key_of(page_key(mock, width, dark), scale_for(window.scale_factor(), zoom));
    ask(key, mock, cx);
    let pictures = cx.global::<Pictures>();
    let drawn = pictures.pages.get(&key.page)?;
    pictures.tick.set(pictures.tick.get() + 1);
    drawn.used.set(pictures.tick.get());
    let near = |scale: u32| (f64::from(scale) / f64::from(key.scale)).ln().abs();
    drawn.images.iter().min_by(|a, b| near(a.0).total_cmp(&near(b.0))).map(|(_, image)| image.clone())
}

fn ask(key: Key, mock: &Rc<Mock>, cx: &mut App) {
    let pictures = cx.default_global::<Pictures>();
    let held = pictures.pages.get(&key.page).is_some_and(|drawn| drawn.images.iter().any(|(scale, _)| *scale == key.scale));
    if held || pictures.failed.contains(&key.page) || !pictures.queue.ask(key) {
        return;
    }
    pictures.wants.insert(key, mock.clone());
    pump(cx);
}

/// Starts waiting pictures while fewer than two are being made.
fn pump(cx: &mut App) {
    loop {
        let pictures = cx.default_global::<Pictures>();
        let Some(key) = pictures.queue.next() else { return };
        let Some(mock) = pictures.wants.remove(&key) else {
            pictures.queue.done(key);
            continue;
        };
        let painter = pictures.painter.clone();
        let kept = pictures.kept.clone();
        let port = pictures.port.clone();
        cx.spawn(async move |cx| {
            let made = make(key, mock, painter, kept, port, cx).await;
            cx.update(|cx| finish(key, made, cx));
        })
        .detach();
    }
}

async fn make(key: Key, mock: Rc<Mock>, painter: Option<Painter>, kept: Arc<KeptCache>, port: Option<Rc<dyn Fn(&App) -> Option<u16>>>, cx: &mut gpui::AsyncApp) -> Result<Picture, String> {
    let painter = painter.unwrap_or_else(|| Rc::new(|_, _, _, _, _| Task::ready(Err("no web view yet".to_string()))));
    // The style set Fleet kept for the node. A node whose set cannot be read is drawn with its own rules alone, as Classic draws it.
    let styles: Option<Arc<KeptStyles>> = match mock.kept.clone() {
        None => None,
        Some(hash) => match kept.held(&hash) {
            Some(held) => Some(held),
            None => {
                let port = cx.update(|cx| port.as_ref().map(|port| port(cx)));
                let read = match port {
                    // The live app, not connected: nothing to ask.
                    Some(None) => Err("Fleet is not connected".to_string()),
                    Some(Some(port)) => cx.background_executor().spawn({
                        let (kept, hash) = (kept.clone(), hash.clone());
                        async move { kept.read(Some(port), &hash) }
                    })
                    .await,
                    None => cx.background_executor().spawn({
                        let (kept, hash) = (kept.clone(), hash.clone());
                        async move { kept.read(None, &hash) }
                    })
                    .await,
                };
                match read {
                    Ok(read) => Some(read),
                    Err(why) => {
                        crate::log::Note::warn("a mock's style set was not read").field("why", why).emit();
                        None
                    }
                }
            }
        },
    };
    let dark = key.page.dark;
    let tokens = cx.update(|cx| super::export::declarations(&cx.try_global::<Look>().cloned().unwrap_or_else(Look::dark)));
    let styles = under_app_tokens(styles.as_deref(), &tokens);
    let html = page(&mock.html, mock.css.as_deref(), Some(&styles), dark);
    let (width, scale) = (key.page.width as f32, key.scale as f32 / 1000.);
    let appearance = if dark { Appearance::Dark } else { Appearance::Light };
    let task = cx.update(|cx| painter(html, width, scale, appearance, cx));
    task.await
}

fn finish(key: Key, made: Result<Picture, String>, cx: &mut App) {
    let pictures = cx.default_global::<Pictures>();
    pictures.queue.done(key);
    match made {
        Ok((image, height, parts)) => {
            let drawn = pictures.pages.entry(key.page).or_insert_with(|| Drawn { height, parts: parts.clone(), images: Vec::new(), used: Cell::new(0) });
            pictures.tick.set(pictures.tick.get() + 1);
            drawn.used.set(pictures.tick.get());
            drawn.height = height;
            drawn.parts = parts;
            let (mut gone, kept): (Vec<_>, Vec<_>) = std::mem::take(&mut drawn.images).into_iter().partition(|(scale, _)| *scale == key.scale);
            drawn.images = kept;
            drawn.images.push((key.scale, image));
            if drawn.images.len() > SCALES_KEPT {
                gone.push(drawn.images.remove(0));
            }
            let mut gone: Vec<Arc<RenderImage>> = gone.into_iter().map(|(_, image)| image).collect();
            if pictures.pages.len() > PAGES_KEPT {
                let oldest = pictures.pages.iter().filter(|(page, _)| **page != key.page).min_by_key(|(_, drawn)| drawn.used.get()).map(|(page, _)| *page);
                if let Some(drawn) = oldest.and_then(|page| pictures.pages.remove(&page)) {
                    gone.extend(drawn.images.into_iter().map(|(_, image)| image));
                }
            }
            for image in gone {
                crate::web::let_go(image, cx);
            }
        }
        Err(why) => {
            if pictures.failed.insert(key.page) {
                crate::log::Note::warn("a mock was not drawn").field("why", why).emit();
            }
        }
    }
    pump(cx);
    // Folded once per arrival: every window draws again and the scenes lay out with the new height.
    cx.refresh_windows();
}

/// A mock node as the canvas holds it for one frame: its picture, where one has come.
pub(super) struct Shown {
    pub(super) image: Option<Arc<RenderImage>>,
}

/// Where a mock stands in the scene: the title above, the frame under it.
fn frame_of(rect: Rect, node: &SceneNode) -> Rect {
    let top = sheet_top(node);
    Rect { x: rect.x, y: rect.y + top, w: rect.w, h: rect.h - top }
}

/// Draws a mock: its title above, then its picture at the node's width in the theme's medium radius, or the plain
/// frame where there is none. A change against an earlier sketch is outlined round the frame in its hue and a step
/// that lights it rings it. **A mock takes no pointer**: nothing here listens.
pub(super) fn paint(ink: &Ink, node: &SceneNode, rect: Rect, shown: &Shown, alpha: f32, lit: bool, window: &mut Window, cx: &mut App) {
    let (look, view) = (ink.look, ink.view);
    if let Some(title) = node.title.as_deref() {
        let muted = look.color(token::FG_MUTED);
        let at = f32::from(look.length(token::TEXT_SM));
        ink.words(title, Pt { x: rect.x, y: rect.y }, Some(rect.w), at, look.number(token::WEIGHT_HEADING), false, muted.opacity(muted.a * alpha), TextAlign::Left, window, cx);
    }
    let frame = frame_of(rect, node);
    let bounds: Bounds<gpui::Pixels> = view.rect(frame);
    let radius = view.len(super::tile::RADIUS);
    match &shown.image {
        Some(image) => {
            let _ = window.paint_image(bounds, bounds, Corners::all(radius), image.clone(), 0, false);
            if alpha < 1. {
                // A picture cannot be drawn faint, so what stands back is veiled in the ground it stands on.
                let veil: Hsla = look.color(token::BG_SUNKEN).opacity(1. - alpha);
                window.paint_quad(quad(bounds, radius, veil, px(0.), gpui::transparent_black(), BorderStyle::Solid));
            }
        }
        None => {
            let fill = look.color(token::BG_SUNKEN);
            let edge = look.color(token::BORDER_SUBTLE);
            window.paint_quad(quad(bounds, radius, fill.opacity(fill.a * alpha), view.len(f32::from(look.length(token::BORDER_WIDTH))), edge.opacity(edge.a * alpha), BorderStyle::Solid));
        }
    }
    let outline = |window: &mut Window, colour: Hsla| {
        let out = Rect { x: frame.x - 2., y: frame.y - 2., w: frame.w + 4., h: frame.h + 4. };
        let outline = Bounds::new(point(view.rect(out).origin.x, view.rect(out).origin.y), size(view.len(out.w), view.len(out.h)));
        window.paint_quad(quad(outline, radius + view.len(2.), gpui::transparent_black(), view.len(2.), colour, BorderStyle::Solid));
    };
    if let Some(hue) = ink.over {
        outline(window, hue);
    }
    if lit {
        outline(window, look.color(token::EDGE_FOCUS));
    }
}

/// Whether the theme is dark: a mock's page switches on it.
pub(super) fn dark(look: &crate::look::Look) -> bool {
    look.tone() == Tone::Dark
}

/// The markup between `<body>` and `</body>` of a page `page` built, which is what a part is read from.
fn html_body(page: &str) -> String {
    let start = page.find("<body>").map_or(0, |at| at + "<body>".len());
    let end = page.rfind("</body>").unwrap_or(page.len());
    page.get(start..end.max(start)).unwrap_or_default().to_string()
}

/// A walk's stand-in for a whole page's first screen, which no web view lays out there: a heading, a line under it, one
/// card for each part the page reports and the page's buttons, drawn `width` points across at `scale` pixels to the point,
/// so a well that fits it never magnifies it. Answers the picture and its height in points.
fn page_stand_in(width: f32, scale: f32, dark: bool, blocks: usize) -> (Arc<RenderImage>, f32) {
    let height = (width * 0.7).round();
    let (across, down) = ((width * scale).round().max(1.) as u32, (height * scale).round().max(1.) as u32);
    let (ground, card, ink, quiet, accent) = if dark {
        ([14u8, 17, 23], [24u8, 29, 38], [226u8, 231, 238], [96u8, 106, 120], [74u8, 144, 226])
    } else {
        ([250, 251, 252], [255, 255, 255], [28, 33, 40], [170, 178, 190], [37, 99, 235])
    };
    let mut image = image::RgbaImage::from_pixel(across, down, image::Rgba([ground[0], ground[1], ground[2], 255]));
    let mut fill = |x: f32, y: f32, w: f32, h: f32, rgb: [u8; 3]| {
        let (x0, y0) = ((x * scale) as u32, (y * scale) as u32);
        let (x1, y1) = ((((x + w) * scale) as u32).min(across), (((y + h) * scale) as u32).min(down));
        for py in y0..y1 {
            for px in x0..x1 {
                image.put_pixel(px, py, image::Rgba([rgb[0], rgb[1], rgb[2], 255]));
            }
        }
    };
    let margin = width * 0.05;
    let wide = width - margin * 2.;
    fill(margin, margin, wide * 0.34, 22., ink);
    fill(margin, margin + 34., wide * 0.58, 8., quiet);
    let rows = blocks.clamp(2, 4);
    let top = margin + 64.;
    let room = (height - top - margin - 36.).max(40.);
    let each = (room / rows as f32).min(86.);
    for row in 0..rows {
        let y = top + each * row as f32;
        fill(margin, y, wide, each - 10., card);
        fill(margin + 16., y + 14., wide * 0.22, 9., ink);
        fill(margin + 16., y + 32., wide * 0.6, 7., quiet);
        if each > 64. {
            fill(margin + 16., y + 46., wide * 0.45, 7., quiet);
        }
    }
    let y = (top + each * rows as f32 + 2.).min(height - margin - 24.);
    for chip in 0..3 {
        fill(margin + (wide * 0.14 + 12.) * chip as f32, y, wide * 0.14, 24., accent);
    }
    for pixel in image.pixels_mut() {
        pixel.0.swap(0, 2);
    }
    (Arc::new(RenderImage::new(vec![image::Frame::new(image)])), height)
}

/// A walk's web view: every page is the fixture picture, as tall as its shape gives at the width asked.
pub fn hold_walk(cx: &mut App) {
    let fixture = include_bytes!("../../fixtures/mock-chip.png");
    let Ok(decoded) = image::load_from_memory_with_format(fixture, image::ImageFormat::Png) else { return };
    let rgba = decoded.into_rgba8();
    let (across, down) = (rgba.width().max(1) as f32, rgba.height() as f32);
    let image = {
        let mut bgra = rgba;
        for pixel in bgra.pixels_mut() {
            pixel.0.swap(0, 2);
        }
        Arc::new(RenderImage::new(vec![image::Frame::new(bgra)]))
    };
    let painter: Painter = Rc::new(move |html, width, scale, appearance, cx: &mut App| {
        let _ = cx;
        // A whole page, as a card's first screen is, has a picture of a page's size; a node's words a small one.
        let page = html.trim_start().to_ascii_lowercase().starts_with("<!doctype");
        let outline = super::mock_page::outline(&html_body(&html));
        let (image, height) = if page { page_stand_in(width, scale, appearance == Appearance::Dark, outline.len()) } else { (image.clone(), (width * down / across).round()) };
        // No web view lays the page out, so its parts stand one under another, each a share of the picture's height.
        let share = height / outline.len().max(1) as f32;
        let parts = outline.into_iter().enumerate().map(|(at, one)| PagePart { key: one.key, label: one.label, x: 0., y: share * at as f32, w: width, h: share, depth: one.depth }).collect();
        Task::ready(Ok((image.clone(), height, Arc::new(parts))))
    });
    set_painter(painter, cx);
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::TestAppContext;
    use std::cell::{Cell, RefCell};

    fn mock(html: &str) -> Rc<Mock> {
        source(&SceneNode {
            id: "m".into(),
            kind: SceneKind::Mock,
            x: 0.,
            y: 0.,
            w: Some(80.),
            h: None,
            title: None,
            body: None,
            icon: None,
            lang: None,
            wire: None,
            step: None,
            shape: None,
            color: None,
            state: None,
            html: Some(html.into()),
            css: None,
            styles: None,
            kept: None,
            screen: None,
        })
        .expect("a mock node")
    }

    #[test]
    fn a_picture_is_keyed_by_words_width_theme_and_stepped_scale() {
        let chip = mock("<b>2 behind</b>");
        let same = page_key(&chip, 80.4, false);
        assert_eq!(same, page_key(&mock("<b>2 behind</b>"), 79.6, false), "a width is kept to the point");
        assert_ne!(same, page_key(&mock("<b>3 behind</b>"), 80., false), "new words draw again");
        assert_ne!(same, page_key(&chip, 81., false), "a width draws again");
        assert_ne!(same, page_key(&chip, 80., true), "a theme draws again");
        // A zoom that moved a little stays in its step; one that moved a step draws again.
        let at = |zoom: f32| key_of(same, scale_for(2., zoom));
        assert_eq!(at(0.5), at(0.52));
        assert_ne!(at(0.5), at(0.5 * 1.25));
        assert_eq!(at(1.).scale, 2000, "a window's own scale at the fit");
    }

    #[test]
    fn at_most_two_pictures_are_made_at_once() {
        let page = PageKey { hash: 1, width: 80, dark: false };
        let key = |scale: u32| Key { page, scale };
        let mut queue = Queue::default();
        for scale in 1..=12 {
            assert!(queue.ask(key(scale)));
        }
        assert!(!queue.ask(key(3)), "a picture already waiting is not asked twice");
        assert_eq!(queue.next(), Some(key(1)));
        assert_eq!(queue.next(), Some(key(2)));
        assert_eq!(queue.next(), None, "two are being made");
        queue.done(key(1));
        assert_eq!(queue.next(), Some(key(3)));
        assert_eq!(queue.next(), None);
        assert!(!queue.ask(key(2)), "one being made is not asked twice");
        queue.done(key(2));
        assert!(queue.ask(key(2)), "one that came may be asked again");
    }

    #[gpui::test]
    fn twelve_mocks_open_two_pages_and_the_height_comes_with_the_picture(cx: &mut TestAppContext) {
        let started = Rc::new(Cell::new(0));
        let senders = Rc::new(RefCell::new(Vec::new()));
        let image = Arc::new(RenderImage::new(vec![image::Frame::new(image::RgbaImage::new(2, 2))]));
        let painter: Painter = {
            let (started, senders, image) = (started.clone(), senders.clone(), image.clone());
            Rc::new(move |_html, _width, _scale, _appearance, cx: &mut App| {
                started.set(started.get() + 1);
                let (send, receive) = async_channel::bounded::<()>(1);
                senders.borrow_mut().push(send);
                let image = image.clone();
                cx.spawn(async move |_| {
                    let _ = receive.recv().await;
                    Ok((image, 48., Arc::default()))
                })
            })
        };
        let mocks: Vec<_> = (0..12).map(|n| mock(&format!("<i>{n}</i>"))).collect();
        cx.update(|cx| {
            set_painter(painter, cx);
            for one in &mocks {
                ask(key_of(page_key(one, 80., false), 2.), one, cx);
            }
        });
        cx.run_until_parked();
        assert_eq!(started.get(), 2, "twelve asked, two started");
        assert!(cx.update(|cx| height(&mocks[0], 80., false, cx)).is_none(), "no height before a picture");
        for send in senders.borrow().iter().take(1) {
            send.try_send(()).expect("sent");
        }
        cx.run_until_parked();
        assert_eq!(started.get(), 3, "one came, the next starts");
        assert_eq!(cx.update(|cx| height(&mocks[0], 80., false, cx)), Some(48.), "the page's height is the content's");
    }

    /// A zoom that moves on makes the page again at the new scale; the scale let go past `SCALES_KEPT` held its GPU texture
    /// for the run until this.
    #[gpui::test]
    fn a_scale_let_go_leaves_the_gpu(cx: &mut TestAppContext) {
        use gpui::Styled as _;
        let made = Rc::new(RefCell::new(Vec::new()));
        let painter: Painter = {
            let made = made.clone();
            Rc::new(move |_html, _width, _scale, _appearance, _cx: &mut App| {
                let image = Arc::new(RenderImage::new(vec![image::Frame::new(image::RgbaImage::new(2, 2))]));
                made.borrow_mut().push(image.clone());
                Task::ready(Ok((image, 48., Arc::default())))
            })
        };
        let page = mock("<b>zoomed</b>");
        cx.update(|cx| set_painter(painter, cx));
        let window = cx.add_empty_window();
        for scale in [1., 1.25, 1.5625, 1.953125] {
            window.update(|_, cx| ask(key_of(page_key(&page, 80., false), scale), &page, cx));
            window.run_until_parked();
            let image = made.borrow().last().cloned().expect("a picture was made");
            window.draw(point(px(0.), px(0.)), size(px(100.), px(100.)), |_, _| gpui::img(image.clone()).size(px(2.)));
        }
        window.run_until_parked();
        let made = made.borrow();
        assert_eq!(made.len(), SCALES_KEPT + 1);
        window.update(|window, _| {
            assert!(!window.has_image_atlas_entry(&made[0]), "the oldest scale, no longer kept, leaves the GPU");
            assert!(made[1..].iter().all(|image| window.has_image_atlas_entry(image)), "the scales kept stay drawable");
        });
    }

    /// Every mock of every sketch opened kept its pictures for the run. Past `PAGES_KEPT` the page least lately drawn goes,
    /// off the GPU too, and one drawn since is kept.
    #[gpui::test]
    fn past_the_pages_kept_the_least_lately_drawn_goes(cx: &mut TestAppContext) {
        use gpui::{ParentElement as _, Styled as _};
        let made = Rc::new(RefCell::new(HashMap::<String, Arc<RenderImage>>::new()));
        let painter: Painter = {
            let made = made.clone();
            Rc::new(move |html: String, _width, _scale, _appearance, _cx: &mut App| {
                let image = Arc::new(RenderImage::new(vec![image::Frame::new(image::RgbaImage::new(2, 2))]));
                made.borrow_mut().insert(html, image.clone());
                Task::ready(Ok((image, 48., Arc::default())))
            })
        };
        let pages: Vec<_> = (0..=PAGES_KEPT).map(|n| mock(&format!("<i>page {n}</i>"))).collect();
        let image_of = |n: usize| made.borrow().iter().find(|(html, _)| html.contains(&format!("<i>page {n}</i>"))).map(|(_, image)| image.clone()).expect("made");
        cx.update(|cx| set_painter(painter, cx));
        let window = cx.add_empty_window();
        for one in &pages[..PAGES_KEPT] {
            window.update(|_, cx| ask(key_of(page_key(one, 80., false), 2.), one, cx));
            window.run_until_parked();
        }
        let (first, second) = (image_of(0), image_of(1));
        window.draw(point(px(0.), px(0.)), size(px(100.), px(100.)), |_, _| gpui::div().child(gpui::img(first.clone()).size(px(2.))).child(gpui::img(second.clone()).size(px(2.))));
        // The first page is drawn again, so the second is now the one least lately drawn.
        window.update(|window, cx| picture(&pages[0], 80., 1., false, window, cx));
        window.run_until_parked();
        let last = &pages[PAGES_KEPT];
        window.update(|_, cx| ask(key_of(page_key(last, 80., false), 2.), last, cx));
        window.run_until_parked();
        window.update(|window, cx| {
            let pictures = cx.global::<Pictures>();
            assert_eq!(pictures.pages.len(), PAGES_KEPT, "one page past the cap goes");
            assert!(!pictures.pages.contains_key(&page_key(&pages[1], 80., false)), "the page least lately drawn goes");
            assert!(pictures.pages.contains_key(&page_key(&pages[0], 80., false)), "a page drawn since stays");
            assert!(!window.has_image_atlas_entry(&second), "the page let go leaves the GPU");
            assert!(window.has_image_atlas_entry(&first));
        });
    }

    #[gpui::test]
    fn a_page_the_web_view_cannot_draw_is_not_asked_again(cx: &mut TestAppContext) {
        let chip = mock("<u>x</u>");
        let key = key_of(page_key(&chip, 80., false), 2.);
        cx.update(|cx| ask(key, &chip, cx));
        cx.run_until_parked();
        cx.update(|cx| {
            assert!(cx.global::<Pictures>().failed.contains(&key.page), "no web view yet");
            ask(key_of(page_key(&chip, 80., false), 2.5), &chip, cx);
            assert_eq!(cx.global::<Pictures>().queue.waiting.len(), 0, "a failed page keeps its plain frame");
        });
    }

    /// The take Fleet holds for "Which status chip reads right on the card?": the chip's rules name tokens, and no sheet kept with it
    /// declares them.
    #[gpui::test]
    fn a_chip_whose_sheet_names_tokens_no_kept_sheet_declares_is_drawn_with_the_apps(cx: &mut TestAppContext) {
        let chip = Rc::new(Mock {
            css: Some(".armada-chip{background:var(--bg-raised);color:var(--fg-muted);font-family:var(--font-mono);font-size:var(--text-2xs)}".into()),
            ..Rc::try_unwrap(mock(r#"<div class="armada-chip" data-part="chip">Ready to try</div>"#)).ok().expect("one owner")
        });
        let seen = Rc::new(RefCell::new(Vec::new()));
        let painter: Painter = {
            let seen = seen.clone();
            Rc::new(move |html, width, _scale, _appearance, _cx: &mut App| {
                seen.borrow_mut().push((html, width));
                Task::ready(Err("not drawn here".to_string()))
            })
        };
        cx.update(|cx| {
            set_painter(painter, cx);
            ask(key_of(page_key(&chip, 260., true), 2.), &chip, cx);
        });
        cx.run_until_parked();
        let seen = seen.borrow();
        let (html, _) = seen.first().expect("the page went to the web view");
        let rule = html.find(".armada-chip{").expect("the chip's own rule");
        let tokens = html.find(":root{").expect("the app's tokens ride on the page");
        assert!(tokens < rule, "the tokens come first, so a kept sheet's own still wins: {html}");
        for name in ["--bg-raised", "--fg-muted", "--font-mono", "--text-2xs"] {
            assert!(html[tokens..rule].contains(&format!("{name}: ")), "{name} is declared for the rule that reads it: {html}");
        }
        let muted = html[tokens..rule].split("--fg-muted: ").nth(1).and_then(|rest| rest.split(';').next()).expect("a value");
        assert!(!matches!(muted, "#000000" | "#000" | "black"), "the chip's text is not the web view's default black: {muted}");
    }

    /// The chip's own sheet, as this crate keeps it. Every token it names must be one the page declares, or the rule that reads it
    /// (a verdict's hue, a run's) falls back to the inherited value as the unstyled chip did.
    #[test]
    fn every_token_the_chip_sheet_names_is_declared_on_the_page() {
        let sheet = include_str!("fact_chip.css");
        let tokens = super::super::export::declarations(&Look::dark());
        let mut named: Vec<&str> = sheet.split("var(").skip(1).filter_map(|rest| rest.split([')', ',']).next()).collect();
        named.sort_unstable();
        named.dedup();
        assert!(named.len() > 8, "the sheet names its tokens: {named:?}");
        let missing: Vec<&&str> = named.iter().filter(|name| !tokens.contains(&format!("{name}: "))).collect();
        assert!(missing.is_empty(), "the page leaves these undeclared: {missing:?}");
    }
}
