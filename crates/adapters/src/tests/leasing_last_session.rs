//! The Session a process holder was acting for, kept on the slot's record so it outlives the process.

use std::path::Path;

use adapter_traits::SlotHeld;

use crate::leasing::{note_session, Holder, Leased, Pool};
use crate::tests::repo::TempRepo;

fn no_seed(_: &Path, _: &Path) -> Result<(), String> {
    Err(String::from("nothing to seed from"))
}

fn here() -> Holder {
    Holder::of(std::process::id()).expect("this process is running")
}

/// A slot leased by this process, which is alive, and the record's path.
fn leased(repo: &TempRepo, holder: &Holder) -> (Pool, std::path::PathBuf, String) {
    let pool = Pool::at(repo.root(), 1, "main", Vec::new());
    let Ok(Leased::Took(lease)) = pool.try_lease("half-done", holder, 7, &no_seed) else {
        panic!("no slot taken");
    };
    let slot = lease.path().to_path_buf();
    let record = format!("{}.lease", slot.display());
    (pool, slot, record)
}

/// **The holder cannot say whose it was once it is gone**, so a live process holder's record is
/// written with the Session whose tree it is in, and a stranded reading gives it back.
#[test]
fn a_stranded_slot_names_the_session_its_holder_was_in() {
    let repo = TempRepo::with_a_commit();
    let (pool, slot, record) = leased(&repo, &here());
    note_session(&slot, "sess-1").expect("written");

    let text = std::fs::read_to_string(&record).unwrap();
    assert!(text.contains("\nsession sess-1\n"), "{text}");
    assert_eq!(pool.readings()[0].last_session, None, "held, so not stranded");

    // The process ends: the same record, a start time no process has.
    let started = match here() {
        Holder::Process { started, .. } => started,
        Holder::Hosted(_) => unreachable!(),
    };
    std::fs::write(&record, text.replace(&started, "a start no process has")).unwrap();
    std::fs::write(slot.join("left.rs"), "fn left() {}\n").unwrap();

    let read = pool.readings();
    assert!(matches!(read[0].held, SlotHeld::Stranded(_)), "{:?}", read[0].held);
    assert_eq!(read[0].last_session.as_deref(), Some("sess-1"));
}

/// A record written before the line existed is read as it was, with no Session.
#[test]
fn a_record_without_the_line_names_no_session() {
    let repo = TempRepo::with_a_commit();
    let (pool, slot, record) = leased(&repo, &here());
    let text = std::fs::read_to_string(&record).unwrap();
    assert!(!text.contains("session"), "{text}");

    let started = match here() {
        Holder::Process { started, .. } => started,
        Holder::Hosted(_) => unreachable!(),
    };
    std::fs::write(&record, text.replace(&started, "a start no process has")).unwrap();
    std::fs::write(slot.join("left.rs"), "fn left() {}\n").unwrap();

    let read = pool.readings();
    assert!(matches!(read[0].held, SlotHeld::Stranded(_)));
    assert_eq!(read[0].last_session, None);
}

/// Only a live process is written on: a Job's record already names the Session it was leased for,
/// and a holder that is gone is not one whose tree anyone is in.
#[test]
fn only_a_live_process_holder_is_written_on() {
    let repo = TempRepo::with_a_commit();
    let (_, slot, record) = leased(&repo, &Holder::hosted("sess-2"));
    let before = std::fs::read_to_string(&record).unwrap();
    note_session(&slot, "sess-2").expect("nothing to do");
    assert_eq!(std::fs::read_to_string(&record).unwrap(), before);
}
