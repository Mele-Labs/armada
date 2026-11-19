//! The card: one decision, drawn whole in front, folded to one line while he talks about it, and whole again where the
//! Operator raises one in the talk. Its edge and chips wear the Operator's hue; a head band names its kind, title and Jobs;
//! My read and the options are its body; Discuss later and Discuss are its foot.
//!
//! **A press on an option, or its number, settles the card at once** with that option. An item that is no card, such as a
//! plan, a walk or a go-ahead to file a note, draws the same card: its options are the item's, its Thens, or its one pick.
//!
//! **[`Card`] is drawn for any view.** A Job's Needs you draws a question the Job asks on it (`job_view/needs_card.rs`),
//! with Discuss putting the caret in the Job's own box; it is never drawn a second way.

use std::rc::Rc;

use gpui::{div, prelude::*, AnyElement, Context, FontWeight, Role, SharedString, Window};
use ipc::operator_page::OperatorTurnItem;

use super::face::{self, Kind};
use super::page::OperatorPageView;
use super::view::{chip, kind_chip, measured, press, pressed, round, Weight};
use crate::labels::labelled;
use crate::look::{token, Look};
use crate::titlebar::parts::{glyph, kbd, tip};

/// What a press on the card does, on the view that draws it.
pub(crate) type Act<V> = Rc<dyn Fn(&mut V, &mut Window, &mut Context<V>)>;

/// One thing a card offers: its label, what it costs or does, whether it is the pick, and what a press does.
pub(crate) struct Offer<V> {
    pub label: SharedString,
    pub cost: Option<SharedString>,
    pub pick: bool,
    pub act: Act<V>,
}

/// The card, as any view fills it: the Operator's page with a card it brings, a Job's Needs you with a question the Job
/// asks. The view keeps what the card does; the card is drawn here once for both.
pub(crate) struct Card<V> {
    /// The item's id, which names the card's elements.
    pub id: String,
    pub kind: Kind,
    pub title: SharedString,
    /// Chips at the head's end: the Jobs a card is about, or what a question names.
    pub names: Vec<AnyElement>,
    /// Where the card stands among the ones behind it, at the head's end.
    pub stepper: Option<AnyElement>,
    /// What it asks, under the title, where the title is not the question.
    pub asks: Option<SharedString>,
    /// My read: the Operator's reading of the card.
    pub read: Option<SharedString>,
    pub offers: Vec<Offer<V>>,
    /// The number keys press its options: their caps show.
    pub keyed: bool,
    /// A press is out: the options wait for Fleet.
    pub pending: bool,
    /// Fleet's refusal of the last press.
    pub refused: Option<SharedString>,
    /// The acts at its foot, right-aligned.
    pub foot: Vec<AnyElement>,
    /// Drawn under the line that raised it, set in.
    pub inset: bool,
}

impl<V: 'static> Card<V> {
    /// The card whole.
    pub(crate) fn draw(self, look: &Look, cx: &Context<V>) -> AnyElement {
        let Card { id, kind, title, names, stepper, asks, read, offers, keyed, pending, refused, foot, inset } = self;
        let head = div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .px(look.length(token::SPACE_4))
            .py(look.length(token::SPACE_3))
            .bg(look.color(token::OPERATOR_FAINT))
            .border_b(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::OPERATOR_EDGE))
            .child(kind_chip(kind, look))
            .child(div().flex_1().min_w(gpui::relative(0.5)).text_size(look.length(token::TEXT_LG)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).child(title.clone()))
            .children(names)
            .children(stepper);
        let read = read.filter(|read| !read.is_empty()).map(|read| {
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
                .child(div().text_size(look.length(token::TEXT_SM)).line_height(look.length(token::LEADING_SM)).child(read))
        });
        let refused = refused.map(|why| {
            labelled(
                div()
                    .id("card-refused")
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_2))
                    .text_size(look.length(token::TEXT_SM))
                    .text_color(look.color(token::STATUS_COMPLETED_FAILED))
                    .child(glyph("circle-alert", look.length(token::SPACE_4), look.color(token::STATUS_COMPLETED_FAILED)))
                    .child(div().flex_1().min_w_0().child(why.clone())),
                Role::Alert,
                why,
            )
        });
        let body = div()
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_3))
            .pl(look.length(token::SPACE_4) + look.length(token::BORDER_WIDTH) * 3.)
            .pr(look.length(token::SPACE_4))
            .py(look.length(token::SPACE_4))
            .children(asks.map(|asks| div().text_size(look.length(token::TEXT_BASE)).font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM))).child(asks)))
            .children(read)
            .child(options(&id, offers, keyed, pending, look, cx))
            .children(refused);
        let foot = div()
            .flex()
            .flex_wrap()
            .items_center()
            .justify_end()
            .gap(look.length(token::SPACE_2))
            .px(look.length(token::SPACE_4))
            .py(look.length(token::SPACE_3))
            .border_t(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_SUBTLE))
            .bg(look.color(token::BG_OVERLAY))
            .children(foot);
        let card = div()
            .id(SharedString::from(format!("card-{id}")))
            .relative()
            .flex_none()
            .when(inset, |card| card.ml(look.length(token::SPACE_8)))
            .rounded(round(look))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::OPERATOR_EDGE))
            .bg(look.color(token::BG_RAISED))
            .overflow_hidden()
            .child(head)
            .child(body)
            .child(foot)
            // The Operator's hue, as a bar down the card's edge.
            .child(div().absolute().left_0().top_0().bottom_0().w(look.length(token::BORDER_WIDTH) * 3.).bg(look.color(token::OPERATOR)));
        labelled(card, Role::Article, title).into_any_element()
    }
}

/// Each option a full-width row: a radio glyph, the label, its cost under it, My pick on the pick and its number at the right.
/// A press settles the card on it.
fn options<V: 'static>(id: &str, offers: Vec<Offer<V>>, keyed: bool, pending: bool, look: &Look, cx: &Context<V>) -> impl IntoElement {
    let edge = look.color(token::OPERATOR);
    let rows = offers.into_iter().enumerate().map(|(at, offer)| {
        let label = offer.label.clone();
        let pick = offer.pick;
        let said = match (&offer.cost, pick) {
            (Some(cost), true) => format!("My pick. {cost}"),
            (Some(cost), false) => cost.to_string(),
            (None, true) => "My pick".into(),
            (None, false) => String::new(),
        };
        let ring = look.edge(token::FOCUS_RING).map_or(look.color(token::ACCENT), |(_, colour)| colour);
        let row = div()
            .id(SharedString::from(format!("option-{id}-{at}")))
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_3))
            .min_h(look.length(token::H_CONTROL_LG) + look.length(token::SPACE_3))
            .py(look.length(token::SPACE_2))
            .px(look.length(token::SPACE_3))
            .rounded(look.length(token::RADIUS_LG) * 1.25)
            .border(look.length(token::BORDER_WIDTH))
            .border_color(look.color(if pick { token::OPERATOR_EDGE } else { token::BORDER_DEFAULT }))
            .bg(look.color(if pick { token::OPERATOR_FAINT } else { token::BG_OVERLAY }))
            .text_size(look.length(token::TEXT_SM))
            .cursor_pointer()
            .hover(move |style| style.border_color(edge))
            .focusable()
            .tab_stop(true)
            .focus_visible(move |style| style.border_color(ring))
            .when(!said.is_empty(), |row| row.tooltip(tip(said.clone())))
            .child(glyph(if pick { "circle-dot" } else { "circle" }, look.length(token::SPACE_4), look.color(if pick { token::OPERATOR } else { token::FG_MUTED })))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .child(div().font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM))).child(label.clone()))
                    .children(offer.cost.clone().map(|cost| div().text_size(look.length(token::TEXT_XS)).text_color(look.color(token::FG_MUTED)).child(cost))),
            )
            .when(pick, |row| row.child(div().text_size(look.length(token::TEXT_2XS)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).text_color(look.color(token::OPERATOR)).child("My pick")))
            .when(keyed, |row| row.child(kbd((at + 1).to_string(), look)));
        let row = if pending {
            row.opacity(0.5)
        } else {
            let act = offer.act;
            pressed(row, cx, move |this, window, cx| act(this, window, cx))
        };
        labelled(row, Role::Button, label)
    });
    div().mt(look.length(token::SPACE_1)).child(labelled(div().id(SharedString::from(format!("options-{id}"))).flex().flex_col().gap(look.length(token::SPACE_2)).children(rows), Role::Group, "Options"))
}

impl OperatorPageView {
    /// What the card offers, in the order to draw it.
    fn offers(&self, turn: &OperatorTurnItem) -> Vec<Offer<Self>> {
        let id = turn.id.clone();
        if face::chooses(turn) {
            let pick = face::pick_of(turn);
            return turn
                .item
                .options
                .iter()
                .enumerate()
                .map(|(at, option)| {
                    let (id, label) = (id.clone(), option.label.clone());
                    Offer {
                        label: option.label.clone().into(),
                        cost: option.description.clone().filter(|said| !said.is_empty()).map(SharedString::from),
                        // The first is the Operator's pick where it names none.
                        pick: pick.as_ref().map_or(at == 0, |pick| pick.as_ref() == option.label),
                        act: Rc::new(move |this: &mut Self, _: &mut Window, cx: &mut Context<Self>| this.settle_option(&id, &label, cx)),
                    }
                })
                .collect();
        }
        if !turn.then.is_empty() {
            return turn
                .then
                .iter()
                .map(|offer| {
                    let to = offer.hand_to.as_ref().map(|to| to.job.as_str());
                    let (id, then) = (id.clone(), offer.then);
                    Offer {
                        label: face::then_said(offer.then, to).0,
                        cost: Some(face::then_does(offer.then, to).into()),
                        pick: offer.pick,
                        act: Rc::new(move |this: &mut Self, _: &mut Window, cx: &mut Context<Self>| this.settle_then(&id, then, cx)),
                    }
                })
                .collect();
        }
        let label = face::pick_word(turn, None);
        let yes: Act<Self> = Rc::new(move |this, window, cx| {
            this.bring(&id, cx);
            this.answer(ipc::operator_page::OperatorAnswer::Yes, window, cx)
        });
        vec![Offer { label, cost: face::pick_sentence(turn), pick: true, act: yes }]
    }

    /// The chips that name the Jobs a card is about, and its repository.
    fn names(&self, turn: &OperatorTurnItem, pressable: bool, look: &Look, cx: &Context<Self>) -> Vec<AnyElement> {
        let sessions: Vec<ipc::SessionId> = turn.card.as_ref().map(|card| card.sessions.clone()).filter(|sessions| !sessions.is_empty()).or_else(|| turn.session_id.clone().map(|id| vec![id])).unwrap_or_default();
        let mut chips: Vec<AnyElement> = sessions
            .into_iter()
            .map(|id| {
                let title = self.title_of_job(&id);
                let said = format!("Open {title}");
                let job = chip(title.clone(), look.color(token::FG_DEFAULT), look.color(token::BG_HOVER), look).child(glyph("box", look.length(token::SPACE_3), look.color(token::FG_MUTED)));
                let element = div().id(SharedString::from(format!("job-{}-{}", turn.id, id.as_str()))).flex_none().tooltip(tip(said.clone())).child(job);
                if !pressable {
                    return labelled(element, Role::Image, said).into_any_element();
                }
                labelled(pressed(element.cursor_pointer(), cx, move |this, _, cx| this.open_job(id.clone(), cx)), Role::Link, said).into_any_element()
            })
            .collect();
        chips.extend(turn.repository.clone().map(|repository| {
            let said = format!("Repository {repository}");
            labelled(
                div().id(SharedString::from(format!("repository-{}", turn.id))).flex_none().tooltip(tip(said.clone())).child(
                    chip(repository, look.color(token::OPERATOR), look.color(token::OPERATOR_FAINT), look).child(glyph("folder-git-2", look.length(token::SPACE_3), look.color(token::OPERATOR))),
                ),
                Role::Image,
                said,
            )
            .into_any_element()
        }));
        chips
    }

    /// The card whole. `inset` draws it as the Operator raised it in the talk, under its line.
    pub(super) fn card(&self, turn: &OperatorTurnItem, inset: bool, look: &Look, cx: &Context<Self>) -> AnyElement {
        let target = self.target().is_some_and(|target| target.id == turn.id);
        Card {
            id: turn.id.clone(),
            kind: face::kind_of(turn),
            title: face::card_title(turn),
            names: self.names(turn, true, look, cx),
            stepper: None,
            asks: face::asks_of(turn),
            read: turn.read.as_ref().map(|read| SharedString::from(read.read.clone())),
            offers: self.offers(turn),
            keyed: target,
            pending: self.pending.contains(&turn.id),
            refused: self.refused.as_ref().filter(|(id, _)| *id == turn.id).map(|(_, why)| why.clone()),
            foot: self.card_foot(target, look, cx),
            inset,
        }
        .draw(look, cx)
    }

    /// Discuss later and Discuss, on the card's lightest step. Their keys show on the card the keys act on.
    fn card_foot(&self, keyed: bool, look: &Look, cx: &Context<Self>) -> Vec<AnyElement> {
        let later = labelled(
            pressed(
                press("turn-later", "Discuss later".into(), Some("clock"), keyed.then(|| "L".into()), Weight::Bare, look)
                    .relative()
                    .when(keyed, |later| later.track_focus(&self.later_focus))
                    .tooltip(tip("Choose when the card comes back"))
                    .child(measured(&self.drawn, "later")),
                cx,
                |this, window, cx| this.open_later(false, window, cx),
            ),
            Role::Button,
            "Discuss later",
        );
        let discuss = labelled(
            pressed(press("turn-discuss", "Discuss".into(), Some("bot"), keyed.then(|| "/".into()), Weight::Pick, look).tooltip(tip("Talk to the Operator about this card")), cx, |this, window, cx| this.discuss(window, cx)),
            Role::Button,
            "Discuss",
        );
        vec![later.into_any_element(), discuss.into_any_element()]
    }

    /// The card folded to one line: its kind, title and Jobs, and a chevron that opens it again.
    pub(super) fn folded(&self, turn: &OperatorTurnItem, open: bool, look: &Look, cx: &Context<Self>) -> AnyElement {
        let title = face::card_title(turn);
        let said = format!("{} {title}", if open { "Fold" } else { "Unfold" });
        let ring = look.edge(token::FOCUS_RING).map_or(look.color(token::ACCENT), |(_, colour)| colour);
        let line = div()
            .id(SharedString::from(format!("folded-{}", turn.id)))
            .flex_none()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .min_h(look.length(token::H_CONTROL_LG) + look.length(token::SPACE_2))
            .px(look.length(token::SPACE_4))
            .rounded(look.length(token::RADIUS_LG) * 1.25)
            .border(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::OPERATOR_EDGE))
            .bg(look.color(token::OPERATOR_FAINT))
            .cursor_pointer()
            .focusable()
            .tab_stop(true)
            .focus_visible(move |style| style.border_color(ring))
            .tooltip(tip(if open { "Fold the card to one line" } else { "Open the card beside the talk" }))
            .child(kind_chip(face::kind_of(turn), look))
            .child(div().flex_none().max_w(gpui::relative(0.5)).truncate().font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).child(title))
            .child(div().flex_1().min_w_0().overflow_hidden().flex().items_center().gap(look.length(token::SPACE_2)).children(self.names(turn, false, look, cx)))
            .child(glyph(if open { "chevron-down" } else { "chevron-right" }, look.length(token::SPACE_4), look.color(token::OPERATOR)));
        labelled(pressed(line, cx, |this, _, cx| this.toggle_fold(cx)), Role::Button, said).into_any_element()
    }
}
