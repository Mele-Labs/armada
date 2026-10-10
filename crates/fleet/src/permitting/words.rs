//! The permission turns, as Armada ships them: each the default of a
//! `prompts.permit…` key in settings.json.

/// A person's answer to a permission question, in the words that ship: each
/// the default of a `prompts.permit…` key. `{rule}` is what was kept and
/// `{command}` what the Drone ran.
pub(crate) const PERMIT_JOB: &str =
    "A person allowed `{command}` for this Job. Run it again now; it will not be \
     refused.";
pub(crate) const PERMIT_REPOSITORY: &str =
    "A person allowed `{rule}` in this repository. Run `{command}` again \
     now; it will not be refused.";
pub(crate) const PERMIT_MACHINE: &str =
    "A person allowed `{rule}` on this machine. Run `{command}` again now; it will not be \
     refused.";
pub(crate) const PERMIT_REJECTED: &str =
    "A person said no to `{command}`. Do not run it, or anything that does the same thing. \
     Carry on without it if the work allows, or ask a question if it cannot be done \
     without it.";
pub(crate) const PERMIT_REJECTED_NOTE: &str = "This is what the person said, in their own words:";
