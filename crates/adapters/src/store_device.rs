//! This machine's identity with the central store: a P-256 key made once, and
//! the device id the store gave back when the key was enrolled.
//!
//! The private key is a [`Secret`], so nothing holding a [`DeviceKey`] can print
//! it, and the file is owner-only from the moment it exists. `pocket`'s
//! `vapid.rs` is the pattern.

use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;

use adapter_traits::Secret;
use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use p256::ecdsa::signature::Signer;
use p256::ecdsa::{Signature, SigningKey};

/// The file in the machine directory holding the private key, in hex.
pub const KEY_FILE: &str = "store-device.key";
/// The file beside it holding the device id the store returned. Not a secret.
pub const ID_FILE: &str = "store-device.id";

/// The longest device id kept, so a header built from it stays a header.
const LONGEST_ID: usize = 64;

pub struct DeviceKey {
    key: Secret<SigningKey>,
}

pub(crate) fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn from_hex(text: &str) -> Option<Vec<u8>> {
    if text.len() % 2 != 0 || !text.is_ascii() {
        return None;
    }
    (0..text.len())
        .step_by(2)
        .map(|at| u8::from_str_radix(&text[at..at + 2], 16).ok())
        .collect()
}

pub(crate) fn random<const N: usize>() -> Result<[u8; N], String> {
    use std::io::Read;
    let mut bytes = [0u8; N];
    std::fs::File::open("/dev/urandom")
        .and_then(|mut f| f.read_exact(&mut bytes))
        .map_err(|why| format!("The system's random numbers could not be read: {why}"))?;
    Ok(bytes)
}

impl DeviceKey {
    pub fn from_secret(secret: &[u8]) -> Result<DeviceKey, String> {
        SigningKey::from_slice(secret)
            .map(|key| DeviceKey { key: Secret::new(key) })
            .map_err(|_| "The store key file does not hold a P-256 key.".to_string())
    }

    /// The key in `dir`, made on first use. `create_new` so two Fleets starting
    /// together cannot each write one and keep different keys.
    pub fn open(dir: &Path) -> Result<DeviceKey, String> {
        let file = dir.join(KEY_FILE);
        match std::fs::read_to_string(&file) {
            Ok(text) => {
                let secret = from_hex(text.trim()).ok_or("The store key file is not hex.")?;
                DeviceKey::from_secret(&secret)
            }
            Err(why) if why.kind() == std::io::ErrorKind::NotFound => {
                let secret = loop {
                    let secret = random::<32>()?;
                    if SigningKey::from_slice(&secret).is_ok() {
                        break secret;
                    }
                };
                match std::fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .mode(0o600)
                    .open(&file)
                    .and_then(|mut f| f.write_all(to_hex(&secret).as_bytes()))
                {
                    Ok(()) => DeviceKey::from_secret(&secret),
                    // Another Fleet made it between the read and the create.
                    Err(why) if why.kind() == std::io::ErrorKind::AlreadyExists => DeviceKey::open(dir),
                    Err(why) => Err(format!("The store key at {} could not be written: {why}", file.display())),
                }
            }
            Err(why) => Err(format!("The store key at {} could not be read: {why}", file.display())),
        }
    }

    /// The public key as the store registers it: base64 of its SPKI encoding.
    pub fn public_key(&self) -> String {
        use p256::pkcs8::EncodePublicKey;
        let der = self.key.expose().verifying_key().to_public_key_der();
        // Encoding a valid point cannot fail; an empty key is refused by the store.
        B64.encode(der.as_ref().map_or(&[][..], |der| der.as_bytes()))
    }

    /// ECDSA P-256 over SHA-256 of `message`, as hex of the raw r||s.
    pub(crate) fn sign(&self, message: &[u8]) -> String {
        let signature: Signature = self.key.expose().sign(message);
        to_hex(&signature.to_bytes())
    }
}

/// The device id the store gave this machine, once it has been enrolled.
pub fn device_id(dir: &Path) -> Result<Option<String>, String> {
    let file = dir.join(ID_FILE);
    match std::fs::read_to_string(&file) {
        Ok(text) => {
            let id = text.trim();
            sound_id(id)
                .then(|| Some(id.to_string()))
                .ok_or_else(|| format!("The device id at {} is not one the store would give.", file.display()))
        }
        Err(why) if why.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(why) => Err(format!("The device id at {} could not be read: {why}", file.display())),
    }
}

/// Keep the device id the store returned. Refuses one that could not sit in a header.
pub fn remember_device(dir: &Path, id: &str) -> Result<(), String> {
    if !sound_id(id) {
        return Err("The store returned a device id this machine will not send back.".to_string());
    }
    std::fs::write(dir.join(ID_FILE), id).map_err(|why| format!("The device id could not be kept: {why}"))
}

/// Letters, digits, `.`, `-` and `_`: what survives a header and a path
/// unchanged, and the store's own device id, an account id, a dot and hex.
pub(crate) fn sound_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= LONGEST_ID
        && id.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'_'))
}
