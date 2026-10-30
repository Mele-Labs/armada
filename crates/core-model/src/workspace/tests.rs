//! A Workspace is the set of repositories a scope names, and nothing about a monorepo.

use alloc::vec::Vec;

use crate::envelope::Ulid;
use crate::lifecycle::ScopeId;
use crate::workflow::{ManifestId, Title};

use super::{MalformedWorkspace, Workspace};

fn repository(id: &str) -> ManifestId {
    ManifestId::carried(Ulid::carried(id))
}

fn workspace(id: &str, repositories: &[&str]) -> Result<Workspace, MalformedWorkspace> {
    let held: Vec<ManifestId> = repositories.iter().map(|one| repository(one)).collect();
    Workspace::declared(ScopeId::new(id).expect("a scope"), Title::new("My store").expect("a title"), held)
}

#[test]
fn a_workspace_holds_the_repositories_it_lists_in_the_order_it_lists_them() {
    let my_store = workspace("my-store", &["store-web", "store-api"]).expect("a Workspace");
    assert_eq!(my_store.repositories(), [repository("store-web"), repository("store-api")]);
    assert!(my_store.holds(&repository("store-api")));
    assert!(!my_store.holds(&repository("back-office")), "a repository it does not list is outside it");
    assert_eq!(my_store.id().as_str(), "my-store");
}

#[test]
fn a_workspace_that_holds_nothing_scopes_nothing_and_is_refused() {
    assert_eq!(workspace("my-store", &[]), Err(MalformedWorkspace::NoRepositories));
}

#[test]
fn a_repository_listed_twice_is_refused_by_name() {
    assert_eq!(
        workspace("my-store", &["store-web", "store-api", "store-web"]),
        Err(MalformedWorkspace::RepositoryTwice { repository: repository("store-web") })
    );
}

#[test]
fn a_workspace_named_like_one_of_its_own_repositories_is_refused() {
    assert_eq!(
        workspace("store-web", &["store-web", "store-api"]),
        Err(MalformedWorkspace::NamedLikeItsRepository { repository: repository("store-web") })
    );
    // Named like a repository it does not hold is another Workspace's business, and is allowed.
    assert_eq!(workspace("shop", &["store-web"]).map(|one| one.repositories().len()), Ok(1));
}

#[test]
fn what_is_wrong_is_said_in_a_sentence_that_names_the_repository() {
    let said = |wrong: MalformedWorkspace| alloc::format!("{wrong}");
    assert!(said(MalformedWorkspace::RepositoryTwice { repository: repository("store-web") }).contains("`store-web`"));
    assert!(said(MalformedWorkspace::NoRepositories).contains("at least one"));
}
