//! The device flow's client: each request as it is built, each reply read as the
//! forge sends it, and the three calls end to end against a socket on this
//! machine standing in for the forge.

use std::net::TcpListener;
use std::time::Duration;

use adapter_traits::{Polled, Secret, SignIn, SignInError};

use super::store::serve;
use crate::sign_in::{self, ForgeSignIn, Reply};

fn reply(status: u16, body: &str) -> Reply {
    Reply { status, body: body.as_bytes().to_vec() }
}

const DEVICE: &str = r#"{"device_code":"dc-secret","user_code":"WDJB-MJHT","verification_uri":"https://forge.test/login/device","expires_in":900,"interval":5}"#;

#[test]
fn beginning_sends_the_client_id_and_the_one_read_only_organisation_scope() {
    assert_eq!(
        sign_in::begin_form("Iv1.abc"),
        [("client_id", "Iv1.abc".to_string()), ("scope", "read:org".to_string())]
    );
}

#[test]
fn polling_sends_the_device_grant_with_the_code_and_still_no_client_secret() {
    let form = sign_in::poll_form("Iv1.abc", &Secret::new("dc-secret".to_string()));
    assert_eq!(
        form,
        [
            ("client_id", "Iv1.abc".to_string()),
            ("device_code", "dc-secret".to_string()),
            ("grant_type", "urn:ietf:params:oauth:grant-type:device_code".to_string()),
        ]
    );
}

#[test]
fn a_device_code_reply_is_read_into_the_card_and_the_secret() {
    let code = sign_in::device_code_of(&reply(200, DEVICE)).unwrap();
    assert_eq!(code.user_code, "WDJB-MJHT");
    assert_eq!(code.verification_uri, "https://forge.test/login/device");
    assert_eq!((code.interval_seconds, code.expires_in_seconds), (5, 900));
    assert_eq!(code.device_code.expose(), "dc-secret");
}

#[test]
fn a_reply_with_no_interval_or_expiry_gets_the_forges_defaults_and_a_zero_interval_is_one() {
    let bare = r#"{"device_code":"d","user_code":"U","verification_uri":"https://forge.test/d"}"#;
    let code = sign_in::device_code_of(&reply(200, bare)).unwrap();
    assert_eq!((code.interval_seconds, code.expires_in_seconds), (5, 900));
    let zero = r#"{"device_code":"d","user_code":"U","verification_uri":"https://forge.test/d","interval":0}"#;
    assert_eq!(sign_in::device_code_of(&reply(200, zero)).unwrap().interval_seconds, 1);
}

#[test]
fn a_page_to_sign_in_at_that_is_not_https_is_refused_because_bridge_opens_it() {
    let plain = DEVICE.replace("https://", "http://");
    assert!(matches!(sign_in::device_code_of(&reply(200, &plain)), Err(SignInError::Refused(_))));
    let script = DEVICE.replace("https://forge.test/login/device", "javascript:alert(1)");
    assert!(matches!(sign_in::device_code_of(&reply(200, &script)), Err(SignInError::Refused(_))));
}

#[test]
fn an_error_the_forge_sends_with_a_200_is_a_refusal_in_its_own_words() {
    let disabled = r#"{"error":"device_flow_disabled","error_description":"Device Flow must be explicitly enabled for this App"}"#;
    let said = sign_in::device_code_of(&reply(200, disabled)).err().unwrap();
    assert_eq!(said, SignInError::Refused("Device Flow must be explicitly enabled for this App".to_string()));
}

#[test]
fn a_forge_that_errors_is_unreachable_and_one_that_says_no_with_a_status_is_refused() {
    assert!(sign_in::device_code_of(&reply(503, "")).err().unwrap().is_unreachable());
    assert!(!sign_in::device_code_of(&reply(404, "")).err().unwrap().is_unreachable());
}

#[test]
fn each_poll_answer_the_forge_gives_is_one_of_the_five() {
    let said = |error: &str| reply(200, &format!(r#"{{"error":"{error}"}}"#));
    assert!(matches!(sign_in::polled_of(&said("authorization_pending")), Ok(Polled::Pending)));
    assert!(matches!(sign_in::polled_of(&said("slow_down")), Ok(Polled::SlowDown)));
    assert!(matches!(sign_in::polled_of(&said("expired_token")), Ok(Polled::Expired)));
    assert!(matches!(sign_in::polled_of(&said("access_denied")), Ok(Polled::Denied)));
    let granted = reply(200, r#"{"access_token":"tok-secret","token_type":"bearer","scope":""}"#);
    let Ok(Polled::Granted(token)) = sign_in::polled_of(&granted) else { panic!("not granted") };
    assert_eq!(token.expose(), "tok-secret");
}

#[test]
fn a_poll_error_the_flow_has_no_word_for_is_a_refusal_not_a_wait() {
    let odd = reply(200, r#"{"error":"incorrect_device_code","error_description":"The device_code provided is not valid."}"#);
    assert_eq!(
        sign_in::polled_of(&odd).err().unwrap(),
        SignInError::Refused("The device_code provided is not valid.".to_string())
    );
    assert!(sign_in::polled_of(&reply(200, "{}")).is_err(), "no token and no error is not a grant");
    assert!(sign_in::polled_of(&reply(502, "")).err().unwrap().is_unreachable());
}

#[test]
fn the_account_is_the_numeric_id_and_the_login() {
    let user = reply(200, r#"{"login":"octocat","id":583231,"node_id":"x","type":"User"}"#);
    let account = sign_in::account_of(&user).unwrap();
    assert_eq!((account.id, account.login.as_str()), (583_231, "octocat"));
    assert!(sign_in::account_of(&reply(200, r#"{"login":"octocat","id":"583231"}"#)).is_err(), "an id that is text is not an id");
    assert!(matches!(sign_in::account_of(&reply(401, r#"{"message":"Bad credentials"}"#)), Err(SignInError::Refused(_))));
}

#[test]
fn a_refusal_never_repeats_a_body_that_could_hold_the_token() {
    let said = sign_in::polled_of(&reply(200, r#"{"access_token":""}"#)).err().unwrap().said();
    assert!(!said.contains("access_token"), "{said}");
    let said = sign_in::account_of(&reply(401, r#"{"message":"tok-secret"}"#)).err().unwrap().said();
    assert!(!said.contains("tok-secret"), "{said}");
}

#[test]
fn the_organisations_are_their_logins_in_lowercase_and_a_refused_listing_is_a_refusal() {
    let listed = reply(200, r#"[{"login":"Mele-Labs","id":1},{"login":"acme","id":2}]"#);
    assert_eq!(sign_in::organisations_of(&listed).unwrap(), ["mele-labs", "acme"]);
    assert_eq!(sign_in::organisations_of(&reply(200, "[]")).unwrap(), Vec::<String>::new());
    assert!(matches!(sign_in::organisations_of(&reply(403, r#"{"message":"tok-secret"}"#)), Err(SignInError::Refused(_))));
    assert!(!sign_in::organisations_of(&reply(403, r#"{"message":"tok-secret"}"#)).err().unwrap().said().contains("tok-secret"));
    assert!(sign_in::organisations_of(&reply(502, "")).err().unwrap().is_unreachable());
}

#[test]
fn organisations_are_read_a_page_at_a_time_until_a_short_page_with_the_token_on_each() {
    let full: &'static str = Box::leak(
        format!("[{}]", (0..100).map(|n| format!(r#"{{"login":"org-{n}"}}"#)).collect::<Vec<_>>().join(",")).into_boxed_str(),
    );
    let (url, server) = serve(vec![(200, full), (200, r#"[{"login":"Last"}]"#)]);
    let listed = forge_at(&url).organisations(&Secret::new("tok-secret".to_string())).unwrap();
    assert_eq!((listed.len(), listed.last().map(String::as_str)), (101, Some("last")));
    let seen = server.join().unwrap();
    let targets: Vec<_> = seen.iter().map(|s| s.target.as_str()).collect();
    assert_eq!(targets, ["/user/orgs?per_page=100&page=1", "/user/orgs?per_page=100&page=2"]);
    assert!(seen.iter().all(|s| s.header("authorization") == Some("Bearer tok-secret")));
}

fn forge_at(url: &str) -> ForgeSignIn {
    ForgeSignIn::against(
        &format!("{url}/login/device/code"),
        &format!("{url}/login/oauth/access_token"),
        &format!("{url}/user"),
        Duration::from_secs(5),
    )
}

#[test]
fn the_three_calls_go_to_their_endpoints_asking_for_json_and_the_token_rides_only_the_last() {
    let (url, server) = serve(vec![
        (200, DEVICE),
        (200, r#"{"error":"authorization_pending"}"#),
        (200, r#"{"access_token":"tok-secret","token_type":"bearer"}"#),
        (200, r#"{"login":"octocat","id":583231}"#),
    ]);
    let forge = forge_at(&url);
    let code = forge.begin("Iv1.abc").unwrap();
    assert!(matches!(forge.poll("Iv1.abc", &code.device_code), Ok(Polled::Pending)));
    let Ok(Polled::Granted(token)) = forge.poll("Iv1.abc", &code.device_code) else { panic!("not granted") };
    let account = forge.who(&token).unwrap();
    assert_eq!(account.id, 583_231);

    let seen = server.join().unwrap();
    let paths: Vec<_> = seen.iter().map(|s| (s.method.as_str(), s.target.as_str())).collect();
    assert_eq!(
        paths,
        [
            ("POST", "/login/device/code"),
            ("POST", "/login/oauth/access_token"),
            ("POST", "/login/oauth/access_token"),
            ("GET", "/user")
        ]
    );
    for one in &seen {
        assert_eq!(one.header("accept"), Some("application/json"));
        assert!(one.header("user-agent").is_some(), "the forge's API refuses a request with none");
    }
    assert_eq!(String::from_utf8(seen[0].body.clone()).unwrap(), "client_id=Iv1.abc&scope=read%3Aorg");
    let polled = String::from_utf8(seen[1].body.clone()).unwrap();
    assert!(polled.contains("device_code=dc-secret") && polled.contains("grant_type=urn%3Aietf%3Aparams%3Aoauth%3Agrant-type%3Adevice_code"), "{polled}");
    assert_eq!(seen[3].header("authorization"), Some("Bearer tok-secret"));
    assert!(seen[..3].iter().all(|s| s.header("authorization").is_none()), "no token before there is one");
}

#[test]
fn a_forge_nothing_listens_for_is_unreachable() {
    let closed = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", closed.local_addr().unwrap());
    drop(closed);
    let said = forge_at(&url).begin("Iv1.abc").err().unwrap();
    assert!(said.is_unreachable(), "{said:?}");
}
