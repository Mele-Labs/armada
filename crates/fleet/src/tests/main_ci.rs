//! What Fleet learns about main's CI from the forge, and how often it asks.
//!
//! The forge is scripted: what the real commands print is `adapters`' to
//! assert. Under test here is what Fleet makes of an answer, what it keeps
//! across a restart, and which asks it does not make.

use adapter_traits::{CiRun, CiState, FromOutside, OpenPull};
use testkit::{FakeVcs, FakeWorkProduct};

use crate::daemon::Fleet;

pub(super) type Fixture = Fleet<testkit::FakeHarness, FakeVcs, FakeWorkProduct>;

pub(super) const ONE: &str = "1111111111111111111111111111111111111111";

/// The Manifest names a base, a Check, and the CI job that answers for it.
pub(super) const MANIFEST: &str = "version: 1\nid: 01FIXTUREMANIFEST\nbase: main\nchecks:\n  test:\n    run: cargo nextest run\n    ci_jobs: [ci]\n  lint:\n    run: cargo fmt --check\n";

pub(super) fn run(name: &str, handle: &str, state: CiState) -> CiRun {
    CiRun {
        name: FromOutside::verbatim(name),
        state,
        handle: FromOutside::verbatim(handle),
        log_url: Some(FromOutside::verbatim(format!(
            "https://forge.invalid/armada/actions/runs/9/job/{handle}"
        ))),
    }
}

pub(super) fn pull(number: u64, ci: Option<CiState>, failing: &[&str]) -> OpenPull {
    OpenPull {
        number,
        title: FromOutside::verbatim(format!("Change {number}")),
        branch: FromOutside::verbatim(format!("armada/{number}")),
        url: FromOutside::verbatim(format!("https://forge.invalid/armada/pull/{number}")),
        author: Some(FromOutside::verbatim("nick")),
        ci,
        head: None,
        failing: failing
            .iter()
            .map(|name| FromOutside::verbatim(*name))
            .collect(),
        auto_merge: false,
        facts: Default::default(),
    }
}


// -- why the forge is quiet

mod quiet_reason {
    use adapters::forge_tally::Failed;
    use core_model::Timestamp;
    use ipc::{Instant, QuietReason};

    use crate::main_hub::quiet_reason;

    fn now() -> Timestamp {
        Timestamp::from_rfc3339("2026-11-16T17:53:30.000Z")
    }

    fn limited(retry_after: Option<u64>, spent_until: Option<u64>) -> Option<Failed> {
        Some(Failed { rate_limited: true, retry_after, spent_until, at: "2026-11-16T17:53:02.000Z".into() })
    }

    #[test]
    fn a_retry_after_counts_from_when_github_refused() {
        let until = Some(Instant::carried("2026-11-16T17:54:02.000Z"));
        assert_eq!(quiet_reason(limited(Some(60), Some(1_795_369_200)), &now()), QuietReason::RateLimited { until });
    }

    #[test]
    fn a_spent_window_waits_for_its_reset() {
        // 1_795_369_200 s is 2026-11-22T17:40:00Z.
        let until = Some(Instant::carried("2026-11-22T17:40:00.000Z"));
        assert_eq!(quiet_reason(limited(None, Some(1_795_369_200)), &now()), QuietReason::RateLimited { until });
    }

    #[test]
    fn a_limit_that_named_no_time_is_rate_limited_with_no_until() {
        assert_eq!(quiet_reason(limited(None, None), &now()), QuietReason::RateLimited { until: None });
    }

    #[test]
    fn any_other_failure_or_none_counted_is_not_answering() {
        let network = Some(Failed { rate_limited: false, ..Default::default() });
        assert_eq!(quiet_reason(network, &now()), QuietReason::NotAnswering);
        assert_eq!(quiet_reason(None, &now()), QuietReason::NotAnswering);
    }
}
