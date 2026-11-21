//! Builds taking turns on a loaded machine, against real processes: each Job a shell running a
//! stand-in `rustc` or `cargo`, the process table read as Fleet reads it, and the signals really
//! sent.

use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::time::{Duration, Instant};

use crate::build_turns::{self, Contender, Signal, Turns, LISTING, TURN};
use crate::tests::tmp::TempDir;

/// This test binary by a link named `rustc`, run as [`a_stand_in_compiler`], so the process table
/// names a compiler. A link, because macOS kills a plain copy of a program at its start
/// (`Killed: 9`). Not `/bin/sleep`: on Ubuntu 26.04 that is a multi-call binary that reads its
/// name and refuses `rustc`.
fn stand_in(dir: &Path) -> PathBuf {
    stand_in_named(dir, "rustc")
}

fn stand_in_named(dir: &Path, name: &str) -> PathBuf {
    let link = dir.join(name);
    let this = std::env::current_exe().expect("the test binary");
    std::os::unix::fs::symlink(this, &link).expect("the test binary links");
    link
}

/// A shell running `program` as [`a_stand_in_compiler`], and the pid of that child once it runs.
fn started(program: &Path) -> (Child, u32) {
    let shell = Command::new("/bin/sh")
        .arg("-c")
        .arg(format!(
            "'{}' tests::build_turns::a_stand_in_compiler --exact --ignored >/dev/null 2>&1; :",
            program.display()
        ))
        .spawn()
        .expect("the shell starts");
    let root = shell.id();
    let begun = Instant::now();
    loop {
        if let Some((pid, ..)) = build_turns::rows_for_test(&listing()).into_iter().find(|(_, parent, ..)| *parent == root) {
            return (shell, pid);
        }
        assert!(begun.elapsed() < Duration::from_secs(5), "the stand-in never started");
        std::thread::sleep(Duration::from_millis(20));
    }
}

fn stopped(pid: u32, expected: bool) -> bool {
    let begun = Instant::now();
    loop {
        let stopped = build_turns::rows_for_test(&listing())
            .into_iter()
            .find(|(one, ..)| *one == pid)
            .is_some_and(|(_, _, stopped, _)| stopped);
        if stopped == expected || begun.elapsed() > Duration::from_secs(2) {
            return stopped;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

/// The compiler [`stand_in`] names: sleeps until it is killed. Ignored, so a run of the suite
/// never starts it on its own.
#[test]
#[ignore = "started by the build-turn tests as their stand-in compiler, never on its own"]
fn a_stand_in_compiler() {
    std::thread::sleep(Duration::from_secs(120));
}

/// A Job's agent: a shell whose child is a compiler, as an agent's Bash call is.
struct Job {
    shell: Child,
    rustc: u32,
}

impl Job {
    fn building(rustc: &Path) -> Job {
        let (shell, rustc) = started(rustc);
        Job { shell, rustc }
    }

    fn as_contender(&self, job: &str, prioritised: bool) -> Contender {
        Contender { job: job.to_string(), roots: vec![self.shell.id()], prioritised }
    }

    /// Whether its compiler is stopped, once the signal has landed.
    fn held(&self, expected: bool) -> bool {
        stopped(self.rustc, expected)
    }
}

impl Drop for Job {
    fn drop(&mut self) {
        build_turns::send(&[Signal::Continue(self.rustc)]);
        let _ = Command::new("kill").args(["-9", &self.rustc.to_string()]).status();
        let _ = self.shell.kill();
        let _ = self.shell.wait();
    }
}

fn listing() -> String {
    let said = Command::new("ps").args(LISTING).output().expect("ps runs");
    String::from_utf8_lossy(&said.stdout).into_owned()
}

/// One look at the real table, its signals sent; no process is in a Check or carries a Job's id.
fn look(turns: &mut Turns, loaded: bool, contenders: &[Contender], now: Instant) {
    let signals = turns.look(loaded, contenders, &listing(), |_| false, |_, _| false, now, TURN);
    build_turns::send(&signals);
}

#[test]
fn on_a_loaded_machine_a_second_jobs_build_is_held_while_the_first_runs_then_released_on_its_turn() {
    let dir = TempDir::new();
    let rustc = stand_in(dir.path());
    let first = Job::building(&rustc);
    let second = Job::building(&rustc);
    let jobs = [first.as_contender("first", false), second.as_contender("second", false)];
    let mut turns = Turns::default();
    let start = Instant::now();

    look(&mut turns, true, &jobs, start);
    assert!(!first.held(false), "the oldest Job's build runs");
    assert!(second.held(true), "the second Job's build is held while the first runs");

    look(&mut turns, true, &jobs, start + TURN / 2);
    assert!(second.held(true), "still held within the first Job's turn");

    look(&mut turns, true, &jobs, start + TURN + Duration::from_secs(1));
    assert!(!second.held(false), "released on its turn");
    assert!(first.held(true), "and the first waits for the next");

    look(&mut turns, false, &jobs, start + TURN * 2);
    assert!(!first.held(false), "under the limit, nothing is held");
    assert!(!second.held(false));
}

#[test]
fn with_a_job_prioritised_its_builds_run_while_the_oldest_jobs_are_held() {
    let dir = TempDir::new();
    let rustc = stand_in(dir.path());
    let oldest = Job::building(&rustc);
    let prioritised = Job::building(&rustc);
    let jobs = [oldest.as_contender("oldest", false), prioritised.as_contender("prioritised", true)];
    let mut turns = Turns::default();
    let start = Instant::now();

    look(&mut turns, true, &jobs, start);
    assert!(!prioritised.held(false), "the prioritised Job's build runs");
    assert!(oldest.held(true), "the oldest Job's is held");

    look(&mut turns, true, &jobs, start + TURN * 3);
    assert!(!prioritised.held(false), "a prioritised Job keeps the turn while it builds");
    assert!(oldest.held(true));
}

/// The cargo of a held Job, as `cargo metadata` under an editor's language server or a build
/// resolving its packages: it holds the package-cache lock every other Job's cargo waits on.
struct Cargo {
    shell: Child,
    pid: u32,
}

impl Drop for Cargo {
    fn drop(&mut self) {
        build_turns::send(&[Signal::Continue(self.pid)]);
        let _ = Command::new("kill").args(["-9", &self.pid.to_string()]).status();
        let _ = self.shell.kill();
        let _ = self.shell.wait();
    }
}

#[test]
fn a_held_jobs_cargo_keeps_running_so_the_package_cache_lock_it_holds_never_starves_another_build() {
    let dir = TempDir::new();
    let rustc = stand_in(dir.path());
    let first = Job::building(&rustc);
    let second = Job::building(&rustc);
    let (shell, pid) = started(&stand_in_named(dir.path(), "cargo"));
    let cargo = Cargo { shell, pid };
    let jobs = [
        first.as_contender("first", false),
        Contender { job: "second".into(), roots: vec![second.shell.id(), cargo.shell.id()], prioritised: false },
    ];
    let mut turns = Turns::default();

    look(&mut turns, true, &jobs, Instant::now());
    assert!(second.held(true), "the second Job's compiler is held");
    assert!(!stopped(cargo.pid, false), "its cargo runs on, so the lock it holds is let go");

    // A cargo an earlier Fleet stopped holds the lock until it runs: the next look lets it go.
    build_turns::send(&[Signal::Stop(cargo.pid)]);
    assert!(stopped(cargo.pid, true));
    look(&mut Turns::default(), true, &jobs, Instant::now());
    assert!(!stopped(cargo.pid, false), "a stopped cargo in a Job's tree is continued");
}

#[test]
fn a_fleet_that_stops_lets_every_build_it_held_go() {
    let dir = TempDir::new();
    let rustc = stand_in(dir.path());
    let first = Job::building(&rustc);
    let second = Job::building(&rustc);
    let jobs = [first.as_contender("first", false), second.as_contender("second", false)];
    let mut turns = Turns::default();

    look(&mut turns, true, &jobs, Instant::now());
    assert!(second.held(true));
    build_turns::send(&turns.released());
    assert!(!second.held(false), "released when Fleet stops");

    // A Fleet that ended without releasing: the next one's first look lets the build go.
    build_turns::send(&[Signal::Stop(second.rustc)]);
    assert!(second.held(true));
    look(&mut Turns::default(), false, &jobs, Instant::now());
    assert!(!second.held(false), "a build left stopped is continued by the next Fleet");
}

#[test]
fn a_checks_build_and_a_lone_jobs_build_are_never_held() {
    let listing = "\
  10     1 S    /usr/bin/agent
  11    10 S    /bin/zsh
  12    11 R    /Users/user/.rustup/toolchains/stable/bin/rustc
  20     1 S    /usr/bin/agent
  21    20 S    /Users/user/Library/Application Support/Armada/armada
  22    21 R    /Users/user/.rustup/toolchains/stable/bin/rustc
  30     1 R    /Users/user/.rustup/toolchains/stable/bin/rustc
";
    let jobs = [
        Contender { job: "a".into(), roots: vec![10], prioritised: false },
        Contender { job: "b".into(), roots: vec![20], prioritised: false },
    ];
    let mut turns = Turns::default();
    // b's compiler runs under a Check, so only a builds: nothing is held.
    let said = turns.look(true, &jobs, listing, |pid| pid == 22, |_, _| false, Instant::now(), TURN);
    assert_eq!(said, vec![]);
    assert_eq!(turns.leader(), None);
    // Out of the Check, b's is held while a, the oldest, runs; pid 30 is no Job's and is left alone.
    let said = turns.look(true, &jobs, listing, |_| false, |_, _| false, Instant::now(), TURN);
    assert_eq!(said, vec![Signal::Stop(22)]);
    assert_eq!(turns.leader(), Some("a"));
}

#[test]
fn a_compiler_outside_every_jobs_tree_is_the_job_whose_id_it_carries() {
    let listing = "\
  10     1 S    /usr/bin/agent
  11    10 R    /bin/rustc
  40     1 R    /bin/rustc
";
    let jobs = [
        Contender { job: "a".into(), roots: vec![10], prioritised: false },
        Contender { job: "b".into(), roots: vec![99], prioritised: false },
    ];
    let said = Turns::default().look(true, &jobs, listing, |_| false, |pid, job| pid == 40 && job == "b", Instant::now(), TURN);
    assert_eq!(said, vec![Signal::Stop(40)]);
}

#[test]
fn a_job_that_stops_building_gives_its_turn_up_at_once() {
    let both = "\
  10     1 S    /usr/bin/agent
  11    10 R    /bin/rustc
  20     1 S    /usr/bin/agent
  21    20 R    /bin/rustc
  30     1 S    /usr/bin/agent
  31    30 R    /bin/rustc
";
    let jobs = [
        Contender { job: "a".into(), roots: vec![10], prioritised: false },
        Contender { job: "b".into(), roots: vec![20], prioritised: false },
        Contender { job: "c".into(), roots: vec![30], prioritised: false },
    ];
    let mut turns = Turns::default();
    let start = Instant::now();
    turns.look(true, &jobs, both, |_| false, |_, _| false, start, TURN);
    assert_eq!(turns.leader(), Some("a"));
    // a's build ended: the turn passes to b, the next after it, not back to the start.
    let without_a = both.lines().filter(|line| !line.contains(" 11 ")).collect::<Vec<_>>().join("\n");
    turns.look(true, &jobs, &without_a, |_| false, |_, _| false, start + Duration::from_secs(1), TURN);
    assert_eq!(turns.leader(), Some("b"));
}

#[test]
fn each_job_held_is_named_while_held_so_it_is_told_both_ways() {
    let both = "\
  10     1 S    /usr/bin/agent
  11    10 R    /bin/rustc
  20     1 S    /usr/bin/agent
  21    20 R    /bin/rustc
";
    let jobs = [
        Contender { job: "a".into(), roots: vec![10], prioritised: false },
        Contender { job: "b".into(), roots: vec![20], prioritised: false },
    ];
    let mut turns = Turns::default();
    let start = Instant::now();
    turns.look(true, &jobs, both, |_| false, |_, _| false, start, TURN);
    assert_eq!(turns.jobs_held(), ["b".to_string()].into(), "b is held, so it is told it is paused");
    let b_stopped = both.replace(" 21    20 R", " 21    20 T");
    turns.look(true, &jobs, &b_stopped, |_| false, |_, _| false, start + TURN + Duration::from_secs(1), TURN);
    assert_eq!(turns.jobs_held(), ["a".to_string()].into(), "the turn passed: a is told it waits, b that it runs");
    turns.look(false, &jobs, both, |_| false, |_, _| false, start + TURN * 2, TURN);
    assert!(turns.jobs_held().is_empty(), "under the limit every Job is told its builds run");
}

#[test]
fn short_memory_holds_builds_and_keeps_holding_for_a_turn_after_it_eases() {
    let mut turns = Turns::default();
    let start = Instant::now();
    assert!(!turns.loaded(false, Some(1), start), "normal pressure and a low load hold nothing");
    assert!(turns.loaded(false, Some(2), start), "warn pressure holds builds");
    assert!(turns.loaded(false, Some(1), start + TURN / 2), "eased within a turn: still held, or every build would start at once");
    assert!(!turns.loaded(false, Some(1), start + TURN + Duration::from_secs(1)), "eased for a turn: let go");
    assert!(turns.loaded(true, None, start + TURN * 2), "the load alone still holds");
}
