//! What a working Drone is told when a person changes the plan, as it ships:
//! each the default of a `prompts.plan…` key in settings.json.

/// The turn when a person adds a task, as it ships. `{id}` and `{title}` name it.
pub(crate) const ADDED: &str =
    "THE PLAN CHANGED\n\nA person added a task to this Job's plan: {id} {title}. It \
             is open, for you or a later part to pick up. This is not a question. Carry \
             on with the part you were given.";

/// The turn when a person drops a task, as it ships.
pub(crate) const DROPPED: &str =
    "THE PLAN CHANGED\n\nA person dropped a task from this Job's plan: {id} \
             {title}. Reason: {reason}. This is settled, not a question to raise — the \
             task stays dropped unless a person adds it back. Carry on with the part \
             you were given.";

/// The turn when a group's checks fail, as it ships.
pub(crate) const GROUP_ROUND: &str =
    "THE GROUP GOES ROUND\n\nThe Checks above ran at the end of group {group}, \
             which is these tasks: {tasks}. Fix what they found across all of them, not only \
             your own, then call submit_evidence once for the group. Every one of them \
             stays handed in while you do. Do not start a task of a later group.";
