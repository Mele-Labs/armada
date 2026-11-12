//! Spine, as Armada Classic's `StoryColumn` draws it outside a pane of its own (`Spine`): one scroll
//! with a line down its left and the zones on it. The Brief leads, then Done, Now and Open, and
//! Deferred at the foot while anything is kept. The line is solid through Done, heavier through Now
//! and dashed through Open; each band is the marker on it, a hollow dot, or a filled one for Now.
//! A band holds at the column's top while its zone passes under it. A press, Enter or Space on a
//! band folds its zone (gpui presses a focused element on Enter and Space), kept per Job on this machine; Done starts folded. ⌥1 to ⌥4 fold a zone and
//! bring its band to the top while the Spine is in front. The Spine opens with Now's band at the top.
//!
//! gpui has no sticky position, so the band of the zone passing under the top is drawn again over
//! the scroll, and its place in the zone is left empty, so each band stands once on screen.
//!
//! Owed: the workflow's head with ⌥W, its step zones with Skip and Jump, a gated plan's zones, and
//! Hide Open group, which land with the Workflow pane; a task chip's press, which opens Projects,
//! a page native does not have yet.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;

use gpui::{div, prelude::*, AnyElement, App, Context, FocusHandle, FontWeight, KeyDownEvent, MouseButton, Pixels, Role, ScrollHandle, SharedString, Window};
use ipc::{DeliveryChoice, ProjectRow, ProjectTaskState, SessionMode, SessionRow, SessionVoice, ThreadItem};

use super::head::drawn;
use super::menus::Teach;
use super::now::lamp_mark;
use super::rows::{glyph, mono, Size};
use super::side::Zone;
use super::JobView;
use crate::dropdown::Entry;
use crate::labels::labelled;
use crate::log::Note;
use crate::look::{token, Look, Token};

/// A zone a band folds, in the order of their keys, ⌥1 to ⌥4 (`ZONE_KEYS`).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(super) enum Area {
    Done,
    Now,
    Open,
    Deferred,
}

impl Area {
    const ALL: [Area; 4] = [Area::Done, Area::Now, Area::Open, Area::Deferred];

    /// Classic's name for the zone (`data-area`), which the fold is kept under and Teach names.
    fn id(self) -> &'static str {
        match self {
            Area::Done => "done",
            Area::Now => "now",
            Area::Open => "next",
            Area::Deferred => "deferred",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Area::Done => "Done",
            Area::Now => "Now",
            Area::Open => "Open",
            Area::Deferred => "Deferred",
        }
    }

    fn key(self) -> &'static str {
        match self {
            Area::Done => "alt-1",
            Area::Now => "alt-2",
            Area::Open => "alt-3",
            Area::Deferred => "alt-4",
        }
    }

    /// The tab that draws the zone alone.
    fn tab(self) -> &'static str {
        match self {
            Area::Done => "done",
            Area::Now => "now",
            Area::Open => "open",
            Area::Deferred => "deferred",
        }
    }

    pub(super) fn of_digit(key: &str) -> Option<Area> {
        Some(match key {
            "1" => Area::Done,
            "2" => Area::Now,
            "3" => Area::Open,
            "4" => Area::Deferred,
            _ => return None,
        })
    }

    pub(super) fn at(self) -> usize {
        Area::ALL.iter().position(|one| *one == self).unwrap_or_default()
    }
}

/// A zone as the column lists them: the Brief, or one that folds.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Kind {
    Brief,
    Zone(Area),
}

/// What the Spine keeps: each Job's folds, its scroll, and the bands' and bodies' keys.
pub(super) struct Spine {
    /// Each zone folded or opened by hand, by Job; a zone not here takes its default.
    folds: HashMap<String, HashMap<Area, bool>>,
    scroll: ScrollHandle,
    /// The zone to bring to the top once the scroll knows where its zones stand. gpui's own
    /// `scroll_to_top_of_item` misses on a scroll's first frame, before it knows its own bounds.
    place: Cell<Option<usize>>,
    /// The Job the Spine last opened at Now for.
    placed: RefCell<Option<SharedString>>,
    /// The zones in the order last drawn, so a key finds its zone's place.
    order: RefCell<Vec<Kind>>,
    /// Where the zones stood and the scroll was when last drawn: while either moves, the Spine draws
    /// again, so the band held at the top is measured from where things came to rest.
    rest: RefCell<Vec<(Pixels, Pixels)>>,
    pub(super) settle: Cell<bool>,
    /// The Brief's band, then each zone's.
    bands: [FocusHandle; 5],
    bodies: [FocusHandle; 4],
    /// Each zone's keys in its own tab, outside the Spine.
    alone: [FocusHandle; 4],
}

/// Where the folds are kept between runs, beside the list's own: `job`, `area`, `open` or `shut`, a
/// line each.
fn folds_file() -> Option<std::path::PathBuf> {
    crate::jobs::deck_file().map(|deck| deck.with_file_name("native-spine-folds"))
}

fn read_folds() -> HashMap<String, HashMap<Area, bool>> {
    let mut folds: HashMap<String, HashMap<Area, bool>> = HashMap::new();
    let text = folds_file().and_then(|path| std::fs::read_to_string(path).ok()).unwrap_or_default();
    for line in text.lines() {
        let mut parts = line.split('\t');
        let (Some(job), Some(area), Some(open)) = (parts.next(), parts.next(), parts.next()) else { continue };
        let Some(area) = Area::ALL.into_iter().find(|one| one.id() == area) else { continue };
        folds.entry(job.to_string()).or_default().insert(area, open == "open");
    }
    folds
}

fn write_folds(folds: &HashMap<String, HashMap<Area, bool>>) {
    let Some(path) = folds_file() else { return };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let mut lines: Vec<String> = folds.iter().flat_map(|(job, areas)| areas.iter().map(move |(area, open)| format!("{job}\t{}\t{}", area.id(), if *open { "open" } else { "shut" }))).collect();
    lines.sort();
    if let Err(why) = std::fs::write(&path, lines.join("\n")) {
        Note::warn("the Spine's folds were not kept").field("why", why.to_string()).emit();
    }
}

impl Spine {
    pub(super) fn new(remember: bool, cx: &mut App) -> Self {
        Spine {
            folds: if remember { read_folds() } else { HashMap::new() },
            scroll: ScrollHandle::new(),
            place: Cell::new(None),
            placed: RefCell::new(None),
            order: RefCell::new(Vec::new()),
            rest: RefCell::new(Vec::new()),
            settle: Cell::new(false),
            bands: std::array::from_fn(|_| cx.focus_handle().tab_stop(true)),
            bodies: std::array::from_fn(|_| cx.focus_handle()),
            alone: std::array::from_fn(|_| cx.focus_handle()),
        }
    }

    /// The Spine is off screen: it opens at Now again when it next comes.
    pub(super) fn left(&self) {
        self.placed.replace(None);
    }

    fn open(&self, job: &str, area: Area) -> bool {
        self.folds.get(job).and_then(|areas| areas.get(&area)).copied().unwrap_or(area != Area::Done)
    }

    fn band(&self, kind: Kind) -> &FocusHandle {
        match kind {
            Kind::Brief => &self.bands[0],
            Kind::Zone(area) => &self.bands[1 + area.at()],
        }
    }
}

/// The brief: his first message, or the message the Job was started with (`briefOf`).
fn brief_of(job: &ipc::SessionRecord, items: &[ThreadItem]) -> Option<String> {
    let sent = items.iter().find_map(|item| match item {
        ThreadItem::Row { row: SessionRow::Message { from: SessionVoice::You, text, .. } } => Some(text.clone()),
        _ => None,
    });
    let text = sent.or_else(|| job.hosted.as_ref().and_then(|hosted| hosted.start.as_ref()).and_then(|start| start.message.clone()))?;
    let text = text.trim();
    (!text.is_empty()).then(|| text.to_string())
}

/// How it started, each row only where it says something (`startedRows`).
fn started_rows(job: &ipc::SessionRecord) -> Vec<(&'static str, String)> {
    let from = job.started.as_ref();
    let hosted = job.hosted.as_ref();
    let delivery = from.and_then(|from| from.delivery).or_else(|| hosted.and_then(|hosted| hosted.start.as_ref()).and_then(|start| start.picked.as_ref()).map(|picked| picked.choice));
    let mode = from.and_then(|from| from.mode).or_else(|| hosted.map(|hosted| hosted.mode));
    let rows = [
        ("Workflow", from.and_then(|from| from.workflow.clone()).or_else(|| job.workflow_copy.as_ref().map(|work| work.name.clone()))),
        (
            "Delivery",
            delivery.map(|choice| {
                match choice {
                    DeliveryChoice::AutoLand => "Auto land",
                    DeliveryChoice::OpenPr => "Open PR only",
                    DeliveryChoice::KeepLocal => "Keep local",
                }
                .to_string()
            }),
        ),
        ("Model", from.and_then(|from| from.model.clone()).or_else(|| hosted.and_then(|hosted| hosted.model.clone()))),
        ("Effort", from.and_then(|from| from.effort.clone()).or_else(|| hosted.and_then(|hosted| hosted.effort.clone()))),
        (
            "Mode",
            mode.map(|mode| {
                match mode {
                    SessionMode::Ask => "Ask",
                    SessionMode::Auto => "Auto",
                    SessionMode::AcceptEdits => "Accept edits",
                    SessionMode::Plan => "Plan",
                }
                .to_string()
            }),
        ),
    ];
    rows.into_iter().filter_map(|(label, value)| value.filter(|value| !value.is_empty()).map(|value| (label, value))).collect()
}

/// A task of his projects the Job holds: its title, its project's name and its state (`useTasksHeld`).
struct Held {
    id: String,
    title: String,
    project: String,
    state: ProjectTaskState,
}

fn tasks_held(projects: &ipc::Projects, job: &str) -> Vec<Held> {
    let named = projects.projects.iter().map(|project| (project.name.as_str(), &project.rows)).chain(std::iter::once(("No project", &projects.loose)));
    named
        .flat_map(|(name, rows)| {
            rows.iter().filter_map(move |row| match row {
                ProjectRow::Task { id, title, state, held_by: Some(holder), .. } if holder.session_id == job => Some(Held { id: id.clone(), title: title.clone(), project: name.to_string(), state: *state }),
                _ => None,
            })
        })
        .collect()
}

/// A task's glyph, word and hue (`TaskChip`).
fn task_look(state: ProjectTaskState) -> (&'static str, &'static str, Token) {
    match state {
        ProjectTaskState::Open => ("circle-dashed", "Open", token::FG_MUTED),
        ProjectTaskState::Held => ("circle-dot", "In a Job", token::STATUS_RUNNING),
        ProjectTaskState::Asked => ("message-circle-question-mark", "Done?", token::STATUS_AWAITING_REVIEW),
        ProjectTaskState::Done => ("check", "Done", token::STEP_ADVANCED),
    }
}

/// Up and Down step between the rows of one zone, and Home and End go to its first and last, as
/// Classic's `arrowsIn` does; the keys never leave the zone.
fn arrows(body: &FocusHandle, event: &KeyDownEvent, window: &mut Window, cx: &mut App) -> bool {
    let keys = &event.keystroke;
    if keys.modifiers.modified() {
        return false;
    }
    let (forward, whole) = match keys.key.as_str() {
        "down" => (true, false),
        "up" => (false, false),
        "end" => (true, true),
        "home" => (false, true),
        _ => return false,
    };
    for _ in 0..if whole { 400 } else { 1 } {
        let before = window.focused(cx);
        if forward {
            window.focus_next(cx);
        } else {
            window.focus_prev(cx);
        }
        if !body.contains_focused(window, cx) || window.focused(cx) == before {
            if let Some(before) = before {
                window.focus(&before, cx);
            }
            break;
        }
    }
    true
}

impl JobView {
    /// A zone drawn in its own tab, with the arrows between its rows as in the Spine.
    pub(super) fn zone_alone(&self, area: Area, body: AnyElement, cx: &Context<Self>) -> AnyElement {
        let at = area.at();
        let keys = self.spine.alone[at].clone();
        div()
            .id(SharedString::from(format!("alone:{}", area.id())))
            .track_focus(&self.spine.alone[at])
            .on_key_down(cx.listener(move |_, event: &KeyDownEvent, window, cx| {
                if arrows(&keys, event, window, cx) {
                    cx.stop_propagation();
                }
            }))
            .child(body)
            .into_any_element()
    }

    /// The Spine on a Job.
    pub(super) fn pane_spine(&self, id: &SharedString, look: &Look, cx: &Context<Self>) -> AnyElement {
        let Some(job) = self.record(id, cx).cloned() else { return div().into_any_element() };
        let items: &[ThreadItem] = self.thread(id, cx).map_or(&[], |held| held.items.as_slice());
        let ground = look.color(token::BG_RAISED);
        let edge = look.length(token::BORDER_WIDTH);
        let line = look.length(token::SPACE_4);
        let lead = line + look.length(token::SPACE_4);
        let band_h = look.length(token::H_CONTROL);

        let mut order = Vec::new();
        if brief_of(&job, items).is_some() {
            order.push(Kind::Brief);
        }
        order.extend([Kind::Zone(Area::Done), Kind::Zone(Area::Now), Kind::Zone(Area::Open)]);
        let deferred = self.deferred_block(id, look, cx);
        if deferred.is_some() {
            order.push(Kind::Zone(Area::Deferred));
        }
        let now_at = order.iter().position(|kind| *kind == Kind::Zone(Area::Now)).unwrap_or_default();
        if self.spine.placed.borrow().as_ref() != Some(id) {
            self.spine.placed.replace(Some(id.clone()));
            self.spine.place.set(Some(now_at));
        }

        // A zone's place is measured as laid out, before the scroll moves it.
        let view = self.spine.scroll.bounds();
        if let Some(at) = self.spine.place.get() {
            if let Some(zone) = self.spine.scroll.bounds_for_item(at).filter(|_| view.size.height > Pixels::ZERO) {
                let most = self.spine.scroll.max_offset().y;
                self.spine.scroll.set_offset(gpui::point(Pixels::ZERO, -(zone.origin.y - view.origin.y).min(most).max(Pixels::ZERO)));
                self.spine.place.set(None);
            }
        }

        // The zone passing under the top, and where its band stands: at the top, or pushed up by
        // the zone's own foot.
        let offset = self.spine.scroll.offset().y;
        let stuck = (0..order.len()).find_map(|at| {
            let zone = self.spine.scroll.bounds_for_item(at)?;
            let from = zone.origin.y - view.origin.y + offset;
            let to = from + zone.size.height;
            (from < -gpui::px(0.5) && to > Pixels::ZERO).then(|| (at, (to - band_h).min(Pixels::ZERO)))
        });
        self.spine.order.replace(order.clone());
        let rest: Vec<(Pixels, Pixels)> = std::iter::once((offset, view.size.height)).chain((0..order.len()).filter_map(|at| self.spine.scroll.bounds_for_item(at)).map(|zone| (zone.origin.y, zone.size.height))).collect();
        if *self.spine.rest.borrow() != rest || self.spine.place.get().is_some() {
            self.spine.rest.replace(rest);
            self.spine.settle.set(true);
        }

        let (landed, toggle, done_body) = self.done_parts(id, look, cx);
        let (now_lamp, now_body) = self.now_parts(id, true, look, cx).unwrap_or((None, None));
        let mut acts: HashMap<Area, AnyElement> = HashMap::from([(Area::Done, toggle), (Area::Now, self.now_acts(id, look, cx))]);
        let mut bodies: HashMap<Area, Option<AnyElement>> = HashMap::from([(Area::Done, done_body), (Area::Now, now_body), (Area::Open, self.open_body(id, true, look, cx)), (Area::Deferred, deferred)]);
        let lamp = |area: Area| match area {
            Area::Done => landed.then(|| lamp_mark(look, "spine-done", token::STATUS_COMPLETED_SUCCESS, "Done", false)),
            Area::Now => now_lamp.map(|lamp| {
                let (hue, said, pulse) = lamp.look();
                lamp_mark(look, "spine-now", hue, said, pulse)
            }),
            _ => None,
        };

        let mut overlay = None;
        let zones: Vec<AnyElement> = order
            .iter()
            .enumerate()
            .map(|(at, kind)| {
                let band = match *kind {
                    Kind::Brief => self.brief_band(id, &job, items, look, cx),
                    Kind::Zone(area) => self.zone_band(id, area, lamp(area), acts.remove(&area), look, cx),
                };
                let band = if let Some((_, y)) = stuck.filter(|(stuck, _)| *stuck == at) {
                    overlay = Some(div().absolute().left_0().right_0().top(y).px(look.length(token::SPACE_2)).child(div().bg(ground).child(band)));
                    div().h(band_h).into_any_element()
                } else {
                    band
                };
                match *kind {
                    Kind::Brief => self.brief_zone(id, &job, items, band, look, cx),
                    Kind::Zone(area) => {
                        let open = self.spine.open(id, area);
                        let body = bodies.remove(&area).flatten().filter(|_| open);
                        let now = area == Area::Now;
                        let thread = (open || body.is_some()).then(|| {
                            let rule = div().absolute().bottom_0().border_l(edge * if now { 2. } else { 1. }).border_color(look.color(if now { token::FG_DEFAULT } else { token::BORDER_STRONG }));
                            let rule = if now { rule.left(line - edge / 2.) } else { rule.left(line) };
                            let rule = if area == Area::Done { rule.top(band_h / 2.) } else { rule.top_0() };
                            if area == Area::Open {
                                rule.border_dashed()
                            } else {
                                rule
                            }
                        });
                        let at = area.at();
                        let body = body.map(|body| {
                            let keys = self.spine.bodies[at].clone();
                            div()
                                .id(SharedString::from(format!("spine-body:{}", area.id())))
                                .track_focus(&self.spine.bodies[at])
                                .pl(lead)
                                .pb(look.length(token::SPACE_3))
                                .on_key_down(cx.listener(move |_, event: &KeyDownEvent, window, cx| {
                                    if arrows(&keys, event, window, cx) {
                                        cx.stop_propagation();
                                    }
                                }))
                                .child(body)
                        });
                        div().relative().flex_none().flex().flex_col().children(thread).child(band).children(body).into_any_element()
                    }
                }
            })
            .collect();

        let column = div()
            .id("spine")
            .size_full()
            .flex()
            .flex_col()
            .overflow_y_scroll()
            .track_scroll(&self.spine.scroll)
            .px(look.length(token::SPACE_2))
            .pb(look.length(token::SPACE_4))
            .bg(ground)
            .on_scroll(cx.listener(|_, _: &gpui::ScrollEvent, _, cx| cx.notify()))
            .children(zones);
        labelled(div().id("spine-frame").relative().flex_1().min_h_0().child(column).children(overlay), Role::Region, "Spine").into_any_element()
    }

    /// The marker on the line at the band's left: hollow, or filled and smaller for Now.
    fn spine_dot(look: &Look, now: bool) -> gpui::Div {
        let edge = look.length(token::BORDER_WIDTH);
        let line = look.length(token::SPACE_4);
        let dot = div().absolute().top_0().bottom_0().flex().items_center();
        if now {
            dot.left(line + edge / 2. - look.length(token::SPACE_2) / 2.).child(div().size(look.length(token::SPACE_2)).rounded_full().bg(look.color(token::FG_DEFAULT)))
        } else {
            dot.left(line - look.length(token::SPACE_1)).child(div().size(look.length(token::SPACE_2) + edge).rounded_full().bg(look.color(token::BG_RAISED)).border(edge).border_color(look.color(token::BORDER_STRONG)))
        }
    }

    /// A zone's band: the lamp, the label with its key, the acts. A press, Enter or Space folds it;
    /// a right press, the ContextMenu key or Shift+F10 opens its menu.
    fn zone_band(&self, id: &SharedString, area: Area, lamp: Option<AnyElement>, acts: Option<AnyElement>, look: &Look, cx: &Context<Self>) -> AnyElement {
        let open = self.spine.open(id, area);
        let label = area.label();
        let said: SharedString = if open { format!("Fold {label}").into() } else { format!("Open {label}").into() };
        let key = gpui::Keystroke::parse(area.key()).ok();
        let now = area == Area::Now;
        let acts = acts.map(|acts| div().flex().items_center().gap(look.length(token::SPACE_2)).on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation()).child(acts));
        let band = div()
            .id(SharedString::from(format!("spine-band:{}", area.id())))
            .track_focus(self.spine.band(Kind::Zone(area)))
            .relative()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_3))
            .h(look.length(token::H_CONTROL))
            .pl(look.length(token::SPACE_4) * 2.)
            .rounded(look.length(token::RADIUS_SM))
            .font_family(mono(look))
            .text_size(look.length(token::TEXT_XS))
            .line_height(look.length(token::LEADING_XS))
            .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
            .text_color(look.color(if now { token::FG_DEFAULT } else { token::FG_MUTED }))
            .cursor_pointer()
            .aria_expanded(open)
            .focus_visible(|style| style.border_1().border_color(look.color(token::ACCENT)))
            .child(JobView::spine_dot(look, now))
            .children(lamp)
            .child(
                div()
                    .id(SharedString::from(format!("spine-label:{}", area.id())))
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .tooltip(move |window, cx| gpui_component::tooltip::Tooltip::new(said.clone()).key_binding(key.clone().map(gpui_component::kbd::Kbd::new)).build(window, cx))
                    .child(label),
            )
            .children(acts)
            .on_click(cx.listener(move |this, _, _, cx| this.spine_fold(area, cx)))
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(move |this, event: &gpui::MouseDownEvent, window, cx| {
                    this.spine_menu(area, event.position, false, window, cx);
                    cx.stop_propagation();
                }),
            )
            .on_key_down(cx.listener(move |this, event: &KeyDownEvent, window, cx| {
                let keys = &event.keystroke;
                if !this.spine.band(Kind::Zone(area)).is_focused(window) {
                    return;
                }
                if keys.key == "menu" || (keys.key == "f10" && keys.modifiers.shift) {
                    let at = drawn(window, cx, Role::Button, label).map_or_else(|| window.mouse_position(), |bounds| gpui::point(bounds.center().x, bounds.bottom()));
                    this.spine_menu(area, at, true, window, cx);
                    cx.stop_propagation();
                }
            }));
        labelled(band, Role::Button, label).into_any_element()
    }

    /// The Brief's band: one press across the width, with the label, the brief's first line while it
    /// is shut, and a caret that turns when it is open (`BriefZone`).
    fn brief_band(&self, id: &SharedString, job: &ipc::SessionRecord, items: &[ThreadItem], look: &Look, cx: &Context<Self>) -> AnyElement {
        let open = self.pages.get(id).is_some_and(|page| page.side.brief_open);
        let text = brief_of(job, items).unwrap_or_default();
        let caret = glyph(look, "chevron-down", Size::Small, look.color(token::FG_SUBTLE));
        let caret = if open { caret.with_transformation(gpui::Transformation::rotate(gpui::percentage(0.5))) } else { caret };
        let excerpt = (!open).then(|| {
            div()
                .flex_1()
                .min_w_0()
                .truncate()
                .font_family(look.font_stack(token::FONT_SANS).into_iter().next().unwrap_or_default())
                .font_weight(FontWeight(look.number(token::WEIGHT_BODY)))
                .text_color(look.color(token::FG_DEFAULT))
                .child(SharedString::from(text.lines().next().unwrap_or_default().to_string()))
        });
        let band = div()
            .id("spine-band:brief")
            .track_focus(self.spine.band(Kind::Brief))
            .relative()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_3))
            .h(look.length(token::H_CONTROL))
            .pl(look.length(token::SPACE_4) * 2.)
            .pr(look.length(token::SPACE_2))
            .rounded(look.length(token::RADIUS_SM))
            .font_family(mono(look))
            .text_size(look.length(token::TEXT_XS))
            .line_height(look.length(token::LEADING_XS))
            .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
            .text_color(look.color(token::FG_MUTED))
            .cursor_pointer()
            .hover(|style| style.text_color(look.color(token::FG_DEFAULT)))
            .aria_expanded(open)
            .focus_visible(|style| style.border_1().border_color(look.color(token::ACCENT)))
            .child(JobView::spine_dot(look, false))
            .child(div().flex_none().child("Brief"))
            .children(excerpt)
            .child(div().flex_1())
            .child(caret)
            .on_click(cx.listener(|this, _, _, cx| this.brief_turn(cx)));
        labelled(band, Role::Button, "Brief").into_any_element()
    }

    fn brief_turn(&mut self, cx: &mut Context<Self>) {
        if let Some(page) = self.shown.clone().and_then(|id| self.pages.get_mut(&id)) {
            page.side.brief_open = !page.side.brief_open;
        }
        cx.notify();
    }

    /// The Brief: the whole brief and how the Job started while open, and under it, open or shut, the
    /// tasks of his projects the Job holds.
    fn brief_zone(&self, id: &SharedString, job: &ipc::SessionRecord, items: &[ThreadItem], band: AnyElement, look: &Look, cx: &Context<Self>) -> AnyElement {
        let open = self.pages.get(id).is_some_and(|page| page.side.brief_open);
        let edge = look.length(token::BORDER_WIDTH);
        let lead = look.length(token::SPACE_4) * 2.;
        let sans = look.font_stack(token::FONT_SANS).into_iter().next().unwrap_or_default();
        let row = |label: &'static str, value: AnyElement| {
            div()
                .flex()
                .items_baseline()
                .gap(look.length(token::SPACE_4))
                .child(div().flex_none().w(look.length(token::SPACE_12) + look.length(token::SPACE_4)).text_size(look.length(token::TEXT_2XS)).line_height(look.length(token::LEADING_2XS)).text_color(look.color(token::FG_SUBTLE)).child(label))
                .child(div().flex_1().min_w_0().text_color(look.color(token::FG_DEFAULT)).child(value))
        };
        let body = open.then(|| {
            let mut rows: Vec<gpui::Div> = started_rows(job).into_iter().map(|(label, value)| row(label, SharedString::from(value).into_any_element())).collect();
            if let Some(by) = job.started.as_ref().and_then(|started| started.by.clone()) {
                let to = by.id.as_str().to_string();
                rows.push(row(
                    "Started by",
                    labelled(
                        div()
                            .id("brief-by")
                            .text_color(look.color(token::ACCENT))
                            .cursor_pointer()
                            .tab_index(0)
                            .hover(|style| style.underline())
                            .focus_visible(|style| style.border_1().border_color(look.color(token::ACCENT)))
                            .child(SharedString::from(by.title.clone()))
                            .on_click(cx.listener(move |this, _, _, cx| this.open_job(to.clone(), cx))),
                        Role::Link,
                        by.title.clone(),
                    )
                    .into_any_element(),
                ));
            }
            let started = (!rows.is_empty()).then(|| {
                labelled(
                    div().id("brief-started").flex().flex_col().gap(look.length(token::SPACE_1)).mt(look.length(token::SPACE_3)).pt(look.length(token::SPACE_2)).pr(look.length(token::SPACE_2)).border_t(edge).border_color(look.color(token::BORDER_SUBTLE)).children(rows),
                    Role::List,
                    "How it started",
                )
            });
            div()
                .pl(lead)
                .pb(look.length(token::SPACE_3))
                .font_family(sans.clone())
                .text_size(look.length(token::TEXT_XS))
                .line_height(look.length(token::LEADING_XS))
                .child(div().pr(look.length(token::SPACE_2)).text_color(look.color(token::FG_DEFAULT)).whitespace_normal().child(SharedString::from(brief_of(job, items).unwrap_or_default())))
                .children(started)
        });
        let projects = match &self.fleet {
            Some(fleet) => fleet.read(cx).held().projects.clone(),
            None => Some(crate::mock::projects()),
        };
        let held = projects.map(|projects| tasks_held(&projects, id)).unwrap_or_default();
        let tasks = (!held.is_empty()).then(|| {
            let label = if held.len() == 1 { "Task" } else { "Tasks" };
            let chips: Vec<AnyElement> = held
                .iter()
                .map(|task| {
                    let (icon, word, ink) = task_look(task.state);
                    labelled(
                        div()
                            .id(SharedString::from(format!("task:{}", task.id)))
                            .flex()
                            .items_center()
                            .gap(look.length(token::SPACE_2))
                            .py(look.length(token::SPACE_1))
                            .px(look.length(token::SPACE_2))
                            .rounded(look.length(token::RADIUS_MD))
                            .child(labelled(div().id(SharedString::from(format!("task-state:{}", task.id))).flex_none().tooltip(super::rows::tip(word)).child(glyph(look, icon, Size::Small, look.color(ink))), Role::Image, word))
                            .child(div().min_w_0().truncate().text_color(look.color(token::FG_DEFAULT)).child(SharedString::from(task.title.clone())))
                            .child(div().flex_none().text_size(look.length(token::TEXT_2XS)).line_height(look.length(token::LEADING_2XS)).text_color(look.color(token::FG_SUBTLE)).child(SharedString::from(task.project.clone()))),
                        Role::Group,
                        format!("Task {}", task.title),
                    )
                    .into_any_element()
                })
                .collect();
            div()
                .flex()
                .items_baseline()
                .gap(look.length(token::SPACE_4))
                .pl(lead)
                .pr(look.length(token::SPACE_2))
                .font_family(sans.clone())
                .text_size(look.length(token::TEXT_XS))
                .line_height(look.length(token::LEADING_XS))
                .child(div().flex_none().text_size(look.length(token::TEXT_2XS)).line_height(look.length(token::LEADING_2XS)).text_color(look.color(token::FG_SUBTLE)).child(label))
                .child(labelled(div().id("brief-tasks").flex_1().min_w_0().flex().flex_col().gap(look.length(token::SPACE_1)).children(chips), Role::List, label))
        });
        let rule = div().absolute().top(look.length(token::H_CONTROL) / 2.).bottom_0().left(look.length(token::SPACE_4)).border_l(edge).border_color(look.color(token::BORDER_STRONG));
        labelled(div().id("spine-brief").relative().flex_none().flex().flex_col().pb(look.length(token::SPACE_3)).child(rule).child(band).children(body).children(tasks), Role::Region, "Brief").into_any_element()
    }

    /// Fold or open a zone on the Job shown, kept on the machine where a Fleet is live.
    fn spine_fold(&mut self, area: Area, cx: &mut Context<Self>) {
        let Some(id) = self.shown.clone() else { return };
        let open = self.spine.open(&id, area);
        self.spine.folds.entry(id.to_string()).or_default().insert(area, !open);
        if self.fleet.is_some() {
            write_folds(&self.spine.folds);
        }
        cx.notify();
    }

    /// The band's menu: Teach (`BandMenu`). Shut, the keys go back to the band.
    fn spine_menu(&mut self, area: Area, at: gpui::Point<Pixels>, from_keys: bool, window: &mut Window, cx: &mut Context<Self>) {
        let entries = vec![Entry::Act { id: "teach", label: "Teach".into(), icon: Some("graduation-cap"), danger: false }];
        let back = Some(self.spine.band(Kind::Zone(area)).clone());
        self.open_pop(
            area.label(),
            entries,
            at,
            from_keys,
            back,
            move |this, picked, window, cx| {
                if picked == "teach" {
                    this.teach_open(Teach { kind: "panel", text: area.label().to_string(), at: None, ids: vec![area.id().to_string()] }, window, cx);
                }
            },
            window,
            cx,
        );
    }

    /// ⌥1 to ⌥4 (`bandKeyFor`): with the Spine in front, fold the zone and bring its band to the top;
    /// otherwise bring the zone's own tab forward, or the Spine where only it is open, or open the
    /// zone's tab.
    pub(super) fn spine_key(&mut self, area: Area, window: &mut Window, cx: &mut Context<Self>) {
        let Some(id) = self.shown.clone() else { return };
        if self.zone_shown() == Some(Zone::Spine) {
            self.spine_fold(area, cx);
            self.spine_place(area, window, cx);
            return;
        }
        if !self.tab_closed(area.tab()) {
            if let Some(zone) = Zone::ALL.into_iter().find(|zone| zone.id() == area.tab()) {
                self.zone = zone;
            }
            cx.notify();
            return;
        }
        if !self.tab_closed("spine") {
            self.zone = Zone::Spine;
            if !self.spine.open(&id, area) {
                self.spine_fold(area, cx);
            }
            self.spine_place(area, window, cx);
            return;
        }
        self.add_tab(area.tab(), window, cx);
    }

    /// The zone's band at the Spine's top, with the keys on it.
    fn spine_place(&mut self, area: Area, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(at) = self.spine.order.borrow().iter().position(|kind| *kind == Kind::Zone(area)) {
            self.spine.place.set(Some(at));
        }
        let band = self.spine.band(Kind::Zone(area)).clone();
        window.focus(&band, cx);
        cx.notify();
    }
}
