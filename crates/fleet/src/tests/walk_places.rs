//! Where a pull request's change runs: found from the pull request, then from what Fleet remembers,
//! then not at all, and the places kept. `docs/concepts/pull-review.md`, *Walk*.
//!
//! **No forge is called**: `FakeVcs` answers the pull request's page as it was told to.

use adapter_traits::{CheckRow, CheckStanding, FileChange, PageFile, PullRequestPageFacts, Said};
use api::PullRequestPanel;
use ipc::{ManifestId, SaveWalkPlace, WalkHow, WalkPlace, WalkSource};

use super::pull_request_panel::a_page;
use super::pull_requesting::{a_fleet, code, manifest, Fixture};
use crate::tests::tmp::TempDir;

fn page_changing(paths: &[&str]) -> PullRequestPageFacts {
    PullRequestPageFacts {
        conversation: Vec::new(),
        checks: Vec::new(),
        files: paths
            .iter()
            .map(|path| PageFile { path: path.to_string(), additions: 4, deletions: 1, change: FileChange::Modified })
            .collect(),
        ..a_page()
    }
}

fn a_bot_saying(body: &str) -> Said {
    Said::Comment { author: "preview-bot".into(), at: "2026-10-16T09:00:00Z".into(), body: body.into() }
}

fn deployed(address: &str) -> WalkPlace {
    WalkPlace { how: WalkHow::Deployed, address: address.into(), command: String::new(), api: String::new() }
}

fn pulled(command: &str, address: &str) -> WalkPlace {
    WalkPlace { how: WalkHow::Pull, address: address.into(), command: command.into(), api: String::new() }
}

fn save(area: &str, place: WalkPlace) -> SaveWalkPlace {
    SaveWalkPlace { area: area.into(), place }
}

fn fleet_and_repository(home: &TempDir) -> (Fixture, ManifestId) {
    let fleet = a_fleet(home);
    let (id, _) = manifest(&fleet);
    (fleet, id)
}

#[tokio::test]
async fn a_preview_the_pull_request_names_is_the_place_and_memory_is_not_asked() {
    let home = TempDir::new();
    let (fleet, id) = fleet_and_repository(&home);
    // Something is remembered for the area, to show it is not the source that answers.
    fleet.save_walk_place(id.clone(), save("crates", deployed("https://staging.invalid"))).await.expect("saved");
    let mut page = page_changing(&["crates/fleet/a.rs"]);
    page.conversation = vec![a_bot_saying("Preview deployed: https://pr-12.preview.invalid/app (expires in 7 days)")];
    fleet.vcs().now_pull_request_page(Some(page));

    let found = fleet.find_walk_place(id, 12).await.expect("found");

    assert_eq!(found.area, "crates/fleet");
    assert_eq!(found.place, Some(deployed("https://pr-12.preview.invalid/app")));
    assert_eq!(found.tried.len(), 1, "the first source to find one ends the search");
    assert_eq!((found.tried[0].source, found.tried[0].found), (WalkSource::PullRequest, true));
    assert_eq!(found.tried[0].by.as_deref(), Some("preview-bot"));
}

#[tokio::test]
async fn a_passed_check_that_deploys_is_a_place_too() {
    let home = TempDir::new();
    let (fleet, id) = fleet_and_repository(&home);
    let mut page = page_changing(&["web/a.ts"]);
    page.checks = vec![
        CheckRow { name: "lint".into(), standing: CheckStanding::Passed, seconds: None, address: Some("https://forge.invalid/run/1".into()) },
        CheckRow { name: "Deploy preview".into(), standing: CheckStanding::Passed, seconds: None, address: Some("https://deploy.invalid/p/12".into()) },
    ];
    fleet.vcs().now_pull_request_page(Some(page));

    let found = fleet.find_walk_place(id, 12).await.expect("found");

    assert_eq!(found.place, Some(deployed("https://deploy.invalid/p/12")));
}

#[tokio::test]
async fn with_nothing_on_the_pull_request_the_place_fleet_remembers_for_the_area_is_found() {
    let home = TempDir::new();
    let (fleet, id) = fleet_and_repository(&home);
    let place = pulled("just serve", "http://localhost:3000");
    // Kept for a directory above the change, which covers it.
    fleet.save_walk_place(id.clone(), save("crates", place.clone())).await.expect("saved");
    fleet.vcs().now_pull_request_page(Some(page_changing(&["crates/fleet/a.rs", "crates/fleet/b.rs", "docs/c.md"])));

    let found = fleet.find_walk_place(id, 12).await.expect("found");

    assert_eq!(found.place, Some(place));
    let sources: Vec<_> = found.tried.iter().map(|it| (it.source, it.found)).collect();
    assert_eq!(sources, [(WalkSource::PullRequest, false), (WalkSource::Memory, true)]);
    assert_eq!(found.tried[1].by.as_deref(), Some("crates/"));
}

#[tokio::test]
async fn the_most_specific_remembered_area_wins_and_a_sibling_does_not_cover() {
    let home = TempDir::new();
    let (fleet, id) = fleet_and_repository(&home);
    fleet.save_walk_place(id.clone(), save("", deployed("https://root.invalid"))).await.expect("saved");
    fleet.save_walk_place(id.clone(), save("crates/fleet", deployed("https://fleet.invalid"))).await.expect("saved");
    fleet.save_walk_place(id.clone(), save("crates/ipc", deployed("https://ipc.invalid"))).await.expect("saved");
    fleet.vcs().now_pull_request_page(Some(page_changing(&["crates/fleet/a.rs"])));

    let found = fleet.find_walk_place(id.clone(), 12).await.expect("found");
    assert_eq!(found.place, Some(deployed("https://fleet.invalid")));

    fleet.vcs().now_pull_request_page(Some(page_changing(&["apps/desktop/a.ts"])));
    let found = fleet.find_walk_place(id, 12).await.expect("found");
    assert_eq!(found.place, Some(deployed("https://root.invalid")), "the repository's root covers every area");
}

#[tokio::test]
async fn with_nothing_anywhere_no_place_is_found_and_both_sources_say_so() {
    let home = TempDir::new();
    let (fleet, id) = fleet_and_repository(&home);
    fleet.vcs().now_pull_request_page(Some(page_changing(&["crates/fleet/a.rs"])));

    let found = fleet.find_walk_place(id, 12).await.expect("found");

    assert_eq!(found.place, None, "asking him is Bridge's, the third source");
    let sources: Vec<_> = found.tried.iter().map(|it| (it.source, it.found)).collect();
    assert_eq!(sources, [(WalkSource::PullRequest, false), (WalkSource::Memory, false)]);
    assert!(found.tried[1].said.contains("Nothing remembered"), "{}", found.tried[1].said);
}

#[tokio::test]
async fn a_forge_that_will_not_give_the_page_is_a_fault_and_not_an_empty_find() {
    let home = TempDir::new();
    let (fleet, id) = fleet_and_repository(&home);
    fleet.vcs().now_pull_request_page(None);

    let refused = fleet.find_walk_place(id, 12).await.unwrap_err();

    assert_eq!(refused.status(), 500);
    assert_eq!(code(&refused), "fleet.review_pull_request_unreadable");
}

#[tokio::test]
async fn a_place_is_saved_listed_replaced_per_area_and_forgotten() {
    let home = TempDir::new();
    let (fleet, id) = fleet_and_repository(&home);

    let first = fleet.save_walk_place(id.clone(), save("crates/fleet/", deployed("https://one.invalid"))).await.expect("saved");
    assert_eq!(first.area, "crates/fleet", "a trailing slash is dropped");
    let other = fleet.save_walk_place(id.clone(), save("web", pulled("npm start", "http://localhost:5173"))).await.expect("saved");
    let again = fleet.save_walk_place(id.clone(), save("crates/fleet", deployed("https://two.invalid"))).await.expect("saved");
    assert_eq!(again.id, first.id, "one place a repository and area: the row is replaced where it was");

    let listed = fleet.list_walk_places().await.expect("listed").places;
    let shown: Vec<_> = listed.iter().map(|it| (it.area.as_str(), it.place.address.as_str())).collect();
    assert_eq!(shown, [("crates/fleet", "https://two.invalid"), ("web", "http://localhost:5173")]);
    assert!(listed.iter().all(|it| it.manifest_id == id && !it.repository.is_empty()));

    let left = fleet.forget_walk_place(id.clone(), first.id.clone()).await.expect("forgotten").places;
    assert_eq!(left.iter().map(|it| it.id.clone()).collect::<Vec<_>>(), [other.id]);
    let left = fleet.forget_walk_place(id, first.id).await.expect("a place already gone is not an error").places;
    assert_eq!(left.len(), 1);
}

#[tokio::test]
async fn a_place_that_cannot_be_walked_and_an_area_outside_the_repository_are_refused_unkept() {
    let home = TempDir::new();
    let (fleet, id) = fleet_and_repository(&home);
    let bad = [
        ("", WalkPlace { how: WalkHow::Deployed, address: String::new(), command: String::new(), api: String::new() }),
        ("", deployed("ftp://files.invalid")),
        ("", deployed("https://ok.invalid").with_command("npm start")),
        ("", pulled("", "http://localhost:3000")),
        ("../elsewhere", deployed("https://ok.invalid")),
        ("a/b/c", deployed("https://ok.invalid")),
    ];
    for (area, place) in bad {
        let refused = fleet.save_walk_place(id.clone(), save(area, place.clone())).await.unwrap_err();
        assert_eq!(refused.status(), 422, "{area:?} {place:?}");
        assert_eq!(code(&refused), "fleet.walk_place_invalid");
    }
    assert!(fleet.list_walk_places().await.expect("listed").places.is_empty());
}

trait WithCommand {
    fn with_command(self, command: &str) -> Self;
}

impl WithCommand for WalkPlace {
    fn with_command(mut self, command: &str) -> Self {
        self.command = command.into();
        self
    }
}
