//! A bucket over HTTP: [`ObjectBucket`] makes each [`Bucket`] call one signed request to an
//! S3-shaped store, `s3://`, `r2://` or `gs://`, and reads the one answer.
//!
//! # Signed by hand, not by an SDK
//!
//! Two calls on one object do not need a provider's whole client. Version 4 signing is an HMAC
//! chain, the other provider takes a bearer token that one signed request buys, and `reqwest`,
//! `sha2`, `hmac`, `p256` and `ring` are already in the build, so the cost is those few
//! functions in `bucket_signing` and a thousand fewer lines to compile than an SDK.
//!
//! **A blocking client, as [`HttpStore`](crate::HttpStore) is**: build it and call it on a
//! blocking thread.
//!
//! **A 5xx is "unreachable"**, a 4xx is "refused", and no credentials is its own answer, which
//! sent nothing.
//!
//! **A missing object reads as 404 only where the key may list the bucket**; the provider answers
//! 403 to a key that may not, and that is refused with the reason spelled out.

use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use adapter_traits::{Bucket, BucketError, Secret};
use reqwest::blocking::Client;
use reqwest::redirect::Policy;
use serde::Deserialize;

use crate::bucket_credentials::{self as credentials, Environment, Google};
use crate::bucket_signing::{self as signing, Keys, Signing};

/// The most of a refusal's own words that is kept.
const SAID_AT_MOST: usize = 200;
const GOOGLE_STORAGE: &str = "https://storage.googleapis.com";
const GOOGLE_SCOPE: &str = "https://www.googleapis.com/auth/devstorage.read_write";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Provider {
    S3,
    R2,
    Gcs,
}

/// Where a bucket is: `s3://bucket/prefix`, `r2://bucket/prefix` or `gs://bucket/prefix`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BucketAddress {
    provider: Provider,
    bucket: String,
    prefix: String,
}

impl BucketAddress {
    /// The address, or the sentence that says what is wrong with it. The prefix is optional and
    /// loses its slashes at both ends.
    pub fn parse(text: &str) -> Result<BucketAddress, String> {
        let text = text.trim();
        let (provider, rest) = match text.split_once("://") {
            Some(("s3", rest)) => (Provider::S3, rest),
            Some(("r2", rest)) => (Provider::R2, rest),
            Some(("gs", rest)) => (Provider::Gcs, rest),
            _ => return Err(format!("{text:?} is not an s3://, r2:// or gs:// address")),
        };
        let (bucket, prefix) = rest.split_once('/').unwrap_or((rest, ""));
        let sound = (3..=63).contains(&bucket.len())
            && bucket.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'-' | b'.' | b'_'));
        if !sound {
            return Err(format!("{bucket:?} is not a bucket name"));
        }
        let prefix = prefix.trim_matches('/');
        if prefix.chars().any(|c| c.is_control() || matches!(c, '?' | '#' | '\\')) || prefix.split('/').any(|part| part == ".." || part == ".") {
            return Err(format!("{prefix:?} is not a prefix a bucket can hold"));
        }
        Ok(BucketAddress { provider, bucket: bucket.to_string(), prefix: prefix.to_string() })
    }

    /// The object called `name` under the prefix.
    pub(crate) fn key(&self, name: &str) -> String {
        match self.prefix.is_empty() {
            true => name.to_string(),
            false => format!("{}/{name}", self.prefix),
        }
    }
}

enum Auth {
    Version4 { keys: Keys, region: String },
    Bearer(Secret<String>),
}

/// One bucket, signed in as this machine's own credentials. No `Debug`: it holds a secret.
pub struct ObjectBucket {
    address: BucketAddress,
    /// `scheme://host[:port]`, without a path.
    base: String,
    auth: Auth,
    client: Client,
    now: fn() -> i64,
}

fn unix_now() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |since| i64::try_from(since.as_secs()).unwrap_or(i64::MAX))
}

fn client(timeout: Duration) -> Result<Client, BucketError> {
    // A redirect would send a signature to a host it was not made for.
    Client::builder()
        .timeout(timeout)
        .connect_timeout(timeout)
        .redirect(Policy::none())
        .build()
        .map_err(|why| BucketError::Refused(format!("The bucket's client could not be made: {why}")))
}

fn said(status: u16, body: &[u8]) -> String {
    let words = String::from_utf8_lossy(body);
    let words: String = words.split_whitespace().collect::<Vec<_>>().join(" ").chars().take(SAID_AT_MOST).collect();
    match words.is_empty() {
        true => format!("status {status}"),
        false => format!("status {status}: {words}"),
    }
}

fn answered(status: u16, body: &[u8]) -> BucketError {
    match status {
        500.. => BucketError::Unreachable(said(status, body)),
        403 => BucketError::Refused(format!(
            "{} (a missing object also reads as 403 unless the key may list the bucket)",
            said(status, body)
        )),
        _ => BucketError::Refused(said(status, body)),
    }
}

#[derive(Deserialize)]
struct Token {
    access_token: String,
}

fn token(client: &Client, google: &Google, now: i64) -> Result<Secret<String>, BucketError> {
    let (uri, form): (&str, Vec<(&str, String)>) = match google {
        Google::Account { email, key, token_uri } => {
            let assertion = signing::assertion(email, token_uri, GOOGLE_SCOPE, key, now).map_err(BucketError::Refused)?;
            (token_uri, vec![("grant_type", "urn:ietf:params:oauth:grant-type:jwt-bearer".to_string()), ("assertion", assertion)])
        }
        Google::Person { client_id, client_secret, refresh_token, token_uri } => (
            token_uri,
            vec![
                ("grant_type", "refresh_token".to_string()),
                ("client_id", client_id.clone()),
                ("client_secret", client_secret.expose().clone()),
                ("refresh_token", refresh_token.expose().clone()),
            ],
        ),
    };
    let response = client
        .post(uri)
        .form(&form)
        .send()
        .map_err(|why| BucketError::Unreachable(why.without_url().to_string()))?;
    let status = response.status().as_u16();
    let body = response.bytes().map_err(|why| BucketError::Unreachable(why.without_url().to_string()))?;
    if !(200..300).contains(&status) {
        return Err(answered(status, &body));
    }
    let token: Token = ipc::decode("Google token", &body).map_err(|why| BucketError::Refused(why.to_string()))?;
    Ok(Secret::new(token.access_token))
}

impl ObjectBucket {
    /// The bucket at `address`, signed in with this machine's own environment and credential
    /// files. `Unsigned` where it has none.
    pub fn open(address: &BucketAddress, timeout: Duration) -> Result<ObjectBucket, BucketError> {
        let var = |name: &str| std::env::var(name).ok();
        let home = std::env::var_os("HOME").map(PathBuf::from);
        ObjectBucket::open_in(address, &Environment { var: &var, home }, timeout, unix_now)
    }

    pub(crate) fn open_in(
        address: &BucketAddress,
        env: &Environment<'_>,
        timeout: Duration,
        now: fn() -> i64,
    ) -> Result<ObjectBucket, BucketError> {
        let client = client(timeout)?;
        let (base, auth) = match address.provider {
            Provider::S3 => {
                let (keys, region, endpoint) = credentials::s3(env).map_err(BucketError::Unsigned)?;
                let base = match endpoint {
                    Some(endpoint) => endpoint,
                    None if address.bucket.contains('.') => format!("https://s3.{region}.amazonaws.com"),
                    None => format!("https://{}.s3.{region}.amazonaws.com", address.bucket),
                };
                (base, Auth::Version4 { keys, region })
            }
            Provider::R2 => {
                let (keys, endpoint) = credentials::r2(env).map_err(BucketError::Unsigned)?;
                (endpoint, Auth::Version4 { keys, region: "auto".to_string() })
            }
            Provider::Gcs => {
                let google = credentials::google(env).map_err(BucketError::Unsigned)?;
                let base = match env.set("STORAGE_EMULATOR_HOST") {
                    Some(host) if host.contains("://") => host,
                    Some(host) => format!("http://{host}"),
                    None => GOOGLE_STORAGE.to_string(),
                };
                (base, Auth::Bearer(token(&client, &google, now())?))
            }
        };
        Ok(ObjectBucket { address: address.clone(), base: base.trim_end_matches('/').to_string(), auth, client, now })
    }

    /// Whether the endpoint already names the bucket, as `https://bucket.s3.region.amazonaws.com`
    /// does, so the path is the key alone.
    fn names_the_bucket(&self) -> bool {
        self.base.contains(&format!("//{}.s3.", self.address.bucket))
    }

    fn send(&self, method: &'static str, name: &str, body: &[u8]) -> Result<(u16, Vec<u8>), BucketError> {
        let key = self.address.key(name);
        let path = match self.names_the_bucket() {
            true => signing::path_of(&format!("/{key}")),
            false => signing::path_of(&format!("/{}/{key}", self.address.bucket)),
        };
        let host = self.base.split_once("://").map_or(self.base.as_str(), |(_, rest)| rest);
        let reqwest_method = reqwest::Method::from_bytes(method.as_bytes()).map_err(|why| BucketError::Refused(why.to_string()))?;
        let mut request = self.client.request(reqwest_method, format!("{}{path}", self.base));
        match &self.auth {
            Auth::Version4 { keys, region } => {
                let headers = signing::sigv4(
                    keys,
                    &Signing { method, host, path: &path, body, extra: &[], region, epoch_seconds: (self.now)() },
                );
                for (name, value) in headers {
                    request = request.header(name, value);
                }
            }
            Auth::Bearer(token) => request = request.bearer_auth(token.expose()),
        }
        if method == "PUT" {
            request = request.body(body.to_vec());
        }
        let response = request.send().map_err(|why| BucketError::Unreachable(why.without_url().to_string()))?;
        let status = response.status().as_u16();
        let body = response.bytes().map_err(|why| BucketError::Unreachable(why.without_url().to_string()))?;
        Ok((status, body.to_vec()))
    }
}

impl Bucket for ObjectBucket {
    fn get(&self, name: &str) -> Result<Option<Vec<u8>>, BucketError> {
        match self.send("GET", name, b"")? {
            (200, body) => Ok(Some(body)),
            (404, _) => Ok(None),
            (status, body) => Err(answered(status, &body)),
        }
    }

    fn put(&self, name: &str, body: &[u8]) -> Result<(), BucketError> {
        match self.send("PUT", name, body)? {
            (200..=299, _) => Ok(()),
            (status, answer) => Err(answered(status, &answer)),
        }
    }
}
