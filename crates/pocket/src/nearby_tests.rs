//! Two Gateways with no tunnel, each with a Wi-Fi door on loopback, told of each other as Bonjour would:
//! each lists the other as nearby, one asks, both show the same four digits, and Confirm links them at
//! their doors, which then refuse any certificate but the one paired with.

use std::collections::HashSet;
use std::net::{IpAddr, Ipv4Addr};

use axum::http::{Method, StatusCode};
use ipc::{FoundMac, FoundMacs, Mac, MacColour, MacLink};

use crate::door::{bind_door_on, Door, DoorCert};
use crate::mac_tests::{until, Side};
use crate::nearby::{read_mac, NearbyMac};
use crate::tunnel::tunnel_name;

/// Open a door on loopback for `side`, with a certificate of its own.
fn open_door(side: &Side) -> Door {
    Door::serve(bind_door_on(Ipv4Addr::LOCALHOST).unwrap(), &DoorCert::made().unwrap(), side.gateway.clone()).unwrap()
}

/// What Bonjour would tell `to` of `side`, whose door is `door`.
fn sees(to: &Side, side: &Side, door: &Door) {
    let mac = NearbyMac {
        id: side.id(),
        name: crate::machine::this_mac(&side.gateway).unwrap().name,
        colour: Some(MacColour::Teal),
        build: None,
        wire: Some(side.gateway.way_in.wire.clone()),
        built_at: side.gateway.way_in.built_at.clone(),
        address: format!("https://127.0.0.1:{}", door.port),
    };
    to.gateway.macs.nearby.saw(format!("{}._armada._tcp.local.", tunnel_name(&side.id())), mac);
}

async fn found(side: &Side) -> FoundMacs {
    let (status, body) = side.admin(Method::GET, "/admin/macs/found", "").await;
    assert_eq!(status, StatusCode::OK, "{body}");
    ipc::decode("found Macs", body.as_bytes()).unwrap()
}

async fn ask(work: &Side, home: &Side) -> (StatusCode, String) {
    let body = ipc::encode(&ipc::AskMacPair { tunnel: tunnel_name(&home.id()) }).unwrap();
    work.admin(Method::POST, "/admin/macs/pair/ask", &body).await
}

/// Two Macs with no tunnel, each with its door open and seeing the other's.
async fn side_by_side() -> (Side, Door, Side, Door) {
    let home = Side::unaddressed("Home").await;
    let work = Side::unaddressed("Work").await;
    let (home_door, work_door) = (open_door(&home), open_door(&work));
    sees(&work, &home, &home_door);
    sees(&home, &work, &work_door);
    (home, home_door, work, work_door)
}

#[tokio::test]
async fn two_macs_with_no_tunnel_find_each_other_nearby_and_pair_at_their_doors() {
    let (home, _home_door, work, _work_door) = side_by_side().await;

    let listed = found(&work).await.macs;
    let expected = FoundMac {
        tunnel: tunnel_name(&home.id()),
        name: "Home".into(),
        colour: Some(MacColour::Teal),
        address: None,
        online: true,
        paired: false,
        build: None,
        nearby: true,
        older_build: false,
    };
    assert_eq!(listed, vec![expected], "found nearby, with no address on any account");

    let (status, body) = ask(&work, &home).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let waiting: Mac = ipc::decode("a Mac", body.as_bytes()).unwrap();
    let code = waiting.code.clone().expect("the asking Mac shows the code");
    assert_eq!((waiting.link.clone(), waiting.address.clone()), (Some(MacLink::ConfirmWaiting), None));
    let there = home.mac(&work.id()).await.expect("the ask is listed on the Mac that was asked");
    assert_eq!((there.link, there.code.as_deref()), (Some(MacLink::ClaimWaiting), Some(code.as_str())), "the same four digits");

    let confirm = ipc::encode(&ipc::MacNamed { mac: work.id() }).unwrap();
    let (status, body) = home.admin(Method::POST, "/admin/macs/pair/confirm", &confirm).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    // Each heard from the other through its door: the link's signed hello went there and was answered.
    until("work hears from home at its door", || async {
        work.mac(&home.id()).await.is_some_and(|mac| mac.link == Some(MacLink::Linked) && mac.last_heard_at.is_some())
    })
    .await;
    until("home hears from work at its door", || async {
        home.mac(&work.id()).await.is_some_and(|mac| mac.link == Some(MacLink::Linked) && mac.last_heard_at.is_some())
    })
    .await;
    let row = work.gateway.pairing.store.lock().unwrap().paired_mac(&home.id()).unwrap().unwrap();
    assert_eq!((row.address.as_str(), row.pin), ("", home.gateway.macs.nearby.own_pin()), "home's pin is kept, and no address");
}

#[tokio::test]
async fn a_paired_door_with_another_certificate_is_refused() {
    let (home, _home_door, work, _work_door) = side_by_side().await;
    assert_eq!(ask(&work, &home).await.0, StatusCode::OK);
    let confirm = ipc::encode(&ipc::MacNamed { mac: work.id() }).unwrap();
    assert_eq!(home.admin(Method::POST, "/admin/macs/pair/confirm", &confirm).await.0, StatusCode::OK);
    let home_id = home.id();
    let hello = || crate::mac_client::signed_whole(&work.gateway, &home_id, "", "GET", "/api/mac/hello", None);
    assert_eq!(hello().await.map(|came| came.status), Ok(200), "the door it paired with");

    // Another door on the same Gateway, as a Mac between them would show: its own certificate.
    let impostor = open_door(&home);
    work.gateway.macs.nearby.saw(format!("{}._armada._tcp.local.", tunnel_name(&home.id())), NearbyMac {
        id: home.id(),
        name: "Home".into(),
        colour: None,
        build: None,
        wire: None,
        built_at: None,
        address: format!("https://127.0.0.1:{}", impostor.port),
    });
    assert!(hello().await.is_err(), "a certificate it did not pair with");
}

#[tokio::test]
async fn the_door_answers_only_what_macs_call_each_other_on() {
    let home = Side::unaddressed("Home").await;
    let door = open_door(&home);
    let (client, seen) = crate::door::client(None).unwrap();
    let get = |path: &'static str| {
        let client = client.clone();
        async move { client.get(format!("https://127.0.0.1:{}{path}", door.port)).send().await.unwrap().status().as_u16() }
    };
    assert_eq!(get("/pair/mac/hello").await, 200);
    assert_eq!(seen.lock().unwrap().clone(), home.gateway.macs.nearby.own_pin(), "the certificate it showed is the one it names");
    for path in ["/admin/macs", "/admin/status", "/pair", "/api/sessions", "/api/machine", "/", "/index.html"] {
        assert_eq!(get(path).await, 404, "{path}");
    }
    assert_eq!(get("/api/mac/sessions").await, 400, "a Mac's route, refused by the signed layer for want of a signature");
}

#[test]
fn bonjour_is_read_for_a_routable_ipv4_and_a_mac_that_says_who_it_is() {
    let said = |id: &str| {
        let id = id.to_string();
        move |key: &str| match key {
            "id" => Some(id.clone()),
            "name" => Some("Home".to_string()),
            "colour" => Some("teal".to_string()),
            "wire" => Some("w2".to_string()),
            "built_at" => Some("2026-11-14T10:00:00Z".to_string()),
            _ => None,
        }
    };
    let ips: HashSet<IpAddr> =
        ["169.254.1.2", "fe80::1", "192.168.1.20", "2001:db8::1"].into_iter().map(|ip| ip.parse().unwrap()).collect();
    let mac = read_mac(said("ab12"), &ips, 5000).unwrap();
    assert_eq!((mac.address.as_str(), mac.colour), ("https://192.168.1.20:5000", Some(MacColour::Teal)));
    assert_eq!((mac.wire.as_deref(), mac.built_at.as_deref()), (Some("w2"), Some("2026-11-14T10:00:00Z")));
    assert_eq!(read_mac(said("not hex"), &ips, 5000), None);
    assert_eq!(read_mac(said("ab12"), &ips, 0), None);
    let only_scoped: HashSet<IpAddr> = ["fe80::1".parse().unwrap()].into();
    assert_eq!(read_mac(said("ab12"), &only_scoped, 5000), None, "a scoped address cannot be written in a URL");
}

/// This Mac's way in, on wire `w2` and built at noon.
fn mine(side: &Side) -> crate::WayIn {
    crate::WayIn { wire: "w2".into(), built_at: Some("2026-11-14T12:00:00Z".into()), ..side.gateway.way_in.clone() }
}

#[tokio::test]
async fn a_mac_is_older_on_another_wire_built_no_later_and_never_on_the_same_wire() {
    let me = mine(&Side::unaddressed("Work").await);
    let older = |wire, built_at| crate::macs::older_build(wire, built_at, &me);
    assert!(older(Some("w1"), Some("2026-11-14T09:00:00Z")), "another wire, built before");
    assert!(older(Some("w1"), Some("2026-11-14T12:00:00Z")), "another wire, built at the same time");
    assert!(!older(Some("w3"), Some("2026-11-14T15:00:00Z")), "another wire, built after, so newer");
    assert!(!older(Some("w2"), Some("2026-11-01T00:00:00Z")), "the same wire is never older, whatever its date");
    assert!(!older(Some("w2"), None), "the same wire, with no date");
    assert!(older(None, Some("2026-11-14T15:00:00Z")), "a build from before wires");
    assert!(older(Some("w1"), None), "another wire that named no build time");
}

#[tokio::test]
async fn a_found_mac_is_marked_older_from_what_it_says_nearby_and_false_where_nothing_was_heard() {
    let home = Side::unaddressed("Home").await;
    let work = Side::unaddressed("Work").await;
    let door = open_door(&home);
    let near = |wire: Option<&str>, built_at: Option<&str>| NearbyMac {
        id: home.id(),
        name: "Home".into(),
        colour: None,
        build: None,
        wire: wire.map(str::to_string),
        built_at: built_at.map(str::to_string),
        address: format!("https://127.0.0.1:{}", door.port),
    };
    let key = format!("{}._armada._tcp.local.", tunnel_name(&home.id()));
    // The test Gateway has no build time, so only the wire and a missing build time tell here; the dates are above.
    let wire = work.gateway.way_in.wire.clone();
    let cases = [
        (near(Some("another-wire"), None), true, "another wire that named no build time"),
        (near(Some("another-wire"), Some("9999-01-01T00:00:00Z")), false, "another wire, built later than a build with no time"),
        (near(Some(&wire), None), false, "the same wire"),
        (near(None, None), true, "a TXT record with no wire is a build from before wires"),
    ];
    for (mac, older, why) in cases {
        work.gateway.macs.nearby.saw(key.clone(), mac);
        assert_eq!(found(&work).await.macs[0].older_build, older, "{why}");
    }
    let never = Side::unaddressed("Never").await;
    assert!(found(&never).await.macs.iter().all(|mac| !mac.older_build), "a Mac never heard from stays false");
}

#[tokio::test]
async fn a_hello_with_no_wire_is_a_build_from_before_wires_and_so_older() {
    let me = mine(&Side::unaddressed("Work").await);
    let hello: crate::macs::MacGreeting = ipc::decode("a hello", r#"{"name":"Home","colour":"teal","build":"abc"}"#.as_bytes()).unwrap();
    assert_eq!((hello.wire.as_deref(), hello.built_at.as_deref()), (None, None));
    assert!(crate::macs::older_build(hello.wire.as_deref(), hello.built_at.as_deref(), &me));
}
