//! Signing in to GitHub with the device flow: [`ForgeSignIn`] makes each
//! [`SignIn`] call one request and reads the one answer. `docs/concepts/account.md`,
//! *Sign-in*.
//!
//! Building a request and reading a reply are plain functions of values, apart
//! from sending, so the wire is tested without a network and the send against a
//! socket on this machine. The address of each endpoint is a field, which is
//! what lets that socket stand in for GitHub.
//!
//! **No client secret exists in the device flow**, so none is read, held or sent.
//! The access token is a [`Secret`] from the reply to the one call that uses it,
//! and the `Authorization` header it rides in is marked sensitive.
//!
//! **No scope is asked for.** An unscoped token reads the account's public
//! profile, which is all `who` needs, and a sign-in that wants more asks for it
//! when the thing that wants it is built.
//!
//! **Blocking**, like [`HttpStore`](crate::HttpStore): every call is made on a
//! thread that may block, never an async worker.

use std::time::Duration;

use adapter_traits::{DeviceCode, Polled, Secret, SignIn, SignInError, SignedInAs};
use reqwest::blocking::Client;
use reqwest::header::{HeaderValue, ACCEPT, AUTHORIZATION, USER_AGENT};
use reqwest::redirect::Policy;
use serde::Deserialize;

const DEVICE_URL: &str = "https://github.com/login/device/code";
const TOKEN_URL: &str = "https://github.com/login/oauth/access_token";
const USER_URL: &str = "https://api.github.com/user";
const GRANT: &str = "urn:ietf:params:oauth:grant-type:device_code";

/// What the forge uses where a reply names no interval.
const DEFAULT_INTERVAL: u64 = 5;
/// And no expiry: the forge's own default for a device code.
const DEFAULT_EXPIRY: u64 = 900;
/// The longest a refusal's own words are kept.
const SAID_AT_MOST: usize = 200;

/// A reply, before it is read.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Reply {
    pub(crate) status: u16,
    pub(crate) body: Vec<u8>,
}

/// An error answer, which the forge sends with a 200 as often as not.
#[derive(Deserialize)]
struct Failure {
    error: Option<String>,
    error_description: Option<String>,
}

#[derive(Deserialize)]
struct DeviceReply {
    device_code: String,
    user_code: String,
    verification_uri: String,
    expires_in: Option<u64>,
    interval: Option<u64>,
}

#[derive(Deserialize)]
struct TokenReply {
    access_token: Option<String>,
}

#[derive(Deserialize)]
struct UserReply {
    id: u64,
    login: String,
}

pub(crate) fn begin_form(client_id: &str) -> Vec<(&'static str, String)> {
    vec![("client_id", client_id.to_string())]
}

pub(crate) fn poll_form(client_id: &str, device_code: &Secret<String>) -> Vec<(&'static str, String)> {
    vec![
        ("client_id", client_id.to_string()),
        ("device_code", device_code.expose().clone()),
        ("grant_type", GRANT.to_string()),
    ]
}

fn said(reply: &Reply) -> String {
    let words = String::from_utf8_lossy(&reply.body);
    let words: String = words.trim().chars().take(SAID_AT_MOST).collect();
    if words.is_empty() {
        format!("status {}", reply.status)
    } else {
        format!("status {}: {words}", reply.status)
    }
}

/// The error for a reply that was not the answer asked for: a 5xx says nothing
/// about the request, so it is unreachable; the rest is the forge saying no.
fn not_answered(reply: &Reply) -> SignInError {
    if reply.status >= 500 {
        SignInError::Unreachable(said(reply))
    } else {
        SignInError::Refused(said(reply))
    }
}

/// The words an error answer carries, if the body is one. **Never the body
/// itself**: an answer that is not an error may hold the token.
fn failure_of(reply: &Reply) -> Option<(String, String)> {
    let failure = ipc::decode::<Failure>("a sign-in refusal", &reply.body).ok()?;
    let code = failure.error?;
    let words = failure.error_description.filter(|words| !words.is_empty()).unwrap_or_else(|| code.clone());
    Some((code, words.chars().take(SAID_AT_MOST).collect()))
}

pub(crate) fn device_code_of(reply: &Reply) -> Result<DeviceCode, SignInError> {
    if reply.status >= 500 {
        return Err(not_answered(reply));
    }
    if let Some((_, words)) = failure_of(reply) {
        return Err(SignInError::Refused(words));
    }
    if reply.status != 200 {
        return Err(not_answered(reply));
    }
    let code: DeviceReply =
        ipc::decode("a device code", &reply.body).map_err(|why| SignInError::Refused(why.to_string()))?;
    // Bridge opens this address for the person, so it is held to a secure page.
    if !code.verification_uri.starts_with("https://") {
        return Err(SignInError::Refused("The forge gave a page to sign in at that is not https.".to_string()));
    }
    if code.user_code.is_empty() || code.device_code.is_empty() {
        return Err(SignInError::Refused("The forge gave no code to sign in with.".to_string()));
    }
    Ok(DeviceCode {
        user_code: code.user_code,
        verification_uri: code.verification_uri,
        device_code: Secret::new(code.device_code),
        // A zero would be a poll with nothing between.
        interval_seconds: code.interval.unwrap_or(DEFAULT_INTERVAL).max(1),
        expires_in_seconds: code.expires_in.unwrap_or(DEFAULT_EXPIRY),
    })
}

pub(crate) fn polled_of(reply: &Reply) -> Result<Polled, SignInError> {
    if reply.status >= 500 {
        return Err(not_answered(reply));
    }
    if let Some((code, words)) = failure_of(reply) {
        return match code.as_str() {
            "authorization_pending" => Ok(Polled::Pending),
            "slow_down" => Ok(Polled::SlowDown),
            "expired_token" => Ok(Polled::Expired),
            "access_denied" => Ok(Polled::Denied),
            _ => Err(SignInError::Refused(words)),
        };
    }
    if reply.status != 200 {
        return Err(not_answered(reply));
    }
    let token: TokenReply =
        ipc::decode("an access token", &reply.body).map_err(|why| SignInError::Refused(why.to_string()))?;
    match token.access_token.filter(|token| !token.is_empty()) {
        Some(token) => Ok(Polled::Granted(Secret::new(token))),
        None => Err(SignInError::Refused("The forge answered with no token.".to_string())),
    }
}

pub(crate) fn account_of(reply: &Reply) -> Result<SignedInAs, SignInError> {
    match reply.status {
        200 => {
            let user: UserReply =
                ipc::decode("an account", &reply.body).map_err(|why| SignInError::Refused(why.to_string()))?;
            Ok(SignedInAs { id: user.id, login: user.login })
        }
        401 | 403 => Err(SignInError::Refused("The forge did not accept the token.".to_string())),
        _ => Err(not_answered(reply)),
    }
}

/// The forge's device flow over HTTPS.
///
/// **A client is made for each call and dropped with it**, because `reqwest`'s
/// blocking client panics if made or dropped on an async worker, and this is
/// held by Fleet for as long as it runs. Every call is on a thread that may block.
pub struct ForgeSignIn {
    timeout: Duration,
    device_url: String,
    token_url: String,
    user_url: String,
}

impl ForgeSignIn {
    pub fn new(timeout: Duration) -> ForgeSignIn {
        ForgeSignIn::against(DEVICE_URL, TOKEN_URL, USER_URL, timeout)
    }

    pub(crate) fn against(device: &str, token: &str, user: &str, timeout: Duration) -> ForgeSignIn {
        ForgeSignIn { timeout, device_url: device.to_string(), token_url: token.to_string(), user_url: user.to_string() }
    }

    fn client(&self) -> Result<Client, SignInError> {
        Client::builder()
            .timeout(self.timeout)
            .connect_timeout(self.timeout)
            // A redirect would carry the token to a host it was not meant for.
            .redirect(Policy::none())
            .build()
            .map_err(|why| SignInError::Refused(format!("The sign-in client could not be made: {why}")))
    }

    fn send(
        &self,
        build: impl FnOnce(&Client) -> reqwest::blocking::RequestBuilder,
    ) -> Result<Reply, SignInError> {
        let client = self.client()?;
        let response = build(&client)
            .header(ACCEPT, "application/json")
            .header(USER_AGENT, "armada")
            .send()
            // Everything `reqwest` raises is the request not completing; a
            // status comes back as a `Response`. The URL is left off: it is
            // the forge's, and says nothing a person can act on.
            .map_err(|why| SignInError::Unreachable(why.without_url().to_string()))?;
        let status = response.status().as_u16();
        let body = response
            .bytes()
            .map_err(|why| SignInError::Unreachable(why.without_url().to_string()))?
            .to_vec();
        Ok(Reply { status, body })
    }
}

impl SignIn for ForgeSignIn {
    fn begin(&self, client_id: &str) -> Result<DeviceCode, SignInError> {
        let form = begin_form(client_id);
        device_code_of(&self.send(|client| client.post(&self.device_url).form(&form))?)
    }

    fn poll(&self, client_id: &str, device_code: &Secret<String>) -> Result<Polled, SignInError> {
        let form = poll_form(client_id, device_code);
        polled_of(&self.send(|client| client.post(&self.token_url).form(&form))?)
    }

    fn who(&self, token: &Secret<String>) -> Result<SignedInAs, SignInError> {
        let mut bearer = HeaderValue::from_str(&format!("Bearer {}", token.expose()))
            .map_err(|_| SignInError::Refused("The token is not one the forge would have given.".to_string()))?;
        bearer.set_sensitive(true);
        account_of(&self.send(|client| client.get(&self.user_url).header(AUTHORIZATION, bearer))?)
    }
}
