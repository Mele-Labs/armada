//! Migrations are named, recorded by name, and tolerant of a newer file when
//! what the newer build added was additive. `docs/practices/store-migrations.md`.

use std::fs;

use rusqlite::Connection;

use crate::error::OpenError;
use crate::migrations::{Migration, LEGACY_COUNT, MIGRATIONS, SCHEMA_VERSION_KEY};
use crate::tests::TempDir;
use crate::Store;

const A: &str = "CREATE TABLE branch_a_things (id TEXT PRIMARY KEY) STRICT;";
const B: &str = "CREATE TABLE branch_b_things (id TEXT PRIMARY KEY) STRICT;";

/// The real list with `extra` after it, which is what a branch that appended
/// its own entries ships.
fn with(extra: &[Migration]) -> Vec<Migration> {
    MIGRATIONS.iter().chain(extra).copied().collect()
}

fn applied(store: &Store) -> Vec<(String, bool)> {
    let mut rows = store
        .conn
        .prepare("SELECT name, additive FROM armada_migrations ORDER BY rowid")
        .expect("the table");
    rows.query_map([], |row| Ok((row.get(0)?, row.get::<_, i64>(1)? == 1)))
        .expect("rows")
        .map(|row| row.expect("a row"))
        .collect()
}

fn has_table(store: &Store, name: &str) -> bool {
    store
        .conn
        .query_row(
            "SELECT count(*) FROM sqlite_master WHERE name = ?1",
            (name,),
            |row| row.get::<_, i64>(0),
        )
        .expect("a count")
        == 1
}

#[test]
fn the_list_has_unique_names_and_the_numbered_list_for_a_prefix() {
    let mut seen = std::collections::BTreeSet::new();
    for migration in MIGRATIONS {
        assert!(!migration.name.is_empty());
        assert!(
            seen.insert(migration.name),
            "{} is on the list twice",
            migration.name
        );
    }
    assert!(MIGRATIONS.len() >= LEGACY_COUNT);
    assert_eq!(MIGRATIONS[0].name, "schema.v1");
    assert_eq!(MIGRATIONS[LEGACY_COUNT - 1].name, "main_ci.v114");
}

/// A file written by the numbered list at every count it ever had opens, takes
/// every later name, and does not re-run the earlier ones: `CREATE TABLE` with
/// no `IF NOT EXISTS`, which most of them are, would fail if it did.
#[test]
fn a_numbered_file_converts_at_every_count_the_old_list_had() {
    let origin = TempDir::new();
    let conn = Connection::open(origin.db()).expect("a file");
    conn.pragma_update(None, "journal_mode", "WAL")
        .expect("wal");
    let want: Vec<&str> = MIGRATIONS.iter().map(|m| m.name).collect();

    for count in 1..=LEGACY_COUNT {
        conn.execute_batch(MIGRATIONS[count - 1].sql)
            .expect("the next migration");
        conn.execute(
            "INSERT INTO armada_meta (key, value) VALUES (?1, ?2)
             ON CONFLICT (key) DO UPDATE SET value = excluded.value",
            (SCHEMA_VERSION_KEY, count.to_string()),
        )
        .expect("the count");
        conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE)")
            .expect("checkpoint");

        let copy = TempDir::new();
        fs::copy(origin.db(), copy.db()).expect("a copy of the file at this count");
        let store = Store::open(&copy.db())
            .unwrap_or_else(|e| panic!("a file at {count} converts and opens: {e}"));
        let names: Vec<String> = applied(&store).into_iter().map(|(n, _)| n).collect();
        let mut sorted = names.clone();
        sorted.sort();
        let mut expected: Vec<String> = want.iter().map(|n| n.to_string()).collect();
        expected.sort();
        assert_eq!(sorted, expected, "at {count}");
        assert_eq!(
            &names[..count],
            &want[..count],
            "the first {count} names are the converted ones, in list order"
        );
    }
}

#[test]
fn two_branches_each_adding_a_migration_both_apply_in_either_merge_order() {
    let a = Migration::additive("branch_a.things", A);
    let b = Migration::additive("branch_b.things", B);

    for merged in [with(&[a, b]), with(&[b, a])] {
        let dir = TempDir::new();
        let store = Store::open_with(&dir.db(), &merged).expect("a fresh file");
        assert!(has_table(&store, "branch_a_things") && has_table(&store, "branch_b_things"));
    }

    // A file one branch already migrated, then the merge of both: the other
    // branch's migration is applied even though it sorts before one already in.
    for (first, merged) in [(a, with(&[b, a])), (b, with(&[a, b])), (a, with(&[a, b]))] {
        let dir = TempDir::new();
        drop(Store::open_with(&dir.db(), &with(&[first])).expect("the branch's own file"));
        let store = Store::open_with(&dir.db(), &merged).expect("the merged list opens it");
        assert!(has_table(&store, "branch_a_things") && has_table(&store, "branch_b_things"));
    }
}

#[test]
fn a_file_with_extra_additive_names_is_opened_by_an_older_build() {
    let dir = TempDir::new();
    let newer = with(&[Migration::additive("unlanded_branch.things", A)]);
    drop(Store::open_with(&dir.db(), &newer).expect("the newer build"));

    let store = Store::open(&dir.db()).expect("an older build accepts additive names it lacks");
    assert_eq!(store.unknown_migrations(), ["unlanded_branch.things"]);
    drop(store);

    // And the newer build still opens it afterwards, applying nothing twice.
    Store::open_with(&dir.db(), &newer).expect("back again");
}

#[test]
fn a_file_with_an_extra_breaking_name_is_refused_by_an_older_build() {
    let dir = TempDir::new();
    let newer = with(&[
        Migration::additive("branch_a.things", A),
        Migration::breaking(
            "branch_b.rebuilt",
            "ALTER TABLE branch_a_things RENAME TO things;",
        ),
    ]);
    drop(Store::open_with(&dir.db(), &newer).expect("the newer build"));

    match Store::open(&dir.db()) {
        Err(OpenError::BreakingMigrationsFromTheFuture { names, .. }) => {
            assert_eq!(names, ["branch_b.rebuilt"]);
        }
        other => panic!("expected a refusal, found {other:?}"),
    }
}

/// Words of `sql` outside comments and string literals, upper-cased.
fn words(sql: &str) -> Vec<String> {
    let mut out = String::new();
    let mut chars = sql.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '-' if chars.peek() == Some(&'-') => while chars.next_if(|&n| n != '\n').is_some() {},
            '/' if chars.peek() == Some(&'*') => {
                let mut last = ' ';
                for n in chars.by_ref() {
                    if last == '*' && n == '/' {
                        break;
                    }
                    last = n;
                }
            }
            '\'' => {
                for n in chars.by_ref() {
                    if n == '\'' {
                        break;
                    }
                }
                out.push(' ');
            }
            c if c.is_alphanumeric() || c == '_' => out.extend(c.to_uppercase()),
            _ => out.push(' '),
        }
    }
    out.split_whitespace().map(str::to_string).collect()
}

/// What in `sql` makes it not additive, if anything: a drop, a rename, or a
/// statement that rewrites rows. An `INSERT` only adds rows, and the event a
/// trigger names (`BEFORE UPDATE`) and a foreign key's `ON DELETE` are not a rewrite.
pub(super) fn not_additive(sql: &str) -> Option<String> {
    let words = words(sql);
    words.iter().enumerate().find_map(|(at, word)| {
        let event = at > 0 && matches!(words[at - 1].as_str(), "BEFORE" | "AFTER" | "OF" | "ON");
        let rewrites = matches!(
            word.as_str(),
            "DROP" | "RENAME" | "UPDATE" | "DELETE" | "REPLACE"
        );
        (rewrites && !event).then(|| word.clone())
    })
}

#[test]
fn the_additive_rule_sees_a_drop_a_rename_and_a_rewrite() {
    for sql in [
        "DROP TABLE old;",
        "alter table a rename to b;",
        "ALTER TABLE a RENAME COLUMN x TO y;",
        "ALTER TABLE a DROP COLUMN x;",
        "UPDATE jobs SET x = 1;",
        "DELETE FROM jobs;",
        "INSERT OR REPLACE INTO a VALUES (1);",
        "CREATE TABLE a (id TEXT); /* ok */ drop index i;",
    ] {
        assert!(not_additive(sql).is_some(), "{sql} is not additive");
    }
    for sql in [
        "CREATE TABLE a (id TEXT) STRICT;",
        "CREATE INDEX i ON a (id);",
        "CREATE TABLE c (a TEXT REFERENCES a ON DELETE CASCADE ON UPDATE CASCADE);",
        "INSERT INTO a SELECT id FROM b;",
        "ALTER TABLE a ADD COLUMN x TEXT NOT NULL DEFAULT '';",
        "-- DROP TABLE a;\nCREATE TABLE b (note TEXT DEFAULT 'drop table x');",
        "CREATE TRIGGER t BEFORE UPDATE ON a BEGIN SELECT RAISE(ABORT, 'no'); END;",
    ] {
        assert_eq!(not_additive(sql), None, "{sql} is additive");
    }
}

/// The flag is true: nothing declared additive drops, renames or rewrites.
/// A migration that must is declared `Migration::breaking`.
#[test]
fn an_entry_declared_additive_drops_renames_and_rewrites_nothing() {
    let lies: Vec<String> = MIGRATIONS
        .iter()
        .filter(|m| !m.breaking)
        .filter_map(|m| not_additive(m.sql).map(|w| format!("{} ({w})", m.name)))
        .collect();
    assert!(
        lies.is_empty(),
        "declared additive, and not: {lies:?}. Declare them with Migration::breaking"
    );
}

/// The files of `dir` as a list that follows the real one, leaked because a
/// `Migration` holds `'static` text.
fn list_from(dir: &std::path::Path) -> Vec<Migration> {
    let files =
        crate::migration_files::read_dir(&dir.join("crates/store/migrations")).expect("files");
    let mut list: Vec<Migration> = MIGRATIONS[..LEGACY_COUNT].to_vec();
    for file in files {
        let name: &'static str = Box::leak(file.name.into_boxed_str());
        let sql: &'static str = Box::leak(
            std::fs::read_to_string(&file.path)
                .expect("sql")
                .into_boxed_str(),
        );
        list.push(if file.breaking {
            Migration::breaking(name, sql)
        } else {
            Migration::additive(name, sql)
        });
    }
    list
}

fn root_of(dir: &TempDir) -> std::path::PathBuf {
    dir.db().parent().expect("a parent").to_path_buf()
}

fn git(dir: &std::path::Path, args: &[&str]) -> String {
    let out = std::process::Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["-c", "user.name=t", "-c", "user.email=t@t"])
        .args(args)
        .output()
        .expect("git");
    assert!(out.status.success(), "git {args:?}: {out:?}");
    String::from_utf8_lossy(&out.stdout).into_owned()
}

/// Two branches each add a migration file; `git merge` of both, in either
/// order, is clean and the merged directory applies both. No shared file is
/// touched, which is what a forge's merge needs: it ignores `merge=union`.
#[test]
fn two_branches_each_adding_a_migration_file_merge_cleanly_and_both_apply() {
    for first in ["a", "b"] {
        let repo = TempDir::new();
        let root = &root_of(&repo);
        let dir = root.join("crates/store/migrations");
        fs::create_dir_all(&dir).expect("dir");
        fs::write(dir.join(".gitkeep"), "").expect("keep");
        git(root, &["init", "-q", "-b", "main"]);
        git(root, &["add", "."]);
        git(root, &["commit", "-q", "-m", "base"]);
        for (branch, file, sql) in [
            ("a", "20261007T0130Z-branch_a.things.sql", A),
            ("b", "20261007T0200Z-branch_b.things.sql", B),
        ] {
            git(root, &["checkout", "-q", "-b", branch, "main"]);
            fs::write(dir.join(file), sql).expect("file");
            git(root, &["add", "."]);
            git(root, &["commit", "-q", "-m", branch]);
        }
        let second = if first == "a" { "b" } else { "a" };
        git(root, &["checkout", "-q", "-b", "merged", first]);
        git(root, &["merge", "-q", "--no-edit", second]);

        let db = TempDir::new();
        let store = Store::open_with(&db.db(), &list_from(root)).expect("the merged directory");
        assert!(has_table(&store, "branch_a_things") && has_table(&store, "branch_b_things"));
    }
}

#[test]
fn a_migration_file_is_named_by_timestamp_and_slug_and_may_say_it_is_breaking() {
    let repo = TempDir::new();
    let dir = root_of(&repo).join("crates/store/migrations");
    fs::create_dir_all(&dir).expect("dir");
    fs::write(
        dir.join("20261007T0130Z-a.one.sql"),
        "-- note\n-- breaking\nDROP TABLE x;",
    )
    .expect("f");
    fs::write(dir.join("20261007T0100Z-a.zero.sql"), A).expect("f");
    let read = crate::migration_files::read_dir(&dir).expect("read");
    let seen: Vec<(&str, bool)> = read.iter().map(|m| (m.name.as_str(), m.breaking)).collect();
    assert_eq!(seen, [("a.zero", false), ("a.one", true)], "by timestamp");
    for bad in ["a.one.sql", "2026-a.one.sql", "20261007T0130Z-NoDot.sql"] {
        fs::write(dir.join(bad), A).expect("f");
        assert!(crate::migration_files::read_dir(&dir).is_err(), "{bad}");
        fs::remove_file(dir.join(bad)).expect("rm");
    }
}

/// What is compiled in is what the directory says, and every file in it holds
/// to the additive rule unless it declares itself breaking.
#[test]
fn the_compiled_list_is_the_directory_after_the_legacy_entries() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("migrations");
    let files = crate::migration_files::read_dir(&dir).expect("files");
    let compiled: Vec<&str> = MIGRATIONS[LEGACY_COUNT..].iter().map(|m| m.name).collect();
    let on_disk: Vec<&str> = files.iter().map(|f| f.name.as_str()).collect();
    assert_eq!(compiled, on_disk);
}
