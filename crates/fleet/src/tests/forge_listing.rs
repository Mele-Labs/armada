//! `list_forge_repositories`: the forge account's repositories, each marked with the served
//! repository that is it, narrowed by `q`, capped, and read from the forge once a minute. A
//! scripted forge stands in.

use std::sync::Arc;

use adapter_traits::ForgeRepository;
use axum::http::StatusCode;
use ipc::{ForgeRepositories, WireError};

use super::repositories::{a_fleet_reading, served, Planted};
use crate::repositories::forge_listing::{MOST_LISTED, MOST_READ};
use crate::tests::http::call;
use crate::tests::tmp::TempDir;

fn on_the_forge(owner: &str, name: &str) -> ForgeRepository {
    ForgeRepository {
        owner: owner.to_string(),
        name: name.to_string(),
        url: format!("https://forge.invalid/{owner}/{name}.git"),
        private: owner == "mele",
        pushed_at: Some("2026-11-12T10:00:00Z".to_string()),
    }
}

fn listing(body: &[u8]) -> ForgeRepositories {
    ipc::decode("the forge's repositories", body).expect("a listing")
}

fn names(listed: &ForgeRepositories) -> Vec<String> {
    listed.repositories.iter().map(|one| format!("{}/{}", one.owner, one.name)).collect()
}

#[tokio::test]
async fn the_forges_repositories_are_listed_and_the_one_served_here_is_marked_with_its_root() {
    let home = TempDir::new();
    let fleet = Arc::new(a_fleet_reading(&home, &Arc::new(Planted::nothing())));
    let root = fleet.repositories().served()[0].root().to_string();
    std::fs::create_dir_all(&root).expect("the served root");
    // The forge names it in capitals; the checkout's remote does not.
    for args in [&["init", "-q"][..], &["remote", "add", "origin", "https://forge.invalid/mele/shop.git"]] {
        let ran = std::process::Command::new("git").args(args).current_dir(&root).output().expect("git");
        assert!(ran.status.success(), "{args:?}");
    }
    let mut shop = on_the_forge("Mele", "Shop");
    shop.private = true;
    fleet.vcs().account_lists(vec![shop, on_the_forge("other", "notes")], false);
    let app = served(&fleet);

    let (status, body) = call(&app, "GET", "/repositories/forge", "").await;

    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    let listed = listing(&body);
    assert_eq!(names(&listed), ["Mele/Shop", "other/notes"]);
    let (shop, notes) = (&listed.repositories[0], &listed.repositories[1]);
    assert_eq!(shop.local_path.as_deref(), Some(root.as_str()), "cloned here, as the root Fleet serves");
    assert_eq!(shop.url, "https://forge.invalid/Mele/Shop.git");
    assert!(shop.private);
    assert_eq!(shop.pushed_at.as_ref().map(ipc::Instant::as_str), Some("2026-11-12T10:00:00Z"));
    assert_eq!(notes.local_path, None, "not served, so not cloned as far as Fleet knows");
    assert!(!notes.private);
    assert!(!listed.cut);
    assert_eq!(fleet.vcs().account_listings_asked(), [MOST_READ]);
}

#[tokio::test]
async fn q_narrows_to_the_repositories_whose_owner_and_name_hold_every_word() {
    let home = TempDir::new();
    let fleet = Arc::new(a_fleet_reading(&home, &Arc::new(Planted::nothing())));
    let forge = vec![on_the_forge("mele", "armada"), on_the_forge("mele", "shop"), on_the_forge("other", "armada-docs")];
    fleet.vcs().account_lists(forge, false);
    let app = served(&fleet);

    for (q, expected) in [
        ("arm", &["mele/armada", "other/armada-docs"][..]),
        ("ARM", &["mele/armada", "other/armada-docs"][..]),
        ("mele%20arm", &["mele/armada"][..]),
        ("mele/sh", &["mele/shop"][..]),
        ("nothing", &[][..]),
        ("", &["mele/armada", "mele/shop", "other/armada-docs"][..]),
    ] {
        let (status, body) = call(&app, "GET", &format!("/repositories/forge?q={q}"), "").await;
        assert_eq!(status, StatusCode::OK, "{q}");
        assert_eq!(names(&listing(&body)), expected, "{q}");
    }
}

#[tokio::test]
async fn more_than_the_cap_is_cut_and_says_so() {
    let home = TempDir::new();
    let fleet = Arc::new(a_fleet_reading(&home, &Arc::new(Planted::nothing())));
    let many: Vec<ForgeRepository> = (0..MOST_LISTED + 5).map(|n| on_the_forge("o", &format!("r{n}"))).collect();
    fleet.vcs().account_lists(many, false);
    let app = served(&fleet);

    let (_, body) = call(&app, "GET", "/repositories/forge", "").await;
    let listed = listing(&body);
    assert_eq!(listed.repositories.len(), MOST_LISTED);
    assert!(listed.cut);
    assert_eq!(listed.repositories[0].name, "r0", "the forge's order, newest pushed first");

    let (_, body) = call(&app, "GET", "/repositories/forge?q=r20", "").await;
    let listed = listing(&body);
    assert_eq!(names(&listed), ["o/r20", "o/r200", "o/r201", "o/r202", "o/r203", "o/r204"]);
    assert!(!listed.cut, "every match is here");
}

#[tokio::test]
async fn a_forge_that_held_more_than_fleet_reads_is_cut_too() {
    let home = TempDir::new();
    let fleet = Arc::new(a_fleet_reading(&home, &Arc::new(Planted::nothing())));
    fleet.vcs().account_lists(vec![on_the_forge("o", "r")], true);
    let app = served(&fleet);

    let (_, body) = call(&app, "GET", "/repositories/forge?q=r", "").await;
    assert!(listing(&body).cut);
}

#[tokio::test]
async fn a_second_ask_within_the_minute_does_not_ask_the_forge_again() {
    let home = TempDir::new();
    let fleet = Arc::new(a_fleet_reading(&home, &Arc::new(Planted::nothing())));
    fleet.vcs().account_lists(vec![on_the_forge("o", "r"), on_the_forge("o", "s")], false);
    let app = served(&fleet);

    let (_, first) = call(&app, "GET", "/repositories/forge?q=r", "").await;
    let (_, second) = call(&app, "GET", "/repositories/forge?q=s", "").await;

    assert_eq!(names(&listing(&first)), ["o/r"]);
    assert_eq!(names(&listing(&second)), ["o/s"]);
    assert_eq!(fleet.vcs().account_listings_asked(), [MOST_READ], "asked once, narrowed twice");
}

#[tokio::test]
async fn a_forge_that_will_not_list_is_a_422_in_its_own_words_and_is_asked_again_next_time() {
    let home = TempDir::new();
    let fleet = Arc::new(a_fleet_reading(&home, &Arc::new(Planted::nothing())));
    fleet.vcs().account_listing_refuses("The forge has no token for this machine. Sign in, then open it again.");
    let app = served(&fleet);

    let (status, body) = call(&app, "GET", "/repositories/forge", "").await;

    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    let refused: WireError = ipc::decode("a refusal", &body).expect("a WireError");
    assert_eq!(refused.code, "fleet.forge_repositories_unreadable");
    assert!(refused.message.contains("Sign in, then open it again"), "{}", refused.message);

    fleet.vcs().account_lists(vec![on_the_forge("o", "r")], false);
    let (status, body) = call(&app, "GET", "/repositories/forge", "").await;
    assert_eq!(status, StatusCode::OK, "a refusal is not kept");
    assert_eq!(names(&listing(&body)), ["o/r"]);
    assert_eq!(fleet.vcs().account_listings_asked().len(), 2);
}
