//! Remotes and links reduced to the same address, so they can be compared.

use crate::{issues_linked_in, linked_in, remote_of, ForgeAddress};

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

#[test]
fn the_issues_linked_into_one_repository_are_its_own_and_nothing_else() {
    let armada = address("https://github.com/mele-labs/armada");
    let words = "https://github.com/Mele-Labs/armada/issues/31#issuecomment-9 then \
                 (https://github.com/mele-labs/armada/issues/7), https://github.com/mele-labs/armada/pull/8, \
                 https://github.com/mele-labs/pocket/issues/9, #10 and https://github.com/mele-labs/armada/issues/7";
    assert_eq!(issues_linked_in(words, &armada), vec![7, 31]);
    assert!(issues_linked_in("#12 and issue 12", &armada).is_empty());
}
