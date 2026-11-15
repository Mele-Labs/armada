//! For you: one Job's own items, read off what Fleet already keeps. `docs/practices/protocol/for-you.md`.
//!
//! **Nothing here has to be filled in.** `waiting` is the Job's `waiting_for`, `building` the live Jobs it
//! started, `landing` its own open pull requests. The store keeps only what was said on top: the order, a
//! line, a withdrawal, and a row an agent added for what Fleet cannot see.

use std::collections::hash_map::DefaultHasher;
use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{
    AddForYou, ForYou, ForYouAdded, ForYouBuild, ForYouBy, ForYouChanged, ForYouItem, ForYouKind, ForYouLanding,
    ForYouOpens, Instant, LineForYou, OrderForYou, SessionId, SessionRecord, SessionState, WaitingActKind, WaitingItem,
    WaitingSource, WithdrawForYou, FOR_YOU_BLANK, FOR_YOU_UNKNOWN,
};

use super::agent_starting::STARTED_BY;
use super::waiting::SESSION_WAITING_UNNAMED;
use crate::daemon::Fleet;
use crate::started_jobs::STARTED_TO;

/// The prefix of a row an agent stated with `waiting_for`, so its id cannot meet one of Fleet's.
const AGENT_PREFIX: &str = "waiting:";
/// The prefix of a row `add_for_you` added.
const ADDED_PREFIX: &str = "added:";

/// What a waiting item is on the tab, and what its press opens. `None` for an item that is not his to act
/// on: `done:`, which the Job's own row says.
fn row_of(item: &WaitingItem, artifacts: &BTreeMap<String, String>) -> Option<(String, ForYouKind, ForYouOpens, ForYouBy)> {
    let waiting = || ForYouOpens::Waiting { id: item.id.clone() };
    let artifact = || artifacts.get(&item.id).map_or_else(waiting, |id| ForYouOpens::Artifact { id: id.clone() });
    let fleet = |kind, opens| Some((item.id.clone(), kind, opens, ForYouBy::Fleet));
    match item.source {
        WaitingSource::Done => None,
        WaitingSource::Agent => {
            let kind = match item.act.as_ref().map(|act| act.kind) {
                _ if item.approves => ForYouKind::Approve,
                Some(WaitingActKind::Run) => ForYouKind::Run,
                _ => ForYouKind::Question,
            };
            // An item naming one of the Job's artifacts opens it, and is one row with Fleet's item for it.
            let opens = item.artifact.clone().map_or_else(waiting, |id| ForYouOpens::Artifact { id });
            Some((format!("{AGENT_PREFIX}{}", item.id), kind, opens, ForYouBy::Agent))
        }
        WaitingSource::Walk => {
            let url = item.act.as_ref().map_or_else(|| item.id.trim_start_matches("walk:").to_string(), |act| act.target.clone());
            fleet(ForYouKind::Walk, ForYouOpens::Walk { url })
        }
        WaitingSource::Plan => fleet(ForYouKind::Plan, ForYouOpens::Plan),
        WaitingSource::Sketch => fleet(ForYouKind::Sketch, artifact()),
        WaitingSource::Document => fleet(ForYouKind::Document, artifact()),
        WaitingSource::Draft => fleet(ForYouKind::Draft, artifact()),
        _ => fleet(ForYouKind::Question, waiting()),
    }
}

/// `kind` as the store keeps it, and back.
fn kind_word(kind: ForYouKind) -> &'static str {
    match kind {
        ForYouKind::Walk => "walk",
        ForYouKind::Document => "document",
        ForYouKind::Plan => "plan",
        ForYouKind::Draft => "draft",
        ForYouKind::Sketch => "sketch",
        ForYouKind::Question => "question",
        ForYouKind::Run => "run",
        ForYouKind::Approve => "approve",
    }
}

fn kind_read(word: &str) -> ForYouKind {
    [
        ForYouKind::Walk,
        ForYouKind::Document,
        ForYouKind::Plan,
        ForYouKind::Draft,
        ForYouKind::Sketch,
        ForYouKind::Run,
        ForYouKind::Approve,
    ]
    .into_iter()
    .find(|kind| kind_word(*kind) == word)
    .unwrap_or(ForYouKind::Question)
}

/// `opens` as the store keeps it, and back.
fn opens_words(opens: &ForYouOpens) -> (&'static str, Option<&str>) {
    match opens {
        ForYouOpens::Artifact { id } => ("artifact", Some(id)),
        ForYouOpens::Walk { url } => ("walk", Some(url)),
        ForYouOpens::Waiting { id } => ("waiting", Some(id)),
        ForYouOpens::Plan => ("plan", None),
    }
}

fn opens_read(word: &str, target: Option<String>) -> ForYouOpens {
    let target = target.unwrap_or_default();
    match word {
        "artifact" => ForYouOpens::Artifact { id: target },
        "walk" => ForYouOpens::Walk { url: target },
        "plan" => ForYouOpens::Plan,
        _ => ForYouOpens::Waiting { id: target },
    }
}

/// Placed rows first in their order, then the rest oldest first.
fn ordered(mut rows: Vec<(Option<u32>, ForYouItem)>) -> Vec<ForYouItem> {
    rows.sort_by(|(a, one), (b, other)| match (a, b) {
        (Some(a), Some(b)) => a.cmp(b),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => one.since.as_str().cmp(other.since.as_str()),
    });
    rows.into_iter().map(|(_, row)| row).collect()
}

/// What a Job's own view is read from, as a print: its waiting items, its pull requests and the Jobs it
/// started. Two equal prints read the same view.
fn own_print(record: &SessionRecord) -> u64 {
    let mut hasher = DefaultHasher::new();
    format!("{:?}", record.state).hash(&mut hasher);
    for item in &record.waiting_for {
        (&item.id, &item.text, item.approves).hash(&mut hasher);
    }
    for one in record.attachments.iter().filter(|one| one.kind == "pr" || one.kind == STARTED_TO) {
        (&one.kind, &one.target, one.detail.get("title"), one.detail.get("state")).hash(&mut hasher);
        format!("{:?}", one.state).hash(&mut hasher);
    }
    hasher.finish()
}

/// What its starter's `building` reads of a Job, as a print.
fn build_print(record: &SessionRecord) -> u64 {
    let mut hasher = DefaultHasher::new();
    (format!("{:?}", record.state), &record.title, record.workflow_step).hash(&mut hasher);
    record.standing.map(|standing| format!("{standing:?}")).hash(&mut hasher);
    hasher.finish()
}

impl<H, V, W> Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    /// The Job a For you call is for: the one the connection places, else the one named.
    fn for_you_placed(&self, named: Option<&SessionId>) -> Result<String, Refusal> {
        api::asking()
            .and_then(|caller| self.session_holding(&caller))
            .or_else(|| named.map(|named| named.as_str().trim().to_string()))
            .filter(|id| !id.is_empty())
            .ok_or_else(|| self.hosted_refusal(SESSION_WAITING_UNNAMED, "a job to read For you for"))
    }

    /// The Job's record, read now.
    async fn for_you_record(&self, id: &str) -> Result<SessionRecord, Refusal> {
        let store = self.store().lock().await;
        let session = store
            .session(id)
            .map_err(|why| self.ledger_fault(why))?
            .ok_or_else(|| self.hosted_refusal(SESSION_WAITING_UNNAMED, "a job Fleet knows"))?;
        self.ledger_row(&store, &session)
    }

    /// `get_for_you` and every act's answer: the Job's own For you, read now.
    pub(crate) async fn for_you_of(&self, id: &str) -> Result<ForYou, Refusal> {
        let record = self.for_you_record(id).await?;
        let job = record.id.clone();
        if record.state == SessionState::Ended {
            return Ok(ForYou { job, waiting: Vec::new(), building: Vec::new(), landing: Vec::new() });
        }
        // A sketch, page or draft opens as its artifact: its waiting id is the artifact's `item`, whatever parts it holds.
        let artifacts: BTreeMap<String, String> = match record.waiting_for.iter().any(|item| {
            matches!(item.source, WaitingSource::Sketch | WaitingSource::Document | WaitingSource::Draft)
        }) {
            true => self
                .job_artifacts(id)
                .await?
                .into_iter()
                .filter_map(|one| Some((one.item?, one.id)))
                .collect(),
            false => BTreeMap::new(),
        };
        let (marks, added) = {
            let store = self.store().lock().await;
            (
                store.for_you_marks(id).map_err(|why| self.ledger_fault(why))?,
                store.for_you_added(id).map_err(|why| self.ledger_fault(why))?,
            )
        };
        let marks: BTreeMap<String, store::KeptForYouMark> = marks.into_iter().map(|one| (one.item.clone(), one)).collect();
        let mut rows: Vec<(Option<u32>, ForYouItem)> = Vec::new();
        let mut take = |id: String, kind, title: String, since: Instant, added_by, opens: ForYouOpens, approves| {
            let mark = marks.get(&id);
            if mark.is_some_and(|mark| mark.withdrawn) {
                return;
            }
            // One thing is one row: what a press opens names it, whoever added it.
            if rows.iter().any(|(_, row)| row.opens == opens) {
                return;
            }
            let place = mark.and_then(|mark| mark.place);
            let line = mark.and_then(|mark| mark.line.clone());
            rows.push((place, ForYouItem { id, kind, title, line, placed: place.is_some(), since, added_by, opens, approves }));
        };
        for item in &record.waiting_for {
            let Some((id, kind, opens, by)) = row_of(item, &artifacts) else { continue };
            let approves = item.approves || kind == ForYouKind::Plan;
            take(id, kind, item.text.clone(), item.since.clone(), by, opens, approves);
        }
        for one in added {
            let kind = kind_read(&one.kind);
            take(
                format!("{ADDED_PREFIX}{}", one.n),
                kind,
                one.title,
                Instant::carried(one.since),
                ForYouBy::Agent,
                opens_read(&one.opens, one.target),
                kind == ForYouKind::Approve,
            );
        }
        let mut building = Vec::new();
        for child in self.started_ids(id).await? {
            let Ok(started) = self.started_job(&child, id).await else { continue };
            if started.summary.state == SessionState::Ended {
                continue;
            }
            building.push(ForYouBuild {
                job: started.summary.id.clone(),
                title: started.summary.title.clone().unwrap_or_default(),
                standing: started.standing,
                line: started.step.map(|step| step.label),
            });
        }
        let landing = ipc::SessionSummary::of(&record)
            .pull_requests
            .into_iter()
            .map(|pr| ForYouLanding { pr: pr.number, title: pr.title })
            .collect();
        Ok(ForYou { job, waiting: ordered(rows), building, landing })
    }

    /// `get_for_you`.
    pub(crate) async fn for_you_asked(&self, named: Option<SessionId>) -> Result<ForYou, Refusal> {
        let id = self.for_you_placed(named.as_ref())?;
        self.for_you_of(&id).await
    }

    /// The view after an act, told to every client.
    async fn for_you_told(&self, id: &str) -> Result<ForYou, Refusal> {
        let view = self.for_you_of(id).await?;
        self.publish(ipc::Event::SessionForYou(ForYouChanged { session_id: view.job.clone() }));
        Ok(view)
    }

    /// Refuses with every id in `ids` that is not a row now.
    fn for_you_known(&self, view: &ForYou, ids: &[String]) -> Result<(), Refusal> {
        let unknown: Vec<&str> =
            ids.iter().map(|id| id.as_str()).filter(|id| !view.waiting.iter().any(|row| row.id == *id)).collect();
        if unknown.is_empty() {
            return Ok(());
        }
        let rows: Vec<&str> = view.waiting.iter().map(|row| row.id.as_str()).collect();
        Err(self.hosted_refusal(
            FOR_YOU_UNKNOWN,
            &format!("not a row on this Job's For you: {}. Its rows: {}", unknown.join(", "), if rows.is_empty() { "none".to_string() } else { rows.join(", ") }),
        ))
    }

    /// `withdraw_for_you`: kept for good, whether the id names a row or not.
    pub(crate) async fn for_you_withdrawn(&self, withdraw: WithdrawForYou) -> Result<ForYou, Refusal> {
        let id = self.for_you_placed(withdraw.session_id.as_ref())?;
        self.for_you_record(&id).await?;
        self.store().lock().await.withdraw_for_you(&id, withdraw.id.trim()).map_err(|why| self.ledger_fault(why))?;
        self.for_you_told(&id).await
    }

    /// `order_for_you`: refused whole where any id is not a row now.
    pub(crate) async fn for_you_ordered(&self, order: OrderForYou) -> Result<ForYou, Refusal> {
        let id = self.for_you_placed(order.session_id.as_ref())?;
        let ids: Vec<String> = order.ids.iter().map(|one| one.trim().to_string()).collect();
        self.for_you_known(&self.for_you_of(&id).await?, &ids)?;
        self.store().lock().await.order_for_you(&id, &ids).map_err(|why| self.ledger_fault(why))?;
        self.for_you_told(&id).await
    }

    /// `line_for_you`: a blank line clears it.
    pub(crate) async fn for_you_lined(&self, line: LineForYou) -> Result<ForYou, Refusal> {
        let id = self.for_you_placed(line.session_id.as_ref())?;
        let row = line.id.trim().to_string();
        self.for_you_known(&self.for_you_of(&id).await?, std::slice::from_ref(&row))?;
        let said = Some(line.line.trim()).filter(|said| !said.is_empty());
        self.store().lock().await.line_for_you(&id, &row, said).map_err(|why| self.ledger_fault(why))?;
        self.for_you_told(&id).await
    }

    /// `add_for_you`: the row listed already that opens the same thing, or a new one.
    pub(crate) async fn for_you_added(&self, add: AddForYou) -> Result<ForYouAdded, Refusal> {
        let id = self.for_you_placed(add.session_id.as_ref())?;
        let title = add.title.trim();
        if title.is_empty() {
            return Err(self.hosted_refusal(FOR_YOU_BLANK, "title needs what waits on him, in a line"));
        }
        if self.for_you_record(&id).await?.state == SessionState::Ended {
            return Err(self.closed(&id));
        }
        let line = add.line.as_deref().map(str::trim).filter(|line| !line.is_empty());
        let listed = self.for_you_of(&id).await?.waiting.into_iter().find(|row| row.opens == add.opens).map(|row| row.id);
        let row = {
            let mut store = self.store().lock().await;
            let row = match listed {
                Some(row) => row,
                None => {
                    let (opens, target) = opens_words(&add.opens);
                    let since = self.clock().now().as_str().to_string();
                    let new = store::NewForYouAdded { title, kind: kind_word(add.kind), opens, target, since: &since };
                    format!("{ADDED_PREFIX}{}", store.add_for_you(&id, new).map_err(|why| self.ledger_fault(why))?)
                }
            };
            if line.is_some() {
                store.line_for_you(&id, &row, line).map_err(|why| self.ledger_fault(why))?;
            }
            row
        };
        self.for_you_told(&id).await?;
        Ok(ForYouAdded { id: row })
    }

    /// After a `session.changed`: `session.for_you` for the Job where what its view reads moved, and for its
    /// starter where what `building` reads of it moved. Prints are held in memory, so the first change after a
    /// start tells once.
    pub(crate) fn for_you_after(&self, record: &SessionRecord) -> Vec<ipc::Event> {
        let id = record.id.as_str().to_string();
        let (own, build) = (own_print(record), build_print(record));
        let (own_moved, build_moved) = {
            let mut prints = self.for_you_prints().lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            let own_moved = prints.insert(format!("own:{id}"), own) != Some(own);
            let build_moved = prints.insert(format!("build:{id}"), build) != Some(build);
            (own_moved, build_moved)
        };
        let mut told = Vec::new();
        if own_moved {
            told.push(ipc::Event::SessionForYou(ForYouChanged { session_id: record.id.clone() }));
        }
        if build_moved {
            for starter in record.attachments.iter().filter(|one| one.kind == STARTED_BY) {
                told.push(ipc::Event::SessionForYou(ForYouChanged { session_id: SessionId::carried(starter.target.clone()) }));
            }
        }
        told
    }
}
