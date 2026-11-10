//! The page drawn, as the approved sketch "Morning report: a written document" lays it out: the head and
//! its pills, the top line, Your turn with one item open and the rest one line each, and the digest beside
//! it, which the chat takes the place of on Ask. Roomier than the sketch: controls a step taller, corners
//! rounder, as the native screens are.

use gpui::{
    div, prelude::*, AnyElement, App, Context, Div, FontWeight, Hsla, MouseButton, Pixels, Role, SharedString, Stateful, Window,
};
use ipc::operator_page::{OperatorAnswer, OperatorPage, OperatorTurnItem, OperatorTurnKind};

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

/// A grey shape where words will be, while the page is read.
fn bone(width: f32, height: Pixels, look: &Look) -> Div {
    div().h(height).w(gpui::relative(width)).rounded(look.length(token::RADIUS_LG)).bg(look.color(token::BG_HOVER))
}

impl OperatorPageView {
    // -- the head ----------------------------------------------------------------------------------

    fn head(&self, look: &Look, cx: &App) -> impl IntoElement {
        let pill = |id: &'static str| {
            div()
                .id(id)
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
        let mains = self.standing.mains.iter().enumerate().map(|(at, (repository, red))| {
            let said = if *red { format!("{repository} main is red") } else { format!("{repository} main is green") };
            let (mark, hue) = if *red { ("circle-x", token::STATUS_COMPLETED_FAILED) } else { ("circle-check", token::STATUS_COMPLETED_SUCCESS) };
            let pill = pill(if at == 0 { "pill-main-0" } else if at == 1 { "pill-main-1" } else { "pill-main" })
                .when(*red, |pill| {
                    pill.border_color(look.color(token::STATUS_COMPLETED_FAILED))
                        .bg(look.color(token::STATUS_COMPLETED_FAILED_BG))
                        .text_color(look.color(token::STATUS_COMPLETED_FAILED))
                })
                .tooltip(tip(said.clone()))
                .child(glyph(mark, look.length(token::SPACE_3) + look.length(token::SPACE_1) / 4., look.color(hue)))
                .child(format!("{repository} main"));
            labelled(pill, Role::Status, said).into_any_element()
        });
        let week = self.standing.week.or_else(|| week_from_bar(cx)).map(|percent| {
            let said = format!("{percent}% of the week's plan used");
            labelled(
                pill("pill-week").tooltip(tip(said.clone())).child(format!("Week {percent}%")).child(meter(percent as f32 / 100., token::STATUS_RUNNING)),
                Role::Status,
                said,
            )
        });
        let slots = self.standing.slots.map(|(held, all)| {
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
                    .text_size(look.length(token::TEXT_XS))
                    .text_color(look.color(token::FG_MUTED))
                    .children(self.back_at.clone().map(|back| format!("back at {back}"))),
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
                    .child(div().flex_1().min_w_0().child(top.clone())),
                Role::Status,
                "Top line",
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

    fn on_it(&self, page: &OperatorPage, look: &Look, cx: &App) -> Option<AnyElement> {
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
                    .child(div().text_size(look.length(token::TEXT_XS)).text_color(look.color(token::FG_SUBTLE)).child(format!("since {}", face::clock(&on_it.since)))),
                Role::Status,
                "On it",
            )
            .into_any_element(),
        )
    }

    fn card(&self, turn: &OperatorTurnItem, look: &Look, cx: &Context<Self>) -> AnyElement {
        let kind = face::kind_of(turn);
        let title = face::title_of(turn);
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
            let key: Option<SharedString> = if yes { Some("A".into()) } else if has_then { None } else { Some((at + 1).to_string().into()) };
            let button = press(format!("answer-{at}"), label.clone(), yes.then_some("check"), key, if yes { Weight::Pick } else { Weight::Plain }, look)
                .tooltip(tip(match answer {
                    OperatorAnswer::Yes => "Do my pick",
                    OperatorAnswer::AlwaysHere => "Do my pick, and keep it as a rule in this project",
                    OperatorAnswer::Always => "Do my pick, and keep it as a rule everywhere",
                    OperatorAnswer::No => "Drop it",
                }))
                .on_click(cx.listener(move |this, _, window, cx| this.answer(answer, window, cx)));
            labelled(button, Role::Button, label).into_any_element()
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
            .children(answers)
            .child(div().flex_1())
            .child(labelled(
                press("turn-later", "Later".into(), None, Some("L".into()), Weight::Bare, look)
                    .tooltip(tip("Last for the rest of today"))
                    .on_click(cx.listener(|this, _, _, cx| this.later(cx))),
                Role::Button,
                "Later",
            ))
            .child(labelled(
                press("turn-ask", "Ask".into(), Some("bot"), Some("/".into()), Weight::Bare, look)
                    .tooltip(tip("Ask the Operator about this"))
                    .on_click(cx.listener(|this, _, window, cx| this.ask(window, cx))),
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
                .tooltip(tip(if pending { "Sending your answer" } else { "Done" }))
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

    fn turn_row(&self, at: usize, turn: &OperatorTurnItem, look: &Look, cx: &Context<Self>) -> AnyElement {
        let kind = face::kind_of(turn);
        let title = face::title_of(turn);
        let id = turn.id.clone();
        let drop_id = turn.id.clone();
        let sub = match (face::pick_of(turn), turn.later_until.is_some()) {
            (Some(pick), true) => Some(format!("Later today. My pick: {pick}")),
            (Some(pick), false) => Some(format!("My pick: {pick}")),
            (None, true) => Some("Later today".to_string()),
            (None, false) => None,
        };
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
                .child(kind_chip(kind, look)),
            Role::Row,
            title,
        )
        .into_any_element()
    }

    fn rest(&self, look: &Look, cx: &Context<Self>) -> Option<AnyElement> {
        let open = self.open.as_deref();
        let mut rows: Vec<AnyElement> = self.done.iter().map(|done| self.done_row(done, look, cx)).collect();
        rows.extend(self.items().iter().enumerate().filter(|(_, turn)| Some(turn.id.as_str()) != open).map(|(at, turn)| self.turn_row(at, turn, look, cx)));
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
                    .children(rows),
                Role::List,
                "The rest of Your turn",
            )
            .into_any_element(),
        )
    }

    // -- the digest --------------------------------------------------------------------------------

    fn tile(&self, name: &'static str, mark: &'static str, hue: Token, aside: Option<String>, rows: Vec<(String, String, Option<Opened>)>, link: Option<(&'static str, Opened)>, look: &Look, cx: &Context<Self>) -> AnyElement {
        // As the chosen side column opens them: what it handled and what it learned.
        let open = self.folded.get(name).copied().unwrap_or(matches!(name, "What I handled" | "Learned"));
        let body = open.then(|| {
            let rows = rows.into_iter().enumerate().map(|(at, (text, right, opened))| {
                let row = div()
                    .id(SharedString::from(format!("{name}-{at}")))
                    .flex()
                    .gap(look.length(token::SPACE_2))
                    .py(look.length(token::SPACE_1))
                    .text_size(look.length(token::TEXT_XS))
                    .child(div().flex_1().min_w_0().child(text.clone()))
                    .child(div().flex_none().text_color(look.color(token::FG_SUBTLE)).child(right))
                    .when_some(opened, |row, opened| row.cursor_pointer().on_click(cx.listener(move |_, _, _, cx| cx.emit(opened.clone()))));
                labelled(row, Role::ListItem, text).into_any_element()
            });
            let link = link.map(|(said, opened)| {
                labelled(
                    div()
                        .id(SharedString::from(format!("{name}-link")))
                        .py(look.length(token::SPACE_1))
                        .text_size(look.length(token::TEXT_XS))
                        .text_color(look.color(token::ACCENT))
                        .cursor_pointer()
                        .hover(|style| style.text_color(cx.look().color(token::ACCENT_HOVER)))
                        .on_click(cx.listener(move |_, _, _, cx| cx.emit(opened.clone())))
                        .child(said),
                    Role::Link,
                    said,
                )
            });
            div()
                .px(look.length(token::SPACE_3))
                .pt(look.length(token::SPACE_1))
                .pb(look.length(token::SPACE_2))
                .border_t(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::BORDER_SUBTLE))
                .children(rows)
                .children(link)
        });
        let no_aside = aside.is_none();
        let head = labelled(
            div()
                .id(SharedString::from(format!("tile-{name}")))
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .px(look.length(token::SPACE_3))
                .h(look.length(token::H_CONTROL_LG) + look.length(token::SPACE_1))
                .cursor_pointer()
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.folded.insert(name, !open);
                    cx.notify();
                }))
                .child(glyph(mark, look.length(token::SPACE_4), look.color(hue)))
                .child(div().font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).text_size(look.length(token::TEXT_SM)).child(name))
                .children(aside.map(|aside| div().flex_1().min_w_0().truncate().text_size(look.length(token::TEXT_XS)).text_color(look.color(token::FG_MUTED)).child(aside)))
                .when(no_aside, |head| head.child(div().flex_1()))
                .child(glyph(if open { "chevron-down" } else { "chevron-right" }, look.length(token::SPACE_3) + look.length(token::SPACE_1) / 4., look.color(token::FG_SUBTLE))),
            Role::Button,
            name,
        );
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
            let rows = digest.landed.iter().map(|one| (format!("#{} {}", one.number, one.title), String::new(), Some(Opened::Job(one.session_id.clone())))).collect();
            tiles.push(self.tile("Landed", "git-merge", token::STATUS_COMPLETED_SUCCESS, None, rows, None, look, cx));
        }
        if !digest.handled.acts.is_empty() || digest.handled.log.is_some() {
            let rows = digest.handled.acts.iter().map(|act| (act.outcome.clone(), face::clock(&act.at).to_string(), None)).collect();
            let link = digest.handled.log.as_ref().map(|log| ("The whole Log", Opened::Log(log.path.clone())));
            tiles.push(self.tile("What I handled", "bot", token::OPERATOR, None, rows, link, look, cx));
        }
        if !digest.running.is_empty() {
            let rows = digest.running.iter().map(|one| (one.title.clone(), one.repository.clone().unwrap_or_default(), Some(Opened::Job(one.session_id.clone())))).collect();
            tiles.push(self.tile("Running", "activity", token::STATUS_RUNNING, None, rows, None, look, cx));
        }
        if !digest.coming_up.is_empty() {
            let rows = digest.coming_up.iter().map(|one| (format!("{}: waits on {}", one.title, one.needs), String::new(), Some(Opened::Job(one.session_id.clone())))).collect();
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
            let rows = digest
                .learned
                .iter()
                .map(|rule| (format!("{}: {}", rule.repository.as_deref().unwrap_or("Everywhere"), rule.said), day_month(&rule.at), None))
                .collect();
            tiles.push(self.tile("Learned", "scroll-text", token::OPERATOR, Some(places.join(", ")), rows, None, look, cx));
        }
        let since = page.looked_at.as_ref().map_or("Since you last looked".to_string(), |at| format!("Since you last looked, at {}", face::clock(at)));
        labelled(
            div()
                .id("digest")
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_3))
                .child(div().text_size(look.length(token::TEXT_2XS)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).text_color(look.color(token::FG_SUBTLE)).child(since))
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
        .border_b(look.length(token::BORDER_WIDTH))
        .border_color(look.color(token::BORDER_SUBTLE))
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

/// `10 Nov`.
fn day_month(at: &ipc::Instant) -> String {
    chrono::DateTime::parse_from_rfc3339(at.as_str()).map(|at| at.with_timezone(&chrono::Local).format("%-d %b").to_string()).unwrap_or_default()
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
            .on_key_down(cx.listener(|_, event: &gpui::KeyDownEvent, window, cx| {
                if crate::keymap::traverse(event, window, cx) {
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
        let off = (!page.on).then(|| div().text_color(look.color(token::FG_MUTED)).child("The Operator is off"));
        let main = div()
            .id("operator-main")
            .flex_1()
            .min_w_0()
            .h_full()
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .px(look.length(token::SPACE_6))
            .py(look.length(token::SPACE_4) + look.length(token::SPACE_1))
            // Held at its own height, so a long queue scrolls instead of squeezing the bars above it.
            .child(
                div()
                    .flex_none()
                    .flex()
                    .flex_col()
                    .gap(look.length(token::SPACE_3))
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
        root.children(self.top_line(page, look))
            .child(div().flex_1().min_h_0().flex().child(main).child(side(look).id("operator-side").overflow_y_scroll().child(aside)))
            .into_any_element()
    }
}
