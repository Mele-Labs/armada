//! The shared home without a forge, a remote or a disk: folders held in memory, one remote every
//! machine's copy reaches, and a harness that installs a plugin by remembering it.
//!
//! **Two machines are two [`FakeHome`]s on one [`FakeRemote`]**, each with a copy of its own, so
//! what one publishes reaches the other only once it opens or pulls, as a clone would.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use adapter_traits::{
    FileEntry, FileRead, HarnessHome, HarnessPiece, HarnessPlace, HomeFailure, HomeRepository, KeptFolder, PluginRecord,
    HOME_REPOSITORY,
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

/// The home's repository at the forge: absent until a machine opens it.
#[derive(Clone)]
pub struct FakeRemote {
    login: String,
    files: Arc<Mutex<Option<BTreeMap<String, String>>>>,
    published: Arc<Mutex<Vec<Vec<String>>>>,
}

impl FakeRemote {
    /// `login`'s forge account, with no home on it yet.
    pub fn of(login: &str) -> FakeRemote {
        FakeRemote { login: login.to_string(), files: Arc::default(), published: Arc::default() }
    }

    pub fn exists(&self) -> bool {
        held(&self.files).is_some()
    }

    /// A file as the remote holds it.
    pub fn text(&self, path: &str) -> Option<String> {
        held(&self.files).as_ref().and_then(|files| files.get(path).cloned())
    }

    pub fn paths(&self) -> Vec<String> {
        held(&self.files).as_ref().map(|files| files.keys().cloned().collect()).unwrap_or_default()
    }

    /// The paths of each publish, in order.
    pub fn published(&self) -> Vec<Vec<String>> {
        held(&self.published).clone()
    }
}

/// One machine's copy of the home.
pub struct FakeHome {
    remote: FakeRemote,
    copy: HeldFolder,
    cloned: Mutex<bool>,
}

impl FakeHome {
    /// A machine signed in as the remote's login, with no copy yet.
    pub fn on(remote: &FakeRemote) -> FakeHome {
        FakeHome { remote: remote.clone(), copy: HeldFolder::new(), cloned: Mutex::new(false) }
    }

    fn fetched(&self) {
        let files = held(&self.remote.files).clone().unwrap_or_default();
        self.copy.replace(files);
    }
}

impl HomeRepository for FakeHome {
    fn login(&self) -> Result<String, HomeFailure> {
        Ok(self.remote.login.clone())
    }

    fn here(&self) -> Option<String> {
        held(&self.cloned).then(|| format!("{}/{HOME_REPOSITORY}", self.remote.login))
    }

    fn open(&self, login: &str) -> Result<(), HomeFailure> {
        if login != self.remote.login {
            return Err(HomeFailure::Refused(format!("{login} is not the account signed in here")));
        }
        held(&self.remote.files).get_or_insert_with(BTreeMap::new);
        self.fetched();
        *held(&self.cloned) = true;
        Ok(())
    }

    fn pull(&self) -> Result<(), HomeFailure> {
        if !*held(&self.cloned) {
            return Err(HomeFailure::Refused("there is no copy of the home here".to_string()));
        }
        self.fetched();
        Ok(())
    }

    fn files(&self) -> &dyn KeptFolder {
        &self.copy
    }

    fn publish(&self, paths: &[String], _message: &str) -> Result<(), HomeFailure> {
        let copy = self.copy.all();
        let mut remote = held(&self.remote.files);
        let Some(files) = remote.as_mut() else {
            return Err(HomeFailure::Refused("the home does not exist".to_string()));
        };
        for path in paths {
            match copy.get(path) {
                Some(text) => files.insert(path.clone(), text.clone()),
                None => files.remove(path),
            };
        }
        held(&self.remote.published).push(paths.to_vec());
        Ok(())
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
