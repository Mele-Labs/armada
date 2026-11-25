use chrono::{Duration, Utc};
use gpui::{Role, TestAppContext, VisualTestContext};
use ipc::LookedTag;

use super::project_model::{self, AreaHue, Glossary, ProjectPage, GLOSSARY_EMPTY};
use super::project_page::{ProjectPageView, Target};
use super::{looked, mock, FeaturesScreen};
use crate::look::token;

fn ready(cx: &mut TestAppContext) {
    cx.update(|cx| {
        gpui_component::init(cx);
        crate::look::init(cx);
        crate::keymap::install(&crate::keymap::Rebinding::default(), cx);
        crate::titlebar::bind(cx);
        looked::forget_all(cx);
    });
}

fn page(cx: &mut TestAppContext, which: ProjectPage) -> (gpui::Entity<ProjectPageView>, &mut VisualTestContext) {
    ready(cx);
    let (page, cx) = cx.add_window_view(|_, cx| ProjectPageView::mock(which, cx));
    cx.run_until_parked();
    (page, cx)
}

fn drawn(cx: &mut VisualTestContext, role: Role, name: &str) -> bool {
    cx.update(|window, cx| window.draw(cx).clear(cx));
    cx.update(|window, cx| cx.default_global::<crate::labels::LabelBook>().find(window.window_handle().window_id(), role, name).is_some())
}

fn areas() -> Vec<AreaHue> {
    vec![AreaHue { slug: "running-a-job".into(), title: "Running a Job".into(), hue: token::STATUS_RUNNING }]
}

#[test]
fn the_overview_keeps_fleets_busiest_order_and_the_empty_glossary_says_when_terms_come() {
    let read = mock::overview();
    let model = project_model::overview(&read, &areas());
    let served: Vec<&str> = read.busiest.iter().map(|one| one.slug.as_str()).collect();
    assert_eq!(model.busiest.iter().map(|one| one.slug.as_str()).collect::<Vec<_>>(), served, "the busiest keep the order they were served in");
    assert_eq!(model.busiest[0].pulls, "#3863 #3862", "the pull request numbers keep Fleet's order");
    assert_eq!(model.glossary, Glossary::Empty("Terms appear as Jobs add them at their end"));
    assert_eq!(GLOSSARY_EMPTY, "Terms appear as Jobs add them at their end");
    assert_eq!(model.key.iter().map(|area| area.slug.as_str()).collect::<Vec<_>>(), ["running-a-job"], "the key holds the hues the tiles use");
    let fleet = model.tiles.iter().find(|tile| tile.module == "fleet").expect("a tile for fleet");
    assert_eq!(fleet.hue, Some(token::STATUS_RUNNING), "a room takes its area's hue");
    assert_eq!(model.tiles.iter().find(|tile| tile.module == "xtask").and_then(|tile| tile.hue), None);
}

#[gpui::test]
fn the_overview_draws_the_lede_the_busiest_rows_and_the_glossary_line(cx: &mut TestAppContext) {
    let (_page, cx) = page(cx, ProjectPage::Overview);
    assert!(drawn(cx, Role::Region, "Code map") && drawn(cx, Role::Region, "Busiest features") && drawn(cx, Role::Region, "Glossary"));
    assert!(drawn(cx, Role::Label, "Terms appear as Jobs add them at their end"));
    assert!(drawn(cx, Role::Button, "Job list and live status"), "a busiest row is pressed to open its feature");
    let sentence = mock::overview().what_it_is.expect("the mock has a sentence").sentence;
    assert!(drawn(cx, Role::Label, &format!("{sentence} From README.md")));
}

#[gpui::test]
fn an_overview_without_a_readme_sentence_has_no_lede(cx: &mut TestAppContext) {
    ready(cx);
    let (_page, cx) = cx.add_window_view(|_, cx| {
        let mut page = ProjectPageView::mock(ProjectPage::Overview, cx);
        let mut read = mock::overview();
        read.what_it_is = None;
        page.fill(Some(read), None, cx);
        page
    });
    cx.run_until_parked();
    assert!(drawn(cx, Role::Region, "Code map"), "the page is drawn");
    assert!(!drawn(cx, Role::Label, &format!("{} From README.md", mock::overview().what_it_is.expect("the mock has a sentence").sentence)));
    assert_eq!(project_model::overview(&{ let mut read = mock::overview(); read.what_it_is = None; read }, &areas()).lede, None);
}

#[test]
fn since_you_looked_keeps_the_tag_order_fleet_served() {
    let read = mock::since(Utc::now() - Duration::days(3));
    let model = project_model::since(&read);
    assert_eq!(model.rows.iter().map(|row| row.tag).collect::<Vec<_>>(), [LookedTag::NeedsYou, LookedTag::Changed, LookedTag::Changed, LookedTag::New]);
    assert_eq!(model.rows.iter().map(|row| row.slug.as_str()).collect::<Vec<_>>(), ["model-and-context", "thread-and-composer", "job-list-and-live-status", "agent-notes-on-a-diff"]);
    assert_eq!(project_model::tag_mark(LookedTag::NeedsYou).1, token::STATUS_AWAITING_REVIEW);
    assert!(model.paragraph.is_some(), "a paragraph card is drawn where Fleet gave one");
    let quiet = project_model::since(&mock::since(Utc::now()));
    assert_eq!(quiet.paragraph, None, "an empty paragraph draws no card");
}

#[gpui::test]
fn the_rows_are_drawn_and_reached_in_fleets_order(cx: &mut TestAppContext) {
    let (page, cx) = page(cx, ProjectPage::Since);
    assert!(drawn(cx, Role::Button, "To confirm: Model and context") && drawn(cx, Role::Button, "New: Agent notes on a diff") && drawn(cx, Role::Button, "Mark read"));
    let targets = page.read_with(cx, |page, _| page.targets());
    assert_eq!(targets.first(), Some(&Target::MarkRead));
    assert_eq!(targets.get(1), Some(&Target::Feature("model-and-context".into())));
    assert_eq!(targets.last(), Some(&Target::Feature("agent-notes-on-a-diff".into())));
}

#[gpui::test]
fn mark_read_sets_the_time_to_now_keeps_it_and_reads_again_for_it(cx: &mut TestAppContext) {
    let (page, cx) = page(cx, ProjectPage::Since);
    let before = page.read_with(cx, |page, _| page.since_at());
    assert!(page.read_with(cx, |page, _| page.targets().len()) > 2, "three days back, several features moved");
    page.update(cx, |page, cx| page.mark_read(cx));
    let after = page.read_with(cx, |page, _| page.since_at());
    assert!(after > before + Duration::days(2), "the time moved to now");
    assert_eq!(cx.update(|_, cx| looked::before_project(mock::PROJECT, cx)), Some(after), "it is kept for the project");
    let targets = page.read_with(cx, |page, _| page.targets());
    assert_eq!(targets, [Target::MarkRead, Target::Feature("model-and-context".into())], "the refetch for the new time leaves only what waits on him");
    assert!(drawn(cx, Role::Label, &project_model::when_said(after)), "the time of his last look is drawn under the title");
}

#[gpui::test]
fn nothing_since_says_so_with_the_time(cx: &mut TestAppContext) {
    let (page, cx) = page(cx, ProjectPage::Since);
    page.update(cx, |page, cx| {
        let mut read = mock::since(Utc::now());
        read.features.clear();
        page.fill(None, Some(read), cx);
    });
    let at = page.read_with(cx, |page, _| page.since_at());
    assert!(drawn(cx, Role::Label, &project_model::nothing_since(at)));
}

#[test]
fn the_first_look_compares_with_the_mark_then_the_librarian_then_fourteen_days_ago() {
    let now = Utc::now();
    let (kept, ran) = (now - Duration::days(1), now - Duration::days(2));
    assert_eq!(project_model::first_since(Some(kept), Some(ran), now), kept);
    assert_eq!(project_model::first_since(None, Some(ran), now), ran);
    assert_eq!(project_model::first_since(None, None, now), now - Duration::days(14));
}

fn screen(cx: &mut TestAppContext) -> (gpui::Entity<FeaturesScreen>, &mut VisualTestContext) {
    ready(cx);
    let (screen, cx) = cx.add_window_view(|window, cx| FeaturesScreen::mock(window, cx));
    cx.update(|window, cx| screen.update(cx, |screen, cx| screen.enter(window, cx)));
    cx.run_until_parked();
    (screen, cx)
}

fn right(screen: &gpui::Entity<FeaturesScreen>, cx: &mut VisualTestContext) -> (Option<ProjectPage>, Option<String>) {
    screen.read_with(cx, |screen, cx| (screen.project_open(cx), screen.opened().map(|row| row.slug.clone())))
}

#[gpui::test]
fn the_three_rows_sit_over_the_areas_and_swap_the_right_panel(cx: &mut TestAppContext) {
    let (screen, cx) = screen(cx);
    assert!(drawn(cx, Role::ListBoxOption, "Overview") && drawn(cx, Role::ListBoxOption, "Since you looked") && drawn(cx, Role::ListBoxOption, "Who's on what"));
    assert_eq!(right(&screen, cx), (None, Some("start-a-job".into())), "a feature's page is beside the list at first");
    // Up from the first feature: Who's on what, Since you looked, then Overview, with the keys as the feature rows have them.
    cx.simulate_keystrokes("k");
    cx.run_until_parked();
    assert_eq!(right(&screen, cx), (Some(ProjectPage::Who), None));
    cx.simulate_keystrokes("k");
    cx.run_until_parked();
    assert_eq!(right(&screen, cx), (Some(ProjectPage::Since), None));
    cx.simulate_keystrokes("k");
    cx.run_until_parked();
    assert_eq!(right(&screen, cx), (Some(ProjectPage::Overview), None));
    cx.simulate_keystrokes("k");
    cx.run_until_parked();
    assert_eq!(right(&screen, cx), (Some(ProjectPage::Overview), None), "Overview is the top");
    cx.simulate_keystrokes("j j j");
    cx.run_until_parked();
    assert_eq!(right(&screen, cx), (None, Some("start-a-job".into())), "down from Who's on what is the first feature");
}

#[gpui::test]
fn a_press_on_a_row_of_a_page_opens_its_feature_in_place_of_the_page(cx: &mut TestAppContext) {
    let (screen, cx) = screen(cx);
    cx.simulate_keystrokes("k k k");
    cx.run_until_parked();
    let page = screen.read_with(cx, |screen, _| screen.project_view()).expect("the Overview is open");
    page.update(cx, |page, cx| page.pick(1, cx));
    cx.run_until_parked();
    assert_eq!(right(&screen, cx), (None, Some("thread-and-composer".into())), "the second busiest feature is open");
}

#[gpui::test]
fn a_library_with_no_feature_shows_the_start_panel_in_the_overviews_place(cx: &mut TestAppContext) {
    ready(cx);
    let (screen, cx) = cx.add_window_view(|window, cx| FeaturesScreen::mock(window, cx));
    cx.update(|window, cx| {
        screen.update(cx, |screen, cx| {
            screen.library = super::Load::Read(ipc::FeatureLibrary { project: mock::PROJECT.into(), areas: Vec::new(), librarian: Default::default() });
            screen.enter(window, cx);
            screen.relist(cx);
        })
    });
    cx.run_until_parked();
    assert_eq!(right(&screen, cx), (None, None));
    assert!(screen.read_with(cx, |screen, _| screen.start.is_some()), "the start panel stands where the Overview would");
}
