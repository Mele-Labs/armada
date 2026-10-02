//! One gate over a built candidate: `verify-foundations` read against the
//! base, then every Check the combination hits. `scripts/land`'s own `gate`.

use std::path::Path;

use super::armada_cli::{check, covers};
use super::batch::{tell, Built};
use super::caches::{base_foundations, checks_on_the_base};
use super::dir::StateDir;
use super::env::Env;
use super::gate::{foundations_delta, not_installed, FoundationsComparison};
use super::outcome::{OutcomePatch, OutcomeState};
use super::prepare::{nothing_left, setup};
use super::queue::QueueEntry;
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
        OutcomePatch::default(),
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

/// Run every Check the candidate hits, once `verify-foundations` passed.
/// `Ok` is green; nothing is pushed here.
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
) -> Result<(), Stopped> {
    let (where_, moved, regenerated) = (&built.worktree, built.moved, built.regenerated);
    let rerun = covers(&env.armada, where_, &built.hit)?;
    let Passed { crashed, log } = passed;
    let mut log_paths = vec![log];

    let mut failed = Vec::new();
    let mut uninstalled = Vec::new();
    let mut timed_out = Vec::new();
    if !rerun.is_empty() {
        setup(&where_, env, logs)?;
        nothing_left(&where_, "preparing the gate")?;
    }
    for name in &rerun {
        tell(
            state,
            group,
            OutcomeState::Gating,
            format!("running {name} ({})", rerun.join(", ")),
            OutcomePatch {
                logs: Some(log_paths.clone()),
                ..OutcomePatch::default()
            },
        )?;
        let log = logs.join(format!("{name}.log"));
        log_paths.push(path_string(&log));
        let ran = check(&env.armada, &where_, name, &log, env.check_limit)?;
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
            None => failed.push(name.clone()),
        }
    }
    nothing_left(&where_, "the Checks")?;

    let mut already = Vec::new();
    let mut base_timed_out = Vec::new();
    if !failed.is_empty() {
        tell(
            state,
            group,
            OutcomeState::Gating,
            format!(
                "{} failed; asking whether {} fails them too",
                failed.join(", "),
                env.base
            ),
            OutcomePatch {
                logs: Some(log_paths.clone()),
                ..OutcomePatch::default()
            },
        )?;
        let on_base = checks_on_the_base(repo, state, base, &failed, env, logs)?;
        (already, base_timed_out) = (on_base.already, on_base.timed_out);
        failed.retain(|name| !already.contains(name));
        for name in &already {
            log_paths.push(path_string(
                &logs.join(format!("{name}-on-{}.log", env.base)),
            ));
        }
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
            "{} passed{}",
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
        ),
        OutcomePatch {
            logs: Some(log_paths),
            ..OutcomePatch::default()
        },
    )?;
    Ok(())
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
