//! `armada` — one binary, and its verbs.
//!
//! `serve` is the daemon; `check` and `run` execute one thing the repository's
//! Manifest declares; `covers` names the Checks a change hits; `clean` gives its worktrees, branches and Jobs back;
//! `mcp` relays an agent's session to the door Fleet serves; `worktree` leases
//! from the pool of warm worktrees. Each verb's own
//! module holds what it does and why.
//!
//! # What exits non-zero
//!
//! A genuine refusal — a command line that could not be read, a name the
//! Manifest does not declare, a Fleet that would not start, a clean that could
//! not finish. `check` and `run` carry the command's own exit code out, because
//! a person running a Check by hand wants the answer the Check gave.
//!
//! **`serve` against a live Fleet exits `0`** and names the pid. The state the
//! caller asked for is the state that already holds, and v1's `start` carried a
//! test by that name. `serve`'s module doc holds the whole argument, including
//! the one launchd-shaped rule that is deliberately not implemented yet.

use std::error::Error;
use std::path::PathBuf;
use std::process::ExitCode;

use adapters::UnmergedWork;
use armada::clean::Scope;
use armada::cli::{self, LandAct, Usage, Verb, WorktreeAct};
use armada::declared::{Asked, Registry};
use armada::serve::PROVISIONAL_CHECK_BUDGET;
use armada::{clean, declared, land, leasing, say, serve};

#[tokio::main]
async fn main() -> ExitCode {
    let asked = match cli::read(std::env::args().skip(1)) {
        Ok(asked) => asked,
        Err(misread) => {
            eprintln!("{misread}");
            return ExitCode::FAILURE;
        }
    };

    match asked {
        Verb::Help => {
            print!("{Usage}");
            ExitCode::SUCCESS
        }
        Verb::Serve { repository } => match serve::serve(repository).await {
            Ok(()) => ExitCode::SUCCESS,
            Err(why) => {
                chain("Fleet did not start", why.as_ref());
                ExitCode::FAILURE
            }
        },
        Verb::Check {
            name,
            test,
            changed,
        } => {
            let paths = match changed {
                true => match changed_on_stdin() {
                    Ok(paths) => Some(paths),
                    Err(code) => return code,
                },
                false => None,
            };
            let asked = match (&test, &paths) {
                (Some(test), _) => Asked::OneTest(test),
                (None, Some(paths)) => Asked::Changed(paths),
                (None, None) => Asked::Whole,
            };
            declared_by_the_manifest(Registry::Checks, &name, asked, "check").await
        }
        Verb::Run { name } => {
            declared_by_the_manifest(Registry::Commands, &name, Asked::Whole, "run").await
        }
        Verb::Covers => checks_this_change_hits(),
        Verb::Clean { everything, force } => clean_this_repository(everything, force),
        // **Blocking, on the runtime's own thread, and alone there.** This
        // verb is a pipe with one message in flight; nothing else is running
        // to be starved. `armada::mcp` holds the argument.
        Verb::Mcp => armada::mcp::speak(),
        Verb::Land(act) => land_verb(act),
        Verb::Worktree(act) => worktree_verb(act),
    }
}

fn worktree_verb(act: WorktreeAct) -> ExitCode {
    let Ok(cwd) = std::env::current_dir() else {
        eprintln!("the working directory could not be read");
        return ExitCode::FAILURE;
    };
    ExitCode::from(match act {
        WorktreeAct::Lease { branch, existing } => leasing::lease(&cwd, &branch, existing),
        WorktreeAct::Release { path } => leasing::release(&cwd, path),
        WorktreeAct::Status => leasing::status(&cwd),
        WorktreeAct::Add => leasing::reshape(&cwd, |pool| {
            pool.add()
                .map(|n| format!("slot-{n} added; the next lease makes it"))
        }),
        WorktreeAct::Remove { slot } => leasing::reshape(&cwd, |pool| {
            pool.remove(slot).map(|()| format!("slot-{slot} removed"))
        }),
        WorktreeAct::Close { slot } => leasing::reshape(&cwd, |pool| {
            pool.close(slot)
                .map(|()| format!("slot-{slot} closed; no lease takes it until it is opened"))
        }),
        WorktreeAct::Open { slot } => leasing::reshape(&cwd, |pool| {
            pool.open(slot).map(|()| format!("slot-{slot} open"))
        }),
    })
}

/// `armada land`'s four visible forms, plus the hidden `--runner` the
/// detached runner starts itself with. Every message here closely
/// paraphrases `scripts/land`'s own prints — this is a person-facing CLI,
/// not a wire contract — and every exit code matches
/// `docs/practices/running-locally.md`'s table.
fn land_verb(act: LandAct) -> ExitCode {
    let env = land::Env::read();
    if let LandAct::Runner { common_git_dir } = act {
        return land::run_runner(&common_git_dir, &env);
    }
    let cwd = match std::env::current_dir() {
        Ok(cwd) => cwd,
        Err(why) => {
            eprintln!("the working directory could not be read: {why}");
            return ExitCode::FAILURE;
        }
    };
    match act {
        LandAct::Runner { .. } => unreachable!("handled above"),
        LandAct::Preflight => match land::preflight(&cwd, &env) {
            Ok(done) => {
                println!(
                    "ready: {}, {}, tree {}",
                    done.branch,
                    done.pull_request
                        .map_or("no pull request".to_string(), |pr| format!(
                            "pull request #{pr}"
                        )),
                    short(&done.tree)
                );
                let checks = if done.checks.is_empty() {
                    "none".to_string()
                } else {
                    done.checks.join(", ")
                };
                println!("Checks this change hits: {checks}. The line runs each of them.");
                println!("Once the self-check passes: armada land");
                ExitCode::SUCCESS
            }
            Err(why) => refused(&why),
        },
        LandAct::Join => match land::land(&cwd, &env) {
            Ok(queued) => {
                let pr = queued
                    .pull_request
                    .map_or(String::new(), |pr| format!(", #{pr}"));
                println!("queued: {}{pr}, {} ahead", queued.branch, queued.ahead);
                println!("poll: armada land --status {}", queued.branch);
                ExitCode::SUCCESS
            }
            Err(why) => refused(&why),
        },
        LandAct::Status { branch } => match land::status(&cwd, branch.as_deref()) {
            Ok(code) => ExitCode::from(code),
            Err(why) => refused(&why),
        },
        LandAct::Withdraw { branch } => match land::withdraw(&cwd, branch.as_deref()) {
            Ok(land::Withdrawn::NotInLine(branch)) => {
                println!("{branch}: not in line, so nothing was withdrawn");
                ExitCode::SUCCESS
            }
            Ok(land::Withdrawn::Waiting(branch)) => {
                println!(
                    "withdrew {branch}: it was waiting, and nothing of it was gated or merged"
                );
                ExitCode::SUCCESS
            }
            Ok(land::Withdrawn::InTurn(branch)) => {
                println!(
                    "withdrew {branch}. It is in a turn now: a gate it is already in will still \
                     finish, and can land it; a gate it is waiting for will not take it"
                );
                ExitCode::SUCCESS
            }
            Err(why) => refused(&why),
        },
    }
}

fn refused(why: &land::Refused) -> ExitCode {
    eprintln!("land: {why}");
    ExitCode::from(1)
}

fn short(sha: &str) -> &str {
    sha.get(..10).unwrap_or(sha)
}

/// Run one Check or one Command in the repository the caller is standing in.
///
/// **The working directory, never a search upward.** `Setup::at` refuses to
/// adopt an ancestor's Manifest for the daemon, and a Check that quietly ran
/// under a different repository's declaration would be the same failure with
/// less warning.
async fn declared_by_the_manifest(
    registry: Registry,
    name: &str,
    asked: Asked<'_>,
    verb: &str,
) -> ExitCode {
    let root = match std::env::current_dir() {
        Ok(root) => root,
        Err(why) => {
            eprintln!("the working directory could not be read: {why}");
            return ExitCode::FAILURE;
        }
    };
    let slots = declared::machine_slots();
    match declared::execute(
        &root,
        registry,
        name,
        asked,
        PROVISIONAL_CHECK_BUDGET,
        slots.as_ref(),
        // Agent priority unless `ARMADA_CHECK_PRIORITY=normal`, which the
        // merge line sets on every Check it runs.
        checks_runner::Priority::from_env(),
    )
    .await
    {
        Ok(ran) => {
            say::ran(&ran, verb);
            ExitCode::from(ran.status())
        }
        Err(why) => {
            eprintln!("{why}");
            ExitCode::FAILURE
        }
    }
}

/// Print, one per line, every Check the Manifest in the working directory runs
/// on the paths read from stdin. Nothing printed is an answer: no Check applies.
fn checks_this_change_hits() -> ExitCode {
    let root = match std::env::current_dir() {
        Ok(root) => root,
        Err(why) => {
            eprintln!("the working directory could not be read: {why}");
            return ExitCode::FAILURE;
        }
    };
    let changed = match changed_on_stdin() {
        Ok(changed) => changed,
        Err(code) => return code,
    };
    match declared::covering(&root, &changed) {
        Ok(names) => {
            for name in names {
                println!("{name}");
            }
            ExitCode::SUCCESS
        }
        Err(why) => {
            eprintln!("{why}");
            ExitCode::FAILURE
        }
    }
}

/// The changed paths, one per line on stdin, for `covers` and `check --changed`.
fn changed_on_stdin() -> Result<Vec<String>, ExitCode> {
    let mut read = String::new();
    if let Err(why) = std::io::Read::read_to_string(&mut std::io::stdin(), &mut read) {
        eprintln!("the changed paths could not be read from stdin: {why}");
        return Err(ExitCode::FAILURE);
    }
    Ok(read
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect())
}

fn clean_this_repository(everything: bool, force: bool) -> ExitCode {
    let (Ok(root), Ok(runtime_file)) = (std::env::current_dir(), fleet::runtime::machine_path())
    else {
        eprintln!("the working directory and HOME are both needed, and one of them is not there");
        return ExitCode::FAILURE;
    };
    let machine: PathBuf = runtime_file
        .parent()
        .expect("the runtime file has a directory")
        .to_path_buf();

    let scope = match everything {
        true => Scope::AndTheMachine,
        false => Scope::Repository,
    };
    let unmerged = match force {
        true => UnmergedWork::Delete,
        false => UnmergedWork::Keep,
    };
    match clean::clean(&root, &machine, scope, unmerged) {
        Ok(cleaned) => {
            say::cleaned(&cleaned);
            // A kept branch is not a fault. A completed Job's branch is
            // unmerged by definition, so failing on one would make the
            // ordinary clean exit non-zero.
            match cleaned.faults.is_empty() {
                true => ExitCode::SUCCESS,
                // Every fault was already named on its own line. This is only
                // the status, so a script wrapping it can tell.
                false => ExitCode::FAILURE,
            }
        }
        Err(why) => {
            // **The reason leads and the outcome trails.** This read
            // `nothing was cleaned: {why}`, and the owner read the first
            // three words as "nothing is running" and stopped there — which
            // is the opposite of what it was saying. What a person can act on
            // goes first; that nothing happened is the least useful part and
            // is what a reader infers anyway from a command that refused.
            eprintln!("{why}\n\nNothing was cleaned.");
            ExitCode::FAILURE
        }
    }
}

/// The whole cause chain, outermost first.
///
/// A failure is carried as a chain rather than a sentence, and printing only
/// the outermost link would throw away the part that says what actually went
/// wrong — which is the entire reason the chain is not flattened until it
/// reaches the wire.
fn chain(what: &str, why: &dyn Error) {
    eprintln!("{what}: {why}");
    let mut cause = why.source();
    while let Some(link) = cause {
        eprintln!("  caused by: {link}");
        cause = link.source();
    }
}
