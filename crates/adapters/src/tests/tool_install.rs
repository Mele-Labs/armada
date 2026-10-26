//! Installing `bd` and setting it up: the commands it runs, where it finds Homebrew, and how it says it did not.
//!
//! **Homebrew answers from a script.** Fleet is started by Bridge and may not have
//! `/opt/homebrew/bin` on its PATH, so Homebrew is looked for in its known places first.

use std::os::unix::process::ExitStatusExt;
use std::process::{ExitStatus, Output};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use adapter_traits::Tool;

use crate::jira_tracker::Run;
use crate::tool_install::installed;

/// Every program in `present` answers `code`, `stderr`; any other is not on the machine. A call
/// whose words contain a key of `answers` answers that instead. **A `brew install` that works puts
/// `bd` on the machine.**
struct Machine {
    present: Mutex<Vec<&'static str>>,
    code: i32,
    stderr: &'static str,
    answers: Vec<(&'static str, i32, &'static str)>,
    calls: Mutex<Vec<(String, Vec<String>)>>,
    dirs: Mutex<Vec<(String, PathBuf)>>,
}

impl Machine {
    fn with(present: &'static [&'static str], code: i32, stderr: &'static str) -> Machine {
        Machine { present: Mutex::new(present.to_vec()), code, stderr, answers: Vec::new(), calls: Mutex::new(Vec::new()), dirs: Mutex::new(Vec::new()) }
    }

    fn answering(mut self, words: &'static str, code: i32, stderr: &'static str) -> Machine {
        self.answers.push((words, code, stderr));
        self
    }

    fn calls(&self) -> Vec<(String, Vec<String>)> {
        self.calls.lock().expect("not poisoned").clone()
    }

    /// Where each call that was started in a directory was started: its line, and the directory.
    fn started_in(&self) -> Vec<(String, PathBuf)> {
        self.dirs.lock().expect("not poisoned").clone()
    }

    /// Every call as one line: `bd bootstrap --yes`.
    fn lines(&self) -> Vec<String> {
        self.calls().iter().map(|(program, args)| format!("{program} {}", args.join(" "))).collect()
    }
}

impl Run for Machine {
    fn run_in(&self, dir: &Path, program: &str, args: &[String], stdin: Option<&[u8]>) -> std::io::Result<Output> {
        self.dirs.lock().expect("not poisoned").push((format!("{program} {}", args.join(" ")), dir.to_path_buf()));
        self.run(program, args, stdin)
    }

    fn run(&self, program: &str, args: &[String], _stdin: Option<&[u8]>) -> std::io::Result<Output> {
        self.calls.lock().expect("not poisoned").push((program.to_string(), args.to_vec()));
        if !self.present.lock().expect("not poisoned").contains(&program) {
            return Err(std::io::ErrorKind::NotFound.into());
        }
        let line = format!("{program} {}", args.join(" "));
        let (code, stderr) = self.answers.iter().find(|(words, ..)| line.contains(words)).map_or((self.code, self.stderr), |(_, code, stderr)| (*code, *stderr));
        if code == 0 && args == ["install", "beads"] {
            self.present.lock().expect("not poisoned").push("bd");
        }
        Ok(Output {
            status: ExitStatus::from_raw(code << 8),
            stdout: Vec::new(),
            stderr: stderr.as_bytes().to_vec(),
        })
    }
}

/// A main checkout with no beads project, as a fresh clone is.
fn checkout(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("armada-tool-install-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join(".git/info")).expect("made");
    dir.canonicalize().expect("there")
}

#[test]
fn bd_is_installed_with_brew_install_beads_through_the_homebrew_it_finds() {
    let machine = Machine::with(&["/opt/homebrew/bin/brew", "git"], 0, "");

    let done = installed(&machine, Tool::Bd, &checkout("brew"));

    assert!(done.done, "{}", done.said);
    let brewed: Vec<String> = machine.lines().into_iter().filter(|line| line.starts_with("/opt/homebrew/bin/brew")).collect();
    assert_eq!(brewed, ["/opt/homebrew/bin/brew install beads"]);
}

#[test]
fn homebrew_is_found_on_the_path_where_it_is_in_no_known_place() {
    let machine = Machine::with(&["brew", "git"], 0, "");

    assert!(installed(&machine, Tool::Bd, &checkout("brew")).done);
    assert!(machine.lines().contains(&"brew install beads".to_string()));
}

#[test]
fn no_homebrew_is_said_and_nothing_else_is_run() {
    let machine = Machine::with(&[], 0, "");

    let not = installed(&machine, Tool::Bd, &checkout("brew"));

    assert!(!not.done);
    assert!(not.said.contains("Homebrew is not available"), "{}", not.said);
    assert_eq!(machine.lines(), ["bd --version", "/opt/homebrew/bin/brew install beads", "/usr/local/bin/brew install beads", "brew install beads"]);
}

#[test]
fn a_brew_that_fails_says_what_it_said() {
    let machine = Machine::with(&["/opt/homebrew/bin/brew"], 1, "Error: no such formula\n");

    let not = installed(&machine, Tool::Bd, &checkout("brew"));

    assert!(!not.done);
    assert!(not.said.contains("no such formula"), "{}", not.said);
}

#[test]
fn bd_already_there_is_not_installed_again_and_the_project_is_made_from_origin_in_the_main_checkout() {
    let main = checkout("setup");
    let machine = Machine::with(&["bd", "git"], 0, "");

    let done = installed(&machine, Tool::Bd, &main);

    assert!(done.done, "{}", done.said);
    let at = main.display();
    assert_eq!(
        machine.lines(),
        [
            "bd --version".to_string(),
            "bd metrics off".to_string(),
            format!("git -C {at} ls-remote --exit-code origin refs/dolt/data"),
            "bd bootstrap --yes".to_string(),
            format!("git -C {at} config beads.role maintainer"),
        ]
    );
    // `-C` makes bd demand the project that bootstrap is making, so it runs in the checkout instead.
    assert_eq!(
        machine.started_in(),
        [("bd metrics off".to_string(), main.clone()), ("bd bootstrap --yes".to_string(), main.clone())]
    );
    let exclude = std::fs::read_to_string(main.join(".git/info/exclude")).expect("written");
    assert_eq!(exclude.lines().collect::<Vec<_>>(), ["/.beads/", "/.beads.gate.lock"]);
}

#[test]
fn a_slot_is_set_up_in_the_main_checkout_it_was_cut_from() {
    let main = checkout("slot-main");
    let slot = main.join(".armada/slots/slot-1");
    std::fs::create_dir_all(main.join(".git/worktrees/slot-1")).expect("made");
    std::fs::create_dir_all(&slot).expect("made");
    std::fs::write(slot.join(".git"), format!("gitdir: {}\n", main.join(".git/worktrees/slot-1").display())).expect("written");
    std::fs::write(main.join(".git/worktrees/slot-1/commondir"), "../..\n").expect("written");
    let machine = Machine::with(&["bd", "git"], 0, "");

    assert!(installed(&machine, Tool::Bd, &slot).done);

    assert!(machine.started_in().contains(&("bd bootstrap --yes".to_string(), main.clone())), "{:?}", machine.started_in());
}

#[test]
fn an_origin_with_no_beads_data_is_said_and_no_empty_backlog_is_started() {
    let main = checkout("no-data");
    let machine = Machine::with(&["bd", "git"], 0, "").answering("ls-remote", 2, "");

    let not = installed(&machine, Tool::Bd, &main);

    assert!(!not.done);
    assert!(not.said.contains("holds no beads data"), "{}", not.said);
    assert!(machine.lines().iter().all(|line| !line.contains("bootstrap")), "{:?}", machine.lines());
}

#[test]
fn a_main_checkout_that_has_a_database_is_left_as_it_is() {
    let main = checkout("has-one");
    std::fs::create_dir_all(main.join(".beads")).expect("made");
    std::fs::write(main.join(".beads/metadata.json"), "{}").expect("written");
    let machine = Machine::with(&["bd", "git"], 0, "");

    let done = installed(&machine, Tool::Bd, &main);

    assert!(done.done);
    assert!(done.said.contains("already has a beads database"), "{}", done.said);
    assert!(machine.lines().iter().all(|line| !line.contains("bootstrap") && !line.contains("ls-remote")), "{:?}", machine.lines());
}

#[test]
fn a_bootstrap_that_fails_says_what_bd_said() {
    let main = checkout("fails");
    let machine = Machine::with(&["bd", "git"], 0, "").answering("bootstrap", 1, "Error: clone refused");

    let not = installed(&machine, Tool::Bd, &main);

    assert!(!not.done);
    assert!(not.said.contains("clone refused"), "{}", not.said);
}

#[test]
fn bd_missing_is_installed_first_and_then_set_up() {
    let main = checkout("both");
    let machine = Machine::with(&["/opt/homebrew/bin/brew", "git"], 0, "");

    let done = installed(&machine, Tool::Bd, &main);

    assert!(done.done, "{}", done.said);
    let lines = machine.lines();
    let at = |wanted: &str| lines.iter().position(|line| line.contains(wanted)).unwrap_or_else(|| panic!("{wanted} in {lines:?}"));
    assert!(at("brew install beads") < at("bootstrap --yes"));
}
