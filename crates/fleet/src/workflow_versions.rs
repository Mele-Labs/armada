//! The versions a workflow has had in one place, and what a copy of one of Armada's began from. Protocol
//! `docs/practices/protocol/protocol-23-96-a-workflow-with-versions-and-a-base.md`.
//!
//! **A publish writes the log.** The first publish into a place keeps the text that was there as version 1 (`found`) where
//! there was one, and every publish adds its own text (`published`). `save_workflow` and Classic's approve write no version.
//! **Going back is a put**: `revert_workflow` puts a version's text in the draft as a change by `you`, and publishes nothing.
//!
//! **A base is kept only where it is known.** A publish of a draft of Armada's records the carried text the draft began from;
//! a `save_workflow` of Armada's carried text, byte for byte, into a place with no base records it, which is how "Make a copy"
//! saves; a reset records the carried text it wrote. A copy made any other way has none, and nothing is guessed. The `version`
//! is the one Armada's definition declared, and the hash is the text's, so a list can say Armada has moved without reading it.

use std::collections::HashMap;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{BasedOn, PutWorkflowDraft, RevertWorkflow, VersionOrigin, WireError, WireValue, WorkflowDefinition, WorkflowDraft, WorkflowId, WorkflowSaved, WorkflowScope};
use ipc::{Instant, WorkflowSummary, WorkflowVersion, WorkflowVersions};
use sha2::{Digest, Sha256};
use store::{KeptBase, NewVersion, WorkflowPlace, VERSION_FOUND};

use crate::daemon::Fleet;
use crate::repositories::Served;

/// The place has no such version. A 404.
const VERSION_NONE: &str = "fleet.workflow_version_none";
/// The copy has no base: it was made before one was kept, or it was never one of Armada's. A 404.
const BASE_NONE: &str = "fleet.workflow_base_none";
/// A base was asked for without saying which copy's. A 422.
const BASE_OF_NEEDED: &str = "fleet.workflow_base_of_needed";

/// The file a base is served as: it is kept by Fleet, so it is not at a path.
const BASE_FILE: &str = "[the base of this copy]";

/// The sha256 of a text, in hex.
pub(crate) fn text_hash(text: &str) -> String {
    Sha256::digest(text.as_bytes()).iter().map(|byte| format!("{byte:02x}")).collect()
}

/// Armada's carried text of `id` and the `version` it declares, where Armada carries one.
fn carried_of(served: &Served, id: &str) -> Option<(u32, String)> {
    let files = served.files();
    let carried = files.iter().find(|file| file.id().as_str() == id && file.source().as_wire() == "armada")?;
    Some((carried.def().version(), carried.text().to_string()))
}

/// Where a copy in `scope` is kept.
fn scope_place(served: &Served, id: &str, scope: &WorkflowScope) -> WorkflowPlace {
    place_of(served, id, scope.as_wire(), scope.project())
}

fn place_of(served: &Served, id: &str, source: &str, project: Option<&str>) -> WorkflowPlace {
    WorkflowPlace {
        source: source.to_string(),
        project: project.unwrap_or_default().to_string(),
        manifest_id: if source == "repository" { served.manifest().id().as_str().to_string() } else { String::new() },
        item_id: id.to_string(),
    }
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
    fn no_version(&self, workflow: &str, source: &str, version: u32) -> Refusal {
        Refusal::NotFound(
            WireError::raised(VERSION_NONE, format!("`{workflow}` from {source} has no version {version}"), self.run_id())
                .with_field("workflow_id", WireValue::Str(workflow.to_string())),
        )
    }

    /// The versions of `workflow` in a place, newest first.
    pub(crate) async fn workflow_versions_read(
        &self,
        workflow: &WorkflowId,
        source: &str,
        project: Option<&str>,
        served: &Served,
    ) -> Result<WorkflowVersions, Refusal> {
        let place = WorkflowPlace::from(&self.workflow_draft_key(served, workflow.as_str(), source, project)?);
        let kept = self.store().lock().await.workflow_versions(&place).map_err(|why| self.draft_not_kept(why))?;
        let versions = kept
            .into_iter()
            .map(|one| WorkflowVersion {
                version: one.version,
                text: one.text,
                hash: one.hash,
                at: Instant::carried(one.at),
                origin: if one.origin == VERSION_FOUND { VersionOrigin::Found } else { VersionOrigin::Published },
            })
            .collect();
        Ok(WorkflowVersions { versions })
    }

    /// Put a version's text in the draft by the path a put takes, so it begins the draft where there is none. Nothing is published.
    pub(crate) async fn workflow_reverted(&self, revert: RevertWorkflow, served: &Served) -> Result<WorkflowDraft, Refusal> {
        let (id, project) = (revert.workflow_id.as_str(), revert.project.as_deref());
        let place = WorkflowPlace::from(&self.workflow_draft_key(served, id, &revert.source, project)?);
        let kept = self.store().lock().await.workflow_version(&place, revert.version).map_err(|why| self.draft_not_kept(why))?;
        let version = kept.ok_or_else(|| self.no_version(id, &revert.source, revert.version))?;
        self.workflow_draft_put(
            PutWorkflowDraft {
                workflow_id: revert.workflow_id.clone(),
                source: revert.source.clone(),
                project: revert.project.clone(),
                text: version.text,
                summary: Some(format!("Back to version {}", revert.version)),
            },
            served,
        )
        .await
    }

    /// Keep a publish into `place`: `found`, the text that was there, as version 1 where this is the first; `text` as the next;
    /// and, where `base` is the carried text the draft began from, the base. The number `text` took.
    pub(crate) async fn workflow_publish_kept(
        &self,
        served: &Served,
        place: &WorkflowPlace,
        found: Option<&str>,
        text: &str,
        base: Option<&str>,
    ) -> Result<u32, Refusal> {
        let at = self.now();
        let (found_hash, hash) = (found.map(text_hash), text_hash(text));
        let found = found.zip(found_hash.as_deref()).map(|(text, hash)| NewVersion { text, hash, at: at.as_str() });
        let base = base.and_then(|text| {
            carried_of(served, &place.item_id).map(|(version, _)| KeptBase { version, hash: text_hash(text), text: text.to_string() })
        });
        self.store()
            .lock()
            .await
            .record_workflow_publish(place, found.as_ref(), &NewVersion { text, hash: &hash, at: at.as_str() }, base.as_ref())
            .map_err(|why| self.draft_not_kept(why))
    }

    /// The place a publish writes to: a draft's own, or the scope an Armada draft picked.
    pub(crate) fn publish_place(&self, served: &Served, id: &str, source: &str, project: Option<&str>, scope: &WorkflowScope) -> WorkflowPlace {
        if source == "armada" { scope_place(served, id, scope) } else { place_of(served, id, source, project) }
    }

    /// `save_workflow` wrote `saved` from `definition`: where the place has no base and the text is Armada's carried text of that
    /// id byte for byte, which is how a copy is made, keep it as the base. Any other save records none.
    pub(crate) async fn workflow_save_based(&self, served: &Served, scope: &WorkflowScope, definition: &str, saved: &WorkflowSaved) -> Result<(), Refusal> {
        let id = saved.workflow_id.as_str();
        let Some((version, carried)) = carried_of(served, id) else { return Ok(()) };
        if carried != definition {
            return Ok(());
        }
        let place = scope_place(served, id, scope);
        let mut store = self.store().lock().await;
        if store.workflow_base(&place).map_err(|why| self.draft_not_kept(why))?.is_some() {
            return Ok(());
        }
        let base = KeptBase { version, hash: text_hash(&carried), text: carried };
        store.put_workflow_base(&place, &base).map_err(|why| self.draft_not_kept(why))
    }

    /// A copy was deleted, or reset to Armada's text: its versions and base are dropped, and a reset keeps the carried text it wrote
    /// as the new base.
    pub(crate) async fn workflow_copy_dropped(&self, served: &Served, id: &str, source: &str, project: Option<&str>, reset: bool) -> Result<(), Refusal> {
        let base = carried_of(served, id).filter(|_| reset).map(|(version, text)| KeptBase { version, hash: text_hash(&text), text });
        self.store()
            .lock()
            .await
            .drop_workflow_place(&place_of(served, id, source, project), base.as_ref())
            .map_err(|why| self.draft_not_kept(why))
    }

    /// What the copy at `place` began from, as the wire carries it.
    pub(crate) async fn workflow_based_on(&self, served: &Served, id: &str, source: &str, project: Option<&str>) -> Option<BasedOn> {
        let base = self.store().lock().await.workflow_base(&place_of(served, id, source, project)).ok().flatten()?;
        Some(BasedOn { version: base.version, hash: base.hash })
    }

    /// The text a copy began from, in the shape of a definition with `source: "base"`.
    pub(crate) async fn workflow_base_read(&self, served: &Served, workflow: &str, of: Option<&str>, project: Option<&str>) -> Result<WorkflowDefinition, Refusal> {
        let Some(of) = of else {
            return Err(Refusal::Unacceptable(
                WireError::raised(BASE_OF_NEEDED, format!("the base of `{workflow}` is one copy's; say which with `of`: kit, repository or project"), self.run_id())
                    .with_field("workflow_id", WireValue::Str(workflow.to_string())),
            ));
        };
        let key = self.workflow_draft_key(served, workflow, of, project)?;
        let kept = self.store().lock().await.workflow_base(&WorkflowPlace::from(&key)).map_err(|why| self.draft_not_kept(why))?;
        let base = kept.ok_or_else(|| {
            Refusal::NotFound(
                WireError::raised(BASE_NONE, format!("the {of} copy of `{workflow}` has no base: it was made before one was kept, or is not a copy of Armada's"), self.run_id())
                    .with_field("workflow_id", WireValue::Str(workflow.to_string())),
            )
        })?;
        Ok(WorkflowDefinition {
            workflow_id: WorkflowId::carried(workflow),
            source: String::from("base"),
            project: Some(key.project).filter(|project| !project.is_empty()),
            file: String::from(BASE_FILE),
            definition: base.text,
            overridden_by: None,
            based_on: Some(BasedOn { version: base.version, hash: base.hash }),
        })
    }

    /// Say on each of `summaries`, which are `served`'s, what its copy was based on and whether Armada's text of it has moved since.
    pub(crate) fn workflow_bases_marked(
        &self,
        summaries: &mut [WorkflowSummary],
        served: &Served,
        bases: &HashMap<WorkflowPlace, (u32, String)>,
    ) {
        for summary in summaries.iter_mut().filter(|summary| summary.source != "armada") {
            let place = place_of(served, summary.id.as_str(), &summary.source, summary.project.as_deref());
            let Some((version, hash)) = bases.get(&place) else { continue };
            summary.based_on = Some(BasedOn { version: *version, hash: hash.clone() });
            summary.armada_changed = carried_of(served, summary.id.as_str()).is_some_and(|(_, text)| text_hash(&text) != *hash);
        }
    }

    /// Every base Fleet keeps, by place, for a list. **A store that will not answer marks nothing** rather than refusing the list.
    pub(crate) async fn workflow_bases_held(&self) -> HashMap<WorkflowPlace, (u32, String)> {
        let heads = self.store().lock().await.workflow_base_heads().unwrap_or_default();
        heads.into_iter().map(|(place, version, hash)| (place, (version, hash))).collect()
    }
}
