//! What a Job needs first: of every ask the Job has open, only the first in Fleet's order stands as a card in the thread, and
//! the next takes its place once it is answered (the scene "What a Job needs first", approved 25 Nov). A bar of steps heads the
//! card: the ask answered last, the ask now, the next, then the third or how many more, and Open Review Sheet. Under the
//! question stands why it is first, then the ask's own answers.

use gpui::{div, prelude::*, AnyElement, Context, FontWeight, Hsla, Role, SharedString, Stateful, Window};
use ipc::{AnswerHow, AnsweredAsk, ItemHow};

use super::ask_card::card_key;
use super::reader_sheet::artifact_in;
use super::review::{answer_batch, instant, page_of_item, Ask, Item, Kind, Verdict};
use super::rows::{glyph, tip, Size};
use super::{JobView, Page};
use crate::labels::labelled;
use crate::look::{token, ActiveLook, Look};
use crate::operator::view::pressed;

/// One step of the bar.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum Step {
    /// The ask he answered last, since the first still open was put to him.
    Done(String),
    Now(Ask, String),
    Next(Ask, String),
    /// How many asks follow the next one, where more than two follow the ask now.
    More(usize),
}

/// The bar's steps: `done` where he answered one since the first open ask was put, the ask now, the next, then the third or
/// how many more where more than two follow the ask now. `asks` are the open asks in order, each with its kind and short words.
pub(super) fn steps(done: Option<String>, asks: &[(Ask, String)]) -> Vec<Step> {
    let mut steps: Vec<Step> = done.into_iter().map(Step::Done).collect();
    let Some((now, words)) = asks.first() else { return steps };
    steps.push(Step::Now(*now, words.clone()));
    let after = &asks[1..];
    match after {
        [] => {}
        [next] => steps.push(Step::Next(next.0, next.1.clone())),
        [next, third] => steps.extend([Step::Next(next.0, next.1.clone()), Step::Next(third.0, third.1.clone())]),
        [next, rest @ ..] => steps.extend([Step::Next(next.0, next.1.clone()), Step::More(rest.len())]),
    }
    steps
}

/// An ask's short words on the bar, from its kind: what he does to it.
pub(super) fn step_words(ask: Ask, read: bool) -> &'static str {
    match ask {
        Ask::Takes => "Pick one",
        Ask::Question => "Answer",
        Ask::Build => "Try the build",
        Ask::Page if read => "Read a page",
        Ask::Page => "Approve a page",
        Ask::Plan => "Approve the plan",
        Ask::Drawing => "Approve a drawing",
        Ask::Draft => "Approve a draft",
    }
}

/// The short words of an ask he answered, as its step read while it was open.
pub(super) fn answered_words(answer: &AnsweredAsk) -> &'static str {
    let item = answer.item.as_str();
    if answer.how == AnswerHow::Picked {
        return step_words(Ask::Takes, false);
    }
    if item.starts_with("ask:") || answer.how == AnswerHow::Answered {
        return step_words(Ask::Question, false);
    }
    match item.split_once(':').map(|(kind, _)| kind) {
        Some("read") => step_words(Ask::Page, true),
        Some("document") => step_words(Ask::Page, false),
        Some("plan") => step_words(Ask::Plan, false),
        Some("draft") => step_words(Ask::Draft, false),
        Some("sketch") => step_words(Ask::Drawing, false),
        Some("try") => step_words(Ask::Build, false),
        _ => "Answered",
    }
}

/// The key the first card takes for `item`: the question's card key, the artifact's own, else `first:<item>`.
pub(super) fn first_key(item: &Item) -> String {
    match item.kind {
        Kind::Question => card_key(&item.id),
        Kind::Artifact => artifact_in(item).unwrap_or(&item.id).to_string(),
        _ => format!("first:{}", item.id),
    }
}

/// A button of the card: its glyph, then its words where `words`, else the glyph alone with its words for a tooltip.
fn glyph_button(look: &Look, id: String, said: &'static str, icon: &'static str, kind: super::sketch_card::Kind, words: bool) -> Stateful<gpui::Div> {
    use super::sketch_card::Kind as K;
    let base = div()
        .id(SharedString::from(id))
        .flex_none()
        .flex()
        .items_center()
        .gap(look.length(token::SPACE_2))
        .h(look.length(token::H_CONTROL))
        .px(look.length(token::SPACE_3))
        .rounded(look.length(token::RADIUS_MD))
        .border(look.length(token::BORDER_WIDTH))
        .border_color(gpui::transparent_black())
        .cursor_pointer()
        .text_size(look.length(token::TEXT_SM))
        .line_height(look.length(token::LEADING_SM))
        .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
        .tab_index(0)
        .focus_visible(|style| style.border_color(look.color(token::ACCENT)));
    let (styled, ink): (Stateful<gpui::Div>, Hsla) = match kind {
        K::Primary | K::Approve => {
            let ink = look.color(token::TEXT_ON_ACCENT);
            (base.bg(look.color(token::ACCENT)).border_color(look.color(token::ACCENT)).text_color(ink).hover(|style| style.bg(look.color(token::ACCENT_HOVER))), ink)
        }
        K::Secondary => {
            let ink = look.color(token::FG_DEFAULT);
            (base.bg(look.color(token::BG_OVERLAY)).border_color(look.color(token::BORDER_STRONG)).text_color(ink).hover(|style| style.bg(look.color(token::BG_HOVER))), ink)
        }
        K::Ghost => {
            let ink = look.color(token::FG_MUTED);
            (base.text_color(ink).hover(|style| style.bg(look.color(token::BG_HOVER)).text_color(look.color(token::FG_DEFAULT))), ink)
        }
    };
    styled.child(glyph(look, icon, Size::Large, ink)).when(words, |button| button.child(said))
}

impl JobView {
    /// The Job's first open ask, in Fleet's order. None where nothing asks.
    pub(super) fn first_ask(&self, job: &str, cx: &gpui::App) -> Option<Item> {
        self.job_asks(job, None, cx).into_iter().next()
    }

    /// The open ask a plan's or sketch's own card stands for, `item` as its card names it: while there is one, the first card
    /// is the only place it is answered, and its own card is a line.
    pub(super) fn open_ask_of(&self, job: &str, item: &str, cx: &gpui::App) -> Option<Item> {
        let plan = item.starts_with("plan:");
        self.job_asks(job, None, cx).into_iter().find(|one| super::ask_line::same_item(&one.id, item) || (plan && one.kind == Kind::Plan))
    }

    /// The first card at list place `ix`: the bar of steps, a hairline, the question and why it is first, then its answers.
    pub(super) fn first_card(&self, ix: usize, page: &Page, look: &Look, cx: &Context<Self>) -> Option<AnyElement> {
        let job = self.shown.clone()?;
        let record = self.record(&job, cx)?;
        let asks = self.job_asks(&job, None, cx);
        let first = asks.first()?.clone();
        let ask = self.ask_of(&first);
        // One he answered stands done on the bar while the asks put with or before it are still open.
        let since = asks.iter().map(|one| instant(&one.since)).min().unwrap_or(i64::MAX);
        let done = record.answers.iter().filter(|one| instant(one.at.as_str()) >= since).max_by_key(|one| instant(one.at.as_str())).map(|one| answered_words(one).to_string());
        let words: Vec<(Ask, String)> = asks
            .iter()
            .map(|one| {
                let kind = self.ask_of(one);
                // A question with options is a pick, as the takes of an artifact are.
                let picks = one.kind == Kind::Question && record.waiting_for.iter().any(|row| row.id == one.id && !row.options.is_empty());
                (kind, step_words(if picks { Ask::Takes } else { kind }, one.read).to_string())
            })
            .collect();
        let bar = self.step_bar(&steps(done, &words), look, cx);
        let (_, why) = self.fleet_place(&job, &first, cx);
        let why = why.trim().to_string();
        let key = first_key(&first);
        let head = div()
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_1))
            .px(look.length(token::SPACE_4))
            .pt(look.length(token::SPACE_4))
            .child(labelled(
                div()
                    .id(SharedString::from(format!("first-question:{}", first.id)))
                    .text_size(look.length(token::TEXT_LG))
                    .line_height(look.length(token::LEADING_LG))
                    .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                    .text_color(look.color(token::FG_DEFAULT))
                    .child(SharedString::from(first.name.clone())),
                Role::Heading,
                first.name.clone(),
            ))
            .children((!why.is_empty()).then(|| {
                labelled(
                    div()
                        .id(SharedString::from(format!("first-why:{}", first.id)))
                        .text_size(look.length(token::TEXT_SM))
                        .line_height(look.length(token::LEADING_SM))
                        .text_color(look.color(token::FG_MUTED))
                        .child(SharedString::from(why.clone())),
                    Role::Label,
                    why.clone(),
                )
            }));
        let padded = |children: Vec<AnyElement>| div().flex().flex_col().gap(look.length(token::SPACE_3)).px(look.length(token::SPACE_4)).pt(look.length(token::SPACE_3)).pb(look.length(token::SPACE_4)).children(children).into_any_element();
        let body = match first.kind {
            Kind::Question => {
                let waiting = record.waiting_for.iter().find(|one| one.id == first.id)?;
                padded(self.question_body(ix, waiting, &first.name, page, look, cx))
            }
            Kind::Artifact => self.parts_inside(ix, artifact_in(&first)?, page, look, cx)?,
            _ => padded(vec![self.first_answers(&first, cx)]),
        };
        let spot = page.keys.spot(&format!("card:{key}"), cx);
        let aim = key.clone();
        let card = div()
            .id(SharedString::from(format!("first:{}", first.id)))
            .flex()
            .flex_col()
            .min_w_0()
            .when(self.band_narrow(look), |card| card.mx(-(look.length(token::SPACE_4) + look.length(token::SPACE_1))))
            .overflow_hidden()
            .rounded(look.length(token::RADIUS_LG))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::STEP_WAITING))
            .bg(look.color(token::BG_RAISED))
            .track_focus(&spot.focus)
            .on_mouse_down(gpui::MouseButton::Left, cx.listener(move |this, _, _, _| this.aim(&aim)))
            .child(bar)
            .child(div().flex_none().w_full().h(look.length(token::BORDER_WIDTH)).bg(look.color(token::BORDER_SUBTLE)))
            .child(head)
            .child(body);
        // An artifact keeps the name its card in the thread had, so a walk or a reader finds it by the same words.
        let named = match artifact_in(&first).and_then(|artifact| self.artifact_of(&job, artifact)).filter(|_| first.kind == Kind::Artifact) {
            Some(held) => format!("Artifact: {}, Revision {}", held.title, held.revisions.last().map_or(1, |last| last.number)),
            None => format!("{}: {}", ask.mark().1, first.name),
        };
        Some(labelled(card, Role::Article, named).into_any_element())
    }

    /// The bar across the card's top: each step with a chevron between, a divider, then Open Review Sheet.
    fn step_bar(&self, steps: &[Step], look: &Look, cx: &Context<Self>) -> AnyElement {
        let chevron = || glyph(look, "chevron-right", Size::Small, look.color(token::BORDER_STRONG)).into_any_element();
        let roomy = self.pane.is_none_or(|width| width >= look.length(token::SPACE_12) * 13.);
        let mut row: Vec<AnyElement> = Vec::new();
        for (at, step) in steps.iter().enumerate() {
            if at > 0 {
                row.push(chevron());
            }
            row.push(self.step(at, step, roomy, look, cx));
        }
        let divider = div().flex_none().w(look.length(token::BORDER_WIDTH)).h(look.length(token::SPACE_6)).bg(look.color(token::BORDER_SUBTLE));
        let open = labelled(
            // Where the thread is too narrow for the steps' words and the button's, the steps keep the room and the button is its glyph.
            pressed(glyph_button(look, "first-review".into(), "Open Review Sheet", "layout-grid", super::sketch_card::Kind::Ghost, roomy).tooltip(tip(if roomy { "Every ask of this Job, in order" } else { "Open Review Sheet" })), cx, |this, window, cx| {
                this.open_first_review(window, cx)
            }),
            Role::Button,
            "Open Review Sheet",
        );
        labelled(
            div()
                .id("first-steps")
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_1))
                .p(look.length(token::SPACE_2))
                .children(row)
                .child(divider)
                .child(open),
            Role::List,
            "Steps",
        )
        .into_any_element()
    }

    /// One step: its glyph and its words, the ask now tinted in the waiting hue. In a narrow thread the step now keeps its words, a
    /// step after it is its glyph with its words in the tooltip, and `N more` is its words.
    fn step(&self, at: usize, step: &Step, roomy: bool, look: &Look, cx: &Context<Self>) -> AnyElement {
        let waiting = look.color(token::STEP_WAITING);
        let muted = look.color(token::FG_MUTED);
        let (icon, ink, words, said, state, now) = match step {
            Step::Done(words) => ("check", look.color(token::STATUS_COMPLETED_SUCCESS), words.clone(), format!("Done: {words}"), "Answered", false),
            Step::Now(ask, words) => (ask.mark().0, waiting, words.clone(), format!("Now: {words}"), "Now", true),
            Step::Next(ask, words) => (ask.mark().0, muted, words.clone(), format!("Next: {words}"), "Next", false),
            Step::More(more) => ("list-ordered", muted, format!("{more} more"), format!("{more} more"), "More asks after these", false),
        };
        let more = matches!(step, Step::More(_));
        let (show_glyph, show_words) = if roomy || now { (true, true) } else { (!more, more) };
        let tooltip = if show_words { state.to_string() } else { said.clone() };
        let cell = div()
            .id(SharedString::from(format!("first-step:{at}")))
            .when(roomy || now, |cell| cell.flex_1())
            .when(!roomy && !now, |cell| cell.flex_none())
            .when(!now, |cell| cell.min_w_0())
            .flex()
            .items_center()
            .justify_center()
            .gap(look.length(token::SPACE_2))
            .p(look.length(token::SPACE_2))
            .rounded(look.length(token::RADIUS_MD))
            .when(now, |cell| cell.bg(look.color(token::STATUS_AWAITING_REVIEW_BG)))
            .tooltip(tip(tooltip))
            .when(show_glyph, |cell| cell.child(glyph(look, icon, Size::Large, ink)))
            .when(show_words, |cell| {
                cell.child(
                    div()
                        .when(now, |words| words.whitespace_nowrap())
                        .when(!now, |words| words.min_w_0().truncate())
                        .text_size(look.length(token::TEXT_SM))
                        .line_height(look.length(token::LEADING_SM))
                        .font_weight(FontWeight(look.number(if now { token::WEIGHT_HEADING } else { token::WEIGHT_BODY })))
                        .text_color(if now { waiting } else { muted })
                        .child(SharedString::from(words)),
                )
            });
        if more {
            let cell = cell.cursor_pointer().hover(|style| style.bg(look.color(token::BG_HOVER))).tab_index(0).focus_visible(|style| style.border_color(look.color(token::ACCENT)));
            return labelled(pressed(cell, cx, |this, window, cx| this.open_first_review(window, cx)), Role::Button, said).into_any_element();
        }
        labelled(cell, Role::ListItem, said).into_any_element()
    }

    /// Open Review Sheet and `N more`: the sheet on this Job's asks, the first chosen.
    pub(super) fn open_first_review(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(job) = self.shown.clone() else { return };
        match self.first_ask(&job, cx) {
            Some(first) => self.open_review_on(&first.id, window, cx),
            None => self.open_review(window, cx),
        }
    }

    /// The answers of an ask the sheet answers by verdict, a plan, a drawing, a page or a draft, and of a build: Approve, Ask for
    /// changes and Read it, as the ask offers them.
    fn first_answers(&self, item: &Item, cx: &Context<Self>) -> AnyElement {
        let look = cx.look();
        let asks = !item.read && self.item_asks(item, cx) && item.kind != Kind::Build;
        let id = item.id.clone();
        let approve = asks.then(|| {
            let held = item.clone();
            labelled(
                pressed(glyph_button(look, format!("first-approve:{id}"), "Approve", "check", super::sketch_card::Kind::Primary, true), cx, move |this, window, cx| this.first_approve(held.clone(), window, cx)),
                Role::Button,
                "Approve",
            )
        });
        let changes = (asks && item.send_back).then(|| {
            let on = id.clone();
            labelled(
                pressed(glyph_button(look, format!("first-changes:{id}"), "Ask for changes", "pencil", super::sketch_card::Kind::Secondary, true).tooltip(tip("Write what to change in the review sheet")), cx, move |this, window, cx| {
                    this.open_review_on(&on, window, cx);
                    this.review_open_changes(window, cx);
                }),
                Role::Button,
                "Ask for changes",
            )
        });
        // A page is read where it opens; anything else is read whole in the sheet, where a build's own answers are.
        let page = page_of_item(&item.id).map(str::to_string);
        let (said, icon, hint): (&'static str, &'static str, &'static str) = match (&page, item.kind) {
            (Some(_), _) => ("Read it", "book-open", "Open the page"),
            (None, Kind::Build) => ("Try it", "panel-right-open", "Try it and answer in the review sheet"),
            _ => ("Read it", "book-open", "Read it whole in the review sheet"),
        };
        let read = {
            let on = id.clone();
            let kind = if approve.is_none() { super::sketch_card::Kind::Primary } else { super::sketch_card::Kind::Ghost };
            labelled(
                pressed(glyph_button(look, format!("first-read:{id}"), said, icon, kind, true).tooltip(tip(hint)), cx, move |this, window, cx| match &page {
                    Some(page) => this.open_document(page, window, cx),
                    None => this.open_review_on(&on, window, cx),
                }),
                Role::Button,
                said,
            )
        };
        div().flex().flex_wrap().items_center().gap(look.length(token::SPACE_2)).children(approve).children(changes).child(read).into_any_element()
    }

    /// Approve on the first card: answered as the sheet's Approve answers it, and the next ask takes the card.
    fn first_approve(&mut self, item: Item, window: &mut Window, cx: &mut Context<Self>) {
        let body = answer_batch(&[&item], ItemHow::Approve, |_| String::new());
        self.review_send(vec![item], Verdict::Approved, "/sessions/items/answer", ipc::encode(&body), window, cx);
    }
}

#[cfg(test)]
mod tests {
    use super::{steps, Step};
    use crate::job_view::parts_fold::{drawn, drawn_like, redraw, rooted_job};
    use crate::job_view::review::{in_fleet_order, Ask, Item, Kind};
    use crate::job_view::JobView;
    use gpui::{px, size, Role};

    const FIRST: &str = "Which colour does the stale chip take?";
    const SECOND: &str = "Where does the chip show how far behind it is?";

    fn asks(n: usize) -> Vec<(Ask, String)> {
        (0..n).map(|at| (Ask::Question, format!("Ask {at}"))).collect()
    }

    fn press(cx: &mut gpui::VisualTestContext, role: Role, name: &str) {
        let at = cx
            .update(|window, cx| cx.global::<crate::labels::LabelBook>().find(window.window_handle().window_id(), role, name).map(|one| one.bounds.center()))
            .unwrap_or_else(|| panic!("{name} was not drawn"));
        cx.simulate_click(at, gpui::Modifiers::none());
        redraw(cx);
    }

    fn thread_shown(cx: &mut gpui::TestAppContext) -> (gpui::Entity<JobView>, &mut gpui::VisualTestContext) {
        let (view, cx) = rooted_job(cx, "job-view", "mock-01", size(px(1440.), px(900.)), |_| {});
        cx.update(|window, cx| view.update(cx, |view, cx| view.set_needs(false, None, window, cx)));
        redraw(cx);
        (view, cx)
    }

    fn ask_ids(view: &gpui::Entity<JobView>, cx: &mut gpui::VisualTestContext) -> Vec<String> {
        view.read_with(cx, |view, cx| view.job_asks("mock-01", None, cx).into_iter().map(|one| one.id).collect())
    }

    #[gpui::test]
    fn only_the_first_ask_has_a_card_and_answering_it_brings_the_next(cx: &mut gpui::TestAppContext) {
        let (_, cx) = thread_shown(cx);
        assert!(drawn(cx, Role::Article, &format!("Question: {FIRST}")), "the Job's first ask is a card");
        assert!(!drawn(cx, Role::Article, &format!("Question: {SECOND}")), "the second has none while the first is open");
        assert!(drawn(cx, Role::Label, "The chip's colour waits on your pick"), "why it is first stands under the question");
        press(cx, Role::Button, "Amber");
        assert!(!drawn(cx, Role::Article, &format!("Question: {FIRST}")), "answered, it leaves the thread");
        assert!(drawn(cx, Role::Article, &format!("Question: {SECOND}")), "and the next takes its card");
        assert!(drawn(cx, Role::Label, "The tooltip's words wait on where the count shows"), "with its own why");
    }

    #[gpui::test]
    fn a_first_ask_that_is_a_plan_stands_once_as_the_first_card(cx: &mut gpui::TestAppContext) {
        let (view, cx) = crate::job_view::parts_fold::rooted_job_with(
            cx,
            "job-view",
            "mock-01",
            size(px(1440.), px(900.)),
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
        cx.update(|window, cx| view.update(cx, |view, cx| view.set_needs(false, None, window, cx)));
        redraw(cx);
        let first = view.read_with(cx, |view, cx| view.first_ask("mock-01", cx)).expect("an ask is open");
        assert_eq!(first.kind, Kind::Plan, "with the questions answered the plan is first");
        let extras = view.read_with(cx, |view, _| view.pages.get("mock-01").map(|page| page.extras.clone()).unwrap_or_default());
        assert!(extras.iter().all(|extra| !matches!(extra, crate::job_view::Extra::Plan(_))), "the plan's own card does not stand: {extras:?}");
        assert_eq!(extras.iter().filter(|extra| matches!(extra, crate::job_view::Extra::First(_))).count(), 1, "the first card does: {extras:?}");
        assert!(!drawn(cx, Role::Article, "Plan to approve"), "nothing else draws the plan as a card");
        assert_eq!(drawn_like(cx, Role::Article, "Plan: ").len(), 1, "the plan stands once, as the first card");
        assert!(drawn(cx, Role::ListItem, "Now: Approve the plan"), "its bar names it now");
    }

    #[gpui::test]
    fn the_step_bar_names_the_ask_now_the_next_and_how_many_more(cx: &mut gpui::TestAppContext) {
        let (view, cx) = thread_shown(cx);
        let open = ask_ids(&view, cx);
        assert!(open.len() > 3, "the fixture has more than three asks open: {open:?}");
        assert!(drawn(cx, Role::List, "Steps"), "the bar heads the card");
        assert!(drawn(cx, Role::ListItem, "Now: Pick one"), "a question with options is a pick");
        assert!(drawn(cx, Role::ListItem, "Next: Pick one"), "the next question is named next");
        assert!(drawn(cx, Role::Button, &format!("{} more", open.len() - 2)), "the rest are counted: {:?}", drawn_like(cx, Role::Button, ""));
        press(cx, Role::Button, &format!("{} more", open.len() - 2));
        assert!(view.read_with(cx, |view, _| view.review_open()), "a press on the count opens the sheet");
    }

    #[gpui::test]
    fn open_review_sheet_opens_the_sheet_on_the_first_ask(cx: &mut gpui::TestAppContext) {
        let (view, cx) = thread_shown(cx);
        press(cx, Role::Button, "Open Review Sheet");
        let chosen = view.read_with(cx, |view, _| view.review_open().then(|| view.review_chosen()).flatten().map(|item| item.id));
        assert_eq!(chosen.as_deref(), Some("ask:call-1"), "the sheet opens on the ask the card holds");
    }

    #[gpui::test]
    fn the_sheet_lists_the_asks_in_fleets_order_each_with_its_why(cx: &mut gpui::TestAppContext) {
        let (view, cx) = thread_shown(cx);
        let open = ask_ids(&view, cx);
        assert_eq!(open.get(..3), Some(&["ask:call-1".to_string(), "ask:call-1#1".into(), "plan:2".into()][..]), "the Job's own order first, then Fleet's");
        press(cx, Role::Button, "Open Review Sheet");
        assert!(drawn(cx, Role::Label, "The tooltip's words wait on where the count shows"), "a row's second line is its why");
        assert!(drawn(cx, Role::Label, "The work waits on your answer"), "Fleet's why where the Job named none");
    }

    fn item(id: &str, since: &str) -> Item {
        Item {
            job: "mock-01".into(),
            id: id.into(),
            kind: Kind::Question,
            name: id.into(),
            since: since.into(),
            for_line: None,
            revision: None,
            send_back: false,
            read: false,
            artifact: None,
        }
    }

    #[test]
    fn the_sheet_keeps_fleets_places_and_puts_an_unplaced_ask_last_by_age() {
        let mut items = vec![item("young", "2026-11-09T14:00:00Z"), item("second", "2026-11-09T13:00:00Z"), item("loose", "2026-11-09T12:00:00Z"), item("first", "2026-11-09T15:00:00Z")];
        in_fleet_order(&mut items, &[(0, String::new()), (2, String::new()), (0, String::new()), (1, String::new())]);
        let ids: Vec<&str> = items.iter().map(|one| one.id.as_str()).collect();
        assert_eq!(ids, ["first", "second", "loose", "young"]);
    }

    #[test]
    fn the_bar_names_the_ask_now_the_next_and_how_many_more() {
        assert_eq!(steps(None, &asks(1)), [Step::Now(Ask::Question, "Ask 0".into())]);
        assert_eq!(steps(None, &asks(3)), [Step::Now(Ask::Question, "Ask 0".into()), Step::Next(Ask::Question, "Ask 1".into()), Step::Next(Ask::Question, "Ask 2".into())], "two after the ask now are both named");
        assert_eq!(steps(None, &asks(7)), [Step::Now(Ask::Question, "Ask 0".into()), Step::Next(Ask::Question, "Ask 1".into()), Step::More(5)], "seven read now, next and 5 more");
        assert_eq!(
            steps(Some("Pick one".into()), &asks(2)),
            [Step::Done("Pick one".into()), Step::Now(Ask::Question, "Ask 0".into()), Step::Next(Ask::Question, "Ask 1".into())],
            "the one answered stands done before the ask now"
        );
        assert!(steps(Some("Pick one".into()), &[]).iter().all(|step| matches!(step, Step::Done(_))), "nothing open leaves no step now");
    }
}
