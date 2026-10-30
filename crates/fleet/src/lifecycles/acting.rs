//! Doing what a rule planned, through the doors Fleet already has: a watch is started as an agent's
//! `start_watch` starts it, the agent is told as Fleet's own notices tell it, a Job is started as an
//! agent's `start_another_job` starts one, and a draft is proposed as `propose_draft` proposes it.
//!
//! **One worker, so an act is never done twice at once.** Planning is store-only and cannot hold the
//! `Arc` the doors need, so it keeps the act and wakes this ([`Owed`]); a Fleet that stopped between
//! the two finds the act at the start and does it then.
//!
//! **A try is counted before it is made.** A try a crash cut short is one of the rule's retries,
//! and not a free one. Retries follow each other at once: what fails here fails for a reason that
//! waiting would not mend, such as a recipe that is not there or a Job that has ended.
//!
//! **A tracker or forge write is done by [`super::writing`]**, and only from here: a rule the owner
//! saved counts as his say, and nothing else in Fleet writes to his tracker or forge. Commands,
//! skills and Drones are not done at all, and [`super::writing::Writes::carries`] keeps them from
//! being planned for doing. **It is asked again before each act is tried**, because the tracker or
//! forge may have changed since the rule planned it, and an act that can no longer be made is settled
//! as not carried rather than failed and tried again.

use std::collections::BTreeMap;
use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Watches;
use core_model::rule::{Act, OnFailure, Then, Words};
use core_model::Lifecycle;
use ipc::{ManifestId, Notice, ProposeDraft, SessionId, StartAnotherSession, StartWatch, WatchStarts};
use store::{rule_act_standing as standing, KeptRuleAct};
use tokio::sync::Notify;

use super::copying::copy_of;
use super::keeping::{DEPLOY, PULL_REQUEST};
use super::saying::{failed, filled, thing, titled};
use crate::daemon::Fleet;

/// Wakes the worker that does what rules planned. A wake that comes while it works is kept, so the
/// act planned in that moment is found on its next pass.
#[derive(Default)]
pub struct Owed(Notify);

impl Owed {
    pub(crate) fn wake(&self) {
        self.0.notify_one();
    }

    pub(crate) async fn woken(&self) {
        self.0.notified().await;
    }
}

/// The act a row stands for, read out of the rule the Job froze, with what to do should it fail
/// and the name of the rule. The Job's lifecycles are the tracker's words for a state.
struct Found {
    act: Act,
    on_failure: OnFailure,
    rule: String,
    lifecycles: Vec<Lifecycle>,
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
    /// Do every act that is planned and not done, oldest first. **Safe to run on every start**: an
    /// act that is done is not planned.
    pub async fn rule_acts_carried_out(self: &Arc<Self>) {
        let owed = self.store().lock().await.rule_acts_owed().unwrap_or_default();
        for row in owed {
            self.rule_act_driven(row.id).await;
        }
    }

    /// Try one act until it is done or its rule has no retries left, then settle it.
    async fn rule_act_driven(self: &Arc<Self>, id: i64) {
        loop {
            let (row, found, tries) = {
                let mut store = self.store().lock().await;
                let Some(row) = store.rule_act(id).ok().flatten().filter(|row| row.state == standing::PLANNED) else { return };
                let found = copy_of(&store, &row.session_id, self.built_in_lifecycles_of_job(&store, &row.session_id)).and_then(|copy| {
                    let rule = copy.rules.iter().find(|rule| rule.id().as_str() == row.rule_id)?;
                    let act = rule.does().get(row.position as usize)?.clone();
                    Some(Found { act, on_failure: rule.on_failure(), rule: rule.name().as_str().to_string(), lifecycles: copy.lifecycles.clone() })
                });
                let Some(found) = found else {
                    // The rule is gone from the copy: nothing says what to do, and the owner is told so.
                    let _ = store.move_rule_act(id, &[standing::PLANNED], standing::STUCK, "the rule is no longer in the Job's copy", self.now().as_str());
                    drop(store);
                    self.rule_act_published(&row.session_id).await;
                    return;
                };
                // A write the tracker or forge can no longer make is not tried: it is settled, with why.
                if let Err(why) = self.rule_act_carried(&store, &row, &found.act, &found.lifecycles) {
                    let _ = store.move_rule_act(id, &[standing::PLANNED], standing::NOT_CARRIED, &why, self.now().as_str());
                    drop(store);
                    self.rule_act_published(&row.session_id).await;
                    return;
                }
                let Ok(tries) = store.begin_rule_attempt(id) else { return };
                (row, found, tries)
            };
            match self.rule_act_done(&row, &found).await {
                Ok(said) => {
                    let at = self.now().as_str().to_string();
                    let _ = self.store().lock().await.move_rule_act(id, &[standing::PLANNED], standing::DONE, &said, &at);
                    self.rule_act_published(&row.session_id).await;
                    return;
                }
                Err(_) if tries <= u32::from(found.on_failure.retries.times()) => continue,
                Err(why) => {
                    self.rule_act_failed(&row, found.on_failure.then, &why).await;
                    return;
                }
            }
        }
    }

    /// An act that failed past its retries: the agent is told where the rule says so, and the owner
    /// is asked where it says that or the agent could not be told.
    async fn rule_act_failed(self: &Arc<Self>, row: &KeptRuleAct, then: Then, why: &str) {
        let line = failed(&row.text, why);
        let told = match then {
            Then::TellAgent => self.told_by_fleet(&row.session_id, &line, Notice::LifecycleRule).await.unwrap_or(false),
            Then::AskOwner => false,
        };
        let to = if told { standing::TOLD } else { standing::STUCK };
        let at = self.now().as_str().to_string();
        let _ = self.store().lock().await.move_rule_act(row.id, &[standing::PLANNED], to, &line, &at);
        self.rule_act_published(&row.session_id).await;
    }

    /// Draw the Job again where an act settled, and tell the Workflow view what it holds, so neither
    /// waits for the next reading.
    pub(super) async fn rule_act_published(&self, session: &str) {
        let _ = self.published_hosted(session).await;
        self.job_lifecycles_published(session).await;
    }

    /// The act itself. `Ok` says what was done, `Err` why it was not.
    async fn rule_act_done(self: &Arc<Self>, row: &KeptRuleAct, found: &Found) -> Result<String, String> {
        let job = row.session_id.as_str();
        match &found.act {
            Act::StartWatch { recipe, values, deploy } => self.rule_watch_started(row, recipe, values, deploy.as_ref()).await,
            Act::Tell { message } => self.rule_told(job, message.as_str()).await,
            Act::Wake => self.rule_told(job, &format!("The rule {} woke you. Carry on.", found.rule)).await,
            Act::StartJob { workflow, prompt } => {
                let manifest = {
                    let store = self.store().lock().await;
                    store.session(job).map_err(|why| why.to_string())?.and_then(|one| one.manifest_id)
                };
                let manifest = manifest.ok_or("the Job is in no repository")?;
                // Checked before the Job is made, so a workflow the repository cannot run starts nothing.
                let named = workflow.as_ref().map(|one| one.as_str().to_string());
                let (workflow, _) = self.workflow_checked(&manifest, named).map_err(|why| why.error().message.clone())?;
                let start = StartAnotherSession {
                    manifest_id: ManifestId::carried(&manifest),
                    title: titled(prompt),
                    message: prompt.as_str().to_string(),
                    group: None,
                    after: None,
                };
                let held = self.another_session_started_by(job.to_string(), start).await.map_err(|why| why.error().message.clone())?;
                let started = held.session_id.as_str();
                // The rule the owner saved chose it, as his pick on the start card would. Not an error once
                // the Job is made: a retry would start another.
                if workflow.is_some() {
                    if let Err(why) = self.workflow_put(started, workflow, ipc::ChangedBy::You).await {
                        return Ok(format!("Started {started} with no workflow: {}", why.error().message));
                    }
                }
                Ok(format!("Started {started}"))
            }
            // Done once proposed: the draft's own wait is the ask, and Skip is withdrawing it.
            Act::Draft { template, .. } => {
                let draft = ProposeDraft {
                    title: match &row.on {
                        Some((lifecycle, key)) => format!("Message on {}", thing(lifecycle, key)),
                        None => String::from("Message"),
                    },
                    text: filled(template.as_str(), row.on.as_ref().map(|(lifecycle, key)| (lifecycle.as_str(), key.as_str()))),
                    before: None,
                    session_id: Some(SessionId::carried(job)),
                };
                self.draft_proposed(None, draft).await.map(|_| String::from("Drafted")).map_err(|why| why.error().message.clone())
            }
            Act::Tracker { write, .. } => self.rule_tracker_written(row, write, &found.lifecycles).await,
            Act::Forge { write, .. } => self.rule_forge_written(row, write).await,
            // Planned and recorded, and nothing here does it: `carries` keeps them from being planned for doing.
            Act::RunCommand { .. } | Act::RunSkill { .. } | Act::RunDrone { .. } => Err(String::from("Fleet does not do this kind of act")),
        }
    }

    async fn rule_told(self: &Arc<Self>, job: &str, text: &str) -> Result<String, String> {
        match self.told_by_fleet(job, text, Notice::LifecycleRule).await {
            Ok(true) => Ok(String::from("Told")),
            Ok(false) => Err(String::from("the agent is not listening, or the notice is muted")),
            Err(why) => Err(why.error().message.clone()),
        }
    }

    /// Start the watch as the agent's `start_watch` starts it. **A watch for a deploy takes the
    /// deploy on first**, as an instance the Job holds, and follows it, so what the watch reads is
    /// where the deploy stands. A deploy whose watch is still running is not started twice.
    async fn rule_watch_started(
        self: &Arc<Self>,
        row: &KeptRuleAct,
        recipe: &Words,
        values: &BTreeMap<Words, Words>,
        deploy: Option<&Words>,
    ) -> Result<String, String> {
        let job = row.session_id.as_str();
        // The pull request it is for: the one the act is done to, else the one whose entering set it off.
        let pull = row.on.iter().chain(row.entered.iter()).find(|(lifecycle, _)| lifecycle == PULL_REQUEST).map(|(_, key)| key.clone());
        if let Some(deploy) = deploy {
            let at = self.now().as_str().to_string();
            let links: Vec<(&str, &str)> = pull.iter().map(|key| (PULL_REQUEST, key.as_str())).collect();
            let mut store = self.store().lock().await;
            store.take_on_lifecycle_instance(job, DEPLOY, deploy.as_str(), &links, &at).map_err(|why| why.to_string())?;
            let following = store
                .lifecycle_instances_of(job)
                .map_err(|why| why.to_string())?
                .into_iter()
                .find(|one| one.lifecycle == DEPLOY && one.key == deploy.as_str())
                .map(|one| one.watch_id)
                .filter(|watch| !watch.is_empty());
            if let Some(watch) = following {
                let live = store.watches_of(job).map_err(|why| why.to_string())?.iter().any(|one| one.watch.live && one.watch.id == watch);
                if live {
                    return Ok(format!("Already followed by watch {watch}"));
                }
            }
        }
        let name = match deploy {
            Some(deploy) => format!("{} deploy", deploy.as_str()),
            None => recipe.as_str().to_string(),
        };
        let start = StartWatch {
            session_id: Some(SessionId::carried(job)),
            name,
            starts: WatchStarts::Recipe {
                recipe: recipe.as_str().to_string(),
                values: values.iter().map(|(name, value)| (name.as_str().to_string(), value.as_str().to_string())).collect(),
            },
            follows: pull.as_deref().and_then(|key| key.parse::<u64>().ok()),
            set_aside: Vec::new(),
        };
        let started = self.start_watch(None, start).await.map_err(|why| why.error().message.clone())?;
        if let Some(deploy) = deploy {
            let mut store = self.store().lock().await;
            store.follow_lifecycle_instance_with_watch(job, DEPLOY, deploy.as_str(), &started.id).map_err(|why| why.to_string())?;
        }
        Ok(format!("Started watch {}", started.id))
    }
}
