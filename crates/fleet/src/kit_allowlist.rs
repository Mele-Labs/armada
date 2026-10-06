//! Kit's allowlist: the commands every Job on this machine may run without
//! asking, in `~/.armada/allowed-commands`. `docs/concepts/kit.md`.
//!
//! **A plain file a person can edit.** One command per line, `#` starts a note,
//! and a line Armada wrote ends in a tab and where it came from. It is read on
//! every permission question, so a line added while Fleet runs is in force.
//!
//! **Matched by [`covers`](crate::permitting::covers)**, as a repository's Always
//! allow is: the command, or it with plain arguments, and never a chained one.
//! It is read after the destructive, check-runner and ungrantable answers, so it
//! widens only what asking would have decided. A chained command is not kept.

use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{KitAllowedCommand, KitAllowedCommands, KitAllowedSource, WireError};

use crate::daemon::Fleet;
use crate::permitting::{chains, covers, NotPermitted};

/// The file's name inside Kit's home.
pub const FILE: &str = "allowed-commands";

const FROM_RETRO: &str = "retro ";
const FROM_ALWAYS: &str = "always allow";

const HEADER: &str = "\
# Commands every Job on this machine may run without asking.
# One per line. A command also allows itself with plain arguments after it,
# and never one that chains another command.
# A line Armada added ends in a tab and where it came from; yours needs neither.
";

/// Not job-scoped, so it earns its own code, `crate::kit`'s reason.
const KIT_ALLOWLIST_REFUSED: &str = "fleet.kit_allowlist_refused";

/// One writer at a time, so two answers arriving together cannot both read the
/// file before either writes it and lose a line.
static WRITING: Mutex<()> = Mutex::new(());

/// What the file holds, line by line.
pub(crate) fn parse(text: &str) -> Vec<KitAllowedCommand> {
    text.lines()
        .filter_map(|line| {
            let line = line.trim_end_matches('\r');
            if line.trim().is_empty() || line.trim_start().starts_with('#') {
                return None;
            }
            let (run, tail) = match line.split_once('\t') {
                Some((run, tail)) => (run, Some(tail.trim())),
                None => (line, None),
            };
            let run = run.trim();
            if run.is_empty() {
                return None;
            }
            let from = match tail {
                Some(FROM_ALWAYS) => KitAllowedSource::AlwaysAllow,
                Some(tail) => match tail.strip_prefix(FROM_RETRO) {
                    Some(id) if !id.trim().is_empty() => KitAllowedSource::RetroItem {
                        lesson_id: id.trim().to_string(),
                    },
                    _ => KitAllowedSource::ByHand,
                },
                None => KitAllowedSource::ByHand,
            };
            Some(KitAllowedCommand {
                run: run.to_string(),
                from,
            })
        })
        .collect()
}

/// Why a command cannot be kept, or `None` where it can.
pub(crate) fn unkeepable(run: &str) -> Option<&'static str> {
    if run.trim().is_empty() {
        Some("there is no command in it")
    } else if run.contains(['\t', '\n', '\r']) {
        Some("a line holds one command, and this one spans lines or holds a tab")
    } else if chains(run) {
        Some("it chains another command, and a standing grant is never given for one")
    } else {
        None
    }
}

fn line_of(run: &str, from: &KitAllowedSource) -> String {
    match from {
        KitAllowedSource::RetroItem { lesson_id } => format!("{run}\t{FROM_RETRO}{lesson_id}\n"),
        KitAllowedSource::AlwaysAllow => format!("{run}\t{FROM_ALWAYS}\n"),
        KitAllowedSource::ByHand => format!("{run}\n"),
    }
}

/// Where an inventory row says a command came from, as a person reads it.
pub(crate) fn said_source(from: &KitAllowedSource) -> String {
    match from {
        KitAllowedSource::RetroItem { lesson_id } => format!("retro item {lesson_id}"),
        KitAllowedSource::AlwaysAllow => String::from("always allow"),
        KitAllowedSource::ByHand => String::from("written by hand"),
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
    fn kit_allowlist_path(&self) -> PathBuf {
        PathBuf::from(&self.host().kit_home).join(FILE)
    }

    /// Every command Kit's allowlist holds, in the order the file holds them.
    /// **Empty where the file is absent or will not read**: nothing is allowed
    /// by a list nobody could read, the closed way to be wrong.
    pub(crate) async fn kit_allowed_commands(&self) -> Vec<KitAllowedCommand> {
        self.kit_allowlist_read().await.unwrap_or_default()
    }

    /// The same read, saying why where it would not. **An absent file is an
    /// empty list**, which it is; a file that is there and will not read is
    /// not, and an inventory says so rather than drawing it as empty.
    pub(crate) async fn kit_allowlist_read(&self) -> Result<Vec<KitAllowedCommand>, String> {
        match tokio::fs::read_to_string(self.kit_allowlist_path()).await {
            Ok(text) => Ok(parse(&text)),
            Err(cause) if cause.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
            Err(cause) => Err(cause.to_string()),
        }
    }

    /// Whether a line of Kit's allowlist covers `command`.
    pub(crate) async fn kit_allows(&self, command: &str) -> bool {
        self.kit_allowed_commands()
            .await
            .iter()
            .any(|allowed| covers(&allowed.run, command))
    }

    /// Add a command to Kit's allowlist, and say whether this call did: **a
    /// command already listed is not added twice**, so agreeing twice, or
    /// pressing Always allow on what a person wrote by hand, leaves one line.
    pub(crate) async fn allow_in_kit(
        &self,
        run: &str,
        from: KitAllowedSource,
    ) -> Result<bool, NotPermitted> {
        let run = run.trim().to_string();
        if let Some(why) = unkeepable(&run) {
            return Err(NotPermitted::NotRecorded {
                cause: format!("`{run}` cannot go in Kit's allowlist: {why}"),
            });
        }
        let path = self.kit_allowlist_path();
        tokio::task::spawn_blocking(move || append(&path, &run, &from))
            .await
            .map_err(|cause| NotPermitted::NotRecorded {
                cause: cause.to_string(),
            })?
            .map_err(|cause| NotPermitted::NotRecorded {
                cause: format!("Kit's allowlist would not write: {cause}"),
            })
    }

    /// `remove_kit_allowed_command`: take the line spelled `run` out, and answer
    /// with what the file holds now. Every other line, notes included, stays as
    /// the person left it.
    pub(crate) async fn remove_from_kit_allowlist(
        &self,
        run: &str,
    ) -> Result<KitAllowedCommands, Refusal> {
        let path = self.kit_allowlist_path();
        let wanted = run.trim().to_string();
        let removed = tokio::task::spawn_blocking(move || remove(&path, &wanted))
            .await
            .map_err(|cause| self.kit_allowlist_refusal(cause.to_string()))?
            .map_err(|cause| {
                self.kit_allowlist_refusal(format!("Kit's allowlist would not write: {cause}"))
            })?;
        if !removed {
            return Err(self.kit_allowlist_refusal(format!(
                "Kit's allowlist holds no command spelled `{}`, so there is nothing to take out",
                run.trim()
            )));
        }
        Ok(KitAllowedCommands {
            commands: self.kit_allowed_commands().await,
        })
    }

    /// A 409 with no Job to name, `crate::kit`'s shape.
    pub(crate) fn kit_allowlist_refusal(&self, why: String) -> Refusal {
        Refusal::Unacceptable(WireError::raised(KIT_ALLOWLIST_REFUSED, why, self.run_id()))
    }
}

/// Write `run` as a new line unless the file already lists it.
fn append(path: &std::path::Path, run: &str, from: &KitAllowedSource) -> std::io::Result<bool> {
    let _one_at_a_time = WRITING.lock().unwrap_or_else(|held| held.into_inner());
    let text = std::fs::read_to_string(path).unwrap_or_default();
    if parse(&text).iter().any(|held| held.run == run) {
        return Ok(false);
    }
    if let Some(folder) = path.parent() {
        std::fs::create_dir_all(folder)?;
    }
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    if text.is_empty() {
        file.write_all(HEADER.as_bytes())?;
    } else if !text.ends_with('\n') {
        file.write_all(b"\n")?;
    }
    file.write_all(line_of(run, from).as_bytes())?;
    Ok(true)
}

/// Rewrite the file without the line spelled `run`, and say whether there was
/// one.
fn remove(path: &std::path::Path, run: &str) -> std::io::Result<bool> {
    let _one_at_a_time = WRITING.lock().unwrap_or_else(|held| held.into_inner());
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(cause) if cause.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(cause) => return Err(cause),
    };
    let mut found = false;
    let kept: String = text
        .split_inclusive('\n')
        .filter(|line| {
            let held = parse(line);
            let spelled = held.first().is_some_and(|held| held.run == run);
            found |= spelled;
            !spelled
        })
        .collect();
    if found {
        std::fs::write(path, kept)?;
    }
    Ok(found)
}
