//! The shape of one retro item as the store keeps it. `docs/concepts/retro.md`.
//!
//! A Session's retros are kept by [`session_retro`](crate::session_retro); the
//! tables a Job's retro was kept in are not read.

use core_model::{Change, LandsIn, Whose};

use crate::error::RowError;
use crate::row::enum_value;

/// One item of a retro: what got in the way, whose way, and the record rows
/// that show it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RetroLine {
    pub whose: Whose,
    /// A headline. **`None` only on an item kept before V105**, as `what` and
    /// `fix` are.
    pub title: Option<String>,
    pub what: Option<String>,
    pub fix: Option<String>,
    pub said: String,
    /// References into the Job's assembled record, `refusal:1` and the like.
    /// **Never empty on a written item**: Fleet drops an item that cites
    /// nothing before it is kept.
    pub evidence: Vec<String>,
    /// Where its fix lands. **`None` only on an item kept before V102**: Fleet
    /// drops a written item that names none.
    pub lands_in: Option<LandsIn>,
    /// The change to Kit that pressing Accept applies. **Only on a Kit item**,
    /// and only where Fleet copied it off a refusal the record shows. Since
    /// V110.
    pub change: Option<Change>,
}

/// The texts of an item beside its `said`, which are all absent on an older item.
pub(crate) struct Texts {
    pub(crate) title: Option<String>,
    pub(crate) what: Option<String>,
    pub(crate) fix: Option<String>,
    pub(crate) change_kind: Option<String>,
    pub(crate) change_command: Option<String>,
}

pub(crate) fn line_of(
    whose: &str,
    said: String,
    evidence: &str,
    lands_in: Option<&str>,
    texts: Texts,
) -> Result<RetroLine, RowError> {
    let change = match (texts.change_kind, texts.change_command) {
        (Some(kind), Some(command)) => Some(Change::from_parts(&kind, command).ok_or(
            RowError::UnknownEnumValue {
                table: "job_retro_items",
                column: "change_kind",
                value: kind,
            },
        )?),
        _ => None,
    };
    Ok(RetroLine {
        change,
        title: texts.title,
        what: texts.what,
        fix: texts.fix,
        whose: enum_value(Whose::from_wire, "job_retro_items", "whose", whose)?,
        lands_in: lands_in
            .map(|lands| enum_value(LandsIn::from_wire, "job_retro_items", "lands_in", lands))
            .transpose()?,
        said,
        evidence: evidence
            .lines()
            .filter(|cited| !cited.is_empty())
            .map(str::to_string)
            .collect(),
    })
}
