//! Reading one column, and refusing one that is not what it was written as.
//!
//! The `table` in an error is a fixed string chosen at the call site, never a
//! value from a row. Nothing here formats a reason into a sentence — the column
//! that failed is a field, because nothing greps a sentence.

use crate::error::{fault, RowError};

pub(crate) fn column(
    table: &'static str,
    name: &'static str,
) -> impl Fn(rusqlite::Error) -> RowError {
    move |error| match error {
        rusqlite::Error::InvalidColumnType(..) => RowError::MalformedColumn {
            table,
            column: name,
            detail: "the column is not the type it was declared".to_string(),
        },
        other => RowError::Database(fault("reading a column")(other)),
    }
}

/// A row read whole, or none where a newer build wrote it with a value this build does not have.
///
/// **An older build leaves out what it does not know**, rather than failing the list the row is in: after a
/// revert the build before reads every row it can, and the row is there again for the build that wrote it.
/// Any other damage still fails. `docs/practices/store-migrations.md`, *Rows from a newer build*.
pub(crate) fn known<T>(read: Result<T, RowError>) -> Result<Option<T>, RowError> {
    match read {
        Ok(value) => Ok(Some(value)),
        Err(RowError::UnknownEnumValue { .. }) => Ok(None),
        Err(other) => Err(other),
    }
}

pub(crate) fn enum_value<T>(
    from_wire: impl Fn(&str) -> Option<T>,
    table: &'static str,
    column: &'static str,
    value: &str,
) -> Result<T, RowError> {
    from_wire(value).ok_or_else(|| RowError::UnknownEnumValue {
        table,
        column,
        value: value.to_string(),
    })
}
