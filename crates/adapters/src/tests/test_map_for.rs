//! Which collector each kind of Check gets, and which directory it is handed.

use std::path::Path;

use adapter_traits::{CollectorSource, LapCost};
use checks_runner::CheckWidth;
use core_model::{Narrowing, Runner};

use crate::test_map_for::{collector_for, Feeder};

fn under(directory: Option<&str>) -> Narrowing {
    Narrowing::declared(
        "cargo nextest run".to_string(),
        "-p {}".to_string(),
        None,
        directory.map(str::to_string),
        Vec::new(),
    )
}

fn fed(run: &str, runner: Option<&Runner>, narrowing: Option<&Narrowing>) -> Option<Feeder> {
    collector_for("check", run, runner, narrowing, Path::new("/repo"), CheckWidth::of(3))
}

#[test]
fn a_check_run_through_vitest_gets_the_import_reader_of_its_directory() {
    let runner = Runner::declared("vitest".to_string(), Some("apps/web".to_string()));
    let feeder = fed("pnpm test", Some(&runner), None).expect("vitest is read");
    assert_eq!(feeder.check(), "check");
    assert_eq!(feeder.collector().source(), CollectorSource::Static);
    assert_eq!(feeder.collector().lap_cost(), LapCost::Nothing);
    assert!(!feeder.collector().needs_rebuild());
    assert_eq!(feeder.dir_in("/checkout"), "apps/web", "the package, under the root it was built with");
}

#[test]
fn a_vitest_check_with_no_directory_reads_the_root() {
    let runner = Runner::declared("vitest".to_string(), None);
    let feeder = fed("pnpm test", Some(&runner), None).expect("vitest is read");
    assert_eq!(feeder.dir_in("/checkout"), ".");
}

#[test]
fn a_nextest_check_that_narrows_under_a_directory_gets_coverage_in_the_checkout() {
    let narrowing = under(Some("crates"));
    let feeder = fed("cargo nextest run --workspace", None, Some(&narrowing)).expect("coverage reads it");
    assert_eq!(feeder.collector().source(), CollectorSource::Measured);
    assert!(feeder.collector().needs_rebuild(), "it is told apart from the import reader by this");
    assert_eq!(feeder.dir_in("/checkout"), "/checkout", "coverage runs in the checkout itself");
}

#[test]
fn a_nextest_check_that_declares_no_under_is_not_fed() {
    assert!(fed("cargo nextest run", None, Some(&under(None))).is_none());
    assert!(fed("cargo nextest run", None, None).is_none());
}

#[test]
fn a_check_that_runs_neither_gets_none() {
    let narrowing = under(Some("crates"));
    assert!(fed("cargo clippy --workspace", None, Some(&narrowing)).is_none(), "narrowing alone is not nextest");
    assert!(fed("cargo test", None, Some(&narrowing)).is_none());
    let other = Runner::declared("pytest".to_string(), Some("py".to_string()));
    assert!(fed("pytest", Some(&other), None).is_none(), "a runner nobody reads");
    assert!(fed("pnpm test", None, None).is_none(), "naming no runner");
}

#[test]
fn a_nextest_token_inside_another_word_is_not_nextest() {
    let narrowing = under(Some("crates"));
    assert!(fed("echo nextest-is-great", None, Some(&narrowing)).is_none());
}
