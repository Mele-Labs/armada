//! What the librarian added to a project's system map and what he answered: the part, why it was added, and whether he
//! kept it, wants it reworked or removed it. **A removed addition stays as a row**, so it is not proposed again.
//! `docs/practices/protocol/projects.md`, *The system map*.

use rusqlite::params;

use crate::error::{fault, WriteError};
use crate::open::Store;

/// Where an addition stands, as its table names it.
pub mod addition_state {
    /// Added and not answered.
    pub const PROPOSED: &str = "proposed";
    pub const KEPT: &str = "kept";
    /// He asked for it to be redone from his note.
    pub const REWORK: &str = "rework";
    pub const REMOVED: &str = "removed";
}

/// One part the librarian added.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptAddition {
    pub project: String,
    pub id: String,
    pub name: String,
    /// `entry`, `service`, `store` or `outside`, as the wire spells a part's kind.
    pub kind: String,
    pub summary: String,
    pub why: String,
    pub paths: Vec<String>,
    /// The repository the paths are in.
    pub manifest: String,
    /// One of [`addition_state`].
    pub state: String,
    /// His note on a rework, empty otherwise.
    pub note: String,
    pub added_at: String,
}

const ADDITION: &str = "project, id, name, kind, summary, why, paths, manifest, state, note, added_at";

fn failed(doing: &'static str) -> impl Fn(rusqlite::Error) -> WriteError {
    move |why| WriteError::Database(fault(doing)(why))
}

impl Store {
    /// A project's additions, oldest first.
    pub fn system_additions(&self, project: &str) -> Result<Vec<KeptAddition>, WriteError> {
        let doing = "reading a project's system additions";
        let mut statement = self.conn.prepare(&format!("SELECT {ADDITION} FROM system_additions WHERE project = ?1 ORDER BY added_at, id")).map_err(failed(doing))?;
        let rows = statement
            .query_map([project], |row| {
                let paths: String = row.get(6)?;
                Ok(KeptAddition {
                    project: row.get(0)?,
                    id: row.get(1)?,
                    name: row.get(2)?,
                    kind: row.get(3)?,
                    summary: row.get(4)?,
                    why: row.get(5)?,
                    paths: paths.lines().map(str::to_string).collect(),
                    manifest: row.get(7)?,
                    state: row.get(8)?,
                    note: row.get(9)?,
                    added_at: row.get(10)?,
                })
            })
            .map_err(failed(doing))?;
        rows.collect::<Result<_, _>>().map_err(failed(doing))
    }

    /// Propose an addition. **One already kept under the id, in any state, is left as it was**, so a part he removed is not
    /// added again. Whether it was added.
    pub fn propose_system_addition(&mut self, add: &KeptAddition) -> Result<bool, WriteError> {
        self.conn
            .execute(
                &format!("INSERT OR IGNORE INTO system_additions ({ADDITION}) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)"),
                params![add.project, add.id, add.name, add.kind, add.summary, add.why, add.paths.join("\n"), add.manifest, add.state, add.note, add.added_at],
            )
            .map(|added| added > 0)
            .map_err(failed("proposing a system addition"))
    }

    /// Answer an addition: its new state and his note. Whether there was one to answer.
    pub fn answer_system_addition(&mut self, project: &str, id: &str, state: &str, note: &str) -> Result<bool, WriteError> {
        self.conn
            .execute("UPDATE system_additions SET state = ?3, note = ?4 WHERE project = ?1 AND id = ?2", params![project, id, state, note])
            .map(|changed| changed > 0)
            .map_err(failed("answering a system addition"))
    }

    /// An addition redone: its summary, why and paths as the librarian wrote them again, standing as proposed once more.
    pub fn redo_system_addition(&mut self, project: &str, id: &str, summary: &str, why: &str, paths: &[String]) -> Result<bool, WriteError> {
        self.conn
            .execute(
                "UPDATE system_additions SET summary = ?3, why = ?4, paths = ?5, state = ?6, note = '' WHERE project = ?1 AND id = ?2",
                params![project, id, summary, why, paths.join("\n"), addition_state::PROPOSED],
            )
            .map(|changed| changed > 0)
            .map_err(failed("redoing a system addition"))
    }

    /// When the librarian last mapped `project`.
    pub fn system_librarian_at(&self, project: &str) -> Result<Option<String>, WriteError> {
        match self.conn.query_row("SELECT ran_at FROM system_librarian WHERE project = ?1", [project], |row| row.get(0)) {
            Ok(at) => Ok(Some(at)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(why) => Err(failed("reading when the librarian mapped a project")(why)),
        }
    }

    pub fn keep_system_librarian_at(&mut self, project: &str, at: &str) -> Result<(), WriteError> {
        self.conn
            .execute("INSERT INTO system_librarian (project, ran_at) VALUES (?1, ?2) ON CONFLICT (project) DO UPDATE SET ran_at = ?2", [project, at])
            .map(drop)
            .map_err(failed("keeping when the librarian mapped a project"))
    }
}
