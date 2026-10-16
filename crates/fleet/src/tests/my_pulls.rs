//! Whose pull requests are mine: either login, in any case, and every pull request where none is
//! known. The forge and the profiles are scripted; what is told with it is `pull_notices`' tests.

use super::repository_profiles::{repo, two_profiles};
use crate::my_pulls::Mine;
use crate::tests::tmp::TempDir;

fn logins(signed: Option<&str>, local: Option<&str>) -> Mine {
    Mine::of([signed.map(str::to_string), local.map(str::to_string)])
}

#[test]
fn a_pull_request_is_mine_by_the_signed_in_login_or_by_the_forge_command_lines_in_any_case() {
    let both = logins(Some("octocat"), Some("Nick"));
    assert!(both.owns(Some("OctoCat")) && both.owns(Some("nick")));
    assert!(!both.owns(Some("teammate")));
    assert!(logins(Some("octocat"), None).owns(Some("octocat")));
    assert!(!logins(Some("octocat"), None).owns(Some("nick")));
    assert!(logins(None, Some("nick")).owns(Some("NICK")));
    assert!(!logins(None, Some("nick")).owns(Some("octocat")));
}

#[test]
fn where_neither_login_is_known_or_the_forge_named_no_author_every_pull_request_is_mine() {
    let unknown = logins(None, Some("  "));
    assert!(unknown.owns(Some("anyone")) && unknown.owns(None));
    assert!(logins(Some("octocat"), None).owns(None), "no author is mine");
    assert!(logins(Some("octocat"), None).owns(Some("")));
}

#[test]
fn the_signed_in_login_is_the_one_of_the_profile_the_repository_is_bound_to() {
    let home = TempDir::new();
    let signing = two_profiles(&home);
    assert_eq!(signing.login_for(&repo("mele-labs/armada")).as_deref(), Some("octocat"));
    assert_eq!(signing.login_for(&repo("acme/site")).as_deref(), Some("hubot"));
    assert_eq!(signing.login_for(&repo("shared-co/lib")), None, "two profiles match, so neither is chosen");
    assert_eq!(signing.login_for(&repo("stranger/thing")), None);
}
