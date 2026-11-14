//! Cloudflare's API over HTTP: [`HttpCloudflare`] puts a project's claims store on the person's own
//! account. `docs/contracts/central-store.md`, *Deploying a store*.
//!
//! Building a request and reading a reply are plain functions of values, as in `store.rs`, so the wire
//! is tested without a network and the send is tested against a socket on this machine.
//!
//! **The token is sent and nothing else.** It goes in a header marked sensitive, and every error a call
//! returns has the token's text replaced, should Cloudflare ever echo it back.

use std::time::Duration;

use adapter_traits::{Cloudflare, CloudflareError, Secret, WorkerUpload};
use reqwest::blocking::Client;
use reqwest::header::{HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use reqwest::redirect::Policy;
use serde::de::IgnoredAny;
use serde::{Deserialize, Serialize};

/// Cloudflare's API, where a deploy calls it.
pub const CLOUDFLARE_API: &str = "https://api.cloudflare.com/client/v4";
/// The date `apps/store/wrangler.jsonc` builds the Worker for.
const COMPATIBILITY_DATE: &str = "2026-10-01";
/// The module's name in the upload, and the metadata's `main_module`.
const MAIN_MODULE: &str = "worker.js";
/// The longest Cloudflare's own words are kept.
const SAID_AT_MOST: usize = 200;
/// Cloudflare's codes for a token that may not sign in: `Authentication error`, and `Unauthorized to
/// access requested resource`.
const SIGN_IN_CODES: [u64; 2] = [10000, 9109];
/// Cloudflare's code for an account with no `workers.dev` subdomain.
const NO_SUBDOMAIN_CODE: u64 = 10007;

/// One request, before the token is added.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Call {
    pub(crate) method: &'static str,
    /// Path and query, after the base.
    pub(crate) target: String,
    pub(crate) content_type: Option<String>,
    pub(crate) body: Vec<u8>,
}

impl Call {
    fn get(target: String) -> Call {
        Call { method: "GET", target, content_type: None, body: Vec::new() }
    }
}

/// What came back.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Reply {
    pub(crate) status: u16,
    pub(crate) body: Vec<u8>,
}

/// Cloudflare's envelope. `success` is missing from some answers, which their status then settles.
#[derive(Deserialize)]
struct Envelope<T> {
    #[serde(default)]
    success: Option<bool>,
    #[serde(default)]
    errors: Vec<Said>,
    result: Option<T>,
}

#[derive(Deserialize)]
struct Said {
    #[serde(default)]
    code: u64,
    #[serde(default)]
    message: String,
}

#[derive(Deserialize)]
struct AccountWire {
    id: String,
}

#[derive(Deserialize)]
struct ScriptWire {
    id: String,
    #[serde(default)]
    migration_tag: Option<String>,
}

#[derive(Deserialize)]
struct SubdomainWire {
    #[serde(default)]
    subdomain: String,
}

#[derive(Serialize)]
struct Metadata<'a> {
    main_module: &'static str,
    compatibility_date: &'static str,
    bindings: Vec<Binding<'a>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    migrations: Option<Migration>,
}

#[derive(Serialize)]
#[serde(tag = "type")]
enum Binding<'a> {
    #[serde(rename = "durable_object_namespace")]
    DurableObject { name: &'static str, class_name: &'static str },
    #[serde(rename = "ratelimit")]
    RateLimit { name: &'static str, namespace_id: &'static str, simple: Simple },
    #[serde(rename = "plain_text")]
    PlainText { name: &'static str, text: &'a str },
}

#[derive(Serialize)]
struct Simple {
    limit: u32,
    period: u32,
}

#[derive(Serialize)]
struct Migration {
    new_tag: &'static str,
    new_sqlite_classes: [&'static str; 1],
}

#[derive(Serialize)]
struct Enabled {
    enabled: bool,
}

fn envelope<T: serde::de::DeserializeOwned>(what: &'static str, reply: &Reply) -> Result<Envelope<T>, CloudflareError> {
    ipc::decode(what, &reply.body).map_err(|why| CloudflareError::Failed(why.to_string()))
}

fn succeeded(reply: &Reply) -> bool {
    if !(200..300).contains(&reply.status) {
        return false;
    }
    // An answer that is not the envelope at all is read by the caller, which says what was missing.
    ipc::decode::<Envelope<IgnoredAny>>("answer", &reply.body).map_or(true, |answer| answer.success != Some(false))
}

fn at_most(text: &str) -> String {
    text.trim().chars().take(SAID_AT_MOST).collect()
}

/// Why `reply` was not what was asked, in Cloudflare's own words. `limit_sent` is whether the request
/// was an upload carrying `ENROL_LIMIT`.
pub(crate) fn refusal(reply: &Reply, limit_sent: bool) -> CloudflareError {
    let (codes, messages) = match ipc::decode::<Envelope<IgnoredAny>>("refusal", &reply.body) {
        Ok(answer) => (
            answer.errors.iter().map(|said| said.code).collect(),
            answer.errors.iter().map(|said| said.message.as_str()).filter(|said| !said.is_empty()).collect::<Vec<_>>().join("; "),
        ),
        Err(_) => (Vec::new(), String::from_utf8_lossy(&reply.body).into_owned()),
    };
    let messages = at_most(&messages);
    let signs_in = reply.status == 401 || (reply.status == 403 && codes.iter().any(|code: &u64| SIGN_IN_CODES.contains(code)));
    if signs_in {
        return CloudflareError::Refused(if messages.is_empty() { format!("status {}", reply.status) } else { messages });
    }
    let lower = messages.to_lowercase();
    if limit_sent && ["ratelimit", "rate limit", "rate_limit"].iter().any(|word| lower.contains(word)) {
        return CloudflareError::LimitRefused(messages);
    }
    let said = if messages.is_empty() { format!("status {}", reply.status) } else { format!("status {}: {messages}", reply.status) };
    CloudflareError::Failed(at_most(&said))
}

/// An account id, as it goes in a path: letters and digits.
fn account_fits(account: &str) -> Result<(), CloudflareError> {
    match !account.is_empty() && account.len() <= 64 && account.bytes().all(|b| b.is_ascii_alphanumeric()) {
        true => Ok(()),
        false => Err(CloudflareError::Failed("That is not a Cloudflare account id.".to_string())),
    }
}

/// A Worker's name: lowercase letters, digits and `-`, at most 63.
fn name_fits(name: &str) -> Result<(), CloudflareError> {
    let fits = !name.is_empty() && name.len() <= 63 && name.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-');
    match fits {
        true => Ok(()),
        false => Err(CloudflareError::Failed(format!("{name:?} is not a name a Worker can have."))),
    }
}

pub(crate) fn accounts_call() -> Call {
    Call::get("/accounts?per_page=50".to_string())
}

pub(crate) fn accounts_read(reply: &Reply) -> Result<Vec<String>, CloudflareError> {
    if !succeeded(reply) {
        return Err(refusal(reply, false));
    }
    let accounts = envelope::<Vec<AccountWire>>("accounts", reply)?.result.unwrap_or_default();
    Ok(accounts.into_iter().map(|account| account.id).collect())
}

pub(crate) fn scripts_call(account: &str) -> Call {
    Call::get(format!("/accounts/{account}/workers/scripts"))
}

/// Whether an upload of `name` runs the migration: when the account has no Worker of that name, or
/// has one no migration ran on.
pub(crate) fn migration_due(reply: &Reply, name: &str) -> Result<bool, CloudflareError> {
    if !succeeded(reply) {
        return Err(refusal(reply, false));
    }
    let scripts = envelope::<Vec<ScriptWire>>("Workers", reply)?.result.unwrap_or_default();
    Ok(match scripts.into_iter().find(|script| script.id == name) {
        None => true,
        Some(script) => script.migration_tag.unwrap_or_default().is_empty(),
    })
}

/// The `metadata` part: the bindings of `apps/store/wrangler.jsonc`, and the migration when it is due.
pub(crate) fn metadata(upload: &WorkerUpload, migrate: bool) -> Result<String, CloudflareError> {
    let mut bindings = vec![Binding::DurableObject { name: "ACCOUNT", class_name: "Account" }];
    if upload.enrol_limit {
        bindings.push(Binding::RateLimit { name: "ENROL_LIMIT", namespace_id: "1001", simple: Simple { limit: 10, period: 60 } });
    }
    if let Some(team) = &upload.team {
        bindings.push(Binding::PlainText { name: "TEAM", text: team });
    }
    let migrations = migrate.then_some(Migration { new_tag: "v1", new_sqlite_classes: ["Account"] });
    let metadata = Metadata { main_module: MAIN_MODULE, compatibility_date: COMPATIBILITY_DATE, bindings, migrations };
    ipc::encode(&metadata).map_err(|why| CloudflareError::Failed(why.to_string()))
}

/// A boundary the parts do not contain.
fn boundary(parts: &[&str]) -> String {
    (0u32..)
        .map(|n| format!("armada-worker-{n}"))
        .find(|boundary| parts.iter().all(|part| !part.contains(boundary.as_str())))
        .unwrap_or_default()
}

/// `PUT` of `upload` as `multipart/form-data`: the `metadata` part, then the module.
pub(crate) fn upload_call(account: &str, upload: &WorkerUpload, migrate: bool) -> Result<Call, CloudflareError> {
    let metadata = metadata(upload, migrate)?;
    let boundary = boundary(&[&metadata, &upload.script]);
    let mut body = String::new();
    body.push_str(&format!("--{boundary}\r\nContent-Disposition: form-data; name=\"metadata\"\r\nContent-Type: application/json\r\n\r\n"));
    body.push_str(&metadata);
    body.push_str(&format!(
        "\r\n--{boundary}\r\nContent-Disposition: form-data; name=\"{MAIN_MODULE}\"; filename=\"{MAIN_MODULE}\"\r\nContent-Type: application/javascript+module\r\n\r\n"
    ));
    body.push_str(&upload.script);
    body.push_str(&format!("\r\n--{boundary}--\r\n"));
    Ok(Call {
        method: "PUT",
        target: format!("/accounts/{account}/workers/scripts/{}", upload.name),
        content_type: Some(format!("multipart/form-data; boundary={boundary}")),
        body: body.into_bytes(),
    })
}

pub(crate) fn uploaded(reply: &Reply, upload: &WorkerUpload) -> Result<(), CloudflareError> {
    match succeeded(reply) {
        true => Ok(()),
        false => Err(refusal(reply, upload.enrol_limit)),
    }
}

pub(crate) fn subdomain_call(account: &str) -> Call {
    Call::get(format!("/accounts/{account}/workers/subdomain"))
}

/// The account's `workers.dev` subdomain.
pub(crate) fn subdomain_read(reply: &Reply) -> Result<String, CloudflareError> {
    if !succeeded(reply) {
        let no_subdomain = reply.status == 404
            || ipc::decode::<Envelope<IgnoredAny>>("refusal", &reply.body)
                .is_ok_and(|answer| answer.errors.iter().any(|said| said.code == NO_SUBDOMAIN_CODE));
        return Err(if no_subdomain { CloudflareError::NoWorkersDev } else { refusal(reply, false) });
    }
    match envelope::<SubdomainWire>("subdomain", reply)?.result {
        Some(found) if !found.subdomain.is_empty() => Ok(found.subdomain),
        _ => Err(CloudflareError::NoWorkersDev),
    }
}

pub(crate) fn enable_call(account: &str, name: &str) -> Result<Call, CloudflareError> {
    let body = ipc::encode(&Enabled { enabled: true }).map_err(|why| CloudflareError::Failed(why.to_string()))?;
    Ok(Call {
        method: "POST",
        target: format!("/accounts/{account}/workers/scripts/{name}/subdomain"),
        content_type: Some("application/json".to_string()),
        body: body.into_bytes(),
    })
}

pub(crate) fn enabled(reply: &Reply) -> Result<(), CloudflareError> {
    match succeeded(reply) {
        true => Ok(()),
        false => Err(refusal(reply, false)),
    }
}

pub(crate) fn address(name: &str, subdomain: &str) -> String {
    format!("https://{name}.{subdomain}.workers.dev")
}

/// `error` with every copy of the token's text taken out.
pub(crate) fn hidden(error: CloudflareError, token: &Secret<String>) -> CloudflareError {
    let token = token.expose();
    if token.is_empty() {
        return error;
    }
    let hide = |said: String| said.replace(token.as_str(), "[token]");
    match error {
        CloudflareError::Refused(said) => CloudflareError::Refused(hide(said)),
        CloudflareError::LimitRefused(said) => CloudflareError::LimitRefused(hide(said)),
        CloudflareError::NoWorkersDev => CloudflareError::NoWorkersDev,
        CloudflareError::Failed(said) => CloudflareError::Failed(hide(said)),
        CloudflareError::Unreachable(said) => CloudflareError::Unreachable(hide(said)),
    }
}

/// Cloudflare's API, reached with whichever token each call is given. Holds no token itself.
pub struct HttpCloudflare {
    base: String,
    client: Client,
}

impl HttpCloudflare {
    /// Cloudflare's own API.
    pub fn new(timeout: Duration) -> Result<HttpCloudflare, String> {
        HttpCloudflare::at(CLOUDFLARE_API, timeout)
    }

    /// The API at `base_url`, for a test that answers on this machine.
    pub fn at(base_url: &str, timeout: Duration) -> Result<HttpCloudflare, String> {
        let client = Client::builder()
            .timeout(timeout)
            .connect_timeout(timeout)
            .user_agent("armada")
            // A redirect would carry the token to a host it was not given for.
            .redirect(Policy::none())
            .build()
            .map_err(|why| format!("Cloudflare's client could not be made: {why}"))?;
        Ok(HttpCloudflare { base: base_url.trim_end_matches('/').to_string(), client })
    }

    fn send(&self, token: &Secret<String>, call: &Call) -> Result<Reply, CloudflareError> {
        let method = reqwest::Method::from_bytes(call.method.as_bytes()).map_err(|why| CloudflareError::Failed(why.to_string()))?;
        let mut bearer = HeaderValue::from_str(&format!("Bearer {}", token.expose()))
            .map_err(|_| CloudflareError::Refused("The token has characters no Cloudflare token has.".to_string()))?;
        bearer.set_sensitive(true);
        let mut request = self.client.request(method, format!("{}{}", self.base, call.target)).header(AUTHORIZATION, bearer);
        if let Some(content_type) = &call.content_type {
            request = request.header(CONTENT_TYPE, content_type).body(call.body.clone());
        }
        // Everything `reqwest` raises is the request not completing; a status comes back as a `Response`.
        let response = request.send().map_err(|why| CloudflareError::Unreachable(why.without_url().to_string()))?;
        let status = response.status().as_u16();
        let body = response.bytes().map_err(|why| CloudflareError::Unreachable(why.without_url().to_string()))?.to_vec();
        Ok(Reply { status, body })
    }
}

impl Cloudflare for HttpCloudflare {
    fn accounts(&self, token: &Secret<String>) -> Result<Vec<String>, CloudflareError> {
        self.send(token, &accounts_call()).and_then(|reply| accounts_read(&reply)).map_err(|why| hidden(why, token))
    }

    fn upload_worker(&self, token: &Secret<String>, account: &str, upload: &WorkerUpload) -> Result<(), CloudflareError> {
        let upload_it = || {
            account_fits(account)?;
            name_fits(&upload.name)?;
            let migrate = migration_due(&self.send(token, &scripts_call(account))?, &upload.name)?;
            uploaded(&self.send(token, &upload_call(account, upload, migrate)?)?, upload)
        };
        upload_it().map_err(|why| hidden(why, token))
    }

    fn workers_dev(&self, token: &Secret<String>, account: &str, name: &str) -> Result<String, CloudflareError> {
        let turn_on = || {
            account_fits(account)?;
            name_fits(name)?;
            let subdomain = subdomain_read(&self.send(token, &subdomain_call(account))?)?;
            enabled(&self.send(token, &enable_call(account, name)?)?)?;
            Ok(address(name, &subdomain))
        };
        turn_on().map_err(|why| hidden(why, token))
    }
}
