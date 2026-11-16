//! A Kit piece's file, read, saved and opened elsewhere: `get_kit_file`, `save_kit_file` and
//! `open_kit_piece`. Which file a piece is comes from an inventory read for the call, so **nothing
//! in a request reaches a path the inventory does not name**, as `crate::kit_pieces`' Remove.
//!
//! A save is `crate::editing::save`: the guard against a file that moved and the rename-over write
//! are the Manifest's, not a second copy.

use std::fs;
use std::path::{Path, PathBuf};

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use config::settings as keys;
use ipc::{
    GetKitFile, Instant, KitFile, KitInventory, KitOpenKind, KitPieceOpened, KitPlace, OpenKitPiece, SaveKitFile,
    WireError, WireValue,
};

use crate::daemon::Fleet;
use crate::editing::NotSaved;
use crate::kit_open;
use crate::kit_pieces::items_of;

const NOT_LISTED: &str = "fleet.kit_file_not_listed";
const AMBIGUOUS: &str = "fleet.kit_file_ambiguous";
const NO_FILE: &str = "fleet.kit_file_none";
const FROM_PLUGIN: &str = "fleet.kit_file_from_plugin";
const MOVED: &str = "fleet.kit_file_moved";
const UNREADABLE: &str = "fleet.kit_file_unreadable";
const UNWRITABLE: &str = "fleet.kit_file_unwritable";
const PLUGIN_IN_EDITOR: &str = "fleet.kit_open_plugin_in_editor";
const NO_EDITOR: &str = "fleet.kit_open_no_editor";
const OPEN_FAILED: &str = "fleet.kit_open_failed";

/// What a skill's folder holds that the harness reads it from.
const SKILL_FILE: &str = "SKILL.md";

/// What the caller was doing, for the refusal's words.
#[derive(Clone, Copy)]
pub(crate) enum Doing {
    Read,
    Save,
    Open,
}

impl Doing {
    fn done(self) -> &'static str {
        match self {
            Doing::Read => "read",
            Doing::Save => "saved",
            Doing::Open => "opened",
        }
    }
}

/// The inventory's word for a kind, and what one is called in a sentence, singular then plural.
fn words_of(kind: KitOpenKind) -> (&'static str, &'static str, &'static str) {
    match kind {
        KitOpenKind::Skills => ("skills", "skill", "skills"),
        KitOpenKind::SubAgents => ("sub_agents", "sub agent", "sub agents"),
        KitOpenKind::Commands => ("commands", "command", "commands"),
        KitOpenKind::AgentFile => ("agent_file", "agent file", "agent files"),
        KitOpenKind::OutputStyles => ("output_styles", "output style", "output styles"),
        KitOpenKind::Plugins => ("plugins", "plugin", "plugins"),
    }
}

/// The one row an inventory names for a piece.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Piece {
    /// What the row's `source` names: a skill's folder, a plugin's install path, otherwise the file.
    pub(crate) place: PathBuf,
    /// What an editor opens and what is read and saved: a skill's `SKILL.md`, otherwise `place`.
    pub(crate) file: PathBuf,
    /// The plugin that brought it, which a save is refused for.
    pub(crate) plugin: Option<String>,
}

/// Why an inventory does not name one piece with a file.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Unnamed {
    NotListed,
    Several(usize),
    /// A built in output style and Armada's own rules: a row with nowhere on disk.
    NoFile,
}

/// A row's `source` as a path: absolute as written, or under the inventory's home. `None` where
/// the row has nowhere on disk, as `kitSourcePath` says in Bridge.
fn source_path(source: &str, home: &str) -> Option<PathBuf> {
    if source.is_empty() || source == "Armada" || source.starts_with("built in") {
        return None;
    }
    Some(match source.starts_with('/') {
        true => PathBuf::from(source),
        false => Path::new(home.trim_end_matches('/')).join(source),
    })
}

/// The piece `name` of `kind`, from an inventory read for the call.
pub(crate) fn piece_named(inventory: &KitInventory, kind: KitOpenKind, name: &str) -> Result<Piece, Unnamed> {
    let named: Vec<&ipc::SetupItem> =
        items_of(inventory, words_of(kind).0).iter().filter(|item| item.name == name).collect();
    let item = match named.as_slice() {
        [one] => *one,
        [] => return Err(Unnamed::NotListed),
        several => return Err(Unnamed::Several(several.len())),
    };
    let place = source_path(&item.source, &inventory.home).ok_or(Unnamed::NoFile)?;
    let plugin = item.plugin.clone().or_else(|| item.style.as_ref().and_then(|style| style.plugin.clone()));
    let file = match kind {
        KitOpenKind::Skills => place.join(SKILL_FILE),
        _ => place.clone(),
    };
    Ok(Piece { place, file, plugin })
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
    /// The piece an inventory, read again for this call, names once.
    async fn kit_piece_found(&self, kind: KitOpenKind, name: &str, doing: Doing) -> Result<Piece, Refusal> {
        let inventory = self.kit_inventory().await;
        piece_named(&inventory, kind, name).map_err(|why| {
            let (_, one, many) = words_of(kind);
            let done = doing.done();
            match why {
                Unnamed::NotListed => {
                    self.kit_file_refusal(NOT_LISTED, format!("Kit lists no {one} named {name}, so nothing was {done}."))
                }
                Unnamed::Several(count) => self.kit_file_refusal(
                    AMBIGUOUS,
                    format!("{count} {many} are named {name}, so nothing was {done}."),
                ),
                Unnamed::NoFile => {
                    self.kit_file_refusal(NO_FILE, format!("The {one} {name} has no file, so nothing was {done}."))
                }
            }
        })
    }

    /// `get_kit_file`: the file whole, as it is on disk.
    pub(crate) async fn kit_file_read(&self, asked: GetKitFile) -> Result<KitFile, Refusal> {
        let piece = self.kit_piece_found(asked.kind.into(), &asked.name, Doing::Read).await?;
        let file = piece.file.clone();
        let text = tokio::task::spawn_blocking(move || fs::read_to_string(&file).map_err(|cause| (file, cause)))
            .await
            .map_err(|why| self.kit_file_fault(UNREADABLE, why.to_string()))?
            .map_err(|(file, cause)| self.unreadable(&file, &cause))?;
        Ok(KitFile {
            path: piece.file.display().to_string(),
            text,
            at: Instant::from(&self.now()),
            plugin: piece.plugin,
        })
    }

    /// `save_kit_file`: the bytes sent, on disk, **only where the file is still what was read** and
    /// the piece is the person's own. A plugin's update overwrites its pieces, so an edit there
    /// would be lost without a word.
    pub(crate) async fn kit_file_saved(&self, asked: SaveKitFile) -> Result<KitFile, Refusal> {
        let piece = self.kit_piece_found(asked.kind.into(), &asked.name, Doing::Save).await?;
        if let Some(plugin) = &piece.plugin {
            return Err(self.kit_file_refusal(
                FROM_PLUGIN,
                format!(
                    "{} comes from the plugin {plugin}, and the plugin's next update overwrites it, so nothing was saved.",
                    asked.name
                ),
            ));
        }
        // A piece kept as a link is written through, so it stays one.
        let target = fs::canonicalize(&piece.file).unwrap_or_else(|_| piece.file.clone());
        let (read, text) = (asked.read, asked.text.clone());
        let written = tokio::task::spawn_blocking(move || crate::editing::save(&target, &read, &text))
            .await
            .map_err(|why| self.kit_file_fault(UNWRITABLE, why.to_string()))?;
        let path = piece.file.display().to_string();
        match written {
            Ok(()) => {}
            Err(NotSaved::Unreadable(cause)) => return Err(self.unreadable(&piece.file, &cause)),
            Err(NotSaved::Unwritable(cause)) => {
                return Err(self.kit_file_fault(
                    UNWRITABLE,
                    format!("{path} could not be written: {cause}. What is on disk is what was there before."),
                ))
            }
            Err(NotSaved::Moved(on_disk)) => {
                let said = match on_disk {
                    Some(_) => format!(
                        "{path} changed after it was read, so nothing was saved. What is on disk now is on `on_disk`."
                    ),
                    None => format!("{path} is gone since it was read, so nothing was saved."),
                };
                let moved = WireError::raised(MOVED, said, self.run_id());
                return Err(Refusal::IllegalMove(match on_disk {
                    Some(text) => moved.with_field("on_disk", WireValue::Str(text)),
                    None => moved,
                }));
            }
        }
        Ok(KitFile { path, text: asked.text, at: Instant::from(&self.now()), plugin: None })
    }

    /// `open_kit_piece`: the piece in the person's editor, or shown in Finder.
    pub(crate) async fn kit_piece_opened(&self, asked: OpenKitPiece) -> Result<KitPieceOpened, Refusal> {
        let piece = self.kit_piece_found(asked.kind, &asked.name, Doing::Open).await?;
        let launch = match asked.place {
            KitPlace::Finder => kit_open::in_finder(&piece.place.display().to_string()),
            KitPlace::Editor if asked.kind == KitOpenKind::Plugins => {
                return Err(self.kit_file_refusal(
                    PLUGIN_IN_EDITOR,
                    format!("{} is a plugin and opens in Finder only, so nothing was opened.", asked.name),
                ))
            }
            KitPlace::Editor => {
                let configured: String = self.machine_settings().now().get(keys::EDITOR_COMMAND);
                // Fleet's own PATH after the one it hands an agent, which names where tools are kept.
                let path_var = format!("{}:{}", self.host().path, std::env::var("PATH").unwrap_or_default());
                kit_open::in_editor(
                    &configured,
                    |name| std::env::var(name).ok(),
                    &path_var,
                    &piece.file.display().to_string(),
                )
                .map_err(|why| self.kit_file_refusal(NO_EDITOR, why))?
            }
        };
        kit_open::start(&launch).map_err(|why| self.kit_file_fault(OPEN_FAILED, why))?;
        let opened = match asked.place {
            KitPlace::Finder => &piece.place,
            KitPlace::Editor => &piece.file,
        };
        Ok(KitPieceOpened { path: opened.display().to_string() })
    }

    fn unreadable(&self, file: &Path, cause: &std::io::Error) -> Refusal {
        self.kit_file_fault(UNREADABLE, format!("{} could not be read: {cause}.", file.display()))
    }

    fn kit_file_refusal(&self, code: &str, why: String) -> Refusal {
        Refusal::Unacceptable(WireError::raised(code, why, self.run_id()))
    }

    fn kit_file_fault(&self, code: &str, why: String) -> Refusal {
        Refusal::Fault(WireError::raised(code, why, self.run_id()))
    }
}
