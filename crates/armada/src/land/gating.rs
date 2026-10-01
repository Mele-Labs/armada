//! One gate over a built candidate: `verify-foundations` read against the
//! base, and every Check the combination hits. `scripts/land`'s own `gate`.

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

/// Measure what [`super::batch::build`] built for `group` against `base`.
/// `Ok` is green; nothing is pushed here.
pub fn gate(
    repo: &Path,
    state: &StateDir,
    env: &Env,
    group: &[QueueEntry],
    base: &str,
    built: &Built,
    logs: &Path,
) -> Result<(), Stopped> {
    let (where_, moved, regenerated) = (&built.worktree, built.moved, built.regenerated);
    let rerun = covers(&env.armada, where_, &built.hit)?;

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
    let ran = super::shell::run(&argv, &where_, None, Some(&log))?;
    let (new_lines, crashed) =
        match foundations_delta(&base_output, &ran.combined(), ran.status_code()) {
            FoundationsComparison::New(new) => (new, None),
            FoundationsComparison::Crashed(last) => (
                Vec::new(),
                Some(format!(
                    "`{}` exited {} naming no failing rule, so nothing was gated ({})",
                    env.foundations.join(" "),
                    ran.status_code(),
                    last.join(" / "),
                )),
            ),
        };
    nothing_left(&where_, "verify-foundations")?;
    let mut log_paths = vec![path_string(&log)];

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
        already = checks_on_the_base(repo, state, base, &failed, env, logs)?;
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
        if !new_lines.is_empty() {
            detail.push_str(&format!(
                ", and {} verify-foundations line(s) {} does not have",
                new_lines.len(),
                env.base
            ));
        }
        return Err(Stopped::stopped(detail).with_patch(OutcomePatch {
            failed: Some(failed),
            new_lines: Some(new_lines),
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
        format!(
            "{}{} already fails on {} itself, so that much is not this branch's — fix {} and land that first",
            killed(&already).map_or(String::new(), |said| format!("{said}, and ")),
            already.join(", "),
            env.base,
            env.base
        )
    });
    if !failed.is_empty() || !new_lines.is_empty() || crashed.is_some() {
        let against = if moved {
            format!("with {} merged in", env.base)
        } else {
            format!("against {}", env.base)
        };
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
        if !new_lines.is_empty() {
            parts.push(format!(
                "{} verify-foundations line(s) {} does not have",
                new_lines.len(),
                env.base
            ));
        }
        if let Some(theirs) = &theirs {
            parts.push(theirs.clone());
        }
        return Err(Stopped::red(
            format!(
                "red {against}: {}. Nothing was pushed or merged.",
                parts.join(", ")
            ),
            OutcomePatch {
                failed: Some(failed),
                already: Some(already),
                new_lines: Some(new_lines),
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

fn path_string(path: &Path) -> String {
    path.display().to_string()
}
