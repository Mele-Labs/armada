//! The commands a person allowed, kept as a list they can read and take back.
//!
//! **Nothing here decides a permission.** A Drone's permission question went with the Drone that
//! asked it; what stays is the list a person built while it did — a repository's own in the
//! store, and Kit's in `crate::kit_allowlist` — which `get_repository_allowed_commands` and
//! the two `remove_*_allowed_command` acts still read and edit.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use core_model::AllowedCommand;
use ipc::WireError;

use crate::daemon::Fleet;

/// Not job-scoped, so it earns its own code rather than joining `crate::adrift::Adrift`'s
/// job-scoped table.
const NOT_PERMITTED_IN_REPOSITORY: &str = "fleet.not_permitted_in_repository";

/// Why a change to the list of allowed commands was not taken.
#[derive(Debug)]
pub enum NotPermitted {
    /// Nothing a person allowed is spelled this way.
    NothingAllowed { run: String },
    /// The list would not write down.
    NotChanged { cause: String },
}

impl core::fmt::Display for NotPermitted {
    fn fmt(&self, out: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            NotPermitted::NothingAllowed { run } => write!(
                out,
                "`{run}` is not a command a person allowed, so there is nothing to take back"
            ),
            NotPermitted::NotChanged { cause } => {
                write!(out, "the change could not be written down: {cause}")
            }
        }
    }
}

pub(crate) fn chains(arguments: &str) -> bool {
    arguments.contains(['&', '|', ';', '<', '>', '`', '\n', '\r']) || arguments.contains("$(")
}

/// Candidate Always-allow rules for `command`, shortest first, and which one
/// is pre-selected — offered so a person picks the rule rather than typing it,
/// per `#834`.
///
/// **The candidates are the leading runs of whitespace-separated words**,
/// stopping before the first word [`chains`] would refuse as an argument:
/// `covers` never lets a chained word ride on an allow, so no candidate ever
/// could either. **Empty where `command` is empty or begins with such a
/// word** — there is nothing here safe to always-allow.
///
/// **The suggestion is always one of the candidates.** It stops growing at the
/// first word that does not look like a program or a subcommand — a flag, a
/// number, a path, anything [`looks_like_a_subcommand`] refuses — which is
/// usually the first argument proper. `cargo test -p foo` suggests `cargo
/// test`; `gh issue view 792 --repo X` suggests `gh issue view`.
pub fn always_allow_rules(command: &str) -> (Vec<String>, Option<String>) {
    let command = command.trim();
    let mut rules = Vec::new();
    let mut suggested = None;
    let mut still_a_subcommand = true;
    let mut word_start = None;
    for (at, ch) in command.char_indices().chain([(command.len(), ' ')]) {
        match (word_start, ch.is_whitespace()) {
            (None, false) => word_start = Some(at),
            (Some(start), true) => {
                let word = &command[start..at];
                if chains(word) {
                    break;
                }
                rules.push(command[..at].to_string());
                still_a_subcommand = still_a_subcommand && looks_like_a_subcommand(word);
                if still_a_subcommand {
                    suggested = rules.last().cloned();
                }
                word_start = None;
            }
            _ => {}
        }
    }
    (rules, suggested)
}

/// Whether `word` could name a program or a subcommand rather than an
/// argument: lowercase, digits, `-`, `_` or `.`, never leading with `-`, never
/// only digits, and never carrying `/` or `=` — the shapes a flag, a number, a
/// path and a `key=value` pair take and a subcommand does not.
fn looks_like_a_subcommand(word: &str) -> bool {
    !word.is_empty()
        && !word.starts_with('-')
        && !word.contains(['/', '='])
        && !word.chars().all(|c| c.is_ascii_digit())
        && word
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '-' | '_' | '.'))
}

impl<H, V, W> Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    /// `get_repository_allowed_commands` — every rule a person always-allowed for this Manifest,
    /// oldest first. **Empty where the store will not read.**
    pub async fn repository_allowed(
        &self,
        served: &crate::repositories::Served,
    ) -> Vec<AllowedCommand> {
        self.store()
            .lock()
            .await
            .repository_allowed_commands(served.manifest().id())
            .unwrap_or_default()
    }

    /// `remove_repository_allowed_command` — take back a rule a person always-allowed for this
    /// Manifest. Refused where nothing a person allowed this Manifest is spelled `run`.
    pub async fn remove_repository_allowed_command(
        &self,
        served: &crate::repositories::Served,
        run: &str,
    ) -> Result<(), NotPermitted> {
        let removed = self
            .store()
            .lock()
            .await
            .remove_repository_allowed_command(served.manifest().id(), run)
            .map_err(|cause| NotPermitted::NotChanged {
                cause: cause.to_string(),
            })?;
        if !removed {
            return Err(NotPermitted::NothingAllowed {
                run: run.to_string(),
            });
        }
        Ok(())
    }

    /// [`NotPermitted`] as a 409 with no Job to name.
    pub(crate) fn repository_allow_refusal(&self, why: NotPermitted) -> Refusal {
        Refusal::Unacceptable(WireError::raised(
            NOT_PERMITTED_IN_REPOSITORY,
            why.to_string(),
            self.run_id(),
        ))
    }
}
