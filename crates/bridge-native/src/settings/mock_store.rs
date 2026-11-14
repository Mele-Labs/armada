//! The walk's and the tests' claims stores, by project: what `GET /projects/project/:project/store` reads and what
//! joining, using none and deploying change. Fleet's own sentences are copied from what the routes refuse with
//! (`crates/ipc/operations/set_project_store.toml`, `deploy_project_store.toml`), so a walk reads as a live one.

use std::collections::BTreeMap;

use ipc::{DeployProjectStore, JoinStore, ProjectStore, ProjectStoreState, StoreDeploy, StoreDeployed, StoreJoin};

use super::wire::Refused;

/// What a project's Macs read as once a store is joined.
const MACS: [&str; 3] = ["Studio", "Work MacBook", "Mac mini"];

/// When the walk's stores were deployed, as Fleet writes it.
const DEPLOYED: &str = "2026-11-14T12:00:00Z";

/// Cloudflare's page that makes a token with the rights a deploy needs.
const TOKEN_PAGE: &str = "https://dash.cloudflare.com/profile/api-tokens?permissionGroupKeys=workers_scripts_write&name=Armada";

/// The team the mock's forge says he is not a member of.
pub const NOT_A_MEMBER: &str = "acme/platform";

/// What the store is told of a team it refuses, in the store's words.
const NOT_A_MEMBER_SAID: &str = "GitHub says you are not a member of acme/platform.";

/// What Cloudflare refused every sign-in with.
const NEEDS_TOKEN_SAID: &str = "Cloudflare refused every sign-in this Mac has.";

/// One call the page made, oldest first, for a test to read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StoreCall {
    Set { project: String, store: Option<JoinStore> },
    Deploy { project: String, deploy: DeployProjectStore },
}

pub struct MockStores {
    pub states: BTreeMap<String, ProjectStoreState>,
    pub calls: Vec<StoreCall>,
    /// A sentence the next deploy is refused with, as Cloudflare's refusal would say it.
    pub deploy_refusal: Option<String>,
}

fn state(project: &str, store: Option<ProjectStore>, deploy: StoreDeploy) -> ProjectStoreState {
    let joined = store.is_some();
    ProjectStoreState {
        project: project.into(),
        macs: if joined { MACS.map(String::from).to_vec() } else { Vec::new() },
        macs_unread: None,
        joined_here: joined,
        store,
        deploy,
        join: StoreJoin::Ready,
        token_page: TOKEN_PAGE.into(),
    }
}

fn deployed_store(team: Option<&str>) -> ProjectStore {
    let worker = team.map_or_else(|| "armada-store".to_string(), |team| format!("armada-store-{}", team.replace('/', "-")));
    ProjectStore { address: format!("https://{worker}.mele.workers.dev"), team: team.map(String::from), worker: Some(worker), deployed: Some(DEPLOYED.into()), joins_unlimited: false }
}

impl MockStores {
    /// Armada has deployed its own store and joined it; every other project names none, and this Mac has no
    /// Cloudflare sign-in a deploy could use.
    pub fn new() -> MockStores {
        let mut states = BTreeMap::new();
        states.insert("armada".into(), state("armada", Some(deployed_store(None)), StoreDeploy::Ready));
        states.insert("storefront".into(), state("storefront", None, StoreDeploy::TryTunnel));
        states.insert("ledger".into(), state("ledger", None, StoreDeploy::TryTunnel));
        MockStores { states, calls: Vec::new(), deploy_refusal: None }
    }

    pub fn read(&self, project: &str) -> Result<ProjectStoreState, Refused> {
        self.states.get(project).cloned().ok_or_else(|| no_such(project))
    }

    pub fn set(&mut self, project: &str, store: Option<JoinStore>) -> Result<ProjectStoreState, Refused> {
        self.calls.push(StoreCall::Set { project: project.into(), store: store.clone() });
        let held = self.states.get_mut(project).ok_or_else(|| no_such(project))?;
        let Some(join) = store else {
            held.store = None;
            held.macs.clear();
            held.macs_unread = None;
            held.joined_here = false;
            return Ok(held.clone());
        };
        if !join.address.starts_with("https://") {
            return Err(Refused { key: None, said: "The address must start with https:// and name a host.".into(), code: Some("fleet.store_malformed".into()) });
        }
        if join.team.as_deref() == Some(NOT_A_MEMBER) {
            return Err(Refused { key: None, said: NOT_A_MEMBER_SAID.into(), code: Some("fleet.store_not_a_member".into()) });
        }
        held.store = Some(ProjectStore { address: join.address, team: join.team, worker: None, deployed: None, joins_unlimited: false });
        held.macs = MACS.map(String::from).to_vec();
        held.macs_unread = None;
        held.joined_here = true;
        Ok(held.clone())
    }

    pub fn deploy(&mut self, project: &str, deploy: DeployProjectStore) -> Result<StoreDeployed, Refused> {
        self.calls.push(StoreCall::Deploy { project: project.into(), deploy: deploy.clone() });
        let refusal = self.deploy_refusal.clone();
        let held = self.states.get_mut(project).ok_or_else(|| no_such(project))?;
        if let Some(said) = refusal {
            return Err(Refused { key: None, said, code: Some("fleet.store_cloudflare_refused".into()) });
        }
        // A sign-in kept for the account deploys at once; the tunnel's is refused, so a token is asked for.
        if deploy.cloudflare_token.is_none() && held.deploy != StoreDeploy::Ready {
            return Ok(StoreDeployed::NeedsToken { page: held.token_page.clone(), said: NEEDS_TOKEN_SAID.into() });
        }
        held.store = Some(deployed_store(deploy.team.as_deref()));
        held.macs = MACS.map(String::from).to_vec();
        held.macs_unread = None;
        held.joined_here = true;
        held.deploy = StoreDeploy::Ready;
        Ok(StoreDeployed::Deployed { state: held.clone(), join_said: None })
    }
}

fn no_such(project: &str) -> Refused {
    Refused { key: None, said: format!("There is no project {project}."), code: Some("fleet.no_such_project".into()) }
}
