//! VAPID (RFC 8292): the key pair that says to a push service which server is
//! sending, generated once and kept in Armada's machine directory.

use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;

use base64::engine::general_purpose::URL_SAFE_NO_PAD as B64;
use base64::Engine;
use p256::ecdsa::signature::Signer;
use p256::ecdsa::{Signature, SigningKey};
use serde::Serialize;

use crate::webpush::random;

/// The file in the machine directory holding the private key.
pub const KEY_FILE: &str = "pocket-vapid.key";

/// Twelve hours: inside RFC 8292's 24, with room for a skewed clock.
const LIFETIME: i64 = 12 * 60 * 60;

#[derive(Clone)]
pub struct Vapid {
    key: SigningKey,
}

#[derive(Serialize)]
struct Claims<'a> {
    aud: &'a str,
    exp: i64,
    sub: &'a str,
}

impl Vapid {
    pub fn from_secret(secret: &[u8]) -> Result<Vapid, String> {
        SigningKey::from_slice(secret)
            .map(|key| Vapid { key })
            .map_err(|_| "The VAPID key file does not hold a P-256 key.".to_string())
    }

    /// The key in `dir`, made on first use. The file is owner-only from the
    /// moment it exists.
    pub fn open(dir: &Path) -> Result<Vapid, String> {
        let file = dir.join(KEY_FILE);
        let read = |why: std::io::Error| format!("The VAPID key at {} could not be read: {why}", file.display());
        match std::fs::read_to_string(&file) {
            Ok(text) => {
                let secret = B64.decode(text.trim()).map_err(|_| "The VAPID key file is not base64url.".to_string())?;
                Vapid::from_secret(&secret)
            }
            Err(why) if why.kind() == std::io::ErrorKind::NotFound => {
                let secret = loop {
                    let secret = random::<32>()?;
                    if SigningKey::from_slice(&secret).is_ok() {
                        break secret;
                    }
                };
                std::fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .mode(0o600)
                    .open(&file)
                    .and_then(|mut f| f.write_all(B64.encode(secret).as_bytes()))
                    .map_err(|why| format!("The VAPID key at {} could not be written: {why}", file.display()))?;
                Vapid::from_secret(&secret)
            }
            Err(why) => Err(read(why)),
        }
    }

    /// The public key as the browser's `applicationServerKey` wants it: the
    /// uncompressed point, base64url.
    pub fn public_key(&self) -> String {
        B64.encode(self.key.verifying_key().to_encoded_point(false).as_bytes())
    }

    /// The `Authorization` header value for a push to `audience`, the endpoint's origin.
    pub fn authorization(&self, audience: &str, subject: &str, now: i64) -> String {
        let claims = Claims { aud: audience, exp: now + LIFETIME, sub: subject };
        let head = B64.encode(br#"{"typ":"JWT","alg":"ES256"}"#);
        let body = B64.encode(ipc::encode(&claims).unwrap_or_default());
        let signed = format!("{head}.{body}");
        let signature: Signature = self.key.sign(signed.as_bytes());
        format!("vapid t={signed}.{}, k={}", B64.encode(signature.to_bytes()), self.public_key())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use p256::ecdsa::signature::Verifier;
    use p256::ecdsa::VerifyingKey;

    #[derive(serde::Deserialize)]
    struct Back {
        aud: String,
        exp: i64,
        sub: String,
    }

    #[test]
    fn the_token_verifies_with_the_public_key_it_carries() {
        let vapid = Vapid::from_secret(&[9u8; 32]).unwrap();
        let header = vapid.authorization("https://push.example.net", "mailto:owner@example.test", 1000);
        let rest = header.strip_prefix("vapid t=").unwrap();
        let (token, k) = rest.split_once(", k=").unwrap();
        let (signed, signature) = token.rsplit_once('.').unwrap();
        let key = VerifyingKey::from_sec1_bytes(&B64.decode(k).unwrap()).unwrap();
        let signature = Signature::from_slice(&B64.decode(signature).unwrap()).unwrap();
        key.verify(signed.as_bytes(), &signature).unwrap();
        let (head, body) = signed.split_once('.').unwrap();
        assert_eq!(B64.decode(head).unwrap(), br#"{"typ":"JWT","alg":"ES256"}"#);
        let claims: Back = ipc::decode("claims", &B64.decode(body).unwrap()).unwrap();
        assert_eq!(claims.aud, "https://push.example.net");
        assert_eq!(claims.sub, "mailto:owner@example.test");
        assert!(claims.exp <= 1000 + 24 * 60 * 60);
        assert_eq!(k, vapid.public_key());
    }

    #[test]
    fn the_key_is_made_once_and_owner_only() {
        use std::os::unix::fs::PermissionsExt;
        let dir = std::env::temp_dir().join(format!("pocket-vapid-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let first = Vapid::open(&dir).unwrap().public_key();
        assert_eq!(Vapid::open(&dir).unwrap().public_key(), first);
        let mode = std::fs::metadata(dir.join(KEY_FILE)).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600);
        std::fs::remove_dir_all(&dir).ok();
    }
}
