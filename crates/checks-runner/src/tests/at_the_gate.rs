//! [`narrowed_at_the_gate`]: the stricter reading the merge line takes, where a
//! narrowed run that measured less than the whole run lands a red on `main`.

use core_model::{Covers, Narrowing, PathPattern};

use crate::narrow::{narrowed_at_the_gate, Narrowed};

fn paths(paths: &[&str]) -> Vec<String> {
    paths.iter().map(|path| path.to_string()).collect()
}

fn by_package(except: &[&str], from: Option<&[&str]>) -> Narrowing {
    Narrowing::declared(
        "cargo nextest run".to_string(),
        "-p {}".to_string(),
        from.and_then(|patterns| {
            Covers::of(
                patterns
                    .iter()
                    .map(|written| PathPattern::parse(written).expect("a pattern"))
                    .collect(),
            )
        }),
        Some("crates".to_string()),
        except.iter().map(|value| value.to_string()).collect(),
    )
}

#[test]
fn every_path_under_the_directory_narrows_to_its_packages() {
    assert_eq!(
        narrowed_at_the_gate(
            Some(&by_package(&[], None)),
            &paths(&[
                "crates/fleet/src/lib.rs",
                "crates/api",
                "crates/fleet/Cargo.toml"
            ])
        ),
        Narrowed::To("cargo nextest run -p api -p fleet".to_string())
    );
}

/// `xtask/` sits outside `crates`, and its tests read the whole tree: one path
/// the narrowing cannot name is a run it cannot vouch for.
#[test]
fn one_path_outside_the_directory_runs_it_whole() {
    for outside in [
        "xtask/src/main.rs",
        "Cargo.lock",
        "apps/desktop/src/a.ts",
        "crates",
    ] {
        assert_eq!(
            narrowed_at_the_gate(
                Some(&by_package(&[], None)),
                &paths(&["crates/fleet/src/lib.rs", outside])
            ),
            Narrowed::Whole,
            "{outside}"
        );
    }
}

/// `narrowed` drops a path `from` does not match; the gate cannot, because the
/// Check still reads it.
#[test]
fn a_path_the_narrowing_filters_out_runs_it_whole() {
    assert_eq!(
        narrowed_at_the_gate(
            Some(&by_package(&[], Some(&["**/*.rs"]))),
            &paths(&["crates/fleet/src/lib.rs", "crates/fleet/data.toml"])
        ),
        Narrowed::Whole
    );
}

/// The exclusion is the whole run's too, so a change reaching only it has
/// nothing the whole run would have measured.
#[test]
fn an_excluded_package_is_dropped_and_alone_is_nothing() {
    let narrowing = by_package(&["acceptance"], None);
    assert_eq!(
        narrowed_at_the_gate(
            Some(&narrowing),
            &paths(&["crates/acceptance/tests/a.rs", "crates/fleet/src/lib.rs"])
        ),
        Narrowed::To("cargo nextest run -p fleet".to_string())
    );
    assert_eq!(
        narrowed_at_the_gate(Some(&narrowing), &paths(&["crates/acceptance/tests/a.rs"])),
        Narrowed::Nothing
    );
}

/// A file list hands the command what changed and nothing it reads beside it —
/// `rustfmt.toml`, an edition — so the gate never narrows to one.
#[test]
fn a_verbatim_narrowing_and_none_both_run_whole() {
    let verbatim = Narrowing::declared(
        "rustfmt --check".to_string(),
        "{}".to_string(),
        None,
        None,
        Vec::new(),
    );
    let changed = paths(&["crates/fleet/src/lib.rs"]);
    assert_eq!(
        narrowed_at_the_gate(Some(&verbatim), &changed),
        Narrowed::Whole
    );
    assert_eq!(narrowed_at_the_gate(None, &changed), Narrowed::Whole);
}

#[test]
fn no_path_at_all_runs_it_whole() {
    assert_eq!(
        narrowed_at_the_gate(Some(&by_package(&[], None)), &[]),
        Narrowed::Whole
    );
}
