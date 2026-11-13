//! The page drawn, as the approved sketch "Morning report: a written document" lays it out: the head and
//! its pills, the top line, Your turn with one item open and the rest one line each, and the digest beside
//! it, which the chat takes the place of on Ask. Roomier than the sketch: controls a step taller, corners
//! rounder, as the native screens are.

use gpui::{
    canvas, div, prelude::*, AnyElement, App, Context, Div, FontWeight, Hsla, MouseButton, Pixels, Role, SharedString, Stateful, Window,
};
use ipc::operator_page::{OperatorAnswer, OperatorPage, OperatorTurnItem, OperatorTurnKind};
use ipc::MainCiState;

use super::face::{self, Kind};
use super::page::{Done, Opened, OperatorPageView};
use crate::labels::labelled;
use crate::look::{token, ActiveLook, Look, Token};
use crate::titlebar::parts::{glyph, kbd, sans, spinner, tip};

/// An item of Your turn held by the pointer.
#[derive(Clone)]
pub struct TurnDrag {
    pub id: String,
    pub title: SharedString,
}

impl Render for TurnDrag {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let look = cx.look();
        div()
            .px(look.length(token::SPACE_3))
            .h(look.length(token::H_CONTROL_LG))
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .rounded(round(look))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::OPERATOR_EDGE))
            .bg(look.color(token::BG_OVERLAY))
            .shadow(look.shadows(token::SHADOW_OVERLAY))
            .font_family(sans(look))
            .text_size(look.length(token::TEXT_SM))
            .text_color(look.color(token::FG_DEFAULT))
            .child(glyph("grip-vertical", look.length(token::SPACE_3), look.color(token::FG_SUBTLE)))
            .child(self.title.clone())
    }
}

/// The card's corners: rounder than a control's, as the native screens draw cards.
fn round(look: &Look) -> Pixels {
    look.length(token::RADIUS_LG) * 1.5
}

pub(crate) fn chip(word: impl Into<SharedString>, hue: Hsla, ground: Hsla, look: &Look) -> Div {
    div()
        .flex_none()
        .flex()
        .items_center()
        .gap(look.length(token::SPACE_1))
        .h(look.length(token::H_BADGE))
        .px(look.length(token::SPACE_2))
        .rounded_full()
        .bg(ground)
        .text_color(hue)
        .text_size(look.length(token::TEXT_2XS))
        .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
        .child(word.into())
}

pub(crate) fn kind_chip(kind: Kind, look: &Look) -> Div {
    chip(kind.word, look.color(kind.hue), look.color(kind.ground), look)
}

/// A press on the page: the pick filled in the Operator's hue, the rest outlined, Later and Ask bare. `JobPick` is a
/// Job's own pick on one of its questions, filled in the accent that rings a Job's pick elsewhere.
#[derive(Clone, Copy, PartialEq)]
pub(crate) enum Weight {
    Pick,
    JobPick,
    Plain,
    Bare,
}

pub(crate) fn press(id: impl Into<SharedString>, label: SharedString, icon: Option<&'static str>, key: Option<SharedString>, weight: Weight, look: &Look) -> Stateful<Div> {
    let id: SharedString = id.into();
    let (ground, hover, ink, edge) = match weight {
        Weight::Pick => (look.color(token::OPERATOR), look.color(token::OPERATOR), look.color(token::FG_INVERSE), look.color(token::OPERATOR)),
        Weight::JobPick => (look.color(token::ACCENT), look.color(token::ACCENT_HOVER), look.color(token::TEXT_ON_ACCENT), look.color(token::ACCENT)),
        Weight::Plain => (look.color(token::BG_RAISED), look.color(token::BG_HOVER), look.color(token::FG_DEFAULT), look.color(token::BORDER_STRONG)),
        Weight::Bare => (gpui::transparent_black(), look.color(token::BG_HOVER), look.color(token::FG_MUTED), gpui::transparent_black()),
    };
    let ring = look.edge(token::FOCUS_RING).map_or(look.color(token::ACCENT), |(_, colour)| colour);
    let cap = key.map(|key| {
        let cap = kbd(key, look);
        if matches!(weight, Weight::Pick | Weight::JobPick) { cap.bg(ink.opacity(0.16)).border_color(ink.opacity(0.5)).text_color(ink) } else { cap }
    });
    div()
        .id(id)
        .flex_none()
        .flex()
        .items_center()
        .gap(look.length(token::SPACE_2))
        .h(look.length(token::H_CONTROL_LG))
        .px(look.length(token::SPACE_3))
        .rounded(look.length(token::RADIUS_LG))
        .border(look.length(token::BORDER_WIDTH))
        .border_color(edge)
        .bg(ground)
        .text_color(ink)
        .text_size(look.length(token::TEXT_SM))
        .font_weight(FontWeight(look.number(if matches!(weight, Weight::Pick | Weight::JobPick) { token::WEIGHT_HEADING } else { token::WEIGHT_MEDIUM })))
        .cursor_pointer()
        .hover(move |style| style.bg(hover))
        .focusable()
        .tab_stop(true)
        .focus_visible(move |style| style.border_color(ring))
        .on_mouse_down(MouseButton::Left, |_, window, _| window.prevent_default())
        .children(icon.map(|icon| glyph(icon, look.length(token::SPACE_4), ink)))
        .child(label)
        .children(cap)
}

/// What a press does on a click, and on Enter or Space while it has the keys: the key goes no further,
/// so the page's own Enter (Yes) is not said as well.
fn pressed(
    element: Stateful<Div>,
    cx: &Context<OperatorPageView>,
    act: impl Fn(&mut OperatorPageView, &mut Window, &mut Context<OperatorPageView>) + 'static,
) -> Stateful<Div> {
    let act = std::rc::Rc::new(act);
    let by_key = act.clone();
    element.on_click(cx.listener(move |this, _, window, cx| act(this, window, cx))).on_key_down(cx.listener(move |this, event: &gpui::KeyDownEvent, window, cx| {
        let key = &event.keystroke;
        if matches!(key.key.as_str(), "enter" | "space") && !key.modifiers.modified() {
            cx.stop_propagation();
            by_key(this, window, cx);
        }
    }))
}

/// A press on one ask: that ask is the open one before the press acts, so in a Job's card each ask's
/// presses act on their own ask, and the keys go on acting on it after.
fn on_ask(
    id: &str,
    act: impl Fn(&mut OperatorPageView, &mut Window, &mut Context<OperatorPageView>) + 'static,
) -> impl Fn(&mut OperatorPageView, &mut Window, &mut Context<OperatorPageView>) + 'static {
    let id = id.to_string();
    move |this, window, cx| {
        this.open = Some(id.clone());
        act(this, window, cx)
    }
}

/// A press drawn as words, such as a link or Show all: a tab stop whose ring sits on the words alone.
fn worded(element: Stateful<Div>, look: &Look) -> Stateful<Div> {
    let ring = look.edge(token::FOCUS_RING).map_or(look.color(token::ACCENT), |(_, colour)| colour);
    element
        .focusable()
        .tab_stop(true)
        .px(look.length(token::SPACE_1))
        .rounded(look.length(token::RADIUS_SM))
        .border(look.length(token::BORDER_WIDTH))
        .border_color(gpui::transparent_black())
        .focus_visible(move |style| style.border_color(ring))
        .cursor_pointer()
}

/// A grey shape where words will be, while the page is read.
fn bone(width: f32, height: Pixels, look: &Look) -> Div {
    div().h(height).w(gpui::relative(width)).rounded(look.length(token::RADIUS_LG)).bg(look.color(token::BG_HOVER))
}

impl OperatorPageView {
    // -- the head ----------------------------------------------------------------------------------

    fn head(&self, look: &Look, cx: &Context<Self>) -> impl IntoElement {
        let ring = look.edge(token::FOCUS_RING).map_or(look.color(token::ACCENT), |(_, colour)| colour);
        // The first pill drawn is where Tab goes from the page itself.
        let first = std::cell::Cell::new(Some(self.first_stop.clone()));
        let pill = |id: &'static str| {
            div()
                .id(id)
                .when_some(first.take(), |pill, keys| pill.track_focus(&keys))
                // Tab reaches each pill, and its tooltip says what it reads.
                .focusable()
                .tab_stop(true)
                .focus_visible(move |style| style.border_color(ring))
                .flex_none()
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .h(look.length(token::H_CONTROL))
                .px(look.length(token::SPACE_3))
                .rounded_full()
                .border(look.length(token::BORDER_WIDTH))
                .border_color(gpui::transparent_black())
                .text_color(look.color(token::FG_MUTED))
                .text_size(look.length(token::TEXT_XS))
                .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
        };
        let meter = |share: f32, fill: Token| {
            div()
                .w(look.length(token::SPACE_8) + look.length(token::SPACE_1))
                .h(look.length(token::SPACE_1) * 1.5)
                .rounded_full()
                .bg(look.color(token::BG_SUNKEN))
                .child(div().h_full().w(gpui::relative(share.clamp(0., 1.))).rounded_full().bg(look.color(fill)))
        };
        let now = self.clock_now.unwrap_or_else(chrono::Local::now);
        let (mains, slots_read) = self.page.as_ref().map_or((&[][..], None), |page| (&page.mains[..], page.slots.as_ref()));
        let mains = mains.iter().enumerate().map(|(at, main)| {
            let since = main.since.as_ref().and_then(|since| minute(since.as_str(), now));
            let (said, mark) = face::main_said(main, since.as_deref());
            let red = main.state == Some(MainCiState::Red);
            let pill = pill(if at == 0 { "pill-main-0" } else if at == 1 { "pill-main-1" } else { "pill-main" })
                .when(red, |pill| {
                    pill.border_color(look.color(token::STATUS_COMPLETED_FAILED))
                        .bg(look.color(token::STATUS_COMPLETED_FAILED_BG))
                        .text_color(look.color(token::STATUS_COMPLETED_FAILED))
                })
                .tooltip(tip(said.clone()))
                .children(mark.map(|(mark, hue)| glyph(mark, look.length(token::SPACE_3) + look.length(token::SPACE_1) / 4., look.color(if red { hue } else { token::FG_MUTED }))))
                .child(format!("{} main", main.repository));
            labelled(pill, Role::Status, said).into_any_element()
        });
        let mains: Vec<AnyElement> = mains.collect();
        // The week's plan stands in the title bar's rings, so the head does not repeat it (12 Nov).
        let slots = slots_read.map(|slots| {
            let (held, all) = (slots.used, slots.total);
            let said = format!("Slots {held} of {all} held");
            labelled(
                pill("pill-slots").tooltip(tip(said.clone())).child(format!("Slots {held} of {all}")).child(meter(held as f32 / all.max(1) as f32, token::FG_MUTED)),
                Role::Status,
                said,
            )
        });
        div()
            .flex_none()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .px(look.length(token::SPACE_6))
            .py(look.length(token::SPACE_3))
            .children(self.day.clone().map(|day| {
                labelled(
                    div().id("operator-day").text_size(look.length(token::TEXT_BASE)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).child(day.clone()),
                    Role::Heading,
                    day,
                )
            }))
            .child(
                div()
                    .flex_1()
                    .flex()
                    .items_center()
                    .text_size(look.length(token::TEXT_XS))
                    .text_color(look.color(token::FG_MUTED))
                    .when_some(self.back_at.as_deref(), |back, at| back.child("back").child(self.stamp(at, Some("at"), (token::TEXT_XS, token::FG_MUTED, token::WEIGHT_BODY), look, cx))),
            )
            .children(mains)
            .children(slots)
    }

    /// The Operator's one line on what changed: grey words under the day, as nothing on it is to be pressed.
    fn top_line(&self, page: &OperatorPage, look: &Look) -> Option<AnyElement> {
        let top = page.top.as_deref()?.replace("**", "");
        Some(
            labelled(
                div()
                    .id("operator-top")
                    .px(look.length(token::SPACE_6))
                    .pb(look.length(token::SPACE_3))
                    .text_size(look.length(token::TEXT_SM))
                    .line_height(look.length(token::LEADING_SM))
                    .text_color(look.color(token::FG_MUTED))
                    .child(top.clone()),
                Role::Status,
                top,
            )
            .into_any_element(),
        )
    }

    // -- Your turn ---------------------------------------------------------------------------------

    fn turn_head(&self, look: &Look) -> impl IntoElement {
        let items = self.entries();
        let open = |run: &[OperatorTurnItem]| run.iter().any(|turn| self.open.as_deref() == Some(turn.id.as_str()));
        let dots = self
            .answered_today()
            .into_iter()
            .map(|_| token::STATUS_COMPLETED_SUCCESS)
            .chain(items.iter().map(|(_, run)| if open(run) { token::OPERATOR } else { token::BORDER_STRONG }));
        let dots: Vec<_> = dots
            .map(|hue| div().w(look.length(token::SPACE_4) + look.length(token::SPACE_1)).h(look.length(token::SPACE_1) * 1.5).rounded_full().bg(look.color(hue)))
            .collect();
        div()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_3))
            .child(glyph("bell", look.length(token::SPACE_4) + look.length(token::SPACE_1) / 4., look.color(token::STATUS_AWAITING_REVIEW)))
            .child(labelled(
                div().id("your-turn").text_size(look.length(token::TEXT_BASE)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).child("Your turn"),
                Role::Heading,
                "Your turn",
            ))
            .when(items.len() > 1, |head| head.child(div().text_size(look.length(token::TEXT_XS)).text_color(look.color(token::FG_SUBTLE)).child("drag to reorder")))
            .child(div().flex_1())
            .when(dots.len() > 1, |head| head.child(div().flex().gap(look.length(token::SPACE_1) * 1.5).children(dots)))
    }

    /// What the Operator reads now: Fleet writes it `Job: what it asks`, so the Job is named, with where it runs.
    fn on_it(&self, page: &OperatorPage, look: &Look, cx: &Context<Self>) -> Option<AnyElement> {
        let on_it = page.on_it.as_ref()?;
        let (job, asks) = match on_it.reading.split_once(": ") {
            Some((job, asks)) => (Some(job.to_string()), asks.to_string()),
            None => (None, on_it.reading.clone()),
        };
        let repository = job.as_deref().and_then(|job| page.digest.running.iter().find(|one| one.title == job)).and_then(|one| one.repository.clone());
        let said = match &job {
            Some(job) => format!("The Operator is reading what {job} asks: {asks}"),
            None => format!("The Operator is reading {asks}"),
        };
        let heading = FontWeight(look.number(token::WEIGHT_HEADING));
        let words = match job {
            Some(job) => div()
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_1))
                .child(div().flex_none().text_color(look.color(token::FG_MUTED)).child("Reading what"))
                .child(glyph("box", look.length(token::SPACE_4), look.color(token::FG_MUTED)))
                .child(div().flex_none().font_weight(heading).child(job))
                .child(div().flex_none().text_color(look.color(token::FG_MUTED)).child("asks:"))
                .child(div().min_w_0().truncate().child(asks)),
            None => div().flex().gap(look.length(token::SPACE_1)).child(div().flex_none().text_color(look.color(token::FG_MUTED)).child("Reading")).child(div().min_w_0().truncate().child(asks)),
        };
        Some(
            labelled(
                div()
                    .id("operator-on-it")
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_3))
                    .px(look.length(token::SPACE_4))
                    .h(look.length(token::H_CONTROL_LG) + look.length(token::SPACE_2))
                    .rounded(round(look))
                    .border(look.length(token::BORDER_WIDTH))
                    .border_dashed()
                    .border_color(look.color(token::OPERATOR_EDGE))
                    .bg(look.color(token::OPERATOR_FAINT))
                    .text_size(look.length(token::TEXT_SM))
                    .tooltip(tip(said.clone()))
                    .child(spinner("on-it-spin", look.length(token::SPACE_4), look.color(token::OPERATOR), cx))
                    .child(div().flex_1().min_w_0().overflow_hidden().child(words))
                    .children(repository.map(|repository| chip(repository, look.color(token::FG_MUTED), look.color(token::BG_HOVER), look)))
                    .child(
                        div()
                            .flex_none()
                            .flex()
                            .items_center()
                            .text_size(look.length(token::TEXT_XS))
                            .text_color(look.color(token::FG_SUBTLE))
                            .child("since")
                            .child(self.stamp(on_it.since.as_str(), None, (token::TEXT_XS, token::FG_SUBTLE, token::WEIGHT_BODY), look, cx)),
                    ),
                Role::Status,
                said,
            )
            .into_any_element(),
        )
    }

    /// Where other Jobs raised the same ask, folded into this one: a mark in its head, naming them in its tooltip.
    fn also_raised(&self, turn: &OperatorTurnItem, look: &Look) -> Option<AnyElement> {
        if turn.also_raised_by.is_empty() {
            return None;
        }
        let said = format!("Also raised by\n{}", turn.also_raised_by.join("\n"));
        Some(
            labelled(
                div().id("card-also-raised").flex_none().tooltip(tip(said.clone())).child(glyph("layers", look.length(token::SPACE_4), look.color(token::FG_MUTED))),
                Role::Image,
                said,
            )
            .into_any_element(),
        )
    }

    /// The marks beside an ask's kind: where other Jobs raised it too, and Later's chip.
    fn marks(&self, turn: &OperatorTurnItem, look: &Look) -> Vec<AnyElement> {
        let mut marks: Vec<AnyElement> = self.also_raised(turn, look).into_iter().collect();
        if turn.later_until.is_some() {
            let said = face::later_said(turn, self.clock_now.unwrap_or_else(chrono::Local::now));
            marks.push(
                labelled(
                    div().id("card-later").flex_none().tooltip(tip(said.clone())).child(chip("Later", look.color(token::STATUS_NOT_STARTED), look.color(token::STATUS_NOT_STARTED_BG), look)),
                    Role::Image,
                    said,
                )
                .into_any_element(),
            );
        }
        marks
    }

    /// The open item's card. A Job's asks share one: its head once, then each ask with its own acts.
    fn card(&self, run: &[OperatorTurnItem], look: &Look, cx: &Context<Self>) -> AnyElement {
        let first = &run[0];
        let grouped = run.len() > 1;
        let title = face::card_title(first);
        let head = div()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .px(look.length(token::SPACE_4))
            .py(look.length(token::SPACE_3))
            .border_b(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_SUBTLE))
            .when(!grouped, |head| head.child(kind_chip(face::kind_of(first), look)))
            .child(div().flex_1().min_w_0().truncate().text_size(look.length(token::TEXT_LG)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).child(title.clone()))
            .when(!grouped, |head| head.children(self.marks(first, look)))
            .children(first.repository.clone().map(|repository| chip(repository, look.color(token::FG_MUTED), look.color(token::BG_HOVER), look)));
        let card = div()
            .id("operator-card")
            .flex_none()
            .rounded(round(look))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(look.color(if !grouped && first.kind == OperatorTurnKind::GoAhead { token::OPERATOR } else { token::OPERATOR_EDGE }))
            .bg(look.color(token::BG_RAISED))
            .overflow_hidden()
            .on_drag(TurnDrag { id: first.id.clone(), title: title.clone() }, |drag, _, _, cx| cx.new(|_| drag.clone()))
            .drag_over::<TurnDrag>(|style, _, _, cx| style.border_color(cx.look().color(token::OPERATOR)))
            .on_drop(cx.listener({
                let id = first.id.clone();
                move |this, drag: &TurnDrag, _, cx| {
                    if let Some(to) = this.items().iter().position(|one| one.id == id) {
                        this.move_to(&drag.id, to, cx);
                    }
                }
            }))
            .child(head);
        let card = if grouped {
            // Ask's chat opens in the ask it is about, else in the open one.
            let open = self.open.as_deref();
            let about = self.chat.as_ref().map(|chat| chat.read(cx).about.clone());
            let chat_at = about.and_then(|about| run.iter().position(|turn| about.as_deref() == Some(turn.id.as_str())).or_else(|| run.iter().position(|turn| Some(turn.id.as_str()) == open)));
            card.children(run.iter().enumerate().map(|(at, turn)| self.ask_section(at, turn, at + 1 == run.len(), chat_at == Some(at), look, cx)))
        } else {
            card.child(self.body(first, None, look, cx)).children(self.chat_in_card(look)).child(self.acts(first, true, true, look, cx))
        };
        labelled(card, Role::Article, title).into_any_element()
    }

    /// One ask in a Job's card: its kind and marks, what it says, and its own acts. The open one, which the
    /// keys act on, has an edge in the Operator's hue; a press anywhere in another opens it.
    fn ask_section(&self, at: usize, turn: &OperatorTurnItem, last: bool, chat: bool, look: &Look, cx: &Context<Self>) -> AnyElement {
        let open = self.open.as_deref() == Some(turn.id.as_str());
        let id = turn.id.clone();
        let name = face::asks_of(turn).unwrap_or_else(|| face::title_of(turn));
        let line = div().flex().items_center().gap(look.length(token::SPACE_2)).child(kind_chip(face::kind_of(turn), look)).children(self.marks(turn, look));
        let edge = open.then(|| {
            div()
                .absolute()
                .left_0()
                .top(look.length(token::SPACE_3))
                .bottom(look.length(token::SPACE_3))
                .w(look.length(token::BORDER_WIDTH) * 3.)
                .rounded_r(look.length(token::RADIUS_SM))
                .bg(look.color(token::OPERATOR))
        });
        labelled(
            div()
                .id(SharedString::from(format!("card-ask-{at}")))
                .relative()
                .when(at > 0, |section| section.border_t(look.length(token::BORDER_WIDTH)).border_color(look.color(token::BORDER_DEFAULT)))
                .on_click(cx.listener(move |this, _, _, cx| this.open_id(&id, cx)))
                .child(self.body(turn, Some(line), look, cx))
                .children(chat.then(|| self.chat_in_card(look)).flatten())
                .child(self.acts(turn, open, last, look, cx))
                .children(edge)
                .child({
                    let (drawn, key, scroll) = (self.ask_bounds.clone(), turn.id.clone(), self.main_scroll.clone());
                    canvas(move |bounds, _, _| { drawn.borrow_mut().insert(key, bounds - gpui::point(gpui::px(0.), scroll.offset().y)); }, |_, _, _, _| {}).absolute().top_0().left_0().size_full()
                }),
            Role::Group,
            name,
        )
        .into_any_element()
    }

    /// What an ask says: its words under a Job's title, his note, My read, the options and Fleet's refusal.
    fn body(&self, turn: &OperatorTurnItem, lead: Option<Div>, look: &Look, cx: &Context<Self>) -> Div {
        let read = turn.read.as_ref().map(|read| {
            div()
                .px(look.length(token::SPACE_3))
                .py(look.length(token::SPACE_3))
                .rounded(look.length(token::RADIUS_LG) * 1.25)
                .bg(look.color(token::OPERATOR_MUTED))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(look.length(token::SPACE_2))
                        .mb(look.length(token::SPACE_1))
                        .text_size(look.length(token::TEXT_2XS))
                        .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                        .text_color(look.color(token::OPERATOR))
                        .child(glyph("bot", look.length(token::SPACE_3) + look.length(token::SPACE_1) / 4., look.color(token::OPERATOR)))
                        .child("My read"),
                )
                .child(div().text_size(look.length(token::TEXT_SM)).line_height(look.length(token::LEADING_SM)).child(read.read.clone()))
        });
        let options = face::chooses(turn).then(|| self.options(turn, look, cx));
        let refused = self.refused.as_ref().filter(|(id, _)| *id == turn.id).map(|(_, why)| {
            labelled(
                div()
                    .id("operator-refused")
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_2))
                    .text_size(look.length(token::TEXT_SM))
                    .text_color(look.color(token::STATUS_COMPLETED_FAILED))
                    .child(glyph("circle-alert", look.length(token::SPACE_4), look.color(token::STATUS_COMPLETED_FAILED)))
                    .child(div().flex_1().min_w_0().child(why.clone())),
                Role::Alert,
                why.clone(),
            )
        });
        div()
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_3))
            .px(look.length(token::SPACE_4))
            .py(look.length(token::SPACE_4))
            .children(lead)
            .children(face::asks_of(turn).map(|asks| div().text_size(look.length(token::TEXT_BASE)).font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM))).child(asks)))
            .children(face::note_of(turn).map(|(since, note)| {
                // The head is one line that keeps its colon on the time; the note under it wraps in the card.
                div()
                    .flex()
                    .flex_col()
                    .gap(look.length(token::SPACE_1))
                    .text_size(look.length(token::TEXT_BASE))
                    .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
                    .child(
                        div()
                            .flex()
                            .items_baseline()
                            .child("Your note from")
                            .child(self.stamp(since.as_str(), None, (token::TEXT_BASE, token::FG_DEFAULT, token::WEIGHT_MEDIUM), look, cx))
                            // The colon closes up on the time, past its ring's hairline.
                            .child(div().ml(-look.length(token::BORDER_WIDTH) * 2.).child(":")),
                    )
                    .child(div().w_full().min_w_0().child(note))
            }))
            .children(read)
            .children(options)
            .children(refused)
    }

    /// Ask's chat, in the card it is about: between what the item says and its answers.
    fn chat_in_card(&self, look: &Look) -> Option<Div> {
        self.chat.clone().map(|chat| {
            div()
                .h(look.length(token::SPACE_12) * 6.)
                .flex()
                .flex_col()
                .px(look.length(token::SPACE_4))
                .py(look.length(token::SPACE_3))
                .border_t(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::BORDER_SUBTLE))
                .bg(look.color(token::BG_BASE))
                .child(chat)
        })
    }

    /// An ask's answers or outcomes, Send changes, then Later and Ask at the right. Each press opens its
    /// ask first, so in a Job's card it acts on its own ask, and only the open ask's show their keys.
    /// `last` rounds the row into the card's corner.
    fn acts(&self, turn: &OperatorTurnItem, open: bool, last: bool, look: &Look, cx: &Context<Self>) -> Div {
        let chosen = self.option_of(turn);
        let answers: Vec<AnyElement> = if !turn.then.is_empty() { self.outcomes(turn, look, cx) } else { face::answers_of(turn).into_iter().enumerate().map(|(at, answer)| {
            let yes = answer == OperatorAnswer::Yes;
            // A decision's Yes says the option it carries, else the Operator's pick, as its card would; a
            // go-ahead's says Yes.
            let label: SharedString = match turn.kind {
                OperatorTurnKind::Decision if yes => face::pick_word(turn, chosen.as_deref()),
                _ if yes && face::picks_later(turn) => "Later".into(),
                _ => face::answer_word(answer).into(),
            };
            // The pick's sentence never sits in the button: it leads the tooltip.
            let said = face::answer_tip(turn, answer, self.then_of(turn), chosen.as_deref());
            let said = match face::pick_sentence(turn).filter(|_| yes && chosen.is_none()) {
                Some(sentence) => format!("{sentence}\n{said}"),
                None => said,
            };
            let key: Option<SharedString> = open.then(|| face::answer_key(answer).into());
            let button = press(format!("answer-{at}"), label.clone(), yes.then_some("check"), key, if yes { Weight::Pick } else { Weight::Plain }, look)
                .tooltip(tip(said));
            let button = pressed(button, cx, on_ask(&turn.id, move |this, window, cx| this.answer(answer, window, cx)));
            labelled(button, Role::Button, label).into_any_element()
        }).collect() };
        let changes = face::sends_changes(turn).then(|| {
            labelled(
                pressed(press("turn-changes", "Send changes".into(), None, open.then(|| "C".into()), Weight::Plain, look).tooltip(tip("Open the plan in its Job to suggest changes")), cx, on_ask(&turn.id, |this, _, cx| {
                    this.send_changes(cx)
                })),
                Role::Button,
                "Send changes",
            )
        });
        div()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .px(look.length(token::SPACE_4))
            .py(look.length(token::SPACE_3))
            .border_t(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_SUBTLE))
            .bg(look.color(token::BG_OVERLAY))
            // The card clips its children square: its ground keeps inside the rounded corner itself.
            .when(last, |acts| acts.rounded_b(round(look) - look.length(token::BORDER_WIDTH)))
            // In a narrow window the row wraps inside the card; Later and Ask keep the right of their line.
            .flex_wrap()
            .children(answers)
            .children(changes)
            .child(div().ml_auto().flex().items_center().gap(look.length(token::SPACE_2)).child(labelled(
                {
                    // Its menu opens under it: where it is drawn is kept for that, by its ask.
                    let (drawn, key) = (self.later_bounds.clone(), turn.id.clone());
                    pressed(press("turn-later", "Later".into(), Some("clock"), open.then(|| "L".into()), Weight::Bare, look).relative().when(open, |later| later.track_focus(&self.later_focus)).tooltip(tip("Choose when it comes back")), cx, on_ask(&turn.id, |this, window, cx| {
                        this.open_later(false, window, cx)
                    }))
                    .child(canvas(move |bounds, _, _| { drawn.borrow_mut().insert(key, bounds); }, |_, _, _, _| {}).absolute().size_full())
                },
                Role::Button,
                "Later",
            ))
            .child(labelled(
                pressed(press("turn-ask", "Ask".into(), Some("bot"), open.then(|| "/".into()), Weight::Bare, look).tooltip(tip("Ask the Operator about this")), cx, on_ask(&turn.id, |this, window, cx| {
                    this.ask(window, cx)
                })),
                Role::Button,
                "Ask",
            )))
    }

    /// A go-ahead to file: one press for each outcome, named for what happens, the Operator's pick filled
    /// with its glyph and why in its tooltip, then Don't file. 1..n press an outcome and D Don't file.
    fn outcomes(&self, turn: &OperatorTurnItem, look: &Look, cx: &Context<Self>) -> Vec<AnyElement> {
        // The keys act on the open ask: only its presses show them.
        let keyed = self.open.as_deref() == Some(turn.id.as_str());
        let why = turn.read.as_ref().map(|read| read.read.clone()).filter(|read| !read.is_empty());
        let mut out: Vec<AnyElement> = turn
            .then
            .iter()
            .enumerate()
            .map(|(at, offer)| {
                let to = offer.hand_to.as_ref().map(|to| to.job.as_str());
                let (label, mark) = face::then_said(offer.then, to);
                let does = face::then_does(offer.then, to);
                let said = match why.as_ref().filter(|_| offer.pick) {
                    Some(why) => format!("My pick: {why}\n{does}"),
                    None => does,
                };
                let then = offer.then;
                let (icon, weight) = if offer.pick { ("check", Weight::Pick) } else { (mark, Weight::Plain) };
                let button = press(format!("outcome-{at}"), label.clone(), Some(icon), keyed.then(|| (at + 1).to_string().into()), weight, look).tooltip(tip(said));
                let button = pressed(button, cx, on_ask(&turn.id, move |this, _, cx| this.outcome(then, cx)));
                labelled(button, Role::Button, label).into_any_element()
            })
            .collect();
        let no = press("outcome-no", "Don't file".into(), None, keyed.then(|| face::answer_key(OperatorAnswer::No).into()), Weight::Plain, look).tooltip(tip("Leave it unfiled, and it leaves Your turn"));
        out.push(labelled(pressed(no, cx, on_ask(&turn.id, |this, window, cx| this.answer(OperatorAnswer::No, window, cx))), Role::Button, "Don't file").into_any_element());
        out
    }

    /// The item's options, where it offers more than one: each a row, his pick or else the Operator's
    /// filled, My pick marked, as Then draws its choices. Yes, Always here and Always carry the filled one.
    fn options(&self, turn: &OperatorTurnItem, look: &Look, cx: &Context<Self>) -> impl IntoElement {
        let keyed = self.open.as_deref() == Some(turn.id.as_str());
        let chosen = self.option_of(turn);
        let pick = face::pick_of(turn);
        let rows = turn.item.options.iter().enumerate().map(|(at, option)| {
            let label: SharedString = option.label.clone().into();
            let on = chosen.as_ref() == Some(&label);
            let id = turn.id.clone();
            let pressed_label = option.label.clone();
            labelled(
                div()
                    .id(SharedString::from(format!("option-{at}")))
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_2))
                    .h(look.length(token::H_CONTROL_LG) + look.length(token::SPACE_1))
                    .px(look.length(token::SPACE_3))
                    .rounded(look.length(token::RADIUS_LG) * 1.25)
                    .border(look.length(token::BORDER_WIDTH))
                    .border_color(look.color(if on { token::OPERATOR } else { token::BORDER_DEFAULT }))
                    .bg(look.color(if on { token::OPERATOR_FAINT } else { token::BG_OVERLAY }))
                    .text_size(look.length(token::TEXT_SM))
                    .font_weight(FontWeight(look.number(if on { token::WEIGHT_HEADING } else { token::WEIGHT_BODY })))
                    .cursor_pointer()
                    .hover(|style| style.border_color(cx.look().color(token::BORDER_STRONG)))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.open = Some(id.clone());
                        this.pick_option(&id, &pressed_label, cx)
                    }))
                    .child(glyph(if on { "circle-dot" } else { "circle" }, look.length(token::SPACE_4), look.color(if on { token::OPERATOR } else { token::FG_MUTED })))
                    .child(div().flex_1().child(label.clone()))
                    .when(pick.as_ref() == Some(&label), |row| {
                        row.child(
                            div()
                                .text_size(look.length(token::TEXT_2XS))
                                .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                                .text_color(look.color(token::OPERATOR))
                                .child("My pick"),
                        )
                    })
                    .when(keyed, |row| row.child(kbd((at + 1).to_string(), look))),
                Role::RadioButton,
                label,
            )
        });
        div()
            .mt(look.length(token::SPACE_1))
            .child(div().mb(look.length(token::SPACE_2)).text_size(look.length(token::TEXT_2XS)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).text_color(look.color(token::FG_SUBTLE)).child("Options"))
            .child(labelled(div().id("options").flex().flex_col().gap(look.length(token::SPACE_2)).children(rows), Role::RadioGroup, "Options"))
    }

    fn done_row(&self, done: &Done, look: &Look, cx: &App) -> AnyElement {
        let pending = self.pending.contains(&done.id);
        let mark = if pending {
            spinner(SharedString::from(format!("done-spin-{}", done.id)), look.length(token::SPACE_4), look.color(token::FG_SUBTLE), cx).into_any_element()
        } else {
            glyph("check", look.length(token::SPACE_4), look.color(token::STATUS_COMPLETED_SUCCESS)).into_any_element()
        };
        let sub: SharedString = done.said.clone().unwrap_or_else(|| "Sending".into());
        labelled(
            row_frame(SharedString::from(format!("done-{}", done.id)), look)
                .text_color(look.color(token::FG_SUBTLE))
                .tooltip(tip(if pending { "Sending" } else { "Answered" }))
                .child(div().w(look.length(token::SPACE_3)))
                .child(mark)
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .child(div().truncate().line_through().child(done.title.clone()))
                        .child(div().truncate().text_size(look.length(token::TEXT_XS)).child(sub.clone())),
                )
                .child(kind_chip(done.kind, look).opacity(0.6)),
            Role::Row,
            format!("{}, {sub}", done.title),
        )
        .into_any_element()
    }

    /// Answered today, folded at the foot of Your turn: what he answered on his local day and what came
    /// of each. Drawn only once something was answered.
    fn answered_fold(&self, look: &Look, cx: &Context<Self>) -> Option<AnyElement> {
        let done = self.answered_today();
        if done.is_empty() {
            return None;
        }
        let open = self.folded.get(ANSWERED).copied().unwrap_or(false);
        let head = labelled(
            row_frame(SharedString::from("answered-today"), look)
                .cursor_pointer()
                .hover(|style| style.bg(cx.look().color(token::BG_HOVER)))
                .text_color(look.color(token::FG_MUTED))
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.folded.insert(ANSWERED, !open);
                    cx.notify();
                }))
                .child(glyph(if open { "chevron-down" } else { "chevron-right" }, look.length(token::SPACE_3) + look.length(token::SPACE_1) / 4., look.color(token::FG_SUBTLE)))
                .child(glyph("check", look.length(token::SPACE_4), look.color(token::STATUS_COMPLETED_SUCCESS)))
                .child(div().flex_1().text_size(look.length(token::TEXT_SM)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).child(ANSWERED)),
            Role::Button,
            ANSWERED,
        )
        .into_any_element();
        let mut rows = vec![head];
        if open {
            rows.extend(done.into_iter().rev().map(|done| self.done_row(done, look, cx)));
        }
        self.rows_group("answered-today-list", ANSWERED, rows, look)
    }

    /// An entry of Your turn not open: one item, or a Job's asks as one row under its title, naming each.
    fn turn_row(&self, at: usize, run: &[OperatorTurnItem], look: &Look, cx: &Context<Self>) -> AnyElement {
        let turn = &run[0];
        let kind = face::kind_of(turn);
        let title = face::title_of(turn);
        let id = turn.id.clone();
        let drop_id = turn.id.clone();
        let sub = if run.len() > 1 {
            Some(run.iter().map(|ask| face::asks_of(ask).unwrap_or_else(|| face::title_of(ask)).to_string()).collect::<Vec<_>>().join(" · "))
        } else {
            face::pick_of(turn).map(|pick| format!("My pick: {pick}"))
        };
        // Put Later: the clock beside its kind, saying when it comes back.
        let later = run.iter().find(|ask| ask.later_until.is_some()).map(|turn| {
            let said = face::later_said(turn, self.clock_now.unwrap_or_else(chrono::Local::now));
            labelled(
                div().id(SharedString::from(format!("later-{at}"))).tooltip(tip(said.clone())).child(glyph("clock", look.length(token::SPACE_4), look.color(token::STATUS_NOT_STARTED))),
                Role::Image,
                said,
            )
        });
        labelled(
            row_frame(SharedString::from(format!("turn-row-{at}")), look)
                .cursor_pointer()
                .hover(|style| style.bg(cx.look().color(token::BG_HOVER)))
                .on_click(cx.listener(move |this, _, _, cx| this.open_id(&id, cx)))
                .on_drag(TurnDrag { id: turn.id.clone(), title: title.clone() }, |drag, _, _, cx| cx.new(|_| drag.clone()))
                .drag_over::<TurnDrag>(|style, _, _, cx| style.bg(cx.look().color(token::OPERATOR_FAINT)))
                .on_drop(cx.listener(move |this, drag: &TurnDrag, _, cx| {
                    if let Some(to) = this.items().iter().position(|one| one.id == drop_id) {
                        this.move_to(&drag.id, to, cx);
                    }
                }))
                .child(div().id(SharedString::from(format!("grip-{at}"))).cursor_grab().tooltip(tip("Drag to reorder")).child(glyph("grip-vertical", look.length(token::SPACE_3), look.color(token::FG_SUBTLE))))
                .child(div().id(SharedString::from(format!("mark-{at}"))).tooltip(tip(kind.word)).child(glyph(kind.glyph, look.length(token::SPACE_4), look.color(kind.hue))))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .child(div().truncate().child(title.clone()))
                        .children(sub.map(|sub| div().truncate().text_size(look.length(token::TEXT_XS)).text_color(look.color(token::FG_MUTED)).child(sub))),
                )
                .children(later)
                .child(kind_chip(kind, look)),
            Role::Row,
            title,
        )
        .into_any_element()
    }

    /// Your turn as a queue: the open item's card stands in its row's place, so a row pressed low opens
    /// where it is and nothing reorders (his ruling, 12 Nov). The rows above it and below it are a list each.
    /// Also says where the card is among them.
    fn queue(&self, look: &Look, cx: &Context<Self>) -> (Vec<AnyElement>, Option<usize>) {
        let open = self.open.as_deref();
        let mut before: Vec<AnyElement> = Vec::new();
        let mut after = Vec::new();
        let mut card = None;
        for (at, run) in self.entries() {
            if run.iter().any(|turn| Some(turn.id.as_str()) == open) {
                card = Some(self.card(run, look, cx));
            } else if card.is_none() {
                before.push(self.turn_row(at, run, look, cx));
            } else {
                after.push(self.turn_row(at, run, look, cx));
            }
        }
        let mut out: Vec<AnyElement> = self.rows_group("turn-before", "The rest of Your turn", before, look).into_iter().collect();
        let at = card.as_ref().map(|_| out.len());
        out.extend(card);
        out.extend(self.rows_group("turn-after", "The rest of Your turn", after, look));
        out.extend(self.answered_fold(look, cx));
        (out, at)
    }

    /// Your turn's entries, each with where it starts: an item, or a Job's asks together.
    fn entries(&self) -> Vec<(usize, &[OperatorTurnItem])> {
        let items = self.items();
        let groups: Vec<Option<&str>> = items.iter().map(|turn| turn.group.as_deref()).collect();
        let mut entries = Vec::new();
        let mut at = 0;
        while at < items.len() {
            let run = super::page::run_at(&groups, at);
            entries.push((at, &items[run.clone()]));
            at = run.end;
        }
        entries
    }

    /// A run of Your turn's rows in one frame, a rule between rows and never under the last.
    fn rows_group(&self, id: &'static str, name: &'static str, rows: Vec<AnyElement>, look: &Look) -> Option<AnyElement> {
        if rows.is_empty() {
            return None;
        }
        Some(
            labelled(
                div()
                    .id(id)
                    .flex_none()
                    .flex()
                    .flex_col()
                    .rounded(round(look))
                    .border(look.length(token::BORDER_WIDTH))
                    .border_color(look.color(token::BORDER_DEFAULT))
                    .overflow_hidden()
                    // A rule between rows, never under the last: a border there runs square past the corner.
                    .children(rows.into_iter().enumerate().flat_map(|(at, row)| {
                        let rule = (at > 0).then(|| div().flex_none().h(look.length(token::BORDER_WIDTH)).bg(look.color(token::BORDER_SUBTLE)).into_any_element());
                        rule.into_iter().chain(std::iter::once(row))
                    })),
                Role::List,
                name,
            )
            .into_any_element(),
        )
    }

    fn stamp(&self, at: &str, lead: Option<&'static str>, (size, ink, weight): (Token, Token, Token), look: &Look, cx: &Context<Self>) -> AnyElement {
        let now = self.clock_now.unwrap_or_else(chrono::Local::now);
        let (on_clock, ago) = (minute(at, now).map(|clock| lead.map_or(clock.clone(), |lead| format!("{lead} {clock}"))), crate::job_view::age(at, now));
        let (shown, other) = if self.aged { (ago, on_clock) } else { (on_clock, ago) };
        let Some(shown) = shown else { return gpui::Empty.into_any_element() };
        let reading = shown.trim_start_matches(lead.unwrap_or_default()).trim_start().to_string();
        let stamp = worded(div().id(SharedString::from(format!("stamp:{at}"))), look)
            .flex_none()
            // A hairline inside the ring, so the gap before it and the ring together are one word space.
            .px(look.length(token::BORDER_WIDTH))
            // Its sentence's face said again: without it the time has drawn in the system's face.
            .font_family(sans(look))
            .font_weight(FontWeight(look.number(weight)))
            .text_size(look.length(size))
            .text_color(look.color(ink))
            .hover(|style| style.text_color(cx.look().color(token::FG_DEFAULT)))
            .tooltip(tip(other.unwrap_or_default()))
            .child(shown);
        let pressable = labelled(
            pressed(stamp, cx, |this, _, cx| {
                cx.stop_propagation();
                this.flip_stamps(cx);
            }),
            Role::Button,
            format!("Time {reading}"),
        );
        // The ring is drawn inside this: the gap before the time is the wrapper's, so it never reaches the word.
        div().flex_none().pl(look.length(token::SPACE_1) / 2.).child(pressable).into_any_element()
    }

    /// A link under a tile's rows: a tab stop Enter presses.
    fn link(&self, said: &'static str, look: &Look, cx: &Context<Self>) -> AnyElement {
        let keys = self.stops.get(said).cloned();
        let link = worded(div().id(SharedString::from(format!("link-{said}"))).when_some(keys, |link, keys| link.track_focus(&keys)), look)
            // As wide as its words, so its ring sits on them and not across the tile; its words line up with the rows'.
            .self_start()
            .ml(-look.length(token::SPACE_1))
            .mt(look.length(token::SPACE_1))
            .text_size(look.length(token::TEXT_XS))
            .text_color(look.color(token::ACCENT))
            .hover(|style| style.text_color(cx.look().color(token::ACCENT_HOVER)))
            .child(said);
        labelled(pressed(link, cx, move |this, window, cx| this.press_stop(said, window, cx)), Role::Link, said).into_any_element()
    }

    /// Forget on a rule's or a hold's row: a press, or Enter, Space or `f` while it has the keys.
    fn forget_button(&self, id: i64, about: &str, look: &Look, cx: &Context<Self>) -> AnyElement {
        // The page's ghost button, at a row's size.
        let button = press(SharedString::from(format!("forget-{id}")), "Forget".into(), None, None, Weight::Bare, look)
            .h(look.length(token::H_CONTROL_SM))
            .px(look.length(token::SPACE_2))
            .text_size(look.length(token::TEXT_XS))
            .on_key_down(cx.listener(move |this, event: &gpui::KeyDownEvent, window, cx| {
                if event.keystroke.key == "f" && !event.keystroke.modifiers.modified() {
                    cx.stop_propagation();
                    this.forget_pressed(id, window, cx);
                }
            }));
        // The press stops here, so a row of Every rule under it is not selected as well.
        let forget = move |this: &mut OperatorPageView, window: &mut Window, cx: &mut Context<OperatorPageView>| {
            cx.stop_propagation();
            this.forget_pressed(id, window, cx);
        };
        labelled(pressed(button, cx, forget), Role::Button, format!("Forget {about}")).into_any_element()
    }

    /// Under a hold's words: each held path in mono, then what lifts it.
    fn hold_line(&self, hold: HoldFacts, look: &Look) -> AnyElement {
        div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap(look.length(token::SPACE_1))
            .text_size(look.length(token::TEXT_2XS))
            .text_color(look.color(token::FG_MUTED))
            .children(hold.paths.into_iter().map(|path| {
                div()
                    .flex_none()
                    .max_w_full()
                    .truncate()
                    .px(look.length(token::SPACE_1))
                    .rounded(look.length(token::RADIUS_SM))
                    .bg(look.color(token::BG_RAISED))
                    .border(look.length(token::BORDER_WIDTH))
                    .border_color(look.color(token::BORDER_SUBTLE))
                    .font_family(look.font_stack(token::FONT_MONO).into_iter().next().unwrap_or_default())
                    .text_color(look.color(token::FG_DEFAULT))
                    .child(path)
            }))
            .children(hold.until.map(|until| div().flex_none().child(until)))
            .into_any_element()
    }

    fn tile(&self, name: &'static str, mark: &'static str, hue: Token, aside: Option<String>, rows: Vec<Row>, link: Option<&'static str>, look: &Look, cx: &Context<Self>) -> AnyElement {
        let open = self.folded.get(name).copied().unwrap_or(opens_unpressed(name));
        let ring = look.edge(token::FOCUS_RING).map_or(look.color(token::ACCENT), |(_, colour)| colour);
        let body = open.then(|| {
            // The first few, one line each: the rest are a press away. Learned's are all on Every rule.
            let more = rows.len() > FEW && link != Some("Every rule");
            let whole = self.whole.contains(name);
            let shown = if link == Some("Every rule") { rows.len().min(FEW) } else if whole || !more { rows.len() } else { FEW };
            // A rule's or a hold's words are what matter in it, so it takes a second line before it is cut.
            let wraps = matches!(name, "Learned" | "Holds");
            let mut last: Option<String> = None;
            let rows = rows.into_iter().take(shown).enumerate().flat_map(|(at, row)| {
                let subhead = row.place.clone().filter(|place| last.as_ref() != Some(place)).map(|place| {
                    last = Some(place.clone());
                    div()
                        .pt(look.length(if at == 0 { token::SPACE_1 } else { token::SPACE_2 }))
                        .text_size(look.length(token::TEXT_2XS))
                        .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                        .text_color(look.color(token::FG_SUBTLE))
                        .child(place)
                        .into_any_element()
                });
                let (text, said) = (row.text, row.said);
                let line = div()
                    .id(SharedString::from(format!("{name}-{at}")))
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_2))
                    .py(look.length(token::SPACE_1))
                    .text_size(look.length(token::TEXT_XS))
                    .tooltip(tip(said.clone()))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .gap(look.length(token::SPACE_1))
                            .child(div().min_w_0().when(wraps, |words| words.line_clamp(2).text_ellipsis()).when(!wraps, |words| words.truncate()).child(text.clone()))
                            .children(row.hold.map(|hold| self.hold_line(hold, look))),
                    )
                    .child(match row.side {
                        Side::None => gpui::Empty.into_any_element(),
                        Side::Text(said) => div().flex_none().text_color(look.color(token::FG_SUBTLE)).child(said).into_any_element(),
                        Side::At(at) => self.stamp(at.as_str(), None, (token::TEXT_XS, token::FG_SUBTLE, token::WEIGHT_BODY), look, cx),
                    })
                    .children(row.forget.map(|id| self.forget_button(id, &said, look, cx)));
                // A row that opens a Job is a tab stop, and Enter opens it as a press does.
                let line = match row.opened {
                    Some(opened) => pressed(worded(line, look).mx(-look.length(token::SPACE_1)), cx, move |_, _, cx| cx.emit(opened.clone())),
                    // The same inset as a row that opens a Job, so a ring on it stands clear of the words.
                    None => line.px(look.length(token::SPACE_1)).mx(-look.length(token::SPACE_1)),
                };
                subhead.into_iter().chain(std::iter::once(labelled(line, Role::ListItem, said).into_any_element()))
            });
            let link = link.map(|said| self.link(said, look, cx));
            let toggle = more.then(|| {
                let said = if whole { "Show fewer" } else { "Show all" };
                let toggle = worded(div().id(SharedString::from(format!("{name}-whole"))), look)
                    .self_start()
                    .ml(-look.length(token::SPACE_1))
                    .mt(look.length(token::SPACE_1))
                    .text_size(look.length(token::TEXT_XS))
                    .text_color(look.color(token::ACCENT))
                    .hover(|style| style.text_color(cx.look().color(token::ACCENT_HOVER)))
                    .child(said);
                labelled(
                    pressed(toggle, cx, move |this, _, cx| {
                        if !this.whole.remove(name) {
                            this.whole.insert(name);
                        }
                        cx.notify();
                    }),
                    Role::Button,
                    format!("{said} {name}"),
                )
            });
            div()
                .flex()
                .flex_col()
                .px(look.length(token::SPACE_3))
                .pt(look.length(token::SPACE_1))
                .pb(look.length(token::SPACE_2))
                .border_t(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::BORDER_SUBTLE))
                .children(rows)
                .children(toggle)
                .children(link)
        });
        let no_aside = aside.is_none();
        let keys = self.stops.get(name).cloned();
        let head = div()
            .id(SharedString::from(format!("tile-{name}")))
            .when_some(keys, |head, keys| head.track_focus(&keys))
            .focusable()
            .tab_stop(true)
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .px(look.length(token::SPACE_3))
            .h(look.length(token::H_CONTROL_LG) + look.length(token::SPACE_1))
            .rounded(look.length(token::RADIUS_LG) * 1.25)
            .border(look.length(token::BORDER_WIDTH))
            .border_color(gpui::transparent_black())
            .focus_visible(move |style| style.border_color(ring))
            .cursor_pointer()
            .child(glyph(mark, look.length(token::SPACE_4), look.color(hue)))
            .child(div().font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).text_size(look.length(token::TEXT_SM)).child(name))
            .children(aside.map(|aside| div().flex_1().min_w_0().truncate().text_size(look.length(token::TEXT_XS)).text_color(look.color(token::FG_MUTED)).child(aside)))
            .when(no_aside, |head| head.child(div().flex_1()))
            .child(glyph(if open { "chevron-down" } else { "chevron-right" }, look.length(token::SPACE_3) + look.length(token::SPACE_1) / 4., look.color(token::FG_SUBTLE)));
        let head = labelled(pressed(head, cx, move |this, window, cx| this.press_stop(name, window, cx)), Role::Button, name);
        div()
            .flex_none()
            .rounded(look.length(token::RADIUS_LG) * 1.25)
            .border(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_DEFAULT))
            .bg(look.color(token::BG_RAISED))
            .child(head)
            .children(body)
            .into_any_element()
    }

    fn digest(&self, page: &OperatorPage, look: &Look, cx: &Context<Self>) -> impl IntoElement {
        let digest = &page.digest;
        let mut tiles: Vec<AnyElement> = Vec::new();
        if !digest.landed.is_empty() {
            let rows = digest.landed.iter().map(|one| Row { text: format!("#{} {}", one.number, one.title), said: format!("#{} {}", one.number, one.title), place: None, side: Side::At(one.at.clone()), opened: Some(Opened::Job(one.session_id.clone())), forget: None, hold: None }).collect();
            tiles.push(self.tile("Landed", "git-merge", token::STATUS_COMPLETED_SUCCESS, None, rows, None, look, cx));
        }
        if !digest.handled.acts.is_empty() || digest.handled.log.is_some() {
            let rows = digest.handled.acts.iter().map(|act| Row { text: handled_said(act), said: handled_said(act), place: None, side: Side::At(act.at.clone()), opened: None, forget: None, hold: None }).collect();
            // Off, nothing new is handled: what is here it did before it was switched off.
            let aside = (!page.on).then(|| "while it was on".to_string());
            let link = digest.handled.log.as_ref().map(|_| "The whole Log");
            tiles.push(self.tile("What I handled", "bot", token::OPERATOR, aside, rows, link, look, cx));
        }
        if !digest.running.is_empty() {
            let rows = by_place(digest.running.iter().map(|one| Row { text: one.title.clone(), said: one.title.clone(), place: one.repository.clone(), side: Side::None, opened: Some(Opened::Job(one.session_id.clone())), forget: None, hold: None }).collect());
            tiles.push(self.tile("Running", "activity", token::STATUS_RUNNING, None, rows, None, look, cx));
        }
        if !digest.coming_up.is_empty() {
            let rows = digest.coming_up.iter().map(|one| Row { text: format!("{}: waits on {}", one.title, one.needs), said: format!("{}: waits on {}", one.title, one.needs), place: None, side: Side::None, opened: Some(Opened::Job(one.session_id.clone())), forget: None, hold: None }).collect();
            tiles.push(self.tile("Coming up", "clock", token::STATUS_NOT_STARTED, None, rows, None, look, cx));
        }
        if !digest.learned.is_empty() {
            let mut places: Vec<String> = Vec::new();
            for rule in &digest.learned {
                let place = rule.repository.clone().unwrap_or_else(|| "everywhere".into());
                if !places.contains(&place) {
                    places.push(place);
                }
            }
            let rows = by_place(
                digest.learned.iter().map(|rule| Row { text: rule.said.clone(), said: rule_said(rule), place: Some(rule.repository.clone().unwrap_or_else(|| "Everywhere".into())), side: Side::At(rule.at.clone()), opened: None, forget: Some(rule.id), hold: None }).collect(),
            );
            tiles.push(self.tile("Learned", "scroll-text", token::OPERATOR, Some(places.join(", ")), rows, Some("Every rule"), look, cx));
        }
        if !digest.holds.is_empty() {
            let rows = by_place(
                digest.holds
                    .iter()
                    .map(|hold| Row {
                        text: hold.said.clone(),
                        said: hold_said(hold),
                        place: Some(hold.repository.clone().unwrap_or_else(|| hold.manifest_id.clone())),
                        side: Side::At(hold.at.clone()),
                        // The one Job that writes under the paths opens on a press, as a running Job's row does.
                        opened: hold.job.clone().map(Opened::Job),
                        forget: Some(hold.id),
                        hold: hold_facts(hold),
                    })
                    .collect(),
            );
            tiles.push(self.tile("Holds", "folder-lock", token::OPERATOR, None, rows, None, look, cx));
        }
        let since = div()
            .flex()
            .items_center()
            .text_size(look.length(token::TEXT_2XS))
            .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
            .text_color(look.color(token::FG_SUBTLE))
            .child("Since you last looked")
            .children(page.looked_at.as_ref().map(|at| self.stamp(at.as_str(), Some("at"), (token::TEXT_2XS, token::FG_SUBTLE, token::WEIGHT_HEADING), look, cx)));
        labelled(
            div()
                .id("digest")
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_3))
                .child(since)
                .children(tiles),
            Role::Region,
            "Since you last looked",
        )
    }

    // -- skeletons ---------------------------------------------------------------------------------

    fn skeleton(&self, look: &Look) -> impl IntoElement {
        let line = look.length(token::LEADING_SM);
        div()
            .flex_1()
            .min_h_0()
            .flex()
            .child(
                div()
                    .flex_1()
                    .flex()
                    .flex_col()
                    .gap(look.length(token::SPACE_4))
                    .px(look.length(token::SPACE_6))
                    .py(look.length(token::SPACE_6))
                    .child(bone(0.2, line, look))
                    .child(div().h(look.length(token::SPACE_12) * 4.).rounded(round(look)).bg(look.color(token::BG_HOVER)))
                    .children((0..3).map(|_| bone(1., look.length(token::H_CONTROL_LG) + look.length(token::SPACE_2), look))),
            )
            .child(side(look).child(div().flex().flex_col().gap(look.length(token::SPACE_3)).child(bone(0.6, line, look)).children((0..4).map(|_| bone(1., look.length(token::H_CONTROL_LG) + look.length(token::SPACE_1), look)))))
    }

    // -- the pane from a Job -----------------------------------------------------------------------

    fn render_pane(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let look = cx.look();
        let rows: Vec<AnyElement> = self
            .items()
            .iter()
            .enumerate()
            .map(|(at, turn)| {
                let kind = face::kind_of(turn);
                let title = face::title_of(turn);
                let id = turn.id.clone();
                labelled(
                    div()
                        .id(SharedString::from(format!("pane-row-{at}")))
                        .flex()
                        .items_center()
                        .gap(look.length(token::SPACE_2))
                        .h(look.length(token::H_ROW) + look.length(token::SPACE_1))
                        .px(look.length(token::SPACE_2))
                        .rounded(look.length(token::RADIUS_LG))
                        .cursor_pointer()
                        .hover(|style| style.bg(cx.look().color(token::BG_HOVER)))
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.open_id(&id, cx);
                            this.ask(window, cx);
                        }))
                        .child(div().id(SharedString::from(format!("pane-mark-{at}"))).tooltip(tip(kind.word)).child(glyph(kind.glyph, look.length(token::SPACE_4), look.color(kind.hue))))
                        .child(div().flex_1().min_w_0().truncate().text_size(look.length(token::TEXT_SM)).child(title.clone())),
                    Role::Row,
                    title,
                )
                .into_any_element()
            })
            .collect();
        let list = if self.page.is_none() {
            div().flex().flex_col().gap(look.length(token::SPACE_2)).children((0..3).map(|_| bone(1., look.length(token::H_ROW), look))).into_any_element()
        } else {
            labelled(div().id("pane-turn").flex().flex_col().children(rows), Role::List, "Your turn").into_any_element()
        };
        div()
            .id("operator-pane")
            .key_context("call")
            .track_focus(&self.focus)
            .on_action(cx.listener(Self::on_act))
            .size_full()
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_3))
            .p(look.length(token::SPACE_3))
            .font_family(sans(look))
            .text_color(look.color(token::FG_DEFAULT))
            .child(list)
            .child(div().flex_1().min_h_0().children(self.chat.clone()))
            .into_any_element()
    }
}

fn row_frame(id: SharedString, look: &Look) -> Stateful<Div> {
    div()
        .id(id)
        .flex()
        .items_center()
        .gap(look.length(token::SPACE_3))
        .px(look.length(token::SPACE_4))
        .py(look.length(token::SPACE_2))
        .min_h(look.length(token::H_CONTROL_LG) + look.length(token::SPACE_3))
        .text_size(look.length(token::TEXT_SM))
}

fn side(look: &Look) -> Div {
    div()
        .flex_none()
        .w(look.length(token::SPACE_12) * 7.5)
        .h_full()
        .p(look.length(token::SPACE_4))
        .border_l(look.length(token::BORDER_WIDTH))
        .border_color(look.color(token::BORDER_DEFAULT))
        .bg(look.color(token::BG_SUNKEN))
}

/// A tile's rows past these are a press away, or in the Log.
const FEW: usize = 4;

/// The fold at the foot of Your turn, and its key in `folded`.
const ANSWERED: &str = "Answered today";

/// Tiles the chosen side column draws open: what it handled, what it learned, and what it holds.
pub(super) fn opens_unpressed(name: &str) -> bool {
    matches!(name, "What I handled" | "Learned" | "Holds")
}

/// One row of a tile: its words, what stands at its right, and where a press on it goes.
pub(super) struct Row {
    /// The words drawn. Under a project's subhead they leave the project out; `said` keeps it for a reader.
    text: String,
    said: String,
    /// The project a row sits under, in the tiles grouped by project.
    place: Option<String>,
    side: Side,
    opened: Option<Opened>,
    /// A rule's or a hold's id, which its Forget names.
    forget: Option<i64>,
    /// What a hold stands over and what lifts it, under its words; none on an answer from before they were sent.
    hold: Option<HoldFacts>,
}

pub(super) struct HoldFacts {
    paths: Vec<String>,
    until: Option<&'static str>,
}

/// A hold's paths and what lifts it, as Fleet sends them (`OperatorHold.paths`, `.until`); nothing when neither came.
pub(super) fn hold_facts(hold: &ipc::operator_page::OperatorHold) -> Option<HoldFacts> {
    let until = hold.until.map(|until| match until {
        ipc::operator_talk::OperatorHoldUntil::Pushes => "until it pushes",
        ipc::operator_talk::OperatorHoldUntil::Ends => "until it ends",
    });
    (!hold.paths.is_empty() || until.is_some()).then(|| HoldFacts { paths: hold.paths.clone(), until })
}

pub(super) enum Side {
    None,
    Text(String),
    At(ipc::Instant),
}

/// A handled act as the Log writes it, `Kind: what`. Where Fleet's outcome is the kind alone (`Started`,
/// `Passed on`), the what is the act's title, which Fleet sends beside it.
pub(super) fn handled_said(act: &ipc::operator_page::OperatorHandledAct) -> String {
    match (act.outcome.contains(':'), act.title.trim()) {
        (false, title) if !title.is_empty() => format!("{}: {title}", act.outcome),
        _ => act.outcome.clone(),
    }
}

/// Rows under their project: Everywhere first, then each project by name, as Every rule has them.
fn by_place(rows: Vec<Row>) -> Vec<Row> {
    let mut places: Vec<Option<String>> = Vec::new();
    for row in &rows {
        if !places.contains(&row.place) {
            places.push(row.place.clone());
        }
    }
    places.sort_by_key(|place| (place.as_deref() != Some("Everywhere"), place.clone()));
    let mut rows = rows;
    rows.sort_by_key(|row| places.iter().position(|place| *place == row.place));
    rows
}

/// A Log day head as the page's head writes a day: `Tuesday 2026-11-10` reads `Tuesday 10 November`.
fn log_day(head: &str) -> String {
    head.split_whitespace()
        .last()
        .and_then(|date| chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d").ok())
        .map_or_else(|| head.to_string(), |date| date.format("%A %-d %B").to_string())
}

/// A time as the page reads it, in his clock: `12:47:30`, with the day when it is not today (`Nov 10 16:00:00`).
pub(super) fn minute(at: &str, now: chrono::DateTime<chrono::Local>) -> Option<String> {
    let then = chrono::DateTime::parse_from_rfc3339(at).ok()?.with_timezone(&chrono::Local);
    Some(crate::settings::time::stamp_of(then, crate::settings::time::held(), now))
}

/// A rule as the sketch writes it: where it holds, then the rule, its first letter small after the colon.
pub(super) fn rule_said(rule: &ipc::operator_page::OperatorLearned) -> String {
    placed(rule.repository.as_deref().unwrap_or("everywhere"), &rule.said)
}

/// A hold as Learned writes a rule: the repository it holds in, then the hold.
pub(super) fn hold_said(hold: &ipc::operator_page::OperatorHold) -> String {
    placed(hold.repository.as_deref().unwrap_or(&hold.manifest_id), &hold.said)
}

fn placed(place: &str, said: &str) -> String {
    let mut said = said.chars();
    let small: String = said.next().map(|first| first.to_lowercase().chain(said).collect()).unwrap_or_default();
    format!("{place}: {small}")
}

impl Render for OperatorPageView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.pane {
            return self.render_pane(cx);
        }
        let look = cx.look();
        let root = div()
            .id("operator-page")
            .key_context("call")
            .track_focus(&self.focus)
            .on_action(cx.listener(Self::on_act))
            .on_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, window, cx| {
                // From the page itself, Tab starts at its first pill, not wherever the window's order wraps to.
                let key = &event.keystroke;
                if key.key == "tab" && !key.modifiers.modified() && this.focus.is_focused(window) {
                    window.focus(&this.first_stop, cx);
                    cx.stop_propagation();
                } else if crate::keymap::traverse(event, window, cx) {
                    cx.stop_propagation();
                }
            }))
            .size_full()
            .flex()
            .flex_col()
            .bg(look.color(token::BG_BASE))
            .text_color(look.color(token::FG_DEFAULT))
            .font_family(sans(look))
            .text_size(look.length(token::TEXT_SM));
        let top = div().flex_none().border_b(look.length(token::BORDER_WIDTH)).border_color(look.color(token::BORDER_SUBTLE)).child(self.head(look, cx));
        let Some(page) = self.page.as_ref() else {
            return root.child(top).child(self.skeleton(look)).child(self.composer.clone()).into_any_element();
        };
        let off = (!page.on).then(|| self.off(look, cx));
        let mut parts: Vec<AnyElement> = vec![self.turn_head(look).into_any_element()];
        parts.extend(self.on_it(page, look, cx));
        parts.extend(off);
        let (queue, card_at) = self.queue(look, cx);
        let card_at = card_at.map(|at| at + parts.len());
        parts.extend(queue);
        // The ideas the Operator kept stand under Your turn; with none kept they draw nothing.
        if self.parked.read(cx).draws() {
            parts.push(self.parked.clone().into_any_element());
        }
        // A key or an answer that moves the card scrolls it into view; a press is already on it. In a Job's
        // card, the open ask's top comes to the top of the view, as the card's own may be far above it.
        let asks = std::mem::take(&mut *self.ask_bounds.borrow_mut());
        if std::mem::take(&mut self.reveal) {
            let open = self.open.as_deref();
            let grouped = self.entries().iter().any(|(_, run)| run.len() > 1 && run.iter().any(|turn| Some(turn.id.as_str()) == open));
            match open.filter(|_| grouped).and_then(|open| asks.get(open)) {
                Some(ask) => {
                    // Where the ask sits in the scrolled content: a wheel turned since it was laid out moves it on screen, not there.
                    let (view, offset) = (self.main_scroll.bounds(), self.main_scroll.offset());
                    let to = view.top() + look.length(token::SPACE_4) - ask.top();
                    self.main_scroll.set_offset(gpui::point(offset.x, to.max(-self.main_scroll.max_offset().y).min(gpui::px(0.))));
                }
                None => {
                    if let Some(at) = card_at {
                        self.main_scroll.scroll_to_item(at);
                    }
                }
            }
        }
        let main = div()
            .id("operator-main")
            .track_scroll(&self.main_scroll)
            .flex_1()
            .min_w_0()
            .h_full()
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .px(look.length(token::SPACE_6))
            .pt(look.length(token::SPACE_4) + look.length(token::SPACE_1))
            // Room under the last row, so it never sits on the window's edge.
            .pb(look.length(token::SPACE_12))
            .gap(look.length(token::SPACE_3))
            // Each part a child of the scroll, held at its own height, so a long queue scrolls instead of
            // squeezing the bars above it, and the card can be scrolled to.
            .children(parts.into_iter().map(|part| div().flex_none().child(part)));
        let aside = self.digest(page, look, cx);
        let sheet = self.rules.map(|at| self.rules_sheet(at, look, cx));
        let log = self.log.as_deref().map(|text| self.log_sheet(text, look, cx));
        root.relative()
            .child(top.children(self.top_line(page, look)))
            .child(div().flex_1().min_h_0().flex().child(main).child(side(look).id("operator-side").overflow_y_scroll().child(aside)))
            .child(self.composer.clone())
            .children(sheet)
            .children(log)
            .children(self.later_menu.as_ref().map(|(menu, _)| menu.clone()))
            .into_any_element()
    }
}

impl OperatorPageView {
    /// Where the page says the Operator is off: what that means, and the switch Electron's Permissions tab has.
    fn off(&self, look: &Look, cx: &Context<Self>) -> AnyElement {
        let switch = if self.switching {
            div()
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .text_color(look.color(token::FG_MUTED))
                .child(spinner("switch-spin", look.length(token::SPACE_4), look.color(token::OPERATOR), cx))
                .child("Switching on")
                .into_any_element()
        } else {
            labelled(
                // The press goes as the page reads on: the keys go to the page, where Tab starts at the pills.
                pressed(press("operator-switch", "Switch on".into(), Some("power"), None, Weight::Pick, look).tooltip(tip("Queue what needs you here")), cx, |this, window, cx| {
                    this.switch_on(cx);
                    window.focus(&this.focus, cx);
                }),
                Role::Button,
                "Switch the Operator on",
            )
            .into_any_element()
        };
        let refused = self.switch_refused.clone().map(|why| {
            labelled(
                div().id("switch-refused").text_size(look.length(token::TEXT_SM)).text_color(look.color(token::STATUS_COMPLETED_FAILED)).child(format!("Not switched on: {why}")),
                Role::Alert,
                format!("Not switched on: {why}"),
            )
        });
        labelled(
            div()
                .id("operator-off")
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_2))
                .p(look.length(token::SPACE_4))
                .rounded(round(look))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::OPERATOR_EDGE))
                .bg(look.color(token::OPERATOR_FAINT))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(look.length(token::SPACE_3))
                        .child(glyph("bot", look.length(token::SPACE_4), look.color(token::FG_MUTED)))
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .child(div().font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).child("The Operator is off"))
                                .child(div().text_size(look.length(token::TEXT_XS)).text_color(look.color(token::FG_MUTED)).child("Every Job asks you directly.")),
                        )
                        .child(switch),
                )
                .children(refused),
            Role::Region,
            "The Operator is off",
        )
        .into_any_element()
    }

    /// Every rule, on a sheet over the page: everywhere first, then each project. The arrows move
    /// through them and Esc shuts it.
    fn rules_sheet(&self, at: usize, look: &Look, cx: &Context<Self>) -> AnyElement {
        let rules = self.rules_in_order();
        let mut body: Vec<AnyElement> = Vec::new();
        let mut place: Option<Option<&str>> = None;
        for (n, rule) in rules.iter().enumerate() {
            let here = rule.repository.as_deref();
            if place != Some(here) {
                place = Some(here);
                let said = here.map_or("Everywhere".to_string(), |repository| repository.to_string());
                body.push(
                    labelled(
                        div()
                            .id(SharedString::from(format!("rules-scope-{n}")))
                            .pt(look.length(if n == 0 { token::SPACE_1 } else { token::SPACE_4 }))
                            .pb(look.length(token::SPACE_1))
                            .text_size(look.length(token::TEXT_2XS))
                            .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                            .text_color(look.color(token::FG_SUBTLE))
                            .child(said.clone()),
                        Role::Heading,
                        said,
                    )
                    .into_any_element(),
                );
            }
            let on = n == at;
            body.push(
                labelled(
                    div()
                        .id(SharedString::from(format!("rule-{}", rule.id)))
                        .flex()
                        .items_center()
                        .gap(look.length(token::SPACE_3))
                        .h(look.length(token::H_CONTROL_LG))
                        .px(look.length(token::SPACE_3))
                        .rounded(look.length(token::RADIUS_LG))
                        .border(look.length(token::BORDER_WIDTH))
                        .border_color(if on { look.color(token::OPERATOR_EDGE) } else { gpui::transparent_black() })
                        .when(on, |row| row.bg(look.color(token::OPERATOR_FAINT)))
                        .cursor_pointer()
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.rules = Some(n);
                            cx.notify();
                        }))
                        .child(div().flex_1().min_w_0().truncate().child(rule.said.clone()))
                        .child(self.stamp(rule.at.as_str(), None, (token::TEXT_XS, token::FG_SUBTLE, token::WEIGHT_BODY), look, cx))
                        .child(self.forget_button(rule.id, &rule.said, look, cx)),
                    Role::ListItem,
                    rule.said.clone(),
                )
                .into_any_element(),
            );
        }
        self.sheet("rules", "Every rule", "scroll-text", &self.rules_focus, &self.rules_scroll, body, Self::close_rules, look, cx)
    }

    /// The whole Log: each day under its heading, each entry at its time, as Fleet's journal writes them.
    fn log_sheet(&self, text: &str, look: &Look, cx: &Context<Self>) -> AnyElement {
        let mut body: Vec<AnyElement> = Vec::new();
        if text.is_empty() {
            body.extend((0..6).map(|n| div().py(look.length(token::SPACE_1)).child(bone(if n % 2 == 0 { 0.8 } else { 0.6 }, look.length(token::LEADING_SM), look)).into_any_element()));
        }
        for (n, line) in text.lines().enumerate() {
            if let Some(day) = line.strip_prefix("## ") {
                let day = log_day(day);
                body.push(
                    labelled(
                        div()
                            .id(SharedString::from(format!("log-day-{n}")))
                            .pt(look.length(if body.is_empty() { token::SPACE_1 } else { token::SPACE_4 }))
                            .pb(look.length(token::SPACE_1))
                            .text_size(look.length(token::TEXT_2XS))
                            .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                            .text_color(look.color(token::FG_SUBTLE))
                            .child(day.to_string()),
                        Role::Heading,
                        day.to_string(),
                    )
                    .into_any_element(),
                );
            } else if let Some(entry) = line.strip_prefix("- ") {
                let (at, said) = entry.split_once(' ').filter(|(at, _)| at.len() == 5 && at.contains(':')).unwrap_or(("", entry));
                body.push(
                    labelled(
                        div()
                            .id(SharedString::from(format!("log-line-{n}")))
                            .flex()
                            .gap(look.length(token::SPACE_3))
                            .py(look.length(token::SPACE_1))
                            .text_size(look.length(token::TEXT_SM))
                            .line_height(look.length(token::LEADING_SM))
                            .child(div().flex_none().w(look.length(token::SPACE_12)).text_color(look.color(token::FG_SUBTLE)).child(at.to_string()))
                            .child(div().flex_1().min_w_0().child(said.to_string())),
                        Role::ListItem,
                        format!("{at} {said}").trim().to_string(),
                    )
                    .into_any_element(),
                );
            } else if !line.trim().is_empty() && !line.starts_with("# ") {
                body.push(div().py(look.length(token::SPACE_1)).text_color(look.color(token::FG_MUTED)).child(line.to_string()).into_any_element());
            }
        }
        self.sheet("log", "The whole Log", "bot", &self.log_focus, &self.log_scroll, body, Self::close_log, look, cx)
    }

    /// A sheet over the page: its head with Close, its list, Esc or a press outside to shut it.
    #[allow(clippy::too_many_arguments)]
    fn sheet(
        &self,
        id: &'static str,
        title: &'static str,
        mark: &'static str,
        focus: &gpui::FocusHandle,
        scroll: &gpui::ScrollHandle,
        body: Vec<AnyElement>,
        close: fn(&mut OperatorPageView, &mut Window, &mut Context<OperatorPageView>),
        look: &Look,
        cx: &Context<Self>,
    ) -> AnyElement {
        let shut = labelled(pressed(press(format!("{id}-close"), "Close".into(), None, Some("Esc".into()), Weight::Bare, look), cx, move |this, window, cx| close(this, window, cx)), Role::Button, "Close");
        let panel = div()
            .id(SharedString::from(format!("{id}-panel")))
            .occlude()
            .w(look.length(token::SPACE_12) * 12.)
            .max_h(gpui::relative(0.8))
            .flex()
            .flex_col()
            .rounded(round(look))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_DEFAULT))
            .bg(look.color(token::BG_OVERLAY))
            .shadow(look.shadows(token::SHADOW_OVERLAY))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_3))
                    .px(look.length(token::SPACE_4))
                    .py(look.length(token::SPACE_3))
                    .border_b(look.length(token::BORDER_WIDTH))
                    .border_color(look.color(token::BORDER_SUBTLE))
                    .child(glyph(mark, look.length(token::SPACE_4), look.color(token::OPERATOR)))
                    .child(labelled(
                        div().id(SharedString::from(format!("{id}-title"))).flex_1().text_size(look.length(token::TEXT_BASE)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).child(title),
                        Role::Heading,
                        title,
                    ))
                    .child(shut),
            )
            .child(div().id(SharedString::from(format!("{id}-list"))).track_scroll(scroll).flex_1().min_h_0().overflow_y_scroll().px(look.length(token::SPACE_3)).py(look.length(token::SPACE_2)).children(body));
        labelled(
            div()
                .id(SharedString::from(format!("{id}-sheet")))
                .track_focus(focus)
                .on_key_down(cx.listener(move |this, event: &gpui::KeyDownEvent, window, cx| {
                    if event.keystroke.key == "escape" {
                        cx.stop_propagation();
                        close(this, window, cx);
                    }
                }))
                .absolute()
                .inset_0()
                .flex()
                .items_center()
                .justify_center()
                .bg(look.color(token::SCRIM))
                .on_mouse_down(MouseButton::Left, cx.listener(move |this, _, window, cx| close(this, window, cx)))
                .child(panel.on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())),
            Role::Dialog,
            title,
        )
        .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_bare_outcome_says_what_it_was_about() {
        let act = |outcome: &str, title: &str| ipc::operator_page::OperatorHandledAct { at: ipc::Instant::carried("2026-11-11T12:20:00-05:00"), title: title.into(), outcome: outcome.into() };
        assert_eq!(handled_said(&act("Started", "Fix desktop_unit after #412")), "Started: Fix desktop_unit after #412");
        assert_eq!(handled_said(&act("Linked: the report to #3398", "Build switcher")), "Linked: the report to #3398");
        assert_eq!(handled_said(&act("Passed on", "")), "Passed on");
    }

    #[test]
    fn a_time_reads_in_his_clock_with_its_day_when_not_today() {
        let now = chrono::DateTime::parse_from_rfc3339("2026-11-11T13:05:00-05:00").expect("a time").with_timezone(&chrono::Local);
        let at = |rfc: &str| chrono::DateTime::parse_from_rfc3339(rfc).expect("a time").with_timezone(&chrono::Local);
        let today = at("2026-11-11T12:47:30-05:00");
        let before = at("2026-11-10T16:00:00-05:00");
        crate::settings::time::hold(r#"{"hour12":false,"seconds":false,"date":false}"#);
        assert_eq!(minute(&today.to_rfc3339(), now).as_deref(), Some(today.format("%H:%M").to_string().as_str()));
        assert_eq!(minute(&before.to_rfc3339(), now), Some(before.format("%b %-d %H:%M").to_string()));
        crate::settings::time::hold(r#"{"hour12":true,"seconds":true,"date":false}"#);
        assert_eq!(minute(&today.to_rfc3339(), now).as_deref(), Some(today.format("%-I:%M:%S %p").to_string().as_str()));
    }

    #[test]
    fn a_rule_reads_where_it_holds_then_the_rule_in_small() {
        let rule = |repository: Option<&str>| ipc::operator_page::OperatorLearned {
            id: 1,
            scope: match repository {
                Some(name) => ipc::operator_page::RuleScope::Repository { manifest_id: name.into() },
                None => ipc::operator_page::RuleScope::Everywhere,
            },
            repository: repository.map(String::from),
            said: "Start a fix when main is red".into(),
            at: ipc::Instant::carried("2026-11-09T10:30:00-05:00"),
        };
        assert_eq!(rule_said(&rule(None)), "everywhere: start a fix when main is red");
        assert_eq!(rule_said(&rule(Some("armada"))), "armada: start a fix when main is red");
    }

    #[test]
    fn a_hold_reads_as_a_rule_does_and_names_its_manifest_where_no_repository_is_given() {
        let hold = |repository: Option<&str>| ipc::operator_page::OperatorHold {
            id: 11,
            manifest_id: "01REPO".into(),
            repository: repository.map(String::from),
            said: "Only Slot pool controls writes under apps/cockpit until it pushes".into(),
            paths: Vec::new(),
            job: None,
            until: None,
            at: ipc::Instant::carried("2026-11-11T12:40:00-05:00"),
        };
        assert_eq!(hold_said(&hold(Some("armada"))), "armada: only Slot pool controls writes under apps/cockpit until it pushes");
        assert_eq!(hold_said(&hold(None)), "01REPO: only Slot pool controls writes under apps/cockpit until it pushes");
    }

    #[test]
    fn a_log_day_reads_as_the_page_head_writes_a_day() {
        assert_eq!(log_day("Tuesday 2026-11-10"), "Tuesday 10 November");
        assert_eq!(log_day("Someday"), "Someday");
    }

    #[test]
    fn rows_sit_under_their_project_everywhere_first_then_by_name() {
        let row = |text: &str, place: &str| Row { text: text.into(), said: text.into(), place: Some(place.into()), side: Side::None, opened: None, forget: None, hold: None };
        let rows = by_place(vec![row("a", "storefront"), row("b", "Everywhere"), row("c", "armada"), row("d", "storefront"), row("e", "Everywhere")]);
        assert_eq!(rows.iter().map(|row| row.text.as_str()).collect::<Vec<_>>(), ["b", "e", "c", "a", "d"]);
    }
}
