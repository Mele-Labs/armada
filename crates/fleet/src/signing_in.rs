//! Signing a machine in to an account, and the profiles it holds after.
//! `docs/concepts/account.md`.
//!
//! # The order a sign-in sets things up
//!
//! 1. The forge gives a short code; the person types it at the forge's page.
//! 2. A thread polls the forge at the interval it gave, longer after it says to
//!    slow down, until the code is typed, turned down or expired.
//! 3. The forge says who signed in. **The numeric id is the identity.** While the
//!    token is in hand it also lists the account's organisations, which are what
//!    a repository's owner is later matched against. A forge that will not list
//!    them leaves `orgs` empty and the profile marked unread, which is not a
//!    failed sign-in.
//! 4. This profile's key is enrolled with the store under a device id, and the
//!    device-flow token is dropped. **The token is a [`Secret`] held on that
//!    thread's stack and nowhere else**: it is never written, logged or kept.
//! 5. Only then does the profile join the list. A flow that ends before this
//!    leaves no profile and no directory.
//!
//! A profile signing in twice is the same profile: its login is brought up to
//! date and its device is enrolled afresh, because the store may have lost the
//! old row or moved. The old device is then signed out of the store.
//!
//! # Blocking
//!
//! The forge and the store are blocking clients, so every call here blocks, and
//! [`serving`] runs them on a blocking thread.

mod enrolling;
mod serving;

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock, PoisonError};
use std::time::{Duration, Instant};

use adapter_traits::{Polled, Secret, SignIn, SignInError, HOME_REPOSITORY};
use ipc::{
    AccountFlow, Accounts, BoundRepository, Profile, ProfileHome, SetUpState, SetUpSteps, SignInEnded, SignInOutcome, SignedOut,
    UnmatchedRepository,
};

pub use enrolling::{Enrolling, OverHttp};
pub(crate) use serving::this_machine;

use crate::clock::Clock;
use crate::profiles::{Joined, Orgs, Profiles};
use crate::repository_profiles::{binding, Binding, Choices, Repo};

/// What a sign-in is begun with: three values read from the machine's settings
/// when the person asks, so a change to them is the next sign-in's.
#[derive(Clone, Debug)]
pub struct Begin {
    /// The OAuth app's client id. Blank means none is registered.
    pub client_id: String,
    /// The central store's address. Blank means none is deployed.
    pub store_url: String,
    /// How the other machines on the account will name this one.
    pub machine: String,
}

/// Why a request on the account was not taken.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Refused {
    /// No sign-in app or no store is set, said plainly.
    Unavailable(String),
    /// The forge would not begin a sign-in.
    NotBegun(String),
    /// This machine has not joined that account.
    NoSuchAccount(u64),
    /// Fleet does not serve that repository.
    NoSuchRepository(String),
    /// Something under the account failed: the profiles could not be read or kept.
    Fault(String),
}

/// How long the thread waits between polls.
#[derive(Clone, Copy, Debug)]
pub struct Pacing {
    /// The least wait, whatever interval the forge named.
    pub floor: Duration,
    /// What a "slow down" adds to the wait, for the rest of the sign-in.
    pub slow_down: Duration,
}

impl Pacing {
    /// The forge's own: its documented five seconds more per "slow down".
    pub const fn forge() -> Pacing {
        Pacing { floor: Duration::from_secs(1), slow_down: Duration::from_secs(5) }
    }
}

/// The profile a repository claims and syncs as, and where its key is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Bound {
    pub account_id: u64,
    pub directory: PathBuf,
}

/// The flow in flight, as the thread leaves it.
enum Flow {
    Idle,
    Code { code: String, address: String, expires: Instant },
    Setup { login: String, steps: SetUpSteps },
}

struct State {
    /// Which `start` the thread belongs to. A newer one replaces an older, whose
    /// thread then ends at its next wake without touching the flow.
    generation: u64,
    flow: Flow,
    ended: Option<SignInEnded>,
}

struct Inner {
    profiles: Profiles,
    choices: Choices,
    forge: Arc<dyn SignIn + Send + Sync>,
    stores: Arc<dyn Enrolling>,
    clock: Arc<dyn Clock>,
    pacing: Pacing,
    state: Mutex<State>,
    /// A profile's shared home as `login/repository`, where this machine holds a copy of it.
    /// `crate::home` hands it in.
    home: OnceLock<HomeHere>,
}

/// Whether this machine holds a copy of a login's shared home, and its name. It is given the
/// owners a copy may be under: the login and its organisations.
type HomeHere = Arc<dyn Fn(&str, &[String]) -> Option<String> + Send + Sync>;

/// The machine's sign-in and its profiles.
#[derive(Clone)]
pub struct SigningIn {
    inner: Arc<Inner>,
}

impl SigningIn {
    /// Sign-in for the machine whose directory is `machine`, through `forge` and
    /// `stores`.
    pub fn new(
        machine: &Path,
        forge: Arc<dyn SignIn + Send + Sync>,
        stores: Arc<dyn Enrolling>,
        clock: Arc<dyn Clock>,
    ) -> SigningIn {
        SigningIn {
            inner: Arc::new(Inner {
                profiles: Profiles::in_machine_directory(machine),
                choices: Choices::in_machine_directory(machine),
                forge,
                stores,
                clock,
                pacing: Pacing::forge(),
                state: Mutex::new(State { generation: 0, flow: Flow::Idle, ended: None }),
                home: OnceLock::new(),
            }),
        }
    }

    /// The same sign-in paced as given. A test's forge answers at once, and
    /// waiting a second between its polls would only make the suite slow.
    pub fn paced(self, pacing: Pacing) -> SigningIn {
        let inner = Arc::try_unwrap(self.inner).unwrap_or_else(|_| unreachable!("paced before it is shared"));
        SigningIn { inner: Arc::new(Inner { pacing, ..inner }) }
    }

    /// Read whether a profile's shared home is here through `here`. Only the first is kept.
    pub fn knowing_home(&self, here: HomeHere) {
        let _ = self.inner.home.set(here);
    }

    /// `login`'s home where this machine holds a copy. **None for a login not read yet**, which a
    /// home cannot be named for.
    fn home_here(&self, login: &str) -> Option<String> {
        if login.is_empty() {
            return None;
        }
        let orgs = self.inner.profiles.list().ok()?.into_iter().find(|one| one.login == login).map(|one| one.orgs).unwrap_or_default();
        let owners: Vec<String> = std::iter::once(login.to_string()).chain(orgs).collect();
        self.inner.home.get().and_then(|here| here(login, &owners))
    }

    /// Every profile this machine holds.
    pub(crate) fn joined(&self) -> Result<Vec<Joined>, String> {
        self.inner.profiles.list()
    }

    fn state(&self) -> MutexGuard<'_, State> {
        self.inner.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// The profiles joined with the `repositories` each is bound to, those bound
    /// to none, and the sign-in in flight.
    pub fn accounts(&self, repositories: &[Repo]) -> Result<Accounts, Refused> {
        // The flow before the profiles: a sign-in joins its profile and then goes idle, so an idle
        // read first always lists the profile it joined.
        let flow = self.flow();
        let profiles = self.inner.profiles.list().map_err(Refused::Fault)?;
        let choices = self.inner.choices.all().map_err(Refused::Fault)?;
        let mut wired: Vec<Profile> = profiles.iter().cloned().map(|joined| self.wire(joined)).collect();
        let mut unmatched = Vec::new();
        for repo in repositories {
            match binding(&profiles, choices.get(&repo.root), repo) {
                Binding::Bound { account_id, how } => {
                    if let Some(profile) = wired.iter_mut().find(|one| one.account_id == account_id) {
                        profile.repositories.push(BoundRepository { root: repo.root.clone(), name: repo.name.clone(), how });
                    }
                }
                Binding::Unbound { candidates, none_chosen } => unmatched.push(UnmatchedRepository {
                    root: repo.root.clone(),
                    name: repo.name.clone(),
                    candidates,
                    none_chosen,
                }),
            }
        }
        Ok(Accounts { profiles: wired, unmatched, flow })
    }

    /// A profile as the wire has it. Its home is the copy here where there is one and else the
    /// default place, which Fleet replaces with what was chosen.
    fn wire(&self, joined: Joined) -> Profile {
        let home = match self.home_here(&joined.login).as_deref().and_then(|here| here.split_once('/')) {
            Some((account, repository)) => ProfileHome { account: account.to_string(), repository: repository.to_string(), made: true },
            None => ProfileHome { account: joined.login.clone(), repository: HOME_REPOSITORY.to_string(), made: false },
        };
        Profile {
            account_id: joined.account_id,
            login: joined.login,
            orgs: joined.orgs,
            chosen: joined.chosen,
            home,
            repositories: Vec::new(),
        }
    }

    fn flow(&self) -> AccountFlow {
        let state = self.state();
        match &state.flow {
            Flow::Idle => AccountFlow::Idle { ended: state.ended.clone() },
            Flow::Code { code, address, expires } => AccountFlow::Code {
                code: code.clone(),
                address: address.clone(),
                expires_in_seconds: expires.saturating_duration_since(Instant::now()).as_secs(),
            },
            Flow::Setup { login, steps } => {
                // Read as it is asked: the home's row is done once this machine holds a copy of
                // this login's own home, and waits while the forge here does not hold the login.
                let home = self.home_here(login);
                let state = if home.is_some() { SetUpState::Done } else { steps.home };
                AccountFlow::Setup {
                    login: login.clone(),
                    repository: home.unwrap_or_default(),
                    steps: SetUpSteps { home: state, ..*steps },
                }
            }
        }
    }

    /// Begin a sign-in and answer with the code to type. **Refused while either
    /// of the two settings it needs is blank**, in words that name which.
    pub fn start(&self, begin: Begin, repositories: &[Repo]) -> Result<Accounts, Refused> {
        let missing = match (begin.client_id.trim().is_empty(), begin.store_url.trim().is_empty()) {
            (false, false) => None,
            (true, false) => Some("the sign-in app"),
            (false, true) => Some("the store address"),
            (true, true) => Some("the sign-in app and the store address"),
        };
        if let Some(missing) = missing {
            let verb = if missing.contains(" and ") { "are" } else { "is" };
            return Err(Refused::Unavailable(format!("Sign-in is unavailable: {missing} {verb} blank in Settings.")));
        }
        let code = self
            .inner
            .forge
            .begin(begin.client_id.trim())
            .map_err(|why| Refused::NotBegun(why.said()))?;
        let generation = {
            let mut state = self.state();
            state.generation += 1;
            state.ended = None;
            state.flow = Flow::Code {
                code: code.user_code.clone(),
                address: code.verification_uri.clone(),
                expires: Instant::now() + Duration::from_secs(code.expires_in_seconds),
            };
            state.generation
        };
        let polling = self.clone();
        std::thread::Builder::new()
            .name("sign-in".to_string())
            .spawn(move || polling.poll_until_done(generation, &begin, &code))
            .map_err(|why| Refused::Fault(format!("The sign-in could not be followed: {why}")))?;
        self.accounts(repositories)
    }

    /// Sign a profile out of the store and off this machine. **The local profile
    /// goes whatever the store says**: a person signing out is not held by a
    /// store that is down, and is told its row remains.
    pub fn sign_out(&self, account_id: u64, store_url: &str, repositories: &[Repo]) -> Result<SignedOut, Refused> {
        let profiles = &self.inner.profiles;
        if !profiles.has(account_id).map_err(Refused::Fault)? {
            return Err(Refused::NoSuchAccount(account_id));
        }
        let directory = profiles.directory_of(account_id);
        let left = match adapters::device_id(&directory) {
            Ok(None) => None,
            Ok(Some(_)) if store_url.trim().is_empty() => {
                Some("No store address is set, so the store was not asked.".to_string())
            }
            Ok(Some(device)) => self
                .inner
                .stores
                .delete_device(store_url.trim(), &directory, &device)
                .err()
                .map(|why| why.said()),
            // The profile still goes; what the store holds is left for a person to clear.
            Err(why) => Some(why),
        };
        profiles.leave(account_id).map_err(Refused::Fault)?;
        Ok(SignedOut { accounts: self.accounts(repositories)?, store_row_remains: left.is_some(), said: left })
    }

    /// The profile `repo` claims and syncs as. **The one function the store's
    /// holders ask**: `None` for a repository that is bound to no profile, was
    /// given "None", or whose profile this machine has signed out of, and then
    /// there is no store, as with the switch off. Also `None` where the lists
    /// cannot be read.
    pub fn profile_for(&self, repo: &Repo) -> Option<Bound> {
        let joined = self.inner.profiles.list().ok()?;
        let choices = self.inner.choices.all().ok()?;
        match binding(&joined, choices.get(&repo.root), repo) {
            Binding::Bound { account_id, .. } => {
                Some(Bound { account_id, directory: self.inner.profiles.directory_of(account_id) })
            }
            Binding::Unbound { .. } => None,
        }
    }

    /// The login of the profile `repo` is bound to, **the account its pull requests are mine by**.
    /// `None` as [`profile_for`](Self::profile_for) is.
    pub fn login_for(&self, repo: &Repo) -> Option<String> {
        let joined = self.inner.profiles.list().ok()?;
        let choices = self.inner.choices.all().ok()?;
        match binding(&joined, choices.get(&repo.root), repo) {
            Binding::Bound { account_id, .. } => {
                joined.iter().find(|one| one.account_id == account_id).map(|one| one.login.clone())
            }
            Binding::Unbound { .. } => None,
        }
    }

    /// Bind the repository at `root` to the profile `account_id`, or to none.
    /// **A choice wins over a match.** Refused for a root that is not among
    /// `repositories` and for an account this machine has not joined.
    pub fn set_repository_profile(
        &self,
        root: &str,
        account_id: Option<u64>,
        repositories: &[Repo],
    ) -> Result<Accounts, Refused> {
        if !repositories.iter().any(|repo| repo.root == root) {
            return Err(Refused::NoSuchRepository(root.to_string()));
        }
        if let Some(id) = account_id {
            if !self.inner.profiles.has(id).map_err(Refused::Fault)? {
                return Err(Refused::NoSuchAccount(id));
            }
        }
        self.inner.choices.choose(root, account_id).map_err(Refused::Fault)?;
        self.accounts(repositories)
    }

    fn current(&self, generation: u64) -> bool {
        self.state().generation == generation
    }

    /// Put the flow right, if this thread's sign-in is still the one in flight.
    fn flow_is(&self, generation: u64, flow: Flow) {
        let mut state = self.state();
        if state.generation == generation {
            state.flow = flow;
        }
    }

    fn end(&self, generation: u64, outcome: SignInOutcome, said: Option<String>) {
        let mut state = self.state();
        if state.generation == generation {
            state.flow = Flow::Idle;
            state.ended = Some(SignInEnded { outcome, said });
        }
    }

    fn poll_until_done(&self, generation: u64, begin: &Begin, code: &adapter_traits::DeviceCode) {
        let started = Instant::now();
        let expires = Duration::from_secs(code.expires_in_seconds);
        let pacing = self.inner.pacing;
        let mut wait = Duration::from_secs(code.interval_seconds).max(pacing.floor);
        let mut last_unreachable: Option<String> = None;
        loop {
            std::thread::sleep(wait);
            if !self.current(generation) {
                return;
            }
            if started.elapsed() >= expires {
                return match last_unreachable {
                    Some(said) => self.end(generation, SignInOutcome::Unreachable, Some(said)),
                    None => self.end(generation, SignInOutcome::Expired, Some("The code ran out before it was typed.".to_string())),
                };
            }
            match self.inner.forge.poll(begin.client_id.trim(), &code.device_code) {
                Ok(Polled::Pending) => last_unreachable = None,
                Ok(Polled::SlowDown) => {
                    last_unreachable = None;
                    wait += pacing.slow_down;
                }
                Ok(Polled::Granted(token)) => return self.finish(generation, begin, token),
                Ok(Polled::Expired) => {
                    return self.end(generation, SignInOutcome::Expired, Some("The code ran out before it was typed.".to_string()))
                }
                Ok(Polled::Denied) => {
                    return self.end(generation, SignInOutcome::Denied, Some("The sign-in was turned down.".to_string()))
                }
                // A network blip is waited out until the code runs out.
                Err(SignInError::Unreachable(why)) => last_unreachable = Some(SignInError::Unreachable(why).said()),
                Err(why) => return self.end(generation, SignInOutcome::Refused, Some(why.said())),
            }
        }
    }

    /// The code was typed: read who signed in, enrol this profile's key, and
    /// take the profile in. **`token` is dropped when this returns.**
    fn finish(&self, generation: u64, begin: &Begin, token: Secret<String>) {
        let steps = |identity, store| SetUpSteps { identity, store, home: SetUpState::Waiting };
        self.flow_is(
            generation,
            Flow::Setup { login: String::new(), steps: steps(SetUpState::Working, SetUpState::Waiting) },
        );
        let account = match self.inner.forge.who(&token) {
            Ok(account) => account,
            Err(why) => return self.end(generation, outcome_of(&why), Some(why.said())),
        };
        // Read now: the token is dropped before the profile joins, and is not asked again.
        let orgs = self.inner.forge.organisations(&token).map_or(Orgs::Unread, Orgs::Read);
        self.flow_is(
            generation,
            Flow::Setup { login: account.login.clone(), steps: steps(SetUpState::Done, SetUpState::Working) },
        );
        let profiles = &self.inner.profiles;
        let rejoining = match profiles.has(account.id) {
            Ok(has) => has,
            Err(why) => return self.end(generation, SignInOutcome::Refused, Some(why)),
        };
        let directory = match profiles.make_directory(account.id) {
            Ok(directory) => directory,
            Err(why) => return self.end(generation, SignInOutcome::Refused, Some(why)),
        };
        let before = adapters::device_id(&directory).ok().flatten();
        let url = begin.store_url.trim();
        let device = match self.inner.stores.enrol(url, &directory, &token, &begin.machine) {
            Ok(device) => device,
            Err(why) => {
                if !rejoining {
                    profiles.remove_directory(account.id).ok();
                }
                return self.end(generation, outcome_of_store(&why), Some(why.said()));
            }
        };
        drop(token);
        if let Err(why) = profiles.join(&account, orgs, self.inner.clock.now().as_str()) {
            self.inner.stores.delete_device(url, &directory, &device).ok();
            if !rejoining {
                profiles.remove_directory(account.id).ok();
            }
            return self.end(generation, SignInOutcome::Refused, Some(why));
        }
        // The old device is the same key under an id the store may still hold.
        if let Some(old) = before.filter(|old| *old != device) {
            self.inner.stores.delete_device(url, &directory, &old).ok();
        }
        self.end(generation, SignInOutcome::SignedIn, None);
    }
}


fn outcome_of(why: &SignInError) -> SignInOutcome {
    if why.is_unreachable() {
        SignInOutcome::Unreachable
    } else {
        SignInOutcome::Refused
    }
}

fn outcome_of_store(why: &adapter_traits::StoreError) -> SignInOutcome {
    if why.is_unreachable() {
        SignInOutcome::Unreachable
    } else {
        SignInOutcome::Refused
    }
}
