//! A Drone asking a person a question, and the answer coming back.
//!
//! # The two moves a Drone that does not know had, and why neither worked
//!
//! **Escalate** stops the Job — the step freezes and the worktree is held until
//! a person moves it — which is right for a Drone that is stuck and wrong for
//! one that needs a single fact. **Guess** is unprevented, and where a step's
//! output is Jobs the cost is Drones spending on work nobody asked for.
//! `crate::silence` names the hole from the other side: the poke's third branch
//! offers an escape hatch and is not sent because none existed. This is it.
//!
//! # Three things it is not, each settled elsewhere
//!
//! **Not a status** — the Job waits at `queued` reading `awaiting_answer`, the
//! shape `over_budget` and a parent waiting on its children already have.
//! **Not a conversation** — one question per Job, asked once and answered once
//! from a closed set the Drone offered, specified by `operations/`'s
//! `answer_question` row. **Not blocking** — a person's wait has no budget an
//! HTTP call could bound.
//!
//! # The asking Drone gives its place back
//!
//! The owner's decision of 11 Oct 2026 (#2094):
//! a Drone that asks stands down at once, its branch and worktree stay, and
//! admission gives its place to the next Job. The question is stored on the Job
//! so a restart in the hours before the answer loses nothing. Answering writes
//! the record, not a session, and the fresh Drone `crate::readmitting` puts
//! back on the same step opens with [`Answered`]. **The cost he took**: the
//! asking Drone's own context is gone, and the new one starts from the branch
//! and the question.
use core_model::{
    Actor, Component, DroneQuestion, Envelope, FieldValue, GivenAnswer, JobId, JobStatus, Level,
    OfferedAnswer, StepId, Target, Timestamp,
};
use store::ExtraEnded;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use ipc::mcp::AskQuestion;
use ipc::AnswerMode;

use crate::adrift::Adrift;
use crate::answer_mode::mode_line;
use crate::daemon::Fleet;

/// A question Fleet has just taken, on the step whose Drone asked it.
///
/// **No caller names the id.** Fleet mints it, because an answer naming an id a
/// peer invented joins to nothing — and the cost of a join that silently
/// succeeds against the wrong question is a dispatched Job.
fn minted(
    id: core_model::Ulid,
    step: StepId,
    asked_at: Timestamp,
    asking: AskQuestion,
) -> DroneQuestion {
    DroneQuestion {
        id,
        step,
        asked_at,
        question: asking.question,
        options: asking
            .options
            .into_iter()
            .map(|option| OfferedAnswer {
                label: option.label,
                consequence: option.consequence,
            })
            .collect(),
        answer: None,
    }
}

/// The wire shape, for `get_job` and for the event. **No step parameter**: a
/// question belongs to the step whose Drone asked it.
pub(crate) fn in_flight(asked: &DroneQuestion) -> ipc::QuestionInFlight {
    ipc::QuestionInFlight {
        question_id: ipc::QuestionId::from(&asked.id),
        step_id: ipc::StepId::from(&asked.step),
        asked_at: (&asked.asked_at).into(),
        question: asked.question.clone(),
        options: asked
            .options
            .iter()
            .map(|option| ipc::AskedOption {
                label: option.label.clone(),
                consequence: option.consequence.clone(),
            })
            .collect(),
    }
}

/// [`Answered`]'s frame as it ships, `prompts.crossingAnswered`'s default.
/// What fills it is filled where the brief is laid out, as a note's is.
pub(crate) const ANSWERED: &str =
    "WHAT A PERSON ANSWERED\n\nThe Drone before you on this part asked a person:\n\n  \
     \"{question}\"\n\nThey answered: {label}\nWhich that Drone said means: {consequence}\n\n\
     It stood down to wait for the answer, and its work so far is on the branch you are in. \
     Carry on this part from there and from the answer. The question is answered and closed \
     — do not ask it again.";

/// [`ANSWERED`]'s frame for an answer that left the choice to the Drone: the line, no label.
pub(crate) const ANSWERED_TO_THE_DRONE: &str =
    "WHAT A PERSON ANSWERED\n\nThe Drone before you on this part asked a person:\n\n  \"{question}\"\n\n\
     They left the choice to you. {deciding}\n\nIt stood down to wait for the answer, and its work so far \
     is on the branch you are in. Carry on this part from there. The question is answered and closed \
     — do not ask it again.";

/// A person's answer, carried into the opening brief of the Drone that takes
/// the part over.
///
/// **It restates the question**, because the Drone reading it is not the one
/// that asked, and a bare label would be a word with no antecedent. The
/// consequence is echoed because it is the asking Drone's own reading of what
/// that option commits to. There is no constructor taking free text: what
/// varies is the question and the option, and both came from a Drone.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Answered {
    question: String,
    label: Option<String>,
    consequence: Option<String>,
    /// The line that left the choice to the Drone. **The label wins where both are named**: the
    /// line rides with it.
    deciding: Option<String>,
}

impl Answered {
    /// The block for a Job whose question has been answered. `None` while it
    /// is unanswered and where nothing was asked.
    pub(crate) fn of(asked: &DroneQuestion) -> Option<Answered> {
        let deciding = asked.answer.as_ref()?.deciding.clone();
        let chosen = asked.chosen();
        if chosen.is_none() && deciding.is_none() {
            return None;
        }
        Some(Answered {
            question: asked.question.clone(),
            label: chosen.map(|chosen| chosen.label.clone()),
            consequence: chosen.map(|chosen| chosen.consequence.clone()),
            deciding,
        })
    }

    /// The block, in the words of `prompts.crossingAnswered`.
    pub fn text(&self, prompts: &crate::prompts::Prompts) -> String {
        match (&self.label, &self.consequence, &self.deciding) {
            (Some(label), Some(consequence), line) => {
                let text = prompts.fill(
                    config::settings::PROMPT_CROSSING_ANSWERED,
                    &[
                        ("question", &self.question),
                        ("label", label),
                        ("consequence", consequence),
                    ],
                );
                match line {
                    Some(line) => format!("{text}\n\n{line}"),
                    None => text,
                }
            }
            (_, _, Some(line)) => crate::prompts::fill(
                ANSWERED_TO_THE_DRONE,
                &[("question", &self.question), ("deciding", line)],
            ),
            _ => String::new(),
        }
    }
}

/// Why a question was not taken.
///
/// **None of these is a gate failure.** Nothing was verified and the step has
/// neither advanced nor failed; what the Drone is told is what is wrong and
/// what to do about it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NotAsked {
    /// The caller holds no slot, or the slot emptied between the call arriving
    /// and this reading it.
    NothingIsWorking,
    /// The Job is not `running`. A Drone at a human gate, or on a Job somebody
    /// has escalated, is not the thing a question can be outstanding on.
    NotRunning { status: JobStatus },
    /// One is already outstanding. **A Drone that could stack questions would
    /// be holding a conversation**, and a queue is a thing a person answers out
    /// of order.
    AlreadyAsking { question: String },
    /// The question would not go onto the record, or the Drone would not stand
    /// down. A question kept nowhere is one nobody could answer.
    NotKept { cause: String },
}

impl core::fmt::Display for NotAsked {
    fn fmt(&self, out: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            NotAsked::NothingIsWorking => write!(
                out,
                "no task is being worked on this connection, so there is no \
                 step for a question to be about"
            ),
            NotAsked::NotRunning { status } => write!(
                out,
                "this task is {} rather than running, so nobody is waiting on \
                 you to ask anything",
                status.as_wire()
            ),
            NotAsked::AlreadyAsking { question } => write!(
                out,
                "you are already waiting on an answer to: {question}. One \
                 question at a time — wait for that answer before asking \
                 another"
            ),
            NotAsked::NotKept { cause } => write!(
                out,
                "the question could not be kept for a person to answer: {cause}"
            ),
        }
    }
}

impl std::error::Error for NotAsked {}

/// Why an answer was not taken. **A person's refusal, not a Drone's**, so it
/// carries the acts a person has instead.
#[derive(Debug)]
pub enum NotAnswered {
    /// No question is outstanding on this Job.
    NothingIsAsking { job: JobId },
    /// The window was open across an answer, so this names a question that is
    /// gone. **Refused rather than applied to whatever is outstanding now**: a
    /// label matching the newer question by chance would dispatch work nobody
    /// chose.
    Superseded { job: JobId },
    /// Neither a label nor a mode: there is nothing to tell the Drone.
    NothingChosen { job: JobId },
    /// The label is not one the Drone offered.
    NotOffered { chose: String, offered: Vec<String> },
    /// The answer would not go onto the record. **The question stays
    /// outstanding**, so a person can answer again.
    NotKept(store::WriteError),
}

impl core::fmt::Display for NotAnswered {
    fn fmt(&self, out: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            NotAnswered::NothingIsAsking { job } => {
                let id = job.as_str();
                write!(out, "job {id} has no drone waiting on an answer")
            }
            NotAnswered::Superseded { job } => write!(
                out,
                "the question being answered on job {} is not the one \
                 outstanding. Read the job again and answer the one it names",
                job.as_str()
            ),
            NotAnswered::NothingChosen { job } => write!(
                out,
                "the answer on job {} names neither one of the drone's answers nor a mode",
                job.as_str()
            ),
            NotAnswered::NotOffered { chose, offered } => write!(
                out,
                "`{chose}` is not one of the answers the drone offered. It \
                 offered {}",
                offered.join(", ")
            ),
            NotAnswered::NotKept(cause) => write!(out, "the answer was not kept: {cause}"),
        }
    }
}

impl std::error::Error for NotAnswered {}

/// What answering did, for a caller that wants to say so.
#[derive(Debug)]
pub struct Told {
    pub job: JobId,
    pub step: StepId,
    /// The label the person picked. Absent where they left it to the Drone.
    pub chose: Option<String>,
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
    /// Take a question from the working Drone, keep it on the Job, and stand
    /// every Drone on the Job down.
    ///
    /// The Drone names no Job and no step; both are read out of **its own** slot
    /// under that slot's lock, exactly as `declare_scope` reads them, so a
    /// question cannot be aimed at some other step.
    ///
    /// **The question is written before the Job moves**, `crate::pausing`'s
    /// order: admission reads it, and a `queued` Job without it is a Job it
    /// would start again at once.
    pub async fn ask_question(
        &self,
        caller: &JobId,
        asking: AskQuestion,
    ) -> Result<DroneQuestion, NotAsked> {
        let Some(slot) = self.slot_of(caller).await else {
            return Err(NotAsked::NothingIsWorking);
        };
        let mut working = slot.lock().await;
        let Some(at_work) = working.as_ref() else {
            return Err(NotAsked::NothingIsWorking);
        };
        let (job, step, _) = at_work.standing();
        let record = self
            .load(&job)
            .await
            .map_err(|_| NotAsked::NothingIsWorking)?;
        if let Some(held) = record.question().filter(|_| record.waiting_on_an_answer()) {
            return Err(NotAsked::AlreadyAsking {
                question: held.question.clone(),
            });
        }
        // Read at all because a Job can be escalated over a live Drone —
        // `stalled` is exactly that — and a question on a Job a person has
        // already stopped is a second thing to answer about a Job they own.
        if record.status() != JobStatus::Running {
            return Err(NotAsked::NotRunning {
                status: record.status(),
            });
        }
        let now = self.now();
        let asked = minted(self.mint().ulid(), step.clone(), now.clone(), asking);
        let not_kept = |cause: String| NotAsked::NotKept { cause };
        self.store()
            .lock()
            .await
            .record_question(&record.asking(asked.clone()))
            .map_err(|why| not_kept(why.to_string()))?;
        self.noted_asked(&job, &step, &asked);
        // A Drone beside the kept one ends in its own slot, while its lock is
        // held, so the crew sweep below finds it already empty.
        if let Some(drone) = crate::crew::calling() {
            self.end_one_beside(&job, &drone, &mut working, ExtraEnded::Killed)
                .await
                .map_err(|why| not_kept(why.to_string()))?;
        }
        drop(working);
        self.stood_down_to_wait(&job)
            .await
            .map_err(|why| not_kept(why.to_string()))?;
        self.events().publish(ipc::Event::JobAsking(ipc::JobAsking {
            job_id: ipc::JobId::from(&job),
            step_id: ipc::StepId::from(&step),
            asking: Some(in_flight(&asked)),
            // **The Drone, not Fleet.** Fleet caused nothing here; it took
            // a question a Drone chose to ask.
            actor: Actor::Drone.into(),
            at: (&now).into(),
        }));
        Ok(asked)
    }

    /// Put the Job back in the queue and end every Drone on it. The step
    /// stays `running`: the run is not over, it is waiting, and the Drone
    /// re-admission puts back continues it rather than starting a new one.
    async fn stood_down_to_wait(&self, job_id: &JobId) -> Result<(), Adrift> {
        let job = self.load(job_id).await?;
        self.move_job(&job, Target::Queued, Actor::Fleet).await?;
        self.end_the_crew(job_id, ExtraEnded::Killed).await;
        // The kept slot by name: a beside Drone's call is answered under its
        // own name, and `slot_of` would hand back its slot.
        let kept = self.slots().lock().await.slot_of(job_id);
        match kept {
            Some(slot) => {
                let mut working = slot.lock().await;
                self.stood_down(job_id, &mut working).await?;
            }
            None => self.every_exit_recorded(job_id).await?,
        }
        Ok(())
    }

    /// Keep a person's answer on the Job.
    ///
    /// **It starts nothing.** The Job is `queued` and was held there for want
    /// of this; with it on the record admission takes the Job in its turn, and
    /// `crate::spawning` folds the answer into the fresh Drone's brief.
    ///
    /// The actor is **human**. Fleet answers nothing of its own accord.
    pub async fn answer_question(
        &self,
        job_id: &JobId,
        question_id: &str,
        chose: Option<&str>,
        mode: Option<AnswerMode>,
    ) -> Result<Told, NotAnswered> {
        let nothing = || NotAnswered::NothingIsAsking {
            job: job_id.clone(),
        };
        let job = self.load(job_id).await.map_err(|_| nothing())?;
        let Some(asked) = job.question() else {
            return Err(nothing());
        };
        if asked.id.as_str() != question_id {
            return Err(NotAnswered::Superseded {
                job: job_id.clone(),
            });
        }
        if !job.waiting_on_an_answer() {
            return Err(nothing());
        }
        let chosen = match chose {
            Some(chose) => Some(asked.offering(chose).ok_or_else(|| NotAnswered::NotOffered {
                chose: chose.to_string(),
                offered: asked.labels().iter().map(|held| held.to_string()).collect(),
            })?),
            None if mode.is_none() => return Err(NotAnswered::NothingChosen { job: job_id.clone() }),
            None => None,
        };
        let step = asked.step.clone();
        let label = chosen.map(|chosen| chosen.label.clone());
        let now = self.now();
        self.store()
            .lock()
            .await
            .record_question(&job.question_answered(GivenAnswer {
                label: label.clone(),
                deciding: mode.map(|mode| mode_line(mode).to_string()),
                at: now.clone(),
            }))
            .map_err(NotAnswered::NotKept)?;
        self.noted_answered(job_id, &step, question_id, label.as_deref(), mode);
        self.events().publish(ipc::Event::JobAsking(ipc::JobAsking {
            job_id: ipc::JobId::from(job_id),
            step_id: ipc::StepId::from(&step),
            // **Absent because it was answered.** What was chosen is in the
            // Job's own log; a field for it here would be a second place a
            // decision is recorded.
            asking: None,
            actor: Actor::Human.into(),
            at: (&now).into(),
        }));
        Ok(Told {
            job: job_id.clone(),
            step,
            chose: label,
        })
    }

    /// Write the question into the Job's own log. **Fields, never an
    /// interpolated message**, so a query can find every step that had to ask
    /// — which is the number that says whether a workflow's brief is enough.
    fn noted_asked(&self, job: &JobId, step: &StepId, asked: &DroneQuestion) {
        let envelope = Envelope::new(
            self.now(),
            Level::Info,
            Component::Fleet,
            self.run().clone(),
            crate::retro::lines::A_DRONE_ASKS,
        )
        .in_job(job.as_ulid().clone())
        .at_step(step.as_str())
        .with_field(
            "question_id",
            FieldValue::Str(asked.id.as_str().to_string()),
        )
        .with_field("question", FieldValue::Str(asked.question.clone()))
        .with_field("offered", FieldValue::Str(asked.labels().join(" | ")));
        // A log line that will not write does not fail the ask, for
        // `crate::silence::noted_quiet`'s reason: what happened is on the
        // record, and the event is published either way.
        self.noted_in_the_log(job, &envelope);
    }

    /// Write the answer into the Job's own log. **Where a person's decision is
    /// recorded, beside the verdicts** — readable afterwards by somebody working
    /// out why a Job was split the way it was.
    fn noted_answered(
        &self,
        job: &JobId,
        step: &StepId,
        question_id: &str,
        chose: Option<&str>,
        mode: Option<AnswerMode>,
    ) {
        let envelope = Envelope::new(
            self.now(),
            Level::Info,
            Component::Fleet,
            self.run().clone(),
            crate::retro::lines::A_PERSON_ANSWERS_A_DRONE,
        )
        .in_job(job.as_ulid().clone())
        .at_step(step.as_str())
        .with_field("question_id", FieldValue::Str(question_id.to_string()))
        .with_field("chose", FieldValue::Str(chose.unwrap_or_default().to_string()))
        .with_field("mode", FieldValue::Str(mode.map_or("", mode_name).to_string()))
        .with_field("actor", FieldValue::Str(Actor::Human.as_wire().to_string()));
        self.noted_in_the_log(job, &envelope);
    }
}

/// The mode as the log spells it, the wire's own word.
pub(crate) fn mode_name(mode: AnswerMode) -> &'static str {
    match mode {
        AnswerMode::Best => "best",
        AnswerMode::Quick => "quick",
    }
}

/// The question this Job is waiting on, for `get_job` and the Board row.
/// `None` once it is answered, though the answer waits on the record for the
/// next Drone's brief.
pub(crate) fn question_awaited(job: &core_model::Job) -> Option<ipc::QuestionInFlight> {
    job.question()
        .filter(|_| job.waiting_on_an_answer())
        .map(in_flight)
}

/// The refusal a Drone reads, from the tool's own error.
impl From<NotAsked> for ipc::mcp::NotRecorded {
    fn from(why: NotAsked) -> ipc::mcp::NotRecorded {
        ipc::mcp::NotRecorded {
            because: why.to_string(),
        }
    }
}

/// The Adrift a person's refusal crosses as. `IllegalMove` in the three a
/// person can act on; a record that would not write is the store's.
impl NotAnswered {
    pub(crate) fn about(self, job: &JobId) -> Adrift {
        match self {
            NotAnswered::NotKept(cause) => Adrift::Writing(cause),
            refused => Adrift::NotAnswerable {
                job: job.clone(),
                because: refused.to_string(),
            },
        }
    }
}
