use gpui::{px, AppContext as _, Role, TestAppContext};

use crate::fleet::FleetConnection;
use crate::look::ActiveLook as _;
use crate::reviews::{panel_inner_radius, ReviewsScreen};

#[gpui::test]
fn the_first_tab_of_a_pane_follows_the_panels_curve_and_the_others_stay_square(cx: &mut TestAppContext) {
    cx.update(|cx| {
        gpui_component::init(cx);
        crate::look::init(cx);
    });
    let fleet = cx.update(|cx| cx.new(FleetConnection::idle));
    let (reviews, cx) = cx.add_window_view(|window, cx| ReviewsScreen::live(&fleet, window, cx));
    cx.update(|window, cx| {
        reviews.update(cx, |reviews, cx| {
            reviews.pages.insert(2231, crate::reviews::mock::page(2231).expect("a mock page"));
            reviews.show(2231, window, cx);
        })
    });
    cx.run_until_parked();
    cx.update(|window, cx| window.draw(cx).clear(cx));
    let inner = cx.update(|_, cx| panel_inner_radius(cx.look()));
    assert!(inner > px(0.));
    let tabs: Vec<_> = cx.update(|window, cx| {
        cx.default_global::<crate::labels::LabelBook>().drawn(window.window_handle().window_id()).iter().filter(|drawn| drawn.role == Role::Tab).map(|drawn| (drawn.bounds.origin, drawn.radius)).collect()
    });
    assert!(tabs.len() > 1, "the page draws a pane with several tabs");
    for (origin, radius) in &tabs {
        // The first tab of a pane is the one with no tab to its left in its row.
        let first = !tabs.iter().any(|(other, _)| other.y == origin.y && other.x < origin.x && (other.y - origin.y).abs() < px(1.));
        let expected = if first { inner } else { px(0.) };
        assert_eq!(*radius, Some(expected), "tab at {origin:?}: only a pane's first tab takes the panel's curve");
    }
    assert!(tabs.iter().any(|(_, radius)| *radius == Some(inner)), "some tab is first in its pane");
}
