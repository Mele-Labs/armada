//! A quick terminal's flag reads as `side_job` after `hosted_sessions.side_job`, from the file the
//! build before it wrote.

use crate::migrations::{Migration, MIGRATIONS};
use crate::tests::TempDir;
use crate::Store;

const AT: &str = "2026-11-13T09:00:00.000Z";

fn before() -> Vec<Migration> {
    MIGRATIONS.iter().copied().filter(|one| one.name != "hosted_sessions.side_job").collect()
}

#[test]
fn a_quick_terminal_kept_before_the_rename_reads_as_a_side_job_and_a_plain_job_does_not() {
    let dir = TempDir::new();
    {
        let store = Store::open_with(&dir.db(), &before()).expect("the file as the last build left it");
        for (id, quick) in [("side", 1), ("plain", 0)] {
            store
                .conn
                .execute(
                    "INSERT INTO sessions (id, harness, origin, manifest_id, cwd, state, started_at, last_seen_at)
                     VALUES (?1, 'hosted', 'bridge', 'armada', '/tmp/armada', 'live', ?2, ?2)",
                    (id, AT),
                )
                .expect("a session");
            store
                .conn
                .execute(
                    "INSERT INTO hosted_sessions (session_id, manifest_id, mode, quick_terminal)
                     VALUES (?1, 'armada', 'auto', ?2)",
                    (id, quick),
                )
                .expect("a hosting");
        }
    }

    let store = Store::open(&dir.db()).expect("the new build opens the file");
    let side_job = |id: &str| store.hosting(id).expect("reads").expect("kept").side_job;
    assert!(side_job("side"));
    assert!(!side_job("plain"));
}
