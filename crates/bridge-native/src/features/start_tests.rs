//! The start panel beside the list of a project with no confirmed or seeded feature, on the mock: what it draws, where the keys
//! go, and what Draft as Pending does to the list.

use gpui::{Focusable as _, Role, TestAppContext, VisualTestContext};

use super::start::{self, Drafting, StartPanel};
use super::{mock, FeaturesScreen, Load};

fn open_with(cx: &mut TestAppContext, library: ipc::FeatureLibrary) -> (gpui::Entity<FeaturesScreen>, &mut VisualTestContext) {
    cx.update(|cx| {
        gpui_component::init(cx);
        crate::look::init(cx);
        crate::keymap::install(&crate::keymap::Rebinding::default(), cx);
        crate::titlebar::bind(cx);
    });
    let (screen, cx) = cx.add_window_view(|window, cx| FeaturesScreen::mock_holding(library, window, cx));
    cx.update(|window, cx| screen.update(cx, |screen, cx| screen.enter(window, cx)));
    cx.run_until_parked();
    cx.update(|window, cx| window.draw(cx).clear(cx));
    (screen, cx)
}

fn drawn(cx: &mut VisualTestContext, role: Role, name: &str) -> bool {
    cx.update(|window, cx| window.draw(cx).clear(cx));
    cx.update(|window, cx| cx.default_global::<crate::labels::LabelBook>().find(window.window_handle().window_id(), role, name).is_some())
}

fn panel(screen: &gpui::Entity<FeaturesScreen>, cx: &mut VisualTestContext) -> Option<gpui::Entity<StartPanel>> {
    screen.read_with(cx, |screen, _| screen.start.clone())
}

#[gpui::test]
fn a_project_with_no_feature_draws_the_start_panel_with_its_modules_and_docs(cx: &mut TestAppContext) {
    let (screen, cx) = open_with(cx, mock::bare());
    assert!(panel(&screen, cx).is_some());
    assert!(drawn(cx, Role::Heading, start::HEADING));
    assert!(drawn(cx, Role::Heading, "Modules in the build files") && drawn(cx, Role::Heading, "Docs in the repo") && drawn(cx, Role::Heading, "Or feed the librarian"));
    assert!(drawn(cx, Role::Label, "ledger-core") && drawn(cx, Role::Label, "worker"), "each module a chip");
    assert!(drawn(cx, Role::Label, "README.md") && drawn(cx, Role::Label, "docs/"), "each doc a chip");
    assert!(drawn(cx, Role::Button, start::DRAFT));
    assert!(drawn(cx, Role::Button, start::READ), "Read them in the docs' band");
}

/// The panel's box, and what its mock librarian was sent.
fn sent(start: &gpui::Entity<StartPanel>, cx: &mut VisualTestContext) -> Vec<ipc::AskLibrarian> {
    let ask = start.read_with(cx, |start, _| start.ask.clone()).expect("the box");
    ask.update(cx, |ask, _| ask.mock().expect("the mock librarian").sent.clone())
}

#[gpui::test]
fn the_start_panel_draws_the_librarians_box_in_its_slot(cx: &mut TestAppContext) {
    let (screen, cx) = open_with(cx, mock::bare());
    let start = panel(&screen, cx).expect("the panel");
    assert!(start.read_with(cx, |start, _| start.ask.is_some()), "the box is made on the first draw");
    assert!(drawn(cx, Role::TextInput, "Ask the librarian"), "the box is drawn in the panel");
    assert_eq!(start.read_with(cx, |start, cx| start.ask.as_ref().map(|ask| ask.read(cx).asking_from().to_string())), Some("features_empty".to_string()), "with the empty page's context");
}

#[gpui::test]
fn read_them_sends_the_docs_with_the_empty_pages_context_and_its_answer_offers_undo(cx: &mut TestAppContext) {
    let (screen, cx) = open_with(cx, mock::bare());
    let start = panel(&screen, cx).expect("the panel");
    cx.update(|window, cx| start.update(cx, |start, cx| start.read_docs(window, cx)));
    cx.run_until_parked();
    let sent = sent(&start, cx);
    assert_eq!(sent.len(), 1, "one ask");
    assert_eq!((sent[0].context.as_str(), sent[0].words.as_str()), ("features_empty", start::READ_ASK));
    let listed = start.read_with(cx, |start, _| match &start.start {
        super::page::Load::Read(start) => start.docs.clone(),
        _ => Vec::new(),
    });
    assert!(!listed.is_empty());
    assert_eq!(sent[0].files, listed, "the docs the panel lists");
    assert!(drawn(cx, Role::Status, "Drafted Posting and Reports as Pending"), "the box's card says what it drafted");
    assert!(drawn(cx, Role::Button, "Undo"));
}

#[gpui::test]
fn tab_goes_from_draft_as_pending_to_read_them_to_the_box_and_back_to_the_list(cx: &mut TestAppContext) {
    let (screen, cx) = open_with(cx, mock::bare());
    cx.simulate_keystrokes("tab");
    let start = panel(&screen, cx).expect("the panel");
    let (body, read, ask) = start.read_with(cx, |start, _| (start.body.clone(), start.read_focus.clone(), start.ask.clone().expect("the box")));
    let input = ask.read_with(cx, |ask, _| ask.input.clone());
    let in_box = |cx: &mut VisualTestContext| cx.update(|window, cx| input.read(cx).focus_handle(cx).is_focused(window));
    assert!(cx.update(|window, _| body.is_focused(window)), "Tab takes the keys to the panel");
    cx.simulate_keystrokes("tab");
    assert!(cx.update(|window, _| read.is_focused(window)), "then to Read them");
    cx.simulate_keystrokes("enter");
    cx.run_until_parked();
    assert_eq!(sent(&start, cx).len(), 1, "Enter on Read them asks once");
    assert!(cx.update(|window, _| read.is_focused(window)), "and the keys stay on it");
    cx.simulate_keystrokes("tab");
    assert!(in_box(cx), "then into the box");
    cx.simulate_keystrokes("shift-tab");
    assert!(cx.update(|window, _| read.is_focused(window)), "⇧Tab goes back to Read them");
    cx.simulate_keystrokes("tab");
    // The card from Read them stands under the box, so Tab is on its Undo first.
    cx.simulate_keystrokes("tab");
    let undo = ask.read_with(cx, |ask, _| ask.undo_focus.clone());
    assert!(cx.update(|window, _| undo.is_focused(window)), "Tab from the box is on the card's Undo");
    cx.simulate_keystrokes("tab");
    assert!(cx.update(|window, cx| screen.read(cx).list_focus.is_focused(window)), "and past it the keys go back to the list");
}

#[gpui::test]
fn a_project_with_a_confirmed_feature_draws_no_start_panel(cx: &mut TestAppContext) {
    let (screen, cx) = open_with(cx, mock::library());
    assert!(panel(&screen, cx).is_none());
    assert!(!drawn(cx, Role::Heading, start::HEADING));
}

#[gpui::test]
fn tab_reaches_draft_as_pending_and_enter_drafts_each_module_as_a_pending_row(cx: &mut TestAppContext) {
    let (screen, cx) = open_with(cx, mock::bare());
    cx.simulate_keystrokes("tab");
    let start = panel(&screen, cx).expect("the panel");
    assert!(cx.update(|window, cx| start.read(cx).body.is_focused(window)), "Tab takes the keys to the panel");
    cx.simulate_keystrokes("enter");
    cx.run_until_parked();
    let pending: Vec<String> = screen.read_with(cx, |screen, _| match &screen.library {
        Load::Read(library) => library.areas.iter().flat_map(|area| &area.features).filter(|row| row.state == ipc::FeatureState::Pending).map(|row| row.slug.clone()).collect(),
        _ => Vec::new(),
    });
    assert_eq!(pending, ["api", "billing", "ledger-core", "web", "worker"], "each module its own pending row");
    assert!(panel(&screen, cx).is_some(), "the panel stays while every feature is pending");
    // The label book keeps the frames drawn before the press, so what is left to draft is read off the panel.
    assert!(start.read_with(cx, |start, _| !start.can_draft()), "nothing is left to draft");
    assert!(drawn(cx, Role::Image, start::DRAFTED), "the glyph says so");
    cx.simulate_keystrokes("escape");
    assert!(cx.update(|window, cx| screen.read(cx).list_focus.is_focused(window)), "Esc gives the keys back to the list");
}

#[gpui::test]
fn a_refused_draft_says_why_in_the_modules_band_and_drafts_nothing(cx: &mut TestAppContext) {
    let (screen, cx) = open_with(cx, mock::bare());
    let said = "This project holds features of its own, so its modules are not drafted.";
    let start = panel(&screen, cx).expect("the panel");
    start.update(cx, |start, cx| {
        start.mock_refusal = Some(said.into());
        start.draft(cx);
    });
    cx.run_until_parked();
    assert_eq!(start.read_with(cx, |start, _| start.drafting.clone()), Drafting::Said(said.into()));
    assert!(drawn(cx, Role::Status, said));
    assert!(screen.read_with(cx, |screen, _| matches!(&screen.library, Load::Read(library) if library.areas.is_empty())));
}

#[test]
fn a_draft_marks_what_it_kept_and_passed_over_so_nothing_is_left_to_draft() {
    let mut start = mock::start(mock::BARE);
    assert!(start::to_draft(&start));
    start::after_draft(&mut start, &ipc::ModulesDrafted { project: mock::BARE.into(), drafted: vec!["api".into(), "billing".into(), "ledger-core".into()], held: vec!["web".into(), "worker".into()] });
    assert!(!start::to_draft(&start));
}

#[test]
fn the_panel_stands_while_every_feature_is_pending() {
    assert!(super::model::starts(&mock::bare()));
    assert!(super::model::starts(&mock::drafted(&mock::start(mock::BARE))));
    assert!(!super::model::starts(&mock::library()));
}
