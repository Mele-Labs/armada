//! Which profile a repository claims as. `docs/concepts/account.md`.
//!
//! **A project picks the login that joins its store** (`store.account_id` in its file), and a
//! repository claims as it. A project that picks none sends nothing. **A repository may override
//! it**: a choice kept per repository root in `repository-profiles.json` in the machine directory
//! wins over the project's login, and may be "None": the repository then claims nothing. A
//! repository's owner binds nothing.
//!
//! **A choice names a profile by id and outlives it**: signed out of that profile, the repository has
//! no store, and is not handed to the project's login.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::profiles::write_beside;

/// The choices, in the machine directory.
pub const FILE: &str = "repository-profiles.json";

/// A served repository as the account needs to know it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Repo {
    pub root: String,
    /// The folder's own name.
    pub name: String,
}

impl Repo {
    pub fn new(root: &str) -> Repo {
        let name = Path::new(root)
            .file_name()
            .map_or_else(|| root.to_string(), |name| name.to_string_lossy().into_owned());
        Repo { root: root.to_string(), name }
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

/// The account a repository claims as: **its own choice where it has one**, "None" included, else the
/// login its project picked. `None` claims nothing, and so does any repository whose project picked no
/// login: there is nothing to override.
pub fn claims_as(choice: Option<&Choice>, project: Option<u64>) -> Option<u64> {
    let project = project?;
    match choice {
        Some(choice) => choice.account_id,
        None => Some(project),
    }
}
