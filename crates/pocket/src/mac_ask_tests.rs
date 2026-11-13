//! Two Gateways on loopback on one fake Cloudflare account: each finds the other, one asks, both show the
//! same four digits, and the other confirms or declines.

use std::sync::Arc;

use axum::http::{Method, StatusCode};
use ipc::{FoundMac, FoundMacs, Mac, MacColour, MacLink};

use crate::mac_tests::{until, Side};
use crate::macs::{digits, Asking, HeldClaim, MacAsk, MacHello};
use crate::tunnel::{tunnel_name, AccountTunnel};
use crate::tunnel_tests::{fake_tunnel, Fake};
use crate::MacKeys;

/// `name`'s Mac, on an account whose tunnels the test sets on the [`Fake`] it answers with.
async fn on_account(name: &str) -> (Side, Arc<Fake>) {
    let mut side = Side::new(name, &[]).await;
    let fake = Arc::new(Fake::default());
    side.stop().await;
    side.gateway.way_in.tunnel = fake_tunnel(fake.clone(), Some(&tunnel_name(&side.id())));
    side.start();
    (side, fake)
}

fn tunnel_of(side: &Side, connected: bool) -> AccountTunnel {
    AccountTunnel { name: tunnel_name(&side.id()), connected, address: Some(side.address.clone()) }
}

fn account(fake: &Fake, tunnels: Vec<AccountTunnel>) {
    *fake.account.lock().unwrap() = Some(Ok(tunnels));
}

async fn found(side: &Side) -> FoundMacs {
    let (status, body) = side.admin(Method::GET, "/admin/macs/found", "").await;
    assert_eq!(status, StatusCode::OK, "{body}");
    ipc::decode("found Macs", body.as_bytes()).unwrap()
}

/// Forget what was read of the account, as the few seconds it is kept would.
fn read_again(side: &Side) {
    side.gateway.macs.found.lock().unwrap().at = None;
}

async fn ask(work: &Side, home: &Side) -> (StatusCode, String) {
    let body = ipc::encode(&ipc::AskMacPair { tunnel: tunnel_name(&home.id()) }).unwrap();
    work.admin(Method::POST, "/admin/macs/pair/ask", &body).await
}

#[tokio::test]
async fn a_mac_on_the_account_is_found_asked_and_paired_with_the_same_code_on_both() {
    let (home, _) = on_account("Home").await;
    let (work, fake) = on_account("Work").await;
    let stranger = AccountTunnel { name: "his-own-tunnel".into(), connected: true, address: Some("https://blog.example.com".into()) };
    account(&fake, vec![tunnel_of(&home, true), tunnel_of(&work, true), stranger]);

    let listed = found(&work).await;
    let expected = FoundMac {
        tunnel: tunnel_name(&home.id()),
        name: "Home".into(),
        colour: Some(MacColour::Teal),
        address: Some(home.address.clone()),
        online: true,
        paired: false,
        build: None,
    };
    assert_eq!(listed, FoundMacs { macs: vec![expected], why: None }, "only his other Armada Mac, and never this one");

    let (status, body) = ask(&work, &home).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let waiting: Mac = ipc::decode("a Mac", body.as_bytes()).unwrap();
    let code = waiting.code.clone().expect("the asking Mac shows the code");
    assert!(code.len() == 4 && code.bytes().all(|b| b.is_ascii_digit()), "{code}");
    assert_eq!((waiting.id.as_str(), waiting.link.clone()), (home.id().as_str(), Some(MacLink::ConfirmWaiting)));
    assert_eq!(waiting.why.as_deref(), Some(format!("Confirm on Home if it shows {code}.").as_str()));
    assert_eq!(work.mac(&home.id()).await, Some(waiting));

    let there = home.mac(&work.id()).await.expect("the ask is listed on the Mac that was asked");
    assert_eq!((there.link, there.code.as_deref()), (Some(MacLink::ClaimWaiting), Some(code.as_str())), "the same four digits");
    assert_eq!(there.why.as_deref(), Some(format!("Work wants to pair. Confirm if it shows {code}.").as_str()));
    assert!(found(&work).await.macs[0].paired, "a Mac waiting on Confirm is not offered again");

    let confirm = ipc::encode(&ipc::MacNamed { mac: work.id() }).unwrap();
    let (status, body) = home.admin(Method::POST, "/admin/macs/pair/confirm", &confirm).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    until("work is linked to home", || async { work.mac(&home.id()).await.is_some_and(|mac| mac.link == Some(MacLink::Linked)) }).await;
    assert_eq!(home.mac(&work.id()).await.unwrap().link, Some(MacLink::Linked));
    assert_eq!(work.mac(&home.id()).await.unwrap().code, None, "the code goes once they are paired");
    let (status, why) = ask(&work, &home).await;
    assert_eq!((status, why.as_str()), (StatusCode::CONFLICT, "Home is paired with this Mac already."));
}

#[tokio::test]
async fn a_declined_ask_ends_on_both_macs() {
    let (home, _) = on_account("Home").await;
    let (work, fake) = on_account("Work").await;
    account(&fake, vec![tunnel_of(&home, true)]);
    assert_eq!(ask(&work, &home).await.0, StatusCode::OK);

    let decline = ipc::encode(&ipc::MacNamed { mac: work.id() }).unwrap();
    assert_eq!(home.admin(Method::POST, "/admin/macs/forget", &decline).await.0, StatusCode::NO_CONTENT);
    assert_eq!(home.mac(&work.id()).await, None);
    until("work's wait ends", || async { work.mac(&home.id()).await.is_none() }).await;
    assert!(work.keys.read(&home.id()).unwrap().is_none(), "and its key toward home goes with it");
}

#[tokio::test]
async fn an_ask_nobody_confirms_is_gone_from_both_macs_after_five_minutes() {
    let (home, _) = on_account("Home").await;
    let (work, fake) = on_account("Work").await;
    account(&fake, vec![tunnel_of(&home, true)]);
    assert_eq!(ask(&work, &home).await.0, StatusCode::OK);

    // The asked Mac: held for five minutes, then not listed and not confirmable.
    let held = home.gateway.macs.claims(home.gateway.pairing.now());
    assert_eq!(held.len(), 1);
    let late = held[0].at + crate::pairing::CODE_TTL;
    assert_eq!(home.gateway.macs.claims(late - 1).len(), 1);
    assert!(home.gateway.macs.claims(late).is_empty() && home.gateway.macs.claim_of(&work.id(), late).is_none());

    // The asking Mac: its wait is dropped, with its key, at the first look after.
    work.gateway.macs.asking.lock().unwrap().insert(home.id(), Asking { digits: "0000".into(), expires: 0 });
    assert_eq!(work.mac(&home.id()).await, None);
    assert!(work.keys.read(&home.id()).unwrap().is_none());
}

#[tokio::test]
async fn a_public_address_holds_three_asks_and_the_minutes_share_of_tries() {
    let home = Side::new("Home", &[]).await;
    let stranger = |n: u8| {
        let key = p256::ecdsa::SigningKey::from_slice(&[n; 32]).unwrap();
        let mac = MacHello { id: format!("{n:02x}{n:02x}"), name: format!("Mac {n}"), colour: MacColour::Blue, address: Some("https://x.example.com".into()), build: None, wire: None, built_at: None, source: None };
        ipc::encode(&MacAsk { mac, spki_hex: crate::mac_keys::spki_hex(&key).unwrap(), back_code: "ab".into() }).unwrap()
    };
    let post = |body: String| {
        let address = home.address.clone();
        async move {
            let answer = reqwest::Client::new()
                .post(format!("{address}/pair/mac/ask"))
                .header("content-type", "application/json")
                .body(body)
                .send()
                .await
                .unwrap();
            (answer.status().as_u16(), answer.text().await.unwrap())
        }
    };
    for n in 1..=3 {
        assert_eq!(post(stranger(n)).await.0, 202, "ask {n} is held");
    }
    let (status, why) = post(stranger(4)).await;
    assert_eq!((status, why.as_str()), (429, "Other Macs are already waiting to pair with Home. Wait a few minutes, then press Pair again."));
    assert_eq!(post(stranger(1)).await.0, 202, "a Mac asking again replaces its own ask");
    assert_eq!(home.gateway.macs.claims(home.gateway.pairing.now()).len(), 3);
    let mut last = (0, String::new());
    for n in 5..=15 {
        last = post(stranger(n)).await;
    }
    assert_eq!(last, (429, "Too many tries. Wait a minute, then try again.".to_string()), "the minute's share is spent");
}

#[tokio::test]
async fn the_unsigned_hello_says_only_name_colour_and_build() {
    let home = Side::new("Home", &[]).await;
    let answer = reqwest::get(format!("{}/pair/mac/hello", home.address)).await.unwrap();
    assert_eq!(answer.status().as_u16(), 200);
    assert_eq!(answer.text().await.unwrap(), r#"{"name":"Home","colour":"teal"}"#);
}

#[tokio::test]
async fn an_account_that_cannot_be_read_says_why_and_keeps_what_it_read_before() {
    let (home, _) = on_account("Home").await;
    let (work, fake) = on_account("Work").await;
    assert_eq!(found(&work).await, FoundMacs { macs: vec![], why: Some("This Mac is not signed in to Cloudflare yet.".into()) });

    account(&fake, vec![tunnel_of(&home, true)]);
    read_again(&work);
    assert_eq!(found(&work).await.macs.len(), 1);

    *fake.account.lock().unwrap() = Some(Err("Cloudflare could not be reached: timed out.".into()));
    read_again(&work);
    let stale = found(&work).await;
    assert_eq!((stale.macs.len(), stale.why.as_deref()), (1, Some("Cloudflare could not be reached: timed out.")));
}

#[tokio::test]
async fn an_asleep_mac_keeps_the_name_it_was_last_read_with() {
    let (mut home, _) = on_account("Home").await;
    let (work, fake) = on_account("Work").await;
    account(&fake, vec![tunnel_of(&home, true)]);
    assert_eq!(found(&work).await.macs[0].name, "Home");

    home.stop().await;
    account(&fake, vec![tunnel_of(&home, false)]);
    read_again(&work);
    let asleep = &found(&work).await.macs[0];
    assert_eq!((asleep.name.as_str(), asleep.online, asleep.colour), ("Home", false, Some(MacColour::Teal)));
}

#[test]
fn both_macs_draw_the_same_four_digits_from_the_key_and_the_random_half() {
    let one = digits("3059301306072a8648ce3d0201", "00112233445566778899aabbccddeeff");
    assert_eq!(one, digits("3059301306072a8648ce3d0201", "00112233445566778899aabbccddeeff"));
    assert_eq!(one.len(), 4);
    assert_ne!(one, digits("3059301306072a8648ce3d0201", "ffeeddccbbaa99887766554433221100"));
    // A claim held for an ask lives five minutes; one on a link waits for Confirm.
    let macs = crate::macs::quiet_macs();
    let claim = |digits: Option<String>| HeldClaim {
        mac: MacHello { id: "ab".into(), name: "Work".into(), colour: MacColour::Blue, address: None, build: None, wire: None, built_at: None, source: None },
        public_key: vec![],
        back_code: "cd".into(),
        at: 100,
        digits,
    };
    assert!(macs.hold_ask("n1".into(), claim(Some("1234".into()))));
    assert!(macs.claims(100 + crate::pairing::CODE_TTL).is_empty());
}
