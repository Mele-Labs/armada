use gpui::{TestAppContext, VisualTestContext};

use super::model::{self, Kind, Line, Mark, Read, Removal};
use super::{mock, sync, KitScreen, Pane, Stop};

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
    assert_eq!(names, ["release-notes", "review-pr", "pdf-tools", "deploy-notes", "tracker-sync", "frontend-design", "deploy-check", "long-notes"]);
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
    cx.simulate_keystrokes("j j j j j j");
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
fn the_keys_are_the_registrys_kit_acts_and_a_key_the_person_rebound_moves_with_it(cx: &mut TestAppContext) {
    let (screen, cx) = open(cx);
    let picked = |screen: &gpui::Entity<KitScreen>, cx: &mut VisualTestContext| screen.read_with(cx, |screen, _| screen.picked().map(|one| one.row.name.clone()));
    // Every key the screen answers is a row of the registry, under the `kit` context.
    let bound = crate::keymap::bound(&crate::keymap::Rebinding::default());
    for id in ["kit_move", "kit_pane", "kit_toggle", "kit_remove", "kit_edit", "kit_open", "kit_filter", "kit_add", "kit_scroll"] {
        assert!(bound.iter().any(|key| key.act.id == id && key.context == Some("kit")), "{id} is bound under kit");
    }
    cx.update(|_, cx| crate::keymap::install(&crate::keymap::Rebinding::read(r#"{"version":1,"bindings":{"kit_move":["n"]}}"#), cx));
    assert_eq!(picked(&screen, cx).as_deref(), Some("release-notes"));
    cx.simulate_keystrokes("n");
    assert_eq!(picked(&screen, cx).as_deref(), Some("review-pr"), "the person's key moves the piece");
    cx.simulate_keystrokes("j");
    assert_eq!(picked(&screen, cx).as_deref(), Some("review-pr"), "the key he gave up does nothing");
    cx.simulate_keystrokes("down");
    assert_eq!(picked(&screen, cx).as_deref(), Some("pdf-tools"), "a slot he left alone still moves it");
}

#[gpui::test]
fn a_bare_key_is_typing_in_the_filter_and_does_not_fire_an_act(cx: &mut TestAppContext) {
    let (screen, cx) = open(cx);
    cx.simulate_keystrokes("/");
    cx.simulate_keystrokes("a");
    assert!(screen.read_with(cx, |screen, _| screen.adding.is_none()), "A typed in the filter does not open the add box");
    cx.simulate_keystrokes("escape");
    assert_eq!(screen.read_with(cx, |screen, _| screen.pane), Pane::List, "Esc gives the keys back to the list");
}

#[gpui::test]
fn a_plugins_question_takes_enter_before_open_and_holds_the_other_keys(cx: &mut TestAppContext) {
    let (screen, cx) = open(cx);
    cx.simulate_keystrokes("shift-tab");
    cx.simulate_keystrokes("j");
    assert_eq!(screen.read_with(cx, |screen, _| screen.kind), Kind::Plugins);
    cx.simulate_keystrokes("tab");
    cx.run_until_parked();
    cx.simulate_keystrokes("j");
    cx.simulate_keystrokes("backspace");
    cx.run_until_parked();
    assert!(screen.read_with(cx, |screen, _| screen.confirming.is_some()), "a plugin asks first");
    cx.simulate_keystrokes("j e");
    assert!(screen.read_with(cx, |screen, _| screen.confirming.is_some() && screen.editing.is_none()), "the question holds the keys");
    cx.simulate_keystrokes("enter");
    cx.run_until_parked();
    assert!(screen.read_with(cx, |screen, _| screen.confirming.is_none()), "Enter answers the question and does not open the editor");
    assert!(!screen.read_with(cx, |screen, _| screen.names()).contains(&"design-kit".to_string()), "the plugin is removed");
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
    cx.simulate_keystrokes("alt-down");
    assert!(screen.read_with(cx, |screen, _| screen.file_scroll.offset().y) < gpui::px(0.), "⌥↓ is a page down too");
    cx.simulate_keystrokes("pagedown");
    cx.update(|_, cx| screen.update(cx, |screen, cx| screen.pick("skills:/work/armada/.claude/skills/deploy-check:deploy-check".to_string(), cx)));
    assert_eq!(screen.read_with(cx, |screen, _| screen.file_scroll.offset().y), gpui::px(0.), "another piece reads from its top");
}

#[test]
fn a_plugin_id_the_inventory_gave_is_kept_whole_for_the_wire_and_a_plugins_group_says_so() {
    let mut inventory = mock::inventory();
    let mut style = ipc::SetupItem { name: "Armada".into(), says: Some("The house voice".into()), source: "/Users/user/.claude/plugins/armada/output-styles/armada.md".into(), style: None, plugin: None, uninstall: None, server: None };
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
    // A server Kit holds is grouped under the word its card says.
    assert_eq!(Mark::Kit.label(), "Kit");
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

// ---- over the Mac link ---------------------------------------------------------------------------

fn standings(linked: mock::Linked) -> Vec<(String, Option<sync::Standing>)> {
    let link = mock::MockKit::linked(linked).link;
    let mut rows = skills();
    rows.extend(sync::elsewhere(&link, Kind::Skills, &rows));
    rows.iter()
        .map(|row| {
            let standing = match &row.elsewhere {
                Some(macs) => Some(sync::Standing::OnlyThere(macs.clone())),
                None => sync::link_row(&link, row).and_then(|one| sync::standing(&link, one)),
            };
            (row.name.clone(), standing)
        })
        .collect()
}

#[test]
fn each_row_is_marked_against_the_linked_mac_and_a_refusal_comes_before_its_mark() {
    use sync::Standing::*;
    let work = "Work Mac".to_string();
    let marks = standings(mock::Linked::Apart);
    let mark = |name: &str| marks.iter().find(|(one, _)| one == name).and_then(|(_, standing)| standing.clone());
    assert_eq!(mark("release-notes"), Some(Clash(work.clone())));
    assert_eq!(mark("review-pr"), Some(Same(work.clone())));
    assert_eq!(mark("pdf-tools"), Some(Differs(work.clone())));
    assert_eq!(mark("deploy-notes"), Some(NewHere(work.clone())));
    assert_eq!(mark("tracker-sync"), Some(Refused(mock::REFUSED.into())), "Fleet's reason, whatever the row's mark");
    assert_eq!(mark("pricing-review"), Some(OnlyThere(work.clone())), "a row only on the other Mac joins the list");
    assert_eq!(mark("frontend-design"), None, "a plugin's piece never crosses");
    assert_eq!(mark("deploy-check"), None, "nor does a repository's");
    assert_eq!(OnlyThere(work.clone()).word(), "Only on Work Mac");
    assert_eq!(Refused(mock::REFUSED.into()).said(), mock::REFUSED, "the tooltip is Fleet's words");
    // With no Mac linked, a row has nothing to stand against: only a refusal is said.
    let alone = standings(mock::Linked::Unlinked);
    assert!(alone.iter().all(|(name, standing)| standing.is_none() || name == "tracker-sync"), "{alone:?}");
    assert!(!alone.iter().any(|(name, _)| name == "pricing-review"));
}

#[test]
fn a_row_against_two_macs_says_its_loudest_mark_and_names_the_macs_it_stands_so_against() {
    let mut link = mock::MockKit::linked(mock::Linked::Apart).link;
    link.macs.push(ipc::KitLinkedMac { mac: "mac-studio".into(), name: "Studio".into(), crossed_at: None, kinds: Vec::new(), items: Vec::new() });
    let row = link.rows.iter_mut().find(|row| row.name == "review-pr").expect("the mock has it");
    row.against.push(ipc::KitAgainst { mac: "mac-studio".into(), mark: ipc::KitMark::Differs, here: None, there: None });
    let row = link.rows.iter().find(|row| row.name == "review-pr").expect("the mock has it");
    assert_eq!(sync::standing(&link, row), Some(sync::Standing::Differs("Studio".into())));
    let row = link.rows.iter_mut().find(|row| row.name == "review-pr").expect("the mock has it");
    row.against[1].mark = ipc::KitMark::Same;
    let row = link.rows.iter().find(|row| row.name == "review-pr").expect("the mock has it");
    assert_eq!(sync::standing(&link, row), Some(sync::Standing::Same("Work Mac and Studio".into())));
}

#[test]
fn the_bar_says_no_mac_linked_in_step_or_what_differs_with_a_clash_first() {
    let bar = |linked| sync::bar(&mock::MockKit::linked(linked).link);
    assert_eq!(bar(mock::Linked::Unlinked), sync::Bar::Unlinked);
    let sync::Bar::Linked { to, crossed, apart } = bar(mock::Linked::InStep) else { panic!("a Mac is linked") };
    assert_eq!((to.as_str(), crossed.len(), apart), ("Work Mac", 1, None), "in step: no chip");
    let sync::Bar::Linked { apart: Some(apart), .. } = bar(mock::Linked::Apart) else { panic!("something differs") };
    assert_eq!(apart, sync::Apart { clash: true, names: vec!["release-notes".into()] }, "a clash outranks the rows that only differ");
    let (word, said) = sync::apart_said(&apart);
    assert_eq!(word, "Clash");
    assert!(said.contains("release-notes") && !said.chars().any(|c| c.is_ascii_digit()), "names the item and counts nothing: {said}");
    // The Account page's row reads the same facts.
    assert_eq!(sync::sync_status(&mock::MockKit::linked(mock::Linked::InStep).link), Some(sync::SyncStatus { macs: vec!["Work Mac".into()], in_step: true }));
    assert_eq!(sync::sync_status(&mock::MockKit::linked(mock::Linked::Apart).link).map(|status| status.in_step), Some(false));
    assert_eq!(sync::sync_status(&mock::MockKit::linked(mock::Linked::Unlinked).link), None);
}

#[test]
fn a_clash_side_tints_its_own_lines_and_marks_where_the_others_sit() {
    use sync::Tint::*;
    let here = sync::compared("a\nmine\nb", "a\nb\ntheirs");
    assert_eq!(here, vec![(Both, "a".into()), (Ours, "mine".into()), (Both, "b".into()), (Theirs, "theirs".into())]);
    let there = sync::compared("a\nb\ntheirs", "a\nmine\nb");
    assert_eq!(there.iter().filter(|(tint, _)| *tint == Ours).count(), 1);
}

#[test]
fn a_plugin_is_named_with_its_marketplace_and_output_styles_do_not_cross() {
    let Read::Rows(rows) = model::read_kind(Kind::Plugins, &mock::inventory(), &[]) else { panic!("plugins read") };
    assert_eq!(sync::link_name(&rows[0]), "design-kit@design-kit");
    assert_eq!(sync::home_kind(Kind::OutputStyles), None);
    assert_eq!(sync::home_kind(Kind::Instructions), Some(ipc::HomeKind::GlobalInstructions));
}

fn open_linked(cx: &mut TestAppContext, linked: mock::Linked) -> (gpui::Entity<KitScreen>, &mut VisualTestContext) {
    ready(cx);
    let (screen, cx) = cx.add_window_view(|window, cx| KitScreen::mock_linked(linked, window, cx));
    cx.update(|window, cx| screen.update(cx, |screen, cx| screen.enter(window, cx)));
    cx.run_until_parked();
    (screen, cx)
}

fn kind_on(screen: &gpui::Entity<KitScreen>, cx: &mut VisualTestContext, kind: Kind) -> Option<bool> {
    screen.read_with(cx, |screen, _| match &screen.link {
        super::Load::Read(link) => sync::kind_on(link, kind),
        _ => None,
    })
}

#[gpui::test]
fn space_in_the_rail_switches_the_kind_over_the_link(cx: &mut TestAppContext) {
    let (screen, cx) = open_linked(cx, mock::Linked::Apart);
    assert_eq!(kind_on(&screen, cx, Kind::Skills), Some(true));
    cx.simulate_keystrokes("shift-tab");
    assert_eq!(screen.read_with(cx, |screen, _| screen.pane), Pane::Rail);
    cx.simulate_keystrokes("space");
    assert_eq!(kind_on(&screen, cx, Kind::Skills), Some(false), "Space in the rail is the kind's switch");
    assert!(screen.read_with(cx, |screen, _| screen.standings.is_empty()), "with Skills off, no row is marked");
    cx.simulate_keystrokes("space");
    assert_eq!(kind_on(&screen, cx, Kind::Skills), Some(true));
    assert!(!screen.read_with(cx, |screen, _| screen.standings.is_empty()), "on again, the marks come back");
    assert!(drawn(cx, gpui::Role::Switch, "Cross Skills"), "the switch is drawn and named");
}

#[gpui::test]
fn tab_reaches_each_side_of_a_clash_after_the_read_pane_and_enter_keeps_it(cx: &mut TestAppContext) {
    let (screen, cx) = open_linked(cx, mock::Linked::Apart);
    assert!(drawn(cx, gpui::Role::Region, "Clash with Work Mac"), "the first row clashes, and its detail shows both sides");
    cx.simulate_keystrokes("tab");
    assert_eq!(screen.read_with(cx, |screen, _| (screen.stop.clone(), screen.pane)), (Some(Stop::CrossItem), Pane::List), "the picked row's switch comes after the list");
    cx.simulate_keystrokes("tab tab");
    let keep = |keep| Some(Stop::Keep { mac: mock::WORK_MAC.0.into(), keep });
    assert_eq!(screen.read_with(cx, |screen, _| screen.stop.clone()), keep(ipc::KitKeep::Here));
    cx.simulate_keystrokes("tab");
    assert_eq!(screen.read_with(cx, |screen, _| screen.stop.clone()), keep(ipc::KitKeep::There));
    cx.simulate_keystrokes("enter");
    cx.run_until_parked();
    let after = screen.read_with(cx, |screen, _| (screen.stop.clone(), screen.pane, screen.standings.get(mock::RELEASE_NOTES_ID).cloned(), screen.file_of_picked().map(|file| file.text.clone())));
    assert_eq!(after.0, None, "the keys go back to the read pane");
    assert_eq!(after.1, Pane::Detail);
    assert_eq!(after.2, Some(sync::Standing::Same("Work Mac".into())), "settled, the row reads the same");
    assert!(after.3.is_some_and(|text| text.contains("Wrap at 72.")), "Work Mac's version is what the file holds now");
    cx.simulate_keystrokes("tab");
    assert_eq!(screen.read_with(cx, |screen, _| (screen.stop.clone(), screen.pane)), (None, Pane::Rail), "with no clash, Tab wraps to the rail");
}

#[gpui::test]
fn with_no_mac_linked_tab_reaches_the_bars_link_before_the_rail(cx: &mut TestAppContext) {
    let (screen, cx) = open_linked(cx, mock::Linked::Unlinked);
    assert!(drawn(cx, gpui::Role::Button, "Link a Mac in Settings"));
    cx.simulate_keystrokes("shift-tab shift-tab");
    assert_eq!(screen.read_with(cx, |screen, _| screen.stop.clone()), Some(Stop::Account));
}

fn rows_of(link: &ipc::KitLink, kind: Kind) -> Vec<model::Row> {
    let Read::Rows(mut rows) = model::read_kind(kind, &mock::inventory(), &[]) else { panic!("the mock's rows read") };
    let there = sync::elsewhere(link, kind, &rows);
    rows.extend(there);
    rows
}

fn item(kind: ipc::HomeKind, name: &str) -> ipc::HomeItem {
    ipc::HomeItem { kind, name: name.into(), repository: None }
}

fn row_named<'a>(rows: &'a [model::Row], name: &str) -> &'a model::Row {
    rows.iter().find(|row| row.name == name).unwrap_or_else(|| panic!("the mock has {name}"))
}

#[test]
fn a_kind_is_on_off_or_some_where_it_is_off_and_some_of_its_items_cross_by_themselves() {
    use sync::KindCross::*;
    let link = mock::MockKit::linked(mock::Linked::Apart).link;
    assert_eq!(sync::kind_cross(&link, Kind::Skills), Some(On), "on: every item, and any added later");
    assert_eq!(sync::kind_cross(&link, Kind::SubAgents), Some(Partly), "off, with code-reviewer on by itself");
    assert_eq!(sync::kind_cross(&link, Kind::Plugins), Some(Off), "off, with nothing on by itself");
    assert_eq!(sync::kind_cross(&link, Kind::OutputStyles), None, "output styles do not cross");
    let agents = rows_of(&link, Kind::SubAgents);
    assert!(sync::row_crosses(&link, row_named(&agents, "code-reviewer")));
    assert!(!sync::row_crosses(&link, row_named(&agents, "test-writer")));
}

#[test]
fn the_rails_switch_takes_some_or_off_to_on_and_on_to_everything_off_items_first() {
    use ipc::HomeKind;
    use sync::Cross;
    let mut link = mock::MockKit::linked(mock::Linked::Apart).link;
    assert_eq!(sync::rail_press(&link, Kind::SubAgents), vec![Cross::Kind(HomeKind::SubAgents, true)], "from some");
    assert_eq!(sync::rail_press(&link, Kind::Plugins), vec![Cross::Kind(HomeKind::Plugins, true)], "from off");
    assert_eq!(sync::rail_press(&link, Kind::Skills), vec![Cross::Kind(HomeKind::Skills, false)], "from on, with no item on by itself");
    // On, with an item still on by itself from before: it goes off first, so a refusal leaves the kind on.
    link.kinds.iter_mut().filter(|one| one.kind == HomeKind::SubAgents).for_each(|one| one.on = true);
    assert_eq!(sync::rail_press(&link, Kind::SubAgents), vec![Cross::Item(item(HomeKind::SubAgents, "code-reviewer"), false), Cross::Kind(HomeKind::SubAgents, false)]);
    assert_eq!(sync::rail_press(&link, Kind::OutputStyles), Vec::new());
}

#[test]
fn a_rows_switch_sends_its_item_while_the_kind_is_off_and_keeps_the_rest_crossing_while_it_is_on() {
    use ipc::HomeKind;
    use sync::Cross;
    let mut link = mock::MockKit::linked(mock::Linked::Apart).link;
    let agents = rows_of(&link, Kind::SubAgents);
    assert_eq!(sync::row_press(&link, row_named(&agents, "test-writer"), &agents), vec![Cross::Item(item(HomeKind::SubAgents, "test-writer"), true)], "kind off: on by itself");
    assert_eq!(sync::row_press(&link, row_named(&agents, "code-reviewer"), &agents), vec![Cross::Item(item(HomeKind::SubAgents, "code-reviewer"), false)], "kind off: off by itself");
    // Kind on: every other row Fleet takes goes on by itself, then the kind off. The refused row, the plugin's
    // piece and the repository's are left out; the row only on Work Mac is in.
    let skills = rows_of(&link, Kind::Skills);
    let on = |name: &str| Cross::Item(item(HomeKind::Skills, name), true);
    assert_eq!(
        sync::row_press(&link, row_named(&skills, "deploy-notes"), &skills),
        vec![on("release-notes"), on("review-pr"), on("pdf-tools"), on("pricing-review"), Cross::Kind(HomeKind::Skills, false)],
    );
    // An item still on by itself from before goes off last; one already on is not sent again.
    link.items.push(item(HomeKind::Skills, "deploy-notes"));
    link.items.push(item(HomeKind::Skills, "review-pr"));
    assert_eq!(
        sync::row_press(&link, row_named(&skills, "deploy-notes"), &skills),
        vec![on("release-notes"), on("pdf-tools"), on("pricing-review"), Cross::Kind(HomeKind::Skills, false), Cross::Item(item(HomeKind::Skills, "deploy-notes"), false)],
    );
    assert_eq!(sync::row_press(&link, row_named(&skills, "tracker-sync"), &skills), Vec::new(), "Fleet refuses it: its switch sends nothing");
    assert_eq!(sync::row_press(&link, row_named(&skills, "frontend-design"), &skills), Vec::new(), "a plugin's piece has no switch");
}

fn sent(screen: &gpui::Entity<KitScreen>, cx: &mut VisualTestContext) -> Vec<sync::Cross> {
    screen.read_with(cx, |screen, _| match &screen.source {
        super::Source::Mock(kit) => kit.sent.clone(),
        super::Source::Live(_) => Vec::new(),
    })
}

fn crosses(screen: &gpui::Entity<KitScreen>, cx: &mut VisualTestContext, name: &str) -> bool {
    screen.read_with(cx, |screen, _| match (&screen.link, &screen.read) {
        (super::Load::Read(link), Read::Rows(rows)) => sync::row_crosses(link, row_named(rows, name)),
        _ => false,
    })
}

fn kind_cross(screen: &gpui::Entity<KitScreen>, cx: &mut VisualTestContext, kind: Kind) -> Option<sync::KindCross> {
    screen.read_with(cx, |screen, _| match &screen.link {
        super::Load::Read(link) => sync::kind_cross(link, kind),
        _ => None,
    })
}

#[gpui::test]
fn a_row_switched_off_while_its_kind_is_on_leaves_the_kind_at_some_with_the_rest_crossing(cx: &mut TestAppContext) {
    use ipc::HomeKind;
    let (screen, cx) = open_linked(cx, mock::Linked::Apart);
    assert!(drawn(cx, gpui::Role::Switch, "Cross deploy-notes"), "every row carries its own switch");
    screen.update(cx, |screen, cx| screen.cross_row("skills:/Users/user/.claude/skills/deploy-notes:deploy-notes", cx));
    cx.run_until_parked();
    let on = |name: &str| sync::Cross::Item(item(HomeKind::Skills, name), true);
    assert_eq!(sent(&screen, cx), vec![on("release-notes"), on("review-pr"), on("pdf-tools"), on("pricing-review"), sync::Cross::Kind(HomeKind::Skills, false)]);
    assert_eq!(kind_cross(&screen, cx, Kind::Skills), Some(sync::KindCross::Partly));
    assert!(!crosses(&screen, cx, "deploy-notes"));
    assert!(["release-notes", "review-pr", "pdf-tools"].iter().all(|name| crosses(&screen, cx, name)), "the rest keep crossing");
    // The rail's switch from some: the kind on, every item.
    screen.update(cx, |screen, cx| screen.cross_kind(Kind::Skills, cx));
    cx.run_until_parked();
    assert_eq!(sent(&screen, cx).last(), Some(&sync::Cross::Kind(HomeKind::Skills, true)));
    assert_eq!(kind_cross(&screen, cx, Kind::Skills), Some(sync::KindCross::On));
    assert!(crosses(&screen, cx, "deploy-notes"));
    // From on: each item on by itself off, then the kind.
    let before = sent(&screen, cx).len();
    screen.update(cx, |screen, cx| screen.cross_kind(Kind::Skills, cx));
    cx.run_until_parked();
    let off = |name: &str| sync::Cross::Item(item(HomeKind::Skills, name), false);
    assert_eq!(sent(&screen, cx)[before..], [off("release-notes"), off("review-pr"), off("pdf-tools"), off("pricing-review"), sync::Cross::Kind(HomeKind::Skills, false)]);
    assert_eq!(kind_cross(&screen, cx, Kind::Skills), Some(sync::KindCross::Off));
}

#[gpui::test]
fn a_refusal_part_way_says_fleets_words_and_leaves_the_rest_crossing(cx: &mut TestAppContext) {
    use ipc::HomeKind;
    let (screen, cx) = open_linked(cx, mock::Linked::Apart);
    screen.update(cx, |screen, _| {
        if let super::Source::Mock(kit) = &mut screen.source {
            kit.refuse = Some(sync::Cross::Kind(HomeKind::Skills, false));
        }
    });
    screen.update(cx, |screen, cx| screen.cross_row("skills:/Users/user/.claude/skills/deploy-notes:deploy-notes", cx));
    cx.run_until_parked();
    assert_eq!(sent(&screen, cx).last(), Some(&sync::Cross::Kind(HomeKind::Skills, false)), "the items went first, and the kind was the call refused");
    assert_eq!(screen.read_with(cx, |screen, _| screen.refused.clone()).as_deref(), Some("Fleet could not write the link's switches on this Mac."));
    assert_eq!(kind_cross(&screen, cx, Kind::Skills), Some(sync::KindCross::On));
    assert!(["release-notes", "review-pr", "pdf-tools", "deploy-notes"].iter().all(|name| crosses(&screen, cx, name)), "nothing stopped crossing");
}

#[gpui::test]
fn tab_reaches_the_picked_rows_switch_and_space_presses_it(cx: &mut TestAppContext) {
    use ipc::HomeKind;
    let (screen, cx) = open_linked(cx, mock::Linked::Apart);
    cx.update(|window, cx| screen.update(cx, |screen, cx| screen.show_kind("sub-agents", window, cx)));
    cx.run_until_parked();
    assert!(drawn(cx, gpui::Role::Switch, "Cross Sub agents"));
    assert!(drawn(cx, gpui::Role::Switch, "Cross test-writer"));
    assert!(!drawn(cx, gpui::Role::Switch, "Cross code-reviewer alone"), "the detail's own switch is gone");
    cx.simulate_keystrokes("tab");
    assert_eq!(screen.read_with(cx, |screen, _| screen.stop.clone()), Some(Stop::CrossItem));
    cx.simulate_keystrokes("space");
    cx.run_until_parked();
    assert_eq!(sent(&screen, cx), vec![sync::Cross::Item(item(HomeKind::SubAgents, "code-reviewer"), false)]);
    assert_eq!(kind_cross(&screen, cx, Kind::SubAgents), Some(sync::KindCross::Off), "nothing of the kind crosses now");
    // A row Fleet refuses has a switch, and no Tab stop.
    cx.update(|window, cx| screen.update(cx, |screen, cx| screen.show_kind("skills", window, cx)));
    screen.update(cx, |screen, cx| screen.pick("skills:/Users/user/.claude/skills/tracker-sync:tracker-sync".into(), cx));
    cx.run_until_parked();
    assert!(drawn(cx, gpui::Role::Switch, "Cross tracker-sync"));
    assert!(!screen.read_with(cx, |screen, _| screen.stops().contains(&Stop::CrossItem)));
}
