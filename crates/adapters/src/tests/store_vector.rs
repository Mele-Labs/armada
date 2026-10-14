//! The signing vector the Worker's tests read too: one fixed request, its signed
//! text, and one signature under a test-only key. `docs/contracts/central-store.md`,
//! *Signing*.
//!
//! **Passes without checking where the file is not in the tree**, which is any
//! checkout before the Worker's branch has landed. The vector is the one check
//! that this signer and the Worker's agree on the bytes, so once it lands a
//! missing file is a deletion worth noticing.

use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use p256::pkcs8::DecodePrivateKey;
use serde::Deserialize;

use crate::store_device::DeviceKey;
use crate::store_signing;
use crate::tests::store::verifies;

#[derive(Deserialize)]
struct Vector {
    private_key_pkcs8_base64: String,
    public_key_spki_base64: String,
    method: String,
    path_and_query: String,
    time: String,
    nonce: String,
    body: String,
    body_sha256_hex: String,
    signed_text: String,
    signature_hex: String,
}

fn vector() -> Option<Vector> {
    let file = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../apps/store/test-vectors/signing.json");
    let bytes = std::fs::read(&file).ok()?;
    Some(ipc::decode("signing vector", &bytes).expect("the vector reads"))
}

fn key_of(vector: &Vector) -> DeviceKey {
    let der = B64.decode(&vector.private_key_pkcs8_base64).expect("base64");
    let key = p256::ecdsa::SigningKey::from_pkcs8_der(&der).expect("a PKCS8 key");
    DeviceKey::from_secret(&key.to_bytes()).expect("a device key")
}

#[test]
fn this_signer_writes_the_signed_text_the_worker_does() {
    let Some(v) = vector() else { return };
    let time: i64 = v.time.parse().expect("seconds");
    let text = store_signing::signed_text(&v.method, &v.path_and_query, time, &v.nonce, v.body.as_bytes());
    assert_eq!(text, v.signed_text);
    assert!(text.ends_with(&v.body_sha256_hex));
}

#[test]
fn a_signature_the_worker_side_made_verifies_here() {
    let Some(v) = vector() else { return };
    assert!(verifies(&v.public_key_spki_base64, &v.signed_text, &v.signature_hex));
}

#[test]
fn a_signature_made_here_over_the_vector_verifies_under_its_public_key() {
    let Some(v) = vector() else { return };
    let key = key_of(&v);
    assert_eq!(key.public_key(), v.public_key_spki_base64, "the SPKI this machine would enrol");
    let time: i64 = v.time.parse().expect("seconds");
    let headers = store_signing::headers(&key, "dev", &v.method, &v.path_and_query, time, &v.nonce, v.body.as_bytes());
    assert!(verifies(&v.public_key_spki_base64, &v.signed_text, &headers[3].1));
}
