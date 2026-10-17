//! Making a fresh checkout workable: running what the repository's `setup.requires` names.
//!
//! A checkout is a branch and its files and nothing else. What a repository's Checks need beyond
//! that is the repository's to say, and `setup.requires` in its `armada.yml` is where it says it.
//! [`prepare_one`] runs one such command, and a seed's or a base checkout's preparation calls it
//! in order, stopping at the first failure.
//!
//! **Not a Check, and the distinction is the whole design.** A Check gates a step and re-runs at
//! the gate; preparation gates nothing and runs before any step exists. Sharing one mechanism
//! would make a failed `pnpm install` read as failing work, which is #227 arriving a second time
//! — so the two share only [`checks_runner::run`], a process and a budget with no opinion about
//! what it runs.

use std::fmt;

use checks_runner::Output;
use verification::{Exit, NeverRan};

/// How much of a failed command's output rides the escalation.
///
/// The tail, for `checks_runner::CAPTURE_LIMIT`'s reason, and far shorter than
/// it: this ends up in one log field a person reads beside a status, not in a
/// pane they scroll. The full capture is on the process's own stdout.
const SAID_LIMIT: usize = 1_500;

/// A worktree a Job cannot be run in, and the command that was supposed to make
/// it one.
///
/// **Every field exists so the reason names the command.** #227 is a Job that
/// failed three times on `Cannot find module 'react'` — an accurate message
/// about the wrong thing, produced by a Check that ran in a tree nothing had
/// prepared. A failure here that said only *setup failed* would be the same
/// defect one layer up.
#[derive(Debug)]
pub struct NotPrepared {
    /// The `setup.requires` entry, as the Manifest wrote it. **What a person
    /// edits.**
    pub command: String,
    /// The line that was executed. What a person runs by hand to see it fail
    /// again.
    pub run: String,
    /// How it ended.
    pub exit: Exit,
    /// What it printed, before the tail this renders.
    pub output: Output,
}

impl NotPrepared {
    /// The tail of what the command printed, for the sentence below.
    ///
    /// **stderr, then stdout.** A tool that fails ordinarily says why on
    /// stderr; one that says nothing there — `tsc`, `pnpm` on a resolution
    /// failure — put it on stdout, and an empty field would send a reader
    /// looking for a log that has the answer in it.
    fn tail(&self) -> &str {
        let said = match self.output.stderr.trim().is_empty() {
            false => self.output.stderr.trim(),
            true => self.output.stdout.trim(),
        };
        let from = said
            .char_indices()
            .nth(said.chars().count().saturating_sub(SAID_LIMIT))
            .map(|(at, _)| at)
            .unwrap_or(0);
        &said[from..]
    }
}

impl fmt::Display for NotPrepared {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            out,
            "the worktree was not prepared: `{}` (`{}`) ",
            self.command, self.run
        )?;
        match &self.exit {
            Exit::Code(code) => write!(out, "exited {code}")?,
            Exit::Signalled { signal } => write!(out, "was ended by signal {signal}")?,
            Exit::TimedOut { after } => write!(out, "ran past its budget of {}s", after.as_secs())?,
            Exit::NeverRan(NeverRan::NothingToRun) => write!(out, "names no program to run")?,
            Exit::NeverRan(NeverRan::NoSuchCommand { program }) => {
                write!(out, "needs `{program}`, which is not on the path")?
            }
            Exit::NeverRan(NeverRan::WorktreeGone { worktree }) => {
                write!(out, "had no worktree to run in — {worktree} is not there")?
            }
            Exit::NeverRan(NeverRan::NotSpawned { program, kind }) => {
                write!(out, "could not start `{program}`: {kind:?}")?
            }
            // Unreachable: a `setup.requires` entry is a Command run directly,
            // and only a Check declares prerequisites. Spelled out rather than
            // matched with `_` so that a preparation which one day did have
            // them fails to compile here instead of printing a sentence about
            // the wrong thing.
            Exit::NeverRan(NeverRan::PrerequisiteFailed { command, .. }) => {
                write!(out, "was blocked by `{command}`, which it should not have")?
            }
        }
        let tail = self.tail();
        match tail.is_empty() {
            true => write!(out, ", and printed nothing"),
            false => write!(out, ". It said: {tail}"),
        }
    }
}

impl std::error::Error for NotPrepared {}
