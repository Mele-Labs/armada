//! The machine's Check slots: one budget every process that runs a Check
//! takes from, whoever started it.
//!
//! **Real files and real locks**, for this crate's reason: that a lock goes
//! with the process holding it is the operating system's promise, and a fake
//! would assert that the test author knows what the kernel does.

use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::Duration;

use crate::slots::{CheckSlots, GateLimits, InUse, LoadWindow, Reading, Short, Waits};

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

/// **The merge line's ask goes first.** While it waits, a freed slot is not
/// for an ordinary ask, however much sooner that ask looks.
#[tokio::test]
async fn a_waiting_ask_ahead_keeps_an_ordinary_ask_out_of_a_freed_slot() {
    let dir = Dir::new("ahead-first");
    let slots = CheckSlots::at(&dir.0, 2);
    let one = slots.try_take(1).expect("writable").expect("free");
    let two = slots.try_take(1).expect("writable").expect("free");

    let ahead = slots.clone().ahead();
    let first = tokio::spawn(async move { ahead.take(1, |_| {}).await });
    tokio::time::sleep(Duration::from_millis(600)).await;
    assert!(!first.is_finished(), "took a slot while both were held");

    drop(one);
    assert!(
        slots.try_take(1).expect("writable").is_err(),
        "an ordinary ask took the slot the line was waiting for"
    );
    let held = tokio::time::timeout(Duration::from_secs(5), first)
        .await
        .expect("the ask ahead took the freed slot")
        .expect("the task ran")
        .expect("writable");

    drop(two);
    assert!(
        slots.try_take(1).expect("writable").is_ok(),
        "nothing waits ahead now, so an ordinary ask takes a free slot"
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
            wants: 1,
            why: Waits::Full,
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
    let mut out = std::io::stdout();
    writeln!(out, "held")
        .and_then(|()| out.flush())
        .expect("said");
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
            wants: 1,
            why: Waits::Full,
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

/// A count moved while a Check holds a slot past it: the holder keeps it, and
/// the next ask is counted against the new number. Every clone sees the move.
#[test]
fn a_resize_reaches_every_clone_and_leaves_a_holder_its_slot() {
    let dir = Dir::new("resize");
    let slots = CheckSlots::at(&dir.0, 3);
    let held = slots.try_take(3).expect("writable").expect("free");

    let shared = slots.clone();
    slots.resize(1);
    assert_eq!(shared.count(), 1, "the clone a Fleet's places hold");
    let Err(in_use) = shared.try_take(1).expect("writable") else { panic!("slot 0 is held") };
    assert_eq!(in_use, InUse { in_use: 1, of: 1, wants: 1, why: Waits::Full });

    drop(held);
    slots.resize(0);
    assert_eq!(slots.count(), 1, "never fewer than one");
    assert!(shared.try_take(1).expect("writable").is_ok());
}

/// Limits that gate nothing, for a test to change one of.
const OPEN: GateLimits = GateLimits {
    pressure_level: 2,
    load_limit: 0,
    load_window: LoadWindow::One,
    while_short: 1,
    spacing: Duration::ZERO,
};

/// Memory at the kernel's normal level, and a load nothing reads.
const QUIET: Reading = Reading { load: Some([40.0, 40.0, 40.0]), pressure: Some(1) };

fn gated(dir: &Dir, count: usize, limits: GateLimits, reading: Reading) -> CheckSlots {
    CheckSlots::at(&dir.0, count).watching(move || (limits, reading))
}

/// **A crowd waiting does not start all at once.** On 28 Nov 2026 seven waiting
/// Checks started in one look the moment the gate opened. With a spacing, a
/// second starts beside a running one only once the spacing has passed since
/// the last start, on any clone; the merge line is never spaced.
#[test]
fn a_waiting_crowd_starts_one_check_per_spacing_and_not_all_at_once() {
    let dir = Dir::new("spaced");
    let spacing = Duration::from_secs(60);
    let slots = gated(&dir, 4, GateLimits { spacing, ..OPEN }, QUIET);
    let first = slots.try_take(1).expect("writable").expect("nothing runs, so the first starts at once");

    let crowd = gated(&dir, 4, GateLimits { spacing, ..OPEN }, QUIET);
    let Err(waits) = crowd.try_take(1).expect("writable") else { panic!("a second started in the same spacing") };
    assert_eq!(waits.why, Waits::Spaced { spacing });
    assert_eq!(
        waits.to_string(),
        "waiting for a Check slot: a Check started in the last 60 seconds, and one starts at a time so each shows before the next"
    );
    let line = slots.clone().ahead().try_take(1).expect("writable").expect("the merge line is never spaced");
    drop(line);

    // The last start a spacing ago, as the started file's time says.
    let started = std::fs::OpenOptions::new().write(true).open(dir.0.join("started")).expect("the first wrote it");
    started.set_modified(std::time::SystemTime::now() - spacing).expect("settable");
    let second = crowd.try_take(1).expect("writable").expect("a spacing on, the next one starts");
    assert!(crowd.try_take(1).expect("writable").is_err(), "and the one after waits its own spacing");
    drop((first, second));

    let unspaced = gated(&dir, 4, OPEN, QUIET);
    let one = unspaced.try_take(1).expect("writable").expect("free");
    assert!(unspaced.try_take(1).expect("writable").is_ok(), "a spacing of zero spaces nothing");
    drop(one);
}

/// **A machine short of memory runs one Check at a time.** At the kernel's warn
/// level a second waits while the first holds a slot, and says why; under it a
/// second starts. The first always starts, the merge line is never held, and
/// `while_short` lets that many slots be in use.
#[test]
fn memory_at_warn_holds_a_second_check_while_one_runs() {
    let dir = Dir::new("memory");
    let warn = Reading { pressure: Some(2), ..QUIET };
    let normal = gated(&dir, 4, OPEN, QUIET);
    let one = normal.try_take(1).expect("writable").expect("free");
    let two = normal.try_take(1).expect("writable").expect("memory normal, a second starts");
    drop(two);

    let short = gated(&dir, 4, OPEN, warn);
    let Err(waits) = short.try_take(1).expect("writable") else { panic!("one Check runs already") };
    let memory = Short::Memory { level: 2, at: 2 };
    assert_eq!(waits.why, Waits::Short { short: memory, while_short: 1 });
    assert_eq!(
        waits.to_string(),
        "waiting for a Check slot: memory is short, at the kernel's pressure level 2, at or over 2, so Checks run one at a time"
    );
    assert!(short.clone().ahead().try_take(1).expect("writable").is_ok(), "the merge line is never held");

    let two_at_a_time = gated(&dir, 4, GateLimits { while_short: 2, ..OPEN }, warn);
    let two = two_at_a_time.try_take(1).expect("writable").expect("one in use, under two");
    assert!(two_at_a_time.try_take(1).expect("writable").is_err(), "two in use, the third waits");
    drop((one, two));

    let first = short.try_take(2).expect("writable").expect("nothing else runs, so the first starts");
    assert!(normal.try_take(2).expect("writable").is_ok(), "the first took only what it wanted");
    drop(first);
    let never = gated(&dir, 4, GateLimits { pressure_level: 0, ..OPEN }, warn);
    let one = never.try_take(1).expect("writable").expect("free");
    assert!(never.try_take(1).expect("writable").is_ok(), "a level of zero never calls memory short");
    drop(one);
}

/// **Load gates nothing unless its limit is set**, and then reads the window
/// chosen: the owner's choice of 28 Nov 2026, as the load reads two to three
/// times the cores busy on his Mac.
#[test]
fn load_holds_a_second_check_only_when_its_limit_is_set() {
    let dir = Dir::new("load");
    let one = gated(&dir, 4, OPEN, QUIET).try_take(1).expect("writable").expect("free");
    assert!(gated(&dir, 4, OPEN, QUIET).try_take(1).expect("writable").is_ok(), "a load of 40 holds nothing at limit 0");

    let busy = Reading { load: Some([31.4, 20.0, 10.0]), ..QUIET };
    let limited = GateLimits { load_limit: 24, ..OPEN };
    let Err(waits) = gated(&dir, 4, limited, busy).try_take(1).expect("writable") else { panic!("over the limit") };
    assert_eq!(
        waits.why,
        Waits::Short { short: Short::Load { load: 31, limit: 24, window: LoadWindow::One }, while_short: 1 }
    );
    assert_eq!(
        waits.to_string(),
        "waiting for a Check slot: the one-minute load is 31, at or over 24, so Checks run one at a time"
    );
    let fifteen = GateLimits { load_window: LoadWindow::Fifteen, ..limited };
    assert!(gated(&dir, 4, fifteen, busy).try_take(1).expect("writable").is_ok(), "the fifteen-minute figure is 10");
    drop(one);
    assert_eq!(
        [1, 3, 5, 10, 15, 60].map(LoadWindow::of_minutes),
        [LoadWindow::One, LoadWindow::One, LoadWindow::Five, LoadWindow::Five, LoadWindow::Fifteen, LoadWindow::Fifteen]
    );
}

/// A machine that cannot be read gates nothing.
#[test]
fn an_unread_machine_gates_nothing() {
    let dir = Dir::new("unread");
    let slots = gated(&dir, 2, GateLimits { load_limit: 24, ..OPEN }, Reading::default());
    let one = slots.try_take(1).expect("writable").expect("free");
    assert!(slots.try_take(1).expect("writable").is_ok());
    drop(one);
}

#[tokio::test]
async fn a_second_check_in_one_checkout_waits_for_the_first_and_another_checkout_does_not() {
    let dir = Dir::new("one-per-checkout");
    let slots = CheckSlots::at(dir.0.join("slots"), 4);
    let (one, two) = (dir.0.join("one"), dir.0.join("two"));
    std::fs::create_dir_all(&one).expect("the directory is writable");
    std::fs::create_dir_all(&two).expect("the directory is writable");

    let first = slots.take_checkout(&one, || panic!("nothing holds it yet")).await.expect("writable");
    // Another checkout is its own, so it does not wait.
    let _other = slots.take_checkout(&two, || panic!("another checkout never waits")).await.expect("writable");

    let told = std::cell::Cell::new(0);
    let second = slots.take_checkout(&one, || told.set(told.get() + 1));
    tokio::pin!(second);
    let waited = tokio::time::timeout(Duration::from_millis(600), &mut second).await;
    assert!(waited.is_err(), "the second Check in one checkout waits while the first holds it");
    drop(first);
    let _second = tokio::time::timeout(Duration::from_secs(5), second)
        .await
        .expect("it starts once the first lets go")
        .expect("writable");
    assert_eq!(told.get(), 1, "it says once that it waits");
}
