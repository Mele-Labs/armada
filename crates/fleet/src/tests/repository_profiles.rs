//! Which profile a repository claims as: the organisations a sign-in keeps, the
//! login its project picks, the choice that overrides it, a repository owner that
//! binds nothing, and the store each repository is claimed in.
//!
//! The sign-ins are the scripted forge and store of `signing_in`'s tests over a
//! real machine directory, so what is on disk is what is read back.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

use adapter_traits::{ClaimKind, Claimant, Store};
use ipc::{Accounts, BoundHow};
use store::{MainCi, MainState};
use testkit::{FakeSignIn, FakeStore, FakeWorkProduct};

use super::red_claim::Viewed;
use super::signing_in::{begin, files_under, holds, quickly, settled, Enrolled};
use crate::profiles;
use crate::red_claim::{keys_of, owner_key_of, Reaching, Taker};
use crate::repositories::{Located, SetUp};
use crate::repository_profiles::{Repo, FILE};
use crate::signing_in::{Refused, SigningIn};
use crate::tests::daemon::fittings;
use crate::tests::planted::Ticking;
use crate::tests::tmp::TempDir;

const OCTOCAT: u64 = 583_231;
const HUBOT: u64 = 9_919;

fn sign_in(home: &TempDir, store: &Arc<Enrolled>, forge: FakeSignIn) -> SigningIn {
    let signing = SigningIn::new(home.path(), Arc::new(forge), store.clone(), Arc::new(Ticking::from_nine())).paced(quickly());
    signing.start(begin(), &[]).expect("begun");
    settled(&signing);
    signing
}

/// Two accounts that share the organisation `shared-co`; each has one of its own.
pub(super) fn two_profiles(home: &TempDir) -> SigningIn {
    let store = Arc::new(Enrolled::new());
    sign_in(
        home,
        &store,
        FakeSignIn::new()
            .grants("tok-a", adapter_traits::SignedInAs { id: OCTOCAT, login: "octocat".into() })
            .in_organisations("tok-a", &["mele-labs", "shared-co"]),
    );
    sign_in(
        home,
        &store,
        FakeSignIn::new()
            .grants("tok-b", adapter_traits::SignedInAs { id: HUBOT, login: "hubot".into() })
            .in_organisations("tok-b", &["acme", "shared-co"]),
    )
}

pub(super) fn repo(slug: &str) -> Repo {
    Repo::new(&format!("/work/{}", slug.replace('/', "-")))
}

/// The account `repo` is listed under and how. **Nothing is ever unmatched** since a project picks the login.
fn standing(accounts: &Accounts, repo: &Repo) -> Option<(u64, BoundHow)> {
    assert!(accounts.unmatched.is_empty(), "no repository is listed as bound to none");
    accounts.profiles.iter().find_map(|profile| {
        profile.repositories.iter().find(|bound| bound.root == repo.root).map(|bound| (profile.account_id, bound.how))
    })
}

/// The account `repo` claims as where its project picked `project`.
fn claims(signing: &SigningIn, repo: &Repo, project: Option<u64>) -> Option<u64> {
    signing.profile_for(&repo.root, project).map(|bound| bound.account_id)
}

#[test]
fn sign_in_keeps_the_organisations_and_when_they_were_read_and_never_the_token() {
    let home = TempDir::new();
    let signing = sign_in(
        &home,
        &Arc::new(Enrolled::new()),
        FakeSignIn::new()
            .grants("tok-secret-1", adapter_traits::SignedInAs { id: OCTOCAT, login: "octocat".into() })
            .in_organisations("tok-secret-1", &["mele-labs", "shared-co"]),
    );
    assert_eq!(signing.accounts(&[]).unwrap().profiles.len(), 1);

    let text = std::fs::read_to_string(home.path().join(profiles::FILE)).expect("the list");
    for held in ["\"orgs\":[\"mele-labs\",\"shared-co\"]", "\"orgs_read_at\":\"2026-"] {
        assert!(text.contains(held), "{held} is not in {text}");
    }
    assert!(!text.contains("orgs_unread"), "{text}");
    for secret in ["tok-secret-1", "fake-device-code"] {
        assert!(!holds(home.path(), secret), "{secret} is on disk: {:?}", files_under(home.path()));
    }
}

#[test]
fn a_forge_that_will_not_list_organisations_keeps_none_and_says_so_and_the_sign_in_stands() {
    let home = TempDir::new();
    let signing = sign_in(
        &home,
        &Arc::new(Enrolled::new()),
        FakeSignIn::new()
            .grants("tok", adapter_traits::SignedInAs { id: OCTOCAT, login: "octocat".into() })
            .refusing_organisations(),
    );
    assert_eq!(signing.accounts(&[]).unwrap().profiles.len(), 1, "a sign-in is not failed for want of the list");

    let text = std::fs::read_to_string(home.path().join(profiles::FILE)).expect("the list");
    assert!(text.contains("\"orgs\":[]") && text.contains("\"orgs_unread\":true"), "{text}");
    assert!(!text.contains("orgs_read_at"), "{text}");
}

#[test]
fn a_repository_owner_or_organisation_binds_nothing_and_the_projects_login_is_used() {
    let home = TempDir::new();
    let signing = two_profiles(&home);
    let (armada, dotfiles) = (repo("mele-labs/armada"), repo("octocat/dotfiles"));
    let read = signing.accounts(&[armada.clone(), dotfiles.clone()]).unwrap();
    assert_eq!((standing(&read, &armada), standing(&read, &dotfiles)), (None, None), "octocat's organisation and login bind nothing");

    assert_eq!(claims(&signing, &armada, None), None, "no login picked, nothing claimed");
    assert_eq!(claims(&signing, &armada, Some(HUBOT)), Some(HUBOT), "the project's login, not the owner's match");
    let bound = signing.profile_for(&dotfiles.root, Some(HUBOT)).expect("bound");
    assert_eq!(bound.login, "hubot");
}

#[test]
fn a_projects_login_this_machine_is_not_signed_in_to_claims_nothing() {
    let home = TempDir::new();
    let signing = two_profiles(&home);
    assert_eq!(claims(&signing, &repo("mele-labs/armada"), Some(7)), None);
}

#[test]
fn a_repositorys_choice_wins_over_its_projects_login_and_survives_a_restart() {
    let home = TempDir::new();
    let signing = two_profiles(&home);
    let (armada, site) = (repo("mele-labs/armada"), repo("acme/site"));
    let both = [armada.clone(), site.clone()];

    let read = signing.set_repository_profile(&armada.root, Some(HUBOT), &both).expect("chosen");
    assert_eq!(standing(&read, &armada), Some((HUBOT, BoundHow::Chosen)));
    assert_eq!(standing(&read, &site), None, "one with no choice is not listed");
    assert_eq!(claims(&signing, &armada, Some(OCTOCAT)), Some(HUBOT), "the choice wins");
    assert_eq!(claims(&signing, &site, Some(OCTOCAT)), Some(OCTOCAT), "no choice is the project's login");
    assert_eq!(claims(&signing, &armada, None), None, "a project that picked no login sends nothing, a choice or not");

    let restarted = SigningIn::new(home.path(), Arc::new(FakeSignIn::new()), Arc::new(Enrolled::new()), Arc::new(Ticking::from_nine()));
    assert_eq!(standing(&restarted.accounts(&both).unwrap(), &armada), Some((HUBOT, BoundHow::Chosen)));
    assert_eq!(claims(&restarted, &armada, Some(OCTOCAT)), Some(HUBOT));

    let text = std::fs::read_to_string(home.path().join(FILE)).expect("the choices");
    assert!(text.contains("\"how\":\"chosen\""), "{text}");
    assert!(files_under(home.path()).iter().all(|file| !file.to_string_lossy().ends_with(".tmp")), "no temp file is left");
}

#[test]
fn choosing_none_claims_nothing_over_the_projects_login() {
    let home = TempDir::new();
    let signing = two_profiles(&home);
    let armada = repo("mele-labs/armada");
    assert_eq!(claims(&signing, &armada, Some(OCTOCAT)), Some(OCTOCAT));

    let read = signing.set_repository_profile(&armada.root, None, &[armada.clone()]).expect("chosen");
    assert_eq!(standing(&read, &armada), None);
    assert_eq!(claims(&signing, &armada, Some(OCTOCAT)), None);
    assert!(std::fs::read_to_string(home.path().join(FILE)).unwrap().contains("\"account_id\":null"));
}

#[test]
fn a_repository_chosen_for_a_profile_that_signed_out_has_no_store_and_is_not_handed_to_the_projects_login() {
    let home = TempDir::new();
    let signing = two_profiles(&home);
    let armada = repo("mele-labs/armada");
    signing.set_repository_profile(&armada.root, Some(HUBOT), &[armada.clone()]).expect("chosen");
    assert_eq!(claims(&signing, &armada, Some(OCTOCAT)), Some(HUBOT));

    let out = signing.sign_out(HUBOT, &[armada.clone()]).expect("signed out");
    assert_eq!(standing(&out.accounts, &armada), None);
    assert_eq!(claims(&signing, &armada, Some(OCTOCAT)), None);
}

#[test]
fn a_choice_for_an_account_never_joined_or_a_repository_never_served_is_refused_and_keeps_nothing() {
    let home = TempDir::new();
    let signing = two_profiles(&home);
    let armada = repo("mele-labs/armada");
    assert_eq!(signing.set_repository_profile(&armada.root, Some(7), &[armada.clone()]), Err(Refused::NoSuchAccount(7)));
    assert_eq!(
        signing.set_repository_profile("/work/elsewhere", None, &[armada.clone()]),
        Err(Refused::NoSuchRepository("/work/elsewhere".to_string()))
    );
    assert!(!home.path().join(FILE).exists());
}

/// A machine that answers for two accounts' views of the store, by the profile
/// directory the call opens, and for each repository's address by its root:
/// the folder called `site` is `acme/site`, any other `mele-labs/armada`.
struct PerProfile {
    stores: BTreeMap<String, Arc<FakeStore>>,
}

impl Reaching for PerProfile {
    fn slug(&self, root: &str) -> Option<String> {
        let site = Path::new(root).file_name().is_some_and(|name| name == "site");
        Some(if site { "acme/site" } else { "mele-labs/armada" }.to_string())
    }

    fn open(&self, _url: &str, directory: &Path) -> Result<Option<Box<dyn Store + Send>>, String> {
        // `<profile>/stores/<the store's>`: the profile's folder is two up.
        let profile = directory.parent().and_then(Path::parent).and_then(Path::file_name);
        let account = profile.and_then(|name| name.to_str()).unwrap_or_default();
        Ok(self.stores.get(account).map(|view| Box::new(Viewed(Arc::clone(view))) as Box<dyn Store + Send>))
    }

    fn bucket(&self, _address: &adapters::BucketAddress) -> Result<Box<dyn adapter_traits::Bucket + Send>, adapter_traits::BucketError> {
        Err(adapter_traits::BucketError::Unsigned(String::from("this fake machine has no bucket")))
    }
}

const SECOND_MANIFEST: &str = "version: 1\nid: 01SECONDMANIFEST\nbase: main\nchecks:\n  test:\n    run: cargo nextest run\npolicy:\n  pull_request_owners: true\n";
const RED: &str = "1111111111111111111111111111111111111111";

struct TwoRepositories {
    fleet: super::main_ci::Fixture,
    first: String,
    second: String,
    octocat: Arc<FakeStore>,
    hubot: Arc<FakeStore>,
    _home: TempDir,
}

/// One Fleet serving `mele-labs/armada` and `acme/site`, each switched to the registry, with a store
/// of its own for each account, in a project whose login is octocat. `acme/site` overrides it with hubot.
async fn two_repositories() -> TwoRepositories {
    let home = TempDir::new();
    two_profiles(&home);
    let (octocat, hubot) = (Arc::new(FakeStore::new()), Arc::new(FakeStore::new()));
    let mut fittings = fittings(&home, FakeWorkProduct::changed(&["src/log.rs"]));
    let manifest = format!("{}policy:\n  pull_request_owners: true\n", super::main_ci::MANIFEST);
    fittings.starting().manifest = config::Manifest::parse(Path::new("armada.yml"), &manifest).expect("parses");
    let second = home.path().join("site").to_string_lossy().to_string();
    fittings.reaching = Arc::new(PerProfile {
        stores: BTreeMap::from([(OCTOCAT.to_string(), Arc::clone(&octocat)), (HUBOT.to_string(), Arc::clone(&hubot))]),
    });
    let fleet = crate::daemon::Fleet::assembled(fittings);
    let changes = [("machine.name", "mac-a")]
        .into_iter()
        .map(|(key, value)| (key.to_string(), Some(ipc::SettingValue::Text(value.to_string()))))
        .collect();
    api::Settings::save_settings(&fleet, ipc::SaveSettings { changes }).await.expect("saved");
    let parsed = config::Manifest::parse(&Path::new(&second).join("armada.yml"), SECOND_MANIFEST).expect("parses");
    fleet
        .repositories()
        .add(Located {
            root: second.clone(),
            records_root: home.path().join("site-records").to_string_lossy().to_string(),
            set_up: Some(SetUp::of(parsed, BTreeMap::new())),
        })
        .expect("served");
    super::red_claim::in_a_project_storing_at(&fleet, "https://store.test").await;
    api::Accounts::set_repository_profile(&fleet, ipc::SetRepositoryProfile { root: second.clone(), account_id: Some(HUBOT) })
        .await
        .expect("chosen");
    let first = super::main_fix::root(&fleet).await;
    TwoRepositories { fleet, first, second, octocat, hubot, _home: home }
}

fn red_at(root: &str, fleet: &super::main_ci::Fixture) -> MainCi {
    MainCi {
        repository: root.to_string(),
        base: "main".to_string(),
        commit: RED.to_string(),
        state: MainState::Red,
        read_at: fleet.now(),
        red_at: Some(fleet.now()),
        red_commit: Some(RED.to_string()),
        decided_commit: Some(RED.to_string()),
        newer_running: 0,
        unfinished: 0,
        failed: Vec::new(),
        merge: None,
    }
}

fn claim_of(store: &FakeStore, slug: &str) -> Option<Claimant> {
    let (repository, red) = keys_of(slug, RED).expect("keys");
    store.read_claim(&repository, ClaimKind::Red, &red).expect("reads")
}

fn owner_of(store: &FakeStore, slug: &str, number: u64) -> bool {
    let key = owner_key_of(slug, number).expect("a key");
    !store.owners(&[key]).expect("reads").is_empty()
}

#[tokio::test]
async fn a_red_is_claimed_as_the_projects_login_or_the_repositorys_override_and_no_other() {
    let two = two_repositories().await;
    let taker = Taker::Session("s-1".to_string());

    assert!(two.fleet.claim_red(&red_at(&two.first, &two.fleet), &taker, false).await.expect("claimed"));
    assert!(two.fleet.claim_red(&red_at(&two.second, &two.fleet), &taker, false).await.expect("claimed"));

    let name = |claim: Option<Claimant>| claim.map(|held| held.name);
    assert_eq!(name(claim_of(&two.octocat, "mele-labs/armada")).as_deref(), Some("octocat"), "the project's login, octocat, in octocat's store");
    assert_eq!(name(claim_of(&two.hubot, "acme/site")).as_deref(), Some("hubot"), "the override, hubot, in hubot's store");
    assert!(claim_of(&two.octocat, "acme/site").is_none() && claim_of(&two.hubot, "mele-labs/armada").is_none());
}

#[tokio::test]
async fn a_pull_request_owner_is_registered_in_the_view_of_the_profile_its_repository_claims_as_and_no_other() {
    let two = two_repositories().await;
    let taker = Taker::Session("s-1".to_string());

    assert!(two.fleet.pull_request_registered(&two.first, 7, &taker).await.expect("registered"));
    assert!(two.fleet.pull_request_registered(&two.second, 8, &taker).await.expect("registered"));

    assert!(owner_of(&two.octocat, "mele-labs/armada", 7) && !owner_of(&two.hubot, "mele-labs/armada", 7));
    assert!(owner_of(&two.hubot, "acme/site", 8) && !owner_of(&two.octocat, "acme/site", 8));
    let read = two.fleet.owners_read(&two.second, vec![8]).await;
    assert_eq!(read.get(&8).map(|owner| owner.holder.id.as_str()), Some("s-1"), "and is read back from the same view");
}

#[tokio::test]
async fn a_repository_chosen_none_claims_nothing_while_the_other_still_does() {
    let two = two_repositories().await;
    let taker = Taker::Session("s-1".to_string());
    api::Accounts::set_repository_profile(&two.fleet, ipc::SetRepositoryProfile { root: two.second.clone(), account_id: None })
        .await
        .expect("chosen");

    assert!(!two.fleet.claim_red(&red_at(&two.second, &two.fleet), &taker, false).await.expect("no store applies"));
    assert!(!two.fleet.pull_request_registered(&two.second, 8, &taker).await.expect("no store applies"));
    assert!(two.fleet.claim_red(&red_at(&two.first, &two.fleet), &taker, false).await.expect("claimed"));
    assert_eq!(two.hubot.calls(), Vec::<&str>::new(), "hubot's store was never asked");
}

#[tokio::test]
async fn a_repository_claiming_as_a_profile_the_machine_signed_out_of_claims_nothing() {
    let two = two_repositories().await;
    let taker = Taker::Session("s-1".to_string());
    let out = api::Accounts::sign_out(&two.fleet, ipc::SignOut { account_id: OCTOCAT }).await.expect("signed out");
    assert_eq!(out.accounts.profiles.len(), 1);

    assert!(!two.fleet.claim_red(&red_at(&two.first, &two.fleet), &taker, false).await.expect("no store applies"));
    assert!(two.fleet.claim_red(&red_at(&two.second, &two.fleet), &taker, false).await.expect("claimed"));
}
