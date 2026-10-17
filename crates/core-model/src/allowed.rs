//! A command a person always-allowed for a repository.

use alloc::string::String;

use crate::envelope::{Actor, Timestamp};

/// A command a person always-allowed for a repository.
///
/// `run` is the command text as the person allowed it. Nothing in this crate
/// interprets it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AllowedCommand {
    pub run: String,
    pub allowed_at: Timestamp,
    pub by: Actor,
}
