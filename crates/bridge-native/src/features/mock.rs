//! A feature library for a walk, a preview and the tests, with no Fleet: four areas out of Armada's own
//! draft, two features with a Job on them now and two with a stale section. Later pieces of the feature
//! screens read it too, so what they draw stands on the same features.

use ipc::{FeatureAreaRow, FeatureJob, FeatureLibrary, FeatureRow, FeatureState, SessionId};

/// The project the mock's library belongs to.
pub const PROJECT: &str = "armada";

fn feature(area: &str, slug: &str, title: &str, summary: &str) -> FeatureRow {
    FeatureRow {
        id: format!("feature-{slug}"),
        slug: slug.into(),
        area: area.into(),
        title: title.into(),
        summary: summary.into(),
        state: FeatureState::Confirmed,
        jobs_now: Vec::new(),
        stale_since: None,
    }
}

fn on(mut row: FeatureRow, jobs: &[(&str, &str)]) -> FeatureRow {
    row.jobs_now = jobs.iter().map(|(id, title)| FeatureJob { session_id: SessionId::carried(*id), title: (*title).into() }).collect();
    row
}

fn stale(mut row: FeatureRow, since: u64) -> FeatureRow {
    row.stale_since = Some(since);
    row
}

fn area(slug: &str, title: &str, features: Vec<FeatureRow>) -> FeatureAreaRow {
    FeatureAreaRow { slug: slug.into(), title: title.into(), features }
}

pub fn library() -> FeatureLibrary {
    let running = "running-a-job";
    let thread = "job-thread-and-terminal";
    let review = "pull-requests-and-review";
    let mut pending = feature("pending", "agent-notes-on-a-diff", "Agent notes on a diff", "A review run by an agent leaves notes on lines");
    pending.state = FeatureState::Pending;
    FeatureLibrary {
        project: PROJECT.into(),
        areas: vec![
            area(
                running,
                "Running a Job",
                vec![
                    on(feature(running, "start-a-job", "Start a Job", "Start card proposes title, workflow, model"), &[("job-start-card", "Add the model picker to the start card")]),
                    on(
                        feature(running, "job-list-and-live-status", "Job list and live status", "What runs, what is wrong, what it waits on"),
                        &[("job-list-scroll", "Keep the Jobs list at its place on refresh"), ("job-needs-chip", "Show what a Job waits on in its row")],
                    ),
                    stale(feature(running, "pause-resume-stop-snooze", "Pause, resume, stop, snooze", "Control a running Job"), 3712),
                    stale(feature(running, "model-and-context", "Model and context", "Context ring, plan limit, downshift"), 3655),
                ],
            ),
            area(
                thread,
                "Job thread and terminal",
                vec![
                    feature(thread, "thread-and-composer", "Thread and composer", "Messages, quick replies, edits"),
                    feature(thread, "terminal-jobs", "Terminal Jobs", "A Job in a shared terminal"),
                    feature(thread, "sessions-without-a-job", "Sessions without a Job", "Raw agent sessions, take over"),
                ],
            ),
            area(
                review,
                "Pull requests and review",
                vec![
                    feature(review, "pull-request-panel", "Pull request panel", "A Job's PR and its description"),
                    feature(review, "reviews", "Reviews", "PRs waiting on you, notes on lines"),
                ],
            ),
            area("pending", "pending", vec![pending]),
        ],
    }
}
