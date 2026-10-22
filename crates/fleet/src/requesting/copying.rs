//! Collections and environments copied from the person's own file into another: the repository's
//! `.armada/requests.toml` on a commit's branch, or a package. **A secret goes as its name and
//! `secret = true`**, which is all [`RequestsFile`] can hold of one.

use ipc::RequestsFile;

/// Why nothing was copied.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NotCopied {
    /// Nothing was chosen, or the person's own holds no collection or environment of that id or
    /// name.
    NotYours(String),
    /// `onto` already holds one of that id or name and `overwrite` was not asked for.
    Exists(String),
}

/// `onto` with the collections `from` holds by id and the environments by name, each replacing
/// one of its id or name where `overwrite` is asked for, and added after the rest otherwise.
pub fn copied(
    mut onto: RequestsFile,
    from: &RequestsFile,
    collections: &[String],
    environments: &[String],
    overwrite: bool,
) -> Result<RequestsFile, NotCopied> {
    if collections.is_empty() && environments.is_empty() {
        return Err(NotCopied::NotYours(String::from("No collection or environment is chosen.")));
    }
    for id in collections {
        let Some(collection) = from.collections.iter().find(|one| &one.id == id) else {
            return Err(NotCopied::NotYours(format!("Your requests for this repository have no collection `{id}`.")));
        };
        match onto.collections.iter_mut().find(|one| &one.id == id) {
            Some(_) if !overwrite => return Err(NotCopied::Exists(format!("collection `{id}`"))),
            Some(held) => *held = collection.clone(),
            None => onto.collections.push(collection.clone()),
        }
    }
    for name in environments {
        let Some(environment) = from.environments.iter().find(|one| &one.name == name) else {
            return Err(NotCopied::NotYours(format!("Your requests for this repository have no environment `{name}`.")));
        };
        match onto.environments.iter_mut().find(|one| &one.name == name) {
            Some(_) if !overwrite => return Err(NotCopied::Exists(format!("environment `{name}`"))),
            Some(held) => *held = environment.clone(),
            None => onto.environments.push(environment.clone()),
        }
    }
    Ok(onto)
}
