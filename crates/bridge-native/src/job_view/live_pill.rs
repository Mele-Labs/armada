//! What the Job is doing now, as a pill in the composer's tool row while the thread is scrolled away from its end: the words its
//! row in the Jobs list uses (`jobs::now_of`), how long the call has run, and a ring that turns. A press takes the thread to its end.
//!
//! It stands in the box, not the thread, at a height the row already has, so showing or hiding it moves nothing the list measures.

use std::time::Duration;

use gpui::{div, prelude::*, Animation, AnimationExt, AnyElement, Context, Role, SharedString};
use ipc::JobLamp;

use super::rows::{elapsed_words, glyph, mono, tip, Size};
use super::runs::running_now;
use super::{JobView, Page};
use crate::labels::labelled;
use crate::look::{token, ActiveLook};

/// Whether the thread shows its end: following its tail, or the last element's top is in view.
fn at_end(page: &Page) -> bool {
    page.list.is_following_tail()
        || page.count.checked_sub(1).and_then(|last| page.list.bounds_for_item(last)).is_some_and(|bounds| bounds.top() < page.list.viewport_bounds().bottom())
}

impl JobView {
    /// What the Job is doing now, where its turn runs and it asks nothing of him.
    pub(super) fn live_words_of(&self, id: &str, cx: &gpui::App) -> Option<String> {
        if !self.working(id, cx) {
            return None;
        }
        let job = self.record(id, cx)?;
        if job.mark.is_some_and(|mark| mark.lamp == JobLamp::Needs) {
            return None;
        }
        crate::jobs::now_of(job)
    }

    /// What the pill says: what the Job is doing now, while the thread shows no end where its own running row says it.
    pub(super) fn pill_words(&self, id: &str, cx: &gpui::App) -> Option<String> {
        let page = self.pages.get(id)?;
        self.live_words_of(id, cx).filter(|_| !at_end(page))
    }

    /// The pill, one line cut with an ellipsis, its whole words in the tooltip. None while the thread shows its end, where its
    /// own running row says the same.
    pub(super) fn render_live_pill(&self, id: &SharedString, cx: &Context<Self>) -> Option<AnyElement> {
        let words = self.pill_words(id, cx)?;
        let since = self.thread(id, cx).and_then(|held| held.head.open_call.as_ref().map(|call| call.since.clone()));
        let look = cx.look();
        let said = SharedString::from(words);
        let (job, back) = (id.to_string(), id.clone());
        let time = since.map(|since| {
            div().id("live-pill-time").flex_none().font_family(mono(look)).text_color(look.color(token::FG_SUBTLE)).with_animation(
                "live-pill-count",
                Animation::new(Duration::from_secs(1)).repeat(),
                move |time, _| time.child(SharedString::from(format!("\u{b7} {}", elapsed_words(since.as_str(), &running_now(&job).to_rfc3339()).unwrap_or_default()))),
            )
        });
        Some(
            labelled(
                div()
                    .id("live-pill")
                    .flex()
                    // It takes the room the row has spare, down to its ring, so the other controls keep as many words as they can.
                    .flex_1()
                    .min_w(look.length(token::SPACE_4) + look.length(token::SPACE_2) * 2. + look.length(token::BORDER_WIDTH) * 2.)
                    .max_w(look.length(token::SPACE_12) * 6.)
                    .overflow_hidden()
                    .items_center()
                    .gap(look.length(token::SPACE_2))
                    .h(look.length(token::H_CONTROL_SM))
                    .px(look.length(token::SPACE_2))
                    .rounded(look.length(token::RADIUS_MD))
                    .border(look.length(token::BORDER_WIDTH))
                    .border_color(look.color(token::BORDER_DEFAULT))
                    .bg(look.color(token::BG_RAISED))
                    .text_size(look.length(token::TEXT_XS))
                    .text_color(look.color(token::FG_MUTED))
                    .cursor_pointer()
                    .hover(|style| style.bg(look.color(token::BG_HOVER)))
                    .tooltip(tip(said.clone()))
                    .child(self.working_mark(look, look.color(token::STATUS_RUNNING)))
                    .child(div().id("live-pill-words").flex_1().min_w_0().truncate().child(said.clone()))
                    .children(time)
                    .child(glyph(look, "arrow-down", Size::Small, look.color(token::FG_MUTED)))
                    // The press is the pill's alone, and puts the list at its tail as the End key does.
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.pin(&back);
                        window.focus(&this.focus, cx);
                        cx.notify();
                    })),
                Role::Button,
                SharedString::from(format!("Now: {said}")),
            )
            .into_any_element(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::job_view::parts_fold::{drawn, drawn_like, folds_job, redraw};
    use crate::mock_parts::JOB;
    use gpui::{px, Modifiers, VisualTestContext};

    const WORDS: &str = "Now: Reading page_votes.rs";

    /// A window short enough that the thread scrolls, as the owner's is.
    fn shorter(cx: &mut VisualTestContext) {
        cx.simulate_resize(gpui::size(px(1400.), px(900.)));
        redraw(cx);
    }

    fn scroll_up(view: &gpui::Entity<JobView>, cx: &mut VisualTestContext) {
        view.update(cx, |view, _| view.pages[JOB].list.scroll_to(gpui::ListOffset { item_ix: 0, offset_in_item: px(0.) }));
        redraw(cx);
    }

    fn says(view: &gpui::Entity<JobView>, cx: &mut VisualTestContext) -> Option<String> {
        view.read_with(cx, |view, cx| view.pill_words(JOB, cx))
    }

    #[gpui::test]
    fn the_pill_is_hidden_while_the_thread_shows_its_end(cx: &mut gpui::TestAppContext) {
        let (view, cx) = folds_job(cx, |_| {});
        shorter(cx);
        assert_eq!(says(&view, cx), None);
        assert!(drawn_like(cx, Role::Button, "Now: ").is_empty());
    }

    #[gpui::test]
    fn the_pill_shows_once_the_thread_is_scrolled_up_while_the_job_works(cx: &mut gpui::TestAppContext) {
        let (view, cx) = folds_job(cx, |_| {});
        shorter(cx);
        scroll_up(&view, cx);
        assert_eq!(says(&view, cx).as_deref(), Some("Reading page_votes.rs"));
        assert!(drawn(cx, Role::Button, WORDS), "scrolled up, the work stands in the box's tool row");
    }

    #[gpui::test]
    fn the_pill_is_hidden_when_the_job_is_idle(cx: &mut gpui::TestAppContext) {
        let (view, cx) = folds_job(cx, |jobs| {
            let one = jobs.iter_mut().find(|one| one.id.as_str() == JOB).expect("the folds' Job");
            one.mark = Some(ipc::JobMark { state: ipc::JobState::Idle, lamp: ipc::JobLamp::Idle, group: ipc::JobGroup::Held });
            one.hosted = None;
        });
        shorter(cx);
        scroll_up(&view, cx);
        assert_eq!(says(&view, cx), None);
        assert!(drawn_like(cx, Role::Button, "Now: ").is_empty());
    }

    #[gpui::test]
    fn the_pill_is_hidden_while_the_job_waits_on_him(cx: &mut gpui::TestAppContext) {
        let (view, cx) = folds_job(cx, |jobs| {
            let one = jobs.iter_mut().find(|one| one.id.as_str() == JOB).expect("the folds' Job");
            one.mark = Some(ipc::JobMark { state: ipc::JobState::Working, lamp: ipc::JobLamp::Needs, group: ipc::JobGroup::Needs });
            one.hosted = None;
        });
        shorter(cx);
        scroll_up(&view, cx);
        assert_eq!(says(&view, cx), None);
        assert!(drawn_like(cx, Role::Button, "Now: ").is_empty());
    }

    #[gpui::test]
    fn a_press_on_the_pill_takes_the_thread_to_its_end_and_the_pill_goes(cx: &mut gpui::TestAppContext) {
        let (view, cx) = folds_job(cx, |_| {});
        shorter(cx);
        scroll_up(&view, cx);
        let at = cx.update(|window, cx| cx.global::<crate::labels::LabelBook>().find(window.window_handle().window_id(), Role::Button, WORDS).map(|pill| pill.bounds.center())).expect("the pill is drawn");
        cx.simulate_click(at, Modifiers::none());
        redraw(cx);
        view.read_with(cx, |view, _| assert!(view.pages[JOB].list.is_following_tail(), "the thread follows its tail again"));
        assert_eq!(says(&view, cx), None);
        assert!(!drawn(cx, Role::Button, WORDS), "at the end the thread's own running row says it");
    }

    /// The pill is in the box's tool row at a height the row already has, so the thread's room is the same with it and without it:
    /// nothing it does reaches the list.
    #[gpui::test]
    fn showing_the_pill_moves_nothing_the_thread_measures(cx: &mut gpui::TestAppContext) {
        let (view, cx) = folds_job(cx, |_| {});
        shorter(cx);
        let region = |cx: &mut VisualTestContext| cx.update(|window, cx| crate::job_view::head::drawn(window, cx, Role::Region, "Thread").map(|bounds| bounds.size));
        let at_end = (region(cx), view.read_with(cx, |view, _| view.pane_height));
        assert!(at_end.0.is_some() && at_end.1.is_some(), "the thread was measured");
        scroll_up(&view, cx);
        assert!(drawn(cx, Role::Button, WORDS));
        for _ in 0..8 {
            cx.run_until_parked();
            cx.update(|window, cx| window.draw(cx).clear(cx));
            let seen = (region(cx), view.read_with(cx, |view, _| view.pane_height));
            assert_eq!(seen, at_end, "the thread's region or room moved with the pill");
        }
    }
}
