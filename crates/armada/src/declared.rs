//! Running one thing the Manifest declares, without a Fleet.
//!
//! # This is the second reader of `armada.yml`, and that is the point
//!
//! A file one program reads is that program's configuration. A file two
//! programs read is a contract, and the difference shows up the first time the
//! second reader disagrees. So `check` and `run` load the same `Manifest`
//! through the same `config` crate, and execute through the same
//! `checks-runner` a Job's gate uses — a Check a person runs here is the Check
//! a Drone is measured by, not an approximation of it.
//!
//! # Two verbs, because the Manifest has two registries
//!
//! Checks gate advancement and Commands do not. Naming a Check at `armada run`
//! is refused rather than quietly obliged: a person who typed the wrong verb is
//! told which one they wanted, and nothing here can run a Command as though it
//! gated something.
//!
//! # There is no shell here either
//!
//! `checks-runner` splits a `run` string into a program and its arguments and
//! spawns it directly, so a `run` that pipes or redirects does not work — for a
//! person exactly as for a Drone. That is a property worth discovering at a
//! terminal rather than in a Job.

use std::fmt;
use std::path::{Path, PathBuf};
use std::time::Duration;

use checks_runner::{
    resolve_width, Attempt, CheckSlots, CheckWidth, Priority, Writing, AHEAD_ENV, HELD_ENV,
};
use config::Manifest;
use verification::{Exit, NeverRan};

use crate::setup::MANIFEST;

/// Which registry a name was looked up in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Registry {
    /// Gates advancement. `armada check`.
    Checks,
    /// Gates nothing. `armada run`.
    Commands,
}

impl Registry {
    fn verb(self) -> &'static str {
        match self {
            Registry::Checks => "check",
            Registry::Commands => "run",
        }
    }

    fn noun(self) -> &'static str {
        match self {
            Registry::Checks => "Check",
            Registry::Commands => "Command",
        }
    }

    fn other(self) -> Registry {
        match self {
            Registry::Checks => Registry::Commands,
            Registry::Commands => Registry::Checks,
        }
    }
}

pub use crate::reaching::{Asked, Reached};

/// One declared thing, resolved and run in `root`.
///
/// Returns what the command did. There is no error return for a command that
/// failed — a failure is an [`Exit`], and the caller turns it into a status.
///
/// `slots` is the machine's: a Check takes its `places` of them for its
/// prerequisites and its run, and a Command takes none. `priority` is a
/// Check's and its prerequisites': a Command runs at normal priority.
pub async fn execute(
    root: &Path,
    registry: Registry,
    name: &str,
    asked: Asked<'_>,
    budget: Duration,
    slots: Option<&CheckSlots>,
    priority: Priority,
) -> Result<Ran, NotDeclared> {
    let test = match asked {
        Asked::OneTest(test) => Some(test),
        _ => None,
    };
    let manifest = Manifest::load(&root.join(MANIFEST)).map_err(|why| NotDeclared::NoManifest {
        path: root.join(MANIFEST),
        why: Box::new(why),
    })?;

    let (command, destructive) = match registry {
        Registry::Checks => (
            manifest.check(name).map(|check| check.run().to_string()),
            false,
        ),
        Registry::Commands => match manifest.command(name) {
            Some(found) => (Some(found.run().to_string()), found.is_destructive()),
            None => (None, false),
        },
    };
    let Some(command) = command else {
        return Err(unknown(&manifest, registry, name));
    };
    let command = match (test, manifest.check(name)) {
        (None, _) => command,
        (Some(test), check) => check
            .and_then(config::Check::one_test)
            .and_then(|one| checks_runner::one_test(one, test))
            .ok_or_else(|| NotDeclared::NoOneTest {
                name: name.to_string(),
                test: test.to_string(),
                path: manifest.path().to_path_buf(),
            })?,
    };
    let (command, narrowed) = match (asked, manifest.check(name)) {
        (Asked::Changed(changed), Some(check)) => match crate::reaching::reached(check, changed) {
            (_, Reached::Nothing) => return Ok(crate::reaching::ran_nothing(name)),
            (narrowed, reached) => (narrowed.unwrap_or(command), Some(reached)),
        },
        _ => (command, None),
    };
    // **`${width}` resolves here too, and to the same number a gate reaches.**
    // A Check a person runs is the Check a Drone is measured by, per this
    // module's own rule, and one left holding the literal `${width}` would be a
    // different command from the one the gate runs — it would not even start,
    // since there is no shell here to expand it.
    //
    // **The shipped Jobs bound, not a saved one.** There is no store open here
    // and nothing to ask; a person who saved a different bound gets the shipped
    // number at the terminal and the gate stays the authority. #1444.
    let width = CheckWidth::read(crate::serve::PROVISIONAL_CONCURRENCY.jobs())
        .narrowed_to(manifest.check(name).and_then(config::Check::width));
    let command = resolve_width(&command, width);

    // **A Check's prerequisites run here too**, for this module's own reason: a
    // Check a person runs is the Check a Drone is measured by. Skipping one
    // would pass where the gate fails, or fail where the gate passes, and
    // either way the terminal would be rehearsing a different question.
    //
    // There is no ledger, because there is one Check: `armada check` twice runs
    // the prerequisite twice, which is what a person typing it twice asked for.
    let requires = manifest
        .check(name)
        .map(config::Check::requires)
        .unwrap_or(&[]);
    let required = requires
        .iter()
        .map(|needed| needed.name().to_string())
        .collect();
    // Held until this returns, across the prerequisites too, as Fleet's gate
    // holds a place for them.
    let _held = match (registry, slots) {
        (Registry::Checks, Some(slots)) => {
            let places = manifest
                .check(name)
                .map_or(1, |check| check.places().get() as usize);
            // Said once, so a pause reads as a queue rather than a hang.
            let mut said = false;
            let waiting = |in_use: checks_runner::InUse| {
                if !std::mem::replace(&mut said, true) {
                    eprintln!("{in_use}");
                }
            };
            match slots.take(places, waiting).await {
                Ok(held) => Some(held),
                Err(why) => {
                    eprintln!("running without a Check slot, which could not be taken: {why}");
                    None
                }
            }
        }
        _ => None,
    };
    let env = [(HELD_ENV.to_string(), String::from("1"))];
    let (env, priority): (&[(String, String)], _) = match registry {
        Registry::Checks => (&env, priority),
        Registry::Commands => (&[], Priority::Normal),
    };

    if let Some(blocked) = first_unmet(requires, root, budget, width, env, priority).await {
        return Ok(Ran {
            name: name.to_string(),
            command,
            test: test.map(str::to_string),
            destructive,
            required,
            attempt: blocked,
            narrowed,
        });
    }

    let attempt = run_at(&command, root, budget, env, priority).await;
    Ok(Ran {
        name: name.to_string(),
        command,
        test: test.map(str::to_string),
        destructive,
        required,
        attempt,
        narrowed,
    })
}

/// The machine's Check slots, as many as `fleet::ChecksAtOnce`'s machine
/// number, beside `fleet.json` so every clone and worktree shares them.
///
/// **`None` inside a Check that already holds them**, for
/// [`checks_runner::HELD_ENV`]'s reason, and where `HOME` is unset.
pub fn machine_slots() -> Option<CheckSlots> {
    if checks_runner::already_held() {
        return None;
    }
    let runtime_file = fleet::runtime::machine_path().ok()?;
    let slots = CheckSlots::at(
        runtime_file.parent()?.join("check-slots"),
        crate::serve::provisional_checks_at_once().get(),
    );
    // The merge line sets it on its own Checks, and nothing else does.
    match std::env::var_os(AHEAD_ENV) {
        Some(_) => Some(slots.ahead()),
        None => Some(slots),
    }
}

/// [`machine_slots`], for Fleet: tried once at start, and a filesystem that
/// refuses them said here, since `fleet::places` has no stderr to say it on.
pub fn machine_slots_for_fleet() -> Option<CheckSlots> {
    let slots = machine_slots()?;
    match slots.try_take(1) {
        Ok(_) => Some(slots),
        Err(why) => {
            eprintln!("Fleet runs Checks without the machine's Check slots, which could not be taken: {why}");
            None
        }
    }
}

/// Every Check the Manifest in `root` would run on a change to `changed`, in
/// the order `armada.yml` writes them.
///
/// **One answer with a Job's gate**: each Check is asked through the same
/// [`config::Check::covers`], so `scripts/land` choosing what to rerun and
/// Fleet choosing what to skip cannot come apart. `docs/capabilities/merge-line.md`.
pub fn covering(root: &Path, changed: &[String]) -> Result<Vec<String>, NotDeclared> {
    let manifest = Manifest::load(&root.join(MANIFEST)).map_err(|why| NotDeclared::NoManifest {
        path: root.join(MANIFEST),
        why: Box::new(why),
    })?;
    Ok(manifest
        .checks_as_written()
        .iter()
        .filter(|name| {
            manifest
                .check(name)
                .is_some_and(|check| check.covers(changed))
        })
        .cloned()
        .collect())
}

/// Run a Check's prerequisites in order and answer with the first that failed.
///
/// `None` where every one of them exited zero, which includes a Check that
/// declares none. The [`Attempt`] handed back carries the **prerequisite's**
/// output under an exit that names it, so the line printed after says which
/// command a person has to go and fix.
async fn first_unmet(
    requires: &[core_model::Prerequisite],
    root: &Path,
    budget: Duration,
    width: CheckWidth,
    env: &[(String, String)],
    priority: Priority,
) -> Option<Attempt> {
    for needed in requires {
        let run = resolve_width(needed.run(), width);
        let attempt = run_at(&run, root, budget, env, priority).await;
        if attempt.exit != Exit::Code(0) {
            return Some(Attempt {
                exit: Exit::NeverRan(NeverRan::PrerequisiteFailed {
                    command: needed.name().to_string(),
                    run: needed.run().to_string(),
                    exit: Box::new(attempt.exit),
                }),
                output: attempt.output,
            });
        }
    }
    None
}

/// `command` in `root`, at `priority`, with nothing to stop it but `budget`.
async fn run_at(
    command: &str,
    root: &Path,
    budget: Duration,
    env: &[(String, String)],
    priority: Priority,
) -> Attempt {
    let stop = std::future::pending();
    checks_runner::run_until(command, root, budget, Writing::Nowhere, env, stop, priority).await
}

/// What running it produced, with everything a caller needs to report it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ran {
    pub name: String,
    /// The `run` string, verbatim as the Manifest wrote it.
    pub command: String,
    /// The one test asked for, where `command` is the Check's `one_test`.
    pub test: Option<String>,
    /// Whether the Manifest calls this destructive. **Said, never enforced** —
    /// the flag pauses a Drone, and the person typing this is already the one
    /// triggering it.
    pub destructive: bool,
    /// The Commands that ran first, in order. **Said because they may have
    /// written to the working tree**, and a person who was not told that reads
    /// the changed files as somebody else's.
    ///
    /// Empty on a Command, which declares no prerequisites, and on a Check that
    /// declares none.
    pub required: Vec<String>,
    pub attempt: Attempt,
    /// What a changed set came to. `None` where none was asked.
    pub narrowed: Option<Reached>,
}

impl Ran {
    /// The status to exit with: the command's own code where there was one.
    ///
    /// A signal, a timeout or a spawn that never happened have no code of their
    /// own, and `1` is what a shell would have shown for them anyway. A code
    /// outside a byte is reported as `1` rather than wrapped, because a wrapped
    /// `256` is `0`.
    pub fn status(&self) -> u8 {
        match self.attempt.exit {
            _ if self.matched_nothing() => 1,
            Exit::Code(0) => 0,
            Exit::Code(code) => u8::try_from(code).unwrap_or(1),
            _ => 1,
        }
    }

    /// Whether the one test asked for matched nothing. **Not a pass**, though
    /// both runners can exit zero on it. `checks_runner::one_test_ran`.
    pub fn matched_nothing(&self) -> bool {
        self.test.is_some()
            && checks_runner::one_test_ran(&self.attempt.exit, &self.attempt.output, 0)
                == checks_runner::OneTestRan::NoMatch
    }
}

/// How it ended, in a sentence, for the line printed after the output.
pub struct Ended<'a>(pub &'a Exit);

impl fmt::Display for Ended<'_> {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            Exit::Code(0) => out.write_str("passed"),
            Exit::Code(code) => write!(out, "failed, exit {code}"),
            Exit::Signalled { signal } => write!(out, "was ended by signal {signal}"),
            Exit::TimedOut { after } => {
                write!(
                    out,
                    "ran past its {}s budget and was killed",
                    after.as_secs()
                )
            }
            Exit::NeverRan(NeverRan::NothingToRun) => {
                out.write_str("declares a `run` with no program in it")
            }
            Exit::NeverRan(NeverRan::NoSuchCommand { program }) => {
                write!(
                    out,
                    "needs `{program}`, which is not on this machine's PATH"
                )
            }
            Exit::NeverRan(NeverRan::WorktreeGone { worktree }) => {
                write!(out, "could not run: {worktree} is not there")
            }
            Exit::NeverRan(NeverRan::NotSpawned { program, kind }) => {
                write!(out, "could not start `{program}`: {kind}")
            }
            Exit::NeverRan(NeverRan::PrerequisiteFailed { command, run, exit }) => {
                write!(
                    out,
                    "did not run: the Command `{command}` (`{run}`) it requires {}",
                    Ended(exit)
                )
            }
        }
    }
}

/// The name is not in the registry that was asked.
#[derive(Debug)]
pub enum NotDeclared {
    /// There is no `armada.yml` here, or it is one Armada will not have.
    NoManifest {
        path: PathBuf,
        why: Box<config::LoadError>,
    },
    /// The other registry has it. The person typed the wrong verb, and is told
    /// which one they wanted rather than which one they used.
    DeclaredAsTheOther {
        name: String,
        asked: Registry,
        path: PathBuf,
    },
    /// A test was named, and the Check declares no `one_test` to run it with,
    /// or the name is blank.
    NoOneTest {
        name: String,
        test: String,
        path: PathBuf,
    },
    /// Nothing declares it, and this is what is declared.
    NoSuchName {
        name: String,
        asked: Registry,
        path: PathBuf,
        declared: Vec<String>,
    },
}

fn unknown(manifest: &Manifest, asked: Registry, name: &str) -> NotDeclared {
    let elsewhere = match asked.other() {
        Registry::Checks => manifest.check(name).is_some(),
        Registry::Commands => manifest.command(name).is_some(),
    };
    if elsewhere {
        return NotDeclared::DeclaredAsTheOther {
            name: name.to_string(),
            asked,
            path: manifest.path().to_path_buf(),
        };
    }
    NotDeclared::NoSuchName {
        name: name.to_string(),
        asked,
        path: manifest.path().to_path_buf(),
        declared: match asked {
            Registry::Checks => manifest.check_names(),
            Registry::Commands => manifest.command_names(),
        },
    }
}

impl fmt::Display for NotDeclared {
    /// **A refusal names what *is* declared.** A message that only says the
    /// name was wrong sends the reader to open the file, which is the one thing
    /// the command could have saved them.
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NotDeclared::NoManifest { path, why } => {
                write!(out, "{} could not be read: {why}", path.display())
            }
            NotDeclared::DeclaredAsTheOther { name, asked, path } => write!(
                out,
                "`{name}` is declared by {} as a {}, not a {} — `armada {} {name}`",
                path.display(),
                asked.other().noun(),
                asked.noun(),
                asked.other().verb()
            ),
            NotDeclared::NoOneTest { name, test, path } => write!(
                out,
                "`{name}` in {} has no `one_test` that can run `{test}` — `armada check {name}` runs it whole",
                path.display()
            ),
            NotDeclared::NoSuchName {
                name,
                asked,
                path,
                declared,
            } if declared.is_empty() => write!(
                out,
                "{} declares no {}s at all, so there is no `{name}` to run",
                path.display(),
                asked.noun()
            ),
            NotDeclared::NoSuchName {
                name,
                asked,
                path,
                declared,
            } => write!(
                out,
                "`{name}` is not a {} {} declares — it declares {}",
                asked.noun(),
                path.display(),
                declared
                    .iter()
                    .map(|one| format!("`{one}`"))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        }
    }
}

impl std::error::Error for NotDeclared {}
