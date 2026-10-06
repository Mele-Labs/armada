//! One gate over a built candidate: `verify-foundations` read against the
//! base, then every Check the combination hits. `scripts/land`'s own `gate`.

use std::path::Path;
use std::time::Duration;

use checks_runner::{reached, Reach};
use fleet::clock::{Clock, SystemClock};

use super::armada_cli::{check, covers};
use super::batch::{tell, Built};
use super::caches::{base_foundations, checks_on_the_base};
use super::dir::StateDir;
use super::env::Env;
use super::gate::{foundations_delta, not_installed, FoundationsComparison};
use super::heads_up::Asked;
use super::outcome::{CheckRun, CheckState, OutcomePatch, OutcomeState};
use super::prepare::{nothing_left, setup};
use super::queue::QueueEntry;
use super::repo::remote_head_within;
use super::shell::spoken;
use super::stop::Stopped;

/// What `verify-foundations` said of a candidate, against the base's own run.
pub enum Read {
    /// No line the base lacks. A run that crashed is here too, so the Checks
    /// beside it still say what they can.
    Passed(Passed),
    /// Lines the base lacks: red whatever the Checks say, so none run.
    New(Vec<String>),
}

pub struct Passed {
    crashed: Option<String>,
    log: String,
}

/// Run `verify-foundations` on what [`super::batch::build`] built for
/// `group`, and read it against `base`.
pub fn foundations(
    repo: &Path,
    state: &StateDir,
    env: &Env,
    group: &[QueueEntry],
    base: &str,
    built: &Built,
    logs: &Path,
) -> Result<Read, Stopped> {
    tell(
        state,
        group,
        OutcomeState::Gating,
        format!("reading verify-foundations against {}", env.base),
        // A half of a split batch gates again: what the whole ran is not this run's.
        OutcomePatch {
            checks: Some(Vec::new()),
            own_failures: Some(Vec::new()),
            ..OutcomePatch::default()
        },
    )?;
    let base_output = base_foundations(repo, state, base, env, logs)?;
    let log = logs.join("foundations.log");
    let argv: Vec<&str> = env.foundations.iter().map(String::as_str).collect();
    let ran = super::shell::run(&argv, &built.worktree, None, Some(&log))?;
    let read = match foundations_delta(&base_output, &ran.combined(), ran.status_code()) {
        FoundationsComparison::New(new) if !new.is_empty() => Read::New(new),
        FoundationsComparison::New(_) => Read::Passed(Passed {
            crashed: None,
            log: path_string(&log),
        }),
        FoundationsComparison::Crashed(last) => Read::Passed(Passed {
            crashed: Some(format!(
                "`{}` exited {} naming no failing rule, so nothing was gated ({})",
                env.foundations.join(" "),
                ran.status_code(),
                last.join(" / "),
            )),
            log: path_string(&log),
        }),
    };
    nothing_left(&built.worktree, "verify-foundations")?;
    Ok(read)
}

/// The red for `lines` the base lacks, with `log` the run that printed them.
pub fn foundations_red(env: &Env, moved: bool, lines: Vec<String>, log: &Path) -> Stopped {
    Stopped::red(
        format!(
            "red {}: {} verify-foundations line(s) {} does not have, so no Check was run. \
             Nothing was pushed or merged.",
            against(env, moved),
            lines.len(),
            env.base
        ),
        OutcomePatch {
            new_lines: Some(lines),
            logs: Some(vec![path_string(log)]),
            ..OutcomePatch::default()
        },
    )
}

/// What one gate of the Checks came to, short of a stop.
pub enum Gated {
    /// Green, with each Check that ran narrowed and what to, said for the outcome.
    Green(String),
    /// The base moved from outside while the Checks ran, so the gate stopped at
    /// a Check boundary and the turn gates again against the new base.
    BaseMoved,
}

/// How long the look at the base at a Check boundary is given. A remote that
/// does not answer in this is read as a base that has not moved.
const LOOK: Duration = Duration::from_secs(3);

/// The base's new head where it is no longer `base`, which only a push from
/// outside this line moves while a turn gates. Written to `moved.log` too,
/// since the status line is overwritten by the next gate.
fn moved_from(repo: &Path, env: &Env, base: &str, logs: &Path, said: &str) -> Option<String> {
    let now = remote_head_within(repo, &env.remote, &env.base, LOOK).filter(|now| now != base)?;
    let _ = std::fs::write(
        logs.join("moved.log"),
        format!("{} moved from {base} to {now}: {said}\n", env.base),
    );
    Some(now)
}

/// Run every Check the candidate hits, once `verify-foundations` passed.
/// `Ok` is green or a base that moved under the gate; nothing is pushed here.
#[allow(clippy::too_many_arguments)]
pub fn checks(
    repo: &Path,
    state: &StateDir,
    env: &Env,
    group: &[QueueEntry],
    base: &str,
    built: &Built,
    logs: &Path,
    passed: Passed,
) -> Result<Gated, Stopped> {
    let (where_, moved, regenerated) = (&built.worktree, built.moved, built.regenerated);
    let rerun = covers(&env.armada, where_, &built.hit)?;
    let Passed { crashed, log } = passed;
    let mut log_paths = vec![log];

    let mut failed = Vec::new();
    let mut uninstalled = Vec::new();
    let mut timed_out = Vec::new();
    // Each Check as it stands, written with every word the turn says from here.
    let mut runs: Vec<CheckRun> = rerun
        .iter()
        .map(|name| CheckRun {
            name: name.clone(),
            state: CheckState::Waiting,
            started_at: None,
        })
        .collect();
    let mut narrowed = Vec::new();
    let mut reach = None;
    let mut asked = Asked::default();
    // Where the base moved, said as the status line says it, once it has.
    let moved_while = |now: &str, said: &str, left: &[String]| {
        let skipped = match left {
            [] => String::new(),
            _ => format!("; skipped {}", left.join(", ")),
        };
        format!(
            "{} moved to {}{said}{skipped}, so gating again against it",
            env.base,
            now.get(..10).unwrap_or(now)
        )
    };
    let moved_tell = |detail: String, runs: &[CheckRun], log_paths: &[String]| {
        tell(
            state,
            group,
            OutcomeState::Gating,
            detail,
            OutcomePatch {
                logs: Some(log_paths.to_vec()),
                checks: Some(runs.to_vec()),
                own_failures: Some(Vec::new()),
                ..OutcomePatch::default()
            },
        )
    };
    if !rerun.is_empty() {
        if let Some(now) = moved_from(repo, env, base, logs, "before the first Check") {
            moved_tell(
                moved_while(&now, " before the first Check", &rerun),
                &runs,
                &log_paths,
            )?;
            return Ok(Gated::BaseMoved);
        }
        setup(&where_, env, logs)?;
        nothing_left(&where_, "preparing the gate")?;
        // Logged because a whole run of a narrowable Check says nothing else.
        let (said, paths) = match reached(where_, &built.hit) {
            Reach::Paths(paths) => (
                format!("narrowed over\n{}\n", paths.join("\n")),
                Some(paths),
            ),
            Reach::Whole(why) => (format!("whole: {why}\n"), None),
        };
        let _ = std::fs::write(logs.join("reach.log"), said);
        reach = paths;
        nothing_left(&where_, "reading the workspace")?;
    }
    for (n, name) in rerun.iter().enumerate() {
        runs[n].state = CheckState::Running;
        runs[n].started_at = Some(SystemClock::new().now().as_str().to_string());
        tell(
            state,
            group,
            OutcomeState::Gating,
            format!(
                "running {name} ({}){}",
                rerun.join(", "),
                said_narrowed(&narrowed, ", with ")
            ),
            OutcomePatch {
                logs: Some(log_paths.clone()),
                checks: Some(runs.clone()),
                ..OutcomePatch::default()
            },
        )?;
        let log = logs.join(format!("{name}.log"));
        log_paths.push(path_string(&log));
        let ran = check(
            &env.armada,
            &where_,
            name,
            reach.as_deref(),
            &log,
            env.check_limit,
        )?;
        if let Some(to) = &ran.narrowed {
            narrowed.push(format!("{name} narrowed to {to}"));
        }
        // One that could not run is not a pass either; the stop says why.
        runs[n].state = match (ran.passed, ran.timed_out) {
            (true, _) => CheckState::Passed,
            (false, true) => CheckState::TimedOut,
            (false, false) => CheckState::Failed,
        };
        // The boundary: a Check that ran is never cut short, but the next one
        // does not start against a base that has moved.
        if let Some(now) = moved_from(repo, env, base, logs, &format!("while {name} ran")) {
            moved_tell(
                moved_while(&now, &format!(" while {name} ran"), &rerun[n + 1..]),
                &runs,
                &log_paths,
            )?;
            return Ok(Gated::BaseMoved);
        }
        if ran.passed {
            continue;
        }
        if ran.timed_out {
            timed_out.push(name.clone());
            failed.push(name.clone());
            continue;
        }
        match not_installed(&ran.output) {
            Some(missing) => uninstalled.push(format!("{name} ({missing})")),
            None => {
                failed.push(name.clone());
                // Said now, not at the end, while a later Check still runs;
                // the last one has the verdict right behind it.
                if n + 1 < rerun.len() {
                    asked.ask(repo, state, env, group, base, logs, name, &runs, &log_paths)?;
                }
            }
        }
    }
    nothing_left(&where_, "the Checks")?;

    let mut already = asked.already.clone();
    let mut base_timed_out = asked.timed_out.clone();
    let unasked: Vec<String> = failed
        .iter()
        .filter(|name| !asked.names.contains(name))
        .cloned()
        .collect();
    if !unasked.is_empty() {
        tell(
            state,
            group,
            OutcomeState::Gating,
            format!(
                "{} failed; asking whether {} fails them too",
                unasked.join(", "),
                env.base
            ),
            OutcomePatch {
                logs: Some(log_paths.clone()),
                checks: Some(runs.clone()),
                ..OutcomePatch::default()
            },
        )?;
        let on_base = checks_on_the_base(repo, state, base, &unasked, env, logs)?;
        already.extend(on_base.already);
        base_timed_out.extend(on_base.timed_out);
    }
    failed.retain(|name| !already.contains(name));
    for name in &already {
        log_paths.push(path_string(
            &logs.join(format!("{name}-on-{}.log", env.base)),
        ));
    }

    if !uninstalled.is_empty() {
        let mut detail = format!(
            "{} could not run: the command each names is not on this machine. Install it \
             and land again — nothing was pushed or merged",
            uninstalled.join(", ")
        );
        if !failed.is_empty() {
            detail.push_str(&format!(". {} failed beside it", failed.join(", ")));
        }
        return Err(Stopped::stopped(detail).with_patch(OutcomePatch {
            failed: Some(failed),
            logs: Some(log_paths),
            checks: Some(runs),
            ..OutcomePatch::default()
        }));
    }

    let killed = |names: &[String]| {
        let past: Vec<String> = names
            .iter()
            .filter(|name| timed_out.contains(*name))
            .cloned()
            .collect();
        (!past.is_empty()).then(|| {
            format!(
                "{} timed out after {} and was killed",
                past.join(", "),
                spoken(env.check_limit)
            )
        })
    };
    let theirs = (!already.is_empty()).then(|| {
        let (slow, red): (Vec<String>, Vec<String>) = already
            .iter()
            .cloned()
            .partition(|name| base_timed_out.contains(name));
        let mut said = Vec::new();
        if !red.is_empty() {
            said.push(format!(
                "{} already fails on {} itself",
                red.join(", "),
                env.base
            ));
        }
        if !slow.is_empty() {
            said.push(format!(
                "{} timed out on {} itself, past its limit of {}",
                slow.join(", "),
                env.base,
                spoken(env.check_limit)
            ));
        }
        // A timeout is not remembered against the base, so the next turn asks again.
        let next = if red.is_empty() {
            "land again".to_string()
        } else {
            format!("fix {} and land that first", env.base)
        };
        format!(
            "{}{}, so that much is not this branch's — {next}",
            killed(&already).map_or(String::new(), |said| format!("{said}, and ")),
            said.join(", and "),
        )
    });
    if !failed.is_empty() || crashed.is_some() {
        let mut parts = Vec::new();
        let red: Vec<String> = failed
            .iter()
            .filter(|name| !timed_out.contains(*name))
            .cloned()
            .collect();
        if !red.is_empty() {
            parts.push(format!("{} failed", red.join(", ")));
        }
        parts.extend(killed(&failed));
        if let Some(crashed) = &crashed {
            parts.push(crashed.clone());
        }
        if let Some(theirs) = &theirs {
            parts.push(theirs.clone());
        }
        parts.extend(narrowed.iter().cloned());
        return Err(Stopped::red(
            format!(
                "red {}: {}. Nothing was pushed or merged.",
                against(env, moved),
                parts.join(", ")
            ),
            OutcomePatch {
                failed: Some(failed),
                already: Some(already),
                logs: Some(log_paths),
                checks: Some(runs),
                ..OutcomePatch::default()
            },
        ));
    }
    if let Some(theirs) = theirs {
        return Err(
            Stopped::stopped(format!("{theirs}. Nothing was pushed or merged")).with_patch(
                OutcomePatch {
                    already: Some(already),
                    logs: Some(log_paths),
                    checks: Some(runs),
                    ..OutcomePatch::default()
                },
            ),
        );
    }

    tell(
        state,
        group,
        OutcomeState::Gating,
        format!(
            "{} passed{}{}",
            if rerun.is_empty() {
                "the gate".to_string()
            } else {
                format!("the gate and {}", rerun.join(", "))
            },
            match (moved, regenerated) {
                (false, false) => String::new(),
                (true, false) => format!(" with {} merged in", env.base),
                (false, true) => " with its generated files regenerated".to_string(),
                (true, true) => format!(
                    " with {} merged in and its generated files regenerated",
                    env.base
                ),
            },
            said_narrowed(&narrowed, ", "),
        ),
        OutcomePatch {
            logs: Some(log_paths),
            checks: Some(runs),
            ..OutcomePatch::default()
        },
    )?;
    Ok(Gated::Green(said_narrowed(&narrowed, ". Gated with ")))
}

/// Each Check that ran narrowed and what to, after `lead`; nothing where none did.
fn said_narrowed(narrowed: &[String], lead: &str) -> String {
    match narrowed.is_empty() {
        true => String::new(),
        false => format!("{lead}{}", narrowed.join(", ")),
    }
}

fn against(env: &Env, moved: bool) -> String {
    if moved {
        format!("with {} merged in", env.base)
    } else {
        format!("against {}", env.base)
    }
}

fn path_string(path: &Path) -> String {
    path.display().to_string()
}
