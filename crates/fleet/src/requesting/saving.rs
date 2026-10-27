//! Saves into the person's own file: a collection whole, an environment whole with its secrets
//! kept in the Keychain first, an environment deleted with its secrets forgotten first, and one
//! request an agent made.
//!
//! **Every save reads the person's own file first and refuses where it will not read**, so a file
//! a person broke by hand is never written over with what an empty one would hold. A save never
//! writes the repository's file.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, SecretVariable, Vcs, WorkProduct};
use api::Refusal;
use ipc::{
    RequestCollection, RequestEnvironmentWrite, RequestPlace, RequestSaved, RequestToSave,
    RequestVariable, Requests, RequestsFile, SavedRequest, WireError,
};

use super::files::{
    check_collection, check_environment_write, check_request, environment_written, merged, unique_id,
    SecretChange,
};
use super::filling::by_id_or_name;
use super::kept::SAVING;
use crate::daemon::Fleet;
use crate::run_memory::places;

/// A collection or a request that breaks a rule. A 422.
pub(super) const COLLECTION_REFUSED: &str = "fleet.request_collection_refused";
/// An environment that breaks a rule. A 422.
pub(super) const ENVIRONMENT_REFUSED: &str = "fleet.request_environment_refused";
/// The person's own holds no environment of that name. A 404.
pub(super) const NO_SUCH_ENVIRONMENT: &str = "fleet.no_such_request_environment";
/// The Keychain would not keep, read or forget a secret. A 500.
pub(super) const SECRET_UNKEPT: &str = "fleet.request_secret_unkept";

/// What an agent's save came to, for its answer and its row.
pub(super) struct SavedTo {
    pub(super) saved: RequestSaved,
    pub(super) name: String,
    pub(super) collection: String,
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
    fn requests_refused(&self, code: &str, why: String) -> Refusal {
        Refusal::Unacceptable(WireError::raised(code, why, self.run_id()))
    }

    /// `save_request_collection`: one collection whole into the person's own, replacing one of
    /// its id.
    pub(super) async fn request_collection_saved(
        &self,
        root: &str,
        job: Option<&str>,
        collection: RequestCollection,
    ) -> Result<Requests, Refusal> {
        check_collection(&collection, "collection").map_err(|why| self.requests_refused(COLLECTION_REFUSED, why))?;
        {
            let _one_save = SAVING.lock().await;
            let mut own = self.own_requests_to_change(root).await?;
            match own.collections.iter_mut().find(|one| one.id == collection.id) {
                Some(held) => *held = collection,
                None => own.collections.push(collection),
            }
            self.own_requests_written(root, &own).await?;
        }
        self.requests_answer(root, job).await
    }

    /// `save_request_environment`: one environment whole into the person's own, each secret's
    /// value into the Keychain first, so the file never names a secret the Keychain refused.
    pub(super) async fn request_environment_saved(
        &self,
        root: &str,
        job: Option<&str>,
        write: RequestEnvironmentWrite,
    ) -> Result<Requests, Refusal> {
        check_environment_write(&write, "environment").map_err(|why| self.requests_refused(ENVIRONMENT_REFUSED, why))?;
        {
            let _one_save = SAVING.lock().await;
            let mut own = self.own_requests_to_change(root).await?;
            let before = own.environments.iter().find(|one| one.name == write.name).cloned();
            let (environment, changes) = environment_written(write, before.as_ref());
            self.secrets_changed(root, &environment.name, changes).await?;
            match own.environments.iter_mut().find(|one| one.name == environment.name) {
                Some(held) => *held = environment,
                None => own.environments.push(environment),
            }
            self.own_requests_written(root, &own).await?;
        }
        self.requests_answer(root, job).await
    }

    /// `delete_request_environment`: the environment `name` out of the person's own, each
    /// secret's Keychain item forgotten first, so the file never loses an environment whose
    /// secrets the Keychain still holds.
    pub(super) async fn request_environment_deleted(
        &self,
        root: &str,
        job: Option<&str>,
        name: &str,
    ) -> Result<Requests, Refusal> {
        {
            let _one_save = SAVING.lock().await;
            let mut own = self.own_requests_to_change(root).await?;
            let Some(at) = own.environments.iter().position(|one| one.name == name) else {
                return Err(Refusal::NotFound(WireError::raised(
                    NO_SUCH_ENVIRONMENT,
                    format!("Your requests for this repository hold no environment called {name}."),
                    self.run_id(),
                )));
            };
            let changes = own.environments[at]
                .variables
                .iter()
                .filter_map(|variable| match variable {
                    RequestVariable::Secret { name } => Some(SecretChange::Forget { name: name.clone() }),
                    RequestVariable::Plain { .. } => None,
                })
                .collect();
            self.secrets_changed(root, name, changes).await?;
            own.environments.remove(at);
            self.own_requests_written(root, &own).await?;
        }
        self.requests_answer(root, job).await
    }

    /// Keep and forget in the Keychain, in order, stopping at the first it will not do.
    async fn secrets_changed(&self, root: &str, environment: &str, changes: Vec<SecretChange>) -> Result<(), Refusal> {
        if changes.is_empty() {
            return Ok(());
        }
        let (repository, environment) = (places::key_of(root), environment.to_string());
        let secrets = Arc::clone(&self.requests().secrets);
        let done = tokio::task::spawn_blocking(move || {
            for change in changes {
                let (name, done) = match change {
                    SecretChange::Keep { name, value } => {
                        let variable = SecretVariable { repository: repository.clone(), environment: environment.clone(), name };
                        let done = secrets.keep(&variable, value);
                        (variable.name, done.map_err(|why| ("keep", why)))
                    }
                    SecretChange::Forget { name } => {
                        let variable = SecretVariable { repository: repository.clone(), environment: environment.clone(), name };
                        let done = secrets.forget(&variable);
                        (variable.name, done.map_err(|why| ("forget", why)))
                    }
                };
                if let Err((verb, why)) = done {
                    return Err(format!("The Keychain would not {verb} the secret {name}: {}", why.said()));
                }
            }
            Ok(())
        })
        .await
        .unwrap_or_else(|why| Err(format!("The Keychain was not reached: {why}")));
        done.map_err(|why| Refusal::Fault(WireError::raised(SECRET_UNKEPT, why, self.run_id())))
    }

    /// A package's collections and environments put into the person's own for `root`, each in
    /// place of one of its id or name. Checked before it is written.
    pub(crate) async fn requests_added(&self, root: &str, adding: &RequestsFile) -> Result<(), Refusal> {
        let collections: Vec<String> = adding.collections.iter().map(|one| one.id.clone()).collect();
        let environments: Vec<String> = adding.environments.iter().map(|one| one.name.clone()).collect();
        let _one_save = SAVING.lock().await;
        let own = self.own_requests_to_change(root).await?;
        let merged = super::copied(own, adding, &collections, &environments, true)
            .map_err(|why| self.requests_refused(COLLECTION_REFUSED, format!("{why:?}")))?;
        super::check(&merged).map_err(|why| self.requests_refused(COLLECTION_REFUSED, why))?;
        self.own_requests_written(root, &merged).await
    }

    /// `save_request_to_collection`: `request` into the collection `named` by id or name. One of
    /// that name is made in the person's own where none is listed, and a repository's is copied
    /// into the person's own first. **A request of the same name is replaced and keeps its id.**
    pub(super) async fn request_saved_to_collection(
        &self,
        root: &str,
        job: &str,
        named: &str,
        request: RequestToSave,
    ) -> Result<SavedTo, Refusal> {
        let refused = |why: String| self.requests_refused(COLLECTION_REFUSED, why);
        let named = named.trim();
        if named.is_empty() {
            return Err(refused(String::from("collection is blank")));
        }
        let RequestToSave { name, method, url, headers, body } = request;
        let name = name.trim().to_string();
        let mut saving = SavedRequest { id: String::from("new"), name, method, url, headers, body };
        check_request(&saving, "request").map_err(refused)?;
        let _one_save = SAVING.lock().await;
        let mut own = self.own_requests_to_change(root).await?;
        let theirs = self.repository_requests(root, Some(job)).await.unwrap_or_default();
        let listed = merged(&own.collections, &theirs.collections, |one| one.id.as_str());
        let found = by_id_or_name(&listed, named, |one| &one.item.id, |one| &one.item.name);
        let at = match found {
            Some(placed) if placed.place == RequestPlace::Yours => {
                own.collections.iter().position(|one| one.id == placed.item.id).unwrap_or_default()
            }
            Some(placed) => {
                own.collections.push(placed.item.clone());
                own.collections.len() - 1
            }
            None => {
                let id = unique_id(named, listed.iter().map(|one| one.item.id.as_str()));
                own.collections.push(RequestCollection { id, name: named.to_string(), headers: Vec::new(), requests: Vec::new() });
                own.collections.len() - 1
            }
        };
        let collection = &mut own.collections[at];
        match collection.requests.iter_mut().find(|one| one.name == saving.name) {
            Some(held) => {
                saving.id = held.id.clone();
                *held = saving.clone();
            }
            None => {
                saving.id = unique_id(&saving.name, collection.requests.iter().map(|one| one.id.as_str()));
                collection.requests.push(saving.clone());
            }
        }
        let (collection_id, collection_name) = (collection.id.clone(), collection.name.clone());
        self.own_requests_written(root, &own).await?;
        Ok(SavedTo {
            saved: RequestSaved { collection_id, request_id: saving.id },
            name: saving.name,
            collection: collection_name,
        })
    }
}
