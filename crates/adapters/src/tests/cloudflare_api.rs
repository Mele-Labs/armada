//! Cloudflare's API: each request as it is built, each answer as it is read, and the calls themselves
//! against a socket on this machine.

use std::net::TcpListener;
use std::time::Duration;

use adapter_traits::{Cloudflare, CloudflareError, Secret, WorkerUpload};

use super::store::serve;
use crate::cloudflare_api::{self, HttpCloudflare, Reply};

const TOKEN: &str = "cf-token-Zq81xY";
const ACCOUNT: &str = "0123456789abcdef0123456789abcdef";

fn token() -> Secret<String> {
    Secret::new(TOKEN.to_string())
}

fn upload(team: Option<&str>, enrol_limit: bool) -> WorkerUpload {
    WorkerUpload {
        name: "armada-store".to_string(),
        script: "export default { fetch() { return new Response('ok') } }".to_string(),
        team: team.map(str::to_string),
        enrol_limit,
    }
}

fn reply(status: u16, body: &str) -> Reply {
    Reply { status, body: body.as_bytes().to_vec() }
}

fn refused_with(code: u64, message: &str) -> String {
    format!(r#"{{"success":false,"errors":[{{"code":{code},"message":"{message}"}}],"messages":[],"result":null}}"#)
}

fn cloudflare_at(url: &str) -> HttpCloudflare {
    HttpCloudflare::at(url, Duration::from_secs(5)).expect("a client")
}

// Fleet makes this inside its async main, where `reqwest`'s blocking client may not be built or dropped.
#[tokio::test]
async fn a_client_is_made_and_dropped_inside_an_async_context() {
    let made = HttpCloudflare::new(Duration::from_secs(5));
    assert!(made.is_ok());
    drop(made);
}

const ACCOUNT_BINDING: &str = r#"{"type":"durable_object_namespace","name":"ACCOUNT","class_name":"Account"}"#;
const LIMIT_BINDING: &str = r#"{"type":"ratelimit","name":"ENROL_LIMIT","namespace_id":"1001","simple":{"limit":10,"period":60}}"#;
const MIGRATION: &str = r#""migrations":{"new_tag":"v1","new_sqlite_classes":["Account"]}"#;

#[test]
fn a_first_upload_binds_the_account_and_the_limit_and_runs_the_migration() {
    let metadata = cloudflare_api::metadata(&upload(None, true), true).unwrap();
    assert_eq!(
        metadata,
        format!(r#"{{"main_module":"worker.js","compatibility_date":"2026-10-01","bindings":[{ACCOUNT_BINDING},{LIMIT_BINDING}],{MIGRATION}}}"#)
    );
}

#[test]
fn a_team_upload_sets_team_as_plain_text() {
    let metadata = cloudflare_api::metadata(&upload(Some("acme/platform"), true), true).unwrap();
    assert_eq!(
        metadata,
        format!(
            r#"{{"main_module":"worker.js","compatibility_date":"2026-10-01","bindings":[{ACCOUNT_BINDING},{LIMIT_BINDING},{{"type":"plain_text","name":"TEAM","text":"acme/platform"}}],{MIGRATION}}}"#
        )
    );
}

#[test]
fn a_redeploy_of_a_migrated_worker_runs_no_migration() {
    let listed = r#"{"success":true,"errors":[],"result":[{"id":"other","migration_tag":""},{"id":"armada-store","migration_tag":"v1"}]}"#;
    assert!(!cloudflare_api::migration_due(&reply(200, listed), "armada-store").unwrap());
    let metadata = cloudflare_api::metadata(&upload(None, true), false).unwrap();
    assert_eq!(metadata, format!(r#"{{"main_module":"worker.js","compatibility_date":"2026-10-01","bindings":[{ACCOUNT_BINDING},{LIMIT_BINDING}]}}"#));
}

#[test]
fn a_worker_absent_or_never_migrated_is_migrated() {
    let listed = r#"{"success":true,"errors":[],"result":[{"id":"other","migration_tag":"v1"},{"id":"unmigrated"},{"id":"empty","migration_tag":""}]}"#;
    for name in ["armada-store", "unmigrated", "empty"] {
        assert!(cloudflare_api::migration_due(&reply(200, listed), name).unwrap(), "{name}");
    }
    assert!(!cloudflare_api::migration_due(&reply(200, listed), "other").unwrap());
}

#[test]
fn an_upload_without_the_limit_binds_no_enrol_limit() {
    let metadata = cloudflare_api::metadata(&upload(None, false), true).unwrap();
    assert_eq!(metadata, format!(r#"{{"main_module":"worker.js","compatibility_date":"2026-10-01","bindings":[{ACCOUNT_BINDING}],{MIGRATION}}}"#));
}

#[test]
fn the_upload_is_multipart_with_the_metadata_part_and_the_module_part() {
    let mut worker = upload(None, true);
    // A script holding the first boundary the call would pick must not end its own part.
    worker.script = "// armada-worker-0\nexport default {}".to_string();
    let call = cloudflare_api::upload_call(ACCOUNT, &worker, true).unwrap();
    assert_eq!((call.method, call.target.as_str()), ("PUT", "/accounts/0123456789abcdef0123456789abcdef/workers/scripts/armada-store"));
    let content_type = call.content_type.unwrap();
    let boundary = content_type.strip_prefix("multipart/form-data; boundary=").expect("multipart");
    assert!(!worker.script.contains(boundary), "{boundary}");
    let metadata = cloudflare_api::metadata(&worker, true).unwrap();
    let body = String::from_utf8(call.body).unwrap();
    assert_eq!(
        body,
        format!(
            "--{boundary}\r\nContent-Disposition: form-data; name=\"metadata\"\r\nContent-Type: application/json\r\n\r\n{metadata}\r\n\
             --{boundary}\r\nContent-Disposition: form-data; name=\"worker.js\"; filename=\"worker.js\"\r\nContent-Type: application/javascript+module\r\n\r\n{}\r\n\
             --{boundary}--\r\n",
            worker.script
        )
    );
}

#[test]
fn a_401_or_a_403_sign_in_error_is_refused_in_cloudflares_words() {
    for (status, code) in [(401, 1000), (403, 10000), (403, 9109)] {
        let said = cloudflare_api::refusal(&reply(status, &refused_with(code, "Authentication error")), false);
        assert_eq!(said, CloudflareError::Refused("Authentication error".to_string()), "{status} {code}");
    }
    assert_eq!(cloudflare_api::refusal(&reply(401, ""), false), CloudflareError::Refused("status 401".to_string()));
}

#[test]
fn a_403_that_is_not_a_sign_in_error_failed() {
    let said = cloudflare_api::refusal(&reply(403, &refused_with(10021, "Script validation failed")), false);
    assert_eq!(said, CloudflareError::Failed("status 403: Script validation failed".to_string()));
}

#[test]
fn a_refused_limit_is_limit_refused_only_when_the_limit_was_sent() {
    for words in ["Unknown binding type RateLimit", "rate limit bindings are not enabled", "invalid rate_limit namespace"] {
        let refused = reply(400, &refused_with(10021, words));
        assert_eq!(cloudflare_api::uploaded(&refused, &upload(None, true)), Err(CloudflareError::LimitRefused(words.to_string())));
        assert_eq!(cloudflare_api::uploaded(&refused, &upload(None, false)), Err(CloudflareError::Failed(format!("status 400: {words}"))));
    }
    let other = reply(400, &refused_with(10021, "Uncaught SyntaxError"));
    assert!(matches!(cloudflare_api::uploaded(&other, &upload(None, true)), Err(CloudflareError::Failed(_))));
}

#[test]
fn another_refusal_failed_with_its_status_and_at_most_200_characters() {
    let long = "x".repeat(400);
    let CloudflareError::Failed(said) = cloudflare_api::refusal(&reply(500, &refused_with(10013, &long)), false) else {
        panic!("not Failed");
    };
    assert!(said.starts_with("status 500: xxx"), "{said}");
    assert_eq!(said.chars().count(), 200);
    // A 200 that says it did not succeed is not a success.
    assert!(matches!(cloudflare_api::uploaded(&reply(200, &refused_with(10013, "no")), &upload(None, false)), Err(CloudflareError::Failed(_))));
    // Words that are not the envelope are kept as they came.
    assert_eq!(cloudflare_api::refusal(&reply(502, "Bad gateway"), false), CloudflareError::Failed("status 502: Bad gateway".to_string()));
}

#[test]
fn a_closed_port_is_unreachable() {
    let closed = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", closed.local_addr().unwrap());
    drop(closed);
    assert!(matches!(cloudflare_at(&url).accounts(&token()), Err(CloudflareError::Unreachable(_))));
}

#[test]
fn the_accounts_are_the_ids_in_the_result_and_every_call_carries_the_token() {
    let (url, server) = serve(vec![(200, r#"{"success":true,"errors":[],"result":[{"id":"aaa111","name":"Mine"},{"id":"bbb222","name":"Work"}]}"#)]);
    assert_eq!(cloudflare_at(&url).accounts(&token()).unwrap(), ["aaa111", "bbb222"]);
    let seen = &server.join().unwrap()[0];
    assert_eq!((seen.method.as_str(), seen.target.as_str()), ("GET", "/accounts?per_page=50"));
    assert_eq!(seen.header("authorization"), Some(format!("Bearer {TOKEN}").as_str()));
    assert_eq!(seen.header("user-agent"), Some("armada"));
}

#[test]
fn an_upload_reads_the_workers_then_puts_the_multipart_without_a_migration_on_a_redeploy() {
    let listed = r#"{"success":true,"errors":[],"result":[{"id":"armada-store","migration_tag":"v1"}]}"#;
    let (url, server) = serve(vec![(200, listed), (200, r#"{"success":true,"errors":[],"result":{"id":"armada-store"}}"#)]);
    cloudflare_at(&url).upload_worker(&token(), ACCOUNT, &upload(None, true)).unwrap();
    let seen = server.join().unwrap();
    assert_eq!((seen[0].method.as_str(), seen[0].target.as_str()), ("GET", "/accounts/0123456789abcdef0123456789abcdef/workers/scripts"));
    assert_eq!((seen[1].method.as_str(), seen[1].target.as_str()), ("PUT", "/accounts/0123456789abcdef0123456789abcdef/workers/scripts/armada-store"));
    assert!(seen[1].header("content-type").is_some_and(|value| value.starts_with("multipart/form-data; boundary=")));
    let body = String::from_utf8(seen[1].body.clone()).unwrap();
    assert!(body.contains("ENROL_LIMIT") && !body.contains("migrations"), "{body}");
    assert!(seen.iter().all(|one| one.header("authorization") == Some(format!("Bearer {TOKEN}").as_str())));
}

#[test]
fn an_account_or_name_that_does_not_fit_a_path_is_refused_before_cloudflare_is_asked() {
    let closed = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", closed.local_addr().unwrap());
    drop(closed);
    let cloudflare = cloudflare_at(&url);
    for account in ["", "../user", "a/b"] {
        assert!(matches!(cloudflare.upload_worker(&token(), account, &upload(None, false)), Err(CloudflareError::Failed(_))), "{account:?}");
    }
    let mut bad = upload(None, false);
    bad.name = "Store/../x".to_string();
    assert!(matches!(cloudflare.upload_worker(&token(), ACCOUNT, &bad), Err(CloudflareError::Failed(_))));
    assert!(matches!(cloudflare.workers_dev(&token(), ACCOUNT, "a b"), Err(CloudflareError::Failed(_))));
}

#[test]
fn workers_dev_turns_the_address_on_and_answers_it() {
    let (url, server) = serve(vec![
        (200, r#"{"success":true,"errors":[],"result":{"subdomain":"nick"}}"#),
        (200, r#"{"success":true,"errors":[],"result":{"enabled":true,"previews_enabled":true}}"#),
    ]);
    let address = cloudflare_at(&url).workers_dev(&token(), ACCOUNT, "armada-store").unwrap();
    assert_eq!(address, "https://armada-store.nick.workers.dev");
    let seen = server.join().unwrap();
    assert_eq!((seen[0].method.as_str(), seen[0].target.as_str()), ("GET", "/accounts/0123456789abcdef0123456789abcdef/workers/subdomain"));
    assert_eq!((seen[1].method.as_str(), seen[1].target.as_str()), ("POST", "/accounts/0123456789abcdef0123456789abcdef/workers/scripts/armada-store/subdomain"));
    assert_eq!(seen[1].body, br#"{"enabled":true}"#);
}

#[test]
fn an_account_without_a_subdomain_is_no_workers_dev() {
    assert_eq!(cloudflare_api::subdomain_read(&reply(404, "")), Err(CloudflareError::NoWorkersDev));
    assert_eq!(cloudflare_api::subdomain_read(&reply(400, &refused_with(10007, "workers.dev subdomain not found"))), Err(CloudflareError::NoWorkersDev));
    assert_eq!(cloudflare_api::subdomain_read(&reply(200, r#"{"success":true,"errors":[],"result":{"subdomain":""}}"#)), Err(CloudflareError::NoWorkersDev));
    assert!(matches!(cloudflare_api::subdomain_read(&reply(401, "")), Err(CloudflareError::Refused(_))));

    // Over the socket, the address is never turned on.
    let not_found: &'static str = Box::leak(refused_with(10007, "not found").into_boxed_str());
    let (url, server) = serve(vec![(404, not_found)]);
    assert_eq!(cloudflare_at(&url).workers_dev(&token(), ACCOUNT, "armada-store"), Err(CloudflareError::NoWorkersDev));
    assert_eq!(server.join().unwrap().len(), 1);
}

#[test]
fn the_token_never_appears_in_an_errors_text() {
    let echoed: &'static str = Box::leak(refused_with(1000, &format!("Invalid token {TOKEN}")).into_boxed_str());
    let rate: &'static str = Box::leak(refused_with(10021, &format!("ratelimit refused for {TOKEN}")).into_boxed_str());
    let (url, server) = serve(vec![(401, echoed), (400, echoed), (200, r#"{"success":true,"result":[]}"#), (400, rate)]);
    let cloudflare = cloudflare_at(&url);
    let errors = [
        cloudflare.accounts(&token()).unwrap_err(),
        cloudflare.accounts(&token()).unwrap_err(),
        cloudflare.upload_worker(&token(), ACCOUNT, &upload(None, true)).unwrap_err(),
    ];
    server.join().unwrap();
    assert!(matches!(errors[0], CloudflareError::Refused(_)));
    assert!(matches!(errors[1], CloudflareError::Failed(_)));
    assert!(matches!(errors[2], CloudflareError::LimitRefused(_)));
    for error in errors {
        assert!(!error.said().contains(TOKEN), "{}", error.said());
        assert!(!format!("{error:?}").contains(TOKEN));
    }
    let closed = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", closed.local_addr().unwrap());
    drop(closed);
    let unreachable = cloudflare_at(&url).accounts(&token()).unwrap_err();
    assert!(!unreachable.said().contains(TOKEN), "{}", unreachable.said());
}
