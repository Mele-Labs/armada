//! What the account's wire must keep true for Bridge's hand-written mirror: a
//! flow is tagged on `phase`, a row's state is a lowercase word, and an end that
//! has nothing to say leaves no key.

use crate::{
    decode, encode, AccountFlow, Accounts, Profile, SetUpState, SetUpSteps, SignInEnded, SignInOutcome, SignedOut,
};

fn octocat(active: bool) -> Profile {
    Profile { account_id: 583_231, login: "octocat".to_string(), active, chosen: false }
}

#[test]
fn a_flow_is_tagged_on_its_phase_and_each_phase_carries_what_the_card_draws() {
    let code = AccountFlow::Code {
        code: "WDJB-MJHT".to_string(),
        address: "https://forge.test/login/device".to_string(),
        expires_in_seconds: 899,
    };
    assert_eq!(
        encode(&code).unwrap(),
        r#"{"phase":"code","code":"WDJB-MJHT","address":"https://forge.test/login/device","expires_in_seconds":899}"#
    );
    let setup = AccountFlow::Setup {
        login: "octocat".to_string(),
        repository: String::new(),
        steps: SetUpSteps { identity: SetUpState::Done, store: SetUpState::Working, home: SetUpState::Waiting },
    };
    assert_eq!(
        encode(&setup).unwrap(),
        r#"{"phase":"setup","login":"octocat","repository":"","steps":{"identity":"done","store":"working","home":"waiting"}}"#
    );
    for flow in [code, setup, AccountFlow::Idle { ended: None }] {
        let json = encode(&flow).unwrap();
        assert_eq!(decode::<AccountFlow>("a flow", json.as_bytes()).unwrap(), flow);
    }
}

#[test]
fn an_idle_flow_names_how_the_last_one_ended_only_when_one_has() {
    assert_eq!(encode(&AccountFlow::Idle { ended: None }).unwrap(), r#"{"phase":"idle"}"#);
    let ended = AccountFlow::Idle {
        ended: Some(SignInEnded { outcome: SignInOutcome::Denied, said: Some("Turned down at the forge.".to_string()) }),
    };
    assert_eq!(
        encode(&ended).unwrap(),
        r#"{"phase":"idle","ended":{"outcome":"denied","said":"Turned down at the forge."}}"#
    );
    let signed_in = AccountFlow::Idle { ended: Some(SignInEnded { outcome: SignInOutcome::SignedIn, said: None }) };
    assert_eq!(encode(&signed_in).unwrap(), r#"{"phase":"idle","ended":{"outcome":"signed_in"}}"#);
}

#[test]
fn the_accounts_list_carries_the_numeric_id_beside_the_login() {
    let accounts = Accounts { profiles: vec![octocat(true)], flow: AccountFlow::Idle { ended: None } };
    assert_eq!(
        encode(&accounts).unwrap(),
        r#"{"profiles":[{"account_id":583231,"login":"octocat","active":true,"chosen":false}],"flow":{"phase":"idle"}}"#
    );
}

#[test]
fn a_sign_out_says_whether_the_store_row_remains_and_why_only_when_it_does() {
    let accounts = Accounts { profiles: Vec::new(), flow: AccountFlow::Idle { ended: None } };
    let clean = SignedOut { accounts: accounts.clone(), store_row_remains: false, said: None };
    assert_eq!(
        encode(&clean).unwrap(),
        r#"{"accounts":{"profiles":[],"flow":{"phase":"idle"}},"store_row_remains":false}"#
    );
    let left = SignedOut { accounts, store_row_remains: true, said: Some("The store could not be reached.".to_string()) };
    assert!(encode(&left).unwrap().ends_with(r#""store_row_remains":true,"said":"The store could not be reached."}"#));
}
