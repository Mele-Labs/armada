//! The `cloud` place, in its two forms: Armada's central store, and a bucket the person owns.
//!
//! | form | holds | credentials |
//! |---|---|---|
//! | `store` | the facts as rows under a hash of the repository's remote | the sign-in of the profile the repository is bound to |
//! | bucket address | the Kit layout as one object, `<prefix>/runs/<owner--name>.toml` | the provider's own environment and files, never kept |
//!
//! **A sync reads, merges and then writes**, as a synced folder does: what another machine added
//! stays, a line the person dropped is not merged back, and the write is the whole set. A read that
//! fails stops the sync before it writes, so a remote copy that would not read is never written over.
//!
//! **A failed sync loses nothing.** The facts stay in Fleet's own record, each marked waiting with
//! what it waits on, and the next turn's end syncs again. Network calls take two opens of the
//! store or the bucket, one to read and one to write, because the merge between them needs Fleet's
//! record, which an async worker holds and a blocking thread cannot.

use std::sync::Arc;

use adapter_traits::{BucketError, RunRow, StoreError};
use adapters::BucketAddress;
use ipc::RunCloud;
use store::KeptRun;

use super::places;
use crate::daemon::Fleet;
use crate::red_claim::repository_key_of;

/// The word the store keeps for the central store, which no bucket address can be.
const STORE_WORD: &str = "store";

/// A parsed `cloud` choice.
#[derive(Clone, Debug)]
pub(super) enum Target {
    Store,
    Bucket(BucketAddress, String),
}

impl Target {
    /// The choice a request carries, or the sentence that says what is wrong with its address.
    pub(super) fn of(cloud: &RunCloud) -> Result<Target, String> {
        match cloud {
            RunCloud::Store {} => Ok(Target::Store),
            RunCloud::Bucket(text) => BucketAddress::parse(text).map(|address| Target::Bucket(address, text.trim().to_string())),
        }
    }

    /// What Fleet's record keeps, to read back with [`Target::kept`].
    pub(super) fn text(&self) -> String {
        match self {
            Target::Store => STORE_WORD.to_string(),
            Target::Bucket(_, text) => text.clone(),
        }
    }

    /// A choice as Fleet's record keeps it.
    pub(super) fn kept(text: &str) -> Result<Target, String> {
        match text {
            STORE_WORD => Ok(Target::Store),
            address => Target::of(&RunCloud::Bucket(address.to_string())),
        }
    }

    pub(super) fn wire(&self) -> RunCloud {
        match self {
            Target::Store => RunCloud::Store {},
            Target::Bucket(_, text) => RunCloud::Bucket(text.clone()),
        }
    }
}

/// Why a cloud place could not be used, with the words for a person.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Fault {
    /// Nothing to sign with: no profile, no store address, no credentials.
    Unsigned(String),
    Unreachable(String),
    Refused(String),
}

impl Fault {
    pub(super) fn said(&self) -> &str {
        match self {
            Fault::Unsigned(why) | Fault::Unreachable(why) | Fault::Refused(why) => why,
        }
    }
}

impl From<StoreError> for Fault {
    fn from(error: StoreError) -> Fault {
        match error.is_unreachable() {
            true => Fault::Unreachable(error.said()),
            false => Fault::Refused(error.said()),
        }
    }
}

impl From<BucketError> for Fault {
    fn from(error: BucketError) -> Fault {
        match error {
            BucketError::Unsigned(why) => Fault::Unsigned(why),
            error if error.is_unreachable() => Fault::Unreachable(error.said()),
            error => Fault::Refused(error.said()),
        }
    }
}

fn kept_from(row: RunRow) -> KeptRun {
    KeptRun {
        workspace: row.workspace,
        kind: row.kind,
        command: row.command,
        port: row.port,
        succeeded: row.succeeded,
        last_ran: row.last_ran,
        source: row.source,
        kept_in: None,
        why: None,
    }
}

fn row_of(run: &KeptRun) -> RunRow {
    RunRow {
        workspace: run.workspace.clone(),
        kind: run.kind.clone(),
        command: run.command.clone(),
        port: run.port,
        succeeded: run.succeeded,
        last_ran: run.last_ran,
        source: run.source.clone(),
    }
}

const NO_STORE: &str = "Armada's store is not available for this repository: it needs a remote, a store address in Settings, \
                        and a profile this machine is signed in to";

impl<H, V, W> Fleet<H, V, W>
where
    H: adapter_traits::AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: adapter_traits::Vcs + adapter_traits::Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: adapter_traits::WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    /// What the cloud holds for the repository at `root`. Empty where it holds none.
    pub(super) async fn cloud_read(&self, root: &str, target: &Target) -> Result<Vec<KeptRun>, Fault> {
        match target {
            Target::Store => {
                let read = self
                    .at_the_store_switched(root, true, |opened| {
                        let Some(key) = repository_key_of(&opened.slug) else { return Ok(None) };
                        opened.store.read_runs(&key).map(|held| Some(held.unwrap_or_default().into_iter().map(kept_from).collect()))
                    })
                    .await?;
                read.flatten().ok_or_else(|| Fault::Unsigned(NO_STORE.to_string()))
            }
            Target::Bucket(address, _) => {
                let (reaching, address, object) = (Arc::clone(self.reaching()), address.clone(), places::object_for(places::FOLDER, root));
                let held = blocking(move || reaching.bucket(&address)?.get(&object)).await?;
                match held {
                    None => Ok(Vec::new()),
                    Some(bytes) => {
                        let named = format!("The object {}", places::object_for(places::FOLDER, root));
                        let text = String::from_utf8(bytes).map_err(|_| Fault::Refused(format!("{named} is not text")))?;
                        places::decode(&text, &named).map_err(Fault::Refused)
                    }
                }
            }
        }
    }

    async fn cloud_write(&self, root: &str, target: &Target, facts: Vec<KeptRun>) -> Result<(), Fault> {
        match target {
            Target::Store => {
                let rows: Vec<RunRow> = facts.iter().map(row_of).collect();
                let wrote = self
                    .at_the_store_switched(root, true, move |opened| {
                        let Some(key) = repository_key_of(&opened.slug) else { return Ok(None) };
                        opened.store.put_runs(&key, &rows).map(Some)
                    })
                    .await?;
                wrote.flatten().ok_or_else(|| Fault::Unsigned(NO_STORE.to_string()))
            }
            Target::Bucket(address, _) => {
                let text = places::encode(root, &facts).map_err(Fault::Refused)?;
                let (reaching, address, object) = (Arc::clone(self.reaching()), address.clone(), places::object_for(places::FOLDER, root));
                blocking(move || reaching.bucket(&address)?.put(&object, text.as_bytes())).await
            }
        }
    }

    /// Bring the cloud and Fleet's record level: read what the cloud holds, merge it in, and write
    /// the whole set back. Quiet success is `Ok`; the record keeps the facts either way.
    pub(super) async fn cloud_synced(&self, root: &str, target: &Target) -> Result<(), Fault> {
        let held = self.cloud_read(root, target).await?;
        let facts = {
            let mut store = self.store().lock().await;
            store.merge_run_facts(root, &held).map_err(|why| Fault::Refused(why.to_string()))?;
            store.run_facts(root).map_err(|why| Fault::Refused(why.to_string()))?
        };
        self.cloud_write(root, target, facts).await
    }
}

/// `work` on a blocking thread, where a blocking client may be made and dropped.
async fn blocking<T: Send + 'static>(work: impl FnOnce() -> Result<T, BucketError> + Send + 'static) -> Result<T, Fault> {
    match tokio::task::spawn_blocking(work).await {
        Ok(done) => done.map_err(Fault::from),
        Err(why) => Err(Fault::Refused(format!("the call to the bucket was abandoned: {why}"))),
    }
}
