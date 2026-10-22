//! Running tests by name, and the tests a change touched, compiling only
//! the members they are in: [`one_test`](crate::one_test) for a Check whose
//! `narrow` says what a member is.
//!
//! Here rather than in [`narrow`](crate::narrowed) because that file knows no
//! layout at all and these read the source of the one a Manifest names.

use std::collections::BTreeSet;
use std::path::Path;

use core_model::Narrowing;

use crate::holding::holders;
use crate::narrow::{one_argument, quoted, regex_escaped};
use crate::touched::touched;

/// Why a named test could not become a command.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OneTestRefused {
    /// A name is blank, or none was given.
    Blank,
    /// No member under `under` declares a function or module called `test`.
    /// `excepted` are the ones that do and the Check leaves to another.
    NoMember {
        test: String,
        under: String,
        excepted: Vec<String>,
    },
}

/// [`one_test`], compiling only the members under `under` that hold the test,
/// where the Check's `narrow` names a layout.
///
/// **The Check's own `narrow` is the one place a Manifest says what a member
/// is and how a command is told one** (`under`, `each`, `except`), so a test
/// by name reads it rather than a second set of keys. A Check with no `narrow`
/// or no `under` gets [`one_test`] as it always did, which is every Check but a
/// Cargo workspace's.
///
/// `dir` is where the Manifest is, which `under` is relative to. The template
/// must not itself select the whole workspace: cargo reads `--workspace` and
/// `-p` together as the whole workspace and drops the `-p`.
pub fn one_test_in(
    dir: &Path,
    run: &str,
    narrowing: Option<&Narrowing>,
    test: &str,
) -> Result<String, OneTestRefused> {
    tests_in(dir, run, narrowing, &[test])
}

/// [`one_test_in`] for several names at once: **one command, so one build.**
/// The names go into `{}` as one alternation and the members are every member
/// that holds any of them, as [`touched_run`] does for a change. A name no
/// member holds refuses the whole run, as it refuses one.
pub fn tests_in(
    dir: &Path,
    run: &str,
    narrowing: Option<&Narrowing>,
    tests: &[&str],
) -> Result<String, OneTestRefused> {
    let tests: Vec<&str> = tests.iter().map(|test| test.trim()).collect();
    if tests.is_empty() || tests.iter().any(|test| test.is_empty()) {
        return Err(OneTestRefused::Blank);
    }
    let command = run.replace("{}", &one_argument(&any_of(&tests)));
    let Some((narrowing, under)) = narrowing.and_then(|n| Some((n, n.under()?))) else {
        return Ok(command);
    };
    let mut members = BTreeSet::new();
    for test in tests {
        let (excepted, held): (Vec<String>, Vec<String>) = holders(dir, under, test)
            .into_iter()
            .partition(|member| narrowing.except().contains(member));
        if held.is_empty() {
            return Err(OneTestRefused::NoMember {
                test: test.to_string(),
                under: under.to_string(),
                excepted,
            });
        }
        members.extend(held);
    }
    let members: Vec<String> = members.into_iter().collect();
    Ok(with_members(command, narrowing, &members))
}

/// Names as one regex that matches any of them, each escaped: one name bare,
/// several as `(?:a|b)`.
fn any_of<S: AsRef<str>>(names: &[S]) -> String {
    let alternatives: Vec<String> = names.iter().map(|n| regex_escaped(n.as_ref())).collect();
    match alternatives.as_slice() {
        [one] => one.clone(),
        many => format!("(?:{})", many.join("|")),
    }
}

/// What a change's touched tests came to, as a command.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TouchedRun {
    /// This command, which runs `said` and nothing else. `mapped` are the
    /// names in it the test map added and no changed file touched.
    Tests {
        command: String,
        said: String,
        mapped: Vec<String>,
    },
    /// No changed file is, or holds, a test.
    None,
    /// This Check cannot run a test by name in a member, so it has no such
    /// reading: it has no `one_test`, or no `narrow` with an `under`.
    NotApplicable,
}

/// The command that runs the tests `changed` touched, in the members they are
/// in and no others: `one_test`'s command with every touched name for the
/// name, and each member as `narrow` spells one. No dependents, and no
/// `narrow.run`: this is not the gate's reading, which is [`narrowed_over`].
pub fn touched_run(
    dir: &Path,
    one_test_run: Option<&str>,
    narrowing: Option<&Narrowing>,
    changed: &[String],
) -> TouchedRun {
    touched_run_with(dir, one_test_run, narrowing, changed, &[])
}

/// [`touched_run`], with the tests the test map says reach `changed` added,
/// each a `(member, name)` with the member as `narrow.under` spells it. **One
/// command, so one build.** A test in a member `except` bars is dropped, as a
/// touched one is, and so is one with no member to spell.
pub fn touched_run_with(
    dir: &Path,
    one_test_run: Option<&str>,
    narrowing: Option<&Narrowing>,
    changed: &[String],
    from_map: &[(String, String)],
) -> TouchedRun {
    let (Some(run), Some(narrowing)) = (one_test_run, narrowing) else {
        return TouchedRun::NotApplicable;
    };
    if narrowing.under().is_none() {
        return TouchedRun::NotApplicable;
    }
    let found = touched(dir, narrowing, changed);
    let mut members: BTreeSet<String> = found.members.into_iter().collect();
    let mut names: BTreeSet<String> = found.names.into_iter().collect();
    let mut mapped = BTreeSet::new();
    for (member, name) in from_map {
        let name = name.trim();
        if member.is_empty() || name.is_empty() || narrowing.except().contains(member) {
            continue;
        }
        members.insert(member.clone());
        if names.insert(name.to_string()) {
            mapped.insert(name.to_string());
        }
    }
    if names.is_empty() {
        return TouchedRun::None;
    }
    let names: Vec<String> = names.into_iter().collect();
    let members: Vec<String> = members.into_iter().collect();
    let command = with_members(
        run.replace("{}", &one_argument(&any_of(&names))),
        narrowing,
        &members,
    );
    let members = spelled_members(narrowing, &members);
    TouchedRun::Tests {
        command,
        said: format!("{} in {members}", names.join(" ")),
        mapped: mapped.into_iter().collect(),
    }
}

/// `command` with each member spelled as `narrowing` says.
fn with_members(command: String, narrowing: &Narrowing, members: &[String]) -> String {
    format!("{command} {}", spelled_members(narrowing, members))
}

fn spelled_members(narrowing: &Narrowing, members: &[String]) -> String {
    members
        .iter()
        .map(|member| narrowing.each().replace("{}", &quoted(member)))
        .collect::<Vec<_>>()
        .join(" ")
}

/// The member and the rest of a path below `under`: `crates/ipc/src/a.rs` is
/// `("ipc", "src/a.rs")`. `None` for a path outside it or a member's bare
/// directory.
pub(crate) fn member_of<'p>(narrowing: &Narrowing, path: &'p str) -> Option<(&'p str, &'p str)> {
    let under = narrowing.under()?.trim_end_matches('/');
    let below = path.strip_prefix(under)?.strip_prefix('/')?;
    below
        .split_once('/')
        .filter(|(member, rest)| !member.is_empty() && !rest.is_empty())
}
