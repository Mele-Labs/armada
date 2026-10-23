//! A taught link rule is kept, read for a repository beside the every-repository ones, and forgotten.

use crate::tests::{open, TempDir};
use crate::TaughtLink;

fn rule(example: &str, repository: Option<&str>) -> TaughtLink {
    TaughtLink {
        id: None,
        pattern: "[A-Z][A-Z0-9]+-\\d+".to_string(),
        address: "https://myproject.jira.com/{match}".to_string(),
        example: example.to_string(),
        repository: repository.map(str::to_string),
        made_at: 10,
    }
}

#[test]
fn a_taught_link_comes_back_whole_after_a_reopen() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let id = store.save_taught_link(&rule("STORE-1234", Some("acme--shop"))).unwrap();
    drop(store);

    let store = open(&dir);
    let read = store.taught_links_for("acme--shop").unwrap();
    assert_eq!(read, vec![TaughtLink { id: Some(id), ..rule("STORE-1234", Some("acme--shop")) }]);
}

#[test]
fn a_repository_reads_its_own_rules_and_the_every_repository_ones() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.save_taught_link(&rule("SHOP-1", Some("acme--shop"))).unwrap();
    store.save_taught_link(&rule("BLOG-2", Some("acme--blog"))).unwrap();
    store.save_taught_link(&rule("ALL-3", None)).unwrap();

    let examples = |key: &str| -> Vec<String> {
        store.taught_links_for(key).unwrap().into_iter().map(|link| link.example).collect()
    };
    assert_eq!(examples("acme--shop"), ["SHOP-1", "ALL-3"]);
    assert_eq!(examples("acme--blog"), ["BLOG-2", "ALL-3"]);
    assert_eq!(examples("elsewhere"), ["ALL-3"]);
}

#[test]
fn a_forgotten_rule_is_gone_and_forgetting_it_again_says_so() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let kept = store.save_taught_link(&rule("SHOP-1", Some("acme--shop"))).unwrap();
    let other = store.save_taught_link(&rule("ALL-3", None)).unwrap();

    assert!(store.forget_taught_link(kept).unwrap());
    assert!(!store.forget_taught_link(kept).unwrap());
    let left: Vec<Option<i64>> = store.taught_links_for("acme--shop").unwrap().into_iter().map(|l| l.id).collect();
    assert_eq!(left, [Some(other)]);
}
