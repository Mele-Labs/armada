//! Where a profile's shared home is made: chosen before it exists, kept per profile on this
//! machine, made there by Sync, and found when another Mac already made it. A Fleet over a
//! temporary directory with the profile joined, its forge a `FakeRemote`.

use std::sync::Arc;

use adapter_traits::SignedInAs;
use api::{Accounts, Refusal, SharedHome};
use ipc::{HomeNamed, HomePlanned, HomeProfile, ProfileHome, SetHomeRepository, SharedHomeRead};
use testkit::{FakeHarness, FakeHarnessHome, FakeHomes, FakeRemote, FakeVcs, FakeWorkProduct, HeldFolder};

use crate::daemon::Fleet;
use crate::home::HomeFittings;
use crate::profiles::{Orgs, Profiles};
use crate::tests::daemon::a_fleet;
use crate::tests::tmp::TempDir;

const LOGIN: &str = "nick";

struct Rig {
    fleet: Fleet<FakeHarness, FakeVcs, FakeWorkProduct>,
    remote: FakeRemote,
    _home: TempDir,
}

/// `nick` joined with the organisations `orgs`, its forge `remote`.
fn rig(remote: FakeRemote, orgs: &[&str]) -> Rig {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&[]));
    let orgs = Orgs::Read(orgs.iter().map(|org| org.to_string()).collect());
    Profiles::in_machine_directory(home.path())
        .join(&SignedInAs { id: 1, login: LOGIN.to_string() }, orgs, "2026-10-25T00:00:00Z")
        .expect("joined");
    fleet.sharing_home_with(HomeFittings {
        homes: Arc::new(FakeHomes::new().holding(&remote)),
        mods: Arc::new(HeldFolder::new()),
        kit: Arc::new(HeldFolder::new()),
        harness: Arc::new(FakeHarnessHome::over(HeldFolder::new())),
    });
    Rig { fleet, remote, _home: home }
}

fn profile() -> HomeProfile {
    HomeProfile { login: LOGIN.to_string() }
}

fn choice(owner: &str, name: &str) -> SetHomeRepository {
    SetHomeRepository { login: LOGIN.to_string(), owner: owner.to_string(), name: name.to_string() }
}

fn planned(account: &str, repository: &str, exists: bool) -> Option<HomePlanned> {
    Some(HomePlanned { account: account.to_string(), repository: repository.to_string(), exists })
}

fn named(account: &str, repository: &str) -> Option<HomeNamed> {
    Some(HomeNamed { account: account.to_string(), repository: repository.to_string() })
}

fn refused<T: std::fmt::Debug>(answer: Result<T, Refusal>) -> (u16, String, String) {
    let refusal = answer.expect_err("refused");
    (refusal.status(), refusal.error().code.clone(), refusal.error().message.clone())
}

async fn read(rig: &Rig) -> SharedHomeRead {
    rig.fleet.get_shared_home(profile()).await.expect("a read")
}

async fn home_of_profile(rig: &Rig) -> (Vec<String>, ProfileHome) {
    let accounts = rig.fleet.get_accounts().await.expect("accounts");
    let nick = accounts.profiles.into_iter().find(|it| it.login == LOGIN).expect("the profile");
    (nick.orgs, nick.home)
}

#[tokio::test]
async fn with_nothing_chosen_the_home_is_planned_under_the_login_and_sync_makes_it_there() {
    let rig = rig(FakeRemote::of(LOGIN), &[]);
    let before = read(&rig).await;
    assert_eq!((before.home, before.planned), (None, planned(LOGIN, "armada-home", false)));
    let (orgs, home) = home_of_profile(&rig).await;
    assert_eq!((orgs, home), (vec![], ProfileHome { account: LOGIN.into(), repository: "armada-home".into(), made: false }));

    let synced = rig.fleet.sync_shared_home(profile()).await.expect("synced");
    assert!(rig.remote.exists() && rig.remote.marked_at(LOGIN, "armada-home"));
    assert_eq!((synced.read.home, synced.read.planned), (named(LOGIN, "armada-home"), None));
    assert_eq!(home_of_profile(&rig).await.1, ProfileHome { account: LOGIN.into(), repository: "armada-home".into(), made: true });
}

#[tokio::test]
async fn a_choice_is_kept_for_its_profile_served_as_planned_and_made_there_by_sync() {
    let rig = rig(FakeRemote::of(LOGIN).making_in("mele-labs"), &["mele-labs"]);
    let chosen = rig.fleet.set_home_repository(choice("Mele-Labs", "kit")).await.expect("recorded");
    assert_eq!((chosen.home, chosen.planned), (None, planned("mele-labs", "kit", false)), "the organisation as the profile keeps it");
    assert_eq!(read(&rig).await.planned, planned("mele-labs", "kit", false));
    let (orgs, home) = home_of_profile(&rig).await;
    assert_eq!((orgs, home), (vec!["mele-labs".to_string()], ProfileHome { account: "mele-labs".into(), repository: "kit".into(), made: false }));
    assert!(!rig.remote.exists_at("mele-labs", "kit"), "nothing is made by a choice");

    let again = rig.fleet.set_home_repository(choice(LOGIN, "notes")).await.expect("a second choice replaces the first");
    assert_eq!(again.planned, planned(LOGIN, "notes", false));
    rig.fleet.set_home_repository(choice("mele-labs", "kit")).await.expect("recorded");

    let synced = rig.fleet.sync_shared_home(profile()).await.expect("synced");
    assert!(rig.remote.exists_at("mele-labs", "kit") && rig.remote.marked_at("mele-labs", "kit"));
    assert!(!rig.remote.exists(), "not the default");
    assert_eq!((synced.read.home, synced.read.planned), (named("mele-labs", "kit"), None));
    assert_eq!(home_of_profile(&rig).await.1, ProfileHome { account: "mele-labs".into(), repository: "kit".into(), made: true });
    let (status, code, _) = refused(rig.fleet.set_home_repository(choice(LOGIN, "notes")).await);
    assert_eq!((status, code.as_str()), (409, "fleet.home_already_made"), "a copy here is shown and not changed");
}

#[tokio::test]
async fn a_choice_that_is_not_a_profiles_owner_or_a_name_the_forge_takes_is_refused() {
    let rig = rig(FakeRemote::of(LOGIN), &["mele-labs"]);
    for (owner, name) in [("stranger", "kit"), (LOGIN, ""), (LOGIN, "."), (LOGIN, ".."), (LOGIN, "a/b"), (LOGIN, "has space"), (LOGIN, &"x".repeat(101))] {
        let (status, code, said) = refused(rig.fleet.set_home_repository(choice(owner, name)).await);
        assert_eq!((status, code.as_str()), (422, "fleet.home_choice_refused"), "{owner}/{name}: {said}");
        assert!(!said.contains('\u{2014}') && !said.contains('\u{2013}'), "no dashes: {said}");
    }
    for (owner, name) in [(LOGIN, "armada-home"), ("MELE-LABS", "a.b_c-1"), (LOGIN, &"x".repeat(100))] {
        rig.fleet.set_home_repository(choice(owner, name)).await.unwrap_or_else(|why| panic!("{owner}/{name}: {why:?}"));
    }
    let stranger = SetHomeRepository { login: "stranger".to_string(), owner: "stranger".to_string(), name: "kit".to_string() };
    let (status, code, _) = refused(rig.fleet.set_home_repository(stranger).await);
    assert_eq!((status, code.as_str()), (404, "fleet.no_such_profile"));
}

#[tokio::test]
async fn an_organisation_the_login_may_not_make_repositories_in_is_refused_by_sync_in_a_sentence_of_its_own() {
    let rig = rig(FakeRemote::of(LOGIN), &["mele-labs"]);
    rig.fleet.set_home_repository(choice("mele-labs", "kit")).await.expect("recorded");
    let (status, code, said) = refused(rig.fleet.sync_shared_home(profile()).await);
    assert_eq!((status, code.as_str()), (422, "fleet.home_refused"));
    assert_eq!(said, "nick cannot make repositories in mele-labs. Ask an owner of mele-labs to allow it, or choose nick");
    assert!(!rig.remote.exists_at("mele-labs", "kit"));
    assert_eq!(read(&rig).await.planned, planned("mele-labs", "kit", false), "still a choice to change");

    let rig = self::rig(FakeRemote::of(LOGIN).with_repository("mele-labs", "kit", false), &["mele-labs"]);
    rig.fleet.set_home_repository(choice("mele-labs", "kit")).await.expect("recorded");
    let (status, code, said) = refused(rig.fleet.sync_shared_home(profile()).await);
    assert_eq!((status, code.as_str()), (422, "fleet.home_refused"));
    assert_eq!(said, "mele-labs/kit exists and nick cannot write to it. Choose another name");
}

#[tokio::test]
async fn a_home_another_mac_made_is_found_once_served_as_existing_and_not_changed_and_sync_clones_it() {
    let rig = rig(FakeRemote::of(LOGIN).with_home("mele-labs", "kit"), &["mele-labs"]);
    assert_eq!(read(&rig).await.planned, planned("mele-labs", "kit", true));
    assert_eq!(read(&rig).await.planned, planned("mele-labs", "kit", true));
    assert_eq!(rig.remote.listings(), 1, "the forge is asked once");
    assert_eq!(home_of_profile(&rig).await.1, ProfileHome { account: "mele-labs".into(), repository: "kit".into(), made: false });

    let (status, code, _) = refused(rig.fleet.set_home_repository(choice(LOGIN, "armada-home")).await);
    assert_eq!((status, code.as_str()), (409, "fleet.home_already_made"));

    let synced = rig.fleet.sync_shared_home(profile()).await.expect("synced");
    assert_eq!(synced.read.home, named("mele-labs", "kit"));
    assert!(!rig.remote.exists(), "no second home is made beside it");
}

#[tokio::test]
async fn a_choice_made_before_any_read_still_finds_a_home_another_mac_made_and_is_refused() {
    let rig = rig(FakeRemote::of(LOGIN).with_home("mele-labs", "kit"), &["mele-labs"]);
    let (status, code, _) = refused(rig.fleet.set_home_repository(choice(LOGIN, "mine")).await);
    assert_eq!((status, code.as_str()), (409, "fleet.home_already_made"));
    assert_eq!(read(&rig).await.planned, planned("mele-labs", "kit", true));
}

#[tokio::test]
async fn a_choice_kept_from_an_earlier_run_is_never_searched_for() {
    let rig = rig(FakeRemote::of(LOGIN).with_home("mele-labs", "kit"), &["mele-labs"]);
    rig.fleet.store().lock().await.set_home_repository(LOGIN, LOGIN, "mine", false).expect("kept");
    assert_eq!(read(&rig).await.planned, planned(LOGIN, "mine", false), "the choice, not the home on the forge");
    rig.fleet.sync_shared_home(profile()).await.expect("synced");
    assert_eq!(rig.remote.listings(), 0, "never asked");
    assert!(rig.remote.exists_at(LOGIN, "mine"));
}

#[tokio::test]
async fn a_forge_that_does_not_answer_leaves_the_default_and_is_asked_once() {
    let rig = rig(FakeRemote::of(LOGIN).failing_to_list(), &[]);
    assert_eq!(read(&rig).await.planned, planned(LOGIN, "armada-home", false));
    assert_eq!(read(&rig).await.planned, planned(LOGIN, "armada-home", false));
    assert_eq!(rig.remote.listings(), 1, "not on every read");
}
