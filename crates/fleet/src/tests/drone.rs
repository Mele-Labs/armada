//! That a Drone gets what Fleet built and nothing it happened to be near.
//!
//! # These tests start a process, and it is never an agent
//!
//! Two of the properties here cannot be asserted on a value: whether the
//! operating system actually cleared the environment, and whether a write to a
//! dead child's pipe kills the writer. Both need a real child, so both use one
//! — a shell that prints something and reads a line. Nothing here starts an
//! agent CLI, and nothing here needs a credential, a network or a dollar.
//!
//! Everything else about a Drone's confinement is a rendering and is asserted
//! in `adapters`, with no process at all.
//!
//! # The planted value
//!
//! One test exports a value into this process's own environment and then asks
//! the child what it can see. That is the shape v1 could not have passed: v1's
//! Drone spawn layered its variables over the operator's, so a token exported
//! in a shell reached every Drone. The assertion is on the child's own reading,
//! not on what Fleet intended.

use adapter_traits::Environment;

use crate::drone::environment;
use crate::drone::HostPaths;

const PLANTED: &str = "ARMADA_TEST_PLANTED_CREDENTIAL";

#[test]
fn fleet_names_every_variable_a_drone_gets() {
    let built = environment(
        HostPaths {
            path: "/usr/bin:/bin",
            home: "/Users/user",
            user: "someone",
            width: checks_runner::CheckWidth::of(2),
        },
        &[],
    )
    .expect("a legal environment");

    // **Five, and `USER` is the fifth.** Without it the agent CLI answers its
    // first turn with `Not logged in`, measured against a live Drone: same
    // binary, same `HOME`, credentials readable from the Keychain either way,
    // `USER` the only difference. `LOGNAME` does not substitute. The sixth
    // holds the agent's own builds to a Check's width.
    assert_eq!(
        built.names(),
        vec!["PATH", "HOME", "LANG", "TERM", "USER", "CARGO_BUILD_JOBS"]
    );
    assert_eq!(
        built.vars().last(),
        Some(&("CARGO_BUILD_JOBS".to_string(), "2".to_string()))
    );
    assert!(
        !built.names().contains(&"SSH_AUTH_SOCK"),
        "no credential, and no agent to ask one of — the second half of a \
         Drone that cannot push"
    );
}

/// **The pool replaces the width, it does not sit beside it**: cargo ignores
/// `CARGO_BUILD_JOBS` once it inherits a jobserver, so naming both would read as
/// a ceiling that is not one. Offers a pool process-wide, so it is its own
/// process under `nextest`.
#[test]
fn a_drones_builds_take_the_machines_pool_instead_of_a_width() {
    checks_runner::offer("-j --jobserver-auth=fifo:/pool".to_string());
    let built = environment(
        HostPaths {
            path: "/usr/bin:/bin",
            home: "/Users/user",
            user: "someone",
            width: checks_runner::CheckWidth::of(2),
        },
        &[],
    )
    .expect("a legal environment");

    assert_eq!(
        built.names(),
        vec!["PATH", "HOME", "LANG", "TERM", "USER", "MAKEFLAGS"]
    );
    assert_eq!(
        built.vars().last(),
        Some(&(
            "MAKEFLAGS".to_string(),
            "-j --jobserver-auth=fifo:/pool".to_string()
        ))
    );
}

/// A declared port rides in beside the guaranteed five, so a Command a Drone
/// runs from its own shell sees `ARMADA_PORT_<NAME>` without Fleet touching
/// the command string at all.
#[test]
fn a_claimed_port_is_set_beside_the_guaranteed_five() {
    let built = environment(
        HostPaths {
            path: "/usr/bin:/bin",
            home: "/Users/user",
            user: "someone",
            width: checks_runner::CheckWidth::of(2),
        },
        &[("ARMADA_PORT_STORYBOOK".to_string(), "41000".to_string())],
    )
    .expect("a legal environment");
    assert!(built.names().contains(&"ARMADA_PORT_STORYBOOK"));
    assert_eq!(
        built.vars().last(),
        Some(&("ARMADA_PORT_STORYBOOK".to_string(), "41000".to_string()))
    );
}

#[test]
fn an_environment_starts_empty_and_there_is_no_way_to_inherit_one() {
    assert!(Environment::nothing().vars().is_empty());
}

#[test]
fn the_same_name_twice_is_refused_rather_than_resolved() {
    let twice = Environment::nothing()
        .and("PATH", "/usr/bin")
        .expect("a legal name")
        .and("PATH", "/opt/bin");
    assert!(twice.is_err(), "which one wins is not a rule to invent");
}

#[tokio::test]
async fn the_planted_value_is_really_in_this_process_s_environment() {
    // The control. Without it, the test above passes against a machine where
    // the plant never happened, proving nothing about clearing.
    std::env::set_var(PLANTED, "a-token-nobody-granted");
    let out = tokio::process::Command::new("/bin/sh")
        .args(["-c", "env"])
        .output()
        .await
        .expect("a shell runs");
    assert!(
        String::from_utf8_lossy(&out.stdout).contains(PLANTED),
        "an ordinary child inherits it — which is what the Drone must not do"
    );
}

// ------------------------------------------------- what a dead Drone leaves

