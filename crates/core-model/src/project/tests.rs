//! A Project is the set of members a scope names, whole repositories or folders of one.

use alloc::vec::Vec;

use crate::envelope::Ulid;
use crate::lifecycle::ScopeId;
use crate::workflow::{ManifestId, Title};

use super::{Folder, MalformedFolder, MalformedProject, Member, Project};

fn repository(id: &str) -> ManifestId {
    ManifestId::carried(Ulid::carried(id))
}

fn whole(id: &str) -> Member {
    Member::whole(repository(id))
}

fn folder_of(id: &str, folder: &str) -> Member {
    Member::folder(repository(id), Folder::new(folder).expect("a folder"))
}

fn project(id: &str, members: Vec<Member>) -> Result<Project, MalformedProject> {
    Project::declared(ScopeId::new(id).expect("a scope"), Title::new("My store").expect("a title"), members)
}

#[test]
fn a_project_holds_the_members_it_lists_in_the_order_it_lists_them() {
    let my_store = project("my-store", alloc::vec![whole("store-web"), folder_of("monorepo", "apps/api")]).expect("a Project");
    assert_eq!(my_store.members(), [whole("store-web"), folder_of("monorepo", "apps/api")]);
    assert!(my_store.holds(&repository("monorepo")), "a folder of a repository holds the repository");
    assert!(!my_store.holds(&repository("back-office")), "a repository no member names is outside it");
    assert_eq!(my_store.id().as_str(), "my-store");
    assert_eq!(my_store.repositories(), [&repository("store-web"), &repository("monorepo")]);
}

#[test]
fn a_new_project_is_empty_and_holds_nothing() {
    let empty = project("my-store", Vec::new()).expect("an empty Project is allowed");
    assert!(empty.members().is_empty());
    assert!(!empty.holds(&repository("store-web")));
}

#[test]
fn a_member_listed_twice_is_refused_by_name() {
    assert_eq!(
        project("my-store", alloc::vec![whole("store-web"), whole("store-api"), whole("store-web")]),
        Err(MalformedProject::MemberTwice { repository: repository("store-web"), folder: None })
    );
    assert_eq!(
        project("my-store", alloc::vec![folder_of("mono", "apps/a"), folder_of("mono", "apps/a")]),
        Err(MalformedProject::MemberTwice { repository: repository("mono"), folder: Folder::new("apps/a").ok() })
    );
    // Two folders of one repository are two members.
    assert!(project("my-store", alloc::vec![folder_of("mono", "apps/a"), folder_of("mono", "apps/b")]).is_ok());
}

#[test]
fn a_whole_repository_beside_a_folder_of_it_is_refused_in_either_order() {
    let said = Err(MalformedProject::WholeBesideFolder { repository: repository("mono") });
    assert_eq!(project("my-store", alloc::vec![whole("mono"), folder_of("mono", "apps/a")]), said);
    assert_eq!(project("my-store", alloc::vec![folder_of("mono", "apps/a"), whole("mono")]), said);
}

#[test]
fn a_project_named_like_one_of_its_own_repositories_is_refused() {
    assert_eq!(
        project("store-web", alloc::vec![whole("store-web"), whole("store-api")]),
        Err(MalformedProject::NamedLikeItsRepository { repository: repository("store-web") })
    );
    // Named like a repository it does not hold is another Project's business, and is allowed.
    assert_eq!(project("shop", alloc::vec![whole("store-web")]).map(|one| one.members().len()), Ok(1));
}

#[test]
fn a_folder_is_a_relative_slash_separated_path_that_stays_inside() {
    assert_eq!(Folder::new("apps/api").map(|one| alloc::string::String::from(one.as_str())), Ok(alloc::string::String::from("apps/api")));
    assert_eq!(Folder::new(""), Err(MalformedFolder::Empty));
    assert_eq!(Folder::new("/apps"), Err(MalformedFolder::EdgeSlash));
    assert_eq!(Folder::new("apps/"), Err(MalformedFolder::EdgeSlash));
    assert_eq!(Folder::new("apps/../x"), Err(MalformedFolder::Segment));
    assert_eq!(Folder::new("./apps"), Err(MalformedFolder::Segment));
    assert_eq!(Folder::new(".."), Err(MalformedFolder::Segment));
    assert_eq!(Folder::new("apps//api"), Err(MalformedFolder::Segment));
    assert!(Folder::new(".github").is_ok(), "a dot inside a name is not a dot segment");
}

#[test]
fn what_is_wrong_is_said_in_a_sentence_that_names_the_repository() {
    let said = |wrong: MalformedProject| alloc::format!("{wrong}");
    assert!(said(MalformedProject::MemberTwice { repository: repository("store-web"), folder: None }).contains("`store-web`"));
    assert!(said(MalformedProject::WholeBesideFolder { repository: repository("mono") }).contains("`mono`"));
}
