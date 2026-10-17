//! The lines a running log carries over its socket, and why one stopped
//! arriving. A merge line's Check, a Session's Check, a server and a run each
//! open their own first message and share these two.

use serde::{Deserialize, Serialize};

/// Lines written since the last message, oldest first, verbatim.
///
/// **Whole lines only, until the end.** A line still being written is held
/// back until its newline arrives, so a reader never shows half a line that
/// the next message would have to take back. The last line of a Check that
/// ended without a newline comes on the final pass.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutputLines {
    pub lines: Vec<String>,
}

/// The end of the stream, said rather than left.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutputClosed {
    pub because: OutputEnded,
}

/// Why a running Check's log stopped arriving.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutputEnded {
    /// The Check ended, and every line it wrote has been sent.
    Finished,
    /// The file is there and could not be read.
    Unreadable,
}
