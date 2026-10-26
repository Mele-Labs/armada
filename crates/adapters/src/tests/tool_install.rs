//! Installing `bd`: the one command it runs, where it finds Homebrew, and how it says it did not.
//!
//! **Homebrew answers from a script.** Fleet is started by Bridge and may not have
//! `/opt/homebrew/bin` on its PATH, so Homebrew is looked for in its known places first.

use std::os::unix::process::ExitStatusExt;
use std::process::{ExitStatus, Output};
use std::sync::Mutex;

use adapter_traits::Tool;

use crate::jira_tracker::Run;
use crate::tool_install::installed;

/// Every program in `present` answers `code`, `stderr`; any other is not on the machine.
struct Machine {
    present: &'static [&'static str],
    code: i32,
    stderr: &'static str,
    calls: Mutex<Vec<(String, Vec<String>)>>,
}

impl Machine {
    fn with(present: &'static [&'static str], code: i32, stderr: &'static str) -> Machine {
        Machine { present, code, stderr, calls: Mutex::new(Vec::new()) }
    }

    fn calls(&self) -> Vec<(String, Vec<String>)> {
        self.calls.lock().expect("not poisoned").clone()
    }
}

impl Run for Machine {
    fn run(&self, program: &str, args: &[String], _stdin: Option<&[u8]>) -> std::io::Result<Output> {
        self.calls.lock().expect("not poisoned").push((program.to_string(), args.to_vec()));
        if !self.present.contains(&program) {
            return Err(std::io::ErrorKind::NotFound.into());
        }
        Ok(Output {
            status: ExitStatus::from_raw(self.code << 8),
            stdout: Vec::new(),
            stderr: self.stderr.as_bytes().to_vec(),
        })
    }
}

#[test]
fn bd_is_installed_with_brew_install_beads_through_the_homebrew_it_finds() {
    let machine = Machine::with(&["/opt/homebrew/bin/brew"], 0, "");

    let done = installed(&machine, Tool::Bd);

    assert!(done.done, "{}", done.said);
    assert_eq!(
        machine.calls(),
        [("/opt/homebrew/bin/brew".to_string(), vec!["install".to_string(), "beads".to_string()])]
    );
}

#[test]
fn homebrew_is_found_on_the_path_where_it_is_in_no_known_place() {
    let machine = Machine::with(&["brew"], 0, "");

    assert!(installed(&machine, Tool::Bd).done);
    assert_eq!(machine.calls().last().map(|(program, _)| program.as_str()), Some("brew"));
}

#[test]
fn no_homebrew_is_said_and_nothing_else_is_run() {
    let machine = Machine::with(&[], 0, "");

    let not = installed(&machine, Tool::Bd);

    assert!(!not.done);
    assert!(not.said.contains("Homebrew is not available"), "{}", not.said);
    assert!(machine.calls().iter().all(|(_, args)| args == &["install".to_string(), "beads".to_string()]));
}

#[test]
fn a_brew_that_fails_says_what_it_said() {
    let machine = Machine::with(&["/opt/homebrew/bin/brew"], 1, "Error: no such formula\n");

    let not = installed(&machine, Tool::Bd);

    assert!(!not.done);
    assert!(not.said.contains("no such formula"), "{}", not.said);
}
