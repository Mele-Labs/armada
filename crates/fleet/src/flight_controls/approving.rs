//! A shell tile a teammate committed asks before it runs on this Mac.
//! `docs/design/who-sees-each-piece-of-configuration.md`, *A tile a teammate committed asks before
//! it runs*; `docs/concepts/kit.md`, *Flight controls*.
//!
//! **An approval is this Mac's.** It is `~/.armada/approved-tiles/<key>.toml` under Kit's folder,
//! whatever place the repository's other pieces are kept in, and holds a hash of the exact command
//! text by tile id. **A changed command has another hash**, so an approved tile cannot later run
//! something else. Only a press that says it came from the repository asks: a person's own tile,
//! and one imported from a package, never carries `repository_tile`.

use std::collections::BTreeMap;
use std::path::PathBuf;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{
    ApproveRepositoryTile, FlightControlKind, FlightControlsRead, RepositoryFlightControls,
    RepositoryTile, TileAsk, WireError, WireValue,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::write_over;
use crate::daemon::Fleet;
use crate::run_memory::places;

/// The folder under Kit's that holds each repository's approvals.
const FOLDER: &str = "approved-tiles";

/// A press of a repository tile whose command is not approved. A 422.
const NOT_APPROVED: &str = "fleet.tile_not_approved";
/// An approval of a command that is no longer the tile's. A 422.
const CHANGED: &str = "fleet.tile_command_changed";
/// An approval of a tile the repository does not hold. A 422.
const NOT_THERE: &str = "fleet.tile_not_in_repository";
/// The approvals could not be written. A 500.
const UNWRITABLE: &str = "fleet.tile_approval_unwritable";

#[derive(Default, Serialize, Deserialize)]
struct Approved {
    /// Tile id to the hash of the command approved.
    #[serde(default)]
    tiles: BTreeMap<String, String>,
}

/// The hash an approval names: the command's exact text.
pub(crate) fn command_hash(command: &str) -> String {
    Sha256::digest(command.as_bytes()).iter().map(|byte| format!("{byte:02x}")).collect()
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
    fn approvals_file(&self, root: &str) -> PathBuf {
        places::file_for(std::path::Path::new(&self.host().kit_home), FOLDER, root)
    }

    /// What this Mac has approved for `root`. A file that will not read approves nothing: the tile
    /// asks again, which loses no more than the file held.
    async fn approvals_read(&self, root: &str) -> Approved {
        let Ok(text) = tokio::fs::read_to_string(self.approvals_file(root)).await else {
            return Approved::default();
        };
        toml::from_str(&text).unwrap_or_default()
    }

    /// Each shell tile `in_repository` holds, and whether its current command is approved.
    pub(super) async fn asking_of(&self, root: &str, in_repository: &FlightControlsRead) -> Vec<TileAsk> {
        let FlightControlsRead::Read { controls } = in_repository else { return Vec::new() };
        let approved = self.approvals_read(root).await;
        let mut asking: Vec<TileAsk> = Vec::new();
        for tile in controls.job.iter().chain(&controls.deck) {
            let (Some(FlightControlKind::Shell), Some(command)) = (tile.kind, tile.text.as_ref()) else { continue };
            let command_hash = command_hash(command);
            if asking.iter().any(|one| one.id == tile.id && one.command_hash == command_hash) {
                continue;
            }
            let approved = approved.tiles.get(&tile.id) == Some(&command_hash);
            asking.push(TileAsk { id: tile.id.clone(), command_hash, command: command.clone(), approved });
        }
        asking
    }

    /// Refuse the press of a repository tile unless this Mac approved this command for it.
    pub(crate) async fn repository_tile_pressed(&self, tile: &RepositoryTile, command: &str) -> Result<(), Refusal> {
        let root = self.run_repository(&tile.repository)?;
        let hash = command_hash(command);
        if self.approvals_read(&root).await.tiles.get(&tile.id) == Some(&hash) {
            return Ok(());
        }
        Err(Refusal::Unacceptable(
            WireError::raised(
                NOT_APPROVED,
                format!("The tile {} runs a command this Mac has not approved: {command}", tile.id),
                self.run_id(),
            )
            .with_field("repository", WireValue::Str(root))
            .with_field("tile_id", WireValue::Str(tile.id.clone()))
            .with_field("command", WireValue::Str(command.to_string()))
            .with_field("command_hash", WireValue::Str(hash)),
        ))
    }

    /// `approve_repository_tile`: keep the hash shown if it is still the tile's command.
    pub(crate) async fn repository_tile_approved(
        &self,
        approving: ApproveRepositoryTile,
    ) -> Result<RepositoryFlightControls, Refusal> {
        let root = self.run_repository(&approving.repository)?;
        let refused = |code: &str, why: String| Refusal::Unacceptable(WireError::raised(code, why, self.run_id()));
        let in_repository = self.in_repository_read(&root, approving.session_id.as_ref()).await;
        let asking = self.asking_of(&root, &in_repository).await;
        let Some(current) = asking.iter().filter(|one| one.id == approving.tile_id).find(|one| one.command_hash == approving.command_hash) else {
            return Err(match asking.iter().any(|one| one.id == approving.tile_id) {
                true => refused(CHANGED, format!("The command of the tile {} is not the one shown.", approving.tile_id)),
                false => refused(NOT_THERE, format!("The repository holds no shell tile {}.", approving.tile_id)),
            });
        };
        let mut approved = self.approvals_read(&root).await;
        approved.tiles.insert(approving.tile_id.clone(), current.command_hash.clone());
        let unwritable = |why: &dyn std::fmt::Display| {
            Refusal::Fault(WireError::raised(UNWRITABLE, format!("The approval could not be written: {why}"), self.run_id()))
        };
        let text = toml::to_string(&approved).map_err(|why| unwritable(&why))?;
        let target = self.approvals_file(&root);
        tokio::task::spawn_blocking(move || write_over(&target, &text))
            .await
            .map_err(|why| unwritable(&why))?
            .map_err(|why| unwritable(&why))?;
        self.repository_flight_controls_read(root, approving.session_id).await
    }
}
