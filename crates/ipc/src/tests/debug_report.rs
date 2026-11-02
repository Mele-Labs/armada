//! The text of Copy debug report: laid out as the approved fixture is, with the home folder as `~`, and
//! nothing a person wrote reaching it from Fleet's log.
//! `apps/desktop/src/renderer/src/mock/debug-report-fixtures.ts` is the layout.

use crate::debug_report::{render, Facts, FleetFacts, Install, JobFacts, Log, ModFacts, RepositoryFacts, Trouble};
use crate::log_trouble::trouble_in;

const FULL: &str = r#"Armada debug report

Install
  armada            0.14.2
  runs from         ~/Applications/Armada.app/Contents/Resources/bin/armada
  armada resolves   no
  agent cli         2.1.310

Mod
  fleet copy           filled
  fleet copy version   0.14.2
  plugin installed     0.14.1

This repository  web-app
  armada.yml   present
  kept in      Kit

Job  Add retry to the sync worker
  id        4f2a91c0
  runs      hosted
  mod       0.14.2
  door      connected
  missing   nothing
  tools     Bash, Edit, Glob, Grep, Read, Write
            mcp__armada-fleet__waiting_for, mcp__armada-fleet__start_another_job
  skills    armada:reading-a-brief, armada:verify-before-done

Job  Fix the flaky checkout test
  id        b81d03e7
  runs      terminal
  mod       none
  door      connected
  missing   mod, armada: skills
  tools     Bash, Edit, Glob, Grep, Read, Write
  skills    none

Recent trouble
  09:12:41   WARN  door       reconnect_refused
  09:12:44   WARN  hosted     init_without_mod
  09:20:03   WARN  store      slow_write

fleet protocol 5.2  taken 2026-11-02T09:31:12Z"#;

const SILENT: &str = r#"Armada debug report

Install
  armada            0.14.2
  runs from         ~/Applications/Armada.app/Contents/Resources/bin/armada
  armada resolves   no
  agent cli         2.1.310

Fleet   did not answer

Recent trouble
  09:12:41   WARN  door       reconnect_refused
  09:12:44   WARN  hosted     init_without_mod
  09:20:03   WARN  store      slow_write

taken 2026-11-02T09:31:12Z"#;

fn names(list: &[&str]) -> Vec<String> {
    list.iter().map(|name| name.to_string()).collect()
}

fn trouble(time: &str, target: &str, code: &str) -> Trouble {
    Trouble { time: time.into(), at: format!("2026-11-02T{time}.000Z"), level: "WARN".into(), target: target.into(), code: Some(code.into()) }
}

fn install() -> Install {
    Install {
        armada: "0.14.2".into(),
        runs_from: "/Users/user/Applications/Armada.app/Contents/Resources/bin/armada".into(),
        resolves: false,
        harness: "agent cli".into(),
        harness_version: Some("2.1.310".into()),
    }
}

fn log() -> Log {
    Log::Read(vec![
        trouble("09:12:41", "door", "reconnect_refused"),
        trouble("09:12:44", "hosted", "init_without_mod"),
        trouble("09:20:03", "store", "slow_write"),
    ])
}

fn fleet() -> FleetFacts {
    FleetFacts {
        protocol: "5.2".into(),
        mod_facts: ModFacts { copy_filled: true, copy_version: Some("0.14.2".into()), installed_version: Some("0.14.1".into()) },
        repository: Some(RepositoryFacts { name: "web-app".into(), armada_yml: "present".into(), kept_in: "Kit".into() }),
        jobs: vec![
            JobFacts {
                title: "Add retry to the sync worker".into(),
                id: "4f2a91c0".into(),
                hosted: true,
                mod_version: Some("0.14.2".into()),
                door: "connected".into(),
                missing: Vec::new(),
                tools: Some(names(&["Bash", "Edit", "Glob", "Grep", "Read", "Write", "mcp__armada-fleet__waiting_for", "mcp__armada-fleet__start_another_job"])),
                skills: Some(names(&["armada:reading-a-brief", "armada:verify-before-done"])),
            },
            JobFacts {
                title: "Fix the flaky checkout test".into(),
                id: "b81d03e7".into(),
                hosted: false,
                mod_version: None,
                door: "connected".into(),
                missing: names(&["mod", "armada: skills"]),
                tools: Some(names(&["Bash", "Edit", "Glob", "Grep", "Read", "Write"])),
                skills: Some(Vec::new()),
            },
        ],
    }
}

fn facts(fleet: Option<FleetFacts>) -> Facts {
    Facts { home: "/Users/user".into(), install: install(), fleet, trouble: Some(log()), taken: "2026-11-02T09:31:12Z".into() }
}

#[test]
fn a_full_report_is_laid_out_as_the_approved_fixture_is() {
    assert_eq!(render(&facts(Some(fleet()))), FULL);
}

#[test]
fn a_report_with_fleet_silent_names_the_install_the_log_and_when_it_was_taken() {
    assert_eq!(render(&facts(None)), SILENT);
}

#[test]
fn the_home_folder_is_written_as_a_tilde_at_a_folder_boundary_and_nowhere_else() {
    let mut taken = facts(Some(fleet()));
    taken.home = "/opt/user".into();
    taken.install.runs_from = "/opt/user/.cargo/bin/armada".into();
    taken.fleet.as_mut().unwrap().jobs[0].title = "Read /opt/users/notes and /opt/user".into();
    let text = render(&taken);
    assert!(text.contains("  runs from         ~/.cargo/bin/armada\n"), "{text}");
    assert!(text.contains("Job  Read /opt/users/notes and ~\n"), "a longer name is not the home: {text}");
    assert!(!text.contains("/opt/user/"), "{text}");
}

#[test]
fn a_plugin_the_cli_does_not_hold_reads_as_not_installed_beside_the_copy_that_is_filled() {
    let mut taken = facts(Some(fleet()));
    taken.fleet.as_mut().unwrap().mod_facts.installed_version = None;
    let text = render(&taken);
    assert!(text.contains("  fleet copy           filled\n  fleet copy version   0.14.2\n  plugin installed     not installed\n"), "{text}");
}

#[test]
fn a_log_with_no_recent_lines_and_one_not_written_for_days_say_so() {
    let mut taken = facts(None);
    taken.trouble = Some(Log::Read(Vec::new()));
    assert!(render(&taken).contains("Recent trouble\n  none\n"));
    taken.trouble = Some(Log::Old { days: 3 });
    assert!(render(&taken).contains("Recent trouble\n  log   last written 3 days ago, not read\n"));
    taken.trouble = None;
    assert!(!render(&taken).contains("Recent trouble"), "no file is no block");
}

fn line(ts: &str, level: &str, extra: &str) -> String {
    format!(r#"{{"ts":"{ts}","level":"{level}","component":"fleet","run_id":"r","msg":"/Users/user/secret.txt was not read: the prompt said hello"{extra}}}"#)
}

#[test]
fn a_log_lines_free_text_does_not_reach_the_report_and_its_code_does_where_it_is_a_code() {
    let log = [
        line("2026-11-02T09:12:41.000Z", "warn", r#","fields":{"code":"fleet.slot_held","why":"the prompt said hello"}"#),
        line("2026-11-02T09:12:44.000Z", "error", r#","fields":{"code":"Please refund order 12 for ada@example.com"}"#),
        line("2026-11-02T09:12:45.000Z", "info", ""),
        "a plain line the program printed with a password in it".to_string(),
        line("2026-11-02T09:12:46.000Z", "warn", ""),
    ]
    .join("\n");
    let since = core_model::Timestamp::from_rfc3339("2026-11-02T00:00:00.000Z").epoch_millis().unwrap();
    let read = trouble_in(&log, since);
    assert_eq!(read.len(), 3, "warnings and errors with a level, and no line without one: {read:?}");

    let mut taken = facts(None);
    taken.trouble = Some(Log::Read(read));
    let text = render(&taken);
    assert!(text.contains("  09:12:41   WARN  fleet      fleet.slot_held\n"), "{text}");
    assert!(text.contains("  09:12:44   ERROR  fleet\n"), "a code that is a sentence is dropped, leaving the target: {text}");
    assert!(text.contains("  09:12:46   WARN  fleet\n"), "{text}");
    for free_text in ["secret", "prompt", "hello", "refund", "example.com", "password", "was not read"] {
        assert!(!text.contains(free_text), "{free_text} reached the report: {text}");
    }
}

#[test]
fn only_the_last_two_hundred_lines_and_only_the_last_day_are_read() {
    let old = line("2026-10-31T09:00:00.000Z", "warn", "");
    let recent = line("2026-11-02T09:00:00.000Z", "warn", "");
    let since = core_model::Timestamp::from_rfc3339("2026-11-01T09:30:00.000Z").epoch_millis().unwrap();
    assert_eq!(trouble_in(&format!("{old}\n{recent}"), since).len(), 1, "yesterday's line is not recent");

    let buried = std::iter::once(recent.clone()).chain(std::iter::repeat(line("2026-11-02T09:00:01.000Z", "info", "")).take(200)).collect::<Vec<_>>().join("\n");
    assert!(trouble_in(&buried, since).is_empty(), "a warning past the last 200 lines is not read");
}
