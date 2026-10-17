//! How a Check's process ended: the fact, before anything decides what it
//! means.
//!
//! **Nothing here parses output** — an exit code, a signal, an expired budget.
//! [`Exit::Code`] is the only variant an expectation is ever compared against,
//! so none of the ways a Check ends without a code can be compared into a
//! pass.

use std::time::Duration;

/// How a Check's process ended. The fact, before anything decides what it
/// means.
///
/// **Three ways a Check fails without failing.** A hanging Check, a Check whose
/// command is not installed, and a Check killed by a signal all produce **no
/// exit code at all**. Each is its own variant rather than a stand-in code,
/// because every stand-in is a number some real command also returns: `124` is
/// `timeout`'s convention and a real program's exit, `127` is a shell's "not
/// found" and a real program's exit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Exit {
    /// It ran to completion and returned a code. **The only variant an
    /// expectation is compared against.**
    Code(i32),
    /// A signal ended it, so there is no code. Nothing here converts a signal
    /// into the `128 + n` a shell would report — that convention is a shell's,
    /// and adopting it would put a signal back in the space of codes a check
    /// can expect.
    Signalled { signal: i32 },
    /// The budget expired and the process was killed. **A hanging Check is a
    /// failure**, and it is this variant rather than an absence.
    TimedOut { after: Duration },
    /// Nothing ran at all.
    NeverRan(NeverRan),
}

/// Why a Check's command never started.
///
/// Separate from every kind of exit, because a Check that did not run has
/// established nothing — and the failure a fresh machine actually produces is
/// a Check command that is not installed. Reporting that as a pass is the
/// vacuous pass by another name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NeverRan {
    /// The Manifest's `run` string held no program.
    NothingToRun,
    /// The program is not on the path.
    NoSuchCommand { program: String },
    /// The Job's worktree is not there. Distinguished from a missing program
    /// because the operating system reports both as *not found* and the two
    /// need opposite responses — install the tool, or find out what removed a
    /// running Job's checkout.
    WorktreeGone { worktree: String },
    /// The spawn was refused for some other reason — a permission, a directory
    /// that is not there. The kind is carried rather than a rendered sentence.
    NotSpawned {
        program: String,
        kind: std::io::ErrorKind,
    },
    /// A Command the Check names in `requires` did not succeed, so the Check
    /// was never started.
    ///
    /// **The failure belongs to the prerequisite and this is what says so.** A
    /// Check reported as failing on its prerequisite's exit code tells a Drone
    /// the wrong thing: a broken `migrate` reads as a broken test suite, and a
    /// `fmt` that will not run reads as code that will not format. `#387` is
    /// that shape — three attempts spent on a Check whose fix the Manifest
    /// already declared.
    ///
    /// It is a [`NeverRan`] rather than a kind of failure because the Check
    /// genuinely did not run, and every consequence of that already follows:
    /// it cannot be compared into a pass, and it is recorded as
    /// [`CheckOutcome::NeverRan`](core_model::CheckOutcome::NeverRan).
    PrerequisiteFailed {
        /// The Commands entry, as the Manifest wrote it. **What a person
        /// edits**, and what the sentence names.
        command: String,
        /// The line that was executed. What a person runs by hand to see it
        /// fail again.
        run: String,
        /// How the prerequisite itself ended. Boxed because a prerequisite is
        /// a process and ends in all the ways one does, this variant included
        /// — which makes the type recursive and the box the price of saying so
        /// once rather than flattening four cases into a string.
        exit: Box<Exit>,
    },
}
