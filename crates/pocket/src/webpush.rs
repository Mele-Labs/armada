//! Web Push: RFC 8291 payload encryption (aes128gcm), RFC 8292 VAPID, and the
//! POST to the phone's push service.

use std::time::Duration;

use aes_gcm::aead::Aead;
use aes_gcm::{Aes128Gcm, KeyInit, Nonce};
use base64::engine::general_purpose::URL_SAFE_NO_PAD as B64;
use base64::Engine;
use hkdf::Hkdf;
use p256::ecdh::diffie_hellman;
use p256::{PublicKey, SecretKey};
use sha2::Sha256;
use store::PushSubscription;

use crate::vapid::Vapid;

/// The record size written into the header. One record holds the whole message.
const RECORD: u32 = 4096;
/// A push is worth nothing a day later: the push service drops it after this.
const TTL_SECONDS: u32 = 24 * 60 * 60;

pub(crate) fn random<const N: usize>() -> Result<[u8; N], String> {
    use std::io::Read;
    let mut bytes = [0u8; N];
    std::fs::File::open("/dev/urandom")
        .and_then(|mut f| f.read_exact(&mut bytes))
        .map_err(|why| format!("The system's random numbers could not be read: {why}"))?;
    Ok(bytes)
}

/// RFC 8291 section 3.4 with the sender's key and salt given, so the RFC's own
/// example can be run through it.
pub(crate) fn encrypt_with(
    plaintext: &[u8],
    ua_public: &[u8],
    auth: &[u8],
    sender: &SecretKey,
    salt: [u8; 16],
) -> Result<Vec<u8>, String> {
    let ua = PublicKey::from_sec1_bytes(ua_public).map_err(|_| "The phone's key is not a P-256 point.".to_string())?;
    let as_public = sender.public_key().to_sec1_bytes();
    let shared = diffie_hellman(sender.to_nonzero_scalar(), ua.as_affine());
    let mut key_info = b"WebPush: info\0".to_vec();
    key_info.extend_from_slice(ua_public);
    key_info.extend_from_slice(&as_public);
    let mut ikm = [0u8; 32];
    Hkdf::<Sha256>::new(Some(auth), shared.raw_secret_bytes())
        .expand(&key_info, &mut ikm)
        .map_err(|_| "key derivation failed")?;
    let prk = Hkdf::<Sha256>::new(Some(&salt), &ikm);
    let (mut cek, mut nonce) = ([0u8; 16], [0u8; 12]);
    prk.expand(b"Content-Encoding: aes128gcm\0", &mut cek).map_err(|_| "key derivation failed")?;
    prk.expand(b"Content-Encoding: nonce\0", &mut nonce).map_err(|_| "key derivation failed")?;
    let mut record = plaintext.to_vec();
    record.push(0x02);
    let sealed = Aes128Gcm::new(&cek.into())
        .encrypt(Nonce::from_slice(&nonce), record.as_slice())
        .map_err(|_| "encryption failed")?;
    let mut body = salt.to_vec();
    body.extend_from_slice(&RECORD.to_be_bytes());
    body.push(as_public.len() as u8);
    body.extend_from_slice(&as_public);
    body.extend(sealed);
    Ok(body)
}

/// A message for one subscription, under a fresh key and salt.
pub fn encrypt(plaintext: &[u8], sub: &PushSubscription) -> Result<Vec<u8>, String> {
    let bad = |what: &str| format!("The phone's push subscription has a bad {what}.");
    let ua_public = B64.decode(sub.p256dh.trim_end_matches('=')).map_err(|_| bad("key"))?;
    let auth = B64.decode(sub.auth.trim_end_matches('=')).map_err(|_| bad("secret"))?;
    let sender = loop {
        if let Ok(key) = SecretKey::from_slice(&random::<32>()?) {
            break key;
        }
    };
    encrypt_with(plaintext, &ua_public, &auth, &sender, random::<16>()?)
}

pub enum Delivery {
    Sent,
    /// The push service answered 404 or 410: the subscription is dead.
    Gone,
    /// Anything else: the send is tried again, then given up.
    Failed,
}

/// Origin of an endpoint URL: `https://host[:port]`.
pub(crate) fn origin(endpoint: &str) -> Option<String> {
    let url = reqwest::Url::parse(endpoint).ok()?;
    Some(url.origin().ascii_serialization())
}

pub async fn deliver(
    client: &reqwest::Client,
    vapid: &Vapid,
    subject: &str,
    sub: &PushSubscription,
    payload: &[u8],
    now: i64,
) -> Delivery {
    let Some(audience) = origin(&sub.endpoint) else {
        return Delivery::Failed;
    };
    let body = match encrypt(payload, sub) {
        Ok(body) => body,
        Err(_) => return Delivery::Failed,
    };
    let authorization = vapid.authorization(&audience, subject, now);
    let sent = client
        .post(&sub.endpoint)
        .header("TTL", TTL_SECONDS.to_string())
        .header("Content-Encoding", "aes128gcm")
        .header("Content-Type", "application/octet-stream")
        .header("Urgency", "high")
        .header("Authorization", authorization)
        .timeout(Duration::from_secs(15))
        .body(body)
        .send()
        .await;
    match sent {
        Ok(answer) if answer.status().is_success() => Delivery::Sent,
        Ok(answer) if matches!(answer.status().as_u16(), 404 | 410) => Delivery::Gone,
        Ok(_) | Err(_) => Delivery::Failed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn b64(text: &str) -> Vec<u8> {
        B64.decode(text).unwrap()
    }

    /// RFC 8291 section 5 and Appendix A, byte for byte.
    #[test]
    fn the_rfcs_example_encrypts_to_the_rfcs_body() {
        let sender = SecretKey::from_slice(&b64("yfWPiYE-n46HLnH0KqZOF1fJJU3MYrct3AELtAQ-oRw")).unwrap();
        let salt: [u8; 16] = b64("DGv6ra1nlYgDCS1FRnbzlw").try_into().unwrap();
        let body = encrypt_with(
            b"When I grow up, I want to be a watermelon",
            &b64("BCVxsr7N_eNgVRqvHtD0zTZsEc6-VV-JvLexhqUzORcxaOzi6-AYWXvTBHm4bjyPjs7Vd8pZGH6SRpkNtoIAiw4"),
            &b64("BTBZMqHH6r4Tts7J_aSIgg"),
            &sender,
            salt,
        )
        .unwrap();
        let expected = "DGv6ra1nlYgDCS1FRnbzlwAAEABBBP4z9KsN6nGRTbVYI_c7VJSPQTBtkgcy27ml\
                        mlMoZIIgDll6e3vCYLocInmYWAmS6TlzAC8wEqKK6PBru3jl7A_yl95bQpu6cVPT\
                        pK4Mqgkf1CXztLVBSt2Ks3oZwbuwXPXLWyouBWLVWGNWQexSgSxsj_Qulcy4a-fN";
        assert_eq!(B64.encode(&body), expected);
    }

    #[test]
    fn the_rfcs_intermediate_values_match() {
        let sender = SecretKey::from_slice(&b64("yfWPiYE-n46HLnH0KqZOF1fJJU3MYrct3AELtAQ-oRw")).unwrap();
        let ua_public = b64("BCVxsr7N_eNgVRqvHtD0zTZsEc6-VV-JvLexhqUzORcxaOzi6-AYWXvTBHm4bjyPjs7Vd8pZGH6SRpkNtoIAiw4");
        let ua = PublicKey::from_sec1_bytes(&ua_public).unwrap();
        let shared = diffie_hellman(sender.to_nonzero_scalar(), ua.as_affine());
        assert_eq!(B64.encode(shared.raw_secret_bytes()), "kyrL1jIIOHEzg3sM2ZWRHDRB62YACZhhSlknJ672kSs");
    }

    #[test]
    fn a_subscription_with_a_bad_key_is_refused_by_name() {
        let sub = PushSubscription { device_id: "d".into(), endpoint: "https://x.test/p".into(), p256dh: "!!".into(), auth: "AA".into() };
        assert!(encrypt(b"x", &sub).unwrap_err().contains("bad key"));
    }
}
