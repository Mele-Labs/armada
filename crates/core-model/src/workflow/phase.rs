//! Which part of a Job's run a workflow step belongs to: setting it up, the
//! work, or delivering it. The approval canvas lays its lanes by it (the owner,
//! 4 Oct 2026), and #1768's progress across workflows reads it too.

/// The three phases, in the order a Job meets them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum StepPhase {
    Setup,
    Work,
    Delivery,
}

impl StepPhase {
    pub const ALL: &'static [StepPhase] = &[StepPhase::Setup, StepPhase::Work, StepPhase::Delivery];

    pub fn as_wire(&self) -> &'static str {
        match self {
            StepPhase::Setup => "setup",
            StepPhase::Work => "work",
            StepPhase::Delivery => "delivery",
        }
    }

    pub fn from_wire(value: &str) -> Option<StepPhase> {
        StepPhase::ALL
            .iter()
            .copied()
            .find(|phase| phase.as_wire() == value)
    }

    /// A step's phase where it declares none: **delivery where it delivers,
    /// the work otherwise**, and never read off its id or its place.
    pub fn of(declared: Option<StepPhase>, delivers: bool) -> StepPhase {
        declared.unwrap_or(if delivers {
            StepPhase::Delivery
        } else {
            StepPhase::Work
        })
    }
}
