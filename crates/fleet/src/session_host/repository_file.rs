//! A file of a Job, read for a client that cannot read the agent's disk: `get_repository_file`.
//!
//! **Electron's file panel is the spec**: `locate`, `inCheckout` and `read` in `file-panel.ts`, `namesFile`
//! and `typeOfFile` in `session-file.ts`. A path is read only where the Job's own ledger names it, which may
//! be anywhere on disk because the agent made or viewed it, or where it ends up, links followed, a regular
//! file strictly inside the Job's checkout. Nothing else is read.

use std::path::{Path, PathBuf};

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use base64::Engine as _;
use ipc::{RepositoryFile, SessionId};
use store::{Holder, KeptAttachment};

use crate::daemon::Fleet;

/// No such Job, or neither rule places the path. A 422.
pub(crate) const REPOSITORY_FILE_NOT_ADDRESSABLE: &str = "fleet.repository_file_not_addressable";
/// Over [`MOST_BYTES`], by its size before it is read. A 422.
pub(crate) const REPOSITORY_FILE_TOO_BIG: &str = "fleet.repository_file_too_big";
/// Not a picture, and a NUL in its first 4096 bytes or not UTF-8. A 422.
pub(crate) const REPOSITORY_FILE_BINARY: &str = "fleet.repository_file_binary";
/// Any other failure to read it. A 500.
pub(crate) const REPOSITORY_FILE_UNREADABLE: &str = "fleet.repository_file_unreadable";

/// The most of a file Fleet reads, so a log or a recording does not fill a client.
const MOST_BYTES: u64 = 8 * 1024 * 1024;

/// How much of the start of a file is looked at for a NUL.
const NUL_LOOKED_FOR_IN: usize = 4096;

/// Why a path was not read, in the order the checks are made.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Unread {
    NotAddressable,
    TooBig,
    Binary,
    Unreadable(String),
}

/// Whether the ledger names `path` as a file the Job made or viewed.
fn named(ledger: &[KeptAttachment], path: &str) -> bool {
    ledger.iter().any(|one| {
        one.kind == "artifact"
            && match one.detail.get("form").map(String::as_str) {
                Some("file" | "image") => one.target == path,
                // A sent note's picture, at the path Fleet recorded for it.
                Some("note") => one.detail.get("picture").is_some_and(|picture| picture == path),
                // A document's or a sketch's pages are the revisions Fleet kept, and only those.
                Some("document" | "sketch") => one
                    .detail
                    .get("revisions")
                    .is_some_and(|kept| ipc::revision_paths_of_detail(kept).iter().any(|kept| kept == path)),
                _ => false,
            }
    })
}

/// Where on disk `path` is, or `None` where neither rule places it. Electron's second candidate, the
/// main checkout, can never pass the inside-the-slot test, so it is left out.
fn located(cwd: &str, ledger: &[KeptAttachment], path: &str) -> Option<PathBuf> {
    if path.is_empty() {
        return None;
    }
    // An absolute path replaces the cwd in a join.
    let from_here = Path::new(cwd).join(path);
    if named(ledger, path) {
        return Some(from_here);
    }
    let root = std::fs::canonicalize(cwd).ok()?;
    let there = std::fs::canonicalize(from_here).ok()?;
    (there != root && there.starts_with(&root) && there.is_file()).then_some(there)
}

/// What a client draws a file as, off its extension.
fn type_of(path: &Path) -> &'static str {
    let ext = path.extension().and_then(|ext| ext.to_str()).map(str::to_lowercase);
    match ext.as_deref() {
        Some("png") => "image/png",
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("webp") => "image/webp",
        Some("svg") => "image/svg+xml",
        Some("md" | "markdown") => "text/markdown",
        _ => "text/plain",
    }
}

/// The file `path` names for a Job whose checkout is `cwd` and whose ledger is `ledger`. **Blocking**.
pub(crate) fn read_repository_file(cwd: &str, ledger: &[KeptAttachment], path: &str) -> Result<RepositoryFile, Unread> {
    let at = located(cwd, ledger, path).ok_or(Unread::NotAddressable)?;
    let unreadable = |why: std::io::Error| Unread::Unreadable(why.to_string());
    if std::fs::metadata(&at).map_err(unreadable)?.len() > MOST_BYTES {
        return Err(Unread::TooBig);
    }
    let bytes = std::fs::read(&at).map_err(unreadable)?;
    let media_type = type_of(&at).to_string();
    if media_type.starts_with("image/") {
        let bytes = base64::engine::general_purpose::STANDARD.encode(bytes);
        return Ok(RepositoryFile::Picture { bytes, media_type });
    }
    if bytes[..bytes.len().min(NUL_LOOKED_FOR_IN)].contains(&0) {
        return Err(Unread::Binary);
    }
    // Strict, so a file that is not UTF-8 is refused and not shown with its bytes replaced.
    let text = String::from_utf8(bytes).map_err(|_| Unread::Binary)?;
    Ok(RepositoryFile::Text { text, media_type })
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
    /// `get_repository_file`. The Job may be hosted or a terminal one: its record is all that is read.
    pub(crate) async fn repository_file_served(
        &self,
        id: SessionId,
        path: String,
    ) -> Result<RepositoryFile, Refusal> {
        let (cwd, ledger) = {
            let store = self.store().lock().await;
            let Some(session) = store.session(id.as_str()).map_err(|why| self.ledger_fault(why))? else {
                return Err(self.hosted_refusal(REPOSITORY_FILE_NOT_ADDRESSABLE, "no job is named that"));
            };
            let held = store.attachments_of(&Holder::session(&session.id)).map_err(|why| self.ledger_fault(why))?;
            (session.cwd, held)
        };
        let read = tokio::task::spawn_blocking(move || read_repository_file(&cwd, &ledger, &path))
            .await
            .map_err(|why| self.hosted_fault(REPOSITORY_FILE_UNREADABLE, &why.to_string()))?;
        read.map_err(|why| match why {
            Unread::NotAddressable => self.hosted_refusal(
                REPOSITORY_FILE_NOT_ADDRESSABLE,
                "that path is neither one the job's ledger names nor a file inside its checkout",
            ),
            Unread::TooBig => self.hosted_refusal(REPOSITORY_FILE_TOO_BIG, "that file is over 8 MiB"),
            Unread::Binary => self.hosted_refusal(REPOSITORY_FILE_BINARY, "that file is not text and not a picture"),
            Unread::Unreadable(why) => self.hosted_fault(REPOSITORY_FILE_UNREADABLE, &why),
        })
    }
}
