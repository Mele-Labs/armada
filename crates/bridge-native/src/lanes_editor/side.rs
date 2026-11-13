//! Two sets of lanes side by side with what differs between them, for him to choose from. Each difference is a row in a
//! list and a mark on the node it is about; pressing either toggles it chosen, and Take hands the chosen rows back.
//!
//! The view knows nothing of what a difference is: its host hands it the two sides' labels and lanes and one [`Spec`] for
//! each difference, and takes the rows chosen back as [`Event::Take`]. Choosing what to take from Armada is the first
//! use; the clash between projects is the next.

use ipc::{JobLanesView, LaneKind, LaneNodeKind};

use gpui::{div, prelude::*, px, AnyElement, Context, Entity, EventEmitter, FocusHandle, FontWeight, Hsla, Keystroke, Role, SharedString, Subscription, Window};

use super::armada::{Difference, Kind};
use super::patch::{node_id, step_node};
use crate::labels::labelled;
use crate::lanes_canvas::{lanes_canvas, LanesHost, LanesState, Pick, LANE_MIN};
use crate::look::{token, ActiveLook, Look};
use crate::settings::change_band::filled;
use crate::settings::definition::steps_of;
use crate::settings::view::{ghost, ringed, tip};
use crate::titlebar::parts::glyph;

/// What happened to the thing a row is about: its glyph and its hue.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mark {
    Added,
    Removed,
    Moved,
    Renamed,
    Words,
}

impl Mark {
    fn glyph(self) -> &'static str {
        match self {
            Mark::Added => "list-plus",
            Mark::Removed => "list-minus",
            Mark::Moved => "arrow-up-down",
            Mark::Renamed => "square-pen",
            Mark::Words => "scroll-text",
        }
    }

    fn hue(self, look: &Look) -> Hsla {
        look.color(match self {
            Mark::Added => token::STEP_ADVANCED,
            Mark::Removed => token::STATUS_COMPLETED_FAILED,
            Mark::Moved => token::STATUS_AWAITING_REVIEW,
            Mark::Renamed => token::ACCENT,
            Mark::Words => token::STATUS_RUNNING,
        })
    }
}

/// One difference: what it says, and the nodes it is marked on, by id, on the left side's lanes and on the right's.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Spec {
    pub mark: Mark,
    pub said: String,
    pub left: Option<String>,
    pub right: Option<String>,
}

/// One side: the name its header carries, the name its lanes are read under, and the lanes.
pub struct Party {
    pub label: &'static str,
    pub named: &'static str,
    pub lanes: JobLanesView,
}

/// What the view is given.
pub struct Sides {
    /// What the whole view is read as, and what the button that hands the choice back says.
    pub title: &'static str,
    pub take: &'static str,
    pub left: Party,
    pub right: Party,
    pub rows: Vec<Spec>,
}

/// What the view tells its host: the rows chosen, by their place in `Sides.rows`, or that he let it go.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    Take(Vec<usize>),
    Cancel,
}

impl EventEmitter<Event> for Side {}

/// A press on a node of one canvas.
struct Pressed(String);

/// One canvas, an entity of its own: it keeps its pan, its zoom and its keys apart from the other's.
struct Canvas {
    id: SharedString,
    state: LanesState,
    lanes: JobLanesView,
}

impl EventEmitter<Pressed> for Canvas {}

impl LanesHost for Canvas {
    fn lanes_state(&self) -> &LanesState {
        &self.state
    }

    fn lanes_of<'a>(&'a self, _id: &SharedString, _cx: &'a gpui::App) -> Option<&'a JobLanesView> {
        Some(&self.lanes)
    }

    /// Every press toggles what the node is about, so the pick is let go at once: the next press on it is a pick again.
    fn lanes_picked(&mut self, id: &SharedString, pick: Option<Pick>, _window: &mut Window, cx: &mut Context<Self>) {
        self.state.pick(id, None);
        if let Some(Pick::Node(node)) = pick {
            cx.emit(Pressed(node));
        }
    }
}

impl gpui::Render for Canvas {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let look = cx.look().clone();
        // One lane and room above and below its steps.
        let tall = px(LANE_MIN) + look.length(token::SPACE_8);
        div().flex().flex_col().h(tall).min_w_0().child(lanes_canvas(self, &self.id, &self.lanes, &look, cx))
    }
}

/// Where a side stands: its header, the lanes it was given and the canvas that draws them with the rows' marks.
struct Seat {
    label: &'static str,
    named: &'static str,
    lanes: JobLanesView,
    canvas: Entity<Canvas>,
}

struct Row {
    spec: Spec,
    chosen: bool,
    focus: FocusHandle,
}

pub struct Side {
    title: &'static str,
    take: &'static str,
    left: Seat,
    right: Seat,
    rows: Vec<Row>,
    take_focus: FocusHandle,
    cancel_focus: FocusHandle,
    /// The view has just opened: the keys go to its first row when it is next drawn.
    focus_first: bool,
    _subs: Vec<Subscription>,
}

/// Which side a press came from.
#[derive(Clone, Copy)]
enum Which {
    Left,
    Right,
}

impl Side {
    pub fn new(sides: Sides, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let canvas = |id: &'static str, lanes: &JobLanesView, cx: &mut Context<Self>| cx.new(|cx| Canvas { id: id.into(), state: LanesState::new(cx), lanes: lanes.clone() });
        let (left, right) = (canvas("take-left", &sides.left.lanes, cx), canvas("take-right", &sides.right.lanes, cx));
        let subs = vec![
            cx.subscribe_in(&left, window, |this, _, pressed: &Pressed, window, cx| this.pressed(Which::Left, &pressed.0, window, cx)),
            cx.subscribe_in(&right, window, |this, _, pressed: &Pressed, window, cx| this.pressed(Which::Right, &pressed.0, window, cx)),
        ];
        let stop = |cx: &mut Context<Self>| cx.focus_handle().tab_stop(true);
        let rows = sides.rows.into_iter().map(|spec| Row { spec, chosen: false, focus: stop(cx) }).collect();
        let mut side = Side {
            title: sides.title,
            take: sides.take,
            left: Seat { label: sides.left.label, named: sides.left.named, lanes: sides.left.lanes, canvas: left },
            right: Seat { label: sides.right.label, named: sides.right.named, lanes: sides.right.lanes, canvas: right },
            rows,
            take_focus: stop(cx),
            cancel_focus: stop(cx),
            focus_first: true,
            _subs: subs,
        };
        side.marks(cx);
        side
    }

    /// The Tab stops in the order they are drawn: each row, Take, Cancel.
    pub fn stops(&self) -> Vec<FocusHandle> {
        self.rows.iter().map(|row| row.focus.clone()).chain([self.take_focus.clone(), self.cancel_focus.clone()]).collect()
    }

    fn chosen(&self) -> Vec<usize> {
        self.rows.iter().enumerate().filter(|(_, row)| row.chosen).map(|(n, _)| n).collect()
    }

    /// Each canvas drawn with the marks of the rows that are about its nodes.
    fn marks(&mut self, cx: &mut Context<Self>) {
        let (left, right) = (marked(&self.left.lanes, &self.rows, Which::Left), marked(&self.right.lanes, &self.rows, Which::Right));
        self.left.canvas.update(cx, |canvas, cx| {
            canvas.lanes = left;
            cx.notify();
        });
        self.right.canvas.update(cx, |canvas, cx| {
            canvas.lanes = right;
            cx.notify();
        });
    }

    fn toggle(&mut self, at: usize, cx: &mut Context<Self>) {
        if let Some(row) = self.rows.get_mut(at) {
            row.chosen = !row.chosen;
            self.marks(cx);
            cx.notify();
        }
    }

    /// A press on a node: what it is about is chosen, or all of it let go where it was all chosen. The keys go to its first
    /// row, so Tab and Space go on from there.
    fn pressed(&mut self, side: Which, node: &str, window: &mut Window, cx: &mut Context<Self>) {
        let about = |row: &Row| match side {
            Which::Left => row.spec.left.as_deref() == Some(node),
            Which::Right => row.spec.right.as_deref() == Some(node),
        };
        let all = self.rows.iter().filter(|row| about(row)).all(|row| row.chosen);
        if !self.rows.iter().any(|row| about(row)) {
            return;
        }
        for row in self.rows.iter_mut().filter(|row| about(row)) {
            row.chosen = !all;
        }
        if let Some(row) = self.rows.iter().find(|row| about(row)) {
            window.focus(&row.focus, cx);
        }
        self.marks(cx);
        cx.notify();
    }

    fn take(&mut self, cx: &mut Context<Self>) {
        let chosen = self.chosen();
        if !chosen.is_empty() {
            cx.emit(Event::Take(chosen));
        }
    }

    /// The keys of the view, for a host that gets them first: Space or Enter chooses the row that has the keys or presses
    /// the button that has them, Esc lets go. `true` where one was taken.
    pub fn key(&mut self, key: &Keystroke, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let mods = &key.modifiers;
        if mods.platform || mods.control || mods.alt {
            return false;
        }
        match key.key.as_str() {
            "escape" => {
                cx.emit(Event::Cancel);
                true
            }
            "enter" | "space" => {
                if let Some(at) = self.rows.iter().position(|row| row.focus.is_focused(window)) {
                    self.toggle(at, cx);
                    true
                } else if self.take_focus.is_focused(window) {
                    self.take(cx);
                    true
                } else if self.cancel_focus.is_focused(window) {
                    cx.emit(Event::Cancel);
                    true
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    fn header(&self, seat: &Seat, tint: Hsla, edge: Hsla, look: &Look) -> AnyElement {
        let head = div()
            .id(SharedString::from(format!("take-head-{}", seat.label)))
            .flex_none()
            .flex()
            .items_center()
            .h(look.length(token::H_CONTROL))
            .px(look.length(token::SPACE_3))
            .rounded(look.length(token::RADIUS_LG))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(edge)
            .bg(tint)
            .text_size(look.length(token::TEXT_SM))
            .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
            .text_color(look.color(token::FG_DEFAULT))
            .child(SharedString::from(seat.label));
        labelled(head, Role::Heading, seat.label).into_any_element()
    }

    fn seat(&self, seat: &Seat, tint: Hsla, edge: Hsla, look: &Look) -> AnyElement {
        let column = div()
            .id(SharedString::from(format!("take-lanes-{}", seat.label)))
            .flex_1()
            .min_w_0()
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_2))
            .child(self.header(seat, tint, edge, look))
            .child(seat.canvas.clone());
        labelled(column, Role::Group, seat.named).into_any_element()
    }

    fn render_rows(&self, look: &Look, window: &Window, cx: &Context<Self>) -> Vec<AnyElement> {
        self.rows
            .iter()
            .enumerate()
            .map(|(n, row)| {
                let said = SharedString::from(row.spec.said.clone());
                let hue = row.spec.mark.hue(look);
                let mark = div()
                    .flex_none()
                    .size(look.length(token::SPACE_6))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded_full()
                    .border(look.length(token::BORDER_WIDTH))
                    .border_color(hue)
                    .bg(hue.opacity(0.14))
                    .child(glyph(row.spec.mark.glyph(), look.length(token::SPACE_4), hue));
                let line = div()
                    .id(SharedString::from(format!("take-row-{n}")))
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_2))
                    .min_h(look.length(token::H_CONTROL))
                    .px(look.length(token::SPACE_2))
                    .rounded(look.length(token::RADIUS_MD))
                    .border(look.length(token::BORDER_WIDTH))
                    .border_color(if row.chosen { look.color(token::ACCENT) } else { gpui::transparent_black() })
                    .bg(if row.chosen { look.color(token::ACCENT_MUTED) } else { gpui::transparent_black() })
                    .text_size(look.length(token::TEXT_SM))
                    .text_color(look.color(token::FG_DEFAULT))
                    .cursor_pointer()
                    .hover(|style| style.bg(look.color(token::BG_HOVER)))
                    .tooltip(tip(if row.chosen { format!("Taking: {}", row.spec.said) } else { format!("Take: {}", row.spec.said) }))
                    .aria_toggled(if row.chosen { gpui::accesskit::Toggled::True } else { gpui::accesskit::Toggled::False })
                    .child(super::ask::tick(look, row.chosen, false))
                    .child(mark)
                    .child(div().flex_1().min_w_0().child(said.clone()));
                let line = ringed(look, line, &row.focus, window).track_focus(&row.focus).on_click(cx.listener(move |this, _, _, cx| this.toggle(n, cx)));
                labelled(line, Role::CheckBox, said).into_any_element()
            })
            .collect()
    }
}

impl gpui::Render for Side {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let look = cx.look().clone();
        if std::mem::take(&mut self.focus_first) {
            window.focus(self.rows.first().map_or(&self.cancel_focus, |row| &row.focus), cx);
        }
        // Where the page is wide enough for two columns they stand side by side; below that, one above the other.
        let wide = window.viewport_size().width >= look.length(token::LAYOUT_BREAKPOINT);
        let sides = if wide { div().flex().flex_row() } else { div().flex().flex_col() };
        let sides = sides
            .gap(look.length(token::SPACE_3))
            .min_w_0()
            .child(self.seat(&self.left, look.color(token::ACCENT_MUTED), look.color(token::ACCENT), &look))
            .child(self.seat(&self.right, look.color(token::YOU).opacity(0.14), look.color(token::YOU_EDGE), &look));
        let list = labelled(div().id("take-list").flex().flex_col().gap(look.length(token::SPACE_1)).min_w_0().children(self.render_rows(&look, window, cx)), Role::List, SharedString::from(format!("From {}", self.left.label)));
        let off = self.chosen().is_empty();
        let take = ringed(&look, filled(&look, "take-take".into(), self.take, off), &self.take_focus, window)
            .track_focus(&self.take_focus)
            .when(!off, |button| button.on_click(cx.listener(|this, _, _, cx| this.take(cx))));
        let cancel = ringed(&look, ghost(&look, "take-cancel", "Cancel", false), &self.cancel_focus, window).track_focus(&self.cancel_focus).on_click(cx.listener(|_, _, _, cx| cx.emit(Event::Cancel)));
        let acts = div().flex().items_center().gap(look.length(token::SPACE_2)).child(labelled(take, Role::Button, self.take)).child(labelled(cancel, Role::Button, "Cancel"));
        labelled(div().id("workflow-take").flex().flex_col().gap(look.length(token::SPACE_3)).min_w_0().child(sides).child(list).child(acts), Role::Region, self.title)
    }
}

/// `lanes` with each node a row of this side is about marked as the canvas marks a changed node, and as a step that went
/// through where one of its rows is chosen.
fn marked(lanes: &JobLanesView, rows: &[Row], side: Which) -> JobLanesView {
    let mut view = lanes.clone();
    for node in view.lanes.iter_mut().flat_map(|lane| lane.nodes.iter_mut()) {
        let mine: Vec<&Row> = rows
            .iter()
            .filter(|row| match side {
                Which::Left => row.spec.left.as_deref() == Some(node.id.as_str()),
                Which::Right => row.spec.right.as_deref() == Some(node.id.as_str()),
            })
            .collect();
        if mine.is_empty() {
            continue;
        }
        node.changed = Some(mine.iter().map(|row| if row.chosen { format!("Taking: {}", row.spec.said) } else { row.spec.said.clone() }).collect::<Vec<_>>().join("\n"));
        node.passed = mine.iter().any(|row| row.chosen);
    }
    view
}

/// The workflow's lane of `lanes` alone, with a node for each step of `text` in its order: Fleet's lanes of a workflow
/// stand where its text says, so where they do this changes nothing. A step Fleet drew keeps its node.
pub fn steps_lane(lanes: &JobLanesView, text: &str) -> JobLanesView {
    let mut view = lanes.clone();
    view.lanes.retain(|lane| lane.kind == LaneKind::Workflow);
    view.rules.clear();
    let steps = steps_of(text);
    let Some(lane) = view.lanes.first_mut().filter(|_| !steps.is_empty()) else { return view };
    let drawn: Vec<ipc::LaneNode> = lane.nodes.iter().filter(|node| node.kind == LaneNodeKind::Step).cloned().collect();
    lane.nodes.retain(|node| node.kind != LaneNodeKind::Step);
    lane.moves.clear();
    for (col, (id, label)) in steps.iter().enumerate() {
        let col = col as u32;
        let node = match drawn.iter().find(|node| node.id == node_id(id)) {
            Some(node) => ipc::LaneNode { label: label.clone(), col, changed: None, ..node.clone() },
            None => ipc::LaneNode { changed: None, ..step_node(id, label, col) },
        };
        lane.nodes.push(node);
    }
    let last = steps.len() as u32 - 1;
    for node in lane.nodes.iter_mut().filter(|node| node.kind == LaneNodeKind::End) {
        node.col = last;
    }
    view
}

/// What each difference Armada made says, and where it is marked: on Armada's node for the step, and for a step Armada took
/// out, which has no node there, on his.
pub fn armada_rows(differences: &[Difference], armada: &str) -> Vec<Spec> {
    let steps = steps_of(armada);
    let label_of = |id: &str| steps.iter().find(|(held, _)| held == id).map_or(id.to_string(), |(_, label)| label.clone());
    let place = |after: &Option<String>, was: &str| match after {
        Some(after) => format!("{was} after {}", label_of(after)),
        None => format!("{was} first"),
    };
    differences
        .iter()
        .map(|one| {
            let node = Some(node_id(&one.step));
            let (mark, said) = match &one.kind {
                Kind::Added { after } => (Mark::Added, place(after, &format!("{} added", one.label))),
                Kind::Removed => (Mark::Removed, format!("{} removed", one.label)),
                Kind::Moved { after } => (Mark::Moved, place(after, &format!("{} moved", one.label))),
                Kind::Renamed { was } => (Mark::Renamed, format!("{} renamed from {was}", one.label)),
                Kind::Words => (Mark::Words, format!("{}: other words", one.label)),
            };
            let removed = mark == Mark::Removed;
            Spec { mark, said, left: node.clone().filter(|_| !removed), right: node.filter(|_| removed) }
        })
        .collect()
}
