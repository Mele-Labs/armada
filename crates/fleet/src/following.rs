//! A growing log, read from an offset.
//!
//! **`crate::journal`'s half of `api::Journal`, one file over.** `api` states
//! what a reader of a growing file answers; this is where the file is.

/// How many lines the first read of a live log carries.
///
/// **The tail**, for `crate::check_output`'s reason: a test runner prints its
/// failures last. Every pass after the first carries whatever was appended.
const A_READING: usize = 2_000;

/// How many bytes of those lines the first read carries, for
/// `crate::check_output::MOST`'s reason: one line can be a whole bundle.
const MOST: usize = 256 * 1024;

/// What [`read_from`] makes of the bytes after `from`; a log held in memory is
/// read the same way.
pub(crate) fn windowed(bytes: &[u8], from: u64, to_the_end: bool) -> api::Followed {
    let upto = match to_the_end {
        true => bytes.len(),
        false => bytes
            .iter()
            .rposition(|byte| *byte == b'\n')
            .map_or(0, |at| at + 1),
    };
    let mut lines: Vec<String> = String::from_utf8_lossy(&bytes[..upto])
        .lines()
        .map(str::to_string)
        .collect();
    let mut skipped = 0u64;
    if from == 0 {
        let mut held: usize = lines.iter().map(String::len).sum();
        let mut first = 0;
        while lines.len() - first > A_READING || (held > MOST && lines.len() - first > 1) {
            held -= lines[first].len();
            first += 1;
        }
        skipped = first as u64;
        lines.drain(..first);
    }
    api::Followed {
        lines,
        from: from + upto as u64,
        skipped,
        unreadable: false,
    }
}
