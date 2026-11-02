//! Remotes and links reduced to the same address, so they can be compared.

use crate::{issues_linked_in, linked_in, links_in, remote_of, ForgeAddress};

use super::repo::TempRepo;

fn address(url: &str) -> ForgeAddress {
    ForgeAddress::of_remote(url).expect("a forge address")
}

#[test]
fn a_remote_reads_the_same_in_each_of_its_spellings() {
    let https = address("https://github.com/Mele-Labs/armada.git");
    assert_eq!(https, address("git@github.com:mele-labs/armada.git"));
    assert_eq!(https, address("ssh://git@github.com:22/Mele-Labs/armada"));
    assert_eq!(https, address("https://user:token@GitHub.com/Mele-Labs/armada/"));
}

#[test]
fn the_slug_is_owner_and_name_in_lower_case_whichever_way_it_was_cloned() {
    assert_eq!(address("git@github.com:Mele-Labs/Armada.git").slug(), "mele-labs/armada");
    assert_eq!(address("https://github.com/Mele-Labs/armada").slug(), "mele-labs/armada");
}

#[test]
fn a_path_on_this_machine_is_no_forge() {
    assert_eq!(ForgeAddress::of_remote("/srv/git/armada.git"), None);
    assert_eq!(ForgeAddress::of_remote("file:///srv/git/armada.git"), None);
    assert_eq!(ForgeAddress::of_remote("https://github.com/only-an-owner"), None);
}

#[test]
fn an_issue_or_pull_request_link_names_its_repository() {
    let words = "see https://github.com/Mele-Labs/armada/issues/12#issuecomment-9 and \
                 (https://github.com/mele-labs/pocket/pull/7/files), plus https://example.com/a/b";
    assert_eq!(
        linked_in(words),
        vec![
            address("https://github.com/mele-labs/armada"),
            address("https://github.com/mele-labs/pocket"),
        ]
    );
}

#[test]
fn a_link_that_is_not_a_ticket_names_nothing() {
    assert!(linked_in("https://github.com/mele-labs/armada").is_empty());
    assert!(linked_in("https://github.com/mele-labs/armada/issues").is_empty());
    assert!(linked_in("https://github.com/mele-labs/armada/issues/new").is_empty());
    assert!(linked_in("https://github.com/mele-labs/armada/blob/main/a.rs").is_empty());
}

#[test]
fn a_checkouts_origin_is_its_address() {
    let repo = TempRepo::with_a_commit();
    assert_eq!(remote_of(repo.root()), None, "no remote at all");

    repo.git(&["remote", "add", "upstream", "git@github.com:other/armada.git"]);
    assert_eq!(remote_of(repo.root()), Some(address("https://github.com/other/armada")));

    repo.git(&["remote", "add", "origin", "https://github.com/mele-labs/armada.git"]);
    assert_eq!(remote_of(repo.root()), Some(address("https://github.com/mele-labs/armada")));
}

/// Run `git` in a directory that is not a `TempRepo` root, and assert it worked.
fn git_in(at: &std::path::Path, args: &[&str]) {
    let run = std::process::Command::new("git")
        .arg("-C")
        .arg(at)
        .args(args)
        .output()
        .expect("git on PATH");
    assert!(run.status.success(), "git {args:?} failed: {}", String::from_utf8_lossy(&run.stderr));
}

/// A cold `Repository::open` fails where `.git/HEAD` is missing and a kept answer does not, so
/// taking HEAD away is how a test sees that no cold open happened.
#[test]
fn a_checkouts_address_is_answered_from_memory_until_its_config_moves() {
    let repo = TempRepo::with_a_commit();
    let a = "https://github.com/mele-labs/armada";
    repo.git(&["remote", "add", "origin", a]);
    assert_eq!(remote_of(repo.root()), Some(address(a)), "read once, opening it");

    let head = repo.root().join(".git/HEAD");
    let away = repo.root().join(".git/HEAD.away");
    std::fs::rename(&head, &away).unwrap();
    assert_eq!(
        remote_of(repo.root()),
        Some(address(a)),
        "a second read answers from memory: opening the repository again fails with HEAD gone"
    );

    std::fs::rename(&away, &head).unwrap();
    let b = "https://github.com/mele-labs/elsewhere";
    repo.git(&["remote", "set-url", "origin", b]);
    assert_eq!(remote_of(repo.root()), Some(address(b)), "a moved config is read again");
}

#[test]
fn a_directory_that_is_no_repository_is_not_remembered_as_one() {
    let repo = TempRepo::with_a_commit();
    let plain = repo.root().with_extension("plain");
    std::fs::create_dir_all(&plain).unwrap();
    assert_eq!(remote_of(&plain), None, "not a repository yet");

    git_in(&plain, &["init", "-q"]);
    git_in(&plain, &["remote", "add", "origin", "https://github.com/mele-labs/armada"]);
    assert_eq!(
        remote_of(&plain),
        Some(address("https://github.com/mele-labs/armada")),
        "a miss is not kept: the directory became a repository"
    );
}

#[test]
fn a_linked_worktree_answers_for_its_main_checkouts_remote_and_sees_it_move() {
    let repo = TempRepo::with_a_commit();
    let a = "https://github.com/mele-labs/armada";
    repo.git(&["remote", "add", "origin", a]);
    let linked = repo.root().with_extension("linked");
    repo.git(&["worktree", "add", "-q", "-b", "linked", &linked.to_string_lossy()]);
    let linked = linked.canonicalize().unwrap();

    assert_eq!(remote_of(&linked), Some(address(a)), "the worktree reads its main checkout's config");

    let b = "https://github.com/mele-labs/elsewhere";
    repo.git(&["remote", "set-url", "origin", b]);
    assert_eq!(remote_of(&linked), Some(address(b)), "set-url from the main checkout is seen through the worktree");
    assert_eq!(remote_of(repo.root()), Some(address(b)));
}

#[test]
fn the_issues_linked_into_one_repository_are_its_own_and_nothing_else() {
    let armada = address("https://github.com/mele-labs/armada");
    let words = "https://github.com/Mele-Labs/armada/issues/31#issuecomment-9 then \
                 (https://github.com/mele-labs/armada/issues/7), https://github.com/mele-labs/armada/pull/8, \
                 https://github.com/mele-labs/pocket/issues/9, #10 and https://github.com/mele-labs/armada/issues/7";
    assert_eq!(issues_linked_in(words, &armada), vec![7, 31]);
    assert!(issues_linked_in("#12 and issue 12", &armada).is_empty());
}

#[test]
fn a_reply_keeps_the_pages_it_names_and_not_the_ones_on_this_machine_or_this_repositorys_tickets() {
    let armada = address("https://github.com/mele-labs/armada");
    let words = "Read https://docs.example.com/guide/start, then open http://localhost:5361/?walk=x \
                 and https://github.com/Mele-Labs/armada/pull/12 for the change.";
    assert_eq!(links_in(words, Some(&armada)), vec!["https://docs.example.com/guide/start"]);
}

#[test]
fn a_link_loses_the_punctuation_that_ended_its_sentence() {
    let armada = address("https://github.com/mele-labs/armada");
    assert_eq!(links_in("See https://example.com/a.", Some(&armada)), vec!["https://example.com/a"]);
    assert_eq!(links_in("(https://example.com/a?b=1, or https://example.com/c!)", Some(&armada)), vec![
        "https://example.com/a?b=1",
        "https://example.com/c",
    ]);
}

#[test]
fn a_markdown_link_yields_its_address() {
    let armada = address("https://github.com/mele-labs/armada");
    assert_eq!(links_in("the [spec](https://example.com/spec#part) says", Some(&armada)), vec!["https://example.com/spec#part"]);
}

#[test]
fn a_link_is_kept_once_and_in_the_order_it_was_written() {
    let armada = address("https://github.com/mele-labs/armada");
    let words = "https://b.example.com/ https://a.example.com/ https://b.example.com/";
    assert_eq!(links_in(words, Some(&armada)), vec!["https://b.example.com/", "https://a.example.com/"]);
}

#[test]
fn only_web_addresses_are_links() {
    let armada = address("https://github.com/mele-labs/armada");
    let words = "git@github.com:a/b.git ssh://git@example.com/a/b file:///tmp/x ftp://example.com/x https://example.com/ok";
    assert_eq!(links_in(words, Some(&armada)), vec!["https://example.com/ok"]);
}

#[test]
fn every_spelling_of_this_machine_is_dropped() {
    let armada = address("https://github.com/mele-labs/armada");
    let words = "http://localhost/ http://127.0.0.1:8080/x http://[::1]:3000/ http://0.0.0.0:1/ \
                 http://app.localhost:5173/ http://user@LOCALHOST:9/ https://notlocalhost.example.com/";
    assert_eq!(links_in(words, Some(&armada)), vec!["https://notlocalhost.example.com/"]);
}

#[test]
fn another_repositorys_tickets_are_links_and_this_ones_are_not() {
    let armada = address("https://github.com/mele-labs/armada");
    let words = "https://github.com/mele-labs/armada/issues/7 https://github.com/Mele-Labs/Armada/pull/8/files \
                 https://github.com/mele-labs/pocket/issues/9 https://github.com/mele-labs/armada/blob/main/a.rs";
    assert_eq!(links_in(words, Some(&armada)), vec![
        "https://github.com/mele-labs/pocket/issues/9",
        "https://github.com/mele-labs/armada/blob/main/a.rs",
    ]);
}

#[test]
fn a_checkout_with_no_forge_drops_only_what_is_on_this_machine() {
    let words = "https://github.com/mele-labs/armada/pull/12 http://localhost:1/";
    assert_eq!(links_in(words, None), vec!["https://github.com/mele-labs/armada/pull/12"]);
}
