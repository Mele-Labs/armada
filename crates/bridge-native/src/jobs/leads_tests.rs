//! The Jobs list by feature and by area.

use super::*;

/// mock-01 writes to two features of one area, mock-02 to one, mock-09 to none.
const ON: &[(&str, &str)] = &[("start-a-job", "mock-01"), ("job-list-and-live-status", "mock-01"), ("job-list-and-live-status", "mock-02")];

/// What `job` writes to of [`ON`], as Fleet carries it on the Job's row: each feature with its area, read from the mock library.
fn writes(job: &str) -> Vec<ipc::SessionFeature> {
    let library = crate::features::mock::library();
    let mut out = Vec::new();
    for (area_place, area) in library.areas.iter().enumerate() {
        for (place, feature) in area.features.iter().enumerate().filter(|(_, feature)| ON.contains(&(feature.slug.as_str(), job))) {
            out.push(ipc::SessionFeature {
                project: library.project.clone(),
                slug: feature.slug.clone(),
                title: feature.title.clone(),
                area: area.slug.clone(),
                area_title: area.title.clone(),
                area_place: area_place as u32,
                place: place as u32,
            });
        }
    }
    out
}

/// Three live Jobs with nothing linking them, mock-01, mock-02 and mock-09, each carrying what it writes to.
fn three() -> Vec<SessionRecord> {
    let mut all: Vec<SessionRecord> = crate::mock::jobs("jobs-many").expect("the fixture decodes").into_iter().filter(|job| ["mock-01", "mock-02", "mock-09"].contains(&job.id.as_str())).collect();
    for job in &mut all {
        job.attachments.retain(|one| one.kind != "started_by");
        job.snoozed_until = None;
        job.mac = None;
        job.features = writes(job.id.as_str());
    }
    all
}

/// The library a feature's board reads, with the same Jobs on its features.
fn leads() -> leads::Leads {
    leads::Leads { libraries: vec![crate::features::mock::library_on(ON)], subs: Vec::new() }
}

/// The list by `group`, with no library held: the headings come from the rows.
fn grouped(group: Group) -> Vec<Line> {
    lines_led(&three(), &[], Scope::Active, "", &[], &Names::default(), None, &[], group, false, &leads::Leads::default())
}

/// Each heading, and each Job by id with its muted line, in the order drawn.
fn drawn(lines: &[Line]) -> Vec<String> {
    lines
        .iter()
        .filter_map(|line| match line {
            Line::Group(head) => Some(format!("# {}", head.name)),
            Line::Job(row) => Some(match &row.aside {
                Some(aside) => format!("{} ({aside})", row.id),
                None => row.id.to_string(),
            }),
            Line::Asleep(_) => None,
        })
        .collect()
}

#[test]
fn by_feature_a_job_stands_under_each_feature_it_writes_to_and_names_the_others() {
    assert_eq!(
        drawn(&grouped(Group::Feature)),
        [
            "# Start a Job",
            "mock-01 (Also under Job list and live status)",
            "# Job list and live status",
            "mock-01 (Also under Start a Job)",
            "mock-02",
            "# No feature",
            "mock-09",
        ]
    );
}

#[test]
fn by_feature_the_keys_pass_a_jobs_second_row_and_a_feature_heading_draws_no_count() {
    let lines = grouped(Group::Feature);
    let again: Vec<&str> = lines.iter().filter_map(|line| match line { Line::Job(row) if row.again => Some(row.id.as_ref()), _ => None }).collect();
    assert_eq!(again, ["mock-01"], "only the second row of mock-01 is a repeat");
    assert!(lines.iter().any(|line| matches!(line, Line::Group(head) if head.name == "Start a Job" && matches!(head.lead, Some(leads::Lead::Feature { .. })))));
}

#[test]
fn by_area_a_job_stands_once_under_its_area() {
    assert_eq!(drawn(&grouped(Group::Area)), ["# Running a Job", "mock-01", "mock-02", "# No feature", "mock-09"]);
}

#[gpui::test]
fn the_view_menu_picks_by_feature_and_the_headings_draw_at_once_from_the_rows_with_no_library_held(cx: &mut gpui::TestAppContext) {
    let list = cx.new(|cx| JobsList::mock(&three(), cx));
    list.update(cx, |list, cx| {
        list.pick_view("feature", cx);
        assert_eq!(list.group, Group::Feature);
        assert!(list.leads.libraries.is_empty(), "no library is read before the headings draw");
        let headings: Vec<String> = list.lines.iter().filter_map(|line| match line { Line::Group(head) => Some(head.name.to_string()), _ => None }).collect();
        assert_eq!(headings, ["Start a Job", "Job list and live status", "No feature"]);
    });
}

fn sub(id: &str, name: &str, jobs: &[&str], features: &[&str]) -> ipc::SubProject {
    ipc::SubProject {
        id: id.into(),
        project: crate::features::mock::PROJECT.into(),
        name: name.into(),
        jobs: jobs.iter().map(|id| ipc::SessionId::carried(*id)).collect(),
        features: features.iter().map(|slug| slug.to_string()).collect(),
        started_at: ipc::Instant::carried("2026-11-26T12:00:00Z"),
    }
}

#[test]
fn a_sub_project_heads_its_jobs_on_top_with_a_joined_one_and_each_still_stands_in_its_group() {
    let mut held = leads::Leads::default();
    // mock-01 he picked; mock-02 writes to the sub project's feature and joins it; one with no Job listed is not drawn.
    held.subs = vec![sub("s1", "Librarian rollout", &["mock-01"], &["job-list-and-live-status"]), sub("s2", "Gone quiet", &["mock-15"], &[])];
    let lines = lines_led(&three(), &[], Scope::Active, "", &[], &Names::default(), None, &[], Group::State, false, &held);
    let drawn = drawn(&lines);
    assert_eq!(drawn[..3], ["# Librarian rollout", "mock-01", "mock-02 (Joined: it writes to Job list and live status)"]);
    assert!(matches!(&lines[0], Line::Group(head) if head.lead == Some(leads::Lead::SubProject { id: "s1".into() })));
    assert!(lines[1..3].iter().all(|line| matches!(line, Line::Job(row) if row.depth == 1 && !row.again)), "nested under it, and the keys land there");
    assert!(!drawn.contains(&"# Gone quiet".to_string()));
    for id in ["mock-01", "mock-02"] {
        let below = lines[3..].iter().filter(|line| matches!(line, Line::Job(row) if row.id == id && row.again && row.depth == 0)).count();
        assert_eq!(below, 1, "{id} still stands in its own group below");
    }
}

/// A window that draws the list as the app does.
struct Hosted(Entity<JobsList>);

impl gpui::Render for Hosted {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        crate::labels::label_root(div().size_full().child(self.0.clone()))
    }
}

#[gpui::test]
fn start_a_sub_project_sends_the_picked_ids_in_his_order_and_heads_them_on_top(cx: &mut gpui::TestAppContext) {
    cx.update(crate::init_app);
    let list = cx.new(|cx| JobsList::mock(&three(), cx));
    let (_, cx) = cx.add_window_view(|_, _| Hosted(list.clone()));
    cx.run_until_parked();
    list.update(cx, |list, cx| {
        list.toggle_selected("mock-02", cx);
        list.toggle_selected("mock-01", cx);
    });
    cx.update(|window, cx| list.update(cx, |list, cx| list.open_name_sheet(list.reached_by("mock-01"), None, window, cx)));
    cx.run_until_parked();
    let sheet = list.read_with(cx, |list, _| list.name_sheet()).expect("the sheet opens");
    let picked: Vec<String> = sheet.read_with(cx, |sheet, _| sheet.ids().iter().map(|id| id.to_string()).collect());
    assert_eq!(picked, ["mock-02", "mock-01"], "the selection, in the order he picked it");
    let body = list.read_with(cx, |list, _| list.sub_project_body(&["mock-02".into(), "mock-01".into()], "  Librarian rollout "));
    let sent: Vec<&str> = body.jobs.iter().map(|id| id.as_str()).collect();
    assert_eq!((sent, body.name.as_str()), (vec!["mock-02", "mock-01"], "Librarian rollout"));
    cx.update(|window, cx| {
        sheet.update(cx, |sheet, cx| {
            sheet.type_name("Librarian rollout", window, cx);
            sheet.start(window, cx);
        })
    });
    cx.run_until_parked();
    list.read_with(cx, |list, _| {
        assert!(list.name_sheet().is_none(), "the sheet goes once it is kept");
        assert!(list.selected().is_empty(), "and the selection is let go");
        let jobs: Vec<&str> = list.leads.subs[0].jobs.iter().map(|id| id.as_str()).collect();
        assert_eq!(jobs, ["mock-02", "mock-01"]);
        assert!(matches!(&list.lines[0], Line::Group(head) if head.name == "Librarian rollout"));
    });
}

/// The Jobs screen on the three Jobs, drawn once.
fn screen(cx: &mut gpui::TestAppContext) -> (Entity<JobsList>, &mut gpui::VisualTestContext) {
    cx.update(crate::init_app);
    let list = cx.new(|cx| JobsList::mock(&three(), cx));
    let held = list.clone();
    let (_, cx) = cx.add_window_view(move |window, cx| JobsScreen::new(held, None, window, cx));
    cx.run_until_parked();
    (list, cx)
}

fn drawn_named(cx: &mut gpui::VisualTestContext, role: Role, name: &str) -> bool {
    cx.run_until_parked();
    cx.update(|window, cx| {
        window.draw(cx).clear(cx);
        cx.default_global::<crate::labels::LabelBook>().find(window.window_handle().window_id(), role, name).is_some()
    })
}

fn enter_on(list: &Entity<JobsList>, stop: &str, cx: &mut gpui::VisualTestContext) {
    let act = Act { id: "open_focused", slot: 0, digit: None };
    let stop: SharedString = stop.to_string().into();
    cx.update(|window, cx| {
        list.update(cx, |list, cx| {
            assert!(list.at_of(&stop).is_some(), "the keys can land on the heading");
            list.focused = Some(stop);
            list.on_act(&act, window, cx);
        })
    });
    cx.run_until_parked();
}

#[gpui::test]
fn enter_on_a_feature_heading_opens_its_board_where_a_job_opens_and_open_opens_the_job(cx: &mut gpui::TestAppContext) {
    let (list, cx) = screen(cx);
    list.update(cx, |list, cx| {
        list.pick_view("feature", cx);
        list.hold_libraries(leads().libraries, cx);
    });
    enter_on(&list, "lead:feature:armada/job-list-and-live-status", cx);
    let board = list.read_with(cx, |list, _| list.board()).expect("the board stands in the right column");
    assert!(matches!(board.read_with(cx, |board, _| board.lead().clone()), leads::Lead::Feature { slug, .. } if slug == "job-list-and-live-status"));
    assert!(drawn_named(cx, Role::Region, "Job list and live status"));
    assert!(drawn_named(cx, Role::Button, "Open Fix the merge line freshness chip"), "a row for each Job in it, with Open");
    assert!(drawn_named(cx, Role::Button, "Open Native Bridge foundation"));
    assert!(!drawn_named(cx, Role::Button, "Ungroup"), "a feature's board has no Ungroup");
    // The keys are on the board: Enter presses the first row's Open.
    cx.simulate_keystrokes("enter");
    cx.run_until_parked();
    list.read_with(cx, |list, _| {
        assert_eq!(list.opened_id().as_deref(), Some("mock-01"));
        assert!(list.board().is_none(), "the Job stands in the board's place");
    });
}

/// Where the heading named so is drawn, once the screen is drawn.
fn heading_at(cx: &mut gpui::VisualTestContext, name: &str) -> gpui::Point<gpui::Pixels> {
    cx.run_until_parked();
    cx.update(|window, cx| {
        window.draw(cx).clear(cx);
        cx.default_global::<crate::labels::LabelBook>().find(window.window_handle().window_id(), Role::Heading, name).map(|one| one.bounds.center())
    })
    .unwrap_or_else(|| panic!("the heading {name} is drawn"))
}

#[gpui::test]
fn a_press_on_a_feature_heading_opens_its_board_where_a_job_opens(cx: &mut gpui::TestAppContext) {
    let (list, cx) = screen(cx);
    list.update(cx, |list, cx| {
        list.pick_view("feature", cx);
        list.hold_libraries(leads().libraries, cx);
    });
    let at = heading_at(cx, "Job list and live status");
    cx.simulate_click(at, gpui::Modifiers::default());
    cx.run_until_parked();
    let board = list.read_with(cx, |list, _| list.board()).expect("the board stands in the right column");
    assert!(matches!(board.read_with(cx, |board, _| board.lead().clone()), leads::Lead::Feature { slug, .. } if slug == "job-list-and-live-status"));
    assert!(drawn_named(cx, Role::Region, "Job list and live status"));
}

#[gpui::test]
fn a_press_on_a_sub_project_heading_opens_its_board_where_a_job_opens(cx: &mut gpui::TestAppContext) {
    let (list, cx) = screen(cx);
    list.update(cx, |list, cx| {
        list.hold_libraries(leads().libraries, cx);
        list.hold_subs(vec![sub("s1", "Librarian rollout", &["mock-01"], &["job-list-and-live-status"])], cx);
    });
    let at = heading_at(cx, "Librarian rollout");
    cx.simulate_click(at, gpui::Modifiers::default());
    cx.run_until_parked();
    let board = list.read_with(cx, |list, _| list.board()).expect("the board stands in the right column");
    assert!(matches!(board.read_with(cx, |board, _| board.lead().clone()), leads::Lead::SubProject { id } if id == "s1"));
    assert!(drawn_named(cx, Role::Button, "Ungroup"));
}

#[gpui::test]
fn a_press_on_a_feature_heading_in_the_cockpit_opens_its_board_over_the_page(cx: &mut gpui::TestAppContext) {
    cx.update(crate::init_app);
    let (screen, cx) = cx.add_window_view(|window, cx| {
        let list = cx.new(|cx| JobsList::mock(&three(), cx));
        let page = cx.new(|cx| crate::operator::page::OperatorPageView::mock(window, cx));
        let mut screen = JobsScreen::new(list, None, window, cx);
        screen.set_home(page, cx);
        screen
    });
    let list = screen.read_with(cx, |screen, _| screen.list.clone());
    list.update(cx, |list, cx| {
        list.pick_view("feature", cx);
        list.hold_libraries(leads().libraries, cx);
    });
    cx.run_until_parked();
    let at = heading_at(cx, "Job list and live status");
    cx.simulate_click(at, gpui::Modifiers::default());
    cx.run_until_parked();
    assert!(list.read_with(cx, |list, _| list.board()).is_some(), "the board stands in the right column");
    assert!(drawn_named(cx, Role::Region, "Job list and live status"));
}

#[gpui::test]
fn a_press_on_a_feature_heading_in_the_window_opens_its_board_in_the_right_column(cx: &mut gpui::TestAppContext) {
    cx.update(crate::init_app);
    let list = cx.new(|cx| JobsList::mock(&three(), cx));
    let held = list.clone();
    let (_window, cx) = cx.add_window_view(move |window, cx| {
        let jobs = cx.new(|cx| JobsScreen::new(held, None, window, cx));
        crate::operator::screen::OperatorScreen::mock_cockpit_holding(jobs, leads().libraries.remove(0), window, cx)
    });
    cx.run_until_parked();
    list.update(cx, |list, cx| {
        list.pick_view("feature", cx);
        list.hold_libraries(leads().libraries, cx);
    });
    let at = heading_at(cx, "Job list and live status");
    cx.simulate_click(at, gpui::Modifiers::default());
    cx.run_until_parked();
    assert!(list.read_with(cx, |list, _| list.board()).is_some(), "the board stands in the right column");
    assert!(drawn_named(cx, Role::Region, "Job list and live status"));
    let region = cx.update(|window, cx| cx.default_global::<crate::labels::LabelBook>().find(window.window_handle().window_id(), Role::Region, "Job list and live status").map(|one| one.bounds)).expect("drawn");
    assert!(region.size.width > gpui::px(100.) && region.origin.x > at.x, "the board is drawn to the right of the list: {region:?}");
}

#[gpui::test]
fn a_press_on_a_feature_heading_after_the_pointer_rested_on_a_row_opens_its_board(cx: &mut gpui::TestAppContext) {
    let (list, cx) = screen(cx);
    list.update(cx, |list, cx| {
        list.pick_view("feature", cx);
        list.hold_libraries(leads().libraries, cx);
    });
    let row = cx.update(|window, cx| cx.default_global::<crate::labels::LabelBook>().find(window.window_handle().window_id(), Role::Row, "Fix the merge line freshness chip").map(|one| one.bounds.center())).expect("the row is drawn");
    cx.simulate_mouse_move(row, None, gpui::Modifiers::default());
    cx.executor().advance_clock(std::time::Duration::from_secs(2));
    cx.run_until_parked();
    assert!(list.read_with(cx, |list, _| list.card.is_some()), "the pointer rested: its card is up");
    let at = heading_at(cx, "Job list and live status");
    cx.simulate_mouse_move(at, None, gpui::Modifiers::default());
    cx.run_until_parked();
    cx.simulate_click(at, gpui::Modifiers::default());
    cx.run_until_parked();
    assert!(list.read_with(cx, |list, _| list.board()).is_some(), "the board stands in the right column");
}

const FEATURE: &str = "Job list and live status";

/// The Jobs screen under the window's root, which clears what the last frame drew, so a label that left is gone.
struct Rooted(Entity<JobsScreen>);

impl gpui::Render for Rooted {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        crate::labels::label_root(div().size_full().child(self.0.clone()))
    }
}

/// The list by feature on its libraries, with the feature's board opened from its heading.
fn board_open(cx: &mut gpui::TestAppContext) -> (Entity<JobsList>, &mut gpui::VisualTestContext) {
    cx.update(crate::init_app);
    let list = cx.new(|cx| JobsList::mock(&three(), cx));
    let held = list.clone();
    let (_, cx) = cx.add_window_view(move |window, cx| {
        let screen = cx.new(|cx| JobsScreen::new(held, None, window, cx));
        Rooted(screen)
    });
    cx.run_until_parked();
    list.update(cx, |list, cx| {
        list.pick_view("feature", cx);
        list.hold_libraries(leads().libraries, cx);
    });
    let at = heading_at(cx, FEATURE);
    cx.simulate_click(at, gpui::Modifiers::default());
    cx.run_until_parked();
    assert!(drawn_named(cx, Role::Button, "Open Fix the merge line freshness chip"), "the board is filled in");
    (list, cx)
}

#[gpui::test]
fn a_failed_read_of_the_features_keeps_the_library_and_the_board_stays_filled(cx: &mut gpui::TestAppContext) {
    let (list, cx) = board_open(cx);
    list.update(cx, |list, cx| list.hold_read(Vec::new(), cx));
    assert!(drawn_named(cx, Role::Region, FEATURE), "the board still stands");
    assert!(drawn_named(cx, Role::Button, "Open Fix the merge line freshness chip"), "with its rows");
    assert!(drawn_named(cx, Role::Heading, FEATURE), "and the heading stays in the list");
}

#[gpui::test]
fn a_board_whose_feature_is_not_held_yet_draws_its_title_and_loading_rows(cx: &mut gpui::TestAppContext) {
    let (list, cx) = board_open(cx);
    list.update(cx, |list, cx| list.hold_libraries(Vec::new(), cx));
    assert!(drawn_named(cx, Role::Region, FEATURE), "the board keeps its title");
    assert!(drawn_named(cx, Role::Image, "Loading"), "and draws placeholder rows");
    assert!(!drawn_named(cx, Role::Button, "Open Fix the merge line freshness chip"));
    list.update(cx, |list, cx| list.hold_libraries(leads().libraries, cx));
    assert!(drawn_named(cx, Role::Button, "Open Fix the merge line freshness chip"), "the read lands and the board fills in");
    assert!(!drawn_named(cx, Role::Image, "Loading"));
}

#[gpui::test]
fn a_board_whose_feature_is_gone_from_an_answered_library_says_so_in_one_line(cx: &mut gpui::TestAppContext) {
    let (list, cx) = board_open(cx);
    let mut library = leads().libraries.remove(0);
    for area in &mut library.areas {
        area.features.retain(|feature| feature.slug != "job-list-and-live-status");
    }
    list.update(cx, |list, cx| list.hold_libraries(vec![library], cx));
    assert!(drawn_named(cx, Role::Region, FEATURE));
    assert!(drawn_named(cx, Role::Label, "No feature Job list and live status in this project now"));
    assert!(!drawn_named(cx, Role::Image, "Loading"));
}

#[gpui::test]
fn ungroup_on_a_sub_projects_board_takes_its_heading_away(cx: &mut gpui::TestAppContext) {
    let (list, cx) = screen(cx);
    list.update(cx, |list, cx| {
        list.hold_libraries(leads().libraries, cx);
        list.hold_subs(vec![sub("s1", "Librarian rollout", &["mock-01"], &["job-list-and-live-status"])], cx);
    });
    enter_on(&list, "lead:sub:s1", cx);
    assert!(drawn_named(cx, Role::Region, "Librarian rollout"));
    assert!(drawn_named(cx, Role::Button, "Ungroup"));
    // Two rows, mock-01 he picked and mock-02 joined, then Ungroup.
    cx.simulate_keystrokes("down down enter");
    cx.run_until_parked();
    list.read_with(cx, |list, _| {
        assert!(list.leads.subs.is_empty() && list.board().is_none());
        assert!(!list.lines.iter().any(|line| matches!(line, Line::Group(head) if head.name == "Librarian rollout")));
    });
}

#[test]
fn a_boards_row_says_what_its_job_waits_on_from_its_record() {
    let mut job = three().remove(0);
    job.waiting_for.clear();
    job.held_open_by = vec![ipc::HeldOpenBy::PullRequest { number: 812, on: ipc::PullRequestWait::Queue }];
    assert_eq!(leads::holds(&job), "Pull request #812 in the merge queue");
    job.held_open_by = vec![ipc::HeldOpenBy::Watch { name: "the ask box reaching main".into() }];
    assert_eq!(leads::holds(&job), "Waits on its watch the ask box reaching main");
}
