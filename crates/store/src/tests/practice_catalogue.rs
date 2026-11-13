//! The practices read from a repository's docs, kept by area and kind for the next pull request.

use crate::tests::{open, TempDir};
use crate::KeptPractices;

fn entry(area: &str, kind: &str, read_at: &str) -> KeptPractices {
    KeptPractices {
        manifest_id: "storefront".to_string(),
        area: area.to_string(),
        kind: kind.to_string(),
        items: r#"[{"item":"Every migration has a down step","source":"docs/migrations.md"}]"#.to_string(),
        sources: r#"[{"path":"docs/migrations.md","blob":"b1"}]"#.to_string(),
        read_at: read_at.to_string(),
    }
}

#[test]
fn practices_are_kept_by_repository_area_and_kind_and_a_new_read_replaces_them() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let kept = entry("db", "migration", "2026-11-10T09:00:00Z");
    store.keep_practices(&kept).expect("kept");
    assert_eq!(store.practices("storefront", "db", "migration").expect("read"), Some(kept));
    assert_eq!(store.practices("storefront", "db", "logic").expect("read"), None);
    assert_eq!(store.practices("storefront", "web", "migration").expect("read"), None);
    assert_eq!(store.practices("elsewhere", "db", "migration").expect("read"), None);

    let again = entry("db", "migration", "2026-11-12T09:00:00Z");
    store.keep_practices(&again).expect("kept");
    drop(store);
    let store = open(&dir);
    assert_eq!(store.practices("storefront", "db", "migration").expect("read"), Some(again), "one entry a key, after a reopen");
}
