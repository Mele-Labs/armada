//! The shared home on GitHub: one repository for each profile, `<login>/armada-home` until the
//! person chooses another owner or name, private, made through the API where it does not exist
//! and kept in the machine directory as a clone, `home/<login>`. `docs/concepts/kit.md`.
//!
//! **Git over HTTPS with `gh`'s own token for that login, never SSH.** An SSH key's passphrase
//! sits in the keychain, which will not open while the screen is locked, so a push at night would
//! stop on a prompt nobody sees. **`gh` can hold several accounts**, so the API and git both ask
//! it for the profile's own token with `--user`, never the active account's.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Output;
use std::sync::{Arc, Mutex, PoisonError};

use adapter_traits::{FileEntry, FileRead, HomeFailure, HomePlace, HomeRepositories, HomeRepository, KeptFolder};
use serde::{Deserialize, Serialize};

use crate::delivery::{run_in, said};
use crate::github_api::{token_from_gh_as, ApiFailure, GitHubApi, API_BASE, GITHUB_HOST};

/// What marks a repository as somebody's shared home, so another Mac can find it.
const HOME_TOPIC: &str = "armada-home";

/// The most pages of a login's repositories read to find its home, a hundred to a page.
const LISTING_PAGES: usize = 10;

/// The most characters a GitHub login holds.
const LOGIN_MOST: usize = 39;

/// A GitHub login: letters, digits and single hyphens, never at either end. **It is written into a
/// credential helper and a folder name**, so nothing else is let through.
pub(crate) fn is_login(login: &str) -> bool {
    (1..=LOGIN_MOST).contains(&login.len())
        && login.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
        && !login.starts_with('-')
        && !login.ends_with('-')
        && !login.contains("--")
}

/// The refusal where `gh` here does not hold `login`: what to run to add it.
pub(crate) fn not_held(login: &str) -> String {
    format!("The GitHub CLI on this Mac is not signed in as {login}. Run `gh auth login` and sign in as {login}, then Sync again")
}

/// What git is told to ask for a credential, **in place of every helper configured**: the empty
/// one clears the list, so a helper that would prompt never runs, and the one left asks `gh` for
/// `login`'s own token. The token is read when git asks, so it is never in an argument.
pub(crate) fn credential(login: &str) -> [String; 4] {
    let helper = format!(
        "credential.helper=!f() {{ test \"$1\" = get || exit 0; echo username=x-access-token; echo \"password=$(gh auth token --hostname {GITHUB_HOST} --user {login})\"; }}; f"
    );
    ["-c".to_string(), "credential.helper=".to_string(), "-c".to_string(), helper]
}

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

/// Every profile's home on GitHub, each cloned at `<dir>/<login>`.
pub struct ForgeHomes {
    dir: PathBuf,
    made: Mutex<BTreeMap<String, Arc<ForgeHome>>>,
}

impl ForgeHomes {
    /// The homes kept under `dir`, in the machine directory.
    pub fn in_dir(dir: impl Into<PathBuf>) -> ForgeHomes {
        ForgeHomes { dir: dir.into(), made: Mutex::default() }
    }
}

impl HomeRepositories for ForgeHomes {
    fn of(&self, login: &str, owners: &[String]) -> Result<Arc<dyn HomeRepository>, HomeFailure> {
        if !is_login(login) {
            return Err(HomeFailure::Refused(format!("{login:?} cannot be a GitHub login")));
        }
        let mut made = self.made.lock().unwrap_or_else(PoisonError::into_inner);
        // A profile signed in again may belong to other organisations: a home made for the old
        // list is made again for the new.
        let home = match made.get(login).filter(|home| home.owners_are(owners)) {
            Some(home) => Arc::clone(home),
            None => {
                let home = Arc::new(ForgeHome::as_login(self.dir.join(login), login, owners));
                made.insert(login.to_string(), Arc::clone(&home));
                home
            }
        };
        Ok(home as Arc<dyn HomeRepository>)
    }
}

/// One login's home on GitHub, cloned at `dir`, through a client that holds that login's token.
pub(crate) struct ForgeHome {
    login: String,
    /// The logins a clone here may be owned by: the login's own and its organisations'.
    owners: Vec<String>,
    /// The list `owners` was made from, to tell a home made for another one.
    given: Vec<String>,
    dir: PathBuf,
    files: Folder,
    api: GitHubApi,
}

#[derive(Serialize)]
struct NewRepository<'a> {
    name: &'a str,
    private: bool,
    auto_init: bool,
    description: &'a str,
}

#[derive(Serialize)]
struct Topics<'a> {
    names: [&'a str; 1],
}

/// What the forge says of a repository that is there: whether the login may push to it.
#[derive(Deserialize)]
struct Existing {
    permissions: Option<Permissions>,
}

#[derive(Deserialize)]
struct Permissions {
    push: bool,
}

/// One repository of the login's listing.
#[derive(Deserialize)]
struct Listed {
    name: String,
    owner: ListedOwner,
    #[serde(default)]
    topics: Vec<String>,
}

#[derive(Deserialize)]
struct ListedOwner {
    login: String,
}

/// The home among a login's repositories: one marked as a home and owned by one of `owners`,
/// the login's own before an organisation's.
fn home_among(listed: &[Listed], login: &str, owners: &[String]) -> Option<HomePlace> {
    let marked = |one: &&Listed| one.topics.iter().any(|topic| topic == HOME_TOPIC);
    let owned = |one: &&Listed| owners.iter().any(|owner| owner.eq_ignore_ascii_case(&one.owner.login));
    let mut homes: Vec<&Listed> = listed.iter().filter(marked).filter(owned).collect();
    homes.sort_by_key(|one| !one.owner.login.eq_ignore_ascii_case(login));
    homes.first().map(|one| HomePlace { owner: one.owner.login.clone(), name: one.name.clone() })
}

impl ForgeHome {
    /// `login`'s home kept at `dir`. Its own client: **the one Fleet shares holds the active
    /// account's token**, which may be another profile's.
    fn as_login(dir: PathBuf, login: &str, owners: &[String]) -> ForgeHome {
        let asked = login.to_string();
        let api = GitHubApi::against(API_BASE, move || token_from_gh_as(GITHUB_HOST, &asked));
        ForgeHome::through(dir, login, owners, api)
    }

    pub(crate) fn through(dir: PathBuf, login: &str, owners: &[String], api: GitHubApi) -> ForgeHome {
        let mut accepted = owners.to_vec();
        if !accepted.iter().any(|owner| owner.eq_ignore_ascii_case(login)) {
            accepted.push(login.to_string());
        }
        ForgeHome { login: login.to_string(), owners: accepted, given: owners.to_vec(), files: Folder::at(&dir), dir, api }
    }

    fn owners_are(&self, owners: &[String]) -> bool {
        self.given == owners
    }

    fn failed(&self, why: ApiFailure) -> HomeFailure {
        match why {
            ApiFailure::NoToken(_) => HomeFailure::NotSignedIn(not_held(&self.login)),
            ApiFailure::Unreachable(said) => HomeFailure::Unreachable(said),
            other => HomeFailure::Refused(other.to_string()),
        }
    }

    /// Refuse, naming the login, where `gh` here does not hold it: git would fail on a credential
    /// it could not name.
    fn holding(&self) -> Result<(), HomeFailure> {
        token_from_gh_as(GITHUB_HOST, &self.login).map(drop).map_err(|why| self.failed(why))
    }

    fn git(&self, at: &Path, args: &[&str]) -> Result<Output, HomeFailure> {
        let credential = credential(&self.login);
        let every: Vec<&str> = credential.iter().map(String::as_str).chain(args.iter().copied()).collect();
        let run = run_in(&at.to_string_lossy(), "git", &every).map_err(|why| HomeFailure::Refused(format!("git could not be started: {why}")))?;
        match run.status.success() {
            true => Ok(run),
            false => Err(HomeFailure::Refused(said(&run))),
        }
    }

    /// One listing as the login, which names each repository's owner and topics.
    pub(crate) fn listed(&self) -> Result<Option<HomePlace>, HomeFailure> {
        let path = "/user/repos?affiliation=owner,organization_member&per_page=100";
        let paged = self.api.get_paged(path, LISTING_PAGES).map_err(|why| self.failed(why))?;
        let mut listed = Vec::new();
        for page in &paged.pages {
            listed.extend(page.decode::<Vec<Listed>>("the repositories of a login").map_err(|why| self.failed(why))?);
        }
        Ok(home_among(&listed, &self.login, &self.owners))
    }

    /// The repository at `place`, made where the forge has none. **As the login's own token**, a
    /// user repository through `/user/repos` and an organisation's through `/orgs/<org>/repos`.
    pub(crate) fn ensured(&self, place: &HomePlace) -> Result<(), HomeFailure> {
        let (owner, name) = (&place.owner, &place.name);
        match self.api.get(&format!("/repos/{owner}/{name}")) {
            Ok(there) => {
                let existing: Existing = there.decode("a repository").map_err(|why| self.failed(why))?;
                return match existing.permissions.is_some_and(|it| it.push) {
                    true => Ok(()),
                    false => Err(HomeFailure::cannot_write(&self.login, place)),
                };
            }
            Err(ApiFailure::Refused { status: 404, .. }) => {}
            Err(why) => return Err(self.failed(why)),
        }
        let asked = NewRepository {
            name,
            private: true,
            auto_init: true,
            description: "Armada's shared home: Kit carried between my machines",
        };
        let own = owner.eq_ignore_ascii_case(&self.login);
        let made = match own {
            true => self.api.post("/user/repos", &asked),
            false => self.api.post(&format!("/orgs/{owner}/repos"), &asked),
        };
        match made {
            Ok(_) => {}
            // GitHub answers 403 to a member who may not create, and 404 to one who is no member.
            Err(ApiFailure::Refused { status: 403 | 404, .. }) if !own => return Err(HomeFailure::cannot_make(&self.login, owner)),
            Err(why) => return Err(self.failed(why)),
        }
        self.api.put(&format!("/repos/{owner}/{name}/topics"), &Topics { names: [HOME_TOPIC] }).map_err(|why| self.failed(why))?;
        Ok(())
    }
}

impl HomeRepository for ForgeHome {
    /// Read off the clone's own `origin`, so a copy made for one login is never taken for another's.
    /// **The config file rather than `git config`**: signing in asks this on every read.
    fn here(&self) -> Option<String> {
        let config = std::fs::read_to_string(self.dir.join(".git").join("config")).ok()?;
        let origin = config.split("[remote \"origin\"]").nth(1)?;
        let url = origin.lines().map(str::trim).take_while(|line| !line.starts_with('[')).find_map(|line| {
            let (key, value) = line.split_once('=')?;
            (key.trim() == "url").then(|| value.trim().to_string())
        })?;
        let path = url.strip_prefix(&format!("https://{GITHUB_HOST}/"))?.trim_end_matches(".git");
        let owner = path.split('/').next()?;
        self.owners.iter().any(|accepted| accepted.eq_ignore_ascii_case(owner)).then(|| path.to_string())
    }

    fn open(&self, place: &HomePlace) -> Result<(), HomeFailure> {
        if self.here().is_some() {
            return self.pull();
        }
        self.holding()?;
        self.ensured(place)?;
        let parent = self.dir.parent().ok_or_else(|| HomeFailure::Refused("the home's folder has no parent".to_string()))?;
        std::fs::create_dir_all(parent).map_err(|why| HomeFailure::Refused(why.to_string()))?;
        let url = format!("https://{GITHUB_HOST}/{}/{}.git", place.owner, place.name);
        let into = self.dir.to_string_lossy().to_string();
        self.git(parent, &["clone", "--quiet", "--", &url, &into]).map(|_| ())
    }

    fn found(&self) -> Result<Option<HomePlace>, HomeFailure> {
        self.holding()?;
        self.listed()
    }

    fn pull(&self) -> Result<(), HomeFailure> {
        self.holding()?;
        self.git(&self.dir, &["pull", "--rebase", "--quiet"]).map(|_| ())
    }

    fn files(&self) -> &dyn KeptFolder {
        &self.files
    }

    /// One retry, after a rebase onto what another machine pushed meanwhile. **Each piece is its
    /// own files**, so two machines adding two pieces never touch one path.
    fn publish(&self, paths: &[String], message: &str) -> Result<(), HomeFailure> {
        self.holding()?;
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
