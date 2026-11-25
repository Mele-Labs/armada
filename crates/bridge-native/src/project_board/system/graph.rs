//! The block drawn: the levels along the top, the graph with its lanes, curves and parts, and the panel beside it. Everything is
//! positioned from the arrangement (`arrange.rs`), in pixels, so the curves and the parts agree to the pixel.

use std::cell::Cell;
use std::rc::Rc;

use gpui::{canvas, div, point, prelude::*, px, Animation, AnimationExt as _, AnyElement, Bounds, ClickEvent, Context, FontWeight, Hsla, MouseButton, PathBuilder, Pixels, Role, SharedString, Window};
use ipc::{PartOrigin, SystemMap, SystemPart, SystemPartKind};

use super::arrange::{self, Arranged, Edge, Hover, Lane, LaneKind, Lit, Metrics, Node, Pt};
use super::{panel, Read, SystemBlock};
use crate::labels::labelled;
use crate::look::{token, ActiveLook, Look, Token};
use crate::project_board::layout::View;
use crate::settings::view::{bone, ring_colour};
use crate::titlebar::parts::{glyph, kbd, mono, sans, tip, Btn, Weight};

/// How strongly a hue is laid on a surface, the strengths the board's own chips and tiles use, named for where they go.
pub(super) mod tint {
    /// A chip or a glyph tile.
    pub const TILE: f32 = 0.16;
    /// A glyph tile on a part that is lit.
    pub const TILE_LIT: f32 = 0.18;
    /// A part's fill when it is lit, and when it is not.
    pub const PART_LIT: f32 = 0.14;
    pub const PART: f32 = 0.08;
    /// A selected row of a list.
    pub const ROW_SELECTED: f32 = 0.12;
    /// A person's outline.
    pub const OUTLINE: f32 = 0.45;
    /// A band's ground over the block's.
    pub const BAND: f32 = 0.7;
    /// What is not lit while something is: a curve, a label, a part.
    pub const DIM_CURVE: f32 = 0.18;
    pub const DIM_LABEL: f32 = 0.15;
    pub const DIM_PART: f32 = 0.28;
}

/// How wide a character of the mono face is, in ems.
const MONO_ADVANCE: f32 = 0.6;
/// The same for a part's name, set in the sans face at its heading weight.
const BOLD_ADVANCE: f32 = 0.62;

/// The sizes the arrangement is made of, from the tokens.
pub(super) fn metrics(look: &Look) -> Metrics {
    let len = |token| f32::from(look.length(token));
    Metrics {
        node: len(token::H_ROW_UNIT) * 1.5,
        node_plain: len(token::H_ROW_UNIT) * 1.2,
        file: len(token::LEADING_2XS),
        gap: len(token::SPACE_4),
        lane_head: len(token::H_ROW_ACTION),
        lane_pad: len(token::SPACE_4) + len(token::SPACE_1),
        lane_gap: len(token::SPACE_6),
        col_gap: len(token::SPACE_12) + len(token::SPACE_4),
        bottom: len(token::SPACE_4) + len(token::SPACE_1),
        char: len(token::TEXT_2XS) * MONO_ADVANCE,
        column: len(token::SPACE_12) * 3.,
        name_char: len(token::TEXT_XS) * BOLD_ADVANCE,
        // A part's tile, the padding round it, the gap before its text and the arrow that shows when it is lit.
        chrome: len(token::SPACE_8) + len(token::SPACE_2) * 4. + len(token::SPACE_4) + len(token::BORDER_WIDTH) * 2.,
    }
}

/// The hue a kind of part wears, on its glyph tile, its frame when it is lit and its chip.
pub(super) fn hue_of(kind: SystemPartKind) -> Token {
    match kind {
        SystemPartKind::Person => token::OPERATOR,
        SystemPartKind::Entry => token::ACCENT,
        SystemPartKind::Service => token::SKETCH_TEAL,
        SystemPartKind::Store => token::SKETCH_AMBER,
        SystemPartKind::Outside => token::SKETCH_GREY,
        SystemPartKind::Piece => token::SKETCH_BLUE,
    }
}

pub(super) fn glyph_of(kind: SystemPartKind) -> &'static str {
    match kind {
        SystemPartKind::Person => "user",
        SystemPartKind::Entry => "app-window",
        SystemPartKind::Service => "server",
        SystemPartKind::Store => "database",
        SystemPartKind::Outside => "globe",
        SystemPartKind::Piece => "package",
    }
}

pub(super) fn kind_word(kind: SystemPartKind) -> &'static str {
    match kind {
        SystemPartKind::Person => "person",
        SystemPartKind::Entry => "entry point",
        SystemPartKind::Service => "service",
        SystemPartKind::Store => "store",
        SystemPartKind::Outside => "outside",
        SystemPartKind::Piece => "part",
    }
}

/// A piece takes the hue of the part it belongs to, so the lanes of level two read as the parts of level one did.
pub(super) fn kind_of(kind: SystemPartKind, within: Option<&str>, top: Option<&SystemMap>) -> SystemPartKind {
    match (kind, within) {
        (SystemPartKind::Piece, Some(parent)) => top.and_then(|top| top.parts.iter().find(|part| part.id == parent)).map_or(kind, |part| part.kind),
        _ => kind,
    }
}

/// A chip with a fill: what kind of row it is.
pub(super) fn chip(look: &Look, hue: Token, words: impl Into<SharedString>) -> gpui::Div {
    chip_at(look, 1., hue, words)
}

/// A chip drawn `k` times its size, for a graph that is drawn smaller.
fn chip_at(look: &Look, k: f32, hue: Token, words: impl Into<SharedString>) -> gpui::Div {
    div()
        .flex_none()
        .flex()
        .items_center()
        .h(look.length(token::H_BADGE) * k)
        .px(look.length(token::SPACE_2) * k)
        .rounded(look.length(token::RADIUS_SM) * k)
        .bg(look.color(hue).opacity(tint::TILE))
        .text_size(look.length(token::TEXT_2XS) * k)
        .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
        .text_color(look.color(hue))
        .whitespace_nowrap()
        .child(words.into())
}

/// Records the room it is given and asks the block to draw again where that changed.
fn measure(seen: Rc<Cell<Option<Bounds<Pixels>>>>, block: gpui::WeakEntity<SystemBlock>) -> AnyElement {
    canvas(
        move |bounds, _, cx| {
            let before = seen.get().map(|held| held.size);
            seen.set(Some(bounds));
            if before != Some(bounds.size) {
                let block = block.clone();
                cx.defer(move |cx| {
                    block.update(cx, |_, cx| cx.notify()).ok();
                });
            }
        },
        |_, _, _, _| {},
    )
    .absolute()
    .size_full()
    .into_any_element()
}

pub(super) fn draw(this: &mut SystemBlock, window: &mut Window, cx: &mut Context<SystemBlock>) -> AnyElement {
    let look = cx.look().clone();
    let crumbs = crumbs(this, &look, cx);
    // A block narrower than two columns of room has its panel under the graph, and the whole block scrolls.
    let stacked = f32::from(window.viewport_size().width) < f32::from(look.length(token::SPACE_12)) * 20.;
    let room = room(this, &look, stacked, window, cx);
    let panel = panel::draw(this, &look, stacked, window, cx);
    let body = div()
        .id("system-body")
        .size_full()
        .min_h_0()
        .flex()
        .font_family(sans(&look))
        .text_color(look.color(token::FG_DEFAULT))
        .on_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, window, cx| this.on_key(event, window, cx)));
    if stacked {
        return body.flex_col().overflow_y_scroll().child(crumbs).child(room).child(panel).into_any_element();
    }
    let left = div().flex_1().min_w_0().min_h_0().flex().flex_col().child(crumbs).child(room);
    body.child(left).child(panel).into_any_element()
}

// ---- the levels along the top ----------------------------------------------------------------------

fn crumbs(this: &SystemBlock, look: &Look, cx: &mut Context<SystemBlock>) -> AnyElement {
    let ring = ring_colour(look);
    let steps = this.trail.crumbs(this.project_name());
    let last = steps.len() - 1;
    let mut row = div().flex().items_center().gap(look.length(token::SPACE_1)).min_w_0();
    for (at, crumb) in steps.into_iter().enumerate() {
        if at == last {
            row = row.child(labelled(
                div().id(SharedString::from(format!("system-crumb-{at}"))).text_size(look.length(token::TEXT_SM)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).child(SharedString::from(crumb.words.clone())),
                Role::Heading,
                crumb.words,
            ));
            continue;
        }
        let depth = crumb.depth;
        row = row
            .child(labelled(
                div()
                    .id(SharedString::from(format!("system-crumb-{at}")))
                    .px(look.length(token::SPACE_1))
                    .rounded(look.length(token::RADIUS_SM))
                    .border(look.length(token::BORDER_WIDTH))
                    .border_color(gpui::transparent_black())
                    .text_size(look.length(token::TEXT_SM))
                    .text_color(look.color(token::FG_MUTED))
                    .cursor_pointer()
                    .hover(|style| style.bg(look.color(token::BG_HOVER)).text_color(look.color(token::FG_DEFAULT)))
                    .focusable()
                    .tab_stop(true)
                    .focus_visible(move |style| style.border_color(ring))
                    .tooltip(tip(format!("Step out to {}", crumb.words)))
                    .on_click(cx.listener(move |this, _, window, cx| this.out_to(depth, window, cx)))
                    .child(SharedString::from(crumb.words.clone())),
                Role::Button,
                format!("Step out to {}", crumb.words),
            ))
            .child(glyph("chevron-right", look.length(token::SPACE_4), look.color(token::BORDER_STRONG)));
    }
    let out = this.trail.depth() > 1 || this.is_grown();
    let librarian = labelled(
        Btn::new("system-librarian", "Find missed parts", Weight::Secondary)
            .pending(this.asking_librarian)
            .draw(look, cx)
            .tooltip(tip("The librarian reads the project for parts its files do not show"))
            .on_click(cx.listener(|this, _, _, cx| this.ask_librarian(cx))),
        Role::Button,
        "Find missed parts",
    );
    div()
        .flex_none()
        .flex()
        .items_center()
        .gap(look.length(token::SPACE_3))
        .min_h(look.length(token::H_ROW_ACTION) + look.length(token::SPACE_2))
        .px(look.length(token::SPACE_4))
        .pt(look.length(token::SPACE_2))
        .child(labelled(div().id("system-levels").flex_1().min_w_0().child(row), Role::Group, "Levels of the system"))
        .children(out.then(|| div().flex().items_center().gap(look.length(token::SPACE_1)).text_size(look.length(token::TEXT_2XS)).text_color(look.color(token::FG_SUBTLE)).child(kbd("Esc", look)).child("steps out")))
        .child(librarian)
        .into_any_element()
}

// ---- the room the graph stands in ---------------------------------------------------------------------

fn room(this: &mut SystemBlock, look: &Look, natural: bool, window: &mut Window, cx: &mut Context<SystemBlock>) -> AnyElement {
    let seen = this.stage_seen();
    let measuring = measure(seen.clone(), cx.entity().downgrade());
    let size = seen.get().map(|bounds| (f32::from(bounds.size.width), f32::from(bounds.size.height)));
    let mut tall = None;
    let content = match (this.level_read().cloned(), size) {
        (_, None) => None,
        (None | Some(Read::Reading), Some(_)) => Some(skeleton(look)),
        (Some(Read::Failed(why)), Some(_)) => Some(
            div()
                .id("system-failed")
                .flex()
                .items_start()
                .gap(look.length(token::SPACE_2))
                .p(look.length(token::SPACE_4))
                .text_size(look.length(token::TEXT_2XS))
                .text_color(look.color(token::FG_MUTED))
                .child(glyph("triangle-alert", look.length(token::SPACE_4), look.color(token::DEGRADED)))
                .child(SharedString::from(why))
                .into_any_element(),
        ),
        (Some(Read::Ready(map)), Some((width, height))) => {
            let top = this.top().cloned();
            match this.view {
                View::List => Some(list(this, &map, top.as_ref(), look, cx)),
                _ => {
                    let inner = (width - f32::from(look.length(token::SPACE_4)) * 2.).max(0.);
                    // Beside the panel a lane too tall for the room wraps into columns; stacked over it the block scrolls instead, and the
                    // arrangement is as tall as its parts need.
                    let arranged = this.arranged(look, (inner, if natural { 0. } else { height })).cloned();
                    tall = arranged.as_ref().map(|arranged| arranged.height);
                    arranged.map(|arranged| stage(this, &arranged, &map, top.as_ref(), look, window, cx))
                }
            }
        }
    };
    let frame = div().relative().child(measuring);
    // Stacked over its panel the graph is as tall as it is; beside it, it fills what is left and scrolls.
    let frame = if natural { frame.flex_none().h(px(tall.unwrap_or_else(|| f32::from(look.length(token::SPACE_12)) * 4.))) } else { frame.flex_1().min_h_0() };
    frame
        .child(labelled(
            div()
                .id("system-graph")
                .size_full()
                .overflow_scroll()
                .px(look.length(token::SPACE_4))
                .track_focus(&this.focus)
                .focus_visible(|style| style)
                .on_mouse_down(MouseButton::Left, cx.listener(|this, _, window, cx| window.focus(&this.focus, cx)))
                .children(content),
            Role::Group,
            "System graph",
        ))
        .into_any_element()
}

/// Lanes of bones where the level is still being read.
fn skeleton(look: &Look) -> AnyElement {
    let lane = |bones: usize| {
        div()
            .flex_1()
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_3))
            .p(look.length(token::SPACE_4))
            .rounded(look.length(token::RADIUS_LG))
            .bg(look.color(token::BG_SUNKEN).opacity(tint::BAND))
            .children((0..bones).map(|at| bone(look, look.length(token::SPACE_12) * (3.5 - at as f32 * 0.4), look.length(token::H_ROW_UNIT))))
    };
    labelled(
        div().id("system-loading").flex().gap(look.length(token::SPACE_8)).pt(look.length(token::SPACE_2)).child(lane(2)).child(lane(4)).child(lane(3)),
        Role::Image,
        "Loading",
    )
    .into_any_element()
}

// ---- the graph ---------------------------------------------------------------------------------------

fn stage(this: &SystemBlock, arranged: &Arranged, map: &SystemMap, top: Option<&SystemMap>, look: &Look, window: &Window, cx: &mut Context<SystemBlock>) -> AnyElement {
    let target = match &this.hover {
        Hover::None if this.focus.is_focused(window) => this.selected.clone().map_or(Hover::None, Hover::Part),
        other => other.clone(),
    };
    let lit = arrange::lit(arranged, map, &target, this.trail.depth());
    let dimming = lit.is_some();
    let lanes = arranged.lanes.iter().map(|lane| lane_box(lane, arranged.scale, look));
    let drawn = curves(arranged, lit.as_ref(), look);
    let pills = arranged.edges.iter().enumerate().filter(|(_, edge)| edge.label.is_some() || edge.step.is_some()).map(|(at, edge)| pill(edge, lit.as_ref().is_none_or(|lit| lit.edges.contains(&at)), dimming, arranged.scale, look));
    let nodes = arranged.nodes.iter().enumerate().map(|(at, node)| part_box(this, node, lit.as_ref().map(|lit| lit.nodes.contains(&at)), top, arranged.scale, look, cx));
    div()
        .relative()
        .w(px(arranged.width))
        .h(px(arranged.height))
        .children(lanes)
        .child(drawn)
        .children(nodes)
        .children(pills)
        .with_animation(SharedString::from(format!("system-level-{}", this.gen())), Animation::new(look.duration(token::DURATION_ENTER)).with_easing(gpui::ease_out_quint()), |stage, at| stage.opacity(at))
        .into_any_element()
}

fn lane_box(lane: &Lane, k: f32, look: &Look) -> AnyElement {
    let len = |token| look.length(token) * k;
    let rect = lane.rect;
    let frame = div().absolute().left(px(rect.x)).top(px(rect.y)).w(px(rect.w)).h(px(rect.h)).rounded(len(token::RADIUS_LG));
    let frame = match lane.kind {
        LaneKind::Band => frame.bg(look.color(token::BG_SUNKEN).opacity(tint::BAND)),
        LaneKind::Boundary => frame.border(len(token::BORDER_WIDTH)).border_dashed().border_color(look.color(token::BORDER_STRONG)),
    };
    frame
        .child(
            div()
                .absolute()
                .left(len(token::SPACE_3))
                .top(len(token::SPACE_2))
                .text_size(len(token::TEXT_2XS))
                .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                .text_color(look.color(token::FG_SUBTLE))
                .child(SharedString::from(lane.title.to_uppercase())),
        )
        .into_any_element()
}

/// The curves, painted in one canvas: every link, bright where it is lit and faint where something else is.
fn curves(arranged: &Arranged, lit: Option<&Lit>, look: &Look) -> AnyElement {
    let len = |token| look.length(token) * arranged.scale;
    struct Drawn {
        edge: Edge,
        lit: bool,
        dim: bool,
    }
    let edges: Vec<Drawn> = arranged.edges.iter().enumerate().map(|(at, edge)| Drawn { edge: edge.clone(), lit: lit.is_some_and(|lit| lit.edges.contains(&at)), dim: lit.is_some_and(|lit| !lit.edges.contains(&at)) }).collect();
    let (base, bright) = (look.color(token::BORDER_STRONG), look.color(token::ACCENT));
    let (plain, bold) = (f32::from(len(token::BORDER_WIDTH)) * 1.5, f32::from(len(token::BORDER_WIDTH)) * 2.);
    let faint = tint::DIM_CURVE;
    let (dash, head) = ((f32::from(len(token::SPACE_1)) * 1.5, f32::from(len(token::SPACE_1)) * 1.25), (f32::from(len(token::SPACE_2)), f32::from(len(token::SPACE_1))));
    canvas(
        |_, _, _| (),
        move |bounds, _, window, _| {
            let at = |pt: Pt| point(bounds.origin.x + px(pt.x), bounds.origin.y + px(pt.y));
            for drawn in &edges {
                let ink: Hsla = if drawn.lit { bright } else if drawn.dim { base.opacity(base.a * faint) } else { base };
                let edge = &drawn.edge;
                let mut line = PathBuilder::stroke(px(if drawn.lit { bold } else { plain }));
                if drawn.lit {
                    for (from, to) in dashes(edge, dash.0, dash.1) {
                        line.move_to(at(from));
                        line.line_to(at(to));
                    }
                } else {
                    line.move_to(at(edge.a));
                    line.cubic_bezier_to(at(edge.b), at(edge.c1), at(edge.c2));
                }
                if let Ok(path) = line.build() {
                    window.paint_path(path, ink);
                }
                let (long, half) = head;
                let (h, tip) = (edge.heading(), edge.b);
                let mut head = PathBuilder::fill();
                head.add_polygon(&[at(tip), at(Pt { x: tip.x - h.x * long - h.y * half, y: tip.y - h.y * long + h.x * half }), at(Pt { x: tip.x - h.x * long + h.y * half, y: tip.y - h.y * long - h.x * half })], true);
                if let Ok(path) = head.build() {
                    window.paint_path(path, ink);
                }
            }
        },
    )
    .absolute()
    .size_full()
    .into_any_element()
}

/// The curve cut into dashes `on` long with `off` between, as pairs of end points along it.
fn dashes(edge: &Edge, on: f32, off: f32) -> Vec<(Pt, Pt)> {
    const STEPS: usize = 48;
    let points: Vec<Pt> = (0..=STEPS).map(|at| edge.at(at as f32 / STEPS as f32)).collect();
    let mut out = Vec::new();
    let (mut run, mut drawing) = (0., true);
    for pair in points.windows(2) {
        let long = (pair[1].x - pair[0].x).hypot(pair[1].y - pair[0].y);
        if drawing {
            out.push((pair[0], pair[1]));
        }
        run += long;
        let limit = if drawing { on } else { off };
        if run >= limit {
            run = 0.;
            drawing = !drawing;
        }
    }
    out
}

/// What a link says, as a pill at the middle of its curve: its step first where it is one of a feature's.
fn pill(edge: &Edge, lit: bool, dimming: bool, k: f32, look: &Look) -> AnyElement {
    let len = |token| look.length(token) * k;
    let width = len(token::SPACE_12) * 4.;
    let height = len(token::H_BADGE);
    let step = edge.step.map(|step| {
        div()
            .flex_none()
            .size(len(token::SPACE_4))
            .flex()
            .items_center()
            .justify_center()
            .rounded_full()
            .bg(look.color(token::ACCENT))
            .text_color(look.color(token::TEXT_ON_ACCENT))
            .text_size(len(token::TEXT_2XS))
            .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
            .child(SharedString::from(step.to_string()))
    });
    let inner = div()
        .flex()
        .items_center()
        .gap(len(token::SPACE_1))
        .h(height)
        .px(len(token::SPACE_2))
        .rounded_full()
        .border(len(token::BORDER_WIDTH))
        .border_color(if lit && dimming { look.color(token::ACCENT) } else { look.color(token::BORDER_SUBTLE) })
        .bg(look.color(token::BG_RAISED))
        .font_family(mono(look))
        .text_size(len(token::TEXT_2XS))
        .text_color(if lit && dimming { look.color(token::FG_DEFAULT) } else { look.color(token::FG_MUTED) })
        .whitespace_nowrap()
        .children(step)
        .children(edge.label.clone().map(SharedString::from));
    div()
        .absolute()
        .left(px(edge.mid.x) - width / 2.)
        .top(px(edge.mid.y) - height / 2.)
        .w(width)
        .flex()
        .justify_center()
        .when(dimming && !lit, |pill| pill.opacity(tint::DIM_LABEL))
        .child(inner)
        .into_any_element()
}

/// One part: its glyph tile, its name and the line under it, in the frame its kind gives it.
fn part_box(this: &SystemBlock, node: &Node, lit: Option<bool>, top: Option<&SystemMap>, k: f32, look: &Look, cx: &mut Context<SystemBlock>) -> AnyElement {
    let len = |token| look.length(token) * k;
    let kind = kind_of(node.kind, node.within.as_deref(), top);
    let hue = look.color(hue_of(kind));
    let selected = this.selected.as_deref() == Some(node.id.as_str());
    let bright = selected || lit == Some(true);
    let dim = lit == Some(false) && !selected;
    let rect = node.rect;
    let person = node.kind == SystemPartKind::Person;
    let (id, hover_id, click_id) = (node.id.clone(), node.id.clone(), node.id.clone());
    let tile = div()
        .flex_none()
        .size(len(token::SPACE_8))
        .flex()
        .items_center()
        .justify_center()
        .rounded(len(token::RADIUS_LG))
        .bg(hue.opacity(tint::TILE_LIT))
        .child(glyph(glyph_of(kind), len(token::SPACE_4), hue));
    let text = div()
        .flex_1()
        .min_w_0()
        .flex()
        .flex_col()
        .gap(len(token::SPACE_1) / 2.)
        .child(
            div()
                .flex()
                .items_center()
                .gap(len(token::SPACE_1))
                .child(div().min_w_0().truncate().text_size(len(token::TEXT_XS)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).child(SharedString::from(node.name.clone()))),
        )
        .children(node.librarian.then(|| div().flex().child(chip_at(look, k, token::SKETCH_AMBER, "librarian"))))
        .children(node.meta.clone().map(|meta| div().truncate().font_family(mono(look)).text_size(len(token::TEXT_2XS)).text_color(look.color(token::FG_SUBTLE)).child(SharedString::from(meta))))
        .children((!node.files.is_empty()).then(|| {
            div().flex().flex_col().pt(len(token::SPACE_1)).children(node.files.iter().map(|file| div().truncate().font_family(mono(look)).text_size(len(token::TEXT_2XS)).text_color(look.color(token::FG_MUTED)).child(SharedString::from(file.clone()))))
        }));
    let said = format!("{}, {}{}", node.name, kind_word(node.kind), if node.librarian { ", from the librarian" } else { "" });
    let summary = this.level().and_then(|map| map.parts.iter().find(|part| part.id == node.id)).map(|part| part.summary.clone()).unwrap_or_default();
    let element = div()
        .id(SharedString::from(format!("system-part-{id}")))
        .absolute()
        .left(px(rect.x))
        .top(px(rect.y))
        .w(px(rect.w))
        .h(px(rect.h))
        .flex()
        .when(node.files.is_empty(), |part| part.items_center())
        .gap(len(token::SPACE_2))
        .p(len(token::SPACE_2))
        .when(person, |part| part.rounded_full())
        .when(!person, |part| part.rounded(len(token::RADIUS_LG)))
        .border(len(token::BORDER_WIDTH))
        .when(node.librarian, |part| part.border_dashed())
        .border_color(if bright { hue } else if person { hue.opacity(tint::OUTLINE) } else { look.color(token::BORDER_DEFAULT) })
        .bg(if selected {
            hue.opacity(tint::PART_LIT)
        } else if bright {
            hue.opacity(tint::PART)
        } else if person || node.librarian {
            gpui::transparent_black()
        } else {
            look.color(token::BG_OVERLAY)
        })
        .when(dim, |part| part.opacity(tint::DIM_PART))
        .cursor_pointer()
        .when(!summary.is_empty(), |part| part.tooltip(tip(summary)))
        .aria_selected(selected)
        .on_hover(cx.listener(move |this, on: &bool, _, cx| {
            if *on {
                this.set_hover(Hover::Part(hover_id.clone()), cx);
            } else if this.hover == Hover::Part(hover_id.clone()) {
                this.set_hover(Hover::None, cx);
            }
        }))
        .on_click(cx.listener(move |this, event: &ClickEvent, window, cx| {
            window.focus(&this.focus, cx);
            this.select(Some(click_id.clone()), cx);
            if event.click_count() >= 2 {
                this.look_inside(window, cx);
            }
        }))
        .child(tile)
        .child(text)
        .children((node.zooms && bright).then(|| glyph("chevron-right", len(token::SPACE_4), look.color(token::FG_SUBTLE))));
    labelled(element, Role::Button, said).into_any_element()
}

// ---- the fallback: a list ------------------------------------------------------------------------------

fn list(this: &SystemBlock, map: &SystemMap, top: Option<&SystemMap>, look: &Look, cx: &mut Context<SystemBlock>) -> AnyElement {
    let ring = ring_colour(look);
    let rows = map.parts.iter().map(|part: &SystemPart| {
        let kind = kind_of(part.kind, part.within.as_deref(), top);
        let hue = look.color(hue_of(kind));
        let selected = this.selected.as_deref() == Some(part.id.as_str());
        let id = part.id.clone();
        let said = format!("{}, {}{}", part.name, kind_word(part.kind), if matches!(part.origin, PartOrigin::Librarian { .. }) { ", from the librarian" } else { "" });
        labelled(
            div()
                .id(SharedString::from(format!("system-row-{}", part.id)))
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .h(look.length(token::H_ROW_ACTION))
                .px(look.length(token::SPACE_2))
                .rounded(look.length(token::RADIUS_MD))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(if selected { hue } else { gpui::transparent_black() })
                .bg(if selected { hue.opacity(tint::ROW_SELECTED) } else { gpui::transparent_black() })
                .text_size(look.length(token::TEXT_XS))
                .cursor_pointer()
                .hover(|style| style.bg(look.color(token::BG_HOVER)))
                .focusable()
                .tab_stop(true)
                .focus_visible(move |style| style.border_color(ring))
                .aria_selected(selected)
                .on_click(cx.listener(move |this, event: &ClickEvent, window, cx| {
                    this.select(Some(id.clone()), cx);
                    if event.click_count() >= 2 {
                        this.look_inside(window, cx);
                    }
                }))
                .child(glyph(glyph_of(kind), look.length(token::SPACE_4), hue))
                .child(div().flex_none().font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).child(SharedString::from(part.name.clone())))
                .child(div().flex_1().min_w_0().truncate().text_size(look.length(token::TEXT_2XS)).text_color(look.color(token::FG_MUTED)).child(SharedString::from(part.summary.clone())))
                .children(matches!(part.origin, PartOrigin::Librarian { .. }).then(|| chip(look, token::SKETCH_AMBER, "librarian")))
                .child(chip(look, hue_of(kind), kind_word(part.kind))),
            Role::Button,
            said,
        )
    });
    labelled(div().id("system-list").flex().flex_col().gap(look.length(token::SPACE_1)).py(look.length(token::SPACE_2)).children(rows), Role::List, "Parts of the system").into_any_element()
}
