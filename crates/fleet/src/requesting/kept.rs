//! Where Requests' two files are for a repository, read on each call and never cached.
//!
//! | place | the person's own |
//! |---|---|
//! | `this_mac` | `requests/<key>.toml` under Kit's folder |
//! | `synced` | the same under the folder the person named |
//! | `cloud`, a bucket | `<prefix>/requests/<key>.toml` |
//! | `cloud`, Armada's store, and `repository` | the same as `this_mac`, and the answer says `this_mac` |
//!
//! The repository's is its committed `.armada/requests.toml`, read from the tree of the Job named
//! where it holds a slot and from the checkout otherwise, as `crate::flight_controls::repository`
//! reads its own. Nothing here writes it.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, SecretVariable, Vcs, WorkProduct};
use adapters::BucketAddress;
use api::Refusal;
use ipc::{
    Placed, RequestEnvironment, RequestEnvironmentRead, RequestPlace, RequestVariable, Requests,
    RequestsFile, RequestsNotRead, RunPlace, SessionId, VariableRead, WireError,
};

use super::files::{merged, parse, written};
use super::Filling;
use crate::daemon::Fleet;
use crate::run_memory::{cloud, places};

/// The folder beside `runs/` that holds each repository's file.
pub(crate) const FOLDER: &str = "requests";
/// The repository's own file, committed, beside `.armada/flight-controls.toml`.
pub(crate) const IN_REPOSITORY: &str = ".armada/requests.toml";
/// The person's own file will not read, so it is not written over. A 409.
pub(crate) const UNREAD: &str = "fleet.requests_unread";
/// The place could not be written. A 500.
pub(crate) const UNWRITABLE: &str = "fleet.requests_unwritable";

/// One writer of a file at a time, so two writes cannot interleave their staging file.
static WRITING: std::sync::Mutex<()> = std::sync::Mutex::new(());
/// One save at a time, so a read, a change and a write are one step and no save is lost.
pub(super) static SAVING: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// Where the person's own file is.
enum Kept {
    File(PathBuf),
    Object(BucketAddress),
    /// The place cannot hold it now: the sentence that says why.
    Nowhere(String),
}

/// The file at `path`. An absent file is an empty one.
pub(crate) async fn read_file(path: &Path) -> Result<RequestsFile, String> {
    match tokio::fs::read_to_string(path).await {
        Ok(text) => parse(&text).map_err(|why| format!("{}: {why}", path.display())),
        Err(cause) if cause.kind() == std::io::ErrorKind::NotFound => Ok(RequestsFile::default()),
        Err(cause) => Err(format!("{}: {cause}", path.display())),
    }
}

/// Write `text` to `<path>.writing` and rename it over `path`.
fn write_over(path: &Path, text: &str) -> std::io::Result<()> {
    let _one_at_a_time = WRITING.lock().unwrap_or_else(|held| held.into_inner());
    if let Some(folder) = path.parent() {
        std::fs::create_dir_all(folder)?;
    }
    let mut staged = path.as_os_str().to_owned();
    staged.push(".writing");
    let staged = PathBuf::from(staged);
    std::fs::write(&staged, text).and_then(|()| std::fs::rename(&staged, path))
}

fn items<T>(placed: Vec<Placed<T>>) -> Vec<T> {
    placed.into_iter().map(|one| one.item).collect()
}

/// An environment as Requests answers it: each secret by whether `kept` says a value is kept.
fn environment_read(environment: &RequestEnvironment, kept: impl Fn(&str) -> bool) -> RequestEnvironmentRead {
    let variables = environment
        .variables
        .iter()
        .map(|variable| match variable {
            RequestVariable::Plain { name, value } => VariableRead::Plain { name: name.clone(), value: value.clone() },
            RequestVariable::Secret { name } => VariableRead::Secret { name: name.clone(), set: kept(name) },
        })
        .collect();
    RequestEnvironmentRead { name: environment.name.clone(), variables }
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
    async fn requests_kept(&self, root: &str) -> Result<(RunPlace, Kept), Refusal> {
        let held = self.kept_place(root).await?;
        let (place, at) = places::where_kept(Path::new(&self.host().kit_home), held.as_ref());
        let kept = match at {
            places::Where::Folder(base) => Kept::File(places::file_for(&base, FOLDER, root)),
            places::Where::Bucket(address, _) => Kept::Object(address),
            places::Where::Nowhere(why) => Kept::Nowhere(why),
        };
        Ok((place, kept))
    }

    /// The person's own file for `root`, and the place it was read at.
    pub(super) async fn own_requests(&self, root: &str) -> Result<(RunPlace, Result<RequestsFile, String>), Refusal> {
        let (place, kept) = self.requests_kept(root).await?;
        let read = match kept {
            Kept::File(path) => read_file(&path).await,
            Kept::Object(address) => {
                let object = places::object_for(FOLDER, root);
                let (reaching, at) = (Arc::clone(self.reaching()), object.clone());
                match cloud::blocking(move || reaching.bucket(&address)?.get(&at)).await {
                    Ok(None) => Ok(RequestsFile::default()),
                    Ok(Some(bytes)) => String::from_utf8(bytes)
                        .map_err(|_| String::from("is not text"))
                        .and_then(|text| parse(&text))
                        .map_err(|why| format!("The object {object}: {why}")),
                    Err(fault) => Err(fault.said().to_string()),
                }
            }
            Kept::Nowhere(why) => Err(why),
        };
        Ok((place, read))
    }

    /// The person's own file for `root`, or why it did not read.
    pub(crate) async fn own_requests_file(&self, root: &str) -> Result<RequestsFile, String> {
        match self.own_requests(root).await {
            Ok((_, read)) => read,
            Err(_) => Err(String::from("the place chosen for this repository could not be read")),
        }
    }

    /// The person's own file, to change and write back. **One that will not read is refused**,
    /// never saved over.
    pub(super) async fn own_requests_to_change(&self, root: &str) -> Result<RequestsFile, Refusal> {
        match self.own_requests(root).await?.1 {
            Ok(file) => Ok(file),
            Err(why) => Err(Refusal::IllegalMove(WireError::raised(
                UNREAD,
                format!("Your requests for this repository did not read, so they are not written over: {why}"),
                self.run_id(),
            ))),
        }
    }

    /// Write the person's own file whole, already checked.
    pub(super) async fn own_requests_written(&self, root: &str, file: &RequestsFile) -> Result<(), Refusal> {
        let unwritable = |named: &str, why: String| {
            Refusal::Fault(WireError::raised(UNWRITABLE, format!("{named} could not be written: {why}"), self.run_id()))
        };
        match self.requests_kept(root).await?.1 {
            Kept::File(path) => {
                let named = path.display().to_string();
                let text = written(file).map_err(|why| unwritable(&named, why))?;
                tokio::task::spawn_blocking(move || write_over(&path, &text))
                    .await
                    .map_err(|why| unwritable(&named, why.to_string()))?
                    .map_err(|why| unwritable(&named, why.to_string()))
            }
            Kept::Object(address) => {
                let object = places::object_for(FOLDER, root);
                let text = written(file).map_err(|why| unwritable(&object, why))?;
                let reaching = Arc::clone(self.reaching());
                cloud::blocking(move || reaching.bucket(&address)?.put(&object, text.as_bytes()))
                    .await
                    .map_err(|fault| Refusal::Unacceptable(WireError::raised(fault.code(), fault.said().to_string(), self.run_id())))
            }
            Kept::Nowhere(why) => Err(unwritable("Your requests for this repository", why)),
        }
    }

    /// The repository's own committed file, from the tree of the Job `job` names or the checkout.
    pub(super) async fn repository_requests(&self, root: &str, job: Option<&str>) -> Result<RequestsFile, String> {
        let named = job.map(SessionId::carried);
        let tree = match self.job_tree_in(root, named.as_ref()).await {
            Some(tree) => PathBuf::from(tree.path),
            None => PathBuf::from(root),
        };
        read_file(&tree.join(IN_REPOSITORY)).await
    }

    /// The repository a Job stands in, where Fleet serves it.
    pub(super) async fn job_repository(&self, job: &str) -> Option<String> {
        let session = self.store().lock().await.session(job).ok().flatten()?;
        let manifest = match session.manifest_id.filter(|id| !id.is_empty()) {
            Some(id) => id,
            None => self.session_and_hosting(job).await.ok()?.1.manifest_id,
        };
        let served = self.served_named(Some(&ipc::ManifestId::carried(&manifest))).ok()?;
        Some(served.root().to_string())
    }

    /// `get_requests`' answer for `root`: both files, the person's own over the repository's,
    /// and a server's environment for each server `job` runs.
    pub(super) async fn requests_answer(&self, root: &str, job: Option<&str>) -> Result<Requests, Refusal> {
        let (place, own) = self.own_requests(root).await?;
        let theirs = self.repository_requests(root, job).await;
        let mut not_read = Vec::new();
        let mut read = |placed: RequestPlace, file: Result<RequestsFile, String>| match file {
            Ok(file) => file,
            Err(why) => {
                not_read.push(RequestsNotRead { place: placed, why });
                RequestsFile::default()
            }
        };
        let (own, theirs) = (read(RequestPlace::Yours, own), read(RequestPlace::Repository, theirs));
        let collections = merged(&own.collections, &theirs.collections, |one| one.id.as_str());
        let environments = merged(&own.environments, &theirs.environments, |one| one.name.as_str());
        let kept = self.secrets_kept(root, &environments).await;
        let mut environments: Vec<Placed<RequestEnvironmentRead>> = environments
            .iter()
            .map(|placed| Placed {
                item: environment_read(&placed.item, |name| kept.contains(&(placed.item.name.clone(), name.to_string()))),
                place: placed.place,
                hides: placed.hides,
            })
            .collect();
        environments.extend(self.server_environments(job).await.iter().map(|server| Placed {
            item: environment_read(server, |_| false),
            place: RequestPlace::Server,
            hides: false,
        }));
        Ok(Requests { repository: root.to_string(), place, collections, environments, not_read })
    }

    /// Each secret of `environments` the Keychain holds a value for, by environment and name. A
    /// Keychain that will not answer holds none.
    async fn secrets_kept(&self, root: &str, environments: &[Placed<RequestEnvironment>]) -> Vec<(String, String)> {
        let repository = places::key_of(root);
        let asked: Vec<SecretVariable> = environments
            .iter()
            .flat_map(|placed| {
                placed.item.variables.iter().filter_map(|variable| match variable {
                    RequestVariable::Secret { name } => Some(SecretVariable {
                        repository: repository.clone(),
                        environment: placed.item.name.clone(),
                        name: name.clone(),
                    }),
                    RequestVariable::Plain { .. } => None,
                })
            })
            .collect();
        if asked.is_empty() {
            return Vec::new();
        }
        let secrets = Arc::clone(&self.requests().secrets);
        tokio::task::spawn_blocking(move || {
            asked
                .into_iter()
                .filter(|one| secrets.is_kept(one).unwrap_or(false))
                .map(|one| (one.environment, one.name))
                .collect()
        })
        .await
        .unwrap_or_default()
    }

    /// The values of the Requests environment `named`, for `job`'s watch command, every secret
    /// read from the Keychain. `Err` says why, and never holds a value.
    pub(crate) async fn watch_values(&self, job: &str, named: &str) -> Result<Option<super::Values>, String> {
        let root = self.job_repository(job).await;
        let filling = self.filling_for(root.as_deref(), Some(job)).await.map_err(|why| why.error().message.clone())?;
        let secrets = std::sync::Arc::clone(&self.requests().secrets);
        let named = named.to_string();
        let read = tokio::task::spawn_blocking(move || filling.every_value(&named, &*secrets))
            .await
            .map_err(|why| format!("The environment could not be read: {why}."))?;
        match read {
            Ok(values) => Ok(Some(values)),
            Err(super::Unsent::Refused(why) | super::Unsent::Unkept(why)) => Err(format!("Its Requests environment: {why}.")),
        }
    }

    /// What a send for `root` is filled from: both files, the person's own over the repository's,
    /// and `job`'s servers. A file that will not read lends nothing.
    pub(super) async fn filling_for(&self, root: Option<&str>, job: Option<&str>) -> Result<Filling, Refusal> {
        let mut filling = Filling::default();
        if let Some(root) = root {
            let own = self.own_requests(root).await?.1.unwrap_or_default();
            let theirs = self.repository_requests(root, job).await.unwrap_or_default();
            filling.collections = items(merged(&own.collections, &theirs.collections, |one| one.id.as_str()));
            filling.environments = items(merged(&own.environments, &theirs.environments, |one| one.name.as_str()));
            filling.repository = Some(places::key_of(root));
        }
        filling.environments.extend(self.server_environments(job).await);
        Ok(filling)
    }
}
