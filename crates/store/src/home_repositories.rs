//! Where each profile's shared home is or will be made, kept on this machine alone.
//!
//! **A login with no row has the default**, `<login>/armada-home`, which is not written here.
//! Whether the owner may take a repository is Fleet's to decide: this keeps what it was told.

use rusqlite::OptionalExtension;

use crate::error::{fault, DatabaseFault, WriteError};
use crate::open::Store;

/// Where a profile's home is made, as the store keeps it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptHomeRepository {
    pub owner: String,
    pub name: String,
    /// The forge already had it, made on another Mac, so it is not a choice to change.
    pub found: bool,
}

impl Store {
    /// The owner and name chosen, or found on the forge, for `login`'s home.
    pub fn home_repository(&self, login: &str) -> Result<Option<KeptHomeRepository>, DatabaseFault> {
        self.conn
            .query_row("SELECT owner, name, found FROM home_repositories WHERE login = ?1", [login], |row| {
                Ok(KeptHomeRepository { owner: row.get(0)?, name: row.get(1)?, found: row.get::<_, i64>(2)? != 0 })
            })
            .optional()
            .map_err(fault("reading where the home is made"))
    }

    /// Record where `login`'s home is made, replacing an earlier choice.
    pub fn set_home_repository(&mut self, login: &str, owner: &str, name: &str, found: bool) -> Result<(), WriteError> {
        self.conn
            .execute(
                "INSERT INTO home_repositories (login, owner, name, found) VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT (login) DO UPDATE SET owner = excluded.owner, name = excluded.name, found = excluded.found",
                (login, owner, name, i64::from(found)),
            )
            .map(|_| ())
            .map_err(fault("keeping where the home is made"))
            .map_err(WriteError::Database)
    }
}
