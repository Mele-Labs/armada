//! This Mac's keys toward its other Macs, and the signature it puts on each call to one.
//!
//! **A private key never leaves the Mac that made it.** Each side makes its own at pairing and
//! hands the other only the public half, so one link and one Confirm give a key each way.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use adapter_traits::Secret;
use p256::ecdsa::signature::RandomizedSigner;
use p256::ecdsa::{Signature, SigningKey};
use p256::elliptic_curve::rand_core::{CryptoRng, Error as NoNoise, RngCore};
use p256::pkcs8::EncodePublicKey;
use sha2::{Digest, Sha256};

use crate::pairing::{from_hex, to_hex};

/// Where this Mac keeps the private key it signs with toward each other Mac, by that Mac's id.
/// Blocking: the Keychain's is a `security` process.
pub trait MacKeys: Send + Sync + 'static {
    /// `key` is the hex of the raw 32-byte P-256 scalar.
    fn keep(&self, account: &str, key: Secret<String>) -> Result<(), String>;
    fn read(&self, account: &str) -> Result<Option<Secret<String>>, String>;
    fn forget(&self, account: &str) -> Result<(), String>;
}

/// Keys in memory, for tests and for nothing else.
#[derive(Default)]
pub struct MemoryMacKeys {
    kept: Mutex<HashMap<String, String>>,
}

impl MacKeys for MemoryMacKeys {
    fn keep(&self, account: &str, key: Secret<String>) -> Result<(), String> {
        self.kept.lock().unwrap().insert(account.to_string(), key.expose().clone());
        Ok(())
    }

    fn read(&self, account: &str) -> Result<Option<Secret<String>>, String> {
        Ok(self.kept.lock().unwrap().get(account).cloned().map(Secret::new))
    }

    fn forget(&self, account: &str) -> Result<(), String> {
        self.kept.lock().unwrap().remove(account);
        Ok(())
    }
}

/// A fresh P-256 key from the system's random numbers.
pub(crate) fn fresh_key() -> Result<SigningKey, String> {
    // A scalar of zero or past the order is refused; the odds are about 2^-128 a draw.
    loop {
        if let Ok(key) = SigningKey::from_slice(&crate::webpush::random::<32>()?) {
            return Ok(key);
        }
    }
}

pub(crate) fn kept_form(key: &SigningKey) -> Secret<String> {
    Secret::new(to_hex(&key.to_bytes()))
}

pub(crate) fn from_kept(kept: &Secret<String>) -> Option<SigningKey> {
    SigningKey::from_slice(&from_hex(kept.expose())?).ok()
}

/// The SPKI DER the other Mac verifies with, as hex.
pub fn spki_hex(key: &SigningKey) -> Result<String, String> {
    let der = key.verifying_key().to_public_key_der().map_err(|why| format!("This Mac's key could not be written out: {why}"))?;
    Ok(to_hex(der.as_bytes()))
}

/// Extra noise for a hedged signature. The nonce is RFC 6979's either way; the noise only makes two
/// signatures of the same request differ, since the Gateway refuses one it has seen.
struct Noise {
    bytes: [u8; 32],
    at: usize,
}

impl RngCore for Noise {
    fn next_u32(&mut self) -> u32 {
        let mut four = [0u8; 4];
        self.fill_bytes(&mut four);
        u32::from_le_bytes(four)
    }

    fn next_u64(&mut self) -> u64 {
        let mut eight = [0u8; 8];
        self.fill_bytes(&mut eight);
        u64::from_le_bytes(eight)
    }

    fn fill_bytes(&mut self, out: &mut [u8]) {
        for byte in out {
            *byte = self.bytes[self.at % self.bytes.len()];
            self.at += 1;
        }
    }

    fn try_fill_bytes(&mut self, out: &mut [u8]) -> Result<(), NoNoise> {
        self.fill_bytes(out);
        Ok(())
    }
}

impl CryptoRng for Noise {}

/// The three headers `signing::signed` reads, for one request from this Mac, or from a phone in a test.
pub fn sign(
    key: &SigningKey,
    device: &str,
    method: &str,
    target: &str,
    time: i64,
    body: &[u8],
) -> Result<[(&'static str, String); 3], String> {
    let message = format!("{method}{target}{time}{}", to_hex(&Sha256::digest(body)));
    let mut noise = Noise { bytes: crate::webpush::random::<32>()?, at: 0 };
    let signature: Signature =
        key.try_sign_with_rng(&mut noise, message.as_bytes()).map_err(|why| format!("This Mac could not sign: {why}"))?;
    Ok([
        ("x-pocket-device", device.to_string()),
        ("x-pocket-time", time.to_string()),
        ("x-pocket-signature", to_hex(&signature.to_bytes())),
    ])
}

/// The keys read this run, so a call does not start a `security` process each time.
#[derive(Clone, Default)]
pub(crate) struct Held(Arc<Mutex<HashMap<String, SigningKey>>>);

impl Held {
    pub(crate) fn get(&self, account: &str) -> Option<SigningKey> {
        self.0.lock().unwrap().get(account).cloned()
    }

    pub(crate) fn put(&self, account: &str, key: SigningKey) {
        self.0.lock().unwrap().insert(account.to_string(), key);
    }

    pub(crate) fn drop_key(&self, account: &str) {
        self.0.lock().unwrap().remove(account);
    }
}
