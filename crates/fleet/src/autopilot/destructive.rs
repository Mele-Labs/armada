//! What Autopilot never answers: a question that may delete or overwrite something, read from its
//! words and its options. **Decided before any call, and again on the answer the call chose.**
//! There is no wire value for it and no box.

use std::time::Duration;

use ipc::WaitingItem;

/// How long an item waits before Autopilot answers it, so an agent can carry on by itself first.
pub const GRACE: Duration = Duration::from_secs(120);

const RECOMMENDED: &str = "(recommended)";

/// Words that mark a question as one Autopilot must not answer. **Conservative on purpose**: a word
/// found inside another ("enforce") holds the question, and a held question costs the person a glance.
const DESTRUCTIVE: &[&str] = &[
    "delete", "drop", "remove", "force", "reset", "overwrite", "irreversible", "destructive", "discard",
    "destroy", "wipe", "purge", "erase", "truncate", "prune", "clobber", "unrecoverable", "permanent",
    "uninstall", "revert", "--hard", "close pr", "close the pr", "close pull", "close the pull", "close the issue",
];

/// Whole words that are too short to match inside another.
const DESTRUCTIVE_WORDS: &[&str] = &["rm", "kill", "undo"];

/// Whether the item, read with its options, may be destructive or irreversible.
pub(crate) fn destructive(item: &WaitingItem) -> bool {
    let mut text = item.text.clone();
    for option in &item.options {
        text.push('\n');
        text.push_str(&option.label);
    }
    destructive_text(&text)
}

/// [`destructive`] over plain text.
pub(crate) fn destructive_text(text: &str) -> bool {
    let text = text.to_lowercase();
    DESTRUCTIVE.iter().any(|word| text.contains(word))
        || text
            .split(|one: char| !one.is_alphanumeric())
            .any(|word| DESTRUCTIVE_WORDS.contains(&word))
}

/// The index of the option labelled "(Recommended)", if one is.
pub(crate) fn recommended(item: &WaitingItem) -> Option<usize> {
    item.options
        .iter()
        .position(|one| one.label.to_lowercase().contains(RECOMMENDED))
}
