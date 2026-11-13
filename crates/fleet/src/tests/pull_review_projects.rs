//! The four review lists filtered to a project: only its members' pull requests, a folder member
//! counting as its repository. **No forge is called**; `FakeVcs` answers each list as told and a
//! machine that names each repository by its folder says which slug is whose.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

use adapter_traits::{Bucket, ListedChecks, ListedPull, ReviewList, ReviewStanding};
use api::PullRequestPanel;
use ipc::PullReviewLists;
use testkit::{FakeHarness, FakeVcs, FakeWorkProduct};

use crate::daemon::Fleet;
use crate::red_claim::Reaching;
use crate::repositories::{Located, SetUp};
use crate::tests::daemon::fittings;
use crate::tests::pull_requesting::code;
use crate::tests::tmp::TempDir;

type Fixture = Fleet<FakeHarness, FakeVcs, FakeWorkProduct>;

/// The folder called `store-api` is `acme/store-api`, any other `acme/store-web`.
struct ByFolder;

impl Reaching for ByFolder {
    fn slug(&self, root: &str) -> Option<String> {
        let api = Path::new(root).file_name().is_some_and(|name| name == "store-api");
        Some(if api { "acme/store-api" } else { "acme/store-web" }.to_string())
    }
    fn open(&self, _url: &str, _directory: &Path) -> Result<Option<Box<dyn adapter_traits::Store + Send>>, String> {
        Ok(None)
    }
    fn bucket(&self, _address: &adapters::BucketAddress) -> Result<Box<dyn Bucket + Send>, adapter_traits::BucketError> {
        Err(adapter_traits::BucketError::Unsigned(String::from("none on this machine")))
    }
}

fn pull(number: u64, repository: &str) -> ListedPull {
    ListedPull {
        repository: repository.into(),
        number,
        title: format!("Pull request {number}"),
        branch: format!("branch-{number}"),
        address: format!("https://forge.invalid/{repository}/pull/{number}"),
        draft: false,
        head: "3f9c1d2e3f9c1d2e3f9c1d2e3f9c1d2e3f9c1d2e".into(),
        author: "ada".into(),
        updated_at: "2026-10-15T09:30:00Z".into(),
        checks: ListedChecks::Running,
        review: ReviewStanding::None,
        team: None,
        my_latest: None,
    }
}

/// A Fleet serving two repositories, `acme/store-web` (the fixture's) and `acme/store-api`, with a
/// pull request in each on every list. Their manifest ids are returned in that order.
fn two_repositories(home: &TempDir) -> (Fixture, String, String) {
    let mut fittings = fittings(home, FakeWorkProduct::changed(&[]));
    fittings.reaching = Arc::new(ByFolder);
    let fleet = Fleet::assembled(fittings);
    let root = home.path().join("store-api").to_string_lossy().to_string();
    std::fs::create_dir_all(&root).expect("a folder");
    let manifest = config::Manifest::parse(&Path::new(&root).join("armada.yml"), "version: 1\nid: 01STOREAPIMANIFEST\nbase: main\n").expect("parses");
    fleet
        .repositories()
        .add(Located {
            records_root: format!("{root}-records"),
            root: root.clone(),
            set_up: Some(SetUp::of(manifest, BTreeMap::new())),
        })
        .expect("served");
    let web = fleet.repositories().first().expect("the fixture's repository").manifest().id().as_str().to_string();
    let api = "01STOREAPIMANIFEST".to_string();
    fleet.vcs().main_ci.login_is(Some("nick"));
    for list in [ReviewList::Requested, ReviewList::Team, ReviewList::Assigned, ReviewList::Mine] {
        fleet.vcs().now_review_list(list, vec![pull(1, "acme/store-web"), pull(2, "acme/store-api")]);
    }
    (fleet, web, api)
}

async fn project_holding(fleet: &Fixture, home: &TempDir, members: &str) {
    let at = crate::tests::projects_kept::file(home, "my-store");
    std::fs::create_dir_all(at.parent().expect("a folder")).expect("its folder");
    std::fs::write(at, format!("version: 2\nproject_id: my-store\nname: My store\nmembers:\n{members}")).expect("its file");
    fleet.projects_reload().await;
}

fn numbers(lists: &PullReviewLists) -> [Vec<u64>; 4] {
    [&lists.requested, &lists.team, &lists.assigned, &lists.mine].map(|rows| rows.iter().map(|row| row.number).collect())
}

#[tokio::test]
async fn every_list_keeps_only_the_rows_of_the_projects_members() {
    let home = TempDir::new();
    let (fleet, _, api) = two_repositories(&home);
    project_holding(&fleet, &home, &format!("  - repository: {api}\n")).await;
    let lists = fleet.get_review_lists(Some("my-store".into())).await.expect("read");
    assert_eq!(numbers(&lists), [vec![2], vec![2], vec![2], vec![2]]);
    assert!(lists.mine.iter().all(|row| row.manifest_id.as_str() == api));
}

#[tokio::test]
async fn a_folder_member_keeps_its_repositorys_rows() {
    let home = TempDir::new();
    let (fleet, web, _) = two_repositories(&home);
    project_holding(&fleet, &home, &format!("  - repository: {web}\n    folder: apps/web\n")).await;
    let lists = fleet.get_review_lists(Some("my-store".into())).await.expect("read");
    assert_eq!(numbers(&lists), [vec![1], vec![1], vec![1], vec![1]]);
}

#[tokio::test]
async fn no_project_leaves_every_repositorys_rows() {
    let home = TempDir::new();
    let (fleet, _, api) = two_repositories(&home);
    project_holding(&fleet, &home, &format!("  - repository: {api}\n")).await;
    let lists = fleet.get_review_lists(None).await.expect("read");
    assert_eq!(numbers(&lists), [vec![1, 2], vec![1, 2], vec![1, 2], vec![1, 2]]);
}

#[tokio::test]
async fn a_project_he_has_none_of_is_a_404_and_the_forge_is_not_asked() {
    let home = TempDir::new();
    let (fleet, ..) = two_repositories(&home);
    let refused = fleet.get_review_lists(Some("nope".into())).await.unwrap_err();
    assert_eq!(refused.status(), 404, "{refused:?}");
    assert_eq!(code(&refused), "fleet.no_such_project");
    assert!(fleet.vcs().review_lists_asked().is_empty());
}

#[tokio::test]
async fn a_project_with_no_served_member_has_four_empty_lists_and_asks_the_forge_nothing() {
    let home = TempDir::new();
    let (fleet, ..) = two_repositories(&home);
    project_holding(&fleet, &home, "  - repository: 01NOTSERVEDHERE\n").await;
    let lists = fleet.get_review_lists(Some("my-store".into())).await.expect("read");
    assert_eq!(numbers(&lists), [(); 4].map(|()| Vec::<u64>::new()));
    assert!(lists.silent.is_empty());
    assert!(fleet.vcs().review_lists_asked().is_empty());
}
