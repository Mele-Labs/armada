//! Selecting another crate beside `fleet` must not change what `fleet` is built
//! on, or the run compiles a second copy of it. `docs/practices/rust.md`,
//! *One build of fleet*.

use std::collections::{BTreeMap, BTreeSet};
use std::process::Command;

/// Each third-party crate the selection builds, with every feature set it is
/// built with: one crate can be built for the host and for the target.
fn features(selected: &[&str]) -> BTreeMap<String, BTreeSet<String>> {
    let mut command = Command::new(std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()));
    command
        .current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
        .args(["tree", "--locked", "-e", "normal,build,dev"])
        .args(["-f", "{p} [{f}]", "--prefix", "none"]);
    for crate_name in selected {
        command.args(["-p", crate_name]);
    }
    let output = command.output().expect("cargo runs");
    let printed = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "cargo tree failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let mut built: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for line in printed.lines().filter(|line| !line.contains("(/")) {
        let line = line.trim_end_matches(" (*)");
        if let Some((name, features)) = line.split_once(" [") {
            built
                .entry(name.to_string())
                .or_default()
                .insert(features.trim_end_matches(']').to_string());
        }
    }
    built
}

#[test]
fn selecting_api_armada_or_pocket_beside_fleet_builds_fleet_on_the_same_dependencies() {
    let fleet = features(&["fleet"]);
    assert!(fleet.keys().any(|name| name.starts_with("tokio v")));
    for selected in [&["api", "fleet"][..], &["armada"], &["pocket", "fleet"]] {
        let changed: Vec<String> = features(selected)
            .iter()
            .filter(|(name, built)| fleet.get(*name).is_some_and(|alone| alone != *built))
            .map(|(name, _)| name.clone())
            .collect();
        assert!(
            changed.is_empty(),
            "-p {selected:?} builds these with other features than -p fleet, so it compiles fleet \
             again: {changed:?}. Declare the features in fleet's [dev-dependencies]."
        );
    }
}
