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
    let hello = || crate::mac_client::signed_whole(&work.gateway, &home.id(), "", "GET", "/api/mac/hello", None);
    assert_eq!(hello().await.map(|came| came.status), Ok(200), "the door it paired with");

    // Another door on the same Gateway, as a Mac between them would show: its own certificate.
    let impostor = open_door(&home);
    work.gateway.macs.nearby.saw(format!("{}._armada._tcp.local.", tunnel_name(&home.id())), NearbyMac {
        id: home.id(),
        name: "Home".into(),
        colour: None,
        build: None,
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
    assert_eq!(get("/api/mac/sessions").await, 401, "a Mac's route, and unsigned");
}

#[test]
fn bonjour_is_read_for_a_routable_ipv4_and_a_mac_that_says_who_it_is() {
    let said = |id: &str| {
        let id = id.to_string();
        move |key: &str| match key {
            "id" => Some(id.clone()),
            "name" => Some("Home".to_string()),
            "colour" => Some("teal".to_string()),
            _ => None,
        }
    };
    let ips: HashSet<IpAddr> =
        ["169.254.1.2", "fe80::1", "192.168.1.20", "2001:db8::1"].into_iter().map(|ip| ip.parse().unwrap()).collect();
    let mac = read_mac(said("ab12"), &ips, 5000).unwrap();
    assert_eq!((mac.address.as_str(), mac.colour), ("https://192.168.1.20:5000", Some(MacColour::Teal)));
    assert_eq!(read_mac(said("not hex"), &ips, 5000), None);
    assert_eq!(read_mac(said("ab12"), &ips, 0), None);
    let only_scoped: HashSet<IpAddr> = ["fe80::1".parse().unwrap()].into();
    assert_eq!(read_mac(said("ab12"), &only_scoped, 5000), None, "a scoped address cannot be written in a URL");
}
