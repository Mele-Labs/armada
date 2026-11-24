//! The box that asks the librarian on the dashboard, and Rank with the librarian in the Issues block, on the mock Fleet.

use gpui::{ClipboardItem, Entity, Focusable as _, Role, TestAppContext, VisualTestContext};
use ipc::{BoardBlock, BoardLayout, Instant, LibrarianAct, LibrarianAnswered};

use super::board::Board;
use super::layout::Layout;
use super::model;
use super::presets;
use super::tests::{board_keeping, drawn, layout_of, press, redraw, screen, PROJECT};
use super::ProjectScreen;
use crate::features::ask_box::{AskBox, MockLibrarian};
use crate::settings::wire::Refused;

fn ask_box(screen: &Entity<ProjectScreen>, cx: &mut VisualTestContext) -> Entity<AskBox> {
    screen.read_with(cx, |screen, _| screen.ask_box.clone())
}

fn librarian<T>(screen: &Entity<ProjectScreen>, cx: &mut VisualTestContext, read: impl FnOnce(&mut MockLibrarian) -> T) -> T {
    ask_box(screen, cx).update(cx, |ask, _| read(ask.mock().expect("the mock librarian")))
}

fn answer(reply: &str, act: Option<LibrarianAct>) -> LibrarianAnswered {
    LibrarianAnswered { project: PROJECT.into(), asked_at: Instant::carried("2026-11-24T02:00:00Z"), reply: reply.into(), act, calls: 12, limit: 40 }
}

fn typed(screen: &Entity<ProjectScreen>, cx: &mut VisualTestContext) -> String {
    let ask = ask_box(screen, cx);
    cx.read(|cx| ask.read(cx).input.read(cx).value().to_string())
}

/// The keys in the box, `words` typed and Enter.
fn ask(screen: &Entity<ProjectScreen>, words: &str, cx: &mut VisualTestContext) {
    let input = ask_box(screen, cx).read_with(cx, |ask, _| ask.input.clone());
    cx.update(|window, cx| input.update(cx, |state, cx| state.focus(window, cx)));
    redraw(cx);
    cx.simulate_input(words);
    cx.simulate_keystrokes("enter");
    redraw(cx);
}

fn checks_only() -> BoardLayout {
    BoardLayout { blocks: vec![BoardBlock { kind: "checks".into(), x: 0, y: 0, w: 6, h: 2, view: "table".into() }] }
}

fn board_layout(screen: &Entity<ProjectScreen>, cx: &mut VisualTestContext) -> Layout {
    let board = screen.read_with(cx, |screen, _| screen.board.clone());
    layout_of(&board, cx)
}

// ---- sending -------------------------------------------------------------------------------------------------------------

#[gpui::test]
fn enter_sends_the_words_with_the_dashboards_context_and_the_board_on_screen(cx: &mut TestAppContext) {
    let (screen, cx) = screen(cx);
    ask(&screen, "Add a Docs block", cx);
    let sent = librarian(&screen, cx, |mock| mock.sent.clone());
    assert_eq!(sent.len(), 1, "one ask");
    let sent = &sent[0];
    assert_eq!((sent.project.as_str(), sent.context.as_str(), sent.words.as_str()), (PROJECT, "dashboard", "Add a Docs block"));
    let board = sent.board.as_ref().expect("the board goes with an ask from the dashboard");
    assert!(board.preset, "a board he never changed is the preset");
    assert_eq!(board.layout, board_layout(&screen, cx).to_wire(&[]), "the board as drawn");
    assert_eq!(board.columns, 6);
    let issues = board.kinds.iter().find(|kind| kind.kind == "issues").expect("every kind the board draws");
    assert_eq!(issues.views, ["table", "list", "cards"]);
    assert_eq!(typed(&screen, cx), "", "the box is empty for the next ask");
}

#[gpui::test]
fn a_pasted_path_is_a_chip_that_goes_with_the_ask(cx: &mut TestAppContext) {
    let (screen, cx) = screen(cx);
    let input = ask_box(&screen, cx).read_with(cx, |ask, _| ask.input.clone());
    cx.update(|window, cx| input.update(cx, |state, cx| state.focus(window, cx)));
    redraw(cx);
    cx.write_to_clipboard(ClipboardItem::new_string("/Users/n/plan.md".into()));
    cx.simulate_keystrokes("cmd-v");
    redraw(cx);
    assert!(drawn(cx, Role::Button, "Remove plan.md"), "a chip in the box");
    assert_eq!(typed(&screen, cx), "", "a path is not words");
    cx.simulate_input("File this under a feature");
    cx.simulate_keystrokes("enter");
    redraw(cx);
    let sent = librarian(&screen, cx, |mock| mock.sent.clone());
    assert_eq!(sent[0].files, ["/Users/n/plan.md"]);
    assert!(!drawn(cx, Role::Button, "Remove plan.md"), "the chip goes with the ask");
}

// ---- the card ------------------------------------------------------------------------------------------------------------

#[gpui::test]
fn a_board_act_is_drawn_at_once_and_undo_puts_the_board_back(cx: &mut TestAppContext) {
    let (screen, cx) = screen(cx);
    let before = board_layout(&screen, cx);
    librarian(&screen, cx, |mock| mock.next = Some(Ok(answer("Kept only Checks on the board", Some(LibrarianAct::Board { layout: checks_only() })))));
    ask(&screen, "Hide everything but Checks", cx);
    assert!(drawn(cx, Role::Image, "Done") && drawn(cx, Role::Status, "Kept only Checks on the board"));
    assert!(drawn(cx, Role::Button, "Undo"), "an act can be taken back");
    assert!(!drawn(cx, Role::Button, "Move"), "Move is for a filed doc");
    assert_eq!(board_layout(&screen, cx).to_wire(&[]), checks_only(), "the board takes the act at once");
    assert!(!drawn(cx, Role::Region, "Run"));
    press(cx, Role::Button, "Undo");
    assert_eq!(librarian(&screen, cx, |mock| mock.undone.clone()), [Instant::carried("2026-11-24T02:00:00Z")]);
    assert!(drawn(cx, Role::Status, "Put back as it was"), "the card says what was taken back");
    assert!(!drawn(cx, Role::Button, "Undo"), "and offers nothing more");
    assert_eq!(board_layout(&screen, cx), before, "the board as it was");
    let kind = screen.read_with(cx, |screen, cx| screen.board.read(cx).kind());
    assert!(before.blocks.starts_with(&presets::preset(kind).blocks), "the preset, with the project's metric blocks after it");
}

#[gpui::test]
fn a_filed_doc_offers_undo_and_move_and_a_feature_picked_by_keys_takes_it(cx: &mut TestAppContext) {
    let (screen, cx) = screen(cx);
    let filed = LibrarianAct::FiledDoc { target: "https://example.com/guide".into(), title: "Guide".into(), feature: "start-a-job".into(), feature_title: "Start a Job".into() };
    librarian(&screen, cx, |mock| mock.next = Some(Ok(answer("Filed Guide under Start a Job", Some(filed)))));
    ask(&screen, "File https://example.com/guide", cx);
    assert!(drawn(cx, Role::Button, "Undo") && drawn(cx, Role::Button, "Move"));
    press(cx, Role::Button, "Move");
    assert!(drawn(cx, Role::ListBox, "Features"), "the project's features open under the card");
    assert!(!drawn(cx, Role::ListBoxOption, "Start a Job"), "not the one it is filed under");
    assert!(drawn(cx, Role::ListBoxOption, "Job list and live status"));
    cx.simulate_keystrokes("down enter");
    redraw(cx);
    assert_eq!(librarian(&screen, cx, |mock| mock.refiled.clone()), [(Instant::carried("2026-11-24T02:00:00Z"), "pause-resume-stop-snooze".to_string())]);
    assert!(!drawn(cx, Role::ListBox, "Features"));
    assert!(drawn(cx, Role::Status, "Moved Guide to Pause, resume, stop, snooze"));
    assert!(drawn(cx, Role::Button, "Move"), "it can move again");
}

#[gpui::test]
fn an_answer_has_no_buttons_and_a_refusal_says_why_and_keeps_the_words(cx: &mut TestAppContext) {
    let (screen, cx) = screen(cx);
    librarian(&screen, cx, |mock| mock.next = Some(Ok(answer("Run starts the app on port 5173", None))));
    ask(&screen, "What runs this project?", cx);
    assert!(drawn(cx, Role::Status, "Run starts the app on port 5173"));
    assert!(!drawn(cx, Role::Button, "Undo") && !drawn(cx, Role::Image, "Done"), "an answer did nothing to take back");
    let spent = Refused { key: None, said: "limit reached".into(), code: Some("fleet.librarian_asks_spent".into()) };
    librarian(&screen, cx, |mock| mock.next = Some(Err(spent)));
    ask(&screen, "Add a Docs block", cx);
    assert!(drawn(cx, Role::Image, "Not done") && drawn(cx, Role::Status, "Tonight's asks are spent"));
    assert!(!drawn(cx, Role::Status, "Run starts the app on port 5173"), "the card is of the last ask");
    assert_eq!(typed(&screen, cx), "Add a Docs block", "a refused ask keeps his words");
    let other = Refused { key: None, said: "The project is not served".into(), code: Some("fleet.unknown_project".into()) };
    librarian(&screen, cx, |mock| mock.next = Some(Err(other)));
    cx.simulate_keystrokes("enter");
    redraw(cx);
    assert!(drawn(cx, Role::Status, "The project is not served"), "any other refusal as Fleet said it");
}

#[gpui::test]
fn tab_reaches_the_cards_buttons_and_escape_closes_the_card(cx: &mut TestAppContext) {
    let (screen, cx) = screen(cx);
    librarian(&screen, cx, |mock| mock.next = Some(Ok(answer("Kept only Checks on the board", Some(LibrarianAct::Board { layout: checks_only() })))));
    ask(&screen, "Hide everything but Checks", cx);
    cx.simulate_keystrokes("tab");
    redraw(cx);
    let (undo, input) = ask_box(&screen, cx).read_with(cx, |ask, _| (ask.undo_focus.clone(), ask.input.clone()));
    assert!(cx.update(|window, _| undo.is_focused(window)), "Tab from the box is on Undo");
    cx.simulate_keystrokes("escape");
    redraw(cx);
    assert!(!drawn(cx, Role::Group, "Librarian"), "Esc closes the card");
    assert!(cx.update(|window, cx| input.read(cx).focus_handle(cx).is_focused(window)), "the keys are back in the box");
    librarian(&screen, cx, |mock| mock.next = Some(Ok(answer("Run starts the app", None))));
    ask(&screen, "What runs?", cx);
    assert!(drawn(cx, Role::Group, "Librarian"));
    cx.simulate_keystrokes("escape");
    redraw(cx);
    assert!(!drawn(cx, Role::Group, "Librarian"), "Esc from the box closes it too");
}

#[gpui::test]
fn prefill_sets_the_words_and_the_context_of_one_ask(cx: &mut TestAppContext) {
    let (screen, cx) = screen(cx);
    let boxed = ask_box(&screen, cx);
    cx.update(|window, cx| boxed.update(cx, |ask, cx| ask.prefill("Errors doubled since the deploy", "metric:errors", window, cx)));
    redraw(cx);
    assert_eq!(typed(&screen, cx), "Errors doubled since the deploy");
    let input = boxed.read_with(cx, |ask, _| ask.input.clone());
    assert!(cx.update(|window, cx| input.read(cx).focus_handle(cx).is_focused(window)), "the box has the keys");
    cx.simulate_keystrokes("enter");
    redraw(cx);
    assert_eq!(librarian(&screen, cx, |mock| mock.sent[0].context.clone()), "metric:errors");
    assert_eq!(boxed.read_with(cx, |ask, _| ask.asking_from().to_string()), "dashboard", "the next ask is the box's own");
    cx.update(|window, cx| boxed.update(cx, |ask, cx| ask.prefill("Why is p95 up?", "metric:latency", window, cx)));
    redraw(cx);
    cx.simulate_keystrokes("escape escape");
    redraw(cx);
    assert_eq!(boxed.read_with(cx, |ask, _| ask.asking_from().to_string()), "dashboard", "Esc puts the box's own context back");
}

// ---- Rank with the librarian ---------------------------------------------------------------------------------------------

#[gpui::test]
fn rank_with_the_librarian_draws_its_order_and_a_why_under_each_row_and_reset_puts_back_the_order_by_features(cx: &mut TestAppContext) {
    let (board, cx) = board_keeping(cx, None);
    let order = |board: &Entity<Board>, cx: &mut VisualTestContext| {
        board.read_with(cx, |board, _| match &board.issues {
            Some(Ok(read)) => model::issues(&read.mine, &read.ranked.might_pick_up).into_iter().map(|item| item.key).collect::<Vec<_>>(),
            _ => Vec::new(),
        })
    };
    assert!(drawn(cx, Role::Button, "Rank with the librarian") && !drawn(cx, Role::Button, "Reset"));
    let by_features = order(&board, cx);
    press(cx, Role::Button, "Rank with the librarian");
    assert!(drawn(cx, Role::Status, "Ranked") && !drawn(cx, Role::Button, "Ranked") && drawn(cx, Role::Button, "Reset"));
    assert!(drawn(cx, Role::Label, "Offline sends are the Pocket gap you hit this week"), "the why under its row");
    assert_eq!(order(&board, cx), ["issue-armada-a3f2", "issue-armada-71cc", "issue-armada-9efp"], "his own first, then the librarian's order");
    press(cx, Role::Button, "Reset");
    assert!(drawn(cx, Role::Button, "Rank with the librarian"));
    assert!(!drawn(cx, Role::Label, "Offline sends are the Pocket gap you hit this week"));
    assert_eq!(order(&board, cx), by_features);
}

#[gpui::test]
fn lists_fleet_answers_ranked_by_the_librarian_draw_ranked(cx: &mut TestAppContext) {
    let (board, cx) = board_keeping(cx, None);
    board.update(cx, |board, cx| {
        if let Some(Ok(read)) = board.issues.as_mut() {
            read.ranked = super::mock::ranked_by_librarian();
        }
        cx.notify();
    });
    redraw(cx);
    assert!(drawn(cx, Role::Status, "Ranked") && !drawn(cx, Role::Button, "Ranked") && drawn(cx, Role::Button, "Reset"));
    assert!(drawn(cx, Role::Label, "Two features you work on most wait on it"));
}
