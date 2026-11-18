//! The card: one decision, drawn whole in front, folded to one line while the talk under it overflows, and whole again where the
//! Operator raises one in the talk. Its frame and chips wear the Operator's hue; a head band names its kind, title and Jobs;
//! My read and the options are its body; Snooze and Discuss are its foot.
//!
//! **A press on an option, or its number, settles the card at once** with that option. An item that is no card, such as a
//! plan, a walk or a go-ahead to file a note, draws the same card: its options are the item's, its Thens, or its one pick.

use gpui::{canvas, div, prelude::*, AnyElement, Context, Div, FontWeight, Role, SharedString, Stateful, Window};
use ipc::operator_page::OperatorTurnItem;

use super::face;
use super::page::OperatorPageView;
use super::view::{chip, kind_chip, measured, press, pressed, round, Weight};
use crate::labels::labelled;
use crate::look::{token, Look};
use crate::titlebar::parts::{glyph, kbd, spinner, tip};

/// An option's name for a reader: its label, then its cost.
pub(super) fn option_name(label: &SharedString, cost: Option<&SharedString>) -> String {
    match cost {
        Some(cost) => format!("{label}, {cost}"),
        None => label.to_string(),
    }
}

/// The card's frame: the Operator's hue on its edge, a raised ground, rounded corners. `inset` indents it under a line of the talk.
pub(super) fn card_frame(id: &str, inset: bool, look: &Look) -> Stateful<Div> {
    div()
        .id(SharedString::from(format!("card-{id}")))
        .flex_none()
        .when(inset, |card| card.ml(look.length(token::SPACE_8)))
        .rounded(round(look))
        .border(look.length(token::BORDER_WIDTH))
        .border_color(look.color(token::OPERATOR_EDGE))
        .bg(look.color(token::BG_RAISED))
        .overflow_hidden()
}

/// The card's head band: its kind chip and its title, with room after them for the chips that name its Jobs.
pub(super) fn card_head(kind: face::Kind, title: SharedString, look: &Look) -> Div {
    div()
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
        .child(div().flex_1().min_w(gpui::relative(0.5)).text_size(look.length(token::TEXT_LG)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).child(title))
}

/// The read under the title: a small heading with its glyph, and the words, on the Operator's muted ground.
pub(super) fn read_block(head: &'static str, mark: &'static str, words: SharedString, look: &Look) -> Div {
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
                .child(glyph(mark, look.length(token::SPACE_3) + look.length(token::SPACE_1) / 4., look.color(token::OPERATOR)))
                .child(head),
        )
        .child(div().text_size(look.length(token::TEXT_SM)).line_height(look.length(token::LEADING_SM)).child(words))
}

/// One thing a card offers: its label, what it costs or does, whether it is the Operator's pick, and what a press does.
struct Offer {
    label: SharedString,
    cost: Option<SharedString>,
    pick: bool,
    act: Act,
}

enum Act {
    Option(String),
    Then(ipc::operator_page::OperatorThen),
    Yes,
}

impl OperatorPageView {
    /// What the card offers, in the order to draw it.
    fn offers(&self, turn: &OperatorTurnItem) -> Vec<Offer> {
        if face::chooses(turn) {
            let pick = face::pick_of(turn);
            return turn
                .item
                .options
                .iter()
                .enumerate()
                .map(|(at, option)| Offer {
                    label: option.label.clone().into(),
                    cost: option.description.clone().filter(|said| !said.is_empty()).map(SharedString::from),
                    // The first is the Operator's pick where it names none.
                    pick: pick.as_ref().map_or(at == 0, |pick| pick.as_ref() == option.label),
                    act: Act::Option(option.label.clone()),
                })
                .collect();
        }
        if !turn.then.is_empty() {
            return turn
                .then
                .iter()
                .map(|offer| {
                    let to = offer.hand_to.as_ref().map(|to| to.job.as_str());
                    Offer { label: face::then_said(offer.then, to).0, cost: Some(face::then_does(offer.then, to).into()), pick: offer.pick, act: Act::Then(offer.then) }
                })
                .collect();
        }
        let label = face::pick_word(turn, None);
        vec![Offer { label, cost: face::pick_sentence(turn), pick: true, act: Act::Yes }]
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
    pub(super) fn card(&self, turn: &OperatorTurnItem, inset: bool, window: &Window, look: &Look, cx: &Context<Self>) -> AnyElement {
        // A Job's build to try stands as the Job's own card, in this card's place: its head names the build, so no frame here.
        if !inset && face::answers_on_card(turn) {
            return div().flex().flex_col().gap(look.length(token::SPACE_3)).child(self.ready_body(look)).children(self.refused_line(turn, look)).into_any_element();
        }
        let title = face::card_title(turn);
        let target = self.target().is_some_and(|target| target.id == turn.id);
        let danger = look.color(token::STATUS_COMPLETED_FAILED);
        let returned = face::returned(turn);
        let head = card_head(face::kind_of(turn), title.clone(), look).when(returned, |head| head.border_color(danger)).children(self.names(turn, true, look, cx));
        if inset {
            let card = card_frame(&turn.id, inset, look).when(returned, |card| card.border_color(danger)).child(head).child(self.card_body(turn, target, window, look, cx)).child(self.card_foot(target, look, cx));
            return labelled(card, Role::Article, title).into_any_element();
        }
        // In front, a tall card scrolls what it says in its own place; its answers and its foot stay whole at its bottom edge.
        // It keeps a line's worth in sight. The page reads how much more of it is drawn, to know the room the card needs
        // without it.
        let (asks, read) = (self.asks(turn, look), self.my_read(turn, look));
        let said = (asks.is_some() || read.is_some()).then(|| {
            let (tall, least) = (self.said_tall.clone(), look.length(token::SPACE_12));
            div()
                .id(SharedString::from(format!("card-said-{}", turn.id)))
                .relative()
                .flex_shrink(100.)
                .min_h(least)
                .overflow_y_scroll()
                .mt(look.length(token::SPACE_4))
                .child(
                    div().flex().flex_col().gap(look.length(token::SPACE_3)).px(look.length(token::SPACE_4)).children(asks).children(read),
                )
                .child(canvas(move |bounds, _, _| tall.set((bounds.size.height - least).max(gpui::px(0.))), |_, _, _, _| {}).absolute().top_0().left_0().size_full())
        });
        // The answers hold their room, and scroll in their place only in a window too short for them and the box.
        // The page reads how much of them a short window scrolled away, to give it back.
        let (inner, hidden) = (std::rc::Rc::new(std::cell::Cell::new(gpui::px(0.))), self.answers_hidden.clone());
        let whole = inner.clone();
        let answers = div()
            .id(SharedString::from(format!("card-answers-{}", turn.id)))
            .relative()
            .flex_shrink(1.)
            .min_h_0()
            .overflow_y_scroll()
            .mt(look.length(if said.is_some() { token::SPACE_3 } else { token::SPACE_4 }))
            .px(look.length(token::SPACE_4))
            .child(
                div()
                    .relative()
                    .child(self.options(turn, target, window, look, cx))
                    .child(canvas(move |bounds, _, _| whole.set(bounds.size.height), |_, _, _, _| {}).absolute().top_0().left_0().size_full()),
            )
            .child(canvas(move |bounds, _, _| hidden.set((inner.get() - bounds.size.height).max(gpui::px(0.))), |_, _, _, _| {}).absolute().top_0().left_0().size_full());
        let refused = div().flex_none().flex().flex_col().px(look.length(token::SPACE_4)).pb(look.length(token::SPACE_4)).children(self.refused_line(turn, look).map(|line| div().pt(look.length(token::SPACE_3)).child(line)));
        let card = card_frame(&turn.id, false, look)
            .flex_shrink(1.)
            .min_h_0()
            .flex()
            .flex_col()
            .when(returned, |card| card.border_color(danger))
            .child(head.flex_none())
            .children(said)
            .child(answers)
            .child(refused)
            .child({
                let low = self.foot_low.clone();
                self.card_foot(target, look, cx).flex_none().relative().child(canvas(move |bounds, _, _| low.set(bounds.bottom()), |_, _, _, _| {}).absolute().top_0().left_0().size_full())
            });
        labelled(card, Role::Article, title).into_any_element()
    }

    /// A card he picked, brought to the front from the feed to be read: its read, every option with his pick marked and none
    /// to press, and what the Operator did on it. It has no Snooze and no Discuss.
    pub(super) fn working_card(&self, turn: &OperatorTurnItem, look: &Look, cx: &Context<Self>) -> AnyElement {
        let title = face::card_title(turn);
        let head = card_head(face::kind_of(turn), title.clone(), look).children(self.names(turn, true, look, cx));
        let body = div()
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_3))
            .px(look.length(token::SPACE_4))
            .py(look.length(token::SPACE_4))
            .children(self.asks(turn, look))
            .children(self.my_read(turn, look))
            .child(self.options_picked(turn, look))
            .children(self.steps_list("card", turn, look, cx).map(|steps| div().mt(look.length(token::SPACE_1)).child(steps)));
        labelled(card_frame(&turn.id, false, look).child(head).child(body), Role::Article, title).into_any_element()
    }

    /// What the card says it asks, under the title, where the title is the Job's.
    fn asks(&self, turn: &OperatorTurnItem, look: &Look) -> Option<Div> {
        face::asks_of(turn).map(|asks| div().text_size(look.length(token::TEXT_BASE)).font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM))).child(asks))
    }

    /// The read section: on a card that came back, the line of what he picked and why, then My read.
    fn my_read(&self, turn: &OperatorTurnItem, look: &Look) -> Option<Div> {
        let danger = look.color(token::STATUS_COMPLETED_FAILED);
        let lead = face::returned_lead(turn).map(|said| {
            labelled(
                div()
                    .id(SharedString::from(format!("returned-{}", turn.id)))
                    .flex()
                    .items_start()
                    .gap(look.length(token::SPACE_2))
                    .px(look.length(token::SPACE_3))
                    .py(look.length(token::SPACE_3))
                    .rounded(look.length(token::RADIUS_LG) * 1.25)
                    .bg(look.color(token::STATUS_COMPLETED_FAILED_BG))
                    .text_color(danger)
                    .text_size(look.length(token::TEXT_SM))
                    .line_height(look.length(token::LEADING_SM))
                    .child(div().flex_none().h(look.length(token::LEADING_SM)).flex().items_center().child(glyph("circle-alert", look.length(token::SPACE_4), danger)))
                    .child(div().flex_1().min_w_0().child(said.clone())),
                Role::Status,
                said,
            )
        });
        let read = turn.read.as_ref().filter(|read| !read.read.is_empty()).map(|read| read_block("My read", "bot", read.read.clone().into(), look));
        (lead.is_some() || read.is_some()).then(|| div().flex().flex_col().gap(look.length(token::SPACE_2)).children(lead).children(read))
    }

    /// My read, the options and Fleet's refusal.
    fn card_body(&self, turn: &OperatorTurnItem, keyed: bool, window: &Window, look: &Look, cx: &Context<Self>) -> Div {
        div()
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_3))
            .px(look.length(token::SPACE_4))
            .py(look.length(token::SPACE_4))
            .children(self.asks(turn, look))
            .children(self.my_read(turn, look))
            .child(self.options(turn, keyed, window, look, cx))
            .children(self.refused_line(turn, look))
    }

    /// Fleet's refusal of his last press on the card, in red.
    fn refused_line(&self, turn: &OperatorTurnItem, look: &Look) -> Option<impl IntoElement> {
        self.refused.as_ref().filter(|(id, _)| *id == turn.id).map(|(_, why)| {
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
        })
    }

    /// A Job's Ready to try, in the card's place: the Job view's own card with its answers, or its skeleton until the view
    /// shows the Job.
    fn ready_body(&self, look: &Look) -> Div {
        let drawn = self.ready_drawn.borrow_mut().take();
        let bones = || {
            div()
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_2))
                .child(super::view::bone(1., look.length(token::H_CONTROL_LG) + look.length(token::SPACE_3), look))
                .child(super::view::bone(0.6, look.length(token::LEADING_SM), look))
                .child(super::view::bone(1., look.length(token::H_CONTROL_LG), look))
        };
        div().relative().child(measured(&self.drawn, "ready")).child(drawn.unwrap_or_else(|| bones().into_any_element()))
    }

    /// The options of a card he has picked on, none to press: his pick lit in the accent with Picked at its right, the rest
    /// dimmed.
    fn options_picked(&self, turn: &OperatorTurnItem, look: &Look) -> impl IntoElement {
        let picked = face::picked_option(turn);
        let accent = look.color(token::ACCENT);
        let rows = self.offers(turn).into_iter().enumerate().map(|(at, offer)| {
            let mine = picked.as_ref().is_some_and(|picked| *picked == offer.label);
            let name = option_name(&offer.label, offer.cost.as_ref());
            let row = div()
                .id(SharedString::from(format!("picked-{}-{at}", turn.id)))
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_3))
                .min_h(look.length(token::H_CONTROL_LG) + look.length(token::SPACE_3))
                .py(look.length(token::SPACE_2))
                .px(look.length(token::SPACE_3))
                .rounded(look.length(token::RADIUS_LG) * 1.25)
                .border(look.length(token::BORDER_WIDTH))
                .border_color(if mine { accent } else { look.color(token::BORDER_DEFAULT) })
                .bg(look.color(if mine { token::ACCENT_FAINT } else { token::BG_OVERLAY }))
                .text_size(look.length(token::TEXT_SM))
                .when(!mine, |row| row.opacity(0.5))
                .tooltip(tip(if mine { "Your pick" } else { "Already picked" }))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .child(div().font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM))).child(offer.label.clone()))
                        .children(offer.cost.clone().map(|cost| div().text_size(look.length(token::TEXT_XS)).text_color(look.color(token::FG_MUTED)).child(cost))),
                )
                .when(mine, |row| row.child(chip("Picked", look.color(token::TEXT_ON_ACCENT), accent, look).child(glyph("check", look.length(token::SPACE_3), look.color(token::TEXT_ON_ACCENT)))));
            labelled(row, Role::ListItem, if mine { format!("{name}, picked") } else { name })
        });
        self.option_group(&turn.id, rows, look)
    }

    /// What the Operator did on a picked card, oldest first: a check on each step done, and a turning ring on the newest.
    /// `scope` keeps a card's list apart from its feed row's.
    pub(super) fn steps_list(&self, scope: &str, turn: &OperatorTurnItem, look: &Look, cx: &Context<Self>) -> Option<crate::labels::Labelled> {
        let steps = &turn.card.as_ref()?.steps;
        let last = steps.len().checked_sub(1)?;
        let size = look.length(token::SPACE_3) + look.length(token::SPACE_1) / 4.;
        let lines = steps.iter().enumerate().map(|(at, step)| {
            let id = SharedString::from(format!("{scope}-step-{}-{at}", turn.id));
            let (mark, said) = if at == last {
                (spinner(SharedString::from(format!("{scope}-spin-{}-{at}", turn.id)), size, look.color(token::ACCENT), cx).into_any_element(), "Working on it")
            } else {
                (glyph("check", size, look.color(token::STATUS_COMPLETED_SUCCESS)).into_any_element(), "Done")
            };
            labelled(
                div()
                    .id(id.clone())
                    .flex()
                    .items_start()
                    .gap(look.length(token::SPACE_2))
                    .text_size(look.length(token::TEXT_XS))
                    .line_height(look.length(token::LEADING_SM))
                    .text_color(look.color(token::FG_MUTED))
                    .child(div().id(SharedString::from(format!("{id}-mark"))).flex_none().h(look.length(token::LEADING_SM)).flex().items_center().tooltip(tip(said)).child(mark))
                    .child(div().flex_1().min_w_0().child(step.said.clone())),
                Role::ListItem,
                step.said.clone(),
            )
        });
        Some(labelled(div().id(SharedString::from(format!("{scope}-steps-{}", turn.id))).flex().flex_col().gap(look.length(token::SPACE_1)).children(lines), Role::List, "Steps"))
    }

    /// Each option a full-width row: its key at the left, the label with its cost under it, My pick on the first. A hover or
    /// the keys lift the row and show Choose at its right. A press settles the card on it.
    fn options(&self, turn: &OperatorTurnItem, keyed: bool, window: &Window, look: &Look, cx: &Context<Self>) -> impl IntoElement {
        let pending = self.pending.contains(&turn.id);
        let rows = self.offers(turn).into_iter().enumerate().map(|(at, offer)| {
            let id = turn.id.clone();
            let row = self.option_row(&turn.id, at, &offer.label, offer.cost.as_ref(), offer.pick, keyed, window, look, cx);
            let row = if pending {
                row.opacity(0.5)
            } else {
                match offer.act {
                    Act::Option(option) => pressed(row, cx, move |this, _, cx| this.settle_option(&id, &option, cx)),
                    Act::Then(then) => pressed(row, cx, move |this, _, cx| this.settle_then(&id, then, cx)),
                    Act::Yes => pressed(row, cx, move |this, window, cx| {
                        this.bring(&id, cx);
                        this.answer(ipc::operator_page::OperatorAnswer::Yes, window, cx)
                    }),
                }
            };
            labelled(row, Role::Button, option_name(&offer.label, offer.cost.as_ref()))
        });
        self.option_group(&turn.id, rows, look)
    }

    /// The group the option rows stand in.
    pub(super) fn option_group(&self, id: &str, rows: impl IntoIterator<Item = impl IntoElement>, look: &Look) -> Div {
        div().mt(look.length(token::SPACE_1)).child(labelled(div().id(SharedString::from(format!("options-{id}"))).flex().flex_col().gap(look.length(token::SPACE_2)).children(rows), Role::Group, "Options"))
    }

    /// One option row, without its press: the key badge where a mark would stand, the label and its cost, My pick on the
    /// Operator's pick, and Choose at the right where the pointer or the keys are on it.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn option_row(&self, card: &str, at: usize, label: &SharedString, cost: Option<&SharedString>, pick: bool, keyed: bool, window: &Window, look: &Look, cx: &Context<Self>) -> Stateful<Div> {
        let handle = self.option_handle(&format!("{card}-{at}"), cx);
        let lifted = handle.is_focused(window);
        let said = match (cost, pick) {
            (Some(cost), true) => format!("My pick. {cost}"),
            (Some(cost), false) => cost.to_string(),
            (None, true) => "My pick".into(),
            (None, false) => String::new(),
        };
        let (accent, fill) = (look.color(token::ACCENT), look.color(token::BG_HOVER));
        let group = SharedString::from(format!("option-{card}-{at}"));
        let choose = chip("Choose", look.color(token::TEXT_ON_ACCENT), accent, look)
            .id(SharedString::from(format!("choose-{card}-{at}")))
            .child(glyph("chevron-right", look.length(token::SPACE_3), look.color(token::TEXT_ON_ACCENT)))
            .opacity(if lifted { 1. } else { 0. })
            .group_hover(group.clone(), |style| style.opacity(1.));
        div()
            .id(SharedString::from(format!("option-{card}-{at}")))
            .group(group)
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
            .hover(move |style| style.bg(fill).border_color(accent))
            .track_focus(&handle)
            .tab_stop(true)
            .focus_visible(move |style| style.bg(fill).border_color(accent))
            .when(lifted, |row| row.bg(fill).border_color(accent))
            .when(!said.is_empty(), |row| row.tooltip(tip(said.clone())))
            .when(keyed, |row| row.child(kbd((at + 1).to_string(), look)))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .child(div().font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM))).child(label.clone()))
                    .children(cost.cloned().map(|cost| div().text_size(look.length(token::TEXT_XS)).text_color(look.color(token::FG_MUTED)).child(cost))),
            )
            .when(pick, |row| row.child(div().text_size(look.length(token::TEXT_2XS)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).text_color(look.color(token::OPERATOR)).child("My pick")))
            .child(choose)
    }

    /// The focus handle of an option row, made once and kept: the row reads whether it has the keys.
    fn option_handle(&self, key: &str, cx: &Context<Self>) -> gpui::FocusHandle {
        self.option_focus.borrow_mut().entry(key.to_string()).or_insert_with(|| cx.focus_handle()).clone()
    }

    /// Snooze and Discuss, on the card's lightest step. Their keys show on the card the keys act on.
    fn card_foot(&self, keyed: bool, look: &Look, cx: &Context<Self>) -> Div {
        let later = labelled(
            pressed(
                press("turn-later", "Snooze".into(), Some("clock"), keyed.then(|| "S".into()), Weight::Bare, look)
                    .relative()
                    .when(keyed, |later| later.track_focus(&self.later_focus))
                    .tooltip(tip("Choose when the card comes back"))
                    .child(measured(&self.drawn, "later")),
                cx,
                |this, window, cx| this.open_later(false, window, cx),
            ),
            Role::Button,
            "Snooze",
        );
        let discuss = labelled(
            pressed(press("turn-discuss", "Discuss".into(), Some("bot"), keyed.then(|| "/".into()), Weight::Pick, look).tooltip(tip("Talk to the Operator about this card")), cx, |this, window, cx| this.discuss(window, cx)),
            Role::Button,
            "Discuss",
        );
        div()
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
            .child(later)
            .child(discuss)
    }

    /// The card in front folded to one line while the talk overflows: its title, the Operator's last act, and a chevron. A
    /// press opens it.
    pub(super) fn folded(&self, id: &str, title: SharedString, look: &Look, cx: &Context<Self>) -> AnyElement {
        let said = format!("Unfold {title}");
        let ring = look.edge(token::FOCUS_RING).map_or(look.color(token::ACCENT), |(_, colour)| colour);
        let now = self.clock_now.unwrap_or_else(chrono::Local::now);
        let last = self.page.as_ref().and_then(|page| page.feed.first()).map(|entry| super::status::last_act(entry, now));
        let line = div()
            .id(SharedString::from(format!("folded-{id}")))
            .flex_none()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_3))
            .h(look.length(token::H_CONTROL_LG) + look.length(token::SPACE_2))
            .px(look.length(token::SPACE_4))
            .rounded(round(look))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::OPERATOR_EDGE))
            .bg(look.color(token::OPERATOR_FAINT))
            .cursor_pointer()
            .focusable()
            .tab_stop(true)
            .focus_visible(move |style| style.border_color(ring))
            .tooltip(tip("Open the card above the talk"))
            .child(div().flex_none().max_w(gpui::relative(0.6)).truncate().font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).child(title))
            .child(div().flex_1().min_w_0().truncate().text_size(look.length(token::TEXT_XS)).text_color(look.color(token::FG_MUTED)).children(last))
            .child(glyph("chevron-right", look.length(token::SPACE_4), look.color(token::OPERATOR)));
        // The line goes once it is open, so the keys go back to the page.
        labelled(
            pressed(line, cx, |this, window, cx| {
                this.unfold(cx);
                window.focus(&this.focus, cx);
            }),
            Role::Button,
            said,
        )
        .into_any_element()
    }
}
