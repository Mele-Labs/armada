//! Running one test by name, and the tests a change touched, compiling only
//! the members they are in: [`one_test`](crate::one_test) for a Check whose
//! `narrow` says what a member is.
//!
//! Here rather than in [`narrow`](crate::narrowed) because that file knows no
//! layout at all and these read the source of the one a Manifest names.

use std::path::Path;

use core_model::Narrowing;

use crate::holding::holders;
use crate::narrow::{one_argument, one_test, quoted, regex_escaped};
use crate::touched::touched;

/// Why a named test could not become a command.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OneTestRefused {
    /// The name is blank.
    Blank,
    /// No member under `under` declares a function or module of that name.
    /// `excepted` are the ones that do and the Check leaves to another.
    NoMember {
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
    let command = one_test(run, test).ok_or(OneTestRefused::Blank)?;
    let Some((narrowing, under)) = narrowing.and_then(|n| Some((n, n.under()?))) else {
        return Ok(command);
    };
    let (excepted, held): (Vec<String>, Vec<String>) = holders(dir, under, test)
        .into_iter()
        .partition(|member| narrowing.except().contains(member));
    if held.is_empty() {
        return Err(OneTestRefused::NoMember {
            under: under.to_string(),
            excepted,
        });
    }
    Ok(with_members(command, narrowing, &held))
}

/// What a change's touched tests came to, as a command.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TouchedRun {
    /// This command, which runs `said` and nothing else.
    Tests { command: String, said: String },
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
    let (Some(run), Some(narrowing)) = (one_test_run, narrowing) else {
        return TouchedRun::NotApplicable;
    };
    if narrowing.under().is_none() {
        return TouchedRun::NotApplicable;
    }
    let found = touched(dir, narrowing, changed);
    if found.is_empty() {
        return TouchedRun::None;
    }
    let alternatives: Vec<String> = found.names.iter().map(|n| regex_escaped(n)).collect();
    let name = match alternatives.as_slice() {
        [one] => one.clone(),
        many => format!("(?:{})", many.join("|")),
    };
    let command = with_members(
        run.replace("{}", &one_argument(&name)),
        narrowing,
        &found.members,
    );
    let members = spelled_members(narrowing, &found.members);
    TouchedRun::Tests {
        command,
        said: format!("{} in {members}", found.names.join(" ")),
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
