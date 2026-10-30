//! A Job's document: an HTML page it designed and wrote to `.armada/documents/<name>.html` in its
//! worktree, where git ignores it. **Each write that changes the page is kept as a revision**, in Fleet's
//! own attachments folder and not in the worktree, so a released slot keeps them. The page is one
//! artifact on the Job's ledger, upserted on its path, whose `revisions` detail names the kept files
//! oldest first. The sketches it embeds are checked as `show_sketch` checks one, and a Job whose page
//! holds one that will not draw is told. The write is never refused. **Each revision kept is put to
//! the person**, read off the page kept here and never off the tool call: `document_answers` answers it.

use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Sessions;
use ipc::{AttachmentReport, SessionFact, SessionId, SessionReport, DOCUMENT_ASKED, DOCUMENT_ITEM_PREFIX, DOCUMENT_KEPT_AT, DOCUMENT_REVISION};

use super::document_page::{sketches_in, title_of};
use crate::daemon::Fleet;

/// What a page's path is cut at to name its folder of revisions, so the same page is the same folder
/// whichever slot the Job wrote it from.
const DOCUMENTS: &str = "/.armada/documents/";

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
    /// Where a Job's pages are kept, a folder to each page.
    pub(crate) fn documents_of(&self, id: &str) -> PathBuf {
        Path::new(&self.host().attachments_dir).join("documents").join(id)
    }

    /// The page at `path` was written and the tool has answered, so the file on disk is the Job's.
    pub(super) async fn page_written(self: &Arc<Self>, id: &str, path: String) {
        let folder = self.documents_of(id).join(folder_of(&path));
        let read = path.clone();
        let kept = tokio::task::spawn_blocking(move || {
            let html = std::fs::read_to_string(&read)?;
            Ok::<_, io::Error>(keep(&folder, html.as_bytes(), "html")?.map(|revisions| (html, revisions)))
        })
        .await;
        // Unreadable, not text, or the bytes the newest revision holds: nothing new to keep.
        let Ok(Ok(Some((html, revisions)))) = kept else { return };
        let names: Vec<String> = revisions.iter().map(|one| one.to_string_lossy().into_owned()).collect();
        let Ok(encoded) = ipc::encode(&names) else { return };
        // The store replaces a row's detail whole, so leaving `settled`, `settled_at` and `words` out of
        // this one is what asks the person again about the new revision.
        let detail = [
            ("form".to_string(), "document".to_string()),
            ("title".to_string(), title_of(&html, &path)),
            ("revisions".to_string(), encoded),
            (DOCUMENT_ASKED.to_string(), "true".to_string()),
            (DOCUMENT_REVISION.to_string(), names.len().to_string()),
            (DOCUMENT_KEPT_AT.to_string(), self.instant().as_str().to_string()),
        ]
        .into();
        // A page he dismissed waits again once it is revised, as a sketch drawn again does.
        let item = format!("{DOCUMENT_ITEM_PREFIX}{path}");
        let _ = self.store().lock().await.restore_waiting(id, &item);
        let _ = self
            .report_session(SessionReport {
                harness: String::from(adapters::HOSTED_HARNESS),
                session_id: SessionId::carried(id),
                fact: SessionFact::Attached {
                    attachment: AttachmentReport { kind: String::from("artifact"), target: path.clone(), detail },
                },
            })
            .await;
        if let Some(told) = sketches_that_will_not_draw(&html, &path) {
            let (fleet, id) = (Arc::clone(self), id.to_string());
            // Off the hearing of the stream: a word to the Job is a send, and the stream is what it waits on.
            tokio::spawn(async move {
                let _ = fleet.told_by_fleet(&id, &told, ipc::Notice::DocumentSketch).await;
            });
        }
    }

    /// The turn ended: every page under `<cwd>/.armada/documents/` is read again, so one a script or a
    /// shell changed is kept too. A page whose bytes the newest revision holds keeps nothing.
    pub(super) async fn pages_reread(self: &Arc<Self>, id: &str, cwd: &str) {
        let folder = Path::new(cwd).join(DOCUMENTS.trim_matches('/'));
        let Ok(pages) = tokio::task::spawn_blocking(move || pages_under(&folder)).await else { return };
        for page in pages {
            self.page_written(id, page).await;
        }
    }
}

/// Every `.html` file under `folder`, its subfolders too, as `written_job_page` allows them. A
/// symlinked folder is not followed, so a link back up cannot loop.
fn pages_under(folder: &Path) -> Vec<String> {
    let mut found = Vec::new();
    let mut open = vec![folder.to_path_buf()];
    while let Some(dir) = open.pop() {
        for entry in std::fs::read_dir(&dir).into_iter().flatten().flatten() {
            let (Ok(kind), path) = (entry.file_type(), entry.path()) else { continue };
            if kind.is_dir() {
                open.push(path);
            } else if kind.is_file() && path.extension().is_some_and(|ext| ext.eq_ignore_ascii_case("html")) {
                found.push(path.to_string_lossy().into_owned());
            }
        }
    }
    found.sort();
    found
}

/// The one line that tells a Job which sketches of its page will not draw, or `None` where all do.
pub(crate) fn sketches_that_will_not_draw(html: &str, path: &str) -> Option<String> {
    let sketches = sketches_in(html);
    let total = sketches.len();
    let failing: Vec<String> = sketches
        .iter()
        .enumerate()
        .filter_map(|(at, sketch)| {
            let problems = ipc::page_scene_problems(&sketch.scene);
            if problems.is_empty() {
                return None;
            }
            let named = sketch.title.as_ref().map(|title| format!(" (\"{title}\")")).unwrap_or_default();
            Some(format!("sketch {} of {total}{named} will not draw: {}.", at + 1, problems.join("; ")))
        })
        .collect();
    (!failing.is_empty()).then(|| format!("In the page {path}, {}", failing.join(" ")))
}

/// The folder a page's revisions are in: its path under `.armada/documents/`, as one name.
fn folder_of(path: &str) -> String {
    let under = path.rsplit_once(DOCUMENTS).map_or(path, |(_, under)| under);
    one_name(under)
}

/// `text` as one folder name: anything but letters, digits, `-` and `.` becomes `_`.
pub(crate) fn one_name(text: &str) -> String {
    text.chars().map(|c| if c.is_ascii_alphanumeric() || matches!(c, '-' | '.') { c } else { '_' }).collect()
}

/// The revisions in `folder` with extension `ext`, oldest first: `r1.<ext>`, `r2.<ext>` and on.
pub(crate) fn revisions_in(folder: &Path, ext: &str) -> Vec<PathBuf> {
    let mut held: Vec<(u32, PathBuf)> = std::fs::read_dir(folder)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            let number = name.strip_prefix('r')?.strip_suffix(ext)?.strip_suffix('.')?.parse().ok()?;
            Some((number, entry.path()))
        })
        .collect();
    held.sort();
    held.into_iter().map(|(_, path)| path).collect()
}

/// Keep `bytes` as the next revision in `folder` unless the newest holds exactly them. The revisions,
/// oldest first, where one was kept.
pub(crate) fn keep(folder: &Path, bytes: &[u8], ext: &str) -> io::Result<Option<Vec<PathBuf>>> {
    std::fs::create_dir_all(folder)?;
    let held = revisions_in(folder, ext);
    if let Some(newest) = held.last() {
        if std::fs::read(newest)? == bytes {
            return Ok(None);
        }
    }
    let next = folder.join(format!("r{}.{ext}", held.len() + 1));
    let staged = folder.join(format!("r{}.writing", held.len() + 1));
    std::fs::write(&staged, bytes)?;
    std::fs::rename(&staged, next)?;
    Ok(Some(revisions_in(folder, ext)))
}
