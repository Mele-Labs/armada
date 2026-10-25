//! The shared home without a forge, a remote or a disk: folders held in memory, one remote every
//! machine's copy reaches, and a harness that installs a plugin by remembering it.
//!
//! **Two machines are two [`FakeHome`]s on one [`FakeRemote`]**, each with a copy of its own, so
//! what one publishes reaches the other only once it opens or pulls, as a clone would. One
//! machine's profiles are a [`FakeHomes`]: a home for each login its forge holds.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use adapter_traits::{
    FileEntry, FileRead, HarnessHome, HarnessPiece, HarnessPlace, HomeFailure, HomePlace, HomeRepositories, HomeRepository, KeptFolder,
    PluginRecord, HOME_REPOSITORY,
};

fn held<T>(lock: &Mutex<T>) -> MutexGuard<'_, T> {
    lock.lock().unwrap_or_else(PoisonError::into_inner)
}

/// A folder in memory: each file's text by its path.
#[derive(Clone, Default)]
pub struct HeldFolder(Arc<Mutex<BTreeMap<String, String>>>);

impl HeldFolder {
    pub fn new() -> HeldFolder {
        HeldFolder::default()
    }

    /// The same folder with `text` at `path`.
    pub fn with(self, path: &str, text: &str) -> HeldFolder {
        held(&self.0).insert(path.to_string(), text.to_string());
        self
    }

    pub fn text(&self, path: &str) -> Option<String> {
        held(&self.0).get(path).cloned()
    }

    pub fn paths(&self) -> Vec<String> {
        held(&self.0).keys().cloned().collect()
    }

    fn replace(&self, files: BTreeMap<String, String>) {
        *held(&self.0) = files;
    }

    fn all(&self) -> BTreeMap<String, String> {
        held(&self.0).clone()
    }
}

fn climbs(path: &str) -> bool {
    path.starts_with('/') || path.split('/').any(|part| part == "..")
}

impl KeptFolder for HeldFolder {
    fn read(&self, path: &str) -> FileRead {
        match held(&self.0).get(path) {
            Some(text) => FileRead::Bytes(text.clone().into_bytes()),
            None => FileRead::Absent,
        }
    }

    fn entries(&self, dir: &str) -> Result<Vec<FileEntry>, String> {
        let prefix = match dir.trim_matches('/') {
            "" => String::new(),
            dir => format!("{dir}/"),
        };
        let mut found: BTreeMap<String, bool> = BTreeMap::new();
        for path in held(&self.0).keys() {
            let Some(rest) = path.strip_prefix(&prefix) else { continue };
            match rest.split_once('/') {
                Some((first, _)) => found.insert(first.to_string(), true),
                None => found.insert(rest.to_string(), false),
            };
        }
        if found.is_empty() {
            return Err(format!("{dir} is not there"));
        }
        Ok(found.into_iter().map(|(name, is_dir)| FileEntry { name, is_dir }).collect())
    }

    fn write(&self, path: &str, text: &str) -> Result<(), String> {
        if climbs(path) {
            return Err(format!("{path} climbs out of the folder"));
        }
        held(&self.0).insert(path.to_string(), text.to_string());
        Ok(())
    }

    fn remove(&self, path: &str) -> Result<(), String> {
        held(&self.0).remove(path);
        Ok(())
    }
}

/// A repository at the forge, as the fake keeps it.
#[derive(Clone, Default)]
struct Repo {
    files: BTreeMap<String, String>,
    /// Marked as a home, so another machine finds it.
    marked: bool,
    /// The login may push to it.
    writable: bool,
}

/// The forge account's repositories, `owner/name` each: absent until a machine opens a home
/// there, or planted. The login may make repositories under its own name, and under any
/// organisation it was given.
#[derive(Clone)]
pub struct FakeRemote {
    login: String,
    repos: Arc<Mutex<BTreeMap<String, Repo>>>,
    /// The organisations the login may make repositories in, besides the ones it only belongs to.
    makes_in: Arc<Mutex<Vec<String>>>,
    published: Arc<Mutex<Vec<Vec<String>>>>,
    /// How many times a machine asked for a home another machine made.
    listings: Arc<AtomicUsize>,
    /// The forge does not answer a listing.
    listing_fails: bool,
}

fn key(owner: &str, name: &str) -> String {
    format!("{}/{name}", owner.to_lowercase())
}

impl FakeRemote {
    /// `login`'s forge account, with no home on it yet.
    pub fn of(login: &str) -> FakeRemote {
        FakeRemote {
            login: login.to_string(),
            repos: Arc::default(),
            makes_in: Arc::default(),
            published: Arc::default(),
            listings: Arc::default(),
            listing_fails: false,
        }
    }

    /// The same account, where the login may make repositories in the organisation `org`.
    pub fn making_in(self, org: &str) -> FakeRemote {
        held(&self.makes_in).push(org.to_lowercase());
        self
    }

    /// The same account with a repository already at `owner/name`, not marked as a home, that the
    /// login may push to or, with `writable` false, may only read.
    pub fn with_repository(self, owner: &str, name: &str, writable: bool) -> FakeRemote {
        held(&self.repos).insert(key(owner, name), Repo { writable, ..Repo::default() });
        self
    }

    /// The same account with a home another machine made at `owner/name`: marked, and writable.
    pub fn with_home(self, owner: &str, name: &str) -> FakeRemote {
        held(&self.repos).insert(key(owner, name), Repo { marked: true, writable: true, ..Repo::default() });
        self
    }

    /// The same account, whose forge does not answer when asked for a home another machine made.
    pub fn failing_to_list(self) -> FakeRemote {
        FakeRemote { listing_fails: true, ..self }
    }

    /// How many times a machine asked this account for a home another machine made.
    pub fn listings(&self) -> usize {
        self.listings.load(Ordering::SeqCst)
    }

    pub fn login(&self) -> &str {
        &self.login
    }

    fn default_key(&self) -> String {
        key(&self.login, HOME_REPOSITORY)
    }

    /// `<login>/armada-home` is there.
    pub fn exists(&self) -> bool {
        self.exists_at(&self.login, HOME_REPOSITORY)
    }

    pub fn exists_at(&self, owner: &str, name: &str) -> bool {
        held(&self.repos).contains_key(&key(owner, name))
    }

    /// `owner/name` is there and marked as a home.
    pub fn marked_at(&self, owner: &str, name: &str) -> bool {
        held(&self.repos).get(&key(owner, name)).is_some_and(|repo| repo.marked)
    }

    /// A file as `<login>/armada-home` holds it.
    pub fn text(&self, path: &str) -> Option<String> {
        self.text_at(&self.login, HOME_REPOSITORY, path)
    }

    pub fn text_at(&self, owner: &str, name: &str, path: &str) -> Option<String> {
        held(&self.repos).get(&key(owner, name)).and_then(|repo| repo.files.get(path).cloned())
    }

    pub fn paths(&self) -> Vec<String> {
        held(&self.repos).get(&self.default_key()).map(|repo| repo.files.keys().cloned().collect()).unwrap_or_default()
    }

    /// The paths of each publish, in order.
    pub fn published(&self) -> Vec<Vec<String>> {
        held(&self.published).clone()
    }
}

/// One machine's copy of one login's home.
pub struct FakeHome {
    remote: FakeRemote,
    copy: HeldFolder,
    /// The repository the copy was cloned from, `owner/name` as the remote keys it.
    cloned: Mutex<Option<String>>,
    /// The forge here holds the remote's login.
    held: bool,
    /// The organisations the login belongs to.
    owners: Mutex<Vec<String>>,
}

impl FakeHome {
    /// A machine whose forge holds the remote's login, with no copy yet.
    pub fn on(remote: &FakeRemote) -> FakeHome {
        FakeHome {
            remote: remote.clone(),
            copy: HeldFolder::new(),
            cloned: Mutex::new(None),
            held: true,
            owners: Mutex::new(vec![remote.login.to_lowercase()]),
        }
    }

    /// A machine whose forge does not hold `login`: every act is refused, naming it.
    pub fn not_held(login: &str) -> FakeHome {
        FakeHome { held: false, ..FakeHome::on(&FakeRemote::of(login)) }
    }

    fn holding(&self) -> Result<(), HomeFailure> {
        match self.held {
            true => Ok(()),
            false => Err(HomeFailure::NotSignedIn(format!("The forge here does not hold {}", self.remote.login))),
        }
    }

    fn fetched(&self) {
        let files = held(&self.cloned).as_ref().and_then(|at| held(&self.remote.repos).get(at).map(|repo| repo.files.clone()));
        self.copy.replace(files.unwrap_or_default());
    }
}

impl HomeRepository for FakeHome {
    fn here(&self) -> Option<String> {
        held(&self.cloned).clone()
    }

    fn open(&self, place: &HomePlace) -> Result<(), HomeFailure> {
        self.holding()?;
        if held(&self.cloned).is_some() {
            return self.pull();
        }
        let at = key(&place.owner, &place.name);
        {
            let mut repos = held(&self.remote.repos);
            match repos.get(&at) {
                Some(there) if !there.writable => return Err(HomeFailure::cannot_write(&self.remote.login, place)),
                Some(_) => repos.get_mut(&at).expect("there").marked = true,
                None => {
                    let own = place.owner.eq_ignore_ascii_case(&self.remote.login);
                    if !own && !held(&self.remote.makes_in).contains(&place.owner.to_lowercase()) {
                        return Err(HomeFailure::cannot_make(&self.remote.login, &place.owner));
                    }
                    repos.insert(at.clone(), Repo { marked: true, writable: true, ..Repo::default() });
                }
            }
        }
        *held(&self.cloned) = Some(at);
        self.fetched();
        Ok(())
    }

    fn found(&self) -> Result<Option<HomePlace>, HomeFailure> {
        self.holding()?;
        self.remote.listings.fetch_add(1, Ordering::SeqCst);
        if self.remote.listing_fails {
            return Err(HomeFailure::Unreachable("the forge here did not answer".to_string()));
        }
        let owners = held(&self.owners).clone();
        let repos = held(&self.remote.repos);
        let homes = repos.iter().filter(|(_, repo)| repo.marked).filter_map(|(at, _)| at.split_once('/'));
        let mut owned: Vec<(&str, &str)> = homes.filter(|(owner, _)| owners.iter().any(|it| it == owner)).collect();
        owned.sort_by_key(|(owner, _)| !owner.eq_ignore_ascii_case(&self.remote.login));
        Ok(owned.first().map(|(owner, name)| HomePlace { owner: owner.to_string(), name: name.to_string() }))
    }

    fn pull(&self) -> Result<(), HomeFailure> {
        self.holding()?;
        if held(&self.cloned).is_none() {
            return Err(HomeFailure::Refused("there is no copy of the home here".to_string()));
        }
        self.fetched();
        Ok(())
    }

    fn files(&self) -> &dyn KeptFolder {
        &self.copy
    }

    fn publish(&self, paths: &[String], _message: &str) -> Result<(), HomeFailure> {
        self.holding()?;
        let copy = self.copy.all();
        let at = held(&self.cloned).clone();
        let mut repos = held(&self.remote.repos);
        let Some(repo) = at.and_then(|at| repos.get_mut(&at)) else {
            return Err(HomeFailure::Refused("the home does not exist".to_string()));
        };
        for path in paths {
            match copy.get(path) {
                Some(text) => repo.files.insert(path.clone(), text.clone()),
                None => repo.files.remove(path),
            };
        }
        held(&self.remote.published).push(paths.to_vec());
        Ok(())
    }
}

/// One machine's homes, a login each: the logins its forge holds, and any other refused by name.
#[derive(Clone, Default)]
pub struct FakeHomes {
    held: Arc<Mutex<BTreeMap<String, Arc<FakeHome>>>>,
}

impl FakeHomes {
    pub fn new() -> FakeHomes {
        FakeHomes::default()
    }

    /// The forge here holds the remote's login, and this machine reaches its home.
    pub fn holding(self, remote: &FakeRemote) -> FakeHomes {
        held(&self.held).insert(remote.login.clone(), Arc::new(FakeHome::on(remote)));
        self
    }
}

impl HomeRepositories for FakeHomes {
    fn of(&self, login: &str, owners: &[String]) -> Result<Arc<dyn HomeRepository>, HomeFailure> {
        let home = held(&self.held).get(login).cloned().unwrap_or_else(|| Arc::new(FakeHome::not_held(login)));
        *held(&home.owners) = owners.iter().map(|owner| owner.to_lowercase()).chain([login.to_lowercase()]).collect();
        Ok(home)
    }
}

/// A harness under a home held in memory, laid out under `harness/`.
#[derive(Default)]
pub struct FakeHarnessHome {
    folder: HeldFolder,
    plugins: Mutex<Vec<PluginRecord>>,
    installed: Mutex<Vec<PluginRecord>>,
}

impl FakeHarnessHome {
    pub fn over(folder: HeldFolder) -> FakeHarnessHome {
        FakeHarnessHome { folder, ..FakeHarnessHome::default() }
    }

    /// The same harness with `plugin` already installed.
    pub fn with_plugin(self, plugin: PluginRecord) -> FakeHarnessHome {
        held(&self.plugins).push(plugin);
        self
    }

    /// What [`HarnessHome::install`] was asked, in order.
    pub fn installed(&self) -> Vec<PluginRecord> {
        held(&self.installed).clone()
    }
}

impl HarnessHome for FakeHarnessHome {
    fn folder(&self) -> &dyn KeptFolder {
        &self.folder
    }

    fn place(&self, piece: HarnessPiece) -> HarnessPlace {
        match piece {
            HarnessPiece::Skills => HarnessPlace::Folders { dir: "harness/skills".to_string() },
            HarnessPiece::SubAgents => HarnessPlace::Files { dir: "harness/agents".to_string(), suffix: ".md".to_string() },
            HarnessPiece::Commands => HarnessPlace::Files { dir: "harness/commands".to_string(), suffix: ".md".to_string() },
            HarnessPiece::GlobalInstructions => HarnessPlace::One { path: "harness/INSTRUCTIONS.md".to_string() },
        }
    }

    fn memory_dir(&self, root: &str) -> String {
        format!("harness/projects/{}/memory", root.replace('/', "-"))
    }

    fn memory_index(&self) -> String {
        "INDEX.md".to_string()
    }

    fn plugins(&self) -> Result<Vec<PluginRecord>, String> {
        Ok(held(&self.plugins).clone())
    }

    fn install(&self, plugin: &PluginRecord) -> Result<(), String> {
        held(&self.plugins).push(plugin.clone());
        held(&self.installed).push(plugin.clone());
        Ok(())
    }
}
