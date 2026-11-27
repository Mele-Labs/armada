//! A board for a walk, a preview and the tests, with no Fleet: three projects, the first with two repositories, and what each
//! block would read from a Fleet that answered. Features and the code map are the Features tab's own mock, so the two tabs stand
//! on the same features.

use std::collections::BTreeMap;

use ipc::{
    AppRun, AppRunCheckout, AppRunDraft, AppRunDraftProcess, AppRunList, AppRunOpensIn, AppRunProcess, AppRunState, Instant, IssueState, ManifestId, MatchedBy, MatchedFeature, MetricAnswer, MetricCredential, MetricPoint,
    MetricReading, MetricReadings, MetricRow, MetricState, MetricValue, RankedIssue, RankedIssues, RankedPull, RunPlace, ServerLink, TrackedIssue,
};

use super::board::{Checkout, IssuesRead};
use super::model::{Button, Item, Press, Project, Repository};
use crate::look::token;

const ARMADA: &str = "armada-main";
const POCKET: &str = "pocket-web";

/// The project `id`, with the repositories the mock gives it: the first has two, the others one.
pub fn project(id: &str, name: &str) -> Project {
    let repositories = if id == crate::features::mock::PROJECT {
        vec![Repository { id: ARMADA.into(), name: "armada".into() }]
    } else {
        vec![Repository { id: POCKET.into(), name: name.to_lowercase() }]
    };
    Project { id: id.to_string(), name: name.to_string(), repositories }
}

/// The projects the mock Fleet of the Settings tab holds, so the page has a project to edit for each chip: the same ids and members
/// the board draws.
pub fn projects() -> Vec<ipc::Project> {
    crate::features::mock::projects()
        .into_iter()
        .map(|(id, name)| {
            let shown = project(&id, &name);
            let members = shown.repositories.iter().map(|one| ipc::ProjectMember { manifest_id: ManifestId::carried(one.id.clone()), folder: None }).collect();
            ipc::Project { id, name, members, ..ipc::Project::default() }
        })
        .collect()
}

/// Jobs waiting on him and a card of the Operator's.
pub fn needs() -> Vec<Item> {
    let row = |key: &str, glyph: &'static str, hue, said: &str, title: &str, meta: &str, tag: &str, press: Press| {
        Item::new(key, glyph, hue, said, title).meta(meta).tag(tag, hue).press(press)
    };
    vec![
        row("job-1", "bell-ring", token::YOU, "Needs you", "Pricing tiers: pick a take", "Pick a take", "Job, 4m", Press::Job("mock-01".into())),
        row("job-2", "bell-ring", token::YOU, "Needs you", "Slot GC fix", "Ready to try", "Job, 12m", Press::Job("mock-02".into())),
        row("card-1", "bot", token::OPERATOR, "Operator card", "Land #4001 or hold?", "Operator", "Card", Press::Operator),
        row("job-3", "bell-ring", token::YOU, "Needs you", "Pocket offline", "Approve the plan", "Job, 1h", Press::Job("mock-03".into())),
    ]
}

/// The Checks and Commands of a manifest, each with a Run button.
pub fn checks() -> Vec<Item> {
    let run = |name: &str, confirm: bool| Some(Button { glyph: "play", name: format!("Run {name}"), press: Press::Command { manifest: ARMADA.into(), name: name.into(), confirm } });
    let check = |name: &str| Item::new(format!("check-{name}"), "list-checks", token::SKETCH_GREY, "Check", name).tag("Check", token::SKETCH_GREY).buttons(vec![run(name, false)]);
    let command = |name: &str, confirm: bool| Item::new(format!("command-{name}"), "terminal", token::SKETCH_BLUE, "Command", name).tag("Command", token::SKETCH_BLUE).buttons(vec![run(name, confirm)]);
    // `restart` is flagged `confirm: true` in the mock's armada.yml, so a press on it asks first.
    vec![check("test"), check("verify-foundations"), check("format"), command("preview", false), command("restart", true)]
}

/// A Job's worktree the Start menu lists.
pub fn worktrees() -> Vec<Checkout> {
    vec![
        Checkout::Job { id: "mock-02".into(), manifest: ARMADA.into(), label: "armada/slot-gc-fix".into() },
        Checkout::Job { id: "mock-03".into(), manifest: ARMADA.into(), label: "armada/pocket-offline".into() },
    ]
}

fn process(name: &str, state: AppRunState, port: Option<u16>, server: Option<&str>) -> AppRunProcess {
    let links = match (state, port) {
        (AppRunState::Ready, Some(port)) => vec![ServerLink { url: format!("http://127.0.0.1:{port}"), name: None }],
        _ => Vec::new(),
    };
    AppRunProcess { name: name.into(), state, port, pinned_port: None, links, server_id: server.map(str::to_string), ended: None }
}

/// What `/app_runs` reads for the project's repositories: `dev` with two processes up and one stopped, and for the second
/// repository none, so the draft is offered.
pub fn runs(project: &Project, on_job: bool) -> BTreeMap<String, Result<AppRunList, String>> {
    let mut all = BTreeMap::new();
    for repository in &project.repositories {
        let list = if repository.id == ARMADA {
            let up = if on_job { AppRunState::Starting } else { AppRunState::Ready };
            vec![AppRun {
                name: "dev".into(),
                label: "Fleet and Bridge".into(),
                opens_in: AppRunOpensIn::Browser,
                manifest_id: ManifestId::carried(ARMADA),
                checkout: AppRunCheckout::Main { path: "/repo/armada".into() },
                processes: vec![
                    process("fleet", up, Some(7420), Some("server-1")),
                    process("pocket", AppRunState::Ready, Some(5173), Some("server-2")),
                    process("docs site", AppRunState::Exited, None, Some("server-3")),
                ],
                last_used: Some(Instant::carried("2026-11-23T09:00:00Z")),
                links: Vec::new(),
            }]
        } else {
            Vec::new()
        };
        all.insert(repository.id.clone(), Ok(AppRunList { runs: list }));
    }
    all
}

/// A run started or stopped on the mock: the processes of the run take the state the press asks for.
pub fn flip(runs: &mut BTreeMap<String, Result<AppRunList, String>>, manifest: &str, name: &str, start: bool) {
    if let Some(Ok(list)) = runs.get_mut(manifest) {
        for run in list.runs.iter_mut().filter(|run| run.name == name) {
            for process in &mut run.processes {
                process.state = if start { AppRunState::Ready } else { AppRunState::Exited };
                process.links = match (start, process.port) {
                    (true, Some(port)) => vec![ServerLink { url: format!("http://127.0.0.1:{port}"), name: None }],
                    _ => Vec::new(),
                };
            }
        }
    }
}

/// The last of a server's output.
pub fn log() -> Vec<String> {
    ["fleet listening on 127.0.0.1:7420", "store opened, 14 migrations", "events socket accepted", "repository read, 3 manifests", "ready"].into_iter().map(str::to_string).collect()
}

/// The run Fleet proposes for a repository that declares none.
pub fn draft() -> AppRunDraft {
    let process = |name: &str, command: &str| AppRunDraftProcess { name: name.into(), dir: None, command: command.into(), port_env: None };
    AppRunDraft { root: "/repo/pocket".into(), place: RunPlace::ThisMac, processes: vec![process("web", "pnpm dev --port ${port}"), process("api", "node server.js")], links: Vec::new() }
}

fn issue(reference: &str, title: &str, assignees: &[&str]) -> TrackedIssue {
    TrackedIssue {
        reference: reference.into(),
        title: title.into(),
        url: format!("https://example.com/issues/{reference}"),
        labels: Vec::new(),
        assignees: assignees.iter().map(|one| one.to_string()).collect(),
        updated_at: Instant::carried("2026-11-22T09:00:00Z"),
        excerpt: String::new(),
        linked_pull_request: false,
        author: None,
        state: IssueState::Open,
        fields: Vec::new(),
        holders: Vec::new(),
    }
}

fn feature(slug: &str, title: &str, by: MatchedBy, jobs: u32) -> MatchedFeature {
    MatchedFeature { slug: slug.into(), title: title.into(), by: vec![by], jobs }
}

/// The might-lists as the librarian ranked them: its own order, with a why on each row.
pub fn ranked_by_librarian() -> RankedIssues {
    let mut ranked = issues().ranked;
    ranked.might_pick_up.reverse();
    let whys = ["Offline sends are the Pocket gap you hit this week", "Two features you work on most wait on it"];
    for (row, why) in ranked.might_pick_up.iter_mut().zip(whys) {
        row.why = Some(why.to_string());
    }
    for row in &mut ranked.might_be_interested {
        row.why = Some("Touches the library you work in".to_string());
    }
    ranked.by_librarian = Some(Instant::carried("2026-11-24T01:00:00Z"));
    ranked
}

/// One issue assigned to him, two ranked ones nobody holds (the first matched two features, the second none), and one pull request
/// of someone else.
pub fn issues() -> IssuesRead {
    let mine = vec![issue("armada-a3f2", "Slot GC leaks", &["you"])];
    let ranked = RankedIssues {
        project: crate::features::mock::PROJECT.into(),
        days: 30,
        fetched_at: None,
        might_pick_up: vec![
            RankedIssue {
                manifest_id: Some(ManifestId::carried(ARMADA)),
                issue: issue("armada-9efp", "Library sync", &[]),
                features: vec![feature("feature-library", "Feature library", MatchedBy::PullRequestFiles, 4), feature("project-board", "Project board", MatchedBy::Label, 2)],
                jobs: 6,
                why: None,
            },
            RankedIssue { manifest_id: Some(ManifestId::carried(ARMADA)), issue: issue("armada-71cc", "Pocket offline", &[]), features: Vec::new(), jobs: 0, why: None },
        ],
        might_be_interested: vec![RankedPull {
            manifest_id: ManifestId::carried(ARMADA),
            repository: "mele-labs/armada".into(),
            number: 4021,
            title: "Rank issues by feature".into(),
            author: "ada".into(),
            draft: false,
            features: vec![feature("feature-library", "Feature library", MatchedBy::Files, 4)],
            jobs: 4,
            why: None,
        }],
        by_librarian: None,
    };
    IssuesRead { mine, ranked }
}

// ---- metrics ---------------------------------------------------------------------------------------------------------

fn at(hour: u32, minute: u32) -> Instant {
    Instant::carried(format!("2026-11-24T{hour:02}:{minute:02}:00Z"))
}

fn row(label: &str, value: f64, unit: Option<&str>, when: Option<Instant>) -> MetricRow {
    MetricRow { label: label.into(), value: MetricValue::Number(value), unit: unit.map(str::to_string), at: when, link: None }
}

/// What `/metrics` reads for the project's recipes, one in each state: errors as a series the answer carries in time, a build time
/// that has only what Fleet kept and a picture, a deploy count that waits for this Mac's token, and a queue that no longer reads.
pub fn readings(project: &str) -> MetricReadings {
    if project != crate::features::mock::PROJECT {
        return MetricReadings { project: project.to_string(), readings: Vec::new() };
    }
    let errors = MetricReading {
        id: "errors-24h".into(),
        title: "Errors, last day".into(),
        state: MetricState::Read,
        answer: Some(MetricAnswer {
            rows: vec![row("Now", 14., Some("errors"), Some(at(9, 0))), row("3 hours ago", 22., Some("errors"), Some(at(6, 0))), row("6 hours ago", 9., Some("errors"), Some(at(3, 0))), row("9 hours ago", 17., Some("errors"), Some(at(0, 0)))],
            picture: None,
        }),
        read_at: Some(at(9, 0)),
        kept: Vec::new(),
    };
    let build = MetricReading {
        id: "build-time".into(),
        title: "Build time".into(),
        state: MetricState::Read,
        answer: Some(MetricAnswer { rows: vec![row("Main", 412., Some("s"), None), row("Last pull request", 388., Some("s"), None)], picture: Some("https://example.com/panels/build-time.png".into()) }),
        read_at: Some(at(9, 1)),
        kept: [(7, 0, 380.), (7, 55, 388.), (8, 20, 401.), (8, 45, 396.), (9, 0, 412.), (9, 1, 412.)].into_iter().map(|(hour, minute, value)| MetricPoint { at: at(hour, minute), value }).collect(),
    };
    let deploys = MetricReading {
        id: "deploys-week".into(),
        title: "Deploys this week".into(),
        state: MetricState::NoToken { credential: MetricCredential { name: "vercel".into(), how: "A Vercel token with read access to deployments".into(), url: Some("https://vercel.com/account/tokens".into()) } },
        answer: None,
        read_at: None,
        kept: Vec::new(),
    };
    let queue = MetricReading {
        id: "queue-depth".into(),
        title: "Queue depth".into(),
        state: MetricState::Broken { why: "HTTP 401 from queue.example.com/depth".into() },
        answer: Some(MetricAnswer { rows: vec![row("Waiting", 23., Some("jobs"), None), row("Oldest", 14., Some("min"), None)], picture: None }),
        read_at: Some(at(8, 12)),
        kept: Vec::new(),
    };
    MetricReadings { project: project.to_string(), readings: vec![errors, build, deploys, queue] }
}

/// A token saved on the mock: the recipe reads from then on.
pub fn token_saved(read: &mut MetricReadings, id: &str) {
    if let Some(one) = read.readings.iter_mut().find(|one| one.id == id) {
        one.state = MetricState::Read;
        one.answer = Some(MetricAnswer { rows: vec![row("Deploys", 31., None, None)], picture: None });
        one.read_at = Some(at(9, 2));
    }
}
