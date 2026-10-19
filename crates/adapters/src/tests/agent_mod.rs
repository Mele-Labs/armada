//! Installing the mod: a person's request is the only way in, and the CLI's own words come back.
//! The CLI here is a script written to the scratch folder; the real one never runs.

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::time::Duration;

use super::scratch::Scratch;
use crate::agent_mod::{can_populate_from, install, is_populated, populate, Asked, Installed, Step, PLUGIN};

const PLENTY: Duration = Duration::from_secs(30);
const MOD_FOLDER: &str = "/support/mod";

fn script(scratch: &Scratch, name: &str, body: &str) -> PathBuf {
    let path = scratch.file(name, &format!("#!/bin/sh\n{body}\n"));
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).expect("executable");
    path
}

fn install_with(program: &Path, scratch: &Scratch) -> Installed {
    let asked = Asked::when(true).expect("asked");
    install(asked, program.as_os_str(), "/usr/bin:/bin", scratch.path(), Path::new(MOD_FOLDER), PLENTY)
}

#[test]
fn nothing_is_installed_for_a_request_that_was_not_made() {
    assert!(Asked::when(false).is_none());
    assert!(Asked::when(true).is_some());
}

#[test]
fn the_two_commands_run_in_order_on_the_mod_folder() {
    let scratch = Scratch::new();
    let log = scratch.path().join("log");
    let cli = script(&scratch, "cli", &format!("echo \"$@\" >> {}\necho done $2", log.display()));

    let installed = install_with(&cli, &scratch);

    assert_eq!(installed, Installed::Done { said: "done install".into() });
    assert_eq!(
        std::fs::read_to_string(log).expect("the log"),
        format!("plugin marketplace add {MOD_FOLDER}\nplugin install {PLUGIN}\n")
    );
}

#[test]
fn a_marketplace_already_added_does_not_stop_the_install() {
    let scratch = Scratch::new();
    let cli = script(
        &scratch,
        "cli",
        "if [ \"$2\" = marketplace ]; then echo already added >&2; exit 1; fi\necho installed",
    );

    assert_eq!(install_with(&cli, &scratch), Installed::Done { said: "installed".into() });
}

#[test]
fn a_failed_install_says_what_the_cli_said() {
    let scratch = Scratch::new();
    let cli = script(&scratch, "cli", "if [ \"$2\" = install ]; then echo no such plugin >&2; exit 1; fi");

    assert_eq!(
        install_with(&cli, &scratch),
        Installed::Failed { step: Step::Install, said: "no such plugin".into() }
    );
}

#[test]
fn when_both_fail_the_marketplace_is_what_stopped_it() {
    let scratch = Scratch::new();
    let cli = script(&scratch, "cli", "echo cannot read the folder >&2\nexit 2");

    assert_eq!(
        install_with(&cli, &scratch),
        Installed::Failed { step: Step::AddMarketplace, said: "cannot read the folder".into() }
    );
}

#[test]
fn a_cli_that_is_not_there_fails_with_why() {
    let scratch = Scratch::new();

    let Installed::Failed { step, said } = install_with(&scratch.path().join("absent"), &scratch) else {
        panic!("a program that is not there installs nothing");
    };

    assert_eq!(step, Step::AddMarketplace);
    assert!(said.contains("No such file"), "{said}");
}

#[test]
fn a_command_that_does_not_finish_is_stopped() {
    let scratch = Scratch::new();
    let cli = script(&scratch, "cli", "sleep 30");
    let asked = Asked::when(true).expect("asked");

    let installed =
        install(asked, cli.as_os_str(), "/usr/bin:/bin", scratch.path(), Path::new(MOD_FOLDER), Duration::from_millis(300));

    let Installed::Failed { said, .. } = installed else {
        panic!("stopped, not installed");
    };
    assert!(said.contains("did not finish"), "{said}");
}

#[test]
fn the_copy_is_made_by_the_script_that_makes_it_on_a_restart() {
    let scratch = Scratch::new();
    scratch.file("tree/plugins/.claude-plugin/marketplace.json", "{}");
    let home = scratch.folder("home");
    let tree = scratch.path().join("tree");
    assert!(!can_populate_from(&tree), "no script yet");
    script(
        &scratch,
        "tree/scripts/sync-mod",
        "mkdir -p \"$HOME/mod/.claude-plugin\"\necho \"$1\" > \"$HOME/mod/.claude-plugin/marketplace.json\"",
    );
    assert!(can_populate_from(&tree));
    assert!(!is_populated(&home.join("mod")));

    populate(&tree, &home, PLENTY).expect("copied");

    assert!(is_populated(&home.join("mod")));
}

#[test]
fn a_script_that_fails_says_why() {
    let scratch = Scratch::new();
    script(&scratch, "tree/scripts/sync-mod", "echo disk full >&2\nexit 1");

    let failed = populate(&scratch.path().join("tree"), scratch.path(), PLENTY);

    assert_eq!(failed, Err("disk full".to_string()));
}
