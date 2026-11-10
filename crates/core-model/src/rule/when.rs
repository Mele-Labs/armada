//! When a rule runs: an instance enters a state, a Job step starts or ends, a gate of the Job's plan
//! closes, or a Job event happens.

use crate::lifecycle::{LifecycleId, StateId};
use crate::workflow::StepId;

/// Something that happens to a Job as a whole, which Fleet sees without asking anyone.
///
/// **A closed set**, and a new event is a new variant, so every `match` on it fails to compile
/// until it is handled. A step starting or ending is its own [`When`], and not one of these.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum JobEvent {
    /// The Job was started.
    Started,
    /// The Job opened a pull request.
    PullRequestOpened,
    /// The Job's pull request landed on `main`.
    Landed,
    /// The Job ended, whether or not its pull request landed.
    Ended,
}

impl JobEvent {
    pub const ALL: &'static [JobEvent] =
        &[JobEvent::Started, JobEvent::PullRequestOpened, JobEvent::Landed, JobEvent::Ended];

    /// The spelling a rule file uses.
    pub fn as_wire(self) -> &'static str {
        match self {
            JobEvent::Started => "started",
            JobEvent::PullRequestOpened => "pull_request_opened",
            JobEvent::Landed => "landed",
            JobEvent::Ended => "ended",
        }
    }

    pub fn from_wire(value: &str) -> Option<JobEvent> {
        JobEvent::ALL.iter().copied().find(|event| event.as_wire() == value)
    }
}

/// A kind of gate a Job's plan names, the fact that ends a group of it. **A closed set, and the same
/// words as `ipc::PlanGate` without its `none`**: a gate that is no gate has no moment.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Gate {
    PlanApproved,
    WalkApproved,
    SketchApproved,
    DocumentApproved,
    YouApproved,
    ChecksGreen,
    Merged,
    JobsEnded,
}

impl Gate {
    pub const ALL: &'static [Gate] =
        &[Gate::PlanApproved, Gate::WalkApproved, Gate::SketchApproved, Gate::DocumentApproved, Gate::YouApproved, Gate::ChecksGreen, Gate::Merged, Gate::JobsEnded];

    /// The spelling a rule file and the plan use.
    pub fn as_wire(self) -> &'static str {
        match self {
            Gate::PlanApproved => "plan_approved",
            Gate::WalkApproved => "walk_approved",
            Gate::SketchApproved => "sketch_approved",
            Gate::DocumentApproved => "document_approved",
            Gate::YouApproved => "you_approved",
            Gate::ChecksGreen => "checks_green",
            Gate::Merged => "merged",
            Gate::JobsEnded => "jobs_ended",
        }
    }

    pub fn from_wire(value: &str) -> Option<Gate> {
        Gate::ALL.iter().copied().find(|gate| gate.as_wire() == value)
    }
}

/// The moment a rule is for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum When {
    /// An instance of `lifecycle` enters `state`.
    Enters { lifecycle: LifecycleId, state: StateId },
    /// A step of the Job starts.
    StepStarts(StepId),
    /// A step of the Job ends.
    StepEnds(StepId),
    /// A gate of the kind closes on the Job's plan, whichever group names it.
    GateCloses(Gate),
    /// The Job as a whole does something.
    Job(JobEvent),
}

impl When {
    /// Whether the moment is one instance's. **Only then is there an instance for a condition or
    /// an act to speak of** as the one that entered, or the ones linked to it.
    pub fn has_an_instance(&self) -> bool {
        matches!(self, When::Enters { .. })
    }

    /// The lifecycle of the instance that enters, where the moment is one's.
    pub fn lifecycle_entered(&self) -> Option<&LifecycleId> {
        match self {
            When::Enters { lifecycle, .. } => Some(lifecycle),
            _ => None,
        }
    }
}
