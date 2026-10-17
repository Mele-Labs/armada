//! Which repository a request's words are about: each signal, the order they
//! decide in, and the answers that are none.

use adapters::ForgeAddress;

use crate::repositories::reading::{read, Candidate};
use crate::tests::tmp::TempDir;

/// A repository with these files in it, under a directory named `directory`.
fn repository(home: &TempDir, directory: &str, manifest: &str, files: &[&str], sessions: usize) -> Candidate {
    let root = home.path().join(directory);
    for file in files {
        let path = root.join(file);
        std::fs::create_dir_all(path.parent().expect("a parent")).expect("a directory");
        std::fs::write(path, b"").expect("a file");
    }
    std::fs::create_dir_all(&root).expect("a root");
    Candidate::of(manifest, &root.to_string_lossy(), sessions)
}

fn pushing_to(mut one: Candidate, url: &str) -> Candidate {
    one.remote = ForgeAddress::of_remote(url);
    one
}

fn ids(read: &ipc::RequestRepository) -> (Option<&str>, Vec<&str>) {
    (
        read.manifest_id.as_ref().map(|id| id.as_str()),
        read.likely.iter().map(|id| id.as_str()).collect(),
    )
}

fn two(home: &TempDir) -> Vec<Candidate> {
    vec![
        pushing_to(repository(home, "storefront", "01STORE", &["src/cart.rs"], 3), "git@forge.test:acme/storefront.git"),
        pushing_to(repository(home, "pocket", "01POCKET", &["app/phone.ts"], 9), "https://forge.test/acme/pocket"),
    ]
}

#[test]
fn a_ticket_link_names_the_repository_that_pushes_to_it() {
    let home = TempDir::new();
    let read = read("look at https://forge.test/Acme/Pocket/issues/41 please", &two(&home));
    assert_eq!(ids(&read), (Some("01POCKET"), vec!["01POCKET", "01STORE"]));

    let read = self::read("https://forge.test/acme/storefront/pull/7", &two(&home));
    assert_eq!(ids(&read).0, Some("01STORE"));
}

#[test]
fn a_link_into_a_repository_nobody_serves_decides_nothing() {
    let home = TempDir::new();
    let read = read("https://forge.test/acme/elsewhere/issues/1", &two(&home));
    assert_eq!(ids(&read), (None, vec!["01POCKET", "01STORE"]));
}

#[test]
fn a_name_in_the_words_is_a_word_in_any_case() {
    let home = TempDir::new();
    let read = read("the Storefront totals are off", &two(&home));
    assert_eq!(ids(&read).0, Some("01STORE"));

    let by_id = self::read("fix 01pocket's build", &two(&home));
    assert_eq!(ids(&by_id).0, Some("01POCKET"), "the manifest id is a name too");
}

#[test]
fn half_a_word_is_not_a_name() {
    let home = TempDir::new();
    let read = read("a storefronts-page and pocketful of notes", &two(&home));
    assert_eq!(ids(&read).0, None);
}

#[test]
fn a_longer_name_wins_over_the_shorter_one_inside_it() {
    let home = TempDir::new();
    let both = vec![
        repository(&home, "armada", "01ARMADA", &[], 5),
        repository(&home, "armada-mods", "01MODS", &[], 1),
    ];
    let read = read("fix armada-mods themes", &both);
    assert_eq!(ids(&read).0, Some("01MODS"));

    let plain = self::read("fix armada themes", &both);
    assert_eq!(ids(&plain).0, Some("01ARMADA"));
}

#[test]
fn a_path_that_exists_in_one_tree_names_it() {
    let home = TempDir::new();
    let read = read("the bug is in src/cart.rs:42, I think", &two(&home));
    assert_eq!(ids(&read).0, Some("01STORE"));

    let mention = self::read("change @app/phone.ts to wait", &two(&home));
    assert_eq!(ids(&mention).0, Some("01POCKET"));

    let top = self::read("everything under app/ moves", &two(&home));
    assert_eq!(ids(&top).0, Some("01POCKET"));
}

#[test]
fn an_absolute_path_belongs_to_the_root_holding_it() {
    // Two-letter directories, which are no names, so only the path can decide.
    let home = TempDir::new();
    let all = vec![
        repository(&home, "aa", "01A", &[], 0),
        repository(&home, "bb", "01B", &[], 0),
    ];
    let inside = all[1].root.join("anything.ts");
    let read = read(&format!("see {}", inside.display()), &all);
    assert_eq!(ids(&read).0, Some("01B"));

    let outside = self::read("see /usr/local/bin/tool", &all);
    assert_eq!(ids(&outside).0, None);
}

#[test]
fn a_path_in_two_trees_or_in_none_decides_nothing() {
    let home = TempDir::new();
    let both = vec![
        repository(&home, "one", "01ONE", &["docs/guide.md"], 0),
        repository(&home, "two", "01TWO", &["docs/guide.md"], 0),
    ];
    assert_eq!(ids(&read("edit docs/guide.md", &both)).0, None);
    assert_eq!(ids(&read("edit docs/missing.md and ../up/there.rs", &both)).0, None);
}

#[test]
fn a_path_cannot_climb_out_of_the_repository() {
    let home = TempDir::new();
    let all = two(&home);
    std::fs::create_dir_all(home.path().join("shared")).expect("a sibling");
    std::fs::write(home.path().join("shared/x.rs"), b"").expect("a file");
    assert!(all[0].root.join("../shared/x.rs").exists(), "there to be found, if climbing were allowed");
    assert_eq!(ids(&read("see ../shared/x.rs", &all)).0, None);
}

#[test]
fn the_only_repository_served_is_the_one_meant() {
    let home = TempDir::new();
    let alone = vec![repository(&home, "storefront", "01STORE", &[], 0)];
    assert_eq!(ids(&read("make it faster", &alone)), (Some("01STORE"), vec!["01STORE"]));
}

#[test]
fn nothing_served_is_nothing_meant() {
    assert_eq!(ids(&read("make it faster", &[])), (None, vec![]));
}

#[test]
fn words_that_say_nothing_are_none_and_likely_is_by_sessions() {
    let home = TempDir::new();
    let read = read("make it faster", &two(&home));
    assert_eq!(ids(&read), (None, vec!["01POCKET", "01STORE"]), "pocket has nine Jobs to storefront's three");
}

#[test]
fn the_stronger_signal_decides_and_leads_the_ranking() {
    let home = TempDir::new();
    let read = read(
        "storefront needs https://forge.test/acme/pocket/issues/2 done",
        &two(&home),
    );
    assert_eq!(ids(&read), (Some("01POCKET"), vec!["01POCKET", "01STORE"]), "a link outranks a name");
}

#[test]
fn a_tier_that_names_two_stops_the_read_rather_than_falling_to_a_weaker_one() {
    let home = TempDir::new();
    let read = read(
        "storefront and pocket both, in src/cart.rs",
        &two(&home),
    );
    assert_eq!(ids(&read).0, None, "the names tied, and the path is not allowed to break it");
    assert_eq!(ids(&read).1[0], "01STORE", "though the path still ranks storefront first of the two");
}
