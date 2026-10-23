//! The helpers a Session's record is cited with: the `cite` of a row, the cut of
//! a long sentence, and the set of cites a record holds, which is what a
//! retro item may name.

use std::collections::BTreeSet;

use core_model::Timestamp;
use ipc::{Instant, RetroRecord};

/// Every `cite` the record holds, which is what a retro item may name.
pub(crate) fn cites(record: &RetroRecord) -> BTreeSet<String> {
    let mut all = BTreeSet::new();
    all.extend(record.refusals.iter().map(|row| row.cite.clone()));
    all.extend(record.notes.iter().map(|row| row.cite.clone()));
    all.extend(record.restarts.iter().map(|row| row.cite.clone()));
    all.extend(record.asks.iter().map(|row| row.cite.clone()));
    all.extend(record.failed_tools.iter().map(|row| row.cite.clone()));
    all.extend(record.corrections.iter().map(|row| row.cite.clone()));
    all.extend(record.subagents.iter().map(|row| row.cite.clone()));
    all
}

pub(super) fn cite(kind: &str, n: usize) -> String {
    format!("{kind}:{}", n + 1)
}

pub(crate) fn cut(text: &str, most: usize) -> String {
    match text.char_indices().nth(most) {
        Some((at, _)) => format!("{}…", &text[..at]),
        None => text.to_string(),
    }
}

/// Milliseconds from one instant to a later one. Zero where either will not
/// read or the order is backwards.
pub(super) fn apart(from: &Instant, to: &Instant) -> u64 {
    let millis = |at: &Instant| Timestamp::from_rfc3339(at.as_str()).epoch_millis();
    match (millis(from), millis(to)) {
        (Some(from), Some(to)) => u64::try_from(to - from).unwrap_or(0),
        _ => 0,
    }
}
