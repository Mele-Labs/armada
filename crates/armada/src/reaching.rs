//! `armada check <name> --changed`: a Check run over what a change reaches,
//! as the merge line asks for it. `git show ecfffb2c1ebd:docs/capabilities/merge-line.md`, *What a
//! narrowed Check runs*.

use std::path::Path;

use checks_runner::{narrowed_over, touched_run, Attempt, Narrowed, Priority, TouchedRun};
use verification::Exit;

use crate::declared::Ran;

/// How much of a Check was asked for. One value, so tests by name and a changed
/// set cannot both be asked.
#[derive(Clone, Copy, Debug)]
pub enum Asked<'a> {
    Whole,
    /// Tests by name, through the Check's `one_test`: one run, so one build.
    Tests(&'a [&'a str]),
    /// What these changed paths reach, through the Check's `narrow` as the
    /// merge line reads it: [`checks_runner::narrowed_over`].
    Changed(&'a [String]),
    /// The tests these changed paths touch, in the members they are in, and
    /// no dependents: an agent's reading of `--changed`. [`asked_changed`] says
    /// who gets it, and a Check that cannot run a test by name reads it as
    /// [`Asked::Changed`].
    Touched(&'a [String]),
}

/// What `--changed` asks for from this process.
///
/// **The merge line's reading belongs to whoever is not an agent.** A run at
/// agent priority on a machine somebody shares gets the tests it touched; a
/// CI runner (`ARMADA_SOLE_TENANT`) and the merge line (`ARMADA_CHECK_PRIORITY=
/// normal`) get what they always did, every member the paths name. An empty
/// list is no change at all and runs the Check whole, as before.
pub fn asked_changed(paths: &[String], priority: Priority, sole_tenant: bool) -> Asked<'_> {
    match (priority, sole_tenant, paths.is_empty()) {
        (Priority::Low, false, false) => Asked::Touched(paths),
        _ => Asked::Changed(paths),
    }
}

/// What [`Asked::Changed`] came to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Reached {
    /// Run whole: a covered path the narrowing cannot name, or none declared.
    Whole,
    /// Run narrowed, with these arguments after the narrowing's own `run`.
    To(String),
    /// Nothing the Check runs, so nothing ran.
    Nothing,
    /// Run the tests the change touched: these names, in these members.
    Touched(String),
    /// The change touched no test, so nothing ran.
    NoTests,
}

/// The narrowed command, where there is one, and what `changed` came to.
pub fn reached(check: &config::Check, changed: &[String]) -> (Option<String>, Reached) {
    match narrowed_over(check.narrow(), changed, |path| {
        check.covers(std::slice::from_ref(path))
    }) {
        Narrowed::Whole => (None, Reached::Whole),
        Narrowed::Nothing => (None, Reached::Nothing),
        Narrowed::To(command) => {
            let run = check.narrow().map_or("", |narrow| narrow.run());
            let to = command.strip_prefix(run).unwrap_or(&command).trim();
            let to = to.to_string();
            (Some(command), Reached::To(to))
        }
    }
}

/// [`reached`], or for an agent's [`Asked::Touched`] the tests the change
/// touched, run from `dir`. A Check that cannot run a test by name has no such
/// reading and is narrowed as [`reached`] narrows it.
pub fn reading(
    dir: &Path,
    check: &config::Check,
    changed: &[String],
    touched: bool,
) -> (Option<String>, Reached) {
    if touched {
        match touched_run(dir, check.one_test(), check.narrow(), changed) {
            TouchedRun::Tests { command, said } => return (Some(command), Reached::Touched(said)),
            TouchedRun::None => return (None, Reached::NoTests),
            TouchedRun::NotApplicable => {}
        }
    }
    reached(check, changed)
}

/// What a Session's thread row says a `--changed` run was narrowed to: the values the narrowing
/// resolved to, after this. Absent where it resolved to none.
const CHANGED: &str = "--changed ";

/// The values `to` names, read back through the narrowing's `each`, so `-p fleet -p ipc` under
/// `-p {}` is `fleet, ipc`. `None` where the Check declares no narrowing or `to` names none.
pub fn target_of(check: &config::Check, to: &str) -> Option<String> {
    let words = |text: &str| {
        let (program, args) = checks_runner::split(text)?;
        Some([vec![program], args].concat())
    };
    let each = words(check.narrow()?.each())?;
    let at = each.iter().position(|word| word.contains("{}"))?;
    let (before, after) = each[at].split_once("{}")?;
    let named = words(to)?;
    let values: Vec<&str> = named
        .chunks(each.len())
        .filter_map(|chunk| chunk.get(at)?.strip_prefix(before)?.strip_suffix(after))
        .collect();
    (!values.is_empty()).then(|| format!("{CHANGED}{}", values.join(", ")))
}

/// A Check with nothing to run, passed without running.
pub fn ran_nothing(name: &str) -> Ran {
    ran_nothing_because(name, Reached::Nothing)
}

/// [`ran_nothing`], saying which kind of nothing.
pub fn ran_nothing_because(name: &str, because: Reached) -> Ran {
    Ran {
        name: name.to_string(),
        command: String::new(),
        test: None,
        destructive: false,
        required: Vec::new(),
        attempt: Attempt {
            exit: Exit::Code(0),
            output: Default::default(),
        },
        narrowed: Some(because),
        target: None,
    }
}
