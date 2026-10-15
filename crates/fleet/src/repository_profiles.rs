//! Which profile a repository claims and syncs as. `docs/concepts/account.md`.
//!
//! **A repository is bound to one profile; the machine is not.** The repository's
//! owner, the first part of its forge slug `owner/name`, is matched to a profile
//! when it equals that profile's login or one of its organisations, which sign-in
//! read and kept. Exactly one match binds it. None, or several, leave it unbound
//! until a person chooses.
//!
//! A choice is kept per repository root in `repository-profiles.json` in the
//! machine directory, wins over a match, and may be "None": the repository then
//! claims and syncs nothing. **A choice names a profile by id and outlives it**:
//! signed out of that profile, the repository has no store, and is not handed to
//! another profile that happens to match.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use ipc::BoundHow;
use serde::{Deserialize, Serialize};

use crate::profiles::{write_beside, Joined};

/// The choices, in the machine directory.
pub const FILE: &str = "repository-profiles.json";

/// A served repository as the account needs to know it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Repo {
    pub root: String,
    /// The folder's own name.
    pub name: String,
    /// `owner/name` on its forge, lowercase. `None` where it has no remote.
    pub slug: Option<String>,
}

impl Repo {
    pub fn new(root: &str, slug: Option<String>) -> Repo {
        let name = Path::new(root)
            .file_name()
            .map_or_else(|| root.to_string(), |name| name.to_string_lossy().into_owned());
        Repo { root: root.to_string(), name, slug }
    }

    fn owner(&self) -> Option<String> {
        let owner = self.slug.as_deref()?.split('/').next()?.trim().to_lowercase();
        (!owner.is_empty()).then_some(owner)
    }
}

/// How a choice was made. Only a person makes one, so there is one word.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Made {
    Chosen,
}

/// What a person chose for a repository.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Choice {
    /// `None` is "None".
    pub account_id: Option<u64>,
    how: Made,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct Kept {
    #[serde(default)]
    repositories: BTreeMap<String, Choice>,
}

/// The choices of one machine directory.
#[derive(Clone, Debug)]
pub struct Choices {
    machine: PathBuf,
}

impl Choices {
    pub fn in_machine_directory(machine: &Path) -> Choices {
        Choices { machine: machine.to_path_buf() }
    }

    fn read(&self) -> Result<Kept, String> {
        let file = self.machine.join(FILE);
        match std::fs::read(&file) {
            Ok(bytes) => ipc::decode("the repository profiles", &bytes)
                .map_err(|why| format!("{} could not be read: {why}", file.display())),
            Err(why) if why.kind() == std::io::ErrorKind::NotFound => Ok(Kept::default()),
            Err(why) => Err(format!("{} could not be read: {why}", file.display())),
        }
    }

    pub fn all(&self) -> Result<BTreeMap<String, Choice>, String> {
        Ok(self.read()?.repositories)
    }

    /// Keep the choice for `root`, replacing an earlier one.
    pub fn choose(&self, root: &str, account_id: Option<u64>) -> Result<(), String> {
        let mut kept = self.read()?;
        kept.repositories.insert(root.to_string(), Choice { account_id, how: Made::Chosen });
        let text = ipc::encode(&kept).map_err(|why| why.to_string())?;
        write_beside(&self.machine, FILE, &text)
    }
}

/// Where a repository stands.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Binding {
    Bound { account_id: u64, how: BoundHow },
    Unbound {
        /// The profiles its owner matched.
        candidates: Vec<u64>,
        /// A person chose "None".
        none_chosen: bool,
    },
}

/// The profiles whose login or organisations are the repository's owner.
fn matching(joined: &[Joined], repo: &Repo) -> Vec<u64> {
    let Some(owner) = repo.owner() else { return Vec::new() };
    joined
        .iter()
        .filter(|one| one.login.to_lowercase() == owner || one.orgs.iter().any(|org| org.to_lowercase() == owner))
        .map(|one| one.account_id)
        .collect()
}

/// Where `repo` stands among `joined`, given the choice made for it.
pub fn binding(joined: &[Joined], choice: Option<&Choice>, repo: &Repo) -> Binding {
    let candidates = matching(joined, repo);
    match choice.map(|choice| choice.account_id) {
        Some(Some(id)) if joined.iter().any(|one| one.account_id == id) => {
            Binding::Bound { account_id: id, how: BoundHow::Chosen }
        }
        Some(Some(_)) => Binding::Unbound { candidates, none_chosen: false },
        Some(None) => Binding::Unbound { candidates, none_chosen: true },
        None => match candidates[..] {
            [only] => Binding::Bound { account_id: only, how: BoundHow::Matched },
            _ => Binding::Unbound { candidates, none_chosen: false },
        },
    }
}
