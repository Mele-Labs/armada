//! The feature library, written before the code: **a Job that changed a feature's files owes that feature's sections
//! before it ends, and what it owes is named.** Files are mapped by prefix, longest first; a section reads as stale
//! since the first other Job's merge that changed a file of its feature; a condensed page keeps its head and loses its
//! tail when over the cap.

use api::{Features, HostedSessions, Projects, Sessions};
use ipc::{
    AllDone, AreaSeed, FeatureSectionKind, FeatureSectionText, FeatureSeed, NewProject, ProjectMember, SessionId, StartSession, UpsertFeatures,
    WriteFeatureSections,
};
use store::{AttachmentState, Holder, KeptAttachment, KeptFeatureMerge};

use super::agent_starts::{caller, rig, Rig};
use crate::clashing::Who;
use crate::feature_library::brief::{condensed, BriefFacts, CAP_CHARS};
use crate::feature_library::mapping::{owns, FeatureMap, How};
use crate::feature_library::staleness::stale_since;

const VOTING: &str = "voting-on-takes";
const SOMEWHERE: &str = "crates/pages/voting";

fn feature(id: &str, slug: &str, paths: &[&str]) -> store::KeptFeature {
    store::KeptFeature {
        id: id.to_string(),
        project: "p1".to_string(),
        area: "takes".to_string(),
        slug: slug.to_string(),
        title: slug.to_string(),
        summary: "one line".to_string(),
        state: "seeded".to_string(),
        manifest_id: String::new(),
        entry_points: Vec::new(),
        paths: paths.iter().map(|one| one.to_string()).collect(),
        position: 0,
        added_at: "2026-11-16T09:00:00.000Z".to_string(),
    }
}

#[test]
fn a_prefix_owns_whole_segments_only() {
    assert!(owns("crates/fleet", "crates/fleet/src/a.rs"));
    assert!(owns("crates/fleet/", "crates/fleet/src/a.rs"));
    assert!(owns("crates/fleet/src/a.rs", "crates/fleet/src/a.rs"));
    assert!(!owns("crates/fleet", "crates/fleet-extra/a.rs"));
    assert!(!owns("", "crates/fleet/a.rs"), "a blank prefix owns nothing");
}

#[test]
fn the_longest_prefix_wins_and_a_tie_or_a_filing_gives_the_file_to_more_than_one() {
    let map = FeatureMap::new(
        vec![
            feature("a", "wide", &["crates/fleet"]),
            feature("b", "narrow", &["crates/fleet/src/votes"]),
            feature("c", "twin", &["crates/fleet/src/votes"]),
            feature("d", "filed", &[]),
        ],
        [("d".to_string(), "crates/fleet/src/votes/one.rs".to_string())],
    );
    let slugs = |path: &str| map.owners(None, path).iter().map(|owner| (owner.feature.slug.clone(), owner.how)).collect::<Vec<_>>();
    assert_eq!(slugs("crates/fleet/src/other.rs"), [("wide".to_string(), How::Prefix)]);
    assert_eq!(
        slugs("crates/fleet/src/votes/one.rs"),
        [("narrow".to_string(), How::Prefix), ("twin".to_string(), How::Prefix), ("filed".to_string(), How::Filed)],
        "narrow and twin tie on the longest prefix, wide loses to them, and a Job filed the file under a fourth"
    );
    assert!(slugs("docs/a.md").is_empty());
}

#[test]
fn a_feature_naming_a_repository_owns_nothing_in_another() {
    let mut named = feature("a", "in-one", &["src"]);
    named.manifest_id = "m1".to_string();
    let map = FeatureMap::new(vec![named], []);
    assert_eq!(map.owners(Some("m1"), "src/a.rs").len(), 1);
    assert!(map.owners(Some("m2"), "src/a.rs").is_empty());
    assert_eq!(map.owners(None, "src/a.rs").len(), 1, "a path whose repository is not said reads as any");
}

fn section(job: &str, at: &str) -> store::KeptSection {
    store::KeptSection { feature_id: "a".into(), section: "test".into(), text: "t".into(), job: job.into(), written_at: at.into(), at_commit: String::new() }
}

fn merge(number: u64, at: &str, job: &str, files: &[&str]) -> KeptFeatureMerge {
    KeptFeatureMerge {
        manifest_id: "m1".into(),
        number,
        merged_at: at.into(),
        at_commit: String::new(),
        job: job.into(),
        files: files.iter().map(|one| one.to_string()).collect(),
    }
}

#[test]
fn a_section_is_stale_since_the_first_merge_after_it_that_changed_a_file_of_its_feature() {
    let map = FeatureMap::new(vec![feature("a", "one", &["src"]), feature("b", "two", &["lib"])], []);
    let written = section("s-1", "2026-11-16T10:00:00.000Z");
    let merges = [
        merge(1, "2026-11-16T09:00:00.000Z", "", &["src/before.rs"]),
        merge(2, "2026-11-16T11:00:00.000Z", "", &["lib/elsewhere.rs"]),
        merge(3, "2026-11-16T12:00:00.000Z", "s-9", &["src/a.rs", "lib/b.rs"]),
        merge(4, "2026-11-16T13:00:00.000Z", "", &["src/c.rs"]),
    ];
    assert_eq!(stale_since(&map, "a", &written, &merges), Some(3), "not #1, which came first, nor #2, which is another feature's");
    assert_eq!(stale_since(&map, "b", &written, &merges), Some(2));
    assert_eq!(stale_since(&map, "a", &written, &merges[..2]), None);
}

#[test]
fn the_merge_of_the_writers_own_pull_request_does_not_make_its_section_stale() {
    let map = FeatureMap::new(vec![feature("a", "one", &["src"])], []);
    let merges = [merge(5, "2026-11-16T12:00:00.000Z", "s-1", &["src/a.rs"])];
    assert_eq!(stale_since(&map, "a", &section("s-1", "2026-11-16T10:00:00.000Z"), &merges), None);
    assert_eq!(stale_since(&map, "a", &section("s-2", "2026-11-16T10:00:00.000Z"), &merges), Some(5));
}

fn facts() -> BriefFacts {
    BriefFacts {
        slug: VOTING.into(),
        title: "Voting on takes".into(),
        area: "Takes".into(),
        current_as_of: Some((812, "2026-11-14".into())),
        stale: vec![("test".into(), 815)],
        jobs_now: vec!["Add rank votes".into()],
        entry_points: (1..=8).map(|n| format!("crates/votes/file{n}.rs:{n}")).collect(),
        test: Some("armada check test page_votes".into()),
        bugs: (1..=5).map(|n| format!("bug {n}")).collect(),
        runs: Some("A page declares a vote, a cast is read off the revision.".into()),
        designs: vec!["Vote results sketch".into()],
        debt: Some("Score votes are untested.".into()),
    }
}

#[test]
fn a_condensed_page_reads_in_its_order_with_five_entry_points_and_three_bugs() {
    let text = condensed(&facts());
    let at = |needle: &str| text.find(needle).unwrap_or_else(|| panic!("{needle} in {text}"));
    let order = ["Voting on takes (Takes)", "Current as of #812, 2026-11-14.", "Stale: test since #815.", "On it now:", "Code:", "Test:", "Open bugs:", "Runs:", "Designs:", "Debt:", "Full sections: get_feature_page feature=voting-on-takes."];
    let places: Vec<usize> = order.iter().map(|needle| at(needle)).collect();
    assert!(places.windows(2).all(|pair| pair[0] < pair[1]), "{text}");
    assert!(text.contains("file5.rs:5") && !text.contains("file6.rs"), "the top five: {text}");
    assert!(text.contains("bug 3") && !text.contains("bug 4"), "the newest three: {text}");
    assert!(text.chars().count() < CAP_CHARS);
}

#[test]
fn a_page_over_the_cap_loses_its_tail_first_and_never_its_head() {
    let mut long = facts();
    let big = |letter: &str| letter.repeat(300);
    long.jobs_now = (0..5).map(|_| big("j")).collect();
    long.entry_points = (0..8).map(|_| big("e")).collect();
    long.test = Some(big("t"));
    long.bugs = (0..5).map(|_| big("b")).collect();
    long.runs = Some(big("r"));
    long.designs = (0..8).map(|_| big("d")).collect();
    long.debt = Some(big("x"));
    let text = condensed(&long);
    assert!(text.chars().count() <= CAP_CHARS, "{}", text.chars().count());
    assert!(text.starts_with("Voting on takes (Takes)\nCurrent as of #812, 2026-11-14.\nStale: test since #815."), "{text}");
    assert!(text.contains("Open bugs:") && text.contains("Runs:"), "what ranks high stays: {text}");
    assert!(!text.contains("Debt:") && !text.contains("Full sections:"), "the tail goes first: {text}");
}

// What follows runs a Job through the library against a real store.

async fn a_project(rig: &Rig) -> String {
    let members = Some(vec![ProjectMember { manifest_id: rig.manifest.clone(), folder: None }]);
    let new = NewProject { name: "Shop".to_string(), members, place: None, values: None };
    rig.fleet.add_project(new).await.expect("added").projects.last().expect("the project").id.clone()
}

async fn an_agent_for(rig: &Rig, project: &str) -> SessionId {
    let start = StartSession {
        project: Some(project.to_string()),
        manifest_id: rig.manifest.clone(),
        title: Some("Add rank votes".into()),
        model: None,
        effort: None,
        mode: None,
        root: None,
        fork: None,
        request: None,
        after: None,
        held: false,
        project_rows: Vec::new(),
        on_your_words: None,
    };
    let id = rig.fleet.start_session(start).await.expect("started").id;
    rig.send(&id, "go on").await;
    rig.stand_in.init(0);
    id
}

fn seeded(project: &str) -> UpsertFeatures {
    let feature = |slug: &str, area: &str, paths: &[&str]| FeatureSeed {
        slug: slug.to_string(),
        area: area.to_string(),
        title: slug.to_string(),
        summary: "One line.".to_string(),
        state: None,
        paths: paths.iter().map(|one| one.to_string()).collect(),
        entry_points: Vec::new(),
        manifest_id: None,
    };
    UpsertFeatures {
        project: project.to_string(),
        areas: vec![AreaSeed { slug: "takes".into(), title: "Takes".into() }],
        features: vec![feature(VOTING, "takes", &[SOMEWHERE]), feature("page-actions", "takes", &["crates/pages/actions"])],
    }
}

/// The Job wrote `path` in the rig's repository, and Fleet filed it.
async fn wrote(rig: &Rig, id: &SessionId, path: &str) {
    rig.fleet.writes().wrote(&Who::Session(id.as_str().to_string()), rig.manifest.as_str(), [path.to_string()], &rig.fleet.now());
    rig.fleet.feature_writes_filed().await;
}

async fn pushed(rig: &Rig, id: &SessionId, pull: &str) {
    let now = rig.fleet.now().as_str().to_string();
    let row = KeptAttachment {
        holder: Holder::session(id.as_str()),
        kind: crate::pull_requesting::PR.into(),
        manifest_id: rig.manifest.as_str().to_string(),
        target: pull.to_string(),
        state: AttachmentState::Standing,
        detail: Default::default(),
        since: now.clone(),
        changed_at: now,
    };
    rig.fleet.store().lock().await.attach(&row, false).expect("attached");
}

fn write_all(id: &SessionId, feature: &str, sections: &[FeatureSectionKind]) -> WriteFeatureSections {
    WriteFeatureSections {
        feature: feature.to_string(),
        sections: sections.iter().map(|section| FeatureSectionText { section: *section, text: format!("The {} text.", section.name()) }).collect(),
        project: None,
        session_id: Some(id.clone()),
    }
}

fn refused(result: Result<ipc::SessionRecord, api::Refusal>) -> (String, String) {
    match result.expect_err("refused") {
        api::Refusal::IllegalMove(why) | api::Refusal::Unacceptable(why) => (why.code.to_string(), why.message),
        other => panic!("{other:?}"),
    }
}

#[tokio::test]
async fn all_done_is_held_while_pushed_work_changed_a_feature_whose_sections_are_not_written_and_names_them() {
    let rig = rig(None);
    let project = a_project(&rig).await;
    rig.fleet.upsert_features(seeded(&project)).await.expect("seeded");
    let id = an_agent_for(&rig, &project).await;
    wrote(&rig, &id, "crates/pages/voting/ranks.rs").await;
    pushed(&rig, &id, "812").await;

    let (code, said) = refused(rig.fleet.all_done(caller(), AllDone { session_id: Some(id.clone()), summary: None }).await);
    assert_eq!(code, "fleet.feature_sections_owed");
    assert!(said.contains("voting-on-takes: test, runs, observe, debt"), "{said}");
    assert!(!said.contains("page-actions"), "a feature the Job did not change is not owed: {said}");

    let written = rig.fleet.write_feature_sections(caller(), write_all(&id, VOTING, &[FeatureSectionKind::Test, FeatureSectionKind::Runs])).await.expect("written");
    assert_eq!(written.owed.len(), 1);
    assert_eq!(written.owed[0].sections, [FeatureSectionKind::Observe, FeatureSectionKind::Debt]);
    let (_, said) = refused(rig.fleet.all_done(caller(), AllDone { session_id: Some(id.clone()), summary: None }).await);
    assert!(said.contains("voting-on-takes: observe, debt"), "{said}");

    rig.fleet.write_feature_sections(caller(), write_all(&id, VOTING, &[FeatureSectionKind::Observe, FeatureSectionKind::Debt])).await.expect("written");
    let done = rig.fleet.all_done(caller(), AllDone { session_id: Some(id.clone()), summary: None }).await.expect("nothing is owed");
    assert!(done.done_since.is_some());
}

#[tokio::test]
async fn a_job_that_changed_no_feature_file_is_never_held_and_neither_is_one_that_pushed_nothing() {
    let rig = rig(None);
    let project = a_project(&rig).await;
    rig.fleet.upsert_features(seeded(&project)).await.expect("seeded");
    let id = an_agent_for(&rig, &project).await;

    // It wrote only where no feature owns the file, and pushed.
    wrote(&rig, &id, "docs/elsewhere.md").await;
    pushed(&rig, &id, "813").await;
    assert!(rig.fleet.all_done(caller(), AllDone { session_id: Some(id.clone()), summary: None }).await.is_ok());
}

#[tokio::test]
async fn work_that_was_never_pushed_holds_nothing_even_where_it_changed_a_feature() {
    let rig = rig(None);
    let project = a_project(&rig).await;
    rig.fleet.upsert_features(seeded(&project)).await.expect("seeded");
    let id = an_agent_for(&rig, &project).await;
    wrote(&rig, &id, "crates/pages/voting/ranks.rs").await;
    assert!(rig.fleet.all_done(caller(), AllDone { session_id: Some(id.clone()), summary: None }).await.is_ok());
}

#[tokio::test]
async fn a_job_is_held_only_for_features_that_were_in_the_catalog_when_it_started() {
    let rig = rig(None);
    let project = a_project(&rig).await;
    let id = an_agent_for(&rig, &project).await;
    {
        let mut store = rig.fleet.store().lock().await;
        for (feature_id, slug, path, added_at) in
            [("f-early", "early-feature", "crates/early", "2000-01-01T00:00:00.000Z"), ("f-late", "late-feature", "crates/late", "2999-01-01T00:00:00.000Z")]
        {
            store
                .put_feature(&store::KeptFeature { project: project.clone(), added_at: added_at.to_string(), ..feature(feature_id, slug, &[path]) })
                .expect("kept");
        }
    }
    wrote(&rig, &id, "crates/early/a.rs").await;
    wrote(&rig, &id, "crates/late/b.rs").await;
    pushed(&rig, &id, "814").await;

    let (code, said) = refused(rig.fleet.all_done(caller(), AllDone { session_id: Some(id.clone()), summary: None }).await);
    assert_eq!(code, "fleet.feature_sections_owed");
    assert!(said.contains("early-feature: test, runs, observe, debt"), "{said}");
    assert!(!said.contains("late-feature"), "the Job started before it was in the catalog: {said}");

    rig.fleet.write_feature_sections(caller(), write_all(&id, "early-feature", &FeatureSectionKind::ALL)).await.expect("written");
    assert!(rig.fleet.all_done(caller(), AllDone { session_id: Some(id.clone()), summary: None }).await.is_ok(), "nothing else is owed");
}

#[tokio::test]
async fn a_second_seed_keeps_the_date_a_feature_entered_the_catalog() {
    let rig = rig(None);
    let project = a_project(&rig).await;
    rig.fleet.upsert_features(seeded(&project)).await.expect("seeded");
    let first = rig.fleet.store().lock().await.feature_named(&project, VOTING).expect("read").expect("the feature").added_at;
    assert!(!first.is_empty());
    rig.fleet.upsert_features(seeded(&project)).await.expect("seeded again");
    let again = rig.fleet.store().lock().await.feature_named(&project, VOTING).expect("read").expect("the feature").added_at;
    assert_eq!(again, first);
}

#[tokio::test]
async fn the_list_shows_who_is_on_a_feature_and_what_a_written_section_reads_as_after_another_jobs_merge() {
    let rig = rig(None);
    let project = a_project(&rig).await;
    let id = an_agent_for(&rig, &project).await;
    rig.fleet.upsert_features(seeded(&project)).await.expect("seeded");
    wrote(&rig, &id, "crates/pages/voting/ranks.rs").await;
    rig.fleet.write_feature_sections(caller(), write_all(&id, VOTING, &[FeatureSectionKind::Test])).await.expect("written");

    let listed = rig.fleet.list_features(None, Some(project.clone())).await.expect("listed");
    let row = &listed.areas[0].features.iter().find(|one| one.slug == VOTING).expect("the feature").clone();
    assert_eq!(row.jobs_now.iter().map(|one| (one.session_id.clone(), one.title.as_str())).collect::<Vec<_>>(), [(id.clone(), "Add rank votes")]);
    assert_eq!(row.stale_since, None);

    // Another Job's merge changes a file of it after the section was written.
    let after = "2999-01-01T00:00:00.000Z".to_string();
    rig.fleet
        .store()
        .lock()
        .await
        .keep_feature_merge(&KeptFeatureMerge {
            manifest_id: rig.manifest.as_str().to_string(),
            number: 900,
            merged_at: after,
            at_commit: String::new(),
            job: "s-other".into(),
            files: vec!["crates/pages/voting/other.rs".into()],
        })
        .expect("kept");
    let page = rig.fleet.get_feature_page(None, Some(project.clone()), VOTING.to_string()).await.expect("the page");
    assert_eq!(page.sections.len(), 1);
    assert_eq!(page.sections[0].stale_since, Some(900));
    assert_eq!(page.feature.stale_since, Some(900));
    assert_eq!(page.files[0].path, "crates/pages/voting/ranks.rs");

    let brief = rig.fleet.get_feature_brief(None, Some(project.clone()), VOTING.to_string()).await.expect("the brief");
    assert!(brief.text.contains("Stale: test since #900."), "{}", brief.text);
    assert!(brief.text.contains("Current as of #900"), "{}", brief.text);
    assert!(brief.tokens <= 1000);

    let of_path = rig.fleet.get_features_of_path(None, Some(project), None, "crates/pages/voting/ranks.rs".to_string()).await.expect("found");
    assert_eq!(of_path.features.iter().map(|one| one.slug.as_str()).collect::<Vec<_>>(), [VOTING]);
}

#[tokio::test]
async fn a_feature_in_an_area_nobody_kept_is_refused_with_nothing_kept_and_a_re_seed_keeps_the_id() {
    let rig = rig(None);
    let project = a_project(&rig).await;
    let mut bad = seeded(&project);
    bad.features[0].area = "nowhere".into();
    let refused = rig.fleet.upsert_features(bad).await.expect_err("refused");
    let api::Refusal::Unacceptable(why) = refused else { panic!("{refused:?}") };
    assert_eq!(why.code.to_string(), "fleet.feature_area_unknown");
    assert!(rig.fleet.list_features(None, Some(project.clone())).await.expect("listed").areas.is_empty(), "nothing of the call was kept");

    let first = rig.fleet.upsert_features(seeded(&project)).await.expect("kept");
    assert_eq!(first.created.len(), 2);
    let id_of = |library: &ipc::FeatureLibrary| library.areas[0].features.iter().find(|one| one.slug == VOTING).map(|one| one.id.clone());
    let before = id_of(&rig.fleet.list_features(None, Some(project.clone())).await.expect("listed"));
    let again = rig.fleet.upsert_features(seeded(&project)).await.expect("kept");
    assert_eq!((again.created.len(), again.updated.len()), (0, 2));
    assert_eq!(id_of(&rig.fleet.list_features(None, Some(project)).await.expect("listed")), before);
}
