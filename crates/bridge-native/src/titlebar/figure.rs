//! The figure beside a place's name: Reviews counts the pull requests asked of him he has not opened.

use gpui::{Entity, Role, TestAppContext, VisualTestContext};

use super::face::Place;
use super::{scenes, TitleBar};

fn redraw(cx: &mut VisualTestContext) {
    for _ in 0..3 {
        cx.run_until_parked();
        cx.update(|window, cx| window.draw(cx).clear(cx));
    }
}

fn drawn(cx: &mut VisualTestContext, role: Role, name: &str) -> bool {
    cx.update(|window, cx| cx.try_global::<crate::labels::LabelBook>().is_some_and(|book| book.find(window.window_handle().window_id(), role, name).is_some()))
}

/// A bar with Jobs and Reviews in the strip, and Reviews' figure at `count`.
fn bar_at(cx: &mut TestAppContext, count: usize) -> (Entity<TitleBar>, &mut VisualTestContext) {
    cx.update(crate::init_app);
    let (bar, cx) = cx.add_window_view(|window, cx| TitleBar::new(window, cx));
    bar.update(cx, |bar, cx| {
        let mut face = scenes::rest();
        face.places.push(Place { id: "reviews", label: "Reviews", glyph: "git-pull-request", shown: true, hideable: true });
        bar.reset(face);
        bar.set_figure("reviews", count, cx);
        cx.notify();
    });
    redraw(cx);
    (bar, cx)
}

#[gpui::test]
fn reviews_carries_the_figure_of_the_requests_he_has_not_opened(cx: &mut TestAppContext) {
    let (bar, cx) = bar_at(cx, 2);
    assert!(drawn(cx, Role::Tab, "Reviews"), "the place keeps its own name, so a walk and VoiceOver still find it");
    assert!(drawn(cx, Role::Status, "2 reviews asked of you, not opened"), "the figure says what it counts");
    bar.update(cx, |bar, cx| bar.set_figure("reviews", 1, cx));
    redraw(cx);
    assert!(drawn(cx, Role::Status, "1 review asked of you, not opened"), "it drops as he opens one");
    bar.update(cx, |bar, cx| bar.set_figure("reviews", 0, cx));
    assert_eq!(bar.read_with(cx, |bar, _| bar.face.figure_of("reviews")), None, "zero draws nothing");
}

#[gpui::test]
fn no_figure_is_drawn_at_zero(cx: &mut TestAppContext) {
    let (_bar, cx) = bar_at(cx, 0);
    assert!(drawn(cx, Role::Tab, "Reviews"));
    assert!(!drawn(cx, Role::Status, "0 reviews asked of you, not opened"));
}
