//! A project's claims store on the wire: what the store setting reads, joining a store by its address
//! or none, and deploying one to his own Cloudflare account. `docs/contracts/central-store.md`, *A store
//! per project* and *Deploying a store*.

use serde::{Deserialize, Serialize};

use crate::{Instant, ProjectStore};

/// `get_project_store`: everything the store setting of one project shows.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectStoreState {
    pub project: String,
    /// The store its file names. **Absent is none: its Jobs send nothing.**
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub store: Option<ProjectStore>,
    /// The names of the Macs signed in to it, oldest first, as the store answered. Empty with no store.
    pub macs: Vec<String>,
    /// Why the Macs could not be read, where they could not: the store did not answer, or this Mac has
    /// not joined it and cannot ask.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub macs_unread: Option<String>,
    /// Whether this Mac has joined it, as the profile its repositories are bound to.
    pub joined_here: bool,
    /// Whether Deploy can run now.
    pub deploy: StoreDeploy,
    /// Whether joining a store can run now.
    pub join: StoreJoin,
    /// Cloudflare's page that makes a token with the rights a deploy needs, filled in. Opened for the
    /// second sign-in.
    pub token_page: String,
}

/// Whether Deploy can run now, and with which Cloudflare sign-in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StoreDeploy {
    /// A token he pasted before is kept for the tunnel's account.
    Ready,
    /// The tunnel's Cloudflare sign-in is tried first. Cloudflare may refuse it, and then a pasted token
    /// is asked for.
    TryTunnel,
    /// This Mac has no Cloudflare sign-in: a pasted token is needed.
    NeedsToken,
}

/// Whether joining a store can run now.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StoreJoin {
    /// A repository of the project is bound to a profile this Mac is signed in to. `gh`'s token for its
    /// login is read when the join runs.
    Ready,
    /// No repository of the project is bound to a profile on this Mac, so there is no one to join as.
    NoProfile,
}

/// `set_project_store`: the store a project's claims go to, joined by its address, or none.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetProjectStore {
    /// Absent is none: its Jobs send nothing from now on, what was still to send included.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub store: Option<JoinStore>,
}

/// A store to join by its address.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JoinStore {
    pub address: String,
    /// `org` or `org/team`, for a team's store.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub team: Option<String>,
}

/// `deploy_project_store`: deploy the project's store to his own Cloudflare account, or redeploy it.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeployProjectStore {
    /// `org` or `org/team` to deploy a team's store. Absent is a store of his own.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub team: Option<String>,
    /// The token he pasted for the second sign-in. **Kept in the Keychain once a deploy with it works,
    /// and never answered back.**
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cloudflare_token: Option<String>,
}

/// What a deploy did.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum StoreDeployed {
    /// The Worker is up and the project names it. `join_said` is why this Mac could not join it yet,
    /// where it could not.
    Deployed {
        state: ProjectStoreState,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        join_said: Option<String>,
    },
    /// Cloudflare refused every sign-in this Mac has: open `page`, make a token and deploy again with it.
    /// Nothing was changed.
    NeedsToken { page: String, said: String },
}

/// `get_project_claims`: every live claim in the project's store, grouped by who holds it. **The store
/// holds hashes**, so a claim is named only where this Mac has the repository and can match the hash; a
/// teammate's claim in a repository this Mac does not have carries its Job's title alone. Since 14 Nov 2026.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectClaims {
    pub project: String,
    /// Whether the project names a store. Without one its Jobs claim nothing and `holders` is empty.
    pub store: bool,
    /// Why the claims could not be read, where they could not: the store did not answer, or this Mac has
    /// not joined it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unread: Option<String>,
    /// One group a holder, in the store's order: by the person, then the Mac, then the Job.
    pub holders: Vec<ClaimsHolder>,
}

/// One Job holding claims, or waiting on one, and what it holds.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClaimsHolder {
    /// The person, as their forge login. Empty where the holding Mac knew none.
    pub name: String,
    /// The holding Job's Mac.
    pub machine: String,
    /// The holding Job's title. Empty where it has none.
    pub title: String,
    /// The holding Job's id on its Mac.
    pub job: String,
    /// Whether it is a Job of this Mac's.
    pub this_mac: bool,
    /// When it last renewed its claims, or for a Job that only waits, last asked. A claim lapses 30 minutes
    /// after it.
    pub renewed: Instant,
    pub claims: Vec<HeldClaim>,
    /// Every claim another Job holds that this Job keeps asking for: what it waits on.
    pub waits: Vec<ClaimsWait>,
}

/// One claim, by kind, named where this Mac can.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeldClaim {
    pub kind: HeldClaimKind,
    /// The item, file, test, pull request number or red commit, where this Mac matched the hash.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub named: Option<String>,
    /// The repository as `owner/name`, where this Mac has it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repository: Option<String>,
    /// When it was taken.
    pub since: Instant,
    /// For a pull request: what it waits on, the failing test or main's red its Job waits for. Empty
    /// otherwise.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub waits_on: Vec<ClaimsWait>,
}

/// What a claim is on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HeldClaimKind {
    Item,
    File,
    Test,
    PullRequest,
    RedMain,
}

/// A claim another Job holds that a Job waits on, with who holds it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClaimsWait {
    pub kind: HeldClaimKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub named: Option<String>,
    /// The holding Job's person, Mac and title, as `ClaimsHolder` has them.
    pub name: String,
    pub machine: String,
    pub title: String,
}
