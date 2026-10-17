//! A Fleet-authored line on Fleet's standard error.
//!
//! A line is `core_model::Envelope` in the shape `docs/concepts/log-envelope.md`
//! names, so a query and the redactor reach it.

mod row;

use std::io;

use core_model::Envelope;

use row::Line;

/// Write one Fleet-authored line to Fleet's standard error, for what belongs to no Job and so has no log to join.
///
/// The same line the Job log carries, so a query and the redactor reach it; a stream that will not take it
/// loses the line and nothing else.
pub fn note_aloud(envelope: &Envelope) {
    use std::io::Write;
    if let Ok(line) = encoded(&Line::from(envelope)) {
        let _ = std::io::stderr().write_all(line.as_bytes());
    }
}

/// One JSON object and the newline after it, through `ipc`'s encoder rather
/// than a second one here.
fn encoded<T: serde::Serialize>(value: &T) -> Result<String, io::Error> {
    let mut line =
        ipc::encode(value).map_err(|why| io::Error::new(io::ErrorKind::InvalidData, why.why))?;
    line.push('\n');
    Ok(line)
}
