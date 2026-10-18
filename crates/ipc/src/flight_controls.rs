//! The Cockpit's flight controls a person arranged, and a shell action run from one.
//! `docs/concepts/kit.md`, *Flight controls*.
//!
//! **The file is the wire.** `~/.armada/flight-controls.toml` is read into [`FlightControls`] and a
//! save sends the same type whole, so nothing is translated between the person's file and what
//! Bridge holds. **Fleet does not know the built-in ids**: they are Bridge's, so what a built-in
//! tile may carry is Bridge's to check and an `id` here is just a name.

use serde::{Deserialize, Serialize};

use crate::sessions::SessionId;

/// The file's version this Fleet reads and writes.
pub const FLIGHT_CONTROLS_VERSION: u32 = 1;

fn current() -> u32 {
    FLIGHT_CONTROLS_VERSION
}

fn is_false(value: &bool) -> bool {
    !*value
}

/// What a person's own action does when pressed. Absent on a built-in tile.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FlightControlKind {
    /// Words sent to the Job.
    Words,
    /// A slash command sent to the Job.
    Command,
    /// A shell line run in the Job's bay: `run_session_shell`.
    Shell,
    /// A page opened.
    Page,
}

/// One tile. **Every field but `id` is the person's change** to what Bridge draws by default.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FlightControl {
    /// A built-in id, or the person's own.
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// A lucide name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub glyph: Option<String>,
    /// One of `1` to `9`, `0`, or a letter.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub hidden: bool,
    /// The words or command a built-in sends, or an own action's words, slash command, shell line
    /// or address.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// The land tile only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub on_text: Option<String>,
    /// The land tile only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub off_text: Option<String>,
    /// A person's own action only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<FlightControlKind>,
}

/// The whole file, and the body of `save_flight_controls`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FlightControls {
    #[serde(default = "current")]
    pub version: u32,
    /// The tiles while a Job is open, in order. A built-in not listed is appended by Bridge.
    #[serde(default)]
    pub job: Vec<FlightControl>,
    /// The tiles with no Job open, in order.
    #[serde(default)]
    pub deck: Vec<FlightControl>,
}

impl Default for FlightControls {
    /// Armada's own: nothing changed.
    fn default() -> FlightControls {
        FlightControls { version: FLIGHT_CONTROLS_VERSION, job: Vec::new(), deck: Vec::new() }
    }
}

/// `get_flight_controls`' answer, and `save_flight_controls`' once it has read the file again.
/// **Tagged**, so a file that will not read is never drawn as the defaults.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "what", rename_all = "snake_case")]
pub enum FlightControlsRead {
    Read { controls: FlightControls },
    NotRead { why: String },
}

/// `run_session_shell`: the person pressed a shell action. `POST /sessions/shell`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunSessionShell {
    pub session_id: SessionId,
    pub command: String,
}

/// `run_session_shell`, answered 200 once the command has ended or been stopped for running too long.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionShellRan {
    /// Absent where a signal ended it, which a timeout does.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exit: Option<i32>,
    pub timed_out: bool,
    /// Standard output and error as they arrived, the last part of it.
    pub output: String,
    /// Whether the start of the output was dropped to fit.
    pub truncated: bool,
}
