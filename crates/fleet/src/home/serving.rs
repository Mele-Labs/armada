//! Fleet's side of the shared home's nine operations: reads what syncs and Kit's servers out of
//! the store, runs the folder and forge work off the async workers, and spells a refusal.

use std::collections::BTreeSet;
use std::path::Path;
use std::sync::{Arc, Mutex, OnceLock, PoisonError};

use adapter_traits::{
    AgentHarness, Delivery, HarnessHome, HomeFailure, HomePlace, HomeRepositories, HomeRepository, KeptFolder, Vcs, WorkProduct,
};
use api::{Refusal, SharedHome};
use config::settings as keys;
use ipc::{
    ChooseHomeSide, HomeItem, HomeKind, HomeKinds, HomePlanned, HomeProfile, HomeRowAt, HomeSide, HomeSynced, SetHomeItem, SetHomeKind,
    SetHomeRepository, SharedHomeRead, WireError,
};

use super::{Places, Refused, Repository, Syncs};
use crate::daemon::Fleet;
use crate::profiles::Joined;

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
/// An owner that is not the login or one of its organisations, or a name the forge would not take.
const HOME_CHOICE_REFUSED: &str = "fleet.home_choice_refused";
/// The home is here or on the forge already, and a made home is shown and not changed. A 409.
const HOME_ALREADY_MADE: &str = "fleet.home_already_made";

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
    /// The logins the forge was asked about a home made on another Mac, this run.
    asked: Mutex<BTreeSet<String>>,
}

/// An item the store keeps, where its kind is one this Fleet knows.
fn item_of(kept: store::KeptHomeItem) -> Option<HomeItem> {
    Some(HomeItem { kind: HomeKind::from_wire(&kept.kind)?, name: kept.name, repository: kept.repository })
}

/// Everything an act reads before it leaves the async workers.
struct Read {
    fittings: Arc<HomeFittings>,
    /// The asked profile's home.
    home: Arc<dyn HomeRepository>,
    servers: Vec<core_model::KitServer>,
    repositories: Vec<Repository>,
    syncs: Syncs,
    /// Where the home is or will be made: what was chosen or found, else `<login>/armada-home`.
    place: HomePlace,
    /// The forge already had it, found for this login, so it is not a choice.
    found: bool,
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
    fn valid_mod(&self, kind: HomeKind, name: &str) -> Result<(), Refused> {
        if kind != HomeKind::Mods {
            return Ok(());
        }
        let examined = crate::mods::examine(Path::new(&self.mods_dir), name);
        match examined.problems.first() {
            Some(problem) => Err(Refused::Cannot(format!("The mod {name} is not valid here: {problem}"))),
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
            self.signing_in().knowing_home(Arc::new(move |login: &str, owners: &[String]| homes.of(login, owners).ok()?.here()));
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
            Refused::Choice(_) => Refusal::Unacceptable(WireError::raised(HOME_CHOICE_REFUSED, said, run)),
            Refused::Made(_) => Refusal::IllegalMove(WireError::raised(HOME_ALREADY_MADE, said, run)),
            Refused::Home(HomeFailure::Unreachable(_)) => Refusal::Fault(WireError::raised(HOME_UNREACHABLE, said, run)),
            Refused::Cannot(_) | Refused::Home(_) => Refusal::Unacceptable(WireError::raised(HOME_REFUSED, said, run)),
        }
    }

    fn fittings(&self) -> Result<Arc<HomeFittings>, Refusal> {
        self.home_held().fittings.get().cloned().ok_or_else(|| {
            Refusal::IllegalMove(WireError::raised(HOME_NOT_SERVED, "this Fleet was started without the shared home", self.run_id()))
        })
    }

    /// This machine's profiles.
    fn profiles(&self) -> Result<Vec<Joined>, Refusal> {
        self.signing_in().joined().map_err(|why| self.home_not_kept(format!("the profiles could not be read: {why}")))
    }

    /// Refuse a login that is not a profile here.
    fn a_profile(&self, login: &str) -> Result<(), Refusal> {
        super::profile_of(login, &self.profiles()?).map(|_| ()).map_err(|refused| self.home_refusal(refused))
    }

    /// `login`'s home, refused where it is not a profile here.
    fn home_for(&self, fittings: &HomeFittings, login: &str) -> Result<Arc<dyn HomeRepository>, Refusal> {
        super::home_of(fittings.homes.as_ref(), login, &self.profiles()?).map_err(|refused| self.home_refusal(refused))
    }

    async fn home_read(&self, login: &str) -> Result<Read, Refusal> {
        let fittings = self.fittings()?;
        let home = self.home_for(&fittings, login)?;
        let (servers, switched, items, kept) = {
            let store = self.store().lock().await;
            // Not defaulted: a server read as missing here would read as only in the home.
            let servers = store.kit_servers().map_err(|why| self.home_not_kept(format!("{why:?}")))?;
            let kept = store.home_repository(login).map_err(|why| self.choice_not_kept(why))?;
            let items = store.home_items(login).map_err(|why| self.home_not_kept(why))?;
            (servers, store.home_kinds(login).map_err(|why| self.home_not_kept(why))?, items, kept)
        };
        let found = kept.as_ref().is_some_and(|it| it.found);
        let place = kept.map_or_else(|| HomePlace::default_for(login), |it| HomePlace { owner: it.owner, name: it.name });
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
            syncs: Syncs { kinds: super::on(&switched), items: items.into_iter().filter_map(item_of).collect() },
            place,
            found,
            machine: self.machine_settings().now().get(keys::MACHINE_NAME),
            mods_dir: self.host().mods_dir.clone(),
        })
    }

    fn home_not_kept(&self, why: impl std::fmt::Display) -> Refusal {
        Refusal::Fault(WireError::raised(HOME_NOT_KEPT, format!("What syncs could not be read: {why}"), self.run_id()))
    }

    fn choice_not_kept(&self, why: impl std::fmt::Display) -> Refusal {
        Refusal::Fault(WireError::raised(HOME_NOT_KEPT, format!("Where the shared home is made could not be read or kept: {why}"), self.run_id()))
    }

    /// Ask the forge for a home another Mac made, **once per login per run** and only where this
    /// machine holds no copy and nothing is recorded: what it finds is recorded as found. A forge
    /// that does not answer leaves the default, and is no error on a read; one that does not hold
    /// the login is asked again once it does.
    async fn finding_home(&self, login: &str) -> Result<(), Refusal> {
        let fittings = self.fittings()?;
        let home = self.home_for(&fittings, login)?;
        if home.here().is_some() || self.store().lock().await.home_repository(login).map_err(|why| self.choice_not_kept(why))?.is_some() {
            return Ok(());
        }
        let first = self.home_held().asked.lock().unwrap_or_else(PoisonError::into_inner).insert(login.to_string());
        if !first {
            return Ok(());
        }
        match tokio::task::spawn_blocking(move || home.found()).await.map_err(|why| self.home_not_kept(why))? {
            Ok(Some(place)) => {
                let mut store = self.store().lock().await;
                store.set_home_repository(login, &place.owner, &place.name, true).map_err(|why| self.choice_not_kept(why))?;
            }
            Ok(None) => {}
            Err(HomeFailure::NotSignedIn(_)) => {
                self.home_held().asked.lock().unwrap_or_else(PoisonError::into_inner).remove(login);
            }
            Err(_) => {}
        }
        Ok(())
    }

    /// Run `act` where blocking is allowed, then read the home again. `needs_copy` refuses an act
    /// on a row while this machine holds no copy.
    async fn at_home(
        &self,
        login: &str,
        needs_copy: bool,
        act: impl FnOnce(&Read, &Places<'_>, &str) -> Result<(), Refused> + Send + 'static,
    ) -> Result<SharedHomeRead, Refusal> {
        self.at_home_with(login, needs_copy, act).await.map(|((), read)| read)
    }

    /// [`Self::at_home`], keeping what `act` answers beside the read.
    async fn at_home_with<T: Send + 'static>(
        &self,
        login: &str,
        needs_copy: bool,
        act: impl FnOnce(&Read, &Places<'_>, &str) -> Result<T, Refused> + Send + 'static,
    ) -> Result<(T, SharedHomeRead), Refusal> {
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
            act(&read, &places, &machine).map(|answer| {
                let mut shown = super::read(&places, &read.syncs);
                if shown.home.is_none() {
                    let HomePlace { owner, name } = read.place.clone();
                    shown.planned = Some(HomePlanned { account: owner, repository: name, exists: read.found });
                }
                (answer, shown)
            })
        })
        .await
        .map_err(|why| self.home_not_kept(why))?;
        done.map_err(|refused| self.home_refusal(refused))
    }

    async fn home_kinds_read(&self, login: &str) -> Result<HomeKinds, Refusal> {
        self.a_profile(login)?;
        let store = self.store().lock().await;
        let switched = store.home_kinds(login).map_err(|why| self.home_not_kept(why))?;
        let items = store.home_items(login).map_err(|why| self.home_not_kept(why))?;
        Ok(super::kinds(login, &switched, items.into_iter().filter_map(item_of).collect()))
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
        self.finding_home(&profile.login).await?;
        self.at_home(&profile.login, false, |_, _, _| Ok(())).await
    }

    async fn sync_shared_home(&self, profile: HomeProfile) -> Result<HomeSynced, Refusal> {
        self.finding_home(&profile.login).await?;
        let (left, read) = self
            .at_home_with(&profile.login, false, |read, places, machine| {
                super::sync(places, &read.syncs, machine, &read.place, &|kind, name| read.valid_mod(kind, name))
            })
            .await?;
        Ok(HomeSynced { read, left })
    }

    async fn add_to_shared_home(&self, at: HomeRowAt) -> Result<SharedHomeRead, Refusal> {
        let login = at.login.clone();
        self.at_home(&login, true, move |read, places, machine| {
            read.valid_mod(at.kind, &at.name)?;
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
                read.valid_mod(choose.row.kind, &choose.row.name)?;
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

    async fn set_home_item(&self, switch: SetHomeItem) -> Result<HomeKinds, Refusal> {
        self.a_profile(&switch.login)?;
        let HomeItem { kind, name, repository } = switch.item;
        let kept = store::KeptHomeItem { kind: kind.as_wire().to_string(), name, repository: repository.filter(|_| kind == HomeKind::Memory) };
        self.store().lock().await.switch_home_item(&switch.login, &kept, switch.on).map_err(|why| self.home_not_kept(why))?;
        self.home_kinds_read(&switch.login).await
    }

    async fn set_home_repository(&self, choice: SetHomeRepository) -> Result<SharedHomeRead, Refusal> {
        let place = {
            let profiles = self.profiles()?;
            let profile = super::profile_of(&choice.login, &profiles).map_err(|refused| self.home_refusal(refused))?;
            super::chosen(profile, &choice.owner, &choice.name).map_err(|refused| self.home_refusal(refused))?
        };
        // A home another Mac made is found first, so a choice never lands beside it.
        self.finding_home(&choice.login).await?;
        let fittings = self.fittings()?;
        let kept = {
            let here = self.home_for(&fittings, &choice.login)?.here();
            (here, self.store().lock().await.home_repository(&choice.login).map_err(|why| self.choice_not_kept(why))?)
        };
        match kept {
            (Some(here), _) => {
                let said = format!("This Mac already holds the shared home, {here}, and it is not moved");
                return Err(self.home_refusal(Refused::Made(said)));
            }
            (None, Some(kept)) if kept.found => {
                let said = format!("The shared home is already at {}/{}, and it is not moved", kept.owner, kept.name);
                return Err(self.home_refusal(Refused::Made(said)));
            }
            _ => {}
        }
        self.store()
            .lock()
            .await
            .set_home_repository(&choice.login, &place.owner, &place.name, false)
            .map_err(|why| self.choice_not_kept(why))?;
        self.at_home(&choice.login, false, |_, _, _| Ok(())).await
    }
}
