//! The Docs card's Check and the flag under a doc that disagrees with the code, on the mock page: what is drawn, where the
//! keys stop, and what each press does.

use gpui::{Role, TestAppContext, VisualTestContext};

use super::doc_checks::{Checking, FlagAnswer, DISAGREE};
use super::page::{FeaturePageView, Load, Region};
use super::{looked, mock};

fn open(cx: &mut TestAppContext) -> (gpui::Entity<FeaturePageView>, &mut VisualTestContext) {
    cx.update(|cx| {
        gpui_component::init(cx);
        crate::look::init(cx);
        crate::keymap::install(&crate::keymap::Rebinding::default(), cx);
        crate::titlebar::bind(cx);
        looked::forget_all(cx);
    });
    let (page, cx) = cx.add_window_view(|window, cx| FeaturePageView::mock("start-a-job".into(), window, cx));
    cx.update(|window, cx| page.update(cx, |page, cx| page.focus_rail(window, cx)));
    page.update(cx, |page, cx| page.select(super::page_model::Section::Docs, cx));
    cx.run_until_parked();
    cx.update(|window, cx| window.draw(cx).clear(cx));
    (page, cx)
}

fn drawn(cx: &mut VisualTestContext, role: Role, name: &str) -> bool {
    cx.update(|window, cx| cx.default_global::<crate::labels::LabelBook>().find(window.window_handle().window_id(), role, name).is_some())
}

fn press(page: &gpui::Entity<FeaturePageView>, region: Region, cx: &mut VisualTestContext) {
    cx.update(|window, cx| page.update(cx, |page, cx| page.focus_region(region, window, cx)));
    cx.simulate_keystrokes("enter");
    cx.run_until_parked();
    cx.update(|window, cx| window.draw(cx).clear(cx));
}

fn flag_of(page: &gpui::Entity<FeaturePageView>, cx: &mut VisualTestContext, target: &str) -> Option<ipc::DocCheck> {
    page.read_with(cx, |page, _| match &page.page {
        Load::Read(read) => read.docs.iter().find(|doc| doc.target == target).and_then(|doc| doc.check.clone()),
        _ => None,
    })
}

#[gpui::test]
fn the_docs_card_has_check_a_flag_under_the_doc_that_disagrees_and_says_when_the_other_agreed(cx: &mut TestAppContext) {
    let (page, cx) = open(cx);
    assert!(drawn(cx, Role::Button, "Check"), "Check sits in the Docs card's header");
    assert!(drawn(cx, Role::Region, &format!("{DISAGREE}: The start card")), "the flag is under the doc that disagrees");
    assert!(drawn(cx, Role::Image, DISAGREE), "its glyph is named");
    assert!(drawn(cx, Role::Button, "Start a Job") && drawn(cx, Role::Button, "Dismiss"));
    let agreed = super::page_model::checked_line(&ipc::Instant::carried("2026-11-15T12:00:00Z"));
    assert!(drawn(cx, Role::Label, &agreed), "the doc that agrees says so in its last line: {agreed}");
    let regions = page.read_with(cx, |page, _| page.regions());
    let at = |region: Region| regions.iter().position(|one| *one == region).unwrap_or_else(|| panic!("{region:?} is a stop: {regions:?}"));
    assert!(at(Region::Rows) < at(Region::Check) && at(Region::Check) < at(Region::FlagStart) && at(Region::FlagStart) < at(Region::FlagDismiss) && at(Region::FlagDismiss) < at(Region::Retire), "{regions:?}");
    page.update(cx, |page, cx| page.select(super::page_model::Section::Code, cx));
    assert!(page.read_with(cx, |page, _| !page.regions().contains(&Region::Check)), "another card's Tab passes them over");
}

#[gpui::test]
fn check_runs_on_the_press_and_a_refusal_stands_in_the_header_line(cx: &mut TestAppContext) {
    let (page, cx) = open(cx);
    page.update(cx, |page, _| page.mock_refusal = Some("The librarian's share is spent tonight: 20 of 20 calls.".into()));
    press(&page, Region::Check, cx);
    assert!(drawn(cx, Role::Status, "The librarian's share is spent tonight: 20 of 20 calls."));
    page.update(cx, |page, _| page.mock_refusal = None);
    press(&page, Region::Check, cx);
    assert_eq!(page.read_with(cx, |page, _| page.checking.clone()), Checking::Idle, "the line goes with an answer");
    assert!(flag_of(&page, cx, "docs/concepts/start-card.md").and_then(|check| check.disagrees).is_some(), "the flag stays after a check that finds it again");
}

#[gpui::test]
fn dismiss_quiets_the_flag_and_the_keys_go_back_to_check(cx: &mut TestAppContext) {
    let (page, cx) = open(cx);
    press(&page, Region::FlagDismiss, cx);
    assert!(flag_of(&page, cx, "docs/concepts/start-card.md").is_some_and(|check| check.dismissed));
    assert!(page.read_with(cx, |page, _| page.flagged().is_empty() && !page.regions().contains(&Region::FlagStart)), "no flag, no stop");
    assert_eq!(cx.update(|window, cx| page.read(cx).region_in(window)), Region::Check);
}

#[gpui::test]
fn start_a_job_keeps_the_flag_and_a_refusal_stands_on_the_flag(cx: &mut TestAppContext) {
    let (page, cx) = open(cx);
    page.update(cx, |page, _| page.mock_refusal = Some("No check found that doc disagreeing with the code.".into()));
    press(&page, Region::FlagStart, cx);
    assert!(drawn(cx, Role::Label, "No check found that doc disagreeing with the code."));
    page.update(cx, |page, _| page.mock_refusal = None);
    press(&page, Region::FlagStart, cx);
    assert_eq!(page.read_with(cx, |page, _| page.flag_answer.clone()), FlagAnswer::Idle, "the answer went through and the refusal went with it");
    assert!(page.read_with(cx, |page, _| page.flagged().len() == 1), "the flag stays until he dismisses it");
}

#[test]
fn a_flag_is_drawn_only_where_a_check_found_a_disagreement_he_has_not_dismissed() {
    use super::page_model::{body, Body, Chip, Section};
    let mut page = mock::page("start-a-job");
    let rows = |page: &ipc::FeaturePage| match body(Section::Docs, page, None, Chip::All, None, &mock::titles()) {
        Body::Reads(rows) => rows,
        _ => panic!("reads"),
    };
    let docs = rows(&page);
    assert_eq!(docs[0].flag.as_ref().map(|flag| flag.code_at.as_str()), Some("crates/fleet/src/start/card.rs:88"));
    assert_eq!(docs[0].checked, None, "a doc that disagrees does not say it agrees");
    assert_eq!(docs[3].checked.as_deref(), Some(super::page_model::checked_line(&ipc::Instant::carried("2026-11-15T12:00:00Z")).as_str()));
    assert!(docs[1].flag.is_none() && docs[1].checked.is_none(), "a doc never checked says nothing of it");
    page.docs[0].check.as_mut().expect("checked").dismissed = true;
    let docs = rows(&page);
    assert!(docs[0].flag.is_none() && docs[0].checked.is_none(), "a dismissed flag draws nothing");
}
