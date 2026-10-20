//! `show_file`: a session puts a file in front of the person. `docs/concepts/session.md`, *Showing a file*.
//!
//! **An artifact of form `file`, keyed by its canonical path.** The ledger row is upserted on kind and
//! target and its detail replaced whole, so a file the session already wrote is the same row with `by`
//! and `shown` added. `shown` is the time of the call: a row whose detail did not change would publish
//! nothing, and Bridge could not tell a second show from the first.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::{Refusal, Sessions};
use ipc::{AttachmentReport, SessionFact, SessionId, SessionRecord, SessionReport, ShowFile};

use crate::daemon::Fleet;

const FILE_UNNAMED: &str = "fleet.file_unnamed";
const FILE_MISSING: &str = "fleet.file_missing";
const FILE_NOT_A_FILE: &str = "fleet.file_not_a_file";
const FILE_TOO_LARGE: &str = "fleet.file_too_large";

/// The most Bridge's panel reads of a file: `MOST_BYTES` in `apps/desktop/src/main/session-file.ts`.
const MOST_BYTES: u64 = 8 * 1024 * 1024;

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
        self.report_session(SessionReport {
            harness,
            session_id: SessionId::carried(&id),
            fact: SessionFact::Attached {
                attachment: AttachmentReport {
                    kind: String::from("artifact"),
                    target: path,
                    detail: [
                        ("form".to_string(), "file".to_string()),
                        ("by".to_string(), "agent".to_string()),
                        ("title".to_string(), title),
                        ("shown".to_string(), self.now().as_str().to_string()),
                    ]
                    .into(),
                },
            },
        })
        .await
    }

    /// The absolute, canonical path of a file Bridge's panel can read, or the refusal that says where
    /// the path resolved and what is allowed.
    async fn file_resolved(&self, cwd: &str, given: &str) -> Result<String, Refusal> {
        let allowed = format!(
            "Allowed: an existing file of at most {MOST_BYTES} bytes, as an absolute path or relative to \
             this job's folder {cwd}"
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
        if meta.len() > MOST_BYTES {
            return Err(self.hosted_refusal(
                FILE_TOO_LARGE,
                &format!("{resolved_text} is {} bytes. {allowed}", meta.len()),
            ));
        }
        Ok(resolved_text)
    }
}
