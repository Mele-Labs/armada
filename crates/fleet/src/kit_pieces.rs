//! Kit's Add and Remove for a plugin or a skill: `look_at_kit_source`, `install_kit_piece`,
//! `remove_kit_piece` and `restore_kit_piece`. What a source is and how each command is spelled
//! are `adapters::kit_pieces`'; this decides what a person's press may reach.
//!
//! **Nothing in a body reaches the disk or a command line.** Install looks again and runs what it
//! found; Remove finds the skill in an inventory read for the call; Restore puts back only a place
//! this Fleet trashed, kept in `~/.armada/kit-trash`.

use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use adapters::kit_pieces::{self, Asked, Cli, Found, ForgeReader, Installed, Piece, Trash};
use api::Refusal;
use ipc::{
    InstallKitPiece, KitInventory, KitPieceInstalled, KitPieceKind, KitPieceRemoved, KitSourceChoice, KitSourceFound,
    LookAtKitSource, RemoveKitPiece, RestoreKitPiece, WhatWasRead, WireError,
};

use crate::daemon::Fleet;

/// Where Fleet keeps each skill it trashed and where it came from, inside Kit's home.
pub const TRASHED: &str = "kit-trash";

/// Not job-scoped, so it earns its own code, `crate::kit`'s reason.
const KIT_PIECE_REFUSED: &str = "fleet.kit_piece_refused";

/// A look reads a page and a repository's file list: a slow network, not a slow machine.
const LOOK_WITHIN: Duration = Duration::from_secs(30);
/// An install fetches a repository and may build; the mod's own install allows as long.
const INSTALL_WITHIN: Duration = Duration::from_secs(300);

/// One writer of the trashed list at a time.
static TRASHING: Mutex<()> = Mutex::new(());

/// The wire's answer for what a look found.
fn found_on_the_wire(found: Found) -> KitSourceFound {
    let shown = |piece: &Piece| piece.commands().iter().map(|line| line.shown()).collect();
    match found {
        Found::Piece(piece) => match &piece {
            Piece::Plugin { name, marketplace, repository, skills, .. } => KitSourceFound::Plugin {
                name: name.clone(),
                marketplace: marketplace.clone(),
                repository: repository.clone(),
                skills: skills.clone(),
                commands: shown(&piece),
            },
            Piece::Skill { name, repository, .. } => KitSourceFound::Skill {
                name: name.clone(),
                repository: repository.clone(),
                commands: shown(&piece),
            },
        },
        Found::Several { repository, choices } => KitSourceFound::Several {
            repository,
            kind: choices.first().map_or(KitPieceKind::Skill, |one| kind_of(&one.piece)),
            choices: choices
                .iter()
                .map(|one| KitSourceChoice {
                    name: one.piece.name().to_string(),
                    says: one.says.clone(),
                    commands: shown(&one.piece),
                })
                .collect(),
        },
        Found::Unknown { why } => KitSourceFound::Unknown { why },
    }
}

fn kind_of(piece: &Piece) -> KitPieceKind {
    match piece {
        Piece::Plugin { .. } => KitPieceKind::Plugin,
        Piece::Skill { .. } => KitPieceKind::Skill,
    }
}

/// The piece an install runs, out of a fresh look and what the person pressed.
/// **A choice is a name matched against what Fleet found**, so a body never reaches a command line.
pub(crate) fn chosen(found: Found, asked: &InstallKitPiece) -> Result<Piece, String> {
    let kind = match asked.kind {
        KitPieceKind::Plugin => "plugin",
        KitPieceKind::Skill => "skill",
    };
    let shown_differs = || {
        format!("Nothing was installed. {} is no longer a {kind}, so what was shown is not what would run.", asked.source)
    };
    let gone = |name: &str| {
        format!(
            "Nothing was installed. {} no longer holds a {kind} named {name}, so what was shown is not what would run.",
            asked.source
        )
    };
    let piece = match found {
        Found::Unknown { why } => return Err(format!("Nothing was installed. {why}")),
        Found::Piece(piece) => match &asked.choice {
            Some(name) if name != piece.name() => return Err(gone(name)),
            _ => piece,
        },
        Found::Several { choices, .. } => {
            let Some(name) = &asked.choice else {
                return Err(format!(
                    "Nothing was installed. {} holds several, and which one to install is not said.",
                    asked.source
                ));
            };
            match choices.into_iter().find(|one| one.piece.name() == name) {
                Some(one) => one.piece,
                None => return Err(gone(name)),
            }
        }
    };
    match kind_of(&piece) == asked.kind {
        true => Ok(piece),
        false => Err(shown_differs()),
    }
}

/// Each `trashed\tfrom` line Fleet wrote.
pub(crate) fn trashed_lines(text: &str) -> Vec<(String, String)> {
    text.lines()
        .filter_map(|line| line.split_once('\t'))
        .map(|(trashed, from)| (trashed.to_string(), from.to_string()))
        .collect()
}

/// The items of one kind an inventory read.
fn items_of<'a>(inventory: &'a KitInventory, kind: &str) -> &'a [ipc::SetupItem] {
    match inventory.kinds.iter().find(|row| row.kind == kind).map(|row| &row.read) {
        Some(WhatWasRead::Read { items, .. }) => items,
        _ => &[],
    }
}

/// The folder of the one skill of the person's own named `name`: under their skills folder and
/// brought by no plugin.
pub(crate) fn own_skill(inventory: &KitInventory, home: &Path, name: &str) -> Result<PathBuf, String> {
    let skills = kit_pieces::skills_folder(home);
    let mine: Vec<PathBuf> = items_of(inventory, "skills")
        .iter()
        .filter(|item| item.name == name && item.plugin.is_none())
        .map(|item| PathBuf::from(&item.source))
        .filter(|source| source.starts_with(&skills) && source != &skills)
        .collect();
    match mine.as_slice() {
        [one] => Ok(one.clone()),
        [] => Err(format!("None of your own skills is named {name}, so nothing was moved to the Trash.")),
        several => Err(format!(
            "{} of your skills are named {name}, so which one to move to the Trash is not said: {}.",
            several.len(),
            several.iter().map(|one| one.display().to_string()).collect::<Vec<_>>().join(", ")
        )),
    }
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
    /// `look_at_kit_source`: off this thread, because it waits on the network.
    pub(crate) async fn kit_source_looked_at(&self, asked: LookAtKitSource) -> Result<KitSourceFound, Refusal> {
        let found = tokio::task::spawn_blocking(move || {
            kit_pieces::look(&asked.source, &ForgeReader { within: LOOK_WITHIN })
        })
        .await
        .map_err(|why| self.kit_piece_fault(why.to_string()))?;
        Ok(found_on_the_wire(found))
    }

    /// `install_kit_piece`: look again, and run what that look found. **Bridge only**, so the
    /// route is the person's press and `Asked` is made here.
    pub(crate) async fn kit_piece_installed(&self, asked: InstallKitPiece) -> Result<KitPieceInstalled, Refusal> {
        let source = asked.source.clone();
        let found = tokio::task::spawn_blocking(move || kit_pieces::look(&source, &ForgeReader { within: LOOK_WITHIN }))
            .await
            .map_err(|why| self.kit_piece_fault(why.to_string()))?;
        let piece = chosen(found, &asked).map_err(|why| self.kit_piece_refusal(why))?;
        let cli = self.kit_cli();
        let ran = tokio::task::spawn_blocking(move || {
            let asked = Asked::when(true).expect("a request made here");
            kit_pieces::install(asked, &piece, &cli)
        })
        .await
        .map_err(|why| self.kit_piece_fault(why.to_string()))?;
        Ok(match ran {
            Installed::Done => KitPieceInstalled::Installed { setup: self.kit_inventory().await },
            Installed::Failed { command, exit_code, error, taken_back } => {
                KitPieceInstalled::Failed { command, exit_code, error, taken_back }
            }
        })
    }

    /// `remove_kit_piece`: a skill of the person's own to the Trash, or a plugin uninstalled.
    pub(crate) async fn kit_piece_removed(&self, asked: RemoveKitPiece) -> Result<KitPieceRemoved, Refusal> {
        let inventory = self.kit_inventory().await;
        let home = PathBuf::from(&self.host().home);
        match asked.kind {
            KitPieceKind::Skill => {
                let folder = own_skill(&inventory, &home, &asked.name).map_err(|why| self.kit_piece_refusal(why))?;
                let ledger = self.kit_trashed_path();
                let trashed = tokio::task::spawn_blocking(move || -> Result<PathBuf, String> {
                    let _one_at_a_time = TRASHING.lock().unwrap_or_else(|held| held.into_inner());
                    let trashed = Trash::of(&home).put(&home, &folder)?;
                    let line = format!("{}\t{}\n", trashed.display(), folder.display());
                    let kept = std::fs::create_dir_all(ledger.parent().unwrap_or(Path::new("/")))
                        .and_then(|()| {
                            use std::io::Write;
                            std::fs::OpenOptions::new().create(true).append(true).open(&ledger)?.write_all(line.as_bytes())
                        });
                    if let Err(why) = kept {
                        // Undo cannot find a folder whose way back was not kept, so it is not left there.
                        let _ = Trash::of(&home).take_back(&home, &trashed, &folder);
                        return Err(format!("where {} came from could not be kept, so it was left where it is: {why}", folder.display()));
                    }
                    Ok(trashed)
                })
                .await
                .map_err(|why| self.kit_piece_fault(why.to_string()))?
                .map_err(|why| self.kit_piece_refusal(why))?;
                Ok(KitPieceRemoved {
                    setup: self.kit_inventory().await,
                    trashed: Some(trashed.display().to_string()),
                })
            }
            KitPieceKind::Plugin => {
                let held = items_of(&inventory, "plugins")
                    .iter()
                    .any(|item| item.name == asked.name && item.uninstall.is_some());
                if !held {
                    return Err(self.kit_piece_refusal(format!(
                        "No installed plugin is named {}, so nothing was uninstalled.",
                        asked.name
                    )));
                }
                let cli = self.kit_cli();
                let name = asked.name.clone();
                tokio::task::spawn_blocking(move || kit_pieces::uninstall(&name, &cli))
                    .await
                    .map_err(|why| self.kit_piece_fault(why.to_string()))?
                    .map_err(|failed| {
                        self.kit_piece_refusal(format!(
                            "`{}` failed with: {}",
                            kit_pieces::uninstall_line(&asked.name).shown(),
                            failed.said
                        ))
                    })?;
                Ok(KitPieceRemoved {
                    setup: self.kit_inventory().await,
                    trashed: None,
                })
            }
        }
    }

    /// `restore_kit_piece`: a place this Fleet trashed, back where it came from.
    pub(crate) async fn kit_piece_restored(&self, asked: RestoreKitPiece) -> Result<KitInventory, Refusal> {
        let home = PathBuf::from(&self.host().home);
        let ledger = self.kit_trashed_path();
        tokio::task::spawn_blocking(move || -> Result<(), String> {
            let _one_at_a_time = TRASHING.lock().unwrap_or_else(|held| held.into_inner());
            let text = std::fs::read_to_string(&ledger).unwrap_or_default();
            let lines = trashed_lines(&text);
            let Some((_, from)) = lines.iter().find(|(trashed, _)| trashed == &asked.trashed) else {
                return Err(format!("{} is not a skill Kit moved to the Trash, so nothing was put back.", asked.trashed));
            };
            Trash::of(&home).take_back(&home, Path::new(&asked.trashed), Path::new(from))?;
            let kept: String = lines
                .iter()
                .filter(|(trashed, _)| trashed != &asked.trashed)
                .map(|(trashed, from)| format!("{trashed}\t{from}\n"))
                .collect();
            std::fs::write(&ledger, kept).map_err(|why| format!("the skill is back, and {} would not write: {why}", ledger.display()))
        })
        .await
        .map_err(|why| self.kit_piece_fault(why.to_string()))?
        .map_err(|why| self.kit_piece_refusal(why))?;
        Ok(self.kit_inventory().await)
    }

    fn kit_cli(&self) -> Cli {
        Cli {
            path: self.host().path.clone(),
            home: PathBuf::from(&self.host().home),
            within: INSTALL_WITHIN,
        }
    }

    fn kit_trashed_path(&self) -> PathBuf {
        PathBuf::from(&self.host().kit_home).join(TRASHED)
    }

    fn kit_piece_refusal(&self, why: String) -> Refusal {
        Refusal::Unacceptable(WireError::raised(KIT_PIECE_REFUSED, why, self.run_id()))
    }

    fn kit_piece_fault(&self, why: String) -> Refusal {
        Refusal::Fault(WireError::raised(KIT_PIECE_REFUSED, why, self.run_id()))
    }
}
