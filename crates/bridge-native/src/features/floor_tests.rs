use gpui::{Role, TestAppContext, VisualTestContext};
use ipc::{FeaturePage, FloorDoor, FloorPlan, FloorRoom};

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

fn room(module: &str, lit: bool) -> FloorRoom {
    FloorRoom { module: module.into(), path: format!("crates/{module}"), kind: "cargo".into(), lit, manifest_id: None }
}

fn door(from: &str, to: &str, contract: bool) -> FloorDoor {
    FloorDoor { from: from.into(), to: to.into(), contract }
}

fn with_plan(plan: Option<FloorPlan>) -> FeaturePage {
    FeaturePage { floor_plan: plan, ..mock::page(SLUG) }
}

fn shown(floor: &Floor) -> Vec<(String, bool)> {
    match floor {
        Floor::Plan(plan) => plan.rooms.iter().map(|room| (room.module.clone(), room.lit)).collect(),
        Floor::Empty(_) => Vec::new(),
    }
}

#[test]
fn lit_rooms_come_first_each_group_in_fleets_order() {
    let page = with_plan(Some(FloorPlan { rooms: vec![room("a", false), room("b", true), room("c", false), room("d", true)], doors: Vec::new() }));
    let order = shown(&page_model::floor(&page, false));
    assert_eq!(order, [("b".to_string(), true), ("d".to_string(), true), ("a".to_string(), false), ("c".to_string(), false)]);
}

#[test]
fn the_mock_plan_has_twelve_rooms_four_lit_three_doors_and_one_contract() {
    let Floor::Plan(plan) = page_model::floor(&mock::page(SLUG), false) else { panic!("the mock page has a floor plan") };
    assert_eq!(plan.rooms.len(), 12);
    assert_eq!(plan.rooms.iter().filter(|room| room.lit).count(), 4);
    assert_eq!(plan.doors.len(), 3);
    assert_eq!(plan.doors.iter().filter(|door| door.contract).count(), 1);
}

#[test]
fn only_doors_from_a_lit_room_are_kept() {
    let page = with_plan(Some(FloorPlan { rooms: vec![room("a", true), room("b", false)], doors: vec![door("a", "b", false), door("b", "a", true)] }));
    let Floor::Plan(plan) = page_model::floor(&page, false) else { panic!("a plan") };
    assert_eq!(plan.doors.iter().map(|door| (door.from.as_str(), door.to.as_str())).collect::<Vec<_>>(), [("a", "b")]);
}

#[test]
fn past_twenty_four_rooms_the_unlit_ones_fold_under_a_count_and_open_on_a_press() {
    let rooms: Vec<FloorRoom> = (0..30).map(|n| room(&format!("m{n}"), n < 3)).collect();
    let page = with_plan(Some(FloorPlan { rooms, doors: Vec::new() }));
    let Floor::Plan(folded) = page_model::floor(&page, false) else { panic!("a plan") };
    assert_eq!((folded.rooms.len(), folded.hidden, folded.folds), (3, 27, true));
    let Floor::Plan(open) = page_model::floor(&page, true) else { panic!("a plan") };
    assert_eq!((open.rooms.len(), open.hidden, open.folds), (30, 0, true));
    let few = with_plan(Some(FloorPlan { rooms: (0..24).map(|n| room(&format!("m{n}"), n < 3)).collect(), doors: Vec::new() }));
    let Floor::Plan(few) = page_model::floor(&few, false) else { panic!("a plan") };
    assert_eq!((few.rooms.len(), few.hidden, few.folds), (24, 0, false), "24 rooms all show");
}

#[gpui::test]
fn a_contract_door_carries_the_contract_chip_and_the_others_do_not(cx: &mut TestAppContext) {
    let (_page, cx) = open(cx);
    cx.update(|window, _| window.refresh());
    cx.run_until_parked();
    cx.update(|window, cx| window.draw(cx).clear(cx));
    assert!(drawn(cx, Role::Region, "Floor plan"));
    assert!(drawn(cx, Role::Label, "Contract: bridge-native to ipc"), "the door to the wire crate is a contract");
    assert!(drawn(cx, Role::ListItem, "bridge-native to ipc"));
    assert!(drawn(cx, Role::ListItem, "fleet to config"));
    assert!(!drawn(cx, Role::Label, "Contract: fleet to config"), "a plain door has no chip");
}

#[gpui::test]
fn lit_rooms_are_drawn_before_the_unlit_ones(cx: &mut TestAppContext) {
    let (_page, cx) = open(cx);
    cx.update(|window, cx| window.draw(cx).clear(cx));
    let at = |cx: &mut VisualTestContext, name: &str| {
        cx.update(|window, cx| cx.default_global::<crate::labels::LabelBook>().find(window.window_handle().window_id(), Role::ListItem, name).map(|drawn| (drawn.bounds.origin.y, drawn.bounds.origin.x)))
    };
    let lit = at(cx, "fleet, in this feature").expect("a lit room");
    let unlit = at(cx, "core-model, not in this feature").expect("an unlit room");
    assert!(lit < unlit, "the lit room stands before the unlit one in reading order");
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
