//! One ask as one line, wherever something points to it: the mark of its kind in its hue, the question itself and its age.
//! An ask is answered on its card in the thread or in the review sheet (25 Nov, reversing one place of 19 Nov). A line in
//! Needs you brings the thread to the first card for the Job's first ask; only that ask stands as a card, so any other
//! is a line in the thread and opens the sheet on it.
//! `.claude/decisions/2026-11-19-one-ask-one-item-one-place.md`.

use gpui::{div, prelude::*, AnyElement, Context, Role, SharedString, Window};
use ipc::{AnswerHow, AnsweredAsk, SessionRow, ThreadHead, ThreadItem};

use super::acts::Answer;
use super::reader_sheet::artifact_in;
use super::review::{Ask, Item};
use super::rows::{glyph, tip, Size};
use super::JobView;
use crate::labels::labelled;
use crate::look::{token, Look};

/// The item id of question `at` of the card `call`, as Fleet names it: `ask:<call>`, then `ask:<call>#<at>`.
pub(super) fn ask_item_id(call: &str, at: usize) -> String {
    match at {
        0 => format!("ask:{call}"),
        at => format!("ask:{call}#{at}"),
    }
}

/// Whether item `item` is a question of the card `call`.
pub(super) fn of_card(item: &str, call: &str) -> bool {
    item.strip_prefix("ask:").is_some_and(|rest| rest == call || rest.strip_prefix(call).is_some_and(|tail| tail.starts_with('#')))
}

/// Where a pointer stands: its row is filled in Needs you and For you, and quiet in the thread.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Stands {
    List,
    Thread,
}

impl JobView {
    /// The sheet's item for Job `job`'s waiting item `item`: the item itself, else the artifact of parts it names while that
    /// still asks. None where the sheet does not hold it.
    pub(super) fn sheet_item(&self, job: &str, item: &str, cx: &gpui::App) -> Option<Item> {
        if let Some(one) = self.review_items(job, cx).into_iter().find(|one| one.id == item) {
            return Some(one);
        }
        let named = self.row_artifact(job, item, cx)?;
        self.artifact_items(job, None).into_iter().find(|one| artifact_in(one) == Some(named.as_str()))
    }

    /// The sheet's items for the open card `call` of Job `job`, each question that still waits.
    pub(super) fn card_sheet_items(&self, job: &str, call: &str, cx: &gpui::App) -> Vec<Item> {
        self.review_items(job, cx).into_iter().filter(|one| of_card(&one.id, call)).collect()
    }

    /// The sheet's item that asks about artifact `artifact` of Job `job`: a waiting item naming it, its page, its sketch or draft
    /// by title, or the artifact of parts itself while it asks. None where nothing asks about it: it only informs.
    pub(super) fn asking_item(&self, job: &str, artifact: &str, cx: &gpui::App) -> Option<String> {
        let named: Vec<String> = self.record(job, cx).map(|record| record.waiting_for.iter().filter(|one| one.artifact.as_deref() == Some(artifact)).map(|one| one.id.clone()).collect()).unwrap_or_default();
        let title = self.artifact_of(job, artifact).map(|one| one.title.clone());
        let titled = |item: &Item| matches!(item.kind, super::review::Kind::Sketch | super::review::Kind::Draft) && title.as_deref() == Some(item.name.as_str());
        self.review_items(job, cx)
            .into_iter()
            .find(|item| named.contains(&item.id) || super::review::page_of_item(&item.id) == Some(artifact) || artifact_in(item) == Some(artifact) || titled(item))
            .map(|item| item.id)
            .or_else(|| self.artifact_items(job, None).into_iter().find(|item| artifact_in(item) == Some(artifact)).map(|item| item.id))
    }

    /// A press on a pointer: Needs you folds and the sheet opens on the item.
    pub(super) fn open_ask(&mut self, item: &str, window: &mut Window, cx: &mut Context<Self>) {
        self.set_needs(false, None, window, cx);
        self.open_review_on(item, window, cx);
    }

    /// One ask as one line: its kind's mark in its hue, the question, and its age. Named as the question reads.
    pub(super) fn ask_pointer(&self, key: &str, item: &Item, stands: Stands, look: &Look, cx: &Context<Self>) -> AnyElement {
        let ask = self.ask_of(item);
        pointer(key, &item.id, ask, &item.name, &item.since, stands, look, cx)
    }

    /// The sheet's item of Job `job` named `key`, among what waits on him and the artifacts of parts that still ask.
    pub(super) fn held_item(&self, job: &str, key: &str, cx: &gpui::App) -> Option<Item> {
        // The plan's waiting item has an id of its own; the plan put to him is one at a time.
        let plan = key.starts_with("plan:");
        self.review_items(job, cx).into_iter().chain(self.artifact_items(job, None)).find(|one| same_item(&one.id, key) || (plan && one.kind == super::review::Kind::Plan))
    }

    /// A press on a line in Needs you: the panel folds and the thread comes to the ask's card, where it is answered. An ask the
    /// thread holds no card for opens the sheet on it.
    pub(super) fn go_to_ask(&mut self, item: &str, window: &mut Window, cx: &mut Context<Self>) {
        let Some(job) = self.shown.clone() else { return };
        let place = self.held_item(&job, item, cx).and_then(|held| self.ask_place(&job, &held, cx));
        let Some((at, key)) = place else {
            self.open_ask(item, window, cx);
            return;
        };
        self.set_needs(false, None, window, cx);
        self.aim(&key);
        if let Some(page) = self.pages.get_mut(&job) {
            page.list.scroll_to(gpui::ListOffset { item_ix: page.entry(at), offset_in_item: gpui::px(0.) });
        }
        cx.notify();
    }

    /// Where the thread draws the card that answers `item`, and the card's key: the first card, where it is the Job's first
    /// ask. Only the first open ask stands as a card; any other is a line in the thread, so the sheet takes it.
    pub(super) fn ask_place(&self, job: &str, item: &Item, cx: &gpui::App) -> Option<(usize, String)> {
        let page = self.pages.get(job)?;
        let first = self.first_ask(job, cx).filter(|first| same_item(&first.id, &item.id) || artifact_in(first).is_some_and(|artifact| artifact_in(item) == Some(artifact)))?;
        let at = page.extras.iter().position(|one| matches!(one, super::Extra::First(_)))?;
        Some((page.items + at, super::first_card::first_key(&first)))
    }

    /// The thread card of a page or a draft that still asks, as one line that opens the sheet on it: only the first open ask
    /// stands as a card. Once settled, here or in Fleet, it draws as its card again.
    pub(super) fn asking_line(&self, item: &ThreadItem, page: &super::Page, look: &Look, cx: &Context<Self>) -> Option<AnyElement> {
        if !matches!(item, ThreadItem::Document(_) | ThreadItem::Row { row: SessionRow::Draft { settled: None, .. } }) {
            return None;
        }
        if page.cards.answers.contains_key(&card_key(item)?) {
            return None;
        }
        let job = self.shown.as_deref()?;
        let key = card_item(item, &self.thread(job, cx)?.items)?;
        let ask = self.open_ask_of(job, &key, cx)?;
        Some(self.ask_pointer(&format!("thread:{key}"), &ask, Stands::Thread, look, cx))
    }

    /// The artifact of parts in the thread, its takes among them, as one line while it asks.
    /// `place` is where it stands, the thread or the artifact's own tab.
    pub(super) fn parts_pointer(&self, place: &str, artifact: &str, look: &Look, cx: &Context<Self>) -> Option<AnyElement> {
        let job = self.shown.clone()?;
        let item = self.asking_item(&job, artifact, cx)?;
        let held = self.held_item(&job, &item, cx)?;
        Some(self.ask_pointer(&format!("{place}:{artifact}"), &held, Stands::Thread, look, cx))
    }

    /// A verdict he gave in the sheet stands at once on the thread cards of its item, as a press on the card would, so they
    /// fold to it. None takes it back, where Fleet refused it.
    pub(super) fn sheet_settles(&mut self, job: &str, item: &str, answer: Option<Answer>, cx: &mut Context<Self>) {
        let keys = self.thread(job, cx).map(|held| card_keys(item, &held.head, &held.items)).unwrap_or_default();
        let Some(page) = self.pages.get_mut(&SharedString::from(job.to_string())) else { return };
        if keys.is_empty() {
            return;
        }
        for key in keys {
            match &answer {
                Some(answer) => page.cards.answers.insert(key, answer.clone()),
                None => page.cards.answers.remove(&key),
            };
        }
        page.list.remeasure();
        cx.notify();
    }

    /// What he sent from the sheet, in the thread, drawn as his own act is (his pick, 13 Nov): a gold band with how he
    /// answered and when, the ask quoted, his pick or words as the main line, what he attached and tagged, and Open where it
    /// was asked on an artifact. No instruction to the agent and no path.
    pub(super) fn answer_card(&self, answer: &AnsweredAsk, look: &Look, cx: &Context<Self>) -> AnyElement {
        let key = answer_key(answer);
        let (icon, label) = how_mark(answer.how);
        let question = answer.question.trim().to_string();
        let you = look.color(token::YOU);
        let edge = look.color(token::YOU_EDGE);
        let band = div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap_x(look.length(token::SPACE_2))
            .gap_y(look.length(token::SPACE_1))
            .px(look.length(token::SPACE_3))
            .py(look.length(token::SPACE_1) * 1.5)
            .bg(you.opacity(0.1))
            .border_b(look.length(token::BORDER_WIDTH))
            .border_color(edge)
            .text_size(look.length(token::TEXT_XS))
            .line_height(look.length(token::LEADING_XS))
            .font_weight(gpui::FontWeight(look.number(token::WEIGHT_HEADING)))
            .text_color(you)
            .child(glyph(look, icon, Size::Small, you))
            .child(div().flex_none().whitespace_nowrap().child(label))
            .child(div().flex_1())
            .child(div().flex_none().whitespace_nowrap().child(self.stamp(&answer.at, look, chrono::Local::now(), true, cx)));
        let quote = (!question.is_empty()).then(|| {
            labelled(
                div()
                    .id(SharedString::from(format!("answer-quote:{key}")))
                    .mx(look.length(token::SPACE_3))
                    .mt(look.length(token::SPACE_2) + look.length(token::SPACE_1) * 0.5)
                    .pl(look.length(token::SPACE_2) + look.length(token::SPACE_1) * 0.5)
                    .border_l(look.length(token::BORDER_WIDTH) * 2.)
                    .border_color(look.color(token::BORDER_DEFAULT))
                    .text_size(look.length(token::TEXT_SM))
                    .line_height(look.length(token::LEADING_SM))
                    .text_color(look.color(token::FG_MUTED))
                    .child(SharedString::from(question.clone())),
                Role::Group,
                "Question",
            )
        });
        let main = answer.pick.clone().filter(|pick| !pick.trim().is_empty());
        let words: Vec<String> = answer.words.clone().into_iter().chain(answer.notes.values().cloned()).filter(|line| !line.trim().is_empty()).collect();
        let main = main.map(|said| {
            div()
                .px(look.length(token::SPACE_3))
                .pt(look.length(token::SPACE_2))
                .text_size(look.length(token::TEXT_BASE))
                .line_height(look.length(token::LEADING_BASE))
                .font_weight(gpui::FontWeight(look.number(token::WEIGHT_MEDIUM)))
                .text_color(look.color(token::FG_DEFAULT))
                .child(SharedString::from(said))
        });
        // His words keep the formatting he gave them, and his tags stand where he wrote them, as his message's do.
        let rest = words.into_iter().enumerate().map(|(at, line)| {
            div()
                .px(look.length(token::SPACE_3))
                .pt(look.length(token::SPACE_1))
                .child(self.prose_tagged(format!("answer-words:{key}:{at}"), &line, token::FG_DEFAULT, &answer.extras.mentions, look, cx))
        });
        let chips: Vec<AnyElement> = answer
            .extras
            .attachments
            .iter()
            .enumerate()
            .map(|(at, upload)| extra_chip(look, &format!("upload:{key}:{at}"), if upload.media_type.starts_with("image/") { "image" } else { "paperclip" }, &upload.name, "Attached"))
            .chain(answer.extras.mentions.iter().enumerate().map(|(at, tag)| extra_chip(look, &format!("tag:{key}:{at}"), "tags", &tag.title, "Tagged")))
            .collect();
        let open = answer.artifact.clone().filter(|artifact| self.shown.as_ref().is_some_and(|job| self.artifact_of(job, artifact).is_some())).map(|artifact| {
            labelled(
                super::acts::button(look, SharedString::from(format!("answer-open:{key}")), "Open", super::acts::Tone::Ghost, false, None, true)
                    .h(look.length(token::H_CONTROL_SM))
                    .px(look.length(token::SPACE_2))
                    .text_size(look.length(token::TEXT_XS))
                    .line_height(look.length(token::LEADING_XS))
                    .tab_index(0)
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.open_named(&artifact, window, cx);
                    })),
                Role::Button,
                "Open",
            )
        });
        let foot = (!chips.is_empty() || open.is_some()).then(|| {
            div()
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .px(look.length(token::SPACE_3))
                .pt(look.length(token::SPACE_2))
                .child(div().flex_1().min_w_0().flex().flex_wrap().gap(look.length(token::SPACE_1)).children(chips))
                .children(open)
        });
        div()
            .flex()
            .justify_end()
            .child(labelled(
                div()
                    .id(SharedString::from(format!("answer:{key}")))
                    .flex()
                    .flex_col()
                    .flex_none()
                    .max_w(gpui::relative(0.8))
                    .pb(look.length(token::SPACE_3))
                    .overflow_hidden()
                    .rounded(look.length(token::RADIUS_LG))
                    .border(look.length(token::BORDER_WIDTH))
                    .border_color(edge)
                    .bg(look.color(token::BG_RAISED))
                    .child(band)
                    .children(quote)
                    .children(main)
                    .children(rest)
                    .children(foot),
                Role::Region,
                format!("{label}: {question}"),
            ))
            .into_any_element()
    }
}

/// How he answered, as his act's band says it: its glyph and its words.
pub(super) fn how_mark(how: AnswerHow) -> (&'static str, &'static str) {
    match how {
        AnswerHow::Picked => ("reply", "You picked"),
        AnswerHow::Approved => ("check", "You approved"),
        AnswerHow::Answered => ("reply", "You answered"),
        AnswerHow::AskedForChanges => ("pencil-line", "You asked for changes"),
        AnswerHow::Dismissed => ("circle-minus", "You dismissed"),
    }
}

/// One picture, file or tag he sent with an answer, by its name.
fn extra_chip(look: &Look, key: &str, icon: &'static str, name: &str, said: &'static str) -> AnyElement {
    let chip = div()
        .id(SharedString::from(format!("answer-extra:{key}")))
        .flex()
        .items_center()
        .gap(look.length(token::SPACE_1))
        .h(look.length(token::H_CONTROL_SM))
        .px(look.length(token::SPACE_2))
        .rounded(look.length(token::RADIUS_SM))
        .bg(look.color(token::BG_SUNKEN))
        .text_size(look.length(token::TEXT_XS))
        .line_height(look.length(token::LEADING_XS))
        .text_color(look.color(token::FG_DEFAULT))
        .tooltip(tip(said))
        .child(glyph(look, icon, Size::Small, look.color(token::FG_MUTED)))
        .child(div().max_w(look.length(token::SPACE_12) * 4.).truncate().child(SharedString::from(name.to_string())));
    labelled(chip, Role::Group, format!("{said}: {name}")).into_any_element()
}

/// An answer's key in the thread: its item and when he gave it, as he may answer one item again after a revision.
pub(super) fn answer_key(answer: &AnsweredAsk) -> String {
    format!("{}@{}", answer.item, answer.at.as_str())
}

/// Whether two item ids name one ask: a page is one ask whether it is to approve or to read.
pub(super) fn same_item(a: &str, b: &str) -> bool {
    a == b || matches!((super::review::page_of_item(a), super::review::page_of_item(b)), (Some(one), Some(other)) if one == other)
}

/// The item a thread card stands for, as the sheet and his answers name it. Only a page's newest card and a sketch's
/// newest revision stand for it.
pub(super) fn card_item(item: &ThreadItem, thread: &[ThreadItem]) -> Option<String> {
    match item {
        ThreadItem::Sketch(card) if !card.replaced => Some(card.sketch.clone()),
        ThreadItem::Document(card) if super::review::document_card(thread, &card.page).is_some_and(|newest| newest.id == card.id) => Some(format!("document:{}", card.page)),
        ThreadItem::Plan(plan) => Some(format!("plan:{}", plan.version)),
        ThreadItem::Row { row: SessionRow::Draft { id, .. } } => Some(format!("draft:{id}")),
        _ => None,
    }
}

/// The key a thread card takes an answer under, for a card that asks.
fn card_key(item: &ThreadItem) -> Option<String> {
    match item {
        ThreadItem::Sketch(card) => Some(card.id.clone()),
        ThreadItem::Document(card) => Some(card.id.clone()),
        ThreadItem::Row { row: SessionRow::Draft { id, .. } } => Some(format!("draft:{id}")),
        ThreadItem::Plan(plan) => Some(plan.id.clone()),
        _ => None,
    }
}

/// The keys the thread cards of item `item` take an answer under: a page's newest card, a sketch's card and its pin over
/// the thread, a draft's row and a plan's card.
pub(super) fn card_keys(item: &str, head: &ThreadHead, thread: &[ThreadItem]) -> Vec<String> {
    let mut keys: Vec<String> = thread
        .iter()
        .filter(|one| card_item(one, thread).is_some_and(|key| same_item(&key, item)))
        .filter_map(card_key)
        .collect();
    let plan = head.plan.as_ref().filter(|plan| format!("plan:{}", plan.version) == item || (item.starts_with("plan:") && plan.state == ipc::PlanCardState::Waiting)).map(|plan| plan.id.clone());
    keys.extend(plan.filter(|id| !keys.contains(id)));
    keys.extend(head.sketch.as_ref().filter(|sketch| sketch.sketch == item).map(|sketch| format!("pin:{}", sketch.id)));
    keys
}

/// Whether the thread draws answer `answer` on the card it was asked on: a thread card, the card over the thread, or the
/// card of the artifact of parts. Any other, a question's and a plan's among them, stands at the time he gave it: a settled
/// card of questions is a call folded into its run, and a plan's card folds.
pub(super) fn answer_in_place(answer: &AnsweredAsk, head: &ThreadHead, thread: &[ThreadItem], parts: &[String]) -> bool {
    // A plan's card folds once answered, so his answer stands where he gave it, as a question's does.
    if answer.item.starts_with("ask:") || answer.item.starts_with("plan:") {
        return false;
    }
    answer.artifact.as_ref().is_some_and(|artifact| parts.contains(artifact))
        || head.sketch.as_ref().is_some_and(|sketch| answer.item == sketch.sketch)
        || thread.iter().any(|item| card_item(item, thread).is_some_and(|key| same_item(&key, &answer.item)))
}

/// The line itself, for an ask whose kind, question and time are known.
#[allow(clippy::too_many_arguments)]
pub(super) fn pointer(key: &str, item: &str, ask: Ask, question: &str, since: &str, stands: Stands, look: &Look, cx: &Context<JobView>) -> AnyElement {
    let age = super::rows::age(since, chrono::Local::now());
    let open = item.to_string();
    let question = super::dock::first_line(question).to_string();
    let tile = ask_tile(look, key, ask);
    let line = div()
        .id(SharedString::from(format!("ask-line:{key}")))
        .flex()
        .items_center()
        .gap(look.length(token::SPACE_2))
        .rounded(look.length(token::RADIUS_MD))
        .border(look.length(token::BORDER_WIDTH))
        .cursor_pointer()
        .text_size(look.length(token::TEXT_SM))
        .line_height(look.length(token::LEADING_SM))
        .text_color(look.color(token::FG_DEFAULT))
        .map(|line| match stands {
            Stands::List => line
                .px(look.length(token::SPACE_3))
                .py(look.length(token::SPACE_2))
                .border_color(look.color(token::BORDER_SUBTLE))
                .bg(look.color(token::BG_HOVER))
                .hover(|style| style.border_color(look.color(token::STEP_WAITING))),
            Stands::Thread => line
                .px(look.length(token::SPACE_2))
                .py(look.length(token::SPACE_1))
                .border_color(gpui::transparent_black())
                .hover(|style| style.bg(look.color(token::SURFACE_ROW_HOVER))),
        })
        .child(tile)
        .child(div().flex_1().min_w_0().truncate().child(SharedString::from(question.clone())))
        .children(age.map(|age| div().flex_none().text_size(look.length(token::TEXT_XS)).text_color(look.color(token::FG_MUTED)).child(age)))
        .child(glyph(look, "chevron-right", Size::Small, look.color(token::FG_MUTED)))
        .tab_index(0)
        .focus_visible(|style| style.border_color(look.color(token::ACCENT)))
        // Needs you brings the thread to the ask's card; a line in a card or a tab opens the sheet on it.
        .on_click(cx.listener(move |this, _, window, cx| match stands {
            Stands::List => this.go_to_ask(&open, window, cx),
            Stands::Thread => this.open_ask(&open, window, cx),
        }));
    labelled(line, Role::Button, question).into_any_element()
}

/// The mark of an ask's kind: its glyph in its hue on a tile of that hue, named in the tooltip.
pub(super) fn ask_tile(look: &Look, key: &str, ask: Ask) -> gpui::Stateful<gpui::Div> {
    let (icon, said, hue) = ask.mark();
    div()
        .id(SharedString::from(format!("ask-kind:{key}")))
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .size(look.length(token::LEADING_SM))
        .rounded(look.length(token::RADIUS_SM))
        .bg(look.color(hue).opacity(0.16))
        .tooltip(tip(said))
        .child(glyph(look, icon, Size::Mark, look.color(hue)))
}

#[cfg(test)]
mod tests {
    use super::super::parts_fold::{drawn, drawn_like, redraw, rooted_job, rooted_job_with};
    use super::*;
    use gpui::{size, px};

    const FIRST: &str = "Which colour does the stale chip take?";
    const SECOND: &str = "Where does the chip show how far behind it is?";

    fn bounds(cx: &mut gpui::VisualTestContext, role: Role, name: &str) -> gpui::Bounds<gpui::Pixels> {
        cx.update(|window, cx| cx.global::<crate::labels::LabelBook>().find(window.window_handle().window_id(), role, name).map(|one| one.bounds))
            .unwrap_or_else(|| panic!("{name} was not drawn"))
    }

    fn answer(item: &str, question: &str, at: &str) -> AnsweredAsk {
        AnsweredAsk {
            item: item.into(),
            question: question.into(),
            how: AnswerHow::Picked,
            words: Some("Amber, as the other waiting marks".into()),
            pick: Some("Amber".into()),
            artifact: None,
            part: None,
            notes: Default::default(),
            extras: ipc::AnswerExtras {
                attachments: vec![ipc::SessionUpload { name: "stale-chip.png".into(), media_type: "image/png".into(), data: String::new() }],
                mentions: vec![ipc::SessionTag { kind: ipc::TagKind::Branch, id: "chip-colour".into(), title: "chip-colour".into(), url: None }],
            },
            at: ipc::Instant::carried(at),
        }
    }

    /// The bounds of the `role` named `name` drawn inside `card`.
    fn within(cx: &mut gpui::VisualTestContext, role: Role, name: &str, card: gpui::Bounds<gpui::Pixels>) -> gpui::Bounds<gpui::Pixels> {
        cx.update(|window, cx| {
            cx.global::<crate::labels::LabelBook>().drawn(window.window_handle().window_id()).iter().filter(|one| one.role == role && one.name.as_ref() == name).map(|one| one.bounds).find(|one| card.contains(&one.center()))
        })
        .unwrap_or_else(|| panic!("{name} was not drawn in its card"))
    }

    fn sheet_holds(view: &gpui::Entity<JobView>, cx: &mut gpui::VisualTestContext, item: &str) -> bool {
        view.read_with(cx, |view, cx| view.review_items("mock-01", cx).iter().any(|one| one.id == item))
    }

    #[gpui::test]
    fn the_first_question_is_the_card_in_the_thread_answered_there_and_the_next_takes_its_place(cx: &mut gpui::TestAppContext) {
        let (view, cx) = rooted_job(cx, "job-view", "mock-01", size(px(1440.), px(900.)), |_| {});
        cx.update(|window, cx| view.update(cx, |view, cx| view.set_needs(false, None, window, cx)));
        redraw(cx);
        assert!(sheet_holds(&view, cx, "ask:call-1"), "the sheet holds the question while it waits");
        let card = bounds(cx, Role::Article, &format!("Question: {FIRST}"));
        assert!(!drawn(cx, Role::Article, &format!("Question: {SECOND}")), "only the first ask stands as a card");
        for choice in ["Amber", "Orange", "Grey"] {
            within(cx, Role::Button, choice, card);
        }
        within(cx, Role::Button, "Answer in words", card);
        let amber = within(cx, Role::Button, "Amber", card).center();
        cx.simulate_click(amber, gpui::Modifiers::none());
        redraw(cx);
        assert!(!view.read_with(cx, |view, _| view.review_open()), "it answers in place, with no sheet");
        assert!(!drawn(cx, Role::Article, &format!("Question: {FIRST}")), "the answered question leaves the thread");
        assert!(drawn(cx, Role::Article, &format!("Question: {SECOND}")), "the next ask takes its card");
        assert!(!sheet_holds(&view, cx, "ask:call-1"), "and leaves the sheet");
        cx.update(|window, cx| view.update(cx, |view, cx| view.set_needs(true, None, window, cx)));
        redraw(cx);
        assert!(drawn_like(cx, Role::Button, FIRST).is_empty(), "nor does Needs you list it");
    }

    #[gpui::test]
    fn both_questions_are_answered_on_their_cards_once_needs_you_brought_the_thread_there(cx: &mut gpui::TestAppContext) {
        let (view, cx) = rooted_job(cx, "job-view", "mock-01", size(px(1440.), px(900.)), |_| {});
        cx.update(|window, cx| view.update(cx, |view, cx| view.go_to_ask("ask:call-1", window, cx)));
        redraw(cx);
        let card = bounds(cx, Role::Article, &format!("Question: {FIRST}"));
        let amber = within(cx, Role::Button, "Amber", card).center();
        cx.simulate_click(amber, gpui::Modifiers::none());
        redraw(cx);
        assert!(!drawn(cx, Role::Article, &format!("Question: {FIRST}")), "the first leaves the thread");
        assert!(!view.read_with(cx, |view, cx| view.needs_up("mock-01", cx)), "and the next question, there since the first, opens no panel over it");
        let card = bounds(cx, Role::Article, &format!("Question: {SECOND}"));
        let tooltip = within(cx, Role::Button, "In the tooltip", card).center();
        cx.simulate_click(tooltip, gpui::Modifiers::none());
        redraw(cx);
        assert!(!drawn(cx, Role::Article, &format!("Question: {SECOND}")), "the second leaves the thread too");
        assert!(!sheet_holds(&view, cx, "ask:call-1#1"), "and the sheet");
    }

    #[gpui::test]
    fn a_question_answered_in_words_on_its_card_sends_his_words(cx: &mut gpui::TestAppContext) {
        let (view, cx) = rooted_job(cx, "job-view", "mock-01", size(px(1440.), px(900.)), |_| {});
        cx.update(|window, cx| view.update(cx, |view, cx| view.set_needs(false, None, window, cx)));
        redraw(cx);
        let card = bounds(cx, Role::Article, &format!("Question: {FIRST}"));
        let words = within(cx, Role::Button, "Answer in words", card).center();
        cx.simulate_click(words, gpui::Modifiers::none());
        redraw(cx);
        let card = bounds(cx, Role::Article, &format!("Question: {FIRST}"));
        assert!(drawn_like(cx, Role::Button, "Amber").is_empty(), "the box takes the options' place");
        within(cx, Role::Button, "Cancel", card);
        cx.simulate_input("Amber, as the other waiting marks");
        redraw(cx);
        let send = within(cx, Role::Button, "Send answer", card).center();
        cx.simulate_click(send, gpui::Modifiers::none());
        redraw(cx);
        assert!(!drawn(cx, Role::Article, &format!("Question: {FIRST}")), "sent, the question leaves the thread");
        assert!(!sheet_holds(&view, cx, "ask:call-1"), "and the sheet");
    }

    #[gpui::test]
    fn a_question_answered_in_the_sheet_leaves_the_thread_and_needs_you_at_once(cx: &mut gpui::TestAppContext) {
        let (view, cx) = rooted_job(cx, "job-view", "mock-01", size(px(1440.), px(900.)), |_| {});
        cx.update(|window, cx| view.update(cx, |view, cx| view.set_needs(false, None, window, cx)));
        redraw(cx);
        cx.update(|window, cx| view.update(cx, |view, cx| view.open_ask("ask:call-1", window, cx)));
        redraw(cx);
        let amber = bounds(cx, Role::RadioButton, "Amber").center();
        cx.simulate_click(amber, gpui::Modifiers::none());
        redraw(cx);
        let send = bounds(cx, Role::Button, "Send answer").center();
        cx.simulate_click(send, gpui::Modifiers::none());
        redraw(cx);
        if view.read_with(cx, |view, _| view.review_open()) {
            cx.simulate_keystrokes("escape");
            redraw(cx);
        }
        assert!(!view.read_with(cx, |view, _| view.review_open()), "Esc shuts the sheet");
        assert!(!drawn(cx, Role::Article, &format!("Question: {FIRST}")), "the question answered in the sheet leaves the thread");
        assert!(drawn(cx, Role::Article, &format!("Question: {SECOND}")), "the one still asking takes the card");
        cx.update(|window, cx| view.update(cx, |view, cx| view.set_needs(true, None, window, cx)));
        redraw(cx);
        assert!(drawn_like(cx, Role::Button, FIRST).is_empty(), "nor does Needs you list it");
    }

    #[gpui::test]
    fn a_question_pressed_in_needs_you_brings_the_thread_to_its_card(cx: &mut gpui::TestAppContext) {
        let (view, cx) = rooted_job(cx, "job-view", "mock-01", size(px(1440.), px(900.)), |_| {});
        cx.update(|window, cx| {
            view.update(cx, |view, cx| {
                view.set_needs(false, None, window, cx);
                if let Some(page) = view.pages.get_mut("mock-01") {
                    page.list.scroll_to(gpui::ListOffset { item_ix: 0, offset_in_item: px(0.) });
                }
                cx.notify();
            })
        });
        redraw(cx);
        assert!(!drawn(cx, Role::Article, &format!("Question: {FIRST}")), "at the thread's top the card is out of sight");
        cx.update(|window, cx| view.update(cx, |view, cx| view.set_needs(true, None, window, cx)));
        redraw(cx);
        for choice in ["Amber", "Orange", "Grey", "In the tooltip", "In the chip"] {
            assert!(drawn_like(cx, Role::Button, choice).is_empty() && drawn_like(cx, Role::RadioButton, choice).is_empty(), "{choice} is not drawn in Needs you");
        }
        let listed = bounds(cx, Role::Button, FIRST);
        cx.simulate_click(listed.center(), gpui::Modifiers::none());
        redraw(cx);
        redraw(cx);
        assert!(!view.read_with(cx, |view, _| view.review_open()), "the sheet stays shut while the thread holds the card");
        assert!(!view.read_with(cx, |view, cx| view.needs_up("mock-01", cx)), "Needs you folds");
        assert!(drawn(cx, Role::Article, &format!("Question: {FIRST}")), "the thread is brought to the question's card");
    }

    #[gpui::test]
    fn review_all_in_needs_you_opens_the_sheet_on_every_open_ask(cx: &mut gpui::TestAppContext) {
        let (view, cx) = rooted_job(cx, "job-view", "mock-01", size(px(1440.), px(900.)), |_| {});
        cx.update(|window, cx| view.update(cx, |view, cx| view.set_needs(true, None, window, cx)));
        redraw(cx);
        let all = bounds(cx, Role::Button, "Review all").center();
        cx.simulate_click(all, gpui::Modifiers::none());
        redraw(cx);
        assert!(view.read_with(cx, |view, _| view.review_open()), "Review all opens the sheet");
        assert!(!view.read_with(cx, |view, cx| view.needs_up("mock-01", cx)), "Needs you folds as it opens");
        let held = view.read_with(cx, |view, cx| view.review_items("mock-01", cx).iter().map(|one| one.id.clone()).collect::<Vec<_>>());
        for item in ["ask:call-1", "ask:call-1#1", "plan:2"] {
            assert!(held.iter().any(|one| one == item), "the sheet holds {item}: {held:?}");
        }
    }

    #[gpui::test]
    fn his_answer_from_the_sheet_is_a_card_in_the_thread_with_his_words_and_what_he_sent(cx: &mut gpui::TestAppContext) {
        let (view, cx) = rooted_job_with(
            cx,
            "job-view",
            "mock-01",
            size(px(1440.), px(2000.)),
            |jobs| {
                let job = jobs.iter_mut().find(|job| job.id.as_str() == "mock-01").expect("the fixture has mock-01");
                job.waiting_for.retain(|item| !item.id.starts_with("ask:call-1"));
                job.answers.push(answer("ask:call-1", FIRST, "2099-01-01T00:00:00Z"));
            },
            |threads| {
                if let Some(thread) = threads.get_mut("mock-01") {
                    thread.head.asked = None;
                }
            },
        );
        cx.update(|window, cx| view.update(cx, |view, cx| view.set_needs(false, None, window, cx)));
        redraw(cx);
        assert!(drawn(cx, Role::Region, &format!("You picked: {FIRST}")), "the card names how he answered and the ask");
        assert!(drawn(cx, Role::Group, "Question"), "the ask is quoted");
        assert!(drawn(cx, Role::Group, "Attached: stale-chip.png"), "his picture stands on it by name");
        assert!(drawn(cx, Role::Group, "Tagged: chip-colour"), "and what he tagged");
        assert!(!drawn(cx, Role::Article, &format!("Question: {FIRST}")), "an answered question has no card to answer");
    }

    #[gpui::test]
    fn his_answer_card_keeps_the_formatting_of_his_words(cx: &mut gpui::TestAppContext) {
        let (view, cx) = rooted_job_with(
            cx,
            "job-view",
            "mock-01",
            size(px(1440.), px(2000.)),
            |jobs| {
                let job = jobs.iter_mut().find(|job| job.id.as_str() == "mock-01").expect("the fixture has mock-01");
                job.waiting_for.retain(|item| !item.id.starts_with("ask:call-1"));
                let mut held = answer("ask:call-1", FIRST, "2099-01-01T00:00:00Z");
                held.words = Some("Amber, and run\n```\narmada check\n```".into());
                job.answers.push(held);
            },
            |threads| {
                if let Some(thread) = threads.get_mut("mock-01") {
                    thread.head.asked = None;
                }
            },
        );
        cx.update(|window, cx| view.update(cx, |view, cx| view.set_needs(false, None, window, cx)));
        redraw(cx);
        let card = bounds(cx, Role::Region, &format!("You picked: {FIRST}"));
        let copy = cx.update(|window, cx| {
            cx.global::<crate::labels::LabelBook>().drawn(window.window_handle().window_id()).iter().filter(|one| one.role == Role::Button && one.name == "Copy").map(|one| one.bounds).find(|one| card.contains(&one.center()))
        });
        assert!(copy.is_some(), "his block of code is drawn as a block, with its Copy, as the composer sent it");
    }

    #[gpui::test]
    fn a_waiting_plan_after_the_first_ask_is_a_line_that_opens_the_sheet_on_it(cx: &mut gpui::TestAppContext) {
        let (view, cx) = rooted_job(cx, "job-view", "mock-01", size(px(1440.), px(900.)), |_| {});
        cx.update(|window, cx| view.update(cx, |view, cx| view.set_needs(false, None, window, cx)));
        redraw(cx);
        assert!(sheet_holds(&view, cx, "plan:2"), "the sheet holds the plan while it waits");
        let extras = view.read_with(cx, |view, _| view.pages.get("mock-01").map(|page| page.extras.clone()).unwrap_or_default());
        let at = extras.iter().position(|extra| matches!(extra, super::super::Extra::Plan(_))).expect("the plan keeps its place over the thread");
        view.update(cx, |view, cx| {
            if let Some(page) = view.pages.get_mut("mock-01") {
                let ix = page.items + at;
                page.list.scroll_to(gpui::ListOffset { item_ix: page.entry(ix), offset_in_item: px(0.) });
            }
            cx.notify();
        });
        redraw(cx);
        assert!(!drawn(cx, Role::Article, "Plan to approve"), "while a question comes first, the plan is no card");
        cx.update(|window, cx| view.update(cx, |view, cx| view.go_to_ask("plan:2", window, cx)));
        redraw(cx);
        let chosen = view.read_with(cx, |view, _| view.review_open().then(|| view.review_chosen()).flatten().map(|item| item.id));
        assert_eq!(chosen.as_deref(), Some("plan:2"), "a press on its line in Needs you opens the sheet on it");
    }

    #[gpui::test]
    fn a_waiting_plan_that_is_first_is_the_first_card_approved_there_and_it_leaves_the_sheet(cx: &mut gpui::TestAppContext) {
        let (view, cx) = rooted_job_with(
            cx,
            "job-view",
            "mock-01",
            size(px(1440.), px(2000.)),
            |jobs| {
                let job = jobs.iter_mut().find(|job| job.id.as_str() == "mock-01").expect("the fixture has mock-01");
                job.waiting_for.retain(|item| !item.id.starts_with("ask:call-1"));
            },
            |threads| {
                if let Some(thread) = threads.get_mut("mock-01") {
                    thread.head.asked = None;
                }
            },
        );
        cx.update(|window, cx| view.update(cx, |view, cx| view.go_to_ask("plan:2", window, cx)));
        redraw(cx);
        redraw(cx);
        assert!(!view.read_with(cx, |view, _| view.review_open()), "the thread holds its card, so the sheet stays shut");
        let name = view.read_with(cx, |view, cx| view.first_ask("mock-01", cx)).map(|first| format!("Plan: {}", first.name)).expect("the plan is first");
        let card = bounds(cx, Role::Article, &name);
        within(cx, Role::Button, "Ask for changes", card);
        let approve = within(cx, Role::Button, "Approve", card).center();
        cx.simulate_click(approve, gpui::Modifiers::none());
        redraw(cx);
        assert!(!view.read_with(cx, |view, _| view.review_open()), "it answers in place");
        assert!(!sheet_holds(&view, cx, "plan:2"), "approved on its card, the plan leaves the sheet");
    }

    #[gpui::test]
    fn a_page_after_the_first_ask_is_a_line_until_its_turn_on_the_first_card(cx: &mut gpui::TestAppContext) {
        const CARD: &str = "Document: Stale branch design, Revision 2";
        const PIN: &str = "Drawing: Tail layout";
        const PAGE: &str = "Page: Approve Stale branch design";
        let first_id = |view: &JobView, cx: &gpui::App| view.first_ask("mock-01", cx).map(|first| first.id).unwrap_or_default();
        let (view, cx) = rooted_job_with(
            cx,
            "job-view",
            "mock-01",
            size(px(1440.), px(900.)),
            |jobs| {
                let job = jobs.iter_mut().find(|job| job.id.as_str() == "mock-01").expect("the fixture has mock-01");
                // Only the sketch and the page ask, so the first card is the sketch's and the page waits its turn.
                job.waiting_for.retain(|item| item.id.starts_with("sketch:"));
                let item = br#"{"id":"document:design","text":"Approve Stale branch design","since":"2026-11-09T13:50:00Z","source":"document"}"#;
                job.waiting_for.push(ipc::decode::<ipc::WaitingItem>("page item", item).expect("the page item decodes"));
            },
            |threads| {
                // The page's card stands last, so the thread's tail draws it.
                let thread = threads.get_mut("mock-01").expect("the fixture has mock-01's thread");
                thread.items.truncate(40);
                thread.items.drain(..36);
                (thread.head.plan, thread.head.asked) = (None, None);
            },
        );
        cx.update(|window, cx| view.update(cx, |view, cx| view.set_needs(false, None, window, cx)));
        redraw(cx);
        let sketch = |view: &gpui::Entity<JobView>, cx: &mut gpui::VisualTestContext| view.read_with(cx, |view, cx| view.review_items("mock-01", cx).iter().any(|one| one.name.contains("Tail layout")));
        assert!(sheet_holds(&view, cx, "document:design") && sketch(&view, cx), "the sheet holds both while they wait");
        // The sketch is first, so the first card is its one place: no pin stands for it while it asks.
        let extras = view.read_with(cx, |view, _| view.pages.get("mock-01").map(|page| page.extras.clone()).unwrap_or_default());
        assert!(extras.iter().all(|extra| !matches!(extra, super::super::Extra::Sketch(_))), "no pin while it asks: {extras:?}");
        cx.update(|window, cx| view.update(cx, |view, cx| view.go_to_ask(&first_id(&view, cx), window, cx)));
        redraw(cx);
        assert!(!drawn(cx, Role::Article, CARD), "while the sketch comes first, the page is no card");
        assert!(drawn(cx, Role::Button, "Approve Stale branch design"), "it is one line in the thread");
        cx.update(|window, cx| view.update(cx, |view, cx| view.go_to_ask("document:design", window, cx)));
        redraw(cx);
        let chosen = view.read_with(cx, |view, _| view.review_open().then(|| view.review_chosen()).flatten().map(|item| item.id));
        assert_eq!(chosen.as_deref(), Some("document:design"), "a press on its line in Needs you opens the sheet on it");
        cx.update(|window, cx| view.update(cx, |view, cx| view.close_review(window, cx)));
        cx.update(|window, cx| view.update(cx, |view, cx| view.go_to_ask(&first_id(&view, cx), window, cx)));
        redraw(cx);
        let pin = bounds(cx, Role::Article, PIN);
        let approve = within(cx, Role::Button, "Approve", pin).center();
        cx.simulate_click(approve, gpui::Modifiers::none());
        redraw(cx);
        assert!(!sketch(&view, cx), "approved on its card, the sketch leaves the sheet");
        // Its turn: the page is first now, and the first card is where it is answered.
        cx.update(|window, cx| view.update(cx, |view, cx| view.go_to_ask("document:design", window, cx)));
        redraw(cx);
        assert!(!view.read_with(cx, |view, _| view.review_open()), "the thread comes to its card");
        let card = bounds(cx, Role::Article, PAGE);
        within(cx, Role::Button, "Ask for changes", card);
        let approve = within(cx, Role::Button, "Approve", card).center();
        cx.simulate_click(approve, gpui::Modifiers::none());
        redraw(cx);
        assert!(!sheet_holds(&view, cx, "document:design"), "approved on the first card, the page leaves the sheet");
        assert!(drawn(cx, Role::Article, CARD), "settled, the page draws as its card again");
    }

    #[test]
    fn a_page_is_one_ask_whether_to_approve_or_to_read() {
        assert!(same_item("document:design", "read:design"));
        assert!(same_item("ask:call-1", "ask:call-1"));
        assert!(!same_item("ask:call-1", "ask:call-1#1"));
        assert!(!same_item("document:design", "document:notes"));
    }

    #[test]
    fn an_item_of_a_card_is_named_as_fleet_names_it() {
        assert_eq!(ask_item_id("call-1", 0), "ask:call-1");
        assert_eq!(ask_item_id("call-1", 2), "ask:call-1#2");
        assert!(of_card("ask:call-1#2", "call-1"));
        assert!(of_card("ask:call-1", "call-1"));
        assert!(!of_card("ask:call-12", "call-1"));
    }

    #[test]
    fn a_question_answered_stands_where_he_gave_it() {
        let mut head = crate::mock::threads("job-view").expect("the threads decode").remove("mock-01").expect("the fixture has mock-01").head;
        (head.plan, head.sketch) = (None, None);
        assert!(!answer_in_place(&answer("ask:call-1", FIRST, "2026-11-20T10:00:00Z"), &head, &[], &[]));
        let mut on_parts = answer("artifact-1", "Which layout?", "2026-11-20T10:00:00Z");
        on_parts.artifact = Some("artifact-1".into());
        assert!(answer_in_place(&on_parts, &head, &[], &["artifact-1".into()]), "an artifact of parts' answer stands on its card");
    }
}
