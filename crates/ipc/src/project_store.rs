//! A project's claims store on the wire: what the store setting reads, joining a store by its address
//! or none, and deploying one to his own Cloudflare account. `docs/contracts/central-store.md`, *A store
//! per project* and *Deploying a store*.

use serde::{Deserialize, Serialize};

use crate::ProjectStore;

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
