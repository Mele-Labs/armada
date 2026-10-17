//! What this crate refuses, by name.
//!
//! Typed leaf enums with structured fields — the values that failed, not a
//! sentence about them. Nothing here is a `String` standing in for a reason,
//! and nothing here is `Box<dyn Error>` at the top level: the box appears only
//! as a *source*, where the underlying fault belongs to SQLite and cannot be
//! named outside this crate, because SQLite is scoped to this crate alone.
//!
//! The wrap into `ArmadaError` happens at a crate boundary and that wrapper is
//! not built. Each enum below is what a caller matches on until it is.
//!
//! # The two kinds of failure, and why they are separate types
//!
//! **A file-level fault refuses the open.** A damaged database, a schema from a
//! newer Armada, a file that is not an Armada store — none of them can be
//! narrowed to one row, and continuing means starting empty with work on disk
//! that nobody is being told about. [`OpenError`] exists so that path ends at a
//! caller who has to handle it.
//!
//! **A row-level fault refuses one row.** [`RowError`] names a column whose
//! stored shape cannot be read back. It does not take the process down.

use std::error::Error;
use std::fmt;

/// A fault that belongs to SQLite. The cause is carried rather than formatted,
/// so the chain stays traversable up to the wire, where it is flattened once.
#[derive(Debug)]
pub struct DatabaseFault {
    /// What was being done. A fixed string chosen here, never user input.
    pub doing: &'static str,
    pub cause: Box<dyn Error + Send + Sync>,
}

impl fmt::Display for DatabaseFault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.doing, self.cause)
    }
}

impl From<DatabaseFault> for WriteError {
    fn from(fault: DatabaseFault) -> WriteError {
        WriteError::Database(fault)
    }
}

pub(crate) fn fault(doing: &'static str) -> impl FnOnce(rusqlite::Error) -> DatabaseFault {
    move |cause| DatabaseFault {
        doing,
        cause: Box::new(cause),
    }
}

/// Why the store would not open. **Every variant is fatal on purpose.**
///
/// An empty database and a damaged one are different events here: an empty file
/// is migrated and opened, and each of these is refused. Starting empty over a
/// database that exists is how a person loses work and is told nothing.
#[derive(Debug)]
pub enum OpenError {
    Database(DatabaseFault),
    /// `PRAGMA integrity_check` did not answer `ok`. The pages are damaged;
    /// nothing read out of them can be trusted, including the parts that parse.
    IntegrityCheckFailed {
        path: String,
        finding: String,
    },
    /// `PRAGMA foreign_key_check` found rows pointing at nothing. A dangling
    /// entry means part of the record has already been lost.
    DanglingReferences {
        path: String,
        rows: usize,
    },
    /// The file has tables and no `armada_meta`. Either it is not an Armada
    /// store, or it predates versioning; both are refused rather than migrated
    /// into, because migrating into the wrong file writes into it.
    NotAnArmadaStore {
        path: String,
        tables: usize,
    },
    /// The file applied a breaking migration this build does not list. A newer
    /// Armada rewrote something this build reads, and the older crate's
    /// assumptions would misread it. An additive one is accepted instead.
    BreakingMigrationsFromTheFuture {
        path: String,
        names: Vec<String>,
    },
    /// A file written by the numbered list records more than the list ever held,
    /// so the conversion to names cannot say which entries it has.
    SchemaVersionFromTheFuture {
        path: String,
        found: u32,
        known: u32,
    },
    /// `armada_meta` exists and its version row is absent or is not a number.
    SchemaVersionUnreadable {
        path: String,
        found: Option<String>,
    },
    /// WAL was requested and SQLite answered something else — some filesystems
    /// refuse it. Recorded rather than accepted: the mode is part of the
    /// durability the rest of the design assumes.
    JournalModeRefused {
        path: String,
        actual: String,
    },
}

/// A row whose stored shape cannot be read back.
///
/// Each variant is a distinguishable kind of damage, because "the row is bad"
/// is not an answer anybody can act on.
#[derive(Debug)]
pub enum RowError {
    Database(DatabaseFault),
    /// A stored enum column holds a value this build does not have. Written by
    /// something that did not share the enum, or by a newer one.
    UnknownEnumValue {
        table: &'static str,
        column: &'static str,
        value: String,
    },
    /// A column is not the shape it was written as.
    MalformedColumn {
        table: &'static str,
        column: &'static str,
        detail: String,
    },
}

/// Why a write did not land.
#[derive(Debug)]
pub enum WriteError {
    Database(DatabaseFault),
    /// A preference save named something outside `preferences`' closed set.
    /// Named no Job: a preference is Fleet-wide, `limits`' own reason.
    UnknownPreference {
        name: String,
    },
}

/// Why a read did not come back.
#[derive(Debug)]
pub enum ReadError {
    Database(DatabaseFault),
    Unreadable(RowError),
}

macro_rules! display {
    ($ty:ty, |$self:ident, $f:ident| $body:expr) => {
        impl fmt::Display for $ty {
            fn fmt(&$self, $f: &mut fmt::Formatter<'_>) -> fmt::Result {
                $body
            }
        }
        impl Error for $ty {}
    };
}

display!(OpenError, |self, f| match self {
    OpenError::Database(fault) => write!(f, "{fault}"),
    OpenError::IntegrityCheckFailed { path, finding } =>
        write!(f, "{path} failed its integrity check: {finding}"),
    OpenError::DanglingReferences { path, rows } => write!(
        f,
        "{path} has {rows} rows referencing records that are gone"
    ),
    OpenError::NotAnArmadaStore { path, tables } => write!(
        f,
        "{path} has {tables} tables and no armada_meta — it is not an Armada store"
    ),
    OpenError::BreakingMigrationsFromTheFuture { path, names } => write!(
        f,
        "{path} has applied breaking migrations this build does not know: {}",
        names.join(", ")
    ),
    OpenError::SchemaVersionFromTheFuture { path, found, known } => write!(
        f,
        "{path} is at schema version {found} and this build knows {known}"
    ),
    OpenError::SchemaVersionUnreadable { path, found } => match found {
        Some(value) => write!(
            f,
            "{path} records schema version `{value}`, which is not a number"
        ),
        None => write!(f, "{path} has armada_meta and no schema version"),
    },
    OpenError::JournalModeRefused { path, actual } =>
        write!(f, "{path} refused WAL and is in `{actual}` mode"),
});

display!(RowError, |self, f| match self {
    RowError::Database(fault) => write!(f, "{fault}"),
    RowError::UnknownEnumValue {
        table,
        column,
        value,
    } => write!(
        f,
        "{table}.{column} holds `{value}`, which is not one this build knows"
    ),
    RowError::MalformedColumn {
        table,
        column,
        detail,
    } => write!(
        f,
        "{table}.{column} is not the shape it was written as: {detail}"
    ),
});

display!(WriteError, |self, f| match self {
    WriteError::Database(fault) => write!(f, "{fault}"),
    WriteError::UnknownPreference { name } =>
        write!(f, "`{name}` is not a preference this build reads"),
});

display!(ReadError, |self, f| match self {
    ReadError::Database(fault) => write!(f, "{fault}"),
    ReadError::Unreadable(cause) => write!(f, "{cause}"),
});
