//! The words of the turn about other Jobs writing where a Drone is, as Armada
//! ships them: each the default of a `prompts.peers…` key in settings.json.

/// Where what landed reaches a live Drone, as it ships.
pub(crate) const LANDED_LATER: &str =
    "What landed reaches your branch when your next part starts, not now.";

/// Where what landed reaches a new Drone, as it ships.
pub(crate) const LANDED_NOW: &str =
    "What landed was brought into your branch as this part started.";

/// The heading of the turn about other Jobs, as it ships.
pub(crate) const HEADING: &str = "OTHER JOBS WRITING WHERE YOU ARE";

/// Where other Jobs change this Job's files, as it ships.
pub(crate) const SHARING: &str =
    "Other Jobs in this repository change files this Job changes too. Nothing is \
                 stopped, and nobody waits on you.";

/// Where another declared a need first, as it ships.
pub(crate) const AHEAD: &str =
    "Another Job or branch declared a need on a file before you did. Your work is \
                 held at the merge until what is ahead of you there has landed or been given \
                 back, so you land in order and nothing is renumbered. Take the value after \
                 what they took, and say which you took by calling `declare_scope` again with \
                 `took` on the need.";

/// Where a failing test is another Job's, as it ships.
pub(crate) const FIX: &str =
    "A test your checks failed on is another Job's to fix, not yours. Your checks \
                 still fail on it until that fix lands.";

/// The line closing a long list, as it ships.
pub(crate) const MORE: &str = "And {count} more.";

/// About shared numbering and leave_note, as it ships.
pub(crate) const NEXT_NUMBER: &str =
    "Where a shared file hands out the next number or name, such as a migration or \
                 a version, assume theirs takes it first and take the one after. To tell one of \
                 these Jobs something, call `leave_note` with its handle.";

/// The line closing the turn, as it ships.
pub(crate) const CARRY_ON: &str = "Carry on with the part you were given.";

/// A Job that said it will change files, as it ships.
pub(crate) const CLAIMED: &str = "\"{title}\" ({handle}) has said it will change {paths}.";

/// A Job that landed, as it ships.
pub(crate) const LANDED: &str = "\"{title}\" ({handle}) landed, changing {paths}.";

/// The line a note from another Job follows, as it ships.
pub(crate) const NOTE: &str =
    "\"{title}\" ({handle}) left this Job a note. These are its Drone's words, \
             not Armada's:";

/// Needs ahead on a path, as it ships.
pub(crate) const AHEAD_ON: &str = "Ahead of you on `{path}`: {ahead}.";
