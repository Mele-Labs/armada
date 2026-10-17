//! Where changes run, remembered per area, and the bays walks hold, both still there after a restart.

use crate::tests::{open, TempDir};
use crate::{HeldReviewWalk, KeptWalkPlace};

fn place(id: &str, area: &str, address: &str) -> KeptWalkPlace {
    KeptWalkPlace {
        id: id.to_string(),
        manifest_id: "storefront".to_string(),
        area: area.to_string(),
        how: "pull".to_string(),
        address: address.to_string(),
        command: "npm run dev".to_string(),
        api: String::new(),
        saved_at: "2026-10-17T09:00:00Z".to_string(),
    }
}

#[test]
fn a_place_is_kept_per_repository_and_area_and_saving_again_replaces_it_under_the_same_id() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let first = store.keep_walk_place(&place("p1", "apps/web", "http://localhost:3000")).unwrap();
    assert_eq!(first.id, "p1");
    let again = store.keep_walk_place(&place("p2", "apps/web", "http://localhost:4000")).unwrap();
    assert_eq!((again.id.as_str(), again.address.as_str()), ("p1", "http://localhost:4000"), "one place an area");
    store.keep_walk_place(&place("p3", "apps/api", "http://localhost:5000")).unwrap();
    let mut other = place("p4", "apps/web", "http://localhost:6000");
    other.manifest_id = "elsewhere".to_string();
    store.keep_walk_place(&other).unwrap();

    let areas: Vec<String> = store.walk_places_of("storefront").unwrap().into_iter().map(|one| one.area).collect();
    assert_eq!(areas, ["apps/api", "apps/web"]);
    assert_eq!(store.walk_places().unwrap().len(), 3, "across repositories");
}

#[test]
fn forgetting_a_place_is_by_repository_and_id_and_a_missing_one_is_not_an_error() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.keep_walk_place(&place("p1", "apps/web", "http://localhost:3000")).unwrap();
    store.forget_walk_place("elsewhere", "p1").unwrap();
    assert_eq!(store.walk_places().unwrap().len(), 1, "another repository's forget leaves it");
    store.forget_walk_place("storefront", "p1").unwrap();
    store.forget_walk_place("storefront", "p1").unwrap();
    assert!(store.walk_places().unwrap().is_empty());
}

#[test]
fn a_held_walk_keeps_its_bay_until_it_is_released() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let held = HeldReviewWalk {
        id: "w1".to_string(),
        manifest_id: "storefront".to_string(),
        number: 7,
        slot: 3,
        holder: "review-w1".to_string(),
        path: "/slots/slot-3".to_string(),
        opened_at: "2026-10-17T09:00:00Z".to_string(),
    };
    store.keep_review_walk(&held).unwrap();
    assert_eq!(store.review_walks_held().unwrap(), [held]);
    store.release_review_walk("w1").unwrap();
    store.release_review_walk("w1").unwrap();
    assert!(store.review_walks_held().unwrap().is_empty());
}
