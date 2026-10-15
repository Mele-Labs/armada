//! The profiles this machine has joined, kept in the machine directory.
//!
//! `profiles.json` lists each as `{account_id, login, added_at, orgs, orgs_read_at,
//! orgs_unread}`. **There is no active profile**: a repository is bound to one,
//! and `crate::repository_profiles` says which. **Each profile has a directory
//! of its own**, `profiles/<account id>/`, holding its own device key and device
//! id, so a profile's credentials are never found by looking at another's and
//! profiles never merge. `docs/concepts/account.md`, *Profiles*.
//!
//! The organisations are what a repository's owner is matched against. They are
//! read once at sign-in, while the device-flow token is in hand, and the token is
//! not kept. Nothing here is secret: the key sits in the directory, not in the list.

use std::io::Write;
use std::os::unix::fs::DirBuilderExt;
use std::path::{Path, PathBuf};

use adapter_traits::SignedInAs;
use serde::{Deserialize, Serialize};

/// The list, in the machine directory.
pub const FILE: &str = "profiles.json";
/// Where each profile's directory is made, under the machine directory.
pub const DIRECTORY: &str = "profiles";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct Entry {
    account_id: u64,
    /// As the forge last said it. A login can be renamed and the id cannot.
    login: String,
    added_at: String,
    /// Logins of the organisations the account belonged to when it signed in, lowercase.
    #[serde(default)]
    orgs: Vec<String>,
    /// When `orgs` was read. Absent where it could not be.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    orgs_read_at: Option<String>,
    /// The forge would not list the organisations, so `orgs` is empty for want of
    /// reading and not for want of any.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    orgs_unread: bool,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct Listed {
    #[serde(default)]
    profiles: Vec<Entry>,
}

/// What a sign-in found of the account's organisations.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Orgs {
    Read(Vec<String>),
    /// The forge would not list them.
    Unread,
}

/// One profile as the list holds it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Joined {
    pub account_id: u64,
    pub login: String,
    /// Lowercase. Empty where none were read as well as where there are none.
    pub orgs: Vec<String>,
    /// Not the first the machine joined: added beside its own on purpose.
    pub chosen: bool,
}

/// What the profiles of one machine directory are.
#[derive(Clone, Debug)]
pub struct Profiles {
    machine: PathBuf,
}

/// Written beside `name` in `machine` and renamed over it, so a reader meets the
/// whole of one version or the whole of the last and never half of either.
pub(crate) fn write_beside(machine: &Path, name: &str, text: &str) -> Result<(), String> {
    let file = machine.join(name);
    let beside = machine.join(format!(".{name}.{}.tmp", std::process::id()));
    let written = std::fs::File::create(&beside)
        .and_then(|mut out| out.write_all(text.as_bytes()).and_then(|()| out.sync_all()))
        .and_then(|()| std::fs::rename(&beside, &file));
    written.map_err(|why| {
        std::fs::remove_file(&beside).ok();
        format!("{} could not be written: {why}", file.display())
    })
}

impl Profiles {
    pub fn in_machine_directory(machine: &Path) -> Profiles {
        Profiles { machine: machine.to_path_buf() }
    }

    /// Where one profile's key and device id are. **Built from a number**, so
    /// no account can name a path outside the profiles directory.
    pub fn directory_of(&self, account_id: u64) -> PathBuf {
        self.machine.join(DIRECTORY).join(account_id.to_string())
    }

    fn file(&self) -> PathBuf {
        self.machine.join(FILE)
    }

    fn read(&self) -> Result<Listed, String> {
        let file = self.file();
        match std::fs::read(&file) {
            Ok(bytes) => ipc::decode("the profiles list", &bytes)
                .map_err(|why| format!("{} could not be read: {why}", file.display())),
            Err(why) if why.kind() == std::io::ErrorKind::NotFound => Ok(Listed::default()),
            Err(why) => Err(format!("{} could not be read: {why}", file.display())),
        }
    }

    fn write(&self, listed: &Listed) -> Result<(), String> {
        let text = ipc::encode(listed).map_err(|why| why.to_string())?;
        write_beside(&self.machine, FILE, &text)
    }

    pub fn list(&self) -> Result<Vec<Joined>, String> {
        let listed = self.read()?;
        Ok(listed
            .profiles
            .iter()
            .enumerate()
            .map(|(at, one)| Joined {
                account_id: one.account_id,
                login: one.login.clone(),
                orgs: one.orgs.clone(),
                chosen: at > 0,
            })
            .collect())
    }

    pub fn has(&self, account_id: u64) -> Result<bool, String> {
        Ok(self.read()?.profiles.iter().any(|one| one.account_id == account_id))
    }

    /// Make the profile's directory, owner-only, where its key will be kept.
    pub fn make_directory(&self, account_id: u64) -> Result<PathBuf, String> {
        let directory = self.directory_of(account_id);
        std::fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(&directory)
            .map_err(|why| format!("{} could not be made: {why}", directory.display()))?;
        Ok(directory)
    }

    /// Take the profile in. **The same account twice is one profile**: it keeps
    /// its place and its directory, and the login and the organisations are
    /// brought up to date, as of `now`.
    pub fn join(&self, account: &SignedInAs, orgs: Orgs, now: &str) -> Result<(), String> {
        let mut listed = self.read()?;
        let (orgs, read_at, unread) = match orgs {
            Orgs::Read(orgs) => (orgs, Some(now.to_string()), false),
            Orgs::Unread => (Vec::new(), None, true),
        };
        match listed.profiles.iter_mut().find(|one| one.account_id == account.id) {
            Some(known) => {
                known.login = account.login.clone();
                known.orgs = orgs;
                known.orgs_read_at = read_at;
                known.orgs_unread = unread;
            }
            None => listed.profiles.push(Entry {
                account_id: account.id,
                login: account.login.clone(),
                added_at: now.to_string(),
                orgs,
                orgs_read_at: read_at,
                orgs_unread: unread,
            }),
        }
        self.write(&listed)
    }

    /// Drop the profile and delete its directory. **The list is written first**:
    /// a crash between leaves a directory nobody lists, never a profile with no key.
    /// `false` for one that was never joined.
    pub fn leave(&self, account_id: u64) -> Result<bool, String> {
        let mut listed = self.read()?;
        let before = listed.profiles.len();
        listed.profiles.retain(|one| one.account_id != account_id);
        if listed.profiles.len() == before {
            return Ok(false);
        }
        self.write(&listed)?;
        self.remove_directory(account_id)?;
        Ok(true)
    }

    /// Delete a profile's directory, where there is one.
    pub fn remove_directory(&self, account_id: u64) -> Result<(), String> {
        let directory = self.directory_of(account_id);
        match std::fs::remove_dir_all(&directory) {
            Ok(()) => Ok(()),
            Err(why) if why.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(why) => Err(format!("{} could not be deleted: {why}", directory.display())),
        }
    }
}
