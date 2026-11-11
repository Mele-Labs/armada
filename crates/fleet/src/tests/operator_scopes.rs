//! Where an Operator rule holds: a repository, a project or everywhere. A rule kept before a scope was
//! one of three still applies as it did, a project's rule covers the items of its members and no
//! others, and Always here on a page filtered to a project keeps one rule for the project.

use std::sync::Arc;

use ipc::operator_page::{OperatorAnswer, OperatorAnswerPress, OperatorRule, RuleScope};

use super::backlog::manifest;
use super::operator_asks::{filed, left, seated, FILE_IT};
use super::operator_notes::{filing, TEXT};
use super::operator_raises::{code, file_go_ahead, page, queued_as, Replies, FILE_PICK};
use super::operator_sources::Answers;
use super::session_host::{eventually, Rig};
use crate::operator::{keep_the_raises, rule_pick};

const STORE: &str = "my-store";

/// A rule as the store kept it before a scope was one of three.
async fn keep_old_rule(rig: &Rig, manifest_id: Option<&str>) {
    let held = manifest_id.map(|id| format!(r#""manifest_id":"{id}","#)).unwrap_or_default();
    let form = format!(r#"{{"kind":"pick_rule",{held}"what":"note:file","pick":"just_file"}}"#);
    let one = store::KeptOrder { n: 0, form, said: "File a note as an issue".into(), until: None, at: "2026-11-10T09:00:00.000Z".into() };
    rig.fleet.store().lock().await.add_operator_order(&one).expect("kept");
}

/// Project `my-store` planted in Kit's folder, holding `members`.
async fn store_holding(rig: &Rig, members: &[&str]) {
    let at = rig.fleet.kit_folder().join("projects").join(format!("{STORE}.yml"));
    std::fs::create_dir_all(at.parent().expect("a folder")).expect("its folder");
    std::fs::write(at, super::projects_kept::text(STORE, "My store", members)).expect("its file");
    rig.fleet.projects_reload().await;
}

async fn keep_project_rule(rig: &Rig, project: &str) {
    let rule = OperatorRule::PickRule { scope: RuleScope::Project { project: project.into() }, what: "note:file".into(), pick: "just_file".into() };
    let one = store::KeptOrder { n: 0, form: rule.kept(), said: "File a note as an issue".into(), until: None, at: "2026-11-10T09:00:00.000Z".into() };
    rig.fleet.store().lock().await.add_operator_order(&one).expect("kept");
}

#[tokio::test]
async fn a_rule_kept_with_a_manifest_id_before_scopes_still_covers_that_repository_and_reads_as_one_for_it() {
    let rig = filing().await;
    keep_old_rule(&rig, Some(manifest(&rig).as_str())).await;
    seated(&rig, Answers::with(&[FILE_IT])).await;

    left(&rig, "n-1", TEXT).await;

    eventually(|| async { filed(&rig, TEXT).await }).await;
    let learned = page(&rig).await.digest.learned;
    assert_eq!(learned.iter().map(|one| one.scope.clone()).collect::<Vec<_>>(), [RuleScope::Repository { manifest_id: manifest(&rig).as_str().to_string() }]);
}

#[tokio::test]
async fn a_rule_kept_without_one_before_scopes_still_covers_every_repository_and_reads_as_everywhere() {
    let rig = filing().await;
    keep_old_rule(&rig, None).await;
    seated(&rig, Answers::with(&[FILE_IT])).await;

    left(&rig, "n-1", TEXT).await;

    eventually(|| async { filed(&rig, TEXT).await }).await;
    let learned = page(&rig).await.digest.learned;
    assert_eq!(learned.iter().map(|one| (one.scope.clone(), one.repository.clone())).collect::<Vec<_>>(), [(RuleScope::Everywhere, None)]);
}

#[tokio::test]
async fn a_project_rule_covers_the_items_of_a_member_and_is_learned_under_the_projects_name() {
    let rig = filing().await;
    store_holding(&rig, &[manifest(&rig).as_str()]).await;
    keep_project_rule(&rig, STORE).await;
    seated(&rig, Answers::with(&[FILE_IT])).await;

    left(&rig, "n-1", TEXT).await;

    eventually(|| async { filed(&rig, TEXT).await }).await;
    assert!(page(&rig).await.turn.is_empty(), "it was not asked");
    let learned = page(&rig).await.digest.learned;
    assert_eq!((learned[0].scope.clone(), learned[0].repository.as_deref()), (RuleScope::Project { project: STORE.into() }, Some("My store")));
}

#[tokio::test]
async fn a_project_rule_does_not_cover_the_items_of_a_repository_that_is_not_a_member() {
    let rig = filing().await;
    store_holding(&rig, &["01SOMEOTHERREPOSITORY"]).await;
    keep_project_rule(&rig, STORE).await;
    seated(&rig, Answers::with(&[FILE_IT])).await;

    left(&rig, "n-1", TEXT).await;

    eventually(|| async { page(&rig).await.turn.len() == 1 }).await;
    assert!(!filed(&rig, TEXT).await, "it asks, since the project does not hold the repository");
}

fn rule(scope: RuleScope, pick: &str) -> (i64, OperatorRule) {
    (0, OperatorRule::PickRule { scope, what: "note:file".into(), pick: pick.into() })
}

#[test]
fn a_rule_for_the_repository_beats_one_for_a_project_which_beats_one_for_everywhere() {
    let everywhere = rule(RuleScope::Everywhere, "everywhere");
    let project = rule(RuleScope::Project { project: STORE.into() }, "project");
    let repository = rule(RuleScope::Repository { manifest_id: "01A".into() }, "repository");
    let held = [STORE.to_string()];
    let pick = |rules: &[(i64, OperatorRule)], repository: &str, projects: &[String]| rule_pick(rules, "note:file", repository, projects);

    assert_eq!(pick(&[everywhere.clone(), project.clone(), repository.clone()], "01A", &held).as_deref(), Some("repository"));
    assert_eq!(pick(&[everywhere.clone(), project.clone(), repository.clone()], "01B", &held).as_deref(), Some("project"));
    assert_eq!(pick(&[everywhere.clone(), project.clone(), repository.clone()], "01B", &[]).as_deref(), Some("everywhere"));
    assert_eq!(pick(&[project.clone(), repository.clone()], "01B", &[]), None, "nothing for this item");
    assert_eq!(pick(&[everywhere.clone(), project, repository], "", &[]).as_deref(), Some("everywhere"), "an item in no repository meets only everywhere");
    assert_eq!(rule_pick(&[everywhere], "red_main", "01A", &[]), None, "another kind of arrival");
}

fn always_here_in(item: &ipc::operator_page::OperatorTurnItem, project: Option<&str>) -> OperatorAnswerPress {
    OperatorAnswerPress { id: item.id.clone(), answer: OperatorAnswer::AlwaysHere, then: Some(ipc::operator_page::OperatorThen::JustFile), project: project.map(str::to_string) }
}

#[tokio::test]
async fn always_here_with_a_project_keeps_one_rule_for_the_project_and_a_second_press_replaces_it() {
    let rig = filing().await;
    store_holding(&rig, &[manifest(&rig).as_str()]).await;
    assert!(rig.fleet.operator_seated_with(Replies::saying(&[Some(FILE_PICK), Some(FILE_PICK)])));
    file_go_ahead(&rig, "n-1", TEXT).await;
    file_go_ahead(&rig, "n-2", "The sidebar should remember its width").await;
    keep_the_raises(Arc::clone(&rig.fleet)).await;
    let numbers: Vec<i64> = rig.fleet.store().lock().await.operator_suggestions_of("").expect("read").iter().map(|one| one.n).collect();
    let (first, second) = (queued_as(&rig, &format!("suggestion:{}", numbers[0])).await, queued_as(&rig, &format!("suggestion:{}", numbers[1])).await);

    let answered = rig.fleet.operator_answered(always_here_in(&first, Some(STORE))).await.expect("answered");
    let learned = &answered.page.digest.learned;
    assert_eq!(learned.len(), 1);
    assert_eq!((learned[0].scope.clone(), learned[0].repository.as_deref()), (RuleScope::Project { project: STORE.into() }, Some("My store")));

    let again = rig.fleet.operator_answered(always_here_in(&second, Some(STORE))).await.expect("answered");
    assert_eq!(again.page.digest.learned.len(), 1, "one rule for the same thing in the same place: {:#?}", again.page.digest.learned);
}

#[tokio::test]
async fn always_here_without_a_project_still_holds_in_the_items_repository() {
    let rig = filing().await;
    assert!(rig.fleet.operator_seated_with(Replies::saying(&[Some(FILE_PICK)])));
    file_go_ahead(&rig, "n-1", TEXT).await;
    keep_the_raises(Arc::clone(&rig.fleet)).await;
    let n = rig.fleet.store().lock().await.operator_suggestions_of("").expect("read")[0].n;
    let one = queued_as(&rig, &format!("suggestion:{n}")).await;

    let answered = rig.fleet.operator_answered(always_here_in(&one, None)).await.expect("answered");

    assert_eq!(answered.page.digest.learned[0].scope, RuleScope::Repository { manifest_id: manifest(&rig).as_str().to_string() });
}

#[tokio::test]
async fn always_here_in_a_project_that_does_not_hold_the_items_repository_is_refused_and_so_is_an_unknown_one() {
    let rig = filing().await;
    store_holding(&rig, &["01SOMEOTHERREPOSITORY"]).await;
    assert!(rig.fleet.operator_seated_with(Replies::saying(&[Some(FILE_PICK)])));
    file_go_ahead(&rig, "n-1", TEXT).await;
    keep_the_raises(Arc::clone(&rig.fleet)).await;
    let n = rig.fleet.store().lock().await.operator_suggestions_of("").expect("read")[0].n;
    let one = queued_as(&rig, &format!("suggestion:{n}")).await;

    let outside = rig.fleet.operator_answered(always_here_in(&one, Some(STORE))).await.expect_err("not a member");
    assert_eq!(code(outside), (422, "fleet.project_not_member".to_string()));
    let unknown = rig.fleet.operator_answered(always_here_in(&one, Some("nope"))).await.expect_err("no such project");
    assert_eq!(code(unknown), (404, "fleet.no_such_project".to_string()));
    assert_eq!(page(&rig).await.turn.len(), 1, "nothing was done");
    assert!(page(&rig).await.digest.learned.is_empty());
}
