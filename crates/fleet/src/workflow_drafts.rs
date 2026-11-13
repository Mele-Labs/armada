//! The draft of a workflow: kept by Fleet while the owner and an agent edit it, and written by Publish.
//! `docs/concepts/workflow.md`, *Drafts and Publish*.
//!
//! **A draft is not a file.** It is a row, keyed by the place and the id (and the repository only where the place is
//! the repository's), so no loader reads it and no Job follows it. Each change keeps the whole text, so the editor undoes
//! by putting an earlier text again and Fleet keeps the list.
//!
//! **Publish is the existing save.** A draft of an existing copy replaces its own place; Armada's, a new file, replaces a
//! copy the scope holds only where the publish says `overwrite`. The loader's refusals pass through unchanged and nothing is written
//! on one; the draft is deleted only once the file is. A draft of one of Armada's own workflows has no scope until then:
//! the copy is made at the scope the owner picks.
//!
//! **`save_workflow` is untouched**, and still writes at once: the Classic Bridge uses it.

use std::path::Path;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{
    DraftBy, DraftChange, Instant, PublishWorkflowDraft, PutWorkflowDraft, SaveWorkflow, WireError, WireValue, WorkflowDraft,
    WorkflowDraftDiscarded, WorkflowId, WorkflowPublished, WorkflowScope,
};
use store::{DraftKey, KeptDraft, NewDraftChange, DRAFT_BY_AGENT};

use crate::authoring::WORKFLOW_UNFIT;
use crate::daemon::Fleet;
use crate::repositories::Served;
use crate::workflow_page::PROJECT_UNNAMED;

/// No draft is kept for that place and id. A 404.
const DRAFT_NONE: &str = "fleet.workflow_draft_none";
/// A draft of Armada's workflow was published without saying where the copy goes. A 422.
const DRAFT_SCOPE_NEEDED: &str = "fleet.workflow_draft_scope_needed";
/// A draft was published to a place other than the one it was made in. A 422.
const DRAFT_SCOPE_DIFFERS: &str = "fleet.workflow_draft_scope_differs";
/// What is published in the draft's place is no longer what the draft began from. A 409.
const DRAFT_STALE: &str = "fleet.workflow_draft_stale";
/// The store would not answer. A 500.
const DRAFT_NOT_KEPT: &str = "fleet.workflow_draft_not_kept";

/// The kind a workflow's draft is kept under; a piece's is its own kind.
pub(crate) const WORKFLOW_KIND: &str = "workflow";

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
    pub(crate) fn draft_not_kept(&self, why: impl std::fmt::Debug) -> Refusal {
        Refusal::Fault(WireError::raised(DRAFT_NOT_KEPT, format!("the draft could not be kept: {why:?}"), self.run_id()))
    }

    fn no_draft(&self, workflow: &str, source: &str) -> Refusal {
        Refusal::NotFound(
            WireError::raised(DRAFT_NONE, format!("no draft of `{workflow}` is kept for {source}"), self.run_id())
                .with_field("workflow_id", WireValue::Str(workflow.to_string())),
        )
    }

    /// The key a draft of `workflow` in a place is kept under, once the place is one this repository can name: a
    /// source that is one of the four, and a project that is named, held, and holds this repository.
    pub(crate) fn workflow_draft_key(
        &self,
        served: &Served,
        workflow: &str,
        source: &str,
        project: Option<&str>,
    ) -> Result<DraftKey, Refusal> {
        let mut key = DraftKey { kind: WORKFLOW_KIND.into(), source: source.into(), item_id: workflow.into(), ..DraftKey::default() };
        match source {
            "armada" | "kit" => {}
            "repository" => key.manifest_id = served.manifest().id().as_str().to_string(),
            "project" => {
                let Some(id) = project else {
                    return Err(Refusal::Unacceptable(
                        WireError::raised(
                            PROJECT_UNNAMED,
                            format!("a draft kept for a project is named by the project too, and `{workflow}` names none"),
                            self.run_id(),
                        )
                        .with_field("workflow_id", WireValue::Str(workflow.to_string())),
                    ));
                };
                self.workflow_scope_held(&WorkflowScope::Project { id: id.to_string() }, served)?;
                key.project = id.to_string();
            }
            other => return Err(self.no_such_definition(workflow, Some(other))),
        }
        Ok(key)
    }

    /// The text published in a place now: the file Fleet holds there, or Armada's carried text. `None` where the place
    /// holds none of that id.
    pub(crate) fn workflow_published_text(&self, served: &Served, workflow: &str, source: &str, project: Option<&str>) -> Option<String> {
        served
            .files()
            .iter()
            .find(|file| {
                file.id().as_str() == workflow
                    && file.source().as_wire() == source
                    && (source != "project" || file.project().map(|held| held.as_str()) == project)
            })
            .map(|file| file.text().to_string())
    }

    /// The place's own scope, which a draft is published to unless it is Armada's.
    fn place_scope(source: &str, project: Option<&str>) -> WorkflowScope {
        match (source, project) {
            ("kit", _) => WorkflowScope::Kit,
            ("project", Some(id)) => WorkflowScope::Project { id: id.to_string() },
            _ => WorkflowScope::Repository,
        }
    }

    /// Why `text` would not load as `workflow`, in the loader's own sentence: what a save checks, and that the text
    /// declares the id the draft is of, since publishing it would write another workflow.
    pub(crate) fn workflow_draft_unfit(&self, served: &Served, workflow: &str, source: &str, project: Option<&str>, text: &str) -> Option<String> {
        let scope = Self::place_scope(source, project);
        if let Err(refusal) = self.check_workflow_file(&SaveWorkflow { scope, definition: text.to_string(), overwrite: true }, served) {
            return Some(refusal.error().message.clone());
        }
        let roster = config::Roster::of(self.models_on());
        let declared = config::WorkflowDef::parse(Path::new("workflow.json"), text.trim(), &roster).ok()?;
        (declared.id().as_str() != workflow)
            .then(|| format!("The `workflow_id` is `{}`, and this is a draft of `{workflow}`", declared.id().as_str()))
    }

    fn workflow_draft_wire(&self, served: &Served, draft: &KeptDraft) -> WorkflowDraft {
        let project = Some(draft.key.project.clone()).filter(|project| !project.is_empty());
        let text = draft.text().to_string();
        WorkflowDraft {
            workflow_id: WorkflowId::carried(draft.key.item_id.clone()),
            source: draft.key.source.clone(),
            unfit: self.workflow_draft_unfit(served, &draft.key.item_id, &draft.key.source, project.as_deref(), &text),
            project,
            text,
            base: draft.base.clone(),
            changes: draft_changes(draft),
            started_at: Instant::carried(draft.started_at.clone()),
            updated_at: Instant::carried(draft.updated_at.clone()),
        }
    }

    pub(crate) async fn workflow_draft_read(
        &self,
        workflow: &WorkflowId,
        source: &str,
        project: Option<&str>,
        served: &Served,
    ) -> Result<WorkflowDraft, Refusal> {
        let key = self.workflow_draft_key(served, workflow.as_str(), source, project)?;
        let kept = self.store().lock().await.workflow_draft(&key).map_err(|why| self.draft_not_kept(why))?;
        let draft = kept.ok_or_else(|| self.no_draft(workflow.as_str(), source))?;
        Ok(self.workflow_draft_wire(served, &draft))
    }

    /// Keep the owner's text: begin the draft from what is published if there is none, and add a change by `you`. **A text the
    /// draft already ends on adds nothing**, so an editor that puts what it holds again does not grow the list.
    pub(crate) async fn workflow_draft_put(&self, put: PutWorkflowDraft, served: &Served) -> Result<WorkflowDraft, Refusal> {
        let (id, project) = (put.workflow_id.as_str(), put.project.as_deref());
        let key = self.workflow_draft_key(served, id, &put.source, project)?;
        let published = self.workflow_published_text(served, id, &put.source, project);
        if put.source == "armada" && published.is_none() {
            return Err(self.no_such_definition(id, Some("armada")));
        }
        let at = self.now();
        let mut store = self.store().lock().await;
        let not_pending = |why: String| {
            Refusal::Unacceptable(
                WireError::raised("fleet.workflow_draft_change_not_pending", why, self.run_id()).with_field("workflow_id", WireValue::Str(id.to_string())),
            )
        };
        // **An agent's put is of one change that is waiting**, which it then marks drafted as the landing does; no other put names a change.
        let agent = match (put.by, put.change) {
            (Some(DraftBy::Agent), Some(change)) => {
                let found = store.workflow_change(change).map_err(|why| self.draft_not_kept(why))?;
                let Some(found) = found else {
                    return Err(not_pending(format!("Fleet holds no change {change} of `{id}` to put in the draft")));
                };
                let place = (found.workflow_id.as_str(), found.source.as_str(), found.project.as_str(), found.manifest_id.as_str());
                let here = (id, put.source.as_str(), project.unwrap_or_default(), if put.source == "repository" { key.manifest_id.as_str() } else { found.manifest_id.as_str() });
                if !found.piece.is_empty() || place != here {
                    return Err(not_pending(format!("change {change} is not a change to `{id}` from {}", put.source)));
                }
                if found.state != store::CHANGE_READY || found.after_text.is_none() {
                    return Err(not_pending(format!("change {change} is not ready, so it has no drafted text to put in the draft")));
                }
                if found.drafted {
                    return Err(not_pending(format!("change {change} is in the draft already")));
                }
                Some(found)
            }
            (Some(DraftBy::Agent), None) => return Err(not_pending(String::from("a put by an agent names the change it is of, and this names none"))),
            (_, Some(change)) => return Err(not_pending(format!("a change is named only by an agent's put, and this put is by you; it names {change}"))),
            (_, None) => None,
        };
        let held = store.workflow_draft(&key).map_err(|why| self.draft_not_kept(why))?;
        if let Some(found) = &agent {
            let change = store::NewDraftChange { made_by: store::DRAFT_BY_AGENT, text: &put.text, summary: found.asked.as_deref(), change: Some(found.id) };
            let kept = store
                .put_draft_change(&key, published.as_deref().unwrap_or(""), &change, at.as_str())
                .map_err(|why| self.draft_not_kept(why))?;
            store.workflow_change_marked_drafted(found.id).map_err(|why| self.draft_not_kept(why))?;
            drop(store);
            return Ok(self.workflow_draft_wire(served, &kept));
        }
        let kept = match held {
            Some(draft) if draft.text() == put.text => draft,
            _ => store
                .put_draft_change(
                    &key,
                    published.as_deref().unwrap_or(""),
                    &NewDraftChange { made_by: store::DRAFT_BY_YOU, text: &put.text, summary: put.summary.as_deref(), change: None },
                    at.as_str(),
                )
                .map_err(|why| self.draft_not_kept(why))?,
        };
        drop(store);
        Ok(self.workflow_draft_wire(served, &kept))
    }

    pub(crate) async fn workflow_draft_discarded(
        &self,
        workflow: WorkflowId,
        source: String,
        project: Option<String>,
        served: &Served,
    ) -> Result<WorkflowDraftDiscarded, Refusal> {
        let key = self.workflow_draft_key(served, workflow.as_str(), &source, project.as_deref())?;
        if !self.store().lock().await.discard_draft(&key).map_err(|why| self.draft_not_kept(why))? {
            return Err(self.no_draft(workflow.as_str(), &source));
        }
        Ok(WorkflowDraftDiscarded { workflow_id: workflow, source, project: Some(key.project).filter(|project| !project.is_empty()) })
    }

    /// Write the draft through the save path, then delete it.
    pub(crate) async fn workflow_draft_published(&self, publish: PublishWorkflowDraft, served: &Served) -> Result<WorkflowPublished, Refusal> {
        let (id, project) = (publish.workflow_id.as_str(), publish.project.as_deref());
        let key = self.workflow_draft_key(served, id, &publish.source, project)?;
        let kept = self.store().lock().await.workflow_draft(&key).map_err(|why| self.draft_not_kept(why))?;
        let draft = kept.ok_or_else(|| self.no_draft(id, &publish.source))?;
        let scope = self.publish_scope(&publish)?;
        // What is published is read again, since a file edited by hand may not have been held yet.
        self.read_workflows_again(served);
        let published = self.workflow_published_text(served, id, &publish.source, project).unwrap_or_default();
        if published != draft.base {
            return Err(Refusal::IllegalMove(
                WireError::raised(
                    DRAFT_STALE,
                    format!(
                        "`{id}` from {} has changed since this draft began from it; nothing was written, and the draft is kept to read against what is there now",
                        publish.source
                    ),
                    self.run_id(),
                )
                .with_field("workflow_id", WireValue::Str(id.to_string())),
            ));
        }
        let text = draft.text().to_string();
        let place = self.publish_place(served, id, &publish.source, project, &scope);
        // What the place holds now, which the first publish into it keeps as version 1.
        let found = self.workflow_published_text(served, id, place.source.as_str(), Some(place.project.as_str()).filter(|project| !project.is_empty()));
        // A draft of Armada's began from the carried text, which is what the copy is then based on.
        let base = if publish.source == "armada" { crate::workflow_versions::carried_base(served, id) } else { None };
        // The loader's refusals pass through the save unchanged; this one it cannot make, since the file's name is the id it declares.
        let roster = config::Roster::of(self.models_on());
        if let Ok(declared) = config::WorkflowDef::parse(Path::new("workflow.json"), text.trim(), &roster) {
            if declared.id().as_str() != id {
                let said = format!("The `workflow_id` is `{}`, and this is a draft of `{id}`", declared.id().as_str());
                return Err(Refusal::Unacceptable(
                    WireError::raised(WORKFLOW_UNFIT, said.clone(), self.run_id()).with_field("reason", WireValue::Str(said)),
                ));
            }
        }
        // Checked before the save is asked for, so a draft that does not load is refused without a write being tried.
        // A draft of an existing copy replaces its own place, which the stale check guards; Armada's is a new file, replacing another only if asked.
        let asked = SaveWorkflow { scope, definition: text.clone(), overwrite: publish.overwrite || publish.source != "armada" };
        self.check_workflow_file(&asked, served)?;
        // **A publish moves no running Job**: each one it would touch is pinned to what its repository runs now, before the save changes that.
        let reach = self.place_reach(id, asked.scope.as_wire(), asked.scope.project(), served)?;
        let pins = self.pins_kept(id, &reach).await?;
        let saved = match self.save_workflow_file(asked, served) {
            Ok(saved) => saved,
            Err(refusal) => {
                self.pins_dropped(&pins).await;
                return Err(refusal);
            }
        };
        // A repository that runs the same text after the save was not touched by it, so its Jobs are not pinned.
        let pinned = self.pins_settled(id, pins, served).await?;
        let version = self.workflow_publish_kept(&place, found.as_deref(), &text, base).await?;
        self.store().lock().await.discard_draft(&key).map_err(|why| self.draft_not_kept(why))?;
        Ok(WorkflowPublished { saved, version, pinned })
    }

    /// Where a publish writes: Armada's draft names it, and every other draft is written where it was made.
    fn publish_scope(&self, publish: &PublishWorkflowDraft) -> Result<WorkflowScope, Refusal> {
        let own = Self::place_scope(&publish.source, publish.project.as_deref());
        match (publish.source.as_str(), &publish.scope) {
            ("armada", Some(scope)) => Ok(scope.clone()),
            ("armada", None) => Err(Refusal::Unacceptable(
                WireError::raised(
                    DRAFT_SCOPE_NEEDED,
                    format!(
                        "`{}` is one of Armada's, which is carried and not written to; say where the copy goes: the repository, Kit or a project",
                        publish.workflow_id.as_str()
                    ),
                    self.run_id(),
                )
                .with_field("workflow_id", WireValue::Str(publish.workflow_id.as_str().to_string())),
            )),
            (_, Some(scope)) if *scope != own => Err(Refusal::Unacceptable(
                WireError::raised(
                    DRAFT_SCOPE_DIFFERS,
                    format!(
                        "this draft of `{}` was made for {}, and is published there; to put it in another place, copy it first",
                        publish.workflow_id.as_str(),
                        publish.source
                    ),
                    self.run_id(),
                )
                .with_field("workflow_id", WireValue::Str(publish.workflow_id.as_str().to_string())),
            )),
            _ => Ok(own),
        }
    }
}

/// A draft's changes as the wire carries them, oldest first.
pub(crate) fn draft_changes(draft: &KeptDraft) -> Vec<DraftChange> {
    draft
        .changes
        .iter()
        .map(|change| DraftChange {
            id: change.id,
            by: if change.made_by == DRAFT_BY_AGENT { DraftBy::Agent } else { DraftBy::You },
            at: Instant::carried(change.at.clone()),
            text: change.text.clone(),
            summary: change.summary.clone(),
            change: change.change,
        })
        .collect()
}
