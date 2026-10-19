//! Fleet over its [`Repositories`](super::Repositories): which one a request
//! is in, and adding one by folder.

use std::path::PathBuf;
use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{AddRepository, RepositoryList, RepositorySummary, WireError};

use super::{NotAdded, NotLocated, Repository, Served};
use crate::adrift::Adrift;
use crate::daemon::Fleet;
use crate::wire::manifest_summary;

/// A folder that is not a repository's root. A 422.
const NOT_A_REPOSITORY: &str = "fleet.not_a_repository";
/// A repository whose `armada.yml` or workflows will not load. A 422.
const REPOSITORY_REFUSED: &str = "fleet.repository_refused";
/// A folder or a Manifest id already served. A 409.
const ALREADY_SERVED: &str = "fleet.repository_served";
/// A repository served and not written down, so a restart forgets it. A 500.
const NOT_REMEMBERED: &str = "fleet.repository_not_remembered";
/// A `?repository=` naming nothing served. A 422.
pub(crate) const NO_SUCH_REPOSITORY: &str = "fleet.no_such_repository";
/// A request that needs a repository, when Fleet serves none yet. A 422.
const NO_REPOSITORY: &str = "fleet.no_repository";

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
    /// The repository a Manifest-scoped request names. Absent is the first
    /// added that has a Manifest, and refused where there is none.
    pub(crate) fn served_named(&self, named: Option<&ipc::ManifestId>) -> Result<Served, Refusal> {
        let Some(named) = named else {
            return self
                .repositories()
                .first()
                .ok_or_else(|| self.nothing_served("has a Manifest"));
        };
        self.repositories().serving(named.as_str()).ok_or_else(|| {
            self.refusal(Adrift::NoSuchManifest {
                named: named.as_str().to_string(),
                held: self.held_manifests(),
            })
        })
    }

    /// Every Manifest id served, for a refusal to name.
    pub(crate) fn held_manifests(&self) -> String {
        let held: Vec<String> = self
            .repositories()
            .served()
            .iter()
            .map(|one| one.manifest().id().as_str().to_string())
            .collect();
        held.join("`, `")
    }

    /// The repository Scan and its proposals read, by root. Absent is the
    /// first added, Manifest or none, and refused where nothing is served.
    pub(crate) fn repository_named(&self, root: Option<&str>) -> Result<Arc<Repository>, Refusal> {
        let Some(root) = root else {
            let first = self.repositories().every().into_iter().next();
            return first.ok_or_else(|| self.nothing_served("is served"));
        };
        self.repositories().at(root).ok_or_else(|| {
            Refusal::Unacceptable(WireError::raised(
                NO_SUCH_REPOSITORY,
                format!("{root} is not a repository this Fleet serves; `list_repositories` says which are"),
                self.run_id(),
            ))
        })
    }

    /// The repository a fixture starts in, for a test that has only the one.
    #[cfg(test)]
    pub(crate) fn first(&self) -> Served {
        self.repositories()
            .first()
            .expect("a fixture starts in a repository")
    }

    /// Refused plainly: Fleet serves no repository that `held` yet.
    fn nothing_served(&self, held: &str) -> Refusal {
        Refusal::Unacceptable(WireError::raised(
            NO_REPOSITORY,
            format!("No repository {held} yet, so add one by folder or clone one from its URL"),
            self.run_id(),
        ))
    }

    /// Say the list changed, whole, so every Bridge's picker reads it again.
    fn published_repositories(&self) {
        let list = self.repository_list();
        self.rehearsals().workspace_dirs().forget(None);
        self.events().publish(ipc::Event::RepositoriesChanged(list));
    }

    /// `list_repositories`.
    pub(crate) fn repository_list(&self) -> RepositoryList {
        RepositoryList {
            repositories: self
                .repositories()
                .every()
                .iter()
                .map(|one| summary_of(one))
                .collect(),
        }
    }

    /// `add_repository`: read the folder, serve it, and reconcile the Jobs the
    /// store already holds for it — a repository served again after a restart
    /// is cleaned up the way the one Fleet started in is.
    pub(crate) async fn added_repository(
        &self,
        asked: AddRepository,
    ) -> Result<RepositorySummary, Refusal> {
        let folder = PathBuf::from(&asked.path);
        // Before the read, so a second add of one folder costs nothing. `add`
        // below asks again under its lock.
        if let Some(served) = folder
            .canonicalize()
            .ok()
            .and_then(|root| self.repositories().at(&root.to_string_lossy()))
        {
            return Err(self.not_added(NotAdded::AlreadyServed {
                root: served.root().to_string(),
            }));
        }
        if !folder.is_absolute() {
            return Err(self.not_located(NotLocated::NotARepository {
                folder: asked.path,
                why: "a relative path names a folder under Fleet's own working directory".into(),
            }));
        }
        let located = self.located_or_started(folder).await?;
        let added = self
            .repositories()
            .add(located)
            .map_err(|why| self.not_added(why))?;
        // Served already; a store that will not write only loses it at restart.
        let at = self.now();
        let remembered = self
            .store()
            .lock()
            .await
            .remember_repository(added.root(), &at);
        // Before reconciling, which can refuse: the repository is served either way.
        self.published_repositories();
        if Served::of(&added).is_some() {
            self.locating().serving(added.root());
        }
        self.reconciled_in(&Served::hosting(&added))
            .await
            .map_err(|why| self.refusal(why))?;
        remembered.map_err(|why| {
            Refusal::Fault(WireError::raised(
                NOT_REMEMBERED,
                format!(
                    "{} is served, and will not be served after a restart: {why}",
                    added.root()
                ),
                self.run_id(),
            ))
        })?;
        Ok(summary_of(&added))
    }

    /// Read `folder`, starting a git repository in it first where it is in none.
    /// A subfolder of a repository is never started, and is refused as it was.
    pub(super) async fn located_or_started(
        &self,
        folder: PathBuf,
    ) -> Result<super::Located, Refusal> {
        let locating = Arc::clone(self.locating());
        let read = {
            let (locating, folder) = (Arc::clone(&locating), folder.clone());
            move || locating.located(&folder)
        };
        let first = tokio::task::spawn_blocking(read.clone())
            .await
            .expect("reading a folder panicked");
        match first {
            Err(NotLocated::InNoRepository { .. }) => {
                tokio::task::spawn_blocking(move || {
                    locating.started(&folder)?;
                    read()
                })
                .await
                .expect("starting a repository panicked")
                .map_err(|why| self.not_located(why))
            }
            other => other.map_err(|why| self.not_located(why)),
        }
    }

    /// Serve at start a folder `armada serve` was given, read before the bind:
    /// added and remembered as an add is, and reconciled with the rest.
    pub async fn served_at_start(&self, located: super::Located) -> Result<(), String> {
        let added = self
            .repositories()
            .add(located)
            .map_err(|why| why.to_string())?;
        let at = self.now();
        let remembered = self
            .store()
            .lock()
            .await
            .remember_repository(added.root(), &at);
        if Served::of(&added).is_some() {
            self.locating().serving(added.root());
        }
        remembered.map_err(|why| format!("{} will not be remembered: {why}", added.root()))
    }

    /// Serve again every repository the store remembers, **before
    /// reconciliation**, so their Jobs are reconciled together. What is served
    /// already is remembered too. Answers what would not be served, and why: a
    /// remembered folder that is gone stays remembered.
    pub async fn served_again(&self) -> Vec<String> {
        let at = self.now();
        let mut store = self.store().lock().await;
        let mut said = Vec::new();
        for held in self.repositories().every() {
            if let Err(why) = store.remember_repository(held.root(), &at) {
                said.push(format!("{} will not be remembered: {why}", held.root()));
            }
        }
        let remembered = match store.remembered_repositories() {
            Ok(roots) => roots,
            Err(why) => return [said, vec![why.to_string()]].concat(),
        };
        drop(store);
        for root in remembered {
            if self.repositories().at(&root).is_some() {
                continue;
            }
            let located = self.locating().located(std::path::Path::new(&root));
            match located.map_err(|why| why.to_string()).and_then(|located| {
                self.repositories()
                    .add(located)
                    .map_err(|why| why.to_string())
            }) {
                Ok(added) => {
                    if Served::of(&added).is_some() {
                        self.locating().serving(added.root());
                    }
                }
                Err(why) => said.push(why),
            }
        }
        said
    }

    /// Give a repository served without a Manifest the one Write just put at
    /// its root. **Quiet where nothing loads**: Write has already answered, and
    /// what the file came to is the Manifest surface's to read.
    pub(crate) fn set_up_after_write(&self, repository: &Repository) {
        if repository.manifest().is_some() {
            return;
        }
        let Ok(located) = self
            .locating()
            .located(std::path::Path::new(repository.root()))
        else {
            return;
        };
        if let Some(set_up) = located.set_up {
            if let Ok(served) = self.repositories().set_up(repository.root(), set_up) {
                self.locating().serving(served.root());
                self.published_repositories();
            }
        }
    }

    fn not_located(&self, why: NotLocated) -> Refusal {
        let code = match why {
            NotLocated::NotARepository { .. } | NotLocated::InNoRepository { .. } => NOT_A_REPOSITORY,
            NotLocated::Refused { .. } => REPOSITORY_REFUSED,
        };
        Refusal::Unacceptable(WireError::raised(code, why.to_string(), self.run_id()))
    }

    fn not_added(&self, why: NotAdded) -> Refusal {
        Refusal::IllegalMove(WireError::raised(
            ALREADY_SERVED,
            why.to_string(),
            self.run_id(),
        ))
    }
}

/// One repository as the wire carries it.
fn summary_of(repository: &Repository) -> RepositorySummary {
    RepositorySummary {
        root: repository.root().to_string(),
        records_root: repository.records_root().to_string(),
        manifest: repository
            .manifest()
            .map(|held| manifest_summary(held, repository.records_root())),
        implicit_id: repository
            .manifest()
            .is_none()
            .then(|| ipc::ManifestId::carried(repository.implicit_id())),
    }
}
