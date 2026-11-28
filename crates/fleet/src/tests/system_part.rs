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

/// `git` in `root` as `author`.
fn git_as(root: &std::path::Path, author: &str, args: &[&str]) {
    let ran = std::process::Command::new("git")
        .args(["-c", &format!("user.name={author}"), "-c", "user.email=someone@example.com", "-C"])
        .arg(root)
        .args(args)
        .output()
        .expect("git runs");
    assert!(ran.status.success(), "git {args:?}: {}", String::from_utf8_lossy(&ran.stderr));
}

/// [`a_project_laid_out`] committed, the app by Ann and the server by Bob, and a second project listing the repository as
/// `members` does: whole, or by folders.
async fn a_project_over(rig: &Rig, members: &[Option<&str>]) -> String {
    a_project_laid_out(rig).await;
    let root = PathBuf::from(rig.fleet.repositories().first().expect("a served repository").root());
    git_as(&root, "Ann", &["init", "-q"]);
    git_as(&root, "Ann", &["add", "crates/bridge-native"]);
    git_as(&root, "Ann", &["commit", "-q", "-m", "the app"]);
    git_as(&root, "Bob", &["add", "."]);
    git_as(&root, "Bob", &["commit", "-q", "-m", "the server"]);
    let members = members.iter().map(|folder| ipc::ProjectMember { manifest_id: rig.manifest.clone(), folder: folder.map(str::to_string) }).collect();
    let new = ipc::NewProject { name: "The app".to_string(), members: Some(members), place: None, values: None };
    let id = rig.fleet.add_project(new).await.expect("added").projects.last().expect("the project").id.clone();
    let seeded = UpsertFeatures {
        project: id.clone(),
        areas: vec![AreaSeed { slug: "board".into(), title: "Board".into() }],
        features: ["jobs-list", "routing"]
            .into_iter()
            .zip([JOBS, "crates/server/src/routes.rs"])
            .map(|(slug, path)| FeatureSeed {
                slug: slug.to_string(),
                area: "board".to_string(),
                title: slug.to_string(),
                summary: "One line.".to_string(),
                state: None,
                paths: vec![path.to_string()],
                entry_points: Vec::new(),
                manifest_id: None,
            })
            .collect(),
    };
    rig.fleet.upsert_features(None, seeded).await.expect("seeded");
    id
}

#[tokio::test]
async fn a_project_over_one_folder_maps_and_answers_only_what_is_in_it() {
    let rig = rig(None);
    let project = a_project_over(&rig, &[Some("crates/bridge-native")]).await;

    let map = rig.fleet.get_system_map(project.clone(), None, None, None).await.expect("mapped");
    let ids: Vec<&str> = map.parts.iter().map(|one| one.id.as_str()).collect();
    assert!(ids.contains(&"bridge-native"), "the folder's app is a part: {ids:?}");
    assert!(!ids.contains(&"server"), "the server outside the folder is not: {ids:?}");
    assert!(map.links.iter().all(|link| link.from != "server" && link.to != "server"), "nor is anything linked to it: {:?}", map.links);
    let slugs: Vec<&str> = map.features.iter().map(|one| one.slug.as_str()).collect();
    assert_eq!(slugs, ["jobs-list"], "only a feature with a file in the folder is listed");
    match rig.fleet.get_system_map(project.clone(), None, Some("routing".to_string()), None).await {
        Err(Refusal::NotFound(why)) => assert_eq!(why.code.to_string(), "fleet.system_feature_unknown"),
        other => panic!("a feature outside the folder has no path: {other:?}"),
    }

    let all = rig.fleet.get_system_part(project.clone(), None, None).await.expect("answered");
    assert_eq!(all.code, ["crates/bridge-native/"], "the folder is the project's code");
    assert_eq!(all.features.iter().map(|one| one.slug.as_str()).collect::<Vec<_>>(), ["jobs-list"]);
    assert_eq!(all.owners.iter().map(|one| one.name.as_str()).collect::<Vec<_>>(), ["Ann"], "owners are who committed in the folder");
    assert!(all.docs.is_empty(), "the root's documents are outside the folder: {:?}", all.docs);

    let root = rig.fleet.list_system_folder(project.clone(), String::new()).await.expect("listed");
    assert_eq!(root.entries, ["crates/"], "above the folder only the way down to it is listed");
    let crates = rig.fleet.list_system_folder(project.clone(), "crates".to_string()).await.expect("listed");
    assert_eq!(crates.entries, ["bridge-native/"]);
    let inside = rig.fleet.list_system_folder(project.clone(), "crates/bridge-native".to_string()).await.expect("listed");
    assert_eq!(inside.entries, ["src/", "Cargo.toml"]);
    match rig.fleet.list_system_folder(project, "crates/server".to_string()).await {
        Err(Refusal::NotFound(why)) => assert_eq!(why.code.to_string(), "fleet.system_folder_unknown"),
        other => panic!("a folder outside the project's is refused: {other:?}"),
    }
}

#[tokio::test]
async fn a_repository_listed_whole_is_read_whole() {
    let rig = rig(None);
    let project = a_project_over(&rig, &[None]).await;

    let map = rig.fleet.get_system_map(project.clone(), None, None, None).await.expect("mapped");
    let ids: Vec<&str> = map.parts.iter().map(|one| one.id.as_str()).collect();
    assert!(ids.contains(&"bridge-native") && ids.contains(&"server"), "{ids:?}");
    assert_eq!(map.features.iter().map(|one| one.slug.as_str()).collect::<Vec<_>>(), ["jobs-list", "routing"]);
    let all = rig.fleet.get_system_part(project.clone(), None, None).await.expect("answered");
    assert_eq!(all.code, ["./"]);
    let mut owners: Vec<&str> = all.owners.iter().map(|one| one.name.as_str()).collect();
    owners.sort_unstable();
    assert_eq!(owners, ["Ann", "Bob"]);
    let root = rig.fleet.list_system_folder(project, String::new()).await.expect("listed");
    assert!(root.entries.contains(&"README.md".to_string()), "{:?}", root.entries);
}
