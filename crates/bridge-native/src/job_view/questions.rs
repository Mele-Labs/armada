//! His questions to a Job and the replies that answer them, as the owner approved them in a sketch on 17 Nov 2026: the Q&A
//! tab, the answer folded under his question in the thread, the one pinned line while an answer is unread, and ⌥↑ ⌥↓ to
//! step between his questions. Fleet's `list_job_questions` is the whole list, read again where `session.story` says it
//! moved. **Nothing here sorts it.**
//!
//! A jump scrolls the thread to the reply's item, reading earlier items until the thread holds it, and tells Fleet he read
//! the answer (`mark_answer_read`).

use std::collections::{HashMap, HashSet};

use chrono::{DateTime, Local};
use gpui::{div, prelude::*, AnyElement, Context, FocusHandle, FontWeight, KeyDownEvent, Role, SharedString, Window};
use ipc::{JobAnswer, JobQuestion, JobQuestions, MarkAnswerRead, SessionId, SessionRow, StoryChanged, ThreadItem};

use super::artifacts_tab::step;
use super::rows::{clock, eyebrow, tip};
use super::{JobView, Page};
use crate::labels::labelled;
use crate::look::{token, ActiveLook, Look};

/// What the view holds of its Jobs' questions.
#[derive(Default)]
pub(super) struct Questions {
    /// Each Job's questions as Fleet last read them. None until the first answer, so the tab draws its shapes.
    read: HashMap<String, Option<JobQuestions>>,
    /// The Job whose first card takes the keys once its list arrives, where the tab was pressed before it had.
    want_focus: Option<String>,
    /// The row a jump goes to that the thread does not hold yet, by Job: earlier items are read until it does.
    seeking: HashMap<String, String>,
    /// Which unread answer the pinned line shows, by Job: its place among them.
    pinned: HashMap<String, usize>,
    /// What the mock Fleet answers for its Job, taken when the view was made.
    mock: Option<JobQuestions>,
}

impl Questions {
    pub(super) fn mock(mock: Option<JobQuestions>) -> Self {
        Questions { mock, ..Default::default() }
    }
}

/// The answers he has not read, in Fleet's order, which is newest first: the pinned line steps through these.
pub(super) fn unread(read: &JobQuestions) -> Vec<&JobQuestion> {
    read.questions.iter().filter(|one| one.answer.as_ref().is_some_and(|answer| !answer.read)).collect()
}

/// The thread item that holds row `row`: one drawn as it came, or one folded into a run of calls, a Check run or a Fleet turn.
pub(super) fn item_holding(items: &[ThreadItem], row: &str) -> Option<usize> {
    items.iter().position(|item| match item {
        ThreadItem::Row { row: held } => held.id() == row,
        ThreadItem::Calls { rows, .. } | ThreadItem::Checks { rows, .. } => rows.iter().any(|one| one.id() == row),
        ThreadItem::FleetTurn { notice, rows, .. } => notice.id() == row || rows.iter().any(|one| one.id() == row),
        _ => false,
    })
}

/// The item ⌥↑ (`by` below zero) or ⌥↓ goes to from the item at the top of the thread, `top`: the nearest question before it
/// or after it. `at` is each question's item, in the thread's order. A `top` below zero stands above every item.
pub(super) fn question_step(at: &[usize], top: isize, by: i32) -> Option<usize> {
    if by < 0 {
        at.iter().rev().find(|&&one| (one as isize) < top).copied()
    } else {
        at.iter().find(|&&one| (one as isize) > top).copied()
    }
}

fn empty(job: &str) -> JobQuestions {
    JobQuestions { session_id: SessionId::carried(job.to_string()), questions: Vec::new() }
}

/// His words on one line, as the pinned line says them.
fn one_line(words: &str) -> String {
    words.split_whitespace().collect::<Vec<_>>().join(" ")
}

impl JobView {
    // ---- reading ---------------------------------------------------------------------------

    /// Read the shown Job's questions the first time it is drawn: the thread folds their answers in, whatever tab is open.
    pub(super) fn sync_questions(&mut self, id: &SharedString, window: &mut Window, cx: &mut Context<Self>) {
        if !self.questions.read.contains_key(id.as_ref()) {
            self.questions.read.insert(id.to_string(), None);
            self.read_questions(id, window, cx);
        }
        self.tend_seek(id, cx);
    }

    fn read_questions(&mut self, id: &str, window: &mut Window, cx: &mut Context<Self>) {
        if self.fleet.is_none() {
            let held = self.questions.mock.clone().filter(|held| held.session_id.as_str() == id);
            self.questions_arrived(id, held, window, cx);
            return;
        }
        let (job, path) = (id.to_string(), format!("/sessions/questions?session_id={}", super::wire::encoded(id)));
        self.get(
            path,
            window,
            cx,
            Box::new(move |this, answer, window, cx| {
                let read = answer.ok().and_then(|bytes| ipc::decode::<JobQuestions>("JobQuestions", &bytes).ok());
                this.questions_arrived(&job, read, window, cx);
                cx.notify();
            }),
        );
    }

    /// Fleet says these Jobs' questions moved: each one held is read again.
    pub(super) fn questions_moved(&mut self, moved: &[StoryChanged], window: &mut Window, cx: &mut Context<Self>) {
        let mut jobs: Vec<String> = moved.iter().map(|one| one.session_id.as_str().to_string()).filter(|job| self.questions.read.contains_key(job)).collect();
        jobs.dedup();
        for job in jobs {
            self.read_questions(&job, window, cx);
        }
    }

    /// A read came back. A refusal leaves what was held, and holds an empty list where there was none.
    fn questions_arrived(&mut self, job: &str, read: Option<JobQuestions>, window: &mut Window, cx: &mut Context<Self>) {
        let rows_of = |held: Option<&JobQuestions>| held.into_iter().flat_map(|held| held.questions.iter().map(|one| one.row_id.clone())).collect::<HashSet<String>>();
        let mut rows = rows_of(self.questions.read.get(job).and_then(Option::as_ref));
        let held = self.questions.read.entry(job.to_string()).or_insert(None);
        match read {
            Some(read) => *held = Some(read),
            None if held.is_none() => *held = Some(empty(job)),
            None => {}
        }
        rows.extend(rows_of(held.as_ref()));
        // His rows draw the answer folded under them, so each is measured again.
        let at: Vec<usize> = self.thread(job, cx).map(|thread| rows.iter().filter_map(|row| item_holding(&thread.items, row)).collect()).unwrap_or_default();
        if let Some(page) = self.pages.get(job) {
            at.into_iter().for_each(|at| page.remeasure(at));
        }
        if self.questions.want_focus.as_deref() == Some(job) {
            self.questions.want_focus = None;
            self.focus_questions(window, cx);
        }
    }

    /// The question at his row `row` in Job `job`, as Fleet last read it.
    fn question(&self, job: &str, row: &str) -> Option<&JobQuestion> {
        self.questions.read.get(job)?.as_ref()?.questions.iter().find(|one| one.row_id == row)
    }

    fn answer_mut(&mut self, job: &str, row: &str) -> Option<&mut JobAnswer> {
        self.questions.read.get_mut(job)?.as_mut()?.questions.iter_mut().find(|one| one.row_id == row)?.answer.as_mut()
    }

    // ---- what a jump does ------------------------------------------------------------------

    /// Tell Fleet he read the answer to his question at `row`. It reads as read at once; a refusal puts it back.
    pub(super) fn mark_answer_read(&mut self, job: &str, row: &str, window: &mut Window, cx: &mut Context<Self>) -> Option<MarkAnswerRead> {
        let answer = self.answer_mut(job, row)?;
        if answer.read {
            return None;
        }
        answer.read = true;
        cx.notify();
        let wire = MarkAnswerRead { session_id: SessionId::carried(job.to_string()), row_id: row.to_string() };
        let (owner, question) = (job.to_string(), row.to_string());
        self.post(
            "/sessions/question/read".into(),
            ipc::encode(&wire),
            window,
            cx,
            Box::new(move |this, answer, _, cx| {
                match answer {
                    // On mock data there is no answer: the read made here stands.
                    Ok(bytes) => {
                        if let Ok(kept) = ipc::decode::<JobQuestion>("JobQuestion", &bytes) {
                            if let Some(held) = this.questions.read.get_mut(&owner).and_then(Option::as_mut).and_then(|held| held.questions.iter_mut().find(|one| one.row_id == kept.row_id)) {
                                *held = kept;
                            }
                        }
                    }
                    Err(_) => {
                        if let Some(answer) = this.answer_mut(&owner, &question) {
                            answer.read = false;
                        }
                    }
                }
                cx.notify();
            }),
        );
        Some(wire)
    }

    /// To the reply that answers his question at `row`, which he has then read; to the question itself while none does.
    pub(super) fn jump_to_answer(&mut self, job: &str, row: &str, window: &mut Window, cx: &mut Context<Self>) {
        let reply = self.question(job, row).and_then(|one| one.answer.as_ref()).map(|answer| answer.reply_row.clone());
        if reply.is_some() {
            self.mark_answer_read(job, row, window, cx);
        }
        let to = reply.unwrap_or_else(|| row.to_string());
        // The thread comes to the front with the keys, so he reads on from there.
        self.add_tab("thread", window, cx);
        if !self.reveal_row(job, &to, cx) {
            self.questions.seeking.insert(job.to_string(), to);
            self.tend_seek(&SharedString::from(job.to_string()), cx);
        }
    }

    /// Scroll the thread so the item holding `row` stands at its top. False where the thread does not hold it.
    fn reveal_row(&mut self, job: &str, row: &str, cx: &mut Context<Self>) -> bool {
        let Some(at) = self.thread(job, cx).and_then(|held| item_holding(&held.items, row)) else { return false };
        let Some(page) = self.pages.get_mut(job) else { return false };
        // A thread that follows its tail would snap back: it stays here, and follows again once he is back at the bottom.
        page.list.pause_following_tail();
        page.list.scroll_to(gpui::ListOffset { item_ix: page.entry(at), offset_in_item: gpui::px(0.) });
        cx.notify();
        true
    }

    /// A jump waiting on earlier items: once the thread holds its row it goes there; while older items remain they are read.
    fn tend_seek(&mut self, id: &SharedString, cx: &mut Context<Self>) {
        let Some(row) = self.questions.seeking.get(id.as_ref()).cloned() else { return };
        if self.reveal_row(id, &row, cx) {
            self.questions.seeking.remove(id.as_ref());
            return;
        }
        let Some(fleet) = self.fleet.clone() else {
            self.questions.seeking.remove(id.as_ref());
            return;
        };
        if fleet.read(cx).reading_older(id) {
            return;
        }
        if self.thread(id, cx).is_some_and(|held| held.older.is_some()) {
            fleet.update(cx, |fleet, cx| fleet.read_older(id, cx));
        } else {
            self.questions.seeking.remove(id.as_ref());
        }
    }

    /// ⌥↑ and ⌥↓: the thread scrolls to his previous or next question from the item at its top. False where there is none.
    pub(super) fn step_question(&mut self, by: i32, cx: &mut Context<Self>) -> bool {
        let Some(job) = self.shown.clone() else { return false };
        let Some(read) = self.questions.read.get(job.as_ref()).and_then(Option::as_ref) else { return false };
        let rows: HashSet<&str> = read.questions.iter().map(|one| one.row_id.as_str()).collect();
        let Some(held) = self.thread(&job, cx) else { return false };
        let at: Vec<usize> = held
            .items
            .iter()
            .enumerate()
            .filter(|(_, item)| matches!(item, ThreadItem::Row { row: SessionRow::Message { id, .. } } if rows.contains(id.as_str())))
            .map(|(at, _)| at)
            .collect();
        let Some(page) = self.pages.get_mut(&job) else { return false };
        let top = page.list.logical_scroll_top().item_ix as isize - (page.lead + page.before) as isize;
        let Some(to) = question_step(&at, top, by) else { return false };
        page.list.pause_following_tail();
        page.list.scroll_to(gpui::ListOffset { item_ix: page.entry(to), offset_in_item: gpui::px(0.) });
        cx.notify();
        true
    }

    // ---- the keys ---------------------------------------------------------------------------

    /// The keys go to the first card of the tab, as a press on its tab gives them to a body that takes them.
    pub(super) fn focus_questions(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(job) = self.shown.clone() else { return };
        let first = match self.questions.read.get(job.as_ref()) {
            Some(Some(read)) => read.questions.first().map(|one| one.row_id.clone()),
            Some(None) => {
                self.questions.want_focus = Some(job.to_string());
                None
            }
            None => {
                self.questions.read.insert(job.to_string(), None);
                self.questions.want_focus = Some(job.to_string());
                self.read_questions(&job, window, cx);
                None
            }
        };
        let Some(row) = first else { return };
        let Some(spot) = self.pages.get(&job).map(|page| page.keys.spot(&format!("qa:{row}"), cx)) else { return };
        window.defer(cx, move |window, cx| window.focus(&spot.focus, cx));
    }

    fn qa_focus(&self, job: &str, row: &str) -> Option<FocusHandle> {
        Some(self.pages.get(job)?.keys.spot_of(&format!("qa:{row}"))?.focus)
    }

    /// Up and Down move the focus over the cards; Enter jumps to the answer. True where the tab took the key.
    fn questions_key(&mut self, job: &str, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) -> bool {
        if event.keystroke.modifiers.modified() {
            return false;
        }
        let Some(read) = self.questions.read.get(job).and_then(Option::as_ref) else { return false };
        let rows: Vec<String> = read.questions.iter().map(|one| one.row_id.clone()).collect();
        let on = rows.iter().find(|row| self.qa_focus(job, row).is_some_and(|focus| focus.is_focused(window))).cloned();
        match event.keystroke.key.as_str() {
            key @ ("down" | "up") => {
                if let Some(focus) = step(&rows, on.as_deref(), if key == "down" { 1 } else { -1 }).and_then(|to| self.qa_focus(job, &to)) {
                    window.focus(&focus, cx);
                }
                true
            }
            "enter" => {
                if let Some(row) = on {
                    self.jump_to_answer(job, &row, window, cx);
                }
                true
            }
            _ => false,
        }
    }

    // ---- the tab -------------------------------------------------------------------------

    /// The Q&A tab: a card a question in Fleet's order, or its shapes while Fleet has not answered. None where he asked none.
    pub(super) fn pane_questions(&self, id: &SharedString, look: &Look, window: &Window, cx: &Context<Self>) -> Option<AnyElement> {
        let page = self.pages.get(id)?;
        let read = self.questions.read.get(id.as_ref())?;
        let Some(read) = read.as_ref() else { return Some(self.for_you_shapes(look)) };
        if read.questions.is_empty() {
            return None;
        }
        let now = Local::now();
        let cards: Vec<AnyElement> = read.questions.iter().map(|one| self.question_card(id, page, one, look, now, window, cx)).collect();
        let keyed = id.to_string();
        Some(
            labelled(
                div()
                    .id("pane-questions")
                    .flex()
                    .flex_col()
                    .gap(look.length(token::SPACE_2))
                    .on_key_down(cx.listener(move |this, event: &KeyDownEvent, window, cx| {
                        if this.questions_key(&keyed, event, window, cx) {
                            cx.stop_propagation();
                        }
                    }))
                    .children(cards),
                Role::List,
                "Your questions",
            )
            .into_any_element(),
        )
    }

    /// One question's card: his words and when he asked, then the answer's first words and when it came, or that none has.
    #[allow(clippy::too_many_arguments)]
    fn question_card(&self, job: &SharedString, page: &Page, one: &JobQuestion, look: &Look, now: DateTime<Local>, window: &Window, cx: &Context<Self>) -> AnyElement {
        let spot = page.keys.spot(&format!("qa:{}", one.row_id), cx);
        let on = spot.focus.is_focused(window);
        let edge = look.length(token::BORDER_WIDTH);
        let hue = look.color(token::YOU);
        let fill = look.color(token::BG_HOVER);
        let lit = fill.blend(hue.opacity(0.10));
        let small = |ink| div().text_size(look.length(token::TEXT_XS)).line_height(look.length(token::LEADING_XS)).text_color(look.color(ink));
        let unread = one.answer.as_ref().is_some_and(|answer| !answer.read);
        // The dot's column is on every card, so the words line up down the list.
        let dot = div().flex_none().w(look.length(token::SPACE_2)).h(look.length(token::LEADING_SM)).flex().items_center().children(unread.then(|| {
            labelled(
                div().id(SharedString::from(format!("qa-unread:{}", one.row_id))).size(look.length(token::SPACE_2)).rounded_full().bg(hue).tooltip(tip("Answer not read")),
                Role::Image,
                "Answer not read",
            )
        }));
        let asked = clock(one.asked_at.as_str(), now).unwrap_or_default();
        let head = div()
            .flex()
            .items_start()
            .gap(look.length(token::SPACE_2))
            .child(div().flex_1().min_w_0().font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM))).line_clamp(2).text_ellipsis().child(SharedString::from(one_line(&one.words))))
            .child(small(token::FG_SUBTLE).flex_none().child(SharedString::from(asked)));
        let under = match &one.answer {
            Some(answer) => {
                let when = clock(answer.answered_at.as_str(), now).map_or_else(|| "Answered".to_string(), |at| format!("Answered {at}"));
                div()
                    .flex()
                    .flex_col()
                    .gap(look.length(token::SPACE_1))
                    .child(small(token::FG_MUTED).line_clamp(3).text_ellipsis().child(SharedString::from(one_line(&answer.excerpt))))
                    .child(small(token::FG_SUBTLE).child(SharedString::from(when)))
            }
            None => div().child(small(token::STATUS_AWAITING_REVIEW).child("Not answered yet")),
        };
        let (job, row) = (job.to_string(), one.row_id.clone());
        let card = div()
            .id(SharedString::from(format!("qa-row:{}", one.row_id)))
            .track_focus(&spot.focus)
            .relative()
            .overflow_hidden()
            .flex()
            .items_start()
            .gap(look.length(token::SPACE_2))
            .py(look.length(token::SPACE_2))
            .pl(look.length(token::SPACE_3) + edge * 2.)
            .pr(look.length(token::SPACE_3))
            .border(edge)
            .border_color(if on { look.color(token::ACCENT) } else { look.color(token::BORDER_SUBTLE).blend(hue.opacity(0.4)) })
            .rounded(look.length(token::RADIUS_MD))
            .bg(if on { lit } else { fill })
            .hover(move |style| style.bg(lit))
            .cursor_pointer()
            .text_size(look.length(token::TEXT_SM))
            .line_height(look.length(token::LEADING_SM))
            .text_color(look.color(token::FG_DEFAULT))
            // His words' stripe, taking the card's inner left corner, since gpui clips to the rectangle and not the radius.
            .child(div().absolute().left_0().top_0().bottom_0().w(edge * 3.).rounded_l(look.length(token::RADIUS_MD) - edge).bg(hue))
            .child(dot)
            .child(div().flex_1().min_w_0().flex().flex_col().gap(look.length(token::SPACE_1)).child(head).child(under))
            .on_click(cx.listener(move |this, _, window, cx| this.jump_to_answer(&job, &row, window, cx)));
        labelled(card, Role::Row, one_line(&one.words)).into_any_element()
    }

    // ---- in the thread ---------------------------------------------------------------------

    /// The answer folded under his question at `row`, inside his message: when it came, its first words and Jump.
    pub(super) fn folded_answer(&self, page: &Page, row: &str, look: &Look, now: DateTime<Local>, cx: &Context<Self>) -> Option<AnyElement> {
        let job = self.shown.as_ref()?;
        let answer = self.question(job, row)?.answer.as_ref()?;
        let when = clock(answer.answered_at.as_str(), now).map_or_else(|| "Answered".to_string(), |at| format!("Answered {at}"));
        let spot = page.keys.spot(&format!("answer-jump:{row}"), cx);
        let jump = self.jump_button(&format!("answer-jump:{row}"), &spot.focus, job, row, look, cx);
        Some(
            labelled(
                div()
                    .id(SharedString::from(format!("answer:{row}")))
                    .flex()
                    .flex_col()
                    .gap(look.length(token::SPACE_1))
                    .p(look.length(token::SPACE_3))
                    .rounded(look.length(token::RADIUS_MD))
                    .bg(look.color(token::BG_SUNKEN))
                    .border(look.length(token::BORDER_WIDTH))
                    .border_color(look.color(token::BORDER_SUBTLE))
                    .child(div().flex().items_center().gap(look.length(token::SPACE_2)).child(eyebrow(look, &when).flex_1().text_color(look.color(token::FG_MUTED))).child(jump))
                    .child(div().text_size(look.length(token::TEXT_SM)).line_height(look.length(token::LEADING_SM)).text_color(look.color(token::FG_DEFAULT)).child(SharedString::from(one_line(&answer.excerpt)))),
                Role::Group,
                "Answer",
            )
            .into_any_element(),
        )
    }

    /// Jump: to the reply that answers his question at `row`, from the keys too.
    fn jump_button(&self, id: &str, focus: &FocusHandle, job: &str, row: &str, look: &Look, cx: &Context<Self>) -> AnyElement {
        let (keyed, pressed) = ((job.to_string(), row.to_string()), (job.to_string(), row.to_string()));
        labelled(
            super::acts::button(look, SharedString::from(id.to_string()), "Jump", super::acts::Tone::Secondary, false, None, true)
                .track_focus(focus)
                .h(look.length(token::H_CONTROL_SM))
                .px(look.length(token::SPACE_2))
                .text_size(look.length(token::TEXT_XS))
                .line_height(look.length(token::LEADING_XS))
                .focus_visible(|button| button.border_color(look.color(token::ACCENT)))
                .tooltip(tip("Go to the answer in the thread"))
                .on_key_down(cx.listener(move |this, event: &KeyDownEvent, window, cx| {
                    if matches!(event.keystroke.key.as_str(), "enter" | "space") && !event.keystroke.modifiers.modified() {
                        cx.stop_propagation();
                        this.jump_to_answer(&keyed.0, &keyed.1, window, cx);
                    }
                }))
                .on_click(cx.listener(move |this, _, window, cx| {
                    cx.stop_propagation();
                    this.jump_to_answer(&pressed.0, &pressed.1, window, cx);
                })),
            Role::Button,
            "Jump to the answer",
        )
        .into_any_element()
    }

    /// The pinned line at the thread's foot while an answer is unread: which question it answers, ‹ › where more than one
    /// is, and Jump. It goes once every answer is read.
    pub(super) fn render_answer_line(&self, id: &SharedString, cx: &Context<Self>) -> Option<AnyElement> {
        let read = self.questions.read.get(id.as_ref())?.as_ref()?;
        let unread = unread(read);
        let page = self.pages.get(id)?;
        let at = self.questions.pinned.get(id.as_ref()).copied().unwrap_or(0).min(unread.len().checked_sub(1)?);
        let question = unread[at];
        let look = cx.look();
        let jump = page.keys.spot("answer-line:jump", cx);
        let arrows = (unread.len() > 1).then(|| {
            let arrow = |key: &'static str, icon: &'static str, name: &'static str, by: isize| {
                let spot = page.keys.spot(&format!("answer-line:{key}"), cx);
                let (keyed, pressed) = (id.to_string(), id.to_string());
                labelled(
                    super::tabs::frame(look, SharedString::from(format!("answer-line:{key}")), icon)
                        .track_focus(&spot.focus)
                        .focus_visible(|button| button.bg(look.color(token::BG_HOVER)))
                        .tooltip(tip(name))
                        .on_key_down(cx.listener(move |this, event: &KeyDownEvent, _, cx| {
                            if matches!(event.keystroke.key.as_str(), "enter" | "space") && !event.keystroke.modifiers.modified() {
                                cx.stop_propagation();
                                this.step_pinned(&keyed, by, cx);
                            }
                        }))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            cx.stop_propagation();
                            this.step_pinned(&pressed, by, cx);
                        })),
                    Role::Button,
                    name,
                )
            };
            div().flex_none().flex().items_center().child(arrow("newer", "chevron-left", "Newer answer", -1)).child(arrow("older", "chevron-right", "Older answer", 1))
        });
        let said = format!("Answered: {}", one_line(&question.words));
        let line = div()
            .id("answer-line")
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .h(look.length(token::H_CONTROL) + look.length(token::SPACE_2))
            .pl(look.length(token::SPACE_3))
            .pr(look.length(token::SPACE_2))
            .rounded(look.length(token::RADIUS_MD))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::YOU_EDGE))
            .bg(look.color(token::BG_RAISED))
            .shadow(look.shadows(token::SHADOW_OVERLAY))
            .text_size(look.length(token::TEXT_SM))
            .text_color(look.color(token::FG_DEFAULT))
            .child(labelled(div().id("answer-line-dot").flex_none().size(look.length(token::SPACE_2)).rounded_full().bg(look.color(token::YOU)).tooltip(tip("Answer not read")), Role::Image, "Answer not read"))
            .child(div().flex_1().min_w_0().truncate().child(SharedString::from(said.clone())))
            .children(arrows)
            .child(self.jump_button("answer-line:jump-button", &jump.focus, id, &question.row_id, look, cx));
        Some(
            div()
                .w_full()
                .px(look.length(token::SPACE_4) + super::bleed(look))
                .flex()
                .justify_center()
                .child(div().w_full().min_w_0().max_w(self.column(look)).child(labelled(line, Role::Group, "Unread answer")))
                .into_any_element(),
        )
    }

    /// The room the pinned line takes at the thread's foot, so the last row is not under it.
    pub(super) fn answer_room(&self, id: &SharedString, cx: &Context<Self>) -> gpui::Pixels {
        let unread = self.questions.read.get(id.as_ref()).and_then(Option::as_ref).is_some_and(|read| !unread(read).is_empty());
        let look = cx.look();
        if unread {
            look.length(token::H_CONTROL) + look.length(token::SPACE_2) * 2.
        } else {
            gpui::px(0.)
        }
    }

    /// ‹ and ›: the pinned line shows the newer or the older unread answer.
    fn step_pinned(&mut self, job: &str, by: isize, cx: &mut Context<Self>) {
        let count = self.questions.read.get(job).and_then(Option::as_ref).map_or(0, |read| unread(read).len());
        if count == 0 {
            return;
        }
        let at = self.questions.pinned.get(job).copied().unwrap_or(0).min(count - 1) as isize;
        self.questions.pinned.insert(job.to_string(), (at + by).clamp(0, count as isize - 1) as usize);
        cx.notify();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn question(row: &str, answer: Option<(&str, bool)>) -> JobQuestion {
        let at = ipc::Instant::carried("2026-11-17T14:00:00Z");
        JobQuestion {
            row_id: row.into(),
            words: format!("Question {row}?"),
            asked_at: at.clone(),
            answer: answer.map(|(reply, read)| JobAnswer { reply_row: reply.into(), excerpt: "Yes.".into(), answered_at: at.clone(), read }),
        }
    }

    #[test]
    fn the_pinned_line_steps_through_the_unread_answers_in_fleets_order_and_never_an_open_question() {
        let read = JobQuestions {
            session_id: SessionId::carried("j".to_string()),
            questions: vec![question("q5", None), question("q4", Some(("r4", false))), question("q3", Some(("r3", true))), question("q2", Some(("r2", false))), question("q1", None)],
        };
        let rows: Vec<&str> = unread(&read).iter().map(|one| one.row_id.as_str()).collect();
        assert_eq!(rows, ["q4", "q2"], "newest first as Fleet serves them; the read and the unanswered are left out");
        let none = JobQuestions { session_id: SessionId::carried("j".to_string()), questions: vec![question("q1", None), question("q2", Some(("r2", true)))] };
        assert!(unread(&none).is_empty(), "no unread answer: no line");
    }

    #[test]
    fn a_reply_is_found_drawn_as_it_came_or_folded_inside_a_fleet_turn() {
        let items: Vec<ThreadItem> = ipc::decode(
            "items",
            br#"[
                {"kind":"row","row":{"kind":"message","id":"m-1","at":"2026-11-17T14:00:00Z","from":{"kind":"you"},"text":"Why?"}},
                {"kind":"fleet_turn","id":"n-2","notice":{"kind":"message","id":"n-2","at":"2026-11-17T14:01:00Z","from":{"kind":"fleet"},"text":"Go on."},"rows":[{"kind":"message","id":"m-3","at":"2026-11-17T14:02:00Z","from":{"kind":"agent"},"text":"Because."}],"handled":true},
                {"kind":"row","row":{"kind":"message","id":"m-4","at":"2026-11-17T14:03:00Z","from":{"kind":"agent"},"text":"Done."}}
            ]"#,
        )
        .expect("the items decode");
        assert_eq!(item_holding(&items, "m-4"), Some(2));
        assert_eq!(item_holding(&items, "m-3"), Some(1), "a reply folded in a Fleet turn is that turn's item");
        assert_eq!(item_holding(&items, "m-9"), None, "a reply the thread does not hold yet");
    }

    #[test]
    fn option_up_and_down_go_to_the_nearest_question_above_or_below_the_top() {
        let at = [2, 7, 12];
        assert_eq!(question_step(&at, 7, 1), Some(12));
        assert_eq!(question_step(&at, 7, -1), Some(2));
        assert_eq!(question_step(&at, 9, -1), Some(7));
        assert_eq!(question_step(&at, 12, 1), None, "past the last question nothing moves");
        assert_eq!(question_step(&at, 2, -1), None);
        assert_eq!(question_step(&at, -1, 1), Some(2), "from above every item the first question");
    }

    #[test]
    fn qa_is_a_tab_a_pane_can_hold_and_a_zone_that_draws_it() {
        assert!(super::super::pane_tree::is_tab_kind("qa"));
        assert_eq!(super::super::panes_place::zone_of("qa").map(super::super::side::Zone::id), Some("qa"));
    }
}
