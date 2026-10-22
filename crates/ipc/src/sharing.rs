//! Share: pieces a person keeps for a repository, made into one `<name>.armada.toml` package, read
//! by a colleague and added to their own Kit. `docs/concepts/kit.md`, *Share*.
//!
//! **Fleet makes the text and reads it.** Bridge writes the file and the clipboard, and takes the
//! text from a dropped file or a paste, so nothing here names a path.

use serde::{Deserialize, Serialize};

use crate::flight_controls::FlightControlKind;
use crate::committing::PieceToCommit;

/// The package version this Fleet writes and reads.
pub const PACKAGE_VERSION: u32 = 1;

/// One chosen piece, and the repository it is kept for.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PieceToShare {
    /// A root `list_repositories` names.
    pub repository: String,
    /// The selector `commit_to_repository` takes: tiles by id in each list, a workflow, a review tool, the person's `armada.yml`.
    pub piece: PieceToCommit,
}

/// `share_package`'s body. `POST /share/package`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SharePieces {
    pub pieces: Vec<PieceToShare>,
}

/// `share_package`'s answer: the text of the package, and a file name to offer for it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SharedPackage {
    /// Ends in `.armada.toml`, and holds no path.
    pub file_name: String,
    pub text: String,
}

/// What a piece is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PieceKind {
    Tile,
    Workflow,
    ReviewTool,
    /// The person's own `armada.yml` for a repository, from the place their Kit keeps it.
    Manifest,
    /// Requests' collections and environments of the person's own for a repository. A secret is
    /// its name alone.
    Requests,
}

/// The list a tile sits in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TileList {
    /// The tiles while a Job is open.
    Job,
    /// The tiles with no Job open.
    Deck,
}

/// `read_package`'s body, and the text `add_package` takes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackageText {
    pub text: String,
}

/// One piece a package holds, and what adding it would do.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackagePiece {
    pub kind: PieceKind,
    /// A tile's `id`, a workflow's `workflow_id`, a review tool's dashed id, a manifest's `armada.yml`.
    pub id: String,
    /// What the piece is called where it is shown.
    pub name: String,
    /// A tile's list. Absent on any other piece.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub list: Option<TileList>,
    /// A tile's own action, where it is a shell line. Absent on any other piece.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub action: Option<FlightControlKind>,
    /// Every shell command the piece would run: a manifest's Check and Command lines. A workflow and a review tool run none themselves.
    pub commands: Vec<String>,
    /// The remote of the repository it is kept for, `owner/name` lowercase. Absent where it has none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remote: Option<String>,
    /// The root of the one repository Fleet serves that the remote names. Absent where none or
    /// several do: Add would keep the piece unbound.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repository: Option<String>,
    /// Whether the person's Kit already holds a piece of this id for that repository.
    pub exists: bool,
}

/// `read_package`'s answer: every piece, in the order the package holds them.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackageRead {
    pub pieces: Vec<PackagePiece>,
}

/// `add_package`'s body. `POST /share/add`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AddPackage {
    pub text: String,
    /// Replace a piece of the same id the person's Kit holds. Refused without it. False where absent.
    #[serde(default)]
    pub replace: bool,
}

/// Where an added piece went.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Landed {
    Added,
    /// A piece of the same id was there and is now this one.
    Replaced,
    /// Its remote matches no repository Fleet serves, so it waits for `bind_unbound_piece`.
    Unbound,
}

/// One piece `add_package` or `bind_unbound_piece` dealt with.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PieceAdded {
    pub kind: PieceKind,
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub list: Option<TileList>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remote: Option<String>,
    /// The root it was written for. Absent where it is unbound.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repository: Option<String>,
    pub landed: Landed,
    /// What `bind_unbound_piece` takes. Present where `landed` is `unbound`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unbound_id: Option<String>,
}

/// `add_package`'s answer.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackageAdded {
    pub pieces: Vec<PieceAdded>,
}

/// A piece kept unbound.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnboundPiece {
    pub unbound_id: String,
    pub kind: PieceKind,
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub list: Option<TileList>,
    pub commands: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remote: Option<String>,
}

/// `list_unbound_pieces`' answer.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnboundPieces {
    pub pieces: Vec<UnboundPiece>,
}

/// `bind_unbound_piece`'s body. `POST /share/bind`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BindUnboundPiece {
    pub unbound_id: String,
    /// A root `list_repositories` names.
    pub repository: String,
    /// Replace a piece of the same id the person's Kit holds for it. Refused without it.
    #[serde(default)]
    pub replace: bool,
}

/// `save_kit_manifest`'s body. `POST /repositories/kit_manifest`. A person's own `armada.yml` for
/// one repository, whole, as a setup Session writes it for them.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SaveKitManifest {
    /// A root `list_repositories` names.
    pub repository: String,
    /// The file: a whole Manifest, or one naming only the Checks and Commands it changes.
    pub text: String,
}

/// `save_kit_manifest`'s answer.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct KitManifestSaved {
    pub repository: String,
    /// Where it was written: a path on this Mac, or the bucket's address.
    pub file: String,
}
