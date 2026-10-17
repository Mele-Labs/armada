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

