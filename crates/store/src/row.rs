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
