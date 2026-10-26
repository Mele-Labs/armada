//! A Job's shell: the one terminal the person and the Job's agent both type into. Typing into it,
//! resizing it, watching it and stopping it are the run operations, on its `run_id`.

use serde::{Deserialize, Serialize};

use crate::sessions::SessionId;

/// `open_terminal`: the person opens the Job's shell in Bridge. `POST /sessions/terminal/open`.
/// Answered 200 with `SessionRunUnderway`: the shell already open, or one opened now.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpenTerminal {
    pub session_id: SessionId,
    /// The terminal's size in cells where it opens now. Absent is 80 by 24.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cols: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rows: Option<u16>,
}

/// `type_in_terminal`: the Job's agent types into the Job's shell, opening it where it has none.
/// `POST /sessions/terminal/type`. Answered 200 with `TerminalTyped`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TypeInTerminal {
    /// Written as it is: Ctrl-C is the character 3.
    pub text: String,
    /// Whether Enter follows `text`. Absent is `true`.
    #[serde(default = "pressed")]
    pub enter: bool,
    /// Whether Ctrl-U goes first, clearing whatever already stands on the line. Absent is `true`.
    /// Text with nothing printable in it, such as Ctrl-C, is typed without it.
    #[serde(default = "pressed")]
    pub clear: bool,
    /// The longest to wait for the shell to go quiet. Absent is 10; at most 60.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wait_seconds: Option<u64>,
    /// Which Job. **Read only where the connection places no hosted Job**: a terminal one's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<SessionId>,
}

fn pressed() -> bool {
    true
}

/// `type_in_terminal`'s answer: what the shell printed after the text was typed, as plain lines.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerminalTyped {
    pub run_id: String,
    /// The screen's lines, escapes and overwrites resolved. The last 200 at most.
    pub printed: String,
    /// Whether output was still arriving when the wait ran out.
    pub still_printing: bool,
    /// `false` where the text typed never appeared in what the shell printed: a password prompt,
    /// or a program that is not reading. `true` where the text held nothing printable.
    pub echoed: bool,
}

/// `read_terminal`: the Job's agent reads the Job's shell. `GET /sessions/terminal/read`, these as
/// its query. Answered 200 with `TerminalScreen`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReadTerminal {
    /// Which Job. **Read only where the connection places no hosted Job**, as `TypeInTerminal`'s.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<SessionId>,
    /// How many of the last lines. Absent is 60; at most 500.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lines: Option<usize>,
    /// Text to wait for in what the shell printed since the agent last typed into it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub until: Option<String>,
    /// Whether `until` is a regular expression rather than text. Absent is `false`.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub regex: bool,
    /// The longest to wait for `until`. Absent is 10; at most 60. Read only with `until`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wait_seconds: Option<u64>,
}

/// `read_terminal`'s answer: the last lines of the Job's shell, as its screen drew them.
/// `GET /sessions/terminal/read`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerminalScreen {
    pub run_id: String,
    /// The screen's lines, escapes and overwrites resolved.
    pub screen: String,
    /// Whether `until` was printed before the wait ran out. Absent where the read named no `until`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub matched: Option<bool>,
}

/// `terminal.opened` and `terminal.exited` on `/events`: a Job's shell opened or ended, so a pane
/// can follow it. What it prints is `observe_session_run`'s, on `run_id`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JobTerminal {
    pub session_id: SessionId,
    pub run_id: String,
}
