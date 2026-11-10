//! `convert_draft_to_document`: the person turns a draft into a Document. The draft's text is written as a
//! page under `.armada/documents/` in the Job's worktree, the draft settles `withdrawn` as `withdraw_draft`
//! settles it, the page is kept as Fleet keeps an agent's own write, and the Job is told, so its next
//! revision goes into the page. `crates/ipc/operations/convert_draft_to_document.toml`.

use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{ConvertDraftToDocument, DraftDocument, DraftHow, SessionRow, WireError};

use super::draft_page::{page_of, slug_of};
use super::drafting::DRAFT_PREFIX;
use crate::daemon::Fleet;

/// A draft the job has no row for. A 404.
const DRAFT_CONVERT_UNKNOWN: &str = "fleet.draft_convert_unknown";
/// A draft already approved, sent back or withdrawn, or being settled by another press. A 409.
const DRAFT_CONVERT_SETTLED: &str = "fleet.draft_convert_settled";
/// A draft that is the say on a workflow, rule or lifecycle change, which has no page to be. A 422.
const DRAFT_CONVERT_PROPOSAL: &str = "fleet.draft_convert_proposal";
/// A job that has not leased a worktree, or has given it back. A 409.
const DRAFT_CONVERT_NO_WORKTREE: &str = "fleet.draft_convert_no_worktree";
/// The page would not write. A 500.
const DRAFT_CONVERT_UNWRITTEN: &str = "fleet.draft_convert_unwritten";

/// Where a Job's documents are in its worktree.
const DOCUMENTS: &str = ".armada/documents";

/// What the Job is sent. `name` is the page's name without `.html`.
pub(crate) fn told(title: &str, name: &str) -> String {
    format!(
        "The owner turned your draft \"{title}\" into the Document {DOCUMENTS}/{name}.html. \
         Revise it there as a Document from now on, and don't propose it as a draft again."
    )
}

/// Write `html` as `<slug>.html` in `worktree`'s documents folder, or `<slug>-2.html` and so on where that
/// name is taken: the file is made only where none is, so a page is never overwritten. `None` where the
/// worktree is not there.
fn written(worktree: &Path, slug: &str, html: &str) -> io::Result<Option<(String, PathBuf)>> {
    if !worktree.is_dir() {
        return Ok(None);
    }
    let folder = worktree.join(DOCUMENTS);
    std::fs::create_dir_all(&folder)?;
    for number in 1.. {
        let name = if number == 1 { slug.to_string() } else { format!("{slug}-{number}") };
        let path = folder.join(format!("{name}.html"));
        match std::fs::OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(mut file) => {
                file.write_all(html.as_bytes())?;
                return Ok(Some((name, path)));
            }
            Err(why) if why.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(why) => return Err(why),
        }
    }
    unreachable!("a number that is free is found")
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
    /// **Nothing is written for a refusal.** The draft is claimed as an answer claims it, so his answer and
    /// this cannot both settle the row. The row settles only once the page is on disk.
    pub(crate) async fn draft_made_a_document(self: Arc<Self>, convert: ConvertDraftToDocument) -> Result<DraftDocument, Refusal> {
        let id = convert.session_id.as_str().trim().to_string();
        let named = convert.draft_id.trim().to_string();
        let row = named.strip_prefix(DRAFT_PREFIX).unwrap_or(&named).trim().to_string();
        let raised = |code: &'static str, why: String, make: fn(WireError) -> Refusal| make(WireError::raised(code, why, self.run_id()));
        let settled = || {
            raised(DRAFT_CONVERT_SETTLED, format!("draft {named} is already settled, so it cannot become a document"), Refusal::IllegalMove)
        };
        let no_worktree = || {
            raised(
                DRAFT_CONVERT_NO_WORKTREE,
                format!("job {id} holds no worktree to put a document in"),
                Refusal::IllegalMove,
            )
        };
        let (_, hosting) = self.session_and_hosting(&id).await?;
        let _settling = self.hosts().settling(&id, &format!("{DRAFT_PREFIX}{row}")).ok_or_else(settled)?;
        let draft = self.rows_of(&id).await?.into_iter().find_map(|one| match one {
            SessionRow::Draft { id: found, title, text, settled, change, .. } if found == row => Some((title, text, settled.is_some(), change)),
            _ => None,
        });
        let Some((title, text, is_settled, change)) = draft else {
            return Err(raised(
                DRAFT_CONVERT_UNKNOWN,
                format!("job {id} has no draft {named}. Name the draft's item id, `draft:<id>`"),
                Refusal::NotFound,
            ));
        };
        if is_settled {
            return Err(settled());
        }
        if change.is_some() {
            return Err(raised(
                DRAFT_CONVERT_PROPOSAL,
                format!("draft {named} proposes a change to be approved, and has no page to become"),
                Refusal::Unacceptable,
            ));
        }
        let served = self.repositories().serving(&hosting.manifest_id).filter(|_| hosting.lease_slot.is_some()).ok_or_else(no_worktree)?;
        let worktree = PathBuf::from(Self::directory_of(&served, &hosting));
        let (slug, html) = (slug_of(&title), page_of(&title, &text));
        let kept = tokio::task::spawn_blocking(move || written(&worktree, &slug, &html))
            .await
            .map_err(|why| io::Error::other(why.to_string()))
            .and_then(|done| done);
        let (name, path) = match kept {
            Ok(Some(page)) => page,
            Ok(None) => return Err(no_worktree()),
            Err(why) => return Err(self.hosted_fault(DRAFT_CONVERT_UNWRITTEN, &format!("the page could not be written: {why}"))),
        };
        let path = path.to_string_lossy().into_owned();
        self.draft_settled(&id, &row, DraftHow::Withdrawn, None).await?;
        self.page_written(&id, path.clone()).await;
        let told = self.told_of_a_conversion(&id, &told(&title, &name)).await.unwrap_or(false);
        Ok(DraftDocument { name, path, told })
    }
}
