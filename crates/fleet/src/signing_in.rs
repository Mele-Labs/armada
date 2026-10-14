//! Signing a machine in to an account, and the profiles it holds after.
//! `docs/concepts/account.md`.
//!
//! # The order a sign-in sets things up
//!
//! 1. The forge gives a short code; the person types it at the forge's page.
//! 2. A thread polls the forge at the interval it gave, longer after it says to
//!    slow down, until the code is typed, turned down or expired.
//! 3. The forge says who signed in. **The numeric id is the identity.**
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

use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use adapter_traits::{Polled, Secret, SignIn, SignInError};
use ipc::{AccountFlow, Accounts, Profile, SetUpState, SetUpSteps, SignInEnded, SignInOutcome, SignedOut};

pub use enrolling::{Enrolling, OverHttp};
#[cfg(test)]
pub(crate) use serving::machine_name;

use crate::clock::Clock;
use crate::profiles::{Joined, Profiles};

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
    forge: Arc<dyn SignIn + Send + Sync>,
    stores: Arc<dyn Enrolling>,
    clock: Arc<dyn Clock>,
    pacing: Pacing,
    state: Mutex<State>,
}

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
                forge,
                stores,
                clock,
                pacing: Pacing::forge(),
                state: Mutex::new(State { generation: 0, flow: Flow::Idle, ended: None }),
            }),
        }
    }

    /// The same sign-in paced as given. A test's forge answers at once, and
    /// waiting a second between its polls would only make the suite slow.
    pub fn paced(self, pacing: Pacing) -> SigningIn {
        let inner = Arc::try_unwrap(self.inner).unwrap_or_else(|_| unreachable!("paced before it is shared"));
        SigningIn { inner: Arc::new(Inner { pacing, ..inner }) }
    }

    fn state(&self) -> MutexGuard<'_, State> {
        self.inner.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// The profiles joined, and the sign-in in flight.
    pub fn accounts(&self) -> Result<Accounts, Refused> {
        let profiles = self.inner.profiles.list().map_err(Refused::Fault)?;
        let state = self.state();
        let flow = match &state.flow {
            Flow::Idle => AccountFlow::Idle { ended: state.ended.clone() },
            Flow::Code { code, address, expires } => AccountFlow::Code {
                code: code.clone(),
                address: address.clone(),
                expires_in_seconds: expires.saturating_duration_since(Instant::now()).as_secs(),
            },
            Flow::Setup { login, steps } => AccountFlow::Setup {
                login: login.clone(),
                // The shared home does not exist yet, so there is no repository to name.
                repository: String::new(),
                steps: *steps,
            },
        };
        drop(state);
        Ok(Accounts { profiles: profiles.into_iter().map(wire).collect(), flow })
    }

    /// Begin a sign-in and answer with the code to type. **Refused while either
    /// of the two settings it needs is blank**, in words that name which.
    pub fn start(&self, begin: Begin) -> Result<Accounts, Refused> {
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
        self.accounts()
    }

    /// Sign a profile out of the store and off this machine. **The local profile
    /// goes whatever the store says**: a person signing out is not held by a
    /// store that is down, and is told its row remains.
    pub fn sign_out(&self, account_id: u64, store_url: &str) -> Result<SignedOut, Refused> {
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
        Ok(SignedOut { accounts: self.accounts()?, store_row_remains: left.is_some(), said: left })
    }

    /// Choose the profile the machine acts as.
    pub fn set_active(&self, account_id: u64) -> Result<Accounts, Refused> {
        if !self.inner.profiles.activate(account_id).map_err(Refused::Fault)? {
            return Err(Refused::NoSuchAccount(account_id));
        }
        self.accounts()
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
        if let Err(why) = profiles.join(&account, self.inner.clock.now().as_str()) {
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

fn wire(joined: Joined) -> Profile {
    Profile { account_id: joined.account_id, login: joined.login, active: joined.active, chosen: joined.chosen }
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
