//! Deploying a project's claims store to his own Cloudflare account. `docs/contracts/central-store.md`,
//! *Deploying a store*.
//!
//! **The sign-ins are tried in his order** (13 and 14 Nov 2026): a token he pasted before for the tunnel's
//! account, then the tunnel's own sign-in, and only where Cloudflare refuses every one is a pasted token
//! asked for. A pasted token is the only one tried when it is given, and is kept once a deploy with it
//! works. **Where Cloudflare refuses the rate limit on joins**, the Worker goes up again without it.
//!
//! Blocking throughout: each step is a call to Cloudflare or a run of `security`.

use std::sync::Arc;
use std::time::Duration;

use adapter_traits::{Cloudflare, CloudflareError, CloudflareTokens, Secret, WorkerUpload};
use core_model::{StoreTeam, WorkerName};

/// How long one call to Cloudflare may take. An upload carries the whole Worker.
pub const CLOUDFLARE_TIMEOUT: Duration = Duration::from_secs(30);

/// The tunnel's Cloudflare sign-in, read when a deploy asks: its account and its token, or `None` where
/// this Mac has none.
pub type TunnelSignIn = Arc<dyn Fn() -> Option<(String, Secret<String>)> + Send + Sync>;

/// What a deploy reaches. **A seam so a test plants Cloudflare, the kept tokens and the tunnel's sign-in.**
#[derive(Clone)]
pub struct Deploying {
    pub cloudflare: Arc<dyn Cloudflare>,
    pub tokens: Arc<dyn CloudflareTokens>,
    pub tunnel: TunnelSignIn,
    /// The store's Worker, bundled.
    pub script: Arc<str>,
    /// Cloudflare's page that makes a token with the rights a deploy needs, filled in.
    pub token_page: String,
}

/// A Worker that is up: where it answers, and whether joins are rate limited.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Up {
    pub address: String,
    pub limited: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Deployed {
    Up(Up),
    /// Cloudflare refused every sign-in this Mac has, in its last words. Nothing was changed.
    NeedsToken(String),
}

/// Which sign-in a try is made with.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Source {
    Kept,
    Tunnel,
    Pasted,
}

/// The Worker's name: `armada-store` for his own, `armada-store-<org>[-<team>]` for a team's.
pub(crate) fn worker_name(team: Option<&StoreTeam>) -> WorkerName {
    let mut name = String::from("armada-store");
    if let Some(team) = team {
        name.push('-');
        name.push_str(team.org());
        if let Some(slug) = team.team() {
            name.push('-');
            name.push_str(slug);
        }
    }
    let name: String = name.chars().map(|c| if c == '_' { '-' } else { c }).take(63).collect();
    WorkerName::new(name.trim_end_matches('-')).unwrap_or_else(|_| WorkerName::new("armada-store").expect("a Worker's name"))
}

impl Deploying {
    /// Whether a deploy can run now: a kept token for the tunnel's account, the tunnel's sign-in to try, or
    /// neither.
    pub(crate) fn readiness(&self) -> ipc::StoreDeploy {
        match (self.tunnel)() {
            Some((account, _)) if matches!(self.tokens.read(&account), Ok(Some(_))) => ipc::StoreDeploy::Ready,
            Some(_) => ipc::StoreDeploy::TryTunnel,
            None => ipc::StoreDeploy::NeedsToken,
        }
    }

    /// Put the Worker `name` up, as `team`'s store where one is given. Blocking.
    pub(crate) fn deploy(&self, name: &WorkerName, team: Option<&StoreTeam>, pasted: Option<Secret<String>>) -> Result<Deployed, CloudflareError> {
        let tunnel = (self.tunnel)();
        let tunnel_account = tunnel.as_ref().map(|(account, _)| account.clone());
        let mut tries: Vec<(Source, Secret<String>)> = Vec::new();
        match pasted {
            Some(token) => tries.push((Source::Pasted, token)),
            None => {
                if let Some(Ok(Some(kept))) = tunnel_account.as_deref().map(|account| self.tokens.read(account)) {
                    tries.push((Source::Kept, kept));
                }
                if let Some((_, token)) = tunnel {
                    tries.push((Source::Tunnel, token));
                }
            }
        }
        let mut refused = String::from("This Mac has no Cloudflare sign-in.");
        for (source, token) in tries {
            let account = match (source, tunnel_account.as_deref()) {
                (Source::Pasted, preferred) => match self.account_for(&token, preferred) {
                    Ok(account) => account,
                    Err(CloudflareError::Refused(said)) => {
                        refused = said;
                        continue;
                    }
                    Err(other) => return Err(other),
                },
                (_, Some(account)) => account.to_string(),
                (_, None) => continue,
            };
            match self.up(&token, &account, name, team) {
                Ok(up) => {
                    if source == Source::Pasted {
                        // A token that could not be kept is asked for again at the next deploy; this one stands.
                        self.tokens.keep(&account, token).ok();
                    }
                    return Ok(Deployed::Up(up));
                }
                Err(CloudflareError::Refused(said)) => refused = said,
                Err(other) => return Err(other),
            }
        }
        Ok(Deployed::NeedsToken(refused))
    }

    /// The account a pasted token deploys to: the tunnel's where the token reaches it, else the first.
    fn account_for(&self, token: &Secret<String>, preferred: Option<&str>) -> Result<String, CloudflareError> {
        let accounts = self.cloudflare.accounts(token)?;
        if let Some(preferred) = preferred.filter(|preferred| accounts.iter().any(|one| one == preferred)) {
            return Ok(preferred.to_string());
        }
        accounts.into_iter().next().ok_or_else(|| CloudflareError::Refused("the token reaches no Cloudflare account".to_string()))
    }

    fn up(&self, token: &Secret<String>, account: &str, name: &WorkerName, team: Option<&StoreTeam>) -> Result<Up, CloudflareError> {
        let mut upload = WorkerUpload {
            name: name.as_str().to_string(),
            script: self.script.to_string(),
            team: team.map(ToString::to_string),
            enrol_limit: true,
        };
        let limited = match self.cloudflare.upload_worker(token, account, &upload) {
            Ok(()) => true,
            Err(CloudflareError::LimitRefused(_)) => {
                upload.enrol_limit = false;
                self.cloudflare.upload_worker(token, account, &upload)?;
                false
            }
            Err(other) => return Err(other),
        };
        let address = self.cloudflare.workers_dev(token, account, name.as_str())?;
        Ok(Up { address, limited })
    }
}
