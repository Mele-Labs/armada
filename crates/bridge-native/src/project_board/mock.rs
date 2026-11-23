//! A board for a walk, a preview and the tests, with no Fleet: three projects, the first with two repositories, and what each
//! block would read from a Fleet that answered. Features and the code map are the Features tab's own mock, so the two tabs stand
//! on the same features.

use std::collections::BTreeMap;

use ipc::{AppRun, AppRunCheckout, AppRunDraft, AppRunDraftProcess, AppRunList, AppRunOpensIn, AppRunProcess, AppRunState, Instant, IssueState, ManifestId, RunPlace, ServerLink, TrackedIssue};

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
    let run = |name: &str| Some(Button { glyph: "play", name: format!("Run {name}"), press: Press::Command { manifest: ARMADA.into(), name: name.into() } });
    let check = |name: &str| Item::new(format!("check-{name}"), "list-checks", token::SKETCH_GREY, "Check", name).tag("Check", token::SKETCH_GREY).buttons(vec![run(name)]);
    let command = |name: &str| Item::new(format!("command-{name}"), "terminal", token::SKETCH_BLUE, "Command", name).tag("Command", token::SKETCH_BLUE).buttons(vec![run(name)]);
    vec![check("test"), check("verify-foundations"), check("format"), command("preview"), command("restart")]
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

/// One issue assigned to him and two open ones nobody holds.
pub fn issues() -> IssuesRead {
    let mine = vec![issue("armada-a3f2", "Slot GC leaks", &["you"])];
    let open = vec![issue("armada-a3f2", "Slot GC leaks", &["you"]), issue("armada-9efp", "Library sync", &[]), issue("armada-71cc", "Pocket offline", &[])];
    IssuesRead { mine, open }
}
