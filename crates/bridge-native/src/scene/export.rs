//! A sketch shared with somebody who has no Armada, as Electron's Share makes it: Mermaid text
//! (`export-mermaid.ts`), one web page with the drawing and its steps (`export-svg.ts`,
//! `export-html.ts`), and a picture (`svgToPng`). Each is drawn from the layout the reader shows.
//!
//! The drawing is written with every colour, size and face as a value on the element, never as a
//! `var()` or a `color-mix()`: resvg, which makes the picture, reads neither. The page adds the rules
//! its player needs on top, which a browser applies over those values.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;
use std::rc::Rc;
use std::sync::{Arc, OnceLock};

use gpui::{App, AssetSource, Hsla, Rgba, SharedString, TextRun, Window};
use ipc::{
    Scene, SceneColor, SceneEdge, SceneKind, SceneNode, SceneShape, ScreenAct, ScreenAlign, ScreenChrome, ScreenElement, ScreenField, ScreenFrom, ScreenGap, ScreenPart, ScreenSize,
    ScreenSizeWord, ScreenText, ScreenVariant, WirePart,
};
use resvg::{tiny_skia, usvg};

use super::geometry::{Pt, Rect};
use super::route::Seg;
use super::{along, ground, hue, laid, part_label, shaped, shaped_padding, sheet_top, tint, trees, Laid, Measure, Way, BORDER, DASH, ICON, LEAD, PAD_X, PAD_Y, RADIUS};
use crate::look::{token, Look, Token};

/// What the toast names after a copy (`MERMAID_COPIED`).
pub(crate) const MERMAID_COPIED: &str = "The Mermaid text";

// ---- Mermaid -------------------------------------------------------------------------------------

/// A box's size where the scene does not give one (`DEFAULT_W`, `DEFAULT_H`).
const DEFAULT_W: f64 = 240.;
const DEFAULT_H: f64 = 90.;

/// Text for inside a quoted Mermaid label. Mermaid reads `#name;` and `#NN;` as entities inside a quote,
/// so `#` goes first and every character that would end the quote, open a tag or start markdown goes as one.
fn mermaid_text(text: &str) -> String {
    text.replace('#', "#35;")
        .replace('"', "#quot;")
        .replace('&', "#amp;")
        .replace('<', "#lt;")
        .replace('>', "#gt;")
        .replace('`', "#96;")
        .replace("\r\n", "<br/>")
        .replace('\n', "<br/>")
}

fn quoted(text: &str) -> String {
    format!("\"{text}\"")
}

/// What a box says: its title over its body, or the words of a label.
fn mermaid_words(node: &SceneNode) -> String {
    if node.kind == SceneKind::Label {
        return mermaid_text(node.title.as_deref().or(node.body.as_deref()).unwrap_or(&node.id));
    }
    let mut all: Vec<String> = Vec::new();
    if let Some(title) = &node.title {
        all.push(format!("<b>{}</b>", mermaid_text(title)));
    }
    if let Some(lang) = &node.lang {
        all.push(mermaid_text(lang));
    }
    if let Some(body) = node.body.as_deref().filter(|body| !body.is_empty()) {
        all.push(mermaid_text(body));
    }
    if all.is_empty() {
        mermaid_text(&node.id)
    } else {
        all.join("<br/>")
    }
}

/// Mermaid's own outline for each of a box's shapes, then the code, wire and screen frames.
fn mermaid_frame(node: &SceneNode, text: &str) -> String {
    let label = quoted(text);
    if node.kind == SceneKind::Box {
        match node.shape {
            Some(SceneShape::Pill) => return format!("([{label}])"),
            Some(SceneShape::Circle) => return format!("(({label}))"),
            Some(SceneShape::Diamond) => return format!("{{{label}}}"),
            Some(SceneShape::Cylinder) => return format!("[({label})]"),
            Some(SceneShape::Hexagon) => return format!("{{{{{label}}}}}"),
            Some(SceneShape::Box) | None => {}
        }
    }
    match node.kind {
        SceneKind::Code => format!("[[{label}]]"),
        SceneKind::Wire | SceneKind::Screen => format!("([{label}])"),
        _ => format!("[{label}]"),
    }
}

#[derive(Clone, Copy)]
struct Span {
    x: f64,
    y: f64,
    r: f64,
    b: f64,
}

impl Span {
    fn of(node: &SceneNode) -> Span {
        // What a screen is taken to measure before anything has laid it out (`screenGuess`).
        let guess = match node.screen.as_ref() {
            None => (DEFAULT_W, DEFAULT_H),
            Some(_) => {
                let tree = node.screen_tree();
                let across = node.w.unwrap_or_else(|| tree.as_ref().map_or(960., ScreenElement::width));
                let phone = tree.as_ref().is_some_and(|tree| matches!(tree.part, ScreenPart::Window { chrome: ScreenChrome::Phone, .. }));
                (across, if phone { across * 2. } else { (across * 0.625).round() })
            }
        };
        Span { x: node.x, y: node.y, r: node.x + node.w.unwrap_or(guess.0), b: node.y + node.h.unwrap_or(guess.1) }
    }

    fn area(&self) -> f64 {
        (self.r - self.x) * (self.b - self.y)
    }

    fn holds_point(&self, x: f64, y: f64) -> bool {
        x >= self.x && x <= self.r && y >= self.y && y <= self.b
    }

    fn holds(&self, inner: &Span) -> bool {
        inner.x >= self.x && inner.y >= self.y && inner.r <= self.r && inner.b <= self.b
    }
}

/// `LR` for a scene wider than it is tall, as most flows are drawn, and `TD` otherwise.
fn direction(scene: &Scene) -> &'static str {
    let spans: Vec<Span> = scene.nodes.iter().filter(|node| !ground(node)).map(Span::of).collect();
    if spans.is_empty() {
        return "TD";
    }
    let across = spans.iter().map(|one| one.r).fold(f64::NEG_INFINITY, f64::max) - spans.iter().map(|one| one.x).fold(f64::INFINITY, f64::min);
    let down = spans.iter().map(|one| one.b).fold(f64::NEG_INFINITY, f64::max) - spans.iter().map(|one| one.y).fold(f64::INFINITY, f64::min);
    if across > down * 1.2 {
        "LR"
    } else {
        "TD"
    }
}

/// The scene's boxes and arrows as a Mermaid flowchart, for somebody who pastes it where Mermaid draws.
struct Flow<'a> {
    scene: &'a Scene,
    ids: HashMap<&'a str, String>,
    children: HashMap<&'a str, Vec<&'a SceneNode>>,
    joined: HashSet<&'a str>,
    lines: Vec<String>,
}

impl<'a> Flow<'a> {
    /// The smallest ground that holds a thing is its parent: a box by its middle, a ground by all of it.
    fn parent_of(&self, node: &SceneNode) -> Option<&'a SceneNode> {
        let span = Span::of(node);
        let mut holds: Vec<&'a SceneNode> = self
            .scene
            .nodes
            .iter()
            .filter(|one| ground(one) && one.id != node.id)
            .filter(|one| {
                let outer = Span::of(one);
                if ground(node) {
                    outer.holds(&span) && outer.area() > span.area()
                } else {
                    outer.holds_point((span.x + span.r) / 2., (span.y + span.b) / 2.)
                }
            })
            .collect();
        holds.sort_by(|a, b| Span::of(a).area().total_cmp(&Span::of(b).area()));
        holds.first().copied()
    }

    fn holding(&self, node: &SceneNode) -> bool {
        self.children.get(node.id.as_str()).is_some_and(|kids| kids.iter().any(|kid| !ground(kid) || self.holding(kid)))
    }

    fn emit(&mut self, node: &'a SceneNode, depth: usize) {
        let pad = "  ".repeat(depth);
        let id = self.ids.get(node.id.as_str()).cloned().unwrap_or_default();
        if !ground(node) {
            self.lines.push(format!("{pad}{id}{}", mermaid_frame(node, &mermaid_words(node))));
            return;
        }
        if !self.holding(node) {
            // A ground is a plain box where an arrow joins it, and nothing otherwise.
            if self.joined.contains(node.id.as_str()) {
                let plain = SceneNode { kind: SceneKind::Box, ..node.clone() };
                self.lines.push(format!("{pad}{id}{}", mermaid_frame(&plain, &mermaid_words(&plain))));
            }
            return;
        }
        let title = node.title.as_deref().or_else(|| node.body.as_deref().and_then(|body| body.split('\n').next())).unwrap_or(&node.id);
        self.lines.push(format!("{pad}subgraph {id}[{}]", quoted(&mermaid_text(title))));
        let kids = self.children.get(node.id.as_str()).cloned().unwrap_or_default();
        for kid in kids {
            self.emit(kid, depth + 1);
        }
        self.lines.push(format!("{pad}end"));
    }
}

/// Mermaid lays the boxes out itself, so where they sit and the order they play in are not carried; what is: each
/// box's words, each arrow and its words, and every group or lane that holds a box, as a `subgraph`. Each colour is
/// the theme's value now, since Mermaid draws in values and not in tokens.
pub(crate) fn mermaid(scene: &Scene, look: &Look) -> String {
    // Mermaid ids are its own: a scene's ids may be words it reserves (`end`) or hold characters it parses.
    let ids: HashMap<&str, String> = scene.nodes.iter().enumerate().map(|(at, node)| (node.id.as_str(), format!("n{}", at + 1))).collect();
    let mut flow = Flow {
        scene,
        ids,
        children: HashMap::new(),
        joined: scene.edges.iter().flat_map(|edge| [edge.from.as_str(), edge.to.as_str()]).collect(),
        lines: vec![format!("flowchart {}", direction(scene))],
    };
    let mut roots: Vec<&SceneNode> = Vec::new();
    for node in &scene.nodes {
        match flow.parent_of(node) {
            Some(parent) => flow.children.entry(parent.id.as_str()).or_default().push(node),
            None => roots.push(node),
        }
    }
    for node in roots {
        flow.emit(node, 1);
    }

    // `linkStyle` counts arrows in the order they are written, so the count is of what was written.
    let hued = |color: Option<SceneColor>| color.map(|color| css(hue(look, Some(color), color)));
    let mut links: Vec<String> = Vec::new();
    let mut written = 0;
    for edge in &scene.edges {
        let (Some(from), Some(to)) = (flow.ids.get(edge.from.as_str()), flow.ids.get(edge.to.as_str())) else { continue };
        let said = edge.label.as_deref().map(str::trim).filter(|label| !label.is_empty()).map(mermaid_text);
        let line = match said {
            Some(text) => format!("  {from} -- {} --> {to}", quoted(&text)),
            None => format!("  {from} --> {to}"),
        };
        flow.lines.push(line);
        if let Some(value) = hued(edge.color) {
            links.push(format!("  linkStyle {written} stroke:{value}"));
        }
        written += 1;
    }
    for node in &scene.nodes {
        if let (Some(value), Some(id)) = (hued(node.color), flow.ids.get(node.id.as_str())) {
            if !ground(node) {
                let line = format!("  style {id} stroke:{value}");
                flow.lines.push(line);
            }
        }
    }
    flow.lines.extend(links);
    format!("{}\n", flow.lines.join("\n"))
}

// ---- values --------------------------------------------------------------------------------------

/// A number to a hundredth, as a coordinate: `12`, `12.5`, never `12.000001`.
fn n(value: f32) -> String {
    let rounded = (f64::from(value) * 100.).round() / 100.;
    if rounded == 0. {
        "0".into()
    } else {
        format!("{rounded}")
    }
}

/// A colour as the text an SVG and a stylesheet both read.
fn css(color: Hsla) -> String {
    let rgba: Rgba = color.into();
    let byte = |channel: f32| (channel.clamp(0., 1.) * 255.).round() as u8;
    if rgba.a >= 0.999 {
        format!("#{:02x}{:02x}{:02x}", byte(rgba.r), byte(rgba.g), byte(rgba.b))
    } else {
        format!("rgba({},{},{},{})", byte(rgba.r), byte(rgba.g), byte(rgba.b), n(rgba.a))
    }
}

/// Text for an XML body or an attribute (`xml`).
fn xml(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            c if (c as u32) < 0x20 && !matches!(c, '\t' | '\n' | '\r') => {}
            c => out.push(c),
        }
    }
    out
}

/// A font stack as CSS: names quoted, the generic ones bare.
fn stack(names: &[SharedString], fallback: &str) -> String {
    const GENERIC: [&str; 7] = ["sans-serif", "serif", "monospace", "system-ui", "ui-sans-serif", "ui-monospace", "ui-serif"];
    if names.is_empty() {
        return fallback.to_string();
    }
    names.iter().map(|name| if GENERIC.contains(&name.as_ref()) { name.to_string() } else { format!("'{name}'") }).collect::<Vec<_>>().join(", ")
}

fn kind_word(kind: SceneKind) -> &'static str {
    match kind {
        SceneKind::Box => "box",
        SceneKind::Group => "group",
        SceneKind::Lane => "lane",
        SceneKind::Label => "label",
        SceneKind::Code => "code",
        SceneKind::Wire => "wire",
        SceneKind::Screen => "screen",
        SceneKind::Mock => "mock",
    }
}

fn color_word(color: SceneColor) -> &'static str {
    match color {
        SceneColor::Blue => "blue",
        SceneColor::Teal => "teal",
        SceneColor::Green => "green",
        SceneColor::Amber => "amber",
        SceneColor::Orange => "orange",
        SceneColor::Red => "red",
        SceneColor::Grey => "grey",
    }
}

fn shape_word(shape: SceneShape) -> &'static str {
    match shape {
        SceneShape::Box => "box",
        SceneShape::Pill => "pill",
        SceneShape::Circle => "circle",
        SceneShape::Diamond => "diamond",
        SceneShape::Cylinder => "cylinder",
        SceneShape::Hexagon => "hexagon",
    }
}

/// The inner markup of a registry glyph, 24 units square and drawn as strokes.
fn glyph_inner(name: &str) -> Option<String> {
    let bytes = crate::assets::Assets.load(&crate::assets::icon_path(name)).ok()??;
    let text = std::str::from_utf8(&bytes).ok()?;
    let (open, close) = (text.find('>')?, text.rfind("</svg>")?);
    (open < close).then(|| text[open + 1..close].lines().map(str::trim).collect::<Vec<_>>().concat())
}

/// A path through rounded corners, each radius clockwise from the top left.
fn round_rect(x: f32, y: f32, w: f32, h: f32, [tl, tr, br, bl]: [f32; 4]) -> String {
    format!(
        "M{} {}H{}A{r2} {r2} 0 0 1 {} {}V{}A{r3} {r3} 0 0 1 {} {}H{}A{r4} {r4} 0 0 1 {} {}V{}A{r1} {r1} 0 0 1 {} {}Z",
        n(x + tl),
        n(y),
        n(x + w - tr),
        n(x + w),
        n(y + tr),
        n(y + h - br),
        n(x + w - br),
        n(y + h),
        n(x + bl),
        n(x),
        n(y + h - bl),
        n(y + tl),
        n(x + tl),
        n(y),
        r1 = n(tl),
        r2 = n(tr),
        r3 = n(br),
        r4 = n(bl),
    )
}

// ---- tokens --------------------------------------------------------------------------------------

/// The values a page's own stylesheet and a screen's read, written in beside them so the file needs nothing of Bridge's
/// (`EXPORT_TOKENS`).
fn declarations(look: &Look) -> String {
    const COLORS: &[Token] = &[
        token::BG_BASE,
        token::BG_SUNKEN,
        token::BG_RAISED,
        token::BG_HOVER,
        token::FG_DEFAULT,
        token::FG_MUTED,
        token::FG_INVERSE,
        token::BORDER_SUBTLE,
        token::BORDER_DEFAULT,
        token::BORDER_STRONG,
        token::STATUS_RUNNING,
        token::STATUS_NOT_STARTED,
        token::STATUS_COMPLETED_SUCCESS,
        token::STATUS_COMPLETED_FAILED,
        token::STATUS_AWAITING_REVIEW,
        token::STATUS_REJECTED,
        token::EDGE_FOCUS,
        token::SKETCH_BLUE,
        token::SKETCH_TEAL,
        token::SKETCH_GREEN,
        token::SKETCH_AMBER,
        token::SKETCH_ORANGE,
        token::SKETCH_RED,
        token::SKETCH_GREY,
    ];
    const LENGTHS: &[Token] = &[
        token::SPACE_1,
        token::SPACE_2,
        token::SPACE_3,
        token::SPACE_4,
        token::SPACE_6,
        token::SPACE_8,
        token::SPACE_12,
        token::TEXT_2XS,
        token::TEXT_XS,
        token::TEXT_SM,
        token::TEXT_BASE,
        token::TEXT_XL,
        token::LEADING_2XS,
        token::LEADING_XS,
        token::LEADING_SM,
        token::LEADING_BASE,
        token::LEADING_XL,
        token::RADIUS_SM,
        token::RADIUS_MD,
        token::RADIUS_LG,
        token::BORDER_WIDTH,
        token::EDGE_ACTIVE,
        token::H_CONTROL_SM,
        token::H_BADGE,
    ];
    let mut out = String::new();
    for one in COLORS {
        let _ = write!(out, "{}: {}; ", one.0, css(look.color(*one)));
    }
    for one in LENGTHS {
        let _ = write!(out, "{}: {}px; ", one.0, n(f32::from(look.length(*one))));
    }
    for one in [token::WEIGHT_HEADING, token::WEIGHT_MEDIUM, token::WEIGHT_BODY] {
        let _ = write!(out, "{}: {}; ", one.0, n(look.number(one)));
    }
    let _ = write!(out, "{}: {}%; ", token::ROW_TINT_RECENT.0, n(look.percent(token::ROW_TINT_RECENT) * 100.));
    let _ = write!(out, "{}: {}; ", token::FONT_SANS.0, stack(&look.font_stack(token::FONT_SANS), "'IBM Plex Sans', sans-serif"));
    let _ = write!(out, "{}: {}; ", token::FONT_MONO.0, stack(&look.font_stack(token::FONT_MONO), "'IBM Plex Mono', monospace"));
    let [a, b, c, d] = look.easing(token::EASE).unwrap_or([0.2, 0., 0., 1.]);
    let _ = write!(out, "{}: cubic-bezier({}, {}, {}, {}); ", token::EASE.0, n(a), n(b), n(c), n(d));
    let _ = write!(out, "{}: {}ms;", token::DURATION_BASE.0, look.duration(token::DURATION_BASE).as_millis());
    out
}

// ---- the drawing ---------------------------------------------------------------------------------

/// A picture is drawn for resvg and holds nothing a page's player needs; a page is drawn for a browser.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Picture,
    Page,
}

/// What the drawing is made with: the theme's colours as values, and the app's own text measure, so a body breaks
/// into the lines the reader shows.
struct Kit<'a> {
    look: &'a Look,
    window: &'a Window,
    sans: SharedString,
    mono: SharedString,
    sans_css: String,
    mono_css: String,
    /// Each style's baseline below the top of its line, which only a shaped line can say.
    baselines: RefCell<HashMap<(u32, u32, bool), f32>>,
    fg: String,
    muted: String,
    inverse: String,
    sunken: String,
    strong: String,
    focus: String,
}

impl<'a> Kit<'a> {
    fn new(look: &'a Look, window: &'a Window) -> Kit<'a> {
        let sans_stack = look.font_stack(token::FONT_SANS);
        let mono_stack = look.font_stack(token::FONT_MONO);
        Kit {
            look,
            window,
            sans: sans_stack.first().cloned().unwrap_or_else(|| "IBM Plex Sans".into()),
            mono: mono_stack.first().cloned().unwrap_or_else(|| "IBM Plex Mono".into()),
            sans_css: stack(&sans_stack, "'IBM Plex Sans', sans-serif"),
            mono_css: stack(&mono_stack, "'IBM Plex Mono', monospace"),
            baselines: RefCell::default(),
            fg: css(look.color(token::FG_DEFAULT)),
            muted: css(look.color(token::FG_MUTED)),
            inverse: css(look.color(token::FG_INVERSE)),
            sunken: css(look.color(token::BG_SUNKEN)),
            strong: css(look.color(token::BORDER_STRONG)),
            focus: css(look.color(token::EDGE_FOCUS)),
        }
    }

    fn measure(&self) -> Measure<'a> {
        Measure { window: self.window, sans: self.sans.clone(), mono: self.mono.clone() }
    }

    /// The lines a text breaks into at `wrap` scene units, as the reader shows it.
    fn wrapped(&self, text: &str, at: f32, weight: f32, mono: bool, wrap: Option<f32>) -> Vec<String> {
        let run = TextRun { len: text.len(), font: self.measure().font(weight, mono), ..Default::default() };
        let Ok(shaped) = self.window.text_system().shape_text(SharedString::from(text.to_string()), gpui::px(at), &[run], wrap.map(gpui::px), None) else {
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

    fn width(&self, text: &str, at: f32, weight: f32, mono: bool) -> f32 {
        self.measure().width(text, at, weight, mono)
    }

    /// Where the baseline sits in a line `LEAD` tall: gpui centres the face's ascent and descent in it.
    fn baseline(&self, at: f32, weight: f32, mono: bool) -> f32 {
        let key = (at.to_bits(), weight.to_bits(), mono);
        if let Some(held) = self.baselines.borrow().get(&key) {
            return *held;
        }
        let run = TextRun { len: 1, font: self.measure().font(weight, mono), ..Default::default() };
        let line = self.window.text_system().shape_line("x".into(), gpui::px(at), &[run], None);
        let (ascent, descent) = (f32::from(line.ascent), f32::from(line.descent));
        let found = (LEAD - ascent - descent) / 2. + ascent;
        self.baselines.borrow_mut().insert(key, found);
        found
    }

    /// One line of words whose line box starts `top` down.
    #[allow(clippy::too_many_arguments)]
    fn say(&self, out: &mut String, x: f32, top: f32, at: f32, weight: f32, mono: bool, fill: &str, anchor: &str, words: &str, extra: &str) {
        if words.is_empty() {
            return;
        }
        let y = top + self.baseline(at, weight, mono);
        let family = if mono { format!(" font-family=\"{}\"", self.mono_css) } else { String::new() };
        let anchored = if anchor == "start" { String::new() } else { format!(" text-anchor=\"{anchor}\"") };
        let _ = write!(
            out,
            r#"<text x="{}" y="{}" font-size="{}" font-weight="{}"{family}{anchored} fill="{fill}"{extra} xml:space="preserve">{}</text>"#,
            n(x),
            n(y),
            n(at),
            n(weight),
            xml(words)
        );
    }

    /// Wrapped words, a line to each `LEAD`, from `top`.
    #[allow(clippy::too_many_arguments)]
    fn say_lines(&self, out: &mut String, x: f32, top: f32, wrap: Option<f32>, at: f32, weight: f32, mono: bool, fill: &str, anchor: &str, words: &str) {
        for (index, line) in self.wrapped(words, at, weight, mono, wrap).iter().enumerate() {
            self.say(out, x, top + index as f32 * LEAD, at, weight, mono, fill, anchor, line, "");
        }
    }

    /// A registry glyph in a 12-unit square, drawn as strokes in `fill`.
    fn icon(&self, out: &mut String, name: &str, x: f32, y: f32, fill: &str) {
        let Some(inner) = glyph_inner(name) else { return };
        let _ = write!(
            out,
            r#"<g role="img" aria-label="{}" transform="translate({} {}) scale({})" fill="none" stroke="{fill}" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">{inner}</g>"#,
            xml(&sentence(name)),
            n(x),
            n(y),
            n(ICON / 24.)
        );
    }
}

/// `git-branch` as `Git branch`: how a glyph is named to somebody who cannot see it.
fn sentence(name: &str) -> String {
    let mut chars = name.chars();
    chars.next().map(|first| first.to_uppercase().collect::<String>() + &chars.as_str().replace('-', " ")).unwrap_or_default()
}

/// The kit's rules a screen is drawn with, as the component's own stylesheet holds them.
const SCREEN_CSS: &str = include_str!("../../../../packages/components/src/compositions/SketchScene/ScreenView.css");

/// What a page's player toggles on the drawing. A rule here wins over the value on the element, as a rule does over a
/// presentation attribute.
const STATE_CSS: &str = r#"
.sk-node[data-dim], .sk-edge[data-dim] { opacity: 0.35; }
.sk-ring { display: none; }
.sk-node[data-current] .sk-ring { display: inline; }
.sk-edge[data-active] .sk-edge__line { stroke: var(--edge-focus); }
.sk-edge[data-active] .sk-edge__head { fill: var(--edge-focus); }
.sk-screen[data-off] { display: none; }
:where(.sk-screen) *, :where(.sk-screen) ::before, :where(.sk-screen) ::after { box-sizing: border-box; margin: 0; padding: 0; border: 0 solid; }
:where(.sk-screen) svg { display: block; vertical-align: middle; }
:where(.sk-screen) table { border-collapse: collapse; text-indent: 0; border-color: inherit; }
"#;

/// The kit's rules with their comments off, as `screenStyles` reads them from the page.
fn screen_css() -> String {
    let mut out = String::with_capacity(SCREEN_CSS.len());
    let mut rest = SCREEN_CSS;
    while let Some(open) = rest.find("/*") {
        out.push_str(&rest[..open]);
        rest = rest[open..].find("*/").map_or("", |close| &rest[open + close + 2..]);
    }
    out.push_str(rest);
    out.lines().map(str::trim).filter(|line| !line.is_empty()).collect::<Vec<_>>().join("\n")
}

/// The frame of the drawing: every box, every arrow's way and words, and every hand line, with `pad` of air round it.
fn frame_of(scene: &Scene, laid: &Laid, pad: f32) -> (f32, f32, f32, f32) {
    let mut xs: Vec<f32> = Vec::new();
    let mut ys: Vec<f32> = Vec::new();
    let mut take = |x: f32, y: f32| {
        xs.push(x);
        ys.push(y);
    };
    for rect in &laid.rects {
        take(rect.x, rect.y);
        take(rect.x + rect.w, rect.y + rect.h);
    }
    for way in laid.ways.iter().flatten() {
        for step in 0..=10 {
            if let Some(at) = along(&way.path, step as f32 / 10.) {
                take(at.x, at.y);
            }
        }
        if let Some((rect, _)) = &way.words {
            take(rect.x, rect.y);
            take(rect.x + rect.w, rect.y + rect.h);
        }
    }
    for stroke in &scene.strokes {
        for point in &stroke.points {
            take(point.x as f32, point.y as f32);
        }
    }
    if xs.is_empty() {
        return (0., 0., pad * 2., pad * 2.);
    }
    let least = |all: &[f32]| all.iter().copied().fold(f32::INFINITY, f32::min);
    let most = |all: &[f32]| all.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    let (x, y) = ((least(&xs) - pad).floor(), (least(&ys) - pad).floor());
    (x, y, (most(&xs) + pad).ceil() - x, (most(&ys) + pad).ceil() - y)
}

fn path_text(path: &[Seg]) -> String {
    path.iter()
        .map(|seg| match *seg {
            Seg::Move(p) => format!("M{} {}", n(p.x), n(p.y)),
            Seg::Line(p) => format!("L{} {}", n(p.x), n(p.y)),
            Seg::Quad(c, p) => format!("Q{} {} {} {}", n(c.x), n(c.y), n(p.x), n(p.y)),
        })
        .collect::<Vec<_>>()
        .join("")
}

/// The owner's ink: a smooth line through his points, as the reader draws it.
fn ink_path(points: &[Pt]) -> String {
    let Some((first, rest)) = points.split_first() else { return String::new() };
    let mut out = format!("M{} {}", n(first.x), n(first.y));
    if rest.is_empty() {
        let _ = write!(out, "L{} {}", n(first.x + 0.5), n(first.y));
    }
    for (at, p) in rest.iter().enumerate() {
        match rest.get(at + 1) {
            Some(next) => {
                let _ = write!(out, "Q{} {} {} {}", n(p.x), n(p.y), n((p.x + next.x) / 2.), n((p.y + next.y) / 2.));
            }
            None => {
                let _ = write!(out, "L{} {}", n(p.x), n(p.y));
            }
        }
    }
    out
}

fn dashed() -> String {
    format!(r#" stroke-dasharray="{} {}""#, n(DASH), n(DASH))
}

/// `--text-base`: what a frame's name is set in where it has no node of its own to say.
const TEXT_BASE: f32 = 14.;

/// One node, at its own origin (`nodeSvg`). `sheets` is each variant of a screen's markup, for a page.
fn node_svg(kit: &Kit, node: &SceneNode, rect: Rect, tree: Option<&Rc<ScreenElement>>, sheets: &[(String, String)], mode: Mode) -> String {
    let look = kit.look;
    let (w, h) = (rect.w, rect.h);
    let shape = shaped(node);
    let mut attrs = format!(r#"class="sk-node" data-node="{}" data-kind="{}""#, xml(&node.id), kind_word(node.kind));
    if let Some(shape) = shape {
        let _ = write!(attrs, r#" data-shape="{}""#, shape_word(shape));
    }
    if let Some(color) = node.color {
        let _ = write!(attrs, r#" data-color="{}""#, color_word(color));
    }
    if ground(node) {
        attrs.push_str(r#" data-ground="""#);
    }
    let _ = write!(attrs, r#" role="group" aria-label="{}" transform="translate({} {})""#, xml(part_label(node)), n(rect.x), n(rect.y));
    let mut out = String::new();

    if let Some(shape) = shape {
        draw_shaped(kit, node, shape, w, h, &mut out);
    } else {
        match node.kind {
            SceneKind::Label => {
                let words = node.title.as_deref().or(node.body.as_deref()).unwrap_or("");
                kit.say(&mut out, 0., 0., 14., 600., false, &kit.muted, "start", words, "");
            }
            SceneKind::Wire => draw_wire(kit, node, w, h, &mut out),
            SceneKind::Group | SceneKind::Lane => draw_ground(kit, node, w, h, &mut out),
            SceneKind::Screen if tree.is_some() => {
                // A screen is a sheet of its own under its title, which a browser draws as HTML. resvg draws no
                // `foreignObject`, so a picture of one is its title over an outlined sheet in the paper colour.
                let top = sheet_top(node);
                if let Some(title) = node.title.as_deref() {
                    let at = f32::from(look.length(token::TEXT_SM));
                    kit.say(&mut out, 0., 0., at, look.number(token::WEIGHT_HEADING), false, &kit.muted, "start", title, "");
                }
                match mode {
                    Mode::Picture => {
                        let _ = write!(
                            out,
                            r#"<rect x="1" y="{}" width="{}" height="{}" fill="none" stroke="{}" stroke-width="2"/>"#,
                            n(top + 1.),
                            n((w - 2.).max(0.)),
                            n((h - top - 2.).max(0.)),
                            kit.fg
                        );
                    }
                    Mode::Page => {
                        for (at, markup) in sheets {
                            let _ = write!(
                                out,
                                r#"<g class="sk-screen" data-at="{at}"{}><foreignObject x="0" y="{}" width="{}" height="{}" overflow="visible"><div>{markup}</div></foreignObject></g>"#,
                                if at == "rest" { "" } else { r#" data-off="""# },
                                n(top),
                                n(w),
                                n((h - top).max(0.))
                            );
                        }
                    }
                }
            }
            // A screen that does not read, and a mock, which the app cannot draw as a page yet: the frame and the name.
            SceneKind::Screen | SceneKind::Mock => {
                let _ = write!(
                    out,
                    r#"<rect x="1" y="1" width="{}" height="{}" rx="{}" fill="{}" stroke="{}" stroke-width="2"{}/>"#,
                    n(w - 2.),
                    n(h - 2.),
                    n(RADIUS - 1.),
                    kit.sunken,
                    kit.strong,
                    dashed()
                );
                if let Some(title) = node.title.as_deref() {
                    kit.say(&mut out, BORDER + PAD_X, BORDER, TEXT_BASE, 600., false, &kit.muted, "start", title, "");
                }
            }
            SceneKind::Box | SceneKind::Code => draw_box(kit, node, w, h, &mut out),
        }
    }

    // The ring a step lights: only a page plays steps.
    if mode == Mode::Page && node.kind != SceneKind::Label {
        let focus = &kit.focus;
        if node.kind == SceneKind::Screen && tree.is_some() {
            let top = sheet_top(node);
            let radius = f32::from(look.length(token::RADIUS_SM));
            let _ = write!(
                out,
                r#"<rect class="sk-ring" x="-2" y="{}" width="{}" height="{}" rx="{}" fill="none" stroke="{focus}" stroke-width="2"/>"#,
                n(top - 2.),
                n(w + 4.),
                n(h - top + 4.),
                n(radius + 2.)
            );
        } else {
            let radius = match shape {
                Some(SceneShape::Pill) => h / 2.,
                Some(SceneShape::Circle) => w / 2.,
                _ => RADIUS,
            };
            let _ = write!(
                out,
                r#"<rect class="sk-ring" x="-2" y="-2" width="{}" height="{}" rx="{}" fill="none" stroke="{focus}" stroke-width="2"/>"#,
                n(w + 4.),
                n(h + 4.),
                n(radius + 2.)
            );
        }
    }
    format!("<g {attrs}>{out}</g>")
}

/// A box or a code block: a tinted frame, its head band, and its words.
fn draw_box(kit: &Kit, node: &SceneNode, w: f32, h: f32, out: &mut String) {
    let look = kit.look;
    let mono = node.kind == SceneKind::Code;
    let color = hue(look, node.color, if mono { SceneColor::Grey } else { SceneColor::Blue });
    let (stroke, fill) = (css(color), css(tint(look, color)));
    let _ = write!(
        out,
        r#"<rect x="1" y="1" width="{}" height="{}" rx="{}" fill="{fill}" stroke="{stroke}" stroke-width="2"/>"#,
        n(w - 2.),
        n(h - 2.),
        n(RADIUS - 1.)
    );
    let inner = Rect { x: BORDER, y: BORDER, w: w - 2. * BORDER, h: h - 2. * BORDER };
    let mut top = inner.y;
    if node.title.is_some() || node.icon.is_some() {
        let lang_w = node.lang.as_deref().map_or(0., |lang| kit.width(lang, 13., 600., false));
        let icon_w = if node.icon.is_some() { ICON + 8. } else { 0. };
        let wrap = (inner.w - 2. * PAD_X - icon_w - if lang_w > 0. { lang_w + 8. } else { 0. }).max(1.);
        let lines = kit.wrapped(node.title.as_deref().unwrap_or(""), 13., 600., false, Some(wrap));
        let band_h = 2. * PAD_Y + LEAD * lines.len() as f32;
        let corners = RADIUS - BORDER;
        let _ = write!(out, r#"<path d="{}" fill="{stroke}"/>"#, round_rect(inner.x, inner.y, inner.w, band_h, [corners, corners, 0., 0.]));
        let mut x = inner.x + PAD_X;
        if let Some(icon) = node.icon.as_deref() {
            kit.icon(out, icon, x, inner.y + PAD_Y + (LEAD - ICON) / 2., &kit.inverse);
            x += icon_w;
        }
        for (index, line) in lines.iter().enumerate() {
            kit.say(out, x, inner.y + PAD_Y + index as f32 * LEAD, 13., 600., false, &kit.inverse, "start", line, "");
        }
        if let Some(lang) = node.lang.as_deref() {
            kit.say(out, inner.x + inner.w - PAD_X - lang_w, inner.y + PAD_Y, 13., 600., false, &kit.inverse, "start", lang, r#" fill-opacity="0.8""#);
        }
        top += band_h;
    }
    if let Some(body) = node.body.as_deref() {
        kit.say_lines(out, inner.x + PAD_X, top + PAD_Y, Some(inner.w - 2. * PAD_X), 13., 400., mono, &kit.fg, "start", body);
    }
}

/// A box in an outline of its own: a pill or a circle are rounded rectangles; a diamond, a hexagon and a cylinder are
/// drawn in a hundred-unit square stretched over the box, 2px inside it.
fn draw_shaped(kit: &Kit, node: &SceneNode, shape: SceneShape, w: f32, h: f32, out: &mut String) {
    let look = kit.look;
    let color = hue(look, node.color, SceneColor::Blue);
    let (stroke, fill) = (css(color), css(tint(look, color)));
    match shape {
        SceneShape::Pill | SceneShape::Circle => {
            let radius = if shape == SceneShape::Pill { (h - 2.) / 2. } else { (w - 2.) / 2. };
            let _ = write!(out, r#"<rect x="1" y="1" width="{}" height="{}" rx="{}" fill="{fill}" stroke="{stroke}" stroke-width="2"/>"#, n(w - 2.), n(h - 2.), n(radius));
        }
        _ => {
            let inner = Rect { x: BORDER, y: BORDER, w: w - 2. * BORDER, h: h - 2. * BORDER };
            let at = |x: f32, y: f32| format!("{},{}", n(inner.x + inner.w * x / 100.), n(inner.y + inner.h * y / 100.));
            let (rx, ry) = (n(inner.w / 2.), n(inner.h * 0.1));
            match shape {
                SceneShape::Diamond => {
                    let _ = write!(out, r#"<polygon points="{} {} {} {}" fill="{fill}" stroke="{stroke}" stroke-width="2"/>"#, at(50., 0.), at(100., 50.), at(50., 100.), at(0., 50.));
                }
                SceneShape::Hexagon => {
                    let _ = write!(
                        out,
                        r#"<polygon points="{} {} {} {} {} {}" fill="{fill}" stroke="{stroke}" stroke-width="2"/>"#,
                        at(12., 0.),
                        at(88., 0.),
                        at(100., 50.),
                        at(88., 100.),
                        at(12., 100.),
                        at(0., 50.)
                    );
                }
                _ => {
                    let (a, b, c, d) = (at(0., 10.), at(100., 10.), at(100., 90.), at(0., 90.));
                    let _ = write!(out, r#"<path d="M{a} A{rx} {ry} 0 0 1 {b} L{c} A{rx} {ry} 0 0 1 {d} Z" fill="{fill}" stroke="{stroke}" stroke-width="2"/>"#);
                    let _ = write!(out, r#"<path d="M{a} A{rx} {ry} 0 0 0 {b}" fill="none" stroke="{stroke}" stroke-width="2"/>"#);
                }
            }
        }
    }
    // The words stand in a column in the middle of the space inside the padding.
    let (inline, top, bottom) = shaped_padding(shape, w);
    let wrap = (w - 2. * BORDER - 2. * inline).max(1.);
    let named = node.icon.is_some() || node.title.is_some();
    let lines = node.body.as_deref().map(|body| kit.wrapped(body, 13., 400., false, Some(wrap))).unwrap_or_default();
    let content = if named { LEAD } else { 0. } + if named && !lines.is_empty() { 4. } else { 0. } + lines.len() as f32 * LEAD;
    let room = h - 2. * BORDER - top - bottom;
    let mut y = BORDER + top + (room - content).max(0.) / 2.;
    let middle = w / 2.;
    if named {
        let title_w = node.title.as_deref().map_or(0., |title| kit.width(title, 13., 600., false));
        let name_w = if node.icon.is_some() { ICON + 8. } else { 0. } + title_w;
        let mut x = middle - name_w / 2.;
        if let Some(icon) = node.icon.as_deref() {
            kit.icon(out, icon, x, y + (LEAD - ICON) / 2., &kit.fg);
            x += ICON + 8.;
        }
        if let Some(title) = node.title.as_deref() {
            kit.say(out, x, y, 13., 600., false, &kit.fg, "start", title, "");
        }
        y += LEAD + if lines.is_empty() { 0. } else { 4. };
    }
    for (index, line) in lines.iter().enumerate() {
        kit.say(out, middle, y + index as f32 * LEAD, 13., 400., false, &kit.fg, "middle", line, "");
    }
}

/// A group or a lane: a dashed outline in its hue with its title in a band at the top left.
fn draw_ground(kit: &Kit, node: &SceneNode, w: f32, h: f32, out: &mut String) {
    let stroke = css(hue(kit.look, node.color, SceneColor::Blue));
    let lane = node.kind == SceneKind::Lane;
    if lane {
        let _ = write!(out, r#"<path d="M0 1H{} M0 {}H{}" fill="none" stroke="{stroke}" stroke-width="2"{}/>"#, n(w), n(h - 1.), n(w), dashed());
    } else {
        let _ = write!(out, r#"<rect x="1" y="1" width="{}" height="{}" rx="{}" fill="none" stroke="{stroke}" stroke-width="2"{}/>"#, n(w - 2.), n(h - 2.), n(RADIUS - 1.), dashed());
    }
    let Some(title) = node.title.as_deref() else { return };
    let band_w = kit.width(title, 13., 600., false) + 2. * PAD_X;
    let inset = if lane { 0. } else { BORDER };
    let corners = [if lane { 0. } else { RADIUS - BORDER }, 0., RADIUS, 0.];
    let _ = write!(out, r#"<path d="{}" fill="{stroke}"/>"#, round_rect(inset, BORDER, band_w, LEAD + 2. * PAD_Y, corners));
    kit.say(out, inset + PAD_X, BORDER + PAD_Y, 13., 600., false, &kit.inverse, "start", title, "");
}

/// A wireframe block: what a part of a screen is, drawn as grey bars and never as a control.
fn draw_wire(kit: &Kit, node: &SceneNode, w: f32, h: f32, out: &mut String) {
    let _ = write!(
        out,
        r#"<rect x="1" y="1" width="{}" height="{}" rx="{}" fill="{}" stroke="{}" stroke-width="2"{}/>"#,
        n(w - 2.),
        n(h - 2.),
        n(RADIUS - 1.),
        kit.sunken,
        kit.strong,
        dashed()
    );
    let inner = Rect { x: BORDER, y: BORDER, w: w - 2. * BORDER, h: h - 2. * BORDER };
    let title = node.title.as_deref();
    match node.wire.unwrap_or(WirePart::Card) {
        part @ (WirePart::Card | WirePart::List | WirePart::Text) => {
            let mut y = inner.y + PAD_X;
            let x = inner.x + PAD_X;
            let across = inner.w - 2. * PAD_X;
            if let Some(title) = title {
                kit.say_lines(out, x, y, Some(across), 13., 600., false, &kit.muted, "start", title);
                y += LEAD + 8.;
            }
            let bars: &[f32] = if part == WirePart::Text { &[1., 1.] } else { &[1., 1., 0.6] };
            for share in bars {
                let _ = write!(out, r#"<rect x="{}" y="{}" width="{}" height="8" rx="3" fill="{}"/>"#, n(x), n(y), n(across * share), kit.strong);
                y += 16.;
            }
        }
        WirePart::Button => {
            let _ = write!(out, r#"<rect x="{}" y="{}" width="{}" height="{}" rx="{}" fill="{}"/>"#, n(inner.x), n(inner.y), n(inner.w), n(inner.h), n(RADIUS - BORDER), kit.strong);
            if let Some(title) = title {
                kit.say_lines(out, inner.x + inner.w / 2., inner.y + (inner.h - LEAD) / 2., Some(inner.w - 2. * PAD_X), 13., 600., false, &kit.fg, "middle", title);
            }
        }
        WirePart::Input => {
            let field = Rect { x: inner.x + 8., y: inner.y + 8., w: inner.w - 16., h: inner.h - 16. };
            let _ = write!(out, r#"<rect x="{}" y="{}" width="{}" height="{}" fill="none" stroke="{}" stroke-width="1"/>"#, n(field.x + 0.5), n(field.y + 0.5), n(field.w - 1.), n(field.h - 1.), kit.strong);
            if let Some(title) = title {
                kit.say_lines(out, field.x + PAD_X, field.y + PAD_Y, Some(field.w - 2. * PAD_X), 13., 600., false, &kit.muted, "start", title);
            }
        }
        WirePart::Nav => {
            if let Some(title) = title {
                kit.say_lines(out, inner.x + PAD_X, inner.y + (inner.h - LEAD) / 2., Some(inner.w - 2. * PAD_X), 13., 600., false, &kit.muted, "start", title);
            }
        }
    }
}

/// One arrow: its way in its hue, dashed where it is a flow, a filled head at its end, and its words on a sunken backing.
fn edge_svg(kit: &Kit, scene: &Scene, edge: &SceneEdge, way: &Way) -> String {
    let look = kit.look;
    let stroke = css(edge.color.map_or(look.color(token::FG_MUTED), |color| hue(look, Some(color), SceneColor::Grey)));
    let name = |id: &str| scene.nodes.iter().find(|node| node.id == id).map_or(id, part_label).to_string();
    let said = edge.label.clone().unwrap_or_else(|| format!("{} to {}", name(&edge.from), name(&edge.to)));
    let mut attrs = format!(r#"class="sk-edge" data-edge="{}""#, xml(&edge.id));
    if let Some(color) = edge.color {
        let _ = write!(attrs, r#" data-color="{}""#, color_word(color));
    }
    let flow = edge.flow != Some(false);
    if flow {
        attrs.push_str(r#" data-flow="""#);
    }
    let _ = write!(attrs, r#" role="group" aria-label="{}""#, xml(&said));

    let (mut last, mut before) = (Pt::default(), Pt::default());
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
    let mut out = format!(r#"<path class="sk-edge__line" d="{}" fill="none" stroke="{stroke}" stroke-width="2"{}/>"#, path_text(&way.path), if flow { dashed() } else { String::new() });
    // The marker's 10-unit triangle drawn 16px long, its point a tenth past the way's end.
    let (dx, dy) = (last.x - before.x, last.y - before.y);
    let long = dx.hypot(dy);
    if long > 0. {
        let (ux, uy) = (dx / long, dy / long);
        let tip = Pt { x: last.x + ux * 1.6, y: last.y + uy * 1.6 };
        let back = Pt { x: tip.x - ux * 16., y: tip.y - uy * 16. };
        let _ = write!(
            out,
            r#"<polygon class="sk-edge__head" points="{},{} {},{} {},{}" fill="{stroke}"/>"#,
            n(tip.x),
            n(tip.y),
            n(back.x - uy * 8.),
            n(back.y + ux * 8.),
            n(back.x + uy * 8.),
            n(back.y - ux * 8.)
        );
    }
    if let (Some((rect, lines)), Some(label)) = (&way.words, edge.label.as_deref()) {
        let _ = write!(out, r#"<g class="sk-edge__words" role="img" aria-label="{}">"#, xml(label));
        let _ = write!(out, r#"<rect x="{}" y="{}" width="{}" height="{}" rx="3" fill="{}"/>"#, n(rect.x), n(rect.y), n(rect.w), n(rect.h), kit.sunken);
        for (index, line) in lines.iter().enumerate() {
            kit.say(&mut out, rect.x + rect.w / 2., rect.y + 4. + index as f32 * LEAD, 13., 400., false, &kit.muted, "middle", line, "");
        }
        out.push_str("</g>");
    }
    format!("<g {attrs}>{out}</g>")
}

/// The scene as an SVG element, framed to what it holds (`sketchSvg`).
fn drawing(key: &SharedString, scene: &Scene, title: &str, mode: Mode, window: &mut Window, cx: &mut App) -> String {
    let look = cx.global::<Look>().clone();
    let laid = laid(key, scene, &look, window, cx);
    let trees = trees(key, scene);
    let window: &Window = window;
    let kit = Kit::new(&look, window);
    let pad = match f32::from(look.length(token::SPACE_6)) {
        0. => 24.,
        pad => pad,
    };
    let (x, y, w, h) = frame_of(scene, &laid, pad);

    // Each screen as markup, at rest and at every step one of its elements carries, which only a page that plays needs.
    let sheets: Vec<Vec<(String, String)>> = trees
        .iter()
        .map(|tree| match (tree, mode) {
            (Some(tree), Mode::Page) => {
                let mut at: Vec<f64> = tree.steps();
                at.sort_by(f64::total_cmp);
                at.dedup();
                std::iter::once(("rest".to_string(), screen_markup(tree, None))).chain(at.into_iter().map(|at| (at.to_string(), screen_markup(tree, Some(at))))).collect()
            }
            _ => Vec::new(),
        })
        .collect();

    let draw = |index: usize, node: &SceneNode| node_svg(&kit, node, laid.rects[index], trees[index].as_ref(), &sheets[index], mode);
    let mut out = String::new();
    let _ = write!(
        out,
        r#"<svg {}class="sk armada-sketch-export" viewBox="{} {} {} {}" width="{}" height="{}" role="group" aria-label="{}" font-family="{}">"#,
        if mode == Mode::Picture { r#"xmlns="http://www.w3.org/2000/svg" "# } else { "" },
        n(x),
        n(y),
        n(w),
        n(h),
        n(w),
        n(h),
        xml(title),
        kit.sans_css
    );
    let _ = write!(out, "<title>{}</title>", xml(title));
    if mode == Mode::Page {
        let _ = write!(out, "<style>.sk {{ {} }}{STATE_CSS}{}</style>", declarations(&look), screen_css());
    }
    let _ = write!(out, r#"<rect class="sk-ground" x="{}" y="{}" width="{}" height="{}" fill="{}"/>"#, n(x), n(y), n(w), n(h), kit.sunken);
    for (index, node) in scene.nodes.iter().enumerate().filter(|(_, node)| ground(node)) {
        out.push_str(&draw(index, node));
    }
    for (index, node) in scene.nodes.iter().enumerate().filter(|(_, node)| !ground(node)) {
        out.push_str(&draw(index, node));
    }
    // Over the boxes, so no box hides where an arrow goes.
    for (edge, way) in scene.edges.iter().zip(&laid.ways) {
        if let Some(way) = way {
            out.push_str(&edge_svg(&kit, scene, edge, way));
        }
    }
    let muted = &kit.muted;
    for stroke in &scene.strokes {
        let points: Vec<Pt> = stroke.points.iter().map(|p| Pt { x: p.x as f32, y: p.y as f32 }).collect();
        let _ = write!(out, r#"<path class="sk-ink" d="{}" fill="none" stroke="{muted}" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" role="img" aria-label="Drawn by hand"/>"#, ink_path(&points));
    }
    out.push_str("</svg>");
    out
}

/// What Save as image writes: the drawing as standalone SVG, framed to it, in the theme's colours. resvg draws no
/// `foreignObject`, so a screen is its title over an outlined sheet here; the page carries the screen itself.
pub(crate) fn svg(key: &SharedString, scene: &Scene, window: &mut Window, cx: &mut App) -> String {
    drawing(key, scene, "Sketch", Mode::Picture, window, cx)
}

// ---- the page ------------------------------------------------------------------------------------

/// The words the page's controls carry: the canvas rail's own, so the two read the same.
const PLAY: &str = "Play the steps";
const STOP: &str = "Stop the steps";
const NEXT: &str = "Next step";
const RESTART: &str = "Restart the steps";
const STEPS: &str = "Steps";

const PAGE_CSS: &str = r#"
* { box-sizing: border-box; }
html, body { margin: 0; min-block-size: 100%; }
body { display: flex; flex-direction: column; gap: var(--space-3); padding: var(--space-4); background: var(--bg-base); color: var(--fg-default); font-family: var(--font-sans); font-size: var(--text-base); }
.page-bar { display: flex; align-items: center; justify-content: space-between; gap: var(--space-4); flex-wrap: wrap; }
.page-bar h1 { margin: 0; font-size: var(--text-base); font-weight: var(--weight-heading); }
.page-bar [role="group"] { display: flex; gap: var(--space-2); }
.page-bar button { padding: var(--space-1) var(--space-3); border: var(--edge-active) solid var(--border-default); border-radius: var(--radius-md); background: var(--bg-raised); color: var(--fg-default); font: inherit; cursor: pointer; }
.page-bar button:hover { background: var(--bg-hover); }
.page-bar button[aria-pressed="true"] { border-color: var(--edge-focus); }
.page-bar button:focus-visible { outline: var(--edge-active) solid var(--edge-focus); outline-offset: var(--edge-active); }
main { flex: 1; min-block-size: 0; border: var(--edge-active) solid var(--border-subtle); border-radius: var(--radius-lg); overflow: hidden; background: var(--bg-sunken); }
main svg { display: block; inline-size: 100%; block-size: auto; max-block-size: calc(100vh - var(--space-6) * 3); }
.sk-node, .sk-edge { transition: opacity var(--duration-base) var(--ease); }
.sk-edge__dot { r: var(--space-2); fill: var(--edge-focus); offset-rotate: 0deg; animation: sk-travel calc(var(--duration-pulse) * 0.75) var(--ease) both; }
.armada-sketch-export:not([data-playing]) .sk-edge[data-flow] .sk-edge__line { animation: sk-flow var(--duration-pulse) linear infinite; }
@keyframes sk-flow { to { stroke-dashoffset: calc(var(--space-2) * -2); } }
@keyframes sk-travel { from { offset-distance: 0%; } to { offset-distance: 100%; } }
@media (prefers-reduced-motion: reduce) {
  .sk-edge__dot { display: none; }
  .armada-sketch-export .sk-edge[data-flow] .sk-edge__line { animation: none; stroke-dasharray: none; }
}
"#;

/// The player. Plain script on purpose: it is read by a browser that has nothing of Armada's.
const PLAYER: &str = r#"
(function () {
  var data = JSON.parse(document.getElementById("sketch-steps").textContent);
  var svg = document.querySelector("svg.armada-sketch-export");
  var play = document.getElementById("sketch-play");
  var restart = document.getElementById("sketch-restart");
  if (!svg || data.steps.length === 0) return;
  var reduced = window.matchMedia && window.matchMedia("(prefers-reduced-motion: reduce)").matches;
  var nodes = {}, edges = {};
  svg.querySelectorAll("[data-node]").forEach(function (el) { nodes[el.getAttribute("data-node")] = el; });
  svg.querySelectorAll("[data-edge]").forEach(function (el) { edges[el.getAttribute("data-edge")] = el; });
  var at = -1, timer = 0;
  function flag(el, name, on) { if (on) el.setAttribute(name, ""); else el.removeAttribute(name); }
  function show(index) {
    at = index;
    svg.querySelectorAll(".sk-edge__dot").forEach(function (dot) { dot.remove(); });
    var step = index < 0 ? null : data.steps[index];
    flag(svg, "data-playing", step !== null);
    Object.keys(nodes).forEach(function (id) {
      var el = nodes[id], lit = step !== null && step.nodes.indexOf(id) >= 0;
      // A screen that holds the step in one of its elements shows itself as it stands at that step, and does not stand back.
      var inner = step !== null && !lit && step.inner !== undefined && step.inner.indexOf(id) >= 0;
      flag(el, "data-current", lit);
      flag(el, "data-dim", step !== null && !lit && !inner && !el.hasAttribute("data-ground"));
      el.querySelectorAll(".sk-screen").forEach(function (view) {
        flag(view, "data-off", view.getAttribute("data-at") !== (inner ? String(step.at) : "rest"));
      });
    });
    Object.keys(edges).forEach(function (id) {
      var el = edges[id], active = step !== null && step.edges.indexOf(id) >= 0;
      flag(el, "data-active", active);
      flag(el, "data-dim", step !== null && !active);
      if (active) {
        var dot = document.createElementNS(svg.namespaceURI, "circle");
        dot.setAttribute("class", "sk-edge__dot");
        dot.style.offsetPath = 'path("' + el.querySelector(".sk-edge__line").getAttribute("d") + '")';
        el.appendChild(dot);
      }
    });
    label();
  }
  function label() {
    play.textContent = reduced ? data.words.next : at >= 0 ? data.words.stop : data.words.play;
    play.setAttribute("aria-pressed", at >= 0 ? "true" : "false");
  }
  function tick() {
    var next = at + 1 < data.steps.length ? at + 1 : reduced ? -1 : 0;
    show(next);
    if (!reduced && next >= 0) timer = setTimeout(tick, data.pulse);
  }
  function stop() { clearTimeout(timer); timer = 0; show(-1); }
  play.addEventListener("click", function () {
    if (reduced) { tick(); return; }
    if (at >= 0) stop(); else tick();
  });
  restart.addEventListener("click", function () {
    clearTimeout(timer);
    show(0);
    if (!reduced) timer = setTimeout(tick, data.pulse);
  });
  label();
  if (!reduced) tick();
})();
"#;

/// JSON that is safe to sit inside a `<script>`: nothing in it can close the tag or open a comment.
fn inert(json: &str) -> String {
    json.replace('<', "\\u003c").replace('>', "\\u003e").replace('&', "\\u0026")
}

/// The player's order: what each step lights and which arrows carry its dot, read as the reader reads it. `inner` lists
/// the screens that hold the step in one of their elements: they ring that element and do not stand back.
fn step_plan(scene: &Scene, laid: &Laid, trees: &[Option<Rc<ScreenElement>>]) -> Vec<serde_json::Value> {
    laid.steps
        .iter()
        .map(|&at| {
            let mut nodes: Vec<&str> = Vec::new();
            let lit = scene.nodes.iter().filter(|node| node.step == Some(at)).map(|node| node.id.as_str());
            let reached = scene.edges.iter().filter(|edge| edge.step == Some(at)).map(|edge| edge.to.as_str());
            for id in lit.chain(reached) {
                if !nodes.contains(&id) {
                    nodes.push(id);
                }
            }
            let inner: Vec<&str> = scene
                .nodes
                .iter()
                .zip(trees)
                .filter(|(node, tree)| !nodes.contains(&node.id.as_str()) && tree.as_ref().is_some_and(|tree| tree.holds(at)))
                .map(|(node, _)| node.id.as_str())
                .collect();
            let edges: Vec<&str> = scene.edges.iter().filter(|edge| edge.step == Some(at)).map(|edge| edge.id.as_str()).collect();
            let mut step = serde_json::json!({ "at": at, "nodes": nodes, "edges": edges });
            if !inner.is_empty() {
                step["inner"] = serde_json::json!(inner);
            }
            step
        })
        .collect()
}

/// The page Save as web page writes: the drawing as the reader draws it, its steps playing in a loop, and Play and
/// Restart (`sketchHtml` over `sketchSvg`). One file, with nothing in it fetched: the rules are inline, the tokens are
/// written in, and the script reads only what is in the page.
pub(crate) fn web_page(key: &SharedString, scene: &Scene, title: &str, window: &mut Window, cx: &mut App) -> String {
    let title = if title.trim().is_empty() { "Sketch" } else { title };
    let look = cx.global::<Look>().clone();
    let drawn = drawing(key, scene, title, Mode::Page, window, cx);
    let laid = laid(key, scene, &look, window, cx);
    let trees = trees(key, scene);
    let steps = step_plan(scene, &laid, &trees);
    let pulse = match look.duration(token::DURATION_PULSE).as_millis() as u64 {
        0 => 1600,
        ms => ms,
    };
    let data = serde_json::json!({
        "pulse": pulse,
        "steps": steps,
        "words": { "play": PLAY, "stop": STOP, "next": NEXT, "restart": RESTART, "steps": STEPS },
    });
    let controls = if steps.is_empty() {
        String::new()
    } else {
        format!(r#"<div role="group" aria-label="{STEPS}"><button type="button" id="sketch-play" aria-pressed="false">{PLAY}</button><button type="button" id="sketch-restart">{RESTART}</button></div>"#)
    };
    [
        "<!doctype html>".to_string(),
        r#"<html lang="en">"#.to_string(),
        "<head>".to_string(),
        r#"<meta charset="utf-8">"#.to_string(),
        r#"<meta name="viewport" content="width=device-width, initial-scale=1">"#.to_string(),
        format!("<title>{}</title>", xml(title)),
        format!("<style>:root {{ {} --duration-pulse: {pulse}ms; }}{PAGE_CSS}</style>", declarations(&look)),
        "</head>".to_string(),
        "<body>".to_string(),
        format!(r#"<header class="page-bar"><h1>{}</h1>{controls}</header>"#, xml(title)),
        format!("<main>{drawn}</main>"),
        format!(r#"<script type="application/json" id="sketch-steps">{}</script>"#, inert(&data.to_string())),
        format!("<script>{PLAYER}</script>"),
        "</body>".to_string(),
        "</html>".to_string(),
        String::new(),
    ]
    .join("\n")
}

// ---- a screen as HTML ----------------------------------------------------------------------------

/// What the walk down from the screen's root has found so far (`Ctx`).
#[derive(Clone, Copy)]
struct Walk {
    at: Option<f64>,
    /// An element above is the one playing, so nothing under it stands back.
    inside: bool,
    /// The way the parent lays its children, which decides how a child sizes and sits.
    axis: Option<&'static str>,
}

fn type_word(part: &ScreenPart) -> &'static str {
    match part {
        ScreenPart::Row(_) => "row",
        ScreenPart::Column(_) => "column",
        ScreenPart::Grid { .. } => "grid",
        ScreenPart::Window { .. } => "window",
        ScreenPart::Nav { .. } => "nav",
        ScreenPart::Panel { .. } => "panel",
        ScreenPart::Tabs { .. } => "tabs",
        ScreenPart::List { .. } => "list",
        ScreenPart::Table { .. } => "table",
        ScreenPart::Button { .. } => "button",
        ScreenPart::Field { .. } => "field",
        ScreenPart::Text { .. } => "text",
        ScreenPart::Image { .. } => "image",
        ScreenPart::Avatar { .. } => "avatar",
        ScreenPart::Badge { .. } => "badge",
        ScreenPart::Divider => "divider",
        ScreenPart::Spacer => "spacer",
        ScreenPart::Chat { .. } => "chat",
        ScreenPart::Modal { .. } => "modal",
    }
}

fn gap_word(gap: ScreenGap) -> &'static str {
    match gap {
        ScreenGap::None => "none",
        ScreenGap::Sm => "sm",
        ScreenGap::Md => "md",
        ScreenGap::Lg => "lg",
    }
}

fn align_word(align: ScreenAlign) -> &'static str {
    match align {
        ScreenAlign::Start => "start",
        ScreenAlign::Center => "center",
        ScreenAlign::End => "end",
        ScreenAlign::Stretch => "stretch",
    }
}

/// A registry glyph as an inline SVG, drawn in the text's colour (`Glyph`).
fn glyph(name: &str, size: u32, width: &str) -> String {
    let Some(inner) = glyph_inner(name) else { return String::new() };
    format!(r#"<svg width="{size}" height="{size}" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="{width}" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">{inner}</svg>"#)
}

/// A button, or a bar's item: the words, the glyph, or both (`Act`).
fn act(one: &ScreenAct, variant: &str, compact: bool) -> String {
    let mut out = format!(r#"<span class="armada-wireframe__button" data-variant="{variant}"{}{}>"#, if compact { r#" data-compact="""# } else { "" }, if one.text().is_none() { r#" data-icon-only="""# } else { "" });
    if let Some(icon) = one.icon() {
        out.push_str(&glyph(icon, if compact { 14 } else { 16 }, "1.75"));
    }
    if let Some(text) = one.text() {
        let _ = write!(out, "<span>{}</span>", xml(text));
    }
    out.push_str("</span>");
    out
}

fn items(all: &[ScreenAct], active: Option<&str>) -> String {
    let mut out = String::new();
    for one in all {
        let same = active.is_some_and(|active| one.text().or(one.icon()) == Some(active));
        let _ = write!(out, r#"<span class="armada-wireframe__item"{}{}>"#, if same { r#" data-active="""# } else { "" }, if one.text().is_none() { r#" data-icon-only="""# } else { "" });
        if let Some(icon) = one.icon() {
            out.push_str(&glyph(icon, 16, "1.75"));
        }
        if let Some(text) = one.text() {
            let _ = write!(out, "<span>{}</span>", xml(text));
        }
        out.push_str("</span>");
    }
    out
}

fn kids(el: &ScreenElement, walk: Walk, axis: &'static str, out: &mut String) {
    for one in el.children() {
        part(one, Walk { axis: Some(axis), ..walk }, out);
    }
}

/// One element and everything under it. Standing back is decided here, on the way down (`Part`).
fn part(el: &ScreenElement, walk: Walk, out: &mut String) {
    let kind = type_word(&el.part);
    let current = walk.at.is_some() && el.step == walk.at;
    let dim = walk.at.is_some_and(|at| !walk.inside && !el.holds(at));
    // A modal with a step comes up when its step does, and the steps before it show the screen without it.
    let hidden = matches!(el.part, ScreenPart::Modal { .. }) && walk.at.zip(el.step).is_some_and(|(at, step)| at < step);
    // Under a part that stands back, nothing else is decided: it is one faded thing.
    let down = if dim { Walk { at: None, ..walk } } else { Walk { inside: walk.inside || current, ..walk } };
    let flex = match (walk.axis, el.size) {
        (None | Some("grid"), _) => None,
        (_, Some(ScreenSize::Word(ScreenSizeWord::Fill))) => Some("1 1 0".to_string()),
        (_, Some(ScreenSize::Weight(weight))) => Some(format!("{weight} 1 0")),
        _ => None,
    };
    let mut root = format!(r#"class="armada-wireframe armada-wireframe--{kind}""#);
    if let Some(axis) = walk.axis {
        let _ = write!(root, r#" data-axis="{axis}""#);
    }
    if let Some(align) = el.align {
        let _ = write!(root, r#" data-align="{}""#, align_word(align));
    }
    // A modal rings its dialog, not the scrim round it.
    if current && kind != "modal" {
        root.push_str(r#" data-current="""#);
    }
    if dim {
        root.push_str(r#" data-dim="""#);
    }
    if el.id.is_some() || el.step.is_some() {
        let _ = write!(root, r#" role="group" aria-label="{}""#, xml(&el.label()));
    }
    let styled = |extra: Option<String>| match (flex.clone(), extra) {
        (None, None) => String::new(),
        (Some(flex), None) => format!(r#" style="flex:{flex}""#),
        (None, Some(extra)) => format!(r#" style="{extra}""#),
        (Some(flex), Some(extra)) => format!(r#" style="flex:{flex};{extra}""#),
    };

    match &el.part {
        ScreenPart::Row(layout) | ScreenPart::Column(layout) | ScreenPart::Grid { layout, .. } => {
            let cols = if let ScreenPart::Grid { cols, .. } = &el.part { Some(format!("--wf-cols:{cols}")) } else { None };
            let _ = write!(
                out,
                r#"<div {root}{} data-gap="{}" data-pad="{}"{}>"#,
                styled(cols),
                gap_word(layout.gap.unwrap_or(ScreenGap::Sm)),
                gap_word(layout.pad.unwrap_or(ScreenGap::None)),
                if layout.border { r#" data-border="""# } else { "" }
            );
            kids(el, down, if kind == "row" { "row" } else if kind == "grid" { "grid" } else { "column" }, out);
            out.push_str("</div>");
        }
        ScreenPart::Window { chrome, title, url, .. } => {
            let word = match chrome {
                ScreenChrome::Desktop => "desktop",
                ScreenChrome::Browser => "browser",
                ScreenChrome::Phone => "phone",
            };
            let _ = write!(out, r#"<div {root}{} data-chrome="{word}">"#, styled(None));
            match chrome {
                ScreenChrome::Desktop => {
                    let _ = write!(
                        out,
                        r#"<div class="armada-wireframe__chrome"><span class="armada-wireframe__dots" aria-hidden="true"><span></span><span></span><span></span></span><span class="armada-wireframe__window-title">{}</span></div>"#,
                        xml(title.as_deref().unwrap_or(""))
                    );
                }
                ScreenChrome::Browser => {
                    let _ = write!(
                        out,
                        r#"<div class="armada-wireframe__chrome"><div class="armada-wireframe__tabstrip"><span class="armada-wireframe__tab">{}</span></div><div class="armada-wireframe__toolbar">{}{}{}<span class="armada-wireframe__address">{}<span>{}</span></span></div></div>"#,
                        xml(title.as_deref().unwrap_or(" ")),
                        glyph("chevron-left", 16, "1.75"),
                        glyph("chevron-right", 16, "1.75"),
                        glyph("refresh-cw", 14, "1.75"),
                        glyph("lock", 12, "1.75"),
                        xml(url.as_deref().unwrap_or(""))
                    );
                }
                ScreenChrome::Phone => {
                    out.push_str(r#"<div class="armada-wireframe__chrome"><span class="armada-wireframe__clock">9:41</span><span class="armada-wireframe__signal" aria-hidden="true"><span></span><span></span><span></span><span class="armada-wireframe__battery"></span></span></div>"#);
                }
            }
            out.push_str(r#"<div class="armada-wireframe__window-body">"#);
            kids(el, down, "column", out);
            out.push_str("</div>");
            if *chrome == ScreenChrome::Phone {
                out.push_str(r#"<span class="armada-wireframe__home" aria-hidden="true"></span>"#);
            }
            out.push_str("</div>");
        }
        ScreenPart::Nav { brand, items: bar, active, actions, vertical } => {
            let _ = write!(out, r#"<div {root}{}{}>"#, styled(None), if *vertical { r#" data-vertical="""# } else { "" });
            if let Some(brand) = brand {
                let logo = if *vertical { "" } else { r#"<span class="armada-wireframe__logo" aria-hidden="true"></span>"# };
                let _ = write!(out, r#"<span class="armada-wireframe__brand">{logo}<span>{}</span></span>"#, xml(brand));
            }
            if let Some(bar) = bar {
                let _ = write!(out, r#"<span class="armada-wireframe__items">{}</span>"#, items(bar, active.as_deref()));
            }
            if let Some(actions) = actions {
                let _ = write!(out, r#"<span class="armada-wireframe__actions">{}</span>"#, actions.iter().map(|one| act(one, "ghost", false)).collect::<String>());
            }
            out.push_str("</div>");
        }
        ScreenPart::Panel { title, icon, actions, gap, pad, .. } => {
            let _ = write!(out, "<div {root}{}>", styled(None));
            if title.is_some() || icon.is_some() || actions.is_some() {
                out.push_str(r#"<div class="armada-wireframe__head">"#);
                if let Some(icon) = icon {
                    out.push_str(&glyph(icon, 14, "1.75"));
                }
                let _ = write!(out, r#"<span class="armada-wireframe__head-title">{}</span>"#, xml(title.as_deref().unwrap_or("")));
                for one in actions.iter().flatten() {
                    out.push_str(&act(one, "secondary", true));
                }
                out.push_str("</div>");
            }
            let _ = write!(out, r#"<div class="armada-wireframe__panel-body" data-gap="{}" data-pad="{}">"#, gap_word(gap.unwrap_or(ScreenGap::Sm)), gap_word(pad.unwrap_or(ScreenGap::Md)));
            kids(el, down, "column", out);
            out.push_str("</div></div>");
        }
        ScreenPart::Tabs { items: names, active } => {
            let _ = write!(out, "<div {root}{}>", styled(None));
            for one in names {
                let _ = write!(out, r#"<span class="armada-wireframe__tab-item"{}>{}</span>"#, if active.as_deref() == Some(one.as_str()) { r#" data-active="""# } else { "" }, xml(one));
            }
            out.push_str("</div>");
        }
        ScreenPart::List { rows } => {
            let _ = write!(out, "<div {root}{}>", styled(None));
            for one in rows {
                let _ = write!(out, r#"<div class="armada-wireframe__row"{}>"#, if one.selected { r#" data-selected="""# } else { "" });
                if let Some(icon) = &one.icon {
                    out.push_str(&glyph(icon, 14, "1.75"));
                }
                let _ = write!(out, r#"<span class="armada-wireframe__row-text">{}</span>"#, xml(&one.text));
                if let Some(meta) = &one.meta {
                    let _ = write!(out, r#"<span class="armada-wireframe__meta">{}</span>"#, xml(meta));
                }
                out.push_str("</div>");
            }
            out.push_str("</div>");
        }
        ScreenPart::Table { columns, rows } => {
            let _ = write!(out, "<div {root}{}><table><thead><tr>", styled(None));
            for one in columns {
                let _ = write!(out, "<th>{}</th>", xml(one));
            }
            out.push_str("</tr></thead><tbody>");
            for cells in rows {
                out.push_str("<tr>");
                for one in cells {
                    let _ = write!(out, "<td>{}</td>", xml(one));
                }
                out.push_str("</tr>");
            }
            out.push_str("</tbody></table></div>");
        }
        ScreenPart::Button { text, variant, icon } => {
            let word = match variant.unwrap_or(ScreenVariant::Secondary) {
                ScreenVariant::Primary => "primary",
                ScreenVariant::Secondary => "secondary",
                ScreenVariant::Ghost => "ghost",
            };
            let one = ScreenAct::Parts { text: text.clone(), icon: icon.clone() };
            let _ = write!(out, "<div {root}{}>{}</div>", styled(None), act(&one, word, false));
        }
        ScreenPart::Field { label, kind: field, value, placeholder } => {
            let word = match field {
                ScreenField::Input => "input",
                ScreenField::Textarea => "textarea",
                ScreenField::Select => "select",
                ScreenField::Checkbox => "checkbox",
                ScreenField::Toggle => "toggle",
                ScreenField::Search => "search",
            };
            let choice = matches!(field, ScreenField::Checkbox | ScreenField::Toggle);
            let on = value.as_deref() == Some("on");
            let _ = write!(out, r#"<div {root}{} data-kind="{word}">"#, styled(None));
            if let (Some(label), false) = (label, choice) {
                let _ = write!(out, r#"<span class="armada-wireframe__label">{}</span>"#, xml(label));
            }
            if choice {
                let _ = write!(
                    out,
                    r#"<span class="armada-wireframe__choice"><span class="armada-wireframe__mark" data-kind="{word}"{}>{}</span>{}</span>"#,
                    if on { r#" data-on="""# } else { "" },
                    if *field == ScreenField::Checkbox && on { glyph("check", 12, "2.5") } else { String::new() },
                    label.as_ref().map(|label| format!("<span>{}</span>", xml(label))).unwrap_or_default()
                );
            } else {
                let shown = value.as_deref().or(placeholder.as_deref()).unwrap_or(" ");
                let _ = write!(
                    out,
                    r#"<span class="armada-wireframe__control"{}>{}<span class="armada-wireframe__value">{}</span>{}</span>"#,
                    if value.is_none() { r#" data-placeholder="""# } else { "" },
                    if *field == ScreenField::Search { glyph("search", 14, "1.75") } else { String::new() },
                    xml(shown),
                    if *field == ScreenField::Select { glyph("chevron-down", 14, "1.75") } else { String::new() }
                );
            }
            out.push_str("</div>");
        }
        ScreenPart::Text { style, text, lines } => {
            let word = match style {
                ScreenText::Heading => "heading",
                ScreenText::Title => "title",
                ScreenText::Body => "body",
                ScreenText::Caption => "caption",
            };
            match text {
                Some(text) => {
                    let _ = write!(out, r#"<p {root}{} data-style="{word}">{}</p>"#, styled(None), xml(text));
                }
                None => {
                    const BARS: [&str; 4] = ["100%", "92%", "96%", "84%"];
                    let count = lines.unwrap_or(1) as usize;
                    let _ = write!(out, r#"<div {root}{} data-style="{word}" data-bars="">"#, styled(None));
                    for at in 0..count {
                        let width = if count > 1 && at == count - 1 { "60%" } else { BARS[at % BARS.len()] };
                        let _ = write!(out, r#"<span class="armada-wireframe__bar" style="inline-size:{width}" aria-hidden="true"></span>"#);
                    }
                    out.push_str("</div>");
                }
            }
        }
        ScreenPart::Image { label } => {
            let _ = write!(
                out,
                r#"<div {root}{}><svg class="armada-wireframe__cross" viewBox="0 0 100 100" preserveAspectRatio="none" aria-hidden="true"><line x1="0" y1="0" x2="100" y2="100" vector-effect="non-scaling-stroke"></line><line x1="100" y1="0" x2="0" y2="100" vector-effect="non-scaling-stroke"></line></svg>{}</div>"#,
                styled(None),
                label.as_ref().map(|label| format!(r#"<span class="armada-wireframe__caption">{}</span>"#, xml(label))).unwrap_or_default()
            );
        }
        ScreenPart::Avatar { text } => {
            let inside = match text {
                Some(text) => format!("<span>{}</span>", xml(text)),
                None => r#"<svg viewBox="0 0 24 24" fill="none" stroke-width="1.75" stroke-linecap="round" aria-hidden="true"><circle cx="12" cy="9" r="3.5"></circle><path d="M5 20c0-3.6 3.1-6 7-6s7 2.4 7 6"></path></svg>"#.to_string(),
            };
            let _ = write!(out, "<div {root}{}>{inside}</div>", styled(None));
        }
        ScreenPart::Badge { text } => {
            let _ = write!(out, "<div {root}{}>{}</div>", styled(None), xml(text));
        }
        ScreenPart::Divider | ScreenPart::Spacer => {
            let _ = write!(out, "<div {root}{}></div>", styled(None));
        }
        ScreenPart::Chat { messages } => {
            let _ = write!(out, "<div {root}{}>", styled(None));
            for one in messages {
                let from = if one.from == ScreenFrom::Me { "me" } else { "them" };
                let _ = write!(out, r#"<p class="armada-wireframe__message" data-from="{from}">{}</p>"#, xml(&one.text));
            }
            out.push_str("</div>");
        }
        ScreenPart::Modal { title, .. } => {
            let _ = write!(out, "<div {root}{}{}>", styled(None), if hidden { r#" data-hidden="""# } else { "" });
            let _ = write!(
                out,
                r#"<div class="armada-wireframe__dialog"{}><div class="armada-wireframe__head"><span class="armada-wireframe__head-title">{}</span>{}</div><div class="armada-wireframe__panel-body" data-gap="sm" data-pad="md">"#,
                if current { r#" data-current="""# } else { "" },
                xml(title.as_deref().unwrap_or("")),
                glyph("x", 16, "1.75")
            );
            kids(el, down, "column", out);
            out.push_str("</div></div></div>");
        }
    }
}

/// A screen as HTML: layout is the browser's, so a tree says what is on the screen and never where. `at` is the step
/// playing when this screen holds it; the element carrying it is ringed and the rest of the screen stands back
/// (`ScreenView`).
fn screen_markup(tree: &ScreenElement, at: Option<f64>) -> String {
    let mut out = String::from(r#"<div class="armada-screen">"#);
    part(tree, Walk { at, inside: false, axis: None }, &mut out);
    out.push_str("</div>");
    out
}

// ---- the picture ---------------------------------------------------------------------------------

/// Most pixels a picture may have: a sketch larger than this is told so rather than asked of the machine.
const MOST_PIXELS: u64 = 100_000_000;

/// The system's fonts and IBM Plex, loaded once: scanning them takes far longer than drawing a sketch.
fn fonts() -> Arc<usvg::fontdb::Database> {
    static FONTS: OnceLock<Arc<usvg::fontdb::Database>> = OnceLock::new();
    FONTS
        .get_or_init(|| {
            let mut db = usvg::fontdb::Database::new();
            db.load_system_fonts();
            for (_, bytes) in crate::look::FONTS {
                db.load_font_data(bytes.to_vec());
            }
            db.set_sans_serif_family("IBM Plex Sans");
            db.set_monospace_family("IBM Plex Mono");
            Arc::new(db)
        })
        .clone()
}

/// That SVG as PNG bytes at twice its size (`svgToPng`). The size is the SVG's own width and height.
pub(crate) fn png(svg: &str) -> Result<Vec<u8>, String> {
    let undrawn = || "The sketch did not draw as an image.".to_string();
    let options = usvg::Options { fontdb: fonts(), font_family: "IBM Plex Sans".into(), ..usvg::Options::default() };
    let tree = usvg::Tree::from_str(svg, &options).map_err(|_| undrawn())?;
    let size = tree.size();
    let (width, height) = ((size.width() * 2.).round() as u32, (size.height() * 2.).round() as u32);
    if width == 0 || height == 0 {
        return Err(undrawn());
    }
    if u64::from(width) * u64::from(height) > MOST_PIXELS {
        return Err("The sketch is too large to save as an image.".to_string());
    }
    let mut pixmap = tiny_skia::Pixmap::new(width, height).ok_or_else(undrawn)?;
    resvg::render(&tree, tiny_skia::Transform::from_scale(2., 2.), &mut pixmap.as_mut());
    pixmap.encode_png().map_err(|_| undrawn())
}

/// The title as a file name: nothing a file system or a shell reads as more than a letter, and never
/// empty (`sketchFileName`).
pub(crate) fn file_name(title: &str, extension: &str) -> String {
    let cleaned: String = title.chars().map(|c| if c.is_control() || "\\/:*?\"<>|".contains(c) { ' ' } else { c }).collect();
    let words = cleaned.split_whitespace().collect::<Vec<_>>().join(" ");
    let base: String = words.trim_matches(|c: char| c == '.' || c.is_whitespace()).chars().take(80).collect();
    let base = base.trim();
    format!("{}.{extension}", if base.is_empty() { "Sketch" } else { base })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(id: &str, title: &str) -> SceneNode {
        SceneNode {
            id: id.into(),
            kind: SceneKind::Box,
            x: 0.,
            y: 0.,
            w: None,
            h: None,
            title: Some(title.into()),
            body: None,
            icon: None,
            lang: None,
            wire: None,
            step: None,
            shape: None,
            color: None,
            html: None,
            css: None,
            styles: None,
            kept: None,
            screen: None,
        }
    }

    fn scene(nodes: Vec<SceneNode>, edges: Vec<SceneEdge>) -> Scene {
        Scene { nodes, edges, strokes: Vec::new() }
    }

    #[test]
    fn mermaid_escapes_a_quote_and_a_newline_in_a_label() {
        let text = mermaid(&scene(vec![node("a", "say \"hi\"\nthere")], Vec::new()), &Look::dark());
        assert!(text.contains(r#"n1["<b>say #quot;hi#quot;<br/>there</b>"]"#), "{text}");
        assert!(!text.contains("hi\"\n"), "{text}");
    }

    #[test]
    fn mermaid_keeps_an_arrows_label() {
        let edge = SceneEdge { id: "e".into(), from: "a".into(), to: "b".into(), label: Some("sends it".into()), flow: None, step: None, color: None, line: None, head: None, path: None };
        let text = mermaid(&scene(vec![node("a", "A"), node("b", "B")], vec![edge]), &Look::dark());
        assert!(text.starts_with("flowchart "), "{text}");
        assert!(text.contains(r#"n1 -- "sends it" --> n2"#), "{text}");
    }

    #[test]
    fn mermaid_gives_an_arrow_its_hue_as_the_theme_holds_it() {
        let edge = SceneEdge { id: "e".into(), from: "a".into(), to: "b".into(), label: None, flow: None, step: None, color: Some(SceneColor::Red), line: None, head: None, path: None };
        let look = Look::dark();
        let text = mermaid(&scene(vec![node("a", "A"), node("b", "B")], vec![edge]), &look);
        assert!(text.contains(&format!("linkStyle 0 stroke:{}", css(look.color(token::SKETCH_RED)))), "{text}");
    }

    #[test]
    fn a_file_name_drops_slashes_and_colons_and_falls_back_to_sketch() {
        assert_eq!(file_name("a/b: c", "png"), "a b c.png");
        assert_eq!(file_name("///", "html"), "Sketch.html");
        assert_eq!(file_name("", "png"), "Sketch.png");
    }

    #[test]
    fn a_png_of_a_tiny_svg_starts_with_the_png_signature() {
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" width="4" height="4"><rect width="4" height="4" fill="#ff0000"/></svg>"##;
        let bytes = png(svg).expect("draws");
        assert_eq!(&bytes[..8], &[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a]);
    }

    #[test]
    fn a_png_of_something_that_is_not_an_svg_is_a_sentence() {
        assert_eq!(png("not a picture"), Err("The sketch did not draw as an image.".to_string()));
    }
}
