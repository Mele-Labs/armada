//! A feature's health, written before the code: **a mark always carries its reasons, and a source Fleet does not keep is
//! not guessed.** A flaky test that reaches a feature makes it `watch`, a stale section is a reason, a failed check in its
//! files makes it `poor`, and a feature nothing speaks against is `good` with no reasons.

use api::Features;
use core_model::{EdgeGrain, EdgeSource, TestMapEdge};
use std::collections::BTreeSet;

use ipc::{FeatureRow, FeatureSectionKind, HealthMark, HealthReasonKind};
use store::{AttachmentState, Holder, KeptAttachment, KeptFeatureMerge};

use super::agent_starts::{caller, rig, Rig};
use super::feature_library::{a_project, an_agent_for, held_slot, seeded, write_all, wrote, SOMEWHERE, VOTING};

const FLAKY: &str = "tests::voting::ranks_settle";

fn edge(rig: &Rig, test: &str, file: &str) -> TestMapEdge {
    TestMapEdge {
        repository: rig.manifest.to_domain(),
        check: "test".to_string(),
        member: String::new(),
        test: test.to_string(),
        file: file.to_string(),
        source: EdgeSource::Declared,
        grain: EdgeGrain::Test,
        commit: "abc".to_string(),
        at: rig.fleet.now(),
        stale: false,
    }
}

async fn row_of(rig: &Rig, project: &str, slug: &str) -> FeatureRow {
    let listed = rig.fleet.list_features(None, Some(project.to_string())).await.expect("listed");
    listed.areas.into_iter().flat_map(|area| area.features).find(|one| one.slug == slug).expect("the feature")
}

#[tokio::test]
async fn a_feature_nothing_speaks_against_is_good_with_no_reasons_and_an_unfed_test_map_is_not_a_reason() {
    let rig = rig(None);
    let project = a_project(&rig).await;
    rig.fleet.upsert_features(None, seeded(&project)).await.expect("seeded");

    let row = row_of(&rig, &project, VOTING).await;
    assert_eq!(row.health.mark, HealthMark::Good);
    assert!(row.health.reasons.is_empty(), "{:?}", row.health.reasons);
}

#[tokio::test]
async fn a_flaky_test_that_reaches_a_feature_makes_it_watch_and_names_the_test() {
    let rig = rig(None);
    let project = a_project(&rig).await;
    rig.fleet.upsert_features(None, seeded(&project)).await.expect("seeded");
    {
        let mut store = rig.fleet.store().lock().await;
        store.record_test_map_edges(&[edge(&rig, FLAKY, &format!("{SOMEWHERE}/ranks.rs"))]).expect("kept");
        let now = rig.fleet.now().as_str().to_string();
        let row = KeptAttachment {
            holder: Holder::session("fixer"),
            kind: crate::flaky_tests::TEST.into(),
            manifest_id: rig.manifest.as_str().to_string(),
            target: FLAKY.to_string(),
            state: AttachmentState::Standing,
            detail: Default::default(),
            since: now.clone(),
            changed_at: now,
        };
        store.attach(&row, false).expect("attached");
    }

    let row = row_of(&rig, &project, VOTING).await;
    assert_eq!(row.health.mark, HealthMark::Watch);
    let reasons: Vec<_> = row.health.reasons.iter().map(|one| (one.kind, one.fact.as_str(), one.target.as_deref())).collect();
    assert_eq!(reasons, [(HealthReasonKind::FlakyTest, "flaky test ranks_settle reaches it", Some(FLAKY))]);

    let other = row_of(&rig, &project, "page-actions").await;
    assert_eq!(other.health.mark, HealthMark::Watch, "the map holds edges and none reaches its files");
    assert_eq!(other.health.reasons.iter().map(|one| one.kind).collect::<Vec<_>>(), [HealthReasonKind::NoTestReaches]);
}

#[tokio::test]
async fn a_stale_test_section_adds_a_reason_with_the_merge_it_is_stale_since() {
    let rig = rig(None);
    let project = a_project(&rig).await;
    let id = an_agent_for(&rig, &project).await;
    rig.fleet.upsert_features(None, seeded(&project)).await.expect("seeded");
    wrote(&rig, &id, &format!("{SOMEWHERE}/ranks.rs")).await;
    held_slot(&rig, &id, "4").await;
    rig.fleet.write_feature_sections(caller(), write_all(&id, VOTING, &[FeatureSectionKind::Test])).await.expect("written");
    assert_eq!(row_of(&rig, &project, VOTING).await.health.mark, HealthMark::Good, "a fresh section is no reason");

    rig.fleet
        .store()
        .lock()
        .await
        .keep_feature_merge(&KeptFeatureMerge {
            manifest_id: rig.manifest.as_str().to_string(),
            number: 900,
            merged_at: "2999-01-01T00:00:00.000Z".to_string(),
            at_commit: String::new(),
            job: "s-other".into(),
            files: vec![format!("{SOMEWHERE}/other.rs")],
        })
        .expect("kept");

    let page = rig.fleet.get_feature_page(None, Some(project), VOTING.to_string(), None, false).await.expect("the page");
    assert_eq!(page.feature.health.mark, HealthMark::Watch);
    let reason = &page.feature.health.reasons[0];
    assert_eq!((reason.kind, reason.fact.as_str(), reason.target.as_deref()), (HealthReasonKind::StaleSection, "test section stale since #900", Some("900")));
}

#[tokio::test]
async fn a_check_that_failed_on_its_last_run_in_a_job_working_in_its_files_makes_it_poor() {
    let rig = rig(None);
    let project = a_project(&rig).await;
    let id = an_agent_for(&rig, &project).await;
    rig.fleet.upsert_features(None, seeded(&project)).await.expect("seeded");
    wrote(&rig, &id, &format!("{SOMEWHERE}/ranks.rs")).await;
    held_slot(&rig, &id, "4").await;
    {
        let mut store = rig.fleet.store().lock().await;
        let at = "2026-11-18T10:00:00.000Z";
        let failed = store.start_session_check(id.as_str(), rig.manifest.as_str(), 4, "clippy", None, at).expect("started");
        store.end_session_check(failed, "failed", at, 10, "", None).expect("ended");
        // A Check that failed and then passed is not failing.
        let first = store.start_session_check(id.as_str(), rig.manifest.as_str(), 4, "test", None, at).expect("started");
        store.end_session_check(first, "failed", at, 10, "", None).expect("ended");
        let again = store.start_session_check(id.as_str(), rig.manifest.as_str(), 4, "test", None, at).expect("started");
        store.end_session_check(again, "passed", at, 10, "", None).expect("ended");
    }

    let row = row_of(&rig, &project, VOTING).await;
    assert_eq!(row.health.mark, HealthMark::Poor);
    let reasons: Vec<_> = row.health.reasons.iter().map(|one| (one.kind, one.target.as_deref())).collect();
    assert_eq!(reasons, [(HealthReasonKind::AnalyzerFailed, Some(id.as_str()))]);
    assert!(row.health.reasons[0].fact.starts_with("clippy failed on the last run"), "{}", row.health.reasons[0].fact);

    let untouched = row_of(&rig, &project, "page-actions").await;
    assert_eq!(untouched.health.mark, HealthMark::Good, "a Job in other files says nothing of it");
}

fn kept(id: &str, manifest: &str, paths: &[&str]) -> store::KeptFeature {
    store::KeptFeature {
        id: id.to_string(),
        project: "p1".to_string(),
        area: "takes".to_string(),
        slug: id.to_string(),
        title: id.to_string(),
        summary: String::new(),
        state: "seeded".to_string(),
        manifest_id: manifest.to_string(),
        entry_points: Vec::new(),
        paths: paths.iter().map(|one| one.to_string()).collect(),
        position: 0,
        added_at: "2026-11-16T09:00:00.000Z".to_string(),
    }
}

#[test]
fn the_features_owning_any_of_a_set_of_files_are_those_that_own_one_of_them_file_by_file() {
    use crate::feature_library::mapping::FeatureMap;
    let map = FeatureMap::new(
        vec![
            kept("wide", "", &["crates/fleet"]),
            kept("narrow", "", &["crates/fleet/src/votes"]),
            kept("twin", "", &["crates/fleet/src/votes/"]),
            kept("exact", "", &["crates/pages/one.rs"]),
            kept("filed", "", &[]),
            kept("elsewhere", "m-2", &["crates/fleet"]),
            kept("near", "", &["crates/fleet-extra"]),
        ],
        [("filed".to_string(), "docs/a.md".to_string())],
    );
    let all = ["crates/fleet/src/other.rs", "crates/fleet/src/votes/one.rs", "crates/pages/one.rs", "crates/pages/two.rs", "docs/a.md", "docs/b.md", "crates/fleet-extra/a.rs"];
    for manifest in [Some("m-1"), Some("m-2"), None] {
        for one in all {
            let files: BTreeSet<String> = [one.to_string()].into();
            let by_file: BTreeSet<String> = map.owners(manifest, one).iter().map(|owner| owner.feature.id.clone()).collect();
            assert_eq!(map.owning_any(manifest, &files), by_file, "{one} in {manifest:?}");
        }
        let files: BTreeSet<String> = all.iter().map(|one| one.to_string()).collect();
        let by_file: BTreeSet<String> = all.iter().flat_map(|one| map.owners(manifest, one)).map(|owner| owner.feature.id.clone()).collect();
        assert_eq!(map.owning_any(manifest, &files), by_file, "all of them in {manifest:?}");
    }
}
