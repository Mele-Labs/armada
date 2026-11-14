//! A [`Cloudflare`] that answers each token as it was told, and a [`CloudflareTokens`] in memory, for
//! the tests that deploy a store.
//!
//! # What it is faithful about
//!
//! A token answers as it was set up, and a token nobody set up is refused, as Cloudflare refuses a
//! token it did not issue. An upload to an account the token cannot see is refused, and a `workers.dev`
//! address is turned on only for a Worker this fake was given. It is **not** faithful about the
//! migration: it remembers no tags.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Mutex, MutexGuard, PoisonError};

use adapter_traits::{Cloudflare, CloudflareError, CloudflareTokens, Secret, WorkerUpload};

/// One call the fake was given.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CloudflareCall {
    /// `accounts`, `upload_worker` or `workers_dev`.
    pub method: &'static str,
    /// The token's text, which a fake may hold.
    pub token: String,
    /// `None` for `accounts`.
    pub account: Option<String>,
    /// The Worker's name, for `upload_worker` and `workers_dev`.
    pub name: Option<String>,
    /// What was uploaded, for `upload_worker`.
    pub upload: Option<WorkerUpload>,
}

/// How one token is answered.
#[derive(Clone, Debug)]
enum Answer {
    /// The token sees these accounts and may do anything on them.
    Accepts(Vec<String>),
    /// Every call with the token is refused.
    Refuses,
    /// The token sees these accounts, and the calls named are refused.
    RefusesAt(Vec<String>, BTreeSet<&'static str>),
}

#[derive(Debug)]
struct State {
    answers: BTreeMap<String, Answer>,
    limit_refused: bool,
    no_workers_dev: bool,
    unreachable: bool,
    subdomain: String,
    /// The Workers uploaded, as `(account, name)`.
    workers: BTreeSet<(String, String)>,
    calls: Vec<CloudflareCall>,
}

impl Default for State {
    fn default() -> State {
        State {
            answers: BTreeMap::new(),
            limit_refused: false,
            no_workers_dev: false,
            unreachable: false,
            subdomain: FAKE_SUBDOMAIN.to_string(),
            workers: BTreeSet::new(),
            calls: Vec::new(),
        }
    }
}

/// The `workers.dev` subdomain the fake's accounts have unless told otherwise.
pub const FAKE_SUBDOMAIN: &str = "person";

/// Cloudflare, answering from what it was told and reaching nothing.
#[derive(Debug, Default)]
pub struct FakeCloudflare {
    state: Mutex<State>,
}

impl FakeCloudflare {
    pub fn new() -> FakeCloudflare {
        FakeCloudflare::default()
    }

    fn state(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn answer(self, token: &str, answer: Answer) -> FakeCloudflare {
        self.state().answers.insert(token.to_string(), answer);
        self
    }

    /// `token` sees `accounts` and may deploy to them.
    pub fn accepts(self, token: &str, accounts: &[&str]) -> FakeCloudflare {
        self.answer(token, Answer::Accepts(accounts.iter().map(|one| one.to_string()).collect()))
    }

    /// Every call with `token` is `Refused`.
    pub fn refuses(self, token: &str) -> FakeCloudflare {
        self.answer(token, Answer::Refuses)
    }

    /// `token` sees `accounts`, and each call named in `methods` (`accounts`, `upload_worker`,
    /// `workers_dev`) is `Refused`: a token that signs in but may not deploy.
    pub fn refuses_at(self, token: &str, accounts: &[&str], methods: &[&'static str]) -> FakeCloudflare {
        let accounts = accounts.iter().map(|one| one.to_string()).collect();
        self.answer(token, Answer::RefusesAt(accounts, methods.iter().copied().collect()))
    }

    /// An upload with `enrol_limit: true` is `LimitRefused`; one without goes through.
    pub fn refuses_the_limit(self) -> FakeCloudflare {
        self.state().limit_refused = true;
        self
    }

    /// The accounts have no `workers.dev` subdomain, so `workers_dev` is `NoWorkersDev`.
    pub fn without_workers_dev(self) -> FakeCloudflare {
        self.state().no_workers_dev = true;
        self
    }

    /// The accounts' `workers.dev` subdomain, [`FAKE_SUBDOMAIN`] until this is called.
    pub fn with_subdomain(self, subdomain: &str) -> FakeCloudflare {
        self.state().subdomain = subdomain.to_string();
        self
    }

    /// Every call is `Unreachable`.
    pub fn unreachable(self) -> FakeCloudflare {
        self.set_unreachable(true);
        self
    }

    /// While set, every call is `Unreachable` and changes nothing.
    pub fn set_unreachable(&self, unreachable: bool) {
        self.state().unreachable = unreachable;
    }

    /// Every call, in the order made.
    pub fn calls(&self) -> Vec<CloudflareCall> {
        self.state().calls.clone()
    }

    /// The token of every call, in the order made.
    pub fn tokens_used(&self) -> Vec<String> {
        self.state().calls.iter().map(|call| call.token.clone()).collect()
    }

    /// Every Worker uploaded, in the order uploaded, refused ones included.
    pub fn uploads(&self) -> Vec<WorkerUpload> {
        self.state().calls.iter().filter_map(|call| call.upload.clone()).collect()
    }

    /// The Workers that went up, as `(account, name)`.
    pub fn workers(&self) -> Vec<(String, String)> {
        self.state().workers.iter().cloned().collect()
    }

    /// Record the call and answer whether it may go on, with the accounts the token sees.
    fn called(&self, call: CloudflareCall) -> Result<Vec<String>, CloudflareError> {
        let mut state = self.state();
        let method = call.method;
        let token = call.token.clone();
        state.calls.push(call);
        if state.unreachable {
            return Err(CloudflareError::Unreachable("the fake is unreachable".to_string()));
        }
        match state.answers.get(&token) {
            Some(Answer::Accepts(accounts)) => Ok(accounts.clone()),
            Some(Answer::RefusesAt(accounts, methods)) if !methods.contains(method) => Ok(accounts.clone()),
            Some(_) => Err(CloudflareError::Refused("Authentication error".to_string())),
            None => Err(CloudflareError::Refused("Invalid API Token".to_string())),
        }
    }
}

fn sees(accounts: &[String], account: &str) -> Result<(), CloudflareError> {
    match accounts.iter().any(|one| one == account) {
        true => Ok(()),
        false => Err(CloudflareError::Refused("Authentication error".to_string())),
    }
}

impl Cloudflare for FakeCloudflare {
    fn accounts(&self, token: &Secret<String>) -> Result<Vec<String>, CloudflareError> {
        self.called(CloudflareCall { method: "accounts", token: token.expose().clone(), account: None, name: None, upload: None })
    }

    fn upload_worker(&self, token: &Secret<String>, account: &str, upload: &WorkerUpload) -> Result<(), CloudflareError> {
        let accounts = self.called(CloudflareCall {
            method: "upload_worker",
            token: token.expose().clone(),
            account: Some(account.to_string()),
            name: Some(upload.name.clone()),
            upload: Some(upload.clone()),
        })?;
        sees(&accounts, account)?;
        let mut state = self.state();
        if upload.enrol_limit && state.limit_refused {
            return Err(CloudflareError::LimitRefused("Unknown binding type: ratelimit".to_string()));
        }
        state.workers.insert((account.to_string(), upload.name.clone()));
        Ok(())
    }

    fn workers_dev(&self, token: &Secret<String>, account: &str, name: &str) -> Result<String, CloudflareError> {
        let accounts = self.called(CloudflareCall {
            method: "workers_dev",
            token: token.expose().clone(),
            account: Some(account.to_string()),
            name: Some(name.to_string()),
            upload: None,
        })?;
        sees(&accounts, account)?;
        let state = self.state();
        if state.no_workers_dev {
            return Err(CloudflareError::NoWorkersDev);
        }
        if !state.workers.contains(&(account.to_string(), name.to_string())) {
            return Err(CloudflareError::Failed(format!("status 404: This Worker does not exist on your account: {name}")));
        }
        Ok(format!("https://{name}.{}.workers.dev", state.subdomain))
    }
}

/// The tokens he pasted, in memory.
#[derive(Debug, Default)]
pub struct FakeCloudflareTokens {
    held: Mutex<BTreeMap<String, String>>,
}

impl FakeCloudflareTokens {
    pub fn new() -> FakeCloudflareTokens {
        FakeCloudflareTokens::default()
    }

    fn held(&self) -> MutexGuard<'_, BTreeMap<String, String>> {
        self.held.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// The token kept for `account`, as text a test can compare.
    pub fn kept(&self, account: &str) -> Option<String> {
        self.held().get(account).cloned()
    }
}

impl CloudflareTokens for FakeCloudflareTokens {
    fn read(&self, account: &str) -> Result<Option<Secret<String>>, String> {
        Ok(self.held().get(account).cloned().map(Secret::new))
    }

    fn keep(&self, account: &str, token: Secret<String>) -> Result<(), String> {
        self.held().insert(account.to_string(), token.expose().clone());
        Ok(())
    }

    fn forget(&self, account: &str) -> Result<(), String> {
        self.held().remove(account);
        Ok(())
    }
}
