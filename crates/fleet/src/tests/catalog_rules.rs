//! Areas he makes, renames, splits and merges, and the rules his reasons become, written before the code: **an area act
//! moves its features with it, a merge of two features keeps what both held, and his words are kept as a rule that the
//! librarian and the Operator are handed whole.** `docs/concepts/feature-catalog.md`, *Areas* and *His rules*.

use api::{Features, Refusal};
use ipc::{AreaChange, AskLibrarian, CatalogReject, CatalogRuleAct, ChangeArea, ChangeFeature, DropCatalogRule, RejectInCatalog};
use store::{feature_link_kind, KeptFeatureLink};
use testkit::FakeJudge;

use super::feature_catalog::{a_project, grown, heard, libraried, rig, started, ACTIONS, VOTING};
use super::session_host::Rig;

fn a_job() -> Option<api::Caller> {
    Some(api::Caller::at("127.0.0.1:50000".parse().unwrap()))
}

fn code(refused: Refusal) -> (u16, String) {
    (refused.status(), refused.error().code.clone())
}

fn area(project: &str, area: &str, change: AreaChange, why: Option<&str>) -> ChangeArea {
    ChangeArea { project: project.to_string(), area: area.to_string(), change, why: why.map(String::from) }
}

fn reject(project: &str, reject: CatalogReject, features: &[&str], paths: &[&str], why: Option<&str>) -> RejectInCatalog {
    RejectInCatalog {
        project: project.to_string(),
        reject,
        features: features.iter().map(|one| one.to_string()).collect(),
        areas: Vec::new(),
        paths: paths.iter().map(|one| one.to_string()).collect(),
        why: why.map(String::from),
    }
}

/// The project's areas in their order, each with its features' slugs.
async fn shape(rig: &Rig, project: &str) -> Vec<(String, Vec<String>)> {
    let store = rig.fleet.store().lock().await;
    let features = store.features_in(project).expect("read");
    store
        .feature_areas(project)
        .expect("read")
        .into_iter()
        .map(|one| (one.slug.clone(), features.iter().filter(|feature| feature.area == one.slug).map(|feature| feature.slug.clone()).collect()))
        .collect()
}

fn owned(rows: &[(&str, &[&str])]) -> Vec<(String, Vec<String>)> {
    rows.iter().map(|(area, features)| (area.to_string(), features.iter().map(|one| one.to_string()).collect())).collect()
}

#[tokio::test]
async fn an_area_is_made_renamed_split_and_merged_and_its_features_go_with_it() {
    let rig = rig();
    let project = a_project(&rig).await;
    libraried(&rig, &project).await;

    let refused = rig.fleet.change_area(a_job(), area(&project, "boards", AreaChange::Make { title: "Boards".into() }, None)).await.expect_err("a Job keeps no area");
    assert_eq!(code(refused).1, "fleet.not_the_operator");
    let made = rig.fleet.change_area(None, area(&project, "boards", AreaChange::Make { title: "Boards".into() }, None)).await.expect("made");
    assert_eq!((made.area.as_str(), made.rule), ("boards", None), "no words keep no rule");
    for taken in ["takes", "pending"] {
        let refused = rig.fleet.change_area(None, area(&project, taken, AreaChange::Make { title: "Again".into() }, None)).await.expect_err("taken");
        assert_eq!(code(refused), (409, "fleet.area_slug_taken".to_string()), "{taken}");
    }
    assert_eq!(shape(&rig, &project).await, owned(&[("takes", &[VOTING, ACTIONS]), ("boards", &[])]), "a new area goes last");

    let same = AreaChange::Rename { slug: Some("takes".into()), title: None };
    assert_eq!(code(rig.fleet.change_area(None, area(&project, "takes", same, None)).await.expect_err("nothing changes")).1, "fleet.area_change");
    let renamed = rig.fleet.change_area(None, area(&project, "takes", AreaChange::Rename { slug: Some("pages".into()), title: Some("Pages".into()) }, None)).await.expect("renamed");
    assert_eq!(renamed.area, "pages");
    assert_eq!(shape(&rig, &project).await, owned(&[("pages", &[VOTING, ACTIONS]), ("boards", &[])]), "its features go with it");

    let split = |features: &[&str]| AreaChange::Split { slug: "actions".into(), title: "Actions".into(), features: features.iter().map(|one| one.to_string()).collect() };
    assert_eq!(code(rig.fleet.change_area(None, area(&project, "pages", split(&[]), None)).await.expect_err("names no feature")).1, "fleet.area_change");
    assert_eq!(code(rig.fleet.change_area(None, area(&project, "pages", split(&["no-such"]), None)).await.expect_err("unknown")), (404, "fleet.feature_unknown".to_string()));
    assert_eq!(code(rig.fleet.change_area(None, area(&project, "boards", split(&[ACTIONS]), None)).await.expect_err("another area's")).1, "fleet.area_change");
    assert_eq!(shape(&rig, &project).await, owned(&[("pages", &[VOTING, ACTIONS]), ("boards", &[])]), "nothing kept on a refusal");
    let parted = rig.fleet.change_area(None, area(&project, "pages", split(&[ACTIONS]), None)).await.expect("split");
    assert_eq!(parted.area, "actions");
    assert_eq!(shape(&rig, &project).await, owned(&[("pages", &[VOTING]), ("actions", &[ACTIONS]), ("boards", &[])]), "the new area sits right after the one split");

    let merge = |into: &str| AreaChange::Merge { into: into.into() };
    assert_eq!(code(rig.fleet.change_area(None, area(&project, "actions", merge("actions"), None)).await.expect_err("itself")).1, "fleet.area_change");
    assert_eq!(code(rig.fleet.change_area(None, area(&project, "actions", merge("nowhere"), None)).await.expect_err("unknown")), (422, "fleet.feature_area_unknown".to_string()));
    let merged = rig.fleet.change_area(None, area(&project, "actions", merge("pages"), None)).await.expect("merged");
    assert_eq!(merged.area, "pages");
    assert_eq!(shape(&rig, &project).await, owned(&[("pages", &[VOTING, ACTIONS]), ("boards", &[])]), "its features join the target after its own");
}

#[tokio::test]
async fn a_feature_merged_into_another_keeps_both_features_files_docs_and_jobs() {
    let rig = rig();
    let project = a_project(&rig).await;
    libraried(&rig, &project).await;
    let (gone, kept) = {
        let store = rig.fleet.store().lock().await;
        (store.feature_named(&project, ACTIONS).expect("read").expect("held").id, store.feature_named(&project, VOTING).expect("read").expect("held").id)
    };
    {
        let mut store = rig.fleet.store().lock().await;
        store.keep_feature_write(&gone, "job-1", "crates/pages/actions/press.rs", "2026-11-28T10:00:00Z").expect("kept");
        store.keep_feature_job_named(&gone, "job-1", "2026-11-28T10:00:00Z").expect("kept");
        let doc = KeptFeatureLink {
            feature_id: gone.clone(),
            kind: feature_link_kind::DOC.to_string(),
            target: "docs/actions.md".to_string(),
            job: "job-1".to_string(),
            title: "Actions".to_string(),
            state: String::new(),
            via: "read".to_string(),
            at: "2026-11-28T10:00:00Z".to_string(),
            gist: String::new(),
            gist_by: String::new(),
        };
        store.put_feature_link(&doc).expect("kept");
    }

    let merge = ChangeFeature {
        project: project.clone(),
        feature: ACTIONS.to_string(),
        into: Some(VOTING.to_string()),
        slug: None,
        title: None,
        area: None,
        why: Some("A vote is pressed as an action.".to_string()),
    };
    let done = rig.fleet.change_feature(None, merge).await.expect("merged");
    assert!(done.rule.is_some(), "his words became a rule");

    let store = rig.fleet.store().lock().await;
    let files = store.feature_files_of_job("job-1").expect("read");
    assert!(files.iter().all(|one| one.feature_id == kept) && !files.is_empty(), "its files are the target's: {files:?}");
    assert_eq!(store.feature_ids_named_by_job("job-1").expect("read"), [kept.clone()], "the Job that named it named the target");
    assert!(store.feature_links(&kept).expect("read").iter().any(|one| one.kind == feature_link_kind::DOC && one.target == "docs/actions.md"), "its doc is the target's");
    let rules = store.catalog_rules(&project).expect("read");
    assert_eq!(rules.len(), 1);
    assert_eq!((rules[0].act.as_str(), rules[0].features.clone()), ("merge_features", vec![ACTIONS.to_string(), VOTING.to_string()]));
}

#[tokio::test]
async fn his_reasons_and_rejects_are_kept_as_rules_newest_first_and_a_dropped_one_is_gone() {
    let rig = rig();
    let project = a_project(&rig).await;
    libraried(&rig, &project).await;
    rig.fleet.change_area(None, area(&project, "boards", AreaChange::Make { title: "Boards".into() }, Some("  Boards are\n their own tool. "))).await.expect("made");
    let blank = rig.fleet.change_area(None, area(&project, "lists", AreaChange::Make { title: "Lists".into() }, Some("   "))).await.expect("made");
    assert_eq!(blank.rule, None, "blank words keep no rule");

    for (refused, why) in [
        (reject(&project, CatalogReject::NotTogether, &[VOTING], &[], None), "one thing"),
        (reject(&project, CatalogReject::NotThisFeature, &[], &[], None), "nothing"),
        (reject(&project, CatalogReject::NotThisFeature, &[VOTING, ACTIONS], &["src/a.rs"], None), "paths with two features"),
    ] {
        assert_eq!(code(rig.fleet.reject_in_catalog(None, refused).await.expect_err(why)), (422, "fleet.catalog_reject".to_string()), "{why}");
    }
    let unknown = reject(&project, CatalogReject::NotTogether, &[VOTING, "no-such"], &[], None);
    assert_eq!(code(rig.fleet.reject_in_catalog(None, unknown).await.expect_err("unknown")), (404, "fleet.feature_unknown".to_string()));
    let refused = rig.fleet.reject_in_catalog(a_job(), reject(&project, CatalogReject::NotTogether, &[VOTING, ACTIONS], &[], None)).await.expect_err("a Job");
    assert_eq!(code(refused).1, "fleet.not_the_operator");

    let apart = rig.fleet.reject_in_catalog(None, reject(&project, CatalogReject::NotTogether, &[VOTING, ACTIONS], &[], None)).await.expect("kept");
    assert_eq!(apart.rule.text, format!("{VOTING} and {ACTIONS} do not go together."), "no words, so the rule says what he rejected");
    assert_eq!(apart.rule.act, CatalogRuleAct::NotTogether);
    let part = rig.fleet.reject_in_catalog(None, reject(&project, CatalogReject::NotThisFeature, &[VOTING], &["crates/pages/voting/tally.rs"], None)).await.expect("kept");
    assert_eq!(part.rule.text, format!("crates/pages/voting/tally.rs is not part of {VOTING}."));
    let worded = rig.fleet.reject_in_catalog(None, reject(&project, CatalogReject::NotThisFeature, &[ACTIONS], &[], Some("Actions are a layout, not a feature."))).await.expect("kept");
    assert_eq!(worded.rule.text, "Actions are a layout, not a feature.");

    let listed = rig.fleet.list_catalog_rules(Some(project.clone())).await.expect("read");
    let texts: Vec<&str> = listed.rules.iter().map(|one| one.text.as_str()).collect();
    assert_eq!(texts, ["Actions are a layout, not a feature.", &part.rule.text, &apart.rule.text, "Boards are their own tool."], "newest first, his words on one line");
    assert_eq!((listed.rules[3].act, listed.rules[3].areas.clone()), (CatalogRuleAct::MakeArea, vec!["boards".to_string()]));

    let left = rig.fleet.drop_catalog_rule(None, DropCatalogRule { project: project.clone(), rule: part.rule.id.clone() }).await.expect("dropped");
    assert_eq!(left.rules.len(), 3);
    assert!(left.rules.iter().all(|one| one.id != part.rule.id));
    let again = rig.fleet.drop_catalog_rule(None, DropCatalogRule { project: project.clone(), rule: part.rule.id }).await.expect_err("gone");
    assert_eq!(code(again), (404, "fleet.catalog_rule_unknown".to_string()));
    assert_eq!(code(rig.fleet.list_catalog_rules(Some("no-such-project".into())).await.expect_err("no project")).1, "fleet.feature_project");
}

#[tokio::test]
async fn the_librarian_is_asked_with_the_projects_rules_inlined() {
    let (rig, judge) = super::librarian::rig_judged(FakeJudge::saying("ACT answer\nREPLY Noted."));
    let (project, _) = super::librarian::libraried(&rig).await;
    rig.fleet
        .reject_in_catalog(None, reject(&project, CatalogReject::NotThisFeature, &["voting-on-takes"], &["crates/pages/voting/tally.rs"], Some("Tallies belong to reports.")))
        .await
        .expect("kept");
    let ask = AskLibrarian { project: project.clone(), context: "dashboard".into(), words: "Where do tallies go?".into(), files: Vec::new(), board: None };
    rig.fleet.ask_librarian(None, ask).await.expect("answered");
    let asked = judge.asked();
    assert_eq!(asked.len(), 1);
    assert!(asked[0].contains("The owner's rules for this catalog"), "the rules reach the model: {}", asked[0]);
    assert!(asked[0].contains("- Tallies belong to reports. (he said this is not the feature; features `voting-on-takes`; paths `crates/pages/voting/tally.rs`"), "{}", asked[0]);
}

#[tokio::test]
async fn a_project_with_no_rules_asks_the_librarian_as_before() {
    let (rig, judge) = super::librarian::rig_judged(FakeJudge::saying("ACT answer\nREPLY Noted."));
    let (project, _) = super::librarian::libraried(&rig).await;
    let ask = AskLibrarian { project, context: "dashboard".into(), words: "Hello".into(), files: Vec::new(), board: None };
    rig.fleet.ask_librarian(None, ask).await.expect("answered");
    assert!(!judge.asked()[0].contains("rules for this catalog"));
}

#[tokio::test]
async fn the_operators_split_tell_carries_the_projects_rules() {
    let rig = rig();
    started(&rig).await;
    let project = a_project(&rig).await;
    libraried(&rig, &project).await;
    rig.fleet.reject_in_catalog(None, reject(&project, CatalogReject::NotTogether, &[VOTING, ACTIONS], &[], Some("Votes and page actions stay apart."))).await.expect("kept");
    grown(&rig, &project, 7).await;
    rig.fleet.sources_handed().await;
    super::session_host::eventually(|| async { heard(&rig, "split") >= 1 }).await;
    assert_eq!(heard(&rig, "Votes and page actions stay apart."), 1, "the rule rides in the tell");
}
