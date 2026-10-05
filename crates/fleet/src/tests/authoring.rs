//! Saving a workflow through Fleet, and reading the folders again.
//!
//! What a save checks and writes is the composition root's and is tested there
//! against real files (`armada::tests::authoring`). What is Fleet's is the
//! answer it makes of a refusal, and that a definition is held the moment the
//! save answers.

use std::sync::Arc;

use axum::http::StatusCode;

use crate::repositories::{Catalogued, SavedWorkflow, WorkflowNotSaved};
use crate::tests::daemon::{one, workflow_named};
use crate::tests::http::call;
use crate::tests::repositories::{a_fleet_reading, code, served, Planted};
use crate::tests::tmp::TempDir;

const SAVE: &str = r#"{"scope":"kit","definition":"{}"}"#;

async fn held(app: &axum::Router) -> Vec<String> {
    let (_, body) = call(app, "GET", "/workflows", "").await;
    let listed: Vec<ipc::WorkflowSummary> = ipc::decode("workflows", &body).expect("a list");
    let mut ids: Vec<String> = listed.iter().map(|w| w.id.as_str().to_string()).collect();
    ids.sort();
    ids
}

/// **A definition that does not fit is refused with the loader's reason, and
/// nothing is held.** The write is the composition root's, so what is checked
/// here is that Fleet does not go on to read the folders and take on whatever
/// is there.
#[tokio::test]
async fn a_definition_that_does_not_fit_is_refused_with_its_reason_and_holds_nothing() {
    let home = TempDir::new();
    let planted = Arc::new(Planted::nothing());
    let fleet = Arc::new(a_fleet_reading(&home, &planted));
    let app = served(&fleet);
    let before = held(&app).await;
    planted.will_save(Err(WorkflowNotSaved::Unfit {
        why: "step `build` needs `lint`, and the declared Checks are `test`".to_string(),
    }));
    planted.will_read(Catalogued {
        workflows: one(workflow_named("not-yet")),
        left_out: Vec::new(),
    });

    let (status, body) = call(&app, "POST", "/workflows/save", SAVE).await;

    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(code(&body), "fleet.workflow_unfit");
    assert!(String::from_utf8_lossy(&body).contains("needs `lint`"));
    assert_eq!(held(&app).await, before, "the folders were not read again");
    assert_eq!(planted.saves().len(), 1);
}

#[tokio::test]
async fn replacing_without_saying_so_is_a_422_naming_the_file() {
    let home = TempDir::new();
    let planted = Arc::new(Planted::nothing());
    let fleet = Arc::new(a_fleet_reading(&home, &planted));
    let app = served(&fleet);
    planted.will_save(Err(WorkflowNotSaved::Exists {
        id: "bug".to_string(),
        file: "/repo/.armada/workflows/bug.json".to_string(),
    }));

    let (status, body) = call(&app, "POST", "/workflows/save", SAVE).await;

    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(code(&body), "fleet.workflow_exists");
    assert!(String::from_utf8_lossy(&body).contains("/repo/.armada/workflows/bug.json"));
}

/// **Held the moment the save answers**, without a restart and without waiting
/// for the watch.
#[tokio::test]
async fn a_saved_definition_is_held_when_the_save_answers() {
    let home = TempDir::new();
    let planted = Arc::new(Planted::nothing());
    let fleet = Arc::new(a_fleet_reading(&home, &planted));
    let app = served(&fleet);
    let mut now = one(workflow_named("fixture-workflow"));
    now.extend(one(workflow_named("fresh")));
    planted.will_save(Ok(SavedWorkflow {
        id: workflow_named("fresh").id().clone(),
        file: "/home/.armada/workflows/fresh.json".to_string(),
        replaced: false,
    }));
    planted.will_read(Catalogued {
        workflows: now,
        left_out: Vec::new(),
    });

    let (status, body) = call(&app, "POST", "/workflows/save", SAVE).await;

    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    let saved: ipc::WorkflowSaved = ipc::decode("a save", &body).expect("a save");
    assert_eq!(saved.workflow_id.as_str(), "fresh");
    assert_eq!(saved.scope, ipc::WorkflowScope::Kit);
    assert!(!saved.replaced);
    assert!(saved.runs_from.is_some());
    assert!(held(&app).await.contains(&"fresh".to_string()));
}

/// A re-read replaces the set whole: a definition whose file is gone is gone,
/// and what was set aside is what the re-read set aside.
#[tokio::test]
async fn reading_the_folders_again_replaces_what_was_held() {
    let home = TempDir::new();
    let planted = Arc::new(Planted::nothing());
    let fleet = Arc::new(a_fleet_reading(&home, &planted));
    let app = served(&fleet);
    planted.will_read(Catalogued {
        workflows: one(workflow_named("only-this")),
        left_out: vec![ipc::LeftOutWorkflow {
            id: None,
            source: "repository".to_string(),
            file: "/repo/.armada/workflows/broken.json".to_string(),
            said: "a definition repository was left out, because it will not parse".to_string(),
            instead: None,
        }],
    });

    fleet.workflows_changed(&home.path().to_string_lossy());

    assert_eq!(held(&app).await, ["only-this"]);
    let (_, body) = call(&app, "GET", "/workflows/left_out", "").await;
    let left: Vec<ipc::LeftOutWorkflow> = ipc::decode("left out", &body).expect("a list");
    assert_eq!(left.len(), 1);
    assert_eq!(left[0].source, "repository");
}
