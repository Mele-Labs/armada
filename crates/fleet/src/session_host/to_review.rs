//! `list_to_review`: everything waiting on the person across a project's Jobs, in one list he answers in place.
//! `crates/ipc/operations/list_to_review.toml` is the contract.
//!
//! **Fed from Needs you, never beside it.** Each Job's rows are [`needs_you_of`] over the same `waiting_for`, so an item
//! here is a row there. This adds the Job, its sub project and features, the kind, and the operation that answers it.

use api::Refusal;
use ipc::{
    NeedsYouAct, NeedsYouRow, NeedsYouSectionKind as Section, ReadyState, SessionFeature, SessionId, ToReview, ToReviewAnswer as Answer,
    ToReviewCard, ToReviewGroup, ToReviewHead, ToReviewItem, ToReviewJob, ToReviewKind as Kind, ToReviewSubProject, WaitingItem, WaitingSource,
};
use store::{KeptSubProject, SessionSearch};

use super::needs_you::needs_you_of;
use crate::build_keeping::Forecasts;

/// One live Job with something waiting, as the list reads it off its row.
pub(crate) struct JobWaiting {
    pub id: SessionId,
    pub title: Option<String>,
    /// `SessionRecord.features`.
    pub features: Vec<SessionFeature>,
    /// `SessionRecord.waiting_for`, after his dismissals and in the Job's order.
    pub waiting: Vec<WaitingItem>,
    /// The projects its work is filed in, as `list_features` files them.
    pub projects: Vec<String>,
    /// Its open Ready to try cards, read only where it has the row.
    pub cards: Vec<ToReviewCard>,
}

/// What an item is. **Exhaustive**, so a new source is placed before it is built: a clash Fleet raises as its own source
/// lands here beside `job_conflict`.
fn kind_of(source: WaitingSource, section: Section) -> Kind {
    match source {
        WaitingSource::Plan => Kind::Plan,
        WaitingSource::Sketch => Kind::Sketch,
        WaitingSource::Document => Kind::Document,
        WaitingSource::Draft => Kind::Draft,
        WaitingSource::ReadyToTry => Kind::ReadyToTry,
        WaitingSource::JobConflict => Kind::Clash,
        WaitingSource::Agent
        | WaitingSource::AskCard
        | WaitingSource::Walk
        | WaitingSource::Permission
        | WaitingSource::WindowAsk
        | WaitingSource::Handoff
        | WaitingSource::FirstMessage
        | WaitingSource::Replied
        | WaitingSource::Done
        | WaitingSource::SlotChoice
        | WaitingSource::Delivery
        | WaitingSource::WordsAddress
        | WaitingSource::FleetCall
        | WaitingSource::TaskDone
        | WaitingSource::WatchNotOurs
        | WaitingSource::HeldChild
        | WaitingSource::PreviewOffer
        | WaitingSource::RuleAct
        | WaitingSource::OperatorSuggestion
        | WaitingSource::BrowserAsk
        | WaitingSource::OperatorUndone
        | WaitingSource::Gate
        | WaitingSource::JobHeld => match section {
            Section::Questions | Section::ToReview => Kind::Question,
            Section::ToDo => Kind::ToDo,
            Section::Walks => Kind::Walk,
            Section::Commands => Kind::Command,
            Section::PullRequests => Kind::PullRequest,
            Section::Restart => Kind::Restart,
            Section::Done => Kind::Done,
        },
    }
}

/// The operation that settles an item, as each source's own answer does. **Exhaustive**, as [`kind_of`] is.
fn answer_of(item: &WaitingItem, row: &NeedsYouRow, job: &str, cards: &[ToReviewCard]) -> Answer {
    let by_act = || match &row.act {
        NeedsYouAct::Done { .. } => Answer::DoneWaiting,
        NeedsYouAct::Answer { .. } | NeedsYouAct::Reply => Answer::AnswerWaiting,
        _ => Answer::Press,
    };
    match item.source {
        WaitingSource::Plan | WaitingSource::Sketch | WaitingSource::Document | WaitingSource::Draft | WaitingSource::Walk => Answer::AnswerItems,
        WaitingSource::Agent if item.approves => Answer::AnswerItems,
        WaitingSource::Agent | WaitingSource::Replied | WaitingSource::OperatorUndone => by_act(),
        WaitingSource::ReadyToTry => Answer::AnswerReadyToTry { cards: cards.to_vec() },
        WaitingSource::FirstMessage => Answer::BeginSession { session_id: SessionId::carried(job) },
        WaitingSource::HeldChild => match &item.act {
            Some(act) => Answer::BeginSession { session_id: SessionId::carried(&act.target) },
            None => Answer::Press,
        },
        WaitingSource::Done => Answer::DismissWaiting,
        WaitingSource::PreviewOffer => Answer::Press,
        WaitingSource::AskCard
        | WaitingSource::Permission
        | WaitingSource::WindowAsk
        | WaitingSource::Handoff
        | WaitingSource::SlotChoice
        | WaitingSource::Delivery
        | WaitingSource::WordsAddress
        | WaitingSource::FleetCall
        | WaitingSource::TaskDone
        | WaitingSource::WatchNotOurs
        | WaitingSource::RuleAct
        | WaitingSource::OperatorSuggestion
        | WaitingSource::BrowserAsk
        | WaitingSource::Gate
        | WaitingSource::JobConflict
        | WaitingSource::JobHeld => Answer::AnswerWaiting,
    }
}

/// Where a group stands: sub projects first, oldest started first, then features in the library's order, then neither.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Place {
    SubProject(usize),
    Feature(String, u32, String, u32, String),
    Neither,
}

fn feature_place(feature: &SessionFeature) -> Place {
    Place::Feature(feature.project.clone(), feature.area_place, feature.area.clone(), feature.place, feature.slug.clone())
}

/// The sub project a Job is one of: the first he picked it for, else the first with a feature it writes to now, as the
/// Jobs list joins it.
fn sub_project_of(subs: &[KeptSubProject], job: &JobWaiting) -> Option<usize> {
    subs.iter().position(|sub| sub.jobs.iter().any(|one| one == job.id.as_str())).or_else(|| {
        subs.iter().position(|sub| job.features.iter().any(|feature| feature.project == sub.project && sub.features.contains(&feature.slug)))
    })
}

/// A sub project as an item names it. `goal` waits on Fleet keeping one.
fn sub_project_wire(sub: &KeptSubProject) -> ToReviewSubProject {
    ToReviewSubProject { id: sub.id.clone(), name: sub.name.clone(), goal: None }
}

/// The list from each Job's `waiting_for`. **Every item of every Job is one item of exactly one group.**
pub(crate) fn to_review_of(project: Option<String>, jobs: Vec<JobWaiting>, subs: &[KeptSubProject]) -> ToReview {
    let mut placed: Vec<(Place, ToReviewHead, ToReviewItem)> = Vec::new();
    for one in jobs {
        let job = one.id.as_str();
        let sub = sub_project_of(subs, &one);
        let first = one.features.iter().min_by_key(|feature| feature_place(feature));
        let (place, head) = match (sub, first) {
            (Some(at), _) => (Place::SubProject(at), ToReviewHead::SubProject(sub_project_wire(&subs[at]))),
            (None, Some(feature)) => (feature_place(feature), ToReviewHead::Feature(feature.clone())),
            (None, None) => (Place::Neither, ToReviewHead::Neither),
        };
        let filed = project.clone().or_else(|| one.projects.first().cloned());
        let title = one.title.clone().filter(|title| !title.trim().is_empty()).unwrap_or_else(|| job.to_string());
        for section in needs_you_of(job, &one.waiting).sections {
            for row in section.rows {
                let Some(item) = one.waiting.iter().find(|waiting| waiting.id == row.id) else { continue };
                let item = ToReviewItem {
                    kind: kind_of(item.source, section.kind),
                    answer: answer_of(item, &row, job, &one.cards),
                    order: item.order,
                    why: item.why.clone(),
                    job: ToReviewJob { id: one.id.clone(), title: title.clone() },
                    project: filed.clone(),
                    sub_project: sub.map(|at| sub_project_wire(&subs[at])),
                    features: one.features.clone(),
                    row,
                };
                placed.push((place.clone(), head.clone(), item));
            }
        }
    }
    // A place first, then the Job's own order, an unplaced item after the placed, then oldest first.
    placed.sort_by(|(a, _, one), (b, _, other)| {
        let key = |item: &ToReviewItem| (item.order == 0, item.order, item.row.since.as_str().to_string(), item.job.id.as_str().to_string());
        a.cmp(b).then_with(|| key(one).cmp(&key(other)))
    });
    let mut groups: Vec<(Place, ToReviewGroup)> = Vec::new();
    for (place, head, item) in placed {
        match groups.last_mut() {
            Some((last, group)) if *last == place => group.items.push(item),
            _ => groups.push((place, ToReviewGroup { head, items: vec![item] })),
        }
    }
    ToReview { project, groups: groups.into_iter().map(|(_, group)| group).collect() }
}

impl<H, V, W> crate::daemon::Fleet<H, V, W>
where
    H: adapter_traits::AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: adapter_traits::Vcs + adapter_traits::Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: adapter_traits::WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    /// `list_to_review`: this Mac's live Jobs in `project`, or every one, read row by row as `list_sessions` reads them.
    pub(crate) async fn to_review_listed(&self, project: Option<String>) -> Result<ToReview, Refusal> {
        let project = project.map(|one| one.trim().to_string()).filter(|one| !one.is_empty());
        let found = {
            let store = self.store().lock().await;
            store
                .find_sessions(&SessionSearch { manifest_id: None, state: Some(store::SessionState::Live), text: None })
                .map_err(|why| self.ledger_fault(why))?
        };
        let mut forecasts = Forecasts::default();
        let mut jobs = Vec::new();
        for session in &found {
            // One row's reads per hold of the lock, as `list_sessions` takes them, so other commands get in between.
            let (projects, reads) = {
                let store = self.store().lock().await;
                let projects = self.projects_of_job_in(&store, &session.id);
                if project.as_ref().is_some_and(|one| !projects.contains(one)) {
                    continue;
                }
                (projects, self.ledger_reads_with(&store, session, &mut forecasts)?)
            };
            let record = self.ledger_built(session, reads);
            if record.waiting_for.is_empty() {
                continue;
            }
            let cards = match record.waiting_for.iter().any(|one| one.source == WaitingSource::ReadyToTry) {
                true => self.open_cards_of(&session.id).await,
                false => Vec::new(),
            };
            jobs.push(JobWaiting { id: record.id, title: record.title, features: record.features, waiting: record.waiting_for, projects, cards });
        }
        let subs = self.store().lock().await.sub_projects(project.as_deref()).map_err(|why| self.ledger_fault(why))?;
        Ok(to_review_of(project, jobs, &subs))
    }

    /// The Job's Ready to try cards still open, past those he dismissed the row for.
    async fn open_cards_of(&self, job: &str) -> Vec<ToReviewCard> {
        let dismissed = self.store().lock().await.dismissed_waiting(job).unwrap_or_default();
        self.ready_cards_of(job)
            .await
            .into_iter()
            .filter(|card| card.state == ReadyState::Open && !dismissed.contains(&card.id))
            .map(|card| ToReviewCard { branch: card.branch, tip: card.tip, about: card.about })
            .collect()
    }
}
