//! A Job's Commander under Fleet: the conversation the owner writes to, the
//! turns Fleet wakes it for, and what a turn costs. `#2166`; `docs/concepts/commander.md`
//! is the design and the decision record `2026-10-11-a-job-has-a-commander` the reason.
//!
//! **Helm's host, keyed by the Job.** The stored session, thread, process host
//! and turn are Helm's (`Fleet::turn_carried`); what is here is what differs.
//! **Nothing here advances a Job**: a turn's words are a thread, and the acts a
//! Commander takes are the door's, each recorded as its own.
//!
//! | Seam | Here | Filled by |
//! |---|---|---|
//! | A reason to wake it | [`Fleet::wake_commander`] | `#2167`, which decides when |
//! | A card that outlives a turn | none yet | `#2166`, second part |
//! | A repair Drone | none | `#2170` |

use std::sync::{Arc, Mutex};

use adapter_traits::{AgentHarness, Delivery, DroneEvent, Vcs, WorkProduct};
use api::{ObservedCommander, Refusal};
use core_model::{Job, JobId};
use ipc::{
    AskCommander, CommanderChanged, CommanderConversation, CommanderMessage, CommanderState,
    CommanderWoke, HelmFresh, HelmUnanswered, Instant, WireError,
};

use super::commander_brief;
use super::conversation::{Commanded, ConversationKey, Input};
use super::hosting::{Carry, Commander, Heard, Seat};
use super::rows::{injected, shown};
use crate::clock::Clock;
use crate::daemon::Fleet;

/// An owner's message to a Job that has no worktree to host a Commander in.
const NO_WORKTREE: &str = "fleet.commander_no_worktree";

/// Why a Commander could not be woken. **The caller escalates as it did before
/// there was a Commander**, and the line naming why is already in the Job's log.
#[derive(Clone, Debug, PartialEq, Eq)]
#[allow(dead_code)] // `#2167` calls the seam; only this crate's tests do until it lands.
pub(crate) struct CannotWake {
    pub(crate) why: String,
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
    /// `observe_commander`: the thread, with the subscription taken first.
    pub(super) async fn commander_observed(
        &self,
        job: api::Resolved,
    ) -> Result<ObservedCommander, Refusal> {
        let (_, commanded) = self.commander_of(&job.id().to_domain()).await?;
        let (live, history, skipped) = commanded.thread.watched();
        Ok(ObservedCommander {
            job_id: job.id(),
            replying: commanded.replying(),
            live,
            history,
            skipped,
        })
    }

    /// `ask_commander`: the owner's words, on the thread at once and in the next
    /// turn. **Refused where the Job has no worktree**, because there is
    /// nowhere to host it that is not the owner's checkout.
    pub(super) async fn commander_asked(
        self: Arc<Self>,
        job: api::Resolved,
        asked: AskCommander,
    ) -> Result<CommanderConversation, Refusal> {
        let job_id = job.id().to_domain();
        let (record, commanded) = self.commander_of(&job_id).await?;
        if let Err(why) = self.commander_directory(&record) {
            return Err(Refusal::IllegalMove(
                WireError::raised(NO_WORKTREE, why, self.run_id()).about_job(job.id()),
            ));
        }
        let key = ConversationKey::of_job(&job_id);
        let resumes = self
            .store()
            .lock()
            .await
            .helm_session(key.as_str())
            .map_err(|why| self.helm_fault(why))?
            .is_some();
        let text = String::from(asked.text);
        let start = commanded.owner_wrote(Instant::from(&self.now()), &text);
        self.stirred(job_id, key, commanded, start);
        Ok(CommanderConversation {
            job_id: job.id(),
            replying: true,
            resumes,
        })
    }

    /// **The seam `#2167` calls**: wake this Job's Commander for a reason Fleet
    /// can say in a line. A wake that arrives while a turn is running joins the
    /// queue, and the next turn takes every one waiting as one.
    ///
    /// `Err` where the Commander cannot start (the Job has no worktree), after
    /// a line in the Job's log naming why. The caller then escalates the Job as
    /// it does today.
    #[allow(dead_code)] // `CannotWake`'s reason.
    pub(crate) async fn wake_commander(
        self: Arc<Self>,
        job: &JobId,
        why: &str,
    ) -> Result<(), CannotWake> {
        let (record, commanded) = match self.commander_of(job).await {
            Ok(found) => found,
            Err(refusal) => {
                return Err(self.unable_to_wake(job, why, refusal.error().message.clone()).await)
            }
        };
        if let Err(reason) = self.commander_directory(&record) {
            return Err(self.unable_to_wake(job, why, reason).await);
        }
        let key = ConversationKey::of_job(job);
        let start = commanded.take(Input::Wake(why.to_string()));
        self.stirred(job.clone(), key, commanded, start);
        Ok(())
    }

    /// The line in the Job's log that names why its Commander could not start.
    #[allow(dead_code)] // `CannotWake`'s reason.
    async fn unable_to_wake(&self, job: &JobId, woken_for: &str, reason: String) -> CannotWake {
        self.noted_setting(
            job,
            "the Job's Commander could not be woken",
            &[("why", reason.clone()), ("woken_for", woken_for.to_string())],
        )
        .await;
        CannotWake { why: reason }
    }

    /// The Job, and the conversation its Commander has.
    async fn commander_of(&self, job: &JobId) -> Result<(Job, Arc<Commanded>), Refusal> {
        let record = self.load(job).await.map_err(|why| self.refusal(why))?;
        let served = self.served_by(&record).map_err(|why| self.refusal(why))?;
        let key = ConversationKey::of_job(job);
        let commanded = self.helm().commanded(&key, served.records_root());
        Ok((record, commanded))
    }

    /// Where the Commander opens: the Job's worktree, or why there is none.
    fn commander_directory(&self, record: &Job) -> Result<String, String> {
        match self.worktree_of(record) {
            Ok(Some(worktree)) => Ok(worktree.path().to_string()),
            Ok(None) => Err(String::from(
                "this Job has no worktree, so a Commander has nowhere to read it",
            )),
            Err(why) => Err(why.to_string()),
        }
    }

    /// Start the turns where none are running; `start` is what putting the
    /// input on the queue answered.
    fn stirred(
        self: &Arc<Self>,
        job: JobId,
        key: ConversationKey,
        commanded: Arc<Commanded>,
        start: bool,
    ) {
        if !start {
            return;
        }
        self.told_commander_state(&job, CommanderState::Working);
        let fleet = Arc::clone(self);
        tokio::spawn(async move {
            while let Some(inputs) = commanded.next(|| fleet.told_commander_state(&job, CommanderState::Idle))
            {
                fleet.commander_turn(&job, &key, &commanded, inputs).await;
            }
        });
    }

    fn told_commander_state(&self, job: &JobId, state: CommanderState) {
        self.events()
            .publish(ipc::Event::CommanderChanged(CommanderChanged {
                job_id: ipc::JobId::from(job),
                state,
            }));
    }

    /// One turn, start to finish: **always ended**, whatever happens, with the
    /// thread saying what and what it cost written down.
    async fn commander_turn(
        &self,
        job: &JobId,
        key: &ConversationKey,
        commanded: &Arc<Commanded>,
        inputs: Vec<Input>,
    ) {
        let _turn = commanded.turn.lock().await;
        self.swept_stale_helm_sessions().await;
        let mut woke: Vec<&str> = Vec::new();
        for input in &inputs {
            if let Input::Wake(why) = input {
                if !woke.contains(&why.as_str()) {
                    woke.push(why);
                    commanded.thread.say(CommanderMessage::Woke(CommanderWoke {
                        ts: Instant::from(&self.now()),
                        why: why.clone(),
                    }));
                }
            }
        }
        let heard = Rows {
            commanded: Arc::clone(commanded),
            clock: Arc::clone(self.clock()),
            ended: Mutex::new(None),
        };
        let outcome = self.commander_carried(job, key, &inputs, &heard).await;
        self.commander_paid(job, &heard).await;
        if let Err(why) = outcome {
            commanded
                .thread
                .say(CommanderMessage::Unanswered(HelmUnanswered {
                    ts: Instant::from(&self.now()),
                    why: why.clone(),
                }));
            self.noted_setting(
                job,
                "a turn of the Job's Commander did not come back",
                &[("why", why)],
            )
            .await;
        }
    }

    async fn commander_carried(
        &self,
        job: &JobId,
        key: &ConversationKey,
        inputs: &[Input],
        heard: &Rows,
    ) -> Result<(), String> {
        let record = self.load(job).await.map_err(|why| why.to_string())?;
        let directory = self.commander_directory(&record)?;
        let model = self.commander_model_for(job).await;
        let prompts = self.prompts();
        let named = format!("{} \"{}\"", record.handle(), record.title().as_str());
        self.turn_carried(
            key,
            heard,
            || {
                heard.commanded.thread.say(CommanderMessage::Fresh(HelmFresh {
                    ts: Instant::from(&self.now()),
                    because: ipc::Freshness::SessionNotFound,
                }))
            },
            |resuming| Carry {
                directory: directory.clone(),
                turn: commander_brief::told(&prompts, &named, inputs, resuming.is_some()),
                seat: Seat::Commander(Commander {
                    job: job.as_str().to_string(),
                    model: model.clone(),
                }),
                resuming,
            },
        )
        .await
    }

    /// Write down what the turn cost. **Best-effort**: a figure that will not
    /// write is a line in the Job's log and does not undo the turn. It lands in
    /// `job_commander_spend` and in no row the cap reads.
    async fn commander_paid(&self, job: &JobId, heard: &Rows) {
        let Some((cost_micros, turns)) = heard.spent() else {
            return;
        };
        let turn_id = self.mint().ulid();
        let wrote = self.store().lock().await.record_commander_spend(
            job,
            turn_id.as_str(),
            Some(cost_micros),
            turns,
        );
        if let Err(why) = wrote {
            self.noted_setting(
                job,
                "what a turn of the Job's Commander cost was not written down",
                &[("why", why.to_string())],
            )
            .await;
        }
    }
}

/// What a Commander's session says, as rows in its thread, and what its turn
/// ended up costing.
struct Rows {
    commanded: Arc<Commanded>,
    clock: Arc<dyn Clock>,
    /// The cost the stream last named and the turns it summed. **Cost is the
    /// last figure and turns are the sum**, `allowance::spent`'s measured fold.
    ended: Mutex<Option<(u64, u64)>>,
}

impl Rows {
    fn spent(&self) -> Option<(u64, u64)> {
        *self.ended.lock().expect("the figure is not held across a panic")
    }
}

impl Heard for Rows {
    fn heard(&self, events: &[DroneEvent]) {
        let at = self.clock.now();
        for event in events {
            if injected(event) {
                continue;
            }
            if let DroneEvent::Ended {
                turns, cost_micros, ..
            } = event
            {
                let mut ended = self
                    .ended
                    .lock()
                    .expect("the figure is not held across a panic");
                let summed = ended.map_or(0, |(_, turns)| turns) + u64::from(*turns);
                *ended = Some((*cost_micros, summed));
            }
            if let Some(shown) = shown(&at, event) {
                self.commanded.thread.say(CommanderMessage::Row(shown));
            }
        }
    }
}
