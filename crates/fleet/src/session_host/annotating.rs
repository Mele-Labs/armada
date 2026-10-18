//! `mark_annotation_done`: a hosted session marks done a note Bridge sent it. The note may sit in
//! a checkout the session holds no slot for, such as the preview's, where the slot guard refuses a
//! write; Fleet's door is the way in, and only for a note sent to the session asking.
//! `docs/concepts/session.md`.

use std::path::{Path, PathBuf};

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::annotations::{is_note_id, marked_done, NotMarked};
use ipc::{AnnotationMarked, MarkAnnotationDone};

use crate::daemon::Fleet;

const NOTE_REFUSED: &str = "fleet.annotation_refused";
const NOTE_UNWRITTEN: &str = "fleet.annotation_unwritten";

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
    /// Find the note under the served root, set its `status` to `done` and its `updatedAt` to now,
    /// and write it back through a rename. Refused unless its `sent.sessionId` is this session.
    pub(crate) async fn annotation_done(
        &self,
        caller: Option<api::Caller>,
        mark: MarkAnnotationDone,
    ) -> Result<AnnotationMarked, Refusal> {
        let id = self.waiting_session(caller, mark.session_id.as_ref())?;
        if !is_note_id(&mark.note_id) {
            return Err(self.hosted_refusal(
                NOTE_REFUSED,
                &format!("{:?} is not a note id, which is letters, digits and `-`", mark.note_id),
            ));
        }
        let (session, hosting) = self.session_and_hosting(&id).await?;
        if session.state == store::SessionState::Ended {
            return Err(self.closed(&id));
        }
        let served = self.served_named(Some(&ipc::ManifestId::carried(&hosting.manifest_id)))?;
        let root = PathBuf::from(served.root());
        let file = format!("{}.json", mark.note_id);
        let looked = adapters::annotation_places(&root);
        let Some(path) = looked.iter().map(|dir| dir.join(&file)).find(|path| path.is_file()) else {
            let named: Vec<String> = looked.iter().map(|dir| dir.display().to_string()).collect();
            return Err(self.hosted_refusal(
                NOTE_REFUSED,
                &format!("no note {file} in {}", named.join(", ")),
            ));
        };
        let bytes = std::fs::read(&path)
            .map_err(|why| self.hosted_fault(NOTE_UNWRITTEN, &format!("{}: {why}", path.display())))?;
        let now = self.now().as_str().to_string();
        let marked = marked_done(&bytes, &id, &now).map_err(|why| match why {
            NotMarked::NotSentHere => self.hosted_refusal(NOTE_REFUSED, "that note was not sent to this job"),
            NotMarked::Unreadable(why) => {
                self.hosted_refusal(NOTE_REFUSED, &format!("{} is not a note: {why}", path.display()))
            }
        })?;
        written(&path, &marked)
            .map_err(|why| self.hosted_fault(NOTE_UNWRITTEN, &format!("{}: {why}", path.display())))?;
        Ok(AnnotationMarked { path: path.display().to_string() })
    }
}

/// `bytes` to `path` through a partial beside it, so Bridge never reads half a note. The partial
/// does not end in `.json`, so Bridge's listing never reads one a crash leaves.
fn written(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|since| since.as_nanos())
        .unwrap_or_default();
    let partial = PathBuf::from(format!("{}.{}-{nanos}.partial", path.display(), std::process::id()));
    std::fs::write(&partial, bytes)?;
    std::fs::rename(&partial, path).inspect_err(|_| {
        let _ = std::fs::remove_file(&partial);
    })
}
