//! What ends a workflow step's plan group: a fact Fleet sees for itself.
//!
//! The words are the plan's own gate words (`ipc::PlanGate`). core-model cannot depend on ipc, so
//! the list is spelled here and a Fleet test holds the two equal.

/// The gate a step's plan group ends on. `None` is a group with no gate.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum StepGate {
    PlanApproved,
    WalkApproved,
    SketchApproved,
    DocumentApproved,
    ChecksGreen,
    Merged,
    #[default]
    None,
}

impl StepGate {
    /// Every gate, in the order the plan lists them.
    pub const ALL: [StepGate; 7] = [
        StepGate::PlanApproved,
        StepGate::WalkApproved,
        StepGate::SketchApproved,
        StepGate::DocumentApproved,
        StepGate::ChecksGreen,
        StepGate::Merged,
        StepGate::None,
    ];

    /// The word a workflow file and the plan write.
    pub fn word(self) -> &'static str {
        match self {
            StepGate::PlanApproved => "plan_approved",
            StepGate::WalkApproved => "walk_approved",
            StepGate::SketchApproved => "sketch_approved",
            StepGate::DocumentApproved => "document_approved",
            StepGate::ChecksGreen => "checks_green",
            StepGate::Merged => "merged",
            StepGate::None => "none",
        }
    }

    /// The gate a word names, or `None` (the Option) for a word no gate has.
    pub fn from_word(word: &str) -> Option<StepGate> {
        StepGate::ALL.into_iter().find(|gate| gate.word() == word)
    }

    /// Whether this is the absent gate.
    pub fn is_none(self) -> bool {
        self == StepGate::None
    }
}
