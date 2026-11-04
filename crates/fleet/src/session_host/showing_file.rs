//! `show_file`: a session puts a file in front of the person. `docs/concepts/session.md`, *Showing a file*.
//!
//! **An artifact of form `file`, keyed by its canonical path.** The ledger row is upserted on kind and
//! target and its detail replaced whole, so a file the session already wrote is the same row with `by`
//! and `shown` added. `shown` is the time of the call: a row whose detail did not change would publish
//! nothing, and Bridge could not tell a second show from the first. **`written` is carried on a row
//! the session wrote**, since replacing the detail would otherwise lose that it did.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::{Refusal, Sessions, ShownFile};
use ipc::{AttachmentReport, SessionFact, SessionId, SessionRecord, SessionReport, ShowFile, ShownText, WireError};
use store::Holder;

use super::document_answers::is_document;
use super::documents::revisions_named;
use crate::daemon::Fleet;

const FILE_UNNAMED: &str = "fleet.file_unnamed";
const FILE_MISSING: &str = "fleet.file_missing";
const FILE_NOT_A_FILE: &str = "fleet.file_not_a_file";
const FILE_TOO_LARGE: &str = "fleet.file_too_large";

const SHOWN_NOT_SHOWN: &str = "fleet.shown_file_not_shown";
const SHOWN_GONE: &str = "fleet.shown_file_gone";
const SHOWN_NOT_TEXT: &str = "fleet.shown_file_not_text";

/// The most the phone is sent of a text file: a phone screen of it.
const MOST_SENT: u64 = 1024 * 1024;

/// The pictures a phone draws, by extension, and the type each is sent under.
pub(super) const PICTURES: [(&str, &str); 6] = [
    ("png", "image/png"),
    ("jpg", "image/jpeg"),
    ("jpeg", "image/jpeg"),
    ("gif", "image/gif"),
    ("webp", "image/webp"),
    ("heic", "image/heic"),
];

/// The videos the phone streams, by extension, and the type each is served under.
const VIDEOS: [(&str, &str); 3] = [("mp4", "video/mp4"), ("mov", "video/quicktime"), ("webm", "video/webm")];

/// The most Bridge's panel reads of a file: `MOST_BYTES` in `apps/desktop/src/main/session-file.ts`.
/// **A picture is sent up to this too, not up to `MOST_SENT`**: a retina screenshot often passes 1 MiB,
/// and whatever `show_file` accepted should open on the phone.
const MOST_BYTES: u64 = 8 * 1024 * 1024;

/// The most a video may be to be shown. It is never read into memory: the phone streams it from the
/// Mac's disk through the Gateway, so the cap bounds what an agent may point at, not what Fleet holds.
const MOST_VIDEO_BYTES: u64 = 256 * 1024 * 1024;

/// What `table` calls the file at `path`, by its extension, in any case.
fn media_type_in(table: &[(&str, &'static str)], path: &str) -> Option<&'static str> {
    let (_, extension) = path.rsplit_once('.')?;
    table.iter().find(|(known, _)| known.eq_ignore_ascii_case(extension)).map(|(_, media_type)| *media_type)
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
    pub(crate) async fn file_shown(
        &self,
        caller: Option<api::Caller>,
        show: ShowFile,
    ) -> Result<SessionRecord, Refusal> {
        let id = caller
            .and_then(|caller| self.session_holding(&caller))
            .or_else(|| show.session_id.map(|named| named.as_str().trim().to_string()))
            .filter(|id| !id.is_empty())
            .ok_or_else(|| {
                self.hosted_refusal(
                    FILE_UNNAMED,
                    "a file needs a job to be shown for, and this call places none",
                )
            })?;
        let (harness, cwd) = {
            let store = self.store().lock().await;
            let session = store
                .session(&id)
                .map_err(|why| self.ledger_fault(why))?
                .ok_or_else(|| self.hosted_refusal(FILE_UNNAMED, "a job Fleet knows"))?;
            (session.harness, session.cwd)
        };
        let path = self.file_resolved(&cwd, show.path.trim()).await?;
        let title = path.rsplit('/').next().unwrap_or(&path).to_string();
        let written = self.file_written(&id, &path).await?;
        let mut detail: std::collections::BTreeMap<String, String> = [
            ("form".to_string(), "file".to_string()),
            ("by".to_string(), "agent".to_string()),
            ("title".to_string(), title),
            ("shown".to_string(), self.now().as_str().to_string()),
        ]
        .into();
        if written {
            detail.insert("written".to_string(), "true".to_string());
        }
        self.report_session(SessionReport {
            harness,
            session_id: SessionId::carried(&id),
            fact: SessionFact::Attached {
                attachment: AttachmentReport {
                    kind: String::from("artifact"),
                    target: path,
                    detail,
                },
            },
        })
        .await
    }

    /// Whether the session wrote `path`: its row is a file with no `by`, or one a show already marked
    /// `written`. A picture it looked at, once shown, is a file it showed and did not write.
    async fn file_written(&self, id: &str, path: &str) -> Result<bool, Refusal> {
        let store = self.store().lock().await;
        let held = store.attachments_of(&Holder::session(id)).map_err(|why| self.ledger_fault(why))?;
        Ok(held.iter().any(|one| {
            one.kind == "artifact"
                && one.target == path
                && one.detail.get("form").map(String::as_str) == Some("file")
                && (!one.detail.contains_key("by") || one.detail.get("written").map(String::as_str) == Some("true"))
        }))
    }

    /// The absolute, canonical path of a file Bridge's panel can read, or the refusal that says where
    /// the path resolved and what is allowed.
    async fn file_resolved(&self, cwd: &str, given: &str) -> Result<String, Refusal> {
        let allowed = format!(
            "Allowed: an existing file of at most {MOST_BYTES} bytes, or {MOST_VIDEO_BYTES} bytes for an mp4, \
             mov or webm video, as an absolute path or relative to this job's folder {cwd}"
        );
        let joined = match std::path::Path::new(given).is_absolute() {
            true => std::path::PathBuf::from(given),
            false => std::path::Path::new(cwd).join(given),
        };
        if given.is_empty() {
            return Err(self.hosted_refusal(FILE_MISSING, &format!("`path` is blank. {allowed}")));
        }
        let Ok(resolved) = tokio::fs::canonicalize(&joined).await else {
            return Err(self.hosted_refusal(
                FILE_MISSING,
                &format!("there is no file at {}. {allowed}", joined.display()),
            ));
        };
        let resolved_text = resolved.to_string_lossy().to_string();
        let Ok(meta) = tokio::fs::metadata(&resolved).await else {
            return Err(self.hosted_refusal(
                FILE_MISSING,
                &format!("{resolved_text} cannot be read. {allowed}"),
            ));
        };
        if !meta.is_file() {
            let what = if meta.is_dir() { "a folder" } else { "not a regular file" };
            return Err(self.hosted_refusal(
                FILE_NOT_A_FILE,
                &format!("{resolved_text} is {what}. {allowed}"),
            ));
        }
        let most = match media_type_in(&VIDEOS, &resolved_text) {
            Some(_) => MOST_VIDEO_BYTES,
            None => MOST_BYTES,
        };
        if meta.len() > most {
            return Err(self.hosted_refusal(
                FILE_TOO_LARGE,
                &format!("{resolved_text} is {} bytes. {allowed}", meta.len()),
            ));
        }
        Ok(resolved_text)
    }

    /// `get_shown_file`: the file at `path`, only where this session showed it or kept it as a revision
    /// of its document. The page in the worktree is never read: another Job's slot may hold it by then.
    pub(crate) async fn shown_file_read(&self, id: &str, path: &str) -> Result<ShownFile, Refusal> {
        let not_found = |code: &'static str, why: String| Refusal::NotFound(WireError::raised(code, why, self.run_id()));
        let showed = {
            let store = self.store().lock().await;
            store
                .session(id)
                .map_err(|why| self.ledger_fault(why))?
                .ok_or_else(|| not_found(SHOWN_NOT_SHOWN, format!("Fleet knows no job {id}")))?;
            let held = store.attachments_of(&Holder::session(id)).map_err(|why| self.ledger_fault(why))?;
            held.iter().any(|one| {
                one.kind == "artifact"
                    && ((one.target == path
                        && one.detail.get("form").map(String::as_str) == Some("file")
                        && one.detail.contains_key("shown"))
                        || (is_document(&one.detail) && revisions_named(&one.detail).iter().any(|kept| kept == path)))
            })
        };
        if !showed {
            return Err(not_found(SHOWN_NOT_SHOWN, format!("job {id} did not show {path}")));
        }
        let gone = |_| not_found(SHOWN_GONE, format!("{path} is no longer there"));
        let size = tokio::fs::metadata(path).await.map_err(gone)?.len();
        if let Some(media_type) = media_type_in(&VIDEOS, path) {
            // Never read, so no size is too large here: the Gateway streams it from the disk.
            return Ok(ShownFile::Video { media_type: media_type.to_string(), bytes: size });
        }
        let picture = media_type_in(&PICTURES, path);
        if size > picture.map_or(MOST_SENT, |_| MOST_BYTES) {
            return Ok(ShownFile::TooLarge { bytes: size });
        }
        let bytes = tokio::fs::read(path).await.map_err(gone)?;
        if let Some(media_type) = picture {
            return Ok(ShownFile::Picture { media_type: media_type.to_string(), bytes });
        }
        match String::from_utf8(bytes) {
            Ok(text) => Ok(ShownFile::Text(ShownText { path: path.to_string(), text })),
            Err(_) => Err(self.hosted_refusal(SHOWN_NOT_TEXT, &format!("{path} is neither text nor a picture"))),
        }
    }
}
