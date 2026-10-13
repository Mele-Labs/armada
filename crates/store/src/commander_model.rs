//! The model and effort a person chose for one Job's Commander. `#2166`.
//!
//! Beside [`crate::review_model`] and on its terms: columns on the Job's row,
//! read at the one moment they matter, the start of a turn. **Any text is
//! kept**, for that module's reason; Fleet checks the model against the roster
//! before it asks for the write.

use core_model::JobId;

use crate::error::{fault, LoadJobError, WriteError};
use crate::open::Store;

/// What a person chose. `None` in either is nobody having chosen it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CommanderChoice {
    pub model: Option<String>,
    pub effort: Option<String>,
}

impl Store {
    /// The model and effort chosen for this Job's Commander.
    pub fn commander_choice(&self, job_id: &JobId) -> Result<CommanderChoice, LoadJobError> {
        self.conn
            .query_row(
                "SELECT commander_model_override, commander_effort_override FROM jobs \
                 WHERE job_id = ?1",
                (job_id.as_str(),),
                |row| {
                    Ok(CommanderChoice {
                        model: row.get(0)?,
                        effort: row.get(1)?,
                    })
                },
            )
            .map_err(|why| match why {
                rusqlite::Error::QueryReturnedNoRows => LoadJobError::NoSuchJob {
                    job_id: job_id.clone(),
                },
                other => LoadJobError::Database(fault(
                    "reading a job's chosen Commander model",
                )(other)),
            })
    }

    /// Set what [`commander_choice`](Store::commander_choice) answers, both
    /// halves at once. `None` clears one.
    pub fn set_commander_choice(
        &mut self,
        job_id: &JobId,
        choice: &CommanderChoice,
    ) -> Result<(), WriteError> {
        let updated = self
            .conn
            .execute(
                "UPDATE jobs SET commander_model_override = ?2, commander_effort_override = ?3 \
                 WHERE job_id = ?1",
                (
                    job_id.as_str(),
                    choice.model.as_deref(),
                    choice.effort.as_deref(),
                ),
            )
            .map_err(fault("recording a job's chosen Commander model"))
            .map_err(WriteError::Database)?;
        if updated == 0 {
            return Err(WriteError::NoSuchJob {
                job_id: job_id.clone(),
            });
        }
        Ok(())
    }
}
