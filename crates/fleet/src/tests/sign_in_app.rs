//! Registering the sign-in app in one press, over a scripted forge and a real
//! settings.json: the state a press issues is good once and for an hour, a state
//! that is not live converts nothing, and a conversion keeps the client id and
//! nothing else, or keeps nothing and says why.

use std::sync::Arc;

use api::{Accounts, Refusal, Settings};
use testkit::{FakeSignIn, FakeVcs, FakeWorkProduct, FakeHarness, APP_CLIENT_ID, APP_CODE, APP_SETTINGS_PAGE};

use crate::daemon::Fleet;
use crate::signing_in::SigningIn;
use crate::tests::daemon::fittings;
use crate::tests::planted::Held;
use crate::tests::settings_json::settings_file_in;
use crate::tests::signing_in::Enrolled;
use crate::tests::tmp::TempDir;

const KEY: &str = "account.clientId";

struct Rig {
    fleet: Fleet<FakeHarness, FakeVcs, FakeWorkProduct>,
    forge: Arc<FakeSignIn>,
    clock: Arc<Held>,
    home: TempDir,
}

fn rig(forge: FakeSignIn) -> Rig {
    let home = TempDir::new();
    let forge = Arc::new(forge);
    let clock = Arc::new(Held::started());
    let mut fitted = fittings(&home, FakeWorkProduct::changed(&[]));
    fitted.signing_in = SigningIn::new(home.path(), forge.clone(), Arc::new(Enrolled::new()), clock.clone());
    Rig { fleet: Fleet::assembled(fitted), forge, clock, home }
}

/// A press, and the state on the address it answers.
async fn pressed(rig: &Rig) -> String {
    let registration = rig.fleet.register_sign_in_app().await.expect("a page to open");
    let (page, state) = registration.url.split_once("?state=").expect("a state on the address");
    assert!(page.starts_with("http://127.0.0.1:") && page.ends_with("/accounts/sign_in_app/form"), "{page}");
    assert_eq!(state.len(), 64, "{state}");
    state.to_string()
}

fn refused<T: std::fmt::Debug>(answer: Result<T, Refusal>) -> (String, String) {
    let refusal = answer.expect_err("refused");
    assert_eq!(refusal.status(), 422, "{refusal:?}");
    (refusal.error().code.clone(), refusal.error().message.clone())
}

async fn client_id(rig: &Rig) -> ipc::SettingValue {
    let list = rig.fleet.get_settings().await.expect("the settings");
    list.settings.into_iter().find(|one| one.key == KEY).expect(KEY).value
}

#[tokio::test]
async fn a_state_carries_the_browser_to_the_forge_and_back_once_and_a_second_return_is_refused() {
    let rig = rig(FakeSignIn::new());
    let state = pressed(&rig).await;

    let page = rig.fleet.get_sign_in_app_form(state.clone()).await.expect("the page");
    assert!(page.contains(&format!("action=\"https://forge.test/settings/apps/new?state={state}\"")), "{page}");
    assert!(page.contains("/accounts/sign_in_app/registered"), "the forge sends the browser back here: {page}");
    let again = rig.fleet.get_sign_in_app_form(state.clone()).await;
    assert!(again.is_ok(), "reading the page leaves the state live");

    let settings_page = rig.fleet.sign_in_app_registered(APP_CODE.to_string(), state.clone()).await.expect("registered");
    assert_eq!(settings_page, APP_SETTINGS_PAGE, "on to where the device flow is turned on");

    let (code, _) = refused(rig.fleet.sign_in_app_registered(APP_CODE.to_string(), state.clone()).await);
    assert_eq!(code, "fleet.sign_in_app_state_unknown");
    let (code, _) = refused(rig.fleet.get_sign_in_app_form(state).await);
    assert_eq!(code, "fleet.sign_in_app_state_unknown");
    assert_eq!(rig.forge.converted(), [APP_CODE], "the second return asked the forge nothing");
}

#[tokio::test]
async fn a_state_never_issued_or_past_its_hour_is_refused_and_converts_nothing() {
    let rig = rig(FakeSignIn::new());
    let (code, said) = refused(rig.fleet.sign_in_app_registered(APP_CODE.to_string(), "forged".to_string()).await);
    assert_eq!(code, "fleet.sign_in_app_state_unknown");
    assert!(said.contains("Press Register"), "{said}");
    assert!(rig.fleet.get_sign_in_app_form("forged".to_string()).await.is_err());

    let state = pressed(&rig).await;
    rig.clock.on(60 * 60);
    let (code, _) = refused(rig.fleet.get_sign_in_app_form(state.clone()).await);
    assert_eq!(code, "fleet.sign_in_app_state_unknown");
    let (code, _) = refused(rig.fleet.sign_in_app_registered(APP_CODE.to_string(), state).await);
    assert_eq!(code, "fleet.sign_in_app_state_unknown");

    assert!(rig.forge.converted().is_empty(), "the forge was asked nothing");
    assert_eq!(client_id(&rig).await, ipc::SettingValue::Text(String::new()));
    assert!(!settings_file_in(&rig.home).exists(), "nothing was written");
}

#[tokio::test]
async fn a_conversion_writes_the_client_id_and_nothing_else_into_settings() {
    let rig = rig(FakeSignIn::new());
    let state = pressed(&rig).await;
    rig.fleet.sign_in_app_registered(APP_CODE.to_string(), state).await.expect("registered");

    assert_eq!(client_id(&rig).await, ipc::SettingValue::Text(APP_CLIENT_ID.to_string()));
    let written = std::fs::read_to_string(settings_file_in(&rig.home)).expect("settings.json");
    let compact: String = written.chars().filter(|c| !c.is_whitespace()).collect();
    assert_eq!(compact, format!("{{\"{KEY}\":\"{APP_CLIENT_ID}\"}}"), "the one key: {written}");
}

#[tokio::test]
async fn a_forge_that_will_not_convert_leaves_the_sign_in_app_blank_and_says_why() {
    let rig = rig(FakeSignIn::new().refusing_to_convert("The code has expired."));
    let state = pressed(&rig).await;
    let (code, said) = refused(rig.fleet.sign_in_app_registered(APP_CODE.to_string(), state.clone()).await);
    assert_eq!(code, "fleet.sign_in_app_not_registered");
    assert!(said.contains("would not register the app") && said.contains("The code has expired."), "{said}");
    assert_eq!(client_id(&rig).await, ipc::SettingValue::Text(String::new()));
    assert!(!settings_file_in(&rig.home).exists(), "nothing was written");

    let (code, _) = refused(rig.fleet.sign_in_app_registered(APP_CODE.to_string(), state).await);
    assert_eq!(code, "fleet.sign_in_app_state_unknown", "a failed return used the state up too");
}

#[tokio::test]
async fn a_forge_out_of_reach_keeps_nothing_and_says_it_could_not_be_reached() {
    let rig = rig(FakeSignIn::new());
    rig.forge.set_unreachable(true);
    let state = pressed(&rig).await;
    let (code, said) = refused(rig.fleet.sign_in_app_registered(APP_CODE.to_string(), state).await);
    assert_eq!(code, "fleet.sign_in_app_not_registered");
    assert!(said.contains("could not be reached"), "{said}");
    assert_eq!(client_id(&rig).await, ipc::SettingValue::Text(String::new()));
}
