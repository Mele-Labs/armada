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
    assert_eq!(names, ["release-notes", "review-pr", "pdf-tools", "frontend-design", "deploy-check", "long-notes"]);
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

fn drawn(cx: &mut VisualTestContext, role: gpui::Role, name: &str) -> bool {
    cx.update(|window, cx| cx.default_global::<crate::labels::LabelBook>().find(window.window_handle().window_id(), role, name).is_some())
}

#[gpui::test]
fn save_is_drawn_only_while_the_text_differs_from_what_was_read(cx: &mut TestAppContext) {
    let (screen, cx) = open(cx);
    cx.simulate_keystrokes("j");
    cx.run_until_parked();
    assert!(!drawn(cx, gpui::Role::Button, "Save"), "nothing is being edited");
    cx.simulate_keystrokes("e");
    cx.run_until_parked();
    assert!(screen.read_with(cx, |screen, _| screen.editing.is_some()), "E turns the pane into a textarea");
    assert!(!drawn(cx, gpui::Role::Button, "Save"), "the text is as it was read");
    cx.simulate_input("x");
    cx.run_until_parked();
    assert!(drawn(cx, gpui::Role::Button, "Save"), "the text now differs");
    let (area, read) = screen.read_with(cx, |screen, _| screen.editing.as_ref().map(|edit| (edit.area.clone(), edit.read.clone())).expect("editing"));
    cx.update(|window, cx| area.update(cx, |area, cx| area.set_value(read, window, cx)));
    screen.update(cx, |_, cx| cx.notify());
    cx.run_until_parked();
    // The window's book of drawn things is only added to between roots, so what leaves is read from the edit itself.
    assert!(!screen.read_with(cx, |screen, cx| screen.editing.as_ref().is_some_and(|edit| edit.changed(cx))), "put back to what was read, Save has nothing to save");
    cx.simulate_keystrokes("escape");
    cx.run_until_parked();
    assert!(screen.read_with(cx, |screen, _| screen.editing.is_none()), "Esc drops the edit");
}

#[gpui::test]
fn a_file_that_moved_on_disk_is_refused_in_fleets_words_with_a_way_to_reload_it(cx: &mut TestAppContext) {
    let (screen, cx) = open(cx);
    cx.simulate_keystrokes("j e");
    cx.run_until_parked();
    cx.simulate_input("x");
    // Another hand changes the file after it was read.
    screen.update(cx, |screen, _| {
        let id = screen.selected.clone().expect("a piece is picked");
        if let super::Source::Mock(kit) = &mut screen.source {
            kit.files.insert(id, "changed elsewhere".into());
        }
    });
    cx.simulate_keystrokes("cmd-s");
    cx.run_until_parked();
    assert!(drawn(cx, gpui::Role::Alert, "The file changed on disk since you opened it."), "Fleet's words, drawn");
    assert!(drawn(cx, gpui::Role::Button, "Reload review-pr from disk"), "what is on disk can be loaded");
    cx.update(|window, cx| screen.update(cx, |screen, cx| screen.reload_from_disk(window, cx)));
    cx.run_until_parked();
    let after = screen.read_with(cx, |screen, _| (screen.editing.is_none(), screen.file_of_picked().map(|file| file.text.clone())));
    assert_eq!(after, (true, Some("changed elsewhere".to_string())));
}

#[gpui::test]
fn the_read_pane_scrolls_on_a_page_key_and_goes_back_to_the_top_for_another_piece(cx: &mut TestAppContext) {
    let (screen, cx) = open(cx);
    cx.update(|_, cx| screen.update(cx, |screen, cx| screen.pick(mock::LONG_NOTES_ID.to_string(), cx)));
    cx.run_until_parked();
    cx.simulate_keystrokes("tab");
    cx.run_until_parked();
    let top = screen.read_with(cx, |screen, _| screen.file_scroll.offset().y);
    assert_eq!(top, gpui::px(0.));
    cx.simulate_keystrokes("pagedown");
    cx.run_until_parked();
    let moved = screen.read_with(cx, |screen, _| screen.file_scroll.offset().y);
    assert!(moved < gpui::px(0.), "a page down moved the pane: {moved:?}");
    cx.simulate_keystrokes("end");
    let end = screen.read_with(cx, |screen, _| (screen.file_scroll.offset().y, screen.file_scroll.max_offset().y));
    assert_eq!(end.0, -end.1, "End goes to the bottom");
    cx.simulate_keystrokes("home");
    assert_eq!(screen.read_with(cx, |screen, _| screen.file_scroll.offset().y), gpui::px(0.));
    cx.simulate_keystrokes("pagedown");
    cx.update(|_, cx| screen.update(cx, |screen, cx| screen.pick("skills:/work/armada/.claude/skills/deploy-check:deploy-check".to_string(), cx)));
    assert_eq!(screen.read_with(cx, |screen, _| screen.file_scroll.offset().y), gpui::px(0.), "another piece reads from its top");
}

#[test]
fn a_plugin_id_the_inventory_gave_is_kept_whole_for_the_wire_and_a_plugins_group_says_so() {
    let mut inventory = mock::inventory();
    let mut style = ipc::SetupItem { name: "Armada".into(), says: Some("The house voice".into()), source: "/Users/you/.claude/plugins/armada/output-styles/armada.md".into(), style: None, plugin: None, uninstall: None, server: None };
    style.style = Some(ipc::OutputStyleFacts { from: "plugin".into(), applied: "available".into(), plugin: Some("armada@armada-local".into()) });
    if let Some(one) = inventory.kinds.iter_mut().find(|one| one.kind == "output_styles") {
        if let ipc::WhatWasRead::Read { items, .. } = &mut one.read {
            items.push(style);
        }
    }
    let Read::Rows(rows) = model::read_kind(Kind::OutputStyles, &inventory, &[]) else { panic!("output styles read") };
    let armada = rows.iter().find(|row| row.name == "Armada").expect("the style is listed");
    assert_eq!(armada.plugin_id.as_deref(), Some("armada@armada-local"), "the whole string, for Fleet to compare");
    assert_eq!(armada.comes_with.as_deref(), Some("armada"), "the short name, for drawing");
    assert_eq!(armada.mark.label(), "armada plugin", "a plugin's group never reads as Armada's own");
    assert_eq!(Mark::Armada.label(), "Armada");
}

#[test]
fn servers_are_grouped_by_where_they_are_configured_and_several_repositories_each_by_their_folder() {
    let facts = |from: &str, folder: &str| Some(ipc::ServerFacts { from: from.into(), folder: folder.into() });
    let mut inventory = mock::inventory();
    if let Some(one) = inventory.kinds.iter_mut().find(|one| one.kind == "mcp_servers") {
        if let ipc::WhatWasRead::Read { items, .. } = &mut one.read {
            let mut other = items.last().cloned().expect("the mock lists servers");
            other.name = "ledger-db".into();
            other.server = facts("repository", "/work/ledger");
            let mut local = other.clone();
            local.name = "scratch".into();
            local.server = facts("project", "/work/notes");
            items.push(other);
            items.push(local);
        }
    }
    let Read::Rows(rows) = model::read_kind(Kind::Servers, &inventory, &[]) else { panic!("servers read") };
    let mark = |name: &str| rows.iter().find(|row| row.name == name).map(|row| row.mark.clone());
    assert_eq!(mark("linear"), Some(Mark::Yours), "the person's own file has no marker");
    assert_eq!(mark("design-docs"), Some(Mark::Plugin("design-kit".into())));
    assert_eq!(mark("ci-logs"), Some(Mark::Folder("armada".into())), "two repositories each name their folder");
    assert_eq!(mark("ledger-db"), Some(Mark::Folder("ledger".into())));
    assert_eq!(mark("scratch"), Some(Mark::Folder("notes".into())), "a project's local scope names its folder");
    let alone = mock::inventory();
    let Read::Rows(rows) = model::read_kind(Kind::Servers, &alone, &[]) else { panic!("servers read") };
    assert_eq!(rows.iter().find(|row| row.name == "ci-logs").map(|row| row.mark.clone()), Some(Mark::Repository), "one repository is just This repository");
}
