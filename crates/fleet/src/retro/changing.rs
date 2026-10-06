//! The change to Kit a retro item may carry, held to the record.
//!
//! **The model names a refusal and Fleet copies the command.** What a person
//! accepts is a command the record shows a Drone was refused, as the Drone
//! typed it, and never one the model wrote from the item's prose. A change that
//! the cited row does not hold is dropped and the item stays.

use core_model::{Change, LandsIn};
use ipc::{RecordRefusal, RetroChangeAnswered};

use crate::permitting::always_allow_rules;

/// The tool whose refusals carry a command.
const SHELL: &str = "Bash";

/// The change an answer asks for, where the record holds what it cites.
///
/// - Only a `kit` item carries one: it is a change to Kit.
/// - The cited row must be a refused shell call whose argument the record kept
///   whole. A row cut for length (it ends in `…`) holds a command nobody read
///   in full, so it is no ground for a standing allow.
/// - Absent a `command`, the change is the whole argument. A `command` is one
///   of the argument's leading cuts, [`always_allow_rules`]' own candidates,
///   so the model can ask for `grep -a -c` and never for a command the Drone
///   did not type. A refused command that chains another has no whole to allow.
pub(crate) fn held_to_the_record(
    asked: Option<RetroChangeAnswered>,
    lands_in: LandsIn,
    refusals: &[RecordRefusal],
) -> Option<Change> {
    let RetroChangeAnswered::AllowCommand { refusal, command } = asked?;
    if lands_in != LandsIn::Kit {
        return None;
    }
    let row = refusals.iter().find(|row| row.cite == refusal)?;
    if row.tool != SHELL {
        return None;
    }
    let tried = row.tried.as_deref()?.trim();
    if tried.ends_with('…') {
        return None;
    }
    let (candidates, _) = always_allow_rules(tried);
    let chosen = match command.as_deref().map(str::trim) {
        Some(cut) => cut,
        None => tried,
    };
    candidates
        .iter()
        .any(|candidate| candidate == chosen)
        .then(|| Change::AllowCommand {
            command: chosen.to_string(),
        })
}
