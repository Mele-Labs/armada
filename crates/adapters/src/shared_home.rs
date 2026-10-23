//! The shared home on GitHub: `<login>/armada-home`, private, made through the API where it does
//! not exist and kept in the machine directory as a clone. `docs/concepts/kit.md`.
//!
//! **Git over HTTPS with `gh`'s own credential, never SSH.** An SSH key's passphrase sits in the
//! keychain, which will not open while the screen is locked, so a push at night would stop on a
//! prompt nobody sees. `gh auth git-credential` hands git the token `gh` already holds.

use std::path::{Path, PathBuf};
use std::process::Output;

use adapter_traits::{FileEntry, FileRead, HomeFailure, HomeRepository, KeptFolder, HOME_REPOSITORY};
use serde::{Deserialize, Serialize};

use crate::delivery::{run_in, said};
use crate::github_api::{ApiFailure, GitHubApi, GITHUB_HOST};

/// What git is told to ask for a credential, **in place of every helper configured**: the empty
/// one clears the list, so a helper that would prompt never runs.
const CREDENTIAL: [&str; 4] = ["-c", "credential.helper=", "-c", "credential.helper=!gh auth git-credential"];

/// A folder on this machine, read and written. **Follows a link**, for [`crate::Home`]'s reason:
/// a person's home is where a dotfiles repository puts its links.
#[derive(Clone, Debug)]
pub struct Folder {
    root: PathBuf,
}

impl Folder {
    pub fn at(root: impl Into<PathBuf>) -> Folder {
        Folder { root: root.into() }
    }

    fn under(&self, path: &str) -> Result<PathBuf, String> {
        let clean = path.trim_matches('/');
        if clean.split('/').any(|part| part == "..") {
            return Err(format!("{path} climbs out of {}", self.root.display()));
        }
        Ok(match clean.is_empty() {
            true => self.root.clone(),
            false => self.root.join(clean),
        })
    }
}

impl KeptFolder for Folder {
    fn read(&self, path: &str) -> FileRead {
        let full = match self.under(path) {
            Ok(full) => full,
            Err(why) => return FileRead::Unreadable(why),
        };
        match std::fs::read(&full) {
            Ok(bytes) => FileRead::Bytes(bytes),
            Err(why) if why.kind() == std::io::ErrorKind::NotFound => FileRead::Absent,
            Err(why) => FileRead::Unreadable(why.to_string()),
        }
    }

    fn entries(&self, dir: &str) -> Result<Vec<FileEntry>, String> {
        let full = self.under(dir)?;
        let mut entries = Vec::new();
        for entry in std::fs::read_dir(&full).map_err(|why| why.to_string())? {
            let entry = entry.map_err(|why| why.to_string())?;
            let is_dir = std::fs::metadata(entry.path()).map(|it| it.is_dir()).unwrap_or(false);
            entries.push(FileEntry { name: entry.file_name().to_string_lossy().to_string(), is_dir });
        }
        entries.sort_by(|left, right| left.name.cmp(&right.name));
        Ok(entries)
    }

    fn write(&self, path: &str, text: &str) -> Result<(), String> {
        let full = self.under(path)?;
        if let Some(parent) = full.parent() {
            std::fs::create_dir_all(parent).map_err(|why| format!("{}: {why}", parent.display()))?;
        }
        std::fs::write(&full, text).map_err(|why| format!("{}: {why}", full.display()))
    }

    fn remove(&self, path: &str) -> Result<(), String> {
        match std::fs::remove_file(self.under(path)?) {
            Err(why) if why.kind() != std::io::ErrorKind::NotFound => Err(why.to_string()),
            _ => Ok(()),
        }
    }
}

/// The home on GitHub, cloned at `dir`.
pub struct ForgeHome {
    dir: PathBuf,
    files: Folder,
    api: GitHubApi,
}

#[derive(Deserialize)]
struct User {
    login: String,
}

#[derive(Serialize)]
struct NewRepository<'a> {
    name: &'a str,
    private: bool,
    auto_init: bool,
    description: &'a str,
}

fn failed(why: ApiFailure) -> HomeFailure {
    match why {
        ApiFailure::NoToken(said) => HomeFailure::NotSignedIn(said),
        ApiFailure::Unreachable(said) => HomeFailure::Unreachable(said),
        other => HomeFailure::Refused(other.to_string()),
    }
}

impl ForgeHome {
    /// The home kept at `dir`, in the machine directory, through the one client Fleet shares.
    pub fn in_dir(dir: impl Into<PathBuf>) -> ForgeHome {
        let dir = dir.into();
        ForgeHome { files: Folder::at(&dir), dir, api: GitHubApi::shared() }
    }

    fn git(&self, at: &Path, args: &[&str]) -> Result<Output, HomeFailure> {
        let every: Vec<&str> = CREDENTIAL.iter().copied().chain(args.iter().copied()).collect();
        let run = run_in(&at.to_string_lossy(), "git", &every).map_err(|why| HomeFailure::Refused(format!("git could not be started: {why}")))?;
        match run.status.success() {
            true => Ok(run),
            false => Err(HomeFailure::Refused(said(&run))),
        }
    }

    fn exists(&self, login: &str) -> Result<bool, HomeFailure> {
        match self.api.get(&format!("/repos/{login}/{HOME_REPOSITORY}")) {
            Ok(_) => Ok(true),
            Err(ApiFailure::Refused { status: 404, .. }) => Ok(false),
            Err(why) => Err(failed(why)),
        }
    }
}

impl HomeRepository for ForgeHome {
    fn login(&self) -> Result<String, HomeFailure> {
        let user: User = self.api.get("/user").and_then(|answer| answer.decode("the signed-in user")).map_err(failed)?;
        Ok(user.login)
    }

    /// Read off the clone's own `origin`, so a copy made for one login is never taken for another's.
    /// **The config file rather than `git config`**: signing in asks this on every read.
    fn here(&self) -> Option<String> {
        let config = std::fs::read_to_string(self.dir.join(".git").join("config")).ok()?;
        let origin = config.split("[remote \"origin\"]").nth(1)?;
        let url = origin.lines().map(str::trim).take_while(|line| !line.starts_with('[')).find_map(|line| {
            let (key, value) = line.split_once('=')?;
            (key.trim() == "url").then(|| value.trim().to_string())
        })?;
        let path = url.strip_prefix(&format!("https://{GITHUB_HOST}/"))?;
        Some(path.trim_end_matches(".git").to_string())
    }

    fn open(&self, login: &str) -> Result<(), HomeFailure> {
        if self.here().is_some() {
            return self.pull();
        }
        if !self.exists(login)? {
            let asked = NewRepository {
                name: HOME_REPOSITORY,
                private: true,
                auto_init: true,
                description: "Armada's shared home: Kit carried between my machines",
            };
            self.api.post("/user/repos", &asked).map_err(failed)?;
        }
        let parent = self.dir.parent().ok_or_else(|| HomeFailure::Refused("the home's folder has no parent".to_string()))?;
        std::fs::create_dir_all(parent).map_err(|why| HomeFailure::Refused(why.to_string()))?;
        let url = format!("https://{GITHUB_HOST}/{login}/{HOME_REPOSITORY}.git");
        let into = self.dir.to_string_lossy().to_string();
        self.git(parent, &["clone", "--quiet", "--", &url, &into]).map(|_| ())
    }

    fn pull(&self) -> Result<(), HomeFailure> {
        self.git(&self.dir, &["pull", "--rebase", "--quiet"]).map(|_| ())
    }

    fn files(&self) -> &dyn KeptFolder {
        &self.files
    }

    /// One retry, after a rebase onto what another machine pushed meanwhile. **Each piece is its
    /// own files**, so two machines adding two pieces never touch one path.
    fn publish(&self, paths: &[String], message: &str) -> Result<(), HomeFailure> {
        let named: Vec<&str> = paths.iter().map(String::as_str).collect();
        let mut add = vec!["add", "-A", "--"];
        add.extend(&named);
        self.git(&self.dir, &add)?;
        let mut staged = vec!["diff", "--cached", "--quiet", "--"];
        staged.extend(&named);
        if self.git(&self.dir, &staged).is_ok() {
            return Ok(());
        }
        let mut commit = vec!["commit", "--quiet", "-m", message, "--"];
        commit.extend(&named);
        self.git(&self.dir, &commit)?;
        match self.git(&self.dir, &["push", "--quiet"]) {
            Ok(_) => Ok(()),
            Err(_) => {
                self.pull()?;
                self.git(&self.dir, &["push", "--quiet"]).map(|_| ())
            }
        }
    }
}
