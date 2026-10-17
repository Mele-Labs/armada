//! The commands a person always-allowed for a repository: the rows Fleet keeps
//! and the request that takes one back.

use serde::{Deserialize, Serialize};

use crate::enums::Actor;
use crate::ids::Instant;

/// One command a person always-allowed for a repository.
///
/// **Every field of the record crosses**, which is a decision rather than a
/// default: `run` is text a person already read in full before allowing it, and
/// nothing else on the record is private to Fleet.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AllowedCommandRow {
    /// The command, as the person allowed it. **What
    /// `remove_repository_allowed_command` names**, so it crosses whole and
    /// never cut to one line.
    pub run: String,
    /// When it was allowed, by Fleet's clock.
    pub allowed_at: Instant,
    pub by: Actor,
}

impl From<&core_model::AllowedCommand> for AllowedCommandRow {
    fn from(allowed: &core_model::AllowedCommand) -> AllowedCommandRow {
        AllowedCommandRow {
            run: allowed.run.clone(),
            allowed_at: (&allowed.allowed_at).into(),
            by: allowed.by.into(),
        }
    }
}

/// `get_repository_allowed_commands`'s answer: every rule a person
/// always-allowed for this Manifest's repository, oldest first. **Since
/// `#836`.**
///
/// **A named wrapper rather than a bare list**, matching every other
/// `list_*` answer on this seam.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepositoryAllowedCommands {
    pub commands: Vec<AllowedCommandRow>,
}

/// The request half of `remove_repository_allowed_command`. **Since `#836`.**
///
/// It names a rule kept for the Manifest. A rule this Manifest holds no allow
/// for is a 409.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoveRepositoryAllowedCommand {
    /// [`AllowedCommandRow::run`], exactly.
    pub run: String,
}
