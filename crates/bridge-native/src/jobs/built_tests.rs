//! By state, a Ready Job stands where the running preview has its build, and an idle Job something else keeps open is
//! Waiting on, each row saying why.

use super::*;

/// mock-02 Ready to try with no item of its own, mock-09 idle with a stale line: the two the tests move.
fn two() -> Vec<SessionRecord> {
    let mut all: Vec<SessionRecord> = crate::mock::jobs("jobs-many").expect("the fixture decodes").into_iter().filter(|job| ["mock-02", "mock-09"].contains(&job.id.as_str())).collect();
    for job in &mut all {
        job.attachments.retain(|one| !["started_by", "pr", "replaced_by"].contains(&one.kind.as_str()));
        job.running.clear();
        job.waiting_for.clear();
        job.snoozed_until = None;
        job.mac = None;
        job.activity = None;
        job.so_far = Some("Running gh api".into());
    }
    let ready = all.iter_mut().find(|job| job.id.as_str() == "mock-02").expect("mock-02");
    ready.mark = Some(JobMark { state: JobState::Ready, lamp: JobLamp::Ready, group: JobGroup::Ready });
    let idle = all.iter_mut().find(|job| job.id.as_str() == "mock-09").expect("mock-09");
    idle.mark = Some(JobMark { state: JobState::Idle, lamp: JobLamp::Idle, group: JobGroup::Done });
    all
}

/// Fleet's offers, oldest first: each `(job, state, in_build, why)`.
fn offers(rows: &[(&str, &str, bool, Option<&str>)]) -> ipc::PreviewOffers {
    let rows: Vec<String> = rows
        .iter()
        .enumerate()
        .map(|(at, (job, state, in_build, why))| {
            let why = why.map(|why| format!(r#","why":"{why}""#)).unwrap_or_default();
            format!(r#"{{"branch":"armada/{job}-{at}","tip":"t{at}","session_id":"{job}","title":"{job}","about":"x","offered_at":"2026-11-27T10:0{at}:00Z","state":"{state}","in_build":{in_build}{why}}}"#)
        })
        .collect();
    ipc::decode::<ipc::PreviewOffers>("offers", format!(r#"{{"offers":[{}]}}"#, rows.join(",")).as_bytes()).expect("offers")
}

fn by_state(all: &[SessionRecord], offers: Option<&ipc::PreviewOffers>) -> Vec<Line> {
    lines_led(all, &[], Scope::Active, "", &[], &Names::default(), None, &[], Group::State, false, &leads::Leads::default(), &built::Builds::of(offers))
}

/// The group `job` stands under, and its row's second line.
fn placed(lines: &[Line], job: &str) -> (String, String) {
    let mut head = String::new();
    for line in lines {
        match line {
            Line::Group(group) => head = group.name.to_string(),
            Line::Job(row) if row.id.as_ref() == job => return (head, row.now.as_deref().unwrap_or_default().to_string()),
            _ => {}
        }
    }
    panic!("{job} is not listed")
}

#[test]
fn a_ready_job_stands_by_where_the_running_preview_has_its_build_and_its_row_says_so() {
    let all = two();
    for (state, in_build, why, group, now) in [
        ("ready", true, None, "In your preview", "In your preview: waits on your Looks good"),
        ("ready", false, None, "Not in your preview yet", "Not in your preview yet: Refresh preview brings it in"),
        ("checking", false, None, "Not in your preview yet", "Checking that the build will work"),
        ("failed", false, Some("E0308 in jobs.rs"), "Not in your preview yet", "Off the preview list: E0308 in jobs.rs"),
    ] {
        let lines = by_state(&all, Some(&offers(&[("mock-02", state, in_build, why)])));
        assert_eq!(placed(&lines, "mock-02"), (group.to_string(), now.to_string()), "{state} in_build={in_build}");
    }
}

#[test]
fn a_ready_job_that_offered_no_build_reads_as_it_always_has() {
    // A Job of a repository with no preview offers nothing: it stays Ready to try, with its own line.
    let lines = by_state(&two(), Some(&offers(&[("mock-09", "ready", true, None)])));
    assert_eq!(placed(&lines, "mock-02"), ("Ready to try".to_string(), "Running gh api".to_string()));
    let lines = by_state(&two(), None);
    assert_eq!(placed(&lines, "mock-02").0, "Ready to try");
}

#[test]
fn a_jobs_newest_offer_decides_where_it_stands() {
    let lines = by_state(&two(), Some(&offers(&[("mock-02", "ready", true, None), ("mock-02", "checking", false, None)])));
    assert_eq!(placed(&lines, "mock-02"), ("Not in your preview yet".to_string(), "Checking that the build will work".to_string()));
}

#[test]
fn an_idle_job_something_else_keeps_open_is_waiting_on_and_its_row_names_what() {
    for (held, now) in [
        (ipc::HeldOpenBy::PullRequest { number: 812, on: ipc::PullRequestWait::Queue }, "Pull request #812 in the merge queue"),
        (ipc::HeldOpenBy::PullRequest { number: 812, on: ipc::PullRequestWait::AutoMerge }, "Pull request #812 merges once its Checks pass"),
        (ipc::HeldOpenBy::Watch { name: "compile on main".into() }, "Waits on its watch compile on main"),
        (ipc::HeldOpenBy::Job { name: "Deep links".into() }, "Waits on its Job Deep links"),
    ] {
        let mut all = two();
        let idle = all.iter_mut().find(|job| job.id.as_str() == "mock-09").expect("mock-09");
        idle.held_open_by = vec![held];
        // Fleet marks a Job held open Held, filed with the quiet ones.
        idle.mark = Some(JobMark { state: JobState::Held, lamp: JobLamp::Done, group: JobGroup::Done });
        assert_eq!(placed(&by_state(&all, None), "mock-09"), ("Waiting on".to_string(), now.to_string()));
    }
}

#[test]
fn an_idle_job_nothing_keeps_open_stays_idle_with_its_own_line() {
    assert_eq!(placed(&by_state(&two(), None), "mock-09"), ("Idle".to_string(), "Running gh api".to_string()));
}

#[test]
fn a_job_with_work_still_running_says_that_work_though_something_keeps_it_open() {
    let mut all = two();
    let idle = all.iter_mut().find(|job| job.id.as_str() == "mock-09").expect("mock-09");
    idle.held_open_by = vec![ipc::HeldOpenBy::Watch { name: "compile on main".into() }];
    idle.mark = Some(JobMark { state: JobState::Held, lamp: JobLamp::Done, group: JobGroup::Done });
    idle.running = vec![ipc::RunningWork { kind: ipc::RunningKind::BackgroundTask, what: "cargo build".into(), since: ipc::Instant::carried("2026-11-27T10:00:00Z") }];
    let (group, now) = placed(&by_state(&all, None), "mock-09");
    assert_ne!(group, "Waiting on", "work going is not waiting");
    assert_eq!(now, "cargo build");
}
