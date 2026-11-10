//! The page drawn, as the approved sketch "Morning report: a written document" lays it out: the head and
//! its pills, the top line, Your turn with one item open and the rest one line each, and the digest beside
//! it, which the chat takes the place of on Ask. Roomier than the sketch: controls a step taller, corners
//! rounder, as the native screens are.

use gpui::{
    div, prelude::*, AnyElement, App, Context, Div, FontWeight, Hsla, MouseButton, Pixels, Role, SharedString, Stateful, Window,
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

fn chip(word: impl Into<SharedString>, hue: Hsla, ground: Hsla, look: &Look) -> Div {
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

fn kind_chip(kind: Kind, look: &Look) -> Div {
    chip(kind.word, look.color(kind.hue), look.color(kind.ground), look)
}

/// A press on the page: the pick filled in the Operator's hue, the rest outlined, Later and Ask bare.
#[derive(Clone, Copy, PartialEq)]
enum Weight {
    Pick,
    Plain,
    Bare,
}

fn press(id: impl Into<SharedString>, label: SharedString, icon: Option<&'static str>, key: Option<SharedString>, weight: Weight, look: &Look) -> Stateful<Div> {
    let id: SharedString = id.into();
    let (ground, hover, ink, edge) = match weight {
        Weight::Pick => (look.color(token::OPERATOR), look.color(token::OPERATOR), look.color(token::FG_INVERSE), look.color(token::OPERATOR)),
        Weight::Plain => (look.color(token::BG_RAISED), look.color(token::BG_HOVER), look.color(token::FG_DEFAULT), look.color(token::BORDER_STRONG)),
        Weight::Bare => (gpui::transparent_black(), look.color(token::BG_HOVER), look.color(token::FG_MUTED), gpui::transparent_black()),
    };
    let ring = look.edge(token::FOCUS_RING).map_or(look.color(token::ACCENT), |(_, colour)| colour);
    let cap = key.map(|key| {
        let cap = kbd(key, look);
        if weight == Weight::Pick { cap.bg(look.color(token::OPERATOR_MUTED)).border_color(look.color(token::OPERATOR_EDGE)).text_color(look.color(token::FG_INVERSE)) } else { cap }
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
        .font_weight(FontWeight(look.number(if weight == Weight::Pick { token::WEIGHT_HEADING } else { token::WEIGHT_MEDIUM })))
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
                .border_color(look.color(token::BORDER_DEFAULT))
                .bg(look.color(token::BG_RAISED))
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
                .children(mark.map(|(mark, hue)| glyph(mark, look.length(token::SPACE_3) + look.length(token::SPACE_1) / 4., look.color(hue))))
                .child(format!("{} main", main.repository));
            labelled(pill, Role::Status, said).into_any_element()
        });
        let mains: Vec<AnyElement> = mains.collect();
        let week = self.standing.week.or_else(|| week_from_bar(cx)).map(|percent| {
            let said = format!("{percent}% of the week's plan used");
            labelled(
                pill("pill-week").tooltip(tip(said.clone())).child(format!("Week {percent}%")).child(meter(percent as f32 / 100., token::STATUS_RUNNING)),
                Role::Status,
                said,
            )
        });
        let slots = slots_read.map(|slots| {
            let (held, all) = (slots.used, slots.total);
            let said = format!("Slots {held} of {all} held");
            labelled(
                pill("pill-slots").tooltip(tip(said.clone())).child(format!("Slots {held} of {all}")).child(meter(held as f32 / all.max(1) as f32, token::STATUS_AWAITING_REVIEW)),
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
            .border_b(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_SUBTLE))
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
            .children(week)
            .children(slots)
    }

    fn top_line(&self, page: &OperatorPage, look: &Look) -> Option<AnyElement> {
        let top = page.top.clone()?;
        Some(
            labelled(
                div()
                    .id("operator-top")
                    .flex_none()
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_3))
                    .px(look.length(token::SPACE_6))
                    .py(look.length(token::SPACE_3))
                    .bg(look.color(token::OPERATOR_FAINT))
                    .border_b(look.length(token::BORDER_WIDTH))
                    .border_color(look.color(token::OPERATOR_EDGE))
                    .text_size(look.length(token::TEXT_SM))
                    .line_height(look.length(token::LEADING_SM))
                    .child(glyph("bot", look.length(token::SPACE_4), look.color(token::OPERATOR)))
                    .child(div().flex_1().min_w_0().child(emphasised(&top, look))),
                Role::Status,
                top.replace("**", ""),
            )
            .into_any_element(),
        )
    }

    // -- Your turn ---------------------------------------------------------------------------------

    fn turn_head(&self, look: &Look) -> impl IntoElement {
        let items = self.items();
        let dots = self.done.iter().map(|_| token::STATUS_COMPLETED_SUCCESS).chain(
            items.iter().map(|turn| if self.open.as_deref() == Some(turn.id.as_str()) { token::OPERATOR } else { token::BORDER_STRONG }),
        );
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

    fn on_it(&self, page: &OperatorPage, look: &Look, cx: &Context<Self>) -> Option<AnyElement> {
        let on_it = page.on_it.as_ref()?;
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
                    .child(spinner("on-it-spin", look.length(token::SPACE_4), look.color(token::OPERATOR), cx))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .truncate()
                            .child(div().flex().gap(look.length(token::SPACE_1)).child(div().font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).child("On it:")).child(format!("reading {}", on_it.reading))),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .text_size(look.length(token::TEXT_XS))
                            .text_color(look.color(token::FG_SUBTLE))
                            .child("since")
                            .child(self.stamp(on_it.since.as_str(), None, (token::TEXT_XS, token::FG_SUBTLE, token::WEIGHT_BODY), look, cx)),
                    ),
                Role::Status,
                "On it",
            )
            .into_any_element(),
        )
    }

    fn card(&self, turn: &OperatorTurnItem, look: &Look, cx: &Context<Self>) -> AnyElement {
        let kind = face::kind_of(turn);
        let title = face::card_title(turn);
        let pick = face::pick_of(turn);
        let has_then = !turn.then.is_empty();
        let head = div()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .px(look.length(token::SPACE_4))
            .py(look.length(token::SPACE_3))
            .border_b(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_SUBTLE))
            .child(kind_chip(kind, look))
            .child(div().flex_1().min_w_0().truncate().text_size(look.length(token::TEXT_LG)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).child(title.clone()))
            .when(turn.later_until.is_some(), |head| {
                head.child(chip("Later", look.color(token::STATUS_NOT_STARTED), look.color(token::STATUS_NOT_STARTED_BG), look))
            })
            .children(turn.repository.clone().map(|repository| chip(repository, look.color(token::FG_MUTED), look.color(token::BG_HOVER), look)));
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
        let then = has_then.then(|| self.thens(turn, look, cx));
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
                    .child(format!("Not done: {why}")),
                Role::Alert,
                format!("Not done: {why}"),
            )
        });
        let body = div()
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_3))
            .px(look.length(token::SPACE_4))
            .py(look.length(token::SPACE_4))
            .children(face::asks_of(turn).map(|asks| div().text_size(look.length(token::TEXT_BASE)).font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM))).child(asks)))
            .children(face::note_of(turn).map(|(since, note)| {
                div()
                    .flex()
                    .flex_wrap()
                    .items_baseline()
                    .text_size(look.length(token::TEXT_BASE))
                    .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
                    .child("Your note from")
                    .child(self.stamp(since.as_str(), None, (token::TEXT_BASE, token::FG_DEFAULT, token::WEIGHT_MEDIUM), look, cx))
                    // The colon closes up on the time, past its ring's hairline.
                    .child(div().ml(-look.length(token::BORDER_WIDTH) * 2.).child(format!(": {note}")))
            }))
            .children(read)
            .children(then)
            .children(refused);
        let answers = face::answers_of(turn).into_iter().enumerate().map(|(at, answer)| {
            let yes = answer == OperatorAnswer::Yes;
            // A decision's Yes says the Operator's pick, as its card would; a go-ahead's says Yes.
            let label: SharedString = match (&pick, turn.kind) {
                (Some(pick), OperatorTurnKind::Decision) if yes => pick.clone(),
                _ => face::answer_word(answer).into(),
            };
            let key: Option<SharedString> = Some(face::answer_key(answer).into());
            let button = press(format!("answer-{at}"), label.clone(), yes.then_some("check"), key, if yes { Weight::Pick } else { Weight::Plain }, look).tooltip(tip(match answer {
                OperatorAnswer::Yes => "Do my pick",
                OperatorAnswer::AlwaysHere => "Do my pick, and keep it as a rule in this project",
                OperatorAnswer::Always => "Do my pick, and keep it as a rule everywhere",
                OperatorAnswer::No => "Drop it",
            }));
            let button = pressed(button, cx, move |this, window, cx| this.answer(answer, window, cx));
            labelled(button, Role::Button, label).into_any_element()
        });
        let changes = face::sends_changes(turn).then(|| {
            labelled(
                pressed(press("turn-changes", "Send changes".into(), None, Some("C".into()), Weight::Plain, look).tooltip(tip("Open the plan in its Job to suggest changes")), cx, |this, _, cx| {
                    this.send_changes(cx)
                }),
                Role::Button,
                "Send changes",
            )
        });
        let acts = div()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .px(look.length(token::SPACE_4))
            .py(look.length(token::SPACE_3))
            .border_t(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_SUBTLE))
            .bg(look.color(token::BG_OVERLAY))
            // The card clips its children square: its ground keeps inside the rounded corner itself.
            .rounded_b(round(look) - look.length(token::BORDER_WIDTH))
            .children(answers)
            .children(changes)
            .child(div().flex_1())
            .child(labelled(
                pressed(press("turn-later", "Later".into(), None, Some("L".into()), Weight::Bare, look).track_focus(&self.later_focus).tooltip(tip("Last for the rest of today")), cx, |this, _, cx| {
                    this.later(cx)
                }),
                Role::Button,
                "Later",
            ))
            .child(labelled(
                pressed(press("turn-ask", "Ask".into(), Some("bot"), Some("/".into()), Weight::Bare, look).tooltip(tip("Ask the Operator about this")), cx, |this, window, cx| {
                    this.ask(window, cx)
                }),
                Role::Button,
                "Ask",
            ));
        let id = turn.id.clone();
        labelled(
            div()
                .id("operator-card")
                .flex_none()
                .rounded(round(look))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(look.color(if turn.kind == OperatorTurnKind::GoAhead { token::OPERATOR } else { token::OPERATOR_EDGE }))
                .bg(look.color(token::BG_RAISED))
                .overflow_hidden()
                .on_drag(TurnDrag { id: turn.id.clone(), title: title.clone() }, |drag, _, _, cx| cx.new(|_| drag.clone()))
                .drag_over::<TurnDrag>(|style, _, _, cx| style.border_color(cx.look().color(token::OPERATOR)))
                .on_drop(cx.listener(move |this, drag: &TurnDrag, _, cx| {
                    if let Some(to) = this.items().iter().position(|one| one.id == id) {
                        this.move_to(&drag.id, to, cx);
                    }
                }))
                .child(head)
                .child(body)
                .child(acts),
            Role::Article,
            title,
        )
        .into_any_element()
    }

    /// Then, on a go-ahead to file: each choice a row, his pick or else the Operator's filled, My pick marked.
    fn thens(&self, turn: &OperatorTurnItem, look: &Look, cx: &Context<Self>) -> impl IntoElement {
        let chosen = self.then_of(turn);
        let rows = turn.then.iter().enumerate().map(|(at, offer)| {
            let (label, mark) = face::then_said(offer.then, offer.hand_to.as_ref().map(|to| to.job.as_str()));
            let on = chosen == Some(offer.then);
            let id = turn.id.clone();
            let then = offer.then;
            labelled(
                div()
                    .id(SharedString::from(format!("then-{at}")))
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
                    .on_click(cx.listener(move |this, _, _, cx| this.pick_then(&id, then, cx)))
                    .child(glyph(mark, look.length(token::SPACE_4), look.color(if on { token::OPERATOR } else { token::FG_MUTED })))
                    .child(div().flex_1().child(label.clone()))
                    .when(offer.pick, |row| {
                        row.child(
                            div()
                                .text_size(look.length(token::TEXT_2XS))
                                .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                                .text_color(look.color(token::OPERATOR))
                                .child("My pick"),
                        )
                    })
                    .child(kbd((at + 1).to_string(), look)),
                Role::RadioButton,
                label,
            )
        });
        div()
            .mt(look.length(token::SPACE_1))
            .child(div().mb(look.length(token::SPACE_2)).text_size(look.length(token::TEXT_2XS)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).text_color(look.color(token::FG_SUBTLE)).child("Then"))
            .child(labelled(div().id("then").flex().flex_col().gap(look.length(token::SPACE_2)).children(rows), Role::RadioGroup, "Then"))
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
                        .child(div().truncate().text_size(look.length(token::TEXT_XS)).child(sub)),
                )
                .child(kind_chip(done.kind, look).opacity(0.6)),
            Role::Row,
            format!("{}, done", done.title),
        )
        .into_any_element()
    }

    fn turn_row(&self, at: usize, turn: &OperatorTurnItem, open: bool, look: &Look, cx: &Context<Self>) -> AnyElement {
        let kind = face::kind_of(turn);
        let title = face::title_of(turn);
        let id = turn.id.clone();
        let drop_id = turn.id.clone();
        let sub = if open { Some("Open above".to_string()) } else { face::pick_of(turn).map(|pick| format!("My pick: {pick}")) };
        // Put Later: the clock beside its kind, as nobody acts on it until later today.
        let later = (turn.later_until.is_some() && !open).then(|| {
            labelled(
                div().id(SharedString::from(format!("later-{at}"))).tooltip(tip("Later today")).child(glyph("clock", look.length(token::SPACE_4), look.color(token::STATUS_NOT_STARTED))),
                Role::Image,
                "Later today",
            )
        });
        labelled(
            row_frame(SharedString::from(format!("turn-row-{at}")), look)
                .when(open, |row| row.text_color(look.color(token::OPERATOR)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))))
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

    fn rest(&self, look: &Look, cx: &Context<Self>) -> Option<AnyElement> {
        let open = self.open.as_deref();
        let mut rows: Vec<AnyElement> = self.done.iter().map(|done| self.done_row(done, look, cx)).collect();
        // The open item keeps its place in the list, marked, so a move or a drag shows where it went.
        rows.extend(self.items().iter().enumerate().map(|(at, turn)| self.turn_row(at, turn, Some(turn.id.as_str()) == open, look, cx)));
        if rows.is_empty() {
            return None;
        }
        Some(
            labelled(
                div()
                    .id("turn-rest")
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
                "The rest of Your turn",
            )
            .into_any_element(),
        )
    }

    // -- the digest --------------------------------------------------------------------------------

    /// A time as the thread draws one: the clock, or its age once a press has flipped every time on the
    /// page. `lead` goes before the clock alone, as `at` in `back at 13:05`, which reads `back 3m ago`.
    /// A press or Enter on any time flips them all.
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

    fn tile(&self, name: &'static str, mark: &'static str, hue: Token, aside: Option<String>, rows: Vec<Row>, link: Option<&'static str>, look: &Look, cx: &Context<Self>) -> AnyElement {
        let open = self.folded.get(name).copied().unwrap_or(opens_unpressed(name));
        let ring = look.edge(token::FOCUS_RING).map_or(look.color(token::ACCENT), |(_, colour)| colour);
        let body = open.then(|| {
            // The first few, one line each: the rest are a press away. Learned's are all on Every rule.
            let more = rows.len() > FEW && link != Some("Every rule");
            let whole = self.whole.contains(name);
            let shown = if whole || !more { rows.len() } else { FEW };
            // A hold's words are what matter in it, so it takes a second line before it is cut.
            let wraps = name == "Holds";
            let rows = rows.into_iter().take(shown).enumerate().map(|(at, row)| {
                let text = row.text;
                let line = div()
                    .id(SharedString::from(format!("{name}-{at}")))
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_2))
                    .py(look.length(token::SPACE_1))
                    .text_size(look.length(token::TEXT_XS))
                    .tooltip(tip(text.clone()))
                    .child(div().flex_1().min_w_0().when(wraps, |words| words.line_clamp(2).text_ellipsis()).when(!wraps, |words| words.truncate()).child(text.clone()))
                    .child(match row.side {
                        Side::None => gpui::Empty.into_any_element(),
                        Side::Text(said) => div().flex_none().text_color(look.color(token::FG_SUBTLE)).child(said).into_any_element(),
                        Side::At(at) => self.stamp(at.as_str(), None, (token::TEXT_XS, token::FG_SUBTLE, token::WEIGHT_BODY), look, cx),
                    })
                    .children(row.forget.map(|id| self.forget_button(id, &text, look, cx)));
                // A row that opens a Job is a tab stop, and Enter opens it as a press does.
                let line = match row.opened {
                    Some(opened) => pressed(worded(line, look).mx(-look.length(token::SPACE_1)), cx, move |_, _, cx| cx.emit(opened.clone())),
                    // The same inset as a row that opens a Job, so a ring on it stands clear of the words.
                    None => line.px(look.length(token::SPACE_1)).mx(-look.length(token::SPACE_1)),
                };
                labelled(line, Role::ListItem, text).into_any_element()
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
            let rows = digest.landed.iter().map(|one| Row { text: format!("#{} {}", one.number, one.title), side: Side::At(one.at.clone()), opened: Some(Opened::Job(one.session_id.clone())), forget: None }).collect();
            tiles.push(self.tile("Landed", "git-merge", token::STATUS_COMPLETED_SUCCESS, None, rows, None, look, cx));
        }
        if !digest.handled.acts.is_empty() || digest.handled.log.is_some() {
            let rows = digest.handled.acts.iter().map(|act| Row { text: handled_said(act), side: Side::At(act.at.clone()), opened: None, forget: None }).collect();
            // Off, nothing new is handled: what is here it did before it was switched off.
            let aside = (!page.on).then(|| "while it was on".to_string());
            let link = digest.handled.log.as_ref().map(|_| "The whole Log");
            tiles.push(self.tile("What I handled", "bot", token::OPERATOR, aside, rows, link, look, cx));
        }
        if !digest.running.is_empty() {
            let rows = digest.running.iter().map(|one| Row { text: one.title.clone(), side: Side::Text(one.repository.clone().unwrap_or_default()), opened: Some(Opened::Job(one.session_id.clone())), forget: None }).collect();
            tiles.push(self.tile("Running", "activity", token::STATUS_RUNNING, None, rows, None, look, cx));
        }
        if !digest.coming_up.is_empty() {
            let rows = digest.coming_up.iter().map(|one| Row { text: format!("{}: waits on {}", one.title, one.needs), side: Side::None, opened: Some(Opened::Job(one.session_id.clone())), forget: None }).collect();
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
            let rows = digest.learned.iter().map(|rule| Row { text: rule_said(rule), side: Side::At(rule.at.clone()), opened: None, forget: Some(rule.id) }).collect();
            tiles.push(self.tile("Learned", "scroll-text", token::OPERATOR, Some(places.join(", ")), rows, Some("Every rule"), look, cx));
        }
        if !digest.holds.is_empty() {
            let rows = digest.holds.iter().map(|hold| Row { text: hold_said(hold), side: Side::At(hold.at.clone()), opened: None, forget: Some(hold.id) }).collect();
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

/// The top line, what the Operator wrote between `**` in its hue and weight: the thing the line is about.
fn emphasised(top: &str, look: &Look) -> gpui::StyledText {
    let (mut said, mut marks) = (String::new(), Vec::new());
    for (at, part) in top.split("**").enumerate() {
        if at % 2 == 1 {
            let style = gpui::HighlightStyle { font_weight: Some(FontWeight(look.number(token::WEIGHT_HEADING))), color: Some(look.color(token::OPERATOR)), ..Default::default() };
            marks.push((said.len()..said.len() + part.len(), style));
        }
        said.push_str(part);
    }
    gpui::StyledText::new(said).with_highlights(marks)
}

/// A tile's rows past these are a press away, or in the Log.
const FEW: usize = 4;

/// Tiles the chosen side column draws open: what it handled, what it learned, and what it holds.
pub(super) fn opens_unpressed(name: &str) -> bool {
    matches!(name, "What I handled" | "Learned" | "Holds")
}

/// One row of a tile: its words, what stands at its right, and where a press on it goes.
pub(super) struct Row {
    text: String,
    side: Side,
    opened: Option<Opened>,
    /// A rule's or a hold's id, which its Forget names.
    forget: Option<i64>,
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

/// A time to the minute, as the page reads it: `12:47`, with the day when it is not today (`Nov 10 16:00`).
/// The thread's clock carries seconds; a page of what happened today does not need them.
pub(super) fn minute(at: &str, now: chrono::DateTime<chrono::Local>) -> Option<String> {
    let then = chrono::DateTime::parse_from_rfc3339(at).ok()?.with_timezone(&chrono::Local);
    let time = then.format("%H:%M").to_string();
    Some(if then.date_naive() == now.date_naive() { time } else { format!("{} {time}", then.format("%b %-d")) })
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

/// The week's share as the title bar's rings read it, on a live Fleet.
fn week_from_bar(cx: &App) -> Option<u32> {
    let bar = crate::titlebar::live(cx)?;
    match &bar.read(cx).face.usage {
        crate::titlebar::Usage::Limits(limits) => limits.iter().find(|limit| limit.name == "Weekly limit").map(|limit| limit.percent),
        _ => None,
    }
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
            .text_size(look.length(token::TEXT_SM))
            .child(self.head(look, cx));
        let Some(page) = self.page.as_ref() else {
            return root.child(self.skeleton(look)).into_any_element();
        };
        let open = self.open_item().map(|turn| self.card(turn, look, cx));
        let off = (!page.on).then(|| self.off(look, cx));
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
            // Held at its own height, so a long queue scrolls instead of squeezing the bars above it.
            .child(
                div()
                    .flex_none()
                    .flex()
                    .flex_col()
                    .gap(look.length(token::SPACE_3))
                    // As wide as the sketch's column at most, so a wide window does not stretch a card's lines.
                    .max_w(look.length(token::SPACE_12) * 16.)
                    .child(self.turn_head(look))
                    .children(self.on_it(page, look, cx))
                    .children(off)
                    .children(open)
                    .children(self.rest(look, cx)),
            );
        let aside = match &self.chat {
            Some(chat) => div().size_full().child(chat.clone()).into_any_element(),
            None => self.digest(page, look, cx).into_any_element(),
        };
        let sheet = self.rules.map(|at| self.rules_sheet(at, look, cx));
        root.relative()
            .children(self.top_line(page, look))
            .child(div().flex_1().min_h_0().flex().child(main).child(side(look).id("operator-side").overflow_y_scroll().child(aside)))
            .children(sheet)
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
        let close = labelled(
            pressed(press("rules-close", "Close".into(), None, Some("Esc".into()), Weight::Bare, look), cx, |this, window, cx| this.close_rules(window, cx)),
            Role::Button,
            "Close",
        );
        let panel = div()
            .id("rules-panel")
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
                    .child(glyph("scroll-text", look.length(token::SPACE_4), look.color(token::OPERATOR)))
                    .child(labelled(
                        div().id("rules-title").flex_1().text_size(look.length(token::TEXT_BASE)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).child("Every rule"),
                        Role::Heading,
                        "Every rule",
                    ))
                    .child(close),
            )
            .child(div().id("rules-list").track_scroll(&self.rules_scroll).flex_1().min_h_0().overflow_y_scroll().px(look.length(token::SPACE_3)).py(look.length(token::SPACE_2)).children(body));
        labelled(
            div()
                .id("rules-sheet")
                .track_focus(&self.rules_focus)
                .on_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, window, cx| {
                    if event.keystroke.key == "escape" {
                        cx.stop_propagation();
                        this.close_rules(window, cx);
                    }
                }))
                .absolute()
                .inset_0()
                .flex()
                .items_center()
                .justify_center()
                .bg(look.color(token::SCRIM))
                .on_mouse_down(MouseButton::Left, cx.listener(|this, _, window, cx| this.close_rules(window, cx)))
                .child(panel.on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())),
            Role::Dialog,
            "Every rule",
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
    fn a_time_reads_to_the_minute_with_its_day_when_not_today() {
        let now = chrono::DateTime::parse_from_rfc3339("2026-11-11T13:05:00-05:00").expect("a time").with_timezone(&chrono::Local);
        let at = |rfc: &str| chrono::DateTime::parse_from_rfc3339(rfc).expect("a time").with_timezone(&chrono::Local);
        let today = at("2026-11-11T12:47:30-05:00");
        let before = at("2026-11-10T16:00:00-05:00");
        assert_eq!(minute(&today.to_rfc3339(), now).as_deref(), Some(today.format("%H:%M").to_string().as_str()));
        assert_eq!(minute(&before.to_rfc3339(), now), Some(before.format("%b %-d %H:%M").to_string()));
        assert!(!minute(&today.to_rfc3339(), now).unwrap_or_default().contains(":30"), "no seconds");
    }

    #[test]
    fn a_rule_reads_where_it_holds_then_the_rule_in_small() {
        let rule = |repository: Option<&str>| ipc::operator_page::OperatorLearned {
            id: 1,
            manifest_id: None,
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
            at: ipc::Instant::carried("2026-11-11T12:40:00-05:00"),
        };
        assert_eq!(hold_said(&hold(Some("armada"))), "armada: only Slot pool controls writes under apps/cockpit until it pushes");
        assert_eq!(hold_said(&hold(None)), "01REPO: only Slot pool controls writes under apps/cockpit until it pushes");
    }
}
