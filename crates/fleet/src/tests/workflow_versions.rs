//! A workflow's versions and what a copy of one of Armada's began from: a publish writes the log, a revert puts one in the draft,
//! a copy made from Armada's text keeps its base, and a list says where Armada has moved. What a save writes is the composition
//! root's and is `Planted` here, which keeps what it was asked to save.

use axum::http::StatusCode;
use ipc::{
    BasedOn, DraftBy, PublishWorkflowDraft, PutWorkflowDraft, RevertWorkflow, SaveWorkflow, VersionOrigin, WorkflowDefinition, WorkflowDraft,
    WorkflowId, WorkflowPublished, WorkflowScope, WorkflowSummary, WorkflowVersions,
};
use store::{KeptBase, NewVersion, WorkflowPlace};

use crate::repositories::SavedWorkflow;
use crate::tests::http::call;
use crate::tests::repositories::code;
use crate::tests::workflow_changes::{decoded, rig, with_refine, Rig};
use crate::tests::workflow_page::{at, carried_id, read, staged_in_repository, STAGED};
use crate::workflow_versions::text_hash;

fn saved(workflow: &str, file: &str) -> SavedWorkflow {
    SavedWorkflow { id: carried_id(workflow), file: file.to_string(), replaced: true }
}

/// Armada's carried text of `feature`, and the `version` it declares.
fn carried_feature() -> (u32, String) {
    let file = config::carried().into_iter().find(|one| one.text().contains("\"workflow_id\": \"feature\"")).expect("Armada carries `feature`");
    let def = config::WorkflowDef::parse(file.path(), file.text().trim(), &config::Roster::of(["haiku", "sonnet", "opus"])).expect("it parses");
    (def.version(), file.text().to_string())
}

fn kit_place() -> WorkflowPlace {
    WorkflowPlace { source: "kit".into(), item_id: "feature".into(), ..Default::default() }
}

impl Rig {
    async fn put_staged(&self, text: &str) {
        let put = ipc::encode(&PutWorkflowDraft { workflow_id: WorkflowId::carried("staged"), source: "repository".into(), text: text.into(), ..Default::default() }).unwrap();
        let (status, body) = call(&self.app, "PUT", "/workflows/draft", &put).await;
        assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    }

    /// Publish `staged` in the repository, the folders reading as they did at the start.
    async fn publish_staged(&self) -> u32 {
        self.planted.will_save(Ok(saved("staged", "/repo/.armada/workflows/staged.json")));
        self.planted.will_read(read(staged_in_repository()));
        let body = ipc::encode(&PublishWorkflowDraft { workflow_id: WorkflowId::carried("staged"), source: "repository".into(), ..Default::default() }).unwrap();
        let (status, body) = call(&self.app, "POST", "/workflows/draft/publish", &body).await;
        assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
        // A publish reads the folders twice, and the fixture answers the first only: they read as they did, again.
        self.planted.will_read(read(staged_in_repository()));
        self.fleet.workflows_changed(&self.home.path().to_string_lossy());
        decoded::<WorkflowPublished>("a publish", &body).version
    }

    async fn versions(&self, query: &str) -> WorkflowVersions {
        let (status, body) = call(&self.app, "GET", &format!("/workflows/versions?{query}"), "").await;
        assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
        decoded("versions", &body)
    }

    async fn get(&self, query: &str) -> (StatusCode, Vec<u8>) {
        call(&self.app, "GET", &format!("/workflows/definition?{query}"), "").await
    }

    /// The folders hold a Kit copy of `feature` with `text`.
    fn holds_kit_copy(&self, text: &str) {
        self.planted.will_read(read(vec![config::Written::in_kit(at("kit", "feature.json"), text.to_string())]));
        self.fleet.workflows_changed(&self.home.path().to_string_lossy());
    }

    async fn summary_of_feature(&self) -> WorkflowSummary {
        let (status, body) = call(&self.app, "GET", "/workflows", "").await;
        assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
        decoded::<Vec<WorkflowSummary>>("workflows", &body).into_iter().find(|one| one.id.as_str() == "feature").expect("`feature` is held")
    }
}

/// **Where a file was there first, its text is version 1 (`found`) and the publish is 2**; each later publish is the next, and the list
/// is newest first with the hash of each text.
#[tokio::test]
async fn the_first_publish_keeps_the_text_that_was_there_as_version_one_and_each_publish_is_the_next() {
    let rig = rig(&[]).await;
    assert!(rig.versions("workflow_id=staged&source=repository").await.versions.is_empty(), "nothing was published");
    let (one, two) = (with_refine(), format!("{}\n# a second thought", with_refine()));

    rig.put_staged(&one).await;
    assert_eq!(rig.publish_staged().await, 2, "`found` is version 1 and the publish's own row follows it");
    rig.put_staged(&two).await;
    assert_eq!(rig.publish_staged().await, 3);

    let kept = rig.versions("workflow_id=staged&source=repository").await.versions;
    assert_eq!(kept.iter().map(|one| one.version).collect::<Vec<_>>(), [3, 2, 1]);
    assert_eq!(kept.iter().map(|one| one.text.as_str()).collect::<Vec<_>>(), [two.as_str(), one.as_str(), STAGED]);
    assert_eq!(kept.iter().map(|one| one.origin).collect::<Vec<_>>(), [VersionOrigin::Published, VersionOrigin::Published, VersionOrigin::Found]);
    assert_eq!(kept[2].hash, text_hash(STAGED));
    assert!(kept[2].hash.len() == 64 && kept[2].hash.chars().all(|digit| digit.is_ascii_hexdigit()), "a sha256 in hex");
    assert!(rig.versions("workflow_id=staged&source=kit").await.versions.is_empty(), "a place keeps its own");
}

/// **A revert puts a version's text in the draft as a change by `you` and publishes nothing**; it begins the draft where there is none, and
/// a version the place does not have is a 404.
#[tokio::test]
async fn a_revert_puts_that_versions_text_in_the_draft_as_a_change_by_you_and_publishes_nothing() {
    let rig = rig(&[]).await;
    let one = with_refine();
    rig.put_staged(&one).await;
    rig.publish_staged().await;
    rig.put_staged(&format!("{one}\n# a second thought")).await;
    rig.publish_staged().await;
    let saves = rig.planted.saves().len();
    assert_eq!(call(&rig.app, "GET", "/workflows/draft?workflow_id=staged&source=repository", "").await.0, StatusCode::NOT_FOUND, "no draft is kept");

    let revert = |version: u32| ipc::encode(&RevertWorkflow { workflow_id: WorkflowId::carried("staged"), source: "repository".into(), project: None, version }).unwrap();
    let (status, body) = call(&rig.app, "POST", "/workflows/revert", &revert(2)).await;

    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    let draft: WorkflowDraft = decoded("a draft", &body);
    assert_eq!(draft.text, one);
    assert_eq!(draft.base, STAGED, "it began from what is published");
    assert_eq!(draft.changes.len(), 1);
    assert_eq!((draft.changes[0].by, draft.changes[0].summary.as_deref()), (DraftBy::You, Some("Back to version 2")));
    assert_eq!(rig.planted.saves().len(), saves, "nothing was published");

    let (status, body) = call(&rig.app, "POST", "/workflows/revert", &revert(1)).await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    let draft: WorkflowDraft = decoded("a draft", &body);
    assert_eq!((draft.text.as_str(), draft.changes.len()), (STAGED, 2), "a revert to another version is the next change in the same draft");

    let (status, body) = call(&rig.app, "POST", "/workflows/revert", &revert(9)).await;
    assert_eq!((status, code(&body).as_str()), (StatusCode::NOT_FOUND, "fleet.workflow_version_none"));
    assert_eq!(rig.planted.saves().len(), saves);
}

/// **A draft of Armada's has no file to find, so its first publish is version 1**, and it records the carried text and `version` the draft
/// began from as the copy's base, which `source=base` serves.
#[tokio::test]
async fn an_armada_drafts_first_publish_is_version_one_and_records_the_carried_version_and_hash_as_its_base() {
    let rig = rig(&[]).await;
    let (carried_version, carried) = carried_feature();
    let edited = format!("{carried}\n");
    let put = ipc::encode(&PutWorkflowDraft { workflow_id: WorkflowId::carried("feature"), source: "armada".into(), text: edited.clone(), ..Default::default() }).unwrap();
    assert_eq!(call(&rig.app, "PUT", "/workflows/draft", &put).await.0, StatusCode::OK);
    rig.planted.will_save(Ok(saved("feature", "/kit/workflows/feature.json")));
    rig.planted.will_read(read(staged_in_repository()));
    let publish = ipc::encode(&PublishWorkflowDraft { workflow_id: WorkflowId::carried("feature"), source: "armada".into(), scope: Some(WorkflowScope::Kit), ..Default::default() }).unwrap();

    let (status, body) = call(&rig.app, "POST", "/workflows/draft/publish", &publish).await;

    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    assert_eq!(decoded::<WorkflowPublished>("a publish", &body).version, 1);
    let kept = rig.versions("workflow_id=feature&source=kit").await.versions;
    assert_eq!((kept.len(), kept[0].version, kept[0].origin, kept[0].text.as_str()), (1, 1, VersionOrigin::Published, edited.as_str()));
    let (status, body) = rig.get("workflow_id=feature&source=base&of=kit").await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    let base: WorkflowDefinition = decoded("a base", &body);
    assert_eq!((base.source.as_str(), base.definition.as_str(), base.project), ("base", carried.as_str(), None));
    assert_eq!(base.based_on, Some(BasedOn { version: carried_version, hash: text_hash(&carried) }));
}

/// **Making a copy is a save of Armada's text, which is how the base is known**; a save of anything else records none, and a base is
/// served only for the copy that has one.
#[tokio::test]
async fn a_save_of_armadas_text_records_the_base_and_a_save_of_edited_text_does_not() {
    let rig = rig(&[]).await;
    let (carried_version, carried) = carried_feature();
    let save = |scope: WorkflowScope, text: &str| ipc::encode(&SaveWorkflow { scope, definition: text.to_string(), overwrite: false }).unwrap();

    rig.planted.will_save(Ok(saved("feature", "/kit/workflows/feature.json")));
    rig.planted.will_read(read(staged_in_repository()));
    let (status, body) = call(&rig.app, "POST", "/workflows/save", &save(WorkflowScope::Kit, &carried)).await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    let (status, body) = rig.get("workflow_id=feature&source=base&of=kit").await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    let base: WorkflowDefinition = decoded("a base", &body);
    assert_eq!((base.definition.as_str(), base.based_on), (carried.as_str(), Some(BasedOn { version: carried_version, hash: text_hash(&carried) })));

    rig.planted.will_save(Ok(saved("feature", "/repo/.armada/workflows/feature.json")));
    rig.planted.will_read(read(staged_in_repository()));
    let edited = carried.replace("\"label\": \"Propose\"", "\"label\": \"Mine\"");
    let (status, body) = call(&rig.app, "POST", "/workflows/save", &save(WorkflowScope::Repository, &edited)).await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    let (status, body) = rig.get("workflow_id=feature&source=base&of=repository").await;
    assert_eq!((status, code(&body).as_str()), (StatusCode::NOT_FOUND, "fleet.workflow_base_none"), "a copy of edited text has no base, and none is guessed");

    let (status, body) = rig.get("workflow_id=feature&source=base").await;
    assert_eq!((status, code(&body).as_str()), (StatusCode::UNPROCESSABLE_ENTITY, "fleet.workflow_base_of_needed"));
    assert!(rig.versions("workflow_id=feature&source=kit").await.versions.is_empty(), "a save writes no version");
}

/// **A summary says what its copy was based on, and that Armada has moved once the carried text no longer hashes to it.**
#[tokio::test]
async fn a_summary_says_armada_changed_where_the_carried_text_differs_from_the_base() {
    let rig = rig(&[]).await;
    let (version, carried) = carried_feature();
    rig.holds_kit_copy(&carried);
    let plain = rig.summary_of_feature().await;
    assert_eq!((plain.source.as_str(), plain.based_on, plain.armada_changed), ("kit", None, false), "a copy with no base says nothing of Armada");

    let base = |hash: String| KeptBase { version, hash, text: carried.clone() };
    rig.fleet.store().lock().await.put_workflow_base(&kit_place(), &base(text_hash(&carried))).unwrap();
    let held = rig.summary_of_feature().await;
    assert_eq!((held.based_on, held.armada_changed), (Some(BasedOn { version, hash: text_hash(&carried) }), false));
    let (_, body) = rig.get("workflow_id=feature&source=kit").await;
    assert_eq!(decoded::<WorkflowDefinition>("a definition", &body).based_on, Some(BasedOn { version, hash: text_hash(&carried) }));

    rig.fleet.store().lock().await.put_workflow_base(&kit_place(), &base(text_hash("{what Armada carried before}"))).unwrap();
    let moved = rig.summary_of_feature().await;
    assert_eq!((moved.based_on, moved.armada_changed), (Some(BasedOn { version, hash: text_hash("{what Armada carried before}") }), true));
}

/// **Deleting a copy drops its base and versions, and resetting it records the carried text as the new base** and begins the versions again.
#[tokio::test]
async fn deleting_a_copy_drops_its_base_and_versions_and_resetting_it_records_the_carried_text() {
    let rig = rig(&[]).await;
    let (version, carried) = carried_feature();
    async fn seed(rig: &Rig) {
        let mut store = rig.fleet.store().lock().await;
        store.record_workflow_publish(&kit_place(), None, &NewVersion { text: "{mine}", hash: "h", at: "2026-11-13T09:00:00.000Z" }, Some(&KeptBase { version: 0, hash: String::from("old"), text: String::from("{old}") })).unwrap();
    }
    async fn held(rig: &Rig) -> (Option<KeptBase>, usize) {
        let store = rig.fleet.store().lock().await;
        (store.workflow_base(&kit_place()).unwrap(), store.workflow_versions(&kit_place()).unwrap().len())
    }

    seed(&rig).await;
    rig.holds_kit_copy(&format!("{carried}\n"));
    rig.planted.will_save(Ok(saved("feature", "/kit/workflows/feature.json")));
    rig.planted.will_read(read(vec![config::Written::in_kit(at("kit", "feature.json"), carried.clone())]));
    let (status, body) = call(&rig.app, "POST", "/workflows/reset", r#"{"workflow_id":"feature","source":"kit"}"#).await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    assert_eq!(held(&rig).await, (Some(KeptBase { version, hash: text_hash(&carried), text: carried.clone() }), 0));

    seed(&rig).await;
    rig.planted.will_remove(Ok("/kit/workflows/feature.json".to_string()));
    rig.planted.will_read(read(Vec::new()));
    let (status, body) = call(&rig.app, "POST", "/workflows/delete", r#"{"workflow_id":"feature","source":"kit"}"#).await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    assert_eq!(held(&rig).await, (None, 0));
}
