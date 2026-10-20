//! The text of a package: a `version` key, then one `[[piece]]` table per piece.
//!
//! **Nothing about the person or the machine has a field to be put in**: a piece is its kind, the
//! piece as stored and the remote of the repository it is kept for, so a path, a user name or a
//! time cannot reach the text by being carried along.

use std::path::PathBuf;

use config::review_tools::{self as tools, Written};
use ipc::{FlightControl, FlightControlKind, FlightControls, PieceKind, TileList, PACKAGE_VERSION};
use serde::{Deserialize, Serialize};

use crate::flight_controls::repository::check_repository;

/// The whole text.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Package {
    pub(super) version: u32,
    #[serde(default, rename = "piece")]
    pub(super) pieces: Vec<Piece>,
}

/// One piece, as stored.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum Piece {
    Tile {
        list: TileList,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        remote: Option<String>,
        /// Under its own key, since a tile has a `kind` of its own.
        tile: FlightControl,
    },
    /// A definition, as its file holds it.
    Workflow {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        remote: Option<String>,
        text: String,
    },
    /// A review tool's file, `<id>.md`.
    ReviewTool {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        remote: Option<String>,
        id: String,
        text: String,
    },
}

/// What a piece is, said without writing anything.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Shown {
    pub(super) kind: PieceKind,
    pub(super) id: String,
    pub(super) name: String,
    pub(super) list: Option<TileList>,
    pub(super) action: Option<FlightControlKind>,
    pub(super) commands: Vec<String>,
    pub(super) remote: Option<String>,
}

impl Piece {
    pub(super) fn remote(&self) -> Option<&str> {
        match self {
            Piece::Tile { remote, .. } | Piece::Workflow { remote, .. } | Piece::ReviewTool { remote, .. } => remote.as_deref(),
        }
    }

    /// The piece's id, name and commands. A workflow and a review tool were checked when the
    /// package was read, so this cannot fail for a piece that came out of [`read`].
    pub(super) fn shown(&self) -> Shown {
        match self {
            Piece::Tile { list, remote, tile } => Shown {
                kind: PieceKind::Tile,
                id: tile.id.clone(),
                name: tile.name.clone().unwrap_or_else(|| tile.id.clone()),
                list: Some(*list),
                action: tile.kind,
                commands: match (tile.kind, &tile.text) {
                    (Some(FlightControlKind::Shell), Some(text)) => vec![text.clone()],
                    _ => Vec::new(),
                },
                remote: remote.clone(),
            },
            Piece::Workflow { remote, text } => {
                let head = config::head_of(text).unwrap_or(config::WorkflowHead { id: String::new(), name: String::new() });
                Shown { kind: PieceKind::Workflow, id: head.id, name: head.name, list: None, action: None, commands: Vec::new(), remote: remote.clone() }
            }
            Piece::ReviewTool { remote, id, text } => {
                let name = review_tool_of(text).map_or_else(|_| id.clone(), |(_, name)| name);
                Shown { kind: PieceKind::ReviewTool, id: id.clone(), name, list: None, action: None, commands: Vec::new(), remote: remote.clone() }
            }
        }
    }

    /// The same piece twice is one the package says twice.
    fn key(&self) -> (PieceKind, Option<TileList>, String, Option<String>) {
        let shown = self.shown();
        (shown.kind, shown.list, shown.id, shown.remote.map(|remote| remote.to_lowercase()))
    }
}

/// A review tool file's id and name, by the rules a file in Kit is held to.
pub(super) fn review_tool_of(text: &str) -> Result<(String, String), String> {
    let resolved = tools::resolve([Written::in_kit(PathBuf::from("package.md"), text.to_string())]);
    match (resolved.defs.first(), resolved.left_out.first()) {
        (Some(def), _) => Ok((def.id.clone(), def.name.clone())),
        (None, Some(out)) => Err(out.said.split_once("because ").map_or(out.said.clone(), |(_, why)| why.to_string())),
        (None, None) => Err(String::from("holds no review tool")),
    }
}

/// The text of a package holding `pieces`.
pub(super) fn written(pieces: Vec<Piece>) -> Result<String, String> {
    toml::to_string(&Package { version: PACKAGE_VERSION, pieces }).map_err(|why| why.to_string())
}

/// A remote is `owner/name`, with no spaces.
fn remote_problem(remote: &str) -> Option<String> {
    let parts: Vec<&str> = remote.split('/').collect();
    match parts.as_slice() {
        [owner, name] if !owner.is_empty() && !name.is_empty() && !remote.contains(char::is_whitespace) => None,
        _ => Some(format!("the remote `{remote}` is not owner/name")),
    }
}

/// The pieces a package's text holds, each checked by the rules the same piece is held to in Kit.
/// The refusal names the piece by its place in the package, counting from 1.
pub(super) fn read(text: &str) -> Result<Vec<Piece>, String> {
    let package: Package = toml::from_str(text).map_err(|why| format!("The package is not one: {}", why.to_string().trim_end()))?;
    if package.version != PACKAGE_VERSION {
        return Err(format!("The package is version {}. This Armada reads version {PACKAGE_VERSION}.", package.version));
    }
    if package.pieces.is_empty() {
        return Err(String::from("The package holds no pieces."));
    }
    for (at, piece) in package.pieces.iter().enumerate() {
        let n = at + 1;
        if let Some(why) = piece.remote().and_then(remote_problem) {
            return Err(format!("Piece {n}: {why}."));
        }
        let problem = match piece {
            Piece::Tile { list, tile, .. } => {
                let mut controls = FlightControls::default();
                match list {
                    TileList::Job => controls.job.push(tile.clone()),
                    TileList::Deck => controls.deck.push(tile.clone()),
                }
                check_repository(&controls).err()
            }
            Piece::Workflow { text, .. } => config::head_of(text).err().map(|why| format!("the workflow {why}")),
            Piece::ReviewTool { id, text, .. } => match review_tool_of(text) {
                Ok((held, _)) if &held == id => None,
                Ok((held, _)) => Some(format!("the review tool `{id}` is `{held}` in its file")),
                Err(why) => Some(format!("the review tool {why}")),
            },
        };
        if let Some(why) = problem {
            return Err(format!("Piece {n}: {why}."));
        }
        if let Some(first) = package.pieces[..at].iter().position(|before| before.key() == piece.key()) {
            return Err(format!("Pieces {} and {n} are the same piece `{}`.", first + 1, piece.shown().id));
        }
    }
    Ok(package.pieces)
}

/// A name for the file: what the pieces are for, as a slug.
pub(super) fn file_name(pieces: &[Piece]) -> String {
    let shown: Vec<Shown> = pieces.iter().map(Piece::shown).collect();
    let remotes: Vec<&str> = shown.iter().filter_map(|one| one.remote.as_deref()).collect();
    let name = match shown.as_slice() {
        [only] => only.id.clone(),
        _ => match remotes.split_first() {
            Some((first, rest)) if rest.len() + 1 == shown.len() && rest.iter().all(|one| one == first) => {
                first.rsplit('/').next().unwrap_or("pieces").to_string()
            }
            _ => String::from("pieces"),
        },
    };
    let mut slug = String::new();
    for c in name.to_lowercase().chars() {
        match c.is_ascii_alphanumeric() {
            true => slug.push(c),
            false if !slug.ends_with('-') && !slug.is_empty() => slug.push('-'),
            false => {}
        }
    }
    let slug = slug.trim_end_matches('-');
    format!("{}.armada.toml", if slug.is_empty() { "pieces" } else { slug })
}
