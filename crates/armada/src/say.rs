//! What each verb prints, so the acting half never holds a format string.
//!
//! # A destructive command that prints nothing is one nobody trusts twice
//!
//! `clean` says what it removed, item by item.
//!
//! Absences are printed too. "There was nothing there" and "it was removed" are
//! different answers, and a command that shows only the second leaves the
//! reader to guess which one they got.

use std::path::Path;

use crate::clean::{Cleaned, FileGone};
use crate::declared::{Ended, Ran, Reached};

/// How `armada check --changed` says what it narrowed to.
pub const NARROWED_TO: &str = "  narrowed to ";

/// What one Check or Command did: its output, then how it ended.
pub fn ran(ran: &Ran, verb: &str) {
    println!("{verb} {} — {}", ran.name, ran.command);
    // `NARROWED_TO` begins the line the merge line reads back; whole says nothing.
    match &ran.narrowed {
        Some(Reached::To(to)) => println!("{NARROWED_TO}{to}"),
        Some(Reached::Nothing) => println!("{NARROWED_TO}nothing it runs"),
        Some(Reached::Touched(said)) => println!("  touched {said}"),
        Some(Reached::NoTests) => println!(
            "  no changed file is a test or holds one, so nothing ran. \
             Name a test with `armada check {} <name>`",
            ran.name
        ),
        Some(Reached::Whole) | None => {}
    }
    // Before the output, because it happened before the output — and because
    // these write to the working tree, which is the one thing a person needs
    // told rather than left to notice in `git status`.
    if !ran.required.is_empty() {
        println!("  after {}, which it requires", ran.required.join(", "));
    }
    if ran.destructive {
        println!("  the Manifest calls this destructive");
    }
    // The whole of what it printed, both streams, before the verdict. Nothing
    // here reads a byte of it — `checks-runner` does not either.
    print!("{}", ran.attempt.output.stdout);
    eprint!("{}", ran.attempt.output.stderr);
    if ran.attempt.output.truncated {
        println!("  (output was longer than the capture limit; this is the tail)");
    }
    match &ran.test {
        Some(test) if ran.matched_nothing() => println!(
            "{} has no test named `{test}`. Name it as the runner prints it: a Rust test by \
             its function name, a path ending in it (`servers::a_span_holding_one_taken_port_is_not_free`) \
             or a module above it (`tests::servers`), a vitest test by any part of its name",
            ran.name
        ),
        Some(test) => match checks_runner::one_test_count(&ran.attempt.output) {
            Some(count) if count > 1 => println!(
                "{} {}: {count} tests matched `{test}`, and all of them ran",
                ran.name,
                Ended(&ran.attempt.exit)
            ),
            _ => println!("{} {}", ran.name, Ended(&ran.attempt.exit)),
        },
        None => println!("{} {}", ran.name, Ended(&ran.attempt.exit)),
    }
}

/// Everything a clean removed, and everything it left where it was.
pub fn cleaned(cleaned: &Cleaned) {
    println!(
        "{} — Manifest `{}`",
        cleaned.repository.display(),
        cleaned.manifest_id
    );

    if !cleaned.unclaimed.is_empty() {
        println!("\nthese worktrees are left from before Jobs, and are left alone:");
        for path in &cleaned.unclaimed {
            println!("  {}", path.display());
        }
    }
    if let Some(directory) = cleaned
        .machine
        .first()
        .and_then(|first| held(first).parent())
    {
        // The directory once, then five file names. Five absolute paths under
        // one directory is four repetitions of the directory.
        println!("\nmachine state, in {}:", directory.display());
        for file in &cleaned.machine {
            println!("  {}", machine(file));
        }
    }

    // **Counted, not listed.** Each name is a forty-character commit id and a
    // person cannot do anything with one; what they want to know is that the
    // disk came back, and how many checkouts it was.
    if !cleaned.bases.is_empty() {
        let count = cleaned.bases.len();
        let plural = if count == 1 { "checkout" } else { "checkouts" };
        println!(
            "\n{count} base {plural} given back — the next run that needs a \
             `before` makes one again"
        );
    }

    if cleaned.unclaimed.is_empty() && cleaned.bases.is_empty() {
        println!("\nthere was nothing to give back");
    }
    for fault in &cleaned.faults {
        eprintln!("  {fault}");
    }
}

fn machine(file: &FileGone) -> String {
    let name = shown(held(file));
    match file {
        FileGone::Removed(_) => format!("removed: {name}"),
        FileGone::Absent(_) => format!("not there: {name}"),
        FileGone::NotRemoved { why, .. } => format!("NOT removed — {why}: {name}"),
    }
}

fn held(file: &FileGone) -> &Path {
    match file {
        FileGone::Removed(path) | FileGone::Absent(path) => path,
        FileGone::NotRemoved { path, .. } => path,
    }
}

fn shown(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}
