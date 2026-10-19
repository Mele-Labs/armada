//! The central store's client: the signature on a request, the key file, each
//! reply read as the wire says, and the sends themselves against a socket on
//! this machine.

use std::io::{Read, Write};
use std::net::TcpListener;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::thread::JoinHandle;
use std::time::Duration;

use adapter_traits::{Claimed, Holder, HolderKind, OwnerKey, RedKey, RepositoryKey, RunRow, Secret, Store};
use p256::ecdsa::signature::Verifier;
use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use p256::ecdsa::{Signature, VerifyingKey};
use p256::pkcs8::DecodePublicKey;

use crate::store::{self, HttpStore, Reply};
use crate::store_device::{self, DeviceKey};
use crate::store_signing;

const EMPTY_SHA256: &str = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
const NONCE: &str = "00112233445566778899aabbccddeeff";

fn at() -> i64 {
    1_700_000_000
}

fn nonce() -> String {
    NONCE.to_string()
}

pub(super) fn key() -> DeviceKey {
    DeviceKey::from_secret(&[9u8; 32]).expect("a key")
}

fn unhex(text: &str) -> Vec<u8> {
    (0..text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&text[i..i + 2], 16).unwrap())
        .collect()
}

pub(super) fn verifies(public_key: &str, text: &str, signature: &str) -> bool {
    let der = B64.decode(public_key).expect("base64");
    let key = VerifyingKey::from_public_key_der(&der).expect("an SPKI public key");
    Signature::from_slice(&unhex(signature)).is_ok_and(|sig| key.verify(text.as_bytes(), &sig).is_ok())
}

fn holder(machine: &str) -> Holder {
    Holder { kind: HolderKind::Session, id: "j1".into(), machine: machine.into(), since: 100 }
}

fn repository() -> RepositoryKey {
    RepositoryKey::from_hash("aaaaaaaa11111111aaaaaaaa11111111aaaaaaaa11111111aaaaaaaa11111111").unwrap()
}

fn red() -> RedKey {
    RedKey::from_hash("bbbbbbbb22222222bbbbbbbb22222222bbbbbbbb22222222bbbbbbbb22222222").unwrap()
}

fn reply(status: u16, body: &str) -> Reply {
    Reply { status, body: body.as_bytes().to_vec() }
}

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("store-{name}-{}", std::process::id()));
    std::fs::remove_dir_all(&dir).ok();
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn the_signed_text_is_method_target_time_nonce_and_the_body_hash_joined_with_nothing() {
    assert_eq!(
        store_signing::signed_text("GET", "/v1/owners?keys=a,b", 1_700_000_000, NONCE, b""),
        format!("GET/v1/owners?keys=a,b1700000000{NONCE}{EMPTY_SHA256}")
    );
}

#[test]
fn a_request_signed_with_a_given_time_and_nonce_is_the_same_four_headers_every_time() {
    let first = store_signing::headers(&key(), "dev_1", "POST", "/v1/claims", at(), NONCE, b"{}");
    let again = store_signing::headers(&key(), "dev_1", "POST", "/v1/claims", at(), NONCE, b"{}");
    assert_eq!(first, again);
    let names: Vec<&str> = first.iter().map(|(name, _)| *name).collect();
    assert_eq!(names, ["X-Armada-Device", "X-Armada-Time", "X-Armada-Nonce", "X-Armada-Signature"]);
    assert_eq!(first[0].1, "dev_1");
    assert_eq!(first[1].1, "1700000000");
    assert_eq!(first[2].1, NONCE);
}

#[test]
fn the_signature_verifies_with_the_public_key_over_the_text_and_over_nothing_else() {
    let headers = store_signing::headers(&key(), "dev_1", "POST", "/v1/claims", at(), NONCE, b"{}");
    let signature = &headers[3].1;
    assert_eq!(signature.len(), 128, "hex of raw r||s");
    let text = store_signing::signed_text("POST", "/v1/claims", at(), NONCE, b"{}");
    assert!(verifies(&key().public_key(), &text, signature));
    for altered in [
        store_signing::signed_text("PUT", "/v1/claims", at(), NONCE, b"{}"),
        store_signing::signed_text("POST", "/v1/claims?x=1", at(), NONCE, b"{}"),
        store_signing::signed_text("POST", "/v1/claims", at() + 1, NONCE, b"{}"),
        store_signing::signed_text("POST", "/v1/claims", at(), "ff", b"{}"),
        store_signing::signed_text("POST", "/v1/claims", at(), NONCE, b"{ }"),
    ] {
        assert!(!verifies(&key().public_key(), &altered, signature), "{altered}");
    }
}

#[test]
fn the_key_is_made_once_and_owner_only_and_the_device_id_is_kept_beside_it() {
    let dir = scratch("key");
    let first = DeviceKey::open(&dir).unwrap().public_key();
    assert_eq!(DeviceKey::open(&dir).unwrap().public_key(), first);
    let mode = std::fs::metadata(dir.join(store_device::KEY_FILE)).unwrap().permissions().mode();
    assert_eq!(mode & 0o777, 0o600);
    assert_eq!(store_device::device_id(&dir).unwrap(), None);
    store_device::remember_device(&dir, "dev_42").unwrap();
    assert_eq!(store_device::device_id(&dir).unwrap().as_deref(), Some("dev_42"));
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_device_id_that_could_not_sit_in_a_header_is_not_kept() {
    let dir = scratch("id");
    for bad in ["", "a b", "a\r\nX-Evil: 1", "../x", &"a".repeat(65)] {
        assert!(store_device::remember_device(&dir, bad).is_err(), "{bad:?}");
    }
    assert_eq!(store_device::device_id(&dir).unwrap(), None);
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_claim_is_won_on_200_and_held_on_409_naming_the_holder() {
    assert_eq!(store::claimed(&reply(200, "{}")).unwrap(), Claimed::Won);
    let held = reply(409, r#"{"holder":{"kind":"session","id":"s9","machine":"mac-a","since":55}}"#);
    assert_eq!(
        store::claimed(&held).unwrap(),
        Claimed::Held(Holder { kind: HolderKind::Session, id: "s9".into(), machine: "mac-a".into(), since: 55 })
    );
}

#[test]
fn a_5xx_is_unreachable_and_any_other_refusal_is_refused() {
    assert!(store::claimed(&reply(503, "")).unwrap_err().is_unreachable());
    let refused = store::claimed(&reply(401, "bad signature")).unwrap_err();
    assert!(!refused.is_unreachable());
    assert_eq!(refused.said(), "The store refused the request: status 401: bad signature");
}

#[test]
fn a_holder_kind_this_build_does_not_know_is_refused_not_guessed() {
    let held = reply(409, r#"{"holder":{"kind":"robot","id":"x","machine":"m","since":1}}"#);
    assert!(!store::claimed(&held).unwrap_err().is_unreachable());
    assert!(store::claimed(&reply(409, "not json")).is_err());
}

#[test]
fn a_read_claim_is_none_on_404_and_a_delete_of_nothing_is_done() {
    assert_eq!(store::held(&reply(404, "")).unwrap(), None);
    assert!(store::deleted(&reply(404, "")).is_ok());
    assert!(store::deleted(&reply(204, "")).is_ok());
    assert!(store::deleted(&reply(403, "")).is_err());
}

#[test]
fn owners_come_back_only_for_the_keys_that_were_asked() {
    let one = OwnerKey::from_hash("eeeeeeee55555555eeeeeeee55555555eeeeeeee55555555eeeeeeee55555555").unwrap();
    let two = OwnerKey::from_hash("ffffffff66666666ffffffff66666666ffffffff66666666ffffffff66666666").unwrap();
    assert_eq!(
        store::owners_target(&[one.clone(), two.clone()]),
        "/v1/owners?keys=eeeeeeee55555555eeeeeeee55555555eeeeeeee55555555eeeeeeee55555555,ffffffff66666666ffffffff66666666ffffffff66666666ffffffff66666666"
    );
    let answered = reply(
        200,
        r#"{"eeeeeeee55555555eeeeeeee55555555eeeeeeee55555555eeeeeeee55555555":{"kind":"session","id":"j1","machine":"mac-a","since":1},
            "9999999999999999999999999999999999999999999999999999999999999999":{"kind":"session","id":"j2","machine":"mac-b","since":2}}"#,
    );
    let found = store::owned(&answered, &[one.clone(), two]).unwrap();
    assert_eq!(found.keys().collect::<Vec<_>>(), [&one]);
}

#[test]
fn the_claim_call_carries_hashes_and_the_holder_and_no_name() {
    let call = store::claim_call(&repository(), &red(), &holder("mac-a")).unwrap();
    assert_eq!((call.method, call.target.as_str()), ("POST", "/v1/claims"));
    assert_eq!(
        String::from_utf8(call.body).unwrap(),
        r#"{"repo":"aaaaaaaa11111111aaaaaaaa11111111aaaaaaaa11111111aaaaaaaa11111111","red":"bbbbbbbb22222222bbbbbbbb22222222bbbbbbbb22222222bbbbbbbb22222222","holder":{"kind":"session","id":"j1","machine":"mac-a","since":100}}"#
    );
}

// A socket on this machine that answers a script, one connection per entry.

pub(super) struct Seen {
    pub(super) method: String,
    pub(super) target: String,
    pub(super) headers: Vec<(String, String)>,
    pub(super) body: Vec<u8>,
}

impl Seen {
    pub(super) fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }
}

pub(super) fn serve(script: Vec<(u16, &'static str)>) -> (String, JoinHandle<Vec<Seen>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let handle = std::thread::spawn(move || {
        script
            .into_iter()
            .map(|(status, body)| {
                let (mut stream, _) = listener.accept().unwrap();
                let seen = read_request(&mut stream);
                let head = format!(
                    "HTTP/1.1 {status} X\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                );
                stream.write_all(head.as_bytes()).unwrap();
                stream.write_all(body.as_bytes()).unwrap();
                seen
            })
            .collect()
    });
    (url, handle)
}

fn read_request(stream: &mut std::net::TcpStream) -> Seen {
    let mut bytes = Vec::new();
    let mut chunk = [0u8; 1024];
    let split = loop {
        let n = stream.read(&mut chunk).unwrap();
        bytes.extend_from_slice(&chunk[..n]);
        if let Some(at) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
            break at + 4;
        }
        assert!(n > 0, "the request ended before its headers did");
    };
    let head = String::from_utf8(bytes[..split].to_vec()).unwrap();
    let mut lines = head.split("\r\n");
    let mut first = lines.next().unwrap().split(' ');
    let (method, target) = (first.next().unwrap().to_string(), first.next().unwrap().to_string());
    let headers: Vec<(String, String)> = lines
        .filter_map(|l| l.split_once(": "))
        .map(|(n, v)| (n.to_string(), v.to_string()))
        .collect();
    let wanted: usize = headers
        .iter()
        .find(|(n, _)| n.eq_ignore_ascii_case("content-length"))
        .map_or(0, |(_, v)| v.parse().unwrap());
    while bytes.len() < split + wanted {
        let n = stream.read(&mut chunk).unwrap();
        bytes.extend_from_slice(&chunk[..n]);
    }
    Seen { method, target, headers, body: bytes[split..split + wanted].to_vec() }
}

fn store_at(url: &str) -> HttpStore {
    HttpStore::made(url, "dev_1", key()).at(at, nonce)
}

#[test]
fn a_claim_goes_out_signed_and_the_signature_verifies_over_what_the_server_received() {
    let (url, server) = serve(vec![(200, "{}")]);
    let claimed = store_at(&url).claim(&repository(), &red(), &holder("mac-a")).unwrap();
    assert_eq!(claimed, Claimed::Won);
    let seen = &server.join().unwrap()[0];
    assert_eq!((seen.method.as_str(), seen.target.as_str()), ("POST", "/v1/claims"));
    assert_eq!(seen.header("x-armada-device"), Some("dev_1"));
    assert_eq!(seen.header("x-armada-time"), Some("1700000000"));
    assert_eq!(seen.header("x-armada-nonce"), Some(NONCE));
    let text = store_signing::signed_text(&seen.method, &seen.target, at(), NONCE, &seen.body);
    assert!(verifies(&key().public_key(), &text, seen.header("x-armada-signature").unwrap()));
}

#[test]
fn a_read_of_owners_signs_the_query_it_sent() {
    let (url, server) = serve(vec![(
        200,
        r#"{"eeeeeeee55555555eeeeeeee55555555eeeeeeee55555555eeeeeeee55555555":{"kind":"session","id":"s1","machine":"mac-a","since":9}}"#,
    )]);
    let asked = OwnerKey::from_hash("eeeeeeee55555555eeeeeeee55555555eeeeeeee55555555eeeeeeee55555555").unwrap();
    let found = store_at(&url).owners(&[asked.clone()]).unwrap();
    assert_eq!(found[&asked].machine, "mac-a");
    let seen = &server.join().unwrap()[0];
    assert_eq!(seen.target, "/v1/owners?keys=eeeeeeee55555555eeeeeeee55555555eeeeeeee55555555eeeeeeee55555555");
    let text = store_signing::signed_text("GET", &seen.target, at(), NONCE, b"");
    assert!(verifies(&key().public_key(), &text, seen.header("x-armada-signature").unwrap()));
}

#[test]
fn taking_over_deletes_the_row_then_claims_it() {
    let (url, server) = serve(vec![(204, ""), (200, "{}")]);
    let taken = store_at(&url).replace_claim(&repository(), &red(), &holder("mac-b")).unwrap();
    assert_eq!(taken, Claimed::Won);
    let seen = server.join().unwrap();
    assert_eq!(
        seen.iter().map(|s| (s.method.as_str(), s.target.as_str())).collect::<Vec<_>>(),
        [("DELETE", "/v1/claims/aaaaaaaa11111111aaaaaaaa11111111aaaaaaaa11111111aaaaaaaa11111111/bbbbbbbb22222222bbbbbbbb22222222bbbbbbbb22222222bbbbbbbb22222222"), ("POST", "/v1/claims")]
    );
}

#[test]
fn a_store_nothing_listens_for_is_unreachable() {
    let closed = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", closed.local_addr().unwrap());
    drop(closed);
    let said = store_at(&url).read_claim(&repository(), &red()).unwrap_err();
    assert!(said.is_unreachable(), "{said:?}");
}

#[test]
fn a_store_that_answers_401_refuses_and_is_not_unreachable() {
    let (url, server) = serve(vec![(401, "unknown device")]);
    let said = store_at(&url).release_claim(&repository(), &red()).unwrap_err();
    server.join().unwrap();
    assert!(!said.is_unreachable());
}

#[test]
fn enrolling_sends_the_public_key_unsigned_and_keeps_the_device_it_is_given() {
    let dir = scratch("enrol");
    let (url, server) = serve(vec![(200, r#"{"device":"dev_77"}"#)]);
    let token = Secret::new("device-flow-token".to_string());
    assert!(HttpStore::enrol(&url, &dir, &token, "mac-a", Duration::from_secs(5)).is_ok());
    let seen = &server.join().unwrap()[0];
    assert_eq!((seen.method.as_str(), seen.target.as_str()), ("POST", "/v1/enrol"));
    assert!(seen.header("x-armada-signature").is_none(), "enrolment carries a token, not a signature");
    let sent = String::from_utf8(seen.body.clone()).unwrap();
    assert!(sent.contains("device-flow-token") && sent.contains("\"machine\":\"mac-a\""));
    assert!(sent.contains(&DeviceKey::open(&dir).unwrap().public_key()));
    assert_eq!(store_device::device_id(&dir).unwrap().as_deref(), Some("dev_77"));
    assert!(HttpStore::open(&url, &dir, Duration::from_secs(5)).unwrap().is_some());
    let kept_the_token = std::fs::read_dir(&dir)
        .unwrap()
        .any(|f| std::fs::read_to_string(f.unwrap().path()).is_ok_and(|text| text.contains("device-flow-token")));
    assert!(!kept_the_token, "the token is not kept");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_machine_that_has_not_enrolled_opens_no_store() {
    let dir = scratch("unenrolled");
    assert!(HttpStore::open("http://127.0.0.1:1", &dir, Duration::from_secs(1)).unwrap().is_none());
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn signing_a_device_out_sends_a_signed_delete_and_a_device_already_gone_is_done() {
    let (url, server) = serve(vec![(204, ""), (404, r#"{"error":"no_device"}"#)]);
    let store = store_at(&url);
    store.delete_device("42.00ff").unwrap();
    store.delete_device("42.00ee").unwrap();
    let seen = server.join().unwrap();
    assert_eq!((seen[0].method.as_str(), seen[0].target.as_str()), ("DELETE", "/v1/devices/42.00ff"));
    let text = store_signing::signed_text("DELETE", &seen[0].target, at(), NONCE, b"");
    assert!(verifies(&key().public_key(), &text, seen[0].header("x-armada-signature").unwrap()));
}

#[test]
fn a_device_id_that_could_not_be_a_path_segment_is_refused_before_any_request() {
    let closed = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", closed.local_addr().unwrap());
    for bad in ["", "../claims", "a/b", "a b"] {
        let said = store_at(&url).delete_device(bad).unwrap_err();
        assert!(!said.is_unreachable(), "{bad:?} reached the network");
    }
}

#[test]
fn enrolling_tells_a_store_that_is_down_from_one_that_said_no() {
    let dir = scratch("enrol-down");
    let token = Secret::new("t".to_string());
    let closed = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", closed.local_addr().unwrap());
    drop(closed);
    assert!(HttpStore::enrol(&url, &dir, &token, "mac-a", Duration::from_secs(2)).err().unwrap().is_unreachable());
    let (url, server) = serve(vec![(401, r#"{"error":"forge_refused"}"#)]);
    let said = HttpStore::enrol(&url, &dir, &token, "mac-a", Duration::from_secs(2)).err().unwrap();
    server.join().unwrap();
    assert!(!said.is_unreachable());
    assert_eq!(store_device::device_id(&dir).unwrap(), None, "a refused enrolment keeps no device");
    std::fs::remove_dir_all(&dir).ok();
}

fn a_run(command: &str, port: Option<u16>) -> RunRow {
    RunRow {
        workspace: ".".into(),
        kind: "dev_server".into(),
        command: command.into(),
        port,
        succeeded: true,
        last_ran: 1_790_848_800_000,
        source: "jobs".into(),
    }
}

#[test]
fn runs_are_put_signed_with_the_whole_set_and_no_port_key_where_there_is_none() {
    let (url, server) = serve(vec![(204, "")]);
    store_at(&url).put_runs(&repository(), &[a_run("pnpm dev", Some(5173)), a_run("pnpm preview", None)]).unwrap();
    let seen = &server.join().unwrap()[0];
    assert_eq!((seen.method.as_str(), seen.target.as_str()), ("PUT", format!("/v1/runs/{}", repository().as_str()).as_str()));
    let body = String::from_utf8(seen.body.clone()).unwrap();
    assert_eq!(
        body,
        r#"{"facts":[{"workspace":".","kind":"dev_server","command":"pnpm dev","port":5173,"succeeded":true,"last_ran":1790848800000,"source":"jobs"},{"workspace":".","kind":"dev_server","command":"pnpm preview","succeeded":true,"last_ran":1790848800000,"source":"jobs"}]}"#
    );
    let text = store_signing::signed_text(&seen.method, &seen.target, at(), NONCE, &seen.body);
    assert!(verifies(&key().public_key(), &text, seen.header("x-armada-signature").unwrap()));
}

#[test]
fn runs_are_read_back_and_a_repository_the_store_holds_none_for_is_none() {
    let (url, server) = serve(vec![
        (200, r#"{"facts":[{"workspace":"web","kind":"tests","command":"pnpm test","succeeded":false,"last_ran":5,"source":"jobs"}]}"#),
        (404, r#"{"error":"no_runs"}"#),
    ]);
    let store = store_at(&url);
    let held = store.read_runs(&repository()).unwrap().expect("some");
    assert_eq!((held[0].workspace.as_str(), held[0].port, held[0].succeeded), ("web", None, false));
    assert_eq!(store.read_runs(&repository()).unwrap(), None);
    server.join().unwrap();
}

#[test]
fn a_store_that_refuses_runs_is_refused_and_one_that_fails_is_unreachable() {
    let (url, server) = serve(vec![(413, r#"{"error":"runs_full"}"#), (503, "")]);
    let store = store_at(&url);
    assert!(!store.put_runs(&repository(), &[]).unwrap_err().is_unreachable());
    assert!(store.read_runs(&repository()).unwrap_err().is_unreachable());
    server.join().unwrap();
}
