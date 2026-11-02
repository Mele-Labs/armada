//! Sleep's tables go, and a night's answer the person has not reviewed reaches them as a card to
//! review. The old rows are written the way Sleep wrote them, into a file at the migration before.

use crate::migrations::MIGRATIONS;
use crate::tests::TempDir;
use crate::Store;

const NIGHT_BEGAN: &str = "2026-10-30T22:00:00.000Z";

fn leave(store: &Store, id: &str, kind: &str, session: Option<&str>, chose: Option<&str>, reviewed: i64) {
    store
        .conn
        .execute(
            "INSERT INTO sleep_rows (id, kind, who, text, session_id, chose, reviewed)
             VALUES (?1, ?2, 'Fix the clock', 'Which clock?', ?3, ?4, ?5)",
            rusqlite::params![id, kind, session, chose, reviewed],
        )
        .expect("a row of the night");
}

fn ledger(store: &Store, session: &str, manifest: &str) {
    store
        .conn
        .execute(
            "INSERT INTO sessions (id, harness, origin, manifest_id, cwd, state, started_at, last_seen_at)
             VALUES (?1, 'harness', 'hosted', ?2, '/repo', 'live', ?3, ?3)",
            rusqlite::params![session, manifest, NIGHT_BEGAN],
        )
        .expect("a ledger row");
}

fn has_table(store: &Store, name: &str) -> bool {
    store
        .conn
        .query_row("SELECT count(*) FROM sqlite_master WHERE name = ?1", [name], |row| row.get::<_, i64>(0))
        .expect("a count")
        == 1
}

#[test]
fn an_unreviewed_answer_of_the_night_is_a_card_and_the_tables_go() {
    let dir = TempDir::new();
    let before = MIGRATIONS.iter().position(|one| one.name == "sleep.folded").expect("the migration is listed");
    {
        let old = Store::open_with(&dir.db(), &MIGRATIONS[..before]).expect("a file as Sleep left it");
        assert!(has_table(&old, "sleep_rows"), "Sleep's tables are there before");
        old.conn.execute("UPDATE sleep_state SET on_now = 0, since = ?1", [NIGHT_BEGAN]).expect("a night");
        ledger(&old, "S1", "one");
        ledger(&old, "S2", "two");
        old.conn
            .execute(
                "INSERT INTO hosted_sessions (session_id, manifest_id, mode) VALUES ('S1', 'one', 'default')",
                [],
            )
            .expect("a hosted row");
        leave(&old, "S1:a", "decided", Some("S1"), Some("Frozen (Recommended)"), 0);
        leave(&old, "S1:b", "decided", Some("S1"), Some("Fake"), 1);
        leave(&old, "S2:a", "decided", Some("S2"), None, 0);
        leave(&old, "GONE:a", "decided", Some("GONE"), Some("Frozen (Recommended)"), 0);
        leave(&old, "S1:c", "blocked", Some("S1"), None, 0);
        leave(&old, "S1:d", "walk", None, None, 0);
    }

    let store = Store::open(&dir.db()).expect("the file opens and is migrated");

    let cards = store.autopilot_decisions_to_review().expect("reads");
    let seen: Vec<_> = cards.iter().map(|one| (one.id.as_str(), one.manifest_id.as_str(), one.chose.as_str())).collect();
    assert_eq!(seen, [("S1:a", "one", "Frozen (Recommended)"), ("S2:a", "two", "")], "the reviewed, held and shown rows, and the one whose Job is gone, are not cards");
    let card = &cards[0];
    assert_eq!(
        (card.kind.as_str(), card.because.as_str(), card.at.as_str(), card.who.as_str(), card.asked.as_str()),
        ("recommended", "", NIGHT_BEGAN, "Fix the clock", "Which clock?")
    );
    assert_eq!(card.session_id.as_deref(), Some("S1"), "a correction still reaches the Job");
    assert!(!card.reviewed && card.corrected.is_none());
    assert!(!has_table(&store, "sleep_rows") && !has_table(&store, "sleep_state"), "Sleep's tables are dropped");
}
