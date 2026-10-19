//! What a request to a bucket is signed with: a version 4 signature for S3-shaped stores, and a
//! service account's signed token for the other. Both are plain functions of values, so each is
//! tested against a published vector without a network.
//!
//! **Time is an argument**, as in `store_signing`: a signature that read the clock could be looked
//! at, never reproduced.

use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};

use crate::store_device::to_hex;

type HmacSha256 = Hmac<Sha256>;

/// The keys a request is signed with.
pub(crate) struct Keys {
    pub(crate) access: String,
    pub(crate) secret: adapter_traits::Secret<String>,
    /// Short-lived credentials carry one more value, which the request must send and sign.
    pub(crate) session: Option<String>,
}

fn mac(key: &[u8], text: &str) -> Vec<u8> {
    // An HMAC takes a key of any length, so this cannot be refused.
    let mut hmac = HmacSha256::new_from_slice(key).expect("an HMAC takes any key length");
    hmac.update(text.as_bytes());
    hmac.finalize().into_bytes().to_vec()
}

/// `(year, month, day)` of a day count since 1970-01-01, in the proleptic Gregorian calendar.
fn civil(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let day_of_era = z.rem_euclid(146_097);
    let year_of_era = (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let shifted_month = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * shifted_month + 2) / 5 + 1;
    let month = if shifted_month < 10 { shifted_month + 3 } else { shifted_month - 9 };
    (year_of_era + era * 400 + i64::from(month <= 2), month, day)
}

/// `20130524T000000Z` for the instant, and `20130524` for its day.
pub(crate) fn stamps(epoch_seconds: i64) -> (String, String) {
    let (year, month, day) = civil(epoch_seconds.div_euclid(86_400));
    let in_day = epoch_seconds.rem_euclid(86_400);
    let date = format!("{year:04}{month:02}{day:02}");
    let stamp = format!("{date}T{:02}{:02}{:02}Z", in_day / 3_600, in_day % 3_600 / 60, in_day % 60);
    (stamp, date)
}

/// A path with each segment escaped and the slashes kept: the canonical form S3 signs, which is
/// not escaped twice.
pub(crate) fn path_of(path: &str) -> String {
    let mut out = String::with_capacity(path.len());
    for byte in path.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' | b'/' => out.push(char::from(byte)),
            other => out.push_str(&format!("%{other:02X}")),
        }
    }
    out
}

/// A request to sign: the host and the escaped path it goes to, the body's hash, and any headers
/// beyond the three that are always sent.
pub(crate) struct Signing<'a> {
    pub(crate) method: &'a str,
    pub(crate) host: &'a str,
    pub(crate) path: &'a str,
    pub(crate) body: &'a [u8],
    pub(crate) extra: &'a [(&'a str, &'a str)],
    pub(crate) region: &'a str,
    pub(crate) epoch_seconds: i64,
}

/// The headers to send with the request, `Authorization` among them. Names are lowercase.
pub(crate) fn sigv4(keys: &Keys, request: &Signing<'_>) -> Vec<(String, String)> {
    let (stamp, date) = stamps(request.epoch_seconds);
    let payload = to_hex(&Sha256::digest(request.body));
    let mut signed: Vec<(String, String)> = vec![
        ("host".to_string(), request.host.to_string()),
        ("x-amz-content-sha256".to_string(), payload.clone()),
        ("x-amz-date".to_string(), stamp.clone()),
    ];
    signed.extend(request.extra.iter().map(|(name, value)| (name.to_ascii_lowercase(), (*value).to_string())));
    if let Some(session) = &keys.session {
        signed.push(("x-amz-security-token".to_string(), session.clone()));
    }
    signed.sort();
    let canonical_headers: String = signed.iter().map(|(name, value)| format!("{name}:{}\n", value.trim())).collect();
    let names = signed.iter().map(|(name, _)| name.as_str()).collect::<Vec<_>>().join(";");
    let canonical = format!("{}\n{}\n\n{canonical_headers}\n{names}\n{payload}", request.method, request.path);
    let scope = format!("{date}/{}/s3/aws4_request", request.region);
    let to_sign = format!("AWS4-HMAC-SHA256\n{stamp}\n{scope}\n{}", to_hex(&Sha256::digest(canonical.as_bytes())));
    let mut key = mac(format!("AWS4{}", keys.secret.expose()).as_bytes(), &date);
    for part in [request.region, "s3", "aws4_request"] {
        key = mac(&key, part);
    }
    let signature = to_hex(&mac(&key, &to_sign));
    let authorization =
        format!("AWS4-HMAC-SHA256 Credential={}/{scope}, SignedHeaders={names}, Signature={signature}", keys.access);
    let mut headers = signed;
    headers.retain(|(name, _)| name != "host");
    headers.push(("authorization".to_string(), authorization));
    headers
}

/// A token request's assertion, `header.claims.signature` in base64url, signed RS256 with the
/// service account's private key (PKCS#8, as the key file writes it between its PEM lines).
pub(crate) fn assertion(
    account: &str,
    audience: &str,
    scope: &str,
    private_key_pem: &adapter_traits::Secret<String>,
    epoch_seconds: i64,
) -> Result<String, String> {
    use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
    use base64::Engine;

    #[derive(serde::Serialize)]
    struct Header {
        alg: &'static str,
        typ: &'static str,
    }
    #[derive(serde::Serialize)]
    struct Claims<'a> {
        iss: &'a str,
        scope: &'a str,
        aud: &'a str,
        iat: i64,
        exp: i64,
    }
    let claims = Claims { iss: account, scope, aud: audience, iat: epoch_seconds, exp: epoch_seconds + 3_600 };
    let header = ipc::encode(&Header { alg: "RS256", typ: "JWT" }).map_err(|why| why.to_string())?;
    let claims = ipc::encode(&claims).map_err(|why| why.to_string())?;
    let signing_input = format!("{}.{}", URL_SAFE_NO_PAD.encode(header), URL_SAFE_NO_PAD.encode(claims));

    let body: String = private_key_pem
        .expose()
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with("-----"))
        .collect();
    let der = STANDARD.decode(body).map_err(|_| "the service account's private key is not PEM".to_string())?;
    let pair = ring::signature::RsaKeyPair::from_pkcs8(&der)
        .map_err(|_| "the service account's private key is not an RSA key in PKCS#8".to_string())?;
    let mut signature = vec![0u8; pair.public().modulus_len()];
    pair.sign(&ring::signature::RSA_PKCS1_SHA256, &ring::rand::SystemRandom::new(), signing_input.as_bytes(), &mut signature)
        .map_err(|_| "the service account's key could not sign".to_string())?;
    Ok(format!("{signing_input}.{}", URL_SAFE_NO_PAD.encode(signature)))
}
