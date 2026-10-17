//! Kit's allowlist: the commands every Job on this machine may run without
//! asking, in `~/.armada/allowed-commands`. `docs/concepts/kit.md`.
//!
//! **A plain file a person can edit.** One command per line, `#` starts a note,
//! and a line Armada wrote ends in a tab and where it came from. It is read on
//! every permission question, so a line added while Fleet runs is in force.
//!
//! **Matched by [`covers`](crate::allowed::covers)**, as a repository's Always
//! allow is: the command, or it with plain arguments, and never a chained one.
//! It is read after the destructive, check-runner and ungrantable answers, so it
//! widens only what asking would have decided. A chained command is not kept.

use std::path::PathBuf;
use std::sync::Mutex;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{KitAllowedCommand, KitAllowedCommands, KitAllowedSource, WireError};

use crate::daemon::Fleet;

/// The file's name inside Kit's home.
pub const FILE: &str = "allowed-commands";

const FROM_RETRO: &str = "retro ";
const FROM_ALWAYS: &str = "always allow";

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

/// Where an inventory row says a command came from, as a person reads it.
pub(crate) fn said_source(from: &KitAllowedSource) -> String {
    match from {
        KitAllowedSource::RetroItem { lesson_id } => format!("retro item {lesson_id}"),
        KitAllowedSource::AlwaysAllow => String::from("always allow"),
        KitAllowedSource::ByHand => String::from("written by hand"),
    }
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
    /// Every command Kit's allowlist holds, in the order the file holds them.
    /// **Empty where the file is absent or will not read**: nothing is allowed
    /// by a list nobody could read, the closed way to be wrong.
    pub(crate) async fn kit_allowed_commands(&self) -> Vec<KitAllowedCommand> {
        self.kit_allowlist_read().await.unwrap_or_default()
    }

    fn kit_allowlist_path(&self) -> PathBuf {
        PathBuf::from(&self.host().kit_home).join(FILE)
    }

    /// A 409 with no Job to name, `crate::kit`'s shape.
    pub(crate) fn kit_allowlist_refusal(&self, why: String) -> Refusal {
        Refusal::Unacceptable(WireError::raised(KIT_ALLOWLIST_REFUSED, why, self.run_id()))
    }
}
