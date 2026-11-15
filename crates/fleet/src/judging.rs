//! Making a model call and reading what it said.
//!
//! **A call that could not be made is not an answer.** A missing program, a
//! non-zero exit, a signal, an expired budget, an empty answer, prose instead of
//! a verdict — each is a [`CallFailed`]. A machine that cannot answer must not
//! produce one, in either direction.
//!
//! `running` names no criterion: it is a child process, a pipe and a budget, and
//! it is what every call Fleet makes of a model goes through. What is shared
//! stays here: the budget, the standing rules a brief carries, and every way a
//! call can fail.

mod running;

pub(crate) use running::{said, watched};

use std::fmt;
use std::time::Duration;

use std::sync::Arc;

use adapter_traits::{Environment, Model, ModelClient};
use verification::{Standing, Unreadable};

/// How long one Judge call may take before it is a failed call.
///
/// A newtype with one constructor and **no `Default`**: `crates/config/settings.toml`
/// names no Judge latency budget, so there is no value to read and one invented
/// here would be a threshold nobody could find.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct JudgeBudget(Duration);

impl JudgeBudget {
    pub fn of(budget: Duration) -> JudgeBudget {
        JudgeBudget(budget)
    }

    pub fn duration(&self) -> Duration {
        self.0
    }
}

/// Everything one plain model call needs in order to ask.
///
/// One client, one budget, Fleet's own environment, and a model that is a value
/// rather than a literal: a Session's judge, a retro and a reading of a command
/// are all this call, and differ only in the dial the model comes off.
#[derive(Clone)]
pub struct Explaining {
    pub client: Arc<dyn ModelClient + Send + Sync>,
    pub budget: JudgeBudget,
    /// What the call is read by. Resolved by the composition root — nothing
    /// below Fleet may spell a vendor's model name.
    pub model: Model,
    /// Fleet's own, because the call authenticates as Fleet.
    pub environment: Environment,
}

/// The file `armada.yml`'s `standing_rules` names, read from `root`.
///
/// `root` is the repository's own checkout and never a Job's worktree, so a
/// Drone cannot rewrite what its Judge is told the repository requires.
pub(crate) fn standing(manifest: &config::Manifest, root: &str, cap: usize) -> Standing {
    let Some(path) = manifest.standing_rules() else {
        return Standing::unstated();
    };
    match std::fs::read_to_string(std::path::Path::new(root).join(path)) {
        Ok(text) => Standing::read_within(path, &text, cap),
        Err(_) => Standing::unreadable(path),
    }
}

/// Why the Judge did not answer. **Never a verdict** — see this module's
/// comment.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CallFailed {
    /// The model name or the question was not one a call could be made from.
    NothingToAsk,
    /// The process would not start.
    NotStarted {
        program: String,
        kind: std::io::ErrorKind,
    },
    /// The question could not be delivered.
    NotAsked { kind: std::io::ErrorKind },
    /// It was still running when the budget expired. **The latency case**, and
    /// the one a person waiting at the gate feels.
    TimedOut,
    /// Somebody watching it decided not to wait, and it was killed.
    ///
    /// **Not a fault, and that is the whole reason it is its own variant.**
    /// Every other arm here is something going wrong; this one is a person
    /// exercising a control that was offered to them, and a surface that folded
    /// it into [`Refused`](CallFailed::Refused) would draw a decision as a
    /// failure. It can only arise on [`watched`], because it is the only call
    /// anybody can reach while it is out.
    Stopped,
    /// It ended badly — the network, the quota, an expired credential.
    ///
    /// `tail` is a capped record of what it printed on stdout and stderr
    /// before it stopped — bounded so a runaway stream cannot fill a refusal
    /// or a log line. `#1047`: it used to carry nothing, so a call that
    /// reached for a tool and spent its one turn said only "code 1".
    ///
    /// `said` is the call's own one line about why, where it gave one: the
    /// harness's error text, else the last line on stderr. **Never the stream**,
    /// so a surface that draws it in a line can.
    Refused { code: Option<i32>, tail: String, said: String },
    /// It ended well and printed nothing.
    SaidNothing,
    /// It answered something this cannot act on.
    Unreadable(Unreadable),
}

impl fmt::Display for CallFailed {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CallFailed::NothingToAsk => {
                out.write_str("the step names a model or a criterion a call cannot be made from")
            }
            CallFailed::NotStarted { program, kind } => {
                write!(out, "`{program}` would not start: {kind}")
            }
            CallFailed::NotAsked { kind } => {
                write!(out, "the question could not be delivered: {kind}")
            }
            CallFailed::TimedOut => out.write_str("the Judge did not answer inside its budget"),
            CallFailed::Stopped => out.write_str("the call was stopped before it answered"),
            CallFailed::Refused { code, tail, .. } => {
                let ended = match code {
                    Some(code) => format!("the Judge call ended with code {code}"),
                    None => "a signal ended the Judge call".to_string(),
                };
                match tail.is_empty() {
                    true => out.write_str(&ended),
                    false => write!(out, "{ended}: {tail}"),
                }
            }
            CallFailed::SaidNothing => out.write_str("the Judge answered with nothing at all"),
            CallFailed::Unreadable(why) => write!(out, "{why}"),
        }
    }
}

impl CallFailed {
    /// One short sentence for a surface that draws it in a line. [`Display`](fmt::Display) keeps
    /// everything a log wants, and for a refused call that is the whole of what it printed.
    pub(crate) fn in_a_line(&self, call: &str) -> String {
        let CallFailed::Refused { code, said, .. } = self else {
            return self.to_string();
        };
        let ended = match code {
            Some(code) => format!("The {call} call ended with code {code}"),
            None => format!("A signal ended the {call} call"),
        };
        let cut = said.chars().count().saturating_sub(SAID_AT_MOST);
        match (said.is_empty(), cut) {
            (true, _) => ended,
            (false, 0) => format!("{ended}: {said}"),
            (false, _) => format!("{ended}: {}…", said.chars().take(SAID_AT_MOST).collect::<String>()),
        }
    }
}

/// How much of what a refused call said is put in a line.
const SAID_AT_MOST: usize = 200;

impl std::error::Error for CallFailed {}
