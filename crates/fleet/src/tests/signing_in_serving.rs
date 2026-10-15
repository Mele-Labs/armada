//! The account's four operations on a real Fleet: the two settings sign-in
//! needs are read where the request is made, a refusal carries a code, and the
//! machine is named as the settings say. What a sign-in leaves is
//! `crate::tests::signing_in`'s.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use api::{Accounts, Refusal, Settings};
use ipc::{AccountFlow, SaveSettings, SetRepositoryProfile, SettingValue, SignOut};
use testkit::{FakeHarness, FakeSignIn, FakeVcs, FakeWorkProduct};

use crate::daemon::Fleet;
use crate::signing_in::SigningIn;
use crate::tests::daemon::fitted_with;
use crate::tests::planted::Ticking;
use crate::tests::signing_in::{quickly, Enrolled};
use crate::tests::tmp::TempDir;

type Fixture = Fleet<FakeHarness, FakeVcs, FakeWorkProduct>;

fn fleet_over(home: &TempDir, forge: FakeSignIn) -> (Fixture, Arc<Enrolled>) {
    let store = Arc::new(Enrolled::new());
    let mut fittings = fitted_with(home, FakeWorkProduct::changed(&[]), FakeHarness::that_listens());
    fittings.signing_in =
        SigningIn::new(home.path(), Arc::new(forge), store.clone(), Arc::new(Ticking::from_nine())).paced(quickly());
    (Fleet::assembled(fittings), store)
}

async fn settings_saved(fleet: &Fixture, pairs: &[(&str, &str)]) {
    let changes = pairs
        .iter()
        .map(|(key, value)| (key.to_string(), Some(SettingValue::Text(value.to_string()))))
        .collect::<BTreeMap<_, _>>();
    fleet.save_settings(SaveSettings { changes }).await.expect("saved");
}

async fn until_idle(fleet: &Fixture) -> ipc::Accounts {
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        let read = fleet.get_accounts().await.expect("reads");
        if matches!(read.flow, AccountFlow::Idle { .. }) {
            return read;
        }
        assert!(Instant::now() < deadline, "the sign-in never finished: {read:?}");
        tokio::time::sleep(Duration::from_millis(2)).await;
    }
}

fn code_of(refusal: Refusal) -> (u16, String) {
    (refusal.status(), refusal.error().code.clone())
}

#[tokio::test]
async fn a_fleet_with_no_sign_in_app_or_store_address_refuses_to_start_one_and_asks_the_forge_nothing() {
    let home = TempDir::new();
    let forge = Arc::new(FakeSignIn::new());
    let mut fittings = fitted_with(&home, FakeWorkProduct::changed(&[]), FakeHarness::that_listens());
    fittings.signing_in = SigningIn::new(home.path(), forge.clone(), Arc::new(Enrolled::new()), Arc::new(Ticking::from_nine()));
    let fleet = Fleet::assembled(fittings);

    let refused = fleet.start_sign_in().await.unwrap_err();
    assert_eq!(code_of(refused.clone()), (422, "fleet.sign_in_unavailable".to_string()));
    assert!(refused.error().message.contains("the sign-in app and the store address are blank"), "{}", refused.error().message);
    assert!(forge.begun().is_empty());

    settings_saved(&fleet, &[("account.clientId", "Iv1.abc")]).await;
    let refused = fleet.start_sign_in().await.unwrap_err();
    assert!(refused.error().message.contains("the store address is blank"), "{}", refused.error().message);
}

#[tokio::test]
async fn with_both_settings_a_fleet_begins_a_sign_in_and_the_profile_is_there_when_it_ends() {
    let home = TempDir::new();
    let (fleet, store) = fleet_over(
        &home,
        FakeSignIn::new().pending(1).grants("tok", adapter_traits::SignedInAs { id: 583_231, login: "octocat".into() }),
    );
    settings_saved(
        &fleet,
        &[("account.clientId", "Iv1.abc"), ("store.url", "https://store.test"), ("machine.name", "mac-a")],
    )
    .await;

    let begun = fleet.start_sign_in().await.expect("begun");
    assert!(matches!(begun.flow, AccountFlow::Code { ref code, .. } if code == testkit::USER_CODE), "{:?}", begun.flow);

    let read = until_idle(&fleet).await;
    assert_eq!(read.profiles.len(), 1);
    assert_eq!(read.profiles[0].login, "octocat");
    assert_eq!(store.machines(), ["mac-a"], "the machine.name setting is how the others see this one");
}

#[tokio::test]
async fn a_blank_machine_name_is_the_hosts_and_the_store_is_told_something() {
    let home = TempDir::new();
    let (fleet, store) = fleet_over(
        &home,
        FakeSignIn::new().grants("tok", adapter_traits::SignedInAs { id: 1, login: "octocat".into() }),
    );
    settings_saved(&fleet, &[("account.clientId", "Iv1.abc"), ("store.url", "https://store.test")]).await;
    fleet.start_sign_in().await.expect("begun");
    until_idle(&fleet).await;
    let machines = store.machines();
    assert_eq!(machines.len(), 1);
    assert!(!machines[0].is_empty() && machines[0].chars().count() <= 64, "{machines:?}");
}

#[tokio::test]
async fn an_account_the_machine_has_not_joined_is_a_422_by_name_for_sign_out_and_for_choosing() {
    let home = TempDir::new();
    let (fleet, _) = fleet_over(&home, FakeSignIn::new());
    let out = fleet.sign_out(SignOut { account_id: 9 }).await.unwrap_err();
    assert_eq!(code_of(out), (422, "fleet.no_such_account".to_string()));
    let root = fleet.repositories().every()[0].root().to_string();
    let chosen = fleet.set_repository_profile(SetRepositoryProfile { root, account_id: Some(9) }).await.unwrap_err();
    assert_eq!(code_of(chosen), (422, "fleet.no_such_account".to_string()));
    let elsewhere = SetRepositoryProfile { root: "/not/served".to_string(), account_id: None };
    let unserved = fleet.set_repository_profile(elsewhere).await.unwrap_err();
    assert_eq!(code_of(unserved), (422, "fleet.no_such_repository".to_string()));
}

#[test]
fn the_machine_is_named_as_the_setting_says_else_as_the_host_does_and_made_to_fit() {
    use crate::signing_in::machine_name;
    assert_eq!(machine_name("work-mac", || "host".to_string()), "work-mac");
    assert_eq!(machine_name("  ", || "host.local\n".to_string()), "host.local");
    assert_eq!(machine_name("", String::new), "this machine");
    assert_eq!(machine_name(&"m".repeat(80), String::new).len(), 64);
    assert_eq!(machine_name("a\u{7}b", String::new), "ab");
}
