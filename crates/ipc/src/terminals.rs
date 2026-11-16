//! A Job's terminals: shells the person and the Job's agent both type into, each named `t1`, `t2`
//! and so on. Typing into one, resizing it, watching it and stopping it are the run operations, on
//! its `run_id`.

use serde::{Deserialize, Serialize};

use crate::sessions::SessionId;

/// `open_terminal`: the person opens one of the Job's terminals in Bridge.
/// `POST /sessions/terminal/open`. Answered 200 with `SessionRunUnderway`: the shell already open,
/// or one opened now. 422 for a `terminal` the Job never had, and for `new` with `terminal`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpenTerminal {
    pub session_id: SessionId,
    /// Which terminal: its running shell, or a new shell under the same ID where it has ended.
    /// Absent with `new` absent is the Job's first terminal, `t1` where it has none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub terminal: Option<String>,
    /// Mint the next ID and start a shell under it. Absent is `false`. Not read with `terminal`.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub new: bool,
    /// The terminal's size in cells where it opens now. Absent is 80 by 24.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cols: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rows: Option<u16>,
}

/// `start_side_job`: a Side Job, a Job that opens straight onto its terminal `t1` and whose
/// agent runs nothing until the person's first message. `POST /sessions/side_job/start`. Answered 200
/// with [`SideJobStarted`]. `docs/concepts/session/a-side-job.md`.
///
/// **Exactly one of `manifest_id` and `dir`.** A `dir` inside a served repository is that
/// repository's Job with `t1` in the folder; anywhere else it is a loose folder, run in place with no
/// Manifest and never a slot, and nothing is written to make it a repository.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StartSideJob {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub manifest_id: Option<crate::ManifestId>,
    /// An absolute path to an existing directory.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dir: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effort: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mode: Option<crate::SessionMode>,
    /// As `start_session`'s. Not read for a loose folder.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project: Option<String>,
    /// `t1`'s size in cells. Absent is 80 by 24.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cols: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rows: Option<u16>,
}

/// `start_side_job`'s answer: the Job, and the run of its terminal `t1`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SideJobStarted {
    pub session_id: SessionId,
    pub terminal: String,
    pub run_id: String,
}

/// `start_terminal_run`: the person runs a command in a directory with no Job behind it, such as the
/// fix a pull request panel names. `POST /sessions/terminal/run`. Answered 200 with
/// `SessionRunUnderway`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StartTerminalRun {
    pub command: String,
    /// The directory the command runs in, which must exist.
    pub dir: String,
    /// The terminal's size in cells. Absent is 80 by 24.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cols: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rows: Option<u16>,
}

/// `type_in_terminal`: the Job's agent types into one of the Job's terminals. `POST
/// /sessions/terminal/type`. Answered 200 with `TerminalTyped`.
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
    /// Which terminal. Refused 422 where it is not open. Absent with `new` absent is the one the
    /// agent last used, else a new one. **Never the person's by default.**
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub terminal: Option<String>,
    /// Open a new terminal of the agent's own, then type. Absent is `false`. Not read with `terminal`.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub new: bool,
}

fn pressed() -> bool {
    true
}

/// `type_in_terminal`'s answer: what the shell printed after the text was typed, as plain lines.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerminalTyped {
    /// Which terminal was typed into: `t1`, `t2`.
    pub terminal: String,
    pub run_id: String,
    /// The screen's lines, escapes and overwrites resolved. The last 200 at most.
    pub printed: String,
    /// Whether output was still arriving when the wait ran out.
    pub still_printing: bool,
    /// `false` where the text typed never appeared in what the shell printed: a password prompt,
    /// or a program that is not reading. `true` where the text held nothing printable.
    pub echoed: bool,
}

/// `read_terminal`: the Job's agent reads one of the Job's terminals. `GET /sessions/terminal/read`,
/// these as its query. Answered 200 with `TerminalScreen`.
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
    /// Which terminal. Absent is the one the agent last used, else the Job's first open one. Refused
    /// 422 where it is not open, or where the Job has none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub terminal: Option<String>,
}

/// `read_terminal`'s answer: the last lines of a Job's terminal, as its screen drew them.
/// `GET /sessions/terminal/read`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerminalScreen {
    /// Which terminal was read: `t1`, `t2`.
    pub terminal: String,
    pub run_id: String,
    /// The screen's lines, escapes and overwrites resolved.
    pub screen: String,
    /// Whether `until` was printed before the wait ran out. Absent where the read named no `until`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub matched: Option<bool>,
}

/// `list_terminals`: the Job's open terminals. `GET /sessions/terminal/list`, these as its query.
/// Answered 200 with `JobTerminals`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListTerminals {
    /// Which Job. **Read only where the connection places no hosted Job**, as `TypeInTerminal`'s.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<SessionId>,
}

/// `show_terminal`: the agent brings one of its Job's terminals to the front in Bridge, without
/// moving the person to the Job. `POST /sessions/terminal/show`. Answered 200 with the terminal as
/// `ListedTerminal`, and `terminal.shown` follows. 422 where it is not open.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShowTerminal {
    /// Which terminal. Absent is the one the agent last used where it is open, else the Job's
    /// first open one, as `ReadTerminal`'s.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub terminal: Option<String>,
    /// Which Job. **Read only where the connection places no hosted Job**, as `TypeInTerminal`'s.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<SessionId>,
}

/// `list_terminals`' answer: the Job's open terminals, the first opened first. One whose shell has
/// ended is not listed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JobTerminals {
    pub terminals: Vec<ListedTerminal>,
}

/// One open terminal of a Job.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListedTerminal {
    /// `t1`, `t2`.
    pub terminal: String,
    /// What the person calls it: `Terminal 2` until they rename it.
    pub name: String,
    pub run_id: String,
    /// Whether a program other than the shell has the foreground.
    pub running: bool,
}

/// `rename_terminal`: the person names one of a Job's terminals. `POST /sessions/terminal/rename`.
/// Answered 200 with the terminal as `ListedTerminal`. 422 for a terminal the Job has not open.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RenameTerminal {
    pub session_id: SessionId,
    pub terminal: String,
    /// Blank restores the default name, `Terminal N`.
    pub name: String,
}

/// `terminal.opened`, `terminal.exited` and `terminal.shown` on `/events`: one of a Job's shells
/// opened, ended or was brought to the front, so a pane can follow it. What it prints is `observe_session_run`'s, on `run_id`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JobTerminal {
    pub session_id: SessionId,
    /// Which terminal: `t1`, `t2`.
    pub terminal: String,
    pub run_id: String,
}

/// `share_job_terminal`: what another Mac asks of a terminal of one of this Mac's Jobs, through the link.
/// `POST /other_mac/terminal`. Each is the person's own act at home, on the Job `session_id` names, and **a run
/// is reached only as one of that Job's open terminals**, so no other run of this Mac is typed into.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TerminalFromMac {
    /// As `open_terminal`. Answered with `SessionRunUnderway`.
    Open(OpenTerminal),
    /// As `list_terminals`. Answered with `JobTerminals`.
    List { session_id: SessionId },
    /// As `write_session_run`. Answered with `SessionRunUnderway`.
    Write { session_id: SessionId, run_id: String, data: String },
    /// As `resize_session_run`. Answered with `SessionRunUnderway`.
    Resize { session_id: SessionId, run_id: String, cols: u16, rows: u16 },
    /// As `stop_session_run`. Answered with `SessionRunUnderway`.
    Stop { session_id: SessionId, run_id: String },
}

impl TerminalFromMac {
    pub fn session_id(&self) -> &SessionId {
        match self {
            TerminalFromMac::Open(open) => &open.session_id,
            TerminalFromMac::List { session_id }
            | TerminalFromMac::Write { session_id, .. }
            | TerminalFromMac::Resize { session_id, .. }
            | TerminalFromMac::Stop { session_id, .. } => session_id,
        }
    }
}

/// `share_job_terminal_output`'s query: one of Job `session_id`'s open terminals, by its run.
/// `GET /other_mac/terminal/observe`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerminalOfJob {
    pub session_id: SessionId,
    pub run_id: String,
}
