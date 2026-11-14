//! Settings' routes, each a request off the main thread on the port the Fleet connection read, or answered at
//! once by the walk's and the tests' mock. `crates/ipc/operations/get_settings.toml`, `save_settings.toml`,
//! `get_values.toml` and `edit_project.toml`.

use std::collections::BTreeMap;

use gpui::{App, AppContext, Entity, Task, Window};
use ipc::{EditProject, Projects, ResolvedValues, SaveSettings, SettingValue, SettingsList, ValuesSet};
use serde::de::DeserializeOwned;

use super::mock::MockFleet;
use crate::fleet::FleetConnection;

/// Why a save did not take: Fleet's own sentence, and the key it named where it named one
/// (`fleet.unacceptable_settings` carries it in `fields.key`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Refused {
    pub key: Option<String>,
    pub said: String,
    /// Fleet's code for it, where Fleet answered with one, such as `fleet.workflow_exists`.
    pub code: Option<String>,
}

impl Refused {
    pub fn said(said: impl Into<String>) -> Refused {
        Refused { key: None, said: said.into(), code: None }
    }

    pub fn is(&self, code: &str) -> bool {
        self.code.as_deref() == Some(code)
    }
}

/// What a preference is saved as: a word or the text a page wrote, or a switch.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Put {
    Text(String),
    Switch(bool),
}

pub enum Wire {
    Live(Entity<FleetConnection>),
    Mock(Box<MockFleet>),
}

enum Ask {
    Get(String),
    Post(String, String),
    Put(String, String),
    Delete(String),
}

fn request<T: DeserializeOwned + Send + 'static>(port: u16, ask: Ask, what: &'static str, cx: &App) -> Task<Result<T, Refused>> {
    cx.background_spawn(async move {
        let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().map_err(|why| Refused::said(why.to_string()))?;
        let (status, bytes) = match ask {
            Ask::Get(path) => runtime.block_on(crate::transport::get(port, &path)),
            Ask::Post(path, body) => runtime.block_on(crate::transport::post(port, &path, body)),
            Ask::Put(path, body) => runtime.block_on(crate::transport::put(port, &path, body)),
            Ask::Delete(path) => runtime.block_on(crate::transport::delete(port, &path)),
        }
        .map_err(Refused::said)?;
        if !(200..300).contains(&status) {
            return Err(match ipc::decode::<ipc::WireError>("WireError", &bytes) {
                Ok(error) => {
                    let key = match error.fields.get("key") {
                        Some(ipc::WireValue::Str(key)) => Some(key.clone()),
                        _ => None,
                    };
                    Refused { key, said: error.message, code: Some(error.code) }
                }
                Err(_) => Refused::said(format!("Fleet answered {status}")),
            });
        }
        ipc::decode::<T>(what, &bytes).map_err(|why| Refused::said(why.to_string()))
    })
}

fn body(value: &impl serde::Serialize) -> String {
    serde_json::to_string(value).unwrap_or_default()
}

/// A route under the Manifest it acts in (`?manifest_id=`, as Classic's `manifestNamed` adds it).
fn scoped(route: &str, manifest: &str) -> String {
    let join = if route.contains('?') { '&' } else { '?' };
    format!("{route}{join}manifest_id={}", crate::link::percent_encoded(manifest))
}

/// A workflow draft's query: its id and place, and the project where the place is one.
fn draft_query(id: &str, source: &str, project: Option<&str>) -> String {
    let project = project.map(|project| format!("&project={}", crate::link::percent_encoded(project))).unwrap_or_default();
    format!("workflow_id={}&source={}{project}", crate::link::percent_encoded(id), crate::link::percent_encoded(source))
}

/// A piece draft's query: its kind and id, and the scope's id as `within` where it is one workflow's, none for everywhere.
fn piece_query(kind: ipc::LifecyclePieceKind, id: &str, scope: &ipc::PieceScope) -> String {
    let within = match scope {
        ipc::PieceScope::Within { id } => format!("&within={}", crate::link::percent_encoded(id)),
        ipc::PieceScope::Everywhere => String::new(),
    };
    format!("kind={}&id={}{within}", kind.as_wire(), crate::link::percent_encoded(id))
}

fn unreachable<T: Send + 'static>() -> Task<Result<T, Refused>> {
    Task::ready(Err(Refused::said("Fleet is not connected")))
}

impl Wire {
    fn port(&self, cx: &App) -> Option<u16> {
        match self {
            Wire::Live(fleet) => fleet.read(cx).held().port(),
            Wire::Mock(_) => None,
        }
    }

    /// Whether a change can be sent: a live Fleet that answers, or the mock, which always does.
    pub fn live(&self, cx: &App) -> bool {
        match self {
            Wire::Live(fleet) => fleet.read(cx).held().port().is_some(),
            Wire::Mock(mock) => mock.connected,
        }
    }

    /// `GET /settings`.
    pub fn settings(&mut self, cx: &App) -> Task<Result<SettingsList, Refused>> {
        if let Wire::Mock(mock) = self {
            return Task::ready(mock.settings());
        }
        self.port(cx).map_or_else(unreachable, |port| request(port, Ask::Get("/settings".into()), "SettingsList", cx))
    }

    /// `POST /settings/save`: each change set, or removed where it is `None`. Fleet answers with the whole list.
    pub fn save(&mut self, changes: BTreeMap<String, Option<SettingValue>>, cx: &App) -> Task<Result<SettingsList, Refused>> {
        if let Wire::Mock(mock) = self {
            return Task::ready(mock.save(&changes));
        }
        let said = body(&SaveSettings { changes });
        self.port(cx).map_or_else(unreachable, |port| request(port, Ask::Post("/settings/save".into(), said), "SettingsList", cx))
    }

    /// What Fleet runs, every repository's, as the connection last read `GET /workflows`; `None` before it has.
    pub fn workflows(&self, cx: &App) -> Option<Vec<ipc::WorkflowSummary>> {
        match self {
            Wire::Live(fleet) => fleet.read(cx).held().workflows.clone(),
            Wire::Mock(mock) => Some(mock.workflows.clone()),
        }
    }

    /// `GET /workflows`, read again after an act changed what Fleet holds.
    pub fn list_workflows(&mut self, cx: &App) -> Task<Result<Vec<ipc::WorkflowSummary>, Refused>> {
        if let Wire::Mock(mock) = self {
            return Task::ready(if mock.connected { Ok(mock.workflows.clone()) } else { Err(Refused::said("Fleet is not connected")) });
        }
        self.port(cx).map_or_else(unreachable, |port| request(port, Ask::Get("/workflows".into()), "workflows", cx))
    }

    /// `GET /workflows/steps`: one definition step by step, from the place named (`armada`, `kit`, `repository`).
    pub fn workflow_steps(&mut self, id: &str, source: &str, manifest: &str, cx: &App) -> Task<Result<ipc::WorkflowSteps, Refused>> {
        if let Wire::Mock(mock) = self {
            return Task::ready(mock.workflow_steps(id, source, manifest));
        }
        let path = scoped(&format!("/workflows/steps?workflow_id={}&source={}", crate::link::percent_encoded(id), crate::link::percent_encoded(source)), manifest);
        self.port(cx).map_or_else(unreachable, |port| request(port, Ask::Get(path), "WorkflowSteps", cx))
    }

    /// `GET /workflows/lanes`: one workflow's lanes at a level, as its editor draws them (`get_workflow_lanes`).
    pub fn workflow_lanes(&mut self, workflow: &str, scope: &str, manifest: &str, cx: &App) -> Task<Result<ipc::JobLanesView, Refused>> {
        if let Wire::Mock(mock) = self {
            return Task::ready(mock.workflow_lanes(workflow, scope));
        }
        let path = scoped(&format!("/workflows/lanes?workflow={}&scope={}", crate::link::percent_encoded(workflow), crate::link::percent_encoded(scope)), manifest);
        self.port(cx).map_or_else(unreachable, |port| request(port, Ask::Get(path), "JobLanesView", cx))
    }

    /// `GET /workflows/definition`: the file's text, as `save_workflow` takes it back.
    pub fn workflow_definition(&mut self, id: &str, source: &str, manifest: &str, cx: &App) -> Task<Result<ipc::WorkflowDefinition, Refused>> {
        if let Wire::Mock(mock) = self {
            return Task::ready(mock.workflow_definition(id, source, manifest));
        }
        let path = scoped(&format!("/workflows/definition?workflow_id={}&source={}", crate::link::percent_encoded(id), crate::link::percent_encoded(source)), manifest);
        self.port(cx).map_or_else(unreachable, |port| request(port, Ask::Get(path), "WorkflowDefinition", cx))
    }

    /// `GET /workflows/versions`: the versions a workflow has had in one place, newest first; none where nothing was published.
    pub fn workflow_versions(&mut self, id: &str, source: &str, project: Option<&str>, manifest: &str, cx: &App) -> Task<Result<ipc::WorkflowVersions, Refused>> {
        if let Wire::Mock(mock) = self {
            return Task::ready(mock.workflow_versions(id, source, project, manifest));
        }
        let path = scoped(&format!("/workflows/versions?{}", draft_query(id, source, project)), manifest);
        self.port(cx).map_or_else(unreachable, |port| request(port, Ask::Get(path), "WorkflowVersions", cx))
    }

    /// `POST /workflows/revert`: a version's text put in the draft as a change of his. Nothing is published.
    pub fn revert_workflow(&mut self, manifest: &str, revert: ipc::RevertWorkflow, cx: &App) -> Task<Result<ipc::WorkflowDraft, Refused>> {
        if let Wire::Mock(mock) = self {
            return Task::ready(mock.revert_workflow(manifest, revert));
        }
        let (path, said) = (scoped("/workflows/revert", manifest), body(&revert));
        self.port(cx).map_or_else(unreachable, |port| request(port, Ask::Post(path, said), "WorkflowDraft", cx))
    }

    /// `GET /workflows/definition` with `source=base`: the text the copy at `of` (`kit`, `repository` or `project`) began from;
    /// `fleet.workflow_base_none` where it has no base.
    pub fn workflow_base(&mut self, id: &str, of: &str, project: Option<&str>, manifest: &str, cx: &App) -> Task<Result<ipc::WorkflowDefinition, Refused>> {
        if let Wire::Mock(mock) = self {
            return Task::ready(mock.workflow_base(id, of, project, manifest));
        }
        let query = format!("{}&of={}", draft_query(id, "base", project), crate::link::percent_encoded(of));
        let path = scoped(&format!("/workflows/definition?{query}"), manifest);
        self.port(cx).map_or_else(unreachable, |port| request(port, Ask::Get(path), "WorkflowDefinition", cx))
    }

    /// `POST /workflows/save` under a Manifest.
    pub fn save_workflow(&mut self, manifest: &str, save: ipc::SaveWorkflow, cx: &App) -> Task<Result<ipc::WorkflowSaved, Refused>> {
        if let Wire::Mock(mock) = self {
            return Task::ready(mock.save_workflow(manifest, save));
        }
        let (path, said) = (scoped("/workflows/save", manifest), body(&save));
        self.port(cx).map_or_else(unreachable, |port| request(port, Ask::Post(path, said), "WorkflowSaved", cx))
    }

    /// `POST /workflows/reset`: a copy rewritten with Armada's definition of the same id.
    pub fn reset_workflow(&mut self, manifest: &str, copy: ipc::WorkflowCopy, cx: &App) -> Task<Result<ipc::WorkflowSaved, Refused>> {
        if let Wire::Mock(mock) = self {
            return Task::ready(mock.reset_workflow(manifest, copy));
        }
        let (path, said) = (scoped("/workflows/reset", manifest), body(&copy));
        self.port(cx).map_or_else(unreachable, |port| request(port, Ask::Post(path, said), "WorkflowSaved", cx))
    }

    /// `POST /workflows/delete`: a repository's or Yours copy removed.
    pub fn delete_workflow(&mut self, manifest: &str, copy: ipc::WorkflowCopy, cx: &App) -> Task<Result<ipc::WorkflowCopyDeleted, Refused>> {
        if let Wire::Mock(mock) = self {
            return Task::ready(mock.delete_workflow(manifest, copy));
        }
        let (path, said) = (scoped("/workflows/delete", manifest), body(&copy));
        self.port(cx).map_or_else(unreachable, |port| request(port, Ask::Post(path, said), "WorkflowCopyDeleted", cx))
    }

    /// `POST /workflows/propose`: a Job on Armada's own repository that lands a definition.
    pub fn propose_workflow(&mut self, proposing: ipc::ProposeToArmada, cx: &App) -> Task<Result<ipc::ArmadaProposed, Refused>> {
        if let Wire::Mock(mock) = self {
            return Task::ready(mock.propose_workflow(proposing));
        }
        let said = body(&proposing);
        self.port(cx).map_or_else(unreachable, |port| request(port, Ask::Post("/workflows/propose".into(), said), "ArmadaProposed", cx))
    }

    /// `GET /workflows/changes`: the open changes to one workflow in one place, drafting, ready, failed or cannot.
    pub fn workflow_changes(&mut self, id: &str, source: &str, manifest: &str, cx: &App) -> Task<Result<ipc::WorkflowChanges, Refused>> {
        if let Wire::Mock(mock) = self {
            return Task::ready(mock.workflow_changes(id, source, manifest));
        }
        let path = scoped(&format!("/workflows/changes?workflow_id={}&source={}", crate::link::percent_encoded(id), crate::link::percent_encoded(source)), manifest);
        self.port(cx).map_or_else(unreachable, |port| request(port, Ask::Get(path), "WorkflowChanges", cx))
    }

    /// `POST /workflows/changes/ask`: an agent drafts a change in his words; nothing is saved.
    pub fn ask_change(&mut self, manifest: &str, asking: ipc::AskWorkflowChange, cx: &App) -> Task<Result<ipc::WorkflowChange, Refused>> {
        if let Wire::Mock(mock) = self {
            return Task::ready(mock.ask_change(manifest, asking));
        }
        let (path, said) = (scoped("/workflows/changes/ask", manifest), body(&asking));
        self.port(cx).map_or_else(unreachable, |port| request(port, Ask::Post(path, said), "WorkflowChange", cx))
    }

    /// `POST /workflows/changes/approve`: a ready change written to its file.
    pub fn approve_change(&mut self, change: i64, cx: &App) -> Task<Result<ipc::WorkflowChangeApproved, Refused>> {
        if let Wire::Mock(mock) = self {
            return Task::ready(mock.approve_change(change));
        }
        let said = body(&ipc::WorkflowChangeRef { change });
        self.port(cx).map_or_else(unreachable, |port| request(port, Ask::Post("/workflows/changes/approve".into(), said), "WorkflowChangeApproved", cx))
    }

    /// `POST /workflows/changes/dismiss`.
    pub fn dismiss_change(&mut self, change: i64, cx: &App) -> Task<Result<ipc::WorkflowChange, Refused>> {
        if let Wire::Mock(mock) = self {
            return Task::ready(mock.dismiss_change(change));
        }
        let said = body(&ipc::WorkflowChangeRef { change });
        self.port(cx).map_or_else(unreachable, |port| request(port, Ask::Post("/workflows/changes/dismiss".into(), said), "WorkflowChange", cx))
    }

    /// `POST /workflows/changes/send_back`: drafted again with his notes.
    pub fn send_back_change(&mut self, back: ipc::SendBackWorkflowChange, cx: &App) -> Task<Result<ipc::WorkflowChange, Refused>> {
        if let Wire::Mock(mock) = self {
            return Task::ready(mock.send_back_change(back));
        }
        let said = body(&back);
        self.port(cx).map_or_else(unreachable, |port| request(port, Ask::Post("/workflows/changes/send_back".into(), said), "WorkflowChange", cx))
    }

    /// `POST /workflows/changes/build`: a Job on Armada's repository that builds what a `cannot` change lacks.
    pub fn build_support(&mut self, change: i64, cx: &App) -> Task<Result<ipc::WorkflowSupportBuilding, Refused>> {
        if let Wire::Mock(mock) = self {
            return Task::ready(mock.build_support(change));
        }
        let said = body(&ipc::WorkflowChangeRef { change });
        self.port(cx).map_or_else(unreachable, |port| request(port, Ask::Post("/workflows/changes/build".into(), said), "WorkflowSupportBuilding", cx))
    }

    /// `GET /lifecycles`: the lifecycles that apply to one repository, and where each was kept.
    pub fn lifecycles(&mut self, manifest: &str, cx: &App) -> Task<Result<ipc::RepositoryLifecycles, Refused>> {
        if let Wire::Mock(mock) = self {
            return Task::ready(mock.lifecycles(manifest));
        }
        let path = scoped("/lifecycles", manifest);
        self.port(cx).map_or_else(unreachable, |port| request(port, Ask::Get(path), "RepositoryLifecycles", cx))
    }

    /// `GET /lifecycles/rules`: the rules that apply to one repository, and where each was kept.
    pub fn lifecycle_rules(&mut self, manifest: &str, cx: &App) -> Task<Result<ipc::RepositoryLifecycleRules, Refused>> {
        if let Wire::Mock(mock) = self {
            return Task::ready(mock.lifecycle_rules(manifest));
        }
        let path = scoped("/lifecycles/rules", manifest);
        self.port(cx).map_or_else(unreachable, |port| request(port, Ask::Get(path), "RepositoryLifecycleRules", cx))
    }

    /// `GET /autonomy`: Armada's default autonomy for each kind of act, and each layer that sets some for one repository.
    pub fn autonomy_defaults(&mut self, manifest: &str, cx: &App) -> Task<Result<ipc::RepositoryAutonomy, Refused>> {
        if let Wire::Mock(mock) = self {
            return Task::ready(mock.autonomy_defaults(manifest));
        }
        let path = scoped("/autonomy", manifest);
        self.port(cx).map_or_else(unreachable, |port| request(port, Ask::Get(path), "RepositoryAutonomy", cx))
    }

    /// `GET /watches/recipes`: the recipes a watch may start from.
    pub fn watch_recipes(&mut self, cx: &App) -> Task<Result<ipc::WatchRecipeList, Refused>> {
        if let Wire::Mock(mock) = self {
            return Task::ready(mock.watch_recipes());
        }
        self.port(cx).map_or_else(unreachable, |port| request(port, Ask::Get("/watches/recipes".into()), "WatchRecipeList", cx))
    }

    /// `POST /lifecycles/pieces/delete`.
    pub fn delete_lifecycle_piece(&mut self, delete: ipc::DeleteLifecyclePiece, cx: &App) -> Task<Result<ipc::LifecyclePieceDeleted, Refused>> {
        if let Wire::Mock(mock) = self {
            return Task::ready(mock.delete_lifecycle_piece(delete));
        }
        let said = body(&delete);
        self.port(cx).map_or_else(unreachable, |port| request(port, Ask::Post("/lifecycles/pieces/delete".into(), said), "LifecyclePieceDeleted", cx))
    }

    /// `GET /workflows/draft`: the draft kept for a workflow in a place; `fleet.workflow_draft_none` where none is.
    pub fn workflow_draft(&mut self, id: &str, source: &str, project: Option<&str>, manifest: &str, cx: &App) -> Task<Result<ipc::WorkflowDraft, Refused>> {
        if let Wire::Mock(mock) = self {
            return Task::ready(mock.workflow_draft(id, source, project, manifest));
        }
        let path = scoped(&format!("/workflows/draft?{}", draft_query(id, source, project)), manifest);
        self.port(cx).map_or_else(unreachable, |port| request(port, Ask::Get(path), "WorkflowDraft", cx))
    }

    /// `PUT /workflows/draft`: the draft text as it is now.
    pub fn put_workflow_draft(&mut self, manifest: &str, put: ipc::PutWorkflowDraft, cx: &App) -> Task<Result<ipc::WorkflowDraft, Refused>> {
        if let Wire::Mock(mock) = self {
            return Task::ready(mock.put_workflow_draft(manifest, put));
        }
        let (path, said) = (scoped("/workflows/draft", manifest), body(&put));
        self.port(cx).map_or_else(unreachable, |port| request(port, Ask::Put(path, said), "WorkflowDraft", cx))
    }

    /// `DELETE /workflows/draft`.
    pub fn discard_workflow_draft(&mut self, id: &str, source: &str, project: Option<&str>, manifest: &str, cx: &App) -> Task<Result<ipc::WorkflowDraftDiscarded, Refused>> {
        if let Wire::Mock(mock) = self {
            return Task::ready(mock.discard_workflow_draft(id, source, project, manifest));
        }
        let path = scoped(&format!("/workflows/draft?{}", draft_query(id, source, project)), manifest);
        self.port(cx).map_or_else(unreachable, |port| request(port, Ask::Delete(path), "WorkflowDraftDiscarded", cx))
    }

    /// `POST /workflows/draft/publish`: the draft written where it applies.
    pub fn publish_workflow_draft(&mut self, manifest: &str, publish: ipc::PublishWorkflowDraft, cx: &App) -> Task<Result<ipc::WorkflowPublished, Refused>> {
        if let Wire::Mock(mock) = self {
            return Task::ready(mock.publish_workflow_draft(manifest, publish));
        }
        let (path, said) = (scoped("/workflows/draft/publish", manifest), body(&publish));
        self.port(cx).map_or_else(unreachable, |port| request(port, Ask::Post(path, said), "WorkflowPublished", cx))
    }

    /// `GET /workflows/affected`: the live Jobs a publish at this place would pin. `manifest_id` goes only for the repository's own.
    pub fn affected_jobs(&mut self, id: &str, source: &str, project: Option<&str>, manifest: &str, cx: &App) -> Task<Result<ipc::AffectedJobs, Refused>> {
        if let Wire::Mock(mock) = self {
            return Task::ready(mock.affected_jobs(id, source, project, manifest));
        }
        let route = format!("/workflows/affected?{}", draft_query(id, source, project));
        let path = if source == "repository" { scoped(&route, manifest) } else { route };
        self.port(cx).map_or_else(unreachable, |port| request(port, Ask::Get(path), "AffectedJobs", cx))
    }

    /// `POST /workflows/migrate`: the Jobs he picked, moved to the published version. Always answered, with a result for each.
    pub fn migrate_jobs(&mut self, manifest: &str, migrate: ipc::MigrateJobs, cx: &App) -> Task<Result<ipc::MigratedJobs, Refused>> {
        if let Wire::Mock(mock) = self {
            return Task::ready(mock.migrate_jobs(migrate));
        }
        let (path, said) = (scoped("/workflows/migrate", manifest), body(&migrate));
        self.port(cx).map_or_else(unreachable, |port| request(port, Ask::Post(path, said), "MigratedJobs", cx))
    }

    /// `GET /lifecycles/pieces/draft`: the draft kept for a piece at a scope; `fleet.piece_draft_none` where none is.
    pub fn piece_draft(&mut self, kind: ipc::LifecyclePieceKind, id: &str, scope: &ipc::PieceScope, cx: &App) -> Task<Result<ipc::PieceDraft, Refused>> {
        if let Wire::Mock(mock) = self {
            return Task::ready(mock.piece_draft(kind, id, scope));
        }
        let path = format!("/lifecycles/pieces/draft?{}", piece_query(kind, id, scope));
        self.port(cx).map_or_else(unreachable, |port| request(port, Ask::Get(path), "PieceDraft", cx))
    }

    /// `PUT /lifecycles/pieces/draft`.
    pub fn put_piece_draft(&mut self, put: ipc::PutPieceDraft, cx: &App) -> Task<Result<ipc::PieceDraft, Refused>> {
        if let Wire::Mock(mock) = self {
            return Task::ready(mock.put_piece_draft(put));
        }
        let said = body(&put);
        self.port(cx).map_or_else(unreachable, |port| request(port, Ask::Put("/lifecycles/pieces/draft".into(), said), "PieceDraft", cx))
    }

    /// `DELETE /lifecycles/pieces/draft`.
    pub fn discard_piece_draft(&mut self, kind: ipc::LifecyclePieceKind, id: &str, scope: &ipc::PieceScope, cx: &App) -> Task<Result<ipc::PieceDraftDiscarded, Refused>> {
        if let Wire::Mock(mock) = self {
            return Task::ready(mock.discard_piece_draft(kind, id, scope));
        }
        let path = format!("/lifecycles/pieces/draft?{}", piece_query(kind, id, scope));
        self.port(cx).map_or_else(unreachable, |port| request(port, Ask::Delete(path), "PieceDraftDiscarded", cx))
    }

    /// `POST /lifecycles/pieces/draft/publish`.
    pub fn publish_piece_draft(&mut self, publish: ipc::PublishPieceDraft, cx: &App) -> Task<Result<ipc::PiecePublished, Refused>> {
        if let Wire::Mock(mock) = self {
            return Task::ready(mock.publish_piece_draft(publish));
        }
        let said = body(&publish);
        self.port(cx).map_or_else(unreachable, |port| request(port, Ask::Post("/lifecycles/pieces/draft/publish".into(), said), "PiecePublished", cx))
    }

    /// `GET /models`: the models the harness offers, and the model a run that names none is given.
    pub fn models(&mut self, cx: &App) -> Task<Result<ipc::ModelChoices, Refused>> {
        if let Wire::Mock(mock) = self {
            return Task::ready(if mock.connected { Ok(mock.models.clone()) } else { Err(Refused::said("Fleet is not connected")) });
        }
        self.port(cx).map_or_else(unreachable, |port| request(port, Ask::Get("/models".into()), "ModelChoices", cx))
    }

    /// `POST /phone {on: true}` on Fleet's socket: the Gateway starts now and at every start after.
    pub fn phone_on(&mut self, cx: &App) -> Task<Result<ipc::PhoneGateway, Refused>> {
        if let Wire::Mock(mock) = self {
            mock.gateway.running = true;
            return Task::ready(Ok(ipc::PhoneGateway { on: true, port: 8443, listening: true, why: None }));
        }
        let said = body(&ipc::SetPhoneGateway { on: true });
        self.port(cx).map_or_else(unreachable, |port| request(port, Ask::Post("/phone".into(), said), "PhoneGateway", cx))
    }

    /// `GET /admin/setup` on the Phone Gateway, which runs whether or not Fleet answers.
    pub fn gateway_setup(&mut self, cx: &App) -> Task<Result<ipc::GatewaySetup, String>> {
        match self {
            Wire::Mock(mock) => Task::ready(mock.gateway.read_setup()),
            Wire::Live(_) => super::gateway::setup(cx),
        }
    }

    pub fn gateway_macs(&mut self, cx: &App) -> Task<Result<Vec<ipc::Mac>, String>> {
        match self {
            Wire::Mock(mock) => Task::ready(mock.gateway.read_macs()),
            Wire::Live(_) => super::gateway::macs(cx),
        }
    }

    pub fn pair_start(&mut self, cx: &App) -> Task<Result<ipc::MacPairLink, String>> {
        match self {
            Wire::Mock(mock) => Task::ready(mock.gateway.pair_start()),
            Wire::Live(_) => super::gateway::pair_start(cx),
        }
    }

    pub fn pair_claim(&mut self, link: String, cx: &App) -> Task<Result<ipc::Mac, String>> {
        match self {
            Wire::Mock(mock) => Task::ready(mock.gateway.pair_claim(&link)),
            Wire::Live(_) => super::gateway::pair_claim(link, cx),
        }
    }

    pub fn pair_confirm(&mut self, mac: String, cx: &App) -> Task<Result<ipc::Mac, String>> {
        match self {
            Wire::Mock(mock) => Task::ready(mock.gateway.pair_confirm(&mac)),
            Wire::Live(_) => super::gateway::pair_confirm(mac, cx),
        }
    }

    pub fn phone_start(&mut self, cx: &App) -> Task<Result<super::gateway::PhoneCode, String>> {
        match self {
            Wire::Mock(mock) => Task::ready(mock.gateway.phone_start()),
            Wire::Live(_) => super::gateway::phone_start(cx),
        }
    }

    pub fn phone_pending(&mut self, cx: &App) -> Task<Result<Vec<super::gateway::PhoneClaim>, String>> {
        match self {
            Wire::Mock(mock) => Task::ready(mock.gateway.phone_pending()),
            Wire::Live(_) => super::gateway::phone_pending(cx),
        }
    }

    pub fn phone_confirm(&mut self, code: String, cx: &App) -> Task<Result<ipc::PairedPhone, String>> {
        match self {
            Wire::Mock(mock) => Task::ready(mock.gateway.phone_confirm(&code)),
            Wire::Live(_) => super::gateway::phone_confirm(code, cx),
        }
    }

    pub fn phone_decline(&mut self, code: String, cx: &App) -> Task<Result<(), String>> {
        match self {
            Wire::Mock(mock) => Task::ready(mock.gateway.phone_decline(&code)),
            Wire::Live(_) => super::gateway::phone_decline(code, cx),
        }
    }

    pub fn forget_mac(&mut self, mac: String, cx: &App) -> Task<Result<(), String>> {
        match self {
            Wire::Mock(mock) => Task::ready(mock.gateway.forget(&mac)),
            Wire::Live(_) => super::gateway::forget(mac, cx),
        }
    }

    /// `POST /macs/update` on Fleet's socket, which the Gateway does not carry: 202 says the restart has begun.
    pub fn update_mac(&mut self, asked: ipc::UpdateMac, cx: &App) -> Task<Result<ipc::FleetBuildChanging, String>> {
        if let Wire::Mock(mock) = self {
            return Task::ready(mock.gateway.update(&asked));
        }
        let said = body(&asked);
        let sent = self.port(cx).map_or_else(unreachable, |port| request(port, Ask::Post("/macs/update".into(), said), "FleetBuildChanging", cx));
        cx.background_spawn(async move { sent.await.map_err(|refused| refused.said) })
    }

    pub fn gateway_phones(&mut self, cx: &App) -> Task<Result<Vec<ipc::PairedPhone>, String>> {
        match self {
            Wire::Mock(mock) => Task::ready(if mock.gateway.running { Ok(mock.gateway.phones.clone()) } else { Err(super::gateway::NOT_RUNNING.into()) }),
            Wire::Live(_) => super::gateway::devices(cx),
        }
    }

    pub fn start_tunnel(&mut self, cx: &App) -> Task<Result<super::gateway::Tunnel, String>> {
        match self {
            Wire::Mock(mock) => Task::ready(mock.gateway.start()),
            Wire::Live(_) => super::gateway::start(cx),
        }
    }

    pub fn gateway_unpair(&mut self, id: String, cx: &App) -> Task<Result<(), String>> {
        match self {
            Wire::Mock(mock) => Task::ready(mock.gateway.unpair(&id)),
            Wire::Live(_) => super::gateway::unpair(id, cx),
        }
    }

    /// Opens a page in his browser; the mock keeps it instead.
    pub fn open_link(&mut self, address: &str, cx: &mut App) {
        match self {
            Wire::Mock(mock) => mock.handed.push(address.to_string()),
            Wire::Live(_) => cx.open_url(address),
        }
    }

    /// Puts words on his clipboard; the mock keeps them instead.
    pub fn copy(&mut self, words: &str, cx: &mut App) {
        match self {
            Wire::Mock(mock) => mock.handed.push(words.to_string()),
            Wire::Live(_) => cx.write_to_clipboard(gpui::ClipboardItem::new_string(words.to_string())),
        }
    }

    /// The project a Job was started for, by id and name, as Fleet holds its record. None for a Job started for
    /// none, or one Fleet does not list.
    pub fn project_of_job(&self, job: &str, cx: &App) -> Option<(String, String)> {
        match self {
            Wire::Live(fleet) => {
                let held = fleet.read(cx).held();
                let id = held.jobs.iter().find(|one| one.id.as_str() == job)?.hosted.as_ref()?.project.clone()?;
                let name = held.projects.as_ref().and_then(|all| all.projects.iter().find(|one| one.id == id)).map_or_else(|| id.clone(), |one| one.name.clone());
                Some((id, name))
            }
            Wire::Mock(mock) => {
                let id = mock.job_projects.iter().find(|(one, _)| *one == job)?.1;
                let name = mock.projects.projects.iter().find(|one| one.id == id).map_or_else(|| id.to_string(), |one| one.name.clone());
                Some((id.to_string(), name))
            }
        }
    }

    /// The repositories Fleet serves, by Manifest id, name and whether its armada.yml turns connectors off.
    pub fn repositories(&self, cx: &App) -> Option<Vec<(String, String, bool)>> {
        let held = match self {
            Wire::Live(fleet) => fleet.read(cx).held().repositories.clone()?,
            Wire::Mock(mock) => mock.repositories.clone(),
        };
        Some(held.into_iter().filter_map(|one| one.manifest.map(|manifest| (manifest.id.as_str().to_string(), manifest.repository, manifest.connectors_off))).collect())
    }

    /// `GET /projects/values?project=<id>`: what the project's values come to.
    pub fn values(&mut self, project: &str, cx: &App) -> Task<Result<ResolvedValues, Refused>> {
        if let Wire::Mock(mock) = self {
            return Task::ready(mock.values(project));
        }
        let path = format!("/projects/values?project={}", crate::link::percent_encoded(project));
        self.port(cx).map_or_else(unreachable, |port| request(port, Ask::Get(path), "ResolvedValues", cx))
    }

    /// `GET /projects/values` for no project: what every project uses, Kit's over the defaults.
    pub fn every_values(&mut self, cx: &App) -> Task<Result<ResolvedValues, Refused>> {
        if let Wire::Mock(mock) = self {
            return Task::ready(mock.every_values());
        }
        self.port(cx).map_or_else(unreachable, |port| request(port, Ask::Get("/projects/values".into()), "ResolvedValues", cx))
    }

    /// `POST /kit/values`: Kit's values, whole. A key left out is taken away.
    pub fn save_kit_values(&mut self, values: ValuesSet, cx: &App) -> Task<Result<ipc::KitValuesSaved, Refused>> {
        if let Wire::Mock(mock) = self {
            return Task::ready(mock.save_kit_values(values));
        }
        let said = body(&ipc::SaveKitValues { values });
        self.port(cx).map_or_else(unreachable, |port| request(port, Ask::Post("/kit/values".into(), said), "KitValuesSaved", cx))
    }

    /// `GET /projects`, read fresh before a project's values are written whole.
    pub fn projects_now(&mut self, cx: &App) -> Task<Result<Projects, Refused>> {
        if let Wire::Mock(mock) = self {
            return Task::ready(Ok(mock.projects.clone()));
        }
        self.port(cx).map_or_else(unreachable, |port| request(port, Ask::Get("/projects".into()), "Projects", cx))
    }

    /// `POST /projects/project/:id/edit` with the project's whole block of values.
    pub fn set_values(&mut self, project: &str, values: ValuesSet, cx: &App) -> Task<Result<Projects, Refused>> {
        if let Wire::Mock(mock) = self {
            return Task::ready(mock.set_values(project, values));
        }
        let path = format!("/projects/project/{}/edit", crate::link::percent_encoded(project));
        let said = body(&EditProject { values: Some(values), ..EditProject::default() });
        self.port(cx).map_or_else(unreachable, |port| request(port, Ask::Post(path, said), "Projects", cx))
    }

    /// `POST /projects/project/:id/edit` with `shared_with_macs` alone.
    pub fn share_with_macs(&mut self, project: &str, on: bool, cx: &App) -> Task<Result<Projects, Refused>> {
        if let Wire::Mock(mock) = self {
            return Task::ready(mock.share_with_macs(project, on));
        }
        let path = format!("/projects/project/{}/edit", crate::link::percent_encoded(project));
        let said = body(&EditProject { shared_with_macs: Some(on), ..EditProject::default() });
        self.port(cx).map_or_else(unreachable, |port| request(port, Ask::Post(path, said), "Projects", cx))
    }

    /// `POST /projects/project/:id/workflow`: one workflow switched on or off for the project.
    pub fn set_project_workflow(&mut self, project: &str, workflow: &str, on: bool, cx: &App) -> Task<Result<Projects, Refused>> {
        if let Wire::Mock(mock) = self {
            return Task::ready(mock.set_project_workflow(project, workflow, on));
        }
        let path = format!("/projects/project/{}/workflow", crate::link::percent_encoded(project));
        let said = body(&ipc::SetProjectWorkflow { workflow: workflow.to_string(), on });
        self.port(cx).map_or_else(unreachable, |port| request(port, Ask::Post(path, said), "Projects", cx))
    }

    /// `POST /projects/project/:id/delete`, no body: the project and its rows. Fleet answers every project that is left.
    pub fn delete_project(&mut self, project: &str, cx: &App) -> Task<Result<Projects, Refused>> {
        if let Wire::Mock(mock) = self {
            return Task::ready(mock.delete_project(project));
        }
        let path = format!("/projects/project/{}/delete", crate::link::percent_encoded(project));
        self.port(cx).map_or_else(unreachable, |port| request(port, Ask::Post(path, String::new()), "Projects", cx))
    }

    /// The person's key bindings as Fleet last said them, or the mock's.
    pub fn key_bindings(&self, cx: &App) -> String {
        match self {
            Wire::Live(fleet) => fleet.read(cx).held().key_bindings.clone().unwrap_or_default(),
            Wire::Mock(mock) => mock.preferences.key_bindings.clone(),
        }
    }

    /// Whether the first-run screen is owed: Fleet has been read and he has not answered. Over a mock Fleet it is
    /// the mock's own preference.
    pub fn first_run_owed(&self, cx: &App) -> bool {
        match self {
            Wire::Live(fleet) => fleet.read(cx).held().first_run_answered == Some(false),
            Wire::Mock(mock) => !mock.preferences.first_run_answered,
        }
    }

    /// `GET /preferences`: every preference in force.
    pub fn preferences(&self, cx: &App) -> Task<Result<ipc::Preferences, Refused>> {
        if let Wire::Mock(mock) = self {
            return Task::ready(if mock.connected { Ok(mock.preferences.clone()) } else { Err(Refused::said("Fleet is not connected")) });
        }
        self.port(cx).map_or_else(unreachable, |port| request(port, Ask::Get("/preferences".into()), "Preferences", cx))
    }

    /// `POST /preferences/save` with a preference that is text. Fleet answers with every preference in force.
    pub fn save_preference(&mut self, name: &str, text: String, cx: &App) -> Task<Result<ipc::Preferences, Refused>> {
        self.save_put(name, Put::Text(text), cx)
    }

    /// One preference saved, as the word or the text it is, or a switch, which rides in `value`.
    pub fn save_put(&mut self, name: &str, put: Put, cx: &App) -> Task<Result<ipc::Preferences, Refused>> {
        if let Wire::Mock(mock) = self {
            return Task::ready(mock.save_put(name, put));
        }
        let said = body(&match put {
            Put::Text(text) => ipc::SavePreference { name: name.to_string(), value: false, text: Some(text) },
            Put::Switch(on) => ipc::SavePreference { name: name.to_string(), value: on, text: None },
        });
        self.port(cx).map_or_else(unreachable, |port| request(port, Ask::Post("/preferences/save".into(), said), "Preferences", cx))
    }

    /// Every preference as Fleet answered a save: folded into what the connection holds, so the look and the keys
    /// follow it as they do a read. The mock's walk has no controller to follow it, so it is put in force here.
    pub fn preferences_saved(&mut self, answer: ipc::Preferences, window: &mut Window, cx: &mut App) {
        match self {
            Wire::Live(fleet) => fleet.update(cx, |fleet, cx| fleet.preferences_saved(answer, cx)),
            Wire::Mock(mock) => {
                super::time::hold(&answer.time_format);
                super::layout::hold(&answer.layout_choices);
                cx.refresh_windows();
                crate::keymap::install(&crate::keymap::Rebinding::read(&answer.key_bindings), cx);
                crate::look::install(super::theme::look_of(&answer.theme, &mock.sheets, window), cx);
            }
        }
    }

    /// A theme picked, drawn before Fleet answers. Only the mock draws it here: over a live Fleet the look
    /// follows the answer, which is the one place the theme in force is decided.
    pub fn show_theme(&self, word: &str, window: &mut Window, cx: &mut App) {
        if let Wire::Mock(mock) = self {
            crate::look::install(super::theme::look_of(word, &mock.sheets, window), cx);
        }
    }

    /// The theme word in force, as Fleet last said it.
    pub fn theme(&self, cx: &App) -> String {
        match self {
            Wire::Live(fleet) => fleet.read(cx).held().theme.clone().unwrap_or_else(|| ipc::DEFAULT_THEME.to_string()),
            Wire::Mock(mock) => mock.preferences.theme.clone(),
        }
    }

    /// Fleet's mod list as it stands.
    pub fn mods(&self, cx: &App) -> Vec<ipc::ModSummary> {
        match self {
            Wire::Live(fleet) => fleet.read(cx).held().mods.clone().unwrap_or_default(),
            Wire::Mock(mock) => mock.mods.clone(),
        }
    }

    /// The checked stylesheets of the theme mods that are on and valid, by name.
    pub fn theme_sheets(&self, cx: &App) -> BTreeMap<String, String> {
        match self {
            Wire::Live(fleet) => fleet.read(cx).held().theme_sheets(),
            Wire::Mock(mock) => mock.sheets.clone(),
        }
    }

    /// `POST /mods/enable`. Fleet answers with the mod's row, and publishes the list again.
    pub fn set_mod_enabled(&mut self, name: &str, enabled: bool, cx: &App) -> Task<Result<ipc::ModSummary, Refused>> {
        if let Wire::Mock(mock) = self {
            return Task::ready(mock.set_mod_enabled(name, enabled));
        }
        let said = body(&ipc::SetModEnabled { name: name.to_string(), enabled });
        self.port(cx).map_or_else(unreachable, |port| request(port, Ask::Post("/mods/enable".into(), said), "ModSummary", cx))
    }

    /// `POST /mods/promote`, into the first repository Fleet serves, as Classic's Promote sends it.
    pub fn promote_mod(&mut self, name: &str, cx: &App) -> Task<Result<ipc::ModPromoted, Refused>> {
        if let Wire::Mock(mock) = self {
            return Task::ready(mock.promote_mod(name));
        }
        let said = body(&ipc::PromoteMod { name: name.to_string(), manifest_id: None });
        self.port(cx).map_or_else(unreachable, |port| request(port, Ask::Post("/mods/promote".into(), said), "ModPromoted", cx))
    }

    /// The connection a mod's panel reads its row from, or none for the mock, whose panel reads the walk's mods.
    pub fn fleet(&self) -> Option<Entity<FleetConnection>> {
        match self {
            Wire::Live(fleet) => Some(fleet.clone()),
            Wire::Mock(_) => None,
        }
    }

    /// `GET /mods/validate?name=`: the file Fleet checked for a mod, which its row opens to.
    pub fn validate_mod(&mut self, name: &str, cx: &App) -> Task<Result<ipc::ModChecked, Refused>> {
        if let Wire::Mock(mock) = self {
            return Task::ready(mock.validate_mod(name));
        }
        let path = format!("/mods/validate?name={}", crate::link::percent_encoded(name));
        self.port(cx).map_or_else(unreachable, |port| request(port, Ask::Get(path), "ModChecked", cx))
    }

    /// `POST /mods/try`: a tracker mod tried against the repository Fleet picks for it.
    pub fn try_tracker_mod(&mut self, name: &str, cx: &App) -> Task<Result<ipc::TrackerTry, Refused>> {
        if let Wire::Mock(mock) = self {
            return Task::ready(mock.try_tracker_mod(name));
        }
        let said = body(&ipc::TryTrackerMod { name: name.to_string(), manifest_id: None });
        self.port(cx).map_or_else(unreachable, |port| request(port, Ask::Post("/mods/try".into(), said), "TrackerTry", cx))
    }

    /// `POST /mods/session` with nothing set: Create mod's empty Mod Job (`startEmptyModJob`).
    pub fn start_mod_session(&mut self, cx: &App) -> Task<Result<ipc::ModSessionStarted, Refused>> {
        if let Wire::Mock(mock) = self {
            return Task::ready(mock.start_mod_session());
        }
        let said = body(&ipc::StartModSession { title: None, model: None, effort: None, aim: None, note: None, said: None, from_job: None, from_tab: None });
        self.port(cx).map_or_else(unreachable, |port| request(port, Ask::Post("/mods/session".into(), said), "ModSessionStarted", cx))
    }

    /// `POST /sessions/dispatch`: a Job started in a repository, in `project` where there is one, with `words` as its first
    /// message, by the route and client Bridge's own start uses. The answer is the Job's id, none where the Operator holds the idea.
    pub fn start_job(&mut self, manifest: &str, project: Option<&str>, words: String, cx: &App) -> Task<Result<Option<String>, Refused>> {
        if let Wire::Mock(mock) = self {
            return Task::ready(mock.start_job(manifest, project, words));
        }
        let Some(port) = self.port(cx) else { return unreachable() };
        let body = match crate::custom_tracker::dispatch_body(manifest, project, &words) {
            Ok(body) => body,
            Err(why) => return Task::ready(Err(Refused::said(why))),
        };
        let sent = crate::custom_tracker::send(port, body, cx);
        cx.background_spawn(async move { sent.await.map_err(Refused::said) })
    }

    /// Open settings.json in the editor settings.json names, off the main thread. The mock opens nothing.
    pub fn open_file(&mut self, list: &SettingsList, cx: &App) -> Task<Result<(), String>> {
        if let Wire::Mock(mock) = self {
            mock.opened += 1;
            return Task::ready(Ok(()));
        }
        let (editor, path) = (crate::editor::configured(list), list.path.clone());
        cx.background_spawn(async move { crate::editor::open(&editor, &path, 1) })
    }
}
