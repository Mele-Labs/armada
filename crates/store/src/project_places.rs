//! Where each project's file is kept, and which committed Workspace files became projects.
//! **The place is a choice per project**; none chosen is this Mac, and `repository` is never one.
//! The file is the truth for a project's name and members, and the `projects` row only keeps his
//! order and the last name Fleet saw.

use rusqlite::{params, OptionalExtension};

use crate::error::{fault, WriteError};
use crate::open::Store;
use crate::run_memory::KeptPlace;

fn database(doing: &'static str) -> impl FnOnce(rusqlite::Error) -> WriteError {
    move |cause| WriteError::Database(fault(doing)(cause))
}

impl Store {
    /// The place chosen for a project. `None` is no choice.
    pub fn project_place(&self, project: &str) -> Result<Option<KeptPlace>, WriteError> {
        self.conn
            .query_row(
                "SELECT place, synced_folder, cloud FROM project_places WHERE project_id = ?1",
                [project],
                |row| Ok(KeptPlace { place: row.get(0)?, synced_folder: row.get(1)?, cloud: row.get(2)? }),
            )
            .optional()
            .map_err(database("reading where a project is kept"))
    }

    /// Choose where a project's file is kept, replacing an earlier choice.
    pub fn set_project_place(
        &mut self,
        project: &str,
        place: &str,
        synced_folder: Option<&str>,
        cloud: Option<&str>,
    ) -> Result<(), WriteError> {
        self.conn
            .execute(
                "INSERT INTO project_places (project_id, place, synced_folder, cloud) VALUES (?1, ?2, ?3, ?4) \
                 ON CONFLICT (project_id) DO UPDATE SET place = ?2, synced_folder = ?3, cloud = ?4",
                params![project, place, synced_folder, cloud],
            )
            .map(|_| ())
            .map_err(database("choosing where a project is kept"))
    }

    /// The project a committed Workspace file at `source` was imported as, if it was.
    pub fn project_imported(&self, source: &str) -> Result<Option<String>, WriteError> {
        self.conn
            .query_row("SELECT project_id FROM project_imports WHERE source = ?1", [source], |row| row.get(0))
            .optional()
            .map_err(database("reading what was imported as a project"))
    }

    /// Remember that the file at `source` became the project `project`, so it is not imported again.
    pub fn record_project_import(&mut self, source: &str, project: &str) -> Result<(), WriteError> {
        self.conn
            .execute(
                "INSERT INTO project_imports (source, project_id) VALUES (?1, ?2) \
                 ON CONFLICT (source) DO UPDATE SET project_id = ?2",
                params![source, project],
            )
            .map(|_| ())
            .map_err(database("remembering what was imported as a project"))
    }
}
