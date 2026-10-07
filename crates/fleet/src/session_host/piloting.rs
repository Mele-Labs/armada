//! A session started on a Job a person takes over. Since 23.51.
//! `docs/concepts/pilot.md`, *The piloted session*.
//!
//! **One call is the whole act.** The Job is marked and its Drone ended by
//! [`Fleet::take_over`], which hands the Job's slot and branch to the session
//! named; this adds the session itself, running in that worktree from its
//! first message, with no lease of its own to take. Its thread opens with a
//! `handoff` row for Bridge, and the bundle as the agent reads it goes ahead of
//! the person's first message, so a session nobody has spoken in has spent
//! nothing.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use core_model::PilotReason;
use ipc::{
    DroneNarrative, HandoffBundle, HandoffPlan, HandoffStep, JobId, PilotFrom, PilotOutcome,
    SessionRecord, SessionRow, StartSession,
};
use store::{AttachmentState, Holder, KeptAttachment, KeptHosting, KeptSession};

use super::rows::new_id;
use super::serving::mode_text;
use crate::daemon::Fleet;

/// A Job with no checkout of its own to hand over. A 422.
const NO_WORKTREE: &str = "fleet.pilot_without_worktree";

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
    /// Take the Job over, then start a session in its worktree. **Refused with
    /// nothing changed** where `take_over` refuses: the session is written
    /// only after the Job is the person's.
    pub(crate) async fn start_piloted(
        &self,
        start: StartSession,
        from: PilotFrom,
    ) -> Result<SessionRecord, Refusal> {
        let job_id = from.job_id.to_domain();
        let reason = match from.outcome {
            PilotOutcome::TakeOver => PilotReason::TakeOver,
            PilotOutcome::RestartStep => PilotReason::RestartStep,
        };
        let id = new_id();
        let job = self
            .take_over(&job_id, reason, Some(&id))
            .await
            .map_err(|why| self.refusal(why))?;
        // `take_over` refuses a Job whose checkout is gone, so this is a Job
        // whose slot was never recorded. It is piloted by now and stays so,
        // as a take over naming no session: every exit still works on it.
        let Some(slot) = job.worktree_slot() else {
            return Err(self.hosted_refusal(
                NO_WORKTREE,
                "this Job is taken over, but it holds no worktree slot to hand to a session",
            ));
        };
        let bundle = self.handoff_bundle(from.job_id.clone()).await?;
        let (directory, branch) = bundle
            .worktree
            .as_ref()
            .map(|worktree| (worktree.path.clone(), worktree.branch.clone()))
            .unwrap_or_default();
        let manifest = job.owner_manifest_id().as_str().to_string();
        let now = self.now().as_str().to_string();
        {
            let mut store = self.store().lock().await;
            store
                .keep_session(&KeptSession {
                    id: id.clone(),
                    harness: String::from(adapters::HOSTED_HARNESS),
                    origin: String::from("bridge"),
                    manifest_id: Some(manifest.clone()),
                    cwd: directory,
                    title: Some(
                        start
                            .title
                            .map(|title| title.trim().to_string())
                            .filter(|title| !title.is_empty())
                            .unwrap_or_else(|| job.title().as_str().to_string()),
                    ),
                    state: store::SessionState::Live,
                    started_at: now.clone(),
                    last_seen_at: now.clone(),
                    last_turn_at: None,
                    ended_at: None,
                    end_reason: None,
                    figures: Default::default(),
                    mod_version: None,
                })
                .map_err(|why| self.ledger_fault(why))?;
            store
                .keep_hosting(&KeptHosting {
                    session_id: id.clone(),
                    manifest_id: manifest.clone(),
                    model: start.model.filter(|model| !model.trim().is_empty()),
                    effort: start.effort.filter(|effort| !effort.trim().is_empty()),
                    mode: mode_text(start.mode.unwrap_or_default()).to_string(),
                    ran: false,
                    // The Job's slot, so `directory_of` says the worktree and
                    // the first write takes no lease.
                    lease_slot: Some(slot),
                    lease_branch: Some(branch.clone()),
                })
                .map_err(|why| self.ledger_fault(why))?;
            // The Job the session is piloting, beside the slot and branch
            // `take_over` handed it. Not `looking`: this is the session's own.
            store
                .attach(
                    &KeptAttachment {
                        holder: Holder::session(&id),
                        kind: String::from("job"),
                        manifest_id: manifest,
                        target: job_id.as_str().to_string(),
                        state: AttachmentState::Standing,
                        detail: Default::default(),
                        since: now.clone(),
                        changed_at: now,
                    },
                    false,
                )
                .map_err(|why| self.ledger_fault(why))?;
        }
        self.row_put(
            &id,
            handoff_row(
                self.row_id(&id),
                self.instant(),
                (job.number().get(), job.title().as_str(), Some(slot)),
                &bundle,
            ),
        )
        .await;
        self.published_hosted(&id).await
    }

    /// The Job a session pilots while it does, from the ledger: the slot it was
    /// handed says whose it was.
    pub(crate) async fn piloted_job_of(&self, session: &str) -> Option<core_model::JobId> {
        let held = self
            .store()
            .lock()
            .await
            .attachments_of(&Holder::session(session))
            .ok()?;
        held.into_iter()
            .filter(|one| one.kind == "slot" && one.state == AttachmentState::Standing)
            .find_map(|one| {
                let handed = one.detail.get("handed")?;
                Some(core_model::JobId::carried(core_model::Ulid::carried(
                    handed.strip_prefix("job ")?,
                )))
            })
    }

    /// What goes ahead of the person's first message in a piloted session: the
    /// bundle, as the agent reads it. **Nothing for any other session**, and
    /// nothing once its process has run, since the transcript carries it then.
    pub(crate) async fn handoff_preface(&self, session: &str) -> Option<String> {
        let job = self.piloted_job_of(session).await?;
        let bundle = self.handoff_bundle(JobId::from(&job)).await.ok()?;
        Some(rendered_for_the_agent(&bundle))
    }

    /// The pilot is over, or its session is: the session holds no slot, so its
    /// next write leases one of its own, and a process still standing in the
    /// Job's worktree is let go so the next turn does not start in it.
    pub(crate) async fn left_the_worktree(&self, session: &str) {
        let hosting = self.store().lock().await.hosting(session);
        let Ok(Some(mut hosting)) = hosting else {
            return;
        };
        hosting.lease_slot = None;
        hosting.lease_branch = None;
        let _ = self.store().lock().await.keep_hosting(&hosting);
        Self::let_go(&mut self.hosts().of(session).state());
        let _ = self.published_hosted(session).await;
    }

    /// A session that piloted a Job is closed while the Job is still piloted:
    /// the worktree goes back to the Job, and **is not parked**, since it is
    /// not the session's to commit.
    pub(crate) async fn closed_while_piloting(&self, job: &core_model::JobId) {
        let Ok(loaded) = self.load(job).await else {
            return;
        };
        let pilot = self.store().lock().await.pilot_of(job);
        if let Ok(Some(pilot)) = pilot {
            self.handed_back(&loaded, &pilot).await;
        }
    }
}

/// The handoff row, from the bundle's structured fields.
pub(crate) fn handoff_row(
    id: String,
    at: ipc::Instant,
    (number, title, slot): (u32, &str, Option<u32>),
    bundle: &HandoffBundle,
) -> SessionRow {
    let in_hand = bundle.stopped_on.as_ref().map(|stopped| &stopped.step_id);
    let step =
        in_hand.and_then(|wanted| bundle.job.steps.iter().find(|step| &step.step_id == wanted));
    let refusals = step
        .map(|step| {
            step.judged
                .iter()
                .filter(|said| said.verdict.as_wire() == "not_met")
                .filter_map(|said| said.produced.clone().or_else(|| said.expected.clone()))
                .collect()
        })
        .unwrap_or_default();
    let plan = in_hand
        .and_then(|wanted| {
            bundle
                .plans
                .iter()
                .rev()
                .find(|plan| &plan.step_id == wanted)
        })
        .or_else(|| bundle.plans.last());
    HandoffRowParts {
        id,
        at,
        number,
        title: title.to_string(),
        slot,
        step,
        refusals,
        plan,
        bundle,
    }
    .into_row()
}

struct HandoffRowParts<'a> {
    id: String,
    at: ipc::Instant,
    number: u32,
    title: String,
    slot: Option<u32>,
    step: Option<&'a ipc::StepDetail>,
    refusals: Vec<String>,
    plan: Option<&'a ipc::DeclaredPlan>,
    bundle: &'a HandoffBundle,
}

impl HandoffRowParts<'_> {
    fn into_row(self) -> SessionRow {
        let bundle = self.bundle;
        let covered = |declared: &str, path: &str| {
            let declared = declared.trim_end_matches('/');
            path == declared || path.starts_with(&format!("{declared}/"))
        };
        let outside = bundle
            .changed
            .iter()
            .filter(|file| file.outside_plan)
            .map(|file| file.path.clone())
            .collect();
        let unwritten = self
            .plan
            .map(|plan| {
                plan.paths
                    .iter()
                    .filter(|declared| {
                        !bundle
                            .changed
                            .iter()
                            .any(|file| covered(declared, &file.path))
                    })
                    .cloned()
                    .collect()
            })
            .unwrap_or_default();
        SessionRow::Handoff {
            id: self.id,
            at: self.at,
            job_id: bundle.job.job.id.clone(),
            number: self.number,
            title: self.title,
            reason: bundle.reason.clone(),
            slot: self.slot,
            branch: bundle
                .worktree
                .as_ref()
                .map(|worktree| worktree.branch.clone())
                .unwrap_or_default(),
            step: self.step.map(|step| HandoffStep {
                id: step.step_id.as_str().to_string(),
                label: step.label.clone(),
            }),
            attempts: self.step.map_or(0, |step| step.attempts.len() as u32),
            refusals: self.refusals,
            plan: HandoffPlan {
                declared: bundle.plan_declared,
                outside,
                unwritten,
            },
            narrative: bundle.narrative.clone().map(|said| DroneNarrative {
                trying_to: said.trying_to,
                blocked_by: said.blocked_by,
                tried: said.tried,
            }),
        }
    }
}

/// The bundle as the agent reads it. **Prose for a reader who has the worktree
/// and not the Board**: what stopped, what was refused, what the plan said
/// against what the tree holds, and what the Drone said it was stuck on, which
/// it is told is the Drone's word.
pub(crate) fn rendered_for_the_agent(bundle: &HandoffBundle) -> String {
    let mut said = String::new();
    let job = &bundle.job.job;
    said.push_str(&format!(
        "A person has taken over Job \"{}\" (id {}) and you are working it with them. Its Drone \
         was stopped, and its worktree is yours: you are already in it.\n",
        job.title,
        job.id.as_str()
    ));
    if let Some(worktree) = &bundle.worktree {
        said.push_str(&format!(
            "Worktree {}, branch {}.\n",
            worktree.path, worktree.branch
        ));
    }
    said.push_str(match bundle.reason.as_str() {
        "restart_step" => {
            "It was taken over to restart its step: a fresh Drone takes the step after the person \
             is done.\n"
        }
        _ => "It was taken over for good: the person ends the pilot when they are done.\n",
    });
    if let Some(stopped) = &bundle.stopped_on {
        let step = bundle
            .job
            .steps
            .iter()
            .find(|step| step.step_id == stopped.step_id);
        said.push_str(&format!(
            "\nIt stopped on step `{}`{}{}, after {} run(s).\n",
            stopped.step_id.as_str(),
            step.map(|step| format!(" \"{}\"", step.label))
                .unwrap_or_default(),
            stopped
                .trigger
                .as_ref()
                .map(|trigger| format!(" ({trigger})"))
                .unwrap_or_default(),
            step.map_or(0, |step| step.attempts.len())
        ));
        if let Some(step) = step {
            let refused: Vec<String> = step
                .judged
                .iter()
                .filter(|said| said.verdict.as_wire() == "not_met")
                .map(|said| {
                    let mut line = format!("- criterion {}", said.criterion_id.as_str());
                    for (label, text) in [
                        ("expected", &said.expected),
                        ("produced", &said.produced),
                        ("consequence", &said.consequence),
                    ] {
                        if let Some(text) = text {
                            line.push_str(&format!("; {label}: {text}"));
                        }
                    }
                    line
                })
                .collect();
            if !refused.is_empty() {
                said.push_str("The Judge refused:\n");
                said.push_str(&refused.join("\n"));
                said.push('\n');
            }
            let failed: Vec<String> = step
                .check_runs
                .iter()
                .filter(|run| run.outcome.as_wire() != "passed")
                .map(|run| format!("- {} (run {})", run.name, run.attempt))
                .collect();
            if !failed.is_empty() {
                said.push_str("Checks that did not pass:\n");
                said.push_str(&failed.join("\n"));
                said.push('\n');
            }
        }
    }
    if bundle.plan_declared {
        let outside: Vec<&str> = bundle
            .changed
            .iter()
            .filter(|file| file.outside_plan)
            .map(|file| file.path.as_str())
            .collect();
        if !outside.is_empty() {
            said.push_str(&format!(
                "\nFiles changed outside the plan the step declared: {}.\n",
                outside.join(", ")
            ));
        }
    }
    if !bundle.changed.is_empty() {
        let files: Vec<&str> = bundle
            .changed
            .iter()
            .map(|file| file.path.as_str())
            .collect();
        said.push_str(&format!(
            "\nThe worktree holds changes to: {}.\n",
            files.join(", ")
        ));
    }
    if let Some(narrative) = &bundle.narrative {
        said.push_str(&format!(
            "\nThe Drone's own account, which is its word and not evidence:\n- trying to: {}\n- \
             blocked by: {}\n- tried: {}\n",
            narrative.trying_to, narrative.blocked_by, narrative.tried
        ));
    }
    said.push_str(
        "\nEvery attempt of every step and the evidence each submitted are in the handoff record, \
         `get_handoff` for this Job id, if you need them.",
    );
    said
}
