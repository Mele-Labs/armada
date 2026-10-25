//! What the account's wire must keep true for Bridge's hand-written mirror: a
//! flow is tagged on `phase`, a row's state is a lowercase word, and an end that
//! has nothing to say leaves no key.

use crate::{
    decode, encode, AccountFlow, Accounts, BoundHow, BoundRepository, Profile, ProfileHome, SetRepositoryProfile, SetUpState,
    SetUpSteps, SignInEnded, SignInOutcome, SignedOut, UnmatchedRepository,
};

fn octocat() -> Profile {
    Profile {
        account_id: 583_231,
        login: "octocat".to_string(),
        orgs: vec!["acme".to_string()],
        chosen: false,
        home: ProfileHome { account: "acme".to_string(), repository: "kit".to_string(), made: false },
        repositories: Vec::new(),
    }
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
    let accounts = Accounts { profiles: vec![octocat()], unmatched: Vec::new(), flow: AccountFlow::Idle { ended: None } };
    assert_eq!(
        encode(&accounts).unwrap(),
        r#"{"profiles":[{"account_id":583231,"login":"octocat","orgs":["acme"],"chosen":false,"home":{"account":"acme","repository":"kit","made":false},"repositories":[]}],"unmatched":[],"flow":{"phase":"idle"}}"#
    );
}

#[test]
fn a_repository_is_bound_to_a_profile_or_unmatched_with_the_profiles_that_fit_it() {
    let mut profile = octocat();
    profile.repositories.push(BoundRepository { root: "/r/armada".to_string(), name: "armada".to_string(), how: BoundHow::Matched });
    profile.repositories.push(BoundRepository { root: "/r/site".to_string(), name: "site".to_string(), how: BoundHow::Chosen });
    let accounts = Accounts {
        profiles: vec![profile],
        unmatched: vec![UnmatchedRepository {
            root: "/r/shared".to_string(),
            name: "shared".to_string(),
            candidates: vec![583_231, 9_919],
            none_chosen: false,
        }],
        flow: AccountFlow::Idle { ended: None },
    };
    let json = encode(&accounts).unwrap();
    assert!(json.contains(r#""repositories":[{"root":"/r/armada","name":"armada","how":"matched"},{"root":"/r/site","name":"site","how":"chosen"}]"#), "{json}");
    assert!(json.contains(r#""unmatched":[{"root":"/r/shared","name":"shared","candidates":[583231,9919],"none_chosen":false}]"#), "{json}");
    assert_eq!(decode::<Accounts>("accounts", json.as_bytes()).unwrap(), accounts);
}

#[test]
fn choosing_none_is_a_null_account_and_not_an_absent_one() {
    let none = SetRepositoryProfile { root: "/r/armada".to_string(), account_id: None };
    assert_eq!(encode(&none).unwrap(), r#"{"root":"/r/armada","account_id":null}"#);
    let one = decode::<SetRepositoryProfile>("a choice", br#"{"root":"/r/armada","account_id":9919}"#).unwrap();
    assert_eq!(one.account_id, Some(9_919));
    assert!(decode::<SetRepositoryProfile>("a choice", br#"{"root":"/r/armada"}"#).is_err(), "a forgotten key is not a choice of none");
}

#[test]
fn a_sign_out_says_whether_the_store_row_remains_and_why_only_when_it_does() {
    let accounts = Accounts { profiles: Vec::new(), unmatched: Vec::new(), flow: AccountFlow::Idle { ended: None } };
    let clean = SignedOut { accounts: accounts.clone(), store_row_remains: false, said: None };
    assert_eq!(
        encode(&clean).unwrap(),
        r#"{"accounts":{"profiles":[],"unmatched":[],"flow":{"phase":"idle"}},"store_row_remains":false}"#
    );
    let left = SignedOut { accounts, store_row_remains: true, said: Some("The store could not be reached.".to_string()) };
    assert!(encode(&left).unwrap().ends_with(r#""store_row_remains":true,"said":"The store could not be reached."}"#));
}
