//! A red main claimed in the central store, so a Fleet on another machine
//! knows it is held. `docs/capabilities/merge-line.md`, *Fixing a red across
//! machines*.
//!
//! **Every call is a blocking round trip**, so it is built, made and dropped on
//! one blocking thread ([`Fleet::at_the_store`]). Where the repository's switch
//! is off, the machine is signed out or no store address is set, the call is
//! never made: [`Fleet::at_the_store`] answers `Ok(None)` and the take goes on
//! locally as it did before the store.
//!
//! **The store sees two hashes.** The repository's `owner/name` and the commit
//! main went red at, each as a sha256 in hex, so a name never leaves the
//! machine.

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use adapter_traits::{
    AgentHarness, Claimed, Delivery, Holder, HolderKind, RedKey, RepositoryKey, Store, StoreError, Vcs, WorkProduct,
};
use config::settings as keys;
use core_model::JobId;
use sha2::{Digest, Sha256};
use store::MainCi;

use crate::daemon::Fleet;
use crate::main_fix::{stopped, MainNotFixable};
use crate::signing_in::this_machine;

/// How long a call to the store may take. A take waits on it, so it is short.
pub const STORE_TIMEOUT: Duration = Duration::from_secs(5);

/// What the store call needs of the machine: a seam so a test can answer for
/// both, as [`Enrolling`](crate::signing_in::Enrolling) is for sign-in.
pub trait Reaching: Send + Sync {
    /// The repository at `root` as `owner/name` on its forge, or `None` where it
    /// has no remote. Reads the checkout, so off the async workers.
    fn slug(&self, root: &str) -> Option<String>;

    /// The store as the profile in `directory` is enrolled with it, or `None`
    /// where that profile never enrolled.
    fn open(&self, url: &str, directory: &Path) -> Result<Option<Box<dyn Store + Send>>, String>;
}

/// The store over HTTP and the remote the checkout has.
pub struct OverTheWire {
    timeout: Duration,
}

impl OverTheWire {
    pub fn within(timeout: Duration) -> OverTheWire {
        OverTheWire { timeout }
    }
}

impl Reaching for OverTheWire {
    fn slug(&self, root: &str) -> Option<String> {
        adapters::remote_of(Path::new(root)).map(|address| address.slug())
    }

    fn open(&self, url: &str, directory: &Path) -> Result<Option<Box<dyn Store + Send>>, String> {
        let opened = adapters::HttpStore::open(url, directory, self.timeout)?;
        Ok(opened.map(|store| Box::new(store) as Box<dyn Store + Send>))
    }
}

fn sha256_hex(text: &str) -> String {
    Sha256::digest(text.as_bytes()).iter().map(|byte| format!("{byte:02x}")).collect()
}

/// The two keys a claim is under: both hashes, 64 hex characters.
pub(crate) fn keys_of(slug: &str, red_commit: &str) -> Option<(RepositoryKey, RedKey)> {
    Some((RepositoryKey::from_hash(&sha256_hex(slug))?, RedKey::from_hash(&sha256_hex(red_commit))?))
}

/// Who takes a red: a Job Fleet released, or a Session that said so.
#[derive(Clone, Debug)]
pub(crate) enum Taker {
    Job(JobId),
    Session(String),
}

impl Taker {
    fn parts(&self) -> (HolderKind, &str) {
        match self {
            Taker::Job(job) => (HolderKind::Job, job.as_str()),
            Taker::Session(id) => (HolderKind::Session, id),
        }
    }
}

/// What a call made at the store has to hand: the store, this red's keys and
/// this machine's name.
pub(crate) struct Reach {
    pub store: Box<dyn Store + Send>,
    pub repository: RepositoryKey,
    pub red: RedKey,
    pub machine: String,
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
    /// Run `work` against the store for `root`'s red at `red_commit`. `Ok(None)`
    /// is **no store applies** and nothing was asked; an `Err` is a store that
    /// would not answer.
    async fn at_the_store<T: Send + 'static>(
        &self,
        root: &str,
        red_commit: &str,
        work: impl FnOnce(&Reach) -> Result<T, StoreError> + Send + 'static,
    ) -> Result<Option<T>, StoreError> {
        let switched_on = self
            .repositories()
            .served()
            .into_iter()
            .find(|served| served.root() == root)
            .is_some_and(|served| served.manifest().pull_request_owners());
        let settings = self.machine_settings().now();
        let url: String = settings.get(keys::STORE_URL);
        let directory = self.signing_in().active_directory();
        let (true, false, Some(directory)) = (switched_on, url.trim().is_empty(), directory) else {
            return Ok(None);
        };
        let configured: String = settings.get(keys::MACHINE_NAME);
        let reaching = Arc::clone(self.reaching());
        let (root, red_commit) = (root.to_string(), red_commit.to_string());
        tokio::task::spawn_blocking(move || {
            let Some(slug) = reaching.slug(&root) else { return Ok(None) };
            let Some((repository, red)) = keys_of(&slug, &red_commit) else { return Ok(None) };
            let Some(store) = reaching.open(url.trim(), &directory).map_err(StoreError::Refused)? else {
                return Ok(None);
            };
            let reach = Reach { store, repository, red, machine: this_machine(&configured) };
            work(&reach).map(Some)
        })
        .await
        .unwrap_or_else(|why| Err(StoreError::Refused(format!("the call to the store was abandoned: {why}"))))
    }

    fn unix_seconds(&self) -> i64 {
        self.now().epoch_millis().map_or(0, |millis| millis / 1000)
    }

    /// Whether `holder` is this machine's own row for a take that is over: its
    /// Job was stopped or is unknown here, or its Session has ended.
    async fn is_ours_and_over(&self, holder: &Holder, machine: &str) -> bool {
        if holder.machine != machine {
            return false;
        }
        let store = self.store().lock().await;
        match holder.kind {
            HolderKind::Job => store.load_job(&JobId::carried(core_model::Ulid::carried(holder.id.clone()))).map_or(true, |job| stopped(job.status())),
            HolderKind::Session => store.session(&holder.id).ok().flatten().map_or(true, |found| found.ended_at.is_some()),
        }
    }

    fn red_held_by(&self, holder: Holder) -> MainNotFixable {
        let held_for = (self.unix_seconds() - holder.since).max(0);
        MainNotFixable::TakenElsewhere { holder, held_for }
    }

    /// Whether this red is free to take, before anything is made for it. **A
    /// read, not the claim**: it spares a Job proposed for a red that is held
    /// the work of being stopped again. The claim itself is [`Self::claim_red`].
    pub(crate) async fn red_is_free(&self, main: &MainCi, take_over: bool) -> Result<(), MainNotFixable> {
        if take_over {
            return Ok(());
        }
        let read = self
            .at_the_store(&main.repository, main.red_commit(), |reach| {
                Ok((reach.store.read_claim(&reach.repository, &reach.red)?, reach.machine.clone()))
            })
            .await;
        match read {
            Err(why) => Err(MainNotFixable::ClaimNotMade(why.said())),
            Ok(Some((Some(holder), machine))) if !self.is_ours_and_over(&holder, &machine).await => Err(self.red_held_by(holder)),
            Ok(_) => Ok(()),
        }
    }

    /// Claim the red for `by`. **Before the Job is released or the work sent
    /// back**, and refused when the store holds it for someone else, cannot be
    /// reached or says no. `Ok(false)` is no store applying: nothing was written.
    /// `take_over` replaces whoever holds it.
    pub(crate) async fn claim_red(&self, main: &MainCi, by: &Taker, take_over: bool) -> Result<bool, MainNotFixable> {
        let since = self.unix_seconds();
        let (kind, id) = by.parts();
        let id = id.to_string();
        let claimed = |replacing: bool| {
            let id = id.clone();
            self.at_the_store(&main.repository, main.red_commit(), move |reach| {
                let holder = Holder::new(kind, &id, &reach.machine, since)
                    .ok_or_else(|| StoreError::Refused("the holder's id could not be written to the store".to_string()))?;
                let won = match replacing {
                    true => reach.store.replace_claim(&reach.repository, &reach.red, &holder)?,
                    false => reach.store.claim(&reach.repository, &reach.red, &holder)?,
                };
                Ok((won, reach.machine.clone()))
            })
        };
        match claimed(take_over).await {
            Err(why) => Err(MainNotFixable::ClaimNotMade(why.said())),
            Ok(None) => Ok(false),
            Ok(Some((Claimed::Won, _))) => Ok(true),
            Ok(Some((Claimed::Held(holder), machine))) => {
                // This machine's own row for a take that ended is replaced, not refused.
                if !self.is_ours_and_over(&holder, &machine).await {
                    return Err(self.red_held_by(holder));
                }
                match claimed(true).await {
                    Ok(Some((Claimed::Won, _))) => Ok(true),
                    Ok(Some((Claimed::Held(holder), _))) => Err(self.red_held_by(holder)),
                    Ok(None) => Ok(false),
                    Err(why) => Err(MainNotFixable::ClaimNotMade(why.said())),
                }
            }
        }
    }

    /// Read the claim on `root`'s red for the hub, and give back this machine's
    /// own once its take is over. **Rides the reading that finds the red**
    /// (`notice_main`), so a remote claim is seen as late as the rotation
    /// reaches its repository. A store that cannot be read leaves no field.
    pub(crate) async fn noticed_elsewhere(&self, root: &str) {
        let main = self.store().lock().await.main_ci(root).ok().flatten();
        let Some(main) = main.filter(|main| main.red_at.is_some()) else {
            self.sweeping().lock().await.elsewhere.remove(root);
            return;
        };
        let read = self
            .at_the_store(root, main.red_commit(), |reach| {
                Ok((reach.store.read_claim(&reach.repository, &reach.red)?, reach.machine.clone()))
            })
            .await;
        let elsewhere = match read {
            Ok(Some((Some(holder), machine))) if holder.machine != machine => Some(holder),
            Ok(Some((Some(holder), machine))) => {
                if self.is_ours_and_over(&holder, &machine).await {
                    let _ = self
                        .at_the_store(root, main.red_commit(), |reach| reach.store.release_claim(&reach.repository, &reach.red))
                        .await;
                }
                None
            }
            _ => None,
        };
        let mut sweep = self.sweeping().lock().await;
        match elsewhere {
            Some(holder) => sweep.elsewhere.insert(root.to_string(), holder),
            None => sweep.elsewhere.remove(root),
        };
    }

    /// Main is green: delete every claim under the repository, its own or not,
    /// so one left by a machine that is asleep ends too.
    pub(crate) async fn cleared_elsewhere(&self, main: &MainCi) {
        let _ = self
            .at_the_store(&main.repository, main.red_commit(), |reach| reach.store.clear_repository(&reach.repository))
            .await;
        self.sweeping().lock().await.elsewhere.remove(&main.repository);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_name_and_a_commit_are_each_a_sha256_in_hex() {
        let (repository, red) = keys_of("mele-labs/armada", "0123abc").expect("keys");
        assert_eq!(repository.as_str(), "b0e2b94c91eef1f1bb6564cbd69c2a16774dca9ac90ffbadc692c96041369b7b");
        assert_eq!(red.as_str().len(), 64);
        assert_ne!(red.as_str(), repository.as_str());
        assert_eq!(sha256_hex("abc"), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
    }
}
