//! The Jobs list by feature and by area.

use super::*;

/// Three live Jobs with nothing linking them: mock-01, mock-02 and mock-09.
fn three() -> Vec<SessionRecord> {
    let mut all: Vec<SessionRecord> = crate::mock::jobs("jobs-many").expect("the fixture decodes").into_iter().filter(|job| ["mock-01", "mock-02", "mock-09"].contains(&job.id.as_str())).collect();
    for job in &mut all {
        job.attachments.retain(|one| one.kind != "started_by");
        job.snoozed_until = None;
        job.mac = None;
    }
    all
}

/// mock-01 writes to two features of one area, mock-02 to one, mock-09 to none.
fn leads() -> leads::Leads {
    let library = crate::features::mock::library_on(&[("start-a-job", "mock-01"), ("job-list-and-live-status", "mock-01"), ("job-list-and-live-status", "mock-02")]);
    leads::Leads { libraries: vec![library], subs: Vec::new() }
}

fn grouped(group: Group) -> Vec<Line> {
    lines_led(&three(), &[], Scope::Active, "", &[], &Names::default(), None, &[], group, false, &leads())
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
fn the_view_menu_picks_by_feature_and_the_list_draws_the_libraries_it_holds(cx: &mut gpui::TestAppContext) {
    let list = cx.new(|cx| JobsList::mock(&three(), cx));
    list.update(cx, |list, cx| {
        list.pick_view("feature", cx);
        assert_eq!(list.group, Group::Feature);
        list.hold_libraries(leads().libraries, cx);
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
    let mut held = leads();
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
