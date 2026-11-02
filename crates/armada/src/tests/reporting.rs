//! `armada report` with no Fleet answering: the report the CLI can read itself, and the verb's shape.
//! **`report` returns a string and cannot fail**, which is how the verb exits 0 whether or not Fleet
//! answered.

use crate::cli::{read, Verb};
use crate::reporting::report;
use crate::tests::TempDir;

fn asked(line: &str) -> Result<Verb, crate::cli::Misread> {
    read(line.split_whitespace().map(str::to_string))
}

#[test]
fn the_report_verb_parses_and_takes_no_path() {
    assert_eq!(asked("report"), Ok(Verb::Report));
    assert!(asked("report /somewhere").is_err());
    assert!(asked("report --all").is_err());
}

#[test]
fn with_no_fleet_it_prints_the_install_the_log_and_when_it_was_taken_and_nothing_a_line_said() {
    let home = TempDir::new();
    let scratch = TempDir::new();
    let bin = TempDir::new();
    let logs = home.path().join("Library/Logs/Armada");
    std::fs::create_dir_all(&logs).unwrap();
    let now = fleet::clock::Clock::now(&fleet::SystemClock::new());
    let line = format!(
        r#"{{"ts":"{}","level":"warn","component":"fleet","run_id":"r","msg":"{}/secret.txt was not read","fields":{{"code":"fleet.slot_held","why":"the prompt said hello"}}}}"#,
        now.as_str(),
        home.path().display()
    );
    std::fs::write(logs.join("com.armada.fleet.log"), format!("{line}\nplain text with a password\n")).unwrap();
    std::env::set_var("HOME", home.path());
    std::env::set_var("TMPDIR", scratch.path());
    std::env::set_var("PATH", bin.path());
    std::env::remove_var("ARMADA_FLEET_LABEL");

    let text = report(scratch.path());

    assert!(text.starts_with("Armada debug report\n\nInstall\n"), "{text}");
    let harness = format!("  {:<18}not found", adapters::debug_facts::HARNESS);
    assert!(text.contains(&format!("  armada resolves   no\n{harness}\n\nFleet   did not answer\n\nRecent trouble\n")), "{text}");
    assert!(text.contains("WARN  fleet      fleet.slot_held\n"), "{text}");
    for left_out in ["secret", "prompt", "hello", "password", "fleet protocol"] {
        assert!(!text.contains(left_out), "{left_out} is in the report: {text}");
    }
    assert!(text.lines().last().is_some_and(|last| last.starts_with("taken 20") && last.ends_with('Z')), "{text}");
}
