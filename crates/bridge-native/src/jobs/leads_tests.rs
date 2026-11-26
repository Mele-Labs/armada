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
    leads::Leads { libraries: vec![library] }
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
