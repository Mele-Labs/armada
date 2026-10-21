//! A repository's own flight controls, kept where its Armada configuration is: the place chosen
//! for its remembered runs, `crate::run_memory`. `docs/concepts/kit.md`, *Flight controls*.
//!
//! | place | kept in |
//! |---|---|
//! | `this_mac` | `flight-controls/<key>.toml` under Kit's folder |
//! | `synced` | the same under the folder the person named |
//! | `cloud`, a bucket | `<prefix>/flight-controls/<key>.toml` |
//! | `cloud`, Armada's store, and `repository` | the same as `this_mac`, and the answer says `this_mac` |
//!
//! A repository's tiles are the person's own and are kept for them alone, so a place that would
//! share them does not. The repository's own `.armada/flight-controls.toml` is a copy a person
//! committed on a Job's branch (`crate::committing`), read beside theirs and never written here.
//!
//! `<key>` is the runs' own (`places::key_of`). **Read on each call, never cached**, and held to
//! the machine file's rules and one more: a repository holds a person's own actions only, so
//! every tile carries a `kind`.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use adapters::BucketAddress;
use api::Refusal;
use ipc::{
    FlightControl, FlightControls, FlightControlsRead, RepositoryFlightControls, RunPlace,
    SaveRepositoryFlightControls, SessionId, WireError,
};

use super::{check, parse, read_at, REFUSED};
use crate::daemon::Fleet;
use crate::run_memory::{cloud, places};

/// The folder beside `runs/` that holds each repository's file.
pub(crate) const FOLDER: &str = "flight-controls";

/// The repository's own file, committed, beside `.armada/workflows/`.
pub(crate) const IN_REPOSITORY: &str = ".armada/flight-controls.toml";

/// A place that cannot take a save now. A 422.
const UNPLACED: &str = "fleet.flight_controls_unplaced";

/// Where a repository's file is.
enum Kept {
    File(PathBuf),
    Object(BucketAddress),
    /// The place cannot hold it now: the sentence that says why.
    Nowhere(String),
}

/// The machine file's rules, and every tile an own action.
pub(crate) fn check_repository(controls: &FlightControls) -> Result<(), String> {
    check(controls)?;
    own_actions("job", &controls.job)?;
    own_actions("deck", &controls.deck)
}

fn own_actions(list: &str, tiles: &[FlightControl]) -> Result<(), String> {
    match tiles.iter().position(|tile| tile.kind.is_none()) {
        Some(at) => Err(format!("{list}[{at}].kind is missing: a repository's tiles are actions of the person's own")),
        None => Ok(()),
    }
}

pub(crate) fn parse_repository(text: &str) -> Result<FlightControls, String> {
    let controls = parse(text)?;
    own_actions("job", &controls.job)?;
    own_actions("deck", &controls.deck)?;
    Ok(controls)
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
    /// Where the file is, and the place that holds it: the one Fleet's record has, `this_mac` absent
    /// a choice or where the choice would share it.
    async fn repository_flight_controls_kept(&self, root: &str) -> Result<(RunPlace, Kept), Refusal> {
        let held = self.kept_place(root).await?;
        let (place, at) = places::where_kept(Path::new(&self.host().kit_home), held.as_ref());
        let kept = match at {
            places::Where::Folder(base) => Kept::File(places::file_for(&base, FOLDER, root)),
            places::Where::Bucket(address, _) => Kept::Object(address),
            places::Where::Nowhere(why) => Kept::Nowhere(why),
        };
        Ok((place, kept))
    }

    /// The repository's own committed file, from the tree of the Job `job` names or the checkout.
    pub(super) async fn in_repository_read(&self, root: &str, job: Option<&SessionId>) -> FlightControlsRead {
        let tree = match self.job_tree_in(root, job).await {
            Some(tree) => PathBuf::from(tree.path),
            None => PathBuf::from(root),
        };
        read_at(&tree.join(IN_REPOSITORY), parse_repository).await
    }

    /// `get_repository_flight_controls`: the file at the repository's place, as it is now, and the
    /// repository's own in the tree of the Job `job` names, or the checkout with none.
    pub(crate) async fn repository_flight_controls_read(
        &self,
        repository: String,
        job: Option<SessionId>,
    ) -> Result<RepositoryFlightControls, Refusal> {
        let root = self.run_repository(&repository)?;
        let in_repository = self.in_repository_read(&root, job.as_ref()).await;
        let (place, kept) = self.repository_flight_controls_kept(&root).await?;
        let read = match kept {
            Kept::File(path) => read_at(&path, parse_repository).await,
            Kept::Object(address) => {
                let object = places::object_for(FOLDER, &root);
                let (reaching, at) = (Arc::clone(self.reaching()), object.clone());
                match cloud::blocking(move || reaching.bucket(&address)?.get(&at)).await {
                    Ok(None) => FlightControlsRead::Read { controls: FlightControls::default() },
                    Ok(Some(bytes)) => match String::from_utf8(bytes).map_err(|_| String::from("is not text")).and_then(|text| parse_repository(&text)) {
                        Ok(controls) => FlightControlsRead::Read { controls },
                        Err(why) => FlightControlsRead::NotRead { why: format!("The object {object}: {why}") },
                    },
                    Err(fault) => FlightControlsRead::NotRead { why: fault.said().to_string() },
                }
            }
            Kept::Nowhere(why) => FlightControlsRead::NotRead { why },
        };
        let asking = self.asking_of(&root, &in_repository).await;
        Ok(RepositoryFlightControls { repository: root, place, read, in_repository, asking })
    }

    /// `save_repository_flight_controls`: check, write whole at the place, and answer with it read
    /// again.
    pub(crate) async fn repository_flight_controls_saved(
        &self,
        saving: SaveRepositoryFlightControls,
    ) -> Result<RepositoryFlightControls, Refusal> {
        let root = self.run_repository(&saving.repository)?;
        let refused = |code: &str, why: String| Refusal::Unacceptable(WireError::raised(code, why, self.run_id()));
        check_repository(&saving.controls).map_err(|why| refused(REFUSED, why))?;
        match self.repository_flight_controls_kept(&root).await?.1 {
            Kept::File(path) => self.flight_controls_written(&path, &saving.controls).await?,
            Kept::Object(address) => {
                let object = places::object_for(FOLDER, &root);
                let text = self.flight_controls_text(&object, &saving.controls)?;
                let reaching = Arc::clone(self.reaching());
                cloud::blocking(move || reaching.bucket(&address)?.put(&object, text.as_bytes()))
                    .await
                    .map_err(|fault| refused(fault.code(), fault.said().to_string()))?;
            }
            Kept::Nowhere(why) => return Err(refused(UNPLACED, why)),
        }
        self.repository_flight_controls_read(root, None).await
    }
}
