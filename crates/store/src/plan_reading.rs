//! The last plan reading, kept as the text Fleet encoded so a restarted Fleet can serve it before any
//! Session reports again. `docs/concepts/machine.md`, *Quota*.

use crate::error::{fault, WriteError};
use crate::open::Store;

impl Store {
    /// The encoded reading last kept, or `None` where none has been.
    pub fn plan_reading(&self) -> Result<Option<String>, WriteError> {
        self.conn
            .query_row("SELECT reading FROM plan_reading", [], |row| row.get(0))
            .map_err(fault("reading the plan's last reading"))
            .map_err(WriteError::Database)
    }

    pub fn keep_plan_reading(&mut self, reading: &str) -> Result<(), WriteError> {
        self.conn
            .execute("UPDATE plan_reading SET reading = ?1", [reading])
            .map(|_| ())
            .map_err(fault("keeping the plan's last reading"))
            .map_err(WriteError::Database)
    }
}
