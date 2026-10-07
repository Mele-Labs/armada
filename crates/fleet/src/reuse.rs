//! What a Drone's asked run found, kept so the gate can trust it instead of
//! asking the same question of the same worktree twice. `#1014`.
//!
//! **In the slot, not the store.** An asked run is true of the worktree at the
//! instant it ran, so a row surviving a restart would be a result nothing
//! could tell from a stale one — `KeptAskedRun` dies with the `Working` that
//! holds it, which makes that true by construction.
//!
//! **A failed, skipped or narrowed Check is dropped at [`KeptAskedRun::of`],
//! before [`trusted`] is ever asked.** A decision made once, folding the asked
//! run down, cannot be forgotten a second time the way a check repeated at
//! every call site could be.
//!
//! **Looked up by name, never by position.** `crate::checking::ran` asks
//! [`KeptAskedRun::passed`] once per declared Check, inside the one loop that
//! already walks `checks` — there is no second sequence for that loop to
//! disagree in length with, which is what a `zip` over two independent
//! slices could not promise. `#1014`'s own review is why this shape replaced
//! one that paired two `Vec`s by position.

use adapter_traits::Footprint;
use core_model::{Attempt, StepCheck, Timestamp};

/// One asked run's Checks that a later gate, of the same step, the same
/// attempt and this same process, may answer from instead of running again.
///
/// **Public only because [`crate::rule_on`] is** — every field stays private,
/// and nothing outside this module builds or reads one.
#[derive(Clone, Debug)]
pub struct KeptAskedRun {
    attempt: Attempt,
    footprint: Footprint,
    /// Only the Checks eligible to be reused, never the whole asked run's rows.
    /// Each already carries its own `reused_from_asked_run` stamp — see `of`.
    checks: Vec<StepCheck>,
}

impl KeptAskedRun {
    /// Fold a asked run's report down to what a gate may ever trust of it.
    /// `checks` and `narrowed_to` are the asked run's own, aligned by position.
    pub(crate) fn of(
        attempt: Attempt,
        at: Timestamp,
        footprint: Footprint,
        checks: Vec<StepCheck>,
        narrowed_to: &[Option<String>],
    ) -> KeptAskedRun {
        let checks = checks
            .into_iter()
            .zip(narrowed_to)
            .filter(|(check, narrowed)| check.outcome.passed() && narrowed.is_none())
            .map(|(check, _)| StepCheck {
                reused_from_asked_run: Some(at.clone()),
                ..check
            })
            .collect();
        KeptAskedRun {
            attempt,
            footprint,
            checks,
        }
    }

    /// The row this asked run kept for a Check named `name`, where it passed
    /// one whole. `None` is "this asked run never answered that name" — every
    /// caller's cue to run the Check rather than guess.
    pub(crate) fn passed(&self, name: &str) -> Option<StepCheck> {
        self.checks.iter().find(|row| row.name == name).cloned()
    }
}

/// Which asked run, if any, a gate may draw on right now: the same attempt,
/// over a worktree that reads the same as it did when the asked run measured
/// it. `None` is "trust nothing" — there is no asked run, a different attempt,
/// or the worktree has moved since.
///
/// **Not indexed by Check.** [`KeptAskedRun::passed`] is what answers for one
/// Check, by name, inside `crate::checking::ran`'s own loop over `checks` —
/// this only decides whether the asked run as a whole is still good for
/// anything.
pub(crate) fn trusted<'a>(
    asked_run: Option<&'a KeptAskedRun>,
    attempt: Attempt,
    footprint_now: Option<&Footprint>,
) -> Option<&'a KeptAskedRun> {
    let kept = asked_run?;
    let now = footprint_now?;
    (kept.attempt == attempt && !now.differs_from(&kept.footprint)).then_some(kept)
}

#[cfg(test)]
mod tests {
    use core_model::CheckOutcome;

    use super::*;

    fn row(name: &str, outcome: CheckOutcome) -> StepCheck {
        StepCheck {
            name: name.to_string(),
            outcome,
            expected: None,
            produced: None,
            output_path: None,
            reused_from_asked_run: None,
        }
    }

    fn at(seconds: &str) -> Timestamp {
        Timestamp::from_rfc3339(format!("2026-09-13T00:00:{seconds}Z"))
    }

    #[test]
    fn a_failed_or_narrowed_check_is_dropped_and_a_passed_whole_one_is_kept() {
        let kept = KeptAskedRun::of(
            Attempt::FIRST,
            at("00"),
            Footprint::nothing(),
            vec![
                row("build", CheckOutcome::Passed),
                row("test", CheckOutcome::Failed),
                row("lint", CheckOutcome::Passed),
                row("fmt", CheckOutcome::Skipped),
            ],
            &[None, None, Some("cargo lint -p fleet".to_string()), None],
        );
        let names: Vec<&str> = kept
            .checks
            .iter()
            .map(|check| check.name.as_str())
            .collect();
        assert_eq!(names, vec!["build"]);
        assert_eq!(kept.checks[0].reused_from_asked_run, Some(at("00")));
    }

    #[test]
    fn passed_answers_only_the_name_it_kept_and_trusted_answers_only_the_matching_asked_run() {
        let kept = KeptAskedRun::of(
            Attempt::FIRST,
            at("00"),
            Footprint::nothing(),
            vec![row("build", CheckOutcome::Passed)],
            &[None],
        );
        assert!(kept.passed("test").is_none());
        assert!(kept.passed("build").is_some());

        assert!(trusted(Some(&kept), Attempt::FIRST, Some(&Footprint::nothing())).is_some());
        assert!(trusted(None, Attempt::FIRST, Some(&Footprint::nothing())).is_none());
        assert!(trusted(Some(&kept), Attempt::FIRST, None).is_none());
        assert!(trusted(
            Some(&kept),
            Attempt::stored(2).expect("a second attempt"),
            Some(&Footprint::nothing())
        )
        .is_none());
    }
}
