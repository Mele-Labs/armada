//! A Job's moves and a Session's row, turned into timeline facts as they are
//! published.
//!
//! **A Job's own events carry the instant they happened**, and are dated by it.
//! A Session row is replaced whole, so what changed is read against what this
//! saw last: the first sighting is its start, a waiting item that was not there
//! is an ask (or, where the agent listed it, a wait), one that left the agent's list is resolved,
//! a subagent attached or spent is its start or finish, and live turning to ended is its end.
//! That memory starts empty,
//! so a Session already live when Fleet starts is entered as started the first
//! time its row changes, with the start the row states.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};

use core_model::{CheckOutcome, JobStatus, StepState};
use ipc::{AttachmentState, Event, SessionRecord, SessionState, TimelineKind, TimelineTone, WaitingSource};

use super::{bare, Fact, Subject, Timeline, SAYS};
use crate::naming::Names;

/// A Job's files changing is entered at most this often, in milliseconds.
const FILES_EVERY: i64 = 60_000;

/// What was last seen, so a repeat is not entered twice.
#[derive(Default)]
struct Seen {
    sessions: BTreeMap<String, SeenSession>,
    /// A Check's result, by Job, the run of the step and the Check's name.
    checks: BTreeSet<(String, u32, String)>,
    /// When a Job's files were last entered, in milliseconds.
    files: BTreeMap<String, i64>,
}

#[derive(Default)]
struct SeenSession {
    ended: bool,
    items: BTreeSet<String>,
    /// What the agent listed itself, by id, so its leaving the list can say what it was.
    listed: BTreeMap<String, String>,
    /// Each subagent the row held, and the state it was last in.
    subagents: BTreeMap<String, AttachmentState>,
}

/// The tap on the event stream.
pub(super) struct Hearing {
    names: Arc<Names>,
    timeline: Timeline,
    seen: Mutex<Seen>,
}

impl Hearing {
    pub(super) fn new(names: Arc<Names>, timeline: Timeline) -> Hearing {
        Hearing {
            names,
            timeline,
            seen: Mutex::new(Seen::default()),
        }
    }

    /// Queue what this event is, if it is anything. Runs inside `publish`.
    pub(super) fn hear(&self, event: &Event) {
        let Ok(mut seen) = self.seen.lock() else {
            return;
        };
        for fact in self.facts(event, &mut seen) {
            self.timeline.note(fact);
        }
    }

    /// A Job's fact, titled and placed from the index when the event names only its id.
    fn job(
        &self,
        id: &ipc::JobId,
        at: &ipc::Instant,
        kind: TimelineKind,
        tone: TimelineTone,
        says: impl Into<String>,
    ) -> Fact {
        let domain = id.to_domain();
        Fact::new(at.as_str(), kind, Subject::Job(id.as_str().to_string()), tone, says)
            .titled(self.names.of(&domain))
            .in_repository(self.names.owner_of(&domain))
    }

    /// The same, from an event that carries the Job whole.
    fn summarised(
        job: &ipc::JobSummary,
        at: &ipc::Instant,
        kind: TimelineKind,
        tone: TimelineTone,
        says: impl Into<String>,
    ) -> Fact {
        Fact::new(at.as_str(), kind, Subject::Job(job.id.as_str().to_string()), tone, says)
            .titled(Some(job.handle.clone()))
            .in_repository(Some(job.owner_manifest_id.as_str().to_string()))
    }

    fn facts(&self, event: &Event, seen: &mut Seen) -> Vec<Fact> {
        use TimelineKind as K;
        use TimelineTone as T;
        match event {
            Event::JobCreated(made) => {
                vec![Self::summarised(&made.job, &made.at, K::JobCreated, T::Idle, "Dispatched")]
            }
            Event::JobStateChanged(moved) => {
                let named = moved.reason.as_ref().and_then(|reason| reason.named.clone());
                let job = |kind, tone, says: &str| self.job(&moved.job_id, &moved.at, kind, tone, says);
                let one = match moved.to.domain() {
                    JobStatus::Queued => job(K::JobQueued, T::Idle, "Queued"),
                    JobStatus::Running if moved.from.domain() != JobStatus::Running => {
                        job(K::JobStarted, T::Run, "Started").live()
                    }
                    JobStatus::AwaitingReview => job(K::JobAsked, T::Ask, "Needs review"),
                    JobStatus::AwaitingRepair => job(K::JobAsked, T::Ask, "Needs repair"),
                    JobStatus::AwaitingAttestation => job(K::JobAsked, T::Ask, "Needs attestation"),
                    JobStatus::Escalated => job(K::JobAsked, T::Ask, "Escalated"),
                    JobStatus::CompletedFailed => job(K::JobFailed, T::Bad, "Failed").over(),
                    JobStatus::Rejected => job(K::JobFailed, T::Bad, "Rejected").over(),
                    JobStatus::Killed => job(K::JobFailed, T::Idle, "Killed").over(),
                    _ => return Vec::new(),
                };
                vec![one.detail(named)]
            }
            Event::JobStepAdvanced(step) => {
                let name = capitalised(step.step_id.as_str());
                let (tone, says) = match step.to.domain() {
                    StepState::Advanced => (T::Ok, format!("{name} passed")),
                    StepState::Running => (T::Run, format!("{name} started")),
                    StepState::Retrying => (T::Run, format!("{name} started again")),
                    StepState::Stopped => (T::Bad, format!("{name} stopped")),
                    StepState::AwaitingHuman => (T::Ask, format!("{name} needs you")),
                    StepState::NotStarted => return Vec::new(),
                };
                vec![Self::summarised(&step.job, &step.at, K::JobStep, tone, says)
                    .detail(step.why.clone())
                    .live()]
            }
            Event::JobChecking(checking) => {
                let job = checking.job_id.as_str().to_string();
                let Some(underway) = &checking.checking else {
                    seen.checks.retain(|(held, ..)| *held != job);
                    return Vec::new();
                };
                underway
                    .checks
                    .iter()
                    .filter_map(|one| {
                        let ran = one.ran.as_ref()?;
                        let (tone, said) = match ran.outcome.domain() {
                            CheckOutcome::Passed => (T::Ok, "passed"),
                            CheckOutcome::Failed | CheckOutcome::Signalled => (T::Bad, "failed"),
                            CheckOutcome::TimedOut => (T::Bad, "timed out"),
                            CheckOutcome::NeverRan => (T::Bad, "did not run"),
                            CheckOutcome::Skipped => return None,
                        };
                        seen.checks
                            .insert((job.clone(), underway.attempt, one.name.clone()))
                            .then(|| {
                                self.job(
                                    &checking.job_id,
                                    &checking.at,
                                    K::JobCheck,
                                    tone,
                                    format!("Check {} {said}", one.name),
                                )
                                .detail(Some(one.name.clone()))
                                .live()
                            })
                    })
                    .collect()
            }
            Event::JobJudging(judging) => judging
                .judging
                .as_ref()
                .map(|call| {
                    vec![self
                        .job(&judging.job_id, &judging.at, K::JobJudged, T::Run, format!("Judge reads {}", call.look))
                        .live()]
                })
                .unwrap_or_default(),
            Event::JobFilesChanged(changed) => {
                let job = changed.job_id.as_str().to_string();
                let at = changed.at.to_domain().epoch_millis().unwrap_or_default();
                if changed.files.is_empty()
                    || seen.files.get(&job).is_some_and(|last| at - last < FILES_EVERY)
                {
                    return Vec::new();
                }
                seen.files.insert(job, at);
                let n = changed.files.len();
                let says = if n == 1 { "1 file changed".to_string() } else { format!("{n} files changed") };
                vec![self
                    .job(&changed.job_id, &changed.at, K::JobFiles, T::Run, says)
                    .detail(changed.files.first().map(|file| file.path.clone()))
                    .live()]
            }
            Event::JobAsking(asking) => asking
                .asking
                .as_ref()
                .map(|question| {
                    let says = match question.decisions.len() {
                        0 => format!("Asked: {}", question.question),
                        n => format!("Asked {n} decisions"),
                    };
                    vec![self.job(&asking.job_id, &asking.at, K::JobAsked, T::Ask, says)]
                })
                .unwrap_or_default(),
            Event::JobCommandWaiting(waiting) => waiting
                .waiting
                .as_ref()
                .map(|command| {
                    vec![self
                        .job(&waiting.job_id, &waiting.at, K::JobAsked, T::Ask, "Asked to run a command")
                        .detail(Some(command.detail.clone()))]
                })
                .unwrap_or_default(),
            Event::JobLanded(landed) => {
                vec![Self::summarised(&landed.job, &landed.at, K::JobLanded, T::Ok, "Landed on main").over()]
            }
            Event::SessionChanged(record) => Self::session(record, seen),
            _ => Vec::new(),
        }
    }

    fn session(record: &SessionRecord, seen: &mut Seen) -> Vec<Fact> {
        use TimelineKind as K;
        use TimelineTone as T;
        let id = record.id.as_str().to_string();
        let fact = |at: &ipc::Instant, kind, tone, says: String| {
            Fact::new(at.as_str(), kind, Subject::Session(id.clone()), tone, says)
                .titled(record.title.clone())
                .in_repository(record.manifest_id.as_ref().map(|id| id.as_str().to_string()))
        };
        let mut facts = Vec::new();
        let first = !seen.sessions.contains_key(&id);
        let before = seen.sessions.entry(id.clone()).or_default();
        if first {
            facts.push(fact(&record.started_at, K::SessionStarted, T::Run, "Started".to_string()).live());
        }
        for item in &record.waiting_for {
            if item.source == WaitingSource::Walk || before.items.contains(&item.id) {
                continue;
            }
            // What the agent listed is its own to say it waits on; the rest Fleet raised.
            facts.push(match item.source {
                WaitingSource::Agent => {
                    fact(&item.since, K::SessionWaiting, T::Ask, bare(&format!("Waiting: {}", item.text), SAYS))
                }
                _ => fact(&item.since, K::SessionAsked, T::Ask, bare(&format!("Asked: {}", item.text), SAYS)),
            });
        }
        before.items = record.waiting_for.iter().map(|item| item.id.clone()).collect();
        let listed: BTreeMap<String, String> = record
            .waiting_for
            .iter()
            .filter(|item| item.source == WaitingSource::Agent)
            .map(|item| (item.id.clone(), item.text.clone()))
            .collect();
        // An ended Session's list is cleared with it, which resolves nothing.
        if record.state == SessionState::Live {
            for (_, text) in before.listed.iter().filter(|(id, _)| !listed.contains_key(*id)) {
                let says = bare(&format!("Resolved: {text}"), SAYS);
                facts.push(fact(&record.last_seen_at, K::SessionResolved, T::Ok, says));
            }
        }
        before.listed = listed;
        for one in record.attachments.iter().filter(|one| one.kind == "subagent") {
            let spent = one.state == AttachmentState::Spent;
            let known = before.subagents.insert(one.target.clone(), one.state);
            // A Session already held subagents when Fleet first saw it: they began before this.
            if first {
                continue;
            }
            let said = |key: &str| one.detail.get(key).map(String::as_str).unwrap_or_default();
            let what = [said("type"), said("description")].iter().filter(|part| !part.is_empty()).cloned().collect::<Vec<_>>().join(": ");
            let subagent = |at: &ipc::Instant, tone, says: &str| {
                fact(at, K::SessionSubagent, tone, says.to_string()).detail(Some(what.clone()))
            };
            if known.is_none() {
                facts.push(subagent(&one.since, T::Run, "Subagent started").live());
            }
            // Its parent sent it more work: standing again, from when it started on it.
            if one.state == AttachmentState::Standing && known.is_some_and(|was| was != AttachmentState::Standing) {
                facts.push(subagent(&one.since, T::Run, "Subagent resumed").live());
            }
            if spent && known != Some(AttachmentState::Spent) {
                facts.push(subagent(&one.changed_at, T::Ok, "Subagent finished"));
            }
        }
        let ended = record.state == SessionState::Ended;
        if ended && !before.ended {
            let at = record.ended_at.as_ref().unwrap_or(&record.last_seen_at);
            facts.push(fact(at, K::SessionEnded, T::Idle, "Ended".to_string()).detail(record.end_reason.clone()).over());
        }
        before.ended = ended;
        facts
    }
}

fn capitalised(word: &str) -> String {
    let mut chars = word.chars();
    chars
        .next()
        .map(|first| first.to_uppercase().chain(chars).collect())
        .unwrap_or_default()
}
