//! A Project piece file: what it reads to, what it is refused for, and what becomes of a set of
//! them. Project here means a set of repositories and monorepo folders, and the monorepo's own
//! Manifest is read elsewhere.

use std::path::{Path, PathBuf};

use core_model::{Folder, ManifestId, Member, Ulid};

use crate::{parse_project, project_text, parse_workspace_v1, ProjectLeftOut, ProjectPlace, ProjectWritten, Projects};

fn text(id: &str, name: &str, members: &[&str]) -> String {
    let listed: String = members.iter().map(|one| format!("  - repository: {one}\n")).collect();
    let members = if members.is_empty() { String::from("members: []\n") } else { format!("members:\n{listed}") };
    format!("version: 2\nproject_id: {id}\nname: {name}\n{members}")
}

fn repository(id: &str) -> ManifestId {
    ManifestId::carried(Ulid::carried(id))
}

fn at(place: ProjectPlace, file: &str, text: String) -> ProjectWritten {
    ProjectWritten::new(place, PathBuf::from("kit/projects").join(file), text)
}

#[test]
fn a_file_reads_to_the_project_it_names_and_the_members_it_lists() {
    let read = parse_project(Path::new("my-store.yml"), &text("my-store", "My store", &["store-web", "store-api"]))
        .expect("a Project that loads");
    assert_eq!(read.id().as_str(), "my-store");
    assert_eq!(read.name().as_str(), "My store");
    assert_eq!(read.members(), [Member::whole(repository("store-web")), Member::whole(repository("store-api"))]);
}

#[test]
fn a_member_may_name_a_folder_of_its_repository() {
    let file = "version: 2\nproject_id: shop\nname: Shop\nmembers:\n  - repository: mono\n    folder: apps/api\n  - repository: web\n";
    let read = parse_project(Path::new("shop.yml"), file).expect("a Project with a folder");
    let folder = Folder::new("apps/api").expect("a folder");
    assert_eq!(read.members(), [Member::folder(repository("mono"), folder), Member::whole(repository("web"))]);
}

#[test]
fn a_project_with_no_members_reads() {
    let read = parse_project(Path::new("new.yml"), &text("new", "New", &[])).expect("an empty Project");
    assert!(read.members().is_empty());
}

#[test]
fn a_file_is_refused_for_every_key_that_is_wrong_and_names_each() {
    let said = parse_project(Path::new("my-store.yml"), "version: 1\nproject_id: my-store\nmembers: []\nmonorepo: true\n")
        .expect_err("a file that does not load")
        .to_string();
    for fault in ["version", "name", "monorepo"] {
        assert!(said.contains(fault), "{fault} is named in: {said}");
    }
    let said = parse_project(
        Path::new("my-store.yml"),
        "version: 2\nproject_id: my-store\nname: S\nmembers:\n  - repository: web\n    path: apps\n",
    )
    .expect_err("an unknown key in a member")
    .to_string();
    assert!(said.contains("members[0].path"), "{said}");
}

#[test]
fn a_folder_that_leaves_its_repository_is_refused_naming_the_key() {
    let file = "version: 2\nproject_id: p\nname: P\nmembers:\n  - repository: mono\n    folder: ../elsewhere\n";
    let said = parse_project(Path::new("p.yml"), file).expect_err("a folder that escapes").to_string();
    assert!(said.contains("members[0].folder") && said.contains("stays inside"), "{said}");
}

#[test]
fn a_project_that_is_wrong_as_a_whole_is_refused_once_its_keys_are_right() {
    let said = parse_project(Path::new("my-store.yml"), &text("my-store", "My store", &["store-web", "store-web"]))
        .expect_err("a repository listed twice")
        .to_string();
    assert!(said.contains("my-store.yml") && said.contains("`store-web` is listed twice"), "{said}");
}

#[test]
fn the_old_workspace_file_reads_for_the_migration_and_a_project_file_does_not_read_as_one() {
    let old = "version: 1\nworkspace_id: my-store\nname: My store\nrepositories:\n  - store-web\n  - store-api\n";
    let read = parse_workspace_v1(Path::new("my-store.yml"), old).expect("the old file");
    assert_eq!(read.id().as_str(), "my-store");
    assert_eq!(read.members(), [Member::whole(repository("store-web")), Member::whole(repository("store-api"))]);
    // Each reader refuses the other's file, so a file is never taken for what it is not.
    assert!(parse_project(Path::new("my-store.yml"), old).is_err());
    assert!(parse_workspace_v1(Path::new("my-store.yml"), &text("my-store", "My store", &["store-web"])).is_err());
}

#[test]
fn every_project_that_reads_runs_and_holds_the_repositories_its_members_name() {
    let projects = Projects::of(
        [
            at(ProjectPlace::ThisMac, "my-store.yml", text("my-store", "My store", &["store-web", "store-api"])),
            at(ProjectPlace::Synced, "platform.yml", text("platform", "Platform", &["store-api"])),
        ],
        Vec::new(),
    );
    assert_eq!(projects.running().count(), 2);
    let held: Vec<_> = projects.holding(&repository("store-api")).iter().map(|one| one.id().as_str()).collect();
    assert_eq!(held, ["my-store", "platform"], "one repository may be in several");
    assert!(projects.holding(&repository("back-office")).is_empty());
    let found = projects.get(&core_model::ScopeId::new("platform").expect("a scope")).expect("platform");
    assert_eq!(found.name().as_str(), "Platform");
}

#[test]
fn two_files_of_one_id_are_both_left_out_and_name_both() {
    let projects = Projects::of(
        [
            at(ProjectPlace::ThisMac, "my-store.yml", text("my-store", "My store", &["store-web"])),
            at(ProjectPlace::Synced, "copy.yml", text("my-store", "My store", &["store-web"])),
        ],
        Vec::new(),
    );
    assert_eq!(projects.running().count(), 0, "neither is guessed to be the one");
    let [one, other] = projects.left_out() else { panic!("both are left out: {:?}", projects.left_out()) };
    for left in [one, other] {
        assert!(left.said().contains("my-store.yml") && left.said().contains("copy.yml"), "{}", left.said());
    }
}

#[test]
fn a_file_that_does_not_fit_is_left_out_with_the_reason_and_the_others_stand() {
    let unread = ProjectLeftOut::new(ProjectPlace::Cloud, PathBuf::from("projects/gone.yml"), String::from("the bucket was not there"));
    let projects = Projects::of(
        [
            at(ProjectPlace::ThisMac, "broken.yml", String::from("version: 2\nproject_id: broken\n")),
            at(ProjectPlace::ThisMac, "my-store.yml", text("my-store", "My store", &["store-web"])),
        ],
        vec![unread],
    );
    assert_eq!(projects.running().count(), 1);
    let left: Vec<_> = projects.left_out().iter().map(|one| (one.path().to_path_buf(), one.place())).collect();
    assert!(left.contains(&(PathBuf::from("projects/gone.yml"), ProjectPlace::Cloud)), "{left:?}");
    let broken = projects.left_out().iter().find(|one| one.path().ends_with("broken.yml")).expect("broken is listed");
    assert!(broken.said().contains("name"), "{}", broken.said());
}

#[test]
fn a_project_is_written_as_the_file_that_reads_back_to_it() {
    let file = "version: 2\nproject_id: shop\nname: Shop\nmembers:\n  - repository: mono\n    folder: apps/api\n  - repository: web\n";
    let read = parse_project(Path::new("shop.yml"), file).expect("a Project");
    let written = project_text(&read).expect("text");
    assert_eq!(parse_project(Path::new("shop.yml"), &written).expect("read back"), read);
    let empty = parse_project(Path::new("new.yml"), &text("new", "New", &[])).expect("an empty Project");
    assert_eq!(parse_project(Path::new("new.yml"), &project_text(&empty).expect("text")).expect("read back"), empty);
}
