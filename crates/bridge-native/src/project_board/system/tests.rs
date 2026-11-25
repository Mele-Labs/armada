//! The System block on the mock Fleet: the levels, the keys, the panel's nine sections and what each says when it is empty, and
//! the presses that send something. The pure parts (the arrangement, the zoom, the tree) have their tests beside them.

use gpui::{div, prelude::*, AnyView, Bounds, Context, Entity, Modifiers, Pixels, Role, TestAppContext, VisualTestContext, Window};
use ipc::AdditionAnswer;

use super::panel::{code_line, issues_line, looks_line, owner_line, share_line, NO_DOCS, NO_FEATURES, NO_JOB, NO_MONITOR, NO_PULLS, SECTIONS};
use super::SystemBlock;
use ipc::SystemPartKind;

fn ready(cx: &mut TestAppContext) {
    cx.update(|cx| {
        gpui_component::init(cx);
        crate::look::init(cx);
        crate::keymap::install(&crate::keymap::Rebinding::default(), cx);
    });
}

struct Rooted(AnyView);

impl Render for Rooted {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        crate::labels::label_root(div().size_full().child(self.0.clone()))
    }
}

fn redraw(cx: &mut VisualTestContext) {
    for _ in 0..4 {
        cx.run_until_parked();
        cx.update(|window, cx| window.draw(cx).clear(cx));
    }
}

fn find(cx: &mut VisualTestContext, role: Role, name: &str) -> Option<Bounds<Pixels>> {
    cx.update(|window, cx| cx.default_global::<crate::labels::LabelBook>().find(window.window_handle().window_id(), role, name).map(|one| one.bounds))
}

fn drawn(cx: &mut VisualTestContext, role: Role, name: &str) -> bool {
    find(cx, role, name).is_some()
}

fn press(cx: &mut VisualTestContext, role: Role, name: &str) {
    let at = find(cx, role, name).unwrap_or_else(|| panic!("{name} is drawn")).center();
    cx.simulate_click(at, Modifiers::default());
    redraw(cx);
}

/// A block on the mock Fleet, following `project`, read and drawn.
fn block<'a>(cx: &'a mut TestAppContext, project: &str) -> (Entity<SystemBlock>, &'a mut VisualTestContext) {
    ready(cx);
    let mut held = None;
    let project = project.to_string();
    let (_, cx) = cx.add_window_view(|_, cx| {
        let block = cx.new(|cx| {
            let mut block = SystemBlock::mock(cx);
            block.follow(&project, "Project", vec!["repo".into()], cx);
            block
        });
        held = Some(block.clone());
        Rooted(block.into())
    });
    cx.update(|window, _| window.activate_window());
    redraw(cx);
    (held.expect("the block was made"), cx)
}

const ARMADA: &str = crate::features::mock::PROJECT;

#[test]
fn the_nine_sections_stand_in_the_order_the_panel_draws_them() {
    let titles: Vec<&str> = SECTIONS.iter().map(|(title, _)| *title).collect();
    assert_eq!(titles, ["Features", "Code", "Working on it", "Owner", "Looks like", "Docs", "Monitor", "Open issues", "Open pull requests"]);
}

#[test]
fn an_empty_section_says_why_in_the_words_the_brief_names() {
    assert_eq!(NO_FEATURES, "No feature owns files in it yet.");
    assert_eq!(NO_JOB, "No Job is writing in it.");
    assert_eq!(looks_line(false), "Not a screen.");
    assert_eq!(looks_line(true), "No shot kept yet.");
    assert_eq!(NO_DOCS, "No doc names it.");
    assert_eq!(NO_MONITOR, "Nothing to watch is declared for it.");
    assert_eq!(issues_line(true), "No open issues name it.");
    assert_eq!(issues_line(false), "No tracker connected.");
    assert_eq!(NO_PULLS, "None open.");
    assert_eq!(code_line(SystemPartKind::Person), "A person.");
    assert_eq!(code_line(SystemPartKind::Outside), "Not in this repository.");
    assert_eq!(code_line(SystemPartKind::Entry), "No folder of its own.");
    assert_eq!(owner_line(SystemPartKind::Outside, 30), "Not ours to own.");
    assert_eq!(share_line(100, 30), "every commit, last 30 days");
    assert_eq!(share_line(60, 30), "60% of commits, last 30 days");
}

#[gpui::test]
fn level_one_draws_each_part_with_a_name_and_a_skeleton_stands_until_the_level_is_read(cx: &mut TestAppContext) {
    let (_, cx) = block(cx, ARMADA);
    for name in ["You, person", "Bridge, entry point", "Fleet daemon, service", "armada.sqlite, store", "GitHub, outside"] {
        assert!(drawn(cx, Role::Button, name), "{name} is drawn");
    }
    assert!(drawn(cx, Role::Heading, "Project"), "the crumb names the project");
    assert!(!drawn(cx, Role::Image, "Loading"), "nothing is being read");
}

#[gpui::test]
fn a_press_on_a_part_selects_it_and_the_panel_draws_all_nine_sections(cx: &mut TestAppContext) {
    let (block, cx) = block(cx, ARMADA);
    press(cx, Role::Button, "Bridge, entry point");
    assert_eq!(block.read_with(cx, |block, _| block.selected.clone()), Some("bridge".to_string()));
    assert!(drawn(cx, Role::Region, "About Bridge"));
    for (title, _) in SECTIONS {
        assert!(drawn(cx, Role::Heading, title), "{title} has its heading");
    }
    assert!(drawn(cx, Role::Button, "Look inside") && drawn(cx, Role::Button, "Open folder"));
    assert!(!drawn(cx, Role::Button, "Keep"), "a part found in files is not the librarian's");
}

#[gpui::test]
fn a_part_with_nothing_of_its_own_says_why_in_every_section(cx: &mut TestAppContext) {
    let (_, cx) = block(cx, ARMADA);
    press(cx, Role::Button, "You, person");
    assert!(!drawn(cx, Role::Button, "Look inside") && !drawn(cx, Role::Button, "Open folder"));
    assert!(drawn(cx, Role::Heading, "Open pull requests"));
    // The lines are text, which the label book does not hold; the model's choice of each is tested above, and the panel draws
    // from it, so what stands here is that every section is present for a part with nothing in it.
    for (title, _) in SECTIONS {
        assert!(drawn(cx, Role::Heading, title), "{title} is drawn though empty");
    }
}

#[gpui::test]
fn return_looks_inside_the_selected_part_esc_steps_out_and_esc_at_level_one_puts_the_block_back(cx: &mut TestAppContext) {
    let (block, cx) = block(cx, ARMADA);
    press(cx, Role::Button, "Bridge, entry point");
    cx.simulate_keystrokes("enter");
    redraw(cx);
    assert_eq!(block.read_with(cx, |block, _| block.trail.depth()), 2);
    assert!(block.read_with(cx, |block, _| block.is_grown()), "a block that is zoomed into fills the board");
    assert!(drawn(cx, Role::Heading, "Bridge"), "the crumb at the end names where he is");
    assert!(drawn(cx, Role::Button, "Step out to Project"));
    assert!(drawn(cx, Role::Button, "project_board, part"), "level two draws the pieces of Bridge");
    assert!(drawn(cx, Role::Button, "Dashboard, draw its path"), "on level two the panel's feature rows draw the feature's path");
    cx.simulate_keystrokes("escape");
    redraw(cx);
    assert_eq!(block.read_with(cx, |block, _| block.trail.depth()), 1);
    assert!(block.read_with(cx, |block, _| block.is_grown()), "stepping out a level does not give the board back");
    cx.simulate_keystrokes("escape");
    redraw(cx);
    assert!(!block.read_with(cx, |block, _| block.is_grown()), "Esc at level one puts it back in its place");
}

#[gpui::test]
fn a_feature_row_of_level_two_draws_its_path_and_a_crumb_steps_back_out_to_the_project(cx: &mut TestAppContext) {
    let (block, cx) = block(cx, ARMADA);
    assert!(drawn(cx, Role::Button, "Dashboard, open the feature"), "on level one a feature row opens the feature's page");
    press(cx, Role::Button, "Bridge, entry point");
    cx.simulate_keystrokes("enter");
    redraw(cx);
    press(cx, Role::Button, "Dashboard, draw its path");
    assert_eq!(block.read_with(cx, |block, _| (block.trail.depth(), block.trail.feature().map(str::to_string))), (3, Some("dashboard".to_string())));
    assert!(drawn(cx, Role::Button, "project_board, part"));
    assert!(drawn(cx, Role::Button, "Open feature page"), "the panel's subject is the feature");
    press(cx, Role::Button, "Step out to Project");
    assert_eq!(block.read_with(cx, |block, _| block.trail.depth()), 1);
}

#[gpui::test]
fn the_arrow_keys_move_between_parts_from_the_top_left_one(cx: &mut TestAppContext) {
    let (block, cx) = block(cx, ARMADA);
    cx.update(|window, cx| {
        let focus = block.read(cx).focus.clone();
        focus.focus(window, cx);
    });
    cx.simulate_keystrokes("right");
    redraw(cx);
    let first = block.read_with(cx, |block, _| block.selected.clone());
    assert!(first.is_some(), "an arrow with nothing selected starts at the top left part");
    cx.simulate_keystrokes("right");
    redraw(cx);
    let second = block.read_with(cx, |block, _| block.selected.clone());
    assert_ne!(first, second, "and the next one moves along");
    cx.simulate_keystrokes("left");
    redraw(cx);
    assert_eq!(block.read_with(cx, |block, _| block.selected.clone()), first);
}

#[gpui::test]
fn a_folder_of_the_code_tree_opens_and_is_filled_from_the_listing(cx: &mut TestAppContext) {
    let (_, cx) = block(cx, ARMADA);
    press(cx, Role::Button, "Bridge, entry point");
    assert!(drawn(cx, Role::TreeItem, "crates/bridge-native folder"), "the root stands shut, its chain of folders folded into one row");
    assert!(!drawn(cx, Role::TreeItem, "src folder"));
    press(cx, Role::TreeItem, "crates/bridge-native folder");
    assert!(drawn(cx, Role::TreeItem, "src folder") && drawn(cx, Role::TreeItem, "Cargo.toml"), "what Fleet listed");
}

#[gpui::test]
fn new_feature_takes_a_sentence_and_start_a_job_sends_it_and_clears_the_draft(cx: &mut TestAppContext) {
    let (block, cx) = block(cx, ARMADA);
    press(cx, Role::Button, "Bridge, entry point");
    press(cx, Role::Button, "New feature");
    assert!(drawn(cx, Role::Button, "Start a Job"));
    cx.simulate_input("Show errors on the board");
    redraw(cx);
    let key = format!("system.feature.{ARMADA}.bridge");
    assert_eq!(cx.update(|_, cx| crate::drafts::kept(&key, cx)), Some("Show errors on the board".to_string()), "the draft survives a closed box");
    press(cx, Role::Button, "Start a Job");
    assert_eq!(block.read_with(cx, |block, _| block.started.clone()), ["New feature, beginning at Bridge: Show errors on the board"]);
    assert!(block.read_with(cx, |block, _| block.compose.is_none()));
    assert_eq!(cx.update(|_, cx| crate::drafts::kept(&key, cx)), None);
}

#[gpui::test]
fn start_a_job_on_a_part_sends_the_part_and_its_code_with_the_sentence(cx: &mut TestAppContext) {
    let (block, cx) = block(cx, ARMADA);
    press(cx, Role::Button, "Bridge, entry point");
    press(cx, Role::Button, "Start a Job here");
    cx.simulate_input("Make the board load faster");
    redraw(cx);
    press(cx, Role::Button, "Start a Job");
    assert_eq!(block.read_with(cx, |block, _| block.started.clone()), ["Work on Bridge, in crates/bridge-native/: Make the board load faster"]);
    assert!(block.read_with(cx, |block, _| block.compose.is_none()));
    assert_eq!(cx.update(|_, cx| crate::drafts::kept(&format!("system.work.{ARMADA}.bridge"), cx)), None);
}

#[test]
fn a_job_on_an_outside_part_is_told_what_the_part_is() {
    let part = ipc::SystemPart { id: "buildkite".into(), name: "Buildkite API".into(), kind: ipc::SystemPartKind::Outside, summary: "api.buildkite.com".into(), within: None, manifest_id: None, paths: Vec::new(), zooms: false, origin: ipc::PartOrigin::Detected { from: "files".into() } };
    assert_eq!(super::work_request(&part, "Retry a failed build"), "Work on Buildkite API (api.buildkite.com): Retry a failed build");
}

#[gpui::test]
fn esc_in_the_box_puts_the_box_away_and_does_not_step_out_of_the_level(cx: &mut TestAppContext) {
    let (block, cx) = block(cx, ARMADA);
    press(cx, Role::Button, "Bridge, entry point");
    cx.simulate_keystrokes("enter");
    redraw(cx);
    press(cx, Role::Button, "New feature");
    cx.simulate_keystrokes("escape");
    redraw(cx);
    assert!(block.read_with(cx, |block, _| block.compose.is_none()));
    assert_eq!(block.read_with(cx, |block, _| block.trail.depth()), 2, "the first Esc was the box's");
}

#[gpui::test]
fn a_part_the_librarian_added_has_keep_work_and_remove_and_each_answers(cx: &mut TestAppContext) {
    let (block, cx) = block(cx, "pocket");
    assert!(drawn(cx, Role::Button, "Bank exports, outside, from the librarian"));
    press(cx, Role::Button, "Bank exports, outside, from the librarian");
    for name in ["Keep", "Work with the librarian", "Remove"] {
        assert!(drawn(cx, Role::Button, name), "{name}");
    }
    press(cx, Role::Button, "Work with the librarian");
    cx.simulate_input("It is the inbox folder, not the bank");
    redraw(cx);
    press(cx, Role::Button, "Send note");
    let answered = block.read_with(cx, |block, _| block.answered.clone());
    assert_eq!(answered.len(), 1);
    assert_eq!(answered[0].addition, "bank-exports");
    assert_eq!(answered[0].answer, AdditionAnswer::Rework { note: "It is the inbox folder, not the bank".into() });

    press(cx, Role::Button, "Keep");
    assert_eq!(block.read_with(cx, |block, _| block.answered.len()), 2);
    assert!(drawn(cx, Role::Button, "Bank exports, outside"), "kept, it is a part like the rest");
    assert!(!drawn(cx, Role::Button, "Bank exports, outside, from the librarian"));
}

#[gpui::test]
fn remove_takes_the_part_and_its_links_off_the_map(cx: &mut TestAppContext) {
    let (block, cx) = block(cx, "pocket");
    press(cx, Role::Button, "Bank exports, outside, from the librarian");
    press(cx, Role::Button, "Remove");
    assert!(!drawn(cx, Role::Button, "Bank exports, outside, from the librarian"));
    assert!(block.read_with(cx, |block, _| block.level().is_some_and(|map| map.parts.iter().all(|part| part.id != "bank") && map.links.iter().all(|link| link.to != "bank"))));
}

#[gpui::test]
fn the_project_and_a_feature_have_all_nine_sections_too(cx: &mut TestAppContext) {
    let (block, cx) = block(cx, ARMADA);
    let sections = |cx: &mut VisualTestContext| super::panel::SECTIONS.iter().all(|(title, _)| drawn(cx, Role::Heading, title));
    assert!(sections(cx), "the project, with nothing selected");
    block.update_in(cx, |block, window, cx| {
        block.select(Some("bridge".into()), cx);
        block.look_inside(window, cx);
        block.follow_feature("dashboard", window, cx);
    });
    redraw(cx);
    assert!(sections(cx), "a feature's path, with nothing selected");
}

#[test]
fn the_mock_says_of_a_part_what_its_map_says() {
    let jobs = super::mock::answers(ARMADA, "b_jobs");
    assert_eq!(jobs.features.iter().map(|one| one.slug.as_str()).collect::<Vec<_>>(), ["jobs-list"]);
    assert!(jobs.has_screen, "a piece of Bridge draws a screen");
    assert!(!super::mock::answers(ARMADA, "f_daemon").has_screen);
    assert!(!super::mock::answers(ARMADA, "b_jobs").features.is_empty());
    assert!(super::mock::answers("auto-ynab", "s_rev").has_screen);
}

#[gpui::test]
fn on_level_two_a_link_inside_a_lane_is_drawn_only_while_a_piece_at_one_end_is_hovered(cx: &mut TestAppContext) {
    let (block, cx) = block(cx, "ynab");
    press(cx, Role::Button, "Web app, entry point");
    cx.simulate_keystrokes("enter");
    redraw(cx);
    assert_eq!(block.read_with(cx, |block, _| block.trail.depth()), 2);
    // main.py calls the backend's other modules, which stand in its own lane; the screens call main.py from theirs.
    let shown = |block: &SystemBlock| {
        let (_, laid) = block.laid.as_ref().expect("the level is laid out");
        let map = block.level().expect("the level is read");
        let drawn = super::arrange::shown(laid, map, &block.hover, block.selected.as_deref());
        let inside = |id: &str| laid.edges.iter().zip(&drawn).filter(|(edge, _)| edge.lane && laid.nodes[edge.from].id == id).map(|(_, drawn)| *drawn).collect::<Vec<bool>>();
        let between = laid.edges.iter().zip(&drawn).filter(|(edge, _)| !edge.lane).all(|(_, drawn)| *drawn);
        (inside("main"), between)
    };
    let (inside, between) = block.read_with(cx, |block, _| shown(block));
    assert!(!inside.is_empty() && inside.iter().all(|drawn| !drawn), "at rest none of main.py's links to its own lane is drawn");
    assert!(between, "every link between lanes is");
    let at = find(cx, Role::Button, "main.py, part").expect("main.py is drawn").center();
    cx.simulate_mouse_move(at, None, Modifiers::default());
    redraw(cx);
    assert_eq!(block.read_with(cx, |block, _| block.hover.clone()), super::arrange::Hover::Part("main".into()));
    let (inside, between) = block.read_with(cx, |block, _| shown(block));
    assert!(inside.iter().all(|drawn| *drawn), "hovering main.py draws them");
    assert!(between, "and the links between lanes are still drawn");
}
