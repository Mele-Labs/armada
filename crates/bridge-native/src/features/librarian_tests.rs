use std::time::Duration;

use chrono::Utc;
use gpui::{Role, TestAppContext, VisualTestContext};

use super::librarian::{card, Card, Press};
use super::mock::{self, Reading};
use super::steps::{rows, Run, Stand};
use super::{chosen, FeaturesScreen, Load};

fn open(cx: &mut TestAppContext) -> (gpui::Entity<FeaturesScreen>, &mut VisualTestContext) {
    cx.update(|cx| {
        gpui_component::init(cx);
        crate::look::init(cx);
        crate::keymap::install(&crate::keymap::Rebinding::default(), cx);
        crate::titlebar::bind(cx);
        chosen::forget_all(cx);
    });
    let (screen, cx) = cx.add_window_view(|window, cx| FeaturesScreen::mock(window, cx));
    cx.update(|window, cx| screen.update(cx, |screen, cx| screen.enter(window, cx)));
    cx.run_until_parked();
    (screen, cx)
}

/// Draw a frame with a book that holds nothing from earlier ones: a test window has no label root to clear it.
fn fresh(cx: &mut VisualTestContext) {
    cx.update(|_, cx| *cx.default_global::<crate::labels::LabelBook>() = crate::labels::LabelBook::default());
    cx.update(|window, cx| window.draw(cx).clear(cx));
}

fn drawn(cx: &mut VisualTestContext, role: Role, name: &str) -> bool {
    fresh(cx);
    cx.update(|window, cx| cx.default_global::<crate::labels::LabelBook>().find(window.window_handle().window_id(), role, name).is_some())
}

/// The middle of the element with this role and name, as drawn now.
fn centre(cx: &mut VisualTestContext, role: Role, name: &str) -> gpui::Point<gpui::Pixels> {
    fresh(cx);
    cx.update(|window, cx| cx.default_global::<crate::labels::LabelBook>().find(window.window_handle().window_id(), role, name).map(|found| found.bounds.center())).unwrap_or_else(|| panic!("{role:?} {name} is not drawn"))
}

fn click(cx: &mut VisualTestContext, role: Role, name: &str) {
    let at = centre(cx, role, name);
    cx.simulate_click(at, gpui::Modifiers::none());
}

fn card_of(screen: &gpui::Entity<FeaturesScreen>, cx: &mut VisualTestContext) -> Option<Card> {
    screen.read_with(cx, |screen, _| screen.librarian_card())
}

/// The mock's run, three steps of 200 ms each, takes this long to come back.
fn let_it_read(cx: &mut VisualTestContext) {
    for _ in 0..4 {
        cx.executor().advance_clock(Duration::from_millis(250));
        cx.run_until_parked();
    }
}

/// The screen as if the library were read with the librarian at `at`.
fn show(screen: &gpui::Entity<FeaturesScreen>, at: Reading, cx: &mut VisualTestContext) {
    screen.update(cx, |screen, cx| {
        let library = ipc::FeatureLibrary { librarian: mock::librarian(at), ..mock::library() };
        screen.run_read(&library);
        screen.library = Load::Read(library);
        cx.notify();
    });
}

/// The card opened as a press leaves it, with no run started.
fn unfold(screen: &gpui::Entity<FeaturesScreen>, cx: &mut VisualTestContext) {
    screen.update(cx, |screen, cx| {
        screen.lib_card.open = screen.library_project.clone();
        cx.notify();
    });
}

fn open_now(screen: &gpui::Entity<FeaturesScreen>, cx: &mut VisualTestContext) -> bool {
    screen.read_with(cx, |screen, _| screen.librarian_open())
}

fn told(step: &str, state: ipc::LibrarianStepState, said: &str) -> ipc::LibrarianStepMoved {
    ipc::LibrarianStepMoved { project: mock::PROJECT.into(), run: ipc::Instant::carried("2026-11-19T10:00:00Z"), step: step.into(), state, said: said.into() }
}

fn tell(screen: &gpui::Entity<FeaturesScreen>, moved: ipc::LibrarianStepMoved, cx: &mut VisualTestContext) {
    screen.update(cx, |screen, cx| screen.step_moved(&moved, cx));
}

#[gpui::test]
fn behind_says_where_it_stopped_and_lists_the_merged_numbers(cx: &mut TestAppContext) {
    let behind = card(&mock::librarian(Reading::Behind), &Press::Idle, None, Utc::now());
    assert_eq!(behind.said(), "Not read since #3861");
    assert_eq!(behind.merged_said().as_deref(), Some("#3862 #3863 #3864"));
    assert_eq!(behind.button(), Some(("Catch up", true, false)), "Catch up is the primary button");
    let (screen, cx) = open(cx);
    assert!(drawn(cx, Role::Label, "Not read since #3861"), "the closed line says where it stopped");
    assert!(drawn(cx, Role::Button, "Catch up"));
    unfold(&screen, cx);
    assert!(drawn(cx, Role::Label, "Not read since #3861. Catch up will:"));
    assert!(drawn(cx, Role::Button, "Catch up"));
}

#[test]
fn never_current_and_working_say_what_they_are() {
    let now = Utc::now();
    let never = card(&mock::librarian(Reading::Never), &Press::Idle, None, now);
    assert_eq!((never.said().as_str(), never.button()), ("Not read yet", Some(("Catch up", false, false))));
    let current = card(&mock::librarian(Reading::Current), &Press::Idle, None, now);
    assert_eq!(current.said(), "Read through #3864, 4 min ago");
    assert_eq!(current.button(), None);
    let working = card(&mock::librarian(Reading::Behind), &Press::Working, None, now);
    assert_eq!((working.said().as_str(), working.button()), ("Catching up", Some(("Stop", false, false))), "the button stops the run while it goes");
}

#[gpui::test]
fn a_press_reads_and_comes_back_current_with_the_new_read_through(cx: &mut TestAppContext) {
    let (screen, cx) = open(cx);
    assert!(matches!(card_of(&screen, cx), Some(Card::Behind { since: Some(3861), .. })));
    cx.simulate_keystrokes("c");
    assert_eq!(card_of(&screen, cx), Some(Card::Working));
    assert!(drawn(cx, Role::Image, "Catching up"), "the spinner glyph names what it is doing");
    assert!(drawn(cx, Role::Label, "Catching up"));
    let_it_read(cx);
    let Some(Card::Current { through, .. }) = card_of(&screen, cx) else { panic!("it is current once the run is back: {:?}", card_of(&screen, cx)) };
    assert_eq!(through, Some(3864), "the newest merged pull request is what it read through");
    assert_eq!(card_of(&screen, cx).and_then(|card| card.button()), None, "a current card has no button");
}

#[gpui::test]
fn a_refused_press_shows_the_sentence_and_retry(cx: &mut TestAppContext) {
    let (screen, cx) = open(cx);
    screen.update(cx, |screen, _| screen.mock_refusal = Some("No project is named armada".into()));
    cx.simulate_keystrokes("c");
    let_it_read(cx);
    assert_eq!(card_of(&screen, cx), Some(Card::Failed("No project is named armada".into())));
    assert!(drawn(cx, Role::Label, "No project is named armada"));
    assert!(drawn(cx, Role::Button, "Retry"));
    screen.update(cx, |screen, _| screen.mock_refusal = None);
    cx.simulate_keystrokes("c");
    let_it_read(cx);
    assert!(matches!(card_of(&screen, cx), Some(Card::Current { .. })), "Retry runs it again");
}

#[gpui::test]
fn a_chip_switch_changes_the_project_the_list_asks_for(cx: &mut TestAppContext) {
    let (screen, cx) = open(cx);
    let asked = |screen: &gpui::Entity<FeaturesScreen>, cx: &mut VisualTestContext| screen.read_with(cx, |screen, cx| (screen.wanted(cx), screen.listed.shown.iter().map(|one| one.row.title.clone()).collect::<Vec<_>>()));
    assert!(drawn(cx, Role::Button, "Armada") && drawn(cx, Role::Button, "Pocket") && drawn(cx, Role::Button, "Store"), "one chip for each project");
    assert_eq!(asked(&screen, cx).0, None, "nothing is chosen yet");
    cx.simulate_keystrokes("2");
    assert_eq!(asked(&screen, cx), (Some("pocket".to_string()), vec!["Pair a phone".to_string(), "Offline queue".to_string()]));
    assert_eq!(cx.update(|_, cx| chosen::get(cx)).as_deref(), Some("pocket"), "the choice is kept for the next start");
    assert_eq!(card_of(&screen, cx).map(|card| card.button()), Some(None), "Pocket's librarian is current, so its card has no button");
    cx.simulate_keystrokes("9");
    assert_eq!(asked(&screen, cx).0.as_deref(), Some("pocket"), "a number past the last chip does nothing");
    cx.simulate_keystrokes("3");
    assert_eq!(card_of(&screen, cx), Some(Card::Never), "Store's librarian never ran");
}

#[gpui::test]
fn the_card_lists_the_steps_a_press_will_take_with_free_and_model(cx: &mut TestAppContext) {
    let (screen, cx) = open(cx);
    assert!(!drawn(cx, Role::List, "Catch up steps"), "the closed line lists no steps");
    unfold(&screen, cx);
    assert!(drawn(cx, Role::List, "Catch up steps"));
    assert!(drawn(cx, Role::ListItem, "Read what merged, free: #3862 #3863 #3864, from Fleet's own record"));
    assert!(drawn(cx, Role::ListItem, "Write what changed since you looked, model: One model call from tonight's share"));
    assert!(drawn(cx, Role::ListItem, "Mark the catalog read, free: Read through #3864"));
    assert!(!drawn(cx, Role::Button, "Stop"), "Stop is for a run going on");
    let plan = rows(&mock::librarian(Reading::Behind).steps, None);
    assert!(plan.iter().all(|row| row.stand == Stand::Waiting && !row.dim), "before a run each step is an empty ring, not dimmed");
    assert_eq!(plan.iter().map(|row| row.model).collect::<Vec<_>>(), [false, true, false]);
}

#[gpui::test]
fn a_step_moving_ticks_it_and_shows_what_it_said(cx: &mut TestAppContext) {
    let (screen, cx) = open(cx);
    cx.simulate_keystrokes("c");
    tell(&screen, told("read_merges", ipc::LibrarianStepState::Running, ""), cx);
    assert!(drawn(cx, Role::ListItem, "Read what merged, free, running: #3862 #3863 #3864, from Fleet's own record"));
    assert!(drawn(cx, Role::ListItem, "Write what changed since you looked, model, to do: One model call from tonight's share"), "a step not reached is waiting");
    tell(&screen, told("read_merges", ipc::LibrarianStepState::Done, "Read 3 merges"), cx);
    assert!(drawn(cx, Role::ListItem, "Read what merged, free, done: Read 3 merges"), "its words take the place of the detail");
    assert!(drawn(cx, Role::Label, "Catching up"));
    let dimmed = screen.read_with(cx, |screen, _| screen.step_rows(false).into_iter().filter(|row| row.dim).count());
    assert_eq!(dimmed, 2, "the steps still to do are dimmed");
}

#[gpui::test]
fn a_move_of_another_run_replaces_the_one_shown(cx: &mut TestAppContext) {
    let (screen, cx) = open(cx);
    tell(&screen, told("read_merges", ipc::LibrarianStepState::Done, "Read 3 merges"), cx);
    let mut later = told("read_merges", ipc::LibrarianStepState::Running, "");
    later.run = ipc::Instant::carried("2026-11-19T11:00:00Z");
    tell(&screen, later, cx);
    let shown = screen.read_with(cx, |screen, _| screen.step_rows(false));
    assert_eq!(shown[0].stand, Stand::Running, "the older run's tick is gone");
    let mut elsewhere = told("read_merges", ipc::LibrarianStepState::Done, "Read 9 merges");
    elsewhere.project = "pocket".into();
    tell(&screen, elsewhere, cx);
    assert_eq!(screen.read_with(cx, |screen, _| screen.step_rows(false))[0].stand, Stand::Running, "another project's step is not this card's");
}

#[gpui::test]
fn stop_shows_only_while_a_run_goes_and_the_key_presses_it(cx: &mut TestAppContext) {
    let (screen, cx) = open(cx);
    assert!(!drawn(cx, Role::Button, "Stop"));
    show(&screen, Reading::Running, cx);
    assert!(!open_now(&screen, cx) && !drawn(cx, Role::Button, "Stop"), "a run another Bridge started does not open the card");
    unfold(&screen, cx);
    assert!(drawn(cx, Role::Button, "Stop"), "a run another Bridge started shows Stop once the card is open");
    assert!(drawn(cx, Role::ListItem, "Read what merged, free, done: Read 3 merges"));
    assert!(drawn(cx, Role::ListItem, "Write what changed since you looked, model, running: One model call from tonight's share"));
    assert_eq!(card_of(&screen, cx).and_then(|card| card.button()), Some(("Stop", false, false)), "Stop is not the primary button");
    cx.simulate_keystrokes("c");
    assert!(screen.read_with(cx, |screen, _| screen.stopping), "the same key asks it to stop");
    show(&screen, Reading::Current, cx);
    screen.update(cx, |screen, _| screen.run.as_mut().map(|run| run.close(false)));
    assert_eq!(card_of(&screen, cx).and_then(|card| card.button()), None, "Stop goes when it ended");
}

#[gpui::test]
fn a_stopped_run_skips_the_steps_after_the_one_running(cx: &mut TestAppContext) {
    let (screen, cx) = open(cx);
    cx.simulate_keystrokes("c");
    cx.executor().advance_clock(Duration::from_millis(100));
    cx.run_until_parked();
    cx.simulate_keystrokes("c");
    let_it_read(cx);
    let shown = screen.read_with(cx, |screen, _| screen.step_rows(false));
    assert_eq!(shown.iter().map(|row| row.stand).collect::<Vec<_>>(), [Stand::Done, Stand::Skipped, Stand::Skipped]);
    assert!(drawn(cx, Role::ListItem, "Write what changed since you looked, model, skipped: Stopped"));
    assert!(matches!(card_of(&screen, cx), Some(Card::Behind { since: Some(3861), .. })), "a stopped run did not read through");
    assert_eq!(card_of(&screen, cx).and_then(|card| card.button()), Some(("Catch up", true, false)), "it can be pressed again");
}

#[gpui::test]
fn a_full_run_ticks_every_step_and_the_card_reads_through_just_now(cx: &mut TestAppContext) {
    let (screen, cx) = open(cx);
    cx.simulate_keystrokes("c");
    let_it_read(cx);
    let shown = screen.read_with(cx, |screen, _| screen.step_rows(false));
    assert!(shown.iter().all(|row| row.stand == Stand::Done), "{shown:?}");
    assert!(drawn(cx, Role::Label, "Read through #3864, just now"));
    assert!(drawn(cx, Role::ListItem, "Mark the catalog read, free, done: Read through #3864"));
    assert_eq!(card_of(&screen, cx).and_then(|card| card.button()), None, "Stop goes with the run");
}

#[test]
fn skipped_and_failed_steps_carry_their_marks_and_their_words() {
    let plan = mock::librarian(Reading::Behind).steps;
    let moved = |step: &str, state, said: &str| told(step, state, said);
    let mut run = Run::begun(mock::PROJECT, &plan, &moved("read_merges", ipc::LibrarianStepState::Done, "Read 3 merges"));
    run.moved(&moved("since_you_looked", ipc::LibrarianStepState::Failed, "The night's share is spent"));
    run.moved(&moved("read_through", ipc::LibrarianStepState::Skipped, "Left out after a failure"));
    let shown = run.rows();
    assert_eq!(shown.iter().map(|row| row.stand).collect::<Vec<_>>(), [Stand::Done, Stand::Failed, Stand::Skipped]);
    assert_eq!(shown[1].name(), "Write what changed since you looked, model, failed: The night's share is spent");
    assert_eq!(shown[2].name(), "Mark the catalog read, free, skipped: Left out after a failure");
    assert!(run.ended() && !run.going(), "every step ended");
}

#[gpui::test]
fn skipped_and_failed_rows_draw_with_their_words(cx: &mut TestAppContext) {
    let (screen, cx) = open(cx);
    unfold(&screen, cx);
    tell(&screen, told("read_merges", ipc::LibrarianStepState::Done, "Read 3 merges"), cx);
    tell(&screen, told("since_you_looked", ipc::LibrarianStepState::Failed, "The night's share is spent"), cx);
    tell(&screen, told("read_through", ipc::LibrarianStepState::Skipped, "Left out after a failure"), cx);
    assert!(drawn(cx, Role::ListItem, "Write what changed since you looked, model, failed: The night's share is spent"));
    assert!(drawn(cx, Role::ListItem, "Mark the catalog read, free, skipped: Left out after a failure"));
}

#[gpui::test]
fn the_card_is_closed_by_default_as_one_line_with_one_button(cx: &mut TestAppContext) {
    let (screen, cx) = open(cx);
    assert!(!open_now(&screen, cx));
    assert!(drawn(cx, Role::Region, "Librarian"));
    assert!(drawn(cx, Role::Label, "Not read since #3861"));
    assert!(drawn(cx, Role::Button, "Catch up"));
    assert!(!drawn(cx, Role::List, "Catch up steps") && !drawn(cx, Role::Button, "Close") && !drawn(cx, Role::Button, "Stop"));
    show(&screen, Reading::Current, cx);
    assert!(drawn(cx, Role::Label, "Read through #3864, 4 min ago"));
    assert!(drawn(cx, Role::Button, "Update"), "a current library offers Update on the line");
    assert!(!drawn(cx, Role::Button, "Catch up"));
    show(&screen, Reading::Never, cx);
    assert!(drawn(cx, Role::Label, "Not read yet") && drawn(cx, Role::Button, "Catch up"));
}

#[gpui::test]
fn the_line_button_opens_the_card_and_starts_the_run(cx: &mut TestAppContext) {
    let (screen, cx) = open(cx);
    assert!(drawn(cx, Role::Button, "Catch up"));
    click(cx, Role::Button, "Catch up");
    assert!(open_now(&screen, cx), "the press opens the card");
    assert_eq!(card_of(&screen, cx), Some(Card::Working), "and starts the run");
    assert!(drawn(cx, Role::Button, "Stop") && drawn(cx, Role::List, "Catch up steps"));
    let_it_read(cx);
    assert!(open_now(&screen, cx), "the card stays open when the run ends");
    assert!(matches!(card_of(&screen, cx), Some(Card::Current { .. })));
    assert!(drawn(cx, Role::Button, "Close"));
}

#[gpui::test]
fn update_on_a_current_library_runs_the_librarian(cx: &mut TestAppContext) {
    let (screen, cx) = open(cx);
    show(&screen, Reading::Current, cx);
    assert!(drawn(cx, Role::Button, "Update"));
    click(cx, Role::Button, "Update");
    assert!(open_now(&screen, cx));
    assert_eq!(card_of(&screen, cx), Some(Card::Working));
    let_it_read(cx);
    assert!(drawn(cx, Role::Button, "Close"));
}

#[gpui::test]
fn close_shows_only_once_the_run_has_ended_and_returns_to_the_line(cx: &mut TestAppContext) {
    let (screen, cx) = open(cx);
    cx.simulate_keystrokes("c");
    assert!(open_now(&screen, cx));
    assert!(!drawn(cx, Role::Button, "Close"), "no Close while the run goes");
    cx.simulate_keystrokes("escape");
    assert!(open_now(&screen, cx), "Esc does not close a card whose run goes on");
    let_it_read(cx);
    assert!(drawn(cx, Role::Button, "Close"));
    click(cx, Role::Button, "Close");
    assert!(!open_now(&screen, cx));
    assert!(drawn(cx, Role::Label, "Read through #3864, just now"));
    assert!(drawn(cx, Role::Button, "Update") && !drawn(cx, Role::Button, "Close") && !drawn(cx, Role::List, "Catch up steps"));
}

#[gpui::test]
fn escape_closes_a_card_whose_run_has_ended(cx: &mut TestAppContext) {
    let (screen, cx) = open(cx);
    cx.simulate_keystrokes("c");
    let_it_read(cx);
    assert!(open_now(&screen, cx));
    cx.simulate_keystrokes("escape");
    assert!(!open_now(&screen, cx));
}

#[gpui::test]
fn a_run_that_failed_stays_open_with_its_reason_until_closed(cx: &mut TestAppContext) {
    let (screen, cx) = open(cx);
    screen.update(cx, |screen, _| screen.mock_refusal = Some("No project is named armada".into()));
    cx.simulate_keystrokes("c");
    let_it_read(cx);
    assert!(open_now(&screen, cx));
    assert!(drawn(cx, Role::Label, "No project is named armada") && drawn(cx, Role::Button, "Retry") && drawn(cx, Role::Button, "Close"));
    click(cx, Role::Button, "Close");
    assert!(!open_now(&screen, cx));
    assert!(matches!(card_of(&screen, cx), Some(Card::Behind { .. })), "the refusal is let go with the card");
    assert!(drawn(cx, Role::Button, "Catch up"));
}

#[gpui::test]
fn a_run_from_elsewhere_keeps_the_card_closed_and_the_button_opens_it_without_a_second_run(cx: &mut TestAppContext) {
    let (screen, cx) = open(cx);
    show(&screen, Reading::Running, cx);
    assert!(!open_now(&screen, cx));
    assert!(drawn(cx, Role::Label, "Reading"), "the line says Reading");
    assert!(drawn(cx, Role::Image, "Catching up"), "the spinner is its glyph");
    assert!(drawn(cx, Role::Button, "Steps"));
    click(cx, Role::Button, "Steps");
    assert!(open_now(&screen, cx));
    assert!(screen.read_with(cx, |screen, _| screen.press == Press::Idle && screen.ran.is_none()), "no second run was started");
    assert!(drawn(cx, Role::List, "Catch up steps") && drawn(cx, Role::Button, "Stop"));
    assert!(!drawn(cx, Role::Button, "Close"), "the run is not over");
}

#[gpui::test]
fn the_open_card_is_at_most_a_fifth_of_the_window_and_scrolls(cx: &mut TestAppContext) {
    let (screen, cx) = open(cx);
    unfold(&screen, cx);
    fresh(cx);
    let (card, window) = cx.update(|window, cx| {
        let id = window.window_handle().window_id();
        let book = cx.default_global::<crate::labels::LabelBook>();
        (book.find(id, Role::Region, "Librarian").map(|found| found.bounds), window.viewport_size().height)
    });
    let card = card.expect("the card is drawn");
    assert!(card.size.height <= window * 0.2 + gpui::px(1.), "{:?} of {:?}", card.size.height, window);
    assert!(drawn(cx, Role::List, "Catch up steps"));
}
