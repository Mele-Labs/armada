//! What the person does that ends a step, as a step's `ends_on` lists it. Any one listed ends the
//! step. `workflowdef-fields.toml`, `ends_on`.

/// One thing the person does that ends a step.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum StepEnd {
    /// He writes after the agent's first turn. What `awaits_reply: true` means.
    Replied,
    /// He approves a window the Job showed.
    WalkApproved,
    /// He approves a sketch the Job showed.
    SketchApproved,
    /// He approves a draft the Job proposed, as written or with his own text.
    DraftApproved,
    /// He answers the Job's question by picking an option it gave a preview.
    IdeaPicked,
}

impl StepEnd {
    pub const ALL: &'static [StepEnd] =
        &[StepEnd::Replied, StepEnd::WalkApproved, StepEnd::SketchApproved, StepEnd::DraftApproved, StepEnd::IdeaPicked];

    pub fn as_wire(&self) -> &'static str {
        match self {
            StepEnd::Replied => "replied",
            StepEnd::WalkApproved => "walk_approved",
            StepEnd::SketchApproved => "sketch_approved",
            StepEnd::DraftApproved => "draft_approved",
            StepEnd::IdeaPicked => "idea_picked",
        }
    }

    pub fn from_wire(value: &str) -> Option<StepEnd> {
        StepEnd::ALL.iter().copied().find(|end| end.as_wire() == value)
    }
}
