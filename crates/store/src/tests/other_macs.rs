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
