//! Whether a gate's red is the work's or the machine's, asked before it is
//! ruled against the step: each failing test alone by `one_test`, then the
//! whole Check alone, which is what the step is ruled on.
//!
//! **Said on the step's own rows**, through the gate's own writer: while a red
//! runs again its row reads running with its live log, and ends at what the
//! run alone came to — the red again where the red stands.
//! `docs/concepts/manifest.md`, Confirming a red, holds the rules and Job 3.

use std::collections::BTreeMap;
use std::num::NonZeroU32;
use std::path::Path;
use std::time::Duration;

use checks_runner::OneTestRan;
use core_model::{Attempt, ResolvedCheck, RunsAt};
use verification::{Exit, Observed};

use crate::checking::{self, Stop};
use crate::gate::CheckOutput;
use crate::places::Room;
use crate::underway::Announcing;

/// The most failing tests on one Check run one at a time; past it, only the
/// whole run alone. A cost bound: each one-test run of a browser suite still
/// starts the browser and collects every file.
pub const ONE_BY_ONE: usize = 5;

/// A red Check the gate ran again alone, carried on its [`CheckOutput`],
/// whose `output` is then the run alone's.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Confirmed {
    /// Every test the red run named as failing, in the order it printed them.
    pub failing: Vec<String>,
    /// Whether each was run alone first and passed: `false` past [`ONE_BY_ONE`].
    pub one_by_one: bool,
}

/// What a run here needs that the gate already holds.
pub(crate) struct Again<'a> {
    pub touched: &'a [String],
    pub worktree: &'a Path,
    pub budget: Duration,
    pub room: &'a Room,
    pub ports: &'a BTreeMap<String, u16>,
    pub env: &'a [(String, String)],
    pub holding_handoff: bool,
    pub attempt: Attempt,
    /// The gate's own writer: each run here is said on the step's own rows,
    /// so a red being run again reads running rather than red.
    pub announcing: &'a Announcing,
}

/// One red Check this can confirm, the tests it named, and its `one_test`.
struct Red {
    at: usize,
    failing: Vec<String>,
    template: String,
}

/// Confirm the step's reds where every one can be, writing each run alone over
/// what it replaces. Both lists are left as they were otherwise.
pub(crate) async fn confirmed(
    checks: &[ResolvedCheck],
    observed: &mut [Observed],
    output: &mut Vec<CheckOutput>,
    again: Again<'_>,
) {
    let Some(reds) = confirmable(checks, observed, output) else {
        return;
    };
    let mut told: Vec<Announcing> = Vec::new();
    for red in reds.iter().filter(|red| red.failing.len() <= ONE_BY_ONE) {
        let telling = again.announcing.again_on(vec![red.at], false);
        for test in &red.failing {
            // A failure, a name that matched nothing, or no run: the red stands.
            if one_alone(&checks[red.at], &red.template, test, &again, &telling).await
                != Some(OneTestRan::Passed)
            {
                for row in told.iter().chain([&telling]) {
                    row.red_stands();
                }
                return;
            }
        }
        told.push(telling);
    }
    // Handoff's Checks the red held back run too, or the step would pass unasked.
    let held_back = observed
        .iter()
        .enumerate()
        .filter(|(_, seen)| matches!(seen, Observed::HeldBack))
        .map(|(at, _)| at);
    let mut rerun: Vec<usize> = reds.iter().map(|red| red.at).chain(held_back).collect();
    rerun.sort_unstable();
    let alone: Vec<ResolvedCheck> = rerun
        .iter()
        .filter_map(|at| holding_every_place(&checks[*at], None))
        .collect();
    let done = checking::ran(
        &alone,
        again.touched,
        false,
        false,
        again.worktree,
        again.budget,
        again.room,
        &again.announcing.again_on(rerun.clone(), true),
        again.ports,
        again.env,
        None,
        &Stop::never().holding_handoff(again.holding_handoff),
        None,
        again.attempt,
        None,
    )
    .await;
    for (at, done) in rerun.into_iter().zip(done) {
        observed[at] = done.observed;
        let Some((check, printed)) = done.printed else {
            continue;
        };
        let alone = reds.iter().find(|red| red.at == at).map(|red| Confirmed {
            failing: red.failing.clone(),
            one_by_one: red.failing.len() <= ONE_BY_ONE,
        });
        let kept = CheckOutput {
            check,
            output: printed,
            alone,
        };
        match output.iter_mut().find(|had| had.check == kept.check) {
            Some(had) => *had = kept,
            None => output.push(kept),
        }
    }
}

/// Every red Check, where each one exited with a code, declares `one_test`
/// and names its failing tests. `None` where there is no red or any other:
/// that step fails whatever confirming the rest would find.
fn confirmable(
    checks: &[ResolvedCheck],
    observed: &[Observed],
    output: &[CheckOutput],
) -> Option<Vec<Red>> {
    let mut reds = Vec::new();
    for (at, (check, seen)) in checks.iter().zip(observed).enumerate() {
        if checking::advances(check, seen) {
            continue;
        }
        let Observed::Command(Exit::Code(_)) = seen else {
            return None;
        };
        let ResolvedCheck::ManifestCheck {
            name,
            one_test: Some(template),
            ..
        } = check
        else {
            return None;
        };
        let printed = output.iter().find(|kept| &kept.check == name)?;
        // The reader `crate::fixing::repeated` compares two attempts with.
        let failing = checks_runner::failing_tests(&printed.output);
        if failing.is_empty() {
            return None;
        }
        reds.push(Red {
            at,
            failing,
            template: template.clone(),
        });
    }
    (!reds.is_empty()).then_some(reds)
}

/// One failing test, run alone by the Check's `one_test` and read as
/// `crate::fixing` reads it on main. `None` where it could not run at all.
async fn one_alone(
    check: &ResolvedCheck,
    template: &str,
    test: &str,
    again: &Again<'_>,
    telling: &Announcing,
) -> Option<OneTestRan> {
    let ResolvedCheck::ManifestCheck {
        expect_exit_code, ..
    } = check
    else {
        return None;
    };
    let one = holding_every_place(check, Some(checks_runner::one_test(template, test)?))?;
    let done = checking::ran(
        std::slice::from_ref(&one),
        &[],
        false,
        false,
        again.worktree,
        again.budget,
        again.room,
        telling,
        again.ports,
        again.env,
        None,
        &Stop::never(),
        None,
        again.attempt,
        None,
    )
    .await
    .into_iter()
    .next()?;
    match (&done.observed, &done.printed) {
        (Observed::Command(Exit::NeverRan(_)), _) => None,
        (Observed::Command(exit), Some((_, printed))) => Some(checks_runner::one_test_ran(
            exit,
            printed,
            *expect_exit_code,
        )),
        _ => None,
    }
}

/// The Check as it runs alone, asking for every place — `crate::places`
/// clamps that to the limit in force. `run` replaces its command for one test.
/// No `requires`: the gate's own run met them in this worktree already.
pub(crate) fn holding_every_place(
    check: &ResolvedCheck,
    run: Option<String>,
) -> Option<ResolvedCheck> {
    let ResolvedCheck::ManifestCheck {
        name,
        run: whole,
        expect_exit_code,
        one_test,
        runs_at,
        width,
        ..
    } = check
    else {
        return None;
    };
    let one = run.is_some();
    Some(ResolvedCheck::ManifestCheck {
        name: name.clone(),
        run: run.unwrap_or_else(|| whole.clone()),
        expect_exit_code: *expect_exit_code,
        when: None,
        requires: Vec::new(),
        narrow: None,
        one_test: if one { None } else { one_test.clone() },
        runs_at: if one { RunsAt::Everywhere } else { *runs_at },
        places: NonZeroU32::MAX,
        width: *width,
        runner: None,
    })
}

impl<H, V, W> crate::daemon::Fleet<H, V, W>
where
    H: adapter_traits::AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: adapter_traits::Vcs + adapter_traits::Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: adapter_traits::WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    /// Write each red this ruling ran again alone into the Job's log, under
    /// its step: a person reads it there, and a retro reads it as Fleet's
    /// friction, since the attempt it cost the machine is not the Drone's.
    pub(crate) fn noted_alone(
        &self,
        job: &core_model::JobId,
        step: &core_model::StepId,
        attempt: Attempt,
        ruling: &crate::gate::Ruling,
    ) {
        use core_model::{Component, Envelope, FieldValue, Level};
        for kept in ruling.output() {
            let Some(alone) = &kept.alone else {
                continue;
            };
            let passed = ruling
                .checks()
                .iter()
                .find(|row| row.name == kept.check)
                .is_some_and(|row| row.outcome.advances());
            let envelope = Envelope::new(
                self.now(),
                Level::Warn,
                Component::Fleet,
                self.run().clone(),
                crate::retro::lines::A_RED_RUN_ALONE,
            )
            .in_job(job.as_ulid().clone())
            .at_step(step.as_str())
            .with_field("check", FieldValue::Str(kept.check.clone()))
            .with_field("attempt", FieldValue::Int(i64::from(attempt.number())))
            .with_field("failed", FieldValue::Int(alone.failing.len() as i64))
            .with_field("one_by_one", FieldValue::Bool(alone.one_by_one))
            .with_field("passed_alone", FieldValue::Bool(passed))
            .with_field("tests", FieldValue::Str(alone.failing.join(" | ")));
            self.noted_in_the_log(job, &envelope);
        }
    }
}
