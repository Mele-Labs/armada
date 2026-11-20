//! Which claims store a repository's calls go to: **the one its projects name, or none.**
//! `docs/contracts/central-store.md`, *A store per project*.
//!
//! There is no machine-wide address. A repository in no project, or in projects that name no store,
//! makes no call, and that holds for what was waiting to be sent too, because the store is read when
//! a call is made and never kept beside it. A repository in two projects that name two different
//! stores also makes none: nothing says which the person meant, so it is said on the log and no claim
//! leaves for either.
//!
//! **Who joins it is the login the project picks** (`store.account_id`), and each repository claims as
//! that login unless it overrides it (`crate::repository_profiles`). A project that picks none sends
//! nothing, **unless exactly one profile here already enrolled at its store**: `store_logins_carried` writes that
//! one in. Projects naming one store with two different logins pick none between them.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use adapter_traits::{AgentHarness, CloudflareError, Delivery, Secret, StoreError, Vcs, WorkProduct};
use api::Refusal;
use config::settings as keys;
use core_model::{MalformedStore, ProjectStore, StoreAddress, StoreTeam};
use ipc::WireError;
use sha2::{Digest, Sha256};

use crate::daemon::Fleet;
use crate::red_claim::Reaching;
use crate::signing_in::{Bound, SigningIn, this_machine};
use crate::store_deploy::{Deployed, worker_name};

const STORE_MALFORMED: &str = "fleet.store_malformed";
const STORE_NO_PROFILE: &str = "fleet.store_no_profile";
const STORE_NEEDS_FORGE: &str = "fleet.store_needs_forge";
const STORE_NOT_A_MEMBER: &str = "fleet.store_not_a_member";
const STORE_ORG_RESTRICTED: &str = "fleet.store_org_restricted";
const STORE_UNREACHABLE: &str = "fleet.store_unreachable";
const STORE_REFUSED: &str = "fleet.store_refused";
const STORE_NO_WORKERS_DEV: &str = "fleet.store_no_workers_dev";
const STORE_CLOUDFLARE_REFUSED: &str = "fleet.store_cloudflare_refused";
const STORE_CLOUDFLARE_UNREACHABLE: &str = "fleet.store_cloudflare_unreachable";
const NO_LOGIN: &str = "No login is picked for this store.";
const LOGIN_NOT_HERE: &str = "This Mac is not signed in as the login picked for this store.";

/// A project's store as the wire carries it.
pub(crate) fn store_on_the_wire(store: &ProjectStore) -> ipc::ProjectStore {
    ipc::ProjectStore {
        address: store.address.as_str().to_string(),
        team: store.team.as_ref().map(ToString::to_string),
        worker: store.worker.as_ref().map(|worker| worker.as_str().to_string()),
        deployed: store.deployed.as_ref().map(|at| at.as_str().to_string()),
        joins_unlimited: store.joins_unlimited,
        account_id: store.account_id,
    }
}

/// Where a profile keeps its key and device id for the store at `address`: a folder of its own under
/// the profile's, named for the address, so one profile enrolled with two stores keeps two devices.
pub(crate) fn store_directory(profile: &Path, address: &StoreAddress) -> PathBuf {
    let digest = Sha256::digest(address.as_str().as_bytes());
    let name: String = digest.iter().take(8).map(|byte| format!("{byte:02x}")).collect();
    profile.join("stores").join(name)
}

/// A store a project here names that this Mac joined, as the Phone Gateway reaches it for pairing through his
/// store.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JoinedStore {
    pub address: String,
    /// The project names a team for it, so it is not his alone.
    pub team: bool,
    /// The folder of the first profile joined to it: its key and device id.
    pub directory: PathBuf,
}

/// **Who this Mac joins as**: the project's `login` first, then each login a repository at `roots`
/// overrides it with, each once, since each repository's claims go as its own profile. Why none, where
/// the project picked no login or this Mac is not signed in to it. Reads files only.
fn bound_profiles(signing: &SigningIn, roots: &[String], login: Option<u64>) -> Result<Vec<Bound>, &'static str> {
    let own = signing.profile(login.ok_or(NO_LOGIN)?).ok_or(LOGIN_NOT_HERE)?;
    let mut bound = vec![own];
    for profile in roots.iter().filter_map(|root| signing.profile_for(root, login)) {
        if !bound.iter().any(|one| one.account_id == profile.account_id) {
            bound.push(profile);
        }
    }
    Ok(bound)
}

/// Why this Mac could not join a store.
enum Joining {
    NoLogin(&'static str),
    Store(StoreError),
}

/// What the store setting reads, off the async threads: who this Mac is to the store, and its Macs.
fn state_read(reaching: &dyn Reaching, signing: &SigningIn, roots: &[String], store: Option<&ProjectStore>) -> (bool, Vec<String>, Option<String>, bool) {
    // Any profile here can be picked to join as.
    let can_join = signing.joined().is_ok_and(|joined| !joined.is_empty());
    let Some(store) = store else { return (false, Vec::new(), None, can_join) };
    let bound = match bound_profiles(signing, roots, store.account_id) {
        Ok(bound) => bound,
        Err(said) => return (false, Vec::new(), Some(said.to_string()), can_join),
    };
    // The Macs are asked as the first profile that joined; one that has not joined cannot ask.
    let joined = bound
        .iter()
        .map(|profile| store_directory(&profile.directory, &store.address))
        .find(|directory| matches!(adapters::device_id(directory), Ok(Some(_))));
    let Some(directory) = joined else {
        return (false, Vec::new(), Some("This Mac has not joined it.".to_string()), can_join);
    };
    match reaching.open(store.address.as_str(), &directory) {
        Ok(Some(opened)) => match opened.signed_in_devices() {
            Ok(devices) => (true, devices.into_iter().map(|device| device.machine).collect(), None, can_join),
            Err(why) => (true, Vec::new(), Some(why.said()), can_join),
        },
        Ok(None) => (true, Vec::new(), Some("This Mac's key for it could not be read.".to_string()), can_join),
        Err(why) => (true, Vec::new(), Some(why), can_join),
    }
}

/// What the projects holding one repository say of its store.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum StoreOf {
    /// No project holds it, or none that does names a store.
    None,
    /// The store, and the login the projects naming it picked. `None` where none picked one, or two
    /// picked different logins.
    One(StoreAddress, Option<u64>),
    /// Two projects holding it name different stores.
    Differs,
}

/// The store of the repository `manifest` from the projects in `projects`.
pub(crate) fn store_of<'p>(projects: impl Iterator<Item = &'p core_model::Project>, manifest: &core_model::ManifestId) -> StoreOf {
    let mut found: Option<&StoreAddress> = None;
    let mut logins: Vec<u64> = Vec::new();
    for project in projects.filter(|project| project.holds(manifest)) {
        let Some(store) = project.store() else { continue };
        match found {
            Some(already) if already != &store.address => return StoreOf::Differs,
            _ => found = Some(&store.address),
        }
        if let Some(login) = store.account_id.filter(|login| !logins.contains(login)) {
            logins.push(login);
        }
    }
    let login = match logins[..] {
        [only] => Some(only),
        _ => None,
    };
    found.map_or(StoreOf::None, |address| StoreOf::One(address.clone(), login))
}

impl<H, V, W> Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    /// What the projects holding the repository at `root` **as Fleet holds them now** say of its store.
    fn store_of_held(&self, root: &str) -> (Option<core_model::ManifestId>, StoreOf) {
        let Some(manifest) = self.manifest_id_at(root) else { return (None, StoreOf::None) };
        let manifest = core_model::ManifestId::carried(core_model::Ulid::carried(manifest));
        let of = store_of(self.projects_held().running(), &manifest);
        (Some(manifest), of)
    }

    /// The login the projects holding the repository at `root` picked for the store it calls, for
    /// [`SigningIn::profile_for`]. Not said on the log: [`Self::store_of_root`] says it for every call.
    pub(crate) fn project_login_of_root(&self, root: &str) -> Option<u64> {
        match self.store_of_held(root).1 {
            StoreOf::One(_, login) => login,
            StoreOf::None | StoreOf::Differs => None,
        }
    }

    /// The store the repository at `root` calls and the login its projects picked for it, from the
    /// projects that hold it **as Fleet holds them now**. `None` makes no call.
    pub(crate) fn store_of_root(&self, root: &str) -> Option<(StoreAddress, Option<u64>)> {
        let (manifest, of) = self.store_of_held(root);
        match of {
            StoreOf::One(address, login) => Some((address, login)),
            StoreOf::None => None,
            StoreOf::Differs => {
                self.values_warned(
                    "a repository's claims were not sent",
                    manifest.as_ref().map(core_model::ManifestId::as_str),
                    "two projects that hold it name different stores",
                );
                None
            }
        }
    }

    /// Every store the projects here name that this Mac joined, in project order, each once. Reads files only.
    pub fn stores_joined_here(&self) -> Vec<JoinedStore> {
        let held = self.projects_held();
        let mut joined: Vec<JoinedStore> = Vec::new();
        for store in held.running().filter_map(core_model::Project::store) {
            if joined.iter().any(|one| one.address == store.address.as_str()) {
                continue;
            }
            if let Some(directory) = self.signing_in().joined_directory(&store.address) {
                joined.push(JoinedStore { address: store.address.as_str().to_string(), team: store.team.is_some(), directory });
            }
        }
        joined
    }

    /// **A store written before a project picked a login takes the one profile here that enrolled at it.** Each
    /// project of this Mac's in `projects` that names a store and no login, and whose store exactly one profile
    /// enrolled at, is written with that profile as its login, in the place its file is kept. None or several
    /// enrolled leaves it. A copy of another Mac's is never in `projects`, so is never written. Whether any was.
    pub(crate) async fn store_logins_carried(&self, projects: &config::Projects) -> bool {
        let mut written = false;
        for project in projects.running() {
            let Some(store) = project.store().filter(|store| store.account_id.is_none()) else { continue };
            let [login] = self.signing_in().enrolled_at(&store.address)[..] else { continue };
            let named = project.clone().with_store(Some(ProjectStore { account_id: Some(login), ..store.clone() }));
            match self.project_stored(&named).await {
                Ok(()) => written = true,
                Err(refused) => self.values_warned("a project's store login was not carried over", Some(project.id().as_str()), &refused.error().message),
            }
        }
        written
    }

    /// The sign-in app's client id, as the machine's settings hold it now. Blank where none is set.
    pub fn sign_in_client_id(&self) -> String {
        self.machine_settings().now().get(keys::ACCOUNT_CLIENT_ID)
    }

    /// Name `store` as project `project`'s, or none, in its file, and answer the projects as every
    /// write does. **Nothing is called here**: a join or a deploy reaches the store first.
    pub(crate) async fn project_store_written(&self, project: &str, store: Option<ProjectStore>) -> Result<ipc::Projects, Refusal> {
        self.store_not_held(project).await?;
        let held = self.project_read(project)?;
        let named = held.clone().with_store(store);
        if named != held {
            self.project_stored(&named).await?;
        }
        self.projects_written_reading(|_, _| Ok(()), true).await
    }

    /// The roots of the served repositories project `project` holds, in served order.
    pub(crate) fn project_roots(&self, project: &core_model::Project) -> Vec<String> {
        let served = self.repositories().served();
        served.iter().filter(|one| project.holds(one.manifest().id())).map(|one| one.root().to_string()).collect()
    }

    /// Everything the store setting of `project` shows, a copy of another Mac's included.
    pub(crate) async fn project_store_state(&self, project: &str) -> Result<ipc::ProjectStoreState, Refusal> {
        let held = self.project_held(project).await?;
        let roots = self.project_roots(&held);
        let reaching = Arc::clone(self.reaching());
        let signing = self.signing_in().clone();
        let deploying = self.deploying().clone();
        let store = held.store().cloned();
        let named = store.clone();
        let ((joined_here, macs, macs_unread, can_join), deploy) = tokio::task::spawn_blocking(move || {
            (state_read(reaching.as_ref(), &signing, &roots, named.as_ref()), deploying.readiness())
        })
        .await
        .map_err(|why| Refusal::Fault(self.project_refusal(STORE_UNREACHABLE, format!("reading the store was abandoned: {why}"))))?;
        Ok(ipc::ProjectStoreState {
            project: project.to_string(),
            store: store.as_ref().map(store_on_the_wire),
            macs,
            macs_unread,
            joined_here,
            deploy,
            join: if can_join { ipc::StoreJoin::Ready } else { ipc::StoreJoin::NoProfile },
            token_page: self.deploying().token_page.clone(),
        })
    }

    /// Join the store `set` names as the login it picks, or the one the project picked, and name both as
    /// `project`'s, or name none. **Nothing is written unless the join worked.**
    pub(crate) async fn project_store_set(&self, project: &str, set: ipc::SetProjectStore) -> Result<ipc::ProjectStoreState, Refusal> {
        self.store_not_held(project).await?;
        let held = self.project_read(project)?;
        let named = match set.store {
            None => None,
            Some(join) => {
                let address = StoreAddress::new(&join.address).map_err(|why| self.store_malformed(&why))?;
                let team = join.team.as_deref().map(StoreTeam::new).transpose().map_err(|why| self.store_malformed(&why))?;
                let login = join.account_id.or_else(|| held.store().and_then(|kept| kept.account_id));
                self.store_joined(&held, &address, login).await?;
                let store = match held.store() {
                    // The same store as before keeps what its deploy wrote.
                    Some(kept) if kept.address == address && kept.team == team => kept.clone(),
                    _ => ProjectStore::joined(address, team),
                };
                Some(ProjectStore { account_id: login, ..store })
            }
        };
        self.project_store_written(project, named).await?;
        self.project_store_state(project).await
    }

    /// Put `project`'s store up on his Cloudflare account, name it in the project's file, and join this
    /// Mac to it. **Nothing is changed where Cloudflare refuses every sign-in.**
    pub(crate) async fn project_store_deployed(&self, project: &str, deploy: ipc::DeployProjectStore) -> Result<ipc::StoreDeployed, Refusal> {
        self.store_not_held(project).await?;
        let held = self.project_read(project)?;
        let team = deploy.team.as_deref().map(StoreTeam::new).transpose().map_err(|why| self.store_malformed(&why))?;
        let name = worker_name(team.as_ref());
        let pasted = deploy.cloudflare_token.map(|token| token.trim().to_string()).filter(|token| !token.is_empty()).map(Secret::new);
        let deploying = self.deploying().clone();
        let (worker, as_team) = (name.clone(), team.clone());
        let deployed = tokio::task::spawn_blocking(move || deploying.deploy(&worker, as_team.as_ref(), pasted))
            .await
            .map_err(|why| CloudflareError::Unreachable(format!("the deploy was abandoned: {why}")))
            .and_then(|deployed| deployed)
            .map_err(|why| self.cloudflare_refusal(&why))?;
        let up = match deployed {
            Deployed::Up(up) => up,
            Deployed::NeedsToken(said) => return Ok(ipc::StoreDeployed::NeedsToken { page: self.deploying().token_page.clone(), said }),
        };
        let address = StoreAddress::new(&up.address).map_err(|why| {
            Refusal::Unacceptable(self.project_refusal(STORE_CLOUDFLARE_REFUSED, format!("Cloudflare answered an address Fleet cannot use: {why}")))
        })?;
        // A redeploy keeps the login the project picked.
        let login = held.store().and_then(|kept| kept.account_id);
        let store =
            ProjectStore { address: address.clone(), team, worker: Some(name), deployed: Some(self.now()), joins_unlimited: !up.limited, account_id: login };
        self.project_store_written(project, Some(store)).await?;
        let join_said = self.store_joined(&held, &address, login).await.err().map(|refused| refused.error().message.clone());
        Ok(ipc::StoreDeployed::Deployed { state: self.project_store_state(project).await?, join_said })
    }

    /// Join this Mac to the store at `address` as `login`, the project's, and each login a repository of
    /// `held` overrides it with. **Stops at the first refusal**; a profile already joined is left as it is.
    async fn store_joined(&self, held: &core_model::Project, address: &StoreAddress, login: Option<u64>) -> Result<(), Refusal> {
        let roots = self.project_roots(held);
        let signing = self.signing_in().clone();
        let configured: String = self.machine_settings().now().get(keys::MACHINE_NAME);
        let address = address.clone();
        let joined = tokio::task::spawn_blocking(move || {
            let bound = bound_profiles(&signing, &roots, login).map_err(Joining::NoLogin)?;
            let machine = this_machine(&configured);
            bound.iter().try_for_each(|profile| signing.join_store(profile.account_id, &address, &machine).map(drop).map_err(Joining::Store))
        })
        .await
        .unwrap_or_else(|why| Err(Joining::Store(StoreError::Unreachable(format!("the join was abandoned: {why}")))));
        joined.map_err(|why| self.join_refusal(why))?;
        // The Gateway says where this Mac is reached to the store it joined, so his other devices find it.
        if let Ok(switch) = self.phone_switch() {
            switch.stores_joined();
        }
        Ok(())
    }

    fn store_malformed(&self, why: &MalformedStore) -> Refusal {
        Refusal::Unacceptable(self.project_refusal(STORE_MALFORMED, why.to_string()))
    }

    /// A join's refusal, by the store's own code where it said one.
    fn join_refusal(&self, why: Joining) -> Refusal {
        let error = |code: &'static str, said: String| -> WireError { self.project_refusal(code, said) };
        match why {
            Joining::NoLogin(said) => Refusal::Unacceptable(error(STORE_NO_PROFILE, said.to_string())),
            Joining::Store(StoreError::Unreachable(said)) => Refusal::Fault(error(STORE_UNREACHABLE, format!("The store did not answer: {said}"))),
            Joining::Store(StoreError::Refused(said)) => {
                let code = if said.contains("gh is not signed in") || said.contains("gh could not be started") {
                    STORE_NEEDS_FORGE
                } else if said.contains("not_a_member") {
                    STORE_NOT_A_MEMBER
                } else if said.contains("org_restricted") {
                    STORE_ORG_RESTRICTED
                } else {
                    STORE_REFUSED
                };
                Refusal::Unacceptable(error(code, said))
            }
        }
    }

    fn cloudflare_refusal(&self, why: &CloudflareError) -> Refusal {
        let said = why.said();
        match why {
            CloudflareError::NoWorkersDev => Refusal::Unacceptable(self.project_refusal(STORE_NO_WORKERS_DEV, said)),
            CloudflareError::Unreachable(_) => Refusal::Fault(self.project_refusal(STORE_CLOUDFLARE_UNREACHABLE, said)),
            CloudflareError::Refused(_) | CloudflareError::LimitRefused(_) | CloudflareError::Failed(_) => {
                Refusal::Unacceptable(self.project_refusal(STORE_CLOUDFLARE_REFUSED, said))
            }
        }
    }
}
