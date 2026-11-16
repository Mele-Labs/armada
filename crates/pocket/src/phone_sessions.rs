//! What the phone may see of a Session.
//!
//! **An allowlist, built field by field**, never by serialising Fleet's record and
//! removing what is unwanted, so a field Fleet adds tomorrow does not reach the
//! phone until somebody writes it here. There is no field for a transcript, a file, a message, a working directory or an
//! environment value, and so none to fill. A Terminal Session has an ask only
//! while Fleet holds its question: [`held_ask`] judges that, and both the row
//! and the answer route read it there.

use ipc::{
    AskedChoice, AskedQuestion, AttachmentState, FleetAway, HeldOpenBy, JobMark, CallAnswer, CallInFlight, PullRequestWait,
    SessionOrigin, SessionRecord, SessionStanding, SessionState, SessionStopped, SessionTurn, SessionWait, StartCondition,
    StopReason, StopRepair, StopRestart, WaitingItem, WaitingSource,
};
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Debug, Serialize)]
pub struct PhoneChoice {
    pub label: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub description: String,
}

#[derive(Debug, Serialize)]
pub struct PhoneQuestion {
    pub question: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub header: String,
    pub multi_select: bool,
    pub options: Vec<PhoneChoice>,
}

#[derive(Debug, Serialize)]
pub struct PhoneAsk {
    /// What an answer names.
    pub ask_id: String,
    /// The tool reached for, where the ask is permission to run one.
    pub tool: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub detail: String,
    /// Empty for a permission ask; the answer is then one of `offers`.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub questions: Vec<PhoneQuestion>,
    /// `allow_once` and `refuse`. The offer to remember a rule writes the
    /// owner's settings and is not made from a phone.
    pub offers: Vec<&'static str>,
}

/// One thing that keeps a Job open besides the owner: a fact the phone words itself.
#[derive(Debug, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PhoneHold {
    Watch { name: String },
    Job { name: String },
    /// `on` is `queue`, `auto_merge` or `checks`; `auto_merge` is set only for `checks`.
    PullRequest {
        number: u64,
        on: &'static str,
        #[serde(skip_serializing_if = "std::ops::Not::not")]
        auto_merge: bool,
    },
}

/// What starts a Job that waits on another, with what the phone needs to word it. The brief it
/// starts on and the model it starts under stay on the Mac.
#[derive(Debug, Serialize)]
pub struct PhoneStart {
    pub on: PhoneLink,
    /// `ends`, `plan`, `pushed` or `lands`.
    pub when: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub part_title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pull_number: Option<u64>,
}

/// Why a Job stopped and what Fleet is doing about it. What its process last said and the tasks it
/// left open stay on the Mac.
#[derive(Debug, Serialize)]
pub struct PhoneStopped {
    pub reason: StopReason,
    pub restart: StopRestart,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repair: Option<StopRepair>,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub gave_up: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub down: Option<FleetAway>,
}

#[derive(Debug, Serialize)]
pub struct PhoneSession {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repository: Option<String>,
    /// `hosted` or `terminal`.
    pub kind: &'static str,
    pub waiting: bool,
    /// A turn is running, whether or not it has gone quiet.
    pub working: bool,
    /// Instants, so the phone counts the ages itself.
    pub started_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ended_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub waiting_since: Option<String>,
    /// A hosted Session's, or a Terminal Session's while Fleet holds it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ask: Option<PhoneAsk>,
    /// Held on its start card until the owner begins it. Neither waiting nor working meanwhile.
    pub held: bool,
    /// The Job that started this one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from: Option<PhoneLink>,
    /// What its last answer had in its window, and that window. Absent until reported.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_tokens: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_window: Option<u64>,
    /// The agent said, through `all_done`, that it has nothing left. Only while the Job is live.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub done_since: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub done_summary: Option<String>,
    /// What Fleet derived of the Job's turn: `working`, `stuck`, `replied`, `done` or `idle`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub standing: Option<SessionStanding>,
    /// Parked by the owner: nothing is asked of him until he resumes it.
    pub paused: bool,
    /// The plan's limit holds it until this instant.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limited_until: Option<String>,
    /// What keeps it open besides the owner, in the order Fleet lists it.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub held_open_by: Vec<PhoneHold>,
    /// It starts when another Job's condition is met, and runs nothing until then.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub starts_when: Option<PhoneStart>,
    /// Why its agent would not start: the cause only, never the log lines.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_failed: Option<String>,
    /// Not running and not parked.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stopped: Option<PhoneStopped>,
    /// Its first open pull request whose Checks failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failing_pull: Option<u64>,
    /// Never spoken in: no title, no turn and nothing attached.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub blank: bool,
    /// The proposer is reading the request, and the Job starts on its answer.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub reading: bool,
    /// The state its row draws: mark, lamp and group, as Fleet read them.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mark: Option<JobMark>,
    /// The project it was started for, by name. Absent where it has none, or where Fleet would not list the projects.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project: Option<String>,
    /// An instant, so the phone counts the age itself.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_turn_at: Option<String>,
    /// The pull request it has standing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pull: Option<u64>,
    /// The issue it was started on.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub issue: Option<PhoneIssue>,
    /// The workflow step it is at, in the step's own words, else the workflow's name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub step: Option<String>,
    /// What it is doing now, else what it last said, else what it has done so far.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub now: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PhoneLink {
    pub id: String,
    pub title: String,
}

#[derive(Debug, Serialize)]
pub struct PhoneIssue {
    /// `#` and the number where the target is all digits, else the target as it is.
    pub number: String,
    /// Empty where the attachment carries none.
    pub title: String,
}

/// The ask a Session is held on. **A Terminal Session's counts only while Fleet
/// still holds it and its mod is listening**: `asked_at` plus `holding_for_seconds`, against `now` in Unix
/// seconds. After that the question is in the terminal and nowhere else.
pub fn held_ask(record: &SessionRecord, now: i64) -> Option<&CallInFlight> {
    match record.origin {
        SessionOrigin::Bridge => record.hosted.as_ref()?.asked.as_ref(),
        SessionOrigin::Terminal => {
            let terminal = record.terminal.as_ref()?;
            // A mod that is not listening will never collect the answer.
            let asked = terminal.asked.as_ref().filter(|_| terminal.listening)?;
            let since = asked.asked_at.to_domain().epoch_millis()?;
            let until = since.saturating_add(i64::try_from(asked.holding_for_seconds).ok()?.saturating_mul(1000));
            (now.saturating_mul(1000) < until).then_some(asked)
        }
    }
}

fn question(asked: &AskedQuestion) -> PhoneQuestion {
    PhoneQuestion {
        question: asked.question.clone(),
        header: asked.header.clone(),
        multi_select: asked.multi_select,
        options: asked.options.iter().map(choice).collect(),
    }
}

fn choice(offered: &AskedChoice) -> PhoneChoice {
    PhoneChoice {
        label: offered.label.clone(),
        description: offered.description.clone(),
    }
}

fn ask(held: &CallInFlight) -> PhoneAsk {
    PhoneAsk {
        ask_id: held.call.clone(),
        tool: held.tool.clone(),
        detail: held.detail.clone(),
        questions: held.questions.iter().map(question).collect(),
        offers: held
            .offers
            .iter()
            .filter_map(|offer| match offer {
                CallAnswer::AllowOnce => Some("allow_once"),
                CallAnswer::Refuse => Some("refuse"),
                // The phone has no folder picker.
                CallAnswer::AllowAndRemember
                | CallAnswer::AlwaysAllow
                | CallAnswer::AlwaysBlock
                | CallAnswer::AllowForSession
                | CallAnswer::AllowLikeThisForSession => None,
            })
            .collect(),
    }
}

fn hold(one: &HeldOpenBy) -> PhoneHold {
    match one {
        HeldOpenBy::Watch { name } => PhoneHold::Watch { name: name.clone() },
        HeldOpenBy::Job { name } => PhoneHold::Job { name: name.clone() },
        HeldOpenBy::PullRequest { number, on } => PhoneHold::PullRequest {
            number: *number,
            on: match on {
                PullRequestWait::Queue => "queue",
                PullRequestWait::AutoMerge => "auto_merge",
                PullRequestWait::Checks { .. } => "checks",
            },
            auto_merge: matches!(on, PullRequestWait::Checks { auto_merge: true }),
        },
    }
}

fn stopped(one: &SessionStopped) -> PhoneStopped {
    PhoneStopped {
        reason: one.reason,
        restart: one.restart,
        repair: one.repair,
        gave_up: one.gave_up,
        down: one.down.clone(),
    }
}

/// The first pull request it holds that has not merged and whose Checks failed, as Bridge reads
/// `failing` in `stateOf`.
fn failing_pull(record: &SessionRecord) -> Option<u64> {
    record
        .attachments
        .iter()
        .filter(|one| one.kind == "pr" && !matches!(one.state, AttachmentState::GivenBack | AttachmentState::Spent))
        .filter(|one| one.detail.get("state").map(String::as_str) != Some("merged"))
        .filter(|one| one.detail.get("checks").map(String::as_str) == Some("failed"))
        .find_map(|one| one.target.parse().ok())
}

fn start(wait: &SessionWait) -> PhoneStart {
    PhoneStart {
        on: PhoneLink { id: wait.on.id.as_str().to_string(), title: wait.on.title.clone() },
        when: match wait.condition {
            StartCondition::Ends => "ends",
            StartCondition::GroupDone { .. } | StartCondition::TaskDone { .. } => "plan",
            StartCondition::PrPushed => "pushed",
            StartCondition::PrLands => "lands",
        },
        part_title: wait.part_title.clone(),
        pull_number: wait.pull.as_ref().map(|pull| pull.number),
    }
}

/// What a live Job owes the person besides an ask, as Bridge counts it (`stateOf` in
/// `session-state.ts`): a slot choice (unless Fleet is creating it), the delivery question, a plan,
/// a sketch, a document or the done ask on its tasks holds the agent whatever its turn says; while
/// a turn runs or is stuck nothing else is owed yet; otherwise every item but the `done` Fleet
/// derives from the turn.
fn owed(record: &SessionRecord) -> Vec<&WaitingItem> {
    if record.state != SessionState::Live {
        return Vec::new();
    }
    let holds = |one: &&WaitingItem| match one.source {
        WaitingSource::SlotChoice => one.creating_since.is_none(),
        WaitingSource::Delivery
        | WaitingSource::Plan
        | WaitingSource::TaskDone
        | WaitingSource::Sketch
        | WaitingSource::Document => true,
        _ => false,
    };
    let items = || record.waiting_for.iter().filter(|one| one.source != WaitingSource::Done);
    let at_work = matches!(record.standing, Some(SessionStanding::Working | SessionStanding::Stuck))
        || matches!(record.hosted.as_ref().map(|h| &h.turn), Some(SessionTurn::Working { .. }));
    if at_work {
        items().filter(holds).collect()
    } else {
        items().collect()
    }
}

impl PhoneSession {
    pub fn of(record: &SessionRecord, repository: Option<&str>, projects: &BTreeMap<String, String>, now: i64) -> PhoneSession {
        let hosted = record.origin == SessionOrigin::Bridge;
        let held = record.hosted.as_ref().is_some_and(|facts| facts.start.is_some());
        // A Terminal Session is waiting when its mod says it asked, held or not.
        let waiting_on = match record.origin {
            SessionOrigin::Bridge => held_ask(record, now),
            SessionOrigin::Terminal => record.terminal.as_ref().and_then(|t| t.asked.as_ref()),
        }
        .filter(|_| record.state == SessionState::Live);
        // Paused, failed to start, stopped, waiting on another Job, held on its start card or held by
        // the plan's limit: nothing is asked of him, whatever it owes, as `stateOf` orders it.
        let paused = record.hosted.as_ref().is_some_and(|facts| facts.paused.is_some());
        let start_failed = record.hosted.as_ref().and_then(|facts| facts.start_failed.as_ref());
        let parked = held
            || paused
            || start_failed.is_some()
            || record.stopped.is_some()
            || record.waits.is_some()
            || record.held_until.is_some();
        let owed = owed(record);
        let since = waiting_on
            .map(|asked| asked.asked_at.as_str())
            .or_else(|| owed.iter().map(|one| &one.since).min().map(|at| at.as_str()))
            .filter(|_| !parked);
        PhoneSession {
            id: record.id.as_str().to_string(),
            title: record.title.clone(),
            repository: repository.map(str::to_string),
            kind: if hosted { "hosted" } else { "terminal" },
            waiting: since.is_some(),
            working: !parked && matches!(record.standing, Some(SessionStanding::Working | SessionStanding::Stuck)),
            started_at: record.started_at.as_str().to_string(),
            ended_at: record.ended_at.as_ref().map(|at| at.as_str().to_string()),
            waiting_since: since.map(str::to_string),
            ask: held_ask(record, now)
                .filter(|_| record.state == SessionState::Live)
                .map(ask),
            held,
            from: record.started.as_ref().and_then(|started| started.by.as_ref()).map(|by| PhoneLink {
                id: by.id.as_str().to_string(),
                title: by.title.clone(),
            }),
            context_tokens: record.usage.context_tokens,
            context_window: record.usage.context_window,
            done_since: record.done_since.as_ref().filter(|_| record.state == SessionState::Live).map(|at| at.as_str().to_string()),
            done_summary: record.done_summary.clone().filter(|_| record.state == SessionState::Live && record.done_since.is_some()),
            standing: record.standing,
            paused,
            limited_until: record.held_until.as_ref().map(|at| at.as_str().to_string()),
            held_open_by: record.held_open_by.iter().map(hold).collect(),
            starts_when: record.waits.as_ref().map(start),
            start_failed: start_failed.map(|failed| failed.cause.clone()),
            stopped: record.stopped.as_ref().map(stopped),
            failing_pull: failing_pull(record),
            blank: record.attachments.is_empty() && record.title.is_none() && record.last_turn_at.is_none(),
            reading: record.proposal.as_ref().is_some_and(|proposal| proposal.reading),
            mark: record.mark,
            project: record.hosted.as_ref().and_then(|hosted| hosted.project.as_ref()).and_then(|id| projects.get(id)).cloned(),
            last_turn_at: record.last_turn_at.as_ref().map(|at| at.as_str().to_string()),
            pull: standing(record, "pr").and_then(|target| target.parse().ok()),
            issue: record.attachments.iter().find(|one| one.kind == "issue").map(|one| PhoneIssue {
                number: if one.target.chars().all(|c| c.is_ascii_digit()) { format!("#{}", one.target) } else { one.target.clone() },
                title: one.detail.get("title").cloned().unwrap_or_default(),
            }),
            step: step(record),
            now: record
                .activity
                .as_ref()
                .and_then(|activity| activity.now.clone().or_else(|| activity.said.clone()))
                .or_else(|| record.so_far.clone()),
        }
    }
}

/// The target of the first attachment of `kind` that still stands.
fn standing<'a>(record: &'a SessionRecord, kind: &str) -> Option<&'a str> {
    record.attachments.iter().find(|one| one.kind == kind && one.state == AttachmentState::Standing).map(|one| one.target.as_str())
}

/// Where the Job is in its workflow: the step's label where Fleet gives a step, else the workflow's name.
fn step(record: &SessionRecord) -> Option<String> {
    let copy = record.workflow_copy.as_ref();
    let name = copy.map(|copy| copy.name.clone()).or_else(|| record.workflow.clone())?;
    Some(record.workflow_step.and_then(|at| copy?.steps.get(at as usize)).map_or(name, |step| step.label.clone()))
}
