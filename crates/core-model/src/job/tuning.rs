//! What a person tunes on one step at the approval press, beyond its gate: the
//! model and effort its Drone runs at, words handed to that Drone, how many
//! Judges answer, and a Check turned off. The approval canvas, 4 Oct 2026.
//!
//! **Frozen into the step**, as the gate is (`ResolvedStep::gated_by_person`),
//! so the spawn, the brief and the gate read it where they already read the
//! step, and the Job keeps it for its life.

use alloc::string::String;
use alloc::vec::Vec;

use crate::job::ids::ModelName;

/// How hard a step's Drone thinks. **A step with none is Armada picking**,
/// which is the harness's own default and has no spelling here.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Effort {
    Low,
    Medium,
    High,
}

impl Effort {
    pub const ALL: &'static [Effort] = &[Effort::Low, Effort::Medium, Effort::High];

    pub fn as_wire(&self) -> &'static str {
        match self {
            Effort::Low => "low",
            Effort::Medium => "medium",
            Effort::High => "high",
        }
    }

    pub fn from_wire(value: &str) -> Option<Effort> {
        Effort::ALL
            .iter()
            .copied()
            .find(|effort| effort.as_wire() == value)
    }
}

/// One step's tuning, already read against the step. **Each `None` and the
/// empty list is the step as its workflow declares it.**
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StepTuning {
    pub model: Option<ModelName>,
    pub effort: Option<Effort>,
    /// Words for the step's Drone, beside its brief. Never blank.
    pub context: Option<String>,
    /// How many Judges answer each criterion. Never zero.
    pub judges: Option<u32>,
    /// Manifest Checks this Job does not run on the step, by name.
    pub checks_off: Vec<String>,
}
