use gpui::{Role, TestAppContext, VisualTestContext};
use ipc::{FeaturePage, FloorDoor, FloorFile, FloorPlan, FloorRoom};

use super::floor_map::{self, Ink, Rect, HEIGHT, WIDTH};
use super::page::{FeaturePageView, Load};
use super::page_model::{self, Floor};
use super::{looked, mock};

const SLUG: &str = "start-a-job";

fn open(cx: &mut TestAppContext) -> (gpui::Entity<FeaturePageView>, &mut VisualTestContext) {
    cx.update(|cx| {
        gpui_component::init(cx);
        crate::look::init(cx);
        crate::keymap::install(&crate::keymap::Rebinding::default(), cx);
        crate::titlebar::bind(cx);
        looked::forget_all(cx);
    });
    let (page, cx) = cx.add_window_view(|window, cx| FeaturePageView::mock(SLUG.into(), window, cx));
    cx.update(|window, cx| page.update(cx, |page, cx| page.focus_rail(window, cx)));
    cx.run_until_parked();
    (page, cx)
}

fn drawn(cx: &mut VisualTestContext, role: Role, name: &str) -> bool {
    cx.update(|window, cx| cx.default_global::<crate::labels::LabelBook>().find(window.window_handle().window_id(), role, name).is_some())
}

fn file(path: &str, bytes: u64, lit: bool, changing: bool) -> FloorFile {
    FloorFile { path: path.into(), bytes, lit, changing, grouped: 0 }
}

fn room(module: &str, lit: bool, files: Vec<FloorFile>) -> FloorRoom {
    FloorRoom { module: module.into(), path: format!("crates/{module}"), kind: "cargo".into(), lit, manifest_id: None, files }
}

fn door(from: &str, to: &str, contract: bool) -> FloorDoor {
    FloorDoor { from: from.into(), to: to.into(), contract }
}

fn with_plan(plan: Option<FloorPlan>) -> FeaturePage {
    FeaturePage { floor_plan: plan, ..mock::page(SLUG) }
}

/// The map at the mock's size, with the mock's name strip.
fn lay(plan: &FloorPlan) -> floor_map::FloorMap {
    floor_map::map(plan, WIDTH, HEIGHT, 15.)
}

fn near(one: f32, other: f32) -> bool {
    (one - other).abs() <= other.abs() * 0.01 + 0.01
}

#[test]
fn squarify_gives_each_item_an_area_in_proportion_to_its_weight_and_fills_the_box() {
    let items = [("a", 600.), ("b", 300.), ("c", 200.), ("d", 100.), ("e", 0.)];
    let laid = floor_map::squarify(&items, Rect { x: 10., y: 20., w: 300., h: 200. });
    assert_eq!(laid.iter().map(|(key, _)| *key).collect::<Vec<_>>(), ["a", "b", "c", "d"], "an item of no weight gets no place");
    let whole = 300. * 200.;
    for (key, rect) in &laid {
        let weight = items.iter().find(|(one, _)| one == key).map(|(_, weight)| *weight).unwrap_or_default() as f32;
        assert!(near(rect.w * rect.h, whole * weight / 1200.), "{key}: {rect:?}");
        assert!(rect.x >= 10. - 0.01 && rect.y >= 20. - 0.01 && rect.x + rect.w <= 310.01 && rect.y + rect.h <= 220.01, "{key} stays in the box");
    }
}

#[test]
fn every_room_is_a_plot_sized_by_its_bytes_and_its_files_are_pills() {
    let plan = FloorPlan {
        rooms: vec![
            room("big", true, vec![file("crates/big/src/a.rs", 3000, true, false), file("crates/big/src/b.rs", 2000, false, true), file("crates/big/c.rs", 1000, false, false)]),
            room("small", false, vec![file("crates/small/src/d.rs", 2000, false, false)]),
        ],
        doors: Vec::new(),
    };
    let map = lay(&plan);
    assert_eq!(map.plots.iter().map(|plot| (plot.name.as_str(), plot.lit)).collect::<Vec<_>>(), [("big", true), ("small", false)], "the larger room first");
    // A plot is its share of the map less the gap round it: 6000 to 2000 bytes is three quarters of the map to one quarter.
    let share = |plot: &floor_map::Plot| (plot.rect.w + 6.) * (plot.rect.h + 6.) / (WIDTH * HEIGHT);
    assert!(near(share(&map.plots[0]), 0.75) && near(share(&map.plots[1]), 0.25), "{:?}", map.plots);
    assert_eq!(map.pills.len(), 4, "every file is a pill");
    let inks: Vec<Ink> = map.pills.iter().map(|pill| pill.ink).collect();
    assert_eq!(inks.iter().filter(|ink| **ink == Ink::Lit).count(), 1, "the file the feature owns stays lit");
    assert_eq!(inks.iter().filter(|ink| **ink == Ink::Changing).count(), 1, "the file a Job is writing stays changing");
    let big = map.plots[0].rect;
    for pill in &map.pills[..3] {
        assert!(pill.rect.x >= big.x && pill.rect.x + pill.rect.w <= big.x + big.w + 0.01 && pill.rect.y >= big.y && pill.rect.y + pill.rect.h <= big.y + big.h + 0.01, "a pill stays in its plot");
    }
}

#[test]
fn small_files_of_a_folder_share_one_pill_but_an_owned_one_stands_alone() {
    let mut files = vec![file("crates/one/src/large.rs", 1_000_000, false, false), file("crates/one/src/mine.rs", 10, true, false)];
    files.extend((0..40).map(|at| file(&format!("crates/one/src/tiny_{at}.rs"), 10, false, false)));
    let map = lay(&FloorPlan { rooms: vec![room("one", true, files)], doors: Vec::new() });
    assert_eq!(map.pills.iter().filter(|pill| pill.ink == Ink::Lit).count(), 1, "the owned file is a pill of its own however small");
    assert!(map.pills.len() <= 3, "the large file, the owned one and one pill for the tiny ones: {}", map.pills.len());
}

#[test]
fn a_grouped_entry_is_one_pill_in_the_folder_it_names() {
    let files = vec![file("crates/one/src/a.rs", 5000, false, false), FloorFile { path: "crates/one/src/".into(), bytes: 5000, lit: false, changing: false, grouped: 12 }];
    let map = lay(&FloorPlan { rooms: vec![room("one", false, files)], doors: Vec::new() });
    assert_eq!(map.pills.len(), 2);
}

#[test]
fn only_contract_doors_are_drawn_and_each_is_named_for_the_room_it_leads_to() {
    let rooms = vec![room("a", true, vec![file("crates/a/x.rs", 10, true, false)]), room("b", false, vec![file("crates/b/y.rs", 10, false, false)])];
    let map = lay(&FloorPlan { rooms, doors: vec![door("a", "b", true), door("b", "a", false), door("a", "gone", true)] });
    assert_eq!(map.doors.iter().map(|one| one.label.as_str()).collect::<Vec<_>>(), ["b"], "a plain door and one to a module not on the map are not drawn");
}

#[test]
fn a_room_with_no_files_listed_is_still_a_plot_and_the_mock_plan_lights_four_of_twelve() {
    let map = lay(&FloorPlan { rooms: vec![room("a", false, Vec::new()), room("b", false, Vec::new())], doors: Vec::new() });
    assert_eq!(map.plots.len(), 2);
    assert!(map.pills.is_empty());
    let Floor::Plan(laid) = page_model::floor(&mock::page(SLUG)) else { panic!("the mock page has a floor plan") };
    assert_eq!(laid.said, "Floor plan: 4 of 12 modules hold this feature's files");
    let plan = laid.at(WIDTH, HEIGHT, 15.);
    assert!(std::rc::Rc::ptr_eq(&plan, &laid.at(WIDTH, HEIGHT, 15.)), "the same width is not laid out again");
    assert_eq!(plan.plots.len(), 12);
    assert_eq!(plan.doors.len(), 3, "three of the four doors are contracts");
    assert!(plan.pills.iter().any(|pill| pill.ink == Ink::Changing) && plan.pills.iter().any(|pill| pill.ink == Ink::Lit));
}

#[gpui::test]
fn the_card_draws_the_map_as_one_labelled_picture_and_the_key(cx: &mut TestAppContext) {
    let (_page, cx) = open(cx);
    cx.update(|window, cx| window.draw(cx).clear(cx));
    assert!(drawn(cx, Role::Region, "Floor plan"));
    assert!(drawn(cx, Role::Image, "Floor plan: 4 of 12 modules hold this feature's files"));
}

#[gpui::test]
fn with_no_floor_plan_the_card_says_why_it_is_empty_and_what_fills_it(cx: &mut TestAppContext) {
    let (page, cx) = open(cx);
    assert!(!drawn(cx, Role::Label, page_model::NO_PLAN));
    page.update(cx, |page, cx| {
        page.page = Load::Read(with_plan(None));
        page.rebuild(cx);
    });
    cx.run_until_parked();
    cx.update(|window, cx| window.draw(cx).clear(cx));
    assert!(drawn(cx, Role::Region, "Floor plan"), "the card is drawn with no plan");
    assert!(drawn(cx, Role::Label, page_model::NO_PLAN));
}
