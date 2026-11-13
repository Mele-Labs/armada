//! What another Mac shares is kept across a reopen, replaced whole per Mac, and forgotten per Mac.

use crate::tests::{open, TempDir};
use crate::{FromMac, KeptFromMac};

fn row(mac: &str, id: &str, body: &str) -> KeptFromMac {
    KeptFromMac { mac: mac.into(), id: id.into(), body: body.into() }
}

fn pairs(rows: &[(&str, &str)]) -> Vec<(String, String)> {
    rows.iter().map(|(id, body)| (id.to_string(), body.to_string())).collect()
}

#[test]
fn another_macs_rows_survive_a_reopen_and_a_replace_drops_what_it_leaves_out() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.replace_from_mac(FromMac::Sessions, "work", &pairs(&[("j1", "one"), ("j2", "two")])).expect("kept");
    store.replace_from_mac(FromMac::Sessions, "home", &pairs(&[("j9", "nine")])).expect("kept");
    drop(store);

    let mut store = open(&dir);
    let held = store.held_from_macs(FromMac::Sessions).expect("reads");
    assert_eq!(held, vec![row("home", "j9", "nine"), row("work", "j1", "one"), row("work", "j2", "two")]);

    store.replace_from_mac(FromMac::Sessions, "work", &pairs(&[("j2", "two, moved")])).expect("replaced");
    let held = store.held_from_macs(FromMac::Sessions).expect("reads");
    assert_eq!(held, vec![row("home", "j9", "nine"), row("work", "j2", "two, moved")], "j1 went, home stayed");
}

#[test]
fn forgetting_a_mac_drops_its_rows_and_reports_and_no_other_macs() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.keep_from_mac(FromMac::Sessions, "work", "j1", "one").expect("kept");
    store.keep_from_mac(FromMac::Sessions, "home", "j9", "nine").expect("kept");
    store.replace_from_mac(FromMac::BugReports, "work", &pairs(&[("b1", "report")])).expect("kept");

    store.forget_mac("work").expect("forgot");

    assert_eq!(store.held_from_macs(FromMac::Sessions).expect("reads"), vec![row("home", "j9", "nine")]);
    assert!(store.held_from_macs(FromMac::BugReports).expect("reads").is_empty());
}

#[test]
fn projects_held_from_a_mac_survive_a_reopen_and_a_replace_drops_what_it_leaves_out() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.replace_from_mac(FromMac::Projects, "work", &pairs(&[("000000:p1", "one"), ("000001:p2", "two")])).expect("kept");
    store.replace_from_mac(FromMac::Projects, "home", &pairs(&[("000000:p9", "nine")])).expect("kept");
    drop(store);

    let mut store = open(&dir);
    let held = store.held_from_macs(FromMac::Projects).expect("reads");
    assert_eq!(held, vec![row("home", "000000:p9", "nine"), row("work", "000000:p1", "one"), row("work", "000001:p2", "two")]);

    store.replace_from_mac(FromMac::Projects, "work", &pairs(&[("000000:p2", "two, first")])).expect("replaced");
    let held = store.held_from_macs(FromMac::Projects).expect("reads");
    assert_eq!(held, vec![row("home", "000000:p9", "nine"), row("work", "000000:p2", "two, first")], "p1 went, home stayed");
}

#[test]
fn edits_queued_for_a_mac_come_back_oldest_first_and_go_when_dropped() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let first = store.queue_project_act("work", 100, "one").expect("queued");
    let other = store.queue_project_act("home", 110, "other").expect("queued");
    let second = store.queue_project_act("work", 120, "two").expect("queued");
    drop(store);

    let mut store = open(&dir);
    let seen = |store: &crate::Store, mac: Option<&str>| {
        store.queued_project_acts(mac).expect("reads").into_iter().map(|one| (one.mac, one.at, one.act)).collect::<Vec<_>>()
    };
    assert_eq!(seen(&store, Some("work")), [("work".to_string(), 100, "one".to_string()), ("work".to_string(), 120, "two".to_string())]);
    assert_eq!(seen(&store, None).len(), 3);
    assert!(first < second && other < second, "the queue only grows");

    store.drop_project_act(first).expect("dropped");
    assert_eq!(seen(&store, Some("work")), [("work".to_string(), 120, "two".to_string())]);
}

#[test]
fn forgetting_a_mac_drops_its_projects_and_its_queued_edits_and_no_other_macs() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.replace_from_mac(FromMac::Projects, "work", &pairs(&[("000000:p1", "one")])).expect("kept");
    store.replace_from_mac(FromMac::Projects, "home", &pairs(&[("000000:p9", "nine")])).expect("kept");
    store.queue_project_act("work", 1, "edit").expect("queued");
    store.queue_project_act("home", 2, "edit").expect("queued");

    store.forget_mac("work").expect("forgot");

    assert_eq!(store.held_from_macs(FromMac::Projects).expect("reads"), vec![row("home", "000000:p9", "nine")]);
    let left = store.queued_project_acts(None).expect("reads");
    assert_eq!(left.iter().map(|one| one.mac.as_str()).collect::<Vec<_>>(), ["home"]);
}
