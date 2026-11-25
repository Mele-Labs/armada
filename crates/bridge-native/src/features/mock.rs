//! A feature library for a walk, a preview and the tests, with no Fleet: four areas out of Armada's own
//! draft, two features with a Job on them now and two with a stale section. Later pieces of the feature
//! screens read it too, so what they draw stands on the same features.

use ipc::{Ask, AskKind, AskState, FeatureAreaRow, FeatureAsks, FeatureDecision, FeatureFile, FeatureJob, FeatureRead, FeatureLibrary, FeatureLink, FeatureLinkKind, FeaturePage, FeatureRow, FeatureSection, FeatureSectionKind, FeatureState, FloorDoor, FloorFile, FloorPlan, FloorRoom, Instant, SessionId};

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
        health: ipc::FeatureHealth { mark: ipc::HealthMark::Good, reasons: Vec::new() },
        retire: None,
        planned: false,
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

/// The projects the mock names, in Fleet's order, as `(id, name)`: the first is the one `library` belongs to.
pub fn projects() -> Vec<(String, String)> {
    [(PROJECT, "Armada"), ("pocket", "Pocket"), ("store", "Store")].into_iter().map(|(id, name)| (id.to_string(), name.to_string())).collect()
}

/// Where the librarian can stand, for a walk and a test to ask for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reading {
    /// It never ran.
    Never,
    /// It read through the last merge, four minutes ago.
    Current,
    /// Three pull requests merged since its last read.
    Behind,
    /// Forty pull requests merged since, more than a line of numbers holds.
    FarBehind,
    /// Behind by three, and a run going: its first step done, its second running, its third not reached.
    Running,
}

/// What a run would do, as Fleet lists it: read what merged, write what changed since he looked, mark the catalog read.
pub fn plan(merges: &str, through: Option<u64>) -> Vec<ipc::LibrarianStep> {
    let step = |id: &str, title: &str, detail: &str, model: bool| ipc::LibrarianStep { id: id.into(), title: title.into(), detail: detail.into(), model };
    vec![
        step("read_merges", "Read what merged", merges, false),
        step("since_you_looked", "Write what changed since you looked", "One model call from tonight's share", true),
        step("read_through", "Mark the catalog read", &through.map_or_else(|| "Nothing merged to read through".to_string(), |n| format!("Read through #{n}")), false),
    ]
}

/// The librarian's state at `at`, with the times as of now so a picture reads the same each day.
pub fn librarian(at: Reading) -> ipc::LibrarianState {
    let ago = |minutes: i64| Instant::carried((chrono::Utc::now() - chrono::Duration::seconds(minutes * 60 + 30)).to_rfc3339());
    match at {
        Reading::Never => ipc::LibrarianState { steps: plan("Every merged pull request Fleet kept", None), ..Default::default() },
        Reading::Current => ipc::LibrarianState { last_run: Some(ago(4)), read_through: Some(3864), merged_since: Vec::new(), steps: plan("Nothing merged since #3864", Some(3864)), running: None },
        Reading::Behind => ipc::LibrarianState { last_run: Some(ago(190)), read_through: Some(3861), merged_since: vec![3862, 3863, 3864], steps: plan("#3862 #3863 #3864, from Fleet's own record", Some(3864)), running: None },
        Reading::FarBehind => ipc::LibrarianState { last_run: Some(ago(60 * 30)), read_through: Some(3820), merged_since: (3821..=3860).collect(), steps: plan("#3821 to #3860, 40 pull requests", Some(3860)), running: None },
        Reading::Running => {
            let run = ago(0);
            let now = |step: &str, state, said: &str| ipc::LibrarianStepNow { step: step.into(), state, said: said.into() };
            ipc::LibrarianState {
                running: Some(ipc::LibrarianRunning {
                    run,
                    steps: vec![now("read_merges", ipc::LibrarianStepState::Done, "Read 3 merges"), now("since_you_looked", ipc::LibrarianStepState::Running, "")],
                }),
                ..librarian(Reading::Behind)
            }
        }
    }
}

/// The library of the project `id`: Armada's own, or a short one for the other projects the mock names.
pub fn library_of(id: &str) -> FeatureLibrary {
    match id {
        "pocket" => {
            let sync = "sync-and-pairing";
            FeatureLibrary {
                project: id.into(),
                areas: vec![area(sync, "Sync and pairing", vec![feature(sync, "pair-a-phone", "Pair a phone", "Scan a code to link a phone"), feature(sync, "offline-queue", "Offline queue", "Sends wait until the Mac is back")])],
                librarian: librarian(Reading::Current),
            }
        }
        "store" => FeatureLibrary { project: id.into(), areas: vec![area("checkout", "Checkout", vec![feature("checkout", "cart", "Cart", "Items, quantities, totals")])], librarian: librarian(Reading::Never) },
        _ => library(),
    }
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
        librarian: librarian(Reading::Behind),
    }
}

/// The library with the Jobs on its features named: `(feature slug, Job id)`, for a walk or a test whose Jobs list holds those Jobs, so
/// a chip on a feature joins the Job's record. No feature has a Job but those named.
pub fn library_on(on: &[(&str, &str)]) -> FeatureLibrary {
    let mut library = library();
    for row in library.areas.iter_mut().flat_map(|area| area.features.iter_mut()) {
        row.jobs_now = on.iter().filter(|(slug, _)| *slug == row.slug).map(|(_, id)| FeatureJob { session_id: SessionId::carried(*id), title: String::new() }).collect();
    }
    library
}

/// The Cockpit's floor on the walk's Jobs: two Working, two Needs you, one Ready to try and one Waiting on ci, on six features.
pub fn floor_library() -> FeatureLibrary {
    library_on(&[
        ("start-a-job", "mock-02"),
        ("job-list-and-live-status", "mock-03"),
        ("job-list-and-live-status", "mock-01"),
        ("terminal-jobs", "mock-14"),
        ("reviews", "mock-04"),
        ("pull-request-panel", "mock-07"),
    ])
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
    let mut row = library.areas.iter().flat_map(|area| &area.features).find(|row| row.slug == slug).cloned().unwrap_or_else(|| feature("running-a-job", slug, slug, ""));
    // Every page is on watch for two reasons, and carries the librarian's proposal to retire the feature.
    row.health = ipc::FeatureHealth {
        mark: ipc::HealthMark::Watch,
        reasons: vec![
            ipc::HealthReason { kind: ipc::HealthReasonKind::StaleSection, fact: "Observe section stale since #3692".into(), target: Some("3692".into()) },
            ipc::HealthReason { kind: ipc::HealthReasonKind::OpenBug, fact: "Open bug: the card loses focus after a retry".into(), target: Some("bug-card-focus".into()) },
        ],
    };
    row.retire = Some(ipc::RetireProposal { why: ipc::RetireWhy::Unused, evidence: "No file of it touched and no Job on it since 12 Jun.".into() });
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
        decisions: vec![
            FeatureDecision { question: "Where does the model picker go on the start card?".into(), choice: "Beside the workflow".into(), note: None, job: "job-start-card".into(), at: at(14) },
            FeatureDecision { question: "Sketch: Start card with the model picker".into(), choice: "Approved".into(), note: Some("Keep the picker one line.".into()), job: "job-start-card".into(), at: at(11) },
        ],
        files: vec![
            FeatureFile { path: "crates/fleet/src/start/card.rs".into(), jobs: 6 },
            FeatureFile { path: "crates/bridge-native/src/dispatch/panel.rs".into(), jobs: 4 },
            FeatureFile { path: "crates/ipc/src/start.rs".into(), jobs: 3 },
            FeatureFile { path: "apps/desktop/src/renderer/src/dispatch/StartCard.tsx".into(), jobs: 2 },
        ],
        docs: vec![
            FeatureRead {
                target: "docs/concepts/start-card.md".into(),
                title: "The start card".into(),
                gist: "The card proposes a title, a workflow and a model from the request.".into(),
                gist_by_job: true,
                job: "job-start-card".into(),
                at: at(14),
                state: String::new(),
                drifted: vec!["StartProposal".into(), "pick_model".into()],
                check: Some(ipc::DocCheck {
                    at: at(15),
                    disagrees: Some(ipc::DocDisagreement {
                        doc_quote: "The card keeps the model he picked last for each repository.".into(),
                        code_at: "crates/fleet/src/start/card.rs:88".into(),
                        code_quote: "takes the machine's Job model on every start; nothing is kept per repository".into(),
                    }),
                    dismissed: false,
                }),
            },
            FeatureRead {
                target: "docs/concepts/dispatch.md".into(),
                title: "Dispatch".into(),
                gist: "One panel starts a Job, a Session or the Autopilot.".into(),
                gist_by_job: false,
                job: "job-card-workflow".into(),
                at: at(3),
                state: String::new(),
                drifted: Vec::new(),
                check: Some(ipc::DocCheck { at: at(15), disagrees: None, dismissed: false }),
            },
            FeatureRead {
                target: "https://notion.example/starting-a-job".into(),
                title: "Starting a Job spec".into(),
                gist: "A request names the repository, and the workflow follows from it.".into(),
                gist_by_job: false,
                job: "job-card-workflow".into(),
                at: at(9),
                state: String::new(),
                drifted: Vec::new(),
                check: None,
            },
            FeatureRead {
                target: "https://figma.example/start-card".into(),
                title: "Start card board".into(),
                gist: "The card with the model picker on one line.".into(),
                gist_by_job: false,
                job: "job-card-title".into(),
                at: at(5),
                state: "gone".into(),
                drifted: Vec::new(),
                check: None,
            },
        ],
        tickets: vec![
            FeatureRead {
                target: "armada-a3f2".into(),
                title: "armada-a3f2 The card loses focus after a retry".into(),
                gist: "Focus returns to the request box when a retry ends.".into(),
                gist_by_job: false,
                job: "job-start-card".into(),
                at: at(14),
                state: String::new(),
                drifted: Vec::new(),
                check: None,
            },
            FeatureRead {
                target: "#3940".into(),
                title: "#3940 Send the request from the keys".into(),
                gist: "Return starts the Job when the card has the keys.".into(),
                gist_by_job: false,
                job: "job-card-title".into(),
                at: at(12),
                state: String::new(),
                drifted: Vec::new(),
                check: None,
            },
        ],
        floor_plan: Some(floor_plan()),
    }
}

/// The code map of the mock's repository: twelve modules sized as Armada's are, the four the feature has files in lit, with their
/// files as Fleet lists them on a read with `files=true`, and four doors, three of them contracts.
fn floor_plan() -> FloorPlan {
    // Each room: its module, folder, kind, whether it is lit, and its folders as (folder, bytes, files).
    let rooms: [(&str, &str, &str, bool, &[(&str, u64, usize)]); 12] = [
        ("desktop", "apps/desktop", "npm", true, &[("src/renderer/src", 7_400_000, 60), ("src/main", 3_900_000, 34), ("src/renderer/src/features", 1_300_000, 12), ("e2e", 2_100_000, 20), ("src/preload", 800_000, 8)]),
        ("bridge-native", "crates/bridge-native", "cargo", true, &[("src/job_view", 4_600_000, 44), ("src", 3_100_000, 36), ("src/features", 1_900_000, 22), ("src/titlebar", 1_500_000, 18), ("src/settings", 1_400_000, 16), ("fixtures", 1_900_000, 12)]),
        ("fleet", "crates/fleet", "cargo", true, &[("src", 4_200_000, 48), ("src/tests", 3_700_000, 40), ("src/feature_library", 1_200_000, 14), ("src/session_host", 2_300_000, 22), ("src/thread_view", 1_900_000, 16)]),
        ("components", "packages/components", "npm", false, &[("src", 4_100_000, 40), ("src/stories", 2_300_000, 24)]),
        ("ipc", "crates/ipc", "cargo", true, &[("src", 2_200_000, 30), ("operations", 900_000, 40)]),
        ("store", "crates/store", "cargo", false, &[("src", 1_300_000, 20), ("src/tests", 500_000, 10), ("migrations", 260_000, 30)]),
        ("config", "crates/config", "cargo", false, &[("src", 900_000, 12), ("presets", 420_000, 8)]),
        ("core-model", "crates/core-model", "cargo", false, &[("src", 520_000, 10), ("domain", 220_000, 6)]),
        ("xtask", "xtask", "cargo", false, &[("src", 627_000, 14)]),
        ("testkit", "crates/testkit", "cargo", false, &[("src", 300_000, 8), ("fixtures", 270_000, 12)]),
        ("tokens", "packages/tokens", "npm", false, &[("src", 300_000, 6), ("build", 137_000, 4)]),
        ("drone", "crates/drone", "cargo", false, &[("src", 260_000, 7)]),
    ];
    // What the feature owns, and what a live Job is writing now.
    let owned = ["apps/desktop/src/renderer/src/features", "crates/bridge-native/src/features", "crates/fleet/src/feature_library", "crates/ipc/src/features.rs", "crates/ipc/src/feature_pages.rs"];
    let changing = ["crates/fleet/src/feature_library/holding.rs", "crates/bridge-native/src/features/floor.rs"];
    let words = ["holding", "floor", "pages", "reading", "filing", "committing", "health", "glossary", "consulting", "pending", "rows", "view", "model", "wire", "tests", "mock", "shots", "keys", "layout", "card"];
    let built = rooms.iter().map(|(module, path, kind, lit, folders)| {
        let ext = if *kind == "npm" { "tsx" } else { "rs" };
        let mut files = Vec::new();
        for (folder, bytes, count) in folders.iter() {
            let base = format!("{path}/{folder}");
            // Sizes fall away as real files do: a few large, a long tail of small ones.
            let shares: Vec<f64> = (0..*count).map(|at| 1. / (at as f64 + 1.).powf(0.9)).collect();
            let whole: f64 = shares.iter().sum();
            let mut tail = (0u64, 0u32);
            for (at, share) in shares.iter().enumerate() {
                let name = if *module == "ipc" && *folder == "src" && at < 2 { ["features", "feature_pages"][at].to_string() } else { format!("{}{}", words[at % words.len()], if at < words.len() { String::new() } else { format!("_{}", at / words.len()) }) };
                let file = format!("{base}/{name}.{ext}");
                let size = (*bytes as f64 * share / whole) as u64;
                let lit = owned.iter().any(|own| file == *own || file.starts_with(&format!("{own}/")) || file == format!("{own}.rs"));
                let now = changing.contains(&file.as_str());
                // Every file the feature owns or a Job is changing stands alone, then the largest; the rest share one entry.
                if lit || now || at < count / 3 {
                    files.push(FloorFile { path: file, bytes: size, lit, changing: now, grouped: 0 });
                } else {
                    tail = (tail.0 + size, tail.1 + 1);
                }
            }
            if tail.1 > 0 {
                files.push(FloorFile { path: format!("{base}/"), bytes: tail.0, lit: false, changing: false, grouped: tail.1 });
            }
        }
        files.sort_by(|a, b| b.bytes.cmp(&a.bytes));
        FloorRoom { module: module.to_string(), path: path.to_string(), kind: kind.to_string(), lit: *lit, manifest_id: None, files }
    });
    let door = |from: &str, to: &str, contract: bool| FloorDoor { from: from.into(), to: to.into(), contract };
    FloorPlan {
        rooms: built.collect(),
        doors: vec![door("bridge-native", "ipc", true), door("fleet", "store", true), door("fleet", "config", true), door("desktop", "components", false)],
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

/// How far back the mock's Since you looked page reaches before he marks it read.
pub const SINCE_DAYS_AGO: i64 = 3;

/// The project's overview as the librarian's first read leaves it: Armada's own modules over the areas of `library`, the
/// busiest features by their merges, and no glossary, which is what Fleet serves until a Job adds a term.
pub fn overview() -> ipc::ProjectOverview {
    let room = |module: &str, path: &str, kind: &str, area: Option<&str>| ipc::MapRoom { module: module.into(), path: path.into(), kind: kind.into(), manifest_id: None, area: area.map(str::to_string) };
    let (running, thread, review) = ("running-a-job", "job-thread-and-terminal", "pull-requests-and-review");
    ipc::ProjectOverview {
        project: PROJECT.into(),
        what_it_is: Some(ipc::ReadmeSentence {
            sentence: "A macOS app that dispatches coding agents as Jobs against git repositories and verifies their work before advancing them.".into(),
            file: "README.md".into(),
        }),
        rooms: vec![
            room("fleet", "crates/fleet", "cargo", Some(running)),
            room("store", "crates/store", "cargo", Some(running)),
            room("config", "crates/config", "cargo", Some(running)),
            room("ipc", "crates/ipc", "cargo", Some(thread)),
            room("bridge-native", "crates/bridge-native", "cargo", Some(thread)),
            room("apps/desktop", "apps/desktop", "npm", Some(thread)),
            room("components", "packages/components", "npm", Some(review)),
            room("plugins/armada", "plugins/armada", "npm", Some(review)),
            room("core-model", "crates/core-model", "cargo", None),
            room("xtask", "xtask", "cargo", None),
        ],
        busiest: vec![
            ipc::BusyFeature { slug: "job-list-and-live-status".into(), title: "Job list and live status".into(), pulls: vec![3863, 3862] },
            ipc::BusyFeature { slug: "thread-and-composer".into(), title: "Thread and composer".into(), pulls: vec![3865] },
            ipc::BusyFeature { slug: "pause-resume-stop-snooze".into(), title: "Pause, resume, stop, snooze".into(), pulls: vec![3864] },
            ipc::BusyFeature { slug: "reviews".into(), title: "Reviews".into(), pulls: vec![3879] },
        ],
        glossary: Vec::new(),
    }
}

/// What moved since `at`: the feature waiting on him whatever the time, and each merge or new feature by how long ago it was.
pub fn since(at: chrono::DateTime<chrono::Utc>) -> ipc::SinceYouLooked {
    let now = chrono::Utc::now();
    // `(hours ago, tag, slug, title, what, pulls)`: Needs you first, then Changed by newest merge, then New.
    let moved: [(i64, ipc::LookedTag, &str, &str, &str, &[u64]); 4] = [
        (0, ipc::LookedTag::NeedsYou, "model-and-context", "Model and context", "Its Test section is older than the last merge that changed its files.", &[]),
        (5, ipc::LookedTag::Changed, "thread-and-composer", "Thread and composer", "Ask head on two lines, Last line in full.", &[3865]),
        (30, ipc::LookedTag::Changed, "job-list-and-live-status", "Job list and live status", "On it now counts Jobs holding a slot. Two panels with rounded corners.", &[3863, 3862]),
        (50, ipc::LookedTag::New, "agent-notes-on-a-diff", "Agent notes on a diff", "Pending, named by a Feature line.", &[]),
    ];
    let features: Vec<ipc::LookedFeature> = moved
        .into_iter()
        .filter(|(hours, tag, ..)| *tag == ipc::LookedTag::NeedsYou || now - chrono::Duration::hours(*hours) > at)
        .map(|(_, tag, slug, title, what, pulls)| ipc::LookedFeature { tag, slug: slug.into(), title: title.into(), what: what.into(), pulls: pulls.to_vec() })
        .collect();
    // The librarian's paragraph covers the merges; with none since, there is nothing for it to say.
    let paragraph = if features.iter().any(|one| one.tag != ipc::LookedTag::NeedsYou) { "The Features screen moved to two panels and its ask cards wrap. A Job's ask head now takes two lines. One section waits on your Confirm." } else { "" };
    ipc::SinceYouLooked { project: PROJECT.into(), paragraph: paragraph.into(), features }
}

/// Who is on what, as Fleet serves it: the owner first and the others by login, the features with most people first and then by
/// title. Two features have two people on them, the owner holds Jobs where the others hold pull requests, and one pull request
/// is a draft.
pub fn who() -> ipc::WhosOnWhat {
    let repository = "Mele-Labs/armada";
    let pull = |number: u64, title: &str, draft: bool| ipc::WhoPull { number, repository: repository.into(), title: title.into(), draft };
    let cell = |login: &str, pulls: Vec<ipc::WhoPull>, jobs: &[&str]| ipc::WhoCell { login: login.into(), pulls, jobs: jobs.iter().map(|id| SessionId::carried(*id)).collect() };
    let feature = |slug: &str, title: &str, cells: Vec<ipc::WhoCell>| ipc::WhoFeature { slug: slug.into(), title: title.into(), cells };
    let person = |login: &str, me: bool| ipc::WhoPerson { login: login.into(), me };
    ipc::WhosOnWhat {
        project: PROJECT.into(),
        people: vec![person("you", true), person("dana", false), person("marco", false), person("priya", false)],
        features: vec![
            feature(
                "pull-request-panel",
                "Pull request panel",
                vec![cell("dana", vec![pull(3871, "Pull request panel in two columns", false)], &[]), cell("priya", vec![pull(3868, "Pull request panel remembers its folds", false)], &[])],
            ),
            feature(
                "start-a-job",
                "Start a Job",
                vec![cell("you", Vec::new(), &["job-start-card"]), cell("priya", vec![pull(3869, "Start card remembers the last workflow", false)], &[])],
            ),
            feature("job-list-and-live-status", "Job list and live status", vec![cell("you", Vec::new(), &["job-list-scroll"])]),
            feature("reviews", "Reviews", vec![cell("marco", vec![pull(3879, "Review list keeps the last read", false), pull(3875, "Review notes on a diff", true)], &[])]),
        ],
    }
}

/// The project the start panel's mock belongs to: one with no feature of its own.
pub const BARE: &str = "ledger-api";

/// A library with no area and no feature, for the start panel.
pub fn bare() -> FeatureLibrary {
    FeatureLibrary { project: BARE.into(), areas: Vec::new(), librarian: librarian(Reading::Never) }
}

/// What a bare project starts from: five modules out of its build files, and a README, a docs folder and an architecture file.
pub fn start(project: &str) -> ipc::LibraryStart {
    let module = |name: &str, path: &str| ipc::StartModule { name: name.into(), slug: super::curate::slug_of(name), path: path.into(), held: false };
    ipc::LibraryStart {
        project: project.into(),
        modules: vec![module("api", "crates/api"), module("billing", "crates/billing"), module("ledger-core", "crates/ledger-core"), module("web", "web"), module("worker", "crates/worker")],
        docs: vec!["README.md".into(), "docs/".into(), "ARCHITECTURE.md".into()],
    }
}

/// The library after Draft as Pending on `start`: the Modules area Fleet keeps to confirm into, and each module pending.
pub fn drafted(start: &ipc::LibraryStart) -> FeatureLibrary {
    let pending = |one: &ipc::StartModule| FeatureRow { state: FeatureState::Pending, summary: String::new(), ..feature(super::curate::PENDING_AREA, &one.slug, &one.name, "") };
    FeatureLibrary {
        project: start.project.clone(),
        areas: vec![area("modules", "Modules", Vec::new()), area(super::curate::PENDING_AREA, "pending", start.modules.iter().map(pending).collect())],
        librarian: librarian(Reading::Never),
    }
}
