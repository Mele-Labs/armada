//! What a message to a hosted session, or to Helm, carried: kept outside every worktree, and read
//! back by the id the thread's row names.
//!
//! **One keeper for both**, with the directory as a parameter: a Session keeps under
//! `sessions/<id>`, Helm under `helm/<conversation>`, and the ids are minted from the key given so
//! two files of one conversation never share one.

use std::path::{Path, PathBuf};

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::{Refusal, StoredFile};
use base64::Engine as _;
use ipc::SentFile;

use super::rows::safe_name;
use crate::daemon::Fleet;

/// An attachment that would not decode, or is too large. A 422.
pub(crate) const ATTACHMENT_REFUSED: &str = "fleet.session_attachment_refused";

/// The most one attachment may be, decoded.
const MOST_AN_ATTACHMENT: usize = 25 * 1024 * 1024;

/// Past this many bytes a file is told with its size. About 8 000 tokens, at four bytes each: the
/// limit Bridge turns a paste into a file at (`LARGE_PASTE_TOKENS`).
const LARGE_ATTACHMENT_BYTES: u64 = 32_000;

/// How a file attached to a message is told to the agent: its path, and for a big one its size and
/// how to read it, since the path alone says nothing of what opening it costs.
///
/// **Never a fault.** A picture is read by the agent as a picture, and a file that cannot be
/// measured is told as a small one is, so a send is never refused for the sake of a warning.
pub(crate) fn attached_line(path: &str) -> String {
    let told = format!("Attached file: {path}");
    let is_picture = Path::new(path)
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| super::showing_file::PICTURES.iter().any(|(known, _)| known.eq_ignore_ascii_case(extension)));
    let Some(bytes) = std::fs::metadata(path).ok().map(|found| found.len()).filter(|bytes| !is_picture && *bytes > LARGE_ATTACHMENT_BYTES) else {
        return told;
    };
    let size = if bytes >= 1 << 20 { format!("{:.1} MB", bytes as f64 / f64::from(1_u32 << 20)) } else { format!("{} KB", (bytes + 512) / 1024) };
    let tokens = (bytes / 4 + 500) / 1000;
    format!("{told} ({size}, about {tokens}k tokens: read it in parts, or search it with grep or jq, rather than all at once)")
}

/// What a message carried, as it was kept.
pub(crate) struct Kept {
    pub files: Vec<SentFile>,
    /// `(media type, base64)` for each picture.
    pub pictures: Vec<(String, String)>,
    /// Where each file that is not a picture is kept.
    pub paths: Vec<String>,
    /// Where each picture is kept, for a session that is told paths and not
    /// pictures.
    pub picture_paths: Vec<String>,
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
    pub(crate) fn uploads_of(&self, id: &str) -> PathBuf {
        std::path::Path::new(&self.host().attachments_dir)
            .join("sessions")
            .join(id)
    }

    /// Store what a message carried, outside every worktree.
    ///
    /// `directory` is where the files go and `key` mints their ids: a session's id for a session,
    /// the conversation's key for Helm.
    pub(crate) fn keep_uploads(
        &self,
        directory: &Path,
        key: &str,
        uploads: &[ipc::SessionUpload],
    ) -> Result<Kept, Refusal> {
        let mut kept = Kept {
            files: Vec::new(),
            pictures: Vec::new(),
            paths: Vec::new(),
            picture_paths: Vec::new(),
        };
        if uploads.is_empty() {
            return Ok(kept);
        }
        std::fs::create_dir_all(directory)
            .map_err(|why| self.hosted_fault(ATTACHMENT_REFUSED, &why.to_string()))?;
        for upload in uploads {
            let bytes = base64::engine::general_purpose::STANDARD
                .decode(upload.data.as_bytes())
                .map_err(|why| {
                    self.hosted_refusal(
                        ATTACHMENT_REFUSED,
                        &format!("`{}` is not base64: {why}", upload.name),
                    )
                })?;
            if bytes.len() > MOST_AN_ATTACHMENT {
                return Err(self.hosted_refusal(
                    ATTACHMENT_REFUSED,
                    &format!(
                        "`{}` is larger than {} MB",
                        upload.name,
                        MOST_AN_ATTACHMENT >> 20
                    ),
                ));
            }
            let file = format!("f{}", self.row_id(key).replace('-', ""));
            let path = directory.join(format!("{file}-{}", safe_name(&upload.name)));
            std::fs::write(&path, &bytes)
                .map_err(|why| self.hosted_fault(ATTACHMENT_REFUSED, &why.to_string()))?;
            if upload.media_type.starts_with("image/") {
                kept.pictures
                    .push((upload.media_type.clone(), upload.data.clone()));
                kept.picture_paths.push(path.to_string_lossy().into_owned());
            } else {
                kept.paths.push(path.to_string_lossy().into_owned());
            }
            kept.files.push(SentFile {
                id: file,
                name: upload.name.clone(),
                media_type: upload.media_type.clone(),
            });
        }
        Ok(kept)
    }

    /// The bytes a row's `SentFile` names, found in `directory` by its id and served under the
    /// media type they were sent as. **The id is only ever compared to file names**, never joined
    /// to a path, so no id can leave the directory.
    pub(crate) fn stored_file(
        &self,
        directory: &Path,
        file: &str,
        media_type: String,
    ) -> Result<StoredFile, Refusal> {
        let at = std::fs::read_dir(directory)
            .ok()
            .and_then(|entries| {
                entries
                    .flatten()
                    .find(|entry| entry.file_name().to_string_lossy().starts_with(&format!("{file}-")))
            })
            .ok_or_else(|| self.hosted_refusal(ATTACHMENT_REFUSED, "that file is no longer kept"))?;
        let bytes = std::fs::read(at.path())
            .map_err(|why| self.hosted_fault(ATTACHMENT_REFUSED, &why.to_string()))?;
        Ok(StoredFile { media_type, bytes })
    }
}
