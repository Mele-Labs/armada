//! Starting a library, written before the code: **a project with no feature of its own is read for the modules its build
//! files name and the docs at the top of its repository, and his Draft as Pending keeps each module as a pending feature**,
//! once, with its folder as its prefix. `docs/concepts/feature-catalog.md`, *Starting a library*.

use api::{Features, Projects};
use ipc::{AreaSeed, DraftModules, FeatureSeed, FeatureState, NewProject, ProjectMember, UpsertFeatures};

use super::doc_drift::write;
use super::librarian::rig;
use super::session_host::Rig;

/// A project of the rig's repository holding no feature, with two crates, a package, a README and a docs folder.
async fn bare(rig: &Rig) -> String {
    let members = Some(vec![ProjectMember { manifest_id: rig.manifest.clone(), folder: None }]);
    let project = rig.fleet.add_project(NewProject { name: "Ledger".to_string(), members, place: None, values: None }).await.expect("added").projects.last().expect("the project").id.clone();
    let root = rig.root.as_str();
    write(root, "crates/api/Cargo.toml", "[package]\nname = \"ledger_api\"\n");
    write(root, "crates/core/Cargo.toml", "[package]\nname = \"ledger-core\"\n");
    write(root, "web/package.json", "{\"name\": \"@ledger/web\"}");
    write(root, "README.md", "# Ledger\n");
    write(root, "docs/one.md", "# One\n");
    project
}

async fn drafted(rig: &Rig, project: &str) -> Result<ipc::ModulesDrafted, api::Refusal> {
    rig.fleet.draft_module_features(None, DraftModules { project: project.to_string() }).await
}

#[tokio::test]
async fn a_bare_project_reads_its_modules_with_their_slugs_and_the_docs_at_its_top() {
    let rig = rig();
    let project = bare(&rig).await;
    let start = rig.fleet.get_library_start(None, Some(project.clone())).await.expect("read");
    let modules: Vec<(&str, &str, &str, bool)> = start.modules.iter().map(|one| (one.name.as_str(), one.slug.as_str(), one.path.as_str(), one.held)).collect();
    assert_eq!(modules, [("ledger_api", "ledger-api", "crates/api", false), ("ledger-core", "ledger-core", "crates/core", false), ("@ledger/web", "ledger-web", "web", false)]);
    assert_eq!(start.docs, ["README.md", "docs/"]);
}

#[tokio::test]
async fn a_press_keeps_each_module_as_a_pending_feature_with_its_folder_and_an_area_to_confirm_it_into() {
    let rig = rig();
    let project = bare(&rig).await;
    let kept = drafted(&rig, &project).await.expect("drafted");
    assert_eq!(kept.drafted, ["ledger-api", "ledger-core", "ledger-web"]);
    assert!(kept.held.is_empty());

    let library = rig.fleet.list_features(None, Some(project.clone())).await.expect("listed");
    let areas: Vec<(&str, usize)> = library.areas.iter().map(|area| (area.slug.as_str(), area.features.len())).collect();
    assert_eq!(areas, [("modules", 0), ("pending", 3)], "an empty area to confirm into, and the drafts pending");
    let rows = &library.areas[1].features;
    assert!(rows.iter().all(|row| row.state == FeatureState::Pending));
    assert_eq!(rows[0].title, "ledger_api");
    let api = rig.fleet.store().lock().await.feature_named(&project, "ledger-api").expect("read").expect("held");
    assert_eq!((api.paths, api.manifest_id.as_str()), (vec!["crates/api".to_string()], rig.manifest.as_str()));

    let start = rig.fleet.get_library_start(None, Some(project.clone())).await.expect("read");
    assert!(start.modules.iter().all(|one| one.held), "each module's slug is held now");
}

#[tokio::test]
async fn a_second_press_drafts_nothing_twice() {
    let rig = rig();
    let project = bare(&rig).await;
    drafted(&rig, &project).await.expect("drafted");
    write(rig.root.as_str(), "crates/cli/Cargo.toml", "[package]\nname = \"ledger-cli\"\n");
    let again = drafted(&rig, &project).await.expect("drafted again");
    assert_eq!(again.drafted, ["ledger-cli"], "only the module that came since");
    assert_eq!(again.held, ["ledger-api", "ledger-core", "ledger-web"]);
    let features = rig.fleet.store().lock().await.features_in(&project).expect("read");
    assert_eq!(features.len(), 4);
}

#[tokio::test]
async fn a_project_with_a_feature_of_its_own_is_refused_in_a_sentence_and_nothing_is_kept() {
    let rig = rig();
    let project = bare(&rig).await;
    let upsert = UpsertFeatures {
        project: project.clone(),
        areas: vec![AreaSeed { slug: "books".into(), title: "Books".into() }],
        features: vec![FeatureSeed { slug: "posting".into(), area: "books".into(), title: "Posting".into(), summary: "One line.".into(), state: None, paths: Vec::new(), entry_points: Vec::new(), manifest_id: None }],
    };
    rig.fleet.upsert_features(None, upsert).await.expect("kept");
    let refused = drafted(&rig, &project).await.expect_err("started already");
    assert_eq!(refused.error().code, "fleet.library_started");
    assert_eq!(refused.error().message, "This project holds features of its own, so its modules are not drafted.");
    assert_eq!(rig.fleet.store().lock().await.features_in(&project).expect("read").len(), 1);
}

#[tokio::test]
async fn a_project_fleet_does_not_hold_is_refused() {
    let rig = rig();
    let refused = drafted(&rig, "nowhere").await.expect_err("no such project");
    assert_eq!(refused.error().code, "fleet.feature_project");
    let refused = rig.fleet.get_library_start(None, Some("nowhere".into())).await.expect_err("no such project");
    assert_eq!(refused.error().code, "fleet.feature_project");
}
