//! A repository's own flight controls, kept where its Armada configuration is: the place chosen
//! for its remembered runs, `crate::run_memory`. `docs/concepts/kit.md`, *Flight controls*.
//!
//! | place | kept in |
//! |---|---|
//! | `this_mac` | `flight-controls/<key>.toml` under Kit's folder |
//! | `synced` | the same under the folder the person named |
//! | `cloud`, a bucket | `<prefix>/flight-controls/<key>.toml` |
//! | `cloud`, Armada's store, and `repository` | not built yet |
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
    SaveRepositoryFlightControls, WireError,
};
use store::KeptPlace;

use super::{check, parse, read_at, REFUSED};
use crate::daemon::Fleet;
use crate::run_memory::{cloud, places};

/// The folder beside `runs/` that holds each repository's file.
pub(crate) const FOLDER: &str = "flight-controls";

/// A place that cannot take a save now. A 422.
const UNPLACED: &str = "fleet.flight_controls_unplaced";

/// Where a repository's file is.
enum Kept {
    File(PathBuf),
    Object(BucketAddress),
    /// The place cannot hold it now, or not yet: the sentence that says why.
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

fn parse_repository(text: &str) -> Result<FlightControls, String> {
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
    /// The repository's place as Fleet's record holds it, `this_mac` absent a choice, and where
    /// in it the file is.
    async fn repository_flight_controls_kept(&self, root: &str) -> Result<(RunPlace, Kept), Refusal> {
        let held = self.store().lock().await.run_memory_place(root).map_err(|why| self.run_fault(why))?;
        let kit = PathBuf::from(&self.host().kit_home);
        let Some(KeptPlace { place, synced_folder, cloud }) = held else {
            return Ok((RunPlace::ThisMac, Kept::File(places::file_for(&kit, FOLDER, root))));
        };
        let place = crate::run_memory::place_of(&place);
        let kept = match place {
            RunPlace::ThisMac => Kept::File(places::file_for(&kit, FOLDER, root)),
            RunPlace::Synced => match synced_folder {
                Some(folder) if Path::new(&folder).is_dir() => Kept::File(places::file_for(Path::new(&folder), FOLDER, root)),
                Some(folder) => Kept::Nowhere(format!("the synced folder {folder} is not there")),
                None => Kept::Nowhere(String::from("no synced folder was chosen")),
            },
            RunPlace::Cloud => match cloud.as_deref().map(cloud::Target::kept) {
                Some(Ok(cloud::Target::Bucket(address, _))) => Kept::Object(address),
                // Not built yet: the next change keeps them in Armada's store beside the runs.
                Some(Ok(cloud::Target::Store)) => {
                    Kept::Nowhere(String::from("Armada's store does not keep a repository's flight controls yet"))
                }
                Some(Err(why)) => Kept::Nowhere(why),
                None => Kept::Nowhere(String::from("no cloud address was chosen")),
            },
            // Not built yet: the next change commits them on a Job's branch.
            RunPlace::Repository => {
                Kept::Nowhere(String::from("a repository's flight controls are not kept in the repository yet"))
            }
        };
        Ok((place, kept))
    }

    /// `get_repository_flight_controls`: the file at the repository's place, as it is now.
    pub(crate) async fn repository_flight_controls_read(
        &self,
        repository: String,
    ) -> Result<RepositoryFlightControls, Refusal> {
        let root = self.run_repository(&repository)?;
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
        Ok(RepositoryFlightControls { repository: root, place, read })
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
        self.repository_flight_controls_read(root).await
    }
}
