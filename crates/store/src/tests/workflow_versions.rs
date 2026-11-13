//! The versions a workflow has had in a place, and what a copy of one of Armada's began from: kept in order across a reopen,
//! one place's and no other's, and dropped with the copy.

use crate::tests::{open, TempDir};
use crate::{KeptBase, NewVersion, WorkflowPlace, VERSION_FOUND, VERSION_PUBLISHED};

const AT: &str = "2026-11-13T09:00:00.000Z";
const LATER: &str = "2026-11-13T09:05:00.000Z";

fn place(source: &str) -> WorkflowPlace {
    WorkflowPlace { source: source.into(), item_id: "feature".into(), ..Default::default() }
}

fn new<'a>(text: &'a str, hash: &'a str, at: &'a str) -> NewVersion<'a> {
    NewVersion { text, hash, at }
}

fn base(version: u32, text: &str) -> KeptBase {
    KeptBase { version, hash: format!("hash-of-{text}"), text: text.into() }
}

#[test]
fn versions_round_trip_in_order_newest_first_and_a_found_text_is_kept_only_where_it_is_not_the_latest() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    assert!(store.workflow_versions(&place("kit")).unwrap().is_empty(), "nothing was published");

    let first = store.record_workflow_publish(&place("kit"), Some(&new("{found}", "h0", AT)), &new("{one}", "h1", AT), None).unwrap();
    let second = store.record_workflow_publish(&place("kit"), Some(&new("{one}", "h1", LATER)), &new("{two}", "h2", LATER), None).unwrap();
    let third = store.record_workflow_publish(&place("kit"), Some(&new("{by hand}", "h3", LATER)), &new("{three}", "h4", LATER), None).unwrap();
    assert_eq!(
        (first, second, third),
        (2, 3, 5),
        "the text found takes a number before the publish's own row where it is not the latest kept: first, and after a hand edit"
    );
    drop(store);

    let store = open(&dir);
    let kept: Vec<_> = store.workflow_versions(&place("kit")).unwrap().into_iter().map(|one| (one.version, one.text, one.hash, one.at, one.origin)).collect();
    let row = |version: u32, text: &str, hash: &str, at: &str, origin: &str| (version, text.to_string(), hash.to_string(), at.to_string(), origin.to_string());
    assert_eq!(
        kept,
        [
            row(5, "{three}", "h4", LATER, VERSION_PUBLISHED),
            row(4, "{by hand}", "h3", LATER, VERSION_FOUND),
            row(3, "{two}", "h2", LATER, VERSION_PUBLISHED),
            row(2, "{one}", "h1", AT, VERSION_PUBLISHED),
            row(1, "{found}", "h0", AT, VERSION_FOUND),
        ]
    );
    assert_eq!(store.workflow_version(&place("kit"), 2).unwrap().expect("kept").text, "{one}");
    assert!(store.workflow_version(&place("kit"), 6).unwrap().is_none());
}

#[test]
fn where_nothing_was_found_the_first_publish_is_version_one_and_a_place_is_one_copys_alone() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    assert_eq!(store.record_workflow_publish(&place("kit"), None, &new("{one}", "h1", AT), None).unwrap(), 1);
    let repository = |manifest: &str| WorkflowPlace { manifest_id: manifest.into(), ..place("repository") };
    let project = |name: &str| WorkflowPlace { project: name.into(), ..place("project") };
    store.record_workflow_publish(&repository("one"), None, &new("{r1}", "h", AT), None).unwrap();
    store.record_workflow_publish(&project("storefront"), None, &new("{p}", "h", AT), None).unwrap();
    store.record_workflow_publish(&WorkflowPlace { item_id: "bug".into(), ..place("kit") }, None, &new("{bug}", "h", AT), None).unwrap();

    assert_eq!(store.workflow_versions(&place("kit")).unwrap().len(), 1);
    assert_eq!(store.workflow_versions(&repository("one")).unwrap()[0].text, "{r1}");
    assert!(store.workflow_versions(&repository("two")).unwrap().is_empty(), "a repository's is its own");
    assert_eq!(store.workflow_versions(&project("storefront")).unwrap()[0].text, "{p}");
    assert!(store.workflow_versions(&project("backoffice")).unwrap().is_empty());
}

#[test]
fn a_base_round_trips_a_publish_may_replace_it_and_a_drop_takes_it_and_the_versions_with_it() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    assert!(store.workflow_base(&place("kit")).unwrap().is_none(), "a copy made before this has none");

    store.record_workflow_publish(&place("kit"), None, &new("{one}", "h1", AT), Some(&base(1, "{carried 1}"))).unwrap();
    drop(store);
    let mut store = open(&dir);
    assert_eq!(store.workflow_base(&place("kit")).unwrap(), Some(base(1, "{carried 1}")));
    store.put_workflow_base(&place("kit"), &base(2, "{carried 2}")).unwrap();
    assert_eq!(store.workflow_base(&place("kit")).unwrap(), Some(base(2, "{carried 2}")));
    store.put_workflow_base(&place("repository"), &base(1, "{other}")).unwrap();
    let mut heads = store.workflow_base_heads().unwrap();
    heads.sort_by(|one, two| one.0.source.cmp(&two.0.source));
    assert_eq!(heads, [(place("kit"), 2, "hash-of-{carried 2}".to_string()), (place("repository"), 1, "hash-of-{other}".to_string())]);

    store.drop_workflow_place(&place("kit"), None).unwrap();
    assert!(store.workflow_base(&place("kit")).unwrap().is_none());
    assert!(store.workflow_versions(&place("kit")).unwrap().is_empty());
    assert!(store.workflow_base(&place("repository")).unwrap().is_some(), "another place's stays");

    store.record_workflow_publish(&place("kit"), None, &new("{one}", "h1", AT), None).unwrap();
    store.drop_workflow_place(&place("kit"), Some(&base(3, "{carried 3}"))).unwrap();
    assert_eq!(store.workflow_base(&place("kit")).unwrap(), Some(base(3, "{carried 3}")), "a reset keeps the carried text as the new base");
    assert!(store.workflow_versions(&place("kit")).unwrap().is_empty());
}
