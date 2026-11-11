//! A `screen` node's sheet, built as Electron's `ScreenView` builds it: one ink on paper, thin lines,
//! the app's sans and square corners, laid out by gpui as the browser lays out the HTML. Every length
//! is `ScreenView.css`'s token times the view's zoom, so the sheet scales with the drawing as a React
//! Flow node does.

use std::cell::Cell;

use gpui::{
    canvas, div, prelude::FluentBuilder, px, relative, svg, AnyElement, Div, FontWeight, Hsla, IntoElement, ParentElement, PathBuilder, Pixels, Role, SharedString,
    InteractiveElement, Styled,
};
use ipc::{ScreenAct, ScreenAlign, ScreenChrome, ScreenElement, ScreenField, ScreenFrom, ScreenGap, ScreenPart, ScreenSize, ScreenSizeWord, ScreenText, ScreenVariant};

use crate::assets::icon_path;
use crate::look::{token, Look, Token};

/// The ink, the paper and the mixes between them, and the zoom every length is drawn at.
pub(super) struct Paper<'a> {
    look: &'a Look,
    z: f32,
    paper: Hsla,
    ink: Hsla,
    quiet: Hsla,
    line: Hsla,
    edge: Hsla,
    bar: Hsla,
    faint: Hsla,
    sans: SharedString,
    /// Elements with an id or a step get a name a walk aims at; only where the scene is read.
    named: bool,
    /// The sheet's width before the zoom: a phone's height is drawn from it.
    width: f32,
    ids: Cell<usize>,
}

/// The step playing, and what the walk down from the root has found (`Ctx`).
#[derive(Clone, Copy)]
struct Ctx {
    at: Option<f64>,
    /// An element above is the one playing, so nothing under it stands back.
    inside: bool,
    /// How the parent lays its children, which decides how a child sizes and sits.
    axis: Option<Axis>,
    /// The last of its parent's children and not the only one: a bar there is a bottom bar.
    ends: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Axis {
    Row,
    Column,
    Grid,
}

const BAR_WIDTHS: [f32; 4] = [1., 0.92, 0.96, 0.84];

impl<'a> Paper<'a> {
    pub(super) fn new(look: &'a Look, z: f32, width: f32, named: bool) -> Self {
        let paper = look.color(token::FG_DEFAULT);
        let ink = look.color(token::BG_BASE);
        let mix = |share: f32| paper.blend(ink.opacity(share));
        Paper {
            look,
            z,
            paper,
            ink,
            quiet: mix(0.58),
            line: mix(0.26),
            edge: mix(0.62),
            bar: mix(0.16),
            faint: mix(0.07),
            sans: look.font_stack(token::FONT_SANS).into_iter().next().unwrap_or_else(|| "IBM Plex Sans".into()),
            named,
            width,
            ids: Cell::new(0),
        }
    }

    /// A token's length at the drawing's zoom.
    fn l(&self, token: Token) -> Pixels {
        self.look.length(token) * self.z
    }

    fn n(&self, n: f32) -> Pixels {
        px(n * self.z)
    }

    fn weight(&self, token: Token) -> FontWeight {
        FontWeight(self.look.number(token))
    }

    fn gap(&self, gap: ScreenGap) -> Pixels {
        match gap {
            ScreenGap::None => px(0.),
            ScreenGap::Sm => self.l(token::SPACE_2),
            ScreenGap::Md => self.l(token::SPACE_4),
            ScreenGap::Lg => self.l(token::SPACE_6),
        }
    }

    fn border(&self) -> Pixels {
        self.l(token::BORDER_WIDTH)
    }

    fn radius(&self) -> Pixels {
        self.l(token::RADIUS_SM)
    }

    /// `--text-2xs` with its leading.
    fn xs2(&self, el: Div) -> Div {
        el.text_size(self.l(token::TEXT_2XS)).line_height(self.l(token::LEADING_2XS))
    }

    fn glyph(&self, name: &str, size: f32, color: Hsla) -> AnyElement {
        svg().path(icon_path(name)).flex_none().size(self.n(size)).text_color(color).into_any_element()
    }
}

/// The sheet `.armada-screen` draws: `tree` at `at`, the step playing when this screen holds it.
pub(super) fn sheet(tree: &ScreenElement, at: Option<f64>, alpha: f32, paper: &Paper) -> AnyElement {
    div()
        .when(alpha < 1., |el| el.opacity(alpha))
        .flex()
        .flex_col()
        .w(paper.n(paper.width))
        .bg(paper.paper)
        .text_color(paper.ink)
        .font_family(paper.sans.clone())
        .text_size(paper.l(token::TEXT_SM))
        .line_height(paper.l(token::LEADING_SM))
        .font_weight(paper.weight(token::WEIGHT_BODY))
        .rounded(paper.radius())
        .child(part(tree, Ctx { at, inside: false, axis: None, ends: false }, paper))
        .into_any_element()
}

fn kids(children: &[ScreenElement], ctx: Ctx, axis: Axis, paper: &Paper) -> Vec<AnyElement> {
    let last = children.len().saturating_sub(1);
    children.iter().enumerate().map(|(at, one)| part(one, Ctx { axis: Some(axis), ends: at == last && at > 0, ..ctx }, paper)).collect()
}

/// A button, or a bar's item: the words, the glyph, or both.
fn act(one: &ScreenAct, variant: ScreenVariant, compact: bool, paper: &Paper) -> Div {
    let words = one.text();
    let icon_only = words.is_none();
    let el = div()
        .flex()
        .items_center()
        .justify_center()
        .gap(paper.l(token::SPACE_2))
        .min_h(if compact { paper.l(token::SPACE_6) } else { paper.l(token::H_CONTROL_SM) })
        .px(if icon_only { px(0.) } else if compact { paper.l(token::SPACE_2) } else { paper.l(token::SPACE_3) })
        .when(icon_only, |el| el.min_w(if compact { paper.l(token::SPACE_6) } else { paper.l(token::H_CONTROL_SM) }))
        .border(paper.border())
        .rounded(paper.radius())
        .font_weight(paper.weight(token::WEIGHT_MEDIUM))
        .whitespace_nowrap();
    let el = if compact { paper.xs2(el) } else { el };
    let (el, color) = match variant {
        ScreenVariant::Primary => (el.border_color(paper.ink).bg(paper.ink).text_color(paper.paper), paper.paper),
        ScreenVariant::Secondary => (el.border_color(paper.edge), paper.ink),
        ScreenVariant::Ghost => (el.border_color(gpui::transparent_black()), paper.ink),
    };
    el.when_some(one.icon(), |el, icon| el.child(paper.glyph(icon, if compact { 14. } else { 16. }, color)))
        .when_some(words, |el, words| el.child(words.to_string()))
}

fn same_item(one: &ScreenAct, active: Option<&str>) -> bool {
    active.is_some_and(|active| one.text().or(one.icon()) == Some(active))
}

/// The ring the step playing wears, `outline: --edge-active solid --edge-focus` offset by a border.
fn ring(paper: &Paper) -> AnyElement {
    let out = -(paper.border() + paper.l(token::EDGE_ACTIVE));
    div()
        .absolute()
        .top(out)
        .left(out)
        .right(out)
        .bottom(out)
        .border(paper.l(token::EDGE_ACTIVE))
        .border_color(paper.look.color(token::EDGE_FOCUS))
        .rounded(paper.radius() - out)
        .into_any_element()
}

/// One element and everything under it. Standing back is decided here, on the way down.
fn part(el: &ScreenElement, ctx: Ctx, paper: &Paper) -> AnyElement {
    let current = ctx.at.is_some() && el.step == ctx.at;
    let dim = ctx.at.is_some_and(|at| !ctx.inside && !el.holds(at));
    let modal = matches!(el.part, ScreenPart::Modal { .. });
    // A modal with a step comes up when its step does, and the steps before it show the screen without it.
    if modal && matches!((ctx.at, el.step), (Some(at), Some(step)) if at < step) {
        return div().absolute().into_any_element();
    }
    // Under a part that stands back, nothing else is decided: it is one faded thing.
    let down = if dim { Ctx { at: None, ..ctx } } else { Ctx { inside: ctx.inside || current, ..ctx } };
    // A modal stands over its parent; everything else is `position: relative` in the flow.
    let mut root = body(el, ctx, down, current, paper).when(!modal, |el| el.relative()).min_w(px(0.));
    root = match ctx.axis {
        Some(Axis::Row | Axis::Column) => match el.size {
            Some(ScreenSize::Word(ScreenSizeWord::Fill)) => root.flex_grow(1.).flex_shrink(1.).flex_basis(px(0.)),
            Some(ScreenSize::Weight(weight)) => root.flex_grow(weight as f32).flex_shrink(1.).flex_basis(px(0.)),
            _ => root,
        },
        None => root.flex_grow(1.),
        Some(Axis::Grid) => root,
    };
    // How a part sits across its parent: a control keeps its own size, anything with room in it stretches.
    let control = matches!(el.part, ScreenPart::Button { .. } | ScreenPart::Badge { .. } | ScreenPart::Avatar { .. });
    let sits = matches!(el.part, ScreenPart::Text { .. } | ScreenPart::Field { .. } | ScreenPart::Tabs { .. });
    root = match ctx.axis {
        Some(Axis::Column) if control => root.self_start(),
        Some(Axis::Row) if control || sits => root.self_center(),
        _ => root,
    };
    root = match el.align {
        Some(ScreenAlign::Start) => root.self_start(),
        Some(ScreenAlign::Center) => root.self_center(),
        Some(ScreenAlign::End) => root.self_end(),
        Some(ScreenAlign::Stretch) => root.self_stretch(),
        None => root,
    };
    // A modal rings its dialog, not the scrim round it.
    let root = root.when(dim, |root| root.opacity(0.3)).when(current && !modal, |root| root.child(ring(paper)));
    if paper.named && (el.id.is_some() || el.step.is_some()) {
        let id = paper.ids.get();
        paper.ids.set(id + 1);
        crate::labels::labelled(root.id(("screen-part", id)), Role::Group, el.label()).into_any_element()
    } else {
        root.into_any_element()
    }
}

/// The element's own box and what is in it, before where it sits is decided.
fn body(el: &ScreenElement, ctx: Ctx, down: Ctx, current: bool, paper: &Paper) -> Div {
    let line = paper.line;
    let b = paper.border();
    match &el.part {
        ScreenPart::Row(layout) | ScreenPart::Column(layout) | ScreenPart::Grid { layout, .. } => {
            let (frame, axis) = match &el.part {
                ScreenPart::Row(_) => (div().flex().flex_row(), Axis::Row),
                ScreenPart::Column(_) => (div().flex().flex_col(), Axis::Column),
                ScreenPart::Grid { cols, .. } => (div().grid().grid_cols(u16::from(*cols)), Axis::Grid),
                _ => unreachable!(),
            };
            frame
                .gap(paper.gap(layout.gap.unwrap_or(ScreenGap::Sm)))
                .p(paper.gap(layout.pad.unwrap_or(ScreenGap::None)))
                .when(layout.border, |el| el.border(b).border_color(line).rounded(paper.radius()))
                .children(kids(&layout.children, down, axis, paper))
        }
        ScreenPart::Window { chrome, title, url, children } => {
            let phone = *chrome == ScreenChrome::Phone;
            let frame = div()
                .flex()
                .flex_col()
                .border(b)
                .border_color(paper.edge)
                .rounded(if phone { paper.l(token::RADIUS_LG) } else { paper.radius() })
                // `aspect-ratio: 9 / 19`: the sheet's own width is known, so a phone at the root is given its height.
                .when(phone && ctx.axis.is_none(), |el| el.min_h(paper.n(paper.width * 19. / 9.)))
                .when(phone && ctx.axis.is_some(), |el| el.aspect_ratio(9. / 19.));
            frame
                .child(window_chrome(*chrome, title.as_deref(), url.as_deref(), paper))
                .child(div().relative().flex().flex_col().flex_grow(1.).min_h_0().children(kids(children, down, Axis::Column, paper)))
                .when(phone, |el| {
                    el.child(div().self_center().w(paper.l(token::SPACE_12) * 2.).h(paper.l(token::SPACE_1)).my(paper.l(token::SPACE_2)).rounded(paper.radius()).bg(paper.ink))
                })
        }
        ScreenPart::Nav { brand, items, active, actions, vertical } => nav(brand.as_deref(), items.as_deref(), active.as_deref(), actions.as_deref(), *vertical, ctx.ends, paper),
        ScreenPart::Panel { title, icon, actions, gap, pad, children } => {
            let headed = title.is_some() || icon.is_some() || actions.is_some();
            div()
                .flex()
                .flex_col()
                .border(b)
                .border_color(line)
                .rounded(paper.radius())
                .when(headed, |el| {
                    el.child(
                        head(paper)
                            .when_some(icon.as_deref(), |el, icon| el.child(paper.glyph(icon, 14., paper.ink)))
                            .child(div().flex_1().min_w_0().children(title.clone()))
                            .children(actions.iter().flatten().map(|one| act(one, ScreenVariant::Secondary, true, paper))),
                    )
                })
                .child(panel_body(gap.unwrap_or(ScreenGap::Sm), pad.unwrap_or(ScreenGap::Md), paper).children(kids(children, down, Axis::Column, paper)))
        }
        ScreenPart::Tabs { items, active } => div().flex().gap(paper.l(token::SPACE_1)).border_b(b).border_color(line).children(items.iter().map(|one| {
            let on = active.as_deref() == Some(one.as_str());
            div()
                .py(paper.l(token::SPACE_2))
                .px(paper.l(token::SPACE_4))
                .mb(-b)
                .border_b(paper.l(token::EDGE_ACTIVE))
                .border_color(if on { paper.ink } else { gpui::transparent_black() })
                .text_color(if on { paper.ink } else { paper.quiet })
                .when(on, |el| el.font_weight(paper.weight(token::WEIGHT_HEADING)))
                .child(one.clone())
        })),
        ScreenPart::List { rows } => div().flex().flex_col().border(b).border_color(line).rounded(paper.radius()).children(rows.iter().enumerate().map(|(at, one)| {
            div()
                .flex()
                .items_center()
                .gap(paper.l(token::SPACE_2))
                .py(paper.l(token::SPACE_2))
                .px(paper.l(token::SPACE_3))
                .border_l(paper.l(token::EDGE_ACTIVE))
                .border_color(if one.selected { paper.ink } else { gpui::transparent_black() })
                .when(at > 0, |el| el.child(div().absolute().top_0().left(-paper.l(token::EDGE_ACTIVE)).right_0().h(b).bg(line)))
                .when(one.selected, |el| el.bg(paper.bar).font_weight(paper.weight(token::WEIGHT_HEADING)))
                .when_some(one.icon.as_deref(), |el, icon| el.child(paper.glyph(icon, 14., paper.ink)))
                .child(div().flex_1().min_w_0().child(one.text.clone()))
                .when_some(one.meta.clone(), |el, meta| el.child(paper.xs2(div()).text_color(paper.quiet).font_weight(paper.weight(token::WEIGHT_BODY)).child(meta)))
        })),
        ScreenPart::Table { columns, rows } => {
            let cell = || div().py(paper.l(token::SPACE_2)).px(paper.l(token::SPACE_3)).min_w_0();
            let heads = columns.iter().map(|one| {
                paper.xs2(cell()).bg(paper.faint).border_b(b).border_color(paper.edge).font_weight(paper.weight(token::WEIGHT_HEADING)).child(one.clone()).into_any_element()
            });
            let cells = rows.iter().enumerate().flat_map(|(at, cells)| {
                (0..columns.len()).map(move |n| {
                    cell().when(at > 0, |el| el.border_t(b).border_color(line)).children(cells.get(n).cloned()).into_any_element()
                })
            });
            div()
                .border(b)
                .border_color(line)
                .rounded(paper.radius())
                .child(div().grid().grid_cols(columns.len().max(1) as u16).children(heads).children(cells))
        }
        ScreenPart::Button { text, variant, icon } => {
            let one = ScreenAct::Parts { text: text.clone(), icon: icon.clone() };
            div().flex().child(act(&one, variant.unwrap_or(ScreenVariant::Secondary), false, paper).flex_1())
        }
        ScreenPart::Field { label, kind, value, placeholder } => field(label.as_deref(), *kind, value.as_deref(), placeholder.as_deref(), paper),
        ScreenPart::Text { style, text, lines } => {
            let el = match style {
                ScreenText::Heading => div().text_size(paper.l(token::TEXT_XL)).line_height(paper.l(token::LEADING_XL)).font_weight(paper.weight(token::WEIGHT_HEADING)),
                ScreenText::Title => div().text_size(paper.l(token::TEXT_BASE)).line_height(paper.l(token::LEADING_BASE)).font_weight(paper.weight(token::WEIGHT_HEADING)),
                ScreenText::Caption => paper.xs2(div()).text_color(paper.quiet),
                ScreenText::Body => div(),
            };
            match text {
                Some(text) => el.child(text.clone()),
                None => {
                    let lines = lines.unwrap_or(1).max(1) as usize;
                    let (tall, widest) = match style {
                        ScreenText::Heading => (paper.l(token::SPACE_4), 0.5),
                        ScreenText::Title => (paper.l(token::SPACE_3), 0.7),
                        ScreenText::Caption => (paper.l(token::SPACE_1), 1.),
                        ScreenText::Body => (paper.l(token::SPACE_2), 1.),
                    };
                    el.flex().flex_col().gap(paper.l(token::SPACE_2)).py(paper.l(token::SPACE_1)).children((0..lines).map(|at| {
                        let share = if lines > 1 && at == lines - 1 { 0.6 } else { BAR_WIDTHS[at % BAR_WIDTHS.len()] };
                        div().h(tall).w(relative(share)).max_w(relative(widest)).rounded(paper.radius()).bg(paper.bar)
                    }))
                }
            }
        }
        ScreenPart::Image { label } => {
            let line_w = b;
            let cross = canvas(
                |_, _, _| (),
                move |bounds, _, window, _| {
                    let mut path = PathBuilder::stroke(line_w);
                    path.move_to(bounds.origin);
                    path.line_to(bounds.bottom_right());
                    path.move_to(bounds.top_right());
                    path.line_to(bounds.bottom_left());
                    if let Ok(path) = path.build() {
                        window.paint_path(path, line);
                    }
                },
            )
            .absolute()
            .inset_0()
            .size_full();
            div()
                .flex()
                .items_center()
                .justify_center()
                .min_h(paper.l(token::SPACE_12) * 2.)
                .aspect_ratio(16. / 10.)
                .border(b)
                .border_color(line)
                .bg(paper.faint)
                .child(cross)
                .when_some(label.clone(), |el, label| {
                    el.child(paper.xs2(div()).relative().py(paper.l(token::SPACE_1)).px(paper.l(token::SPACE_2)).bg(paper.paper).text_color(paper.quiet).child(label))
                })
        }
        ScreenPart::Avatar { text } => {
            let side = paper.l(token::H_CONTROL_SM);
            paper
                .xs2(div())
                .flex()
                .flex_none()
                .items_center()
                .justify_center()
                .size(side)
                .border(b)
                .border_color(paper.edge)
                .rounded_full()
                .bg(paper.faint)
                .font_weight(paper.weight(token::WEIGHT_HEADING))
                .map(|el| match text {
                    Some(text) => el.child(text.clone()),
                    None => el.child(svg().path(icon_path("user")).size(side * 0.7).text_color(paper.ink)),
                })
        }
        ScreenPart::Badge { text } => paper
            .xs2(div())
            .flex()
            .items_center()
            .px(paper.l(token::SPACE_2))
            .border(b)
            .border_color(paper.edge)
            .rounded(paper.radius())
            .font_weight(paper.weight(token::WEIGHT_MEDIUM))
            .whitespace_nowrap()
            .child(text.clone()),
        ScreenPart::Divider => {
            let el = div().flex_none().self_stretch().bg(line);
            if ctx.axis == Some(Axis::Row) {
                el.w(b)
            } else {
                el.h(b)
            }
        }
        ScreenPart::Spacer => div().flex_grow(1.).flex_shrink(1.).flex_basis(px(0.)),
        ScreenPart::Chat { messages } => div().flex().flex_col().gap(paper.l(token::SPACE_2)).min_h(paper.l(token::SPACE_12) * 3.).children(messages.iter().map(|one| {
            let me = one.from == ScreenFrom::Me;
            div()
                .max_w(relative(0.72))
                .py(paper.l(token::SPACE_2))
                .px(paper.l(token::SPACE_3))
                .border(b)
                .rounded(paper.radius())
                .map(|el| if me { el.self_end().border_color(gpui::transparent_black()).bg(paper.bar) } else { el.self_start().border_color(line) })
                .child(one.text.clone())
        })),
        ScreenPart::Modal { title, children } => {
            let dialog = div()
                .relative()
                .flex()
                .flex_col()
                .w_full()
                .max_w(paper.l(token::SPACE_12) * 9.)
                .border(b)
                .border_color(paper.edge)
                .rounded(paper.radius())
                .bg(paper.paper)
                .child(head(paper).child(div().flex_1().min_w_0().children(title.clone())).child(paper.glyph("x", 16., paper.ink)))
                .child(panel_body(ScreenGap::Sm, ScreenGap::Md, paper).children(kids(children, down, Axis::Column, paper)))
                .when(current, |el| el.child(ring(paper)));
            div().absolute().inset_0().flex().items_center().justify_center().p(paper.l(token::SPACE_4)).bg(paper.ink.opacity(0.5)).child(dialog)
        }
    }
}

/// A panel's or a dialog's head band.
fn head(paper: &Paper) -> Div {
    div()
        .flex()
        .items_center()
        .gap(paper.l(token::SPACE_2))
        .py(paper.l(token::SPACE_2))
        .px(paper.l(token::SPACE_3))
        .border_b(paper.border())
        .border_color(paper.line)
        .rounded_t(paper.radius())
        .bg(paper.faint)
        .text_size(paper.l(token::TEXT_XS))
        .line_height(paper.l(token::LEADING_XS))
        .font_weight(paper.weight(token::WEIGHT_HEADING))
}

fn panel_body(gap: ScreenGap, pad: ScreenGap, paper: &Paper) -> Div {
    div().relative().flex().flex_col().flex_grow(1.).min_h_0().gap(paper.gap(gap)).p(paper.gap(pad))
}

/// A window's frame above its body: a desktop's dots and title, a browser's tab and address, a phone's status bar.
fn window_chrome(chrome: ScreenChrome, title: Option<&str>, url: Option<&str>, paper: &Paper) -> Div {
    let b = paper.border();
    let band = paper.xs2(div()).flex().flex_col().border_b(b).border_color(paper.line).bg(paper.faint);
    match chrome {
        ScreenChrome::Desktop => {
            let dot = || div().size(paper.l(token::SPACE_3)).border(b).border_color(paper.edge).rounded_full();
            band.flex_row()
                .items_center()
                .py(paper.l(token::SPACE_2))
                .px(paper.l(token::SPACE_3))
                .child(div().flex_1().flex().gap(paper.l(token::SPACE_2)).child(dot()).child(dot()).child(dot()))
                .child(div().text_color(paper.quiet).font_weight(paper.weight(token::WEIGHT_MEDIUM)).children(title.map(str::to_string)))
                .child(div().flex_1())
        }
        ScreenChrome::Browser => band
            .child(
                div().flex().pt(paper.l(token::SPACE_2)).px(paper.l(token::SPACE_3)).bg(paper.bar).child(
                    div()
                        .min_w(paper.l(token::SPACE_12) * 3.)
                        .py(paper.l(token::SPACE_1))
                        .px(paper.l(token::SPACE_4))
                        .border(b)
                        .border_b(px(0.))
                        .border_color(paper.line)
                        .rounded_t(paper.radius())
                        .bg(paper.paper)
                        .font_weight(paper.weight(token::WEIGHT_MEDIUM))
                        .child(title.unwrap_or(" ").to_string()),
                ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(paper.l(token::SPACE_3))
                    .py(paper.l(token::SPACE_2))
                    .px(paper.l(token::SPACE_3))
                    .bg(paper.paper)
                    .text_color(paper.quiet)
                    .child(paper.glyph("chevron-left", 16., paper.quiet))
                    .child(paper.glyph("chevron-right", 16., paper.quiet))
                    .child(paper.glyph("refresh-cw", 14., paper.quiet))
                    .child(
                        div()
                            .flex()
                            .flex_1()
                            .items_center()
                            .gap(paper.l(token::SPACE_2))
                            .py(paper.l(token::SPACE_1))
                            .px(paper.l(token::SPACE_3))
                            .border(b)
                            .border_color(paper.line)
                            .rounded(paper.radius())
                            .text_color(paper.ink)
                            .child(paper.glyph("lock", 12., paper.ink))
                            .children(url.map(str::to_string)),
                    ),
            ),
        ScreenChrome::Phone => {
            let signal = |tall: Token| div().w(paper.l(token::SPACE_1)).h(paper.l(tall)).bg(paper.ink);
            paper
                .xs2(div())
                .flex()
                .items_center()
                .justify_between()
                .py(paper.l(token::SPACE_2))
                .px(paper.l(token::SPACE_6))
                .bg(paper.paper)
                .font_weight(paper.weight(token::WEIGHT_HEADING))
                .child("9:41")
                .child(
                    div()
                        .flex()
                        .items_end()
                        .gap(paper.l(token::SPACE_1))
                        .child(signal(token::SPACE_2))
                        .child(signal(token::SPACE_3))
                        .child(signal(token::SPACE_4))
                        .child(
                            div()
                                .w(paper.l(token::SPACE_6))
                                .h(paper.l(token::SPACE_3))
                                .ml(paper.l(token::SPACE_2))
                                .border(b)
                                .border_color(paper.ink)
                                .rounded(paper.radius())
                                .child(div().w(relative(0.7)).h_full().bg(paper.ink)),
                        ),
                )
        }
    }
}

/// A top, side or bottom bar: its brand, its items with the active one filled, and its acts.
fn nav(brand: Option<&str>, items: Option<&[ScreenAct]>, active: Option<&str>, actions: Option<&[ScreenAct]>, vertical: bool, ends: bool, paper: &Paper) -> Div {
    let b = paper.border();
    // A bar that ends its container is a bottom bar: its rule is above it and its items share the width, each glyph over its words.
    let bottom = !vertical && ends;
    let frame = div().flex().gap(paper.l(token::SPACE_4)).border_color(paper.line);
    let frame = if vertical {
        frame.flex_col().gap(paper.l(token::SPACE_3)).py(paper.l(token::SPACE_3)).px(paper.l(token::SPACE_2)).border_r(b)
    } else {
        frame.items_center().py(paper.l(token::SPACE_2)).px(paper.l(token::SPACE_4)).map(|el| if bottom { el.border_t(b) } else { el.border_b(b) })
    };
    let brand = brand.map(|brand| {
        let el = div().flex().items_center().gap(paper.l(token::SPACE_2)).font_weight(paper.weight(token::WEIGHT_HEADING));
        if vertical {
            paper.xs2(el).px(paper.l(token::SPACE_3)).text_color(paper.quiet).child(brand.to_string())
        } else {
            el.child(div().size(paper.l(token::SPACE_4)).bg(paper.ink).rounded(paper.radius())).child(brand.to_string())
        }
    });
    let items = items.map(|items| {
        let row = div().flex().gap(paper.l(token::SPACE_1));
        let row = if vertical { row.flex_col() } else { row.items_center() };
        row.when(bottom, |el| el.flex_1().justify_around()).children(items.iter().map(|one| {
            let on = same_item(one, active);
            let bare = one.text().is_none();
            let el = div()
                .flex()
                .items_center()
                .gap(paper.l(token::SPACE_2))
                .py(paper.l(token::SPACE_1))
                .px(paper.l(token::SPACE_3))
                .rounded(paper.radius())
                .whitespace_nowrap()
                .text_color(if on { paper.ink } else { paper.quiet })
                .when(on, |el| el.bg(paper.bar).font_weight(paper.weight(token::WEIGHT_HEADING)))
                .when(vertical && bare, |el| el.justify_center().p(paper.l(token::SPACE_2)))
                .when(bottom, |el| paper.xs2(el.flex_col().gap(px(0.))));
            let color = if on { paper.ink } else { paper.quiet };
            el.when_some(one.icon(), |el, icon| el.child(paper.glyph(icon, 16., color))).when_some(one.text(), |el, text| el.child(text.to_string()))
        }))
    });
    let actions = actions.map(|actions| {
        let row = div().flex().gap(paper.l(token::SPACE_1));
        let row = if vertical { row.flex_col().mt_auto() } else { row.items_center().ml_auto() };
        row.children(actions.iter().map(|one| act(one, ScreenVariant::Ghost, false, paper)))
    });
    frame.children(brand).children(items).children(actions)
}

fn field(label: Option<&str>, kind: ScreenField, value: Option<&str>, placeholder: Option<&str>, paper: &Paper) -> Div {
    let b = paper.border();
    let frame = div().flex().flex_col().gap(paper.l(token::SPACE_1));
    if matches!(kind, ScreenField::Checkbox | ScreenField::Toggle) {
        let on = value == Some("on");
        let side = paper.l(token::SPACE_4);
        let mark = div().flex().items_center().border(b).border_color(paper.edge).h(side).when(on, |el| el.bg(paper.ink).text_color(paper.paper));
        let mark = if kind == ScreenField::Checkbox {
            mark.justify_center().w(side).rounded(paper.radius()).when(on, |el| el.child(svg().path(icon_path("check")).size(paper.n(12.)).text_color(paper.paper)))
        } else {
            let knob = side - b * 3.;
            mark.w(paper.l(token::SPACE_8))
                .p(b)
                .rounded(side)
                .map(|el| if on { el.justify_end() } else { el.justify_start() })
                .child(div().size(knob).rounded_full().bg(if on { paper.paper } else { paper.ink }))
        };
        return frame.child(div().flex().items_center().gap(paper.l(token::SPACE_2)).child(mark).children(label.map(str::to_string)));
    }
    let shown = value.or(placeholder);
    let tall = match kind {
        ScreenField::Textarea => paper.l(token::LEADING_SM) * 3. + paper.l(token::SPACE_2) + b * 2.,
        _ => paper.l(token::H_CONTROL_SM),
    };
    let control = div()
        .flex()
        .map(|el| if kind == ScreenField::Textarea { el.items_start() } else { el.items_center() })
        .gap(paper.l(token::SPACE_2))
        .min_h(tall)
        .py(paper.l(token::SPACE_1))
        .px(paper.l(token::SPACE_3))
        .border(b)
        .border_color(paper.edge)
        .rounded(paper.radius())
        .bg(paper.paper)
        .when(value.is_none(), |el| el.text_color(paper.quiet))
        .when(kind == ScreenField::Search, |el| el.child(paper.glyph("search", 14., paper.quiet)))
        .child(div().flex_1().min_w_0().child(shown.unwrap_or(" ").to_string()))
        .when(kind == ScreenField::Select, |el| el.child(paper.glyph("chevron-down", 14., paper.ink)));
    frame
        .when_some(label, |el, label| el.child(paper.xs2(div()).font_weight(paper.weight(token::WEIGHT_MEDIUM)).child(label.to_string())))
        .child(control)
}
