use chrono::{DateTime, TimeZone, Utc};
use gpui::{Role, TestAppContext, VisualTestContext};
use ipc::{AskState, FeatureLinkKind};

use super::page::{FeaturePageView, Load, Region};
use super::page_model::{self, Body, Chip, Section, Standing};
use super::{looked, mock};

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
    assert_eq!(order, ["Who worked on it", "Code", "How to test it", "Designs", "Bugs and issues", "Change requests", "Ideas", "How it runs", "How to observe it", "Tech debt"]);
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
    let bare = ipc::FeaturePage { sections: Vec::new(), links: Vec::new(), files: Vec::new(), entry_points: Vec::new(), paths: Vec::new(), ..page };
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
fn j_and_k_move_the_open_section_and_tab_goes_between_the_rail_the_chips_and_the_rows(cx: &mut TestAppContext) {
    let (page, cx) = open(cx);
    let section = |page: &gpui::Entity<FeaturePageView>, cx: &mut VisualTestContext| page.read_with(cx, |page, _| page.section());
    let region = |page: &gpui::Entity<FeaturePageView>, cx: &mut VisualTestContext| cx.update(|window, cx| page.read(cx).region_in(window));
    assert_eq!(section(&page, cx), Section::Who);
    cx.simulate_keystrokes("j");
    assert_eq!(section(&page, cx), Section::Code, "the section the cursor is on is the one open");
    cx.simulate_keystrokes("down down down");
    assert_eq!(section(&page, cx), Section::Bugs);
    cx.simulate_keystrokes("k");
    assert_eq!(section(&page, cx), Section::Designs);
    cx.simulate_keystrokes("j up up up up up up");
    assert_eq!(section(&page, cx), Section::Who, "it stops at the first");
    cx.simulate_keystrokes("j j j j");
    assert_eq!(section(&page, cx), Section::Bugs);
    // Tab: rail, then the chips, then the rows, round to the rail.
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
    assert_eq!(region(&page, cx), Region::Rail, "it wraps");
    cx.simulate_keystrokes("shift-tab");
    assert_eq!(region(&page, cx), Region::Rows);
    cx.simulate_keystrokes("shift-tab");
    assert_eq!(region(&page, cx), Region::Chips);
    // A section with no chips and nothing to pick goes straight round.
    cx.simulate_keystrokes("shift-tab");
    assert_eq!(region(&page, cx), Region::Rail);
    cx.simulate_keystrokes("j j");
    assert_eq!(section(&page, cx), Section::Ideas);
    cx.simulate_keystrokes("j j");
    assert_eq!(section(&page, cx), Section::Observe);
    cx.simulate_keystrokes("tab");
    assert_eq!(region(&page, cx), Region::Rail, "the written section has no chips and no rows");
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
    assert!(drawn(cx, Role::TabList, "Sections"));
    for section in Section::ALL {
        assert!(drawn(cx, Role::Tab, section.title()), "{} is a tab", section.title());
    }
    assert!(drawn(cx, Role::Region, "Bugs and issues"), "the pane is a region named by its section");
    assert!(drawn(cx, Role::Heading, "Bugs and issues"));
    for chip in Chip::ALL {
        assert!(drawn(cx, Role::Button, chip.label()), "{} is a button", chip.label());
    }
    assert!(drawn(cx, Role::ListBox, "Bugs and issues"));
    assert!(drawn(cx, Role::ListBoxOption, "Native draws no sketch in a page"));
    assert!(drawn(cx, Role::Image, "Handed"), "a row's state has a name");
    page.update(cx, |page, cx| page.select(Section::Code, cx));
    cx.run_until_parked();
    assert!(drawn(cx, Role::Region, "Code"));
    assert!(drawn(cx, Role::Image, "Jobs that wrote crates/fleet/src/start/card.rs: 6"));
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
    assert!(drawn(cx, Role::Tab, "Code"));
    assert_eq!(page.read_with(cx, |page, _| page.entries.len()), 10);
}
