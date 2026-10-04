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

/// This repository's `test`: `run` carries `-p xtask`, whose tests are the only
/// ones reading `apps/` and `packages/`. Owner, 4 Oct 2026, after Job 3.
fn reading_outside() -> Narrowing {
    Narrowing::declared(
        "cargo nextest run -p xtask".to_string(),
        "-p {}".to_string(),
        None,
        Some("crates".to_string()),
        vec!["acceptance".to_string()],
    )
    .with_outside(Covers::of(vec![
        PathPattern::parse("apps/**").expect("a pattern"),
        PathPattern::parse("packages/**").expect("a pattern"),
    ]))
}

/// Job 3's case: a Bridge-only change ran every Rust test after a cold compile,
/// when the only tests reading what it touched were `xtask`'s.
#[test]
fn a_change_only_outside_runs_the_narrowed_command_alone() {
    for changed in [
        paths(&["apps/x.ts"]),
        paths(&["apps/x.ts", "packages/screens/src/a.ts"]),
        // An excluded package adds nothing either, and the outside path still
        // asks for `run` — not the skip it would be on its own.
        paths(&["apps/x.ts", "crates/acceptance/tests/a.rs"]),
    ] {
        assert_eq!(
            narrowed_at_the_gate(Some(&reading_outside()), &changed),
            Narrowed::To("cargo nextest run -p xtask".to_string()),
            "{changed:?}"
        );
    }
}

/// `reached` has already widened `crates/fleet` to its dependents, so `api`
/// arrives here as a path of its own.
#[test]
fn a_path_outside_beside_a_crate_adds_nothing_to_its_packages() {
    assert_eq!(
        narrowed_at_the_gate(
            Some(&reading_outside()),
            &paths(&["apps/x.ts", "crates/fleet/src/lib.rs", "crates/api"])
        ),
        Narrowed::To("cargo nextest run -p xtask -p api -p fleet".to_string())
    );
}

/// `outside` names what `run` reads; everything else the strict reading
/// cannot name still runs whole.
#[test]
fn a_covered_path_neither_under_nor_outside_still_runs_it_whole() {
    for neither in [
        "xtask/src/main.rs",
        "docs/spikes/a.md",
        ".config/nextest.toml",
    ] {
        assert_eq!(
            narrowed_at_the_gate(
                Some(&reading_outside()),
                &paths(&["apps/x.ts", "crates/fleet/src/lib.rs", neither])
            ),
            Narrowed::Whole,
            "{neither}"
        );
    }
}
