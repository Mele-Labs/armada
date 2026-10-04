//! `armada check <name> --changed`: a Check run over what a change reaches,
//! as the merge line asks for it. `docs/capabilities/merge-line.md`, *What a
//! narrowed Check runs*.

use checks_runner::{narrowed_over, Attempt, Narrowed};
use verification::Exit;

use crate::declared::Ran;

/// How much of a Check was asked for. One value, so one test and a changed set
/// cannot both be asked.
#[derive(Clone, Copy, Debug)]
pub enum Asked<'a> {
    Whole,
    /// One test, through the Check's `one_test`.
    OneTest(&'a str),
    /// What these changed paths reach, through the Check's `narrow` as the
    /// merge line reads it: [`checks_runner::narrowed_over`].
    Changed(&'a [String]),
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

/// A Check with nothing to run, passed without running.
pub fn ran_nothing(name: &str) -> Ran {
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
        narrowed: Some(Reached::Nothing),
    }
}
