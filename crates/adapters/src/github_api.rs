//! Fleet's calls to GitHub over HTTPS through one pooled client, in place of a
//! `gh` process for each. A `gh` call measured about 0.9 s: the process, a
//! keychain read for the token, a fresh TLS handshake, then GitHub. A call here
//! is GitHub's own time on a connection already open. `gh` stays for two
//! things: reading the token once, and `pr merge`.
//!
//! **One client for the whole Fleet**, cloned by `Arc`, so calls from every
//! blocking thread share its pool, and over HTTP/2 one connection.
//!
//! **The client is built on the first call, never by [`GitHubApi::new`]**:
//! `reqwest`'s blocking client panics if it is made on an async worker, and a
//! call is only ever made from a thread that may block (`spawn_blocking`), as
//! for [`HttpStore`](crate::HttpStore). Making a `GitHubApi` does no I/O, so it
//! can be made anywhere, and dropping it hands the client to a thread of its
//! own, because dropping one joins its runtime thread.
//!
//! **Only `github.com`.** A repository on any other host is
//! [`NotGitHub::Elsewhere`], so the caller routes the whole repository to the
//! `gh` adapter rather than mixing the two.

use std::fmt;
use std::path::Path;
use std::sync::{Arc, Mutex, OnceLock, PoisonError};
use std::time::Duration;

use adapter_traits::Secret;
use reqwest::blocking::{Client, Response};
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, AUTHORIZATION, CONTENT_TYPE, LINK};
use reqwest::Method;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use crate::forge_address::{remote_of, ForgeAddress};

/// The one host this adapter answers for.
pub const GITHUB_HOST: &str = "github.com";
const API_BASE: &str = "https://api.github.com";
/// The REST version every call is pinned to, so a change at GitHub is opted into.
const API_VERSION: &str = "2022-11-28";
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
/// Enough idle HTTP/1.1 connections for Fleet's blocking pool; over HTTP/2 one is used.
const IDLE_PER_HOST: usize = 8;
const IDLE_FOR: Duration = Duration::from_secs(90);
/// The most pages [`GitHubApi::get_paged`] reads: 1,000 rows at `per_page=100`.
pub const MOST_PAGES: usize = 10;
/// The longest a refusal's own words are kept.
const SAID_AT_MOST: usize = 300;

/// A repository on `github.com`, which this adapter can answer for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GitHubRepository {
    address: ForgeAddress,
}

/// Why a checkout is not this adapter's.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NotGitHub {
    /// It is on another forge: route the whole repository to the `gh` adapter.
    Elsewhere(ForgeAddress),
    /// It has no remote a forge address can be read from.
    NoRemote,
}

impl GitHubRepository {
    pub fn of(address: ForgeAddress) -> Result<GitHubRepository, NotGitHub> {
        if address.host() == GITHUB_HOST {
            Ok(GitHubRepository { address })
        } else {
            Err(NotGitHub::Elsewhere(address))
        }
    }

    /// The repository a checkout's `origin` names.
    pub fn of_checkout(root: &Path) -> Result<GitHubRepository, NotGitHub> {
        GitHubRepository::of(remote_of(root).ok_or(NotGitHub::NoRemote)?)
    }

    pub fn address(&self) -> &ForgeAddress {
        &self.address
    }

    /// `/repos/owner/name`, which every REST path about the repository starts with.
    pub fn path(&self) -> String {
        format!("/repos/{}", self.address.slug())
    }
}

/// One answer below 400: its status and its body, to decode.
#[derive(Debug)]
pub struct Answer {
    pub status: u16,
    pub body: Vec<u8>,
    next: Option<String>,
}

impl Answer {
    pub fn decode<T: DeserializeOwned>(&self, expected: &'static str) -> Result<T, ApiFailure> {
        ipc::decode(expected, &self.body).map_err(|why| ApiFailure::Malformed(why.to_string()))
    }
}

/// What GitHub said about its rate limit on a 403 or 429. No sleeping or retry
/// is done here: the caller decides whether to wait.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RateLimit {
    /// Calls left in the window: `x-ratelimit-remaining`.
    pub remaining: Option<u64>,
    /// When the window resets, in seconds since the epoch: `x-ratelimit-reset`.
    pub reset_at: Option<u64>,
    /// Seconds to wait, for a secondary limit: `retry-after`.
    pub retry_after: Option<u64>,
}

/// Why a call did not answer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ApiFailure {
    /// `gh` would not give a token.
    NoToken(String),
    /// Nothing answered: no network, a timeout.
    Unreachable(String),
    /// GitHub answered 400 or above. `limit` is read on a 403 or a 429.
    Refused { status: u16, said: String, limit: Option<RateLimit> },
    /// A GraphQL answer carried `errors`, whatever its status: their messages.
    GraphQl(Vec<String>),
    /// A body that could not be read, or written.
    Malformed(String),
}

impl fmt::Display for ApiFailure {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ApiFailure::NoToken(why) => write!(out, "No GitHub token: {why}"),
            ApiFailure::Unreachable(why) => write!(out, "GitHub did not answer: {why}"),
            ApiFailure::Refused { status, said, limit } => {
                write!(out, "GitHub answered {status}: {said}")?;
                match limit {
                    Some(RateLimit { retry_after: Some(seconds), .. }) => write!(out, " (retry after {seconds} s)"),
                    Some(RateLimit { remaining: Some(0), reset_at: Some(at), .. }) => {
                        write!(out, " (rate limit spent until {at})")
                    }
                    _ => Ok(()),
                }
            }
            ApiFailure::GraphQl(messages) => write!(out, "GitHub refused the query: {}", messages.join("; ")),
            ApiFailure::Malformed(why) => write!(out, "{why}"),
        }
    }
}

type ReadToken = dyn Fn() -> Result<Secret<String>, ApiFailure> + Send + Sync;

/// GitHub's API, through one client every clone shares.
#[derive(Clone)]
pub struct GitHubApi {
    inner: Arc<Inner>,
}

struct Inner {
    base: String,
    client: OnceLock<Client>,
    /// Read once, and again only on a 401.
    token: Mutex<Option<Secret<String>>>,
    read_token: Box<ReadToken>,
}

impl Drop for Inner {
    fn drop(&mut self) {
        if let Some(client) = self.client.take() {
            // Dropping a client joins its runtime thread, which must not block an async worker.
            std::thread::spawn(move || drop(client));
        }
    }
}

impl Default for GitHubApi {
    fn default() -> GitHubApi {
        GitHubApi::new()
    }
}

impl GitHubApi {
    /// `api.github.com`, with the token `gh` holds for `github.com`. No I/O until the first call.
    pub fn new() -> GitHubApi {
        GitHubApi::against(API_BASE, || token_from_gh(GITHUB_HOST))
    }

    /// Another address and token source: a socket on this machine in the tests.
    pub(crate) fn against(
        base: &str,
        read_token: impl Fn() -> Result<Secret<String>, ApiFailure> + Send + Sync + 'static,
    ) -> GitHubApi {
        GitHubApi {
            inner: Arc::new(Inner {
                base: base.trim_end_matches('/').to_string(),
                client: OnceLock::new(),
                token: Mutex::new(None),
                read_token: Box::new(read_token),
            }),
        }
    }

    pub fn get(&self, path: &str) -> Result<Answer, ApiFailure> {
        self.call(Method::GET, &self.url(path), None)
    }

    /// Every page of a list, following `Link: rel="next"`, at most [`MOST_PAGES`].
    /// A next link off this API's address is not followed: it would carry the token there.
    pub fn get_paged(&self, path: &str) -> Result<Vec<Answer>, ApiFailure> {
        let within = format!("{}/", self.inner.base);
        let mut url = self.url(path);
        let mut pages = Vec::new();
        while pages.len() < MOST_PAGES {
            let page = self.call(Method::GET, &url, None)?;
            let next = page.next.clone().filter(|next| next.starts_with(&within));
            pages.push(page);
            match next {
                Some(next) => url = next,
                None => break,
            }
        }
        Ok(pages)
    }

    /// `POST /graphql`, returning `data`. An answer carrying `errors` is
    /// [`ApiFailure::GraphQl`], since GitHub sends them with a 200.
    pub fn graphql<T: DeserializeOwned, V: Serialize>(&self, query: &str, variables: &V) -> Result<T, ApiFailure> {
        let answer = self.post("/graphql", &GraphQlRequest { query, variables })?;
        // Errors first: `data` beside them may hold nulls `T` cannot be read from.
        let errors: GraphQlErrors = answer.decode("a GraphQL answer")?;
        if let Some(errors) = errors.errors.filter(|errors| !errors.is_empty()) {
            return Err(ApiFailure::GraphQl(errors.into_iter().map(|error| error.message).collect()));
        }
        let reply: GraphQlData<T> = answer.decode("a GraphQL answer")?;
        reply.data.ok_or_else(|| ApiFailure::Malformed("A GraphQL answer carried no data.".to_string()))
    }

    pub fn post<B: Serialize>(&self, path: &str, body: &B) -> Result<Answer, ApiFailure> {
        self.call(Method::POST, &self.url(path), Some(&encoded(body)?))
    }

    pub fn patch<B: Serialize>(&self, path: &str, body: &B) -> Result<Answer, ApiFailure> {
        self.call(Method::PATCH, &self.url(path), Some(&encoded(body)?))
    }

    pub fn put<B: Serialize>(&self, path: &str, body: &B) -> Result<Answer, ApiFailure> {
        self.call(Method::PUT, &self.url(path), Some(&encoded(body)?))
    }

    pub fn delete(&self, path: &str) -> Result<Answer, ApiFailure> {
        self.call(Method::DELETE, &self.url(path), None)
    }

    /// A `DELETE` that names what to remove in its body, as removing a label or a reviewer does.
    pub fn delete_with<B: Serialize>(&self, path: &str, body: &B) -> Result<Answer, ApiFailure> {
        self.call(Method::DELETE, &self.url(path), Some(&encoded(body)?))
    }

    fn url(&self, path: &str) -> String {
        format!("{}/{}", self.inner.base, path.trim_start_matches('/'))
    }

    /// The call, and on a 401 the token read again and the call made once more.
    fn call(&self, method: Method, url: &str, body: Option<&[u8]>) -> Result<Answer, ApiFailure> {
        let client = self.client()?;
        let first = self.send(client, &method, url, body, false)?;
        if first.status().as_u16() != 401 {
            return answer_of(first);
        }
        drop(first);
        answer_of(self.send(client, &method, url, body, true)?)
    }

    fn send(
        &self,
        client: &Client,
        method: &Method,
        url: &str,
        body: Option<&[u8]>,
        fresh_token: bool,
    ) -> Result<Response, ApiFailure> {
        let mut request = client.request(method.clone(), url).header(AUTHORIZATION, self.bearer(fresh_token)?);
        if let Some(body) = body {
            request = request.header(CONTENT_TYPE, "application/json").body(body.to_vec());
        }
        // The URL is left off: it says nothing a person can act on.
        request.send().map_err(|why| ApiFailure::Unreachable(why.without_url().to_string()))
    }

    /// The token as a sensitive header, read from `gh` the first time and when `fresh`.
    /// The lock is held over the read, so threads arriving together run `gh` once.
    fn bearer(&self, fresh: bool) -> Result<HeaderValue, ApiFailure> {
        let mut held = self.inner.token.lock().unwrap_or_else(PoisonError::into_inner);
        let token = match held.take() {
            Some(token) if !fresh => token,
            _ => (self.inner.read_token)()?,
        };
        let header = HeaderValue::from_str(&format!("Bearer {}", token.expose()));
        *held = Some(token);
        let mut header = header.map_err(|_| ApiFailure::NoToken("gh gave a token no header can carry.".to_string()))?;
        header.set_sensitive(true);
        Ok(header)
    }

    /// The one client, built by the first call on whichever blocking thread makes it.
    /// Two threads racing both build one; the loser's is dropped on its own blocking thread.
    fn client(&self) -> Result<&Client, ApiFailure> {
        if let Some(client) = self.inner.client.get() {
            return Ok(client);
        }
        let client = built()?;
        Ok(self.inner.client.get_or_init(|| client))
    }
}

fn built() -> Result<Client, ApiFailure> {
    let mut headers = HeaderMap::new();
    headers.insert(ACCEPT, HeaderValue::from_static("application/vnd.github+json"));
    headers.insert("x-github-api-version", HeaderValue::from_static(API_VERSION));
    Client::builder()
        .user_agent("armada")
        .default_headers(headers)
        .connect_timeout(CONNECT_TIMEOUT)
        .timeout(REQUEST_TIMEOUT)
        .pool_max_idle_per_host(IDLE_PER_HOST)
        .pool_idle_timeout(IDLE_FOR)
        .build()
        .map_err(|why| ApiFailure::Unreachable(format!("The GitHub client could not be made: {why}")))
}

#[derive(Serialize)]
struct GraphQlRequest<'a, V> {
    query: &'a str,
    variables: &'a V,
}

#[derive(Deserialize)]
struct GraphQlData<T> {
    data: Option<T>,
}

#[derive(Deserialize)]
struct GraphQlErrors {
    errors: Option<Vec<GraphQlError>>,
}

#[derive(Deserialize)]
struct GraphQlError {
    message: String,
}

/// GitHub's own words on a refusal: `{"message": ...}`.
#[derive(Deserialize)]
struct Refusal {
    message: Option<String>,
}

/// A request body, through `ipc`'s encoder: this crate parses no JSON by hand.
fn encoded<B: Serialize>(body: &B) -> Result<Vec<u8>, ApiFailure> {
    ipc::encode(body).map(String::into_bytes).map_err(|why| ApiFailure::Malformed(why.to_string()))
}

fn answer_of(response: Response) -> Result<Answer, ApiFailure> {
    let status = response.status().as_u16();
    let next = response.headers().get(LINK).and_then(|link| link.to_str().ok()).and_then(next_link);
    let limit = matches!(status, 403 | 429).then(|| rate_limit_of(response.headers()));
    let body = response
        .bytes()
        .map_err(|why| ApiFailure::Unreachable(why.without_url().to_string()))?
        .to_vec();
    if status >= 400 {
        return Err(ApiFailure::Refused { status, said: said_in(&body), limit });
    }
    Ok(Answer { status, body, next })
}

fn said_in(body: &[u8]) -> String {
    let said = match ipc::decode::<Refusal>("a refusal", body) {
        Ok(Refusal { message: Some(message) }) => message,
        _ => String::from_utf8_lossy(body).into_owned(),
    };
    said.chars().take(SAID_AT_MOST).collect()
}

fn rate_limit_of(headers: &HeaderMap) -> RateLimit {
    let number = |name: &str| headers.get(name)?.to_str().ok()?.trim().parse().ok();
    RateLimit {
        remaining: number("x-ratelimit-remaining"),
        reset_at: number("x-ratelimit-reset"),
        retry_after: number("retry-after"),
    }
}

/// The `rel="next"` target in a `Link` header:
/// `<https://api.github.com/...?page=2>; rel="next", <...>; rel="last"`.
pub(crate) fn next_link(link: &str) -> Option<String> {
    link.split(',').find_map(|part| {
        let (target, parameters) = part.split_once(';')?;
        let next = parameters.split(';').any(|parameter| {
            parameter
                .trim()
                .strip_prefix("rel=")
                .is_some_and(|rel| rel.trim_matches('"').split_whitespace().any(|one| one == "next"))
        });
        next.then(|| target.trim().trim_start_matches('<').trim_end_matches('>').to_string())
    })
}

/// The token `gh` holds for `host`, read at utility priority like Fleet's other reads.
fn token_from_gh(host: &str) -> Result<Secret<String>, ApiFailure> {
    let run = crate::background::command("gh")
        .args(["auth", "token", "--hostname", host])
        .output()
        .map_err(|why| ApiFailure::NoToken(format!("gh could not be started: {why}")))?;
    if !run.status.success() {
        return Err(ApiFailure::NoToken(String::from_utf8_lossy(&run.stderr).trim().to_string()));
    }
    match String::from_utf8(run.stdout) {
        Ok(token) if !token.trim().is_empty() => Ok(Secret::new(token.trim().to_string())),
        _ => Err(ApiFailure::NoToken("gh printed no token.".to_string())),
    }
}
