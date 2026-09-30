//! The machine's Check slots: one budget every process that runs a Check
//! takes from, whoever started it.
//!
//! **Real files and real locks**, for this crate's reason: that a lock goes
//! with the process holding it is the operating system's promise, and a fake
//! would assert that the test author knows what the kernel does.

use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::Duration;

use crate::slots::{CheckSlots, InUse};

/// A directory of its own for one test, gone when it is dropped.
struct Dir(PathBuf);

impl Dir {
    fn new(test: &str) -> Dir {
        let at =
            std::env::temp_dir().join(format!("armada-check-slots-{}-{test}", std::process::id()));
        let _ = std::fs::remove_dir_all(&at);
        Dir(at)
    }
}

impl Drop for Dir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[tokio::test]
async fn a_second_holder_over_one_slot_waits_until_the_first_lets_go() {
    let dir = Dir::new("second-waits");
    let slots = CheckSlots::at(&dir.0, 1);
    let first = slots
        .try_take(1)
        .expect("the directory is writable")
        .expect("nothing else holds it");

    let waiting = slots.clone();
    let second = tokio::spawn(async move { waiting.take(1, |_| {}).await });
    tokio::time::sleep(Duration::from_millis(600)).await;
    assert!(!second.is_finished(), "the second took a slot already held");

    drop(first);
    let held = tokio::time::timeout(Duration::from_secs(5), second)
        .await
        .expect("the second took the slot once the first let go")
        .expect("the task ran")
        .expect("the directory is writable");
    assert!(
        slots.try_take(1).expect("writable").is_err(),
        "the second holds the one slot there is"
    );
    drop(held);
}

#[test]
fn a_holder_that_dies_frees_its_slot() {
    let dir = Dir::new("holder-dies");
    let mut holder = Command::new(std::env::current_exe().expect("the test binary"))
        .args([
            "tests::slots::holds_a_slot_until_killed",
            "--exact",
            "--ignored",
            "--nocapture",
        ])
        .env(HOLD_IN, &dir.0)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("the test binary starts again");
    let said = BufReader::new(holder.stdout.take().expect("piped"));
    let held = said
        .lines()
        .map_while(Result::ok)
        .any(|line| line.trim() == "held");
    assert!(held, "the other process never said it held the slot");

    let slots = CheckSlots::at(&dir.0, 1);
    assert_eq!(
        slots.try_take(1).expect("writable").err(),
        Some(InUse {
            in_use: 1,
            of: 1,
            wants: 1
        }),
        "the other process holds the one slot"
    );

    holder.kill().expect("the holder is killed");
    holder.wait().expect("the holder is reaped");
    assert!(
        slots.try_take(1).expect("writable").is_ok(),
        "a holder that died still holds its slot"
    );
}

/// Where [`holds_a_slot_until_killed`] takes its slot.
const HOLD_IN: &str = "ARMADA_TEST_HOLD_A_CHECK_SLOT_IN";

/// The other process in [`a_holder_that_dies_frees_its_slot`]: takes the one
/// slot, says so, and sleeps until it is killed. Ignored, so a run of the
/// suite never starts it on its own.
#[test]
#[ignore = "started by a_holder_that_dies_frees_its_slot, never on its own"]
fn holds_a_slot_until_killed() {
    let Some(dir) = std::env::var_os(HOLD_IN) else {
        return;
    };
    let _held = CheckSlots::at(PathBuf::from(dir), 1)
        .try_take(1)
        .expect("writable")
        .expect("nothing else holds it");
    println!("held");
    std::thread::sleep(Duration::from_secs(60));
}

#[tokio::test]
async fn the_wait_says_how_many_are_in_use() {
    let dir = Dir::new("says-in-use");
    let slots = CheckSlots::at(&dir.0, 4);
    let taken = slots.try_take(4).expect("writable").expect("all free");

    let full = slots
        .try_take(1)
        .expect("writable")
        .err()
        .expect("all held");
    assert_eq!(full.to_string(), "waiting for a Check slot: 4 of 4 in use");

    let told = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let hearing = std::sync::Arc::clone(&told);
    let waiting = slots.clone();
    let asking = tokio::spawn(async move {
        waiting
            .take(1, move |in_use| {
                hearing.lock().expect("unpoisoned").push(in_use)
            })
            .await
    });
    tokio::time::sleep(Duration::from_millis(400)).await;
    drop(taken);
    let _held = tokio::time::timeout(Duration::from_secs(5), asking)
        .await
        .expect("taken once the four let go")
        .expect("the task ran")
        .expect("writable");
    assert_eq!(
        told.lock().expect("unpoisoned").first().copied(),
        Some(InUse {
            in_use: 4,
            of: 4,
            wants: 1
        })
    );
}

#[test]
fn a_wide_check_takes_every_slot_it_wants_or_none() {
    let dir = Dir::new("wide");
    let slots = CheckSlots::at(&dir.0, 3);
    let one = slots.try_take(1).expect("writable").expect("free");

    let wide = slots
        .try_take(3)
        .expect("writable")
        .err()
        .expect("one held");
    assert_eq!(wide.to_string(), "waiting for 3 Check slots: 1 of 3 in use");
    // Nothing kept from the refused ask: the other two are still free.
    let two = slots.try_take(2).expect("writable").expect("two are free");
    drop((one, two));
}

#[test]
fn a_check_wider_than_the_machine_takes_every_slot_there_is() {
    let dir = Dir::new("wider");
    let slots = CheckSlots::at(&dir.0, 2);
    let all = slots
        .try_take(5)
        .expect("writable")
        .expect("clamped to two");
    assert!(slots.try_take(1).expect("writable").is_err());
    drop(all);
}
