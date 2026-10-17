//! A Job held back because the machine has too little left.
//!
//! **The pre-spawn half only.** Every case here is a Job that has not started.
//! A Job that runs out of something while running escalates as
//! `resource_exhausted` and nothing raises that yet — the last case asserts it
//! stays that way, because the cheapest way to conflate the two failures is to
//! make the poll fire on a Job it was never meant to see.
//!
//! **The board and admission are asserted together, never apart.** Each of them
//! alone can pass while the two disagree, which is the drift one predicate
//! exists to prevent.
//!
//! **Over 500 lines, and left as one file.** The parsers, the threshold and the
//! Fleet cases are one claim proved at three depths — a split would put the
//! proof that `df` is read correctly in a different file from the proof that a
//! full disk holds a Job, and it is the pairing that catches a change to
//! either.

use crate::headroom::{
    disk_free, load_in_use, memory_in_use, Bytes, Headroom, InUse, Machine, Reading,
    Short, Spare, TheMachine,
};

/// The threshold that ships. Every fixture is held against the real one.
const SHIPPED: Headroom = Headroom::of(Spare::percent(15), Bytes::gibibytes(10));

/// A machine with room to spare on all three.
const PLENTY: Reading = Reading::of(
    InUse::percent(10),
    InUse::percent(20),
    Bytes::gibibytes(500),
);

/// A machine with plenty of everything, and no way to change its mind.
///
/// **What every fixture in this crate but this file's gets.** Reading the real
/// machine in `tests::daemon::fittings` would make every test here pass or fail
/// on whatever else the operator happens to be running.
pub struct Plentiful;

impl Machine for Plentiful {
    fn read(&self) -> Option<Reading> {
        Some(PLENTY)
    }

    fn disk_free_at(&self, _path: &std::path::Path) -> Option<Bytes> {
        Some(PLENTY.disk_free())
    }
}

// ------------------------------------------------------------- the threshold

/// Enough of all three is no answer at all.
#[test]
fn a_machine_with_room_is_short_of_nothing() {
    assert_eq!(SHIPPED.short_of(&PLENTY), None);
}

/// Each resource on its own, so a passing suite cannot be one signal doing the
/// work of two.
#[test]
fn each_resource_holds_a_job_back_on_its_own() {
    let no_disk = Reading::of(InUse::percent(10), InUse::percent(20), Bytes::gibibytes(9));
    let no_memory = Reading::of(
        InUse::percent(10),
        InUse::percent(95),
        Bytes::gibibytes(500),
    );

    assert_eq!(SHIPPED.short_of(&no_disk), Some(Short::Disk));
    assert_eq!(SHIPPED.short_of(&no_memory), Some(Short::Memory));
}

/// **CPU never refuses**, however far past its cores the machine is. The
/// operating system schedules CPU; a Fleet that yielded to it was the only
/// thing on a machine loaded to 27 on 10 cores that did.
#[test]
fn a_saturated_cpu_is_short_of_nothing() {
    let saturated = Reading::of(
        InUse::percent(270),
        InUse::percent(20),
        Bytes::gibibytes(500),
    );
    assert_eq!(SHIPPED.short_of(&saturated), None);
}

/// Exactly at the threshold is enough. The refusal is short *of* it, not at it,
/// so a machine sitting on the number is not held back for ever.
#[test]
fn a_machine_exactly_at_the_threshold_has_room() {
    let exactly = Reading::of(InUse::percent(85), InUse::percent(85), Bytes::gibibytes(10));
    assert_eq!(SHIPPED.short_of(&exactly), None);
}

/// Disk is named first when more than one is short: it is the one that has
/// actually run out here, and the only one whose exhaustion destroys work.
#[test]
fn disk_is_the_one_named_when_everything_is_short() {
    let nothing_left = Reading::of(InUse::percent(99), InUse::percent(99), Bytes::gibibytes(0));
    assert_eq!(SHIPPED.short_of(&nothing_left), Some(Short::Disk));
}

/// A load above the cores there are reads as over capacity rather than as full,
/// so the difference between busy and hopeless survives the reading.
#[test]
fn more_runnable_work_than_cores_reads_past_full() {
    assert_eq!(
        load_in_use("load averages: 25.00 9.60 6.61", 10),
        Some(InUse::percent(250))
    );
    assert_eq!(InUse::percent(250).spare(), Spare::percent(0));
}

// ---------------------------------------------------------------- the parsers

/// Both spellings of `uptime`. Darwin writes three bare numbers and Linux
/// writes three comma-separated ones, and the last three fields are the
/// averages on both.
#[test]
fn the_load_average_is_read_off_either_platforms_uptime() {
    let darwin = "15:39  up  6:07, 8 users, load averages: 4.00 9.60 6.61";
    let linux = "15:39:12 up 3 days,  2:11,  5 users,  load average: 4.00, 0.58, 0.59";

    assert_eq!(load_in_use(darwin, 8), Some(InUse::percent(50)));
    assert_eq!(load_in_use(linux, 8), Some(InUse::percent(50)));
}

/// A machine reporting nothing readable is not a machine reporting zero.
#[test]
fn an_unreadable_uptime_is_no_reading() {
    assert_eq!(load_in_use("", 8), None);
    assert_eq!(load_in_use("uptime: command not found", 8), None);
}

/// `ps` writes one share per line, and a process that exits mid-walk can leave
/// a short one. The short line is skipped rather than taking the reading down.
#[test]
fn memory_is_the_sum_of_what_ps_printed() {
    assert_eq!(
        memory_in_use(" 12.4\n  0.6\n 30.0\n"),
        Some(InUse::percent(43))
    );
    assert_eq!(memory_in_use(" 12.4\n\n  0.6\n"), Some(InUse::percent(13)));
    assert_eq!(memory_in_use("\n"), None);
}

/// The fourth field of `df -P -k`, in kibibytes, as bytes.
#[test]
fn the_free_disk_is_the_fourth_field_of_df() {
    let said = "Filesystem   1024-blocks      Used Available Capacity  Mounted on\n\
                /dev/disk3s5   482797652 227768264   1048576    50%    /System/Volumes/Data\n";
    assert_eq!(disk_free(said), Some(Bytes::gibibytes(1)));
    assert_eq!(disk_free("Filesystem 1024-blocks Used Available\n"), None);
}

// ------------------------------------------------------- against the machine

/// **The one case that asks the operating system.** Every parser above is fed a
/// string somebody typed, and a string somebody typed cannot tell you the flags
/// are right — `df` without `-P` wraps a long device name onto its own line and
/// moves every column. This reads the machine the tests are running on and
/// asserts only that all three came back, because the numbers are whatever the
/// machine happens to be doing.
#[test]
fn the_real_machine_answers_all_three() {
    let read = TheMachine::watching(".")
        .read()
        .expect("uptime, ps and df all answered");

    assert!(
        read.disk_free().count() > 0,
        "a volume with no bytes free at all would not be running this test"
    );
    assert!(read.memory().percentage() > 0, "something is using memory");
}

// ------------------------------------------------------------ against a Fleet

// -------------------------------------------------------- one Fleet, two volumes

// ----------------------------------------------- a person's act, and the queue

