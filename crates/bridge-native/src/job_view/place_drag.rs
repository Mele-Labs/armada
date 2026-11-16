//! A tab dragged to another place, as Classic's `PaneView` and `JobPanes` take it: dropped in a strip it goes
//! between the tabs either side of the pointer; dropped on a pane's body it joins the pane in its middle, or
//! splits the pane on the side it is nearest (the bands a quarter deep, top and bottom winning the corners);
//! dropped in the gap between two panes it gets a pane of its own there; dropped on an outer edge of the area
//! it gets a pane along that whole edge. Each drop is one step back for ⌘Z.
//!
//! **Where the pointer is comes from where things were drawn.** Each strip, tab, body and gap records its
//! box as it is laid out; one listener on the whole area reads them, so the outer edges win over a gap, a gap
//! over a strip, and a strip over a body, whichever element hears the pointer first.
//!
//! **A drop that would change nothing, or leave a pane under its least size, is refused** (Classic's
//! `droppable`): nothing is lit and the pointer says so. Esc lets go of the drag.

use std::borrow::Cow;

use gpui::{canvas, div, prelude::*, relative, AnyElement, Bounds, Context, CursorStyle, DragMoveEvent, KeyDownEvent, Pixels, Point, SharedString, Window};

use super::pane_tree::{self as tree, Arrangement, Side, Size, Zone};
use super::JobView;
use crate::assets::icon_path;
use crate::look::{token, ActiveLook, Look};

/// What a drag of a tab carries: the tab, and what its picture beside the pointer reads.
#[derive(Clone)]
pub(super) struct TabDrag {
    pub tab: String,
    pub word: SharedString,
    pub icon: &'static str,
    /// Where in the tab it was taken, which gpui sets the picture back by; the picture stands below and right of the
    /// pointer instead, so it hides no bar or shade it would land on.
    pub grabbed: Point<Pixels>,
}

impl TabDrag {
    pub fn new(tab: String, word: SharedString, icon: &'static str) -> Self {
        TabDrag { tab, word, icon, grabbed: Point::default() }
    }

    /// The picture a drag of this tab draws, taken at `grabbed`.
    pub fn lifted(&self, grabbed: Point<Pixels>, cx: &mut gpui::App) -> gpui::Entity<TabDrag> {
        let mut drag = self.clone();
        drag.grabbed = grabbed;
        cx.new(|_| drag)
    }
}

impl gpui::Render for TabDrag {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let look = cx.look().clone();
        let beside = look.length(token::SPACE_3);
        let chip = div()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .px(look.length(token::SPACE_3))
            .h(look.length(token::H_CONTROL_SM))
            .rounded(look.length(token::RADIUS_MD))
            .bg(look.color(token::BG_RAISED))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_STRONG))
            .text_size(look.length(token::TEXT_XS))
            .text_color(look.color(token::FG_DEFAULT))
            .opacity(0.9)
            .child(gpui::svg().path(icon_path(self.icon)).size(look.length(token::SPACE_4)).text_color(look.color(token::FG_DEFAULT)))
            .child(self.word.clone());
        div().pl(self.grabbed.x + beside).pt(self.grabbed.y + beside).child(chip)
    }
}

/// Where a dragged tab would go.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum Target {
    /// Among a pane's tabs, at this place among the ones left once the tab is out.
    Strip { pane: String, index: usize },
    /// On a pane's body: its middle, or one of its sides.
    Zone { pane: String, zone: Zone },
    /// The gap of a split.
    Gap { split: String },
    /// An outer edge of the whole area.
    End { side: Side },
}

impl Target {
    /// The pane the pointer is over, for a strip or a body.
    pub fn pane(&self) -> Option<&str> {
        match self {
            Target::Strip { pane, .. } | Target::Zone { pane, .. } => Some(pane),
            _ => None,
        }
    }
}

/// A tab being dragged, and where it is over.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Hover {
    pub tab: String,
    pub target: Option<Target>,
    /// The drop would land.
    pub ok: bool,
}

/// The arrangement a drop at `target` makes, or None where it is refused.
pub(super) fn landed(was: &Arrangement, size: Option<Size>, tab: &str, target: &Target) -> Option<Arrangement> {
    tree::rearrange(
        was,
        |arrangement| match target {
            Target::Strip { pane, index } => tree::move_tab_to_pane(arrangement, tab, pane, Some(*index)),
            Target::Zone { pane, zone: Zone::Middle } => tree::move_tab_to_pane(arrangement, tab, pane, None),
            Target::Zone { pane, zone: Zone::Edge(side) } => tree::move_tab_to_edge(arrangement, tab, pane, *side),
            Target::Gap { split } => tree::move_tab_to_gap(arrangement, tab, split),
            Target::End { side } => tree::move_tab_to_end(arrangement, tab, *side),
        },
        size,
        false,
    )
}

/// The zone of a body a point is in: bands a quarter deep on each side, top and bottom across the whole width.
pub(super) fn zone_in(body: Bounds<Pixels>, at: Point<Pixels>) -> Zone {
    let (w, h) = (f32::from(body.size.width).max(1.), f32::from(body.size.height).max(1.));
    let x = f32::from(at.x - body.origin.x) / w;
    let y = f32::from(at.y - body.origin.y) / h;
    if y < 0.25 {
        Zone::Edge(Side::Top)
    } else if y > 0.75 {
        Zone::Edge(Side::Bottom)
    } else if x < 0.25 {
        Zone::Edge(Side::Left)
    } else if x > 0.75 {
        Zone::Edge(Side::Right)
    } else {
        Zone::Middle
    }
}

/// The outer edge a point is on: within half a gap of the area's edge.
pub(super) fn end_at(area: Bounds<Pixels>, at: Point<Pixels>, half: Pixels) -> Option<Side> {
    if at.x < area.origin.x + half {
        Some(Side::Left)
    } else if at.x > area.origin.x + area.size.width - half {
        Some(Side::Right)
    } else if at.y < area.origin.y + half {
        Some(Side::Top)
    } else if at.y > area.origin.y + area.size.height - half {
        Some(Side::Bottom)
    } else {
        None
    }
}

impl JobView {
    /// An element's box, recorded under `key` as it is laid out. Its parent is `relative`.
    pub(super) fn mark_drawn(&self, key: String) -> impl IntoElement {
        let drawn = self.arranged.drawn_at.clone();
        canvas(move |bounds, _, _| drawn.borrow_mut().insert(key, bounds), |_, _, _, _| {}).absolute().size_full()
    }

    /// The tab being dragged, while one is.
    pub(super) fn dragged_tab(&self) -> Option<&str> {
        self.arranged.drop.as_ref().map(|hover| hover.tab.as_str())
    }

    /// Where a drop would land, while it would.
    pub(super) fn drop_lit(&self) -> Option<&Target> {
        self.arranged.drop.as_ref().filter(|hover| hover.ok).and_then(|hover| hover.target.as_ref())
    }

    /// Where the pointer is, read from the boxes drawn: an outer edge, a gap, a strip, a body.
    fn target_at(&self, tab: &str, at: Point<Pixels>, area: Bounds<Pixels>, look: &Look) -> Option<Target> {
        if !area.contains(&at) {
            return None;
        }
        if self.zoomed().is_none() {
            if let Some(side) = end_at(area, at, look.length(token::GAP_PANEL) / 2.) {
                return Some(Target::End { side });
            }
        }
        let drawn = self.arranged.drawn_at.borrow();
        let hit = |prefix: &str| drawn.iter().find(|(key, bounds)| key.starts_with(prefix) && bounds.contains(&at)).map(|(key, bounds)| (key[prefix.len()..].to_string(), *bounds));
        if let Some((split, _)) = hit("gap:") {
            return Some(Target::Gap { split });
        }
        let arrangement = self.arrangement();
        if let Some((pane, _)) = hit("strip:") {
            let one = tree::pane_by_id(&arrangement, &pane)?;
            let index = one.tabs.iter().filter(|one| one.id != tab).filter(|one| drawn.get(&format!("tab:{}", one.id)).is_some_and(|bounds| bounds.center().x < at.x)).count();
            return Some(Target::Strip { pane, index });
        }
        if let Some((pane, body)) = hit("body:") {
            return Some(Target::Zone { pane, zone: zone_in(body, at) });
        }
        None
    }

    /// The pointer moved while a tab is dragged: what it is over now, and whether a drop there lands.
    pub(super) fn drag_tab_over(&mut self, event: &DragMoveEvent<TabDrag>, window: &mut Window, cx: &mut Context<Self>) {
        let tab = event.drag(cx).tab.clone();
        let look = cx.look().clone();
        let target = self.target_at(&tab, event.event.position, event.bounds, &look);
        let size = self.arranged.area.get();
        let ok = target.as_ref().is_some_and(|target| landed(&self.arrangement(), size, &tab, target).is_some());
        let hover = Some(Hover { tab, target, ok });
        if self.arranged.drop != hover {
            let refused = hover.as_ref().is_some_and(|hover| hover.target.is_some() && !hover.ok);
            cx.set_active_drag_cursor_style(if refused { CursorStyle::OperationNotAllowed } else { CursorStyle::ClosedHand }, window);
            self.arranged.drop = hover;
            cx.notify();
        }
    }

    /// The tab let go of: where it lands, it moves there, as one step back.
    pub(super) fn drop_tab(&mut self, drag: &TabDrag, window: &mut Window, cx: &mut Context<Self>) {
        let Some(hover) = self.arranged.drop.take() else { return };
        cx.notify();
        let (true, Some(target)) = (hover.ok && hover.tab == drag.tab, hover.target) else { return };
        let tab = drag.tab.clone();
        if self.rearrange(None, false, move |arrangement| landed(arrangement, None, &tab, &target).map_or(Cow::Borrowed(arrangement), Cow::Owned), cx) {
            self.want_pane_reads(window, cx);
        }
    }

    /// Esc while a tab is dragged lets go of it where it was.
    pub(super) fn drag_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        if event.keystroke.key == "escape" && cx.has_active_drag() && self.arranged.drop.is_some() {
            cx.stop_propagation();
            cx.stop_active_drag(window);
            self.arranged.drop = None;
            cx.notify();
        }
    }

    /// What a drag draws over a pane: a wash while the pointer is over it, and the part a drop on its body would take.
    pub(super) fn pane_drop_marks(&self, pane: &str, look: &Look) -> Vec<AnyElement> {
        let Some(hover) = self.arranged.drop.as_ref() else { return Vec::new() };
        let mut marks = Vec::new();
        if hover.target.as_ref().and_then(Target::pane) == Some(pane) {
            marks.push(
                div()
                    .absolute()
                    .inset_0()
                    .bg(look.color(token::FG_DEFAULT).opacity(0.06))
                    .border(look.length(token::BORDER_WIDTH) * 2.)
                    .border_color(look.color(token::BORDER_STRONG))
                    .into_any_element(),
            );
        }
        marks
    }

    /// The half or the whole of a body a drop would take, drawn over the body.
    pub(super) fn body_drop_mark(&self, pane: &str, look: &Look) -> Option<AnyElement> {
        let Some(Target::Zone { pane: over, zone }) = self.drop_lit() else { return None };
        if over != pane {
            return None;
        }
        let mark = div().absolute().bg(look.color(token::FG_DEFAULT).opacity(0.14)).border(look.length(token::BORDER_WIDTH)).border_color(look.color(token::BORDER_STRONG));
        let mark = match zone {
            Zone::Middle => mark.inset_0(),
            Zone::Edge(Side::Left) => mark.top_0().bottom_0().left_0().w(relative(0.5)),
            Zone::Edge(Side::Right) => mark.top_0().bottom_0().right_0().w(relative(0.5)),
            Zone::Edge(Side::Top) => mark.left_0().right_0().top_0().h(relative(0.5)),
            Zone::Edge(Side::Bottom) => mark.left_0().right_0().bottom_0().h(relative(0.5)),
        };
        Some(mark.into_any_element())
    }

    /// Where among a strip's tabs a drop would go: a bar just inside the left of the tab it would stand before, or the right
    /// of the last, drawn in that tab so it stands over it.
    pub(super) fn strip_bar_on(&self, pane: &str, tab: &str, look: &Look) -> Option<AnyElement> {
        let Some(Target::Strip { pane: over, index }) = self.drop_lit() else { return None };
        if over != pane {
            return None;
        }
        let dragged = self.dragged_tab()?;
        let arrangement = self.arrangement();
        let others: Vec<&str> = tree::pane_by_id(&arrangement, pane)?.tabs.iter().map(|one| one.id.as_str()).filter(|one| *one != dragged).collect();
        let at = others.iter().position(|one| *one == tab)?;
        let thick = look.length(token::SPACE_1);
        let bar = div().absolute().top_0().bottom_0().w(thick).bg(look.color(token::FG_MUTED));
        if at == *index {
            Some(bar.left_0().into_any_element())
        } else if at + 1 == others.len() && *index == others.len() {
            Some(bar.right_0().into_any_element())
        } else {
            None
        }
    }

    /// The bar along an outer edge a drop would take a pane along.
    pub(super) fn end_drop_mark(&self, look: &Look) -> Option<AnyElement> {
        let Some(Target::End { side }) = self.drop_lit() else { return None };
        let (thick, bar) = (look.length(token::SPACE_1), div().absolute().bg(look.color(token::FG_MUTED)));
        let bar = match side {
            Side::Left => bar.top_0().bottom_0().left_0().w(thick),
            Side::Right => bar.top_0().bottom_0().right_0().w(thick),
            Side::Top => bar.left_0().right_0().top_0().h(thick),
            Side::Bottom => bar.left_0().right_0().bottom_0().h(thick),
        };
        Some(bar.into_any_element())
    }

    /// Whether a drop would take this split's gap.
    pub(super) fn gap_lit(&self, split: &str) -> bool {
        matches!(self.drop_lit(), Some(Target::Gap { split: lit }) if lit == split)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{point, px, size};

    fn body() -> Bounds<Pixels> {
        Bounds { origin: point(px(100.), px(100.)), size: size(px(400.), px(200.)) }
    }

    #[test]
    fn a_body_splits_on_the_side_the_pointer_is_nearest_and_takes_the_tab_in_its_middle() {
        assert_eq!(zone_in(body(), point(px(300.), px(200.))), Zone::Middle);
        assert_eq!(zone_in(body(), point(px(110.), px(200.))), Zone::Edge(Side::Left));
        assert_eq!(zone_in(body(), point(px(490.), px(200.))), Zone::Edge(Side::Right));
        assert_eq!(zone_in(body(), point(px(300.), px(110.))), Zone::Edge(Side::Top));
        assert_eq!(zone_in(body(), point(px(300.), px(290.))), Zone::Edge(Side::Bottom));
    }

    #[test]
    fn top_and_bottom_win_the_corners() {
        assert_eq!(zone_in(body(), point(px(105.), px(105.))), Zone::Edge(Side::Top));
        assert_eq!(zone_in(body(), point(px(495.), px(295.))), Zone::Edge(Side::Bottom));
    }

    #[test]
    fn an_outer_edge_is_half_a_gap_deep() {
        let area = body();
        assert_eq!(end_at(area, point(px(103.), px(200.)), px(6.)), Some(Side::Left));
        assert_eq!(end_at(area, point(px(497.), px(200.)), px(6.)), Some(Side::Right));
        assert_eq!(end_at(area, point(px(300.), px(103.)), px(6.)), Some(Side::Top));
        assert_eq!(end_at(area, point(px(300.), px(297.)), px(6.)), Some(Side::Bottom));
        assert_eq!(end_at(area, point(px(110.), px(200.)), px(6.)), None);
    }

    #[test]
    fn a_drop_that_changes_nothing_is_refused_and_one_that_moves_the_tab_lands() {
        let was = tree::default_arrangement();
        let panes = tree::panes_of(&was.root);
        let (thread, top) = (panes[0].clone(), panes[1].clone());
        let alone = thread.tabs[0].id.clone();
        // The thread pane's only tab, dropped on its own edge, stays as it is.
        assert!(landed(&was, None, &alone, &Target::Zone { pane: thread.id.clone(), zone: Zone::Edge(Side::Left) }).is_none());
        // Dropped on the top pane's middle, it joins that pane in front.
        let next = landed(&was, None, &alone, &Target::Zone { pane: top.id.clone(), zone: Zone::Middle }).expect("lands");
        let held = tree::pane_of_tab(&next, &alone).expect("still held");
        assert_eq!(held.id, top.id);
        assert_eq!(held.active.as_deref(), Some(alone.as_str()));
    }

    #[test]
    fn a_drop_in_a_strip_goes_to_the_place_among_the_other_tabs() {
        let was = tree::default_arrangement();
        let top = tree::panes_of(&was.root)[1].clone();
        assert!(top.tabs.len() >= 2, "the top pane holds Open, Done and Deferred");
        let last = top.tabs.last().expect("a tab").id.clone();
        let next = landed(&was, None, &last, &Target::Strip { pane: top.id.clone(), index: 0 }).expect("lands");
        assert_eq!(tree::pane_by_id(&next, &top.id).expect("pane").tabs[0].id, last);
    }

    #[test]
    fn a_drop_that_would_squeeze_a_pane_under_its_least_size_is_refused() {
        let was = tree::default_arrangement();
        let panes = tree::panes_of(&was.root);
        let tab = panes[1].tabs[0].id.clone();
        let small = Some(Size { w: 300., h: 200. });
        assert!(landed(&was, small, &tab, &Target::End { side: Side::Left }).is_none());
    }
}
