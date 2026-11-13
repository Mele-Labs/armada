//! Mock data: Fleet's own answers, written as JSON beside the crate and decoded through `ipc` as a
//! live answer is, so a fixture that no longer matches the wire fails to decode instead of drawing.
//! A screen on mock data is the same screen; only where its data comes from differs.

use std::collections::{BTreeMap, HashMap};

use std::sync::Mutex;

use ipc::{
    IssueList, JobThread, ModelChoices, NeedsYou, NeedsYouAct, NeedsYouDismiss, NeedsYouGlyph, NeedsYouGlyphKind, NeedsYouRow, NeedsYouSection,
    NeedsYouSectionKind, SessionList, SessionQuick, SessionRecord, SettingValue, WaitingActKind, WaitingItem, WaitingSource, WorkflowChange,
    WorkflowChanges, DOCUMENT_ITEM_PREFIX,
};
use serde::Deserialize;

/// The scenario the last [`jobs`] call named, which the message box reads its catalogue by.
static SCENARIO: Mutex<String> = Mutex::new(String::new());

/// The branches offered for the preview, as `GET /fleet/build/offers` answers: one ready, one being checked and one
/// failed, with the Refresh row naming the ready one. Skipped, Fleet's answer is the same offers with no row.
pub fn preview_offers(skipped: bool) -> ipc::PreviewOffers {
    let mut offers = ipc::decode::<ipc::PreviewOffers>("PreviewOffers", include_bytes!("../fixtures/preview-offers.json"))
        .unwrap_or_else(|why| panic!("the preview offers fixture does not decode: {why}"));
    if skipped {
        offers.refresh = None;
    }
    offers
}

/// The Jobs of a named scenario, as `GET /sessions` would answer them.
pub fn jobs(scenario: &str) -> Result<Vec<SessionRecord>, String> {
    if let Ok(mut held) = SCENARIO.lock() {
        // The switching walk is the tour's Jobs, so what reads the name (the message box's catalogue) reads the tour's.
        *held = if scenario == "jobs-switching" { "jobs-screens" } else { scenario }.to_string();
    }
    // The preferences a scenario is read with: Fleet answers none on mock data, and the rest read as defaults.
    crate::ways::set(if scenario == "ways-buttons" { ipc::AnswerWith::Buttons } else { ipc::AnswerWith::Type }, true);
    let bytes: &[u8] = match scenario {
        // `jobs-snoozed` is the same Jobs with the first snoozed, on a clock of its own (`fixed_now`).
        "jobs-many" | "jobs-snoozed" | "job-view" | "job-reasoning" | "run-commands" | "workflow-card" => include_bytes!("../fixtures/jobs-many.json"),
        // The same Jobs, with a terminal Job just started: Fleet's title for it, nothing in its thread.
        "terminal-job" => include_bytes!("../fixtures/terminal-jobs.json"),
        // The same Jobs, with one handing off and one that took an earlier Job's place, that one ended behind it.
        "jobs-handoff" => include_bytes!("../fixtures/jobs-handoff.json"),
        // The same Jobs, with Try amber forked from the first at its message naming the tone rule.
        "history" => include_bytes!("../fixtures/jobs-history.json"),
        // The same Jobs, with Checkout page loads twice moved from sample-web to sample-api.
        "thread-moved" => include_bytes!("../fixtures/jobs-moved.json"),
        "job-bands" => include_bytes!("../fixtures/jobs-bands.json"),
        // The same Jobs, armada's started for his Armada project and sample-web's for Storefront, sample-api's
        // for none, and Quiet the write-clash notices not titled yet.
        "jobs-projects" => include_bytes!("../fixtures/jobs-many.json"),
        // The same Jobs, with the first holding a plan, a sketch, a document and two drafts to read.
        "review-palette" => include_bytes!("../fixtures/review-palette.json"),
        // The same Jobs: the first owing a draft and two native walks, Audit the docs index a draft whose
        // row has gone, and Quiet the write-clash notices a page alone.
        "job-tabs" => include_bytes!("../fixtures/jobs-tabs.json"),
        // The panes' first Job with its plan approved and two groups finished, three pull requests
        // merged, its issue closed, a Job it started finished, and what Fleet did for it.
        "job-done" => include_bytes!("../fixtures/jobs-done.json"),
        // The same Jobs: Quiet the write-clash notices a mod Job that made two mods, Audit the docs index on a
        // workflow that lands in a report, and Checkout page loads twice holding a stack of four pull requests.
        "job-made" => include_bytes!("../fixtures/jobs-made.json"),
        // The panes' Jobs as Now draws each: the first working its plan with subagents, a walk and Fleet
        // at work; one stuck, one stopped, one at work with no plan, one failing and one drafting.
        "job-now" => include_bytes!("../fixtures/jobs-now.json"),
        // The same, the first with two more groups to come, one going to a Job not started yet, an issue
        // and a pull request no group holds, and the plan being drafted carrying a sketch.
        "job-open" => include_bytes!("../fixtures/jobs-open.json"),
        // The same, the first holding four watches: one live, one ended not ours that Needs you asks
        // about, one passed and one broken.
        "job-watching" => include_bytes!("../fixtures/jobs-open.json"),
        // The same, the first with how it was started and two of his project's tasks held: Audit the
        // docs index has a brief and kept nothing, Flaky login test has no brief.
        "job-spine" => include_bytes!("../fixtures/jobs-open.json"),
        // The same Jobs on workflows: the first at Build of five steps with Review skipped, its workflow
        // picked by the proposer; one at a step he skipped; one at its last step; two plans on gates, one
        // held on him and one whose Checks failed; and one on a workflow with no step set.
        "job-workflow" => include_bytes!("../fixtures/jobs-workflow.json"),
        // The same Jobs, Checkout page loads twice at Build with its lanes as Fleet works them out: Build lit and waiting, Walk skipped and
        // Land changed, holding a Linear ticket, a draft pull request, two watches and a Job it started.
        "job-lanes" => include_bytes!("../fixtures/jobs-workflow.json"),
        // The same, with a workflow editor's marks on its lanes: Plan set otherwise in Mobile, #88's Open a
        // tie between Workspaces, Smoke tests' lifecycle waiting on a removal, and a proposed rule from #88's Merged.
        "job-lanes-marks" => include_bytes!("../fixtures/jobs-workflow.json"),
        "job-view-composer" | "thread-presses" | "sent-messages" => include_bytes!("../fixtures/jobs-composer.json"),
        // The whole Jobs tour: the review palette's Jobs, with the Judge's reading on the first and
        // the second held Job holding its three waits.
        "jobs-screens" => include_bytes!("../fixtures/jobs-screens.json"),
        // Jobs started by Jobs: one starter with two, a chain two deep beside a third, and a lone Job; the
        // starter and the deepest Job need him.
        "jobs-tree" => include_bytes!("../fixtures/jobs-tree.json"),
        // The terminal Job's Jobs: the New project sheet's walk ends on the Job it starts in the project it made.
        "new-project" => include_bytes!("../fixtures/terminal-jobs.json"),
        // One Job with only its plan waiting on him, as Fleet served it.
        "needs-plan" => include_bytes!("../fixtures/needs-plan.json"),
        // One Job asking two questions, answered in buttons: the first has three options, one the Job's pick.
        "ways-buttons" => include_bytes!("../fixtures/ways-buttons.json"),
        // Five Jobs each running one kind of call after a finished one, and one whose run is over.
        "tool-rows" | "call-rows" | "sketch-card" | "sketch-approve" | "sketches-several" | "sketch-share" | "sketch-changes" | "subagent" => include_bytes!("../fixtures/jobs-tools.json"),
        // One Job with an open draft and a workflow draft, near the thread's end.
        "drafts" => include_bytes!("../fixtures/jobs-drafts.json"),
        // The box's Jobs, with the first holding a plan, two pull requests and a Job it started with one of its own.
        "job-panes" => include_bytes!("../fixtures/jobs-panes.json"),
        // The same Jobs, the first holding something of every kind the Ledger groups.
        "job-panes-more" => include_bytes!("../fixtures/jobs-panes-more.json"),
        // The box's Jobs, the first holding five pull requests: Armada Classic's panel pages, one a judge read.
        "job-pulls" => include_bytes!("../fixtures/jobs-pulls.json"),
        // The same Jobs, the first working five issues, one a tracker's ticket and one a #7 that sample-web has too,
        // started by one Job, starting two, one in sample-web, and forked from another.
        "job-issues" => include_bytes!("../fixtures/jobs-issues.json"),
        // The box's Jobs, the first holding four bays, two stranded, and clashing with the second and main.
        "job-system" => include_bytes!("../fixtures/jobs-panes.json"),
        // The tour's Jobs, for the walk that switches among them.
        "jobs-switching" => include_bytes!("../fixtures/jobs-screens.json"),
        other => return Err(format!("no mock scenario named {other}")),
    };
    let mut sessions = ipc::decode::<SessionList>("SessionList", bytes).map(|list| list.sessions).map_err(|why| why.to_string())?;
    // Now's figures count from now, so the scenario's clock is moved to the run's.
    if matches!(scenario, "job-now" | "job-open" | "job-watching" | "job-spine" | "job-workflow" | "job-lanes" | "job-lanes-marks") {
        for one in &mut sessions {
            one.last_heard_at = one.last_heard_at.as_ref().map(|at| shifted(NOW_ANCHOR, at));
            if let Some(held) = one.plan.as_mut().and_then(|plan| plan.held.as_mut()) {
                held.since = shifted(NOW_ANCHOR, &held.since);
            }
        }
    }
    // Every other walk was drawn before a workflow's steps were zones: their Jobs follow none, but for
    // the made walk's Job on a workflow that lands in a report.
    if !matches!(scenario, "job-workflow" | "job-lanes" | "job-lanes-marks") {
        for one in sessions.iter_mut().filter(|one| !(scenario == "job-made" && one.id.as_str() == "mock-04")) {
            one.workflow = None;
            one.workflow_step = None;
            one.workflow_copy = None;
        }
    }
    if scenario == "jobs-projects" {
        let facts = |project: &str| {
            let raw = format!(r#"{{"turn":{{"state":"idle"}},"mode":"auto","running":false,"project":"{project}"}}"#);
            ipc::decode::<ipc::HostedFacts>("HostedFacts", raw.as_bytes()).map_err(|why| why.to_string())
        };
        for one in &mut sessions {
            let project = match one.manifest_id.as_ref().map(|id| id.as_str()) {
                Some("armada") => "armada",
                Some("sample-web") => "storefront",
                _ => continue,
            };
            match one.hosted.as_mut() {
                Some(hosted) => hosted.project = Some(project.to_string()),
                None => one.hosted = Some(facts(project)?),
            }
        }
        if let Some(one) = sessions.iter_mut().find(|one| one.id.as_str() == "mock-06") {
            one.title = None;
        }
    }
    if scenario == "job-spine" {
        if let Some(one) = sessions.iter_mut().find(|one| one.id.as_str() == "mock-01") {
            one.started = Some(spine_starts().started);
        }
    }
    if matches!(scenario, "job-lanes" | "job-lanes-marks") {
        let mut lanes = ipc::decode::<ipc::JobLanesView>("JobLanesView", include_bytes!("../fixtures/job-lanes.json")).map_err(|why| why.to_string())?;
        if scenario == "job-lanes-marks" {
            lanes_marks(&mut lanes);
        }
        if let Some(one) = sessions.iter_mut().find(|one| one.id.as_str() == "mock-07") {
            one.workflow_step = Some(1);
            one.workflow_visits.truncate(2);
            one.lanes = Some(lanes);
        }
        // The Job it started has a Workflow of its own, which its lane opens.
        let child = ipc::decode::<ipc::JobLanesView>("JobLanesView", include_bytes!("../fixtures/job-lanes-child.json")).map_err(|why| why.to_string())?;
        if let Some(one) = sessions.iter_mut().find(|one| one.id.as_str() == "mock-10") {
            one.lanes = Some(child);
        }
    }
    // The gated plan's live stretch is held on his walk, which Fleet asks him about.
    if scenario == "job-workflow" {
        if let Some(one) = sessions.iter_mut().find(|one| one.id.as_str() == "mock-04") {
            let item = ipc::decode::<ipc::WaitingItem>("walk item", br#"{"id":"walk:armada://walk/docsIndex","text":"Walk the new index","since":"2026-11-09T14:20:00Z","source":"walk","act":{"kind":"walk","target":"armada://walk/docsIndex"}}"#).map_err(|why| why.to_string())?;
            one.waiting_for.push(item);
        }
    }
    // Each open sketch waits on him as its own item, newest drawn first, as Fleet lists them.
    if scenario == "sketches-several" {
        if let Some(one) = sessions.iter_mut().find(|one| one.id.as_str() == "tool-edit") {
            for (title, since) in [("Freshness chip", "2026-11-10T22:00:41Z"), ("Merge line flow", "2026-11-10T21:58:12Z")] {
                let wire = format!(r#"{{"id":"sketch:{title}","text":"{title}","since":"{since}","source":"sketch","act":{{"kind":"message","target":"tool-edit"}}}}"#);
                one.waiting_for.push(ipc::decode::<WaitingItem>("sketch item", wire.as_bytes()).map_err(|why| why.to_string())?);
            }
        }
    }
    // The first Job needs him and is snoozed until the afternoon: it keeps its items and raises nothing.
    if scenario == "jobs-snoozed" {
        if let (Some(one), Some(now)) = (sessions.iter_mut().find(|one| one.id.as_str() == "mock-01"), fixed_now()) {
            one.snoozed_until = Some(crate::snooze::instant_of((now + chrono::Duration::minutes(148)).with_timezone(&chrono::Utc)));
        }
    }
    // Needs you asks about the watch that ended not ours, as Fleet derives the item.
    if scenario == "job-watching" {
        for ask in watch_reads().asks {
            if let Some(one) = sessions.iter_mut().find(|one| one.id.as_str() == ask.session_id) {
                one.waiting_for.push(ask.item);
            }
        }
    }
    Ok(sessions)
}

/// The moment `jobs-snoozed` counts from, so the times a snooze names read the same on every run and in every
/// zone: 14:02 on 13 Nov 2026, local. `None` for any other scenario, which reads the real clock.
pub fn fixed_now() -> Option<chrono::DateTime<chrono::Local>> {
    use chrono::TimeZone;
    let held = SCENARIO.lock().ok()?;
    if *held != "jobs-snoozed" {
        return None;
    }
    chrono::Local.with_ymd_and_hms(2026, 11, 13, 14, 2, 0).single()
}

/// The instant `job-now` was written at, which a run moves to its own now.
const NOW_ANCHOR: &str = "2026-11-09T14:45:00Z";

/// The repositories Fleet would serve (`GET /repositories`) in the walks: the four the title bar's
/// switcher is drawn from, the last with no Manifest.
pub fn repositories() -> Vec<ipc::RepositorySummary> {
    ipc::decode::<ipc::RepositoryList>("RepositoryList", include_bytes!("../fixtures/repositories.json"))
        .unwrap_or_else(|why| panic!("the repositories fixture does not decode: {why}"))
        .repositories
}

/// The folders of the walks' repositories, as `GET /repository_areas` would answer them: the New project
/// sheet opens a repository on these.
pub fn repository_areas() -> ipc::RepositoryAreas {
    ipc::decode::<ipc::RepositoryAreas>("RepositoryAreas", include_bytes!("../fixtures/new-project-areas.json"))
        .unwrap_or_else(|why| panic!("the repository areas fixture does not decode: {why}"))
}

/// His projects as `GET /projects` would answer them in the walks: two that read, over the repositories
/// above, and one whose file did not, which the switcher leaves out.
pub fn projects() -> ipc::Projects {
    let mut projects = ipc::decode::<ipc::Projects>("Projects", include_bytes!("../fixtures/projects.json"))
        .unwrap_or_else(|why| panic!("the projects fixture does not decode: {why}"));
    // On the Spine's walk, Armada holds two tasks the first Job took on.
    if SCENARIO.lock().is_ok_and(|named| named.as_str() == "job-spine") {
        projects.projects[0].rows = spine_starts().tasks;
    }
    projects
}

/// The workflows Fleet would serve (`GET /workflows`) in the walks: three of Armada's.
/// The mods on this machine: those a Job made in the scenario that walks them, Classic's `mods-themes`
/// Dusk in the one Settings walks, and none elsewhere.
pub fn mods() -> Vec<ipc::ModSummary> {
    let bytes: &[u8] = match SCENARIO.lock().as_deref().map(String::as_str) {
        Ok("job-made") => include_bytes!("../fixtures/mods-made.json"),
        Ok("jobs-many") => include_bytes!("../fixtures/mods-settings.json"),
        _ => return Vec::new(),
    };
    ipc::decode::<ipc::ModList>("mods", bytes).unwrap_or_else(|why| panic!("the mods fixture does not decode: {why}")).mods
}

/// A mock mod's sheet, as the mod folder holds it: Dusk's `theme.css` as the example mod ships it.
pub fn mod_sheet(name: &str) -> Option<&'static str> {
    (name == "dusk").then_some(include_str!("../../../packages/tokens/themes/example-mod/dusk/theme.css"))
}

pub fn workflows() -> Vec<ipc::WorkflowSummary> {
    ipc::decode::<Vec<ipc::WorkflowSummary>>("workflows", include_bytes!("../fixtures/workflows.json")).unwrap_or_else(|why| panic!("the workflows fixture does not decode: {why}"))
}

/// How the Spine's first Job was started, and the tasks of his it holds.
#[derive(Deserialize)]
struct SpineStarts {
    started: ipc::SessionStarted,
    tasks: Vec<ipc::ProjectRow>,
}

fn spine_starts() -> SpineStarts {
    ipc::decode::<SpineStarts>("spine starts", include_bytes!("../fixtures/spine-starts.json")).unwrap_or_else(|why| panic!("the spine starts fixture does not decode: {why}"))
}

/// The threads Fleet would serve for a scenario's Jobs, by Job id, as `GET /sessions/thread` answers
/// them. A scenario that serves threads serves one for every Job it lists, empty where its fixture
/// holds none, as Fleet answers for a Job with nothing in it yet; one that serves none leaves its Jobs
/// on their skeletons, and a thread Fleet refuses stays unread.
pub fn threads(scenario: &str) -> Result<HashMap<String, JobThread>, String> {
    let mut all = fixture_threads(scenario)?;
    if all.is_empty() {
        return Ok(all);
    }
    for job in jobs(scenario)? {
        let id = job.id.as_str();
        if all.contains_key(id) || refused(scenario, id).is_some() {
            continue;
        }
        all.insert(id.to_string(), empty_thread(id, job.started_at.as_str())?);
    }
    Ok(all)
}

/// The thread Fleet answers for a Job with nothing in it yet: a head with its start, and no items.
pub fn empty_thread(id: &str, started: &str) -> Result<JobThread, String> {
    let empty = format!(r#"{{"session_id":"{id}","head":{{"trail":{{"started":"{started}"}}}},"items":[]}}"#);
    ipc::decode::<JobThread>("empty thread", empty.as_bytes()).map_err(|why| why.to_string())
}

/// The threads a scenario's fixtures hold.
fn fixture_threads(scenario: &str) -> Result<HashMap<String, JobThread>, String> {
    #[derive(Deserialize)]
    struct Threads {
        threads: Vec<JobThread>,
    }
    let bytes: &[u8] = match scenario {
        "jobs-screens" => return screens_threads(),
        "job-view" | "review-palette" | "terminal-job" | "new-project" => include_bytes!("../fixtures/job-threads.json"),
        // The same threads, and the first Job's agent showing three commands to run.
        "run-commands" => include_bytes!("../fixtures/run-threads.json"),
        // The same threads, and Quiet the write-clash notices holding the page it asks him to read.
        "job-tabs" => include_bytes!("../fixtures/tabs-threads.json"),
        // The panes' threads, with two walks ended, two commits and Fleet's acts on the first.
        "job-done" => include_bytes!("../fixtures/done-threads.json"),
        // The same threads, and the mod Job's two Checks, the newest running.
        "job-made" => include_bytes!("../fixtures/made-threads.json"),
        "job-now" => include_bytes!("../fixtures/now-threads.json"),
        // The same, with Fleet's two acts to come on the first and the drafted plan's sketch.
        "job-open" | "job-watching" | "job-spine" | "job-workflow" | "job-lanes" | "job-lanes-marks" => include_bytes!("../fixtures/open-threads.json"),
        // The same threads, and the earlier Job's, ended where it handed on.
        "jobs-handoff" => include_bytes!("../fixtures/jobs-handoff-threads.json"),
        // The same threads, with the Job that took the router upgrade on and the forked one.
        "history" => include_bytes!("../fixtures/history-threads.json"),
        "thread-moved" => include_bytes!("../fixtures/moved-threads.json"),
        // The thread the box was walked against: no question or plan waiting, so the quick bar draws.
        "job-view-composer" | "job-panes" | "job-pulls" | "job-issues" | "job-system" => include_bytes!("../fixtures/composer-threads.json"),
        // The first Job's thread with a run of Bash calls, two of them commits, and the Audit Job's quiet one.
        "job-panes-more" => include_bytes!("../fixtures/panes-more-threads.json"),
        // The box's threads, with Audit the docs index ending on a message that names a Job, holds a block and an address.
        "thread-presses" => include_bytes!("../fixtures/thread-presses-threads.json"),
        // The box's threads, with Audit the docs index ending on his message that tags a Job in its words and a pull request beside them.
        "sent-messages" => include_bytes!("../fixtures/sent-messages-threads.json"),
        "job-bands" => include_bytes!("../fixtures/bands-threads.json"),
        "needs-plan" => include_bytes!("../fixtures/needs-plan-threads.json"),
        "ways-buttons" => include_bytes!("../fixtures/ways-buttons-threads.json"),
        "tool-rows" => include_bytes!("../fixtures/tools-threads.json"),
        // The tool-rows threads, with the finished Job's retro being written and Search for the queue asking with previews.
        "call-rows" => include_bytes!("../fixtures/call-rows-threads.json"),
        // The tool-rows threads, with Edit the merge line putting a sketch to him: every kind of node, arrows with words, his ink and steps.
        // Edit the merge line putting two sketches to him: Freshness chip pinned, Merge line flow with its card in the thread.
        "sketch-card" | "sketch-approve" | "sketches-several" => include_bytes!("../fixtures/sketch-card-threads.json"),
        "sketch-share" => include_bytes!("../fixtures/sketch-share-threads.json"),
        // The same, with Edit the merge line drawing the store clock twice: Pinned clock read against Store clock reads, which has two revisions.
        "sketch-changes" => include_bytes!("../fixtures/sketch-changes-threads.json"),
        // The tool-rows threads, with Brief a reviewer handing work to a reviewer that finished and an explorer still running.
        "subagent" => include_bytes!("../fixtures/subagent-threads.json"),
        "drafts" => include_bytes!("../fixtures/drafts-threads.json"),
        "job-reasoning" => include_bytes!("../fixtures/reasoning-threads.json"),
        "jobs-switching" => return switching_threads(),
        // The Audit Job's thread ending on a proposed workflow's draft.
        "workflow-card" => include_bytes!("../fixtures/workflow-card-threads.json"),
        _ => return Ok(HashMap::new()),
    };
    let read = ipc::decode::<Threads>("job threads", bytes).map_err(|why| why.to_string())?;
    let mut all: HashMap<String, JobThread> = read.threads.into_iter().map(|thread| (thread.session_id.as_str().to_string(), thread)).collect();
    if matches!(scenario, "job-now" | "job-open" | "job-watching" | "job-spine" | "job-workflow" | "job-lanes" | "job-lanes-marks") {
        for thread in all.values_mut() {
            if let Some(call) = thread.head.open_call.as_mut() {
                call.since = shifted(NOW_ANCHOR, &call.since);
            }
            for act in &mut thread.head.fleet_acts {
                act.at = shifted(NOW_ANCHOR, &act.at);
            }
        }
    }
    // The walk the gated plan's live stretch is held on, as the Audit Job showed it.
    if scenario == "job-workflow" {
        if let Some(thread) = all.get_mut("mock-04") {
            let row = ipc::decode::<ipc::ThreadItem>("walk row", br#"{"kind":"row","row":{"kind":"window","id":"win-wf-1","at":"2026-11-09T14:20:00Z","title":"The new docs index","url":"armada://walk/docsIndex"}}"#).map_err(|why| why.to_string())?;
            thread.items.push(row);
        }
    }
    // As Fleet folds two open sketches: the newest is the head's pin and its card no item, the other keeps its card asked.
    if scenario == "sketches-several" {
        if let Some(thread) = all.get_mut("tool-edit") {
            thread.items.retain(|item| !matches!(item, ipc::ThreadItem::Sketch(card) if card.sketch == "sketch:Freshness chip"));
            for item in &mut thread.items {
                if let ipc::ThreadItem::Sketch(card) = item {
                    card.asked = true;
                }
            }
        }
    }
    // The Job whose system Fleet cannot read has a thread with nothing in it yet, as Fleet answers for one.
    if scenario == "job-system" {
        let empty = ipc::decode::<JobThread>("empty thread", br#"{"session_id":"mock-03","head":{"trail":{"started":"2026-11-11T11:40:00Z"}},"items":[]}"#).map_err(|why| why.to_string())?;
        all.insert("mock-03".to_string(), empty);
    }
    Ok(all)
}

/// The workflow changes Fleet would list for the scenario last named by [`jobs`], by change id, as
/// `GET /workflows/changes` answers them. Empty for a scenario that proposes no workflow.
pub fn workflow_changes() -> Result<BTreeMap<i64, WorkflowChange>, String> {
    let named = SCENARIO.lock().map(|held| held.clone()).unwrap_or_default();
    let bytes: &[u8] = match named.as_str() {
        "workflow-card" => include_bytes!("../fixtures/workflow-card-changes.json"),
        _ => return Ok(BTreeMap::new()),
    };
    let read = ipc::decode::<WorkflowChanges>("WorkflowChanges", bytes).map_err(|why| why.to_string())?;
    Ok(read.changes.into_iter().map(|change| (change.id, change)).collect())
}

/// The tour's threads, each from the fixture it was walked against: the first Job's question, plan and
/// cards and the long thread of the second from `job-threads`, the Audit Job's quiet thread from
/// `composer-threads` so its quick bar draws, and the held Job's from `screens-threads`.
fn screens_threads() -> Result<HashMap<String, JobThread>, String> {
    #[derive(Deserialize)]
    struct Threads {
        threads: Vec<JobThread>,
    }
    let read = |name: &'static str, bytes: &[u8]| ipc::decode::<Threads>(name, bytes).map(|read| read.threads).map_err(|why| why.to_string());
    let mut all = HashMap::new();
    for (id, thread) in read("job threads", include_bytes!("../fixtures/job-threads.json"))?.into_iter().map(|thread| (thread.session_id.as_str().to_string(), thread)) {
        if id != "mock-04" {
            all.insert(id, thread);
        }
    }
    for thread in read("composer threads", include_bytes!("../fixtures/composer-threads.json"))? {
        if thread.session_id.as_str() == "mock-04" {
            all.insert("mock-04".to_string(), thread);
        }
    }
    for thread in read("screens threads", include_bytes!("../fixtures/screens-threads.json"))? {
        all.insert(thread.session_id.as_str().to_string(), thread);
    }
    Ok(all)
}

/// The tour's threads with the second Job's grown to 600 items, for the walk that times a switch to a long thread.
fn switching_threads() -> Result<HashMap<String, JobThread>, String> {
    #[derive(Deserialize)]
    struct Threads {
        threads: Vec<JobThread>,
    }
    let mut all = screens_threads()?;
    let long = ipc::decode::<Threads>("switching threads", include_bytes!("../fixtures/switching-threads.json")).map_err(|why| why.to_string())?;
    for thread in long.threads {
        all.insert(thread.session_id.as_str().to_string(), thread);
    }
    Ok(all)
}

/// The thread Fleet refuses to read in the scenario last named, with its words: Quiet the write-clash
/// notices in `job-system`, so a walk can show the refusal's toast and the thread drawing nothing.
pub fn thread_refused(id: &str) -> Option<String> {
    let named = SCENARIO.lock().ok()?;
    refused(named.as_str(), id)
}

fn refused(scenario: &str, id: &str) -> Option<String> {
    (scenario == "job-system" && id == "mock-06").then(|| "Fleet could not read this Job's ledger: it ends mid-row at line 2214".to_string())
}

/// What Fleet answers Open on a native walk with, in the scenario last named: Chip colours in
/// `job-tabs` has no build to walk yet, so a walk can show Fleet's words under Open.
pub fn walk_refused(url: &str) -> Option<String> {
    let named = SCENARIO.lock().ok()?;
    (named.as_str() == "job-tabs" && url == "armada://walk/chipColours").then(|| "Build the walk first: cargo build in crates/bridge-native in slot-4".to_string())
}

/// Whether a Job's thread is read again for a moment when it comes on screen, as Fleet's read takes
/// time: in `thread-moved`, so a walk can show the row that says new messages are loading.
pub fn reads_slowly() -> bool {
    SCENARIO.lock().is_ok_and(|named| named.as_str() == "thread-moved")
}

/// Fleet's refusal of an act on this route, in the scenario last named: `sketch-approve` refuses a
/// sketch's answer as Fleet does a drawing read before it was drawn again.
pub fn act_refused(route: &str) -> Option<String> {
    let named = SCENARIO.lock().ok()?;
    (named.as_str() == "sketch-approve" && route == "/sessions/sketch/answer")
        .then(|| "the sketch `Freshness chip` is not the drawing that was read: the job has drawn it again since".into())
}

/// How many sends the scenario last named refuses before it takes them: one in `sent-messages`, so a
/// walk can show a message Fleet did not take and its Retry.
pub fn refusals() -> usize {
    SCENARIO.lock().map_or(0, |named| usize::from(named.as_str() == "sent-messages"))
}

/// A subagent's thread as `GET /sessions/subagent` would answer it, in the scenario last named.
pub fn subagent<T: serde::de::DeserializeOwned>(job: &str, id: &str) -> Option<T> {
    #[derive(Deserialize)]
    struct One<T> {
        session_id: String,
        subagent_id: String,
        read: T,
    }
    #[derive(Deserialize)]
    struct Reads<T> {
        subagents: Vec<One<T>>,
    }
    let named = SCENARIO.lock().ok()?.clone();
    let bytes: &[u8] = match named.as_str() {
        "subagent" => include_bytes!("../fixtures/subagent-reads.json"),
        "job-now" => include_bytes!("../fixtures/now-subagent-reads.json"),
        "job-panes-more" => include_bytes!("../fixtures/panes-more-subagents.json"),
        _ => return None,
    };
    let reads = ipc::decode::<Reads<T>>("subagent reads", bytes).unwrap_or_else(|why| panic!("the subagent reads fixture does not decode: {why}"));
    reads.subagents.into_iter().find(|one| one.session_id == job && one.subagent_id == id).map(|one| one.read)
}

/// The call-rows walk opens a call's file and its Check run from the panes' fixtures.
fn call_rows_named() -> bool {
    SCENARIO.lock().is_ok_and(|named| named.as_str() == "call-rows")
}

/// The scenario last named is the panes' own: their reads answer only there.
fn panes_named() -> bool {
    SCENARIO.lock().is_ok_and(|named| matches!(named.as_str(), "job-panes" | "job-panes-more" | "job-pulls" | "job-issues" | "job-system" | "job-open" | "job-spine" | "job-workflow" | "job-lanes" | "job-lanes-marks"))
}

/// A pull request's files as `GET /pull_request_patches/:repository/:number` answers them, by
/// `manifest#number`.
pub fn patches(key: &str) -> Option<ipc::PullRequestFilePatches> {
    #[derive(Deserialize)]
    struct One {
        key: String,
        #[serde(flatten)]
        patches: ipc::PullRequestFilePatches,
    }
    #[derive(Deserialize)]
    struct Pulls {
        pulls: Vec<One>,
    }
    if !panes_named() {
        return None;
    }
    let pulls = ipc::decode::<Pulls>("pull request patches", include_bytes!("../fixtures/panes-patches.json"))
        .unwrap_or_else(|why| panic!("the patches fixture does not decode: {why}"));
    pulls.pulls.into_iter().find(|one| one.key == key).map(|one| one.patches)
}

/// A pull request's page as `GET /pull_request_details/:repository/:number` answers it, by
/// `manifest#number`: Armada Classic's mock pages, narrowed to three.
pub fn pull_page(key: &str) -> Option<ipc::PullRequestPage> {
    #[derive(Deserialize)]
    struct One {
        key: String,
        #[serde(flatten)]
        page: ipc::PullRequestPage,
    }
    #[derive(Deserialize)]
    struct Pages {
        pulls: Vec<One>,
    }
    if !panes_named() {
        return None;
    }
    let pages = ipc::decode::<Pages>("pull request pages", include_bytes!("../fixtures/pulls-pages.json")).unwrap_or_else(|why| panic!("the pull request pages fixture does not decode: {why}"));
    pages.pulls.into_iter().find(|one| one.key == key).map(|one| one.page)
}

/// The merge lines a pull request's panel finds its place in, as Fleet's merge line view holds them.
pub fn merge_lines() -> Option<Vec<ipc::MergeLineView>> {
    #[derive(Deserialize)]
    struct Lines {
        #[serde(default)]
        merge_lines: Vec<ipc::MergeLineView>,
    }
    if !panes_named() {
        return None;
    }
    let lines = ipc::decode::<Lines>("merge lines", include_bytes!("../fixtures/pulls-pages.json")).unwrap_or_else(|why| panic!("the pull request pages fixture does not decode: {why}"));
    Some(lines.merge_lines)
}

/// His notes on a pull request's lines as Fleet keeps them, by `manifest#number`.
pub fn pull_notes(key: &str) -> Option<Vec<ipc::PullRequestNote>> {
    #[derive(Deserialize)]
    struct Notes {
        #[serde(default)]
        notes: std::collections::HashMap<String, Vec<ipc::PullRequestNote>>,
    }
    if !panes_named() {
        return None;
    }
    let mut read = ipc::decode::<Notes>("pull request notes", include_bytes!("../fixtures/pulls-pages.json")).unwrap_or_else(|why| panic!("the pull request pages fixture does not decode: {why}"));
    read.notes.remove(key)
}

/// What `GET /test_map/reaching` answers for a pull request's changed files, by `manifest#number`.
pub fn tests_reaching(key: &str) -> Option<ipc::TestsReaching> {
    #[derive(Deserialize)]
    struct One {
        key: String,
        #[serde(flatten)]
        answer: ipc::TestsReaching,
    }
    #[derive(Deserialize)]
    struct Reaching {
        #[serde(default)]
        reaching: Vec<One>,
    }
    if !panes_named() {
        return None;
    }
    let read = ipc::decode::<Reaching>("tests reaching", include_bytes!("../fixtures/pulls-pages.json")).unwrap_or_else(|why| panic!("the pull request pages fixture does not decode: {why}"));
    read.reaching.into_iter().find(|one| one.key == key).map(|one| one.answer)
}

#[derive(Deserialize)]
struct IssueReads {
    issues: Vec<KeyedIssue>,
    lifecycles: Vec<ipc::RepositoryLifecycles>,
}

#[derive(Deserialize)]
struct KeyedIssue {
    key: String,
    #[serde(flatten)]
    read: ipc::IssueRead,
}

fn issue_reads() -> Option<IssueReads> {
    panes_named().then(|| ipc::decode::<IssueReads>("issue reads", include_bytes!("../fixtures/issue-reads.json")).unwrap_or_else(|why| panic!("the issue reads fixture does not decode: {why}")))
}

/// An issue as `GET /issues/:repository/:reference` answers it, by `manifest#reference`.
pub fn issue(key: &str) -> Option<ipc::IssueRead> {
    issue_reads()?.issues.into_iter().find(|one| one.key == key).map(|one| one.read)
}

/// A repository's lifecycles as `GET /lifecycles?manifest_id=` answers them.
pub fn lifecycles(manifest: &str) -> Option<ipc::RepositoryLifecycles> {
    issue_reads()?.lifecycles.into_iter().find(|one| one.manifest_id.as_str() == manifest)
}

/// A file of the Job's repository as `GET /sessions/repository_file` will answer it: its text, or
/// Fleet's refusal word (`not_addressable`, `too_big`, `binary`, `unreadable`).
pub fn repository_file(path: &str) -> Option<Result<String, String>> {
    #[derive(Deserialize)]
    struct One {
        path: String,
        text: Option<String>,
        refused: Option<String>,
    }
    #[derive(Deserialize)]
    struct Files {
        files: Vec<One>,
    }
    if !panes_named() && !call_rows_named() {
        return None;
    }
    if let Some(text) = log(path) {
        return Some(Ok(text));
    }
    let files = ipc::decode::<Files>("repository files", include_bytes!("../fixtures/panes-files.json"))
        .unwrap_or_else(|why| panic!("the repository files fixture does not decode: {why}"));
    let one = files.files.into_iter().find(|one| one.path == path)?;
    Some(one.text.ok_or_else(|| one.refused.unwrap_or_else(|| "unreadable".into())))
}

/// A Check run's output as `GET /sessions/checks/:run/output` answers it.
pub fn check_output(run: u64) -> Option<ipc::CheckOutput> {
    #[derive(Deserialize)]
    struct One {
        run: u64,
        #[serde(flatten)]
        output: ipc::CheckOutput,
    }
    #[derive(Deserialize)]
    struct Outputs {
        outputs: Vec<One>,
    }
    if !panes_named() && !call_rows_named() {
        return None;
    }
    let outputs = ipc::decode::<Outputs>("check outputs", include_bytes!("../fixtures/panes-checks.json"))
        .unwrap_or_else(|why| panic!("the check outputs fixture does not decode: {why}"));
    outputs.outputs.into_iter().find(|one| one.run == run).map(|one| one.output)
}

/// The models Fleet offers (`GET /models`), for every scenario: a live Fleet always answers it, so
/// no walk draws the box without its model. `None` off the mock, where no scenario was named.
pub fn models() -> Option<ModelChoices> {
    if SCENARIO.lock().ok()?.is_empty() {
        return None;
    }
    let models = ipc::decode("ModelChoices", include_bytes!("../fixtures/composer-models.json"));
    Some(models.unwrap_or_else(|why| panic!("the models fixture does not decode: {why}")))
}

/// What the message box reads from Fleet beside the Job, past its models: the quick bar's look and
/// answers, the repository's files and its issues.
pub struct Catalogue {
    pub look: SettingValue,
    pub quick: SessionQuick,
    pub files: Vec<String>,
    pub issues: IssueList,
    /// What a mock helper says to Talk in each Job, by Job id.
    pub talk: HashMap<String, crate::voice::Script>,
}

/// The catalogue of the scenario last named, where it has one.
pub fn catalogue() -> Option<Catalogue> {
    #[derive(Deserialize)]
    struct Files {
        paths: Vec<String>,
    }
    #[derive(Deserialize)]
    struct Talk {
        job: String,
        #[serde(flatten)]
        script: crate::voice::Script,
    }
    #[derive(Deserialize)]
    struct Talks {
        scripts: Vec<Talk>,
    }
    let named = SCENARIO.lock().ok()?.clone();
    if !matches!(named.as_str(), "job-view-composer" | "jobs-screens" | "thread-presses" | "sent-messages" | "history" | "job-panes" | "job-panes-more") {
        return None;
    }
    let read = || -> Result<Catalogue, String> {
        let why = |why: ipc::Undecodable| why.to_string();
        Ok(Catalogue {
            look: ipc::decode("thread.look", include_bytes!("../fixtures/composer-look.json")).map_err(why)?,
            quick: ipc::decode("SessionQuick", include_bytes!("../fixtures/composer-quick.json")).map_err(why)?,
            files: ipc::decode::<Files>("files", include_bytes!("../fixtures/composer-files.json")).map_err(why)?.paths,
            issues: ipc::decode("IssueList", include_bytes!("../fixtures/composer-issues.json")).map_err(why)?,
            talk: ipc::decode::<Talks>("talk", include_bytes!("../fixtures/composer-talk.json")).map_err(why)?.scripts.into_iter().map(|one| (one.job, one.script)).collect(),
        })
    };
    read().ok()
}

/// What `GET /sessions/needs_you` would answer for a mock Job: its `waiting_for` less the items put away here,
/// grouped as Fleet groups them. **This is the mock's own grouping, written once for every scenario**, so a
/// fixture's `waiting_for` is all a scenario carries and a walk's dismissal comes out as Fleet's view would.
/// It is `crates/fleet/src/session_host/needs_you.rs` read for the sources the fixtures hold: a source
/// added there is placed here when a fixture holds one.
pub fn needs_you(job: &str, items: &[WaitingItem], away: &dyn Fn(&str) -> bool) -> NeedsYou {
    use NeedsYouSectionKind as Section;
    let rows: Vec<(Section, NeedsYouRow)> = items
        .iter()
        .filter(|item| !away(&item.id))
        .map(|item| {
            let act = mock_act(item, job);
            let (section, dismiss) = mock_place(item, &act);
            let row = NeedsYouRow { id: item.id.clone(), source: item.source, text: item.text.clone(), since: item.since.clone(), glyph: mock_glyph(item, &act), act, dismiss };
            (section, row)
        })
        .collect();
    let listed = |section: Section| matches!(section, Section::Questions | Section::ToDo | Section::Walks | Section::Commands | Section::PullRequests);
    let owed = rows.iter().filter(|(section, _)| *section != Section::Done).count();
    let reviewing = rows.iter().filter(|(section, _)| *section == Section::ToReview).count();
    let alone_is_a_document = owed == 1 && rows.iter().any(|(_, row)| row.source == WaitingSource::Document);
    let review_in_band = reviewing > 1 || alone_is_a_document;
    let mut glyphs: Vec<NeedsYouGlyph> = rows.iter().filter(|(section, _)| listed(*section)).map(|(_, row)| NeedsYouGlyph { kind: row.glyph.kind, said: row.text.clone() }).collect();
    if review_in_band {
        let mut pages: Vec<&NeedsYouRow> = rows.iter().filter(|(section, _)| *section == Section::ToReview).map(|(_, row)| row).collect();
        pages.sort_by(|a, b| a.since.as_str().cmp(b.since.as_str()));
        glyphs.extend(pages.into_iter().map(|row| row.glyph.clone()));
    }
    let order = [
        (Section::Questions, "Questions"),
        (Section::ToDo, "To do"),
        (Section::ToReview, "To review"),
        (Section::Walks, "Walks"),
        (Section::Commands, "Commands"),
        (Section::PullRequests, "PRs"),
        (Section::Restart, "Restart"),
        (Section::Done, "Done"),
    ];
    let sections = order
        .iter()
        .filter_map(|(kind, label)| {
            let mut mine: Vec<NeedsYouRow> = rows.iter().filter(|(section, _)| section == kind).map(|(_, row)| row.clone()).collect();
            if *kind == Section::ToReview {
                mine.sort_by(|a, b| a.since.as_str().cmp(b.since.as_str()));
            }
            (!mine.is_empty()).then(|| NeedsYouSection { kind: *kind, label: (*label).into(), rows: mine })
        })
        .collect();
    NeedsYou { session_id: ipc::SessionId::carried(job), sections, fold: rows.iter().any(|(section, _)| listed(*section)), review_in_band, glyphs }
}

/// When an item of the thread was written, where it says.
fn written(item: &ipc::ThreadItem) -> Option<&str> {
    match item {
        ipc::ThreadItem::Row { row } => Some(row.at().as_str()),
        ipc::ThreadItem::Calls { rows, .. } => rows.first().map(|row| row.at().as_str()),
        ipc::ThreadItem::Subagent { at, .. } => Some(at.as_str()),
        ipc::ThreadItem::Sketch(card) => Some(card.at.as_str()),
        _ => None,
    }
}

/// What Fleet's thread does once a sketch is answered: the card settles, and where it was the pin, the
/// pin is read again, so it is the newest sketch still open (its card leaves the items) and the answered
/// card goes back among them. Returns the first item that changed.
pub fn sketch_answered(head: &mut ipc::ThreadHead, items: &mut Vec<ipc::ThreadItem>, sketch: &str, how: ipc::SketchHow, at: &str, words: Option<String>) -> Option<usize> {
    let settled = ipc::CardSettled { how, at: ipc::Instant::carried(at.to_string()), words };
    let open = |card: &ipc::ThreadSketch| card.asked && !card.withdrawn && !card.replaced && card.settled.is_none();
    if head.sketch.as_ref().is_some_and(|pin| pin.sketch == sketch) {
        let mut answered = head.sketch.take()?;
        answered.settled = Some(settled);
        let newest = items
            .iter()
            .enumerate()
            .filter_map(|(at, item)| match item {
                ipc::ThreadItem::Sketch(card) if open(card) => Some((at, card.at.as_str().to_string())),
                _ => None,
            })
            .max_by(|a, b| a.1.cmp(&b.1))
            .map(|(at, _)| at);
        let mut changed = newest;
        if let Some(at) = newest {
            if let ipc::ThreadItem::Sketch(card) = items.remove(at) {
                head.sketch = Some(card);
            }
        }
        let back = items.iter().position(|item| written(item).is_some_and(|then| then > answered.at.as_str())).unwrap_or(items.len());
        items.insert(back, ipc::ThreadItem::Sketch(answered));
        changed = Some(changed.map_or(back, |at| at.min(back)));
        return changed;
    }
    let at = items.iter().rposition(|item| matches!(item, ipc::ThreadItem::Sketch(card) if card.sketch == sketch && open(card)))?;
    if let ipc::ThreadItem::Sketch(card) = &mut items[at] {
        card.settled = Some(settled);
    }
    Some(at)
}

fn mock_act(item: &WaitingItem, job: &str) -> NeedsYouAct {
    let named = |prefix: &str| item.id.strip_prefix(prefix).unwrap_or(&item.id).to_string();
    let command = item.act.as_ref().filter(|act| act.kind == WaitingActKind::Run).map(|act| act.target.clone());
    let pressed = match item.source {
        WaitingSource::Plan => NeedsYouAct::OpenPlan,
        WaitingSource::Sketch => NeedsYouAct::OpenSketch { title: named("sketch:") },
        WaitingSource::Document => NeedsYouAct::OpenDocument { path: named(DOCUMENT_ITEM_PREFIX) },
        WaitingSource::Draft => NeedsYouAct::OpenDraft { row: named("draft:") },
        _ if !item.options.is_empty() => NeedsYouAct::Answer { options: item.options.clone(), other: true },
        // A restart is the band's key even where the agent gave steps, as Classic's `restartOf` reads it.
        _ if !item.steps.is_empty() && command.as_deref().and_then(mock_restart).is_none() => NeedsYouAct::Steps { steps: item.steps.clone(), command: command.clone() },
        _ => {
            let words = matches!(item.source, WaitingSource::Agent | WaitingSource::Replied);
            match &item.act {
                None if words => NeedsYouAct::Reply,
                None => NeedsYouAct::None,
                Some(act) => match act.kind {
                    WaitingActKind::Walk => NeedsYouAct::Walk { url: act.target.clone() },
                    WaitingActKind::Answer if words => NeedsYouAct::Reply,
                    WaitingActKind::Answer => NeedsYouAct::Answer { options: Vec::new(), other: false },
                    WaitingActKind::ApprovePr => NeedsYouAct::ApprovePr { target: act.target.clone() },
                    WaitingActKind::Run => match mock_restart(&act.target) {
                        Some((label, build, adopt)) => NeedsYouAct::Restart { label: label.into(), build, adopt, command: act.target.clone() },
                        None => NeedsYouAct::Run { command: act.target.clone() },
                    },
                    WaitingActKind::Message if act.target != job => NeedsYouAct::OpenJob { id: act.target.clone() },
                    WaitingActKind::Message => NeedsYouAct::Message,
                },
            }
        }
    };
    let answers = item.act.as_ref().is_some_and(|act| act.kind == WaitingActKind::Answer);
    let to_do = item.source == WaitingSource::Agent
        && item.options.is_empty()
        && !answers
        && !matches!(pressed, NeedsYouAct::Walk { .. } | NeedsYouAct::Run { .. } | NeedsYouAct::Restart { .. } | NeedsYouAct::ApprovePr { .. } | NeedsYouAct::Steps { command: Some(_), .. });
    if to_do {
        NeedsYouAct::Done { steps: item.steps.clone() }
    } else {
        pressed
    }
}

fn mock_restart(command: &str) -> Option<(&'static str, ipc::BuildSource, bool)> {
    let mut words = command.split_whitespace();
    let script = words.next().unwrap_or_default();
    let flags: Vec<&str> = words.collect();
    let (label, build, flag) = if script.ends_with("scripts/preview") {
        ("Refresh preview", ipc::BuildSource::Preview, "--restart")
    } else if script.ends_with("scripts/restart") {
        ("Update to main", ipc::BuildSource::Main, "--main")
    } else {
        return None;
    };
    (flags.contains(&flag) && flags.iter().all(|one| *one == flag || *one == "--adopt")).then_some((label, build, flags.contains(&"--adopt")))
}

/// Where a row goes and how it is put away, by its source.
fn mock_place(item: &WaitingItem, act: &NeedsYouAct) -> (NeedsYouSectionKind, NeedsYouDismiss) {
    use NeedsYouDismiss as Dismiss;
    use NeedsYouSectionKind as Section;
    let by_act = || match act {
        NeedsYouAct::Restart { .. } if item.source == WaitingSource::PreviewOffer => Section::Commands,
        NeedsYouAct::Restart { .. } => Section::Restart,
        NeedsYouAct::Walk { .. } => Section::Walks,
        NeedsYouAct::Done { .. } => Section::ToDo,
        NeedsYouAct::Run { .. } | NeedsYouAct::Steps { .. } => Section::Commands,
        NeedsYouAct::ApprovePr { .. } => Section::PullRequests,
        _ => Section::Questions,
    };
    match item.source {
        WaitingSource::Agent => (by_act(), Dismiss::Reason),
        WaitingSource::Replied => (by_act(), if matches!(act, NeedsYouAct::ApprovePr { .. }) { Dismiss::Plain } else { Dismiss::NotAQuestion }),
        WaitingSource::PreviewOffer => (by_act(), Dismiss::Skip),
        WaitingSource::Walk => (Section::Walks, Dismiss::Plain),
        WaitingSource::Draft => (Section::ToReview, Dismiss::Reason),
        WaitingSource::Sketch | WaitingSource::Document | WaitingSource::Plan => (Section::ToReview, Dismiss::Plain),
        WaitingSource::Done => (Section::Done, Dismiss::Plain),
        WaitingSource::WordsAddress | WaitingSource::FleetCall | WaitingSource::WatchNotOurs | WaitingSource::BrowserAsk => (Section::Questions, Dismiss::Plain),
        WaitingSource::RuleAct | WaitingSource::OperatorSuggestion => (Section::Questions, Dismiss::Skip),
        _ => (Section::Questions, Dismiss::None),
    }
}

fn mock_glyph(item: &WaitingItem, act: &NeedsYouAct) -> NeedsYouGlyph {
    use NeedsYouGlyphKind as Kind;
    let glyph = |kind, said: &str| NeedsYouGlyph { kind, said: said.to_string() };
    match item.source {
        WaitingSource::Plan => return glyph(Kind::Plan, "Plan"),
        WaitingSource::Sketch => return glyph(Kind::Sketch, "Sketch"),
        WaitingSource::Draft => return glyph(Kind::Draft, "Draft"),
        WaitingSource::Document => return glyph(Kind::Document, "Document"),
        WaitingSource::Done => return glyph(Kind::Done, "Done"),
        WaitingSource::OperatorSuggestion => return glyph(Kind::Operator, "What the Operator would do"),
        _ => {}
    }
    let asks = matches!(act, NeedsYouAct::Answer { options, .. } if !options.is_empty());
    let replies = matches!(act, NeedsYouAct::Reply);
    match act {
        NeedsYouAct::Done { .. } => glyph(Kind::Todo, "To do"),
        NeedsYouAct::Restart { label, .. } => glyph(Kind::Restart, label),
        _ if item.source == WaitingSource::Replied && (asks || replies) => glyph(Kind::ReadFromReply, "Read from the reply"),
        _ if asks => glyph(Kind::Question, "Question"),
        _ if item.source == WaitingSource::HeldChild => glyph(Kind::LinkedJob, "Opens the linked Job"),
        NeedsYouAct::Reply => glyph(Kind::Reply, "Reply"),
        NeedsYouAct::Walk { .. } => glyph(Kind::Walk, "Opens the walk"),
        NeedsYouAct::Answer { .. } => glyph(Kind::Answer, "Goes to the question"),
        NeedsYouAct::ApprovePr { .. } => glyph(Kind::PullRequest, "Opens the pull request"),
        NeedsYouAct::Run { .. } => glyph(Kind::Run, "Copies the command"),
        NeedsYouAct::Message | NeedsYouAct::OpenJob { .. } => glyph(Kind::Message, "Goes to the message"),
        NeedsYouAct::Steps { .. } => glyph(Kind::Steps, "Steps"),
        _ => glyph(Kind::Waiting, "Waiting on you"),
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_mock_job_s_needs_you_is_grouped_in_fleet_s_order_and_a_put_away_row_is_left_out() {
        use ipc::{NeedsYouAct, NeedsYouDismiss, NeedsYouSectionKind as Section};
        let jobs = super::jobs("jobs-screens").expect("decodes");
        let waiting = &jobs.iter().find(|job| job.id.as_str() == "mock-01").expect("the first Job").waiting_for;
        let view = super::needs_you("mock-01", waiting, &|_| false);
        let kinds: Vec<Section> = view.sections.iter().map(|section| section.kind).collect();
        assert_eq!(kinds, [Section::Questions, Section::ToDo, Section::ToReview, Section::Walks, Section::Commands, Section::PullRequests]);
        assert!(view.fold && view.review_in_band);
        let todo = &view.sections[1].rows[0];
        assert!(matches!(&todo.act, NeedsYouAct::Done { steps } if steps.len() == 2));
        assert_eq!(todo.dismiss, NeedsYouDismiss::Reason);
        let walk = view.sections.iter().find(|section| section.kind == Section::Walks).expect("walks").rows[0].id.clone();
        let less = super::needs_you("mock-01", waiting, &|id| id == walk);
        assert!(less.sections.iter().all(|section| section.kind != Section::Walks));
        assert_eq!(less.glyphs.len(), view.glyphs.len() - 1);
    }

    #[test]
    fn the_several_sketches_scenario_lists_each_open_sketch_newest_first_and_pins_the_newest() {
        let jobs = super::jobs("sketches-several").expect("decodes");
        let job = jobs.iter().find(|job| job.id.as_str() == "tool-edit").expect("the Edit Job");
        assert_eq!(job.waiting_for.iter().map(|item| item.id.as_str()).collect::<Vec<_>>(), ["sketch:Freshness chip", "sketch:Merge line flow"]);
        let thread = super::threads("sketches-several").expect("decodes").remove("tool-edit").expect("its thread");
        assert_eq!(thread.head.sketch.as_ref().map(|card| card.sketch.as_str()), Some("sketch:Freshness chip"));
        let kept: Vec<(&str, bool)> = thread.items.iter().filter_map(|item| if let ipc::ThreadItem::Sketch(card) = item { Some((card.sketch.as_str(), card.asked)) } else { None }).collect();
        assert_eq!(kept, [("sketch:Merge line flow", true)]);
    }

    #[test]
    fn a_plan_alone_is_to_review_and_the_band_does_not_offer_the_sheet() {
        let jobs = super::jobs("needs-plan").expect("decodes");
        let view = super::needs_you("mock-01", &jobs[0].waiting_for, &|_| false);
        assert!(!view.fold && !view.review_in_band && view.glyphs.is_empty());
    }

    #[test]
    fn every_scenario_decodes_on_this_build_s_wire() {
        assert!(super::jobs("jobs-many").expect("decodes").len() > 10);
        let view = super::threads("job-view").expect("decodes");
        for id in ["mock-01", "mock-02", "mock-04"] {
            assert!(view.contains_key(id), "{id}");
        }
        assert!(super::jobs("job-view-composer").expect("decodes").len() > 10);
        assert!(super::models().is_some_and(|models| models.offered.len() == 3));
        assert!(super::jobs("jobs-screens").expect("decodes").len() > 10);
        let screens = super::threads("jobs-screens").expect("decodes");
        for id in ["mock-01", "mock-02", "mock-04", "mock-06"] {
            assert!(screens.contains_key(id), "{id}");
        }
        assert!(super::catalogue().is_some());
        assert_eq!(super::jobs("tool-rows").expect("decodes").len(), 7);
        assert_eq!(super::threads("tool-rows").expect("decodes").len(), 7);
        assert_eq!(super::threads("drafts").expect("decodes").len(), 1);
        assert!(super::jobs("job-tabs").expect("decodes").len() > 10);
        let tabs = super::threads("job-tabs").expect("decodes");
        for id in ["mock-01", "mock-02", "mock-04", "mock-06"] {
            assert!(tabs.contains_key(id), "{id}");
        }
        assert!(super::jobs("job-done").expect("decodes").len() > 10);
        assert!(!super::threads("job-done").expect("decodes").is_empty());
        assert!(super::jobs("job-now").expect("decodes").len() > 10);
        assert!(super::threads("job-now").expect("decodes").len() > 5);
        assert!(super::jobs("job-open").expect("decodes").len() > 10);
        assert!(super::threads("job-open").expect("decodes").len() > 5);
        assert!(super::jobs("job-watching").expect("decodes").iter().any(|one| one.waiting_for.iter().any(|item| item.source == ipc::WaitingSource::WatchNotOurs)));
        assert_eq!(super::watches("mock-01").map(|list| list.len()), Some(4));
        assert!(super::watch_log("w-release", "/Users/nick/.armada/watches/w-release/2026-11-09T13-41-00.log").is_some());
    }

    #[test]
    fn workflow_card_scenario_decodes_with_its_change() {
        assert!(super::jobs("workflow-card").expect("decodes").len() > 10);
        let threads = super::threads("workflow-card").expect("decodes");
        let changes = super::workflow_changes().expect("decodes");
        let change = changes.get(&31).expect("change 31");
        let shown = threads["mock-04"].items.iter().find_map(|item| match item {
            ipc::ThreadItem::Row { row: ipc::SessionRow::Draft { text, change: Some(31), .. } } => Some(text.clone()),
            _ => None,
        });
        // The card draws lanes only where the change drafts the text the row shows.
        assert_eq!(change.after.as_deref(), shown.as_deref());
        assert_eq!(change.after_steps.as_ref().map(Vec::len), Some(5));
    }
}

/// The System and Bays tabs' reads in `job-system`: each Job's system answer and its graphs' past,
/// its time, the pool, a bay's history, the clashes and a clash's file. Times are written against
/// `anchor` and read as that long before now.
#[derive(Deserialize)]
struct SystemReads {
    anchor: String,
    systems: Vec<SystemSeed>,
    times: Vec<ipc::JobTime>,
    pool: Vec<ipc::WorktreeSlot>,
    histories: Vec<ipc::BayHistory>,
    clashes: ipc::AllWriteClashes,
    clash_files: Vec<ClashFileRead>,
    logs: Vec<LogRead>,
}

/// One Job's system as the mock's Fleet answers it: after `first_read_ms`, its reading with the
/// graphs' past readings, or its refusal.
#[derive(Deserialize, Clone)]
pub struct SystemSeed {
    session_id: String,
    pub first_read_ms: u64,
    #[serde(default)]
    pub cpu: Vec<f64>,
    #[serde(default)]
    pub memory: Vec<f64>,
    #[serde(default)]
    pub system: Option<ipc::SessionSystem>,
    #[serde(default)]
    pub refused: Option<String>,
}

#[derive(Deserialize)]
struct ClashFileRead {
    other: String,
    path: String,
    #[serde(flatten)]
    file: ipc::ClashFile,
}

#[derive(Deserialize)]
struct LogRead {
    path: String,
    text: String,
}

fn system_reads() -> Option<SystemReads> {
    SCENARIO.lock().is_ok_and(|named| named.as_str() == "job-system").then(|| {
        ipc::decode::<SystemReads>("system reads", include_bytes!("../fixtures/system-reads.json")).unwrap_or_else(|why| panic!("the system reads fixture does not decode: {why}"))
    })
}

#[derive(Deserialize)]
struct WatchReads {
    asks: Vec<WatchAsk>,
    lists: Vec<ipc::WatchList>,
    logs: Vec<ipc::WatchLog>,
}

#[derive(Deserialize)]
struct WatchAsk {
    session_id: String,
    item: WaitingItem,
}

fn watch_reads() -> WatchReads {
    ipc::decode::<WatchReads>("watch reads", include_bytes!("../fixtures/watches.json")).unwrap_or_else(|why| panic!("the watches fixture does not decode: {why}"))
}

/// A Job's watches as `GET /watches?session_id=&ended=true` answers them, moved to the run's clock.
/// Only `job-watching` holds any.
pub fn watches(job: &str) -> Option<Vec<ipc::WatchSummary>> {
    if !SCENARIO.lock().is_ok_and(|named| named.as_str() == "job-watching") {
        return None;
    }
    let mut list = watch_reads().lists.into_iter().find(|one| one.session_id.as_str() == job)?.watches;
    let moved = |reading: &mut ipc::WatchReading| match reading {
        ipc::WatchReading::Read { at, last_at, .. } | ipc::WatchReading::Broken { at, last_at, .. } => {
            *at = shifted(NOW_ANCHOR, at);
            *last_at = last_at.as_ref().map(|at| shifted(NOW_ANCHOR, at));
        }
    };
    for watch in &mut list {
        watch.started_at = shifted(NOW_ANCHOR, &watch.started_at);
        watch.last.iter_mut().for_each(moved);
        watch.readings.iter_mut().for_each(moved);
        for verdict in &mut watch.verdicts {
            verdict.at = shifted(NOW_ANCHOR, &verdict.at);
        }
    }
    Some(list)
}

/// A reading's log as `GET /watches/log?watch=&log=` answers it.
pub fn watch_log(watch: &str, log: &str) -> Option<ipc::WatchLog> {
    watch_reads().logs.into_iter().find(|one| one.watch == watch && one.log == log)
}

/// A fixture's time moved so its anchor is now.
fn shifted(anchor: &str, at: &ipc::Instant) -> ipc::Instant {
    let (Ok(anchor), Ok(at)) = (chrono::DateTime::parse_from_rfc3339(anchor), chrono::DateTime::parse_from_rfc3339(at.as_str())) else { return at.clone() };
    let moved = chrono::Utc::now() - (anchor - at);
    ipc::Instant::carried(moved.to_rfc3339_opts(chrono::SecondsFormat::Secs, true))
}

/// A Job's system as `GET /sessions/system` answers it, with the graphs' past.
pub fn system(job: &str) -> Option<SystemSeed> {
    system_reads()?.systems.into_iter().find(|one| one.session_id == job)
}

/// A Job's time as `GET /sessions/time` answers it.
pub fn job_time(job: &str) -> Option<ipc::JobTime> {
    system_reads()?.times.into_iter().find(|one| one.session_id == job)
}

/// The pool as `GET /worktrees` answers it.
pub fn pool() -> Option<Vec<ipc::WorktreeSlot>> {
    let reads = system_reads()?;
    let anchor = reads.anchor;
    Some(
        reads
            .pool
            .into_iter()
            .map(|mut slot| {
                slot.since = slot.since.as_ref().map(|at| shifted(&anchor, at));
                if let Some(stranded) = slot.stranded.as_mut() {
                    stranded.told = stranded.told.as_ref().map(|at| shifted(&anchor, at));
                }
                slot
            })
            .collect(),
    )
}

/// The clashes `write_clashes.changed` carries in `job-system`.
pub fn bay_clashes() -> Option<ipc::AllWriteClashes> {
    Some(system_reads()?.clashes)
}

/// A bay's history as `GET /slot_history` answers it.
pub fn bay_history(manifest: &str, slot: u32) -> Option<ipc::BayHistory> {
    let reads = system_reads()?;
    let anchor = reads.anchor;
    let mut history = reads.histories.into_iter().find(|one| one.manifest_id.as_str() == manifest && one.slot == slot)?;
    for entry in history.entries.iter_mut() {
        let (ipc::BayHistoryEntry::Leased { at, .. } | ipc::BayHistoryEntry::Released { at, .. } | ipc::BayHistoryEntry::Checkout { at, .. }) = entry;
        *at = shifted(&anchor, at);
    }
    Some(history)
}

/// A clash's file as `GET /clashes/file` answers it, by the other holder and the path.
pub fn clash_file(other: &str, path: &str) -> Option<ipc::ClashFile> {
    system_reads()?.clash_files.into_iter().find(|one| one.other == other && one.path == path).map(|one| one.file)
}

/// A Job's log, which only the mock reads: Fleet serves none.
fn log(path: &str) -> Option<String> {
    system_reads()?.logs.into_iter().find(|one| one.path == path).map(|one| one.text)
}

/// A pool act on the mock's pool, as `POST /worktrees/slots` would change it.
pub fn pool_act(pool: &mut Vec<ipc::WorktreeSlot>, manifest: &str, slot: u32, act: ipc::SlotAct) -> Result<Option<ipc::SlotPoolChanged>, (String, Option<&'static str>)> {
    let at = pool.iter().position(|one| one.manifest_id.as_str() == manifest && one.slot == slot).ok_or_else(|| (format!("slot-{slot} is not in the pool"), None))?;
    let mut released = None;
    match act {
        ipc::SlotAct::Close => pool[at].closed = true,
        ipc::SlotAct::Open => pool[at].closed = false,
        ipc::SlotAct::Remove => {
            if pool[at].stranded.as_ref().is_some_and(|stranded| !stranded.uncommitted.is_empty()) {
                return Err((format!("slot-{slot} holds uncommitted changes"), None));
            }
            pool.remove(at);
        }
        ipc::SlotAct::Release => {
            // The mock's hosted Sessions are mid-turn, so Fleet refuses to let their slot go.
            if let ipc::SlotHolding::HostedSession { session_title, .. } = &pool[at].held {
                let name = session_title.as_deref().unwrap_or("The job");
                return Err((format!("{name} is working"), Some("fleet.slot_session_working")));
            }
            let one = &mut pool[at];
            released = Some(ipc::SlotReleased { branch: one.branch.clone().unwrap_or_default(), saved: None });
            one.held = ipc::SlotHolding::Free;
            one.branch = None;
            one.since = None;
        }
        ipc::SlotAct::Add => {}
    }
    Ok(Some(ipc::SlotPoolChanged { manifest_id: ipc::ManifestId::carried(manifest), slot, released }))
}

/// A rescue act on the mock's pool, as `POST /worktrees/slots/rescue` would change it. Stash is
/// refused, as a push origin rejects is.
pub fn rescue_act(pool: &mut [ipc::WorktreeSlot], manifest: &str, slot: u32, act: ipc::RescueAct) -> Result<Option<ipc::SlotRescued>, String> {
    let one = pool.iter_mut().find(|one| one.manifest_id.as_str() == manifest && one.slot == slot).ok_or_else(|| format!("slot-{slot} is not in the pool"))?;
    let branch = one.branch.clone();
    let rescued = |committed: Option<&str>| ipc::SlotRescued { manifest_id: ipc::ManifestId::carried(manifest), slot, branch: branch.clone(), branch_kept: true, committed: committed.map(str::to_string) };
    match act {
        ipc::RescueAct::Start => {
            one.rescue = Some(ipc::SlotFinding {
                state: ipc::SlotFindingState::Reading,
                commit: "0000000".into(),
                uncommitted: true,
                cut: 0,
                read: Vec::new(),
                searched: Vec::new(),
                verdict: None,
                items: Vec::new(),
                summary: None,
                why: None,
                cost_micros: None,
            });
            Ok(None)
        }
        ipc::RescueAct::Stop => {
            if let Some(rescue) = one.rescue.as_mut() {
                rescue.state = ipc::SlotFindingState::Stopped;
            }
            Ok(None)
        }
        ipc::RescueAct::Stash => Err(format!("git push was refused: origin rejected {}", branch.clone().unwrap_or_default())),
        ipc::RescueAct::Scrap | ipc::RescueAct::PickUp => {
            let committed = (act == ipc::RescueAct::PickUp).then_some("4be1c07d2a90");
            one.held = ipc::SlotHolding::Free;
            one.stranded = None;
            one.rescue = None;
            one.branch = None;
            Ok(Some(rescued(committed)))
        }
    }
}

/// A workflow editor's marks on the lanes fixture, which Fleet serves only to the editor, so the Job view's
/// canvas can be seen drawing them.
fn lanes_marks(lanes: &mut ipc::JobLanesView) {
    for lane in &mut lanes.lanes {
        if lane.id == "w-smoke" {
            lane.pending = Some(ipc::LanePending::Removal);
        }
        for node in &mut lane.nodes {
            match node.id.as_str() {
                "workflow/plan" => node.differs = vec!["Mobile".into(), "sample-web".into()],
                "pr-88/open" => node.clash = Some("Web and Mobile differ".into()),
                _ => {}
            }
        }
    }
    if let Some(mut proposed) = lanes.rules.iter().find(|rule| rule.id == "ready:pr-88").cloned() {
        proposed.id = "ticket-done:i-shop-142".into();
        proposed.rule = "ticket-done".into();
        proposed.from = "pr-88/merged".into();
        proposed.to = "i-shop-142/done".into();
        proposed.label = "Ticket to Done".into();
        proposed.waits = None;
        proposed.proposed = true;
        proposed.pending = Some(ipc::LanePending::Keep);
        lanes.rules.push(proposed);
    }
}
