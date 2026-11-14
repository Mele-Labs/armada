//! Deploying a project's claims store to the person's own Cloudflare account, and keeping the token he
//! pasted for it. `docs/contracts/central-store.md`, *Deploying a store*.
//!
//! **Blocking**: every call is a round trip to Cloudflare or a run of `security`, so a caller makes it
//! on a blocking thread.

use crate::Secret;

/// The Worker to put on an account: its name, the script, and what it is bound to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkerUpload {
    /// The Worker's name on the account: lowercase letters, digits and `-`, at most 63.
    pub name: String,
    /// The bundled module, `apps/store`'s Worker.
    pub script: String,
    /// `org` or `org/team` for a team store, set as the Worker's `TEAM` var. `None` for his own.
    pub team: Option<String>,
    /// Whether to bind `ENROL_LIMIT`, the Rate Limiting binding that counts enrolments per address.
    pub enrol_limit: bool,
}

/// Why Cloudflare did not do what was asked.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CloudflareError {
    /// The token may not do this: a `401`, or a `403` that is an authentication error. **A deploy reads
    /// this as a request for the second sign-in.** Cloudflare's own words.
    Refused(String),
    /// The upload was refused for its `ENROL_LIMIT` binding, so the account cannot have it. A deploy
    /// goes again without it.
    LimitRefused(String),
    /// The account has no `workers.dev` subdomain yet, so the Worker has no address to answer at.
    NoWorkersDev,
    /// Any other refusal, in Cloudflare's own words.
    Failed(String),
    /// Cloudflare did not answer.
    Unreachable(String),
}

impl CloudflareError {
    /// The sentence a person reads.
    pub fn said(&self) -> String {
        match self {
            CloudflareError::Refused(said) => format!("Cloudflare refused the sign-in: {said}"),
            CloudflareError::LimitRefused(said) => format!("Cloudflare refused the rate limit on joins: {said}"),
            CloudflareError::NoWorkersDev => {
                "The Cloudflare account has no workers.dev subdomain yet. Open Workers in the Cloudflare dashboard once to pick one.".to_string()
            }
            CloudflareError::Failed(said) => format!("Cloudflare refused the deploy: {said}"),
            CloudflareError::Unreachable(said) => format!("Cloudflare could not be reached: {said}"),
        }
    }
}

/// Cloudflare's API, as a deploy uses it.
pub trait Cloudflare: Send + Sync {
    /// The ids of the accounts `token` can see, in Cloudflare's order.
    fn accounts(&self, token: &Secret<String>) -> Result<Vec<String>, CloudflareError>;

    /// Put `upload` on `account`, replacing a Worker of that name. The first upload of a name also runs
    /// the migration that makes its SQLite Durable Object class; a later one runs none.
    fn upload_worker(&self, token: &Secret<String>, account: &str, upload: &WorkerUpload) -> Result<(), CloudflareError>;

    /// Turn on the Worker's `workers.dev` address and answer it, `https://<name>.<subdomain>.workers.dev`.
    fn workers_dev(&self, token: &Secret<String>, account: &str, name: &str) -> Result<String, CloudflareError>;
}

/// The Cloudflare token he pasted for an account, kept so a redeploy does not ask again. **Never
/// printed, logged or sent anywhere but Cloudflare.**
pub trait CloudflareTokens: Send + Sync {
    fn read(&self, account: &str) -> Result<Option<Secret<String>>, String>;
    fn keep(&self, account: &str, token: Secret<String>) -> Result<(), String>;
    fn forget(&self, account: &str) -> Result<(), String>;
}
