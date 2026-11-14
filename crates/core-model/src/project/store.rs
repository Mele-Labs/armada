//! The claims store a Project names: where its Jobs' claims go, and the team that shares it where it
//! is one. `docs/contracts/central-store.md`, *A store per project*.
//!
//! **A Project that names none sends nothing.** There is no machine-wide address to fall back on.

use alloc::format;
use alloc::string::{String, ToString};
use core::fmt;

use crate::Timestamp;

/// The address of a store: `https://` and a host, nothing after it but an optional `/`.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct StoreAddress(String);

impl StoreAddress {
    /// The address as written, with a trailing `/` taken off, or why it is not one.
    pub fn new(written: &str) -> Result<StoreAddress, MalformedStore> {
        let written = written.trim();
        let refused = || MalformedStore::Address(written.to_string());
        let host = written.strip_prefix("https://").ok_or_else(refused)?.trim_end_matches('/');
        let fits = |b: u8| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b':');
        if host.is_empty() || host.len() > 253 || !host.bytes().all(fits) || host.starts_with(['.', '-', ':']) {
            return Err(refused());
        }
        Ok(StoreAddress(format!("https://{}", host.to_ascii_lowercase())))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// The GitHub org, or one team in it, whose members a team store admits. Lowercase.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct StoreTeam {
    org: String,
    team: Option<String>,
}

impl StoreTeam {
    /// `org` or `org/team-slug`, or why it is not one.
    pub fn new(written: &str) -> Result<StoreTeam, MalformedStore> {
        let written = written.trim();
        let refused = || MalformedStore::Team(written.to_string());
        let (org, team) = match written.split_once('/') {
            Some((org, team)) => (org, Some(team)),
            None => (written, None),
        };
        let org_fits = !org.is_empty() && org.len() <= 39 && org.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-');
        let team_fits = team.is_none_or(|slug| {
            !slug.is_empty() && slug.len() <= 100 && slug.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
        });
        if !org_fits || !team_fits || org.starts_with('-') {
            return Err(refused());
        }
        Ok(StoreTeam { org: org.to_ascii_lowercase(), team: team.map(str::to_ascii_lowercase) })
    }

    pub fn org(&self) -> &str {
        &self.org
    }

    /// The team's slug, where it admits one team and not the whole org.
    pub fn team(&self) -> Option<&str> {
        self.team.as_deref()
    }
}

impl fmt::Display for StoreTeam {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.team {
            Some(team) => write!(f, "{}/{team}", self.org),
            None => f.write_str(&self.org),
        }
    }
}

/// A Worker's name on a Cloudflare account: lowercase letters, digits and `-`, at most 63.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct WorkerName(String);

impl WorkerName {
    pub fn new(written: &str) -> Result<WorkerName, MalformedStore> {
        let written = written.trim();
        let fits = |b: u8| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-';
        if written.is_empty() || written.len() > 63 || !written.bytes().all(fits) || written.starts_with('-') || written.ends_with('-') {
            return Err(MalformedStore::Worker(written.to_string()));
        }
        Ok(WorkerName(written.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// What a Project says of its store. `worker` and `deployed` are written by a deploy from this Mac or
/// another, and are absent for a store joined by its address alone.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectStore {
    pub address: StoreAddress,
    pub team: Option<StoreTeam>,
    pub worker: Option<WorkerName>,
    pub deployed: Option<Timestamp>,
}

impl ProjectStore {
    /// A store joined by its address, personal or a team's.
    pub fn joined(address: StoreAddress, team: Option<StoreTeam>) -> ProjectStore {
        ProjectStore { address, team, worker: None, deployed: None }
    }
}

/// Why a store's part is not one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MalformedStore {
    Address(String),
    Team(String),
    Worker(String),
    Deployed(String),
}

impl fmt::Display for MalformedStore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MalformedStore::Address(it) => write!(f, "`{it}` is not a store address: it is `https://` and a host, and nothing after it"),
            MalformedStore::Team(it) => write!(f, "`{it}` is not a GitHub org or `org/team`"),
            MalformedStore::Worker(it) => write!(f, "`{it}` is not a Worker name: lowercase letters, digits and `-`, at most 63"),
            MalformedStore::Deployed(it) => write!(f, "`{it}` is not a time, RFC 3339 in UTC"),
        }
    }
}

impl core::error::Error for MalformedStore {}
