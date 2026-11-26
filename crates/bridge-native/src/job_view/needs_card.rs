//! A question the Job asks, drawn in Needs you on the Operator's own card (`operator/card.rs`) while he answers with
//! buttons (take A, owner, 18 Nov). One question stands in front and the others wait behind it; the stepper at the
//! card's head brings the one before or after. A press on an option, or its number, answers at once as the band's
//! buttons did (`answer_row`). Discuss stands where Answer in words stood: the panel folds and the caret goes in the
//! Job's own box, which then answers the question (`dock.answering`) and sends nothing until he does.
//! A question the review sheet holds is one line in Needs you (`ask_line`): it brings the thread to the question's card
//! where the thread has one, and opens the sheet on it where not. The card draws only a question the sheet does not hold.

use gpui::{div, prelude::*, AnyElement, Context, FontWeight, Role, SharedString, Window};
use ipc::{NeedsYouAct, NeedsYouDismiss, NeedsYouRow, WaitingSource};

use super::dock::{row_options, RowBoxFor, Thing};
use super::rows::{glyph, tip, Size};
use super::JobView;
use crate::labels::labelled;
use crate::look::{token, Look};
use crate::operator::card::{Act, Card, Offer};
use crate::operator::face;
use crate::operator::view::{chip, press, pressed, Weight};
use crate::ways;

/// Whether Needs you draws a row on the card: a question with options, while he answers with buttons. A handoff keeps
/// its own card, and a question asked in pictures its stills.
pub(super) fn on_card(row: &NeedsYouRow) -> bool {
    ways::answers_with_buttons()
        && matches!(row.act, NeedsYouAct::Answer { .. })
        && !matches!(row.source, WaitingSource::Handoff | WaitingSource::ReadyToTry)
        && !row_options(row).is_empty()
        && row_options(row).iter().all(|option| option.picture.is_none())
}

/// Whether the row is the Approve that ends the Job's step: an item the agent raised with `approves`, on a step whose end is that
/// Approve (`ends_when` says `you approve`). The agent's word never ends a step; his press on this card does.
pub(super) fn is_step_done(job: &ipc::SessionRecord, row: &str) -> bool {
    let ends_on_it = job.workflow_strip.as_ref().is_some_and(|strip| strip.ends_when.iter().any(|end| end == "you approve"));
    ends_on_it && job.waiting_for.iter().any(|item| item.id == row && item.source == WaitingSource::Agent && item.approves)
}

/// The question `by` steps from `at` among `count`, held at either end.
pub(super) fn stepped(at: usize, by: isize, count: usize) -> usize {
    (at as isize + by).clamp(0, count.saturating_sub(1) as isize) as usize
}

impl JobView {
    /// The questions the card holds, in Fleet's order.
    pub(super) fn card_questions<'a>(&'a self, id: &str, cx: &'a gpui::App) -> Vec<&'a NeedsYouRow> {
        self.things(id, cx)
            .0
            .into_iter()
            .filter_map(|thing| match thing {
                Thing::Row(row) if on_card(row) => Some(row),
                _ => None,
            })
            .collect()
    }

    /// The question in front: the one stepped to while it still waits, else the first.
    pub(super) fn front_question<'a>(&'a self, id: &str, cx: &'a gpui::App) -> Option<&'a NeedsYouRow> {
        let questions = self.card_questions(id, cx);
        let chosen = self.dock_of(id).and_then(|dock| dock.front.as_deref());
        questions.iter().find(|row| Some(row.id.as_str()) == chosen).or(questions.first()).copied()
    }

    /// The stepper: the question before (`by` -1) or after (1) comes to the front, and the keys are the panel's, so a
    /// number answers it.
    pub(super) fn step_card_question(&mut self, by: isize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(id) = self.shown.clone() else { return };
        let to = {
            let questions = self.card_questions(&id, cx);
            let front = self.front_question(&id, cx).map(|row| row.id.clone());
            let at = questions.iter().position(|row| Some(&row.id) == front.as_ref()).unwrap_or(0);
            questions.get(stepped(at, by, questions.len())).map(|row| row.id.clone())
        };
        if let Some(dock) = self.dock_mut() {
            dock.front = to;
        }
        window.focus(&self.dock_focus, cx);
        cx.notify();
    }

    /// Discuss: the panel folds and the caret is in the Job's box, which answers the question. Nothing is sent.
    pub(super) fn discuss_question(&mut self, item: &str, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(dock) = self.dock_mut() {
            dock.answering = Some(item.to_string());
            dock.open = Some(false);
            dock.detail = None;
            dock.choosing = None;
        }
        self.to_composer(window, cx);
        cx.notify();
    }

    /// Where the card in front stands among the questions, and the way to the one before and after: drawn as the band's
    /// own stepper is, with more than one question.
    fn question_stepper(&self, at: usize, count: usize, look: &Look, cx: &Context<Self>) -> Option<AnyElement> {
        if count < 2 {
            return None;
        }
        let ink = look.color(token::FG_MUTED);
        let ring = look.edge(token::FOCUS_RING).map_or(look.color(token::ACCENT), |(_, colour)| colour);
        let step = |name: &'static str, said: &'static str, icon: &'static str, by: isize, on: bool| {
            let key = div()
                .id(name)
                .flex_none()
                .flex()
                .items_center()
                .justify_center()
                .size(look.length(token::H_CONTROL_SM))
                .rounded(look.length(token::RADIUS_MD))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(gpui::transparent_black())
                .tooltip(tip(said))
                .child(glyph(look, icon, Size::Small, ink));
            let key = if on {
                pressed(key.cursor_pointer().hover(|style| style.bg(look.color(token::BG_HOVER))).focusable().tab_stop(true).focus_visible(move |style| style.border_color(ring)), cx, move |this, window, cx| this.step_card_question(by, window, cx))
            } else {
                key.opacity(0.4)
            };
            labelled(key, Role::Button, said)
        };
        let said = format!("{} of {count}", at + 1);
        Some(
            div()
                .flex_none()
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_1))
                .child(step("needs-card-previous", "Previous question", "chevron-up", -1, at > 0))
                .child(labelled(div().id("needs-card-place").text_size(look.length(token::TEXT_XS)).font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM))).text_color(ink).child(SharedString::from(said.clone())), Role::Status, said))
                .child(step("needs-card-next", "Next question", "chevron-down", 1, at + 1 < count))
                .into_any_element(),
        )
    }

    /// The artifact the question names, as a chip at the card's head that opens it, or says it is no longer held.
    fn question_names(&self, id: &str, row: &NeedsYouRow, look: &Look, cx: &Context<Self>) -> Option<AnyElement> {
        let named = self.row_artifact(id, &row.id, cx)?;
        let title = self.artifact_of(id, &named).map_or_else(|| named.clone(), |one| one.title.clone());
        let element = div().id(SharedString::from(format!("needs-card-artifact:{}", row.id))).flex_none();
        if self.artifact_gone(id, &named) {
            let chip = chip("No longer held", look.color(token::FG_MUTED), look.color(token::BG_HOVER), look);
            return Some(labelled(element.child(chip), Role::Label, "No longer held").into_any_element());
        }
        let said = format!("Open {title}");
        let chip = chip(title, look.color(token::FG_DEFAULT), look.color(token::BG_HOVER), look).child(glyph(look, "file-text", Size::Small, look.color(token::FG_MUTED)));
        Some(
            labelled(
                pressed(element.cursor_pointer().tooltip(tip(said.clone())).child(chip), cx, move |this, window, cx| {
                    this.open_named(&named, window, cx);
                }),
                Role::Link,
                said,
            )
            .into_any_element(),
        )
    }

    /// The card's foot: Fleet's way of putting the question away, where it gives one, and Discuss.
    fn question_foot(&self, row: &NeedsYouRow, look: &Look, cx: &Context<Self>) -> Vec<AnyElement> {
        let item = row.id.clone();
        let away = match row.dismiss {
            NeedsYouDismiss::Plain | NeedsYouDismiss::Skip | NeedsYouDismiss::Reason => {
                let (word, icon) = if row.dismiss == NeedsYouDismiss::Skip { ("Skip", None) } else { ("Remove", Some("x")) };
                let reason = row.dismiss == NeedsYouDismiss::Reason;
                let item = item.clone();
                Some(
                    labelled(
                        pressed(press(format!("needs-card-away:{}", row.id), word.into(), icon, None, Weight::Bare, look).tooltip(tip(if reason { "Remove, with a reason" } else { word })), cx, move |this, window, cx| {
                            if reason {
                                this.open_row_box(&item, RowBoxFor::Reason, window, cx);
                            } else {
                                this.dismiss_row(&item, None, false, window, cx);
                            }
                        }),
                        Role::Button,
                        word,
                    )
                    .into_any_element(),
                )
            }
            NeedsYouDismiss::None | NeedsYouDismiss::NotAQuestion => None,
        };
        let discuss = labelled(
            pressed(press(format!("needs-card-discuss:{}", row.id), "Discuss".into(), Some("message-square"), None, Weight::Pick, look).tooltip(tip("Answer in your own words in the message box")), cx, move |this, window, cx| {
                this.discuss_question(&item, window, cx)
            }),
            Role::Button,
            "Discuss",
        );
        away.into_iter().chain([discuss.into_any_element()]).collect()
    }

    /// A question on the card. `stepper` puts where it stands among the others at its head, as the list draws the one in
    /// front; opened whole, the panel's own head has Back.
    pub(super) fn needs_card(&self, id: &SharedString, row: &NeedsYouRow, stepper: bool, look: &Look, cx: &Context<Self>) -> AnyElement {
        let options = row_options(row);
        let pick = ways::pick_of(options);
        let offers = options
            .iter()
            .enumerate()
            .map(|(at, option)| {
                let item = row.id.clone();
                let act: Act<Self> = std::rc::Rc::new(move |this, window, cx| this.answer_row(&item, at, window, cx));
                Offer { label: ways::name_of(&option.label).to_string().into(), cost: ways::button_tip(options, at).map(SharedString::from), pick: pick == Some(at), act }
            })
            .collect();
        let questions = self.card_questions(id, cx);
        let at = questions.iter().position(|one| one.id == row.id).unwrap_or(0);
        let misread = (stepper && row.dismiss == NeedsYouDismiss::NotAQuestion).then(|| {
            let confirmed = self.dock_of(id).is_some_and(|dock| dock.confirmed.contains(&row.id)) || self.record(id, cx).and_then(|record| record.waiting_for.iter().find(|one| one.id == row.id)).is_some_and(|one| one.confirmed);
            self.thumbs(&row.id, confirmed, self.dock_focus.clone(), look, cx)
        });
        let card = Card {
            id: row.id.clone(),
            kind: if self.record(id, cx).is_some_and(|job| is_step_done(job, &row.id)) { face::STEP_DONE } else { face::QUESTION },
            title: row.text.clone().into(),
            names: self.question_names(id, row, look, cx).into_iter().chain(misread).collect(),
            stepper: if stepper { self.question_stepper(at, questions.len(), look, cx) } else { None },
            asks: None,
            read: None,
            returned: None,
            offers,
            keyed: self.keyed_row(id, cx).is_some_and(|(item, _)| item == row.id),
            focus: Vec::new(),
            lifted: None,
            pending: false,
            refused: self.dock_of(id).and_then(|dock| dock.said.get(&format!("item:{}", row.id)).cloned()),
            foot: self.question_foot(row, look, cx),
            inset: false,
            pinned: None,
        }
        .draw(look, cx);
        match self.row_box_under(id, &row.id, look, cx) {
            Some(reason) => div().flex().flex_col().gap(look.length(token::SPACE_2)).child(card).child(reason).into_any_element(),
            None => card,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::parts_fold::{drawn, redraw};
    use super::super::JobView;
    use super::stepped;
    use gpui::{prelude::*, Bounds, Pixels, Role};

    /// The Job that asks two questions, while he answers with buttons.
    const JOB: &str = "mock-01";
    const FIRST: &str = "The old /v1/sessions endpoint has no callers left. What happens to it?";
    const SECOND: &str = "Land on a branch or a pull request?";
    /// The Job's pick, named as a reader hears it: its label, then what it does.
    const REDIRECT: &str = "Redirect it, The old path answers with a redirect to the new one. No caller breaks, and it stays.";

    /// The Job on screen with its Needs you open.
    fn opened(cx: &mut gpui::TestAppContext) -> (gpui::Entity<JobView>, &mut gpui::VisualTestContext) {
        opened_with(cx, |_| {})
    }

    /// The question the owner met: the pick first, and no option saying what it costs, as Fleet keeps an agent's labels,
    /// with Fleet's walk waiting beside it.
    const PICK: &str = "File it and fix it now";
    fn labels_only(jobs: &mut [ipc::SessionRecord]) {
        let job = jobs.iter_mut().find(|job| job.id.as_str() == JOB).expect("the Job");
        let question = job.waiting_for.iter_mut().find(|one| one.id == "agent-1").expect("the first question");
        question.options = ["File it and fix it now (Recommended)", "Fix it now", "File it for later"].into_iter().map(|label| ipc::WaitingOption { label: label.into(), description: None, picture: None }).collect();
        let mut walk = question.clone();
        walk.id = "walk:http://127.0.0.1:47811/".into();
        walk.text = "http://127.0.0.1:47811/".into();
        walk.source = ipc::WaitingSource::Walk;
        walk.options = Vec::new();
        walk.act = Some(ipc::WaitingAct { kind: ipc::WaitingActKind::Walk, target: walk.text.clone(), on: None });
        job.waiting_for.push(walk);
    }

    /// The Job on screen with its Needs you open, its records changed first by `change`.
    fn opened_with(cx: &mut gpui::TestAppContext, change: impl FnOnce(&mut [ipc::SessionRecord])) -> (gpui::Entity<JobView>, &mut gpui::VisualTestContext) {
        cx.update(|cx| {
            gpui_component::init(cx);
            crate::look::init(cx);
        });
        let mut jobs = crate::mock::jobs("ways-buttons").expect("the fixture decodes");
        change(&mut jobs);
        let threads = crate::mock::threads("ways-buttons").expect("the threads decode");
        let list = cx.new(|cx| crate::jobs::JobsList::mock(&jobs, cx));
        let (view, cx) = cx.add_window_view({
            let list = list.clone();
            move |window, cx| JobView::new(list, Some(threads), window, cx)
        });
        cx.simulate_resize(gpui::size(gpui::px(1400.), gpui::px(1200.)));
        list.update(cx, |list, cx| list.open(JOB.into(), cx));
        redraw(cx);
        view.update_in(cx, |view, window, cx| view.set_needs(true, None, window, cx));
        fresh(cx);
        (view, cx)
    }

    /// Draw again with the book emptied first, so a name drawn before stands in it no longer.
    fn fresh(cx: &mut gpui::VisualTestContext) {
        redraw(cx);
        cx.update(|_, cx| cx.set_global(crate::labels::LabelBook::default()));
        cx.update(|window, cx| window.draw(cx).clear(cx));
    }

    fn bounds_of(cx: &mut gpui::VisualTestContext, role: Role, name: &str) -> Bounds<Pixels> {
        cx.update(|window, cx| cx.global::<crate::labels::LabelBook>().drawn(window.window_handle().window_id()).iter().rev().find(|one| one.role == role && one.name == name).map(|one| one.bounds))
            .unwrap_or_else(|| panic!("{name} was not drawn"))
    }

    fn press(cx: &mut gpui::VisualTestContext, name: &str) {
        let at = bounds_of(cx, Role::Button, name).center();
        cx.simulate_click(at, gpui::Modifiers::none());
        fresh(cx);
    }

    /// The agent's Approve on a step that ends on it reads as the step done; the same item on a step that ends otherwise, or any other item, is a question.
    #[test]
    fn the_approve_a_step_ends_on_reads_as_the_step_done_and_no_other_item_does() {
        let mut job = crate::mock::jobs("ways-buttons").expect("the fixture decodes").into_iter().find(|job| job.id.as_str() == JOB).expect("the Job");
        let item = job.waiting_for.iter().position(|one| one.id == "agent-1").expect("the first question");
        assert!(!super::is_step_done(&job, "agent-1"), "a question of the Job's own, on a Job with no step reading");
        job.workflow_strip = Some(ipc::WorkflowStrip { ends_when: vec!["you approve".into()], ..ipc::WorkflowStrip::default() });
        assert!(!super::is_step_done(&job, "agent-1"), "the item asks no Approve");
        job.waiting_for[item].approves = true;
        assert!(super::is_step_done(&job, "agent-1"), "his press on it ends the step");
        assert!(!super::is_step_done(&job, "agent-2"), "another item is not it");
        job.workflow_strip = Some(ipc::WorkflowStrip { ends_when: vec!["you approve the plan".into()], ..ipc::WorkflowStrip::default() });
        assert!(!super::is_step_done(&job, "agent-1"), "a step that ends on something else is not ended by it");
        job.workflow_strip = Some(ipc::WorkflowStrip { ends_when: vec!["you approve".into()], ..ipc::WorkflowStrip::default() });
        job.waiting_for[item].source = ipc::WaitingSource::Walk;
        assert!(!super::is_step_done(&job, "agent-1"), "only the agent's own item");
    }

    #[gpui::test]
    /// One ask, one place (19 Nov): the review sheet answers a question, and Needs you lists it as one line that opens
    /// the sheet on it. No card, option or stepper is drawn in the panel.
    fn a_question_in_needs_you_is_one_line_each_and_no_card_answers_it_there(cx: &mut gpui::TestAppContext) {
        let (_view, cx) = opened(cx);
        assert!(drawn(cx, Role::Button, FIRST), "the first question is its line");
        assert!(drawn(cx, Role::Button, SECOND), "and the second its own, not behind it");
        // The thread's first card answers the first question; the panel draws none of it.
        assert!(!in_panel(cx, Role::Article, FIRST), "no card");
        assert!(!in_panel(cx, Role::Group, "Options"), "no options");
        assert!(!in_panel(cx, Role::Button, REDIRECT), "no option to press");
        assert!(!drawn(cx, Role::Status, "1 of 2"), "no stepper");
    }

    /// Whether `name` is drawn inside the panel's list of questions.
    fn in_panel(cx: &mut gpui::VisualTestContext, role: Role, name: &str) -> bool {
        let panel = bounds_of(cx, Role::Region, "Questions");
        cx.update(|window, cx| cx.global::<crate::labels::LabelBook>().drawn(window.window_handle().window_id()).iter().any(|one| one.role == role && one.name.as_ref() == name && panel.contains(&one.bounds.center())))
    }

    /// The Job's pick, among labels with no word on what each costs, is its first ask: a press on the line folds the panel
    /// and brings the thread to its card, and the panel draws no option of it.
    #[gpui::test]
    fn a_press_on_the_line_of_the_first_question_brings_the_thread_to_its_card(cx: &mut gpui::TestAppContext) {
        let (view, cx) = opened_with(cx, labels_only);
        assert!(!in_panel(cx, Role::Button, PICK), "the pick is not pressed in the panel");
        press(cx, FIRST);
        assert!(!view.read_with(cx, |view, _| view.review_open()), "the sheet stays shut while the thread holds the card");
        assert!(drawn(cx, Role::Article, &format!("Question: {FIRST}")), "the thread stands at the question's card");
        view.read_with(cx, |view, _| assert!(view.dock_of(JOB).is_some_and(|dock| dock.hidden.is_empty()), "nothing is answered by the press"));
    }

    /// A question after the first has no card yet, so a press on its line opens the sheet on it.
    #[gpui::test]
    fn a_press_on_the_line_of_a_later_question_opens_the_sheet_on_it(cx: &mut gpui::TestAppContext) {
        let (view, cx) = opened(cx);
        press(cx, SECOND);
        let chosen = view.read_with(cx, |view, _| view.review_open().then(|| view.review_chosen()).flatten().map(|item| item.id));
        assert_eq!(chosen.as_deref(), Some("agent-2"), "the sheet opens on that question");
    }

    #[test]
    fn the_stepper_holds_at_either_end() {
        assert_eq!(stepped(0, 1, 3), 1);
        assert_eq!(stepped(2, 1, 3), 2, "the last has none after it");
        assert_eq!(stepped(0, -1, 3), 0, "the first has none before it");
        assert_eq!(stepped(0, 1, 0), 0);
    }
}
