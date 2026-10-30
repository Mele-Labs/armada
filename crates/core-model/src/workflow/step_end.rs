//! What ends a step, as a step's `ends_on` lists it. Any one listed ends the step.
//! `workflowdef-fields.toml`, `ends_on`.
//!
//! Most are a thing the person does. **A lifecycle state is the one that is not**: the world
//! reaches it, Fleet reads that, and the step ends without anyone being asked.

use alloc::borrow::Cow;

use crate::lifecycle::LifecycleEnd;

/// One thing that ends a step.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
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
    /// Instances of a lifecycle the Job holds reach a state: `pr: approved`. **Added after the five
    /// above, so a copy written before it reads as it did.**
    Lifecycle(LifecycleEnd),
}

impl StepEnd {
    /// The words the person's acts are written as. The lifecycle form is not among them.
    pub const ALL: &'static [StepEnd] =
        &[StepEnd::Replied, StepEnd::WalkApproved, StepEnd::SketchApproved, StepEnd::DraftApproved, StepEnd::IdeaPicked];

    pub fn as_wire(&self) -> Cow<'static, str> {
        match self {
            StepEnd::Replied => Cow::Borrowed("replied"),
            StepEnd::WalkApproved => Cow::Borrowed("walk_approved"),
            StepEnd::SketchApproved => Cow::Borrowed("sketch_approved"),
            StepEnd::DraftApproved => Cow::Borrowed("draft_approved"),
            StepEnd::IdeaPicked => Cow::Borrowed("idea_picked"),
            StepEnd::Lifecycle(end) => Cow::Owned(end.as_wire()),
        }
    }

    /// A word of the person's acts, or a lifecycle end written as `as_wire` writes it.
    pub fn from_wire(value: &str) -> Option<StepEnd> {
        match StepEnd::ALL.iter().find(|end| end.as_wire() == value) {
            Some(word) => Some(word.clone()),
            None => LifecycleEnd::from_wire(value).ok().map(StepEnd::Lifecycle),
        }
    }

    /// The lifecycle end, where this is one.
    pub fn lifecycle(&self) -> Option<&LifecycleEnd> {
        match self {
            StepEnd::Lifecycle(end) => Some(end),
            _ => None,
        }
    }
}
