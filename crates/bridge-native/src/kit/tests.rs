use gpui::{TestAppContext, VisualTestContext};

use super::model::{self, Kind, Line, Mark, Read, Removal};
use super::{mock, KitScreen, Pane};

fn skills() -> Vec<model::Row> {
    match model::read_kind(Kind::Skills, &mock::inventory(), &[]) {
        Read::Rows(rows) => rows,
        Read::NotRead(why) => panic!("the mock's skills read: {why}"),
    }
}

fn ready(cx: &mut TestAppContext) {
    cx.update(|cx| {
        gpui_component::init(cx);
        crate::look::init(cx);
        crate::keymap::install(&crate::keymap::Rebinding::default(), cx);
        crate::titlebar::bind(cx);
    });
}

fn open(cx: &mut TestAppContext) -> (gpui::Entity<KitScreen>, &mut VisualTestContext) {
    ready(cx);
    let (screen, cx) = cx.add_window_view(|window, cx| KitScreen::mock(window, cx));
    cx.update(|window, cx| screen.update(cx, |screen, cx| screen.enter(window, cx)));
    cx.run_until_parked();
    (screen, cx)
}

#[test]
fn pieces_are_grouped_by_where_they_come_from_yours_first_and_a_plugin_before_this_repository() {
    let listed = model::listed(&skills(), &[], "");
    let heads: Vec<Mark> = listed.lines.iter().filter_map(|line| if let Line::Head(mark) = line { Some(mark.clone()) } else { None }).collect();
    assert_eq!(heads, [Mark::Yours, Mark::Plugin("design-kit".into()), Mark::Repository]);
    // A group's chip is followed by its own pieces and no other's.
    let names: Vec<&str> = listed.shown.iter().map(|one| one.row.name.as_str()).collect();
    assert_eq!(names, ["release-notes", "review-pr", "pdf-tools", "frontend-design", "deploy-check"]);
}

#[test]
fn a_piece_a_plugin_brought_has_no_remove_and_names_the_plugin_that_does() {
    let rows = skills();
    let brought = rows.iter().find(|row| row.name == "frontend-design").expect("the mock has it");
    assert_eq!(brought.removal(), Removal::Locked("design-kit".into()));
    let own = rows.iter().find(|row| row.name == "release-notes").expect("the mock has it");
    assert!(matches!(own.removal(), Removal::Piece(_)));
    // The repository's own skill is not his to remove from here, and has no switch either.
    let repository = rows.iter().find(|row| row.name == "deploy-check").expect("the mock has it");
    assert_eq!(repository.removal(), Removal::None);
    assert!(!repository.switchable());
}

#[test]
fn skills_off_round_trips_through_the_setting_and_an_empty_map_is_taken_away() {
    let repositories = vec!["armada".to_string(), "ledger".to_string()];
    let off = model::parse_off(None);
    let changes = model::switched(&off, &repositories, "pdf-tools", false);
    let saved = changes.get(model::SKILLS_OFF).cloned().flatten();
    let held = model::parse_off(saved.as_ref());
    assert_eq!(held.get("armada"), Some(&vec!["pdf-tools".to_string()]));
    assert_eq!(held.get("ledger"), Some(&vec!["pdf-tools".to_string()]));
    assert!(!model::on_in(&held, &repositories, "pdf-tools"));
    assert!(model::on_in(&held, &repositories, "review-pr"));
    let back = model::switched(&held, &repositories, "pdf-tools", true);
    assert_eq!(back.get(model::SKILLS_OFF), Some(&None), "nothing left off sends null, which takes the key away");
}

#[gpui::test]
fn j_and_k_move_the_piece_picked_and_tab_moves_between_the_panes(cx: &mut TestAppContext) {
    let (screen, cx) = open(cx);
    let picked = |screen: &gpui::Entity<KitScreen>, cx: &mut VisualTestContext| screen.read_with(cx, |screen, _| screen.picked().map(|one| one.row.name.clone()));
    assert_eq!(picked(&screen, cx).as_deref(), Some("release-notes"));
    cx.simulate_keystrokes("j");
    assert_eq!(picked(&screen, cx).as_deref(), Some("review-pr"));
    cx.simulate_keystrokes("j j j");
    assert_eq!(picked(&screen, cx).as_deref(), Some("frontend-design"), "past the removed piece and the group's chip, to the next group's first");
    cx.simulate_keystrokes("k");
    assert_eq!(picked(&screen, cx).as_deref(), Some("old-triage"));
    cx.simulate_keystrokes("tab");
    assert_eq!(screen.read_with(cx, |screen, _| screen.pane), Pane::Detail);
    cx.simulate_keystrokes("tab");
    assert_eq!(screen.read_with(cx, |screen, _| screen.pane), Pane::Rail);
    // In the rail the same keys move between kinds.
    cx.simulate_keystrokes("j");
    assert_eq!(screen.read_with(cx, |screen, _| screen.kind), Kind::Plugins);
}

#[gpui::test]
fn a_removed_skill_stays_amber_with_restore_until_kit_is_left(cx: &mut TestAppContext) {
    let (screen, cx) = open(cx);
    let names = |screen: &gpui::Entity<KitScreen>, cx: &mut VisualTestContext| screen.read_with(cx, |screen, _| screen.names());
    assert!(names(&screen, cx).contains(&"old-triage".to_string()), "the mock starts with one removed");
    cx.simulate_keystrokes("j j");
    cx.simulate_keystrokes("backspace");
    cx.run_until_parked();
    let gone = screen.read_with(cx, |screen, _| screen.picked().map(|one| (one.row.name.clone(), one.gone.clone())));
    assert_eq!(gone.as_ref().map(|(name, _)| name.as_str()), Some("pdf-tools"));
    assert!(gone.is_some_and(|(_, gone)| gone.is_some()), "it stays in the list as removed");
    cx.simulate_keystrokes("cmd-z");
    cx.run_until_parked();
    let back = screen.read_with(cx, |screen, _| screen.picked().map(|one| one.gone.is_none()));
    assert_eq!(back, Some(true), "⌘Z brings it back");
    cx.update(|_, cx| screen.update(cx, |screen, cx| screen.leave(cx)));
    assert!(!names(&screen, cx).contains(&"old-triage".to_string()), "leaving Kit stops offering what was removed");
}
