//! One artifact a Job grows. **While an artifact still asks him, anything new the Job shows joins it as a part**:
//! a sketch, a draft, a page that asks, or a part from `add_part`. It never becomes a second artifact or a second
//! Needs you item, and Fleet enforces it where each is kept, never by asking the agent. Once every part that asks
//! is answered the artifact settles, and the next thing the Job shows starts a new one.
//! `docs/concepts/artifact.md`, *Parts*.
//!
//! **How it is kept.** The artifact is a sketch row, `sketch:<title>`: the first thing shown, or a scene-less one
//! Fleet makes where a draft or a page came first. A sketch, a draft or a page that joined carries `in`, that
//! row's id, on its own row, so each keeps every rule it had (a drawing is answered at its `drawn`, a page at its
//! revision, a draft at its row) and raises no item of its own. The parts `add_part` makes, words, takes and
//! buttons, are kept on the artifact's own row in `parts`.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::{Refusal, Sessions};
use ipc::{
    AddPart, AnswerDocument, AnswerPart, AnswerSketch, ArtifactForm, ArtifactPart, ArtifactState, AttachmentReport, DocumentHow, DraftHow, Instant,
    JobArtifact, PartAnswer, PartContent, PartHow, PartKind, PartState, SessionFact, SessionId, SessionRecord, SessionReport, SketchHow, WireError,
    ARTIFACT_IN, ARTIFACT_PARTS, MOST_PARTS, MOST_PART_BYTES, SKETCH_ASKED, SKETCH_SETTLED, SKETCH_WITHDRAWN,
};

use super::artifact_for::with_for;
use super::drafting::DRAFT_PREFIX;
use super::sketching::TARGET_PREFIX;
use crate::daemon::Fleet;

pub(crate) const PART_REFUSED: &str = "fleet.part_refused";
const PART_UNNAMED: &str = "fleet.part_unnamed";
const PART_UNKNOWN: &str = "fleet.part_unknown";
const PART_NOT_OPEN: &str = "fleet.part_not_open";
pub(crate) const PART_REDRAWN: &str = "fleet.part_redrawn";
const PART_ANSWER_EMPTY: &str = "fleet.part_answer_empty";
const PART_HOW: &str = "fleet.part_how";
/// The most ids a refusal names.
const MOST_NAMED: usize = 20;

// ---- what the ledger rows say ----

/// The parts the Job added with `add_part`, as an artifact's row keeps them.
pub(crate) fn stored(detail: &BTreeMap<String, String>) -> Vec<ArtifactPart> {
    detail.get(ARTIFACT_PARTS).and_then(|written| ipc::decode("an artifact's parts", written.as_bytes()).ok()).unwrap_or_default()
}

/// [`store_parts`] for a caller that keeps what it read, so it cannot grow past what it held.
pub(crate) fn keep_stored(detail: &mut BTreeMap<String, String>, parts: &[ArtifactPart]) {
    let _ = store_parts(detail, parts);
}

fn store_parts(detail: &mut BTreeMap<String, String>, parts: &[ArtifactPart]) -> Result<(), String> {
    if parts.is_empty() {
        detail.remove(ARTIFACT_PARTS);
        return Ok(());
    }
    let written = ipc::encode(&parts).map_err(|why| why.to_string())?;
    if written.len() > MOST_PART_BYTES {
        return Err(format!("the parts it holds weigh {} bytes and an artifact keeps at most {MOST_PART_BYTES}. Add less, or put it in a new revision of a part", written.len()));
    }
    detail.insert(ARTIFACT_PARTS.into(), written);
    Ok(())
}

/// Whether a row's detail is a sketch row, which is what an artifact is kept as.
fn is_sketch(one: &ipc::Attachment) -> bool {
    one.kind == "artifact" && one.detail.get("form").map(String::as_str) == Some("sketch")
}

/// The artifact a row joined, where that artifact is held. **A row that names one is a part and raises no item.**
pub(crate) fn joined_of<'a>(attachments: &'a [ipc::Attachment], one: &'a ipc::Attachment) -> Option<&'a str> {
    let root = one.detail.get(ARTIFACT_IN)?.as_str();
    (root != one.target && attachments.iter().any(|held| is_sketch(held) && held.target == root)).then_some(root)
}

/// Whether any draft, page or sketch joined the artifact at `root`.
pub(crate) fn holds_members(attachments: &[ipc::Attachment], root: &str) -> bool {
    attachments.iter().any(|one| one.target != root && joined_of(attachments, one) == Some(root))
}

/// The drawing a sketch row holds is put to him and neither answered nor taken back.
fn drawing_asks(detail: &BTreeMap<String, String>) -> bool {
    detail.get(SKETCH_ASKED).map(String::as_str) == Some("true") && !detail.contains_key(SKETCH_SETTLED) && !detail.contains_key(SKETCH_WITHDRAWN)
}

/// Whether a sketch row has a part still asking: its drawing, or one `add_part` put to him.
pub(crate) fn row_asks(detail: &BTreeMap<String, String>) -> bool {
    drawing_asks(detail) || stored(detail).iter().any(ArtifactPart::is_open_ask)
}

/// One artifact that asks him now, as Needs you raises it.
pub(crate) struct Asking {
    /// The artifact's id, which is the item's.
    pub root: String,
    /// What is still open, each named for the part: a drawing or a draft by its title, a page by its title, a part
    /// by its id.
    pub names: Vec<String>,
    /// How many parts it holds, those answered ones too.
    pub size: usize,
}

/// Where the artifact at `root` stands in `groups`, which it joins at the end where it is not there yet.
fn group_at(groups: &mut Vec<Asking>, root: &str) -> usize {
    match groups.iter().position(|one| one.root == root) {
        Some(at) => at,
        None => {
            groups.push(Asking { root: root.to_string(), names: Vec::new(), size: 0 });
            groups.len() - 1
        }
    }
}

/// Every artifact with a part still asking, in the order its first part was kept. `in_plan` is the titles of the sketches the
/// pending plan names. `open_drafts` is the ledger id of
/// each draft whose row is still open, and `dismissed` what he passed on: both are what a part's own item answers to.
pub(crate) fn asking_groups(attachments: &[ipc::Attachment], in_plan: &[String], open_drafts: &BTreeSet<String>, dismissed: &[String]) -> Vec<Asking> {
    let mut groups: Vec<Asking> = Vec::new();
    let titled = |one: &ipc::Attachment| one.detail.get("title").cloned().unwrap_or_else(|| one.target.trim_start_matches(TARGET_PREFIX).to_string());
    for one in attachments.iter().filter(|one| one.kind == "artifact") {
        let member = joined_of(attachments, one);
        match one.detail.get("form").map(String::as_str) {
            Some("sketch") => {
                let group = group_at(&mut groups, member.unwrap_or(&one.target));
                let has_scene = one.detail.contains_key("scene");
                if has_scene {
                    groups[group].size += 1;
                }
                // A drawing the pending plan names is approved with the plan, so it is no ask of the artifact's own.
                if drawing_asks(&one.detail) && !in_plan.contains(&titled(one)) {
                    groups[group].names.push(titled(one));
                }
                for part in stored(&one.detail) {
                    groups[group].size += 1;
                    if part.is_open_ask() {
                        groups[group].names.push(part.id);
                    }
                }
            }
            Some("draft") if member.is_some() => {
                let group = group_at(&mut groups, member.unwrap_or_default());
                groups[group].size += 1;
                if open_drafts.contains(&one.target) {
                    groups[group].names.push(titled(one));
                }
            }
            Some("document") if member.is_some() => {
                let group = group_at(&mut groups, member.unwrap_or_default());
                groups[group].size += 1;
                if super::document_answers::in_front_of_him(one, dismissed) {
                    groups[group].names.push(titled(one));
                }
            }
            _ => {}
        }
    }
    groups.retain(|one| !one.names.is_empty() && !dismissed.contains(&one.root));
    groups
}

/// The ledger id of each draft whose row is still open, which is what a draft that joined an artifact asks by.
pub(crate) fn open_drafts(attachments: &[ipc::Attachment], rows: &[ipc::SessionRow]) -> BTreeSet<String> {
    let open: BTreeSet<&str> = rows
        .iter()
        .filter_map(|row| match row {
            ipc::SessionRow::Draft { id, settled: None, change: None, .. } => Some(id.as_str()),
            _ => None,
        })
        .collect();
    attachments
        .iter()
        .filter(|one| one.kind == "artifact" && one.detail.get("form").map(String::as_str) == Some("draft"))
        .filter(|one| one.detail.get(super::drafting::DRAFT_ROW).is_some_and(|row| open.contains(row.as_str())))
        .map(|one| one.target.clone())
        .collect()
}

/// The row id of each draft that joined an artifact, which raises no item of its own.
pub(crate) fn joined_drafts(attachments: &[ipc::Attachment]) -> BTreeSet<String> {
    attachments
        .iter()
        .filter(|one| one.kind == "artifact" && one.detail.get("form").map(String::as_str) == Some("draft") && joined_of(attachments, one).is_some())
        .filter_map(|one| one.detail.get(super::drafting::DRAFT_ROW).cloned())
        .collect()
}

/// The item text for an artifact: its title alone where it is one drawing, else the title and what is still open.
pub(crate) fn asking_text(title: &str, asking: &Asking) -> String {
    match (asking.size, asking.names.as_slice()) {
        (0 | 1, _) => title.to_string(),
        (_, [only]) if only == title => title.to_string(),
        _ => format!("{title}: {}", asking.names.join(", ")),
    }
}

// ---- reading an artifact's parts ----

/// A row of the Job's, as one part of the artifact it is in.
pub(crate) fn part_of(artifact: &JobArtifact, one: &store::KeptAttachment, draft_text: Option<&str>) -> Option<ArtifactPart> {
    let said = |key: &str| one.detail.get(key).cloned();
    let state = match artifact.state {
        ArtifactState::Approved => PartState::Approved,
        ArtifactState::SentBack => PartState::SentBack,
        ArtifactState::Withdrawn => PartState::Withdrawn,
        _ => PartState::Open,
    };
    let asks = matches!(artifact.state, ArtifactState::Asking | ArtifactState::Approved | ArtifactState::SentBack | ArtifactState::Withdrawn);
    let (content, drawn) = match artifact.form {
        ArtifactForm::Sketch => (PartContent::Drawing { scene: said("scene")? }, said(ipc::SKETCH_DRAWN).unwrap_or_default()),
        ArtifactForm::Draft => (PartContent::Words { text: draft_text?.to_string() }, said(super::drafting::DRAFT_ROW).unwrap_or_default()),
        ArtifactForm::Page => (PartContent::Page { path: one.target.clone() }, said(ipc::DOCUMENT_REVISION).unwrap_or_default()),
    };
    let answered = matches!(state, PartState::Approved | PartState::SentBack);
    Some(ArtifactPart {
        id: artifact.id.clone(),
        content,
        asks,
        state,
        revision: artifact.revisions.len() as u32,
        drawn,
        added: Instant::carried(one.since.clone()),
        answer: answered.then(|| PartAnswer { words: artifact.words.clone(), ..PartAnswer::default() }),
    })
}

/// `form` for a client that reads no parts: the first part's kind.
fn form_of_first(part: &ArtifactPart) -> ArtifactForm {
    match part.content.kind() {
        PartKind::Drawing | PartKind::Takes => ArtifactForm::Sketch,
        PartKind::Words => ArtifactForm::Draft,
        PartKind::Buttons | PartKind::Page => ArtifactForm::Page,
    }
}

/// The artifacts folded: each sketch, draft or page that joined one is a part of it and no artifact of its own, and
/// each artifact's state is read from all its parts. **One that holds no other part is left as it was read.**
/// `rows` are the rows `artifacts` were read from, in the same order, and `dismissed` what he passed on.
pub(crate) fn folded(artifacts: Vec<JobArtifact>, rows: &[&store::KeptAttachment], dismissed: &[String]) -> Vec<JobArtifact> {
    let ids: Vec<String> = artifacts.iter().map(|one| one.id.clone()).collect();
    let mut members: Vec<(String, JobArtifact)> = Vec::new();
    let mut roots: Vec<(JobArtifact, &store::KeptAttachment)> = Vec::new();
    for (artifact, one) in artifacts.into_iter().zip(rows) {
        let root = one.detail.get(ARTIFACT_IN).filter(|root| **root != artifact.id && ids.contains(root) && root.starts_with(TARGET_PREFIX));
        match root {
            Some(root) => members.push((root.clone(), artifact)),
            None => roots.push((artifact, one)),
        }
    }
    roots
        .into_iter()
        .map(|(mut root, one)| {
            let theirs: Vec<ArtifactPart> = members.iter().filter(|(of, _)| *of == root.id).flat_map(|(_, member)| member.parts.clone()).collect();
            let added = stored(&one.detail);
            if theirs.is_empty() && added.is_empty() {
                return root;
            }
            root.parts.extend(theirs);
            root.parts.extend(added);
            root.parts.sort_by(|a, b| a.added.as_str().cmp(b.added.as_str()));
            if let Some(first) = root.parts.first() {
                root.form = form_of_first(first);
            }
            let (state, words, item) = group_state(&root, dismissed);
            JobArtifact { state, words, item, ..root }
        })
        .collect()
}

/// Where an artifact stands once all its parts are read. It asks while any part that asks is open; then he may have
/// dismissed it; and where every part that asks is answered it is sent back if any was, else approved.
fn group_state(root: &JobArtifact, dismissed: &[String]) -> (ArtifactState, Option<String>, Option<String>) {
    let open = root.parts.iter().any(ArtifactPart::is_open_ask);
    let answered: Vec<&ArtifactPart> =
        root.parts.iter().filter(|part| part.asks && !matches!(part.state, PartState::Open | PartState::Withdrawn)).collect();
    // The dismissal an archive wrote is not his Dismiss.
    let his = root.archived.is_none() && dismissed.contains(&root.id);
    if his {
        return (ArtifactState::Dismissed, None, None);
    }
    if open {
        return (ArtifactState::Asking, None, root.archived.is_none().then(|| root.id.clone()));
    }
    if answered.is_empty() {
        // The Job took every ask back: nothing was answered, so the artifact reads as withdrawn.
        if root.parts.iter().any(|part| part.state == PartState::Withdrawn) {
            return (ArtifactState::Withdrawn, None, None);
        }
        return (root.state, root.words.clone(), None);
    }
    let state = match answered.iter().any(|part| part.state == PartState::SentBack) {
        true => ArtifactState::SentBack,
        false => ArtifactState::Approved,
    };
    let said: Vec<(&str, &str)> = answered.iter().filter_map(|part| Some((part.id.as_str(), part.answer.as_ref()?.words.as_deref()?))).collect();
    let words = match said.as_slice() {
        [] => None,
        [(_, only)] => Some((*only).to_string()),
        many => Some(many.iter().map(|(id, words)| format!("{id}: {words}")).collect::<Vec<_>>().join("\n")),
    };
    (state, words, None)
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
    /// **Where a thing the Job now shows goes**, `target` being the ledger id it will be kept under: the id of the
    /// artifact still asking him, which it joins as a part, else `None`, and it starts alone.
    /// Where the open one is a lone draft or page, which cannot hold parts, a sketch becomes the artifact and takes
    /// it in, and anything else puts a scene-less artifact over both. A thing already a part of one keeps it.
    pub(crate) async fn artifact_joined(&self, id: &str, harness: &str, target: &str, title: &str, is_sketch: bool) -> Result<Option<String>, Refusal> {
        let open = self
            .job_artifacts(id)
            .await?
            .into_iter()
            .find(|one| one.state == ArtifactState::Asking && one.archived.is_none() && one.id != target && !one.parts.iter().any(|part| part.id == target));
        let Some(open) = open else { return Ok(None) };
        if open.id.starts_with(TARGET_PREFIX) {
            return Ok(Some(open.id));
        }
        let root = match is_sketch {
            true => target.to_string(),
            false => self.container_made(id, harness, title).await?,
        };
        self.row_joined(id, harness, &open.id, &root).await?;
        Ok((!is_sketch).then_some(root))
    }

    /// A scene-less artifact titled `title`, which holds parts and is no drawing. Its target is unique among the Job's.
    async fn container_made(&self, id: &str, harness: &str, title: &str) -> Result<String, Refusal> {
        let held = self.held_rows(id).await?;
        let mut target = format!("{TARGET_PREFIX}{title}");
        let mut again = 1;
        while held.iter().any(|one| one.target == target) {
            again += 1;
            target = format!("{TARGET_PREFIX}{title} ({again})");
        }
        let detail = [("form", "sketch"), ("by", "agent"), ("title", title)].into_iter().map(|(key, word)| (key.to_string(), word.to_string())).collect();
        self.report_session(SessionReport {
            harness: harness.to_string(),
            session_id: SessionId::carried(id),
            fact: SessionFact::Attached { attachment: AttachmentReport { kind: String::from("artifact"), target: target.clone(), detail } },
        })
        .await?;
        Ok(target)
    }

    /// The row at `target` written again as a part of the artifact `root`.
    async fn row_joined(&self, id: &str, harness: &str, target: &str, root: &str) -> Result<(), Refusal> {
        let Some(mut row) = self.held_rows(id).await?.into_iter().find(|one| one.kind == "artifact" && one.target == target) else { return Ok(()) };
        row.detail.insert(ARTIFACT_IN.into(), root.to_string());
        self.report_session(SessionReport {
            harness: harness.to_string(),
            session_id: SessionId::carried(id),
            fact: SessionFact::Attached { attachment: AttachmentReport { kind: row.kind, target: row.target, detail: row.detail } },
        })
        .await?;
        Ok(())
    }

    /// His answer to a part put again since he read it.
    pub(crate) fn part_redrawn(&self, part: &str) -> Refusal {
        Refusal::IllegalMove(WireError::raised(
            PART_REDRAWN,
            format!("`{part}` is not the revision he read: the job has put it again since"),
            self.run_id(),
        ))
    }

    async fn held_rows(&self, id: &str) -> Result<Vec<ipc::Attachment>, Refusal> {
        let store = self.store().lock().await;
        let session = store.session(id).map_err(|why| self.ledger_fault(why))?.ok_or_else(|| self.hosted_refusal(PART_UNNAMED, "a job Fleet knows"))?;
        Ok(self.ledger_row(&store, &session)?.attachments)
    }

    /// `add_part`: one more part on the artifact that asks him, or on a new one where none does. A part put under
    /// an id the Job used before is that part's next revision, and asks again.
    pub(crate) async fn part_added(&self, caller: Option<api::Caller>, add: AddPart) -> Result<SessionRecord, Refusal> {
        let checked = add.checked().map_err(|why| self.hosted_refusal(PART_REFUSED, &why))?;
        let said = add.for_checked().map_err(|why| self.hosted_refusal(PART_REFUSED, &why))?;
        let id = caller
            .and_then(|caller| self.session_holding(&caller))
            .or_else(|| add.session_id.map(|named| named.as_str().trim().to_string()))
            .filter(|id| !id.is_empty())
            .ok_or_else(|| self.hosted_refusal(PART_UNNAMED, "a part needs a job to go on, and this call places none"))?;
        let harness = {
            let store = self.store().lock().await;
            store.session(&id).map_err(|why| self.ledger_fault(why))?.ok_or_else(|| self.hosted_refusal(PART_UNNAMED, "a job Fleet knows"))?.harness
        };
        let held = self.held_rows(&id).await?;
        // A part the Job put before is revised where it stands.
        let earlier = held.iter().filter(|one| is_sketch(one)).find(|one| stored(&one.detail).iter().any(|part| part.id == checked.id)).map(|one| one.target.clone());
        let target = match earlier {
            Some(target) => target,
            None => match self.artifact_joined(&id, &harness, "", &checked.title, false).await? {
                Some(root) => root,
                None => self.container_made(&id, &harness, &checked.title).await?,
            },
        };
        self.part_put(&id, &harness, &target, &checked, said).await
    }

    /// `checked` put on the artifact at `target`, kept, and the artifact asked again where the part asks.
    pub(crate) async fn part_put(&self, id: &str, harness: &str, target: &str, checked: &ipc::CheckedPart, said: Option<ipc::ArtifactFor>) -> Result<SessionRecord, Refusal> {
        let held = self.held_rows(id).await?;
        let row = held.into_iter().find(|one| is_sketch(one) && one.target == target).ok_or_else(|| self.hosted_refusal(PART_UNKNOWN, "an artifact the job holds"))?;
        let mut detail = row.detail;
        let mut parts = stored(&detail);
        let now = self.instant();
        let drawn = self.row_id(id);
        match parts.iter().position(|part| part.id == checked.id) {
            Some(at) => {
                let was = parts.remove(at);
                if was.content.kind() != checked.content.kind() {
                    return Err(self.hosted_refusal(
                        PART_REFUSED,
                        &format!("`{}` is a {} part, and a part keeps its kind. Use another id for a {} part", was.id, was.content.kind().word(), checked.content.kind().word()),
                    ));
                }
                let part = ArtifactPart { content: checked.content.clone(), asks: checked.asks, state: PartState::Open, revision: was.revision + 1, drawn, answer: None, ..was };
                parts.insert(at, part.clone());
                part
            }
            None => {
                if parts.len() + 1 >= MOST_PARTS {
                    return Err(self.hosted_refusal(PART_REFUSED, &format!("an artifact holds at most {MOST_PARTS} parts. Settle this one, or put the part in a new artifact once it is")));
                }
                let part = ArtifactPart { id: checked.id.clone(), content: checked.content.clone(), asks: checked.asks, state: PartState::Open, revision: 1, drawn, added: now.clone(), answer: None };
                parts.push(part.clone());
                part
            }
        };
        store_parts(&mut detail, &parts).map_err(|why| self.hosted_refusal(PART_REFUSED, &why))?;
        if said.is_some() || !detail.contains_key(ipc::ARTIFACT_FOR) {
            with_for(&mut detail, self.for_line(id, said).await.as_ref());
        }
        // An artifact he dismissed asks again once the Job puts something on it.
        self.store().lock().await.restore_waiting(id, target).map_err(|why| self.ledger_fault(why))?;
        self.report_session(SessionReport {
            harness: harness.to_string(),
            session_id: SessionId::carried(id),
            fact: SessionFact::Attached { attachment: AttachmentReport { kind: row.kind, target: row.target, detail } },
        })
        .await
    }

    /// `answer_part`: he answers one asking part of an artifact on its own, and the Job is told as his message. The
    /// artifact settles once every part that asks is answered.
    pub(crate) async fn part_answered(self: Arc<Self>, said: AnswerPart) -> Result<JobArtifact, Refusal> {
        let id = said.session_id.as_str().trim().to_string();
        let artifact = self.artifact_held(&id, &said.artifact).await?;
        let named = said.part.trim();
        let Some(part) = artifact.parts.iter().find(|part| part.id == named).cloned() else {
            let ids: Vec<&str> = artifact.parts.iter().take(MOST_NAMED).map(|part| part.id.as_str()).collect();
            let holds = if ids.is_empty() { String::from("none") } else { ids.join(", ") };
            return Err(self.hosted_refusal(PART_UNKNOWN, &format!("`{}` holds no part `{named}`. Parts it holds: {holds}", artifact.title)));
        };
        if artifact.archived.is_some() {
            return Err(self.hosted_refusal(super::page_actions::PAGE_ARCHIVED, &format!("`{}` is archived. Restore it to answer it", artifact.title)));
        }
        let not_open = |why: String| self.hosted_refusal(PART_NOT_OPEN, &why);
        if !part.asks {
            return Err(not_open(format!("`{}` was not put to him: a part that only informs, or acts when pressed, has nothing to answer", part.id)));
        }
        if part.state != PartState::Open {
            return Err(not_open(format!("`{}` was {} already", part.id, part.state.word().replace('_', " "))));
        }
        if part.drawn != said.drawn {
            return Err(self.part_redrawn(&part.id));
        }
        let words = said.words.as_deref().map(str::trim).filter(|words| !words.is_empty()).map(str::to_string);
        let wrong = |why: &str| self.hosted_refusal(PART_HOW, why);
        if said.how == PartHow::SentBack && words.is_none() {
            return Err(self.hosted_refusal(PART_ANSWER_EMPTY, "a part is sent back with a note saying what to change"));
        }
        let kind = part.content.kind();
        if (said.how == PartHow::Picked) != (kind == PartKind::Takes) {
            return Err(wrong(match kind {
                PartKind::Takes => "a takes part is answered by a vote, never an Approve or a note: send a pick, with how `picked` and the take he chose",
                _ => "only a takes part is answered with a pick. Words, a drawing and a page are approved or sent back",
            }));
        }
        if let PartContent::Takes { form, .. } = &part.content {
            if *form != ipc::VoteForm::Pick {
                return Err(wrong(&format!("`{}` is a {} vote, which carries more than one take: cast it with cast_page_vote", part.id, form.word())));
            }
        }
        let sketch_how = if said.how == PartHow::Approved { SketchHow::Approved } else { SketchHow::SentBack };
        match (&part.content, kind) {
            (PartContent::Drawing { .. }, _) => {
                let answer = AnswerSketch { session_id: said.session_id.clone(), sketch: part.id.clone(), drawn: part.drawn.clone(), how: sketch_how, words };
                Arc::clone(&self).sketch_answered(answer).await?;
            }
            (PartContent::Page { path }, _) => {
                let how = if said.how == PartHow::Approved { DocumentHow::Approved } else { DocumentHow::SentBack };
                let answer = AnswerDocument { session_id: said.session_id.clone(), document: path.clone(), revision: part.drawn.clone(), how, words };
                Arc::clone(&self).document_answered(answer).await?;
            }
            (PartContent::Words { .. }, _) if part.id.starts_with(DRAFT_PREFIX) => {
                Arc::clone(&self).member_draft_answered(&id, &part, said.how, words).await?;
            }
            (PartContent::Words { .. }, _) => {
                Arc::clone(&self).stored_words_answered(&id, &artifact, &part, said.how, words).await?;
            }
            (PartContent::Takes { .. }, _) => {
                let Some(take) = said.take.as_deref().map(str::trim).filter(|take| !take.is_empty()) else {
                    return Err(self.hosted_refusal(PART_ANSWER_EMPTY, "a pick names the take: `take`"));
                };
                let result = ipc::VoteResult::Pick { take: take.to_string(), notes: said.notes.clone() };
                Arc::clone(&self).part_voted(&id, &artifact, &part, part.revision, result, words).await?;
            }
            (PartContent::Buttons { .. }, _) => {
                return Err(not_open(format!("`{}` holds buttons, which act when he presses them", part.id)));
            }
        }
        self.artifact_held(&id, &artifact.id).await
    }

    /// Every part of the artifact still asking, answered one after another the same way: his Approve of the artifact,
    /// or his note sent back on each. **A takes part is answered with a pick, so an artifact that holds one is not
    /// approved whole.** Each part is told to the Job as its own answer.
    pub(crate) async fn artifact_answered_all(self: Arc<Self>, id: &str, root: &str, approve: bool, words: Option<String>) -> Result<JobArtifact, Refusal> {
        let artifact = self.artifact_held(id, root).await?;
        let open: Vec<ArtifactPart> = artifact.parts.iter().filter(|part| part.is_open_ask()).cloned().collect();
        if let Some(takes) = open.iter().find(|part| part.content.kind() == PartKind::Takes) {
            return Err(self.hosted_refusal(PART_HOW, &format!("`{}` holds a pick, `{}`, which is a vote: send a pick on it with the take he chose, which no Approve of the whole stands in for", artifact.title, takes.id)));
        }
        for part in open {
            let said = AnswerPart {
                session_id: SessionId::carried(id),
                artifact: artifact.id.clone(),
                part: part.id,
                drawn: part.drawn,
                how: if approve { PartHow::Approved } else { PartHow::SentBack },
                take: None,
                words: words.clone(),
                notes: BTreeMap::new(),
            };
            Box::pin(Arc::clone(&self).part_answered(said)).await?;
        }
        self.artifact_held(id, root).await
    }

    /// A draft that joined an artifact, approved or sent back as its own card would, the Job told first.
    async fn member_draft_answered(self: Arc<Self>, id: &str, part: &ArtifactPart, how: PartHow, words: Option<String>) -> Result<(), Refusal> {
        let row = part.drawn.clone();
        let title = part.id.strip_prefix(DRAFT_PREFIX).unwrap_or(&part.id).to_string();
        let _settling = self
            .hosts()
            .settling(id, &format!("{DRAFT_PREFIX}{row}"))
            .ok_or_else(|| self.hosted_refusal(PART_NOT_OPEN, &format!("`{}` is being answered already", part.id)))?;
        let open = self.rows_of(id).await?.iter().any(|one| matches!(one, ipc::SessionRow::Draft { id, settled: None, .. } if *id == row));
        if !open {
            return Err(self.hosted_refusal(PART_NOT_OPEN, &format!("`{}` was answered already", part.id)));
        }
        let (body, settled) = match how {
            PartHow::Approved => {
                let his = Box::pin(self.draft_his(id, &row)).await;
                let body = match his {
                    Some(his) => super::drafting::approved_his(&title, &his, words.as_deref()),
                    None => super::drafting::approved(&title, words.as_deref()),
                };
                (body, DraftHow::Approved)
            }
            _ => (super::drafting::sent_back(&title, words.as_deref().unwrap_or_default()), DraftHow::SentBack),
        };
        self.told_as_him(id, body).await?;
        self.draft_settled(id, &row, settled, words).await?;
        Ok(())
    }

    /// A words part, approved or sent back, the Job told first and naming the artifact and the part.
    async fn stored_words_answered(self: Arc<Self>, id: &str, artifact: &JobArtifact, part: &ArtifactPart, how: PartHow, words: Option<String>) -> Result<(), Refusal> {
        let _settling = self
            .hosts()
            .settling(id, &format!("{}#{}", artifact.id, part.id))
            .ok_or_else(|| self.hosted_refusal(PART_NOT_OPEN, &format!("`{}` is being answered already", part.id)))?;
        let mut body = match how {
            PartHow::Approved => format!("Approved part \"{}\" of \"{}\".\nGo on from it as written.", part.id, artifact.title),
            _ => format!(
                "Sent back part \"{}\" of \"{}\".\n{}\nChange it and put it again with add_part under the same id.",
                part.id,
                artifact.title,
                words.as_deref().unwrap_or_default()
            ),
        };
        if let (PartHow::Approved, Some(words)) = (how, words.as_deref()) {
            body.push('\n');
            body.push_str(words);
        }
        self.told_as_him(id, body).await?;
        let state = if how == PartHow::Approved { PartState::Approved } else { PartState::SentBack };
        let answer = PartAnswer { words, at: Some(self.instant()), ..PartAnswer::default() };
        self.stored_part_settled(id, &artifact.id, &part.id, &part.drawn, state, answer).await?;
        self.published_hosted(id).await?;
        Ok(())
    }

    /// Keep how a stored part was answered, where it is still the revision he read. `false` where it was put again.
    pub(crate) async fn stored_part_settled(&self, id: &str, root: &str, part: &str, drawn: &str, state: PartState, answer: PartAnswer) -> Result<bool, Refusal> {
        let held = self.held_rows(id).await?;
        let Some(row) = held.into_iter().find(|one| is_sketch(one) && one.target == root) else { return Ok(false) };
        let harness = {
            let store = self.store().lock().await;
            store.session(id).map_err(|why| self.ledger_fault(why))?.map(|session| session.harness).unwrap_or_default()
        };
        let mut detail = row.detail;
        let mut parts = stored(&detail);
        let Some(one) = parts.iter_mut().find(|one| one.id == part && one.drawn == drawn) else { return Ok(false) };
        one.state = state;
        one.answer = Some(answer);
        store_parts(&mut detail, &parts).map_err(|why| self.hosted_refusal(PART_REFUSED, &why))?;
        self.report_session(SessionReport {
            harness,
            session_id: SessionId::carried(id),
            fact: SessionFact::Attached { attachment: AttachmentReport { kind: row.kind, target: row.target, detail } },
        })
        .await?;
        Ok(true)
    }

    /// **The Job archived an artifact that still asks him**: every ask on it is taken back, as `withdraw_sketch`,
    /// `withdraw_draft` and `withdraw_document` take theirs, so it leaves his list marked the Job's own and reads
    /// `withdrawn`. Nothing is sent to the Job, which made the call. His own Archive never comes here.
    pub(crate) async fn asks_withdrawn(&self, id: &str, artifact: &JobArtifact) -> Result<(), Refusal> {
        let rows = self.held_rows(id).await?;
        let drafts = open_drafts(&rows, &self.rows_of(id).await?);
        let dismissed = self.store().lock().await.dismissed_waiting(id).map_err(|why| self.ledger_fault(why))?;
        let mine = |one: &&ipc::Attachment| one.kind == "artifact" && (one.target == artifact.id || joined_of(&rows, one) == Some(artifact.id.as_str()));
        for one in rows.iter().filter(mine) {
            match one.detail.get("form").map(String::as_str) {
                Some("sketch") => {
                    if drawing_asks(&one.detail) {
                        self.sketch_withdrawn_mark(id, &one.target, true).await;
                    }
                    for part in stored(&one.detail).into_iter().filter(ArtifactPart::is_open_ask) {
                        let answer = PartAnswer { at: Some(self.instant()), ..PartAnswer::default() };
                        self.stored_part_settled(id, &one.target, &part.id, &part.drawn, PartState::Withdrawn, answer).await?;
                    }
                }
                Some("draft") if drafts.contains(&one.target) => {
                    let Some(row) = one.detail.get(super::drafting::DRAFT_ROW) else { continue };
                    // Held as his answer holds it: one being answered now settles without this.
                    if let Some(_settling) = self.hosts().settling(id, &format!("{DRAFT_PREFIX}{row}")) {
                        self.draft_settled(id, row, DraftHow::Withdrawn, None).await?;
                    }
                }
                Some("document") if super::document_answers::in_front_of_him(one, &dismissed) => {
                    self.store()
                        .lock()
                        .await
                        .dismiss_waiting(id, &format!("{}{}", ipc::DOCUMENT_ITEM_PREFIX, one.target))
                        .map_err(|why| self.ledger_fault(why))?;
                    let (harness, page) = self.document_held(id, &one.target).await?;
                    if let Some(mut page) = page {
                        page.detail.insert(ipc::ARTIFACT_WITHDRAWN_BY.into(), "agent".into());
                        let attachment = AttachmentReport { kind: page.kind, target: page.target, detail: page.detail };
                        self.report_session(SessionReport { harness, session_id: SessionId::carried(id), fact: SessionFact::Attached { attachment } }).await?;
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }
}
