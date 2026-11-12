//! A proposed workflow's draft card drawn as the workflow: its name and what it is for, then one
//! lane a step, each with what it does, what ends it and how it delivers, and what the agent is told
//! folded under a toggle. Fleet reads the steps off the draft (`WorkflowChange::after_steps`); a card
//! whose steps have not arrived draws the definition's text, as it did before.

use gpui::{div, prelude::*, Animation, AnimationExt, AnyElement, Context, FontWeight, Role, SharedString, Window};
use ipc::{AnswerWaiting, ChangeOrigin, ChangeStep, SessionId, SessionRow, ThreadItem, WorkflowChange};
use std::rc::Rc;

use super::acts::{button, Answer, Mode, Tone};
use super::rows::{glyph, mono, tip, Size};
use super::JobView;
use crate::labels::labelled;
use crate::look::{token, Look, Token};

/// The draft row's title for a workflow change: `Workflow <id>`. Rules and lifecycles carry no steps.
const WORKFLOW_TITLE: &str = "Workflow ";

/// What a lane's gate glyph draws: the glyph, its tooltip and its ink.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct GateMark {
    pub glyph: &'static str,
    pub named: &'static str,
    pub ink: Token,
}

/// One fact about how a step delivers, drawn as a line in its lane.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Fact {
    pub glyph: &'static str,
    pub said: &'static str,
}

/// One step, as its lane draws it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Lane {
    pub id: String,
    pub label: String,
    pub about: Option<String>,
    /// `Ends when …`, as Fleet reads the step's end.
    pub ends: Option<String>,
    pub gate: Option<GateMark>,
    pub facts: Vec<Fact>,
    pub instructions: Option<String>,
    pub delivers: bool,
}

/// What the draft proposes: a workflow nothing held before, or new steps for one that exists.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Proposal {
    New,
    Change,
}

impl Proposal {
    /// The card's eyebrow, in place of the draft card's `Workflow`.
    pub fn eyebrow(self) -> &'static str {
        match self {
            Proposal::New => "New workflow",
            Proposal::Change => "Workflow change",
        }
    }
}

/// What the change proposes, read off where it came from and what it replaces.
pub(super) fn proposal(change: &WorkflowChange) -> Proposal {
    if change.origin == ChangeOrigin::New || change.before.trim().is_empty() {
        Proposal::New
    } else {
        Proposal::Change
    }
}

/// The head's line saying what Approve does with the change.
pub(super) fn approves(proposal: Proposal, source: &str, id: &str) -> String {
    let kit = source == "kit";
    match proposal {
        Proposal::New if kit => String::from("Approve adds it to the workflows of every repository on this Mac."),
        Proposal::New => String::from("Approve adds it to this repository's workflows."),
        Proposal::Change if kit => format!("Approve replaces the steps of {id} on every repository on this Mac."),
        Proposal::Change => format!("Approve replaces the steps of {id}."),
    }
}

/// A proposed workflow, as the card draws it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Card {
    pub name: String,
    pub id: String,
    pub proposal: Proposal,
    /// What Approve does, said under the name.
    pub approves: String,
    pub for_requests: Option<String>,
    pub lanes: Vec<Lane>,
}

/// The glyph and name of a plan gate, as the reverted Electron lanes and a plan's Now draw them.
/// His own gates take his colour; the ones Fleet sees for itself take the kind they close.
pub(super) fn gate_mark(word: &str) -> Option<GateMark> {
    let (glyph, named, ink) = match word {
        "plan_approved" => ("stamp", "Your approval of the plan", token::YOU),
        "walk_approved" => ("footprints", "Your approval of the walk", token::YOU),
        "sketch_approved" => ("pencil-ruler", "Your approval of the sketch", token::YOU),
        "document_approved" => ("notebook-pen", "Your approval of the document", token::YOU),
        "you_approved" => ("circle-check", "Your Approve", token::YOU),
        "checks_green" => ("shield-check", "Checks pass", token::KIND_GATE),
        "merged" => ("git-merge", "The merge", token::KIND_DELIVERY),
        "jobs_ended" => ("link", "The Jobs it started end", token::KIND_GATE),
        _ => return None,
    };
    Some(GateMark { glyph, named, ink })
}

/// The delivery facts of a step, each a line in its lane.
pub(super) fn facts(step: &ChangeStep) -> Vec<Fact> {
    let mut facts = Vec::new();
    if step.asks_delivery {
        facts.push(Fact { glyph: "signpost", said: "Asks how to deliver" });
    }
    if step.delivers {
        facts.push(Fact { glyph: "git-pull-request", said: "Delivers" });
    }
    facts
}

pub(super) fn lane(step: &ChangeStep) -> Lane {
    Lane {
        id: step.id.as_str().to_string(),
        label: step.label.clone(),
        about: step.about.clone(),
        ends: step.ends_when.as_ref().map(|ends| format!("Ends when {ends}")),
        gate: step.gate.as_deref().and_then(gate_mark),
        facts: facts(step),
        instructions: step.instructions.clone(),
        delivers: step.delivers,
    }
}

/// The card for the change Fleet listed, where its steps arrived and it drafts the text the row
/// shows. `None` draws the text: no change read, no steps, or a send back redrafted it since.
pub(super) fn card_of(change: Option<&WorkflowChange>, text: &str) -> Option<Card> {
    let change = change?;
    if change.after.as_deref() != Some(text) {
        return None;
    }
    let steps = change.after_steps.as_ref().filter(|steps| !steps.is_empty())?;
    let id = change.workflow_id.as_str().to_string();
    Some(Card {
        name: change.after_name.clone().unwrap_or_else(|| id.clone()),
        proposal: proposal(change),
        approves: approves(proposal(change), &change.source, &id),
        id,
        for_requests: change.after_for_requests.clone(),
        lanes: steps.iter().map(lane).collect(),
    })
}

/// The button that sends the card's notes, and the line under each saved note that names it.
pub(super) const SEND_NOTES: &str = "Send notes";
const SENT_WITH: &str = "Goes to the agent when you press Send notes";

/// How a note on the whole workflow starts, in the text sent back.
pub(super) const WHOLE: &str = "Whole workflow";

/// The card key a note box is kept under: one for the whole workflow, one a lane.
pub(super) fn note_key(key: &str, lane: Option<&str>) -> String {
    match lane {
        Some(id) => format!("{key}:lane:{id}"),
        None => format!("{key}:whole"),
    }
}

/// The notes as one text, each line led by what it is on: `Quiz (quiz): …`, the whole workflow's
/// first. `None` while every note is blank, and Send notes is not offered.
pub(super) fn notes_sent(card: &Card, note: impl Fn(Option<&str>) -> String) -> Option<String> {
    let whole = std::iter::once((WHOLE.to_string(), note(None)));
    let lanes = card.lanes.iter().map(|lane| (format!("{} ({})", lane.label, lane.id), note(Some(&lane.id))));
    let lines: Vec<String> = whole
        .chain(lanes)
        .filter_map(|(on, said)| {
            let said = said.trim();
            (!said.is_empty()).then(|| format!("{on}: {said}"))
        })
        .collect();
    (!lines.is_empty()).then(|| lines.join("\n"))
}

/// The open workflow drafts of a thread whose change is to be read: change id, workflow id, text.
pub(super) fn wanted(items: &[ThreadItem]) -> Vec<(i64, String, String)> {
    items
        .iter()
        .filter_map(|item| match item {
            ThreadItem::Row { row: SessionRow::Draft { title, text, settled: None, change: Some(change), .. } } => {
                title.strip_prefix(WORKFLOW_TITLE).map(|workflow| (*change, workflow.trim().to_string(), text.clone()))
            }
            _ => None,
        })
        .collect()
}

impl JobView {
    /// Ask Fleet for the change behind each open workflow draft on screen. Run as Fleet moves, never in a render.
    pub(super) fn read_workflow_changes(&mut self, cx: &mut Context<Self>) {
        let (Some(id), Some(fleet)) = (self.shown.clone(), self.fleet.clone()) else { return };
        let Some(held) = self.thread(&id, cx) else { return };
        let wanted = wanted(&held.items);
        if wanted.is_empty() {
            return;
        }
        let manifest = self.record(&id, cx).and_then(|record| record.manifest_id.as_ref()).map(|manifest| manifest.as_str().to_string());
        fleet.update(cx, |fleet, cx| {
            for (change, workflow, text) in &wanted {
                fleet.read_workflow_change(*change, workflow, manifest.as_deref(), text, cx);
            }
        });
    }

    /// The workflow change Fleet (or the mock scenario) holds by id.
    fn workflow_change<'a>(&'a self, change: i64, cx: &'a gpui::App) -> Option<&'a WorkflowChange> {
        match &self.fleet {
            Some(fleet) => fleet.read(cx).held().workflow_changes.get(&change),
            None => self.mock_changes.get(&change),
        }
    }

    /// The workflow draft a card key names, as the thread holds it: its change, and whether it still waits.
    fn workflow_draft(&self, key: &str, cx: &gpui::App) -> Option<(i64, bool)> {
        let id = self.shown.clone()?;
        let row = key.strip_prefix("draft:")?;
        let held = self.thread(&id, cx)?;
        let (change, settled) = held.items.iter().find_map(|item| match item {
            ThreadItem::Row { row: SessionRow::Draft { id, change: Some(change), settled, .. } } if id == row => Some((*change, settled.is_some())),
            _ => None,
        })?;
        let answered = self.pages.get(&id).is_some_and(|page| page.cards.answers.contains_key(key));
        Some((change, !settled && !answered))
    }

    /// Whether a note box of the card is open for typing.
    fn note_open(&self, note: &str) -> bool {
        self.shown.as_ref().and_then(|id| self.pages.get(id)).and_then(|page| page.cards.modes.get(note)) == Some(&Mode::Note)
    }

    /// Save a note's words in its lane and shut its box; blank words save nothing.
    fn save_note(&mut self, ix: usize, note: &str, cx: &mut Context<Self>) {
        if self.typed_in(note, Mode::Note, cx).trim().is_empty() {
            self.close_box(ix, note, cx);
            return;
        }
        if let Some(page) = self.shown.clone().and_then(|id| self.pages.get_mut(&id)) {
            page.cards.modes.remove(note);
        }
        self.card_changed(ix, cx);
    }

    /// The notes on a workflow draft's card as the text sent back, while it waits and has one.
    fn workflow_notes(&self, key: &str, text: &str, cx: &Context<Self>) -> Option<String> {
        let (change, open) = self.workflow_draft(key, cx)?;
        let card = card_of(self.workflow_change(change, cx), text).filter(|_| open)?;
        notes_sent(&card, |lane| self.typed_in(&note_key(key, lane), Mode::Note, cx))
    }

    /// Send the card's notes back to the Job that proposed it, as a draft's note is.
    fn send_notes_back(&mut self, ix: usize, key: &str, text: &str, window: &mut Window, cx: &mut Context<Self>) {
        let Some(notes) = self.workflow_notes(key, text, cx) else { return };
        let session_id = SessionId::carried(self.shown.clone().unwrap_or_default().to_string());
        let body = AnswerWaiting { session_id, item_id: key.to_string(), choice: None, text: Some(notes.clone()), mode: None, fixed: None };
        let body = ipc::encode(&body).map_err(|why| format!("{why:?}"));
        self.settle(ix, key.to_string(), Answer::SentBack(notes), "/sessions/waiting/answer", body, window, cx);
    }

    /// The thread's Send back key on a workflow draft with a note.
    pub(super) fn workflow_send_back_key(&self, ix: usize, key: &str, text: &str, cx: &Context<Self>) -> Option<(String, super::keys::Run)> {
        self.workflow_notes(key, text, cx)?;
        let (key, text) = (key.to_string(), text.to_string());
        Some((SEND_NOTES.to_string(), Rc::new(move |this: &mut JobView, window: &mut Window, cx: &mut Context<JobView>| this.send_notes_back(ix, &key, &text, window, cx)) as super::keys::Run))
    }

    /// Send notes beside Approve and Dismiss, once the card has a note.
    pub(super) fn workflow_send_back(&self, ix: usize, key: &str, text: &str, chip: Option<&'static str>, look: &Look, cx: &Context<Self>) -> Option<AnyElement> {
        self.workflow_notes(key, text, cx)?;
        let (sent, text) = (key.to_string(), text.to_string());
        Some(
            labelled(
                button(look, SharedString::from(format!("send-notes:{key}")), SEND_NOTES, Tone::Secondary, false, chip, true)
                    .on_click(cx.listener(move |this, _, window, cx| this.send_notes_back(ix, &sent, &text, window, cx))),
                Role::Button,
                SEND_NOTES,
            )
            .into_any_element(),
        )
    }

    /// The disclosure's chevron: right while shut, down while open, turning when he presses it.
    fn told_chevron(&self, told_key: &str, told: bool, look: &Look) -> AnyElement {
        let turn = |at: f32| gpui::Transformation::rotate(gpui::percentage(at / 4.));
        let chevron = glyph(look, "chevron-right", Size::Small, look.color(token::FG_MUTED));
        // Drawn still until he first presses it, so a lane does not turn as it scrolls in.
        if !self.open_in_card(&format!("{told_key}:turned")) {
            return chevron.with_transformation(turn(if told { 1. } else { 0. })).into_any_element();
        }
        chevron
            .with_animation(
                SharedString::from(format!("turn:{told_key}:{told}")),
                Animation::new(look.duration(token::DURATION_FAST)).with_easing(gpui::ease_out_quint()),
                move |chevron, at| chevron.with_transformation(turn(if told { at } else { 1. - at })),
            )
            .into_any_element()
    }

    /// Open a note's box where it is drawn, with its words in it, and bring it up clear of the composer.
    fn edit_note(&mut self, ix: usize, note: &str, window: &mut Window, cx: &mut Context<Self>) {
        let card = note.rsplit_once(if note.ends_with(":whole") { ":whole" } else { ":lane:" }).map(|(card, _)| card.to_string()).unwrap_or_default();
        self.open_box(ix, note, Mode::Note, "", window, cx);
        if let Some(page) = self.shown.clone().and_then(|id| self.pages.get_mut(&id)) {
            page.cards.reveal.insert(note.to_string(), Rc::new(std::cell::Cell::new(true)));
        }
        // The thread keys stay on the card, so Send notes still reaches it.
        self.aim(&card);
    }

    /// Add note in its fixed column, while the step has none; a saved note is edited where it is drawn.
    fn note_button(&self, ix: usize, note: &str, on: &str, open: bool, look: &Look, cx: &Context<Self>) -> Option<AnyElement> {
        if !open || self.note_open(note) || !self.typed_in(note, Mode::Note, cx).trim().is_empty() {
            return None;
        }
        let opened = note.to_string();
        Some(
            labelled(
                button(look, SharedString::from(format!("Add note:{note}")), "Add note", Tone::Ghost, false, None, true)
                    .flex_none()
                    // Held to the label's line, so a lane is as tall with its control as without.
                    .my(-(look.length(token::H_CONTROL) - look.length(token::LEADING_SM)) / 2.)
                    .on_click(cx.listener(move |this, _, window, cx| this.edit_note(ix, &opened, window, cx))),
                Role::Button,
                format!("Add note on {on}"),
            )
            .into_any_element(),
        )
    }

    /// A saved note, drawn apart from the step as his: a header, his words, and where they go. A press
    /// on it, or Enter while it has the keys, opens it for editing in place.
    fn saved_note(&self, ix: usize, note: &str, on: &str, said: &str, open: bool, look: &Look, cx: &Context<Self>) -> AnyElement {
        let group = SharedString::from(format!("note:{note}"));
        let (edited, gone) = (note.to_string(), note.to_string());
        let header = div()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .text_size(look.length(token::TEXT_XS))
            .line_height(look.length(token::LEADING_XS))
            .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
            .text_color(look.color(token::FG_MUTED))
            .child(glyph(look, "sticky-note", Size::Small, look.color(token::YOU)))
            .child(div().flex_1().child("Your note"))
            .when(open, |header| {
                header
                    // The pencil shows while the pointer is on the note, to say a press edits it.
                    .child(
                        div()
                            .flex_none()
                            .opacity(0.)
                            .group_hover(group.clone(), |style| style.opacity(1.))
                            .child(glyph(look, "pencil", Size::Small, look.color(token::FG_MUTED))),
                    )
                    .child(labelled(
                        div()
                            .id(SharedString::from(format!("remove-note:{note}")))
                            .flex_none()
                            .p(look.length(token::SPACE_1))
                            .rounded(look.length(token::RADIUS_SM))
                            .cursor_pointer()
                            .tab_index(0)
                            .hover(|style| style.bg(look.color(token::BG_HOVER)))
                            .focus_visible(|style| style.border_1().border_color(look.color(token::ACCENT)))
                            .tooltip(tip("Remove note"))
                            .child(glyph(look, "x", Size::Small, look.color(token::FG_MUTED)))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                cx.stop_propagation();
                                this.close_box(ix, &gone, cx);
                            })),
                        Role::Button,
                        format!("Remove note on {on}"),
                    ))
            });
        let body = div()
            .id(group.clone())
            .group(group)
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_1))
            .mt(look.length(token::SPACE_2))
            .px(look.length(token::SPACE_3))
            .py(look.length(token::SPACE_2))
            .rounded(look.length(token::RADIUS_MD))
            .border(look.length(token::BORDER_WIDTH))
            .border_l(look.length(token::BORDER_WIDTH) * 2.)
            .border_color(look.color(token::YOU_EDGE))
            .bg(look.color(token::YOU).opacity(0.14))
            .child(header)
            .child(labelled(
                div()
                    .id(SharedString::from(format!("note-said:{note}")))
                    .text_size(look.length(token::TEXT_SM))
                    .line_height(look.length(token::LEADING_SM))
                    .text_color(look.color(token::FG_DEFAULT))
                    .child(SharedString::from(said.to_string())),
                Role::Label,
                said.to_string(),
            ))
            .child(labelled(
                div()
                    .id(SharedString::from(format!("note-goes:{note}")))
                    .text_size(look.length(token::TEXT_2XS))
                    .line_height(look.length(token::LEADING_2XS))
                    .text_color(look.color(token::FG_MUTED))
                    .child(SENT_WITH),
                Role::Label,
                SENT_WITH,
            ));
        if !open {
            return body.into_any_element();
        }
        labelled(
            body.cursor_pointer()
                .tab_index(0)
                .hover(|style| style.border_color(look.color(token::YOU)))
                .focus_visible(|style| style.border_color(look.color(token::ACCENT)))
                .on_click(cx.listener(move |this, _, window, cx| this.edit_note(ix, &edited, window, cx))),
            Role::Button,
            format!("Edit note on {on}"),
        )
        .into_any_element()
    }

    /// Under its lane or the head: the note's box while it is typed, its words quoted once saved, and where they go.
    fn note_below(&self, ix: usize, note: &str, on: &str, open: bool, look: &Look, cx: &Context<Self>) -> Option<AnyElement> {
        if open && self.note_open(note) {
            let blank = self.typed_in(note, Mode::Note, cx).trim().is_empty();
            let (saved, gone) = (note.to_string(), note.to_string());
            let row = div()
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .child(labelled(
                    button(look, SharedString::from(format!("Save note:{note}")), "Save note", Tone::Primary, false, None, !blank)
                        .when(!blank, |b| b.on_click(cx.listener(move |this, _, _, cx| this.save_note(ix, &saved, cx)))),
                    Role::Button,
                    format!("Save note on {on}"),
                ))
                .child(labelled(
                    button(look, SharedString::from(format!("Remove note:{note}")), "Remove note", Tone::Ghost, false, None, true)
                        .on_click(cx.listener(move |this, _, _, cx| this.close_box(ix, &gone, cx))),
                    Role::Button,
                    format!("Remove note on {on}"),
                ));
            // Once, as it is first measured: a box opened low in the thread is brought up clear of the composer.
            let reveal = self.shown.as_ref().and_then(|id| self.pages.get(id)).and_then(|page| {
                let flag = page.cards.reveal.get(note).filter(|flag| flag.get())?.clone();
                let list = page.list.clone();
                Some(
                    gpui::canvas(
                        move |bounds, window, cx| {
                            if !flag.get() {
                                return;
                            }
                            // The list is mid-layout here; it is read once this frame is done. A
                            // measuring pass lays the box out away from the thread, and is passed over.
                            let (list, flag) = (list.clone(), flag.clone());
                            window.defer(cx, move |window, _| {
                                let view = list.viewport_bounds();
                                if !flag.get() || bounds.top() < view.top() || bounds.top() > view.bottom() {
                                    return;
                                }
                                flag.set(false);
                                let line = view.top() + view.size.height * 0.6;
                                if bounds.bottom() > line {
                                    list.scroll_by(bounds.bottom() - line);
                                    window.refresh();
                                }
                            });
                        },
                        |_, _, _, _| {},
                    )
                    .absolute()
                    .top_0()
                    .left_0()
                    .size_full(),
                )
            });
            return Some(
                div()
                    .relative()
                    .flex()
                    .flex_col()
                    .gap(look.length(token::SPACE_2))
                    .pt(look.length(token::SPACE_1))
                    .children(self.box_of(note, Mode::Note, format!("Note for the agent on {on}"), look, cx))
                    .child(row)
                    .children(reveal)
                    .into_any_element(),
            );
        }
        let said = self.typed_in(note, Mode::Note, cx);
        let said = said.trim();
        (!said.is_empty()).then(|| self.saved_note(ix, note, on, said, open, look, cx))
    }

    /// The eyebrow of a workflow draft whose card draws: `New workflow` or `Workflow change`. `None`
    /// keeps the draft card's own eyebrow and title, as the text draws.
    pub(super) fn workflow_proposal(&self, change: Option<i64>, text: &str, cx: &gpui::App) -> Option<&'static str> {
        card_of(change.and_then(|change| self.workflow_change(change, cx)), text).map(|card| card.proposal.eyebrow())
    }

    /// The draft card's body for a proposed workflow whose steps Fleet read. `None` draws the text.
    pub(super) fn workflow_card(&self, ix: usize, key: &str, change: Option<i64>, text: &str, look: &Look, cx: &Context<Self>) -> Option<AnyElement> {
        let card = card_of(change.and_then(|change| self.workflow_change(change, cx)), text)?;
        let open = self.workflow_draft(key, cx).is_some_and(|(_, open)| open);
        let whole = note_key(key, None);
        let named = div()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_3))
            .child(glyph(look, "workflow", Size::Mark, look.color(token::KIND_STEP)))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .text_size(look.length(token::TEXT_BASE))
                    .line_height(look.length(token::LEADING_BASE))
                    .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                    .text_color(look.color(token::FG_DEFAULT))
                    .child(SharedString::from(card.name.clone())),
            )
            .child(
                div()
                    .flex_none()
                    .font_family(mono(look))
                    .text_size(look.length(token::TEXT_2XS))
                    .line_height(look.length(token::LEADING_2XS))
                    .text_color(look.color(token::FG_MUTED))
                    .child(SharedString::from(card.id.clone())),
            )
            .children(self.note_button(ix, &whole, "the whole workflow", open, look, cx));
        let approves = div()
            .id(SharedString::from(format!("approves:{key}")))
            .text_size(look.length(token::TEXT_SM))
            .line_height(look.length(token::LEADING_SM))
            .text_color(look.color(token::FG_MUTED))
            .child(SharedString::from(card.approves.clone()));
        let head = div()
            .id(SharedString::from(format!("head:{key}")))
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_1))
            .px(look.length(token::SPACE_3))
            .py(look.length(token::SPACE_2))
            .rounded(look.length(token::RADIUS_MD))
            .bg(look.color(token::BG_OVERLAY))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_DEFAULT))
            .child(named)
            .child(labelled(approves, Role::Label, card.approves.clone()));
        let for_requests = card.for_requests.clone().map(|asked| {
            div()
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_1))
                .text_size(look.length(token::TEXT_SM))
                .line_height(look.length(token::LEADING_SM))
                .text_color(look.color(token::FG_DEFAULT))
                .child(super::rows::eyebrow(look, "For requests").text_color(look.color(token::FG_MUTED)))
                .child(SharedString::from(asked))
        });
        let last = card.lanes.len().saturating_sub(1);
        let lanes = card.lanes.iter().enumerate().map(|(at, lane)| self.lane(ix, key, at, at == last, open, lane, look, cx));
        Some(
            div()
                .flex()
                .flex_col()
                .min_w_0()
                .gap(look.length(token::SPACE_3))
                .child(labelled(head, Role::Heading, card.name.clone()))
                .children(for_requests)
                .children(self.note_below(ix, &whole, "the whole workflow", open, look, cx))
                .child(div().flex().flex_col().min_w_0().children(lanes))
                .into_any_element(),
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn lane(&self, ix: usize, key: &str, at: usize, last: bool, open: bool, lane: &Lane, look: &Look, cx: &Context<Self>) -> AnyElement {
        let note = note_key(key, Some(&lane.id));
        let kind = if lane.delivers { token::KIND_DELIVERY } else { token::KIND_STEP };
        let told_key = format!("{key}:step:{}", lane.id);
        let told = self.open_in_card(&told_key);
        // The rail: the step's number in a ring of its kind, and a hairline down to the next.
        let rail = div()
            .flex()
            .flex_col()
            .items_center()
            .flex_none()
            .w(look.length(token::H_CONTROL))
            .child(
                div()
                    .flex()
                    .flex_none()
                    .items_center()
                    .justify_center()
                    .size(look.length(token::H_CONTROL))
                    .rounded_full()
                    .border(look.length(token::BORDER_WIDTH))
                    .border_color(look.color(kind))
                    .bg(look.color(token::BG_RAISED))
                    .text_color(look.color(kind))
                    .font_family(mono(look))
                    .text_size(look.length(token::TEXT_XS))
                    .line_height(look.length(token::LEADING_XS))
                    .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                    .child(SharedString::from((at + 1).to_string())),
            )
            .when(!last, |rail| rail.child(div().flex_1().w(look.length(token::BORDER_WIDTH) * 2.).bg(look.color(token::BORDER_STRONG))));
        let line = |said: SharedString, ink: Token| {
            div()
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .text_size(look.length(token::TEXT_XS))
                .line_height(look.length(token::LEADING_XS))
                .text_color(look.color(ink))
                .child(said)
        };
        let ends = lane.ends.clone().map(|ends| {
            let row = div()
                .id(SharedString::from(format!("ends:{key}:{}", lane.id)))
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .text_size(look.length(token::TEXT_XS))
                .line_height(look.length(token::LEADING_XS))
                .text_color(look.color(token::FG_DEFAULT));
            let named = ends.clone();
            let row = match lane.gate {
                Some(gate) => row
                    .child(
                        labelled(
                            div()
                                .id(SharedString::from(format!("gate:{key}:{}", lane.id)))
                                .flex_none()
                                .child(glyph(look, gate.glyph, Size::Small, look.color(gate.ink)))
                                .tooltip(tip(gate.named)),
                            Role::Image,
                            gate.named,
                        ),
                    )
                    .child(SharedString::from(ends)),
                // A step no gate ends keeps the glyph's column, so every lane's words start at one edge.
                None => row.child(div().flex_none().size(look.length(token::SPACE_3))).child(SharedString::from(ends)),
            };
            labelled(row, Role::Label, named)
        });
        let facts = lane.facts.iter().map(|fact| {
            let row = div()
                .id(SharedString::from(format!("fact:{key}:{}:{}", lane.id, fact.glyph)))
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .text_size(look.length(token::TEXT_XS))
                .line_height(look.length(token::LEADING_XS))
                .text_color(look.color(token::FG_MUTED))
                .child(glyph(look, fact.glyph, Size::Small, look.color(token::KIND_DELIVERY)))
                .child(fact.said);
            labelled(row, Role::Label, fact.said)
        });
        let toggle = lane.instructions.as_ref().map(|instructions| {
            let name = format!("What the agent is told on {}", lane.label);
            let flip = told_key.clone();
            div()
                .flex()
                .flex_col()
                .items_start()
                .gap(look.length(token::SPACE_2))
                .child(labelled(
                    // The chevron sits in the column the gate glyphs use, so the words start where the lane's lines do.
                    div()
                        .id(SharedString::from(format!("told:{told_key}")))
                        .flex()
                        .items_center()
                        .gap(look.length(token::SPACE_2))
                        .h(look.length(token::H_CONTROL))
                        .px(look.length(token::SPACE_3))
                        .ml(-look.length(token::SPACE_3))
                        .rounded(look.length(token::RADIUS_MD))
                        .text_size(look.length(token::TEXT_SM))
                        .line_height(look.length(token::LEADING_SM))
                        .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
                        .text_color(look.color(token::FG_MUTED))
                        .cursor_pointer()
                        .hover(|style| style.bg(look.color(token::BG_HOVER)).text_color(look.color(token::FG_DEFAULT)))
                        .child(self.told_chevron(&told_key, told, look))
                        .child("What the agent is told")
                        .on_click(cx.listener(move |this, _, _, cx| {
                            if let Some(page) = this.shown.clone().and_then(|id| this.pages.get_mut(&id)) {
                                page.cards.opened.insert(format!("{flip}:turned"));
                            }
                            this.flip_in_card(ix, &flip, cx)
                        })),
                    Role::Button,
                    name,
                ))
                .children(told.then(|| {
                    let instructions_name = format!("Instructions for {}", lane.label);
                    labelled(div()
                        .id(SharedString::from(format!("instructions:{told_key}")))
                        .w_full()
                        .min_w_0()
                        .px(look.length(token::SPACE_3))
                        .py(look.length(token::SPACE_2))
                        .rounded(look.length(token::RADIUS_SM))
                        .bg(look.color(token::BG_BASE))
                        .child(self.prose(format!("{told_key}:text"), instructions, look)), Role::Group, instructions_name)
                }))
        });
        let body = div()
            .flex()
            .flex_col()
            .flex_1()
            .min_w_0()
            .gap(look.length(token::SPACE_1))
            .mb(look.length(token::SPACE_3))
            .px(look.length(token::SPACE_3))
            .py(look.length(token::SPACE_2))
            .rounded(look.length(token::RADIUS_MD))
            .bg(look.color(token::BG_SUNKEN))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_SUBTLE))
            .child(
                // The label, and the note control at the right edge, in one column down every lane.
                div()
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_2))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .text_size(look.length(token::TEXT_SM))
                            .line_height(look.length(token::LEADING_SM))
                            .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                            .text_color(look.color(token::FG_DEFAULT))
                            .child(SharedString::from(lane.label.clone())),
                    )
                    .children(self.note_button(ix, &note, &lane.label, open, look, cx)),
            )
            .children(lane.about.clone().map(|about| line(SharedString::from(about), token::FG_MUTED)))
            .children(ends)
            .children(facts)
            .children(toggle)
            .children(self.note_below(ix, &note, &lane.label, open, look, cx));
        labelled(
            div().id(SharedString::from(format!("lane:{key}:{}", lane.id))).flex().gap(look.length(token::SPACE_3)).min_w_0().child(rail).child(body),
            Role::Group,
            lane.label.clone(),
        )
        .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ipc::{ChangeState, DraftHow, DraftSettled, Instant, StepId, WorkflowId};

    fn step(id: &str) -> ChangeStep {
        ChangeStep {
            id: StepId::carried(id),
            label: format!("Step {id}"),
            instructions: None,
            ends_when: None,
            ends_any: Vec::new(),
            passed_over_when: None,
            about: None,
            gate: None,
            asks_delivery: false,
            delivers: false,
        }
    }

    fn change(steps: Option<Vec<ChangeStep>>) -> WorkflowChange {
        WorkflowChange {
            id: 7,
            workflow_id: WorkflowId::carried("annotations"),
            source: String::from("repository"),
            step_id: None,
            origin: ChangeOrigin::New,
            before: String::new(),
            after: Some(String::from("{\"name\":\"annotations\"}")),
            state: ChangeState::Ready,
            reason: None,
            missing: Vec::new(),
            before_steps: None,
            after_steps: steps,
            after_name: Some(String::from("Work the notes")),
            after_for_requests: Some(String::from("Notes he left on Bridge")),
            notes: None,
            at: Instant::carried("2026-11-12T09:00:00.000Z"),
        }
    }

    const TEXT: &str = "{\"name\":\"annotations\"}";

    #[test]
    fn workflow_card_reads_each_step_into_a_lane() {
        let mut read = step("read");
        read.about = Some(String::from("Reads each open note."));
        read.ends_when = Some(String::from("you approve"));
        read.gate = Some(String::from("you_approved"));
        read.instructions = Some(String::from("Open every note."));
        let card = card_of(Some(&change(Some(vec![read, step("fix")]))), TEXT).expect("a card");
        assert_eq!(card.name, "Work the notes");
        assert_eq!(card.id, "annotations");
        assert_eq!(card.proposal, Proposal::New);
        assert_eq!(card.approves, "Approve adds it to this repository's workflows.");
        assert_eq!(card.for_requests.as_deref(), Some("Notes he left on Bridge"));
        assert_eq!(card.lanes.len(), 2);
        let first = &card.lanes[0];
        assert_eq!(first.label, "Step read");
        assert_eq!(first.about.as_deref(), Some("Reads each open note."));
        assert_eq!(first.ends.as_deref(), Some("Ends when you approve"));
        assert_eq!(first.gate.map(|gate| (gate.glyph, gate.named)), Some(("circle-check", "Your Approve")));
        assert_eq!(first.instructions.as_deref(), Some("Open every note."));
        assert_eq!(card.lanes[1].ends, None);
        assert_eq!(card.lanes[1].gate, None);
    }

    #[test]
    fn workflow_card_falls_back_to_the_text_without_steps() {
        assert_eq!(card_of(None, TEXT), None);
        assert_eq!(card_of(Some(&change(None)), TEXT), None);
        assert_eq!(card_of(Some(&change(Some(Vec::new()))), TEXT), None);
        // A send back redrafted it since Fleet was read: the row's text stands.
        assert_eq!(card_of(Some(&change(Some(vec![step("read")]))), "{\"name\":\"other\"}"), None);
    }

    #[test]
    fn workflow_card_says_delivery_as_facts_in_the_lane() {
        let mut ship = step("ship");
        ship.asks_delivery = true;
        ship.delivers = true;
        let lane = lane(&ship);
        assert_eq!(lane.facts.iter().map(|fact| fact.said).collect::<Vec<_>>(), vec!["Asks how to deliver", "Delivers"]);
        assert!(lane.delivers);
        assert!(super::lane(&step("read")).facts.is_empty());
    }

    #[test]
    fn workflow_card_reads_only_open_workflow_drafts() {
        let draft = |id: &str, title: &str, change: Option<i64>, settled: Option<DraftSettled>| ThreadItem::Row {
            row: SessionRow::Draft {
                id: id.to_string(),
                at: Instant::carried("2026-11-12T09:00:00.000Z"),
                title: title.to_string(),
                text: TEXT.to_string(),
                before: None,
                settled,
                change,
            },
        };
        let items = vec![
            draft("a", "Workflow annotations", Some(7), None),
            draft("b", "Rule tidy", Some(8), None),
            draft("c", "README", None, None),
            draft("d", "Workflow done", Some(9), Some(DraftSettled { how: DraftHow::Approved, words: None, at: Instant::carried("2026-11-12T09:01:00.000Z") })),
        ];
        assert_eq!(wanted(&items), vec![(7, String::from("annotations"), TEXT.to_string())]);
    }

    #[test]
    fn workflow_card_says_whether_it_adds_or_changes_a_workflow() {
        let new = change(Some(vec![step("read")]));
        assert_eq!(proposal(&new), Proposal::New);
        assert_eq!(Proposal::New.eyebrow(), "New workflow");
        let mut edit = change(Some(vec![step("read")]));
        edit.origin = ChangeOrigin::Job;
        edit.before = String::from("{\"name\":\"annotations\",\"steps\":[]}");
        assert_eq!(proposal(&edit), Proposal::Change);
        assert_eq!(Proposal::Change.eyebrow(), "Workflow change");
        // A change he asked for that has nothing before it adds a workflow too.
        let mut asked = change(Some(vec![step("read")]));
        asked.origin = ChangeOrigin::Asked { text: String::from("a workflow for notes") };
        assert_eq!(proposal(&asked), Proposal::New);
    }

    #[test]
    fn workflow_card_says_what_approve_does() {
        assert_eq!(approves(Proposal::New, "repository", "annotations"), "Approve adds it to this repository's workflows.");
        assert_eq!(approves(Proposal::New, "kit", "annotations"), "Approve adds it to the workflows of every repository on this Mac.");
        assert_eq!(approves(Proposal::Change, "repository", "annotations"), "Approve replaces the steps of annotations.");
        assert_eq!(approves(Proposal::Change, "kit", "annotations"), "Approve replaces the steps of annotations on every repository on this Mac.");
    }

    #[test]
    fn workflow_card_joins_notes_led_by_their_step() {
        let card = card_of(Some(&change(Some(vec![step("quiz"), step("walk"), step("ship")]))), TEXT).expect("a card");
        let notes = |lane: Option<&str>| match lane {
            None => String::from("Name it for the notes it works"),
            Some("quiz") => String::from("  Ask one question at a time\n"),
            Some("ship") => String::from("Open the pull request as a draft"),
            Some(_) => String::new(),
        };
        assert_eq!(
            notes_sent(&card, notes).as_deref(),
            Some("Whole workflow: Name it for the notes it works\nStep quiz (quiz): Ask one question at a time\nStep ship (ship): Open the pull request as a draft")
        );
    }

    #[test]
    fn workflow_card_offers_send_back_only_with_a_note() {
        let card = card_of(Some(&change(Some(vec![step("quiz")]))), TEXT).expect("a card");
        assert_eq!(notes_sent(&card, |_| String::new()), None);
        assert_eq!(notes_sent(&card, |_| String::from("  \n ")), None);
        assert!(notes_sent(&card, |lane| if lane == Some("quiz") { String::from("Shorter") } else { String::new() }).is_some());
    }
}
