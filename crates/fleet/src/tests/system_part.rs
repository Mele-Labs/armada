//! Every level of the System block shows the same nine sections: a part or a piece of one, a feature, and the whole project
//! when nothing is selected. Each test lays a repository out in the served checkout and asks Fleet as Bridge does.

use std::path::PathBuf;

use api::{Features, Projects, Refusal};
use ipc::{AreaSeed, FeatureSeed, UpsertFeatures};

use super::agent_starts::{rig, Rig};
use super::feature_library::{a_project, an_agent_for, wrote};
use super::system_map::{cargo, write};

const JOBS: &str = "crates/bridge-native/src/jobs.rs";

/// A native app with a screen, a server behind it, the documents at the root and two features.
async fn a_project_laid_out(rig: &Rig) -> String {
    let root = PathBuf::from(rig.fleet.repositories().first().expect("a served repository").root());
    write(&root, "crates/bridge-native/Cargo.toml", &cargo("bridge-native", "gpui = \"1\""));
    write(&root, "crates/bridge-native/src/main.rs", "fn main() {}\n");
    write(&root, JOBS, "//! The Jobs list.\nfn draw() {}\n");
    write(&root, "crates/bridge-native/src/titlebar.rs", "//! The title bar.\nfn draw() {}\n");
    write(&root, "crates/server/Cargo.toml", &cargo("server", "axum = \"0.7\""));
    write(&root, "crates/server/src/main.rs", "fn main() {}\n");
    write(&root, "crates/server/src/routes.rs", "//! The routes.\nfn routes() {}\n");
    write(&root, "README.md", "# The project\n");
    write(&root, "docs/guide.md", "How it works.\n");
    let project = a_project(rig).await;
    let feature = |slug: &str, paths: &[&str]| FeatureSeed {
        slug: slug.to_string(),
        area: "board".to_string(),
        title: slug.to_string(),
        summary: "One line.".to_string(),
        state: None,
        paths: paths.iter().map(|one| one.to_string()).collect(),
        entry_points: Vec::new(),
        manifest_id: None,
    };
    let seeded = UpsertFeatures {
        project: project.clone(),
        areas: vec![AreaSeed { slug: "board".into(), title: "Board".into() }],
        features: vec![feature("jobs-list", &[JOBS]), feature("routing", &["crates/server/src/routes.rs"])],
    };
    rig.fleet.upsert_features(None, seeded).await.expect("seeded");
    project
}

#[tokio::test]
async fn nothing_selected_answers_the_whole_project_in_the_same_nine_sections() {
    let rig = rig(None);
    let project = a_project_laid_out(&rig).await;
    let id = an_agent_for(&rig, &project).await;
    wrote(&rig, &id, JOBS).await;

    let all = rig.fleet.get_system_part(project, None, None).await.expect("answered");
    assert_eq!(all.part, "", "no part is named for the whole project");
    assert!(all.feature.is_none());
    assert_eq!(all.code, ["./"], "the repository's root is its code");
    let slugs: Vec<&str> = all.features.iter().map(|one| one.slug.as_str()).collect();
    assert_eq!(slugs, ["jobs-list", "routing"], "every feature, each with the parts it falls in");
    assert_eq!(all.features[0].parts, ["bridge-native"]);
    assert_eq!(all.docs, ["README.md", "docs/guide.md"], "the documents at the root, README first");
    assert_eq!(all.working.len(), 1, "a Job writing anywhere in it is working on it");
    assert!(all.has_screen, "a project with a native app draws a screen");
    assert_eq!(all.owner_days, 30);
}

#[tokio::test]
async fn a_feature_answers_its_own_files_jobs_and_screen() {
    let rig = rig(None);
    let project = a_project_laid_out(&rig).await;
    let id = an_agent_for(&rig, &project).await;
    wrote(&rig, &id, JOBS).await;

    let one = rig.fleet.get_system_part(project.clone(), None, Some("jobs-list".to_string())).await.expect("answered");
    assert_eq!(one.part, "");
    assert_eq!(one.feature.as_deref(), Some("jobs-list"));
    assert_eq!(one.code, [JOBS], "its prefixes are its code");
    assert_eq!(one.features.iter().map(|each| each.slug.as_str()).collect::<Vec<_>>(), ["jobs-list"]);
    assert_eq!(one.working.len(), 1, "the Job that wrote its file");
    assert!(one.has_screen, "its file is in the native app");

    let other = rig.fleet.get_system_part(project.clone(), None, Some("routing".to_string())).await.expect("answered");
    assert!(other.working.is_empty(), "a Job writing another feature's file is not working on this one");
    assert!(!other.has_screen, "a server draws nothing");

    match rig.fleet.get_system_part(project, None, Some("nothing-of-the-kind".to_string())).await {
        Err(Refusal::NotFound(why)) => assert_eq!(why.code.to_string(), "fleet.system_feature_unknown"),
        other => panic!("{other:?}"),
    }
}

#[tokio::test]
async fn a_piece_within_an_app_that_draws_a_screen_has_one() {
    let rig = rig(None);
    let project = a_project_laid_out(&rig).await;

    let jobs = rig.fleet.get_system_part(project.clone(), Some("bridge-native/jobs".to_string()), None).await.expect("answered");
    assert_eq!(jobs.part, "bridge-native/jobs");
    assert!(jobs.has_screen, "the Jobs piece of a native app is a screen");

    let routes = rig.fleet.get_system_part(project.clone(), Some("server/routes".to_string()), None).await.expect("answered");
    assert!(!routes.has_screen, "a piece of a server is not");

    let app = rig.fleet.get_system_part(project, Some("bridge-native".to_string()), None).await.expect("answered");
    assert!(app.has_screen);
}
