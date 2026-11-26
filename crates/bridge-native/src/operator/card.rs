//! The card: one decision, drawn whole in front, folded to one line while the talk under it overflows, and whole again where the
//! Operator raises one in the talk. Its frame and chips wear the Operator's hue; a head band names its kind, title and Jobs;
//! My read and the options are its body; Snooze and Discuss are its foot.
//!
//! **A press on an option, or its number, settles the card at once** with that option. An item that is no card, such as a
//! plan, a walk or a go-ahead to file a note, draws the same card: its options are the item's, its Thens, or its one pick.
//!
//! **[`Card`] is drawn for any view.** A Job's Needs you draws a question the Job asks on it (`job_view/needs_card.rs`),
//! with Discuss putting the caret in the Job's own box; it is never drawn a second way.

use std::rc::Rc;

use gpui::{div, prelude::*, AnyElement, Context, Div, FocusHandle, FontWeight, Role, SharedString, Stateful, Window};
use ipc::operator_page::OperatorTurnItem;

use super::face::{self, Kind};
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
pub(super) fn card_head(kind: Kind, title: SharedString, look: &Look) -> Div {
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

/// What the card says it asks, under the title, where the title is not the question.
fn asks_line(asks: SharedString, look: &Look) -> Div {
    div().text_size(look.length(token::TEXT_BASE)).font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM))).child(asks)
}

/// The read section: on a card that came back, the line of what he picked and why, then My read.
fn read_section(id: &str, returned: Option<SharedString>, read: Option<SharedString>, look: &Look) -> Option<Div> {
    let danger = look.color(token::STATUS_COMPLETED_FAILED);
    let lead = returned.map(|said| {
        labelled(
            div()
                .id(SharedString::from(format!("returned-{id}")))
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
    let read = read.filter(|read| !read.is_empty()).map(|read| read_block("My read", "bot", read, look));
    (lead.is_some() || read.is_some()).then(|| div().flex().flex_col().gap(look.length(token::SPACE_2)).children(lead).children(read))
}

/// Fleet's refusal of his last press on the card, in red.
fn refused_line(why: SharedString, look: &Look) -> impl IntoElement {
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
        why,
    )
}

/// The group the option rows stand in.
fn option_group(id: &str, rows: impl IntoIterator<Item = impl IntoElement>, look: &Look) -> Div {
    div().mt(look.length(token::SPACE_1)).child(labelled(div().id(SharedString::from(format!("options-{id}"))).flex().flex_col().gap(look.length(token::SPACE_2)).children(rows), Role::Group, "Options"))
}

/// One option row, without its press: the key badge where a mark would stand, the label and its cost, My pick on the
/// pick, and Choose at the right where the pointer or the keys are on it. `focus` is the row's kept handle, where its
/// view keeps one; `lifted` says the keys are on it.
#[allow(clippy::too_many_arguments)]
fn option_row(card: &str, at: usize, label: &SharedString, cost: Option<&SharedString>, pick: bool, keyed: bool, focus: Option<&FocusHandle>, lifted: bool, look: &Look) -> Stateful<Div> {
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
    let row = div()
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
        .hover(move |style| style.bg(fill).border_color(accent));
    let row = match focus {
        Some(handle) => row.track_focus(handle),
        None => row.focusable(),
    };
    row.tab_stop(true)
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
    /// A card that did not settle: what he picked and why it came back, over My read, and a red edge.
    pub returned: Option<SharedString>,
    pub offers: Vec<Offer<V>>,
    /// The number keys press its options: their caps show.
    pub keyed: bool,
    /// Each option's kept focus handle, in order; where there are none a row takes focus as it is drawn.
    pub focus: Vec<FocusHandle>,
    /// The option that has the keys: it is lifted, with Choose at its right.
    pub lifted: Option<usize>,
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
        let Card { id, kind, title, names, stepper, asks, read, returned, offers, keyed, focus, lifted, pending, refused, foot, inset } = self;
        let danger = look.color(token::STATUS_COMPLETED_FAILED);
        let back = returned.is_some();
        let head = card_head(kind, title.clone(), look).when(back, |head| head.border_color(danger)).children(names).children(stepper);
        let (asks, read) = (asks.map(|asks| asks_line(asks, look)), read_section(&id, returned, read, look));
        let rows = offers.into_iter().enumerate().map(|(at, offer)| {
            let row = option_row(&id, at, &offer.label, offer.cost.as_ref(), offer.pick, keyed, focus.get(at), lifted == Some(at), look);
            let row = if pending {
                row.opacity(0.5)
            } else {
                let act = offer.act;
                pressed(row, cx, move |this, window, cx| act(this, window, cx))
            };
            labelled(row, Role::Button, option_name(&offer.label, offer.cost.as_ref()))
        });
        let options = option_group(&id, rows, look);
        let refused = refused.map(|why| refused_line(why, look));
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
        // The card is drawn whole wherever it stands: nothing inside it scrolls, and the talk under it gives up its room.
        let body = div().flex().flex_col().gap(look.length(token::SPACE_3)).px(look.length(token::SPACE_4)).py(look.length(token::SPACE_4)).children(asks).children(read).child(options).children(refused);
        let card = card_frame(&id, inset, look).when(back, |card| card.border_color(danger)).child(head).child(body).child(foot);
        labelled(card, Role::Article, title).into_any_element()
    }
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
    pub(super) fn card(&self, turn: &OperatorTurnItem, inset: bool, window: &Window, look: &Look, cx: &Context<Self>) -> AnyElement {
        // A Job's build to try stands as the Job's own card, in this card's place: its head names the build, so no frame here.
        if !inset && face::answers_on_card(turn) {
            return div().flex().flex_col().gap(look.length(token::SPACE_3)).child(self.ready_body(look)).children(self.refused_line(turn, look)).into_any_element();
        }
        let target = self.target().is_some_and(|target| target.id == turn.id);
        let offers = self.offers(turn);
        let focus: Vec<FocusHandle> = (0..offers.len()).map(|at| self.option_handle(&format!("{}-{at}", turn.id), cx)).collect();
        let lifted = focus.iter().position(|handle| handle.is_focused(window));
        Card {
            id: turn.id.clone(),
            kind: face::kind_of(turn),
            title: face::card_title(turn),
            names: self.names(turn, true, look, cx),
            stepper: None,
            asks: face::asks_of(turn),
            read: turn.read.as_ref().map(|read| SharedString::from(read.read.clone())),
            returned: face::returned_lead(turn),
            offers,
            keyed: target,
            focus,
            lifted,
            pending: self.pending.contains(&turn.id),
            refused: self.refused.as_ref().filter(|(id, _)| *id == turn.id).map(|(_, why)| why.clone()),
            foot: self.card_foot(target, look, cx),
            inset,
        }
        .draw(look, cx)
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
            .children(face::asks_of(turn).map(|asks| asks_line(asks, look)))
            .children(read_section(&turn.id, face::returned_lead(turn), turn.read.as_ref().map(|read| read.read.clone().into()), look))
            .child(self.options_picked(turn, look))
            .children(self.steps_list("card", turn, look, cx).map(|steps| div().mt(look.length(token::SPACE_1)).child(steps)));
        labelled(card_frame(&turn.id, false, look).child(head).child(body), Role::Article, title).into_any_element()
    }

    /// Fleet's refusal of his last press on the card, in red.
    fn refused_line(&self, turn: &OperatorTurnItem, look: &Look) -> Option<impl IntoElement> {
        self.refused.as_ref().filter(|(id, _)| *id == turn.id).map(|(_, why)| refused_line(why.clone(), look))
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
        option_group(&turn.id, rows, look)
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

    /// The group the option rows stand in, for a card drawn on this page outside [`Card`].
    pub(super) fn option_group(&self, id: &str, rows: impl IntoIterator<Item = impl IntoElement>, look: &Look) -> Div {
        option_group(id, rows, look)
    }

    /// One option row as [`Card`] draws it, without its press, with its focus handle kept on this page.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn option_row(&self, card: &str, at: usize, label: &SharedString, cost: Option<&SharedString>, pick: bool, keyed: bool, window: &Window, look: &Look, cx: &Context<Self>) -> Stateful<Div> {
        let handle = self.option_handle(&format!("{card}-{at}"), cx);
        let lifted = handle.is_focused(window);
        option_row(card, at, label, cost, pick, keyed, Some(&handle), lifted, look)
    }

    /// The focus handle of an option row, made once and kept: the row reads whether it has the keys.
    fn option_handle(&self, key: &str, cx: &Context<Self>) -> FocusHandle {
        self.option_focus.borrow_mut().entry(key.to_string()).or_insert_with(|| cx.focus_handle()).clone()
    }

    /// Snooze and Discuss, on the card's lightest step. Their keys show on the card the keys act on.
    fn card_foot(&self, keyed: bool, look: &Look, cx: &Context<Self>) -> Vec<AnyElement> {
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
        vec![later.into_any_element(), discuss.into_any_element()]
    }

    /// The card in front folded to one line while the talk overflows: its title, the muted line beside it, and a chevron. A
    /// press opens it. A card's line is what it asks; the Operator's state, with no card in front, has its last act.
    pub(super) fn folded(&self, id: &str, title: SharedString, aside: Option<SharedString>, look: &Look, cx: &Context<Self>) -> AnyElement {
        let said = format!("Unfold {title}");
        let ring = look.edge(token::FOCUS_RING).map_or(look.color(token::ACCENT), |(_, colour)| colour);
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
            .child(div().flex_1().min_w_0().truncate().text_size(look.length(token::TEXT_XS)).text_color(look.color(token::FG_MUTED)).children(aside.map(|aside| labelled(div().id(SharedString::from(format!("folded-{id}-asks"))).truncate().child(aside.clone()), Role::Status, aside))))
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
