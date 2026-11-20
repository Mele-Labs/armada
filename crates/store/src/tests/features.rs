//! The feature library survives a reopen, keeps an id through a re-seed, and answers what a page is made of.

use crate::tests::{open, TempDir};
use crate::{feature_link_kind, feature_section, feature_state, KeptArea, KeptFeature, KeptFeatureLink, KeptFeatureMerge, KeptSection};

fn feature(id: &str, slug: &str, paths: &[&str]) -> KeptFeature {
    KeptFeature {
        id: id.to_string(),
        project: "p1".to_string(),
        area: "takes".to_string(),
        slug: slug.to_string(),
        title: "Voting on takes".to_string(),
        summary: "Pick, rank or score a page's takes.".to_string(),
        state: feature_state::SEEDED.to_string(),
        manifest_id: String::new(),
        entry_points: vec!["crates/fleet/src/session_host/page_votes.rs:40".to_string()],
        paths: paths.iter().map(|one| one.to_string()).collect(),
        position: 0,
        added_at: "2026-11-16T09:00:00.000Z".to_string(),
    }
}

#[test]
fn a_feature_reads_back_with_its_area_in_order_and_keeps_its_id_through_a_re_seed() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.put_feature_area(&KeptArea { project: "p1".into(), slug: "takes".into(), title: "Takes".into(), position: 1 }).unwrap();
    store.put_feature_area(&KeptArea { project: "p1".into(), slug: "jobs".into(), title: "Jobs".into(), position: 0 }).unwrap();
    let first = store.put_feature(&feature("f-1", "voting-on-takes", &["crates/fleet/src/session_host/page_votes.rs"])).unwrap();
    let again = store.put_feature(&feature("f-2", "voting-on-takes", &["crates/fleet/src/session_host/"])).unwrap();
    assert_eq!((first.as_str(), again.as_str()), ("f-1", "f-1"), "a slug the project has keeps the id it was given first");
    drop(store);

    let store = open(&dir);
    let areas = store.feature_areas("p1").unwrap();
    assert_eq!(areas.iter().map(|one| one.slug.as_str()).collect::<Vec<_>>(), ["jobs", "takes"]);
    let held = store.features_in("p1").unwrap();
    assert_eq!(held.len(), 1);
    assert_eq!(held[0].paths, ["crates/fleet/src/session_host/"], "the prefixes of the re-seed replace the first's");
    assert_eq!(held[0].entry_points, ["crates/fleet/src/session_host/page_votes.rs:40"]);
    assert!(store.feature_by_id("f-1").unwrap().is_some());
    assert!(store.features_in("other").unwrap().is_empty(), "a feature is its project's alone");
}

#[test]
fn a_section_is_rewritten_in_place_and_listed_in_the_order_a_page_shows_them() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.put_feature(&feature("f-1", "voting-on-takes", &[])).unwrap();
    let write = |section: &str, text: &str, at: &str| KeptSection {
        feature_id: "f-1".into(),
        section: section.into(),
        text: text.into(),
        job: "s-1".into(),
        written_at: at.into(),
        at_commit: "abc1234".into(),
    };
    store.put_feature_section(&write(feature_section::DEBT, "none", "2026-11-16T10:00:00.000Z")).unwrap();
    store.put_feature_section(&write(feature_section::TEST, "armada check test page_votes", "2026-11-16T10:00:01.000Z")).unwrap();
    store.put_feature_section(&write(feature_section::DEBT, "one vote kind is untested", "2026-11-16T11:00:00.000Z")).unwrap();
    let held = store.feature_sections("f-1").unwrap();
    assert_eq!(held.iter().map(|one| one.section.as_str()).collect::<Vec<_>>(), ["test", "debt"]);
    assert_eq!(held[1].text, "one vote kind is untested");
    assert_eq!(held[1].written_at, "2026-11-16T11:00:00.000Z");
}

#[test]
fn a_link_keeps_its_first_sighting_and_a_job_is_asked_for_its_own() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let link = |at: &str| KeptFeatureLink {
        feature_id: "f-1".into(),
        kind: feature_link_kind::PULL_REQUEST.into(),
        target: "812".into(),
        job: "s-1".into(),
        title: String::new(),
        state: String::new(),
        via: "detector".into(),
        at: at.into(),
        gist: String::new(),
        gist_by: String::new(),
    };
    assert!(store.put_feature_link(&link("2026-11-16T10:00:00.000Z")).unwrap());
    assert!(!store.put_feature_link(&link("2026-11-16T12:00:00.000Z")).unwrap(), "seen again is not a new link");
    let held = store.feature_links_of_job("s-1", feature_link_kind::PULL_REQUEST).unwrap();
    assert_eq!(held.len(), 1);
    assert_eq!(held[0].at, "2026-11-16T10:00:00.000Z");
    assert!(store.feature_links_of_job("s-2", feature_link_kind::PULL_REQUEST).unwrap().is_empty());
}

#[test]
fn files_weigh_the_jobs_that_wrote_them_and_the_heaviest_come_first() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    for at in ["2026-11-16T10:00:00.000Z", "2026-11-16T10:01:00.000Z"] {
        store.keep_feature_write("f-1", "s-1", "a.rs", at).unwrap();
    }
    store.keep_feature_write("f-1", "s-2", "a.rs", "2026-11-16T10:02:00.000Z").unwrap();
    store.keep_feature_write("f-1", "s-1", "b.rs", "2026-11-16T10:03:00.000Z").unwrap();
    assert_eq!(store.feature_top_files("f-1", 5).unwrap(), [("a.rs".to_string(), 2), ("b.rs".to_string(), 1)], "a file weighs the Jobs that wrote it");
    let mine = store.feature_files_of_job("s-1").unwrap();
    assert_eq!(mine.iter().map(|one| one.path.as_str()).collect::<Vec<_>>(), ["a.rs", "b.rs"]);
    assert_eq!(mine[0].first_at, "2026-11-16T10:00:00.000Z");
    assert_eq!(mine[0].last_at, "2026-11-16T10:01:00.000Z");
}

#[test]
fn merges_are_kept_once_and_read_after_a_moment_oldest_first() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let merge = |number: u64, at: &str, files: &[&str]| KeptFeatureMerge {
        manifest_id: "m1".into(),
        number,
        merged_at: at.into(),
        at_commit: String::new(),
        job: String::new(),
        files: files.iter().map(|one| one.to_string()).collect(),
    };
    assert!(store.keep_feature_merge(&merge(7, "2026-11-16T10:00:00.000Z", &["a.rs"])).unwrap());
    assert!(!store.keep_feature_merge(&merge(7, "2026-11-16T10:00:00.000Z", &["other.rs"])).unwrap());
    assert!(store.keep_feature_merge(&merge(9, "2026-11-16T12:00:00.000Z", &["b.rs"])).unwrap());
    let after = store.feature_merges_after("m1", "2026-11-16T09:00:00.000Z").unwrap();
    assert_eq!(after.iter().map(|one| one.number).collect::<Vec<_>>(), [7, 9]);
    assert_eq!(after[0].files, ["a.rs"]);
    assert_eq!(store.feature_merges_after("m1", "2026-11-16T10:00:00.000Z").unwrap().len(), 1);
    assert_eq!(store.feature_merge_newest("m1").unwrap(), Some((9, "2026-11-16T12:00:00.000Z".to_string())));
    assert!(store.feature_merge_newest("m2").unwrap().is_none());
}

#[test]
fn a_feature_a_job_named_is_kept_once_with_its_first_naming() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    assert!(store.keep_feature_job_named("f-1", "s-1", "2026-11-18T10:00:00.000Z").unwrap());
    assert!(!store.keep_feature_job_named("f-1", "s-1", "2026-11-18T11:00:00.000Z").unwrap(), "a repeat is not a new naming");
    assert!(store.keep_feature_job_named("f-2", "s-1", "2026-11-18T12:00:00.000Z").unwrap());
    assert_eq!(store.feature_ids_named_by_job("s-1").unwrap(), ["f-1", "f-2"]);
    assert!(store.feature_ids_named_by_job("s-2").unwrap().is_empty());
}
