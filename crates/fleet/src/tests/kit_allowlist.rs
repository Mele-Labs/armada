//! Kit's allowlist, `~/.armada/allowed-commands`: read from a file, taken out of
//! by a person, and read back by `get_kit_inventory`. `docs/concepts/kit.md`.
//!
//! **Nothing here decides a permission.** A Drone's permission question went with
//! the Drone; the list a person built is what stays.

use std::path::PathBuf;

use ipc::{KitAllowedSource, WhatWasRead};
use testkit::FakeWorkProduct;

use crate::kit_allowlist::{parse, FILE};
use crate::tests::tmp::TempDir;

fn the_file(home: &TempDir) -> PathBuf {
    home.path().join("kit").join(FILE)
}

/// A person writing the file by hand.
fn written(home: &TempDir, text: &str) {
    let path = the_file(home);
    std::fs::create_dir_all(path.parent().expect("a folder")).expect("Kit's home");
    std::fs::write(path, text).expect("the file");
}

fn on_disk(home: &TempDir) -> String {
    std::fs::read_to_string(the_file(home)).unwrap_or_default()
}

/// **Lines are read as written**: notes and blanks skipped, a line Armada added
/// says where it came from, and one a person wrote needs nothing.
#[test]
fn the_file_reads_as_a_person_would_write_it() {
    let text = "# Commands.\n\nls -la\ngh issue view\talways allow\n\
                grep -a\tretro 01JOB-3\r\nrg\twhatever a person typed\n   \n";
    let rows: Vec<(String, KitAllowedSource)> = parse(text)
        .into_iter()
        .map(|row| (row.run, row.from))
        .collect();
    assert_eq!(
        rows,
        vec![
            (String::from("ls -la"), KitAllowedSource::ByHand),
            (String::from("gh issue view"), KitAllowedSource::AlwaysAllow),
            (
                String::from("grep -a"),
                KitAllowedSource::RetroItem {
                    lesson_id: String::from("01JOB-3")
                }
            ),
            (String::from("rg"), KitAllowedSource::ByHand),
        ]
    );
}

/// **Removing takes one line out and leaves the rest as a person left them**,
/// and a command the file does not hold is refused.
#[tokio::test]
async fn removing_takes_one_command_out_and_leaves_the_rest() {
    let home = TempDir::new();
    let fleet = crate::tests::daemon::a_fleet(&home, FakeWorkProduct::changed(&[]));
    written(
        &home,
        "# mine\nls -la\ngh issue view\talways allow\ngrep -a\tretro 01JOB-3\n",
    );

    let left = fleet
        .remove_from_kit_allowlist("gh issue view")
        .await
        .expect("it was there");
    let names: Vec<&str> = left.commands.iter().map(|row| row.run.as_str()).collect();
    assert_eq!(names, ["ls -la", "grep -a"]);
    assert_eq!(
        on_disk(&home),
        "# mine\nls -la\ngrep -a\tretro 01JOB-3\n",
        "the note and the other lines are as they were"
    );
    let missing = fleet.remove_from_kit_allowlist("gh issue view").await;
    assert!(missing.is_err(), "nothing is spelled that now");
}

/// **`get_kit_inventory` reads the allowlist rather than saying not read yet**,
/// with where each command came from and nothing else. A file that is there
/// and will not read is `not_read`, never an empty list.
#[tokio::test]
async fn the_inventory_reads_the_allowlist_with_where_each_came_from() {
    let home = TempDir::new();
    let fleet = crate::tests::daemon::a_fleet(&home, FakeWorkProduct::changed(&[]));
    let row = |inventory: ipc::KitInventory| {
        inventory
            .kinds
            .into_iter()
            .find(|row| row.kind == "allowlist")
            .expect("the allowlist is a kind of Kit")
            .read
    };

    let WhatWasRead::Read { items, .. } = row(fleet.kit_inventory().await) else {
        panic!("an absent file is an empty list, and it was looked at");
    };
    assert!(items.is_empty());

    written(
        &home,
        "ls -la\ngh issue view\talways allow\ngrep -a\tretro 01JOB-3\n",
    );
    let WhatWasRead::Read { items, unreadable } = row(fleet.kit_inventory().await) else {
        panic!("the allowlist is read");
    };
    assert!(unreadable.is_empty());
    let seen: Vec<(&str, &str, Option<&str>)> = items
        .iter()
        .map(|item| {
            (
                item.name.as_str(),
                item.source.as_str(),
                item.says.as_deref(),
            )
        })
        .collect();
    assert_eq!(
        seen,
        [
            ("ls -la", "written by hand", None),
            ("gh issue view", "always allow", None),
            ("grep -a", "retro item 01JOB-3", None),
        ],
        "the command text and where it came from, and nothing that could widen it"
    );

    std::fs::remove_file(the_file(&home)).expect("a file to replace");
    std::fs::create_dir(the_file(&home)).expect("a directory where the file is");
    assert!(
        matches!(
            row(fleet.kit_inventory().await),
            WhatWasRead::NotRead { .. }
        ),
        "a file that will not read is said so"
    );
}
