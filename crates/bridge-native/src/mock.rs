//! Mock data: Fleet's own answers, written as JSON beside the crate and decoded through `ipc` as a
//! live answer is, so a fixture that no longer matches the wire fails to decode instead of drawing.
//! A screen on mock data is the same screen; only where its data comes from differs.

use std::collections::HashMap;

use std::sync::Mutex;

use ipc::{
    IssueList, JobThread, ModelChoices, NeedsYou, NeedsYouAct, NeedsYouDismiss, NeedsYouGlyph, NeedsYouGlyphKind, NeedsYouRow, NeedsYouSection,
    NeedsYouSectionKind, SessionList, SessionQuick, SessionRecord, SettingValue, WaitingActKind, WaitingItem, WaitingSource, DOCUMENT_ITEM_PREFIX,
};
use serde::Deserialize;

/// The scenario the last [`jobs`] call named, which the message box reads its catalogue by.
static SCENARIO: Mutex<String> = Mutex::new(String::new());

/// The Jobs of a named scenario, as `GET /sessions` would answer them.
pub fn jobs(scenario: &str) -> Result<Vec<SessionRecord>, String> {
    if let Ok(mut held) = SCENARIO.lock() {
        *held = scenario.to_string();
    }
    let bytes: &[u8] = match scenario {
        "jobs-many" | "job-view" | "job-reasoning" => include_bytes!("../fixtures/jobs-many.json"),
        // The same Jobs, with one handing off and one that took an earlier Job's place, that one ended behind it.
        "jobs-handoff" => include_bytes!("../fixtures/jobs-handoff.json"),
        // The same Jobs, with Try amber forked from the first at its message naming the tone rule.
        "history" => include_bytes!("../fixtures/jobs-history.json"),
        // The same Jobs, with Checkout page loads twice moved from sample-web to sample-api.
        "thread-moved" => include_bytes!("../fixtures/jobs-moved.json"),
        "job-bands" => include_bytes!("../fixtures/jobs-bands.json"),
        // The same Jobs, with the first holding a plan, a sketch, a document and two drafts to read.
        "review-palette" => include_bytes!("../fixtures/review-palette.json"),
        "job-view-composer" | "thread-presses" | "sent-messages" => include_bytes!("../fixtures/jobs-composer.json"),
        // The whole Jobs tour: the review palette's Jobs, with the Judge's reading on the first and
        // the second held Job holding its three waits.
        "jobs-screens" => include_bytes!("../fixtures/jobs-screens.json"),
        // One Job with only its plan waiting on him, as Fleet served it.
        "needs-plan" => include_bytes!("../fixtures/needs-plan.json"),
        // Five Jobs each running one kind of call after a finished one, and one whose run is over.
        "tool-rows" | "call-rows" | "sketch-card" | "sketch-share" | "sketch-changes" | "subagent" => include_bytes!("../fixtures/jobs-tools.json"),
        // One Job with an open draft and a workflow draft, near the thread's end.
        "drafts" => include_bytes!("../fixtures/jobs-drafts.json"),
        // The box's Jobs, with the first holding a plan, two pull requests and a Job it started with one of its own.
        "job-panes" => include_bytes!("../fixtures/jobs-panes.json"),
        other => return Err(format!("no mock scenario named {other}")),
    };
    ipc::decode::<SessionList>("SessionList", bytes).map(|list| list.sessions).map_err(|why| why.to_string())
}

/// The repositories Fleet would serve (`GET /repositories`) in the walks: the four the title bar's
/// switcher is drawn from, the last with no Manifest.
pub fn repositories() -> Vec<ipc::RepositorySummary> {
    ipc::decode::<ipc::RepositoryList>("RepositoryList", include_bytes!("../fixtures/repositories.json"))
        .unwrap_or_else(|why| panic!("the repositories fixture does not decode: {why}"))
        .repositories
}

/// His projects as `GET /projects` would answer them in the walks: two that read, over the repositories
/// above, and one whose file did not, which the switcher leaves out.
pub fn projects() -> ipc::Projects {
    ipc::decode::<ipc::Projects>("Projects", include_bytes!("../fixtures/projects.json"))
        .unwrap_or_else(|why| panic!("the projects fixture does not decode: {why}"))
}

/// The threads Fleet would serve for a scenario's Jobs, by Job id, as `GET /sessions/thread` answers
/// them. A Job with none stays on its skeleton, as one Fleet has not answered for does.
pub fn threads(scenario: &str) -> Result<HashMap<String, JobThread>, String> {
    #[derive(Deserialize)]
    struct Threads {
        threads: Vec<JobThread>,
    }
    let bytes: &[u8] = match scenario {
        "jobs-screens" => return screens_threads(),
        "job-view" | "review-palette" => include_bytes!("../fixtures/job-threads.json"),
        // The same threads, and the earlier Job's, ended where it handed on.
        "jobs-handoff" => include_bytes!("../fixtures/jobs-handoff-threads.json"),
        // The same threads, with the Job that took the router upgrade on and the forked one.
        "history" => include_bytes!("../fixtures/history-threads.json"),
        "thread-moved" => include_bytes!("../fixtures/moved-threads.json"),
        // The thread the box was walked against: no question or plan waiting, so the quick bar draws.
        "job-view-composer" | "job-panes" => include_bytes!("../fixtures/composer-threads.json"),
        // The box's threads, with Audit the docs index ending on a message that names a Job, holds a block and an address.
        "thread-presses" => include_bytes!("../fixtures/thread-presses-threads.json"),
        // The box's threads, with Audit the docs index ending on his message that tags a Job in its words and a pull request beside them.
        "sent-messages" => include_bytes!("../fixtures/sent-messages-threads.json"),
        "job-bands" => include_bytes!("../fixtures/bands-threads.json"),
        "needs-plan" => include_bytes!("../fixtures/needs-plan-threads.json"),
        "tool-rows" => include_bytes!("../fixtures/tools-threads.json"),
        // The tool-rows threads, with the finished Job's retro being written and Search for the queue asking with previews.
        "call-rows" => include_bytes!("../fixtures/call-rows-threads.json"),
        // The tool-rows threads, with Edit the merge line putting a sketch to him: every kind of node, arrows with words, his ink and steps.
        "sketch-card" => include_bytes!("../fixtures/sketch-card-threads.json"),
        "sketch-share" => include_bytes!("../fixtures/sketch-share-threads.json"),
        // The same, with Edit the merge line drawing the store clock twice: Pinned clock read against Store clock reads, which has two revisions.
        "sketch-changes" => include_bytes!("../fixtures/sketch-changes-threads.json"),
        // The tool-rows threads, with Brief a reviewer handing work to a reviewer that finished and an explorer still running.
        "subagent" => include_bytes!("../fixtures/subagent-threads.json"),
        "drafts" => include_bytes!("../fixtures/drafts-threads.json"),
        "job-reasoning" => include_bytes!("../fixtures/reasoning-threads.json"),
        _ => return Ok(HashMap::new()),
    };
    let read = ipc::decode::<Threads>("job threads", bytes).map_err(|why| why.to_string())?;
    Ok(read.threads.into_iter().map(|thread| (thread.session_id.as_str().to_string(), thread)).collect())
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

/// Whether a Job's thread is read again for a moment when it comes on screen, as Fleet's read takes
/// time: in `thread-moved`, so a walk can show the row that says new messages are loading.
pub fn reads_slowly() -> bool {
    SCENARIO.lock().is_ok_and(|named| named.as_str() == "thread-moved")
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
        _ => return None,
    };
    let reads = ipc::decode::<Reads<T>>("subagent reads", bytes).unwrap_or_else(|why| panic!("the subagent reads fixture does not decode: {why}"));
    reads.subagents.into_iter().find(|one| one.session_id == job && one.subagent_id == id).map(|one| one.read)
}

/// The scenario last named is the panes' own: their reads answer only there.
fn panes_named() -> bool {
    SCENARIO.lock().is_ok_and(|named| named.as_str() == "job-panes")
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
    if !panes_named() {
        return None;
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
    if !panes_named() {
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
    if !matches!(named.as_str(), "job-view-composer" | "jobs-screens" | "thread-presses" | "sent-messages" | "history" | "job-panes") {
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

fn mock_act(item: &WaitingItem, job: &str) -> NeedsYouAct {
    let named = |prefix: &str| item.id.strip_prefix(prefix).unwrap_or(&item.id).to_string();
    let command = item.act.as_ref().filter(|act| act.kind == WaitingActKind::Run).map(|act| act.target.clone());
    let pressed = match item.source {
        WaitingSource::Plan => NeedsYouAct::OpenPlan,
        WaitingSource::Sketch => NeedsYouAct::OpenSketch { title: named("sketch:") },
        WaitingSource::Document => NeedsYouAct::OpenDocument { path: named(DOCUMENT_ITEM_PREFIX) },
        WaitingSource::Draft => NeedsYouAct::OpenDraft { row: named("draft:") },
        _ if !item.options.is_empty() => NeedsYouAct::Answer { options: item.options.clone(), other: true },
        _ if !item.steps.is_empty() => NeedsYouAct::Steps { steps: item.steps.clone(), command: command.clone() },
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
    fn a_plan_alone_is_to_review_and_the_band_does_not_offer_the_sheet() {
        let jobs = super::jobs("needs-plan").expect("decodes");
        let view = super::needs_you("mock-01", &jobs[0].waiting_for, &|_| false);
        assert!(!view.fold && !view.review_in_band && view.glyphs.is_empty());
    }

    #[test]
    fn every_scenario_decodes_on_this_build_s_wire() {
        assert!(super::jobs("jobs-many").expect("decodes").len() > 10);
        assert_eq!(super::threads("job-view").expect("decodes").len(), 3);
        assert!(super::jobs("job-view-composer").expect("decodes").len() > 10);
        assert!(super::models().is_some_and(|models| models.offered.len() == 3));
        assert!(super::jobs("jobs-screens").expect("decodes").len() > 10);
        let screens = super::threads("jobs-screens").expect("decodes");
        for id in ["mock-01", "mock-02", "mock-04", "mock-06"] {
            assert!(screens.contains_key(id), "{id}");
        }
        assert!(super::catalogue().is_some());
        assert_eq!(super::jobs("tool-rows").expect("decodes").len(), 6);
        assert_eq!(super::threads("tool-rows").expect("decodes").len(), 6);
        assert_eq!(super::threads("drafts").expect("decodes").len(), 1);
    }
}
