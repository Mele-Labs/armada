//! The default model's step down: where it stands, and the window it was last offered for.
//! `docs/concepts/machine.md`, *Quota*.
//!
//! **One row, and it survives a restart on purpose.** A Fleet that restarted could not tell a lowered
//! default from one the owner chose; this is what it asks.

use crate::error::{fault, WriteError};
use crate::open::Store;

/// The row. `phase` is `offered`, `shifted` or `reset`, and `None` where there is no state; the
/// columns a phase does not use are left at their zero.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct KeptDownshift {
    pub phase: Option<String>,
    pub percent: u32,
    /// The default when the offer was made.
    pub was: String,
    /// The lower model.
    pub to: String,
    /// The reset of the window it is shifted for.
    pub until: Option<String>,
    /// What settings.json held for `models.default` before the step down; `None` is the key not saved.
    pub restore_to: Option<String>,
    /// The reset of the window an offer was last made for, kept after the state clears so none is made twice.
    pub offered_for: Option<String>,
}

impl Store {
    pub fn model_downshift(&self) -> Result<KeptDownshift, WriteError> {
        self.conn
            .query_row(
                "SELECT phase, percent, was, to_model, until, restore_to, offered_for FROM model_downshift",
                [],
                |row| {
                    Ok(KeptDownshift {
                        phase: row.get(0)?,
                        percent: row.get(1)?,
                        was: row.get::<_, Option<String>>(2)?.unwrap_or_default(),
                        to: row.get::<_, Option<String>>(3)?.unwrap_or_default(),
                        until: row.get(4)?,
                        restore_to: row.get(5)?,
                        offered_for: row.get(6)?,
                    })
                },
            )
            .map_err(fault("reading the default model's step down"))
            .map_err(WriteError::Database)
    }

    pub fn keep_model_downshift(&mut self, kept: &KeptDownshift) -> Result<(), WriteError> {
        self.conn
            .execute(
                "UPDATE model_downshift SET phase = ?1, percent = ?2, was = ?3, to_model = ?4, until = ?5, \
                 restore_to = ?6, offered_for = ?7",
                (&kept.phase, kept.percent, &kept.was, &kept.to, &kept.until, &kept.restore_to, &kept.offered_for),
            )
            .map(|_| ())
            .map_err(fault("keeping the default model's step down"))
            .map_err(WriteError::Database)
    }
}
