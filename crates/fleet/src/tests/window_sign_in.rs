//! What `show_window` takes beyond a page: the origins its window may pass through, and the
//! addresses a session signs in with. `docs/concepts/session.md`, *Showing a window*, and
//! `docs/practices/capture-window.md`.
//!
//! **Origins are refused unless they are origins, and a sign-in is refused unless the window may
//! go there.** The sign-in is a login, so the case that matters is the last one: it reaches no
//! row, no record, no event and no timeline entry, and Bridge takes it once.

use std::time::Duration;

use api::{Next, Sessions};
use ipc::{SessionFact, SessionId, SessionOrigin, SessionReport, ShowWindow, SignIn, TakeWindowSignIn};

use super::timeline::{kept, read, Fixture};
use crate::tests::tmp::TempDir;

const PAGE: &str = "https://staging.example.com/app";
const SSO: &str = "https://login.example.org";
const TOKEN: &str = "s3cr3t-t0k3n";

fn id() -> SessionId {
    SessionId::carried("t1")
}

async fn started(fleet: &Fixture) {
    fleet
        .report_session(SessionReport {
            harness: "a-harness".to_string(),
            session_id: id(),
            fact: SessionFact::Started {
                cwd: "/work".to_string(),
                title: Some("Walk".to_string()),
                origin: SessionOrigin::Terminal,
                mod_version: None,
            },
        })
        .await
        .expect("a session");
}

fn show(origins: &[&str], sign_in: &[&str]) -> ShowWindow {
    ShowWindow {
        url: PAGE.to_string(),
        title: Some("Staging".to_string()),
        session_id: Some(id()),
        origins: origins.iter().map(|one| one.to_string()).collect(),
        sign_in: SignIn(sign_in.iter().map(|one| one.to_string()).collect()),
        ..Default::default()
    }
}

#[tokio::test]
async fn an_origin_is_an_http_or_https_origin_and_nothing_longer() {
    let home = TempDir::new();
    let (fleet, _) = kept(&home);
    started(&fleet).await;

    for bad in [
        "https://login.example.org/",
        "https://login.example.org/sso",
        "https://login.example.org?next=1",
        "https://user@login.example.org",
        "ftp://login.example.org",
        "file:///etc/passwd",
        "login.example.org",
        "https://",
        "",
    ] {
        let refused = fleet.show_window(None, show(&[SSO, bad], &[])).await;
        assert!(refused.is_err(), "{bad:?} is not an origin");
    }
    let record = fleet.show_window(None, show(&[SSO, "http://localhost:8443"], &[])).await.expect("origins are shown");
    let window = record
        .attachments
        .iter()
        .find(|one| one.detail.get("form").map(String::as_str) == Some("window"))
        .expect("the window is on the ledger");
    let kept = window.detail.get("origins").expect("and its origins with it");
    assert!(kept.contains(SSO) && kept.contains("localhost:8443"), "{kept}");
}

#[tokio::test]
async fn a_sign_in_address_is_on_the_shown_origin_or_a_named_one() {
    let home = TempDir::new();
    let (fleet, _) = kept(&home);
    started(&fleet).await;

    let on_page = "https://staging.example.com/dev/login?token=abc";
    let on_sso = "https://login.example.org/start?code=abc";
    for (origins, sign_in) in [
        (vec![], vec!["https://elsewhere.example.net/login"]),
        (vec![], vec![on_sso]),
        (vec![SSO], vec![on_page, "https://elsewhere.example.net/login"]),
        (vec![], vec!["file:///etc/passwd"]),
        (vec![], vec!["javascript:alert(1)"]),
        (vec![], vec![""]),
        (vec![], vec!["https://staging.example.com.evil.example.net/login"]),
    ] {
        let refused = fleet.show_window(None, show(&origins, &sign_in)).await;
        assert!(refused.is_err(), "{sign_in:?} with {origins:?}");
    }
    assert!(fleet.show_window(None, show(&[], &[on_page])).await.is_ok(), "the shown origin");
    assert!(fleet.show_window(None, show(&[SSO], &[on_sso, on_page])).await.is_ok(), "a named origin");
}

#[tokio::test]
async fn a_sign_in_reaches_no_row_record_event_or_timeline_entry_and_is_taken_once() {
    let home = TempDir::new();
    let (fleet, _) = kept(&home);
    started(&fleet).await;
    let mut watching = fleet.events().subscribe();
    let link = format!("https://staging.example.com/dev/login?token={TOKEN}");

    let record = fleet.show_window(None, show(&[SSO], &[&link])).await.expect("shown");

    let mut said = ipc::encode(&record).expect("the record");
    let rows = fleet.rows_of(id().as_str()).await.expect("the thread");
    assert!(!rows.is_empty(), "the window row is on the thread");
    said.push_str(&ipc::encode(&rows).expect("the thread's rows"));
    while let Ok(Some(Next::Send(delivered))) = tokio::time::timeout(Duration::from_millis(100), watching.next()).await {
        said.push_str(&ipc::encode(&delivered.event).expect("an event"));
    }
    said.push_str(&ipc::encode(&read(&fleet).await).expect("the timeline"));
    assert!(said.contains("staging.example.com"), "what was read holds the window: {said}");
    assert!(!said.contains(TOKEN), "the sign-in reached a record, a row, an event or the timeline");

    let take = || TakeWindowSignIn { session_id: id(), url: PAGE.to_string() };
    assert_eq!(fleet.take_window_sign_in(take()).await.expect("taken").sign_in.0, [link]);
    assert!(fleet.take_window_sign_in(take()).await.expect("taken").sign_in.0.is_empty(), "once");
}

#[tokio::test]
async fn showing_the_page_again_without_a_sign_in_leaves_none_to_take() {
    let home = TempDir::new();
    let (fleet, _) = kept(&home);
    started(&fleet).await;
    let link = format!("https://staging.example.com/dev/login?token={TOKEN}");

    fleet.show_window(None, show(&[], &[&link])).await.expect("shown");
    fleet.show_window(None, show(&[], &[])).await.expect("shown again");
    let taken = fleet.take_window_sign_in(TakeWindowSignIn { session_id: id(), url: PAGE.to_string() }).await.expect("taken");
    assert!(taken.sign_in.0.is_empty());
}

#[test]
fn a_sign_in_printed_for_debugging_says_how_many_and_nothing_else() {
    let held = show(&[], &["https://staging.example.com/dev/login?token=s3cr3t-t0k3n"]);
    assert!(!format!("{held:?}").contains(TOKEN));
}

#[tokio::test]
async fn a_sign_in_nobody_took_is_gone_once_its_session_ends() {
    let home = TempDir::new();
    let (fleet, _) = kept(&home);
    started(&fleet).await;
    let link = format!("https://staging.example.com/dev/login?token={TOKEN}");
    let take = || TakeWindowSignIn { session_id: id(), url: PAGE.to_string() };

    fleet.show_window(None, show(&[], &[&link])).await.expect("shown");
    fleet
        .report_session(SessionReport {
            harness: "a-harness".to_string(),
            session_id: id(),
            fact: SessionFact::Ended { reason: "exit".to_string() },
        })
        .await
        .expect("ended");
    assert!(fleet.take_window_sign_in(take()).await.expect("taken").sign_in.0.is_empty());
}
