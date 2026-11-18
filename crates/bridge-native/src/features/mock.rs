//! A feature library for a walk, a preview and the tests, with no Fleet: four areas out of Armada's own
//! draft, two features with a Job on them now and two with a stale section. Later pieces of the feature
//! screens read it too, so what they draw stands on the same features.

use ipc::{Ask, AskKind, AskState, FeatureAreaRow, FeatureAsks, FeatureFile, FeatureJob, FeatureLibrary, FeatureLink, FeatureLinkKind, FeaturePage, FeatureRow, FeatureSection, FeatureSectionKind, FeatureState, Instant, SessionId};

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

/// The library with `more` features added to the first area, for the areas past eight: the mock's own has four there.
pub fn library_with(more: usize) -> FeatureLibrary {
    let mut library = library();
    let running = "running-a-job";
    let titles = ["Quick dispatch", "Snooze a Job", "Move a Job to a project", "Job time buckets", "Handoff to a fresh Job", "Fork from a selection", "Pause on a limit", "Plan downshift"];
    for (n, title) in titles.iter().take(more).enumerate() {
        library.areas[0].features.push(feature(running, &format!("{}-{n}", title.to_lowercase().replace(' ', "-")), title, ""));
    }
    library
}

fn at(day: u32) -> Instant {
    Instant::carried(format!("2026-11-{day:02}T12:00:00Z"))
}

fn link(kind: FeatureLinkKind, target: &str, title: &str, state: &str, day: u32) -> FeatureLink {
    FeatureLink { kind, target: target.into(), title: title.into(), state: state.into(), job: None, via: "write".into(), at: at(day) }
}

fn section(kind: FeatureSectionKind, text: &str, job: &str, day: u32, stale_since: Option<u64>) -> FeatureSection {
    FeatureSection { section: kind, text: text.into(), job: job.into(), written_at: at(day), commit: None, stale_since }
}

/// A feature's page for a walk, a preview and the tests: the row the library holds under `slug`, with what Jobs
/// wrote of it. Every feature shows the same page, the one of Start a Job.
pub fn page(slug: &str) -> FeaturePage {
    let library = library();
    let row = library.areas.iter().flat_map(|area| &area.features).find(|row| row.slug == slug).cloned().unwrap_or_else(|| feature("running-a-job", slug, slug, ""));
    let who = "Add the model picker to the start card";
    FeaturePage {
        project: PROJECT.into(),
        feature: row,
        manifest_id: None,
        paths: vec!["crates/fleet/src/start/".into(), "crates/bridge-native/src/dispatch/".into()],
        entry_points: vec!["crates/fleet/src/start/card.rs:42".into(), "crates/bridge-native/src/dispatch/mod.rs".into()],
        sections: vec![
            section(
                FeatureSectionKind::Test,
                "armada check test start_card\narmada check crates/bridge-native:test dispatch\nThe card proposes a title, a workflow and a model from the request.",
                who,
                13,
                None,
            ),
            section(FeatureSectionKind::Runs, "Fleet reads the request, asks the small model for a title and a workflow, and keeps the proposal on the card until it is started.", who, 13, None),
            section(FeatureSectionKind::Observe, "Watch the proposal in the card's thread: it names the model it picked.", "Keep the Jobs list at its place on refresh", 10, Some(3692)),
            section(FeatureSectionKind::Debt, "The picker keeps the last model per repository in a file Fleet does not read.", who, 14, None),
        ],
        links: vec![
            link(FeatureLinkKind::Job, "job-start-card", who, "running", 14),
            link(FeatureLinkKind::Job, "job-card-title", "Propose the title from the first line", "done", 12),
            link(FeatureLinkKind::Job, "job-card-workflow", "Pick the workflow from the repository", "done", 9),
            link(FeatureLinkKind::Merge, "3720", "", "", 15),
            link(FeatureLinkKind::Merge, "3655", "", "", 8),
            link(FeatureLinkKind::Sketch, "sketches/start-card", "Start card with the model picker", "", 11),
            link(FeatureLinkKind::Page, "documents/start-flow", "How a Job starts", "", 7),
            link(FeatureLinkKind::Walk, "walks/start-card", "Walk of the start card", "", 13),
            link(FeatureLinkKind::Bug, "bug-card-focus", "The card loses focus after a retry", "open", 14),
        ],
        files: vec![
            FeatureFile { path: "crates/fleet/src/start/card.rs".into(), jobs: 6 },
            FeatureFile { path: "crates/bridge-native/src/dispatch/panel.rs".into(), jobs: 4 },
            FeatureFile { path: "crates/ipc/src/start.rs".into(), jobs: 3 },
            FeatureFile { path: "apps/desktop/src/renderer/src/dispatch/StartCard.tsx".into(), jobs: 2 },
        ],
        docs: Vec::new(),
        tickets: Vec::new(),
        floor_plan: None,
    }
}

fn ask(id: &str, kind: AskKind, title: &str, state: AskState, opened: u32, moved: u32) -> Ask {
    Ask {
        ask_id: id.into(),
        kind,
        title: title.into(),
        first_words: title.into(),
        first_row: format!("row-{id}"),
        said_in: SessionId::carried("job-start-card"),
        building: matches!(state, AskState::Building | AskState::Handed).then(|| SessionId::carried("job-start-card")),
        features: vec!["start-a-job".into()],
        state,
        state_at: at(moved),
        opened_at: at(opened),
        said: Vec::new(),
    }
}

/// The ask said in `said_in` (none is outside a Job) and built by `building`.
fn by(mut ask: Ask, said_in: &str, building: Option<&str>) -> Ask {
    ask.said_in = SessionId::carried(said_in);
    ask.building = building.map(SessionId::carried);
    ask
}

/// The ask with his words as he wrote them, which are not its title.
fn worded(mut ask: Ask, words: &str) -> Ask {
    ask.first_words = words.into();
    ask
}

/// The titles of the Jobs the mock's asks name, by id. `job-gone` has none, as a Job Fleet no longer holds.
pub fn titles() -> std::collections::HashMap<String, String> {
    [
        ("job-start-card", "Add the model picker to the start card"),
        ("job-card-title", "Propose the title from the first line"),
        ("job-card-workflow", "Pick the workflow from the repository"),
    ]
    .into_iter()
    .map(|(id, title)| (id.to_string(), title.to_string()))
    .collect()
}

/// The asks filed under a feature: bugs, change requests and ideas in each state.
pub fn asks(slug: &str) -> FeatureAsks {
    FeatureAsks {
        project: PROJECT.into(),
        feature: Some(slug.into()),
        asks: vec![
            by(ask("ask-1", AskKind::Bug, "Native draws no sketch in a page", AskState::Handed, 14, 14), "job-card-title", Some("job-start-card")),
            worded(by(ask("ask-2", AskKind::Bug, "They show as i never voted", AskState::Built, 12, 15), "job-start-card", Some("job-card-workflow")), "they show as i never voted"),
            ask("ask-3", AskKind::Bug, "The model name is cut off in the card", AskState::Backlog, 10, 10),
            ask("ask-4", AskKind::Bug, "Retry started a second Job", AskState::Dropped, 6, 8),
            by(ask("ask-5", AskKind::ChangeRequest, "True voting with sketches", AskState::Building, 13, 14), "job-card-title", Some("job-start-card")),
            by(ask("ask-6", AskKind::Fix, "Keep the request when the card closes", AskState::Backlog, 9, 9), "", None),
            ask("ask-7", AskKind::Idea, "Start from the phone", AskState::Backlog, 11, 11),
            by(ask("ask-8", AskKind::Idea, "Suggest a repository from the request", AskState::Backlog, 5, 5), "job-gone", None),
            worded(
                by(ask("ask-9", AskKind::Bug, "The title is proposed twice on a slow start", AskState::Built, 4, 7), "job-card-title", Some("job-card-workflow")),
                "when the start is slow the card asks the small model for a title two times and the second one replaces what i typed in the first line",
            ),
            by(ask("ask-10", AskKind::Bug, "The workflow picker forgets the last repository", AskState::Built, 3, 6), "job-card-workflow", Some("job-card-title")),
        ],
    }
}
