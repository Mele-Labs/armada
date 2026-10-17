//! What a Check produced, written down and read back: the output to a file,
//! the row to the store, and the file to whoever opens that row.
//!
//! # A file, with the path on the row
//!
//! The same call the Drone transcript got, for the same reason: a large
//! artifact with its own retention profile lives on disk and the record holds a
//! reference. `Ruling::Failed` carried the bytes in memory and they died with
//! the process, so a failed Check showed its exit code and never its output.
//!
//! # The path is a function of the row's key, and nothing else
//!
//! `.armada/checks/<job-id>/<step-id>.<attempt>.<ordinal>.log`, which is the
//! row's key exactly: `job_step_checks` is keyed by Job, step, attempt and
//! ordinal, and every one of the four is here.
//!
//! The attempt was missing while nothing could run a step twice. Now a failed
//! Check hands the step back, so a second run wrote over the first run's files
//! while `store` kept both runs' rows — leaving the first attempt's row
//! pointing at the second attempt's output, which is worse than pointing at
//! nothing. A path that is the whole key cannot do that.
//!
//! # Over 500 lines: `#737`'s [`excerpt`] shares [`windowed`] with
//! [`kept_output`] rather than copying its bound into a file of its own.

use std::collections::VecDeque;

/// Whether an id can stand as one path component.
///
/// **One predicate, two callers.** `crate::asked` names files after a step id
/// and a criterion id under the same rule and for the same reason; a second
/// spelling of it here and there is how the two would come to disagree about
/// which id is safe.
pub fn one_component(id: &str) -> bool {
    !id.is_empty()
        && id != "."
        && id != ".."
        && !id.contains('/')
        && !id.contains('\\')
        && !id.contains('\0')
}

/// How many lines of a Check's output one read carries.
///
/// **The tail, for the reason `checks_runner` captures the tail**: a test
/// runner prints its failures last, and a runaway command prints forever, so
/// keeping the beginning keeps the part nobody opened the Job for.
///
/// `crate::asked::read_back` reads a kept brief through the same window.
pub(crate) const A_READING: usize = 2_000;

/// How many bytes of those lines one read carries.
///
/// A second bound beside the line count, because one line can be a whole
/// minified bundle. The window loses its oldest lines to stay under it, which
/// keeps this read's cost a property of the code rather than of whatever a
/// Check printed.
pub(crate) const MOST: usize = 256 * 1024;

/// A window kept over a stream of lines: whichever bound is hit first evicts
/// the oldest line, so what survives is the tail under both — [`kept_output`]
/// and [`excerpt`] open through this rather than each keeping its own copy of
/// the eviction rule.
///
/// Returns the window, the file's own number for the window's first line
/// (counted from one), and how many lines the stream held in total.
///
/// `pub(crate)` for `crate::studio_runs`, which windows a swept run's log to
/// the Studio's own bound rather than restating the eviction rule.
pub(crate) fn windowed(
    lines: impl Iterator<Item = String>,
    most_lines: usize,
    most_bytes: usize,
) -> (VecDeque<String>, u32, u32) {
    let mut window: VecDeque<String> = VecDeque::new();
    let mut held = 0usize;
    let mut total = 0u32;
    let mut first = 1u32;
    for line in lines {
        total = total.saturating_add(1);
        held += line.len();
        window.push_back(line);
        while window.len() > most_lines || (held > most_bytes && window.len() > 1) {
            held -= window
                .pop_front()
                .map(|gone| gone.len())
                .unwrap_or_default();
            first = first.saturating_add(1);
        }
    }
    (window, first, total)
}

