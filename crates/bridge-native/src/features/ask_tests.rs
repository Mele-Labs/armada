use std::cell::RefCell;
use std::rc::Rc;

use gpui::{Modifiers, Role, TestAppContext, VisualTestContext};
use ipc::{Ask, AskState, SessionId};

use super::page::{FeaturePageEvent, FeaturePageView, Region};
use super::page_model::{self, Body, Chip, Hue, Item, Section, Titles};
use super::story_link::{self, StoryRequest};
use super::{looked, mock, FeaturesScreen};

const SLUG: &str = "start-a-job";

fn ask_of(state: AskState, said_in: &str, building: Option<&str>) -> Ask {
    let mut ask = mock::asks(SLUG).asks.remove(0);
    ask.state = state;
    ask.said_in = SessionId::carried(said_in);
    ask.building = building.map(SessionId::carried);
    ask
}

fn titles() -> Titles {
    mock::titles()
}

/// The row an ask makes: its second line as one string, and the Job it names as a link.
fn line(ask: &Ask, titles: &Titles) -> (String, Option<String>) {
    let Body::Rows(rows) = page_model::body(Section::Bugs, &mock::page(SLUG), Some(std::slice::from_ref(ask)), Chip::All, None, titles) else { panic!("rows") };
    let row = rows.into_iter().find(|row| row.key == ask.ask_id).expect("the ask is a row");
    let line = row.ask.expect("an ask row");
    assert_eq!(line.before.clone() + line.job.as_deref().unwrap_or_default() + &line.after, row.meta);
    (row.meta, line.job)
}

fn date() -> String {
    page_model::day(page_model::when(&mock::asks(SLUG).asks[0].opened_at).unwrap())
}

// ---- the model -------------------------------------------------------------------------------

#[test]
fn each_state_says_where_the_ask_stands_in_a_sentence() {
    let titles = titles();
    let d = date();
    let said = |state, said_in, building| line(&ask_of(state, said_in, building), &titles).0;
    assert_eq!(said(AskState::Handed, "job-card-title", Some("job-start-card")), format!("You, {d}, in Propose the title from the first line. Handed to Add the model picker to the start card"));
    assert_eq!(said(AskState::Built, "job-start-card", Some("job-card-workflow")), format!("You, {d}, in Add the model picker to the start card. Fixed by Pick the workflow from the repository"));
    assert_eq!(said(AskState::Building, "job-card-title", Some("job-start-card")), format!("You, {d}, in Propose the title from the first line. Building in Add the model picker to the start card"));
    assert_eq!(said(AskState::Building, "job-start-card", Some("job-start-card")), format!("You, {d}, in Add the model picker to the start card. Building"), "building where it was said is not said twice");
    assert_eq!(said(AskState::Backlog, "job-start-card", None), format!("You, {d}, in Add the model picker to the start card. Backlog"));
    assert_eq!(said(AskState::Dropped, "job-start-card", None), format!("You, {d}, in Add the model picker to the start card. Dropped"));
}

#[test]
fn an_ask_filed_outside_a_job_says_it_went_to_the_operator() {
    let (meta, job) = line(&ask_of(AskState::Backlog, "", None), &titles());
    assert_eq!(meta, format!("You, {}, to the Operator. Backlog", date()));
    assert_eq!(job, None);
}

#[test]
fn a_job_with_no_title_is_not_named_and_never_by_its_id() {
    let d = date();
    let (meta, job) = line(&ask_of(AskState::Built, "job-gone", Some("job-also-gone")), &titles());
    assert_eq!(meta, format!("You, {d}. Fixed"));
    assert_eq!(job, None);
    assert!(!meta.contains("job-"), "{meta}");
    let (meta, _) = line(&ask_of(AskState::Handed, "job-start-card", Some("job-gone")), &titles());
    assert_eq!(meta, format!("You, {d}, in Add the model picker to the start card. Handed"));
}

#[test]
fn his_words_are_the_first_line_in_quotes_and_the_title_where_they_run_long() {
    let titles = titles();
    let first = |ask: &Ask| {
        let Body::Rows(rows) = page_model::body(Section::Bugs, &mock::page(SLUG), Some(std::slice::from_ref(ask)), Chip::All, None, &titles) else { panic!("rows") };
        rows.into_iter().find(|row| row.key == ask.ask_id).and_then(|row| row.ask).map(|line| line.words).unwrap()
    };
    let mut ask = ask_of(AskState::Backlog, "job-start-card", None);
    ask.first_words = "they show as i never voted".into();
    assert_eq!(first(&ask), "\"they show as i never voted\"");
    ask.first_words = "when the start is slow the card asks the small model for a title two times and the second one replaces mine".into();
    assert_eq!(first(&ask), ask.title, "long words give way to the title, unquoted");
    ask.first_words = "one line\nand another".into();
    assert_eq!(first(&ask), ask.title);
    ask.first_words = "  ".into();
    assert_eq!(first(&ask), ask.title);
}

#[test]
fn the_story_opens_the_job_it_was_said_in_and_the_one_building_it_only_where_it_was_said_in_none() {
    let story = |said_in, building| page_model::story_of(&ask_of(AskState::Handed, said_in, building)).map(|id| id.as_str().to_string());
    assert_eq!(story("job-a", Some("job-b")).as_deref(), Some("job-a"));
    assert_eq!(story("job-a", None).as_deref(), Some("job-a"));
    assert_eq!(story("", Some("job-b")).as_deref(), Some("job-b"));
    assert_eq!(story("", None), None);
}

#[test]
fn a_state_has_its_own_hue_and_a_backlog_ask_is_not_the_dropped_one() {
    let titles = titles();
    let asks = mock::asks(SLUG).asks;
    let Body::Rows(rows) = page_model::body(Section::Bugs, &mock::page(SLUG), Some(&asks), Chip::All, None, &titles) else { panic!("rows") };
    let hue = |key: &str| rows.iter().find(|row| row.key == key).map(|row| (row.hue, row.glyph, row.said.clone())).unwrap();
    assert_eq!(hue("ask-1"), (Hue::Handed, "circle", "Handed".into()));
    assert_eq!(hue("ask-2"), (Hue::Done, "circle-check", "Fixed".into()));
    assert_eq!(hue("ask-3"), (Hue::NotStarted, "circle-dashed", "Backlog".into()));
    assert_eq!(hue("ask-4"), (Hue::Muted, "circle-x", "Dropped".into()));
}

#[test]
fn the_fold_shows_five_rows_then_one_line_and_the_rest_only_while_open() {
    let rows = |count, open| page_model::items(Section::Bugs, count, open);
    assert_eq!(rows(5, false), (0..5).map(Item::Row).collect::<Vec<_>>(), "five rows need no fold");
    assert_eq!(rows(7, false), [Item::Row(0), Item::Row(1), Item::Row(2), Item::Row(3), Item::Row(4), Item::Fold]);
    assert_eq!(rows(7, true).len(), 8);
    assert_eq!(rows(7, true)[5], Item::Fold, "the line stays under the fifth row");
    assert_eq!(rows(7, true)[7], Item::Row(6));
    assert_eq!(page_model::items(Section::Who, 7, false).len(), 7, "only the asks fold");
}

// ---- the page --------------------------------------------------------------------------------

fn ready(cx: &mut TestAppContext) {
    cx.update(|cx| {
        gpui_component::init(cx);
        crate::look::init(cx);
        crate::keymap::install(&crate::keymap::Rebinding::default(), cx);
        crate::titlebar::bind(cx);
        looked::forget_all(cx);
    });
}

type Heard = Rc<RefCell<Vec<(String, String)>>>;

/// The page on the Bugs section with the keys on its rows, and what it says about a Story.
fn open(cx: &mut TestAppContext) -> (gpui::Entity<FeaturePageView>, Heard, &mut VisualTestContext) {
    ready(cx);
    let (page, cx) = cx.add_window_view(|window, cx| FeaturePageView::mock(SLUG.into(), window, cx));
    let heard: Heard = Rc::default();
    let sink = heard.clone();
    cx.update(|_, cx| {
        cx.subscribe(&page, move |_, event: &FeaturePageEvent, _| match event {
            FeaturePageEvent::Story { session_id, ask_id } => sink.borrow_mut().push((session_id.as_str().to_string(), ask_id.clone())),
            FeaturePageEvent::Escape => sink.borrow_mut().push(("escape".into(), String::new())),
            FeaturePageEvent::Design { job, artifact } => sink.borrow_mut().push((format!("design {job}"), artifact.clone())),
            FeaturePageEvent::Retired => sink.borrow_mut().push(("retired".into(), String::new())),
        })
        .detach();
    });
    cx.update(|window, cx| {
        page.update(cx, |page, cx| {
            page.select(Section::Bugs, cx);
            page.focus_region(Region::Rows, window, cx);
        })
    });
    cx.run_until_parked();
    (page, heard, cx)
}

fn drawn(cx: &mut VisualTestContext, role: Role, name: &str) -> bool {
    cx.update(|window, cx| cx.default_global::<crate::labels::LabelBook>().find(window.window_handle().window_id(), role, name).is_some())
}

fn row_at(page: &gpui::Entity<FeaturePageView>, cx: &mut VisualTestContext) -> Option<usize> {
    page.read_with(cx, |page, _| page.row_at)
}

#[gpui::test]
fn enter_and_s_on_an_ask_say_which_job_and_which_ask_and_a_row_with_no_job_says_nothing(cx: &mut TestAppContext) {
    let (page, heard, cx) = open(cx);
    // The rows: ask-1, the bug a Job filed, ask-2. The keys start on the first.
    assert_eq!(row_at(&page, cx), Some(0));
    cx.simulate_keystrokes("enter");
    cx.simulate_keystrokes("s");
    assert_eq!(*heard.borrow(), [("job-card-title".to_string(), "ask-1".to_string()), ("job-card-title".to_string(), "ask-1".to_string())], "Enter and s are the same press, and the Job it was said in is the one that opens");
    heard.borrow_mut().clear();
    cx.simulate_keystrokes("j");
    cx.simulate_keystrokes("enter s");
    assert!(heard.borrow().is_empty(), "a bug a Job filed is no ask and has no Story");
    cx.simulate_keystrokes("j enter");
    assert_eq!(*heard.borrow(), [("job-start-card".to_string(), "ask-2".to_string())]);
}

#[gpui::test]
fn the_story_link_is_a_link_named_for_its_ask_and_a_click_on_it_opens_the_story(cx: &mut TestAppContext) {
    let (page, heard, cx) = open(cx);
    assert!(drawn(cx, Role::Link, "Story: Native draws no sketch in a page"));
    assert!(drawn(cx, Role::Link, "Story: They show as i never voted"));
    assert!(!drawn(cx, Role::Link, "Story: The card loses focus after a retry"), "a bug a Job filed has no link");
    let at = cx.update(|window, cx| cx.default_global::<crate::labels::LabelBook>().find(window.window_handle().window_id(), Role::Link, "Story: They show as i never voted").map(|one| one.bounds.center())).expect("drawn");
    cx.simulate_click(at, Modifiers::none());
    assert_eq!(*heard.borrow(), [("job-start-card".to_string(), "ask-2".to_string())]);
    assert_eq!(row_at(&page, cx), Some(0), "a press on the link opens the story and does not move the row");
}

#[gpui::test]
fn an_ask_with_no_job_at_all_has_no_link_and_s_does_nothing(cx: &mut TestAppContext) {
    let (page, heard, cx) = open(cx);
    page.update(cx, |page, cx| page.select(Section::Changes, cx));
    cx.update(|window, cx| page.update(cx, |page, cx| page.focus_region(Region::Rows, window, cx)));
    cx.run_until_parked();
    assert!(drawn(cx, Role::Link, "Story: True voting with sketches"));
    assert!(!drawn(cx, Role::Link, "Story: Keep the request when the card closes"), "said to the Operator, built by nobody");
    cx.simulate_keystrokes("j s");
    assert!(heard.borrow().is_empty());
}

#[gpui::test]
fn the_older_asks_fold_under_one_line_that_enter_opens_and_escape_folds_again(cx: &mut TestAppContext) {
    let (page, heard, cx) = open(cx);
    let count = |page: &gpui::Entity<FeaturePageView>, cx: &mut VisualTestContext| page.read_with(cx, |page, _| page.row_count());
    assert_eq!(count(&page, cx), 6, "five rows and the line");
    assert!(drawn(cx, Role::Button, "Earlier, folded"));
    cx.simulate_keystrokes("j j j j j");
    assert_eq!(row_at(&page, cx), Some(5), "the keys reach the line");
    cx.simulate_keystrokes("enter");
    assert!(heard.borrow().is_empty(), "the line opens the rest and tells nobody");
    assert_eq!(count(&page, cx), 8);
    assert!(page.read_with(cx, |page, _| page.is_unfolded()));
    cx.simulate_keystrokes("j");
    assert_eq!(row_at(&page, cx), Some(6), "the older rows are in reach");
    cx.simulate_keystrokes("escape");
    assert_eq!(count(&page, cx), 6, "Esc folds them");
    assert_eq!(row_at(&page, cx), Some(5), "and the keys stay on the line");
    assert!(heard.borrow().is_empty(), "the page was not left");
    cx.simulate_keystrokes("escape");
    assert_eq!(*heard.borrow(), [("escape".to_string(), String::new())], "Esc again leaves the page");
    // The same press folds again.
    cx.simulate_keystrokes("enter");
    assert_eq!(count(&page, cx), 8);
    cx.simulate_keystrokes("enter");
    assert_eq!(count(&page, cx), 6);
}

#[gpui::test]
fn a_press_on_the_fold_line_opens_it_and_each_section_keeps_its_own_fold(cx: &mut TestAppContext) {
    let (page, _heard, cx) = open(cx);
    let at = cx.update(|window, cx| cx.default_global::<crate::labels::LabelBook>().find(window.window_handle().window_id(), Role::Button, "Earlier, folded").map(|one| one.bounds.center())).expect("drawn");
    cx.simulate_click(at, Modifiers::none());
    assert_eq!(page.read_with(cx, |page, _| page.row_count()), 8);
    assert!(drawn(cx, Role::Button, "Earlier"), "the line stays, and no number is on it");
    page.update(cx, |page, cx| page.select(Section::Changes, cx));
    assert!(!page.read_with(cx, |page, _| page.is_unfolded()), "the Change requests are folded");
    page.update(cx, |page, cx| page.select(Section::Bugs, cx));
    assert!(page.read_with(cx, |page, _| page.is_unfolded()), "the Bugs stay open where they were");
    let (other, cx) = cx.add_window_view(|window, cx| FeaturePageView::mock("reviews".into(), window, cx));
    other.update(cx, |page, cx| page.select(Section::Bugs, cx));
    assert!(!other.read_with(cx, |page, _| page.is_unfolded()), "another page starts folded");
}

#[gpui::test]
fn the_row_of_an_ask_is_found_by_its_title_and_its_state_by_its_name(cx: &mut TestAppContext) {
    let (_page, _heard, cx) = open(cx);
    assert!(drawn(cx, Role::ListBoxOption, "Native draws no sketch in a page"));
    assert!(drawn(cx, Role::Image, "Handed"));
    assert!(drawn(cx, Role::Image, "Fixed"));
    assert!(drawn(cx, Role::Image, "Backlog"));
    assert!(drawn(cx, Role::Image, "Dropped"));
}

#[gpui::test]
fn the_story_act_is_in_the_features_scope(cx: &mut TestAppContext) {
    let (_page, _heard, _cx) = open(cx);
    let bound = crate::keymap::bound(&crate::keymap::Rebinding::default());
    assert!(bound.iter().any(|key| key.act.id == "features_story" && key.context == Some("features")));
}

#[gpui::test]
fn the_screen_leaves_the_ask_for_the_story_tab_and_asks_for_the_job_to_open(cx: &mut TestAppContext) {
    ready(cx);
    let asked: Rc<RefCell<Vec<crate::titlebar::Asked>>> = Rc::default();
    let heard = asked.clone();
    cx.update(|cx| cx.set_global(crate::titlebar::AskedSink(Rc::new(move |one: &crate::titlebar::Asked, _: &mut gpui::App| heard.borrow_mut().push(one.clone())))));
    let (screen, cx) = cx.add_window_view(|window, cx| FeaturesScreen::mock(window, cx));
    cx.update(|window, cx| {
        screen.update(cx, |screen, cx| {
            screen.enter(window, cx);
            screen.open_cursor(window, cx);
            let page = screen.page.clone().expect("the page opened");
            page.update(cx, |page, cx| {
                page.select(Section::Bugs, cx);
                page.focus_region(Region::Rows, window, cx);
            });
        })
    });
    cx.run_until_parked();
    cx.simulate_keystrokes("enter");
    assert!(matches!(asked.borrow().as_slice(), [crate::titlebar::Asked::OpenJob(id)] if id == "job-card-title"), "the Job opens as any screen opens one");
    assert_eq!(cx.update(|_, cx| story_link::take(cx)), Some(StoryRequest { session_id: SessionId::carried("job-card-title"), ask_id: "ask-1".into() }));
    assert_eq!(cx.update(|_, cx| story_link::take(cx)), None, "taking it leaves none");
}

#[gpui::test]
fn enter_or_a_press_on_a_designs_row_opens_its_artifact_in_the_job_that_made_it(cx: &mut TestAppContext) {
    use super::artifact_link::{self, ArtifactRequest};
    ready(cx);
    let asked: Rc<RefCell<Vec<crate::titlebar::Asked>>> = Rc::default();
    let heard = asked.clone();
    cx.update(|cx| cx.set_global(crate::titlebar::AskedSink(Rc::new(move |one: &crate::titlebar::Asked, _: &mut gpui::App| heard.borrow_mut().push(one.clone())))));
    let (screen, cx) = cx.add_window_view(|window, cx| FeaturesScreen::mock(window, cx));
    let page = cx.update(|window, cx| {
        screen.update(cx, |screen, cx| {
            screen.enter(window, cx);
            screen.open_cursor(window, cx);
            screen.page.clone().expect("the page opened")
        })
    });
    // As Fleet files a sketch: by the artifact's id, with the Job that made it.
    cx.update(|window, cx| {
        page.update(cx, |page, cx| {
            if let super::page::Load::Read(read) = &mut page.page {
                let sketch = read.links.iter_mut().find(|link| link.kind == ipc::FeatureLinkKind::Sketch).expect("a sketch");
                sketch.target = "sketch:Start card with the model picker".into();
                sketch.job = Some("job-start-card".into());
            }
            page.rebuild(cx);
            page.select(Section::Designs, cx);
            page.focus_region(Region::Rows, window, cx);
        })
    });
    cx.run_until_parked();
    let wanted = ArtifactRequest { job: "job-start-card".into(), artifact: "sketch:Start card with the model picker".into() };
    let opened = [crate::titlebar::Asked::OpenJob("job-start-card".into()), crate::titlebar::Asked::Place("jobs")];
    let place = |key: &str, page: &super::page::FeaturePageView| {
        let Body::Rows(rows) = &page.body else { panic!("rows") };
        page.items.iter().position(|item| matches!(item, Item::Row(row) if rows[*row].key == key)).expect("a row")
    };
    let (walk, sketch) = page.read_with(cx, |page, _| (place("walks/start-card", page), place("sketch:Start card with the model picker", page)));
    page.update(cx, |page, cx| page.pick_row(walk, cx));
    cx.simulate_keystrokes("enter");
    assert_eq!(*asked.borrow(), [], "Fleet names no Job for the walk, so nothing opens");
    page.update(cx, |page, cx| page.pick_row(sketch, cx));
    cx.simulate_keystrokes("enter");
    assert_eq!(*asked.borrow(), opened, "the Job opens, then the Jobs screen comes to the front");
    assert_eq!(cx.update(|_, cx| artifact_link::take(cx)), Some(wanted.clone()), "the artifact is left for the Job's view to open");
    asked.borrow_mut().clear();
    cx.update(|window, cx| window.draw(cx).clear(cx));
    let at = cx.update(|window, cx| cx.default_global::<crate::labels::LabelBook>().find(window.window_handle().window_id(), Role::ListBoxOption, "Start card with the model picker").map(|one| one.bounds.center())).expect("drawn");
    cx.simulate_click(at, Modifiers::none());
    assert_eq!(*asked.borrow(), opened, "a press opens it as Enter does");
    assert_eq!(cx.update(|_, cx| artifact_link::take(cx)), Some(wanted));
}
