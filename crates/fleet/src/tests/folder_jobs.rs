//! A Job on any folder: a repository with no `armada.yml` is hosted under an implicit Manifest and
//! leases from the default pool, a folder in no git repository is started as one, and a new
//! project is made, started and served.

use std::collections::BTreeMap;
use std::sync::Arc;

use api::HostedSessions;
use axum::http::StatusCode;
use ipc::{RepositoryList, RepositorySummary};

use super::repositories::{a_fleet_reading, body_of, code, served, Planted};
use super::session_host::{denied, rig_placing};
use crate::repositories::{Located, SetUp};
use crate::tests::http::call;
use crate::tests::tmp::TempDir;

fn without_a_manifest(home: &TempDir, name: &str) -> Located {
    let root = home.path().join(name);
    std::fs::create_dir_all(&root).expect("a folder");
    Located {
        root: root.to_string_lossy().to_string(),
        records_root: home.path().join(format!("{name}-records")).to_string_lossy().to_string(),
        set_up: None,
    }
}

fn summary(body: &[u8]) -> RepositorySummary {
    ipc::decode("a repository", body).expect("a repository")
}

#[tokio::test]
async fn a_repository_with_no_manifest_is_listed_with_the_id_a_job_there_carries() {
    let home = TempDir::new();
    let bare = without_a_manifest(&home, "Shop App");
    let root = bare.root.clone();
    let planted = Arc::new(Planted::nothing().with(&root, bare));
    let fleet = Arc::new(a_fleet_reading(&home, &planted));
    let app = served(&fleet);

    let (status, body) = call(&app, "POST", "/repositories/add", &body_of(&root)).await;
    assert_eq!(status, StatusCode::CREATED);
    let added = summary(&body);
    assert!(added.manifest.is_none());
    let id = added.implicit_id.expect("an id a Job carries").as_str().to_string();
    assert!(id.starts_with("shop-app-"), "named for its folder: {id}");

    let (_, body) = call(&app, "GET", "/repositories", "").await;
    let listed: RepositoryList = ipc::decode("repositories", &body).expect("a list");
    let there = listed.repositories.iter().find(|one| one.root == root).expect("listed");
    assert_eq!(there.implicit_id.as_ref().map(|one| one.as_str()), Some(id.as_str()));
    let fixture = listed.repositories.iter().find(|one| one.root != root).expect("the fixture's");
    assert!(fixture.implicit_id.is_none(), "one with a Manifest names itself");
}

#[tokio::test]
async fn a_repository_with_no_manifest_answers_its_logbook_and_is_taught() {
    let home = TempDir::new();
    let bare = without_a_manifest(&home, "shop");
    let root = bare.root.clone();
    let planted = Arc::new(Planted::nothing().with(&root, bare));
    let fleet = Arc::new(a_fleet_reading(&home, &planted));
    let app = served(&fleet);
    let (status, _) = call(&app, "POST", "/repositories/add", &body_of(&root)).await;
    assert_eq!(status, StatusCode::CREATED);

    let taught = format!(r#"{{"section":"repo","repository":{root:?},"kind":"convention","text":"Run the shop's own tests."}}"#);
    let (status, body) = call(&app, "POST", "/logbook/entries", &taught).await;
    assert_eq!(status, StatusCode::OK, "taught: {}", String::from_utf8_lossy(&body));

    let asked = format!("/logbook?section=repo&repository={}", root.replace('/', "%2F"));
    let (status, body) = call(&app, "GET", &asked, "").await;
    assert_eq!(status, StatusCode::OK, "read: {}", String::from_utf8_lossy(&body));
    let read: ipc::Logbook = ipc::decode("a Logbook", &body).expect("a Logbook");
    assert!(read.entries.iter().any(|one| one.text == "Run the shop's own tests."));
}

#[tokio::test]
async fn the_implicit_id_names_the_repository_before_and_after_its_manifest_is_written() {
    let home = TempDir::new();
    let bare = without_a_manifest(&home, "shop");
    let root = bare.root.clone();
    let planted = Arc::new(Planted::nothing());
    let fleet = a_fleet_reading(&home, &planted);
    let added = fleet.repositories().add(bare).expect("added");
    let implicit = added.implicit_id().to_string();

    let before = fleet.repositories().serving(&implicit).expect("hosted by its implicit id");
    assert_eq!(before.root(), root);
    assert!(before.manifest().check_names().is_empty(), "nothing to gate on");
    assert!(before.workflows().is_empty());
    assert_eq!(crate::leasing::pool_of(&before).slots(), 8, "the default pool");
    assert!(
        fleet.repositories().served().iter().all(|one| one.root() != root),
        "and it is still not one with a Manifest"
    );

    let manifest = config::Manifest::parse(
        &std::path::Path::new(&root).join("armada.yml"),
        "version: 1\nid: written\n",
    )
    .expect("a Manifest");
    fleet.repositories().set_up(&root, SetUp::of(manifest, BTreeMap::new())).expect("set up");

    assert_eq!(fleet.repositories().serving("written").expect("by its own").root(), root);
    assert_eq!(fleet.repositories().serving(&implicit).expect("and by the old").root(), root);
}

#[tokio::test]
async fn a_job_on_a_repository_with_no_manifest_leases_a_slot_at_its_first_write() {
    let rig = rig_placing(None);
    let home = TempDir::new();
    let bare = without_a_manifest(&home, "scratch");
    let root = bare.root.clone();
    let added = rig.fleet.repositories().add(bare).expect("added");

    let id = Arc::clone(&rig.fleet)
        .start_session(ipc::StartSession {
            manifest_id: ipc::ManifestId::carried(added.implicit_id()),
            title: None,
            model: None,
            effort: None,
            mode: None,
            root: None,
            fork: None,
            request: None,
            after: None,
        })
        .await
        .expect("started")
        .id;
    rig.send(&id, "make a file").await;
    rig.stand_in.init(0);

    let first = rig
        .gate_in(&root, &id, "Write", &format!(r#"{{"file_path":"{root}/a.txt"}}"#))
        .await;

    assert!(denied(&first), "the checkout is not written: the job is moving to a slot");
    let held = Arc::clone(&rig.fleet).get_session(id.clone()).await.expect("read").session;
    assert!(
        held.attachments.iter().any(|one| one.kind == "slot"),
        "and it holds one: {:?}",
        held.attachments
    );
}

#[tokio::test]
async fn a_folder_in_no_repository_is_started_as_one_and_served() {
    let home = TempDir::new();
    let folder = home.path().join("fresh");
    std::fs::create_dir_all(&folder).expect("a folder");
    let planted = Arc::new(Planted::nothing());
    planted.in_no_repository(&folder);
    let fleet = Arc::new(a_fleet_reading(&home, &planted));
    let app = served(&fleet);

    let (status, body) = call(&app, "POST", "/repositories/add", &body_of(&folder.to_string_lossy())).await;

    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(planted.started(), [folder.clone()]);
    let added = summary(&body);
    assert_eq!(added.root, folder.to_string_lossy());
    assert!(added.implicit_id.is_some());
}

#[tokio::test]
async fn a_new_project_is_made_started_and_served() {
    let home = TempDir::new();
    let planted = Arc::new(Planted::nothing());
    let fleet = Arc::new(a_fleet_reading(&home, &planted));
    let app = served(&fleet);
    let parent = home.path().canonicalize().expect("a parent");
    let asked = |parent: &str, name: &str| format!(r#"{{"parent":"{parent}","name":"{name}"}}"#);

    let (status, body) = call(&app, "POST", "/repositories/create", &asked(&parent.to_string_lossy(), "notes")).await;

    assert_eq!(status, StatusCode::CREATED, "{}", String::from_utf8_lossy(&body));
    let made = parent.join("notes");
    assert!(made.is_dir());
    assert_eq!(planted.started(), [made.clone()]);
    assert_eq!(summary(&body).root, made.to_string_lossy());

    // It exists with something in it now.
    std::fs::write(made.join("a"), "x").expect("a file");
    let (status, body) = call(&app, "POST", "/repositories/create", &asked(&parent.to_string_lossy(), "notes")).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(code(&body), "fleet.destination_occupied");

    // An empty folder that is already there is used, and left in place.
    std::fs::create_dir_all(parent.join("empty")).expect("an empty folder");
    let (status, _) = call(&app, "POST", "/repositories/create", &asked(&parent.to_string_lossy(), "empty")).await;
    assert_eq!(status, StatusCode::CREATED);

    for name in ["", "..", "a/b", "."] {
        let (status, body) = call(&app, "POST", "/repositories/create", &asked(&parent.to_string_lossy(), name)).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{name:?}");
        assert_eq!(code(&body), "fleet.project_name_refused");
    }
    for nowhere in ["relative/folder", "/nowhere/at/all"] {
        let (status, body) = call(&app, "POST", "/repositories/create", &asked(nowhere, "x")).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{nowhere}");
        assert_eq!(code(&body), "fleet.no_such_folder");
    }
    assert!(!parent.join("x").exists() && !parent.join("a").exists());
}

#[tokio::test]
async fn a_repository_whose_branch_is_not_main_leases_from_the_branch_it_has() {
    let home = TempDir::new();
    let bare = without_a_manifest(&home, "legacy");
    let git = |args: &[&str]| {
        let done = std::process::Command::new("git")
            .args(["-c", "user.name=a", "-c", "user.email=a@example.invalid"])
            .args(args)
            .current_dir(&bare.root)
            .status()
            .expect("git runs");
        assert!(done.success(), "git {args:?}");
    };
    git(&["init", "--quiet", "-b", "trunk"]);
    git(&["commit", "--quiet", "--allow-empty", "-m", "first"]);
    let fleet = a_fleet_reading(&home, &Arc::new(Planted::nothing()));

    let added = fleet.repositories().add(bare).expect("added");

    let pool = crate::leasing::pool_of(&crate::repositories::Served::hosting(&added));
    assert_eq!(pool.base(), "trunk");
}
