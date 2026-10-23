//! Whose pull requests are mine: either login, in any case, and every pull request where none is
//! known, and the person's own read by their login where the listing stops short of them. The
//! forge and the profiles are scripted; what is told with it is `pull_notices`' tests.

use adapter_traits::{FromOutside, OpenPull};

use super::pull_notices::{a_rig, Rig};
use super::repository_profiles::{repo, two_profiles};
use crate::my_pulls::Mine;
use crate::tests::tmp::TempDir;

fn logins(signed: Option<&str>, local: Option<&str>) -> Mine {
    Mine::of([signed.map(str::to_string), local.map(str::to_string)])
}

fn opened(number: u64, by: &str) -> OpenPull {
    OpenPull {
        number,
        title: FromOutside::verbatim(format!("Pull request {number}")),
        branch: FromOutside::verbatim(format!("armada/{number}")),
        url: FromOutside::verbatim(format!("https://forge.invalid/armada/pull/{number}")),
        author: Some(FromOutside::verbatim(by)),
        ci: None,
        head: None,
        failing: Vec::new(),
        auto_merge: false,
        facts: Default::default(),
    }
}

/// The hub's pull requests as the wire carries them: number, and whether they are the person's.
async fn on_the_wire(rig: &Rig) -> Vec<(u64, bool)> {
    let served = rig.fleet.repositories().served().remove(0);
    assert!(rig.fleet.notice_pulls(&served).await, "the listing answered");
    let hubs = rig.fleet.merge_hubs().await;
    let [(_, hub)] = hubs.as_slice() else { panic!("one repository's hub: {hubs:?}") };
    hub.pull_requests.iter().map(|pull| (pull.number, pull.mine)).collect()
}

/// **A busy repository's newest are all other people's**, so the listing stops before the
/// person's own: it is asked for by their login, and a pull request both name is drawn once.
#[tokio::test]
async fn the_persons_own_pull_request_the_listing_left_out_is_on_the_merge_line_as_theirs() {
    let rig = a_rig();
    let forge = &rig.fleet.vcs().main_ci;
    forge.login_is(Some("octocat"));
    forge.pulls_are(Some(vec![opened(30, "teammate"), opened(29, "teammate")]));
    forge.open_by_author_are(Some(vec![opened(30, "teammate"), opened(12, "OctoCat")]));

    assert_eq!(on_the_wire(&rig).await, [(30, false), (29, false), (12, true)]);
    assert_eq!(forge.authors_asked(), ["octocat"]);
}

/// The person's own are a second call: **a forge silent on it keeps what the listing gave**.
#[tokio::test]
async fn a_forge_silent_on_the_persons_own_keeps_what_the_listing_gave() {
    let rig = a_rig();
    let forge = &rig.fleet.vcs().main_ci;
    forge.login_is(Some("octocat"));
    forge.pulls_are(Some(vec![opened(30, "teammate")]));

    assert_eq!(on_the_wire(&rig).await, [(30, false)]);
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
