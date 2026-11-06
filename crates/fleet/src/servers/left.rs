//! The servers a Fleet left running, and the record that finds them.
//!
//! **A file per running server, beside its log**: `held` in the instance's
//! own directory under `.armada/servers`, naming its id, whose it is, its
//! process group, when that process started and the row Fleet showed for it.
//! Written once `serve` is up, again when it is serving, and removed however it
//! ends, so a record still there at startup is a server whose Fleet never saw it
//! end. **No table**: a run's record is a file the same way, and the store's
//! next migrations are another session's. It is Fleet's own file, written
//! whole by rename.
//!
//! **A server outlives the Fleet that started it**, and the next one adopts it:
//! where the group's leader is still the process recorded and the checkout it
//! serves is still there, startup hands the record back as a [`Kept`] and the
//! server goes back in the running set as it was — starting again, with its
//! `ready` asked afresh, where the record was written before that passed. Anything else a record names
//! that is still running is ended — a server nobody holds would hold its port
//! with nothing left to stop it.
//!
//! **A reused pid is never killed or adopted.** A group is touched only where
//! the process at its pid started when the record says it did —
//! `crate::process`, the identity check the runtime file already relies on.

use std::num::NonZeroU32;
use std::path::{Path, PathBuf};
use std::time::Duration;

use ipc::ServerState;
use serde::{Deserialize, Serialize};

use crate::process::{holder_of, Holder, StartedAt};

const RECORD: &str = "held";

/// How many times startup asks whether an ended group's leader is gone, 20ms
/// apart, so the ports it held are free before the main checkout's span is
/// probed. **Asks, not a duration**: about 2s idle, and longer under load
/// rather than run out by it.
const GOING: u32 = 100;

/// What asks a server that was still starting whether it is up, once another Fleet holds it.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Probe {
    /// The `ready` command, `${port.NAME}` resolved. None where the server declares none.
    pub ready: Option<String>,
    /// Where `ready` runs.
    pub worktree: String,
    /// The port variables it runs with.
    pub env: Vec<(String, String)>,
}

/// Write one server's record, whole or not at all. **`state` is the row as it
/// stands, and `probe` how to ask whether it is up**, which is all the next
/// Fleet needs to put the server back.
pub fn recorded(
    dir: &Path,
    state: &ServerState,
    probe: &Probe,
    whose: &str,
    group: u32,
    started: &StartedAt,
) -> std::io::Result<()> {
    let id = &state.id;
    let row = ipc::encode(state)
        .map_err(|why| std::io::Error::new(std::io::ErrorKind::InvalidData, why.to_string()))?;
    let probe = ipc::encode(probe)
        .map_err(|why| std::io::Error::new(std::io::ErrorKind::InvalidData, why.to_string()))?;
    let text = format!(
        "server {id}\nholder {whose}\ngroup {group}\nstarted {started}\nstate {row}\nprobe {probe}\n"
    );
    let next = dir.join(format!("{RECORD}.next"));
    std::fs::write(&next, text)?;
    std::fs::rename(next, dir.join(RECORD))
}

/// The server ended while this Fleet held it: there is nothing left to find.
pub fn forgotten(dir: &Path) {
    let _ = std::fs::remove_file(dir.join(RECORD));
}

/// A server still running, found again: what it takes to put it back in the
/// running set. Its record is left where it is, for the end to remove.
pub struct Kept {
    pub state: ServerState,
    pub probe: Probe,
    pub group: NonZeroU32,
    pub started: StartedAt,
    /// Its own directory, which holds the log and the record.
    pub dir: PathBuf,
}

/// What startup did with the records it found.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Reaped {
    /// Still the process recorded, with its checkout there: kept running, and
    /// handed back to be held.
    pub adopted: usize,
    /// Still the process recorded, and ended.
    pub ended: usize,
    /// Gone, or a different process now at the pid: left alone.
    pub left_alone: usize,
    /// The probe itself failed. The record is kept for the next start.
    pub unasked: usize,
}

/// End every group a record under `records_root` names that is still the
/// process it recorded, and drop each record that was answered. **With
/// `adopting`, a group whose checkout is still there is kept running** and
/// handed back instead.
pub fn reaped(records_root: &str, adopting: bool) -> (Reaped, Vec<Kept>) {
    let mut reaped = Reaped::default();
    let mut kept = Vec::new();
    let mut found = Vec::new();
    walked(
        &Path::new(records_root).join(".armada").join("servers"),
        3,
        &mut found,
    );
    for record in found {
        let Some((group, started, state, probe)) = read(&record) else {
            // Nothing to confirm a process against, so nothing is killed.
            let _ = std::fs::remove_file(&record);
            reaped.left_alone += 1;
            continue;
        };
        match holder_of(group.get()) {
            Ok(Holder::Held(now)) if now == started => {
                // A row whose checkout is gone has no holder to hand the server to.
                let holder = state
                    .filter(|state| adopting && Path::new(&state.checkout.path).is_dir());
                if let (Some(state), Some(dir)) = (holder, record.parent()) {
                    reaped.adopted += 1;
                    kept.push(Kept {
                        state,
                        probe,
                        group,
                        started,
                        dir: dir.to_path_buf(),
                    });
                    continue;
                }
                crate::group::end_the_group(group);
                gone(group, &started);
                reaped.ended += 1;
            }
            Ok(_) => reaped.left_alone += 1,
            Err(_) => {
                reaped.unasked += 1;
                continue;
            }
        }
        let _ = std::fs::remove_file(&record);
    }
    (reaped, kept)
}

/// Wait, bounded, for the ended leader to stop being the process recorded.
fn gone(group: NonZeroU32, started: &StartedAt) {
    for _ in 0..GOING {
        match holder_of(group.get()) {
            Ok(Holder::Held(now)) if &now == started => {
                std::thread::sleep(Duration::from_millis(20))
            }
            _ => return,
        }
    }
}

/// Every record at most `depth` directories below `dir`: `main/<id>` and
/// `jobs/<handle>/<id>`.
fn walked(dir: &Path, depth: u8, found: &mut Vec<PathBuf>) {
    let Ok(listing) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in listing.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if depth > 0 {
                walked(&path, depth - 1, found);
            }
        } else if path.file_name().is_some_and(|name| name == RECORD) {
            found.push(path);
        }
    }
}

/// The group and start time that identify the process, and the row where it
/// can be read: a record without one is still ended, but never adopted.
fn read(record: &Path) -> Option<(NonZeroU32, StartedAt, Option<ServerState>, Probe)> {
    let text = std::fs::read_to_string(record).ok()?;
    let field = |key: &str| {
        text.lines().find_map(|line| {
            line.strip_prefix(key)?
                .strip_prefix(' ')
                .map(str::to_string)
        })
    };
    let group = NonZeroU32::new(field("group")?.parse().ok()?)?;
    let state = field("state").and_then(|row| ipc::decode("a kept server row", row.as_bytes()).ok());
    let probe = field("probe")
        .and_then(|row| ipc::decode("a kept server probe", row.as_bytes()).ok())
        .unwrap_or_default();
    Some((group, StartedAt::carried(field("started")?), state, probe))
}
