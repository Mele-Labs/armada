//! The Helm Job's flag becomes `on_your_words` and is cleared, its two tables are dropped and the
//! Studio authors it wrote read as a Job's: `on_your_words.renamed`.
//!
//! Tested from the file the build before it wrote, through raw SQL, so what migrates is what the
//! owner already has.

use core_model::{StudioAuthor, StudioId, Ulid};
use rusqlite::Connection;

use crate::migrations::{Migration, MIGRATIONS};
use crate::tests::TempDir;
use crate::Store;

const AT: &str = "2026-11-05T09:00:00.000Z";
const NAME: &str = "on_your_words.renamed";

fn before() -> Vec<Migration> {
    MIGRATIONS.iter().copied().filter(|one| one.name != NAME).collect()
}

fn count(conn: &Connection, sql: &str) -> i64 {
    conn.query_row(sql, [], |row| row.get(0)).expect("a count")
}

/// A file with four Helm threads, one of them live and holding no slot, one live and holding one,
/// a plain Job, and a Studio Helm named and added to.
fn a_file_with_helm_threads(dir: &TempDir) {
    let store = Store::open_with(&dir.db(), &before()).expect("the file as the last build left it");
    let conn = &store.conn;
    for (id, state) in [("ended", "ended"), ("live", "live"), ("held", "live"), ("plain", "live")] {
        conn.execute(
            "INSERT INTO sessions (id, harness, origin, manifest_id, cwd, state, started_at, last_seen_at)
             VALUES (?1, 'hosted', 'bridge', 'armada', '/tmp/armada', ?2, ?3, ?3)",
            (id, state, AT),
        )
        .expect("a session");
        let helm = i64::from(id != "plain");
        let slot = (id == "held").then_some(3);
        conn.execute(
            "INSERT INTO hosted_sessions (session_id, manifest_id, mode, helm, lease_slot)
             VALUES (?1, 'armada', 'auto', ?2, ?3)",
            (id, helm, slot),
        )
        .expect("a hosting");
    }
    conn.execute("INSERT INTO helm_sessions (conversation, session_id, kept_at) VALUES ('armada', 'live', ?1)", [AT])
        .expect("a conversation");
    conn.execute("INSERT INTO helm_tuning (conversation, model, effort) VALUES ('armada', 'opus', NULL)", [])
        .expect("a tuning");
    conn.execute_batch(&format!(
        "INSERT INTO studios VALUES ('01OLD', 'armada', 'Named by Helm', '{AT}', '{AT}', 'helm');
         INSERT INTO studio_nodes (id, studio_id, kind, state, content, within, x, y, created_at, added_by)
             VALUES ('01FRAME', '01OLD', 'zone', NULL, '{{}}', NULL, 0, 0, '{AT}', 'person');
         INSERT INTO studio_nodes (id, studio_id, kind, state, content, within, x, y, created_at, added_by)
             VALUES ('01INSIDE', '01OLD', 'note', NULL, '{{\"said\":\"a\"}}', '01FRAME', 1, 1, '{AT}', 'helm');
         INSERT INTO studio_nodes (id, studio_id, kind, state, content, within, x, y, created_at, added_by)
             VALUES ('01PERSON', '01OLD', 'note', NULL, '{{\"said\":\"b\"}}', NULL, 9, 0, '{AT}', 'person');
         INSERT INTO studio_edges VALUES ('01E', '01OLD', '01INSIDE', '01PERSON', 'same_as', 'proposed', '{AT}', 'helm');"
    ))
    .expect("a Studio Helm wrote");
}

#[test]
fn every_row_reads_as_not_on_your_words_and_the_helm_tables_are_gone() {
    let dir = TempDir::new();
    a_file_with_helm_threads(&dir);

    let store = Store::open(&dir.db()).expect("the new build opens the file");
    assert_eq!(
        count(&store.conn, "SELECT count(*) FROM hosted_sessions WHERE on_your_words <> 0"),
        0,
        "no old thread keeps the flag"
    );
    assert_eq!(count(&store.conn, "SELECT count(*) FROM hosted_sessions"), 4, "no thread is deleted");
    for gone in ["helm_sessions", "helm_tuning"] {
        assert_eq!(
            count(&store.conn, &format!("SELECT count(*) FROM sqlite_master WHERE name = '{gone}'")),
            0,
            "{gone} is dropped"
        );
    }
    assert_eq!(
        count(&store.conn, "SELECT count(*) FROM pragma_table_info('job_work_plan_changes') WHERE name = 'by_job'"),
        1
    );
}

#[test]
fn a_helm_thread_still_live_and_holding_no_slot_is_ended_and_one_holding_a_slot_is_left() {
    let dir = TempDir::new();
    a_file_with_helm_threads(&dir);

    let store = Store::open(&dir.db()).expect("opens");
    let state = |id: &str| -> String {
        store
            .conn
            .query_row("SELECT state FROM sessions WHERE id = ?1", [id], |row| row.get(0))
            .expect("a session")
    };
    assert_eq!(state("live"), "ended");
    assert_eq!(state("held"), "live", "a slot is Fleet's to release");
    assert_eq!(state("plain"), "live", "a plain Job is not touched");
    assert_eq!(state("ended"), "ended");
}

#[test]
fn what_helm_wrote_on_a_studio_reads_as_a_jobs_and_nothing_moves() {
    let dir = TempDir::new();
    a_file_with_helm_threads(&dir);

    let store = Store::open(&dir.db()).expect("opens");
    let graph = store.studio(&StudioId::carried(Ulid::carried("01OLD"))).expect("reads");
    assert_eq!(graph.studio.named_by, Some(StudioAuthor::Job));
    let by = |id: &str| graph.nodes.iter().find(|node| node.id().as_str() == id).expect("a node").added_by();
    assert_eq!(by("01INSIDE"), Some(StudioAuthor::Job));
    assert_eq!(by("01PERSON"), Some(StudioAuthor::Person));
    assert_eq!(by("01FRAME"), Some(StudioAuthor::Person));
    assert_eq!(graph.edges[0].added_by(), Some(StudioAuthor::Job));
    assert_eq!(graph.nodes.len(), 3, "every node, the one inside a frame included");
}
