//! `get_health`'s disk: additive, so a Fleet that does not send it still reads, and one with
//! nothing to say of it leaves no key.

use crate::{decode, encode, ActionAuthority, DiskHealth, FleetHealth, ReclaimPass};

const GIB: u64 = 1024 * 1024 * 1024;

fn health(disk: Option<DiskHealth>) -> FleetHealth {
    FleetHealth { probes: Vec::new(), not_probed: Vec::new(), action_authority: ActionAuthority::Acting, workflows_left_out: false, disk }
}

#[test]
fn a_disk_with_a_pass_and_folders_to_drop_round_trips() {
    let disk = DiskHealth {
        free_bytes: 20 * GIB,
        reclaim_floor_bytes: 100 * GIB,
        hold_floor_bytes: 30 * GIB,
        last_pass: Some(ReclaimPass { at: "2026-11-13T10:00:00.000Z".into(), freed_bytes: 8 * GIB }),
        next_to_drop: vec!["/r/.armada/slots/slot-2/target".into()],
    };
    let sent = health(Some(disk));
    let json = encode(&sent).expect("health is plain data");
    assert_eq!(decode::<FleetHealth>("health", json.as_bytes()).expect("it round-trips"), sent);
}

#[test]
fn a_disk_with_no_pass_and_nothing_to_drop_leaves_those_keys_out() {
    let disk = DiskHealth { free_bytes: 1, reclaim_floor_bytes: 0, hold_floor_bytes: 2, last_pass: None, next_to_drop: Vec::new() };
    let json = encode(&health(Some(disk))).expect("health is plain data");
    assert!(json.contains(r#""disk":{"free_bytes":1,"reclaim_floor_bytes":0,"hold_floor_bytes":2}"#), "{json}");
    assert!(decode::<FleetHealth>("health", json.as_bytes()).is_ok());
}

#[test]
fn a_health_answer_without_a_disk_still_reads_and_writes_no_disk_key() {
    let older = r#"{"probes":[],"not_probed":[],"action_authority":"acting","workflows_left_out":false}"#;
    let read: FleetHealth = decode("health", older.as_bytes()).expect("an older Fleet's answer reads");
    assert_eq!(read.disk, None);
    assert_eq!(encode(&read).expect("health is plain data"), older);
}
