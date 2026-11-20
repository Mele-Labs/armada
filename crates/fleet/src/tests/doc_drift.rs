//! The librarian's free drift part, written before the code: **a document linked to a feature is flagged for a name it gives
//! that the code had when the document was last changed and no longer has.** A name the code never had is the document's
//! own and is no drift. Each test lays a repository out under the rig's root and presses the librarian over it.
//! `docs/concepts/feature-catalog.md`, *Librarian*.

use std::process::Command;

use api::Features;
use ipc::{LibrarianPart, LibrarianStepState, SettingValue};
use store::{feature_link_kind, KeptFeatureLink};

use super::librarian::{libraried, pressed, rig, set, step, steps_listed, steps_told};
use super::session_host::Rig;
use crate::feature_library::doc_drift::names_in;

pub(super) fn git(root: &str, args: &[&str]) {
    let out = Command::new("git")
        .current_dir(root)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .args(["-c", "user.name=Test", "-c", "user.email=test@example.com", "-c", "commit.gpgsign=false"])
        .args(args)
        .output()
        .expect("git ran");
    assert!(out.status.success(), "git {args:?}: {}", String::from_utf8_lossy(&out.stderr));
}

pub(super) fn write(root: &str, path: &str, text: &str) {
    let at = std::path::Path::new(root).join(path);
    std::fs::create_dir_all(at.parent().expect("a parent")).expect("made");
    std::fs::write(at, text).expect("written");
}

pub(super) fn commit(root: &str, paths: &[&str], message: &str) {
    let mut add = vec!["add", "--"];
    add.extend(paths);
    git(root, &add);
    git(root, &["commit", "-q", "-m", message]);
}

const GUIDE: &str = "Call `old_function()` after `kept_function`, as `KeptThing` does.\n\nIt is served as `GET /features/page?slug=a`.\n\n```\nfenced_only_name()\n```\n\nSee `invented_name`.\n";
const CODE: &str = "pub fn old_function() {}\npub fn kept_function() {}\npub struct KeptThing;\n";
const ROUTES: &str = "router.get(\"/features/page\", page);\n";

/// A repository at the rig's root where `docs/guide.md` was last changed while the code held every name it gives, and
/// then the code dropped `old_function` and the page route.
fn repository_that_moved_on(rig: &Rig) {
    let root = rig.root.as_str();
    git(root, &["init", "-q", "-b", "main"]);
    write(root, "src/lib.rs", CODE);
    write(root, "src/routes.rs", ROUTES);
    write(root, "docs/guide.md", GUIDE);
    commit(root, &["src/lib.rs", "src/routes.rs", "docs/guide.md"], "the doc and the code");
    write(root, "src/lib.rs", "pub fn kept_function() {}\npub struct KeptThing;\n");
    write(root, "src/routes.rs", "router.get(\"/features/list\", list);\n");
    commit(root, &["src/lib.rs", "src/routes.rs"], "the code moves on");
}

/// A link of `kind` from `feature` to `target`, as a Job's read files it.
async fn linked(rig: &Rig, feature: &str, kind: &str, target: &str) {
    let link = KeptFeatureLink {
        feature_id: feature.to_string(),
        kind: kind.to_string(),
        target: target.to_string(),
        job: "j1".to_string(),
        title: String::new(),
        gist: String::new(),
        gist_by: String::new(),
        state: String::new(),
        via: "read".to_string(),
        at: "2026-11-19T08:00:00.000Z".to_string(),
    };
    rig.fleet.store().lock().await.put_feature_link(&link).expect("kept");
}

/// Only the drift part is on, so no model is asked.
async fn drift_only(rig: &Rig) {
    set(rig, "features.librarianParts", SettingValue::List(vec![SettingValue::Text("drift".into())])).await;
}

async fn drift_kept(rig: &Rig, project: &str) -> Vec<(String, Vec<String>)> {
    rig.fleet.store().lock().await.doc_drift_in(project).expect("read").into_iter().map(|one| (one.target, one.names)).collect()
}

fn names(list: &[&str]) -> Vec<String> {
    list.iter().map(|one| one.to_string()).collect()
}

#[tokio::test]
async fn a_doc_naming_what_was_removed_after_its_last_change_is_flagged_with_those_names_and_the_page_says_so() {
    let rig = rig();
    let (project, feature) = libraried(&rig).await;
    repository_that_moved_on(&rig);
    drift_only(&rig).await;
    linked(&rig, &feature, feature_link_kind::DOC, "docs/guide.md").await;

    let ran = pressed(&rig, &project).await;
    assert_eq!(ran.ran, [LibrarianPart::Drift]);
    assert_eq!(
        drift_kept(&rig, &project).await,
        [("docs/guide.md".to_string(), names(&["old_function", "/features/page"]))],
        "the function and the route went; the type stayed, the fenced name and the invented one were never counted"
    );
    let page = rig.fleet.get_feature_page(None, Some(project), "voting-on-takes".to_string(), None, false).await.expect("the page");
    assert_eq!(page.docs.len(), 1);
    assert_eq!(page.docs[0].drifted, names(&["old_function", "/features/page"]));
}

#[tokio::test]
async fn a_doc_linked_by_its_absolute_path_under_the_repository_is_read_like_one_linked_by_a_relative_path() {
    let rig = rig();
    let (project, feature) = libraried(&rig).await;
    repository_that_moved_on(&rig);
    drift_only(&rig).await;
    let canonical = std::fs::canonicalize(&rig.root).expect("the root");
    linked(&rig, &feature, feature_link_kind::DOC, &canonical.join("docs/guide.md").to_string_lossy()).await;

    pressed(&rig, &project).await;
    let kept = drift_kept(&rig, &project).await;
    assert_eq!(kept.len(), 1);
    assert_eq!(kept[0].1, names(&["old_function", "/features/page"]));
}

#[tokio::test]
async fn a_doc_naming_only_what_never_existed_or_what_still_exists_is_not_flagged() {
    let rig = rig();
    let (project, feature) = libraried(&rig).await;
    repository_that_moved_on(&rig);
    drift_only(&rig).await;
    let root = rig.root.as_str();
    write(root, "docs/invented.md", "It will have `never_existed_fn()` and `NeverThing`, served at `/never/served`.\n");
    write(root, "docs/whole.md", "It has `kept_function` and `KeptThing`.\n");
    commit(root, &["docs/invented.md", "docs/whole.md"], "two docs");
    linked(&rig, &feature, feature_link_kind::DOC, "docs/invented.md").await;
    linked(&rig, &feature, feature_link_kind::DOC, "docs/whole.md").await;

    let ran = pressed(&rig, &project).await;
    assert_eq!(ran.ran, [LibrarianPart::Drift], "the part ran");
    assert_eq!(drift_kept(&rig, &project).await, Vec::<(String, Vec<String>)>::new());
}

#[tokio::test]
async fn a_flag_clears_when_a_later_run_finds_nothing() {
    let rig = rig();
    let (project, feature) = libraried(&rig).await;
    repository_that_moved_on(&rig);
    drift_only(&rig).await;
    linked(&rig, &feature, feature_link_kind::DOC, "docs/guide.md").await;
    pressed(&rig, &project).await;
    assert_eq!(drift_kept(&rig, &project).await.len(), 1, "flagged");

    // Nothing moved, so the same finding stands.
    pressed(&rig, &project).await;
    assert_eq!(drift_kept(&rig, &project).await.len(), 1, "still flagged");

    // The doc is changed to say what the code says now, so what it names was never in the code when it was last changed.
    write(rig.root.as_str(), "docs/guide.md", "Call `kept_function`, as `KeptThing` does.\n");
    commit(rig.root.as_str(), &["docs/guide.md"], "the doc follows the code");
    pressed(&rig, &project).await;
    assert_eq!(drift_kept(&rig, &project).await, Vec::<(String, Vec<String>)>::new(), "cleared");
    let page = rig.fleet.get_feature_page(None, Some(project), "voting-on-takes".to_string(), None, false).await.expect("the page");
    assert!(page.docs[0].drifted.is_empty());
}

#[tokio::test]
async fn an_address_a_ticket_and_a_doc_outside_the_repository_are_skipped() {
    let rig = rig();
    let (project, feature) = libraried(&rig).await;
    repository_that_moved_on(&rig);
    drift_only(&rig).await;
    linked(&rig, &feature, feature_link_kind::DOC, "https://example.com/docs/guide.md").await;
    linked(&rig, &feature, feature_link_kind::TICKET, "docs/guide.md").await;
    linked(&rig, &feature, feature_link_kind::DOC, "/nowhere/else/guide.md").await;
    linked(&rig, &feature, feature_link_kind::DOC, "docs/not_in_git.md").await;

    pressed(&rig, &project).await;
    assert_eq!(drift_kept(&rig, &project).await, Vec::<(String, Vec<String>)>::new());
}

#[tokio::test]
async fn the_step_appears_only_where_drift_is_on_and_says_how_many_docs_name_what_is_gone() {
    let rig = rig();
    let (project, feature) = libraried(&rig).await;
    repository_that_moved_on(&rig);
    write(rig.root.as_str(), "docs/whole.md", "It has `kept_function`.\n");
    commit(rig.root.as_str(), &["docs/whole.md"], "a second doc");
    linked(&rig, &feature, feature_link_kind::DOC, "docs/guide.md").await;
    linked(&rig, &feature, feature_link_kind::DOC, "docs/whole.md").await;
    linked(&rig, &feature, feature_link_kind::DOC, "https://example.com/spec").await;

    set(&rig, "features.librarianParts", SettingValue::List(vec![SettingValue::Text("floor_plans".into())])).await;
    let ids: Vec<String> = steps_listed(&rig, &project).await.into_iter().map(|(id, _, _)| id).collect();
    assert_eq!(ids, ["read_merges", "read_through"], "drift is off");

    drift_only(&rig).await;
    let listed = rig.fleet.list_features(None, Some(project.clone())).await.expect("listed").librarian.steps;
    let ids: Vec<&str> = listed.iter().map(|one| one.id.as_str()).collect();
    assert_eq!(ids, ["read_merges", "check_drift", "read_through"], "after the merges, before anything that asks a model");
    assert_eq!(
        (listed[1].title.as_str(), listed[1].detail.as_str(), listed[1].model),
        ("Check docs against the code", "The 2 docs linked to features, against the code at HEAD", false)
    );

    let mut watching = rig.fleet.events().subscribe();
    pressed(&rig, &project).await;
    let told = steps_told(&mut watching, &project).await;
    assert_eq!(told[2], step("check_drift", LibrarianStepState::Running, ""));
    assert_eq!(told[3], step("check_drift", LibrarianStepState::Done, "1 doc names what the code no longer has"));

    // A run over a project whose docs are whole says so.
    write(rig.root.as_str(), "docs/guide.md", "Call `kept_function`.\n");
    commit(rig.root.as_str(), &["docs/guide.md"], "the doc follows the code");
    let mut watching = rig.fleet.events().subscribe();
    pressed(&rig, &project).await;
    let told = steps_told(&mut watching, &project).await;
    assert_eq!(told[3], step("check_drift", LibrarianStepState::Done, "No doc names what the code no longer has"));
}

#[test]
fn names_come_from_inline_code_only_as_identifiers_or_endpoints_of_four_characters_or_more() {
    let markdown = "Use `old_function()` and `Voting::rank_all` with `ranker::Ranker`, `JobCard` and `GET /api/v1/features?x=1`.\n\
                    Not these: `foo.rs`, `src/lib.rs`, `two words`, `go()`, `Plain`, `lowercase`, `HTTP`, `a_b`, `/ws`, `/v1`, `/docs/guide.md`, `Vec<String>`.\n\
                    ```rust\nfenced_name()\n```\n\
                    ~~~\nother_fenced\n~~~\n\
                    After the fences, `old_function` again and ``double_tick_name`` once.\n";
    assert_eq!(names_in(markdown), names(&["old_function", "rank_all", "Ranker", "JobCard", "/api/v1/features", "double_tick_name"]));
}

#[test]
fn a_document_names_at_most_forty_names() {
    let markdown: String = (0..60).map(|n| format!("`name_{n}` ")).collect();
    let found = names_in(&markdown);
    assert_eq!((found.len(), found[0].as_str(), found[39].as_str()), (40, "name_0", "name_39"));
}
