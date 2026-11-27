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
use crate::feature_library::staleness::{owned, stale_since};

pub(super) const VOTING: &str = "voting-on-takes";
pub(super) const SOMEWHERE: &str = "crates/pages/voting";

pub(super) fn feature(id: &str, slug: &str, paths: &[&str]) -> store::KeptFeature {
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
    let merges = owned(&map, merges.to_vec());
    assert_eq!(stale_since("a", &written, &merges), Some(3), "not #1, which came first, nor #2, which is another feature's");
    assert_eq!(stale_since("b", &written, &merges), Some(2));
    assert_eq!(stale_since("a", &written, &merges[..2]), None);
}

#[test]
fn the_merge_of_the_writers_own_pull_request_does_not_make_its_section_stale() {
    let map = FeatureMap::new(vec![feature("a", "one", &["src"])], []);
    let merges = owned(&map, vec![merge(5, "2026-11-16T12:00:00.000Z", "s-1", &["src/a.rs"])]);
    assert_eq!(stale_since("a", &section("s-1", "2026-11-16T10:00:00.000Z"), &merges), None);
    assert_eq!(stale_since("a", &section("s-2", "2026-11-16T10:00:00.000Z"), &merges), Some(5));
}

fn facts() -> BriefFacts {
    BriefFacts {
        slug: VOTING.into(),
        title: "Voting on takes".into(),
        area: "Takes".into(),
        current_as_of: Some((812, "2026-11-14".into())),
        stale: vec![("test".into(), 815)],
        jobs_now: vec!["Add rank votes".into()],
        elsewhere: Vec::new(),
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
    let order = ["Voting on takes (Takes)", "Current as of #812, 2026-11-14.", "Stale: test since #815.", "On it now:", "Code:", "Test:", "Open bugs:", "Runs:", "Designs:", "Debt:", "One full section: get_feature_page feature=voting-on-takes section=<test|runs|observe|debt>."];
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
    assert!(!text.contains("Debt:"), "the tail goes first: {text}");
    assert!(text.ends_with("One full section: get_feature_page feature=voting-on-takes section=<test|runs|observe|debt>."), "the last line stays: {text}");
}

// What follows runs a Job through the library against a real store.

pub(super) async fn a_project(rig: &Rig) -> String {
    let members = Some(vec![ProjectMember { manifest_id: rig.manifest.clone(), folder: None }]);
    let new = NewProject { name: "Shop".to_string(), members, place: None, values: None };
    rig.fleet.add_project(new).await.expect("added").projects.last().expect("the project").id.clone()
}

pub(super) async fn an_agent_for(rig: &Rig, project: &str) -> SessionId {
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
        workflow: None,
    };
    let id = rig.fleet.start_session(start).await.expect("started").id;
    rig.send(&id, "go on").await;
    rig.stand_in.init(0);
    id
}

pub(super) fn seeded(project: &str) -> UpsertFeatures {
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

/// The Job's commit named `slugs` in its `Feature:` line, and Fleet's commit gate let it through.
pub(super) async fn named(rig: &Rig, id: &SessionId, slugs: &str) {
    let command = format!("git commit -m \"Work\" -m \"Feature: {slugs}\"");
    assert!(rig.fleet.check_commit_feature(commit_of(id, &command, &[])).await.expect("asked").allowed);
}

/// The Job wrote `path` in the rig's repository, and Fleet filed it.
pub(super) async fn wrote(rig: &Rig, id: &SessionId, path: &str) {
    rig.fleet.writes().wrote(&Who::Session(id.as_str().to_string()), rig.manifest.as_str(), [path.to_string()], &rig.fleet.now());
    rig.fleet.feature_writes_filed().await;
}

pub(super) async fn pushed(rig: &Rig, id: &SessionId, pull: &str) {
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

/// The Job stands in pool slot `slot`, as the ledger holds it.
pub(super) async fn held_slot(rig: &Rig, id: &SessionId, slot: &str) {
    let now = rig.fleet.now().as_str().to_string();
    let row = KeptAttachment {
        holder: Holder::session(id.as_str()),
        kind: "slot".into(),
        manifest_id: rig.manifest.as_str().to_string(),
        target: slot.to_string(),
        state: AttachmentState::Standing,
        detail: Default::default(),
        since: now.clone(),
        changed_at: now,
    };
    rig.fleet.store().lock().await.attach(&row, false).expect("attached");
}

pub(super) fn write_all(id: &SessionId, feature: &str, sections: &[FeatureSectionKind]) -> WriteFeatureSections {
    WriteFeatureSections {
        feature: feature.to_string(),
        sections: sections.iter().map(|section| FeatureSectionText { section: *section, text: format!("The {} text.", section.name()) }).collect(),
        glossary: Vec::new(),
        gists: Vec::new(),
        project: None,
        session_id: Some(id.clone()),
    }
}

pub(super) fn refused(result: Result<ipc::SessionRecord, api::Refusal>) -> (String, String) {
    match result.expect_err("refused") {
        api::Refusal::IllegalMove(why) | api::Refusal::Unacceptable(why) => (why.code.to_string(), why.message),
        other => panic!("{other:?}"),
    }
}

#[tokio::test]
async fn all_done_is_held_while_pushed_work_changed_a_feature_whose_sections_are_not_written_and_names_them() {
    let rig = rig(None);
    let project = a_project(&rig).await;
    rig.fleet.upsert_features(None, seeded(&project)).await.expect("seeded");
    let id = an_agent_for(&rig, &project).await;
    wrote(&rig, &id, "crates/pages/voting/ranks.rs").await;
    named(&rig, &id, VOTING).await;
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
    rig.fleet.upsert_features(None, seeded(&project)).await.expect("seeded");
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
    rig.fleet.upsert_features(None, seeded(&project)).await.expect("seeded");
    let id = an_agent_for(&rig, &project).await;
    wrote(&rig, &id, "crates/pages/voting/ranks.rs").await;
    named(&rig, &id, VOTING).await;
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
    named(&rig, &id, "early-feature, late-feature").await;
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
    rig.fleet.upsert_features(None, seeded(&project)).await.expect("seeded");
    let first = rig.fleet.store().lock().await.feature_named(&project, VOTING).expect("read").expect("the feature").added_at;
    assert!(!first.is_empty());
    rig.fleet.upsert_features(None, seeded(&project)).await.expect("seeded again");
    let again = rig.fleet.store().lock().await.feature_named(&project, VOTING).expect("read").expect("the feature").added_at;
    assert_eq!(again, first);
}

#[tokio::test]
async fn a_live_job_is_on_a_feature_only_while_it_holds_a_slot() {
    let rig = rig(None);
    let project = a_project(&rig).await;
    let at_work = an_agent_for(&rig, &project).await;
    let stopped = an_agent_for(&rig, &project).await;
    rig.fleet.upsert_features(None, seeded(&project)).await.expect("seeded");
    wrote(&rig, &at_work, "crates/pages/voting/ranks.rs").await;
    wrote(&rig, &stopped, "crates/pages/voting/ranks.rs").await;
    held_slot(&rig, &at_work, "4").await;

    let listed = rig.fleet.list_features(None, Some(project.clone())).await.expect("listed");
    let row = listed.areas[0].features.iter().find(|one| one.slug == VOTING).expect("the feature");
    assert_eq!(row.jobs_now.iter().map(|one| one.session_id.clone()).collect::<Vec<_>>(), [at_work], "the live Job holding no slot is not on it");
}

/// The features `id`'s row carries, as `list_sessions` answers it.
async fn features_on_record(rig: &Rig, id: &SessionId) -> Vec<ipc::SessionFeature> {
    let listed = rig.fleet.list_sessions(None, None, None).await.expect("listed").sessions;
    listed.into_iter().find(|one| &one.id == id).expect("the Job").features
}

#[tokio::test]
async fn a_job_that_writes_a_file_under_a_feature_carries_that_feature_on_its_record() {
    let rig = rig(None);
    let project = a_project(&rig).await;
    let id = an_agent_for(&rig, &project).await;
    rig.fleet.upsert_features(None, seeded(&project)).await.expect("seeded");
    wrote(&rig, &id, "crates/pages/voting/ranks.rs").await;
    assert_eq!(features_on_record(&rig, &id).await, [], "a live Job holding no slot is at work on nothing");

    held_slot(&rig, &id, "4").await;
    let on = ipc::SessionFeature {
        project,
        slug: VOTING.into(),
        title: VOTING.into(),
        area: "takes".into(),
        area_title: "Takes".into(),
        area_place: 0,
        place: 0,
    };
    assert_eq!(features_on_record(&rig, &id).await, [on], "the feature whose path it wrote under, with its area, and not the other");
}

#[tokio::test]
async fn the_list_shows_who_is_on_a_feature_and_what_a_written_section_reads_as_after_another_jobs_merge() {
    let rig = rig(None);
    let project = a_project(&rig).await;
    let id = an_agent_for(&rig, &project).await;
    rig.fleet.upsert_features(None, seeded(&project)).await.expect("seeded");
    wrote(&rig, &id, "crates/pages/voting/ranks.rs").await;
    held_slot(&rig, &id, "4").await;
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
    let page = rig.fleet.get_feature_page(None, Some(project.clone()), VOTING.to_string(), None, false).await.expect("the page");
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
    let refused = rig.fleet.upsert_features(None, bad).await.expect_err("refused");
    let api::Refusal::Unacceptable(why) = refused else { panic!("{refused:?}") };
    assert_eq!(why.code.to_string(), "fleet.feature_area_unknown");
    assert!(rig.fleet.list_features(None, Some(project.clone())).await.expect("listed").areas.is_empty(), "nothing of the call was kept");

    let first = rig.fleet.upsert_features(None, seeded(&project)).await.expect("kept");
    assert_eq!(first.created.len(), 2);
    let id_of = |library: &ipc::FeatureLibrary| library.areas[0].features.iter().find(|one| one.slug == VOTING).map(|one| one.id.clone());
    let before = id_of(&rig.fleet.list_features(None, Some(project.clone())).await.expect("listed"));
    let again = rig.fleet.upsert_features(None, seeded(&project)).await.expect("kept");
    assert_eq!((again.created.len(), again.updated.len()), (0, 2));
    assert_eq!(id_of(&rig.fleet.list_features(None, Some(project)).await.expect("listed")), before);
}

// What follows hands a Job the pages of the features it works in.

pub(super) fn hand(id: &SessionId, agent: Option<&str>, paths: &[&str], brief: Option<&str>) -> ipc::HandOverFeatures {
    ipc::HandOverFeatures {
        session_id: id.clone(),
        agent_id: agent.map(str::to_string),
        paths: paths.iter().map(|one| one.to_string()).collect(),
        brief: brief.map(str::to_string),
    }
}

#[tokio::test]
async fn a_feature_page_is_handed_once_per_job_and_once_per_subagent_and_the_record_outlives_the_call() {
    let rig = rig(None);
    let project = a_project(&rig).await;
    rig.fleet.upsert_features(None, seeded(&project)).await.expect("seeded");
    let id = an_agent_for(&rig, &project).await;
    let voting = "crates/pages/voting/ranks.rs";

    let first = rig.fleet.hand_over_features(hand(&id, None, &[voting], None)).await.expect("asked");
    assert_eq!(first.features, [VOTING]);
    assert!(first.text.starts_with("Feature pages from Armada's library"), "{}", first.text);
    assert!(first.text.contains("voting-on-takes (Takes)") && first.text.contains("One full section:"), "the condensed page: {}", first.text);

    let again = rig.fleet.hand_over_features(hand(&id, None, &[voting, "crates/pages/voting"], None)).await.expect("asked");
    assert!(again.features.is_empty() && again.text.is_empty(), "the second read of the feature hands nothing: {}", again.text);

    let other = rig.fleet.hand_over_features(hand(&id, Some("agent-1"), &[voting], None)).await.expect("asked");
    assert_eq!(other.features, [VOTING], "a subagent has a context of its own");
    assert!(rig.fleet.hand_over_features(hand(&id, Some("agent-1"), &[voting], None)).await.expect("asked").features.is_empty());

    let store = rig.fleet.store().lock().await;
    assert_eq!(store.feature_handoffs(id.as_str(), "").expect("read").len(), 1);
    assert_eq!(store.feature_handoffs(id.as_str(), "agent-1").expect("read").len(), 1);
}

#[tokio::test]
async fn a_brief_hands_over_the_features_it_names_and_a_later_read_of_their_files_hands_nothing() {
    let rig = rig(None);
    let project = a_project(&rig).await;
    rig.fleet.upsert_features(None, seeded(&project)).await.expect("seeded");
    let id = an_agent_for(&rig, &project).await;
    let brief = "Add rank votes.\n\nFeature: no-such-feature, voting-on-takes\nSee also `page-actions` and `cargo test`.";

    let named = rig.fleet.hand_over_features(hand(&id, None, &[], Some(brief))).await.expect("asked");
    assert_eq!(named.features, [VOTING, "page-actions"], "a slug the project does not hold names nothing");
    let read = rig.fleet.hand_over_features(hand(&id, None, &["crates/pages/actions/a.rs"], None)).await.expect("asked");
    assert!(read.features.is_empty(), "the brief already handed it over");
    let told_again = rig.fleet.hand_over_features(hand(&id, None, &[], Some(brief))).await.expect("asked");
    assert!(told_again.text.is_empty());
}

#[tokio::test]
async fn a_file_several_features_own_hands_over_three_pages_and_the_rest_at_the_next_read() {
    let rig = rig(None);
    let project = a_project(&rig).await;
    let id = an_agent_for(&rig, &project).await;
    {
        let mut store = rig.fleet.store().lock().await;
        for (n, slug) in ["one", "two", "three", "four"].into_iter().enumerate() {
            store.put_feature(&store::KeptFeature { project: project.clone(), ..feature(&format!("f-{n}"), slug, &["crates/wide"]) }).expect("kept");
        }
    }
    let path = ["crates/wide/a.rs"];
    let first = rig.fleet.hand_over_features(hand(&id, None, &path, None)).await.expect("asked");
    assert_eq!(first.features.len(), 3);
    let second = rig.fleet.hand_over_features(hand(&id, None, &path, None)).await.expect("asked");
    assert_eq!(second.features.len(), 1);
    assert!(!first.features.contains(&second.features[0]));
}

#[tokio::test]
async fn nothing_is_handed_to_a_job_fleet_does_not_know_a_project_with_no_library_or_a_file_no_feature_owns() {
    let rig = rig(None);
    let project = a_project(&rig).await;
    let id = an_agent_for(&rig, &project).await;
    let voting = "crates/pages/voting/ranks.rs";
    assert!(rig.fleet.hand_over_features(hand(&id, None, &[voting], None)).await.expect("asked").text.is_empty(), "no library yet");

    rig.fleet.upsert_features(None, seeded(&project)).await.expect("seeded");
    let stranger = SessionId::carried("s-unknown");
    assert!(rig.fleet.hand_over_features(hand(&stranger, None, &[voting], None)).await.expect("asked").text.is_empty());
    assert!(rig.fleet.hand_over_features(hand(&id, None, &["docs/elsewhere.md"], None)).await.expect("asked").text.is_empty());
    assert!(rig.fleet.store().lock().await.feature_handoffs(id.as_str(), "").expect("read").is_empty(), "nothing was handed, so nothing is kept");
}

// What follows holds a commit to naming its feature, for the Jobs started after the catalog was seeded.

pub(super) fn commit_of(id: &SessionId, command: &str, files: &[&str]) -> ipc::CheckCommitFeature {
    ipc::CheckCommitFeature { session_id: id.clone(), command: command.to_string(), files: files.iter().map(|one| one.to_string()).collect() }
}

#[test]
fn only_a_commit_that_gives_its_message_in_the_line_is_read() {
    use crate::feature_library::committing::gives_message;
    for line in [
        r#"git commit -m "Add votes""#,
        r#"git add -A && git commit -am "Add votes""#,
        "git commit --message=\"Add votes\"",
        "git -C sub commit -sm\"Add votes\"",
        "git commit -m \"$(cat <<'EOF'\nAdd votes\nEOF\n)\"",
    ] {
        assert!(gives_message(line), "{line}");
    }
    for line in ["git commit --amend --no-edit", "git commit -F message.txt", "git commit -C HEAD", "git log -m", "echo commit -m x", "git status"] {
        assert!(!gives_message(line), "{line}");
    }
}

#[tokio::test]
async fn a_job_started_after_the_catalog_was_seeded_is_refused_a_commit_with_no_feature_line_and_told_which_to_copy() {
    let rig = rig(None);
    let project = a_project(&rig).await;
    rig.fleet.upsert_features(None, seeded(&project)).await.expect("seeded");
    let id = an_agent_for(&rig, &project).await;
    let files = ["crates/pages/voting/a.rs", "crates/pages/voting/b.rs", "crates/pages/actions/c.rs", "docs/elsewhere.md"];

    let refused = rig.fleet.check_commit_feature(commit_of(&id, r#"git commit -m "Add rank votes""#, &files)).await.expect("asked");
    assert!(!refused.allowed);
    assert_eq!(refused.features, [VOTING, "page-actions"], "the most files first");
    assert!(refused.says.contains("Feature: voting-on-takes\nFeature: page-actions\n"), "a line to copy for each: {}", refused.says);

    let named = rig.fleet.check_commit_feature(commit_of(&id, "git commit -m \"Add rank votes\" -m \"Feature: voting-on-takes\"", &files)).await.expect("asked");
    assert!(named.allowed && named.says.is_empty());
}

#[tokio::test]
async fn files_no_feature_owns_are_refused_without_a_feature_to_copy() {
    let rig = rig(None);
    let project = a_project(&rig).await;
    rig.fleet.upsert_features(None, seeded(&project)).await.expect("seeded");
    let id = an_agent_for(&rig, &project).await;

    let refused = rig.fleet.check_commit_feature(commit_of(&id, r#"git commit -m "Fix a typo""#, &["docs/elsewhere.md"])).await.expect("asked");
    assert!(!refused.allowed && refused.features.is_empty());
    assert!(refused.says.contains("No feature owns the files you are committing"), "{}", refused.says);
}

#[tokio::test]
async fn a_job_older_than_the_catalog_a_project_with_no_feature_and_a_commit_nothing_reads_are_never_refused() {
    let rig = rig(None);
    let project = a_project(&rig).await;
    let files = ["crates/pages/voting/a.rs"];
    let line = r#"git commit -m "Add rank votes""#;
    let id = an_agent_for(&rig, &project).await;
    assert!(rig.fleet.check_commit_feature(commit_of(&id, line, &files)).await.expect("asked").allowed, "the project has no feature");

    // The catalog is seeded after the Job started, so the Job was never asked for a feature.
    rig.fleet
        .store()
        .lock()
        .await
        .put_feature(&store::KeptFeature { project: project.clone(), added_at: "2999-01-01T00:00:00.000Z".to_string(), ..feature("f-late", VOTING, &[SOMEWHERE]) })
        .expect("kept");
    assert!(rig.fleet.check_commit_feature(commit_of(&id, line, &files)).await.expect("asked").allowed, "the Job started before the catalog did");

    rig.fleet.upsert_features(None, seeded(&project)).await.expect("seeded");
    let later = an_agent_for(&rig, &project).await;
    let ask = |command: &str, files: &[&str]| commit_of(&later, command, files);
    assert!(!rig.fleet.check_commit_feature(ask(line, &files)).await.expect("asked").allowed, "a Job started after is held to it");
    assert!(rig.fleet.check_commit_feature(ask("git commit --amend --no-edit", &files)).await.expect("asked").allowed, "no message is given");
    assert!(rig.fleet.check_commit_feature(ask(line, &[])).await.expect("asked").allowed, "nothing to commit");
    assert!(rig.fleet.check_commit_feature(commit_of(&SessionId::carried("s-unknown"), line, &files)).await.expect("asked").allowed, "a Job Fleet does not know");
}

// What follows files the sketches, pages, walks and drafts that name features.

fn made(form: &str, slugs: &str) -> std::collections::BTreeMap<String, String> {
    [("form", form), ("feature", slugs), ("title", "Vote results")].into_iter().map(|(key, word)| (key.to_string(), word.to_string())).collect()
}

#[tokio::test]
async fn an_artifact_is_linked_to_each_feature_it_names_and_a_name_the_project_does_not_hold_is_a_pending_feature() {
    let rig = rig(None);
    let project = a_project(&rig).await;
    rig.fleet.upsert_features(None, seeded(&project)).await.expect("seeded");
    let id = an_agent_for(&rig, &project).await;
    let now = rig.fleet.now().as_str().to_string();
    let mut store = rig.fleet.store().lock().await;

    rig.fleet.artifact_filed(&mut store, id.as_str(), "sketch:Vote results", &made("sketch", "voting-on-takes,page-actions,brand-new"), &now);
    rig.fleet.artifact_filed(&mut store, id.as_str(), "http://localhost:5173/", &made("window", "voting-on-takes"), &now);
    rig.fleet.artifact_filed(&mut store, id.as_str(), "draft:Vote results", &made("draft", "voting-on-takes"), &now);
    rig.fleet.artifact_filed(&mut store, id.as_str(), "/docs/votes.html", &made("document", "voting-on-takes"), &now);
    rig.fleet.artifact_filed(&mut store, id.as_str(), "sketch:Unnamed", &[("form".to_string(), "sketch".to_string())].into(), &now);

    let kinds_of = |store: &store::Store, slug: &str| {
        let feature = store.feature_named(&project, slug).expect("read").expect("the feature");
        store.feature_links(&feature.id).expect("read").into_iter().map(|link| (link.kind, link.target, link.title)).collect::<Vec<_>>()
    };
    let voting = kinds_of(&store, VOTING);
    for (kind, target) in [("sketch", "sketch:Vote results"), ("walk", "http://localhost:5173/"), ("draft", "draft:Vote results"), ("page", "/docs/votes.html")] {
        assert!(voting.contains(&(kind.to_string(), target.to_string(), "Vote results".to_string())), "{kind}: {voting:?}");
    }
    assert_eq!(kinds_of(&store, "page-actions"), [("sketch".to_string(), "sketch:Vote results".to_string(), "Vote results".to_string())]);

    let pending = store.feature_named(&project, "brand-new").expect("read").expect("a pending feature for the Operator to confirm");
    assert_eq!(pending.state, store::feature_state::PENDING);
    assert_eq!(kinds_of(&store, "brand-new"), [("sketch".to_string(), "sketch:Vote results".to_string(), "Vote results".to_string())]);
}

#[tokio::test]
async fn a_name_in_a_project_with_no_library_is_not_kept_for_the_operator() {
    let rig = rig(None);
    let project = a_project(&rig).await;
    let id = an_agent_for(&rig, &project).await;
    let now = rig.fleet.now().as_str().to_string();
    let mut store = rig.fleet.store().lock().await;
    rig.fleet.artifact_filed(&mut store, id.as_str(), "sketch:Vote results", &made("sketch", "voting-on-takes"), &now);
    assert!(store.feature_named(&project, "voting-on-takes").expect("read").is_none());
}

#[tokio::test]
async fn a_page_asked_for_one_section_holds_that_section_alone() {
    let rig = rig(None);
    let project = a_project(&rig).await;
    let id = an_agent_for(&rig, &project).await;
    rig.fleet.upsert_features(None, seeded(&project)).await.expect("seeded");
    wrote(&rig, &id, "crates/pages/voting/ranks.rs").await;
    rig.fleet.write_feature_sections(caller(), write_all(&id, VOTING, &FeatureSectionKind::ALL)).await.expect("written");

    let whole = rig.fleet.get_feature_page(None, Some(project.clone()), VOTING.to_string(), None, false).await.expect("the page");
    assert_eq!(whole.sections.len(), 4);
    assert!(!whole.files.is_empty(), "the whole page keeps its files");

    for asked in ["debt", " Debt "] {
        let page = rig.fleet.get_feature_page(None, Some(project.clone()), VOTING.to_string(), Some(asked.to_string()), false).await.expect("the page");
        assert_eq!(page.sections.iter().map(|one| (one.section, one.text.as_str())).collect::<Vec<_>>(), [(FeatureSectionKind::Debt, "The debt text.")]);
        assert!(page.links.is_empty() && page.files.is_empty(), "a section is asked for alone");
        assert_eq!(page.feature.slug, VOTING);
    }
    let blank = rig.fleet.get_feature_page(None, Some(project.clone()), VOTING.to_string(), Some("  ".into()), false).await.expect("the page");
    assert_eq!(blank.sections.len(), 4, "a blank section is none");

    let refused = rig.fleet.get_feature_page(None, Some(project.clone()), VOTING.to_string(), Some("tests".into()), false).await.expect_err("refused");
    let api::Refusal::Unacceptable(why) = refused else { panic!("{refused:?}") };
    assert_eq!(why.code, "fleet.feature_section_unknown");
    assert!(why.message.contains("`tests`") && why.message.contains("test, runs, observe or debt"), "{}", why.message);

    let observe = rig.fleet.get_feature_page(None, Some(project), VOTING.to_string(), Some("observe".into()), false).await.expect("the page");
    assert_eq!(observe.sections.len(), 1);
}
