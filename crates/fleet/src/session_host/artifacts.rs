//! `get_job_artifacts`: a Job's sketches, drafts and pages read as one list, each with its revisions and
//! where its newest stands. Reads only, off the ledger rows the three are kept as, their history, his
//! dismissals, the archives, the revisions he wrote and, for a draft, its newest row. `docs/concepts/artifact.md`.

use std::collections::BTreeMap;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{
    ArtifactArchived, ArtifactBy, ArtifactForm, ArtifactRevision, ArtifactState, DraftHow, DraftSettled, Instant, JobArtifact, SessionRow,
    ARTIFACT_WITHDRAWN_BY, DOCUMENT_ASKED, DOCUMENT_ITEM_PREFIX, DOCUMENT_SETTLED, DOCUMENT_WORDS, SKETCH_ASKED, SKETCH_IN_PLAN,
    SKETCH_SETTLED, SKETCH_WITHDRAWN, SKETCH_WORDS,
};
use store::{Holder, KeptArchive, KeptAttachment, KeptAttachmentVersion};

use super::drafting::{DRAFT_PREFIX, DRAFT_ROW};
use super::sketching::TARGET_PREFIX;
use crate::daemon::Fleet;

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
    /// Every artifact the Job holds, newest first by when its newest revision was kept.
    pub(crate) async fn job_artifacts(&self, id: &str) -> Result<Vec<JobArtifact>, Refusal> {
        let id = id.trim();
        let (held, versions, dismissed, archives, his, plan) = {
            let store = self.store().lock().await;
            if store.session(id).map_err(|why| self.ledger_fault(why))?.is_none() {
                return Err(self.ledger_unnamed("a job Fleet knows"));
            }
            let holder = Holder::session(id);
            let held = store.attachments_of(&holder).map_err(|why| self.ledger_fault(why))?;
            let versions = store.attachment_versions(&holder).map_err(|why| self.ledger_fault(why))?;
            let dismissed = store.dismissed_waiting(id).map_err(|why| self.ledger_fault(why))?;
            let archives = store.artifact_archives(id).map_err(|why| self.ledger_fault(why))?;
            let plan = store.session_plan(id).map_err(|why| self.ledger_fault(why))?;
            (held, versions, dismissed, archives, store.owner_revisions(id).map_err(|why| self.ledger_fault(why))?, plan)
        };
        let pending = plan.as_ref().map(|plan| (plan.version, crate::plan_record::pending_sketches(plan)));
        let made: Vec<(ArtifactForm, KeptAttachment)> =
            held.into_iter().filter(|one| one.kind == "artifact").filter_map(|one| Some((form_of(&one)?, one))).collect();
        let drafts: BTreeMap<String, Option<DraftSettled>> = match made.iter().any(|(form, _)| *form == ArtifactForm::Draft) {
            true => self
                .rows_of(id)
                .await?
                .into_iter()
                .filter_map(|row| match row {
                    SessionRow::Draft { id, settled, .. } => Some((id, settled)),
                    _ => None,
                })
                .collect(),
            false => BTreeMap::new(),
        };
        let mut artifacts: Vec<JobArtifact> =
            made.iter().map(|(form, one)| in_plan(archived(*form, one, &versions, &dismissed, &drafts, &archives), one, pending.as_ref())).collect();
        for artifact in &mut artifacts {
            for revision in &mut artifact.revisions {
                if let Some(kept) = his.iter().find(|kept| kept.artifact == artifact.id && kept.revision == revision.number) {
                    *revision = ArtifactRevision { by: ArtifactBy::Owner, at: Instant::carried(kept.at.clone()), ..revision.clone() };
                }
            }
        }
        artifacts.sort_by(|a, b| newest(b).cmp(&newest(a)));
        Ok(artifacts)
    }
}

/// A sketch with the plan version it goes with. **One still asking that the pending plan names waits as the
/// plan's item**, so the plan's one Approve answers it; one that plan's Approve settled names its version.
fn in_plan(artifact: JobArtifact, one: &KeptAttachment, pending: Option<&(u32, Vec<String>)>) -> JobArtifact {
    if artifact.form != ArtifactForm::Sketch {
        return artifact;
    }
    if let Some(version) = one.detail.get(SKETCH_IN_PLAN).and_then(|version| version.parse().ok()) {
        return JobArtifact { in_plan: Some(version), ..artifact };
    }
    match pending {
        Some((version, titles)) if artifact.state == ArtifactState::Asking && artifact.item.is_some() && titles.contains(&artifact.title) => {
            JobArtifact { in_plan: Some(*version), item: Some(format!("plan:{version}")), ..artifact }
        }
        _ => artifact,
    }
}

fn form_of(one: &KeptAttachment) -> Option<ArtifactForm> {
    match one.detail.get("form")?.as_str() {
        "sketch" => Some(ArtifactForm::Sketch),
        "document" => Some(ArtifactForm::Page),
        "draft" => Some(ArtifactForm::Draft),
        _ => None,
    }
}

fn newest(artifact: &JobArtifact) -> Option<&Instant> {
    artifact.revisions.last().map(|one| &one.at)
}

/// One artifact off its row, with its archive. **A dismissal the archive wrote is not his Dismiss**: the
/// artifact reads as it stood before, and names no item while that dismissal stands.
fn archived(
    form: ArtifactForm,
    one: &KeptAttachment,
    versions: &[KeptAttachmentVersion],
    dismissed: &[String],
    drafts: &BTreeMap<String, Option<DraftSettled>>,
    archives: &[KeptArchive],
) -> JobArtifact {
    let Some(archive) = archives.iter().find(|archive| archive.artifact == one.target) else {
        return artifact_of(form, one, versions, dismissed, drafts);
    };
    let his: Vec<String> = dismissed.iter().filter(|item| archive.item.as_ref() != Some(*item)).cloned().collect();
    let mut artifact = artifact_of(form, one, versions, &his, drafts);
    if artifact.item.as_ref().is_some_and(|item| dismissed.contains(item)) {
        artifact.item = None;
    }
    let by = match archive.by.as_str() {
        "owner" => ArtifactBy::Owner,
        _ => ArtifactBy::Agent,
    };
    JobArtifact { archived: Some(ArtifactArchived { by, at: Instant::carried(archive.at.clone()) }), ..artifact }
}

/// One artifact off its row. `drafts` is every draft row's settling by its id.
fn artifact_of(
    form: ArtifactForm,
    one: &KeptAttachment,
    versions: &[KeptAttachmentVersion],
    dismissed: &[String],
    drafts: &BTreeMap<String, Option<DraftSettled>>,
) -> JobArtifact {
    let said = |key: &str| one.detail.get(key).map(String::as_str);
    let gone = |item: &str| dismissed.iter().any(|one| one == item);
    let settled = |how: &str| match how {
        "sent_back" => ArtifactState::SentBack,
        _ => ArtifactState::Approved,
    };
    let (state, words, item) = match form {
        ArtifactForm::Sketch => match said(SKETCH_SETTLED) {
            Some(how) => (settled(how), said(SKETCH_WORDS), None),
            // The Job's withdraw_sketch writes the dismissal his Dismiss writes, and says it was the Job's.
            None if said(ARTIFACT_WITHDRAWN_BY) == Some("agent") => (ArtifactState::Withdrawn, None, None),
            None if gone(&one.target) => (ArtifactState::Dismissed, None, None),
            None if said(SKETCH_WITHDRAWN).is_some() => (ArtifactState::Withdrawn, None, None),
            None if said(SKETCH_ASKED) == Some("true") => (ArtifactState::Asking, None, Some(one.target.clone())),
            None => (ArtifactState::Reading, None, None),
        },
        ArtifactForm::Page => {
            let item = format!("{DOCUMENT_ITEM_PREFIX}{}", one.target);
            match said(DOCUMENT_SETTLED) {
                Some(how) => (settled(how), said(DOCUMENT_WORDS), None),
                None if said(ARTIFACT_WITHDRAWN_BY) == Some("agent") => (ArtifactState::Withdrawn, None, None),
                None if gone(&item) => (ArtifactState::Dismissed, None, None),
                // The Job deleting the page withdraws it, as Needs you reads it (`in_front_of_him`).
                None if said(DOCUMENT_ASKED) == Some("true") && !std::path::Path::new(&one.target).is_file() => (ArtifactState::Withdrawn, None, None),
                None if said(DOCUMENT_ASKED) == Some("true") => (ArtifactState::Asking, None, Some(item)),
                None => (ArtifactState::Reading, None, None),
            }
        }
        ArtifactForm::Draft => {
            let row = said(DRAFT_ROW).unwrap_or_default();
            let item = format!("{DRAFT_PREFIX}{row}");
            match drafts.get(row).cloned().flatten() {
                None => (ArtifactState::Asking, None, Some(item)),
                Some(DraftSettled { how, words, .. }) => {
                    let state = match how {
                        DraftHow::Approved | DraftHow::Fixed => ArtifactState::Approved,
                        DraftHow::SentBack => ArtifactState::SentBack,
                        // His Dismiss keeps the item's id; the Job's withdraw_draft, and a draft of the same title, do not.
                        DraftHow::Withdrawn if gone(&item) => ArtifactState::Dismissed,
                        DraftHow::Withdrawn => ArtifactState::Withdrawn,
                    };
                    return JobArtifact { words, ..whole(form, one, versions, state, None) };
                }
            }
        }
    };
    JobArtifact { words: words.map(str::to_string), ..whole(form, one, versions, state, item) }
}

/// The artifact with no `words`.
fn whole(form: ArtifactForm, one: &KeptAttachment, versions: &[KeptAttachmentVersion], state: ArtifactState, item: Option<String>) -> JobArtifact {
    let title = one.detail.get("title").cloned().unwrap_or_else(|| {
        one.target.strip_prefix(TARGET_PREFIX).or_else(|| one.target.strip_prefix(DRAFT_PREFIX)).unwrap_or(&one.target).to_string()
    });
    let for_line = ipc::ArtifactFor::of_detail(&one.detail);
    JobArtifact { id: one.target.clone(), title, form, revisions: revisions_of(form, one, versions), state, words: None, item, archived: None, for_line, in_plan: None }
}

/// Every revision the row's `revisions` names, oldest first, and one where it names none. The first was
/// kept when the row was; a later one when the row's history first named it, else at the row's last change.
fn revisions_of(form: ArtifactForm, one: &KeptAttachment, versions: &[KeptAttachmentVersion]) -> Vec<ArtifactRevision> {
    let mine: Vec<&KeptAttachmentVersion> =
        versions.iter().filter(|version| version.kind == one.kind && version.manifest_id == one.manifest_id && version.target == one.target).collect();
    (1..=kept(&one.detail).max(1))
        .map(|number| {
            let at = match number {
                1 => one.since.clone(),
                _ => mine.iter().find(|version| kept(&version.detail) >= number).map_or_else(|| one.changed_at.clone(), |version| version.at.clone()),
            };
            ArtifactRevision { number: number as u32, at: Instant::carried(at), by: ArtifactBy::Agent, form }
        })
        .collect()
}

/// How many revisions a row's `revisions` names.
fn kept(detail: &BTreeMap<String, String>) -> usize {
    detail.get("revisions").and_then(|kept| ipc::decode::<Vec<String>>("an artifact's revisions", kept.as_bytes()).ok()).map_or(0, |kept| kept.len())
}
