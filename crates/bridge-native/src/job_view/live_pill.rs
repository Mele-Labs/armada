//! What the Job is doing now, as a pill in the composer's tool row while the thread is scrolled away from its end: the words its
//! row in the Jobs list uses (`jobs::now_of`), how long the call has run, and a ring that turns. A press takes the thread to its end.
//!
//! It stands in the box, not the thread, at a height the row already has, so showing or hiding it moves nothing the list measures.

use std::time::Duration;

use gpui::{div, prelude::*, Animation, AnimationExt, AnyElement, Context, Pixels, Role, SharedString, Window};
use ipc::JobLamp;

use super::rows::{elapsed_words, glyph, mono, tip, Size};
use super::runs::running_now;
use super::{JobView, Page};
use crate::labels::labelled;
use crate::look::{token, ActiveLook, Look};

/// Whether the thread shows its end: following its tail, or the last element's top is in view.
fn at_end(page: &Page) -> bool {
    page.list.is_following_tail()
        || page.count.checked_sub(1).and_then(|last| page.list.bounds_for_item(last)).is_some_and(|bounds| bounds.top() < page.list.viewport_bounds().bottom())
}

/// The room the row's spare width must hold for the pill to be worth drawing: the ring, about twelve letters, the arrow, and the
/// pill's own padding and gaps.
fn needed(look: &Look) -> Pixels {
    look.length(token::SPACE_4) * 2. + look.length(token::SPACE_2) * 4. + look.length(token::BORDER_WIDTH) * 2. + look.length(token::TEXT_XS) * 7.
}

/// Whether the tool row has that room past its other controls, from where it drew the sketch button and the mic in the last
/// frame. The span between them is the same with the pill drawn or not, so asking it feeds nothing back, and a row too narrow
/// for the pill keeps its words for Auto and the model.
fn fits(window: &Window, cx: &gpui::App, look: &Look, gap: Pixels) -> bool {
    let (Some(from), Some(to)) = (super::head::drawn(window, cx, Role::Button, "Draw sketch"), super::head::drawn(window, cx, Role::Button, "Talk")) else { return false };
    to.left() - from.right() - gap * 3. >= needed(look)
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
    pub(super) fn render_live_pill(&self, id: &SharedString, gap: Pixels, window: &Window, cx: &Context<Self>) -> Option<AnyElement> {
        let words = self.pill_words(id, cx)?;
        if !fits(window, cx, cx.look(), gap) {
            return None;
        }
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
                    .relative()
                    .flex()
                    // It takes the room the row has spare, which `fits` has found to be enough, so the other controls keep their words.
                    .flex_1()
                    .min_w_0()
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
                    .child(div().id("live-pill-words").flex_1().min_w_0().truncate().child(said.clone()))
                    .children(time)
                    .child(glyph(look, "arrow-down", Size::Small, look.color(token::FG_MUTED)))
                    // The busy ring runs on the pill's border, where a spinner glyph stood.
                    .child(crate::parts::ring::ring_overlay(look, "live-pill-ring", look.length(token::RADIUS_MD), look.color(token::STATUS_RUNNING)))
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

    /// What the tool row's buttons measure, by name, left to right.
    fn row(cx: &mut VisualTestContext) -> Vec<(String, gpui::Pixels)> {
        cx.update(|window, cx| {
            let book = cx.global::<crate::labels::LabelBook>();
            let drawn = book.drawn(window.window_handle().window_id());
            let Some(mode) = drawn.iter().find(|one| one.role == Role::Button && one.name == "Permission mode") else { return Vec::new() };
            let mut one: Vec<_> = drawn.iter().filter(|one| one.role == Role::Button && (one.bounds.origin.y - mode.bounds.origin.y).abs() < px(20.) && !one.name.starts_with("Now: ")).map(|one| (one.bounds.origin.x, one.name.to_string(), one.bounds.size.width)).collect();
            one.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
            one.into_iter().map(|(_, name, width)| (name, width)).collect()
        })
    }

    #[gpui::test]
    fn a_tool_row_too_narrow_for_the_pill_draws_none_and_auto_and_the_model_read_whole(cx: &mut gpui::TestAppContext) {
        let (view, cx) = folds_job(cx, |_| {});
        cx.simulate_resize(gpui::size(px(660.), px(900.)));
        redraw(cx);
        let at_end = row(cx);
        assert!(at_end.len() >= 5, "the row's buttons were drawn: {at_end:?}");
        scroll_up(&view, cx);
        redraw(cx);
        assert_eq!(says(&view, cx).as_deref(), Some("Reading page_votes.rs"), "it works and the thread is scrolled up");
        assert!(drawn_like(cx, Role::Button, "Now: ").is_empty(), "no room, so no pill");
        // A card scrolled up behind the composer draws its own buttons at the row's height, so only the row's own are compared.
        let now: Vec<_> = row(cx).into_iter().filter(|(name, _)| at_end.iter().any(|(own, _)| own == name)).collect();
        assert_eq!(now, at_end, "Auto, the model and the rest measure as they do with no pill");
    }

    #[gpui::test]
    fn the_running_card_holds_a_row_for_each_background_task_until_fleet_stops_listing_it(cx: &mut gpui::TestAppContext) {
        let (view, cx) = folds_job(cx, |jobs| {
            let one = jobs.iter_mut().find(|one| one.id.as_str() == JOB).expect("the folds' Job");
            one.mark = Some(ipc::JobMark { state: ipc::JobState::Idle, lamp: ipc::JobLamp::Idle, group: ipc::JobGroup::Held });
            one.hosted = None;
            one.running = vec![ipc::RunningWork { kind: ipc::RunningKind::BackgroundTask, what: "Break down develop folder by project".into(), since: ipc::Instant::carried("2026-11-16T16:40:10Z") }];
        });
        assert_eq!(drawn_like(cx, Role::Region, "Running now").len(), 1, "the running card stands for the task");
        view.update(cx, |view, cx| {
            if let Some(cards) = view.cards_mut() {
                cards.opened.insert("subagents".to_string());
            }
            cx.notify();
        });
        redraw(cx);
        assert_eq!(drawn_like(cx, Role::Status, "Background task Break down develop folder by project").len(), 1, "one row for the one task");
        view.update(cx, |view, cx| {
            view.list.update(cx, |list, cx| {
                list.record_mut(JOB).expect("the folds' Job").running.clear();
                cx.notify();
            });
        });
        redraw(cx);
        assert!(drawn_like(cx, Role::Status, "Background task").is_empty(), "the row goes once Fleet stops listing it");
        assert!(drawn_like(cx, Role::Region, "Running now").is_empty(), "and the card with it");
    }
}
