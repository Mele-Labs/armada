//! What a project says on the wire now that its name and members are a file: the members, the place and
//! what would not read; what `add_project` and `edit_project` write and where; the members a task brings;
//! and the project a Job is started for and keeps.

use std::sync::Arc;

use api::{HostedSessions, Projects, Sessions};
use ipc::{EditProject, ManifestId, NewProject, ProjectMember, ProjectPlace, RunCloud, RunPlace};
use testkit::{FakeTracker, FakeWorkProduct};

use crate::tests::daemon::a_fleet;
use crate::tests::project_holds::start;
use crate::tests::projects::{a_fleet as a_fleet_with_tracker, code_of, manifest, new_issue, new_task};
use crate::tests::projects_kept::file;
use crate::tests::pull_notices::a_rig;
use crate::tests::tmp::TempDir;

fn whole(id: &str) -> ProjectMember {
    ProjectMember { manifest_id: ManifestId::carried(id), folder: None }
}

fn folder(id: &str, at: &str) -> ProjectMember {
    ProjectMember { manifest_id: ManifestId::carried(id), folder: Some(at.to_string()) }
}

fn named(name: &str, members: Option<Vec<ProjectMember>>, place: Option<ProjectPlace>) -> NewProject {
    NewProject { name: name.to_string(), members, place }
}

fn this_mac() -> ProjectPlace {
    ProjectPlace { place: RunPlace::ThisMac, synced_folder: None, cloud: None }
}

fn synced(at: &std::path::Path) -> ProjectPlace {
    ProjectPlace { place: RunPlace::Synced, synced_folder: Some(at.to_string_lossy().into_owned()), cloud: None }
}

fn members_of(project: &ipc::Project) -> Vec<(String, Option<String>)> {
    project.members.iter().map(|one| (one.manifest_id.as_str().to_string(), one.folder.clone())).collect()
}

#[tokio::test]
async fn the_list_answers_each_projects_members_and_place_and_names_the_file_that_would_not_read() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&[]));
    let synced_at = home.path().join("sync");
    std::fs::create_dir_all(&synced_at).expect("a folder");
    let shop = fleet
        .add_project(named("Shop", Some(vec![whole("web"), folder("mono", "apps/api")]), Some(synced(&synced_at))))
        .await
        .expect("added")
        .projects[0]
        .id
        .clone();
    let lost = fleet.add_project(named("Lost", None, None)).await.expect("added").projects[1].id.clone();
    std::fs::remove_file(file(&home, &lost)).expect("its file goes");
    std::fs::write(home.path().join("kit/projects/stray.yml"), "not: a project\n").expect("a stray");
    fleet.projects_reload().await;

    let read = fleet.list_projects().await.expect("listed");
    let (one, two) = (&read.projects[0], &read.projects[1]);
    assert_eq!((one.id.as_str(), one.name.as_str(), one.unread.as_deref()), (shop.as_str(), "Shop", None));
    assert_eq!(members_of(one), [("web".to_string(), None), ("mono".to_string(), Some("apps/api".to_string()))]);
    assert_eq!(one.place, synced(&synced_at), "where its file is kept, as it was chosen");

    assert_eq!((two.id.as_str(), two.name.as_str()), (lost.as_str(), "Lost"), "the store's name stands where the file cannot be read");
    assert!(two.members.is_empty(), "and it holds no member");
    assert!(two.unread.as_deref().is_some_and(|why| why.contains("not there")), "{:?}", two.unread);
    assert_eq!(two.place, this_mac(), "none chosen is this Mac");

    let mut left: Vec<(String, String)> = read.left_out.iter().map(|one| (one.path.rsplit('/').next().unwrap_or_default().to_string(), format!("{:?}", one.place))).collect();
    left.sort();
    assert_eq!(left, [(format!("{lost}.yml"), "ThisMac".to_string()), ("stray.yml".to_string(), "ThisMac".to_string())]);
}

#[tokio::test]
async fn a_project_added_with_members_and_a_place_writes_its_file_there_and_the_row() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&[]));
    let synced_at = home.path().join("sync");
    std::fs::create_dir_all(&synced_at).expect("a folder");

    let made = fleet.add_project(named("  Shop ", Some(vec![whole("web"), whole("api")]), Some(synced(&synced_at)))).await.expect("added");

    let id = made.projects[0].id.clone();
    assert_eq!(made.projects[0].name, "Shop", "trimmed");
    assert_eq!(members_of(&made.projects[0]), [("web".to_string(), None), ("api".to_string(), None)]);
    let there = synced_at.join("projects").join(format!("{id}.yml"));
    let written = std::fs::read_to_string(&there).expect("written in the synced folder");
    assert!(written.contains("name: Shop") && written.contains("repository: web"), "{written}");
    assert!(!file(&home, &id).exists(), "and not in Kit's");
    assert_eq!(fleet.store().lock().await.projects().expect("read").projects[0].id, id, "the row is in his order");
    assert_eq!(made.projects[0].place, synced(&synced_at));

    let bare = fleet.add_project(named("Bare", None, None)).await.expect("added");
    assert!(file(&home, &bare.projects[1].id).is_file() && bare.projects[1].members.is_empty(), "this Mac and empty where neither is said");
}

#[tokio::test]
async fn a_project_that_cannot_be_made_is_refused_before_a_file_or_a_row() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&[]));
    let bad = [
        (named("Shop", Some(vec![whole("web"), whole("web")]), None), "fleet.project_malformed"),
        (named("Shop", Some(vec![whole("web"), folder("web", "apps/x")]), None), "fleet.project_malformed"),
        (named("Shop", Some(vec![folder("web", "/apps")]), None), "fleet.project_malformed"),
        (named("Shop", None, Some(ProjectPlace { place: RunPlace::Repository, synced_folder: None, cloud: None })), "fleet.project_place_refused"),
        (named("Shop", None, Some(ProjectPlace { place: RunPlace::Synced, synced_folder: Some("/no/such/folder".into()), cloud: None })), "fleet.project_place_refused"),
        (named("Shop", None, Some(ProjectPlace { place: RunPlace::Cloud, synced_folder: None, cloud: None })), "fleet.project_place_refused"),
        (named("  ", None, None), "fleet.project_blank"),
    ];
    for (body, code) in bad {
        let refused = fleet.add_project(body).await.expect_err("refused");
        assert_eq!(code_of(refused), (422, code.to_string()));
    }
    assert!(fleet.store().lock().await.projects().expect("read").projects.is_empty(), "no row was made");
    assert!(!home.path().join("kit/projects").exists() || std::fs::read_dir(home.path().join("kit/projects")).expect("a folder").count() == 0, "and no file");
}

#[tokio::test]
async fn edit_renames_replaces_the_members_and_moves_the_file_and_what_is_left_out_stays() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&[]));
    let id = fleet.add_project(named("Shop", Some(vec![whole("web")]), None)).await.expect("added").projects[0].id.clone();

    let renamed = fleet.edit_project(id.clone(), EditProject { name: Some(" Storefront ".into()), ..Default::default() }).await.expect("edited");
    assert_eq!((renamed.projects[0].name.as_str(), members_of(&renamed.projects[0])), ("Storefront", vec![("web".to_string(), None)]), "members stay where absent");
    assert!(std::fs::read_to_string(file(&home, &id)).expect("a file").contains("name: Storefront"));

    let changed = fleet.edit_project(id.clone(), EditProject { members: Some(vec![whole("api"), folder("mono", "apps/web")]), ..Default::default() }).await.expect("edited");
    assert_eq!(members_of(&changed.projects[0]), [("api".to_string(), None), ("mono".to_string(), Some("apps/web".to_string()))]);
    assert_eq!(changed.projects[0].name, "Storefront", "and the name stays");

    let synced_at = home.path().join("sync");
    std::fs::create_dir_all(&synced_at).expect("a folder");
    let moved = fleet.edit_project(id.clone(), EditProject { place: Some(synced(&synced_at)), ..Default::default() }).await.expect("moved");
    assert_eq!(moved.projects[0].place, synced(&synced_at));
    assert!(synced_at.join("projects").join(format!("{id}.yml")).is_file(), "written there");
    assert!(!file(&home, &id).exists(), "and taken from Kit, where Fleet kept it");
    assert_eq!(members_of(&moved.projects[0]).len(), 2, "its members came with it");

    let emptied = fleet.edit_project(id.clone(), EditProject { members: Some(Vec::new()), place: Some(this_mac()), ..Default::default() }).await.expect("edited");
    assert!(emptied.projects[0].members.is_empty() && file(&home, &id).is_file(), "[] takes every member away, and a place may go with it");
    let bucket = ProjectPlace { place: RunPlace::Cloud, synced_folder: None, cloud: Some(RunCloud::Bucket("nonsense".into())) };
    let refused = fleet.edit_project(id.clone(), EditProject { place: Some(bucket), ..Default::default() }).await.expect_err("refused");
    assert_eq!(code_of(refused), (422, "fleet.project_place_refused".to_string()));
}

#[tokio::test]
async fn an_edit_is_refused_for_what_is_not_a_project_a_file_that_would_not_read_and_members_that_do_not_fit() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&[]));
    let id = fleet.add_project(named("Shop", Some(vec![whole("web")]), None)).await.expect("added").projects[0].id.clone();

    let missing = fleet.edit_project("nope".into(), EditProject::default()).await.expect_err("refused");
    assert_eq!(code_of(missing), (404, "fleet.no_such_project".to_string()));
    let blank = fleet.edit_project(id.clone(), EditProject { name: Some("  ".into()), ..Default::default() }).await.expect_err("refused");
    assert_eq!(code_of(blank), (422, "fleet.project_blank".to_string()));
    let twice = fleet.edit_project(id.clone(), EditProject { members: Some(vec![whole("a"), whole("a")]), ..Default::default() }).await.expect_err("refused");
    assert_eq!(code_of(twice), (422, "fleet.project_malformed".to_string()));

    std::fs::write(file(&home, &id), "not: a project\n").expect("broken");
    fleet.projects_reload().await;
    let unread = fleet.edit_project(id.clone(), EditProject { name: Some("Later".into()), ..Default::default() }).await.expect_err("refused");
    assert_eq!(code_of(unread), (422, "fleet.project_unwritable".to_string()));
    assert_eq!(std::fs::read_to_string(file(&home, &id)).expect("a file"), "not: a project\n", "a file that will not read is not written over");
}

#[tokio::test]
async fn delete_takes_the_file_away_where_fleet_keeps_it_and_leaves_a_synced_one() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&[]));
    let kit = fleet.add_project(named("Kit's", None, None)).await.expect("added").projects[0].id.clone();
    let synced_at = home.path().join("sync");
    std::fs::create_dir_all(&synced_at).expect("a folder");
    let theirs = fleet.add_project(named("Theirs", None, Some(synced(&synced_at)))).await.expect("added").projects[1].id.clone();

    fleet.delete_project(kit.clone()).await.expect("deleted");
    assert!(!file(&home, &kit).exists(), "the file Fleet kept itself goes");
    let left = fleet.delete_project(theirs.clone()).await.expect("deleted");
    assert!(left.projects.is_empty());
    assert!(synced_at.join("projects").join(format!("{theirs}.yml")).is_file(), "a synced folder's is the person's, and stays");
    fleet.projects_reload().await;
    assert!(fleet.projects_held().files().is_empty(), "and is not read again");
}

#[tokio::test]
async fn a_task_or_an_issue_written_in_a_repository_the_project_lacks_makes_it_a_whole_member() {
    let home = TempDir::new();
    let fleet = a_fleet_with_tracker(&home, &Arc::new(FakeTracker::empty()));
    let at = manifest(&fleet);
    let id = fleet.add_project(named("Shop", Some(vec![whole("web")]), None)).await.expect("added").projects[0].id.clone();

    let read = fleet.add_project_task(new_task(at.clone(), Some(id.clone()), "Ship it")).await.expect("added");
    assert_eq!(members_of(&read.projects[0]), [("web".to_string(), None), (at.as_str().to_string(), None)], "after the ones it had");
    assert!(std::fs::read_to_string(file(&home, &id)).expect("a file").contains(at.as_str()), "written to the file");
    let again = fleet.add_project_task(new_task(at.clone(), Some(id.clone()), "And this")).await.expect("added");
    assert_eq!(again.projects[0].members.len(), 2, "a repository it holds is not added again");

    let loose = fleet.add_project_task(new_task(at.clone(), None, "Loose")).await.expect("added");
    assert_eq!(loose.projects[0].members.len(), 2, "a task in no project changes none");
    let other = fleet.add_project(named("Other", None, None)).await.expect("added").projects[1].id.clone();
    let issue = fleet.link_project_issue(new_issue(at.clone(), Some(other.clone()), "7")).await.expect("linked");
    assert_eq!(members_of(&issue.projects[1]), [(at.as_str().to_string(), None)], "an issue row is a task here");
    assert!(fleet.projects_held().get(&core_model::ScopeId::new(&other).unwrap()).expect("held").holds(&core_model::ManifestId::carried(core_model::Ulid::carried(at.as_str()))));
}

#[tokio::test]
async fn a_row_moved_into_a_project_brings_its_repository_and_a_folder_member_already_holds_it() {
    let home = TempDir::new();
    let fleet = a_fleet_with_tracker(&home, &Arc::new(FakeTracker::empty()));
    let at = manifest(&fleet);
    let empty = fleet.add_project(named("Empty", None, None)).await.expect("added").projects[0].id.clone();
    let folded = fleet.add_project(named("Folded", Some(vec![folder(at.as_str(), "apps/web")]), None)).await.expect("added").projects[1].id.clone();
    let task = fleet.add_project_task(new_task(at.clone(), None, "Loose")).await.expect("added").loose[0].clone();
    let row = match task {
        ipc::ProjectRow::Task { id, .. } | ipc::ProjectRow::Issue { id, .. } => id,
    };

    let into = |project: &str| ipc::MoveProjectRow { project: Some(project.to_string()), position: 0, parent: None };
    let moved = fleet.move_project_row(row.clone(), into(&folded)).await.expect("moved");
    assert_eq!(members_of(&moved.projects[1]), [(at.as_str().to_string(), Some("apps/web".to_string()))], "a folder of it already holds it");
    let moved = fleet.move_project_row(row, into(&empty)).await.expect("moved");
    assert_eq!(members_of(&moved.projects[0]), [(at.as_str().to_string(), None)], "an empty project gains it whole");
}

#[tokio::test]
async fn a_task_moved_to_another_repository_makes_that_repository_a_member_of_its_project() {
    let home = TempDir::new();
    let fleet = a_fleet_with_tracker(&home, &Arc::new(FakeTracker::empty()));
    let at = manifest(&fleet);
    let id = fleet.add_project(named("Shop", Some(vec![whole(at.as_str())]), None)).await.expect("added").projects[0].id.clone();
    let task = fleet.add_project_task(new_task(at.clone(), Some(id.clone()), "Ship it")).await.expect("added");
    let row = match &task.projects[0].rows[0] {
        ipc::ProjectRow::Task { id, .. } | ipc::ProjectRow::Issue { id, .. } => id.clone(),
    };
    let other = home.path().join("other");
    std::fs::create_dir_all(&other).expect("a folder");
    let root = other.to_string_lossy().to_string();
    fleet.repositories().add(crate::repositories::Located { records_root: format!("{root}-records"), root: root.clone(), set_up: None }).expect("served");
    let there = ManifestId::from(crate::repositories::Served::hosting(&fleet.repositories().at(&root).expect("held")).manifest().id());

    let moved = fleet.move_project_task_repository(row, there.clone()).await.expect("moved");

    assert_eq!(members_of(&moved.projects[0]), [(at.as_str().to_string(), None), (there.as_str().to_string(), None)]);
}

#[tokio::test]
async fn a_job_started_for_a_project_keeps_it_and_serves_it() {
    let rig = a_rig();
    let at = ManifestId::carried(rig.manifest());
    let id = rig.fleet.add_project(named("Shop", Some(vec![ProjectMember { manifest_id: at.clone(), folder: None }]), None)).await.expect("added").projects[0].id.clone();

    let mut one = start(&rig, "Ship it", Vec::new());
    one.project = Some(id.clone());
    let job = rig.fleet.start_session(one).await.expect("started");

    assert_eq!(job.hosted.as_ref().and_then(|hosted| hosted.project.as_deref()), Some(id.as_str()), "on the record it answers");
    let kept = rig.fleet.store().lock().await.hosting(job.id.as_str()).expect("read").expect("hosted");
    assert_eq!(kept.project.as_deref(), Some(id.as_str()), "and on the hosting row, so a restart keeps it");
    let listed = rig.fleet.list_sessions(None, None, None).await.expect("listed");
    assert!(listed.sessions.iter().any(|one| one.hosted.as_ref().is_some_and(|hosted| hosted.project.as_deref() == Some(id.as_str()))), "list_sessions carries it");
    let plain = rig.fleet.start_session(start(&rig, "None", Vec::new())).await.expect("started");
    assert!(plain.hosted.expect("hosted").project.is_none(), "a Job started for none says none");
}

#[tokio::test]
async fn a_start_for_an_unknown_project_is_a_404_and_one_that_does_not_hold_its_repository_a_422_and_neither_makes_a_job() {
    let rig = a_rig();
    let kept_before = rig.fleet.list_sessions(None, None, None).await.expect("listed").sessions.len();
    let elsewhere = rig.fleet.add_project(named("Elsewhere", Some(vec![whole("some-other-repository")]), None)).await.expect("added").projects[0].id.clone();

    let mut unknown = start(&rig, "Ship it", Vec::new());
    unknown.project = Some("nope".to_string());
    let refused = rig.fleet.start_session(unknown).await.expect_err("refused");
    assert_eq!(code_of(refused), (404, "fleet.no_such_project".to_string()));

    let mut outside = start(&rig, "Ship it", Vec::new());
    outside.project = Some(elsewhere);
    let refused = rig.fleet.start_session(outside).await.expect_err("refused");
    assert_eq!(code_of(refused), (422, "fleet.project_not_member".to_string()));

    assert_eq!(rig.fleet.list_sessions(None, None, None).await.expect("listed").sessions.len(), kept_before, "nothing was made");
}

#[tokio::test]
async fn a_start_for_a_project_whose_file_did_not_read_is_a_422_of_its_own() {
    let rig = a_rig();
    let at = ManifestId::carried(rig.manifest());
    let id = rig.fleet.add_project(named("Shop", Some(vec![ProjectMember { manifest_id: at, folder: None }]), None)).await.expect("added").projects[0].id.clone();
    std::fs::remove_file(crate::run_memory::places::project_file(&rig.fleet.kit_folder(), &id)).expect("its file was on this Mac");
    rig.fleet.projects_reload().await;

    let mut one = start(&rig, "Ship it", Vec::new());
    one.project = Some(id);
    let refused = rig.fleet.start_session(one).await.expect_err("refused");

    assert_eq!(code_of(refused), (422, "fleet.project_unreadable".to_string()));
}
