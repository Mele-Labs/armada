//! The words a piloted session's preface is written in, as Armada ships them:
//! each the default of a `prompts.pilot…` key in settings.json.

/// The opening. The rows under the authored lines are the record, laid by code.
pub(crate) const PILOT_OPENING: &str = "A person has taken over Job \"{title}\" (id {id}) and you are \
     working it with them. Its Drone was stopped, and its worktree is yours: you are already in it.";
pub(crate) const PILOT_WORKTREE: &str = "Worktree {path}, branch {branch}.";
pub(crate) const PILOT_RESTART: &str =
    "It was taken over to restart its step: a fresh Drone takes \
     the step after the person is done.";
pub(crate) const PILOT_FOR_GOOD: &str =
    "It was taken over for good: the person ends the pilot when they are done.";
pub(crate) const PILOT_STOPPED: &str =
    "It stopped on step `{step}`{label}{trigger}, after {runs} run(s).";
pub(crate) const PILOT_JUDGE_REFUSED: &str = "The Judge refused:";
pub(crate) const PILOT_CHECKS_FAILED: &str = "Checks that did not pass:";
pub(crate) const PILOT_OUTSIDE_PLAN: &str =
    "Files changed outside the plan the step declared: {files}.";
pub(crate) const PILOT_CHANGED: &str = "The worktree holds changes to: {files}.";
pub(crate) const PILOT_NARRATIVE: &str = "The Drone's own account, which is its word and not \
     evidence:\n- trying to: {trying_to}\n- blocked by: {blocked_by}\n- tried: {tried}";
pub(crate) const PILOT_RECORD: &str =
    "Every attempt of every step and the evidence each submitted \
     are in the handoff record, `get_handoff` for this Job id, if you need them.";
