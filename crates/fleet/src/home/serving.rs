//! Fleet's side of the shared home's seven operations: reads what syncs and Kit's servers out of
//! the store, runs the folder and forge work off the async workers, and spells a refusal.

use std::path::Path;
use std::sync::{Arc, OnceLock};

use adapter_traits::{
    AgentHarness, Delivery, HarnessHome, HomeFailure, HomeRepositories, HomeRepository, KeptFolder, Vcs, WorkProduct,
};
use api::{Refusal, SharedHome};
use config::settings as keys;
use ipc::{ChooseHomeSide, HomeKind, HomeKinds, HomeProfile, HomeRowAt, HomeSide, SetHomeKind, SharedHomeRead, WireError};

use super::{Places, Refused, Repository};
use crate::daemon::Fleet;

/// This Fleet was started without the shared home's adapters: a test's.
const HOME_NOT_SERVED: &str = "fleet.home_not_served";
/// An act on a row while this machine holds no copy of the home. A 409: Sync first.
const NO_HOME_YET: &str = "fleet.no_home_yet";
const NO_SUCH_HOME_ROW: &str = "fleet.no_such_home_row";
/// A home asked for by a login that is not a profile on this machine.
const NO_SUCH_PROFILE: &str = "fleet.no_such_profile";
/// The forge here does not hold the profile's login: the reason names the login and how to add it.
const HOME_LOGIN_NOT_HELD: &str = "fleet.home_login_not_held";
const HOME_ROW_DIFFERS: &str = "fleet.home_row_differs";
const HOME_SECRET_REFUSED: &str = "fleet.home_secret_refused";
const HOME_REFUSED: &str = "fleet.home_refused";
const HOME_UNREACHABLE: &str = "fleet.home_unreachable";
const HOME_NOT_KEPT: &str = "fleet.home_not_kept";

/// What the composition root hands Fleet for the shared home.
pub struct HomeFittings {
    /// Every profile's home, a login each.
    pub homes: Arc<dyn HomeRepositories>,
    /// The machine's mods folder.
    pub mods: Arc<dyn KeptFolder>,
    /// Kit's home.
    pub kit: Arc<dyn KeptFolder>,
    pub harness: Arc<dyn HarnessHome>,
}

/// The fittings, once handed in.
#[derive(Default)]
pub(crate) struct HomeHeld {
    fittings: OnceLock<Arc<HomeFittings>>,
}

/// Everything an act reads before it leaves the async workers.
struct Read {
    fittings: Arc<HomeFittings>,
    /// The asked profile's home.
    home: Arc<dyn HomeRepository>,
    servers: Vec<core_model::KitServer>,
    repositories: Vec<Repository>,
    on: Vec<HomeKind>,
    machine: String,
    mods_dir: String,
}

impl Read {
    fn places(&self) -> Places<'_> {
        Places {
            home: self.home.as_ref(),
            mods: self.fittings.mods.as_ref(),
            kit: self.fittings.kit.as_ref(),
            harness: self.fittings.harness.as_ref(),
            servers: &self.servers,
            repositories: &self.repositories,
        }
    }

    /// Refuse a mod that is not valid here before it travels: `crate::mods`' own reading of it.
    fn valid_mod(&self, at: &HomeRowAt) -> Result<(), Refused> {
        if at.kind != HomeKind::Mods {
            return Ok(());
        }
        let examined = crate::mods::examine(Path::new(&self.mods_dir), &at.name);
        match examined.problems.first() {
            Some(problem) => Err(Refused::Cannot(format!("The mod {} is not valid here: {problem}", at.name))),
            None => Ok(()),
        }
    }
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
    /// Hand Fleet the shared home's adapters. Only the first is kept. **Signing in reads whether
    /// a profile's home is here through them too**, so its set-up row is done once it is.
    pub fn sharing_home_with(&self, fittings: HomeFittings) {
        let homes = Arc::clone(&fittings.homes);
        if self.home_held().fittings.set(Arc::new(fittings)).is_ok() {
            self.signing_in().knowing_home(Arc::new(move |login: &str| homes.of(login).ok()?.here()));
        }
    }

    fn home_refusal(&self, refused: Refused) -> Refusal {
        let run = self.run_id();
        let said = refused.said();
        match refused {
            Refused::NotThere(_) => Refusal::NotFound(WireError::raised(NO_SUCH_HOME_ROW, said, run)),
            Refused::NoProfile(_) => Refusal::NotFound(WireError::raised(NO_SUCH_PROFILE, said, run)),
            Refused::Home(HomeFailure::NotSignedIn(_)) => Refusal::Unacceptable(WireError::raised(HOME_LOGIN_NOT_HELD, said, run)),
            Refused::Differs(_) => Refusal::IllegalMove(WireError::raised(HOME_ROW_DIFFERS, said, run)),
            Refused::Secret(_) => Refusal::Unacceptable(WireError::raised(HOME_SECRET_REFUSED, said, run)),
            Refused::Home(HomeFailure::Unreachable(_)) => Refusal::Fault(WireError::raised(HOME_UNREACHABLE, said, run)),
            Refused::Cannot(_) | Refused::Home(_) => Refusal::Unacceptable(WireError::raised(HOME_REFUSED, said, run)),
        }
    }

    fn fittings(&self) -> Result<Arc<HomeFittings>, Refusal> {
        self.home_held().fittings.get().cloned().ok_or_else(|| {
            Refusal::IllegalMove(WireError::raised(HOME_NOT_SERVED, "this Fleet was started without the shared home", self.run_id()))
        })
    }

    /// The logins of this machine's profiles.
    fn profile_logins(&self) -> Result<Vec<String>, Refusal> {
        self.signing_in().logins().map_err(|why| self.home_not_kept(format!("the profiles could not be read: {why}")))
    }

    /// Refuse a login that is not a profile here.
    fn a_profile(&self, login: &str) -> Result<(), Refusal> {
        match self.profile_logins()?.iter().any(|profile| profile == login) {
            true => Ok(()),
            false => Err(self.home_refusal(Refused::NoProfile(format!("{login} is not a profile on this machine")))),
        }
    }

    /// `login`'s home, refused where it is not a profile here.
    fn home_for(&self, fittings: &HomeFittings, login: &str) -> Result<Arc<dyn HomeRepository>, Refusal> {
        super::home_of(fittings.homes.as_ref(), login, &self.profile_logins()?).map_err(|refused| self.home_refusal(refused))
    }

    async fn home_read(&self, login: &str) -> Result<Read, Refusal> {
        let fittings = self.fittings()?;
        let home = self.home_for(&fittings, login)?;
        let (servers, switched) = {
            let store = self.store().lock().await;
            // Not defaulted: a server read as missing here would read as only in the home.
            let servers = store.kit_servers().map_err(|why| self.home_not_kept(format!("{why:?}")))?;
            (servers, store.home_kinds(login).map_err(|why| self.home_not_kept(why))?)
        };
        let repositories = self
            .repositories()
            .every()
            .iter()
            .map(|one| Repository {
                name: Path::new(one.root()).file_name().map(|name| name.to_string_lossy().to_string()).unwrap_or_default(),
                root: one.root().to_string(),
            })
            .collect();
        Ok(Read {
            fittings,
            home,
            servers,
            repositories,
            on: super::on(&switched),
            machine: self.machine_settings().now().get(keys::MACHINE_NAME),
            mods_dir: self.host().mods_dir.clone(),
        })
    }

    fn home_not_kept(&self, why: impl std::fmt::Display) -> Refusal {
        Refusal::Fault(WireError::raised(HOME_NOT_KEPT, format!("What syncs could not be read: {why}"), self.run_id()))
    }

    /// Run `act` where blocking is allowed, then read the home again. `needs_copy` refuses an act
    /// on a row while this machine holds no copy.
    async fn at_home(
        &self,
        login: &str,
        needs_copy: bool,
        act: impl FnOnce(&Read, &Places<'_>, &str) -> Result<(), Refused> + Send + 'static,
    ) -> Result<SharedHomeRead, Refusal> {
        let read = self.home_read(login).await?;
        if needs_copy && read.home.here().is_none() {
            return Err(Refusal::IllegalMove(WireError::raised(
                NO_HOME_YET,
                "This machine holds no copy of the shared home yet: Sync makes it",
                self.run_id(),
            )));
        }
        let done = tokio::task::spawn_blocking(move || {
            let machine = crate::signing_in::this_machine(&read.machine);
            let places = read.places();
            act(&read, &places, &machine).map(|()| super::read(&places, &read.on))
        })
        .await
        .map_err(|why| self.home_not_kept(why))?;
        done.map_err(|refused| self.home_refusal(refused))
    }

    async fn home_kinds_read(&self, login: &str) -> Result<HomeKinds, Refusal> {
        self.a_profile(login)?;
        let switched = self.store().lock().await.home_kinds(login).map_err(|why| self.home_not_kept(why))?;
        Ok(super::kinds(login, &switched))
    }
}

impl<H, V, W> SharedHome for Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    async fn get_shared_home(&self, profile: HomeProfile) -> Result<SharedHomeRead, Refusal> {
        self.at_home(&profile.login, false, |_, _, _| Ok(())).await
    }

    async fn sync_shared_home(&self, profile: HomeProfile) -> Result<SharedHomeRead, Refusal> {
        self.at_home(&profile.login, false, |_, places, _| super::sync(places.home)).await
    }

    async fn add_to_shared_home(&self, at: HomeRowAt) -> Result<SharedHomeRead, Refusal> {
        let login = at.login.clone();
        self.at_home(&login, true, move |read, places, machine| {
            read.valid_mod(&at)?;
            super::add(places, &at, machine)
        })
        .await
    }

    async fn bring_from_shared_home(&self, at: HomeRowAt) -> Result<SharedHomeRead, Refusal> {
        let login = at.login.clone();
        self.at_home(&login, true, move |_, places, _| super::bring(places, &at)).await
    }

    async fn choose_shared_home_side(&self, choose: ChooseHomeSide) -> Result<SharedHomeRead, Refusal> {
        let login = choose.row.login.clone();
        self.at_home(&login, true, move |read, places, machine| {
            if choose.side == HomeSide::Here {
                read.valid_mod(&choose.row)?;
            }
            super::choose(places, &choose.row, choose.side, machine)
        })
        .await
    }

    async fn get_home_kinds(&self, profile: HomeProfile) -> Result<HomeKinds, Refusal> {
        self.home_kinds_read(&profile.login).await
    }

    async fn set_home_kind(&self, switch: SetHomeKind) -> Result<HomeKinds, Refusal> {
        self.a_profile(&switch.login)?;
        self.store()
            .lock()
            .await
            .switch_home_kind(&switch.login, switch.kind.as_wire(), switch.on)
            .map_err(|why| self.home_not_kept(why))?;
        self.home_kinds_read(&switch.login).await
    }
}
