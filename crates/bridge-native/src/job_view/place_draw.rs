//! The Job's panes drawn from its arrangement: each split two nodes with a divider between, each pane a strip
//! of tabs over the body of the tab in front. The bodies are the ones a fixed pane drew; what has changed is
//! that any pane may hold any of them.
//!
//! **A divider is the gap between two nodes**, `--gap-panel` wide, so a Job in its default arrangement
//! reads as it did. It draws nothing at rest, a hairline that takes the accent while the pointer is on it
//! or it is dragged, and has the resize cursor. A drag sets the split's share through `resize_split`, held
//! to the panes' least sizes; a double press sets it back to half.

use gpui::{canvas, AnimationExt, div, prelude::*, px, AnyElement, Context, CursorStyle, DragMoveEvent, FocusHandle, KeyDownEvent, MouseButton, MouseDownEvent, Role, ScrollHandle, SharedString, Window};

use super::pane_tree::{self as tree, Dir, Node, Pane, Split, Tab};
use super::place_drag::TabDrag;
use super::panes_place::{front_of, row_kind_of, slot_of, visible, zone_of, Slot};
use super::side::{pane, strip, tab, Zone};
use super::spine::Area;
use super::JobView;
use crate::labels::labelled;
use crate::look::{token, ActiveLook, Look};

/// What a drag of a divider carries: the split it moves.
#[derive(Clone)]
pub(super) struct DividerDrag {
    split: String,
}

impl gpui::Render for DividerDrag {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        gpui::Empty
    }
}

/// What the tree's drawing hands down: the thread's body for the pane that holds it, and the sheets and menus the Job lays over
/// its thread, for the pane they stand over.
pub(super) struct Frame {
    pub job: SharedString,
    pub thread_body: Option<AnyElement>,
    pub popups: Vec<AnyElement>,
    /// The pane the sheets stand over: the one that holds the thread, where that is drawn.
    pub host: Option<String>,
    pub zoomed: Option<String>,
}

/// A node's size along its parent's axis, as a share.
type Share = Option<(Dir, f32)>;

fn sized<T: gpui::Styled>(element: T, share: Share) -> T {
    match share {
        None => element.size_full(),
        Some((Dir::Row, grow)) => element.h_full().min_w(px(tree::MIN_PANE_W as f32)).flex_grow(grow * 100.).flex_shrink(1.).flex_basis(px(0.)),
        Some((Dir::Column, grow)) => element.w_full().min_h(px(tree::MIN_PANE_H as f32)).flex_grow(grow * 100.).flex_shrink(1.).flex_basis(px(0.)),
    }
}

/// How far an arrow on a focused divider, or Grow pane and Shrink pane, moves it, in points.
pub(super) const KEY_STEP: f64 = 24.;

fn find_split<'a>(node: &'a Node, id: &str) -> Option<&'a Split> {
    match node {
        Node::Pane(_) => None,
        Node::Split(split) if split.id == id => Some(split),
        Node::Split(split) => find_split(&split.a, id).or_else(|| find_split(&split.b, id)),
    }
}

impl JobView {
    /// The Job's panes: the tree, or the one pane zoomed over it. `thread_body` is what the thread's tab draws, and `popups` are laid over the
    /// pane that holds the thread, or over the whole area where none does.
    pub(super) fn render_panes(&mut self, id: &SharedString, thread_body: Option<AnyElement>, popups: Vec<AnyElement>, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let look = cx.look().clone();
        let look = &look;
        let arrangement = self.arrangement().into_owned();
        let zoomed = self.zoomed();
        // A drag let go of outside the panes, or ended by gpui, leaves nothing lit.
        if !cx.has_active_drag() {
            self.arranged.drop = None;
        }
        self.arranged.drawn_at.borrow_mut().clear();
        let holder = tree::holder_of(&arrangement, "thread").map(|pane| pane.id.clone());
        let host = holder.filter(|holder| zoomed.as_ref().is_none_or(|zoomed| zoomed == holder));
        if !self.zones_shown().contains(&Zone::Spine) {
            self.spine.left();
        }
        let mut frame = Frame { job: id.clone(), thread_body, popups, host: host.clone(), zoomed: zoomed.clone() };
        let drawn = match &zoomed {
            Some(zoomed) => {
                let one = tree::pane_by_id(&arrangement, zoomed).cloned();
                one.map(|one| self.draw_pane(&one, None, &mut frame, look, window, cx))
            }
            None => Some(self.draw_node(&arrangement.root, None, &mut frame, look, window, cx)),
        };
        if self.spine.settle.take() {
            window.request_animation_frame();
        }
        let area = self.arranged.area.clone();
        let release = cx.listener(|this, _: &gpui::MouseUpEvent, _, cx| {
            if this.arranged.dragging.take().is_some() {
                cx.notify();
            }
        });
        let release_out = cx.listener(|this, _: &gpui::MouseUpEvent, _, cx| {
            if this.arranged.dragging.take().is_some() {
                cx.notify();
            }
        });
        labelled(
            div()
            .id("job-panes")
            .size_full()
            .relative()
            .flex()
            .on_mouse_up(MouseButton::Left, release)
            .on_mouse_up_out(MouseButton::Left, release_out)
            .on_drag_move(cx.listener(Self::drag_tab_over))
            .on_drop(cx.listener(|this, drag: &TabDrag, window, cx| this.drop_tab(drag, window, cx)))
            .capture_key_down(cx.listener(Self::drag_key))
            .children(drawn)
            .children(self.end_drop_mark(look))
            // What is left over when no pane holds the thread: the sheets and menus lie over the whole area.
            .when(host.is_none(), |root| root.key_context("thread").on_action(cx.listener(Self::on_act)).on_action(cx.listener(Self::on_search)).child(gpui_base::TextSelectionLayer).children(std::mem::take(&mut frame.popups)))
            .child(
                canvas(
                    move |bounds, _, _| area.set(Some(tree::Size { w: f64::from(f32::from(bounds.size.width)), h: f64::from(f32::from(bounds.size.height)) })),
                    |_, _, _, _| {},
                )
                .absolute()
                .size_full(),
            ),
            Role::Region,
            "Job panes",
        )
        .into_any_element()
    }

    fn draw_node(&mut self, node: &Node, share: Share, frame: &mut Frame, look: &Look, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        match node {
            Node::Pane(one) => self.draw_pane(one, share, frame, look, window, cx),
            Node::Split(split) => {
                let row = split.dir == Dir::Row;
                let a = self.draw_node(&split.a, Some((split.dir, split.ratio as f32)), frame, look, window, cx);
                let b = self.draw_node(&split.b, Some((split.dir, 1. - split.ratio as f32)), frame, look, window, cx);
                let divider = self.divider(split, look, window, cx);
                let (id, dir) = (split.id.clone(), split.dir);
                sized(div().id(SharedString::from(format!("split:{}", split.id))).min_w_0().min_h_0().flex().when(!row, |one| one.flex_col()), share)
                    .on_drag_move(cx.listener(move |this, event: &DragMoveEvent<DividerDrag>, _, cx| {
                        if event.drag(cx).split == id {
                            this.drag_divider(&id, dir, event, cx);
                        }
                    }))
                    .child(a)
                    .child(divider)
                    .child(b)
                    .into_any_element()
            }
        }
    }

    /// The line between a split's two sides: the gap itself, with a hairline in it.
    fn divider(&mut self, split: &Split, look: &Look, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let row = split.dir == Dir::Row;
        let hot = self.arranged.dragging.as_deref() == Some(split.id.as_str());
        let accent = look.color(token::ACCENT);
        let group = SharedString::from(format!("divider:{}", split.id));
        let (id, drag) = (split.id.clone(), DividerDrag { split: split.id.clone() });
        let handle = self.shown.clone().map(|job| self.divider_handle(&job, &split.id, cx));
        let focused = handle.as_ref().is_some_and(|handle| handle.is_focused(window));
        let (down, up) = (id.clone(), id.clone());
        // A tab dragged over the gap lights a bar as thick as `--space-1`, where a drop puts a pane of its own.
        let lit = self.gap_lit(&split.id);
        let thick = if lit { look.length(token::SPACE_1) } else { look.length(token::BORDER_WIDTH) * 2. };
        let line = div().flex_none().rounded_full().when(row, |line| line.w(thick).h_full()).when(!row, |line| line.h(thick).w_full());
        let line = if lit {
            line.bg(look.color(token::FG_MUTED))
        } else if hot || focused {
            line.bg(accent)
        } else {
            line.group_hover(group.clone(), move |style| style.bg(accent))
        };
        let gap = look.length(token::GAP_PANEL);
        labelled(
            div()
                .id(SharedString::from(format!("pane-divider:{}", split.id)))
                .group(group)
                .relative()
                .child(self.mark_drawn(format!("gap:{}", split.id)))
                .flex_none()
                .flex()
                .items_center()
                .justify_center()
                .when(row, |one| one.w(gap).h_full().cursor(CursorStyle::ResizeLeftRight))
                .when(!row, |one| one.h(gap).w_full().cursor(CursorStyle::ResizeUpDown))
                .when_some(handle, |one, handle| one.track_focus(&handle).tab_stop(true))
                .on_drag(drag, |drag, _, _, cx| cx.new(|_| drag.clone()))
                .on_mouse_down(MouseButton::Left, cx.listener(move |this, _: &MouseDownEvent, _, cx| {
                    this.arranged.dragging = Some(down.clone());
                    cx.notify();
                }))
                .on_click(cx.listener(move |this, event: &gpui::ClickEvent, _, cx| {
                    this.arranged.dragging = None;
                    if event.click_count() == 2 {
                        let id = up.clone();
                        this.rearrange(None, false, move |arrangement| std::borrow::Cow::Owned(tree::resize_split(arrangement, &id, 0.5)), cx);
                    }
                }))
                .on_key_down(cx.listener(move |this, event: &KeyDownEvent, _, cx| {
                    let by = match (event.keystroke.key.as_str(), row) {
                        ("left", true) | ("up", false) => -1.,
                        ("right", true) | ("down", false) => 1.,
                        _ => return,
                    };
                    cx.stop_propagation();
                    this.nudge_divider(&id, by * KEY_STEP, cx);
                }))
                .child(line),
            Role::Splitter,
            "Resize the panes",
        )
        .into_any_element()
    }

    fn divider_handle(&mut self, job: &SharedString, split: &str, cx: &mut Context<Self>) -> FocusHandle {
        self.tab_handle(job, &format!("split:{split}"), "split", cx)
    }

    /// The split's first side as far along its box as the pointer is, held to what each side keeps.
    fn drag_divider(&mut self, id: &str, dir: Dir, event: &DragMoveEvent<DividerDrag>, cx: &mut Context<Self>) {
        let gap = f64::from(f32::from(cx.look().length(token::GAP_PANEL)));
        let (origin, extent, at) = match dir {
            Dir::Row => (event.bounds.origin.x, event.bounds.size.width, event.event.position.x),
            Dir::Column => (event.bounds.origin.y, event.bounds.size.height, event.event.position.y),
        };
        let extent = f64::from(f32::from(extent)) - gap;
        let first = f64::from(f32::from(at - origin)) - gap / 2.;
        self.resize_to(id, first, extent, true, cx);
    }

    /// The divider moves by `by` px from where it stands.
    pub(super) fn nudge_divider(&mut self, id: &str, by: f64, cx: &mut Context<Self>) {
        let arrangement = self.arrangement().into_owned();
        let (Some(area), Some(split)) = (self.arranged.area.get(), find_split(&arrangement.root, id)) else { return };
        let rects = tree::rects_of(&arrangement.root);
        let Some(rect) = rects.splits.iter().find(|one| one.id == id).map(|one| one.rect) else { return };
        let extent = if split.dir == Dir::Row { rect.w * area.w } else { rect.h * area.h };
        self.resize_to(id, extent * split.ratio + by, extent, false, cx);
    }

    /// Sets a split so its first side is `first` px of `extent`: each side keeps the room its panes need, or the share the
    /// ratio keeps where the area is too small for that.
    fn resize_to(&mut self, id: &str, first: f64, extent: f64, dragged: bool, cx: &mut Context<Self>) {
        let arrangement = self.arrangement().into_owned();
        let Some(split) = find_split(&arrangement.root, id) else { return };
        if extent <= 0. {
            return;
        }
        let low = (extent * tree::RATIO_MIN).max(tree::min_extent(&split.a, split.dir));
        let high = extent - (extent * tree::RATIO_MIN).max(tree::min_extent(&split.b, split.dir));
        let held = if low <= high { first.clamp(low, high) } else { extent * split.ratio };
        if dragged && self.arranged.dragging.as_deref() != Some(id) {
            self.arranged.dragging = Some(id.to_string());
            cx.notify();
        }
        let (split_id, ratio) = (id.to_string(), held / extent);
        let merge = format!("resize {id}");
        self.rearrange(Some(&merge), false, move |arrangement| std::borrow::Cow::Owned(tree::resize_split(arrangement, &split_id, ratio)), cx);
    }

    // ---- one pane ----------------------------------------------------------------------------

    fn draw_pane(&mut self, one: &Pane, share: Share, frame: &mut Frame, look: &Look, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let job = frame.job.clone();
        let slot = slot_of(&one.id);
        let tabs: Vec<Tab> = visible(one).cloned().collect();
        let front = front_of(one).cloned();
        let thread_like = tabs.iter().any(|tab| tab.kind == "thread");
        let hosts = frame.host.as_deref() == Some(one.id.as_str());
        let zoomed = frame.zoomed.as_deref() == Some(one.id.as_str());

        // The strip scrolls sideways once its tabs are wider than the pane, and keeps the tab in front in view.
        let key = format!("{job}/{}", one.id);
        let entry = self.arranged.scrolls.entry(key).or_insert_with(|| (ScrollHandle::new(), None));
        let scroll = entry.0.clone();
        let front_id = front.as_ref().map(|tab| tab.id.clone());
        if entry.1 != front_id {
            entry.1 = front_id.clone();
            if let Some(at) = tabs.iter().position(|tab| Some(&tab.id) == front_id.as_ref()) {
                scroll.scroll_to_item(at);
            }
        }

        let strip_tabs: Vec<AnyElement> = tabs.iter().map(|tab| self.strip_tab(&job, one, tab, Some(&tab.id) == front_id.as_ref(), look, cx)).collect();
        let pane_id = one.id.clone();
        let strip_id = SharedString::from(format!("pane-strip:{}", one.id));
        let strip_div = if thread_like {
            div()
                .flex_none()
                .flex()
                .items_stretch()
                .h(look.length(token::H_CONTROL_LG))
                .bg(look.color(token::BG_SUNKEN))
                .border_b(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::BORDER_GLASS))
        } else {
            strip(look)
        };
        let (zoom_id, keys_id, add_id) = (pane_id.clone(), pane_id.clone(), pane_id.clone());
        let strip_el = strip_div
            .id(strip_id)
            .relative()
            .child(self.mark_drawn(format!("strip:{}", one.id)))
            .on_click(cx.listener(move |this, event: &gpui::ClickEvent, _, cx| {
                if event.click_count() == 2 {
                    this.toggle_zoom(&zoom_id, cx);
                }
            }))
            .on_key_down(cx.listener(move |this, event: &KeyDownEvent, window, cx| this.strip_key(&keys_id, event, window, cx)))
            .child(
                labelled(
                    div().id(SharedString::from(format!("pane-tabs:{}", one.id))).flex().items_stretch().min_w_0().overflow_x_scroll().track_scroll(&scroll).children(strip_tabs),
                    Role::TabList,
                    "Tabs",
                ),
            )
            .child(self.add_button(&add_id, look, cx))
            .children(self.strip_end(one, zoomed, tabs.is_empty(), look, cx));

        // The body of the tab in front.
        let body: Option<AnyElement> = match front.as_ref() {
            None => None,
            Some(tab) => self.draw_body(&job, tab, frame, look, window, cx),
        };
        let said = if thread_like { self.head_said_row(&job, cx) } else { None };
        let region: Option<SharedString> = match slot {
            Slot::Thread => None,
            Slot::Top => Some("Open, Done and Deferred".into()),
            Slot::Now => Some("Job column: Now".into()),
            Slot::Other => Some(match front.as_ref() {
                Some(tab) => format!("Pane: {}", self.tab_word(&job, tab)).into(),
                None => "Empty pane".into(),
            }),
        };
        let popups = if hosts { std::mem::take(&mut frame.popups) } else { Vec::new() };
        let press = one.id.clone();
        let body_marks = self.body_drop_mark(&one.id, look);
        let pane_marks = self.pane_drop_marks(&one.id, look);
        let element = sized(pane(look).id(SharedString::from(format!("pane:{}", one.id))), share)
            .relative()
            .when(hosts, |pane| pane.key_context("thread").on_action(cx.listener(Self::on_act)).on_action(cx.listener(Self::on_search)).child(gpui_base::TextSelectionLayer))
            // A press in the pane makes it the one the keys act on.
            .on_any_mouse_down(cx.listener(move |this, _, _, cx| this.pane_pressed(&press, cx)))
            .child(strip_el)
            .children(said)
            .child(div().flex_1().min_h_0().flex().flex_col().relative().children(body).child(self.mark_drawn(format!("body:{}", one.id))).children(body_marks))
            .children(pane_marks)
            .children(popups);
        match region {
            Some(name) => labelled(element, Role::Region, name).into_any_element(),
            None => element.into_any_element(),
        }
    }

    /// The word a tab reads.
    pub(super) fn tab_word(&self, job: &SharedString, tab: &Tab) -> SharedString {
        match tab.kind.as_str() {
            "thread" => "Thread".into(),
            "now" => "Now".into(),
            "terminal" => self.view_title(job, tab).unwrap_or_else(|| "Terminal".into()),
            kind => match (row_kind_of(kind), tab.row.as_deref()) {
                (Some(row), Some(key)) => self.row_tab_of(job, row, key).map_or_else(|| tab.label.clone().unwrap_or_default().into(), |held| held.title.clone().into()),
                _ => zone_of(kind).map_or_else(|| kind.to_string().into(), |zone| zone.look().1.into()),
            },
        }
    }

    fn view_title(&self, job: &SharedString, tab: &Tab) -> Option<SharedString> {
        let view = self.view_id_of(job, tab)?;
        self.view_tabs.by_job.get(job)?.iter().find(|one| one.id == view).map(|one| one.title.clone())
    }

    /// The body of a pane's front tab. None for a tab of a kind with no body built here.
    fn draw_body(&mut self, job: &SharedString, tab: &Tab, frame: &mut Frame, look: &Look, window: &mut Window, cx: &mut Context<Self>) -> Option<AnyElement> {
        let id = job.clone();
        let scrolled = |name: String, inner: Option<AnyElement>| {
            div()
                .id(SharedString::from(name))
                .flex_1()
                .min_h_0()
                .overflow_y_scroll()
                .px(look.length(token::SPACE_3))
                .pt(look.length(token::SPACE_2))
                .pb(look.length(token::SPACE_4))
                .children(inner)
                .into_any_element()
        };
        let plain = |name: String, inner: Option<AnyElement>| div().id(SharedString::from(name)).flex_1().min_h_0().flex().flex_col().children(inner).into_any_element();
        match tab.kind.as_str() {
            "thread" => frame.thread_body.take(),
            "now" => {
                let body = self.zone_alone(Area::Now, self.flow_alone(&id, Area::Now, look, cx).unwrap_or_else(|| self.side_now(&id, look, cx)), cx);
                Some(scrolled(format!("pane-body:{}", tab.id), Some(body)))
            }
            "terminal" => {
                let view = self.view_id_of(&id, tab).and_then(|view| self.view_tabs.by_job.get(&id)?.iter().find(|one| one.id == view).map(|one| one.view.clone()))?;
                Some(plain(format!("pane-body:{}", tab.id), Some(div().size_full().child(view).into_any_element())))
            }
            kind if row_kind_of(kind).is_some() => {
                let (row, key) = (row_kind_of(kind)?, tab.row.clone()?);
                self.row_tab_seen(&id, row, &key);
                let held = self.row_tab_of(&id, row, &key).cloned()?;
                Some(self.row_tab_body(&id, &held, look, window, cx))
            }
            kind => {
                let zone = zone_of(kind)?;
                let body = match zone {
                    Zone::Open => Some(self.zone_alone(Area::Open, self.flow_alone(&id, Area::Open, look, cx).unwrap_or_else(|| self.side_open(&id, look, cx)), cx)),
                    Zone::Done => Some(self.zone_alone(Area::Done, self.flow_alone(&id, Area::Done, look, cx).unwrap_or_else(|| self.side_done(&id, look, cx)), cx)),
                    Zone::Deferred => Some(self.zone_alone(Area::Deferred, self.side_deferred(&id, look, cx), cx)),
                    Zone::Plan => self.pane_plan(&id, look, cx),
                    Zone::Changes => self.pane_changes(&id, look, cx),
                    Zone::Pulls => self.pane_pulls(&id, look, cx),
                    Zone::Issues => self.pane_issues(&id, look, cx),
                    Zone::Jobs => self.pane_jobs(&id, look, cx),
                    Zone::Ledger => self.pane_ledger(&id, look, cx),
                    Zone::Files => Some(self.pane_files(&id, look, cx)),
                    Zone::Subagents => self.pane_subagents(&id, look, cx),
                    Zone::Checks => Some(self.pane_checks(&id, look, cx)),
                    Zone::Workflow => self.pane_lanes(&id, look, cx),
                    Zone::System => self.pane_system(&id, look, cx),
                    Zone::Bays => self.pane_bays(&id, look, cx),
                    Zone::Spine => Some(self.pane_spine(&id, look, cx)),
                    Zone::Judge => self.pane_judge(&id, look, cx),
                    Zone::Artifacts => self.pane_artifacts(&id, look, cx),
                };
                // Changes, Files and the rest draw their own strip over a body that scrolls, as `.job-pane__pick` does.
                let own_frame = matches!(zone, Zone::Changes | Zone::Files | Zone::Judge | Zone::Spine | Zone::Workflow);
                let name = format!("pane-body:{}", tab.id);
                Some(if own_frame { plain(name, body) } else { scrolled(name, body) })
            }
        }
    }

    // ---- the strip ---------------------------------------------------------------------------

    /// The end of a pane's strip: Restore while it is zoomed, and Close pane while it holds no tab.
    fn strip_end(&self, pane: &Pane, zoomed: bool, empty: bool, look: &Look, cx: &Context<Self>) -> Vec<AnyElement> {
        let mut end = Vec::new();
        let id = pane.id.clone();
        if zoomed {
            let pane = id.clone();
            end.push(
                labelled(
                    super::tabs::frame(look, SharedString::from(format!("restore:{id}")), "minimize-2")
                        .tooltip(|window, cx| {
                            let spelled = crate::generated::actions::act("pane_zoom").and_then(|row| crate::keymap::parse_chord(row.shortcut)).map(|chord| chord.keystroke());
                            let key = spelled.and_then(|spelled| gpui::Keystroke::parse_with_os(&spelled, gpui::OperatingSystem::Mac).ok());
                            gpui_component::tooltip::Tooltip::new("Restore").key_binding(key.map(gpui_component::kbd::Kbd::new)).build(window, cx)
                        })
                        .on_click(cx.listener(move |this, _, _, cx| {
                            cx.stop_propagation();
                            this.toggle_zoom(&pane, cx);
                        })),
                    Role::Button,
                    "Restore",
                )
                .into_any_element(),
            );
        }
        if empty {
            let pane = id.clone();
            end.push(
                labelled(
                    super::tabs::frame(look, SharedString::from(format!("close-pane:{id}")), "x").tooltip(super::head::tip("Close pane".into())).on_click(cx.listener(move |this, _, window, cx| {
                        cx.stop_propagation();
                        this.close_pane_id(&pane, window, cx);
                    })),
                    Role::Button,
                    "Close pane",
                )
                .into_any_element(),
            );
        }
        end
    }

    /// ← → Home End along a focused strip walk its tabs, each coming to the front with the keys; Enter and Space bring the focused one forward.
    fn strip_key(&mut self, pane: &str, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let Some(job) = self.shown.clone() else { return };
        let arrangement = self.arrangement().into_owned();
        let Some(one) = tree::pane_by_id(&arrangement, pane) else { return };
        let tabs: Vec<&Tab> = visible(one).collect();
        let handles: Vec<FocusHandle> = tabs.iter().map(|tab| self.tab_handle(&job, &tab.id, &tab.kind, cx)).collect();
        let Some(at) = handles.iter().position(|handle| handle.is_focused(window)) else { return };
        let keys = &event.keystroke;
        if keys.modifiers.modified() {
            return;
        }
        let to = match keys.key.as_str() {
            "home" => 0,
            "end" => tabs.len() - 1,
            "left" => at.saturating_sub(1),
            "right" => (at + 1).min(tabs.len() - 1),
            "enter" | "space" => at,
            _ => return,
        };
        cx.stop_propagation();
        let target = tabs[to].id.clone();
        self.activate_tab(&target, window, cx);
        window.focus(&handles[to], cx);
    }

    /// One tab of a strip.
    fn strip_tab(&mut self, job: &SharedString, pane: &Pane, tab_of: &Tab, front: bool, look: &Look, cx: &mut Context<Self>) -> AnyElement {
        let (tab_id, pane_id) = (tab_of.id.clone(), pane.id.clone());
        let middle = {
            let id = tab_id.clone();
            cx.listener(move |this, event: &MouseDownEvent, window, cx| {
                if event.button == MouseButton::Middle {
                    cx.stop_propagation();
                    this.close_tab_id(&id, window, cx);
                }
            })
        };
        let dragged = self.dragged_tab() == Some(tab_id.as_str());
        if tab_of.kind == "thread" {
            let drag = TabDrag::new(tab_id.clone(), "Thread".into(), "message-square");
            return self
                .thread_tab(job, &tab_id, front, cx)
                .on_mouse_down(MouseButton::Middle, middle)
                .id(SharedString::from(format!("pane-tab-drag:{tab_id}")))
                .relative()
                .child(self.mark_drawn(format!("tab:{tab_id}")))
                .children(self.strip_bar_on(&pane_id, &tab_id, look))
                .when(dragged, |one| one.opacity(0.5))
                .on_drag(drag, |drag, grabbed, _, cx| drag.lifted(grabbed, cx))
                .into_any_element();
        }
        let handle = self.tab_handle(job, &tab_id, &tab_of.kind, cx);
        let group = SharedString::from(format!("tab:{tab_id}"));
        let word = self.tab_word(job, tab_of);
        let row = tab_of.row.as_deref().zip(row_kind_of(&tab_of.kind));
        let (icon, name): (&'static str, String) = match tab_of.kind.as_str() {
            "now" => ("activity", "Job column: Now".into()),
            "terminal" => (self.view_glyph(job, tab_of), format!("Job column: {word}")),
            _ => match row {
                Some((_, _)) => (row_kind_of(&tab_of.kind).map_or("file", |kind| kind.look().0), format!("Job tab: {word}")),
                None => match zone_of(&tab_of.kind) {
                    Some(zone) => (zone.look().0, format!("Job column: {word}")),
                    None => ("file", format!("Job column: {word}")),
                },
            },
        };
        let (pick, menu_for, keys_for) = (tab_id.clone(), tab_id.clone(), tab_id.clone());
        let field = (tab_of.kind == "terminal").then(|| self.view_rename_field(job, tab_of, look, cx)).flatten();
        let said: SharedString = if field.is_some() { SharedString::default() } else { word.clone() };
        let pulses = row.is_some_and(|(key, kind)| kind == super::row_tabs::RowKind::Walk && self.row_tab_of(job, kind, key).is_some_and(|held| !held.seen));
        let close = self.close_button(&tab_id, &word, group.clone(), front, look, cx);
        let ink = look.color(if front { token::FG_DEFAULT } else { token::FG_MUTED });
        // A tab being renamed does not drag, as Classic's is not `draggable` then.
        let drag = field.is_none().then(|| TabDrag::new(tab_id.clone(), word.clone(), icon));
        let base = tab(look, SharedString::from(format!("pane-tab:{tab_id}")), icon, said, front)
            .relative()
            .child(self.mark_drawn(format!("tab:{tab_id}")))
            .children(self.strip_bar_on(&pane_id, &tab_id, look))
            .when(dragged, |one| one.opacity(0.5))
            .when_some(drag, |one, drag| one.on_drag(drag, |drag, grabbed, _, cx| drag.lifted(grabbed, cx)));
        let tint = look.color(token::STATUS_AWAITING_REVIEW);
        let _ = (ink, &pane_id);
        labelled(
            base.group(group)
                .track_focus(&handle)
                .tab_stop(front)
                .focus_visible(|style| style.border_1().border_color(look.color(token::ACCENT)))
                .when(row.is_some(), |one| one.max_w(look.length(token::SPACE_12) * 5.).border_r(look.length(token::BORDER_WIDTH)).border_color(look.color(token::BORDER_GLASS)))
                .when(pulses, |one| {
                    one.relative().child(div().absolute().inset_0().with_animation(
                        SharedString::from(format!("row-tab-owes:{tab_id}")),
                        gpui::Animation::new(look.duration(token::DURATION_PULSE)).repeat().with_easing(gpui::pulsating_between(0.06, 0.14)),
                        move |tinted, at| tinted.bg(tint.opacity(at)),
                    ))
                })
                .children(field)
                .child(close)
                .on_click(cx.listener(move |this, _, window, cx| {
                    this.activate_tab(&pick, window, cx);
                    this.focus_front_body(&pick, window, cx);
                }))
                .on_mouse_down(MouseButton::Right, cx.listener(move |this, event: &MouseDownEvent, window, cx| {
                    cx.stop_propagation();
                    this.open_tab_menu(&menu_for, event.position, false, window, cx);
                }))
                .on_mouse_down(MouseButton::Middle, middle)
                .on_key_down(cx.listener(move |this, event: &KeyDownEvent, window, cx| {
                    if matches!(event.keystroke.key.as_str(), "menu") || (event.keystroke.key == "f10" && event.keystroke.modifiers.shift) {
                        cx.stop_propagation();
                        let at = super::head::drawn(window, cx, Role::Tab, &name_of(&keys_for, this)).map_or_else(|| window.mouse_position(), super::head::below_left);
                        this.open_tab_menu(&keys_for, at, true, window, cx);
                    }
                })),
            Role::Tab,
            name,
        )
        .into_any_element()
    }

    /// A press on a tab gives the keys to its body where the body takes them.
    fn focus_front_body(&mut self, tab: &str, window: &mut Window, cx: &mut Context<Self>) {
        let Some(held) = self.tab_of(tab) else { return };
        if row_kind_of(&held.kind).is_some() {
            window.focus(&self.row_focus, cx);
        }
    }

    fn view_glyph(&self, job: &SharedString, tab: &Tab) -> &'static str {
        self.view_id_of(job, tab).and_then(|view| self.view_tabs.by_job.get(job)?.iter().find(|one| one.id == view).map(|one| one.glyph)).unwrap_or("terminal")
    }

    /// The title of a terminal tab as a field, while it is being renamed.
    fn view_rename_field(&self, job: &SharedString, tab: &Tab, look: &Look, cx: &Context<Self>) -> Option<AnyElement> {
        let view = self.view_id_of(job, tab)?;
        let rename = self.view_tabs.renaming.as_ref().filter(|rename| rename.job == *job && rename.id == view)?;
        let title = self.view_title(job, tab).unwrap_or_default();
        Some(
            labelled(
                div()
                    .id("view-tab-rename")
                    .key_context(crate::keymap::TEXT_BOX)
                    .min_w(look.length(token::SPACE_12) * 3.)
                    .capture_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                        if event.keystroke.key == "escape" {
                            this.finish_view_rename(false, window, cx);
                            cx.stop_propagation();
                        }
                    }))
                    .child(gpui_component::input::Input::new(&rename.input).appearance(false).cleanable(false)),
                Role::TextInput,
                format!("Rename {title}"),
            )
            .into_any_element(),
        )
    }
}

fn name_of(tab: &str, view: &JobView) -> String {
    let Some(job) = view.shown.as_ref() else { return String::new() };
    view.tab_of(tab).map(|held| match held.kind.as_str() {
        "now" => "Job column: Now".to_string(),
        kind if row_kind_of(kind).is_some() => format!("Job tab: {}", view.tab_word(job, &held)),
        _ => format!("Job column: {}", view.tab_word(job, &held)),
    }).unwrap_or_default()
}
