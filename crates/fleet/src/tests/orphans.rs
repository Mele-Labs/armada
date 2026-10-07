//! Drones whose Fleet is gone are ended, and nobody else's are.
//!
//! The fake Drone is `node`, not a shell: `ps eww` hides the environment of
//! Apple's binaries, so a mark on `/bin/sh` could never be read back. Every
//! case spawns and reaps its own child and ends it through its own group; the
//! sweep is asked what it finds, and only the group a case made is ended.

use std::num::NonZeroU32;
use std::process::Command as Plain;
use std::time::Duration;

use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Child;

use crate::orphans::{end, find, listed_mark, mark_in_environment, mark_of, names_a_dead_test};
use crate::process::{holder_of, Holder};
use crate::Detached;

const HOLDS_FOR_A_MINUTE: &str = "console.log('up'); setTimeout(() => {}, 60000)";
const IGNORES_THE_POLITE_SIGNAL: &str =
    "process.on('SIGTERM', () => {}); console.log('up'); setTimeout(() => {}, 60000)";

fn node() -> std::path::PathBuf {
    std::env::split_paths(&std::env::var_os("PATH").expect("a PATH"))
        .map(|dir| dir.join("node"))
        .find(|path| path.is_file())
        .expect("node, which the repository's own tooling needs")
}

/// A Drone, ended with the case even where it fails.
struct Drone(Child);

impl Drone {
    fn group(&self) -> NonZeroU32 {
        NonZeroU32::new(self.0.id().expect("running")).expect("a pid")
    }

    async fn is_gone_within(&mut self, within: Duration) -> bool {
        tokio::time::timeout(within, self.0.wait()).await.is_ok()
    }
}

impl Drop for Drone {
    fn drop(&mut self) {
        let _ = self.0.start_kill();
    }
}

async fn up(mut child: Child) -> Drone {
    let mut line = String::new();
    BufReader::new(child.stdout.take().expect("piped"))
        .read_line(&mut line)
        .await
        .expect("it says it is up");
    Drone(child)
}

async fn spawned(script: &str, mark: Option<&str>) -> Drone {
    let mut drone = Detached::program(node())
        .args(["-e", script])
        .capturing_output();
    if let Some(mark) = mark {
        drone = drone.marked_by(mark);
    }
    up(drone.spawn().expect("node spawns")).await
}

fn found(drone: &Drone) -> bool {
    find(&[]).expect("ps runs").contains(&drone.group())
}

/// The mark the sweep reads off the Drone's row. Fails where the row is not
/// listed, so a broken listing cannot pass a case that expects nothing found.
fn mark_read(drone: &Drone) -> Option<(u32, String)> {
    listed_mark(drone.group().get())
        .expect("ps runs")
        .expect("the Drone is listed")
        .flatten()
}

/// A mark naming a process that is not there: a pid a child had and gave back.
fn a_fleet_that_is_gone() -> String {
    let mut child = Plain::new("/usr/bin/true").spawn().expect("true runs");
    let pid = child.id();
    child.wait().expect("it ends");
    mark_of(pid, "Mon Oct  7 10:00:00 2026")
}

fn a_live_other_fleet() -> (u32, String) {
    let pid = std::os::unix::process::parent_id();
    let Ok(Holder::Held(started)) = holder_of(pid) else {
        panic!("the test runner is running");
    };
    (pid, mark_of(pid, started.as_str()))
}

#[tokio::test]
async fn a_drone_marked_by_a_fleet_that_is_gone_is_found_and_ended() {
    let gone_mark = a_fleet_that_is_gone();
    let mut drone = spawned(HOLDS_FOR_A_MINUTE, Some(&gone_mark)).await;
    assert!(found(&drone));
    end(vec![drone.group()], Duration::from_millis(100), || vec![]);
    assert!(drone.is_gone_within(Duration::from_secs(5)).await);
}

#[tokio::test]
async fn a_pid_that_came_round_as_another_process_is_a_fleet_that_is_gone() {
    let (pid, _) = a_live_other_fleet();
    let drone = spawned(
        HOLDS_FOR_A_MINUTE,
        Some(&mark_of(pid, "Mon Oct  7 10:00:00 2026")),
    )
    .await;
    assert!(found(&drone));
}

#[tokio::test]
async fn a_drone_of_another_fleet_that_is_running_is_left_alone() {
    let (pid, mark) = a_live_other_fleet();
    let drone = spawned(HOLDS_FOR_A_MINUTE, Some(&mark)).await;
    assert_eq!(mark_read(&drone).map(|(by, _)| by), Some(pid));
    assert!(!found(&drone));
}

#[tokio::test]
async fn a_drone_of_this_fleet_is_left_alone() {
    let drone = spawned(HOLDS_FOR_A_MINUTE, None).await;
    assert_eq!(
        mark_read(&drone).map(|(by, _)| by),
        Some(std::process::id())
    );
    assert!(!found(&drone));
}

#[tokio::test]
async fn a_process_nothing_marked_is_left_alone() {
    let child = tokio::process::Command::new(node())
        .args(["-e", HOLDS_FOR_A_MINUTE])
        .process_group(0)
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("node spawns");
    let drone = up(child).await;
    assert_eq!(mark_read(&drone), None);
    assert!(!found(&drone));
}

#[tokio::test]
async fn a_group_a_recorded_drone_leads_is_left_alone_though_its_fleet_is_gone() {
    let drone = spawned(HOLDS_FOR_A_MINUTE, Some(&a_fleet_that_is_gone())).await;
    let held = [drone.group().get()];
    assert!(mark_read(&drone).is_some());
    assert!(!find(&held).expect("ps runs").contains(&drone.group()));
}

#[tokio::test]
async fn a_drone_that_ignores_the_polite_signal_is_ended_after_the_grace() {
    let mut drone = spawned(IGNORES_THE_POLITE_SIGNAL, Some(&a_fleet_that_is_gone())).await;
    let group = drone.group();
    end(vec![group], Duration::from_millis(300), || vec![group]);
    assert!(drone.is_gone_within(Duration::from_secs(5)).await);
}

#[test]
fn a_fake_drone_left_by_a_test_that_is_gone_is_named_by_its_directory() {
    let temp = vec![String::from("/var/folders/y0/T")];
    let command =
        "sh -c ( exec 3>>\"$1\" ) sh /var/folders/y0/T/armada-fleet-65966-725/.armada/t.jsonl";
    let mut vacant = |pid: u32, started: Option<&str>| pid == 65966 && started.is_none();
    assert!(names_a_dead_test(command, &temp, &mut vacant));
    let mut alive = |_: u32, _: Option<&str>| false;
    assert!(!names_a_dead_test(command, &temp, &mut alive));
    let elsewhere = "vim /srv/armada-fleet-65966-725/x";
    assert!(!names_a_dead_test(elsewhere, &temp, &mut vacant));
}

#[test]
fn a_mark_is_read_out_of_a_nul_separated_environment() {
    let environment = b"PATH=/bin\0ARMADA_SPAWNED_BY=42:Mon_Oct_7_10:00:00_2026\0HOME=/x\0";
    assert_eq!(
        mark_in_environment(environment),
        Some(Some((42, "Mon_Oct_7_10:00:00_2026".to_string())))
    );
    assert_eq!(mark_in_environment(b"PATH=/bin\0HOME=/x\0"), None);
    assert_eq!(mark_in_environment(b"ARMADA_SPAWNED_BY=junk\0"), Some(None));
}
