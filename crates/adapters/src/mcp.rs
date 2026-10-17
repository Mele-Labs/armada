//! [`publish_the_agents_door`]: the entry that tells an agent working in a
//! repository how to reach Fleet, in a file Armada does not own.

use std::fs;
use std::io;
use std::path::Path;

use serde::Serialize;

/// The `type` the agent CLI's `--mcp-config` schema accepts for the one server
/// this crate publishes: a program the agent starts for itself.
#[derive(Serialize)]
#[serde(rename_all = "lowercase")]
enum Transport {
    Stdio,
}

/// Where a repository tells an agent working in it which servers to open.
///
/// **The repository's own file, and that is what makes it safe.** An unattended
/// launch runs under `--strict-mcp-config` against a document of its own, so it
/// never reads this one. Fleet control is therefore published in the one place
/// an unattended agent cannot inherit it from.
pub const REPOSITORY_CONFIG: &str = ".mcp.json";

/// A server an agent starts for itself, as the agent CLI's schema reads one.
///
/// **A program and not an address.** Fleet's port is leased rather than fixed,
/// so a URL written into a repository is a number that goes stale the next time
/// Fleet starts — and the port it goes stale *onto* may belong to something
/// else entirely by then. The program named here reads the runtime file at the
/// moment a session opens and decides what to do about what it finds.
#[derive(Serialize)]
struct StdioServer<'a> {
    #[serde(rename = "type")]
    transport: Transport,
    command: &'a str,
    args: &'a [&'a str],
}

/// What publishing did. **Said out loud rather than inferred**, so a boot can
/// report a file it changed and stay quiet about one it did not.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Published {
    /// The entry was not there, or named something else. The file was written.
    Written,
    /// The file already says exactly this. Nothing was touched — no mtime
    /// moved and no working tree went dirty on a Fleet restart.
    AlreadyThere,
}

/// Register the agent's door in `at`, keeping everything else in the file.
///
/// **The opposite guarantee to the document above.** That one is Armada's,
/// written whole, holding only what Fleet resolved; this one is the
/// repository's, so one entry is added and every other key survives.
///
/// `command` and `args` are the caller's, because the program is `armada`'s own
/// verb and this crate may not know it. What this crate owns is the file's
/// name, the schema's spellings and the merge that does not destroy what a
/// person wrote.
///
/// **It is a selection and not a grant.** Publishing the door here tells an
/// agent standing in this repository where to knock; it confers nothing,
/// because the listener is on loopback with no authentication and every process
/// on this machine was already inside that boundary.
pub fn publish_the_agents_door(
    at: &Path,
    command: &str,
    args: &[&str],
) -> Result<Published, io::Error> {
    let existing = match fs::read(at) {
        Ok(bytes) => Some(bytes),
        Err(cause) if cause.kind() == io::ErrorKind::NotFound => None,
        Err(cause) => return Err(cause),
    };
    let written = ipc::document::merged_into(
        existing.as_deref(),
        SERVERS,
        ipc::door::SERVER,
        &StdioServer {
            transport: Transport::Stdio,
            command,
            args,
        },
    )
    .map_err(|why| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{} was left alone: {why}", at.display()),
        )
    })?;

    if existing.as_deref() == Some(written.as_bytes()) {
        return Ok(Published::AlreadyThere);
    }
    if let Some(parent) = at.parent().filter(|parent| !parent.as_os_str().is_empty()) {
        fs::create_dir_all(parent)?;
    }
    fs::write(at, written)?;
    // **No `restrict` here, unlike the document above.** That one names the
    // address a Drone reports evidence to and is Armada's alone; this one is a
    // file in somebody's repository, which they edit, review and may commit.
    Ok(Published::Written)
}

/// The key both documents hold their servers under.
///
/// **A second spelling of the `rename` above, because an attribute takes a
/// literal and not a constant.** The pair is what a test asserts rather than
/// what a reader is asked to notice: the Drone's document is built by `serde`
/// and an agent's is merged into a file, so the two reach the same key by
/// different routes and could drift apart silently.
const SERVERS: &str = "mcpServers";
