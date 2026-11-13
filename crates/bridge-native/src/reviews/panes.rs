//! A pull request's page on free panes, as Classic's tree draws them: any tab dragged to another
//! pane, or onto a pane's edge to split it, either way and nested, and every split resized by its gap.

use std::rc::Rc;

use gpui::{canvas, div, prelude::*, relative, AnyElement, Context, DragMoveEvent, FontWeight, Role, SharedString, Window};

use super::model::Page;
use super::{panel, said, tabs, ReviewsScreen};
use crate::job_view::pane_tree::{self as tree, Dir, Node, Pane, Side, Size, Zone};
use crate::labels::labelled;
use crate::look::{token, ActiveLook, Look};
use crate::titlebar::parts::{glyph, mono, small, tip};

/// A tab being dragged.
#[derive(Clone)]
pub(super) struct TabDrag {
    tab: String,
    label: SharedString,
}

/// A split's gap being dragged.
#[derive(Clone)]
struct GapDrag(String);

/// The tab under the pointer while it is dragged.
struct TabGhost(SharedString);

impl Render for TabGhost {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let look = cx.look();
        div()
            .px(look.length(token::SPACE_3))
            .py(look.length(token::SPACE_1))
            .rounded(look.length(token::RADIUS_LG))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::ACCENT))
            .bg(look.color(token::BG_RAISED))
            .shadow(look.shadows(token::SHADOW_OVERLAY))
            .text_size(look.length(token::TEXT_XS))
            .text_color(look.color(token::FG_DEFAULT))
            .child(self.0.clone())
    }
}

impl Render for GapDrag {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}

/// Where on a pane a point is: its tab strip or middle, or the edge it is nearest within a quarter.
fn zone_at(fx: f32, fy: f32, on_strip: bool) -> Zone {
    if on_strip {
        return Zone::Middle;
    }
    let near = [(fx, Side::Left), (1. - fx, Side::Right), (fy, Side::Top), (1. - fy, Side::Bottom)];
    let (by, side) = near.into_iter().fold((f32::MAX, Side::Left), |best, one| if one.0 < best.0 { one } else { best });
    if by < 0.25 { Zone::Edge(side) } else { Zone::Middle }
}

impl ReviewsScreen {
    /// The page: the panes of the open pull request, its skeleton while it is read, or the empty place.
    pub(super) fn render_page(&mut self, look: &Look, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let area = Rc::clone(&self.area);
        let measure = canvas(move |bounds, _, _| area.set(Some(Size { w: f64::from(f32::from(bounds.size.width)), h: f64::from(f32::from(bounds.size.height)) })), |_, _, _, _| {})
            .absolute()
            .top_0()
            .left_0()
            .size_full();
        let shown: AnyElement = match (self.page().cloned(), self.waiting.or(if self.inbox.refreshing { self.restore } else { None })) {
            (_, Some(number)) => self.skeleton(number, look),
            (Some(page), None) => {
                let root = self.arrangement.root.clone();
                self.node(&root, &page, look, window, cx)
            }
            (None, None) => labelled(
                panel(look)
                    .id("page-none")
                    .size_full()
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_size(look.length(token::TEXT_SM))
                    .text_color(look.color(token::FG_MUTED))
                    .child("No pull request open"),
                Role::Status,
                "No pull request open",
            )
            .into_any_element(),
        };
        div().id("page").relative().flex_1().min_w_0().h_full().child(measure).child(shown).into_any_element()
    }

    /// While its fresh page is read: the panes' shape in grey, and what it waits on.
    fn skeleton(&self, number: u64, look: &Look) -> AnyElement {
        let said = if self.waiting == Some(number) { format!("Reading #{number}") } else { format!("#{number} opens once the refresh confirms it") };
        let bar = |id: String, share: f32| {
            div().id(SharedString::from(id)).h(look.length(token::SPACE_3)).w(relative(share)).rounded(look.length(token::RADIUS_SM)).bg(look.color(token::BG_SUNKEN))
        };
        let column = |name: &'static str, shares: &[f32]| {
            panel(look)
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_3))
                .p(look.length(token::SPACE_4))
                .child(div().h(look.length(token::H_CONTROL)).w_full().rounded(look.length(token::RADIUS_SM)).bg(look.color(token::BG_SUNKEN)))
                .children(shares.iter().enumerate().map(|(at, share)| bar(format!("{name}-{at}"), *share)))
        };
        let status = labelled(
            div()
                .id("page-reading")
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .text_size(look.length(token::TEXT_SM))
                .text_color(look.color(token::FG_MUTED))
                .child(glyph("hourglass", look.length(token::SPACE_4), look.color(token::STATUS_RUNNING)))
                .child(SharedString::from(said.clone())),
            Role::Status,
            said,
        );
        div()
            .size_full()
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_2))
            .child(status)
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .gap(look.length(token::GAP_PANEL))
                    .child(column("main", &[0.9, 0.7, 0.8, 0.5, 0.75, 0.6]).w(relative(0.64)))
                    .child(column("side", &[0.8, 0.6, 0.7]).flex_1()),
            )
            .into_any_element()
    }

    fn node(&mut self, node: &Node, page: &Page, look: &Look, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let split = match node {
            Node::Pane(pane) => return self.pane(pane, page, look, window, cx),
            Node::Split(split) => split,
        };
        let a = self.node(&split.a, page, look, window, cx);
        let b = self.node(&split.b, page, look, window, cx);
        let row = split.dir == Dir::Row;
        let ratio = split.ratio as f32;
        let gap = look.length(token::GAP_PANEL);
        let id = split.id.clone();
        let moving = id.clone();
        let line = look.color(token::ACCENT);
        let handle = labelled(
            div()
                .id(SharedString::from(format!("gap-{id}")))
                .absolute()
                .when(row, |handle| handle.top_0().bottom_0().left(relative(ratio)).w(gap).cursor(gpui::CursorStyle::ResizeLeftRight))
                .when(!row, |handle| handle.left_0().right_0().top(relative(ratio)).h(gap).cursor(gpui::CursorStyle::ResizeUpDown))
                .flex()
                .items_center()
                .justify_center()
                .group("pane-gap")
                .on_drag(GapDrag(id.clone()), |drag, _, _, cx| cx.new(|_| drag.clone()))
                .child(
                    div()
                        .when(row, |mark| mark.h_full().w(look.length(token::BORDER_WIDTH) * 2.))
                        .when(!row, |mark| mark.w_full().h(look.length(token::BORDER_WIDTH) * 2.))
                        .rounded_full()
                        .group_hover("pane-gap", move |style| style.bg(line)),
                ),
            Role::Splitter,
            "Resize panes",
        );
        div()
            .id(SharedString::from(format!("split-{id}")))
            .relative()
            .size_full()
            .min_w_0()
            .min_h_0()
            .flex()
            .when(!row, |split| split.flex_col())
            .gap(gap)
            .on_drag_move(cx.listener(move |this, event: &DragMoveEvent<GapDrag>, _, cx| {
                if event.drag(cx).0 != moving {
                    return;
                }
                let bounds = event.bounds;
                let to = if row {
                    f32::from(event.event.position.x - bounds.origin.x) / f32::from(bounds.size.width)
                } else {
                    f32::from(event.event.position.y - bounds.origin.y) / f32::from(bounds.size.height)
                };
                let next = tree::resize_split(&this.arrangement, &moving, f64::from(to));
                if next != this.arrangement && tree::allowed(this.area.get(), &this.arrangement, &next) {
                    this.arrangement = next;
                    cx.notify();
                }
            }))
            .child(
                div()
                    .flex_none()
                    .min_w_0()
                    .min_h_0()
                    .when(row, |side| side.h_full().w(relative(ratio)))
                    .when(!row, |side| side.w_full().h(relative(ratio)))
                    .child(a),
            )
            .child(div().flex_1().min_w_0().min_h_0().when(row, |side| side.h_full()).when(!row, |side| side.w_full()).child(b))
            .child(handle)
            .into_any_element()
    }

    fn pane(&mut self, pane: &Pane, page: &Page, look: &Look, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let labels: Vec<&str> = pane.tabs.iter().map(|tab| said(&tab.kind)).collect();
        let name = if labels.is_empty() { "Pane: empty".to_string() } else { format!("Pane: {}", labels.join(", ")) };
        let strip_h = look.length(token::H_CONTROL_LG);
        let active = pane.active.clone().or_else(|| pane.tabs.first().map(|tab| tab.id.clone()));
        let tabs: Vec<AnyElement> = pane.tabs.iter().map(|tab| self.tab(tab, active.as_deref() == Some(&tab.id), look, cx)).collect();
        let add_name = "Add tab";
        let add_pane = pane.id.clone();
        let muted = look.color(token::FG_MUTED);
        let hover = look.color(token::BG_HOVER);
        let add = labelled(
            div()
                .id(SharedString::from(format!("add-{}", pane.id)))
                .flex_none()
                .flex()
                .items_center()
                .justify_center()
                .size(look.length(token::H_CONTROL))
                .rounded(look.length(token::RADIUS_MD))
                .cursor_pointer()
                .hover(move |style| style.bg(hover))
                .tooltip(tip("Add tab"))
                .on_click(cx.listener(move |this, event: &gpui::ClickEvent, window, cx| {
                    let at = event.position();
                    this.open_add(add_pane.clone(), at, window, cx);
                }))
                .child(glyph("plus", look.length(token::SPACE_4), muted)),
            Role::Button,
            add_name,
        );
        let strip = div()
            .id(SharedString::from(format!("strip-{}", pane.id)))
            .flex_none()
            .h(strip_h)
            .flex()
            .items_end()
            .pr(look.length(token::SPACE_1))
            .bg(look.color(token::BG_SUNKEN))
            .border_b(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_DEFAULT))
            .child(labelled(div().id(SharedString::from(format!("tabs-{}", pane.id))).flex().h_full().min_w_0().overflow_x_hidden().children(tabs), Role::TabList, name.clone()))
            .child(div().flex_1())
            .child(div().h_full().flex().items_center().child(add));
        let kind = pane.tabs.iter().find(|tab| Some(&tab.id) == active.as_ref()).map(|tab| tab.kind.clone());
        let body: AnyElement = match kind {
            Some(kind) => self.tab_body(&kind, page, look, window, cx),
            None => div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .text_size(look.length(token::TEXT_SM))
                .text_color(muted)
                .child("Drag a tab here, or add one with +")
                .into_any_element(),
        };
        let dragging = cx.has_active_drag();
        let overlay = self.over.as_ref().filter(|(at, _, can)| dragging && *can && *at == pane.id).map(|(_, zone, _)| {
            let wash = div().absolute().bg(look.color(token::SELECT_TINT)).border(look.length(token::BORDER_WIDTH) * 2.).border_color(look.color(token::ACCENT)).rounded(look.length(token::RADIUS_LG));
            match zone {
                Zone::Middle => wash.inset_0(),
                Zone::Edge(Side::Left) => wash.top_0().bottom_0().left_0().w(relative(0.5)),
                Zone::Edge(Side::Right) => wash.top_0().bottom_0().right_0().w(relative(0.5)),
                Zone::Edge(Side::Top) => wash.left_0().right_0().top_0().h(relative(0.5)),
                Zone::Edge(Side::Bottom) => wash.left_0().right_0().bottom_0().h(relative(0.5)),
            }
        });
        let over_pane = pane.id.clone();
        let drop_pane = pane.id.clone();
        let focused = self.arrangement.focus == pane.id;
        labelled(
            panel(look)
                .id(SharedString::from(format!("pane-{}", pane.id)))
                .relative()
                .size_full()
                .flex()
                .flex_col()
                .when(focused, |frame| frame.border_color(look.color(token::BORDER_STRONG)))
                .on_drag_move(cx.listener(move |this, event: &DragMoveEvent<TabDrag>, _, cx| {
                    let (bounds, at) = (event.bounds, event.event.position);
                    if !bounds.contains(&at) {
                        return;
                    }
                    let fx = f32::from(at.x - bounds.origin.x) / f32::from(bounds.size.width);
                    let fy = f32::from(at.y - bounds.origin.y) / f32::from(bounds.size.height);
                    let zone = zone_at(fx, fy, at.y - bounds.origin.y < strip_h);
                    let tab = event.drag(cx).tab.clone();
                    let can = tree::droppable(&this.arrangement, this.area.get(), &tab, &over_pane, zone);
                    let now = Some((over_pane.clone(), zone, can));
                    if this.over != now {
                        this.over = now;
                        this.lifted = Some(tab);
                        cx.notify();
                    }
                }))
                .on_drop(cx.listener(move |this, drag: &TabDrag, _, cx| this.drop_tab(&drag.tab, &drop_pane, cx)))
                .child(strip)
                .child(div().id(SharedString::from(format!("body-{}", pane.id))).flex_1().min_h_0().overflow_y_scroll().child(body))
                .children(overlay),
            Role::Region,
            name,
        )
        .into_any_element()
    }

    fn tab(&self, tab: &tree::Tab, front: bool, look: &Look, cx: &mut Context<Self>) -> AnyElement {
        let label = said(&tab.kind);
        let id = tab.id.clone();
        let lifted = cx.has_active_drag() && self.lifted.as_deref() == Some(&tab.id);
        let ink = if front { look.color(token::FG_DEFAULT) } else { look.color(token::FG_MUTED) };
        let hover = look.color(token::FG_DEFAULT);
        let drag = TabDrag { tab: tab.id.clone(), label: label.into() };
        labelled(
            div()
                .id(SharedString::from(format!("tab-{}", tab.id)))
                .flex_none()
                .h_full()
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .px(look.length(token::SPACE_3))
                .text_size(look.length(token::TEXT_XS))
                .text_color(ink)
                .border_r(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::BORDER_SUBTLE))
                .when(front, |tab| {
                    tab.bg(look.color(token::BG_RAISED))
                        .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                        .border_t(look.length(token::BORDER_WIDTH) * 2.)
                        .border_color(look.color(token::ACCENT))
                })
                .when(lifted, |tab| tab.opacity(0.5))
                .cursor_pointer()
                .hover(move |style| style.text_color(hover))
                .aria_selected(front)
                .on_drag(drag, |drag, _, _, cx| cx.new(|_| TabGhost(drag.label.clone())))
                .on_click(cx.listener(move |this, _, _, cx| {
                    if let Some(next) = tree::rearrange(&this.arrangement, |was| tree::activate(was, &id), None, true) {
                        this.arrangement = next;
                        cx.notify();
                    }
                }))
                .child(glyph(tabs::glyph_of(&tab.kind), look.length(token::SPACE_4), if front { look.color(token::ACCENT) } else { ink }))
                .child(label),
            Role::Tab,
            label,
        )
        .into_any_element()
    }

    /// A tab let go over a pane: into it, or onto the edge it was nearest, splitting the pane.
    fn drop_tab(&mut self, tab: &str, pane: &str, cx: &mut Context<Self>) {
        let zone = self.over.as_ref().filter(|(at, _, _)| at == pane).map_or(Zone::Middle, |(_, zone, _)| *zone);
        let next = match zone {
            Zone::Middle => tree::rearrange(&self.arrangement, |was| tree::move_tab_to_pane(was, tab, pane, None), self.area.get(), false),
            Zone::Edge(side) => tree::rearrange(&self.arrangement, |was| tree::move_tab_to_edge(was, tab, pane, side), self.area.get(), false),
        };
        if let Some(next) = next {
            self.arrangement = next;
        }
        self.over = None;
        self.lifted = None;
        cx.notify();
    }
}

/// A small mono line, as file places and ranges are set.
pub(super) fn place_line(id: String, words: String, look: &Look) -> AnyElement {
    labelled(
        small(look).id(SharedString::from(id)).font_family(mono(look)).text_color(look.color(token::FG_MUTED)).child(SharedString::from(words.clone())),
        Role::Label,
        words,
    )
    .into_any_element()
}
