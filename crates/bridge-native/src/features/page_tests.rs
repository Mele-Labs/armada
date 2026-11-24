use chrono::{DateTime, TimeZone, Utc};
use gpui::{Role, TestAppContext, VisualTestContext};
use ipc::{AskState, FeatureLinkKind};

use super::page::{FeaturePageView, Load, Region};
use super::page_model::{self, Body, Chip, Section, Standing};
use super::{looked, mock};
use crate::look::ActiveLook as _;

const SLUG: &str = "start-a-job";

fn ready(cx: &mut TestAppContext) {
    cx.update(|cx| {
        gpui_component::init(cx);
        crate::look::init(cx);
        crate::keymap::install(&crate::keymap::Rebinding::default(), cx);
        crate::titlebar::bind(cx);
        looked::forget_all(cx);
    });
}

fn open(cx: &mut TestAppContext) -> (gpui::Entity<FeaturePageView>, &mut VisualTestContext) {
    ready(cx);
    let (page, cx) = cx.add_window_view(|window, cx| FeaturePageView::mock(SLUG.into(), window, cx));
    cx.update(|window, cx| page.update(cx, |page, cx| page.focus_rail(window, cx)));
    cx.run_until_parked();
    (page, cx)
}

fn drawn(cx: &mut VisualTestContext, role: Role, name: &str) -> bool {
    cx.update(|window, cx| cx.default_global::<crate::labels::LabelBook>().find(window.window_handle().window_id(), role, name).is_some())
}

fn at(day: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 11, day, 12, 0, 0).unwrap()
}

fn entries_of(page: &ipc::FeaturePage) -> Vec<(Section, String, bool)> {
    page_model::entries(page, Some(&mock::asks(SLUG).asks)).into_iter().map(|entry| (entry.section, entry.summary, entry.stale)).collect()
}

fn summary(page: &ipc::FeaturePage, section: Section) -> String {
    entries_of(page).into_iter().find(|entry| entry.0 == section).map(|entry| entry.1).unwrap_or_default()
}

fn titles(body: Body) -> Vec<String> {
    match body {
        Body::Rows(rows) => rows.into_iter().map(|row| row.title).collect(),
        _ => Vec::new(),
    }
}

// ---- the model -------------------------------------------------------------------------------

#[test]
fn the_rail_names_the_sections_in_the_catalogs_order_each_with_a_line_from_the_data() {
    let page = mock::page(SLUG);
    let entries = entries_of(&page);
    let order: Vec<&str> = entries.iter().map(|entry| entry.0.title()).collect();
    assert_eq!(order, ["Who worked on it", "Code", "How to test it", "Designs", "Bugs and issues", "Change requests", "Ideas", "How it runs", "How to observe it", "Tech debt", "Decisions", "Docs", "Tickets"]);
    let said = |section| summary(&page, section);
    assert_eq!(said(Section::Who), "Now: Add the model picker to the start card");
    assert_eq!(said(Section::Code), "card.rs, panel.rs", "the first two files, by name");
    assert_eq!(said(Section::Test), "armada check test start_card", "the first line of what was written");
    assert_eq!(said(Section::Designs), "Walk of the start card", "the newest of the sketches, documents, walks and drafts");
    assert_eq!(said(Section::Bugs), "Open: Native draws no sketch in a page");
    assert_eq!(said(Section::Changes), "Being built: True voting with sketches");
    assert_eq!(said(Section::Ideas), "Start from the phone", "the newest idea");
    assert!(said(Section::Runs).starts_with("Fleet reads the request"));
    assert!(said(Section::Debt).starts_with("The picker keeps the last model"));
    assert_eq!(said(Section::Docs), "The start card", "the newest title");
    assert_eq!(said(Section::Tickets), "armada-a3f2 The card loses focus after a retry", "the newest title");
    assert_eq!(said(Section::Decisions), "Where does the model picker go on the start card?", "the newest question");
}

#[test]
fn a_stale_section_says_since_which_merge_and_is_drawn_in_the_awaiting_review_hue() {
    let page = mock::page(SLUG);
    let observe = entries_of(&page).into_iter().find(|entry| entry.0 == Section::Observe).unwrap();
    assert_eq!((observe.1.as_str(), observe.2), ("Stale since #3692", true), "the stale mark stands over the first line");
    assert!(entries_of(&page).iter().filter(|entry| entry.2).count() == 1);
}

#[test]
fn a_line_falls_back_the_way_the_catalog_says_and_an_empty_section_stays_empty() {
    let mut page = mock::page(SLUG);
    page.feature.jobs_now.clear();
    assert_eq!(summary(&page, Section::Who), format!("Last: Add the model picker to the start card, {}", page_model::day(at(14))), "no Job on it: the newest Job and its day");
    page.files.clear();
    assert_eq!(summary(&page, Section::Code), "card.rs, mod.rs", "entry points stand in for files");
    page.sections[1].text = "  \n".into();
    assert_eq!(summary(&page, Section::Runs), format!("Written {}", page_model::day(at(13))), "a section with no line says when it was written");
    let bare = ipc::FeaturePage { sections: Vec::new(), links: Vec::new(), files: Vec::new(), entry_points: Vec::new(), paths: Vec::new(), docs: Vec::new(), tickets: Vec::new(), decisions: Vec::new(), ..page };
    for entry in page_model::entries(&bare, None) {
        assert_eq!(entry.summary, "", "{} has nothing to say", entry.section.title());
    }
}

#[test]
fn a_change_request_that_is_not_being_built_says_open_and_a_dropped_idea_is_not_the_line() {
    let mut asks = mock::asks(SLUG).asks;
    asks.retain(|ask| ask.ask_id != "ask-5");
    asks.iter_mut().find(|ask| ask.ask_id == "ask-6").unwrap().state = AskState::Handed;
    let page = mock::page(SLUG);
    let entries = page_model::entries(&page, Some(&asks));
    assert_eq!(entries[5].summary, "Open: Keep the request when the card closes");
    asks.iter_mut().find(|ask| ask.ask_id == "ask-7").unwrap().state = AskState::Dropped;
    assert_eq!(page_model::entries(&page, Some(&asks))[6].summary, "Suggest a repository from the request");
    assert_eq!(page_model::entries(&page, None)[4].summary, "", "the asks are not read yet");
}

#[test]
fn the_chips_map_the_states_open_is_building_and_handed_and_dropped_is_under_all_alone() {
    assert_eq!(Standing::of(AskState::Building), Standing::Open);
    assert_eq!(Standing::of(AskState::Handed), Standing::Open);
    assert_eq!(Standing::of(AskState::Backlog), Standing::Backlog);
    assert_eq!(Standing::of(AskState::Built), Standing::Fixed);
    assert_eq!(Standing::of(AskState::Dropped), Standing::Dropped);
    let kept = |chip: Chip, standing| chip.keeps(standing, at(14), None);
    for standing in [Standing::Open, Standing::Backlog, Standing::Fixed, Standing::Dropped] {
        assert!(kept(Chip::All, standing));
        assert_eq!(kept(Chip::Open, standing), standing == Standing::Open);
        assert_eq!(kept(Chip::Backlog, standing), standing == Standing::Backlog);
        assert_eq!(kept(Chip::Fixed, standing), standing == Standing::Fixed);
    }
}

#[test]
fn since_my_last_look_keeps_what_moved_after_it_and_every_ask_where_there_was_no_look() {
    assert!(Chip::Since.keeps(Standing::Fixed, at(1), None), "a feature never looked at shows every ask");
    assert!(Chip::Since.keeps(Standing::Fixed, at(14), Some(at(13))));
    assert!(!Chip::Since.keeps(Standing::Fixed, at(12), Some(at(13))));
    assert!(!Chip::Since.keeps(Standing::Fixed, at(13), Some(at(13))), "what moved at the look is seen");
}

#[test]
fn the_asks_of_a_kind_come_newest_first_with_the_bugs_a_job_filed_and_each_chip_keeps_its_own() {
    let page = mock::page(SLUG);
    let asks = mock::asks(SLUG).asks;
    let body = |chip, looked| titles(page_model::body(Section::Bugs, &page, Some(&asks), chip, looked, &page_model::Titles::new()));
    assert_eq!(body(Chip::All, None), ["Native draws no sketch in a page", "The card loses focus after a retry", "They show as i never voted", "The model name is cut off in the card", "Retry started a second Job", "The title is proposed twice on a slow start", "The workflow picker forgets the last repository"]);
    assert_eq!(body(Chip::Open, None), ["Native draws no sketch in a page", "The card loses focus after a retry"]);
    assert_eq!(body(Chip::Backlog, None), ["The model name is cut off in the card"]);
    assert_eq!(body(Chip::Fixed, None), ["They show as i never voted", "The title is proposed twice on a slow start", "The workflow picker forgets the last repository"]);
    assert_eq!(body(Chip::Since, Some(at(13))), ["Native draws no sketch in a page", "The card loses focus after a retry", "They show as i never voted"]);
    assert_eq!(titles(page_model::body(Section::Changes, &page, Some(&asks), Chip::All, None, &page_model::Titles::new())), ["True voting with sketches", "Keep the request when the card closes"], "a fix is a change request");
    assert_eq!(titles(page_model::body(Section::Ideas, &page, Some(&asks), Chip::All, None, &page_model::Titles::new())), ["Start from the phone", "Suggest a repository from the request"]);
}

#[test]
fn a_job_on_the_feature_stands_first_with_its_chip_and_the_rest_come_newest_first() {
    let page = mock::page(SLUG);
    let Body::Rows(rows) = page_model::body(Section::Who, &page, None, Chip::All, None, &page_model::Titles::new()) else { panic!("rows") };
    let seen: Vec<(&str, bool)> = rows.iter().map(|row| (row.title.as_str(), row.on)).collect();
    assert_eq!(seen, [("Add the model picker to the start card", true), ("Propose the title from the first line", false), ("Pick the workflow from the repository", false)]);
    assert!(rows[0].meta.starts_with("Running"), "the link's state and day: {}", rows[0].meta);
}

#[test]
fn designs_are_the_sketches_documents_walks_and_drafts_newest_first_and_the_top_strip_names_the_newest_merge() {
    let mut page = mock::page(SLUG);
    assert_eq!(titles(page_model::body(Section::Designs, &page, None, Chip::All, None, &page_model::Titles::new())), ["Walk of the start card", "Start card with the model picker", "How a Job starts"]);
    assert_eq!(page_model::current_as_of(&page), Some(format!("Current as of #3720, {}", page_model::day(at(15)))));
    page.links.retain(|link| link.kind != FeatureLinkKind::Merge);
    assert_eq!(page_model::current_as_of(&page), None, "no merge, nothing said");
}

#[test]
fn a_written_section_is_commands_in_monospace_and_paragraphs_otherwise() {
    let page = mock::page(SLUG);
    let Body::Written(Some(written)) = page_model::body(Section::Test, &page, None, Chip::All, None, &page_model::Titles::new()) else { panic!("written") };
    assert_eq!(written.blocks.len(), 2);
    assert!(written.blocks[0].command && written.blocks[0].text.contains('\n'), "the two commands are one block");
    assert!(!written.blocks[1].command);
    assert!(written.by.starts_with("Written by Add the model picker to the start card, "));
    assert_eq!(written.stale, None);
    let Body::Written(Some(observe)) = page_model::body(Section::Observe, &page, None, Chip::All, None, &page_model::Titles::new()) else { panic!("written") };
    assert_eq!(observe.stale.as_deref(), Some("Stale since #3692"));
    let Body::Written(Some(debt)) = page_model::body(Section::Debt, &page, None, Chip::All, None, &page_model::Titles::new()) else { panic!("written") };
    assert!(debt.blocks.iter().all(|block| !block.command), "tech debt is read as text");
    let mut bare = page;
    bare.sections.clear();
    assert_eq!(page_model::body(Section::Test, &bare, None, Chip::All, None, &page_model::Titles::new()), Body::Written(None), "where none was written the pane is empty");
}

fn reads_of(section: Section, page: &ipc::FeaturePage) -> Vec<page_model::ReadRow> {
    match page_model::body(section, page, None, Chip::All, None, &mock::titles()) {
        Body::Reads(rows) => rows,
        _ => panic!("reads"),
    }
}

#[test]
fn a_doc_is_a_file_or_a_globe_by_its_target_and_a_ticket_is_a_ticket_newest_first_with_the_job_named_by_its_title() {
    let page = mock::page(SLUG);
    let docs = reads_of(Section::Docs, &page);
    assert_eq!(docs.iter().map(|row| row.title.as_str()).collect::<Vec<_>>(), ["The start card", "Starting a Job spec", "Start card board"]);
    assert_eq!(docs.iter().map(|row| row.glyph).collect::<Vec<_>>(), ["file-text", "globe", "globe"], "a path is a file, an address a globe");
    assert_eq!(docs[0].last, format!("Add the model picker to the start card, {}", page_model::day(at(14))), "the Job's title and the day, never its id");
    let tickets = reads_of(Section::Tickets, &page);
    assert!(tickets.iter().all(|row| row.glyph == "ticket" && row.drifted.is_empty()), "a ticket has no drift line");
    assert_eq!(tickets[1].title, "#3940 Send the request from the keys");
    let mut page = page;
    page.docs[0].job = "job-gone".into();
    assert_eq!(reads_of(Section::Docs, &page)[0].last, page_model::day(at(14)), "a Job Fleet holds no title for is not named, and never by its id");
}

#[test]
fn a_gone_doc_keeps_its_gist_a_jobs_own_gist_is_marked_and_drift_lists_the_names() {
    let page = mock::page(SLUG);
    let docs = reads_of(Section::Docs, &page);
    assert!(docs[0].by_job && !docs[1].by_job, "only the gist the Job wrote is marked");
    assert_eq!(docs[0].drifted, ["StartProposal", "pick_model"]);
    assert!(docs[1].drifted.is_empty());
    assert!(docs[2].gone && !docs[0].gone && !docs[1].gone);
    assert_eq!(docs[2].gist, "The card with the model picker on one line.", "a gone address keeps what it said");
    let mut page = page;
    page.docs[0].gist.clear();
    assert!(!reads_of(Section::Docs, &page)[0].by_job, "no gist, nothing to mark as the Job's");
}

#[test]
fn a_decision_is_its_question_its_choice_and_its_note_where_there_is_one() {
    let page = mock::page(SLUG);
    let Body::Decisions(rows) = page_model::body(Section::Decisions, &page, None, Chip::All, None, &mock::titles()) else { panic!("decisions") };
    assert_eq!(rows.iter().map(|row| (row.question.as_str(), row.choice.as_str(), row.note.as_deref())).collect::<Vec<_>>(), [
        ("Where does the model picker go on the start card?", "Beside the workflow", None),
        ("Sketch: Start card with the model picker", "Approved", Some("Keep the picker one line.")),
    ]);
    assert_eq!(rows[0].last, format!("Add the model picker to the start card, {}", page_model::day(at(14))));
    let mut page = page;
    page.decisions[1].note = Some("  ".into());
    let Body::Decisions(rows) = page_model::body(Section::Decisions, &page, None, Chip::All, None, &mock::titles()) else { panic!("decisions") };
    assert_eq!(rows[1].note, None, "a blank note is no note");
}

#[test]
fn an_empty_docs_tickets_or_decisions_card_has_no_rows_and_a_line_of_its_own() {
    let mut page = mock::page(SLUG);
    page.docs.clear();
    page.tickets.clear();
    page.decisions.clear();
    for section in [Section::Docs, Section::Tickets] {
        assert_eq!(reads_of(section, &page), Vec::new());
        assert_eq!(summary(&page, section), "");
    }
    assert_eq!(page_model::body(Section::Decisions, &page, None, Chip::All, None, &mock::titles()), Body::Decisions(Vec::new()));
    assert_eq!(summary(&page, Section::Decisions), "");
    for key in ["docs", "tickets", "decisions"] {
        let said = super::page_view::said_empty(key);
        assert!(!said.contains('\u{2014}') && !said.contains(" - "), "{key} has no dash");
        assert!(said.ends_with('.') && said.matches('.').count() == 1, "{key} is one fact line");
    }
}

#[test]
fn watch_and_poor_draw_a_chip_with_the_reasons_and_good_draws_none() {
    let mut page = mock::page(SLUG);
    let watch = page_model::health(&page).expect("watch has a chip");
    assert_eq!((watch.word, watch.glyph, watch.mark), ("Watch", "triangle-alert", ipc::HealthMark::Watch));
    assert_eq!(watch.line, "Observe section stale since #3692 \u{b7} Open bug: the card loses focus after a retry", "the reasons joined by a middle dot");
    assert_eq!(watch.tip, watch.line, "the tooltip says the same");
    page.feature.health.mark = ipc::HealthMark::Poor;
    let poor = page_model::health(&page).expect("poor has a chip");
    assert_eq!((poor.word, poor.glyph), ("Poor", "circle-alert"));
    page.feature.health.reasons.clear();
    let bare = page_model::health(&page).expect("a mark with no reasons still has its chip");
    assert_eq!(bare.line, "");
    assert_eq!(bare.tip, "A check failed, or flaky tests or open bugs piled up", "the tooltip says what the mark means");
    page.feature.health.mark = ipc::HealthMark::Good;
    assert_eq!(page_model::health(&page), None, "good draws nothing");
}

#[test]
fn the_retire_row_is_there_only_where_the_librarian_proposed_it() {
    let mut page = mock::page(SLUG);
    assert_eq!(page_model::retire_evidence(&page), Some("No file of it touched and no Job on it since 12 Jun."));
    page.feature.retire = None;
    assert_eq!(page_model::retire_evidence(&page), None);
}

// ---- the page --------------------------------------------------------------------------------

#[gpui::test]
fn he_last_looked_at_a_feature_when_he_leaves_its_page_and_never_when_he_opens_it(cx: &mut TestAppContext) {
    let (page, cx) = open(cx);
    assert_eq!(cx.update(|_, cx| looked::before(mock::PROJECT, SLUG, cx)), None, "opening stamps nothing");
    assert_eq!(page.read_with(cx, |page, _| page.looked), None);
    page.update(cx, |page, cx| page.select(Section::Bugs, cx));
    page.update(cx, |page, cx| page.apply(Chip::Since, cx));
    assert_eq!(page.read_with(cx, |page, _| page.row_count()), 6, "a feature never looked at shows every ask under the chip: five rows and the fold line");
    page.update(cx, |page, cx| page.leave(cx));
    assert!(cx.update(|_, cx| looked::before(mock::PROJECT, SLUG, cx)).is_some(), "leaving stamps it, the page having had Tab into its sections");
    assert_eq!(page.read_with(cx, |page, _| page.looked), None, "and the page he left still compares with the look before it");
}

#[gpui::test]
fn a_page_opened_after_a_look_compares_with_that_look_and_another_feature_is_not_touched(cx: &mut TestAppContext) {
    ready(cx);
    cx.update(|cx| looked::stamp(mock::PROJECT, SLUG, at(13), cx));
    let (page, cx) = cx.add_window_view(|window, cx| FeaturePageView::mock(SLUG.into(), window, cx));
    assert_eq!(page.read_with(cx, |page, _| page.looked), Some(at(13)));
    page.update(cx, |page, cx| {
        page.select(Section::Bugs, cx);
        page.apply(Chip::Since, cx);
    });
    assert_eq!(page.read_with(cx, |page, _| page.row_count()), 3, "what moved after the look");
    assert_eq!(cx.update(|_, cx| looked::before(mock::PROJECT, "reviews", cx)), None);
    let (other, cx) = cx.add_window_view(|window, cx| FeaturePageView::mock("reviews".into(), window, cx));
    other.update(cx, |page, cx| {
        page.select(Section::Bugs, cx);
        page.apply(Chip::Since, cx);
    });
    assert_eq!(other.read_with(cx, |page, _| page.row_count()), 6, "no look at that one, so every ask: five rows and the fold line");
}

#[gpui::test]
fn j_and_k_move_the_card_the_keys_are_on_and_tab_goes_between_the_cards_the_chips_the_rows_and_the_start_button(cx: &mut TestAppContext) {
    let (page, cx) = open(cx);
    let section = |page: &gpui::Entity<FeaturePageView>, cx: &mut VisualTestContext| page.read_with(cx, |page, _| page.section());
    let region = |page: &gpui::Entity<FeaturePageView>, cx: &mut VisualTestContext| cx.update(|window, cx| page.read(cx).region_in(window));
    assert_eq!(section(&page, cx), Section::Who);
    cx.simulate_keystrokes("j");
    assert_eq!(section(&page, cx), Section::Bugs, "the next card down the first column");
    cx.simulate_keystrokes("down down down");
    assert_eq!(section(&page, cx), Section::Decisions, "the asks are three lanes of one card, and Decisions stands under them");
    cx.simulate_keystrokes("down");
    assert_eq!(section(&page, cx), Section::Test, "the next column follows");
    cx.simulate_keystrokes("up up");
    assert_eq!(section(&page, cx), Section::Ideas);
    cx.simulate_keystrokes("up up up up up up up up up up up up");
    assert_eq!(section(&page, cx), Section::Who, "it stops at the first");
    cx.simulate_keystrokes("j");
    assert_eq!(section(&page, cx), Section::Bugs);
    // Tab: the cards, then the chips, then the rows, then the button, round to the cards.
    assert_eq!(region(&page, cx), Region::Rail);
    cx.simulate_keystrokes("tab");
    assert_eq!(region(&page, cx), Region::Chips);
    cx.simulate_keystrokes("right right");
    assert_eq!(page.read_with(cx, |page, _| (page.chip(), page.chip_at)), (Chip::All, Chip::Backlog), "the chips move and nothing applies yet");
    cx.simulate_keystrokes("enter");
    assert_eq!(page.read_with(cx, |page, _| page.chip()), Chip::Backlog, "Enter applies the chip");
    assert_eq!(page.read_with(cx, |page, _| page.row_count()), 1);
    cx.simulate_keystrokes("left left left");
    cx.simulate_keystrokes("enter");
    assert_eq!(page.read_with(cx, |page, _| page.chip()), Chip::All);
    cx.simulate_keystrokes("tab");
    assert_eq!(region(&page, cx), Region::Rows);
    assert_eq!(page.read_with(cx, |page, _| page.row_at), Some(0));
    cx.simulate_keystrokes("j j");
    assert_eq!(page.read_with(cx, |page, _| page.row_at), Some(2));
    cx.simulate_keystrokes("tab");
    assert_eq!(region(&page, cx), Region::Retire, "the two buttons that answer the proposal to retire it");
    cx.simulate_keystrokes("tab");
    assert_eq!(region(&page, cx), Region::Keep);
    cx.simulate_keystrokes("tab");
    assert_eq!(region(&page, cx), Region::Start, "the button that starts a Job on it");
    cx.simulate_keystrokes("tab");
    assert_eq!(region(&page, cx), Region::Rail, "it wraps");
    cx.simulate_keystrokes("shift-tab");
    assert_eq!(region(&page, cx), Region::Start);
    cx.simulate_keystrokes("shift-tab shift-tab");
    assert_eq!(region(&page, cx), Region::Retire);
    cx.simulate_keystrokes("shift-tab");
    assert_eq!(region(&page, cx), Region::Rows);
    cx.simulate_keystrokes("shift-tab");
    assert_eq!(region(&page, cx), Region::Chips);
    cx.simulate_keystrokes("shift-tab");
    assert_eq!(region(&page, cx), Region::Rail);
    // A written card has no chips and nothing to pick: Tab goes to its command well, then the button.
    cx.simulate_keystrokes("j j j j");
    assert_eq!(section(&page, cx), Section::Test);
    cx.simulate_keystrokes("tab");
    assert_eq!(region(&page, cx), Region::Well, "the written card has a command to copy");
    cx.simulate_keystrokes("tab tab tab");
    assert_eq!(region(&page, cx), Region::Start);
    cx.simulate_keystrokes("tab");
    assert_eq!(region(&page, cx), Region::Rail);
    // A written card with no command has no well.
    cx.simulate_keystrokes("j");
    assert_eq!(section(&page, cx), Section::Runs);
    cx.simulate_keystrokes("tab tab tab");
    assert_eq!(region(&page, cx), Region::Start, "no command, no well");
}

#[gpui::test]
fn enter_on_a_command_well_copies_the_command(cx: &mut TestAppContext) {
    let (page, cx) = open(cx);
    page.update(cx, |page, cx| page.select(Section::Test, cx));
    cx.update(|window, cx| page.update(cx, |page, cx| page.focus_region(Region::Well, window, cx)));
    cx.run_until_parked();
    assert_eq!(cx.update(|window, cx| page.read(cx).region_in(window)), Region::Well);
    cx.update(|_, cx| cx.write_to_clipboard(gpui::ClipboardItem::new_string("nothing".into())));
    cx.simulate_keystrokes("enter");
    let copied = cx.update(|_, cx| cx.read_from_clipboard().and_then(|item| item.text()));
    assert_eq!(copied.as_deref(), Some("armada check test start_card\narmada check crates/bridge-native:test dispatch"));
}

#[gpui::test]
fn enter_on_the_start_button_opens_the_dispatch_panel(cx: &mut TestAppContext) {
    ready(cx);
    let asked: std::rc::Rc<std::cell::RefCell<Vec<crate::titlebar::Asked>>> = Default::default();
    let heard = asked.clone();
    cx.update(|cx| cx.set_global(crate::titlebar::AskedSink(std::rc::Rc::new(move |one: &crate::titlebar::Asked, _: &mut gpui::App| heard.borrow_mut().push(one.clone())))));
    let (page, cx) = cx.add_window_view(|window, cx| FeaturePageView::mock(SLUG.into(), window, cx));
    cx.update(|window, cx| page.update(cx, |page, cx| page.focus_rail(window, cx)));
    cx.run_until_parked();
    cx.simulate_keystrokes("shift-tab");
    assert_eq!(cx.update(|window, cx| page.read(cx).region_in(window)), Region::Start);
    cx.simulate_keystrokes("enter");
    assert!(matches!(asked.borrow().as_slice(), [crate::titlebar::Asked::Dispatch]));
}

#[gpui::test]
fn a_feature_with_only_code_draws_all_twelve_cards_and_eleven_empty_lines(cx: &mut TestAppContext) {
    let (page, cx) = open(cx);
    page.update(cx, |page, cx| {
        let mut read = mock::page(SLUG);
        read.sections.clear();
        read.links.clear();
        read.feature.jobs_now.clear();
        read.feature.retire = None;
        read.docs.clear();
        read.tickets.clear();
        read.decisions.clear();
        page.page = Load::Read(read);
        page.asks = Load::Read(ipc::FeatureAsks { asks: Vec::new(), ..mock::asks(SLUG) });
        page.rebuild(cx);
    });
    cx.run_until_parked();
    cx.update(|window, cx| window.draw(cx).clear(cx));
    for card in ["On it now", "Who worked on it", "Asks", "Decisions", "How to test it", "How it runs", "Designs", "Docs", "Code", "Tickets", "How to observe it", "Tech debt"] {
        assert!(drawn(cx, Role::Heading, card), "{card} has its title");
        assert!(drawn(cx, Role::Region, card), "{card} is a region");
    }
    let empty = ["on", "who", "asks", "decisions", "test", "runs", "designs", "docs", "tickets", "observe", "debt"];
    for key in empty {
        assert!(drawn(cx, Role::Label, super::page_view::said_empty(key)), "{key} says why it is empty");
    }
    assert!(!drawn(cx, Role::Label, super::page_view::said_empty("code")), "Code has files and says nothing");
    assert!(!page.read_with(cx, |page, _| page.has_asks), "an empty Asks card has no chips to draw");
    page.update(cx, |page, cx| page.select(Section::Who, cx));
    cx.update(|window, cx| page.update(cx, |page, cx| page.focus_rail(window, cx)));
    cx.simulate_keystrokes("j");
    assert_eq!(page.read_with(cx, |page, _| page.section()), Section::Code, "the keys go to the one card with something in it");
    cx.simulate_keystrokes("tab");
    assert_eq!(cx.update(|window, cx| page.read(cx).region_in(window)), Region::Start, "no rows and no well: Tab goes to the button");
}

#[gpui::test]
fn the_new_keys_are_registry_acts_in_the_features_scope(cx: &mut TestAppContext) {
    let (_page, _cx) = open(cx);
    let bound = crate::keymap::bound(&crate::keymap::Rebinding::default());
    for id in ["features_pane", "features_chip"] {
        assert!(bound.iter().any(|key| key.act.id == id && key.context == Some("features")), "{id} is bound under features");
    }
}

#[gpui::test]
fn every_part_of_the_page_is_found_by_its_role_and_name(cx: &mut TestAppContext) {
    let (page, cx) = open(cx);
    page.update(cx, |page, cx| page.select(Section::Bugs, cx));
    cx.run_until_parked();
    assert!(drawn(cx, Role::Heading, "Start a Job"));
    assert!(drawn(cx, Role::Button, "Start a Job on it"));
    assert!(drawn(cx, Role::Label, "Area: running a job"), "the area chip is named");
    for card in ["On it now", "Who worked on it", "Asks", "Decisions", "Designs", "Docs", "Code", "Tickets", "How to test it", "How it runs", "How to observe it", "Tech debt"] {
        assert!(drawn(cx, Role::Region, card), "{card} is a region");
        assert!(drawn(cx, Role::Heading, card), "{card} has its title");
    }
    for lane in ["Bug", "Change request", "Idea"] {
        assert!(drawn(cx, Role::Heading, lane), "{lane} heads its asks");
    }
    for chip in Chip::ALL {
        assert!(drawn(cx, Role::Button, chip.label()), "{} is a button", chip.label());
    }
    assert!(drawn(cx, Role::ListBox, "Bugs and issues"));
    assert!(drawn(cx, Role::ListBoxOption, "Native draws no sketch in a page"));
    assert!(drawn(cx, Role::Image, "Handed"), "a row's state has a name");
    assert!(drawn(cx, Role::Image, "Jobs that wrote crates/fleet/src/start/card.rs: 6"));
    assert!(drawn(cx, Role::Image, "Stale since #3692"), "a stale section says since which merge");
    assert!(drawn(cx, Role::Button, "Copy"), "a command can be copied");
}

#[gpui::test]
fn the_page_draws_bones_while_it_is_read_and_its_sections_once_it_answers(cx: &mut TestAppContext) {
    let (page, cx) = open(cx);
    page.update(cx, |page, cx| {
        page.page = Load::Reading;
        page.asks = Load::Reading;
        page.rebuild(cx);
    });
    cx.run_until_parked();
    assert!(drawn(cx, Role::Image, "Loading"), "bones stand where the sections will be");
    assert!(page.read_with(cx, |page, _| page.entries.is_empty()), "no section is held while the read is pending");
    page.update(cx, |page, cx| {
        page.page = Load::Read(mock::page(SLUG));
        page.asks = Load::Read(mock::asks(SLUG));
        page.rebuild(cx);
    });
    cx.run_until_parked();
    assert!(drawn(cx, Role::Region, "Code"));
    assert_eq!(page.read_with(cx, |page, _| page.entries.len()), Section::ALL.len());
}

/// The corner radius of this element as last drawn (the page has no window root to clear the book each frame, so the last one stands).
fn radius(cx: &mut VisualTestContext, role: Role, name: &str) -> Option<gpui::Pixels> {
    cx.update(|window, cx| cx.default_global::<crate::labels::LabelBook>().drawn(window.window_handle().window_id()).iter().rev().find(|drawn| drawn.role == role && drawn.name == name).and_then(|drawn| drawn.radius))
}

#[gpui::test]
fn what_is_painted_flush_against_a_rounded_parent_rounds_its_own_corners_to_the_inner_radius(cx: &mut TestAppContext) {
    let (page, cx) = open(cx);
    page.update(cx, |page, cx| {
        page.select(Section::Bugs, cx);
        page.set_flush(true, cx);
    });
    cx.run_until_parked();
    cx.update(|window, cx| window.draw(cx).clear(cx));
    let (panel, card) = cx.update(|_, cx| {
        let look = cx.look().clone();
        (crate::reviews::panel_inner_radius(&look), crate::reviews::card_inner_radius(&look))
    });
    assert!(panel > gpui::px(0.) && card > gpui::px(0.));
    assert_eq!(radius(cx, Role::Region, "Feature band"), Some(panel), "the band sits on the page panel's top edge");
    for head in ["On it now", "Who worked on it", "Asks", "Designs", "Code", "Floor plan", "How to test it", "How it runs"] {
        assert_eq!(radius(cx, Role::Heading, head), Some(card), "{head} sits on its card's top edge");
    }
    assert_eq!(radius(cx, Role::Image, "Stale since #3692"), Some(card), "a stale mark is the card's first child");
    assert_eq!(radius(cx, Role::Heading, "How to observe it"), None, "a head under the stale mark is not on the edge");
}

#[gpui::test]
fn a_band_under_a_back_row_is_not_on_the_panels_edge_and_keeps_square_corners(cx: &mut TestAppContext) {
    let (_page, cx) = open(cx);
    assert_eq!(radius(cx, Role::Region, "Feature band"), Some(gpui::px(0.)));
}

#[gpui::test]
fn the_band_draws_the_health_chip_with_its_reasons_and_planned_where_it_is(cx: &mut TestAppContext) {
    let (page, cx) = open(cx);
    cx.update(|window, cx| window.draw(cx).clear(cx));
    assert!(drawn(cx, Role::Status, "Health: Watch"), "the chip is named");
    assert!(drawn(cx, Role::Label, "Observe section stale since #3692 \u{b7} Open bug: the card loses focus after a retry"), "the reasons stand beside it");
    assert!(!drawn(cx, Role::Status, "Planned"));
    page.update(cx, |page, cx| {
        if let Load::Read(read) = &mut page.page {
            read.feature.planned = true;
            read.feature.health.mark = ipc::HealthMark::Good;
            read.feature.health.reasons.clear();
        }
        page.rebuild(cx);
    });
    cx.run_until_parked();
    cx.update(|window, cx| window.draw(cx).clear(cx));
    assert!(drawn(cx, Role::Status, "Planned"), "a planned feature says so");
    // The page has no window root to clear the book each frame, so a chip that went stands in it: what the band holds is asked of the model.
    assert!(page.read_with(cx, |page, _| matches!(&page.page, Load::Read(read) if page_model::health(read).is_none())), "good draws no chip");
}

#[gpui::test]
fn the_new_cards_draw_their_rows_by_name_with_the_marks_a_row_carries(cx: &mut TestAppContext) {
    let (page, cx) = open(cx);
    cx.update(|window, cx| window.draw(cx).clear(cx));
    for card in ["Docs", "Tickets", "Decisions"] {
        assert!(drawn(cx, Role::Region, card) && drawn(cx, Role::Heading, card), "{card} is drawn");
    }
    assert!(drawn(cx, Role::ListBoxOption, "The start card"));
    assert!(drawn(cx, Role::ListBoxOption, "armada-a3f2 The card loses focus after a retry"));
    assert!(drawn(cx, Role::ListBoxOption, "Where does the model picker go on the start card?"));
    assert!(drawn(cx, Role::Label, "Job's words"), "the gist the Job wrote is marked");
    assert!(drawn(cx, Role::Label, "gone"), "an address that no longer answers says so");
    assert!(drawn(cx, Role::Label, "Names the code no longer has: StartProposal, pick_model"));
    assert!(drawn(cx, Role::Image, "A web or MCP address") && drawn(cx, Role::Image, "A ticket") && drawn(cx, Role::Image, "A file in the repository"));
    // The keys reach each card and its rows.
    page.update(cx, |page, cx| page.select(Section::Docs, cx));
    cx.update(|window, cx| page.update(cx, |page, cx| page.focus_region(Region::Rows, window, cx)));
    cx.simulate_keystrokes("j");
    assert_eq!(page.read_with(cx, |page, _| (page.section(), page.row_at, page.row_count())), (Section::Docs, Some(1), 3));
}

fn press(page: &gpui::Entity<FeaturePageView>, region: Region, cx: &mut VisualTestContext) {
    cx.update(|window, cx| page.update(cx, |page, cx| page.focus_region(region, window, cx)));
    cx.simulate_keystrokes("enter");
    cx.run_until_parked();
    cx.update(|window, cx| window.draw(cx).clear(cx));
}

#[gpui::test]
fn the_retire_row_has_its_two_buttons_and_keep_takes_the_row_away(cx: &mut TestAppContext) {
    let (page, cx) = open(cx);
    cx.update(|window, cx| window.draw(cx).clear(cx));
    assert!(drawn(cx, Role::Region, "Retire this feature?"));
    assert!(drawn(cx, Role::Button, "Retire") && drawn(cx, Role::Button, "Keep"));
    assert!(page.read_with(cx, |page, _| page.regions().contains(&Region::Retire) && page.regions().contains(&Region::Keep)));
    press(&page, Region::Keep, cx);
    // The page has no window root to clear the book each frame, so what went is asked of the page: it draws the row only while it proposes.
    assert!(page.read_with(cx, |page, _| !page.proposes_retire()), "the proposal is gone");
    assert!(page.read_with(cx, |page, _| !page.regions().contains(&Region::Retire)), "and the keys no longer stop on it");
    assert_eq!(cx.update(|window, cx| page.read(cx).region_in(window)), Region::Rail, "the keys leave the buttons that went");
}

#[gpui::test]
fn retire_says_the_feature_left_the_catalog_and_tells_whoever_holds_the_page(cx: &mut TestAppContext) {
    let (page, cx) = open(cx);
    let told = std::rc::Rc::new(std::cell::Cell::new(0));
    let heard = told.clone();
    let _sub = cx.update(|_, cx| cx.subscribe(&page, move |_, event: &super::page::FeaturePageEvent, _| if matches!(event, super::page::FeaturePageEvent::Retired) { heard.set(heard.get() + 1) }));
    press(&page, Region::Retire, cx);
    assert_eq!(told.get(), 1, "the list that holds the feature reads again");
    assert!(drawn(cx, Role::Status, "Retired. It left the catalog."));
    assert!(page.read_with(cx, |page, _| !page.proposes_retire() && !page.regions().contains(&Region::Retire)), "the buttons are gone");
}

#[gpui::test]
fn a_refused_answer_stands_on_the_row_in_fleets_words_and_the_buttons_stay(cx: &mut TestAppContext) {
    let (page, cx) = open(cx);
    page.update(cx, |page, _| page.mock_refusal = Some("This feature was not proposed for retiring".into()));
    press(&page, Region::Retire, cx);
    assert!(drawn(cx, Role::Label, "This feature was not proposed for retiring"));
    assert!(page.read_with(cx, |page, _| page.proposes_retire()), "the row stays to answer again");
    page.update(cx, |page, _| page.mock_refusal = None);
    press(&page, Region::Keep, cx);
    assert!(page.read_with(cx, |page, _| !page.proposes_retire()), "the answer goes through the second time");
}

#[gpui::test]
fn a_page_with_no_proposal_has_no_retire_row_and_no_stop_for_the_keys(cx: &mut TestAppContext) {
    let (page, cx) = open(cx);
    page.update(cx, |page, cx| {
        if let Load::Read(read) = &mut page.page {
            read.feature.retire = None;
        }
        page.rebuild(cx);
    });
    assert!(page.read_with(cx, |page, _| !page.proposes_retire() && !page.regions().contains(&Region::Retire) && !page.regions().contains(&Region::Keep)));
}
