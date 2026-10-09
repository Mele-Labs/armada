//! A person's preferences, kept the way limits are: one row per preference,
//! and an absent row is the shipped default.
//!
//! **Keyed by name, not by column.** `crate::limits`'s table is one column per
//! limit, which is right there because the three are fixed and reading all of
//! them is the only read this crate ever does. A preference is added one at a
//! time — the first is `where_things_are_open` and it will not be the last —
//! and a column-per-preference table would make every new one a migration.
//! `CHECK (name IN (...))` is the closed set the wire's own refusal answers
//! to: [`Store::save_preference`] refuses an unrecognised name before it ever
//! reaches SQL, and the constraint is what keeps a second writer — a hand-edited
//! file, a Store built without this check — from putting one there anyway.

use crate::error::{fault, DatabaseFault, WriteError};
use crate::open::Store;

/// Version 62 — a person's preferences, one row per name. `CHECK (value IN (0,
/// 1))` is what makes the one preference this build has a boolean; a
/// preference that is not one will need its own column or its own table, not a
/// looser check here.
pub(crate) const V62: &str = r#"
CREATE TABLE preferences (
    name  TEXT PRIMARY KEY CHECK (name IN ('where_things_are_open')),
    value INTEGER NOT NULL CHECK (value IN (0, 1))
) STRICT;
"#;

/// The one preference name this build reads and writes. Kept in the store, not
/// in `ipc`, because it is the store's `CHECK` that is authoritative and a
/// second copy of the set at the wire would be the thing that drifts from it.
const WHERE_THINGS_ARE_OPEN: &str = "where_things_are_open";

/// Whether this machine offers a pull request as a draft unless the repository,
/// the workflow or the Job says otherwise. **Kept in a table of its own**, one
/// row or none, because widening `preferences`' `CHECK` is a rebuild and a
/// rebuild is a breaking migration.
const DRAFT_PULL_REQUESTS: &str = "draft_pull_requests";

/// Every preference Fleet knows, and what a person saved for each. **A row
/// nobody wrote reads as the shipped default**, not as unset — there is
/// nothing else it could mean, since a preference with no row has never been
/// touched.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Preferences {
    pub where_things_are_open: bool,
    pub draft_pull_requests: bool,
    /// The theme's id, in a table of its own for `DRAFT_PULL_REQUESTS`' reason.
    pub theme: String,
    /// The text of the owner's `layout.json`, or empty where he has made no choice.
    pub layout_choices: String,
    /// The text of the owner's key bindings, or empty where he has changed none.
    pub key_bindings: String,
}

/// The theme nobody has chosen away from.
pub const SHIPPED_THEME: &str = "dark";

impl Default for Preferences {
    fn default() -> Preferences {
        Preferences {
            where_things_are_open: false,
            draft_pull_requests: false,
            theme: SHIPPED_THEME.to_string(),
            layout_choices: String::new(),
            key_bindings: String::new(),
        }
    }
}

impl Store {
    /// Every preference, folded over the rows a person actually saved.
    ///
    /// **A name this build does not read is skipped, not refused.** It can
    /// only arrive from a newer Armada's row in an older one's database, the
    /// same reading an unknown field on the wire gets — ignored rather than a
    /// reason to fail the whole read.
    pub fn preferences(&self) -> Result<Preferences, DatabaseFault> {
        let mut statement = self
            .conn
            .prepare("SELECT name, value FROM preferences")
            .map_err(fault("reading the saved preferences"))?;
        let rows = statement
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
            })
            .map_err(fault("reading the saved preferences"))?;
        let mut preferences = Preferences::default();
        for row in rows {
            let (name, value) = row.map_err(fault("reading the saved preferences"))?;
            if name == WHERE_THINGS_ARE_OPEN {
                preferences.where_things_are_open = value != 0;
            }
        }
        preferences.draft_pull_requests = match self.conn.query_row(
            "SELECT value FROM draft_pull_requests WHERE id = 1",
            [],
            |row| row.get::<_, i64>(0),
        ) {
            Ok(value) => value != 0,
            Err(rusqlite::Error::QueryReturnedNoRows) => false,
            Err(other) => return Err(fault("reading the saved preferences")(other)),
        };
        match self.conn.query_row(
            "SELECT value FROM theme_preference WHERE id = 1",
            [],
            |row| row.get::<_, String>(0),
        ) {
            Ok(theme) => preferences.theme = theme,
            Err(rusqlite::Error::QueryReturnedNoRows) => {}
            Err(other) => return Err(fault("reading the saved preferences")(other)),
        }
        match self.conn.query_row(
            "SELECT value FROM layout_preference WHERE id = 1",
            [],
            |row| row.get::<_, String>(0),
        ) {
            Ok(choices) => preferences.layout_choices = choices,
            Err(rusqlite::Error::QueryReturnedNoRows) => {}
            Err(other) => return Err(fault("reading the saved preferences")(other)),
        }
        match self.conn.query_row(
            "SELECT value FROM key_bindings_preference WHERE id = 1",
            [],
            |row| row.get::<_, String>(0),
        ) {
            Ok(bindings) => preferences.key_bindings = bindings,
            Err(rusqlite::Error::QueryReturnedNoRows) => {}
            Err(other) => return Err(fault("reading the saved preferences")(other)),
        }
        Ok(preferences)
    }

    /// Save the theme's id, and answer with every preference now in force.
    /// **Not checked here against what a theme can be called**: the store keeps
    /// the word, and Fleet decides what words it accepts.
    pub fn save_theme(&mut self, theme: &str) -> Result<Preferences, WriteError> {
        self.conn
            .execute(
                "INSERT INTO theme_preference (id, value) VALUES (1, ?1)
                 ON CONFLICT (id) DO UPDATE SET value = excluded.value",
                (theme,),
            )
            .map_err(fault("saving the theme"))
            .map_err(WriteError::Database)?;
        self.preferences().map_err(WriteError::Database)
    }

    /// Save the owner's layout choices, and answer with every preference now in force. **Empty
    /// takes them back**: no row is no choices. Not checked here, for `save_theme`'s reason.
    pub fn save_layout_choices(&mut self, text: &str) -> Result<Preferences, WriteError> {
        let saved = match text.is_empty() {
            true => self.conn.execute("DELETE FROM layout_preference WHERE id = 1", []),
            false => self.conn.execute(
                "INSERT INTO layout_preference (id, value) VALUES (1, ?1)
                 ON CONFLICT (id) DO UPDATE SET value = excluded.value",
                (text,),
            ),
        };
        saved
            .map_err(fault("saving the layout choices"))
            .map_err(WriteError::Database)?;
        self.preferences().map_err(WriteError::Database)
    }

    /// Save the owner's key bindings, and answer with every preference now in force.
    /// **Empty takes them back**, `save_layout_choices`' terms.
    pub fn save_key_bindings(&mut self, text: &str) -> Result<Preferences, WriteError> {
        let saved = match text.is_empty() {
            true => self.conn.execute("DELETE FROM key_bindings_preference WHERE id = 1", []),
            false => self.conn.execute(
                "INSERT INTO key_bindings_preference (id, value) VALUES (1, ?1)
                 ON CONFLICT (id) DO UPDATE SET value = excluded.value",
                (text,),
            ),
        };
        saved
            .map_err(fault("saving the key bindings"))
            .map_err(WriteError::Database)?;
        self.preferences().map_err(WriteError::Database)
    }

    /// This machine's switch for each mod somebody has moved. A mod with no row
    /// is on.
    pub fn mod_switches(&self) -> Result<std::collections::BTreeMap<String, bool>, DatabaseFault> {
        let mut statement = self
            .conn
            .prepare("SELECT name, enabled FROM mod_switches")
            .map_err(fault("reading the mod switches"))?;
        let rows = statement
            .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)? != 0)))
            .map_err(fault("reading the mod switches"))?;
        rows.collect::<Result<_, _>>()
            .map_err(fault("reading the mod switches"))
    }

    /// Switch one mod. **The name is not looked up**: a switch for a mod that is
    /// not there yet holds until it is, and one left behind by a mod that was
    /// deleted does nothing.
    pub fn switch_mod(&mut self, name: &str, enabled: bool) -> Result<(), WriteError> {
        self.conn
            .execute(
                "INSERT INTO mod_switches (name, enabled) VALUES (?1, ?2)
                 ON CONFLICT (name) DO UPDATE SET enabled = excluded.enabled",
                (name, i64::from(enabled)),
            )
            .map(|_| ())
            .map_err(fault("switching a mod"))
            .map_err(WriteError::Database)
    }

    /// Save one preference by name, and answer with every preference now in
    /// force. **A name outside the closed set is [`WriteError::UnknownPreference`]**
    /// — checked here, before a statement is built, so the message names
    /// exactly what was sent rather than reporting whatever SQLite's own
    /// `CHECK` violation says.
    pub fn save_preference(&mut self, name: &str, value: bool) -> Result<Preferences, WriteError> {
        if name != WHERE_THINGS_ARE_OPEN && name != DRAFT_PULL_REQUESTS {
            return Err(WriteError::UnknownPreference {
                name: name.to_string(),
            });
        }
        let saved = match name == DRAFT_PULL_REQUESTS {
            true => self.conn.execute(
                "INSERT INTO draft_pull_requests (id, value) VALUES (1, ?1)
                 ON CONFLICT (id) DO UPDATE SET value = excluded.value",
                (i64::from(value),),
            ),
            false => self.conn.execute(
                "INSERT INTO preferences (name, value) VALUES (?1, ?2)
                 ON CONFLICT (name) DO UPDATE SET value = excluded.value",
                (name, i64::from(value)),
            ),
        };
        saved
            .map_err(fault("saving a preference"))
            .map_err(WriteError::Database)?;
        self.preferences().map_err(WriteError::Database)
    }
}
