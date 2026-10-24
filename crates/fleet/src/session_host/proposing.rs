//! The proposer at a Session's dispatch, and what a change made in a panel does to the agent.
//! `docs/concepts/job-proposer.md`, *A Session's request*; `docs/concepts/session.md`, *A
//! Session's proposal*.
//!
//! **Every Session is read once, at its first message.** A start with a request is read at the start. Any
//! other is read when its first message is sent (`reading_begun`, called from the send) or, for a Session an
//! agent started, from the agent's brief at creation. A fork, Helm, Setup, Fleet's own words and a Session's
//! Fleet-written brief are not read.
//!
//! **The press never waits on the call.** `/sessions/start` answers with the Session already
//! made and `proposal.reading` set; the call runs in a task. The dispatcher's first message is
//! written to the thread when it arrives and its turn is held here, and the process starts when
//! the answer lands, on the values the answer settled. A call that fails, times out or cannot be
//! read starts the Session on this machine's defaults and says why on Fleet's stderr. Nothing is
//! refused.
//!
//! **What the person set wins.** A title, model or effort on the start, or changed in a panel
//! while the call is out, is not overwritten. A field the proposer did fill is listed in
//! `proposal.picked` until the person changes it.
//!
//! **Every change made through a panel reaches the agent.** A workflow change is a message at
//! once; a model or effort change is a line on the turn that starts on it.

use std::collections::VecDeque;
use std::sync::Arc;

use adapter_traits::{AgentHarness, Ask, Delivery, Vcs, WorkProduct};
use api::{HostedSessions, Refusal};
use config::ResolvedWorkflow;
use core_model::{Component, Envelope, FieldValue, Level, Ulid, WorkflowId};
use ipc::{ManifestId, ProposedField, SessionRecord, SetSessionWorkflow, StartSession};
use ipc::{TimelineKind, TimelineTone};
use store::{KeptHosting, KeptSession, Store};

use super::serving::SESSION_COMMAND_REFUSED;
use super::uploads::Kept;
use crate::daemon::Fleet;
use crate::judging::watched;
use crate::session_links::LinkedBy;
use crate::session_proposer::{SessionBrief, SessionProposed};
use crate::timeline::{Fact, Subject};

/// A workflow id the Session's repository does not hold. A 422.
const WORKFLOW_NOT_HELD: &str = "fleet.session_workflow_not_held";

/// A workflow set while `features.workflows` is off. A 422.
const WORKFLOWS_OFF: &str = "fleet.session_workflows_off";

/// A Session whose proposal call is out. In memory: a Fleet that restarts reads nothing.
#[derive(Debug, Default)]
pub(crate) struct Reading {
    /// The fields the person set while the call was out, which the answer leaves alone.
    person: Vec<ProposedField>,
}

/// A message already on the thread whose turn waits for the proposal.
pub(crate) struct Held {
    pub text: String,
    pub kept: Kept,
    pub addressed: Vec<(ipc::SessionTag, Option<String>)>,
    pub ahead: Option<String>,
    /// What the agent reads in place of `text`, where Fleet adds to the words: a pinned note's.
    pub instead: Option<String>,
}

/// What a Session's runtime keeps while its proposal is being read and its held turns are sent.
#[derive(Default)]
pub(crate) struct ProposalState {
    pub reading: Option<Reading>,
    /// Turns held, oldest first. Drained in order before anything newer is sent.
    pub held: VecDeque<Held>,
    /// The held turns are being sent; a message arriving now joins their line.
    pub draining: bool,
    /// A reading was begun for this Session, whether or not it has landed. A Session is read once.
    pub asked: bool,
}

impl ProposalState {
    /// Whether a new message waits behind the proposal.
    pub(crate) fn holds_back(&self) -> bool {
        self.reading.is_some() || self.draining
    }

    /// The person set `field`: the answer leaves it alone.
    pub(crate) fn person_set(&mut self, field: ProposedField) {
        if let Some(reading) = &mut self.reading {
            if !reading.person.contains(&field) {
                reading.person.push(field);
            }
        }
    }

    pub(crate) fn begin(person: Vec<ProposedField>) -> ProposalState {
        ProposalState {
            reading: Some(Reading { person }),
            asked: true,
            ..ProposalState::default()
        }
    }
}

/// The fields of a hosting's `proposed` list.
pub(crate) fn picked_of(proposed: &str) -> Vec<ProposedField> {
    proposed
        .split(',')
        .filter_map(|one| ProposedField::from_wire(one.trim()))
        .collect()
}

fn listed(picked: &[ProposedField]) -> String {
    picked.iter().map(|one| one.as_wire()).collect::<Vec<_>>().join(",")
}

/// How a model or an effort reads in a line to the agent. An absent effort is the CLI's own.
fn shown(value: Option<&str>) -> &str {
    value.unwrap_or("the default")
}

/// The workflow's name where the repository holds it, else its id.
fn named(held: &std::collections::BTreeMap<WorkflowId, ResolvedWorkflow>, id: Option<&str>) -> String {
    match id {
        None => String::from("custom"),
        Some(id) => held
            .get(&WorkflowId::carried(Ulid::carried(id)))
            .map(|found| found.name().to_string())
            .unwrap_or_else(|| id.to_string()),
    }
}

/// What the agent is told of the workflow it follows. `changed` is a person's change to a Session
/// that was already working, and not the first turn's account of where it stands.
fn workflow_words(
    held: &std::collections::BTreeMap<WorkflowId, ResolvedWorkflow>,
    workflow: Option<&str>,
    changed: bool,
) -> Option<String> {
    let Some(id) = workflow else {
        return changed.then(|| {
            String::from("Fleet: this Job no longer follows a workflow. It is a custom Job.")
        });
    };
    let found = held.get(&WorkflowId::carried(Ulid::carried(id)))?;
    let steps: Vec<&str> = found.steps().iter().map(|step| step.label()).collect();
    let verb = if changed { "now follows" } else { "follows" };
    Some(format!(
        "Fleet: this Job {verb} the {} workflow. Its steps, in order: {}.",
        found.name(),
        steps.join(" -> ")
    ))
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
    /// `start_session`, reading `request` where the start is a plain one. **Answers at once**: the
    /// Session exists with `proposal.reading` set and the call runs in a task.
    pub(crate) async fn started_reading(
        self: Arc<Self>,
        mut start: StartSession,
    ) -> Result<SessionRecord, Refusal> {
        let plain = start.fork.is_none() && start.root.is_none();
        let request = start
            .request
            .take()
            .map(|words| words.trim().to_string())
            .filter(|words| !words.is_empty() && plain);
        // A Session that waits on another holds the request as its brief and sends nothing. The Session it
        // waits on is checked before anything is made, so a refused wait leaves no Session behind.
        let after = start.after.take().filter(|_| request.is_some());
        if let Some(after) = &after {
            self.after_checked("", after).await?;
        }
        let Some(request) = request else {
            return self.start_session(start).await;
        };
        // No proposer to ask is a start on the defaults, as a failed call is.
        if let Err(why) = self.proposing() {
            eprintln!("a job's request was not read: {why:?}");
            if after.is_some() && start.title.is_none() {
                start.title = Some(super::after::first_line(&request)).filter(|title| !title.is_empty());
            }
            let record = self.start_session(start).await?;
            self.issues_named(record.id.as_str(), &request, &[]).await;
            self.links_seen(record.id.as_str(), &request, LinkedBy::You).await;
            return self.after_begun(record, after, request).await;
        }
        start.request = Some(request.clone());
        let record = self.started_hosted(start, false).await?;
        let id = record.id.as_str().to_string();
        let fleet = Arc::clone(&self);
        let words = request.clone();
        tokio::spawn(async move { fleet.read_request(id, words).await });
        // The request is the person's first message, sent or not: a Session that waits holds it as its brief.
        self.issues_named(record.id.as_str(), &request, &[]).await;
        self.links_seen(record.id.as_str(), &request, LinkedBy::You).await;
        self.after_begun(record, after, request).await
    }

    /// A Session made to wait: the wait is kept and both Sessions are published, and the record answered is
    /// the one that carries it.
    async fn after_begun(
        self: &Arc<Self>,
        record: SessionRecord,
        after: Option<ipc::StartAfter>,
        brief: String,
    ) -> Result<SessionRecord, Refusal> {
        let Some(after) = after else {
            return Ok(record);
        };
        self.wait_begun(record.id.as_str(), after, brief, false, ipc::WaitBy::You, true).await
    }

    /// A Session's first message, read where the Session was not started with a request: an annotation's
    /// notes, a brief an agent wrote, a lesson, a plan group. **Once**, and only for a Session that has not
    /// run, is not a fork, is not Helm or Setup's and holds no proposal; what it already carries is the
    /// person's (or the agent's) and stands over the answer. `true` where a reading began, and the turn is
    /// held behind it as a dispatch's is. A missing proposer reads nothing and the Session starts as it is.
    pub(crate) fn reading_begun(
        self: &Arc<Self>,
        session: &KeptSession,
        hosting: &KeptHosting,
        brief: &str,
    ) -> bool {
        let brief = brief.trim();
        let readable = !brief.is_empty()
            && !hosting.ran
            && hosting.fork_of.is_none()
            && !hosting.helm
            && !hosting.setup
            && hosting.proposed.is_empty();
        if !readable {
            return false;
        }
        if let Err(why) = self.proposing() {
            self.session_warned(&session.id, "a job's first message was not read", format!("{why:?}"));
            return false;
        }
        let set = |value: &Option<String>| value.as_ref().is_some_and(|one| !one.trim().is_empty());
        let person: Vec<ProposedField> = [
            (ProposedField::Title, session.title.is_some()),
            (ProposedField::Workflow, hosting.workflow.is_some()),
            (ProposedField::Model, set(&hosting.model)),
            (ProposedField::Effort, set(&hosting.effort)),
        ]
        .into_iter()
        .filter_map(|(field, given)| given.then_some(field))
        .collect();
        {
            let runtime = self.hosts().of(&session.id);
            let mut state = runtime.state();
            if state.proposal.asked {
                return false;
            }
            state.proposal = ProposalState::begin(person);
        }
        let (fleet, id, words) = (Arc::clone(self), session.id.clone(), brief.to_string());
        tokio::spawn(async move { fleet.read_request(id, words).await });
        true
    }

    async fn read_request(self: Arc<Self>, id: String, request: String) {
        let read = self.proposed_for_session(&id, &request).await;
        if let Err(why) = &read {
            eprintln!("job {id}: the request was not read, so it starts on the defaults: {why}");
        }
        self.reading_ended(&id, &request, read.ok()).await;
    }

    /// One call, one reading. Whatever goes wrong is words for the log.
    async fn proposed_for_session(
        &self,
        id: &str,
        request: &str,
    ) -> Result<SessionProposed, String> {
        let proposing = self.proposing().map_err(|why| format!("{why:?}"))?;
        let (_, hosting) = self
            .session_and_hosting(id)
            .await
            .map_err(|_| String::from("the job is gone"))?;
        let served = self
            .served_named(Some(&ManifestId::carried(&hosting.manifest_id)))
            .map_err(|_| String::from("the repository is not served"))?;
        // With workflows off the proposer is shown none, so it never picks one.
        let workflows = if self.workflows_on() { served.workflows() } else { Default::default() };
        // His renames teach the title. One that will not read leaves the question as it was.
        let renames = self
            .store()
            .lock()
            .await
            .renames_for(&hosting.manifest_id, crate::session_proposer::RENAMES_SHOWN)
            .unwrap_or_default();
        let brief = SessionBrief::about(request, &workflows, &proposing.choices, &renames);
        let ask = Ask::put(
            proposing.model.clone(),
            brief.question(),
            proposing.environment.clone(),
        )
        .map_err(|_| String::from("there was nothing to ask"))?;
        let reply = watched(
            proposing.client.as_ref(),
            &ask,
            proposing.budget,
            &|_| {},
            std::future::pending::<()>(),
        )
        .await
        .map_err(|why| why.to_string())?;
        brief
            .read(&reply, &workflows, &proposing.choices)
            .map_err(|why| why.to_string())
    }

    /// The call is over. Settle the Session's values, publish it, and send the turns that waited.
    async fn reading_ended(
        self: &Arc<Self>,
        id: &str,
        request: &str,
        read: Option<SessionProposed>,
    ) {
        let runtime = self.hosts().of(id);
        let person = runtime
            .state()
            .proposal
            .reading
            .as_ref()
            .map(|reading| reading.person.clone())
            .unwrap_or_default();
        {
            let mut store = self.store().lock().await;
            if let (Ok(Some(mut session)), Ok(Some(mut hosting))) =
                (store.session(id), store.hosting(id))
            {
                let mut picked = picked_of(&hosting.proposed);
                let mut take = |field: ProposedField| {
                    let taken = !person.contains(&field);
                    if taken && !picked.contains(&field) {
                        picked.push(field);
                    }
                    taken
                };
                if let Some(proposed) = read {
                    if session.title.is_none() && take(ProposedField::Title) {
                        session.title = proposed.title;
                    }
                    if let Some(workflow) = proposed.workflow {
                        if take(ProposedField::Workflow) {
                            hosting.workflow = Some(workflow.as_str().to_string());
                        }
                    }
                    if hosting.model.is_none() {
                        if let Some(model) = proposed.model {
                            if take(ProposedField::Model) {
                                hosting.model = Some(model);
                            }
                        }
                    }
                    if hosting.effort.is_none() {
                        if let Some(effort) = proposed.effort {
                            if take(ProposedField::Effort) {
                                hosting.effort = Some(effort);
                            }
                        }
                    }
                }
                // A field the answer did not fill is not the proposer's.
                picked.retain(|field| match field {
                    ProposedField::Title => session.title.is_some(),
                    ProposedField::Workflow => hosting.workflow.is_some(),
                    ProposedField::Model => hosting.model.is_some(),
                    ProposedField::Effort => hosting.effort.is_some(),
                });
                hosting.proposed = listed(&picked);
                // The words stand in for a title that nothing gave.
                if session.title.is_none() {
                    session.title = super::serving::title_of(request);
                }
                let _ = store.keep_session(&session);
                let _ = store.keep_hosting(&hosting);
            }
        }
        // Held turns go first, in the order they came, and nothing newer overtakes them.
        {
            let mut state = runtime.state();
            state.proposal.reading = None;
            state.proposal.draining = true;
        }
        let _ = self.published_hosted(id).await;
        loop {
            let next = {
                let mut state = runtime.state();
                match state.proposal.held.pop_front() {
                    Some(held) => Some(held),
                    None => {
                        state.proposal.draining = false;
                        None
                    }
                }
            };
            let Some(held) = next else { break };
            if let Err(why) = self.deliver_turn(id, held).await {
                eprintln!("job {id}: a turn held for the proposal was not sent: {why:?}");
            }
        }
        let _ = self.published_hosted(id).await;
    }

    /// The person changed `field`: the answer to a call still out leaves it alone, and it is no
    /// longer the proposer's pick. Called with the store held, from the change itself.
    pub(crate) fn person_changed(&self, store: &mut Store, id: &str, field: ProposedField) {
        self.hosts().of(id).state().proposal.person_set(field);
        if let Ok(Some(mut hosting)) = store.hosting(id) {
            let mut picked = picked_of(&hosting.proposed);
            if picked.contains(&field) {
                picked.retain(|one| *one != field);
                hosting.proposed = listed(&picked);
                let _ = store.keep_hosting(&hosting);
            }
        }
    }

    /// A warning in the envelope on Fleet's standard error, about a Session. A Session has no Job log to
    /// write into, and what failed here raises nothing for the person.
    pub(crate) fn session_warned(&self, session: &str, msg: &str, why: impl std::fmt::Display) {
        let envelope = Envelope::new(self.now(), Level::Warn, Component::Fleet, self.run().clone(), msg)
            .with_field("session", FieldValue::Str(session.to_string()))
            .with_field("why", FieldValue::Str(why.to_string()));
        crate::transcript::note_aloud(&envelope);
    }

    /// An entry on the Session's timeline, for the thread to draw.
    pub(crate) fn session_noted(&self, session: &KeptSession, kind: TimelineKind, says: String) {
        self.timeline().note(
            Fact::new(
                self.timeline().now(),
                kind,
                Subject::Session(session.id.clone()),
                TimelineTone::Idle,
                says,
            )
            .titled(session.title.clone())
            .in_repository(session.manifest_id.clone()),
        );
    }

    /// The words a turn begins with when the Session's model or effort is not what the last turn
    /// ran on, and the first turn's account of its workflow. **Called where the turn is built**,
    /// so the line is on the very turn that runs on the change.
    ///
    /// The first turn compares against nothing: it is where the baseline is kept, and it tells the
    /// agent only the workflow. A change made while a turn runs waits for the process to restart
    /// on it, and the turn that starts then tells.
    pub(crate) async fn turn_told(&self, id: &str, hosting: &KeptHosting) -> Option<String> {
        let mut said: Vec<String> = Vec::new();
        if !hosting.ran && self.workflows_on() {
            if let Ok(served) = self.served_named(Some(&ManifestId::carried(&hosting.manifest_id))) {
                said.extend(workflow_words(&served.workflows(), hosting.workflow.as_deref(), false));
            }
        }
        if !self.hosts().of(id).state().restart_after_turn {
            said.extend(self.values_told(id).await);
        }
        (!said.is_empty()).then(|| said.join(" "))
    }

    /// Compare what this turn runs on with the last turn's, keep it, and enter what differs.
    async fn values_told(&self, id: &str) -> Option<String> {
        let mut store = self.store().lock().await;
        let (Ok(Some(session)), Ok(Some(mut hosting))) = (store.session(id), store.hosting(id)) else {
            return None;
        };
        let model = hosting
            .model
            .clone()
            .unwrap_or_else(|| self.session_model());
        let effort = hosting.effort.clone();
        let was = (hosting.turn_model.clone(), hosting.turn_effort.clone());
        if was.0.as_deref() == Some(model.as_str()) && was.1 == effort {
            return None;
        }
        hosting.turn_model = Some(model.clone());
        hosting.turn_effort = effort.clone();
        store.keep_hosting(&hosting).ok()?;
        drop(store);
        // The first turn's values are where the Session begins, not a change.
        let was_model = was.0?;
        let mut said = Vec::new();
        if was_model != model {
            self.session_noted(
                &session,
                TimelineKind::SessionModel,
                format!("model: {was_model} → {model}"),
            );
            said.push(format!("your model is {model}, not {was_model}"));
        }
        if was.1 != effort {
            self.session_noted(
                &session,
                TimelineKind::SessionEffort,
                format!("effort: {} → {}", shown(was.1.as_deref()), shown(effort.as_deref())),
            );
            said.push(format!(
                "your effort is {}, not {}",
                shown(effort.as_deref()),
                shown(was.1.as_deref())
            ));
        }
        (!said.is_empty()).then(|| format!("Fleet: from this turn {}.", said.join(" and ")))
    }

    /// `set_session_workflow`: the person picked the Session's workflow in a panel, or cleared it.
    pub(crate) async fn workflow_set(
        self: &Arc<Self>,
        set: SetSessionWorkflow,
    ) -> Result<SessionRecord, Refusal> {
        let id = set.session_id.as_str().to_string();
        if self.terminal_session(&id).await?.is_some() {
            return Err(self.hosted_refusal(
                SESSION_COMMAND_REFUSED,
                &format!("job {id} is a terminal job, and only a hosted one has a workflow"),
            ));
        }
        let (session, hosting) = self.session_and_hosting(&id).await?;
        if session.state == store::SessionState::Ended {
            return Err(self.closed(&id));
        }
        let held = self
            .served_named(Some(&ManifestId::carried(&hosting.manifest_id)))
            .map(|served| served.workflows())
            .unwrap_or_default();
        let workflow = set
            .workflow
            .map(|named| named.trim().to_string())
            .filter(|named| !named.is_empty());
        if workflow.is_some() && !self.workflows_on() {
            return Err(self.hosted_refusal(
                WORKFLOWS_OFF,
                "workflows are switched off in Settings, so every Job is a plain one",
            ));
        }
        if let Some(named) = &workflow {
            if !held.contains_key(&WorkflowId::carried(Ulid::carried(named.as_str()))) {
                return Err(self.hosted_refusal(
                    WORKFLOW_NOT_HELD,
                    &format!("this repository holds no workflow named `{named}`"),
                ));
            }
        }
        if hosting.workflow == workflow {
            return self.published_hosted(&id).await;
        }
        {
            let mut store = self.store().lock().await;
            let mut fresh = store
                .hosting(&id)
                .map_err(|why| self.ledger_fault(why))?
                .unwrap_or_else(|| hosting.clone());
            fresh.workflow = workflow.clone();
            store
                .keep_hosting(&fresh)
                .map_err(|why| self.ledger_fault(why))?;
            self.person_changed(&mut store, &id, ProposedField::Workflow);
        }
        self.session_noted(
            &session,
            TimelineKind::SessionWorkflow,
            format!(
                "workflow: {} → {}",
                named(&held, hosting.workflow.as_deref()),
                named(&held, workflow.as_deref())
            ),
        );
        // A Session that has not run is told with its first turn, which is when it starts.
        if hosting.ran {
            if let Some(words) = workflow_words(&held, workflow.as_deref(), true) {
                if let Err(why) = self.told_by_fleet(&id, &words, ipc::Notice::WorkflowChanged).await {
                    eprintln!("job {id}: the workflow change was not sent to the agent: {why:?}");
                }
            }
        }
        self.published_hosted(&id).await
    }
}
