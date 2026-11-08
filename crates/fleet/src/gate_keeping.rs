//! The gates a Job's plan names, kept as Fleet sees each close. `docs/concepts/session/the-plan.md`,
//! *Gates*.
//!
//! **A row is written once, from a fact Fleet saw, and never worked out again**: the rail placed a Job
//! afresh on every draw, so a fact that dropped out moved it backwards (spike 43). **Each gate is
//! watched on its own**, whatever the gates before it say, so a merge is kept while an earlier gate is
//! open. A fact closes one gate of its kind, the first open one in plan order, and no other; the
//! plan's approval closes every `plan_approved` gate of the version approved.
//!
//! **No loop of its own**: it hears the row changes the rules' tap hears, for a Job whose plan has an
//! open gate, and reads the store only when something a gate reads moved.

use std::collections::hash_map::DefaultHasher;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex};

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use ipc::{Event, PlanGate, SessionRecord, SessionRow, SessionVoice, DOCUMENT_SETTLED_AT};
use store::{Holder, KeptAttachment, KeptGate, KeptPlan, PLAN_APPROVED};
use tokio::sync::{mpsc, Notify};
use tokio::task::JoinHandle;

use crate::daemon::Fleet;
use crate::session_host::document_answers::approved_on_newest;
use crate::session_host::sketching::APPROVED as SKETCH_APPROVED;
use crate::session_host::walking::{replaced, walk_approval};

/// One fact a gate can close on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Seen {
    pub gate: PlanGate,
    /// What it is, such as `pr:3360`, so it closes one gate of its kind and no other.
    pub source: String,
    pub at: String,
    /// One line, as a person reads it.
    pub fact: String,
    pub commit: Option<String>,
}

/// Every fact on the Job a gate can close on: his approval of the plan, of every walk shown, of a
/// sketch and of a page's newest revision, and a pull request's merge. **Only what happened once the
/// plan was first recorded** (`since`): a walk he approved before there was a plan is not its gate's.
pub(crate) fn facts_seen(plan: &KeptPlan, rows: &[SessionRow], held: &[KeptAttachment], dismissed: &[String]) -> Vec<Seen> {
    let since = rows
        .iter()
        .find_map(|row| match row {
            SessionRow::Plan { at, .. } => Some(at.as_str().to_string()),
            _ => None,
        })
        .unwrap_or_else(|| plan.recorded_at.clone());
    let mut seen = Vec::new();
    if let Some(at) = &plan.approved_at {
        seen.push(Seen {
            gate: PlanGate::PlanApproved,
            source: format!("plan:v{}", plan.version),
            at: at.clone(),
            fact: format!("You approved plan v{}", plan.version),
            commit: None,
        });
    }
    // Every walk shown since stands approved, a window a newer one of its page replaced or one taken back
    // waiting on nothing: his last Approve is the fact.
    let shown: Vec<&str> = rows
        .iter()
        .filter_map(|row| match row {
            SessionRow::Window { url, at, .. } if at.as_str() >= since.as_str() => Some(url.as_str()),
            _ => None,
        })
        .collect();
    let waiting = shown
        .iter()
        .any(|url| !replaced(rows, url) && !dismissed.contains(&format!("walk:{url}")) && walk_approval(rows, url).is_none());
    if !waiting {
        if let Some((url, at)) = shown.iter().filter_map(|url| walk_approval(rows, url).map(|at| (*url, at))).max_by_key(|(_, at)| *at) {
            seen.push(Seen {
                gate: PlanGate::WalkApproved,
                source: format!("walk:{}", at.as_str()),
                at: at.as_str().to_string(),
                fact: format!("You approved the walk of {url}"),
                commit: None,
            });
        }
    }
    for row in rows {
        if let SessionRow::Message { id, at, from: SessionVoice::You, text, .. } = row {
            let Some(title) = text.strip_prefix(SKETCH_APPROVED) else { continue };
            if at.as_str() >= since.as_str() {
                let title = title.lines().next().unwrap_or_default().trim();
                seen.push(Seen {
                    gate: PlanGate::SketchApproved,
                    source: format!("sketch:{id}"),
                    at: at.as_str().to_string(),
                    fact: format!("You approved the sketch {title}"),
                    commit: None,
                });
            }
        }
    }
    for one in held {
        if one.kind == "artifact" && approved_on_newest(&one.detail) {
            let at = one.detail.get(DOCUMENT_SETTLED_AT).cloned().unwrap_or_else(|| one.changed_at.clone());
            let revision = one.detail.get(ipc::DOCUMENT_REVISION).cloned().unwrap_or_default();
            let title = one.detail.get("title").cloned().unwrap_or_else(|| one.target.clone());
            if at >= since {
                seen.push(Seen {
                    gate: PlanGate::DocumentApproved,
                    source: format!("document:{}@{revision}", one.target),
                    at,
                    fact: format!("You approved {title}, revision {revision}"),
                    commit: None,
                });
            }
        }
        if one.kind == "pr" && crate::build_record::merged(one) && one.changed_at >= since {
            seen.push(Seen {
                gate: PlanGate::Merged,
                source: format!("pr:{}", one.target),
                at: one.changed_at.clone(),
                fact: format!("Pull request #{} merged", one.target),
                commit: None,
            });
        }
    }
    seen
}

/// The gates `seen` closes now, that `kept` has not. The plan's approval closes each group gated on it
/// for the version approved; any other fact, oldest first, closes the first open gate of its kind in plan
/// order, and a fact a row already names closes nothing more.
pub(crate) fn closing(plan: &KeptPlan, kept: &[KeptGate], seen: &[Seen]) -> Vec<KeptGate> {
    let mut closed: Vec<KeptGate> = Vec::new();
    let is_closed = |closed: &[KeptGate], group: &str, gate: &str| {
        kept.iter().chain(closed).any(|one| one.closes(group, gate, plan.version))
    };
    let mut facts: Vec<&Seen> = seen.iter().collect();
    facts.sort_by(|a, b| a.at.cmp(&b.at));
    for fact in facts {
        let word = fact.gate.word();
        let row = |group: &str| KeptGate {
            group_id: group.to_string(),
            gate: word.to_string(),
            version: if word == PLAN_APPROVED { plan.version } else { 0 },
            closed_at: fact.at.clone(),
            fact: fact.fact.clone(),
            source: fact.source.clone(),
            commit: fact.commit.clone(),
            already_red: Vec::new(),
            skipped: false,
        };
        let gated = plan.groups.iter().filter(|group| group.gate.as_deref() == Some(word));
        if fact.gate == PlanGate::PlanApproved {
            for group in gated {
                if !is_closed(&closed, &group.id, word) {
                    closed.push(row(&group.id));
                }
            }
            continue;
        }
        if kept.iter().chain(&closed).any(|one| one.gate == word && one.source == fact.source) {
            continue;
        }
        if let Some(group) = gated.into_iter().find(|group| !is_closed(&closed, &group.id, word)) {
            closed.push(row(&group.id));
        }
    }
    closed
}

/// Whether the plan on the row has a gate this keeper closes that is still open.
fn waits_on_a_gate(record: &SessionRecord) -> bool {
    record.plan.as_ref().is_some_and(|plan| {
        plan.groups.iter().any(|group| !matches!(group.gate, PlanGate::None | PlanGate::ChecksGreen) && group.gate_closed.is_none())
    })
}

/// What of a row a gate reads: the plan's version and approval, what the Job holds, and what waits on
/// the person, which a walk's Approve settles.
fn look_of(record: &SessionRecord) -> u64 {
    let mut hash = DefaultHasher::new();
    if let Some(plan) = &record.plan {
        (plan.version, plan.approved).hash(&mut hash);
    }
    for one in &record.attachments {
        (&one.kind, &one.target, one.state == ipc::AttachmentState::Standing, &one.detail).hash(&mut hash);
    }
    for item in &record.waiting_for {
        item.id.hash(&mut hash);
    }
    hash.finish()
}

/// Start keeping the gates of Jobs that name any. Production starts it beside the rules' tap, and a test
/// calls [`Fleet::gates_kept`] itself.
pub fn keep_the_gates<H, V, W>(fleet: Arc<Fleet<H, V, W>>) -> JoinHandle<()>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    // A Job heard again while its sweep is owed is swept once.
    let owed: Arc<Mutex<HashSet<String>>> = Arc::default();
    let woken = Arc::new(Notify::new());
    let (tell, mut heard) = mpsc::unbounded_channel::<String>();
    let last: Mutex<HashMap<String, u64>> = Mutex::new(HashMap::new());
    fleet.events().tap(move |event| {
        let Event::SessionChanged(record) = event else { return };
        if !waits_on_a_gate(record) {
            return;
        }
        let look = look_of(record);
        let id = record.id.as_str().to_string();
        let changed = last.lock().map(|mut held| held.insert(id.clone(), look) != Some(look)).unwrap_or(true);
        if changed {
            let _ = tell.send(id);
        }
    });
    let queued = Arc::clone(&owed);
    let wake = Arc::clone(&woken);
    tokio::spawn(async move {
        while let Some(id) = heard.recv().await {
            if let Ok(mut owed) = queued.lock() {
                owed.insert(id);
            }
            wake.notify_one();
        }
    });
    // A push of a Job whose `checks_green` gate is open queues its Checks, which run behind every Check.
    let pushes = Arc::clone(&fleet);
    tokio::spawn(async move {
        loop {
            pushes.gate_runs().woken().await;
            pushes.gate_checks_started();
        }
    });
    tokio::spawn(async move {
        loop {
            woken.notified().await;
            let due: Vec<String> = owed.lock().map(|mut owed| owed.drain().collect()).unwrap_or_default();
            for id in due {
                fleet.gates_kept(&id).await;
            }
        }
    })
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
    /// Keep every gate of the Job whose fact Fleet now sees, and draw the Job again where one closed.
    /// Answers the gates it closed.
    pub(crate) async fn gates_kept(self: &Arc<Self>, id: &str) -> Vec<KeptGate> {
        let closed = {
            let mut store = self.store().lock().await;
            let Ok(Some(plan)) = store.session_plan(id) else { return Vec::new() };
            if !plan.groups.iter().any(|group| group.closed.is_none() && group.gate.is_some()) {
                return Vec::new();
            }
            let rows: Vec<SessionRow> = store
                .session_rows(id)
                .unwrap_or_default()
                .iter()
                .filter_map(|body| ipc::decode::<SessionRow>("a job row", body.as_bytes()).ok())
                .collect();
            let held = store.attachments_of(&Holder::session(id)).unwrap_or_default();
            let dismissed = store.dismissed_waiting(id).unwrap_or_default();
            let kept = store.session_gates(id).unwrap_or_default();
            let closing = closing(&plan, &kept, &facts_seen(&plan, &rows, &held, &dismissed));
            closing.into_iter().filter(|gate| store.close_session_gate(id, gate).unwrap_or(false)).collect::<Vec<_>>()
        };
        if !closed.is_empty() {
            let _ = self.published_hosted(id).await;
            self.gates_closed(id, &closed).await;
        }
        closed
    }
}

