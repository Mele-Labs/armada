//! A judge reading a Session's work, and what the person does with what it says. `docs/concepts/judge.md`,
//! *A Session's judge*; `docs/concepts/session.md`, *The judges*.
//!
//! **The same stateless call a Job's step is judged by**, on the Judge model setting. It is shown the
//! request the person dispatched with, the plan, the repository's standing rules and the diff against
//! the base, and **never the transcript**: `verification::Work` has no field for one. One call is made
//! for each question, and an answer that cannot be read raises nothing.
//!
//! **Advisory.** A concern is a waiting item. It reaches the agent only when the person sends it, as his
//! message; dismissing it tells the agent nothing. Either way it stays on the record as settled.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Ask, Delivery, Vcs, WorkProduct, Worktree};
use api::Refusal;
use core_model::{CriterionId, JudgeCriterion, OnRefusal};
use ipc::{DismissConcern, JudgeSession, ManifestId, SendConcern, SendSessionMessage, SessionRecord, SessionRow, SessionVoice};
use store::{KeptCite, KeptConcern, KeptReading};
use verification::{Brief, Work};

use super::serving::SESSION_COMMAND_REFUSED;
use super::Runtime;
use crate::daemon::Fleet;
use crate::judging::said;
use crate::rail_keeping::concern_number;

/// The most of a diff a reading is shown, in characters. A diff cut here says so.
const DIFF_AT_MOST: usize = 120_000;

/// The question the Judge tile always asks, beside the current step's own.
const ON_PLAN: &str = "Does the diff carry out the plan, and stay inside what the person asked for? Name \
anything the plan or the request asks for that the diff leaves out, and anything the diff changes that \
neither asks for. If the plan is empty, judge the diff against the request alone.";

/// Marks a reading as out for as long as it is held.
pub(crate) struct Reading {
    runtime: Arc<Runtime>,
}

impl Reading {
    pub(crate) fn begin(runtime: Arc<Runtime>) -> Reading {
        runtime.state().judging += 1;
        Reading { runtime }
    }
}

impl Drop for Reading {
    fn drop(&mut self) {
        let mut state = self.runtime.state();
        state.judging = state.judging.saturating_sub(1);
    }
}

/// What one reading asks.
pub(crate) struct Question {
    /// The step the Session just left, or `None` for the tile.
    pub from: Option<(u32, String)>,
    /// What the Session stands at, in the brief's own words.
    pub label: String,
    pub criteria: Vec<JudgeCriterion>,
}

impl Question {
    /// The tile: whether the work carries out the plan, and each question of the step the Session is on.
    pub(crate) fn tile(label: String, step: Vec<JudgeCriterion>) -> Question {
        let mut criteria = vec![JudgeCriterion {
            criterion_id: CriterionId::new("on_plan"),
            question: ON_PLAN.to_string(),
            on_refusal: OnRefusal::Ask,
        }];
        criteria.extend(step);
        Question { from: None, label, criteria }
    }
}

/// Where in the changed file a line of the brief's diff part falls: the file's path, and the line as the
/// change leaves it. `None` for a line under no file's header or outside a hunk.
pub(crate) fn place_in_diff(brief: &str, line: u32) -> Option<(String, u32)> {
    let mut path: Option<String> = None;
    let mut next: Option<u32> = None;
    for (at, text) in brief.lines().enumerate() {
        let at = at as u32 + 1;
        if at > line {
            break;
        }
        if let Some(header) = text.strip_prefix("+++ ") {
            path = header.strip_prefix("b/").map(str::to_string);
            next = None;
        } else if text.starts_with("diff --git ") {
            path = None;
            next = None;
        } else if let Some(hunk) = text.strip_prefix("@@ ") {
            next = hunk
                .split_whitespace()
                .find_map(|part| part.strip_prefix('+'))
                .and_then(|part| part.split(',').next())
                .and_then(|start| start.parse().ok());
        } else if at == line {
            return Some((path?, next?));
        } else if let Some(now) = next {
            if !text.starts_with('-') {
                next = Some(now + 1);
            }
        }
    }
    None
}

/// What the agent is told when the person sends a concern on.
pub(crate) fn concern_message(kept: &KeptConcern) -> String {
    let origin = match &kept.from_step {
        Some((_, label)) => format!("after the \"{label}\" step"),
        None => String::from("from the Judge tile"),
    };
    let mut said = format!("A judge read your work {origin} and raised this concern:\n\n{}", kept.text);
    let cites: Vec<String> = kept
        .cites
        .iter()
        .filter_map(|one| one.path.as_ref().map(|path| format!("{path}:{}", one.from_line)))
        .collect();
    if !cites.is_empty() {
        said.push_str(&format!("\n\nIt points at {}.", cites.join(", ")));
    }
    said
}

fn cut(diff: &str) -> String {
    match diff.char_indices().nth(DIFF_AT_MOST) {
        Some((at, _)) => format!("{}\n[the diff continues; the rest is not shown]", &diff[..at]),
        None => diff.to_string(),
    }
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
    /// `judge_session`: answer at once with `judging` set, and read in a task.
    pub(crate) async fn judge_asked(self: Arc<Self>, judge: JudgeSession) -> Result<SessionRecord, Refusal> {
        let id = judge.session_id.as_str().trim().to_string();
        if self.terminal_session(&id).await?.is_some() {
            return Err(self.hosted_refusal(
                SESSION_COMMAND_REFUSED,
                &format!("job {id} is a terminal job, and Fleet does not read its work"),
            ));
        }
        let (session, _) = self.session_and_hosting(&id).await?;
        if session.state == store::SessionState::Ended {
            return Err(self.closed(&id));
        }
        let runtime = self.hosts().of(&id);
        if runtime.state().judging > 0 {
            return self.published_hosted(&id).await;
        }
        let record = {
            let store = self.store().lock().await;
            self.ledger_row(&store, &session)?
        };
        let steps = record
            .workflow
            .as_deref()
            .zip(session.manifest_id.as_deref())
            .and_then(|(workflow, manifest)| self.rail_steps(manifest, workflow));
        let (label, criteria) = match (&steps, record.workflow_step) {
            (Some(steps), Some(at)) => steps
                .get(at as usize)
                .map(|step| (step.label.clone(), step.criteria.clone()))
                .unwrap_or_default(),
            _ => Default::default(),
        };
        let label = if label.is_empty() { String::from("Work so far") } else { label };
        let reading = Reading::begin(runtime);
        let reply = self.published_hosted(&id).await;
        let fleet = Arc::clone(&self);
        tokio::spawn(async move {
            fleet.judged(&id, Question::tile(label, criteria)).await;
            drop(reading);
            let _ = fleet.published_hosted(&id).await;
        });
        reply
    }

    /// Read the Session's work once for each question, keep the concerns and the reading, and publish.
    /// Whatever goes wrong is words for Fleet's log, and raises nothing.
    pub(crate) async fn judged(self: &Arc<Self>, id: &str, question: Question) {
        let (concerns, readable) = match self.concerns_read(id, &question).await {
            Ok(read) => read,
            Err(why) => {
                self.session_warned(id, "the work was not judged", why);
                return;
            }
        };
        if readable == 0 {
            return;
        }
        let now = self.now().as_str().to_string();
        let raised = concerns.len() as u32;
        {
            let mut store = self.store().lock().await;
            let held = store.session_concerns(id).unwrap_or_default();
            for mut one in concerns {
                if held.iter().any(|waiting| waiting.settled.is_none() && waiting.text == one.text) {
                    continue;
                }
                one.since = now.clone();
                one.from_step = question.from.clone();
                if let Err(why) = store.keep_session_concern(id, &one) {
                    self.session_warned(id, "a concern was not kept", why);
                }
            }
            if let Err(why) = store.keep_session_judge_reading(id, &KeptReading { at: now, concerns: raised }) {
                self.session_warned(id, "the reading was not kept", why);
            }
        }
        let _ = self.published_hosted(id).await;
    }

    /// The concerns the questions raised, and how many of the calls were read.
    async fn concerns_read(&self, id: &str, question: &Question) -> Result<(Vec<KeptConcern>, usize), String> {
        let (session, hosting) = self.session_and_hosting(id).await.map_err(|_| String::from("the job is gone"))?;
        let served = self
            .served_named(Some(&ManifestId::carried(&hosting.manifest_id)))
            .map_err(|_| String::from("the repository is not served"))?;
        let explaining = self.explaining().map_err(|why| format!("{why:?}"))?;
        let request = self
            .rows_of(id)
            .await
            .map_err(|_| String::from("the thread would not read"))?
            .into_iter()
            .find_map(|row| match row {
                SessionRow::Message { from: SessionVoice::You, text, .. } => Some(text),
                _ => None,
            })
            .unwrap_or_default();
        let plan = {
            let store = self.store().lock().await;
            store.session_plan(id).map_err(|why| why.to_string())?
        };
        let plan = plan.as_ref().map(|kept| plan_told(&crate::plan_record::wire(kept))).unwrap_or_default();
        let diff = match hosting.lease_branch.as_deref() {
            Some(branch) => {
                let worktree = self.based(&served, Worktree::at(session.cwd.clone(), branch));
                self.work().patch(&worktree).map(|patch| cut(patch.as_str())).map_err(|why| why.to_string())?
            }
            None => String::new(),
        };
        let cap = self.tuned(|dials| dials.standing_rules_bytes);
        let standing = crate::judging::standing(served.manifest(), served.root(), cap);
        let wording = self.prompts().wording();
        let work = Work { label: &question.label, request: &request, plan: &plan, diff: &diff };
        let mut concerns = Vec::new();
        let mut readable = 0;
        for criterion in &question.criteria {
            let brief = Brief::about_work(&work, criterion, &standing, &wording);
            let ask = Ask::put(explaining.model.clone(), brief.question(), explaining.environment.clone())
                .map_err(|_| String::from("there was nothing to ask"))?;
            let reply = match said(explaining.client.as_ref(), &ask, explaining.budget).await {
                Ok(reply) => reply,
                Err(why) => {
                    self.session_warned(id, "a judge's call failed", why);
                    continue;
                }
            };
            let judged = match brief.read(&reply) {
                Ok(judged) => judged,
                Err(why) => {
                    self.session_warned(id, "a judge's answer was not readable", format!("{why:?}"));
                    continue;
                }
            };
            readable += 1;
            if !judged.verdict.refuses() {
                continue;
            }
            let mut cites: Vec<KeptCite> = Vec::new();
            for one in judged.cited.iter().flatten().filter(|one| one.region == "diff") {
                if let Some((path, line)) = place_in_diff(brief.question(), one.from_line) {
                    let kept = KeptCite { region: String::from("diff"), path: Some(path), from_line: line, to_line: line };
                    if !cites.contains(&kept) {
                        cites.push(kept);
                    }
                }
            }
            let part = |label: &str, said: Option<String>| said.map(|said| format!("{label}: {said}"));
            let text = [
                part("Expected", judged.expected.clone()),
                part("Produced", judged.produced.clone()),
                part("Consequence", judged.consequence.clone()),
            ]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
            .join("\n");
            concerns.push(KeptConcern {
                n: 0,
                text,
                expected: judged.expected,
                produced: judged.produced,
                consequence: judged.consequence,
                since: String::new(),
                from_step: None,
                cites,
                settled: None,
            });
        }
        Ok((concerns, readable))
    }

    /// `send_concern`: the concern goes to the agent as the person's message, then it is settled.
    /// **Sent before it is settled**, so a send that fails leaves it waiting.
    pub(crate) async fn concern_sent(self: Arc<Self>, send: SendConcern) -> Result<SessionRecord, Refusal> {
        let id = send.session_id.as_str().trim().to_string();
        let (n, kept) = self.concern_waiting(&id, &send.item_id).await?;
        let _settling = self.hosts().settling(&id, &send.item_id).ok_or_else(|| self.settled_already())?;
        let sent = SendSessionMessage {
            session_id: send.session_id.clone(),
            text: concern_message(&kept),
            attachments: Vec::new(),
            mentions: Vec::new(),
            aim: None,
        };
        Arc::clone(&self).send_known(None, sent).await?;
        self.concern_settled(&id, n, "sent").await
    }

    /// `dismiss_concern`: the concern is settled and the agent is told nothing.
    pub(crate) async fn concern_dismissed(self: Arc<Self>, dismiss: DismissConcern) -> Result<SessionRecord, Refusal> {
        let id = dismiss.session_id.as_str().trim().to_string();
        let (n, _) = self.concern_waiting(&id, &dismiss.item_id).await?;
        let _settling = self.hosts().settling(&id, &dismiss.item_id).ok_or_else(|| self.settled_already())?;
        self.concern_settled(&id, n, "dismissed").await
    }

    /// The concern `item_id` names, if it still waits.
    async fn concern_waiting(&self, id: &str, item_id: &str) -> Result<(u32, KeptConcern), Refusal> {
        let n = concern_number(item_id).ok_or_else(|| self.settled_already())?;
        let kept = {
            let store = self.store().lock().await;
            store.session(id).map_err(|why| self.ledger_fault(why))?.ok_or_else(|| self.ledger_unnamed("a job Fleet knows"))?;
            store.session_concerns(id).map_err(|why| self.ledger_fault(why))?
        };
        kept.into_iter()
            .find(|one| one.n == n && one.settled.is_none())
            .map(|one| (n, one))
            .ok_or_else(|| self.settled_already())
    }

    async fn concern_settled(&self, id: &str, n: u32, how: &str) -> Result<SessionRecord, Refusal> {
        let now = self.now().as_str().to_string();
        let taken = self
            .store()
            .lock()
            .await
            .settle_session_concern(id, n, how, &now)
            .map_err(|why| self.ledger_fault(why))?;
        if !taken {
            return Err(self.settled_already());
        }
        self.published_hosted(id).await
    }
}

/// The plan as a judge reads it: each group, its tasks and how far Fleet observed each carried out.
fn plan_told(plan: &ipc::SessionPlan) -> String {
    let mut told = String::new();
    for group in &plan.groups {
        told.push_str(&format!("{} ({})\n", group.title, group.paths.join(", ")));
        for task in &group.tasks {
            told.push_str(&format!("  - {} [{}]\n", task.text, task_state(task.state)));
        }
    }
    told
}

fn task_state(state: ipc::PlanTaskState) -> &'static str {
    match state {
        ipc::PlanTaskState::Open => "open",
        ipc::PlanTaskState::Working => "working",
        ipc::PlanTaskState::HandedIn => "handed in",
        ipc::PlanTaskState::Done => "done",
        ipc::PlanTaskState::Failed => "failed",
    }
}
