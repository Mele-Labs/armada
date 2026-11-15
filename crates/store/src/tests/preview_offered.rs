//! The branches Jobs offer for the preview: one a (Job, branch), replaced by a re-offer, judged at the
//! tip it was offered at, and the Refresh row he skipped.

use crate::tests::{open, TempDir};
use crate::KeptStep;

const B: &str = "armada/work";

#[test]
fn a_re_offer_replaces_the_offer_at_the_new_tip_as_checking_and_keeps_what_the_job_was_told() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.keep_preview_offered("s1", B, "t1", "one line", None, &[], "2026-11-13T09:00:00Z").expect("kept");
    store.keep_preview_checked("s1", B, "t1", "failed", Some("error[E0308]"), &["a.rs".to_string()], "2026-11-13T09:05:00Z").expect("checked");
    store.keep_preview_told("s1", B, "t1 failed").expect("told");

    store.keep_preview_offered("s1", B, "t2", "two lines", None, &[], "2026-11-13T10:00:00Z").expect("kept");

    let kept = store.preview_offered_of("s1", B).expect("reads").expect("an offer");
    assert_eq!((kept.tip.as_str(), kept.about.as_str(), kept.state.as_str()), ("t2", "two lines", "checking"));
    assert!(kept.why.is_none() && kept.files.is_empty() && kept.checked_at.is_none(), "{kept:?}");
    assert_eq!(kept.told.as_deref(), Some("t1 failed"));
    assert_eq!(store.preview_offered_all().expect("reads").len(), 1, "one row a Job and branch");
}

#[test]
fn a_check_lands_only_on_the_tip_it_was_made_at_and_offers_list_oldest_first() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.keep_preview_offered("s2", "b2", "u1", "later", None, &[], "2026-11-13T11:00:00Z").expect("kept");
    store.keep_preview_offered("s1", B, "t1", "earlier", None, &[], "2026-11-13T09:00:00Z").expect("kept");
    store.keep_preview_offered("s1", B, "t2", "earlier", None, &[], "2026-11-13T09:30:00Z").expect("re-offered while it was checked");

    assert!(!store.keep_preview_checked("s1", B, "t1", "ready", None, &[], "2026-11-13T09:40:00Z").expect("checked"), "t1 is not the offer now");
    assert!(store.keep_preview_checked("s1", B, "t2", "ready", None, &[], "2026-11-13T09:40:00Z").expect("checked"));

    let listed: Vec<(String, String)> = store.preview_offered_all().expect("reads").into_iter().map(|one| (one.branch, one.state)).collect();
    assert_eq!(listed, [(B.to_string(), "ready".to_string()), ("b2".to_string(), "checking".to_string())]);
    assert!(store.remove_preview_offered("s1", B).expect("removed"));
    assert!(!store.remove_preview_offered("s1", B).expect("removed"), "nothing left to take away");
}

#[test]
fn only_the_newest_skipped_refresh_is_kept_and_it_survives_a_reopen() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    assert_eq!(store.preview_refresh_skipped().expect("reads"), None);

    store.keep_preview_refresh_skipped("p-1", "2026-11-13T09:00:00Z").expect("kept");
    store.keep_preview_refresh_skipped("p-2", "2026-11-13T10:00:00Z").expect("kept");

    drop(store);
    let store = open(&dir);
    assert_eq!(store.preview_refresh_skipped().expect("reads").as_deref(), Some("p-2"));
}

fn step(say: &str, act: Option<&str>) -> KeptStep {
    KeptStep { say: say.to_string(), act: act.map(str::to_string) }
}

#[test]
fn an_offer_keeps_its_look_and_steps_and_its_note_outlives_the_offer() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let steps = [step("Press Projects", Some("open_projects")), step("Look at the toasts", None)];
    store.keep_preview_offered("s1", B, "t1", "one line", Some("The toasts stack"), &steps, "2026-11-13T09:00:00Z").expect("kept");

    let offer = store.preview_offered_of("s1", B).expect("reads").expect("an offer");
    assert_eq!((offer.look.as_deref(), offer.steps.as_slice()), (Some("The toasts stack"), steps.as_slice()));
    let note = store.build_note_of(B, "t1").expect("reads").expect("a note");
    assert_eq!((note.session_id.as_str(), note.about.as_str(), note.look.as_deref(), note.looked_at.as_deref()), ("s1", "one line", Some("The toasts stack"), None));
    assert_eq!(note.steps, steps);

    assert!(store.remove_preview_offered("s1", B).expect("removed"));
    assert_eq!(store.build_note_of(B, "t1").expect("reads"), Some(note), "the note stays when the offer goes");
}

#[test]
fn a_note_is_looked_at_for_its_tip_alone_and_a_re_offer_at_the_same_tip_keeps_the_mark() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.keep_preview_offered("s1", B, "t1", "first", Some("one"), &[], "2026-11-13T09:00:00Z").expect("kept");
    assert!(store.keep_build_note_looked_at(B, "t1", "2026-11-13T09:30:00Z").expect("marked"));
    assert!(store.keep_build_note_looked_at(B, "t1", "2026-11-13T09:45:00Z").expect("marked again"));
    assert!(!store.keep_build_note_looked_at(B, "t9", "2026-11-13T09:45:00Z").expect("no such tip"));
    assert_eq!(store.build_note_of(B, "t1").expect("reads").expect("a note").looked_at.as_deref(), Some("2026-11-13T09:30:00Z"), "the first time stands");

    store.keep_preview_offered("s1", B, "t1", "second", Some("two"), &[], "2026-11-13T10:00:00Z").expect("same tip again");
    let again = store.build_note_of(B, "t1").expect("reads").expect("a note");
    assert_eq!((again.about.as_str(), again.look.as_deref(), again.looked_at.is_some()), ("second", Some("two"), true), "updated, still looked at");

    store.keep_preview_offered("s1", B, "t2", "second", Some("two"), &[], "2026-11-13T11:00:00Z").expect("a new tip");
    assert_eq!(store.build_note_of(B, "t2").expect("reads").expect("a note").looked_at, None, "a new tip is a new note");
    assert!(store.build_note_of(B, "t1").expect("reads").expect("the old tip's note").looked_at.is_some());
}
