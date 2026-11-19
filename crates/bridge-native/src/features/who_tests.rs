use std::cell::RefCell;
use std::rc::Rc;

use gpui::{Role, TestAppContext, VisualTestContext};
use ipc::WhosOnWhat;

use super::project_model::ProjectPage;
use super::project_page::{ProjectPageEvent, ProjectPageView, Target};
use super::who_model::{self, Stop, WhoModel, WhoView};
use super::{looked, mock, who_kept, FeaturesScreen};

fn ready(cx: &mut TestAppContext) {
    cx.update(|cx| {
        gpui_component::init(cx);
        crate::look::init(cx);
        crate::keymap::install(&crate::keymap::Rebinding::default(), cx);
        crate::titlebar::bind(cx);
        looked::forget_all(cx);
        who_kept::forget_all(cx);
    });
}

fn page(cx: &mut TestAppContext) -> (gpui::Entity<ProjectPageView>, &mut VisualTestContext) {
    ready(cx);
    let (page, cx) = cx.add_window_view(|_, cx| ProjectPageView::mock(ProjectPage::Who, cx));
    cx.run_until_parked();
    (page, cx)
}

fn drawn(cx: &mut VisualTestContext, role: Role, name: &str) -> bool {
    cx.update(|window, cx| window.draw(cx).clear(cx));
    cx.update(|window, cx| cx.default_global::<crate::labels::LabelBook>().find(window.window_handle().window_id(), role, name).is_some())
}

fn jobs() -> Vec<(String, String)> {
    vec![("job-start-card".into(), "Add the model picker to the start card".into()), ("job-list-scroll".into(), "Keep the Jobs list at its place on refresh".into())]
}

fn model() -> WhoModel {
    who_model::who(&mock::who(), &jobs())
}

fn said(event: &ProjectPageEvent) -> String {
    match event {
        ProjectPageEvent::Open(slug) => format!("open {slug}"),
        ProjectPageEvent::Pull { repository, number } => format!("pull {repository}#{number}"),
        ProjectPageEvent::Job(id) => format!("job {id}"),
        ProjectPageEvent::Escape => "escape".into(),
    }
}

/// What the page says to whoever holds it, in order.
fn listen(page: &gpui::Entity<ProjectPageView>, cx: &mut VisualTestContext) -> Rc<RefCell<Vec<String>>> {
    let heard: Rc<RefCell<Vec<String>>> = Rc::default();
    let keep = heard.clone();
    cx.update(|_, cx| cx.subscribe(page, move |_, event: &ProjectPageEvent, _| keep.borrow_mut().push(said(event))).detach());
    heard
}

/// The screen on the mock, with Who's on what open beside the list.
fn screen_on_who(cx: &mut TestAppContext) -> (gpui::Entity<FeaturesScreen>, &mut VisualTestContext) {
    ready(cx);
    let (screen, cx) = cx.add_window_view(|window, cx| FeaturesScreen::mock(window, cx));
    cx.update(|window, cx| screen.update(cx, |screen, cx| screen.enter(window, cx)));
    cx.run_until_parked();
    cx.simulate_keystrokes("k");
    cx.run_until_parked();
    (screen, cx)
}

#[test]
fn one_answer_makes_the_grid_the_feature_blocks_and_the_person_cards() {
    let model = model();
    assert_eq!(model.people.iter().map(|one| one.name.as_str()).collect::<Vec<_>>(), ["You", "Dana", "Marco", "Priya"], "the owner is You and comes first, then Fleet's order");
    assert_eq!(model.features.iter().map(|one| one.title.as_str()).collect::<Vec<_>>(), ["Pull request panel", "Start a Job", "Job list and live status", "Reviews"], "the features keep Fleet's order");
    assert_eq!(model.cards.iter().map(|card| model.people[card.person].name.as_str()).collect::<Vec<_>>(), ["Dana", "Marco", "Priya", "You"], "the cards put You last");
    let you = model.cards.last().expect("a card for You");
    assert_eq!(you.held.iter().map(|held| held.title.as_str()).collect::<Vec<_>>(), ["Start a Job", "Job list and live status"], "a person's card lists the features in Fleet's order");
    assert!(you.held.iter().flat_map(|held| &held.marks).all(|mark| matches!(mark, who_model::Mark::Job { .. })), "the owner's cell holds Jobs, not pull requests");
    let reviews = &model.features[3].cells[0].marks;
    assert_eq!(reviews.len(), 2);
    assert!(matches!(&reviews[0], who_model::Mark::Pull(pull) if pull.number == 3879 && !pull.draft), "newest first, as Fleet served them");
    assert!(matches!(&reviews[1], who_model::Mark::Pull(pull) if pull.number == 3875 && pull.draft), "one pull request is a draft");
    assert!(matches!(&model.features[1].cells[0].marks[0], who_model::Mark::Job { id, title: Some(title) } if id == "job-start-card" && title == "Add the model picker to the start card"), "a Job is named by the library");
}

#[test]
fn a_feature_with_two_people_is_an_overlap_and_so_is_the_card_of_each_of_them() {
    let model = model();
    assert_eq!(model.features.iter().map(|one| one.overlap).collect::<Vec<_>>(), [true, true, false, false]);
    assert_eq!(model.overlap_said(&model.features[0]).as_deref(), Some("Dana and Priya"));
    assert_eq!(model.overlap_said(&model.features[1]).as_deref(), Some("You and Priya"), "the head names its people, the owner as You");
    assert_eq!(model.overlap_said(&model.features[3]), None);
    assert_eq!(model.cards.iter().map(|card| card.overlap).collect::<Vec<_>>(), [true, false, true, true], "Marco shares nothing, and the rest share a feature with someone");
    assert_eq!(who_model::names_said(&["You", "Priya", "Dana"]), "You, Priya and Dana");
}

#[test]
fn each_layout_stops_the_keys_on_every_feature_name_pull_request_and_job() {
    let model = model();
    let numbers = |stops: &[Stop]| stops.iter().filter_map(|stop| if let Stop::Pull { number, .. } = stop { Some(*number) } else { None }).collect::<Vec<_>>();
    let grid = model.stops(WhoView::Grid);
    assert_eq!(numbers(&grid), [3871, 3868, 3869, 3879, 3875], "the grid reads a row at a time, across the people");
    assert_eq!(grid.iter().filter(|stop| matches!(stop, Stop::Job(_))).count(), 2);
    assert_eq!(grid.first(), Some(&Stop::Feature("pull-request-panel".into())));
    assert_eq!(model.stops(WhoView::ByFeature), grid, "by feature reads the same cells in the same order");
    let by_person = model.stops(WhoView::ByPerson);
    assert_eq!(numbers(&by_person), [3871, 3879, 3875, 3868, 3869], "by person reads a card at a time, You last");
    assert_eq!(by_person.len(), grid.len() + 2, "a feature two people are on is a stop in each of their cards");
}

#[gpui::test]
fn the_grid_is_the_first_layout_and_holds_the_pull_requests_and_the_jobs(cx: &mut TestAppContext) {
    let (page, cx) = page(cx);
    assert_eq!(page.read_with(cx, |page, _| page.view()), WhoView::Grid);
    assert!(drawn(cx, Role::Region, "Who's on what") && drawn(cx, Role::Heading, "Who's on what"));
    assert!(drawn(cx, Role::Button, "Start a Job") && drawn(cx, Role::Button, "Pull request panel"), "a feature name opens its page");
    assert!(drawn(cx, Role::Button, "Pull request #3869, Start card remembers the last workflow"), "a pull request number opens the pull request");
    assert!(drawn(cx, Role::Button, "Draft pull request #3875, Review notes on a diff"), "a draft reads as one");
    assert!(drawn(cx, Role::Button, "Job"), "the owner's cell holds a Job chip");
    assert!(drawn(cx, Role::RadioGroup, "Layout") && drawn(cx, Role::RadioButton, "Grid") && drawn(cx, Role::RadioButton, "By feature") && drawn(cx, Role::RadioButton, "By person"));
}

#[gpui::test]
fn by_feature_and_by_person_draw_the_same_answer(cx: &mut TestAppContext) {
    let (page, cx) = page(cx);
    page.update(cx, |page, cx| page.set_view(WhoView::ByFeature, cx));
    assert!(drawn(cx, Role::Region, "Who's on what, by feature") && drawn(cx, Role::Group, "Start a Job"));
    assert!(drawn(cx, Role::Button, "Pull request #3868, Pull request panel remembers its folds") && drawn(cx, Role::Image, "Priya"), "a person's line has the initial and the title");
    page.update(cx, |page, cx| page.set_view(WhoView::ByPerson, cx));
    assert!(drawn(cx, Role::Region, "Who's on what, by person") && drawn(cx, Role::Group, "You") && drawn(cx, Role::Group, "Priya") && drawn(cx, Role::Group, "Marco"));
    assert!(drawn(cx, Role::Button, "Draft pull request #3875, Review notes on a diff"));
}

#[gpui::test]
fn the_key_cycles_the_three_layouts_and_round(cx: &mut TestAppContext) {
    let (page, cx) = page(cx);
    cx.update(|window, cx| page.update(cx, |page, cx| page.focus_body(window, cx)));
    cx.simulate_keystrokes("v");
    assert_eq!(page.read_with(cx, |page, _| page.view()), WhoView::ByFeature);
    cx.simulate_keystrokes("v");
    assert_eq!(page.read_with(cx, |page, _| page.view()), WhoView::ByPerson);
    cx.simulate_keystrokes("v");
    assert_eq!(page.read_with(cx, |page, _| page.view()), WhoView::Grid, "after the last, the first");
}

#[gpui::test]
fn the_key_works_from_the_list_while_the_page_is_beside_it_and_leaves_another_page_alone(cx: &mut TestAppContext) {
    ready(cx);
    let (screen, cx) = cx.add_window_view(|window, cx| FeaturesScreen::mock(window, cx));
    cx.update(|window, cx| screen.update(cx, |screen, cx| screen.enter(window, cx)));
    cx.run_until_parked();
    cx.simulate_keystrokes("v");
    assert!(screen.read_with(cx, |screen, cx| screen.project_open(cx)).is_none(), "a feature's page is open, which the key leaves alone");
    cx.simulate_keystrokes("k");
    cx.run_until_parked();
    let who = screen.read_with(cx, |screen, _| screen.project_view()).expect("Who's on what is open");
    cx.simulate_keystrokes("v");
    assert_eq!(who.read_with(cx, |page, _| page.view()), WhoView::ByFeature, "the list's key cycles the page beside it");
}

#[gpui::test]
fn the_layout_is_kept_for_the_next_page_and_for_the_next_start(cx: &mut TestAppContext) {
    let (page, cx) = page(cx);
    page.update(cx, |page, cx| page.set_view(WhoView::ByPerson, cx));
    assert_eq!(cx.update(|_, cx| who_kept::get(cx)), Some(WhoView::ByPerson));
    let (again, _) = cx.add_window_view(|_, cx| ProjectPageView::mock(ProjectPage::Who, cx));
    assert_eq!(again.read_with(cx, |page, _| page.view()), WhoView::ByPerson, "a page made after opens in the layout he left");

    // The next start reads the file the choice was written to.
    let dir = std::env::temp_dir().join(format!("armada-who-kept-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a folder to keep it in");
    let file = dir.join("native-features-who-view");
    cx.update(|_, cx| {
        who_kept::forget_all(cx);
        who_kept::persist_at(file.clone(), cx);
        who_kept::set(WhoView::ByFeature, cx);
    });
    cx.run_until_parked();
    assert_eq!(std::fs::read_to_string(&file).as_deref().ok(), Some("By feature"));
    cx.update(|_, cx| {
        who_kept::forget_all(cx);
        assert_eq!(who_kept::get(cx), None);
        who_kept::persist_at(file.clone(), cx);
        assert_eq!(who_kept::get(cx), Some(WhoView::ByFeature), "read back at the next start");
    });
}

/// A page whose first draw is the state a test is about, since a label drawn once stays in the book.
fn page_with(cx: &mut TestAppContext, set: impl FnOnce(&mut ProjectPageView, &mut gpui::Context<ProjectPageView>) + 'static) -> (gpui::Entity<ProjectPageView>, &mut VisualTestContext) {
    ready(cx);
    let (page, cx) = cx.add_window_view(|_, cx| {
        let mut page = ProjectPageView::mock(ProjectPage::Who, cx);
        set(&mut page, cx);
        page
    });
    cx.run_until_parked();
    (page, cx)
}

#[gpui::test]
fn an_answer_with_nothing_in_it_says_so_in_one_line_and_draws_nothing_else(cx: &mut TestAppContext) {
    let (page, cx) = page_with(cx, |page, cx| page.fill_who(WhosOnWhat { project: mock::PROJECT.into(), people: Vec::new(), features: Vec::new() }, cx));
    assert!(drawn(cx, Role::Label, who_model::EMPTY));
    assert_eq!(who_model::EMPTY, "No open pull requests touch a feature.");
    assert!(drawn(cx, Role::Heading, "Who's on what"), "the page keeps its title");
    assert!(!drawn(cx, Role::RadioGroup, "Layout") && !drawn(cx, Role::Region, "Who's on what"), "no control and no grid");
    assert!(page.read_with(cx, |page, _| page.targets().is_empty()));
}

#[gpui::test]
fn a_refused_read_says_what_fleet_said(cx: &mut TestAppContext) {
    let (_page, cx) = page_with(cx, |page, cx| page.refuse_who("The project is not known", cx));
    assert!(drawn(cx, Role::Label, "The project is not known"));
    assert!(!drawn(cx, Role::RadioGroup, "Layout"));
}

#[gpui::test]
fn j_and_enter_reach_a_feature_a_pull_request_and_a_job_and_open_what_each_names(cx: &mut TestAppContext) {
    let (page, cx) = page(cx);
    let heard = listen(&page, cx);
    cx.update(|window, cx| page.update(cx, |page, cx| page.focus_body(window, cx)));
    cx.simulate_keystrokes("j enter");
    assert_eq!(*heard.borrow(), ["open pull-request-panel"], "the first stop is a feature's name");
    cx.simulate_keystrokes("j enter");
    assert_eq!(heard.borrow().last().map(String::as_str), Some("pull Mele-Labs/armada#3871"), "then its pull request");
    for view in WhoView::ALL {
        page.update(cx, |page, cx| page.set_view(view, cx));
        let stops = page.read_with(cx, |page, _| page.targets().len());
        assert_eq!(stops, model().stops(view).len(), "{view:?}");
        heard.borrow_mut().clear();
        for at in 0..stops {
            page.update(cx, |page, cx| page.pick(at, cx));
        }
        let expected: Vec<String> = model()
            .stops(view)
            .iter()
            .map(|stop| match stop {
                Stop::Feature(slug) => format!("open {slug}"),
                Stop::Pull { repository, number } => format!("pull {repository}#{number}"),
                Stop::Job(id) => format!("job {id}"),
            })
            .collect();
        assert_eq!(*heard.borrow(), expected, "a press on each stop says what it opens in {view:?}");
        assert!(heard.borrow().iter().any(|line| line == "job job-start-card"), "a Job chip opens its Job");
        // The page checks as it draws that it draws each stop, in the order the keys step through.
        assert!(drawn(cx, Role::RadioGroup, "Layout"));
    }
}

#[gpui::test]
fn a_job_chip_opens_its_job_on_the_jobs_screen_and_says_what_the_job_is(cx: &mut TestAppContext) {
    ready(cx);
    let asked: Rc<RefCell<Vec<crate::titlebar::Asked>>> = Rc::default();
    let heard = asked.clone();
    cx.update(|cx| cx.set_global(crate::titlebar::AskedSink(Rc::new(move |one: &crate::titlebar::Asked, _: &mut gpui::App| heard.borrow_mut().push(one.clone())))));
    let (screen, cx) = screen_on_who(cx);
    assert!(drawn(cx, Role::Button, "Job: Add the model picker to the start card"), "the library names the Job the chip stands for");
    let page = screen.read_with(cx, |screen, _| screen.project_view()).expect("Who's on what is open");
    let at = page.read_with(cx, |page, _| page.targets().iter().position(|target| *target == Target::Who(Stop::Job("job-list-scroll".into())))).expect("a stop on the Job");
    page.update(cx, |page, cx| page.pick(at, cx));
    cx.run_until_parked();
    assert_eq!(*asked.borrow(), [crate::titlebar::Asked::OpenJob("job-list-scroll".into()), crate::titlebar::Asked::Place("jobs")]);
}

#[gpui::test]
fn a_feature_name_opens_its_page_in_place_of_who_is_on_what(cx: &mut TestAppContext) {
    let (screen, cx) = screen_on_who(cx);
    let page = screen.read_with(cx, |screen, _| screen.project_view()).expect("Who's on what is open");
    let at = page.read_with(cx, |page, _| page.targets().iter().position(|target| *target == Target::Who(Stop::Feature("reviews".into())))).expect("a stop on the feature");
    page.update(cx, |page, cx| page.pick(at, cx));
    cx.run_until_parked();
    assert_eq!(screen.read_with(cx, |screen, cx| (screen.project_open(cx), screen.opened().map(|row| row.slug.clone()))), (None, Some("reviews".into())));
}
