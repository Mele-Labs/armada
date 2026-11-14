//! `save_artifact_edit`: his Edit of a page's or a draft's words, kept as the artifact's next revision and
//! marked his, with the Job sent what changed. **The Job is told first**, as his answers are, so a send that
//! fails keeps nothing. **An Edit settles nothing**: a page or a draft that asked still asks on his revision.
//! `crates/ipc/operations/save_artifact_edit.toml` is the contract, `docs/concepts/artifact.md` the rules.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::{Refusal, Sessions};
use ipc::{
    ArtifactForm, ArtifactState, AttachmentReport, JobArtifact, SaveArtifactEdit, SendSessionMessage, SessionFact, SessionId, SessionReport,
    SessionRow, WireError, DOCUMENT_ITEM_PREFIX,
};
use store::{AttachmentState, Holder, KeptOwnerRevision};

use super::documents::{folder_of, keep, revisions_named};
use super::drafting::{DRAFT_PREFIX, DRAFT_ROW, PR_DRAFT, PR_DRAFT_TEXT};
use super::page_words::words_changed;
use super::sketching::titled_folder;
use crate::daemon::Fleet;

const EDIT_NOT_NEWEST: &str = "fleet.edit_not_newest";
const EDIT_NOT_WORDS: &str = "fleet.edit_not_words";
const EDIT_SETTLED: &str = "fleet.edit_settled";
const EDIT_ARCHIVED: &str = "fleet.edit_archived";
const EDIT_UNKEPT: &str = "fleet.edit_unkept";
/// The most changes the Job's message lists.
const MOST_CHANGES: usize = 40;
/// The most characters of a block or a line the Job's message quotes.
const MOST_QUOTED: usize = 300;
/// The most cells a draft's line diff fills, past which every line is read as changed.
const MOST_CELLS: usize = 4_000_000;

/// What the Job is sent for his Edit: which revision is his, each change, and what to do now.
fn edited_told(title: &str, revision: u32, changes: &[String], then: &str) -> String {
    let mut lines = vec![format!("He edited \"{title}\": revision {revision} is his, from revision {}.", revision - 1)];
    lines.extend(changes.iter().take(MOST_CHANGES).cloned());
    if changes.len() > MOST_CHANGES {
        lines.push(format!("and {} more", changes.len() - MOST_CHANGES));
    }
    lines.push(then.to_string());
    lines.join("\n")
}

/// `text` cut to [`MOST_QUOTED`] characters.
fn quoted(text: &str) -> String {
    let cut: String = text.trim().chars().take(MOST_QUOTED).collect();
    match cut.len() < text.trim().len() {
        true => format!("{cut}…"),
        false => cut,
    }
}

/// `old` to `new` as `- ` and `+ ` lines, the lines both keep left out.
fn lines_changed(old: &str, new: &str) -> Vec<String> {
    let (old, new): (Vec<&str>, Vec<&str>) = (old.lines().collect(), new.lines().collect());
    let head = old.iter().zip(&new).take_while(|(one, other)| one == other).count();
    let tail = old[head..].iter().rev().zip(new[head..].iter().rev()).take_while(|(one, other)| one == other).count();
    let (a, b) = (&old[head..old.len() - tail], &new[head..new.len() - tail]);
    let gone = |line: &&str| format!("- {}", quoted(line));
    let added = |line: &&str| format!("+ {}", quoted(line));
    if a.len().saturating_mul(b.len()) > MOST_CELLS {
        return a.iter().map(gone).chain(b.iter().map(added)).collect();
    }
    // `common[i * width + j]` is the most lines `a[i..]` and `b[j..]` share in order.
    let width = b.len() + 1;
    let mut common = vec![0u32; (a.len() + 1) * width];
    for i in (0..a.len()).rev() {
        for j in (0..b.len()).rev() {
            common[i * width + j] = match a[i] == b[j] {
                true => common[(i + 1) * width + j + 1] + 1,
                false => common[(i + 1) * width + j].max(common[i * width + j + 1]),
            };
        }
    }
    let (mut i, mut j, mut lines) = (0, 0, Vec::new());
    while i < a.len() && j < b.len() {
        if a[i] == b[j] {
            (i, j) = (i + 1, j + 1);
        } else if common[(i + 1) * width + j] >= common[i * width + j + 1] {
            lines.push(gone(&a[i]));
            i += 1;
        } else {
            lines.push(added(&b[j]));
            j += 1;
        }
    }
    lines.extend(a[i..].iter().map(gone));
    lines.extend(b[j..].iter().map(added));
    lines
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
    /// Keep his Edit as the artifact's next revision, his. An Edit that changes no words keeps and sends nothing.
    pub(crate) async fn artifact_edited(self: Arc<Self>, edit: SaveArtifactEdit) -> Result<JobArtifact, Refusal> {
        let id = edit.session_id.as_str().trim().to_string();
        let artifact = self.artifact_held(&id, &edit.artifact).await?;
        if artifact.form == ArtifactForm::Sketch {
            return Err(self.hosted_refusal(EDIT_NOT_WORDS, "a sketch takes notes, not edits"));
        }
        if artifact.archived.is_some() {
            return Err(self.hosted_refusal(EDIT_ARCHIVED, &format!("`{}` is archived. Restore it to edit it", artifact.title)));
        }
        let newest = artifact.revisions.last().map_or(0, |one| one.number);
        if edit.revision != newest {
            let why = format!("revision {} of `{}` is not the newest: revision {newest} is. Edit revision {newest}", edit.revision, artifact.title);
            return Err(Refusal::IllegalMove(WireError::raised(EDIT_NOT_NEWEST, why, self.run_id())));
        }
        if artifact.form == ArtifactForm::Draft && artifact.state != ArtifactState::Asking {
            let state = match artifact.state {
                ArtifactState::Approved => "approved",
                ArtifactState::SentBack => "sent back",
                ArtifactState::Dismissed => "dismissed",
                _ => "withdrawn",
            };
            return Err(self.hosted_refusal(EDIT_SETTLED, &format!("the draft `{}` is {state} already, and only one still asking takes an Edit", artifact.title)));
        }
        let item = match artifact.form {
            ArtifactForm::Page => format!("{DOCUMENT_ITEM_PREFIX}{}", artifact.id),
            _ => artifact.item.clone().unwrap_or_default(),
        };
        // Held as an answer holds it, so his answer and his Edit cannot cross.
        let _settling = self
            .hosts()
            .settling(&id, &item)
            .ok_or_else(|| self.hosted_refusal(EDIT_SETTLED, &format!("`{}` is being answered now", artifact.title)))?;
        match artifact.form {
            ArtifactForm::Page => Arc::clone(&self).page_edit_kept(&id, &artifact, edit.text).await?,
            _ => Arc::clone(&self).draft_edit_kept(&id, &artifact, &item, edit.text).await?,
        }
        self.published_hosted(&id).await?;
        self.artifact_held(&id, &artifact.id).await
    }

    /// The page he edited, checked for words alone against the revision he read, then written to its file in the
    /// worktree so the agent's next write builds on it, and kept as he wrote it. The turn's end reads the same bytes
    /// as the newest revision, so it keeps nothing more.
    async fn page_edit_kept(self: Arc<Self>, id: &str, artifact: &JobArtifact, html: String) -> Result<(), Refusal> {
        let path = artifact.id.clone();
        let (_, held) = self.document_held(id, &path).await?;
        let newest = held.and_then(|page| revisions_named(&page.detail).pop()).ok_or_else(|| self.unkept("the revision he edited is not kept"))?;
        let base = tokio::task::spawn_blocking(move || std::fs::read_to_string(newest))
            .await
            .ok()
            .and_then(Result::ok)
            .ok_or_else(|| self.unkept("the revision he edited could not be read"))?;
        let number = artifact.revisions.len() as u32;
        let changes = words_changed(&base, &html, number).map_err(|why| self.hosted_refusal(EDIT_NOT_WORDS, &why))?;
        if changes.is_empty() {
            return Ok(());
        }
        let lines: Vec<String> =
            changes.iter().map(|one| format!("- {}: \"{}\" → \"{}\"", one.section, quoted(&one.old), quoted(&one.new))).collect();
        let then = format!("The page at {path} holds his words. Build on them.");
        self.told_of_edit(id, edited_told(&artifact.title, number + 1, &lines, &then)).await?;
        let folder = self.documents_of(id).join(folder_of(&path));
        let (file, bytes) = (path.clone(), html.clone().into_bytes());
        let kept = tokio::task::spawn_blocking(move || {
            std::fs::write(&file, &bytes)?;
            keep(&folder, &bytes, "html")
        })
        .await
        .ok()
        .and_then(Result::ok)
        .flatten()
        .ok_or_else(|| self.unkept("his Edit was sent to the job, and his page could not be written"))?;
        self.his_revision(id, &path, kept.len() as u32).await?;
        // Problems with its sketches were the agent's to hear about on its own revision, and his Edit changed none.
        let _ = self.page_kept(id, &path, &html, &kept).await;
        Ok(())
    }

    /// The draft he edited, kept as its next text, his, and put on its open row, so an Approve applies his words.
    async fn draft_edit_kept(self: Arc<Self>, id: &str, artifact: &JobArtifact, item: &str, text: String) -> Result<(), Refusal> {
        let row_id = item.strip_prefix(DRAFT_PREFIX).unwrap_or(item).to_string();
        let open = self.rows_of(id).await?.into_iter().find(|row| matches!(row, SessionRow::Draft { id, settled: None, .. } if *id == row_id));
        let Some(SessionRow::Draft { id: row, at, title, text: base, before, settled, change, for_line }) = open else {
            return Err(self.hosted_refusal(EDIT_SETTLED, &format!("the draft `{}` is settled already", artifact.title)));
        };
        let text = text.trim().to_string();
        if text == base.trim() {
            return Ok(());
        }
        let lines = lines_changed(base.trim(), &text);
        let number = artifact.revisions.len() as u32;
        self.told_of_edit(id, edited_told(&artifact.title, number + 1, &lines, "Use his text as the draft from now on.")).await?;
        let folder = std::path::Path::new(&self.host().attachments_dir).join("drafts").join(id).join(titled_folder(&title));
        let bytes = text.clone().into_bytes();
        let kept = tokio::task::spawn_blocking(move || keep(&folder, &bytes, "md"))
            .await
            .ok()
            .and_then(Result::ok)
            .flatten()
            .ok_or_else(|| self.unkept("his Edit was sent to the job, and his text could not be kept"))?;
        self.his_revision(id, &artifact.id, kept.len() as u32).await?;
        let names: Vec<String> = kept.iter().map(|one| one.to_string_lossy().into_owned()).collect();
        let (harness, attachment, link) = {
            let store = self.store().lock().await;
            let harness = store.session(id).map_err(|why| self.ledger_fault(why))?.map(|session| session.harness).unwrap_or_default();
            let held = store.attachments_of(&Holder::session(id)).map_err(|why| self.ledger_fault(why))?;
            let attachment = held.iter().find(|one| one.kind == "artifact" && one.target == artifact.id && one.detail.get(DRAFT_ROW) == Some(&row)).cloned();
            let link = held.into_iter().find(|one| one.kind == PR_DRAFT && one.target == row && one.state == AttachmentState::Standing);
            (harness, attachment, link)
        };
        if let Some(mut link) = link {
            link.detail.insert(PR_DRAFT_TEXT.to_string(), text.clone());
            let _ = self.store().lock().await.attach(&link, false);
        }
        self.row_put(id, SessionRow::Draft { id: row, at, title, text, before, settled, change, for_line }).await;
        if let Some(mut attachment) = attachment {
            attachment.detail.insert("revisions".to_string(), ipc::encode(&names).map_err(|_| self.unkept("his Edit was sent to the job, and the draft's revisions could not be named"))?);
            let attachment = AttachmentReport { kind: attachment.kind, target: attachment.target, detail: attachment.detail };
            self.report_session(SessionReport { harness, session_id: SessionId::carried(id), fact: SessionFact::Attached { attachment } }).await?;
        }
        Ok(())
    }

    /// The text of draft row `row` where its newest revision is his, so his Approve says the words he left.
    pub(crate) async fn draft_his(&self, id: &str, row: &str) -> Option<String> {
        let (title, text) = self.rows_of(id).await.ok()?.into_iter().find_map(|one| match one {
            SessionRow::Draft { id, title, text, .. } if id == row => Some((title, text)),
            _ => None,
        })?;
        let target = format!("{DRAFT_PREFIX}{title}");
        let store = self.store().lock().await;
        let held = store.attachments_of(&Holder::session(id)).ok()?;
        let draft = held.iter().find(|one| one.kind == "artifact" && one.target == target && one.detail.get(DRAFT_ROW).map(String::as_str) == Some(row))?;
        let newest = revisions_named(&draft.detail).len() as u32;
        let his = store.owner_revisions(id).ok()?.iter().any(|kept| kept.artifact == target && kept.revision == newest);
        his.then_some(text)
    }

    /// Send the Job his Edit as his message.
    async fn told_of_edit(self: &Arc<Self>, id: &str, text: String) -> Result<(), Refusal> {
        let terminal = self.terminal_session(id).await?;
        let told = SendSessionMessage { session_id: SessionId::carried(id), text, attachments: Vec::new(), mentions: Vec::new(), sketches: Vec::new() };
        Arc::clone(self).send_known(terminal, told).await.map(drop)
    }

    /// Keep that revision `number` of `artifact` is his.
    async fn his_revision(&self, id: &str, artifact: &str, number: u32) -> Result<(), Refusal> {
        let kept = KeptOwnerRevision { artifact: artifact.to_string(), revision: number, at: self.instant().as_str().to_string() };
        self.store().lock().await.keep_owner_revision(id, &kept).map_err(|why| self.ledger_fault(why))
    }

    fn unkept(&self, why: &str) -> Refusal {
        self.hosted_fault(EDIT_UNKEPT, why)
    }
}
