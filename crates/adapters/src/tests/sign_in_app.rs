//! Registering the sign-in app: the manifest the form carries, a conversion read
//! with its secrets left behind, and the call end to end against a socket on this
//! machine standing in for GitHub.

use std::time::Duration;

use adapter_traits::{SignInApp, SignInError};

use super::store::serve;
use crate::sign_in::{ForgeSignIn, Reply};
use crate::sign_in_app::{self, AppUrls};

const CONVERTED: &str = r#"{"id":42,"slug":"armada-sign-in-1a2b3c","client_id":"Iv23.app","client_secret":"cs-secret","pem":"pem-secret","webhook_secret":"wh-secret","html_url":"https://github.com/apps/armada-sign-in-1a2b3c"}"#;

fn reply(status: u16, body: &str) -> Reply {
    Reply { status, body: body.as_bytes().to_vec() }
}

fn forge_at(url: &str) -> ForgeSignIn {
    ForgeSignIn::new(Duration::from_secs(5)).registering_at(AppUrls {
        new_app: format!("{url}/settings/apps/new"),
        conversions: format!("{url}/app-manifests"),
        apps: format!("{url}/settings/apps"),
    })
}

#[test]
fn the_form_posts_a_private_manifest_with_no_permission_event_or_webhook_and_the_state_on_its_address() {
    let form = ForgeSignIn::new(Duration::from_secs(5)).form("Armada sign-in 1a2b3c", "http://127.0.0.1:7777/back", "f00d");
    assert_eq!(form.action, "https://github.com/settings/apps/new?state=f00d");
    assert_eq!(form.field, "manifest");
    assert_eq!(
        form.manifest,
        r#"{"name":"Armada sign-in 1a2b3c","url":"https://github.com","redirect_url":"http://127.0.0.1:7777/back","public":false,"hook_attributes":{"url":"https://github.com","active":false},"default_permissions":{},"default_events":[]}"#
    );
}

#[test]
fn a_conversion_keeps_the_client_id_and_the_settings_page_and_none_of_the_three_secrets() {
    let app = sign_in_app::registered_of(&reply(201, CONVERTED), "https://github.com/settings/apps").unwrap();
    assert_eq!(app.client_id, "Iv23.app");
    assert_eq!(app.settings_page, "https://github.com/settings/apps/armada-sign-in-1a2b3c");
    let everything = format!("{app:?}");
    for secret in ["cs-secret", "pem-secret", "wh-secret"] {
        assert!(!everything.contains(secret), "{secret} in {everything}");
    }
}

#[test]
fn a_refused_conversion_says_githubs_message_and_never_the_body() {
    let gone = sign_in_app::registered_of(&reply(404, r#"{"message":"Not Found","documentation_url":"x"}"#), "a").err().unwrap();
    assert_eq!(gone, SignInError::Refused("Not Found".to_string()));
    assert!(sign_in_app::registered_of(&reply(502, ""), "a").err().unwrap().is_unreachable());
    let unread = sign_in_app::registered_of(&reply(201, r#"{"client_secret":"cs-secret"}"#), "a").err().unwrap();
    assert!(!unread.said().contains("cs-secret"), "{unread:?}");
}

#[test]
fn a_code_that_is_not_githubs_shape_is_refused_before_anything_is_sent() {
    let forge = forge_at("http://127.0.0.1:9");
    for code in ["", "../user", "a b", "x?y=1"] {
        assert!(matches!(forge.convert(code), Err(SignInError::Refused(_))), "{code}");
    }
}

#[test]
fn a_conversion_is_one_post_with_no_credential_to_the_codes_own_address() {
    let (url, server) = serve(vec![(201, CONVERTED)]);
    let app = forge_at(&url).convert("abc123").unwrap();
    assert_eq!(app.settings_page, format!("{url}/settings/apps/armada-sign-in-1a2b3c"));
    let seen = server.join().unwrap();
    assert_eq!(seen.len(), 1);
    assert_eq!((seen[0].method.as_str(), seen[0].target.as_str()), ("POST", "/app-manifests/abc123/conversions"));
    assert!(seen[0].header("authorization").is_none());
    assert!(seen[0].header("user-agent").is_some(), "the forge's API refuses a request with none");
}
