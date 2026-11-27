//! Sub projects: the Jobs the owner picked in the Jobs list, kept under a name and his goal with the features they worked
//! in at the press, and the orchestrator, an ordinary Job that coordinates them. Fleet keeps only what he picked; the Jobs
//! list adds the live Jobs that join from `list_features`. `docs/concepts/feature-catalog.md`, *Sub projects*.
//!
//! **The Goal met? card is derived on every read** of the orchestrator's list, from the sub project and its Jobs, so a
//! restart keeps it. It stands once the orchestrator's `all_done` stands or every Job of it merged or ended. Agree folds
//! the sub project away and Not yet holds the card back until the orchestrator next calls `all_done`.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::{HostedSessions, Refusal};
use ipc::{
    CloseSession, GoalMet, Instant, LandedPull, ManifestId, ReleaseSessionHolds, SendSessionMessage, SessionId, SessionRecord, SessionRow,
    SessionVoice, StartSession, StartSubProject, SubProject, SubProjects, TurnIntoOrchestrator, UngroupSubProject, WaitingAct, WaitingActKind,
    WaitingItem, WaitingOption, WaitingSource, WireError,
};
use store::{Holder, KeptSubProject, SessionState, Store, WriteError};

use crate::daemon::Fleet;
use crate::session_host::serving::NO_SUCH_SESSION;

const NO_NAME: &str = "fleet.sub_project_no_name";
const NO_GOAL: &str = "fleet.sub_project_no_goal";
const NO_JOBS: &str = "fleet.sub_project_no_jobs";
/// A picked Job is filed in no project, or in another one than the sub project's.
const ACROSS_PROJECTS: &str = "fleet.sub_project_across_projects";
const NO_SUCH_SUB_PROJECT: &str = "fleet.no_such_sub_project";
/// The Job to turn has ended, is not Fleet's to host, or orchestrates a sub project already.
const NOT_TURNABLE: &str = "fleet.sub_project_not_turnable";

/// The id of the Goal met? item: `goal_met:<sub project id>`.
pub(crate) const GOAL_MET: &str = "goal_met:";
/// Choice 0 folds the sub project away.
pub(crate) const AGREE: u32 = 0;
/// Choice 1 keeps it open.
pub(crate) const NOT_YET: u32 = 1;

fn wire_of(kept: KeptSubProject) -> SubProject {
    SubProject {
        id: kept.id,
        project: kept.project,
        name: kept.name,
        goal: kept.goal,
        jobs: kept.jobs.iter().map(SessionId::carried).collect(),
        features: kept.features,
        orchestrator: kept.orchestrator.map(SessionId::carried),
        started_at: Instant::carried(kept.started_at),
        ended_at: kept.ended_at.map(Instant::carried),
    }
}

/// What the orchestrator is told it is: its sub project, the goal and the Jobs, by title and id.
fn orchestrator_brief(kept: &KeptSubProject, jobs: &[(String, String)]) -> String {
    let mut brief = format!(
        "You are the orchestrator of the sub project \"{}\" in project {}. The owner talks to you here as he talks to any Job.\n\nGoal: {}\n\nIts Jobs:\n",
        kept.name, kept.project, kept.goal
    );
    for (title, id) in jobs {
        brief.push_str(&format!("- {title} ({id})\n"));
    }
    brief.push_str(
        "\nCoordinate these Jobs toward the goal: read where each stands, tell a Job what it needs, and start another Job \
         with start_another_job where the goal needs one. The Jobs write the code. Call all_done once the goal is met, \
         and the owner answers Goal met? on your card.",
    );
    brief
}

/// What Fleet reads for the Goal met? card of the sub project a Job orchestrates.
#[derive(Clone, Debug)]
pub(crate) struct GoalAsk {
    kept: KeptSubProject,
    /// Every Job of it merged a pull request or ended.
    members_done: bool,
    landed: Vec<LandedPull>,
}

/// The Goal met? read for `session`, where it orchestrates an open sub project.
pub(crate) fn goal_ask(store: &Store, session: &str) -> Result<Option<GoalAsk>, WriteError> {
    let Some(kept) = store.sub_project_orchestrated_by(session)? else { return Ok(None) };
    let mut landed: Vec<(String, LandedPull)> = Vec::new();
    let mut members_done = !kept.jobs.is_empty();
    for job in &kept.jobs {
        // To him a hand is one Job: the one carrying its work now says whether it ended.
        let now = crate::session_host::replacing::handed_to(store, job).unwrap_or_else(|| job.clone());
        let mut merged = false;
        let chain: &[&str] = if now == *job { &[job.as_str()] } else { &[job.as_str(), now.as_str()] };
        for &id in chain {
            for pull in store.attachments_of(&Holder::session(id))?.into_iter().filter(|one| one.kind == "pr" && crate::build_record::merged(one)) {
                merged = true;
                let Ok(number) = pull.target.parse::<u64>() else { continue };
                if landed.iter().all(|(_, one)| one.number != number) {
                    let title = pull.detail.get("title").cloned().unwrap_or_default();
                    landed.push((pull.changed_at.clone(), LandedPull { number, title }));
                }
            }
        }
        let ended = store.session(&now)?.is_none_or(|one| one.state != SessionState::Live);
        members_done &= merged || ended;
    }
    landed.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(Some(GoalAsk { kept, members_done, landed: landed.into_iter().map(|(_, pull)| pull).collect() }))
}

/// The Goal met? item, where it stands: the orchestrator's `done` since `done`, after his last Not yet, or every Job of
/// the sub project done before he said Not yet at all.
pub(crate) fn goal_met_item(ask: &GoalAsk, done: Option<&str>) -> Option<WaitingItem> {
    let not_yet = ask.kept.not_yet_at.as_deref();
    let by_done = done.filter(|done| not_yet.is_none_or(|said| *done > said));
    let by_jobs = (ask.members_done && not_yet.is_none()).then_some(ask.kept.started_at.as_str());
    let since = by_done.or(by_jobs)?;
    let option = |label: &str| WaitingOption { label: label.to_string(), description: None, picture: None };
    let id = format!("{GOAL_MET}{}", ask.kept.id);
    Some(WaitingItem {
        order: 0,
        why: String::new(),
        approves: false,
        act: Some(WaitingAct { kind: WaitingActKind::Answer, target: id.clone(), on: None }),
        id,
        text: String::from("Goal met?"),
        since: Instant::carried(since),
        source: WaitingSource::GoalMet,
        options: vec![option("Agree"), option("Not yet")],
        steps: Vec::new(),
        place: None,
        confirmed: false,
        holders: Vec::new(),
        creating_since: None,
        create_refused: None,
        operator_read: None,
        artifact: None,
        goal_met: Some(GoalMet { sub_project: ask.kept.id.clone(), goal: ask.kept.goal.clone(), landed: ask.landed.clone() }),
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
    fn sub_project_refused(&self, code: &'static str, why: &str) -> Refusal {
        Refusal::Unacceptable(WireError::raised(code, why, self.run_id()))
    }

    /// The Jobs are filed in `project`, every one: a sub project holds the Jobs of one project.
    fn filed_in(&self, store: &Store, project: &str, jobs: &[&str]) -> Result<(), Refusal> {
        for job in jobs {
            if !self.projects_of_job_in(store, job).iter().any(|one| one == project) {
                return Err(Refusal::IllegalMove(WireError::raised(
                    ACROSS_PROJECTS,
                    format!("job {job} is not filed in project {project}: a sub project holds the Jobs of one project"),
                    self.run_id(),
                )));
            }
        }
        Ok(())
    }

    /// The slugs of the features of `project` whose `jobs_now` holds one of `jobs`.
    fn features_held(&self, store: &Store, project: &str, jobs: &[&str]) -> Result<Vec<String>, Refusal> {
        let on = self.jobs_on_features(store, project)?;
        Ok(store
            .features_in(project)
            .map_err(|why| self.ledger_fault(why))?
            .into_iter()
            .filter(|feature| on.get(&feature.id).is_some_and(|held| held.iter().any(|one| jobs.contains(&one.session_id.as_str()))))
            .map(|feature| feature.slug)
            .collect())
    }

    /// A Job of the project in `manifest`, titled `title`. Its id.
    async fn job_begun(&self, project: &str, manifest: &str, title: &str) -> Result<SessionId, Refusal> {
        let start = StartSession {
            project: Some(project.to_string()),
            manifest_id: ManifestId::carried(manifest),
            title: Some(title.to_string()),
            model: None,
            effort: None,
            mode: None,
            root: None,
            fork: None,
            request: None,
            after: None,
            held: false,
            project_rows: Vec::new(),
            on_your_words: None,
            workflow: None,
        };
        Ok(HostedSessions::start_session(self, start).await?.id)
    }

    /// The titles and ids of `kept`'s Jobs, and the repository the first of them works in.
    async fn jobs_named(&self, kept: &KeptSubProject) -> Result<(Vec<(String, String)>, String), Refusal> {
        let store = self.store().lock().await;
        let mut named = Vec::new();
        let mut manifest = None;
        for job in &kept.jobs {
            let session = store.session(job).map_err(|why| self.ledger_fault(why))?;
            manifest = manifest.or_else(|| session.as_ref().and_then(|one| one.manifest_id.clone()));
            named.push((session.and_then(|one| one.title).unwrap_or_else(|| job.clone()), job.clone()));
        }
        Ok((named, manifest.unwrap_or_default()))
    }

    /// Sends `text` to `session` as its next message. **Not an error** where it is refused: the Job stands, and its
    /// thread is where he sees it.
    async fn word_sent(self: &Arc<Self>, session: &str, text: String) {
        let _ = Arc::clone(self)
            .send_session_message(SendSessionMessage { session_id: SessionId::carried(session), text, attachments: Vec::new(), mentions: Vec::new(), sketches: Vec::new() })
            .await;
    }

    /// `start_sub_project`. **Nothing is kept on a refusal**, and the orchestrator is started before the sub project is
    /// kept, so one is never kept without it.
    pub(super) async fn sub_project_started(self: &Arc<Self>, start: StartSubProject) -> Result<SubProject, Refusal> {
        let name = start.name.trim();
        if name.is_empty() {
            return Err(self.sub_project_refused(NO_NAME, "name the sub project"));
        }
        let goal = start.goal.trim();
        if goal.is_empty() {
            return Err(self.sub_project_refused(NO_GOAL, "give the sub project a goal"));
        }
        if start.jobs.is_empty() {
            return Err(self.sub_project_refused(NO_JOBS, "pick a Job to group"));
        }
        self.feature_writes_filed().await;
        let jobs: Vec<&str> = start.jobs.iter().map(SessionId::as_str).collect();
        let features = {
            let store = self.store().lock().await;
            for job in &jobs {
                if store.session(job).map_err(|why| self.ledger_fault(why))?.is_none() {
                    return Err(Refusal::NotFound(WireError::raised(NO_SUCH_SESSION, format!("no job is named {job}"), self.run_id())));
                }
            }
            self.filed_in(&store, &start.project, &jobs)?;
            self.features_held(&store, &start.project, &jobs)?
        };
        let mut kept = KeptSubProject {
            id: self.mint().ulid().as_str().to_lowercase(),
            project: start.project.clone(),
            name: name.to_string(),
            goal: goal.to_string(),
            jobs: jobs.iter().map(|job| job.to_string()).collect(),
            features,
            orchestrator: None,
            started_at: self.now().as_str().to_string(),
            not_yet_at: None,
            ended_at: None,
        };
        let (named, manifest) = self.jobs_named(&kept).await?;
        let orchestrator = self.job_begun(&kept.project, &manifest, &kept.goal).await?.as_str().to_string();
        kept.orchestrator = Some(orchestrator.clone());
        self.store().lock().await.put_sub_project(&kept).map_err(|why| self.ledger_fault(why))?;
        self.word_sent(&orchestrator, orchestrator_brief(&kept, &named)).await;
        Ok(wire_of(kept))
    }

    /// `list_sub_projects`. An open sub project kept without an orchestrator has one started here, **once**: the row is
    /// marked asked before the start, so a second read, or a start that failed, starts none.
    pub(super) async fn sub_projects_listed(self: &Arc<Self>, project: Option<String>, ended: bool) -> Result<SubProjects, Refusal> {
        let project = project.as_deref().map(str::trim).filter(|project| !project.is_empty());
        let mut kept = self.store().lock().await.sub_projects(project, ended).map_err(|why| self.ledger_fault(why))?;
        for one in kept.iter_mut().filter(|one| one.orchestrator.is_none() && one.ended_at.is_none()) {
            let at = self.now().as_str().to_string();
            if !self.store().lock().await.ask_sub_project_orchestrator(&one.id, &at).map_err(|why| self.ledger_fault(why))? {
                continue;
            }
            let (named, manifest) = self.jobs_named(one).await?;
            let Ok(orchestrator) = self.job_begun(&one.project, &manifest, &one.goal).await else { continue };
            let orchestrator = orchestrator.as_str().to_string();
            self.store().lock().await.set_sub_project_orchestrator(&one.id, &orchestrator).map_err(|why| self.ledger_fault(why))?;
            one.orchestrator = Some(orchestrator.clone());
            self.word_sent(&orchestrator, orchestrator_brief(one, &named)).await;
        }
        Ok(SubProjects { sub_projects: kept.into_iter().map(wire_of).collect() })
    }

    /// `turn_into_orchestrator`. **The handoff is composed before anything moves**, so it names what the Job holds; then
    /// the first child is started, takes the Job's slot, branch and pull requests, and is told what it carries on. A
    /// hand that fails closes the child, so the Job stands as it was.
    pub(super) async fn orchestrator_turned(self: &Arc<Self>, turn: TurnIntoOrchestrator) -> Result<SubProject, Refusal> {
        let goal = turn.goal.trim();
        if goal.is_empty() {
            return Err(self.sub_project_refused(NO_GOAL, "give the sub project a goal"));
        }
        let job = turn.session_id.as_str().to_string();
        self.feature_writes_filed().await;
        let (title, manifest, features) = {
            let store = self.store().lock().await;
            let session = store
                .session(&job)
                .map_err(|why| self.ledger_fault(why))?
                .ok_or_else(|| Refusal::NotFound(WireError::raised(NO_SUCH_SESSION, format!("no job is named {job}"), self.run_id())))?;
            let hosting = store.hosting(&job).map_err(|why| self.ledger_fault(why))?;
            let orchestrates = store.sub_project_orchestrated_by(&job).map_err(|why| self.ledger_fault(why))?.is_some();
            let Some(hosting) = hosting.filter(|_| session.state == SessionState::Live && !orchestrates) else {
                return Err(Refusal::IllegalMove(WireError::raised(
                    NOT_TURNABLE,
                    format!("job {job} has ended, runs outside Fleet or orchestrates a sub project already"),
                    self.run_id(),
                )));
            };
            self.filed_in(&store, &turn.project, &[job.as_str()])?;
            let features = self.features_held(&store, &turn.project, &[job.as_str()])?;
            (session.title.unwrap_or_else(|| job.clone()), hosting.manifest_id, features)
        };
        let mut context = self.handoff_of(&job).await?;
        let first = self.rows_of(&job).await?.into_iter().find_map(|row| match row {
            SessionRow::Message { from: SessionVoice::You, text, .. } if !text.trim().is_empty() => Some(text),
            _ => None,
        });
        if let Some(first) = first {
            let first: String = first.chars().take(crate::session_host::replacing::BRIEF_CHARS).collect();
            context.push_str(&format!("\n\nIts brief:\n{first}"));
        }
        let mut kept = KeptSubProject {
            id: self.mint().ulid().as_str().to_lowercase(),
            project: turn.project.clone(),
            name: title.clone(),
            goal: goal.to_string(),
            jobs: Vec::new(),
            features,
            orchestrator: Some(job.clone()),
            started_at: self.now().as_str().to_string(),
            not_yet_at: None,
            ended_at: None,
        };
        let child = self.job_begun(&kept.project, &manifest, &title).await?;
        let handed = self
            .session_holds_released(ReleaseSessionHolds { session_id: turn.session_id.clone(), to_session_id: Some(child.clone()), rows: None })
            .await;
        if let Err(refused) = handed {
            let _ = self.closed_and_parked(CloseSession { session_id: child }).await;
            return Err(refused);
        }
        let child = child.as_str().to_string();
        kept.jobs.push(child.clone());
        self.store().lock().await.put_sub_project(&kept).map_err(|why| self.ledger_fault(why))?;
        // Nested under the orchestrator, which follows it as it follows any Job it started.
        let _ = self.started_linked(&job, &child, &manifest, None).await;
        self.word_sent(
            &child,
            format!(
                "This job carries on the work of \"{title}\" ({job}), which now orchestrates the sub project \"{}\". Its slot, branch and pull requests were handed to you. What it left you is below.\n\n{context}",
                kept.name
            ),
        )
        .await;
        let brief = orchestrator_brief(&kept, &[(title.clone(), child.clone())]);
        self.word_sent(&job, format!("Your own work moved to \"{title}\" ({child}), the first Job of this sub project.\n\n{brief}")).await;
        let _ = self.published_hosted(&job).await;
        let _ = self.published_hosted(&child).await;
        Ok(wire_of(kept))
    }

    /// `ungroup_sub_project`.
    pub(super) async fn sub_project_ungrouped(&self, ungroup: UngroupSubProject) -> Result<(), Refusal> {
        let mut store = self.store().lock().await;
        match store.drop_sub_project(&ungroup.id).map_err(|why| self.ledger_fault(why))? {
            true => Ok(()),
            false => Err(Refusal::NotFound(WireError::raised(NO_SUCH_SUB_PROJECT, format!("no sub project is kept under {}", ungroup.id), self.run_id()))),
        }
    }

    /// `answer_waiting` on a `goal_met` item: Agree folds the sub project away, Not yet keeps it open. **The agent is
    /// told nothing.** Settled in the store first, so a second answer finds it settled and is a 409.
    pub(crate) async fn goal_met_chosen(&self, session: &str, item: &str, choice: Option<u32>) -> Result<SessionRecord, Refusal> {
        let agree = match choice {
            Some(AGREE) => true,
            Some(NOT_YET) => false,
            _ => {
                return Err(self.hosted_refusal(
                    crate::session_host::waiting::SESSION_WAITING_EMPTY,
                    "whether the goal is met is answered by choosing Agree or Not yet",
                ))
            }
        };
        let id = item.strip_prefix(GOAL_MET).ok_or_else(|| self.settled_already())?;
        let at = self.now().as_str().to_string();
        let moved = {
            let mut store = self.store().lock().await;
            // The sub project must be this Job's: an id from another Job's list settles nothing here.
            let mine = store.sub_project(id).map_err(|why| self.ledger_fault(why))?.is_some_and(|one| one.orchestrator.as_deref() == Some(session));
            match (mine, agree) {
                (false, _) => false,
                (true, true) => store.end_sub_project(id, &at).map_err(|why| self.ledger_fault(why))?,
                (true, false) => store.sub_project_not_yet(id, &at).map_err(|why| self.ledger_fault(why))?,
            }
        };
        if !moved {
            return Err(self.settled_already());
        }
        self.published_hosted(session).await
    }

    /// A Job ended or merged a pull request: the orchestrator of each open sub project it is in is published, so its
    /// Goal met? card reaches Bridge when the last Job is done.
    pub(crate) async fn orchestrators_published(&self, member: &str) {
        let orchestrators: Vec<String> = {
            let store = self.store().lock().await;
            store
                .sub_projects(None, false)
                .unwrap_or_default()
                .into_iter()
                .filter(|one| {
                    one.jobs.iter().any(|job| job == member || crate::session_host::replacing::handed_to(&store, job).as_deref() == Some(member))
                })
                .filter_map(|one| one.orchestrator)
                .collect()
        };
        for orchestrator in orchestrators {
            let _ = self.published_hosted(&orchestrator).await;
        }
    }

    /// Pull request `number` merged: [`Self::orchestrators_published`] for every Job that held it.
    pub(crate) async fn orchestrators_told_of_merge(&self, served: &crate::repositories::Served, number: u64) {
        let manifest = served.manifest().id().as_str().to_string();
        let holders: Vec<String> = self
            .store()
            .lock()
            .await
            .attachments_at("pr", &number.to_string(), Some(&manifest))
            .unwrap_or_default()
            .into_iter()
            .filter(|row| row.holder.kind == store::HolderKind::Session)
            .map(|row| row.holder.id)
            .collect();
        for holder in holders {
            self.orchestrators_published(&holder).await;
        }
    }
}
