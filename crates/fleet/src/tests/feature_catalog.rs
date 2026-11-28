//! The Operator keeps the catalog, written before the code: **Armada's own catalog is kept once from the seed Fleet carries,
//! by Fleet itself with the Operator on or off, and each feature is stamped with the moment it was kept.** What Jobs and he name later is told to
//! the Operator, who brings one card for it. `docs/concepts/feature-catalog.md`.

use std::collections::BTreeSet;

use api::{Features, Projects, Refusal, Story};
use adapters::feature_commits::NamingCommit;
use api::HostedSessions;
use ipc::{AreaSeed, AskKind, AskState, ChangeFeature, FeatureSeed, FeatureState, FileAsk, NewProject, ProjectMember, SeedFeatures, SessionId, StartSession, UpsertFeatures};

use super::operator_session::switched;
use super::peer::Placing;
use super::session_host::{eventually, rig_placing, Rig};
use crate::feature_library::Seed;

const OPERATOR_PORT: u16 = 50999;
const JOB_PORT: u16 = 50001;
const HOST_PORT: u16 = 12345;

pub(super) fn rig() -> Rig {
    let peers = Placing::nothing();
    peers.holding(1000, OPERATOR_PORT, HOST_PORT);
    peers.holding(1001, JOB_PORT, HOST_PORT);
    rig_placing(Some(peers))
}

fn the_operator() -> Option<api::Caller> {
    Some(api::Caller::at(format!("127.0.0.1:{OPERATOR_PORT}").parse().unwrap()))
}

fn a_job() -> Option<api::Caller> {
    Some(api::Caller::at("127.0.0.1:50000".parse().unwrap()))
}

/// A Job's own agent, placed to its session by the connection it holds.
fn a_placed_job() -> Option<api::Caller> {
    Some(api::Caller::at(format!("127.0.0.1:{JOB_PORT}").parse().unwrap()))
}

fn code(refused: Refusal) -> (u16, String) {
    (refused.status(), refused.error().code.clone())
}

/// The Operator's session, started and its process up.
pub(super) async fn started(rig: &Rig) -> String {
    switched(rig, true).await;
    rig.fleet.operator_session_kept().await;
    let id = rig.fleet.operator_session_id().await.expect("started");
    eventually(|| async { rig.stand_in.starts().iter().any(|one| one.session == id) }).await;
    id
}

pub(super) fn heard(rig: &Rig, containing: &str) -> usize {
    rig.stand_in.process(0).sent.lock().unwrap().iter().filter(|line| line.contains(containing)).count()
}

pub(super) async fn a_project(rig: &Rig) -> String {
    let members = Some(vec![ProjectMember { manifest_id: rig.manifest.clone(), folder: None }]);
    rig.fleet.add_project(NewProject { name: "Armada".to_string(), members, place: None, values: None }).await.expect("added").projects.last().expect("the project").id.clone()
}

/// The repository holds what the seed's marker names, so the seed describes it.
fn armadas(rig: &Rig) {
    for path in Seed::armada().expect("reads").marker {
        std::fs::create_dir_all(std::path::Path::new(&rig.root).join(path)).expect("made");
    }
}

#[test]
fn the_seed_holds_eighteen_areas_whose_features_each_name_their_code() {
    let seed = Seed::armada().expect("the seed Fleet carries reads");
    assert_eq!(seed.areas.len(), 18, "the owner approved eighteen areas");
    let areas: BTreeSet<&str> = seed.areas.iter().map(|one| one.slug.as_str()).collect();
    assert_eq!(areas.len(), 18, "an area is named once");
    let mut slugs = BTreeSet::new();
    for feature in &seed.features {
        assert!(slugs.insert(feature.slug.as_str()), "`{}` is named twice", feature.slug);
        assert!(areas.contains(feature.area.as_str()), "`{}` is in `{}`, which is no area", feature.slug, feature.area);
        assert!(feature.slug.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'), "`{}` is not a slug a trailer can carry", feature.slug);
        assert!(!feature.title.trim().is_empty() && !feature.summary.trim().is_empty(), "`{}` needs its title and its line", feature.slug);
        assert!(!feature.paths.is_empty() && !feature.entry_points.is_empty(), "`{}` names no code", feature.slug);
    }
    for area in &seed.areas {
        assert!(seed.features.iter().any(|one| one.area == area.slug), "`{}` holds no feature", area.slug);
    }
}

#[tokio::test]
async fn only_the_operator_seeds_a_project_and_the_seed_is_kept_once() {
    let rig = rig();
    started(&rig).await;
    armadas(&rig);
    let project = a_project(&rig).await;
    let seed = || SeedFeatures { project: project.clone() };

    let refused = rig.fleet.seed_features(a_job(), seed()).await.expect_err("a Job is not the Operator");
    assert_eq!(code(refused).1, "fleet.not_the_operator");
    let refused = rig.fleet.seed_features(None, seed()).await.expect_err("nobody is not the Operator");
    assert_eq!(code(refused).1, "fleet.not_the_operator");
    assert!(!rig.fleet.store().lock().await.project_has_features(&project).unwrap(), "nothing was kept");

    let before = rig.fleet.now().as_str().to_string();
    let kept = rig.fleet.seed_features(the_operator(), seed()).await.expect("seeded");
    let carried = Seed::armada().expect("reads");
    assert_eq!(kept.created.len(), carried.features.len());
    assert_eq!(kept.areas, 18);

    let library = rig.fleet.list_features(None, Some(project.clone())).await.expect("listed");
    assert_eq!(library.areas.len(), 18);
    assert!(library.areas.iter().flat_map(|area| &area.features).all(|row| row.state == ipc::FeatureState::Seeded));
    for feature in rig.fleet.store().lock().await.features_in(&project).expect("read") {
        assert!(feature.added_at >= before, "`{}` is stamped when it was kept, never earlier: {}", feature.slug, feature.added_at);
    }

    let again = rig.fleet.seed_features(the_operator(), seed()).await.expect_err("kept once");
    assert_eq!(code(again), (409, "fleet.feature_seeded".to_string()));
}

#[tokio::test]
async fn a_project_none_of_whose_repositories_is_armadas_is_not_seeded() {
    let rig = rig();
    started(&rig).await;
    let project = a_project(&rig).await;
    let refused = rig.fleet.seed_features(the_operator(), SeedFeatures { project: project.clone() }).await.expect_err("not Armada's");
    assert_eq!(code(refused), (422, "fleet.seed_not_for_project".to_string()));
    let unknown = rig.fleet.seed_features(the_operator(), SeedFeatures { project: "no-such".into() }).await.expect_err("no project");
    assert_eq!(code(unknown).1, "fleet.feature_project");
    assert!(!rig.fleet.store().lock().await.project_has_features(&project).unwrap());
}

#[tokio::test]
async fn a_path_the_repository_does_not_hold_is_left_out_and_named() {
    let rig = rig();
    started(&rig).await;
    armadas(&rig);
    let project = a_project(&rig).await;
    let carried = Seed::armada().expect("reads");
    let first = &carried.features[0];

    let kept = rig.fleet.seed_features(the_operator(), SeedFeatures { project: project.clone() }).await.expect("seeded");
    assert!(kept.dropped.iter().any(|one| one.starts_with(&format!("{}: ", first.slug))), "the rig holds none of its code: {:?}", kept.dropped);
    let page = rig.fleet.get_feature_page(None, Some(project), first.slug.clone(), None, false).await.expect("page");
    assert!(page.paths.iter().all(|path| std::path::Path::new(&rig.root).join(path).exists()), "only what the repository holds is owned: {:?}", page.paths);
}

#[tokio::test]
async fn fleet_seeds_a_project_with_armadas_repository_and_no_feature_with_the_operator_off_and_once() {
    let rig = rig();
    armadas(&rig);
    let project = a_project(&rig).await;
    assert!(!rig.fleet.operator_on(), "the Operator is off");

    rig.fleet.projects_seeded().await;
    let carried = Seed::armada().expect("reads");
    let library = rig.fleet.list_features(None, Some(project.clone())).await.expect("listed");
    assert_eq!(library.areas.len(), carried.areas.len());
    assert_eq!(library.areas.iter().map(|area| area.features.len()).sum::<usize>(), carried.features.len());
    assert!(library.areas.iter().flat_map(|area| &area.features).all(|row| row.state == FeatureState::Seeded));

    let stamps: Vec<String> = rig.fleet.store().lock().await.features_in(&project).expect("read").into_iter().map(|one| one.added_at).collect();
    rig.fleet.projects_seeded().await;
    let again: Vec<String> = rig.fleet.store().lock().await.features_in(&project).expect("read").into_iter().map(|one| one.added_at).collect();
    assert_eq!(stamps, again, "a second pass changes nothing");
}

#[tokio::test]
async fn a_project_that_is_not_armadas_is_never_seeded_by_fleet() {
    let rig = rig();
    let project = a_project(&rig).await;
    rig.fleet.projects_seeded().await;
    rig.fleet.projects_seeded().await;
    assert!(!rig.fleet.store().lock().await.project_has_features(&project).unwrap());
}

// Asks from outside a Job.

pub(super) const VOTING: &str = "voting-on-takes";
pub(super) const ACTIONS: &str = "page-actions";

/// A small library: two features in one area, `VOTING` owning `crates/pages/voting`.
pub(super) async fn libraried(rig: &Rig, project: &str) {
    let feature = |slug: &str, paths: &[&str], entry: &[&str]| FeatureSeed {
        slug: slug.to_string(),
        area: "takes".to_string(),
        title: slug.to_string(),
        summary: "One line.".to_string(),
        state: None,
        paths: paths.iter().map(|one| one.to_string()).collect(),
        entry_points: entry.iter().map(|one| one.to_string()).collect(),
        manifest_id: None,
    };
    let upsert = UpsertFeatures {
        project: project.to_string(),
        areas: vec![AreaSeed { slug: "takes".into(), title: "Takes".into() }],
        features: vec![
            feature(VOTING, &["crates/pages/voting"], &["crates/pages/voting/vote_button.rs"]),
            feature(ACTIONS, &["crates/pages/actions"], &[]),
        ],
    };
    rig.fleet.upsert_features(None, upsert).await.expect("kept");
}

fn filing(project: &str, words: &str, features: &[&str]) -> FileAsk {
    FileAsk { project: project.to_string(), kind: AskKind::Idea, words: words.to_string(), title: None, features: features.iter().map(|one| one.to_string()).collect(), note: None }
}

#[tokio::test]
async fn an_ask_said_outside_a_job_is_filed_under_its_features_with_no_job_and_shows_on_each() {
    let rig = rig();
    started(&rig).await;
    let project = a_project(&rig).await;
    libraried(&rig, &project).await;

    let refused = rig.fleet.file_ask(a_job(), filing(&project, "Vote from the phone", &[VOTING])).await.expect_err("a Job is not the Operator");
    assert_eq!(code(refused).1, "fleet.not_the_operator");
    for (file, expected) in [
        (filing(&project, "  ", &[VOTING]), (422, "fleet.ask_words_blank")),
        (filing("no-such", "Vote from the phone", &[]), (422, "fleet.feature_project")),
        (filing(&project, "Vote from the phone", &["no-such-feature"]), (404, "fleet.feature_unknown")),
    ] {
        let refused = rig.fleet.file_ask(the_operator(), file).await.expect_err("refused");
        assert_eq!(code(refused), (expected.0, expected.1.to_string()));
    }
    let none = rig.fleet.list_feature_asks(project.clone(), None).await.expect("listed");
    assert!(none.asks.is_empty(), "nothing was kept: {:?}", none.asks);

    let ask = rig.fleet.file_ask(the_operator(), filing(&project, "Vote from the phone\nOn the page I am reading", &[VOTING, ACTIONS])).await.expect("filed");
    assert_eq!((ask.kind, ask.state), (AskKind::Idea, AskState::Backlog), "nothing builds it yet");
    assert!(ask.said_in.as_str().is_empty() && ask.building.is_none(), "no Job: {ask:?}");
    assert_eq!(ask.first_words, "Vote from the phone\nOn the page I am reading");
    assert_eq!(ask.title, "Vote from the phone");
    assert_eq!(ask.features, [ACTIONS.to_string(), VOTING.to_string()]);

    for slug in [VOTING, ACTIONS] {
        let listed = rig.fleet.list_feature_asks(project.clone(), Some(slug.to_string())).await.expect("listed");
        assert_eq!(listed.asks.iter().map(|one| one.ask_id.as_str()).collect::<Vec<_>>(), [ask.ask_id.as_str()], "on {slug}");
    }
}

#[tokio::test]
async fn a_note_filed_twice_is_one_ask_and_the_second_call_changes_nothing() {
    let rig = rig();
    started(&rig).await;
    let project = a_project(&rig).await;
    libraried(&rig, &project).await;
    let mut file = filing(&project, "The rank bar is too tight", &[VOTING]);
    file.note = Some("mfz1-abc".to_string());
    file.kind = AskKind::Bug;

    let first = rig.fleet.file_ask(the_operator(), file.clone()).await.expect("filed");
    assert_eq!(first.first_row, "note:mfz1-abc");
    file.features = vec![ACTIONS.to_string()];
    file.words = "Different words".to_string();
    let second = rig.fleet.file_ask(the_operator(), file).await.expect("filed again");
    assert_eq!(second, first, "the note's own ask, as it was");
    assert_eq!(rig.fleet.list_feature_asks(project, None).await.expect("listed").asks.len(), 1);
}

#[tokio::test]
async fn a_note_points_to_the_features_of_the_files_it_was_drawn_from_and_failing_those_its_component() {
    let rig = rig();
    started(&rig).await;
    let project = a_project(&rig).await;
    libraried(&rig, &project).await;
    let manifest = rig.manifest.as_str().to_string();
    let read = |files: &[&str], component: Option<&str>| {
        let files: Vec<String> = files.iter().map(|one| one.to_string()).collect();
        let (project, manifest) = (project.clone(), manifest.clone());
        let component = component.map(str::to_string);
        let fleet = rig.fleet.clone();
        async move {
            let store = fleet.store().lock().await;
            crate::feature_library::features_of_note(&store, &project, &manifest, &files, component.as_deref()).expect("read")
        }
    };
    assert_eq!(read(&["crates/pages/voting/ranks.rs"], Some("PageActions")).await, [VOTING.to_string()], "the file wins over the name");
    assert_eq!(read(&[], Some("VoteButton")).await, [VOTING.to_string()], "a component named for an entry point's file");
    assert!(read(&[], Some("Rail")).await.is_empty(), "a name no file carries points nowhere");
    assert!(read(&["docs/elsewhere.md"], None).await.is_empty());

    let told = rig.fleet.note_features(&manifest, &["crates/pages/voting/ranks.rs".to_string()], None).await;
    assert!(told.contains(&format!("project {project}: {VOTING}")), "{told}");
    let arrival = crate::operator::Note { id: "n-1".into(), manifest_id: manifest, text: "Too tight".into(), screen: Some("takes".into()), component: None, features: told }.arrival();
    assert!(arrival.words.contains("Left on the takes screen") && arrival.words.contains(VOTING), "{}", arrival.words);
}

// A feature a Job names that the library does not hold, and what the Operator does with it.

pub(super) async fn a_job_in(rig: &Rig, project: &str) -> SessionId {
    let start = StartSession {
        project: Some(project.to_string()),
        manifest_id: rig.manifest.clone(),
        title: Some("Rank the takes".into()),
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
    id
}

pub(super) fn trailer(slug: &str, sha: &str, files: &[&str]) -> NamingCommit {
    NamingCommit { sha: sha.to_string(), features: vec![slug.to_string()], files: files.iter().map(|one| one.to_string()).collect() }
}

pub(super) async fn commits_filed(rig: &Rig, job: &SessionId, named: &[NamingCommit]) {
    let now = rig.fleet.now().as_str().to_string();
    let mut store = rig.fleet.store().lock().await;
    rig.fleet.commits_filed_in(&mut store, job.as_str(), named, &now);
}

async fn state_of(rig: &Rig, project: &str, slug: &str) -> Option<(FeatureState, String)> {
    let library = rig.fleet.list_features(None, Some(project.to_string())).await.expect("listed");
    library.areas.into_iter().flat_map(|area| area.features).find(|row| row.slug == slug).map(|row| (row.state, row.area))
}

#[tokio::test]
async fn a_slug_a_job_names_that_the_library_lacks_is_filed_under_a_pending_feature_and_an_unseeded_project_files_nothing() {
    let rig = rig();
    let project = a_project(&rig).await;
    let job = a_job_in(&rig, &project).await;
    commits_filed(&rig, &job, &[trailer("rank-votes", "abc1", &["crates/pages/voting/ranks.rs"])]).await;
    assert!(!rig.fleet.store().lock().await.project_has_features(&project).unwrap(), "a project with no library makes no pending feature, so its seeded-at is never moved");

    libraried(&rig, &project).await;
    let before = rig.fleet.store().lock().await.features_in(&project).expect("read");
    commits_filed(&rig, &job, &[trailer("rank-votes", "abc1", &["crates/pages/voting/ranks.rs"])]).await;
    commits_filed(&rig, &job, &[trailer("rank-votes", "abc1", &["crates/pages/voting/ranks.rs"]), trailer("not a slug", "abc2", &[])]).await;

    assert_eq!(state_of(&rig, &project, "rank-votes").await, Some((FeatureState::Pending, "pending".to_string())));
    assert_eq!(state_of(&rig, &project, VOTING).await.map(|one| one.0), Some(FeatureState::Seeded), "a feature that was there is as it was");
    let page = rig.fleet.get_feature_page(None, Some(project.clone()), "rank-votes".to_string(), None, false).await.expect("page");
    assert_eq!(page.files.iter().map(|one| one.path.as_str()).collect::<Vec<_>>(), ["crates/pages/voting/ranks.rs"]);
    assert!(page.links.iter().any(|one| one.target == "abc1" && one.via == "trailer"), "{:?}", page.links);
    let after = rig.fleet.store().lock().await.features_in(&project).expect("read");
    assert_eq!(after.len(), before.len() + 1, "one pending feature, named twice");
    for old in before {
        assert_eq!(after.iter().find(|one| one.slug == old.slug).map(|one| one.added_at.clone()), Some(old.added_at), "the catalog is never back-dated");
    }
}

#[tokio::test]
async fn an_act_that_names_a_feature_the_library_lacks_files_its_artifact_under_a_pending_one() {
    let rig = rig();
    let project = a_project(&rig).await;
    libraried(&rig, &project).await;
    let job = a_job_in(&rig, &project).await;
    let detail = std::collections::BTreeMap::from([("feature".to_string(), "phone-voting".to_string()), ("form".to_string(), "sketch".to_string())]);
    let now = rig.fleet.now().as_str().to_string();
    rig.fleet.artifact_filed(&mut *rig.fleet.store().lock().await, job.as_str(), "sketches/phone.html", &detail, &now);

    assert_eq!(state_of(&rig, &project, "phone-voting").await.map(|one| one.0), Some(FeatureState::Pending));
    let page = rig.fleet.get_feature_page(None, Some(project), "phone-voting".to_string(), None, false).await.expect("page");
    assert!(page.links.iter().any(|one| one.target == "sketches/phone.html" && one.via == "artifact"));
}

#[tokio::test]
async fn the_operator_is_told_of_a_pending_feature_once_with_who_named_it_and_the_features_that_cover_its_files() {
    let rig = rig();
    started(&rig).await;
    let project = a_project(&rig).await;
    libraried(&rig, &project).await;
    let job = a_job_in(&rig, &project).await;
    commits_filed(&rig, &job, &[trailer("rank-votes", "abc1", &["crates/pages/voting/ranks.rs"])]).await;

    rig.fleet.sources_handed().await;
    eventually(|| async { heard(&rig, "rank-votes") >= 1 }).await;
    assert_eq!(heard(&rig, "Rank the takes"), 1, "names the Job");
    assert_eq!(heard(&rig, "crates/pages/voting/ranks.rs"), 1, "names the file");
    assert_eq!(heard(&rig, "voting-on-takes (1)"), 1, "and the feature whose prefix covers it");
    assert_eq!(heard(&rig, "change_feature"), 1);
    rig.fleet.sources_handed().await;
    assert_eq!(heard(&rig, "rank-votes"), 1, "told once");
}

#[tokio::test]
async fn a_pending_feature_merged_into_another_moves_what_it_holds_and_its_old_slug_files_under_the_target() {
    let rig = rig();
    started(&rig).await;
    let project = a_project(&rig).await;
    libraried(&rig, &project).await;
    let job = a_job_in(&rig, &project).await;
    commits_filed(&rig, &job, &[trailer("rank-votes", "abc1", &["crates/pages/voting/ranks.rs"])]).await;
    let ask = rig.fleet.file_ask(the_operator(), filing(&project, "Rank them", &[VOTING])).await.expect("filed");
    rig.fleet.change_ask(ipc::ChangeAsk { ask_id: ask.ask_id.clone(), state: None, kind: None, title: None, features: Some(vec!["rank-votes".to_string(), VOTING.to_string()]) }).await.expect("filed twice");

    let change = |into: Option<&str>, slug: Option<&str>, title: Option<&str>| ChangeFeature {
        project: project.clone(),
        feature: "rank-votes".to_string(),
        into: into.map(String::from),
        slug: slug.map(String::from),
        title: title.map(String::from),
        area: None,
        why: None,
    };
    let refused = rig.fleet.change_feature(a_job(), change(Some(VOTING), None, None)).await.expect_err("not the Operator");
    assert_eq!(code(refused).1, "fleet.not_the_operator");
    for (one, expected) in [
        (change(Some(VOTING), Some("x"), None), (422, "fleet.feature_change")),
        (change(None, None, None), (422, "fleet.feature_change")),
        (change(Some("rank-votes"), None, None), (422, "fleet.feature_change")),
        (change(Some("no-such"), None, None), (404, "fleet.feature_unknown")),
        (change(None, Some("Not A Slug"), None), (422, "fleet.feature_slug")),
        (change(None, Some(ACTIONS), None), (409, "fleet.feature_slug_taken")),
    ] {
        let refused = rig.fleet.change_feature(the_operator(), one).await.expect_err("refused");
        assert_eq!(code(refused), (expected.0, expected.1.to_string()));
    }
    assert!(state_of(&rig, &project, "rank-votes").await.is_some(), "nothing was kept");

    let done = rig.fleet.change_feature(the_operator(), change(Some(VOTING), None, None)).await.expect("merged");
    assert_eq!((done.feature.as_str(), done.merged_into.as_deref()), (VOTING, Some(VOTING)));
    assert!(state_of(&rig, &project, "rank-votes").await.is_none(), "it is gone");
    let page = rig.fleet.get_feature_page(None, Some(project.clone()), VOTING.to_string(), None, false).await.expect("page");
    assert!(page.links.iter().any(|one| one.target == "abc1"), "its commit is the target's now");
    assert!(page.files.iter().any(|one| one.path == "crates/pages/voting/ranks.rs"), "and its file");
    let asks = rig.fleet.list_feature_asks(project.clone(), Some(VOTING.to_string())).await.expect("listed");
    assert_eq!(asks.asks[0].features, [VOTING.to_string()], "an ask on both is on the target once");
    let by_alias = rig.fleet.get_feature_page(None, Some(project.clone()), "rank-votes".to_string(), None, false).await.expect("the old slug reads");
    assert_eq!(by_alias.feature.slug, VOTING);

    let before = rig.fleet.store().lock().await.features_in(&project).expect("read").len();
    commits_filed(&rig, &job, &[trailer("rank-votes", "abc3", &["crates/pages/voting/more.rs"])]).await;
    assert_eq!(rig.fleet.store().lock().await.features_in(&project).expect("read").len(), before, "the old slug makes no new pending feature");
    let page = rig.fleet.get_feature_page(None, Some(project), VOTING.to_string(), None, false).await.expect("page");
    assert!(page.links.iter().any(|one| one.target == "abc3"), "it filed under the target");
}

#[tokio::test]
async fn a_pending_feature_is_confirmed_by_an_upsert_and_keeps_what_was_filed_under_it() {
    let rig = rig();
    let project = a_project(&rig).await;
    libraried(&rig, &project).await;
    let job = a_job_in(&rig, &project).await;
    commits_filed(&rig, &job, &[trailer("rank-votes", "abc1", &["crates/pages/voting/ranks.rs"])]).await;
    let id = rig.fleet.store().lock().await.feature_named(&project, "rank-votes").expect("read").expect("pending").id;

    let confirm = UpsertFeatures {
        project: project.clone(),
        areas: Vec::new(),
        features: vec![FeatureSeed {
            slug: "rank-votes".to_string(),
            area: "takes".to_string(),
            title: "Rank votes".to_string(),
            summary: "Order the takes by votes.".to_string(),
            state: Some(FeatureState::Confirmed),
            paths: vec!["crates/pages/voting/ranks.rs".to_string()],
            entry_points: Vec::new(),
            manifest_id: None,
        }],
    };
    rig.fleet.upsert_features(None, confirm).await.expect("confirmed");
    assert_eq!(state_of(&rig, &project, "rank-votes").await, Some((FeatureState::Confirmed, "takes".to_string())));
    let kept = rig.fleet.store().lock().await.feature_named(&project, "rank-votes").expect("read").expect("held");
    assert_eq!(kept.id, id, "the same feature");
    let page = rig.fleet.get_feature_page(None, Some(project), "rank-votes".to_string(), None, false).await.expect("page");
    assert!(page.links.iter().any(|one| one.target == "abc1"), "what was filed stays");
}

#[tokio::test]
async fn a_feature_renamed_keeps_its_old_slug_as_an_alias_and_its_asks_follow_it() {
    let rig = rig();
    started(&rig).await;
    let project = a_project(&rig).await;
    libraried(&rig, &project).await;
    let ask = rig.fleet.file_ask(the_operator(), filing(&project, "Buttons on a page", &[ACTIONS])).await.expect("filed");

    let rename = ChangeFeature { project: project.clone(), feature: ACTIONS.to_string(), into: None, slug: Some("page-buttons".to_string()), title: Some("Page buttons".to_string()), area: None, why: None };
    let done = rig.fleet.change_feature(the_operator(), rename).await.expect("renamed");
    assert_eq!((done.feature.as_str(), done.merged_into), ("page-buttons", None));
    assert!(state_of(&rig, &project, ACTIONS).await.is_none() && state_of(&rig, &project, "page-buttons").await.is_some());
    let page = rig.fleet.get_feature_page(None, Some(project.clone()), ACTIONS.to_string(), None, false).await.expect("the old slug reads");
    assert_eq!((page.feature.slug.as_str(), page.feature.title.as_str()), ("page-buttons", "Page buttons"));
    let asks = rig.fleet.list_feature_asks(project.clone(), Some("page-buttons".to_string())).await.expect("listed");
    assert_eq!(asks.asks.iter().map(|one| one.ask_id.clone()).collect::<Vec<_>>(), [ask.ask_id]);
    assert_eq!(asks.asks[0].features, ["page-buttons".to_string()]);

    let retitle = ChangeFeature { project, feature: "page-buttons".to_string(), into: None, slug: None, title: Some("Buttons".to_string()), area: None, why: None };
    assert_eq!(rig.fleet.change_feature(the_operator(), retitle).await.expect("retitled").feature, "page-buttons");
}

#[tokio::test]
async fn two_seeded_features_that_are_one_merge_with_their_code_and_the_target_keeps_its_date() {
    let rig = rig();
    started(&rig).await;
    let project = a_project(&rig).await;
    libraried(&rig, &project).await;
    let was = rig.fleet.store().lock().await.feature_named(&project, VOTING).expect("read").expect("held");

    let merge = ChangeFeature { project: project.clone(), feature: ACTIONS.to_string(), into: Some(VOTING.to_string()), slug: None, title: None, area: None, why: None };
    rig.fleet.change_feature(the_operator(), merge).await.expect("merged");
    let now = rig.fleet.store().lock().await.feature_named(&project, VOTING).expect("read").expect("held");
    assert_eq!((now.id.as_str(), now.added_at.as_str(), now.state.as_str()), (was.id.as_str(), was.added_at.as_str(), "seeded"));
    assert_eq!(now.paths, ["crates/pages/actions".to_string(), "crates/pages/voting".to_string()], "both own the target's code now");

    let undo = UpsertFeatures {
        project: project.clone(),
        areas: Vec::new(),
        features: vec![FeatureSeed {
            slug: ACTIONS.to_string(),
            area: "takes".to_string(),
            title: ACTIONS.to_string(),
            summary: "One line.".to_string(),
            state: None,
            paths: vec!["crates/pages/actions".to_string()],
            entry_points: Vec::new(),
            manifest_id: None,
        }],
    };
    rig.fleet.upsert_features(None, undo).await.expect("kept again");
    assert_eq!(state_of(&rig, &project, ACTIONS).await.map(|one| one.0), Some(FeatureState::Seeded), "a merge is undone by keeping the old feature again");
    assert!(rig.fleet.store().lock().await.feature_aliases(&project).expect("read").is_empty(), "and the alias goes");
}

// An area that grows past about eight features is told to the Operator, who proposes a split on a card.

/// `count` more features in the area `takes`, past the two `libraried` keeps.
pub(super) async fn grown(rig: &Rig, project: &str, count: usize) {
    let features = (1..=count)
        .map(|n| FeatureSeed {
            slug: format!("take-{n}"),
            area: "takes".to_string(),
            title: format!("Take {n}"),
            summary: "One line.".to_string(),
            state: None,
            paths: Vec::new(),
            entry_points: Vec::new(),
            manifest_id: None,
        })
        .collect();
    rig.fleet.upsert_features(None, UpsertFeatures { project: project.to_string(), areas: Vec::new(), features }).await.expect("kept");
}

#[tokio::test]
async fn an_area_past_eight_features_is_told_to_the_operator_once_with_its_features_and_eight_is_not() {
    let rig = rig();
    started(&rig).await;
    let project = a_project(&rig).await;
    libraried(&rig, &project).await;
    grown(&rig, &project, 6).await;
    rig.fleet.sources_handed().await;
    assert_eq!(heard(&rig, "split"), 0, "eight features are not too many");

    grown(&rig, &project, 7).await;
    rig.fleet.sources_handed().await;
    eventually(|| async { heard(&rig, "split") >= 1 }).await;
    assert_eq!(heard(&rig, "take-7"), 1, "names the features it holds");
    assert_eq!(heard(&rig, "`change_area` and the act `split`"), 1, "says how to split it");
    rig.fleet.sources_handed().await;
    assert_eq!(heard(&rig, "take-7"), 1, "told once");
}

#[tokio::test]
async fn a_pending_feature_is_not_counted_toward_an_areas_eight() {
    let rig = rig();
    started(&rig).await;
    let project = a_project(&rig).await;
    libraried(&rig, &project).await;
    grown(&rig, &project, 6).await;
    let job = a_job_in(&rig, &project).await;
    commits_filed(&rig, &job, &[trailer("rank-votes", "abc1", &[])]).await;
    assert_eq!(state_of(&rig, &project, "rank-votes").await.map(|one| one.0), Some(FeatureState::Pending));
    rig.fleet.sources_handed().await;
    eventually(|| async { heard(&rig, "rank-votes") >= 1 }).await;
    assert_eq!(heard(&rig, "split"), 0);
}

#[tokio::test]
async fn a_feature_moves_to_an_area_the_project_keeps_and_to_no_other() {
    let rig = rig();
    started(&rig).await;
    let project = a_project(&rig).await;
    libraried(&rig, &project).await;
    rig.fleet
        .upsert_features(None, UpsertFeatures { project: project.clone(), areas: vec![AreaSeed { slug: "actions".into(), title: "Actions".into() }], features: Vec::new() })
        .await
        .expect("kept");
    let ask = rig.fleet.file_ask(the_operator(), filing(&project, "Buttons on a page", &[ACTIONS])).await.expect("filed");
    let move_to = |area: &str, into: Option<&str>| ChangeFeature {
        project: project.clone(),
        feature: ACTIONS.to_string(),
        into: into.map(String::from),
        slug: None,
        title: None,
        area: Some(area.to_string()),
        why: None,
    };

    let refused = rig.fleet.change_feature(a_job(), move_to("actions", None)).await.expect_err("not the Operator");
    assert_eq!(code(refused).1, "fleet.not_the_operator");
    let refused = rig.fleet.change_feature(the_operator(), move_to("no-such-area", None)).await.expect_err("no such area");
    assert_eq!(code(refused), (422, "fleet.feature_area_unknown".to_string()));
    let refused = rig.fleet.change_feature(the_operator(), move_to("actions", Some(VOTING))).await.expect_err("a merge does not move");
    assert_eq!(code(refused).1, "fleet.feature_change");
    assert_eq!(state_of(&rig, &project, ACTIONS).await, Some((FeatureState::Seeded, "takes".to_string())), "nothing moved on a refusal");

    let done = rig.fleet.change_feature(the_operator(), move_to("actions", None)).await.expect("moved");
    assert_eq!((done.feature.as_str(), done.merged_into), (ACTIONS, None));
    assert_eq!(state_of(&rig, &project, ACTIONS).await, Some((FeatureState::Seeded, "actions".to_string())));
    let asks = rig.fleet.list_feature_asks(project, Some(ACTIONS.to_string())).await.expect("asks");
    assert!(asks.asks.iter().any(|one| one.ask_id == ask.ask_id), "its asks stay with it");
}

// A note left on a screen, with the Operator off.

const RANK_BAR: &str = "kind: new_ask\nask_kind: bug\nask: none\ntitle: Rank bar is too tight";

/// A note in the served root's annotations, open and sent to no Job, drawn from `file`.
fn note_drawn_from(rig: &Rig, id: &str, file: &str) -> ipc::annotations::NoteRead {
    let dir = std::path::Path::new(&rig.root).join(".armada/annotations");
    std::fs::create_dir_all(&dir).expect("made");
    let body = format!(
        "{{\n  \"id\": \"{id}\",\n  \"status\": \"open\",\n  \"text\": \"The rank bar is too tight\",\n  \"source\": {{ \"file\": \"{file}\" }},\n  \
         \"element\": {{ \"tag\": \"div\" }},\n  \"screen\": \"takes\"\n}}\n"
    );
    std::fs::write(dir.join(format!("{id}.json")), &body).expect("written");
    ipc::annotations::note_read(body.as_bytes()).expect("a note")
}

/// A rig whose sorter answers `RANK_BAR`, with the Operator `on` or off and a library for the project.
async fn noted(on: bool) -> (Rig, String) {
    let judge = std::sync::Arc::new(testkit::FakeJudge::saying(RANK_BAR));
    let rig = super::session_host::rig_fitted(move |fittings| fittings.judge = judge);
    switched(&rig, on).await;
    let project = a_project(&rig).await;
    libraried(&rig, &project).await;
    rig.fleet.operator_seat().note_wait_is(std::time::Duration::ZERO);
    (rig, project)
}

#[tokio::test]
async fn with_the_operator_off_a_note_left_unsent_is_filed_under_the_features_of_its_file_and_a_second_filing_is_the_same_ask() {
    let (rig, project) = noted(false).await;
    rig.fleet.sources_handed().await;
    let read = note_drawn_from(&rig, "n-rank", "crates/pages/voting/ranks.rs");
    rig.fleet.sources_handed().await;

    let listed = rig.fleet.list_feature_asks(project.clone(), Some(VOTING.to_string())).await.expect("listed");
    assert_eq!(listed.asks.len(), 1, "{:?}", listed.asks);
    let ask = &listed.asks[0];
    assert_eq!((ask.kind, ask.state, ask.title.as_str()), (AskKind::Bug, AskState::Backlog, "Rank bar is too tight"));
    assert!(ask.said_in.as_str().is_empty() && ask.building.is_none(), "no Job: {ask:?}");
    assert_eq!((ask.features.as_slice(), ask.first_row.as_str()), ([VOTING.to_string()].as_slice(), "note:n-rank"));

    rig.fleet.note_filed(rig.manifest.as_str(), &read).await.expect("filed again");
    rig.fleet.sources_handed().await;
    assert_eq!(rig.fleet.list_feature_asks(project, None).await.expect("listed").asks.len(), 1, "the note's own ask, once");
}

#[tokio::test]
async fn with_the_operator_on_fleet_files_no_note_itself() {
    let (rig, project) = noted(true).await;
    rig.fleet.sources_handed().await;
    note_drawn_from(&rig, "n-rank", "crates/pages/voting/ranks.rs");
    rig.fleet.sources_handed().await;
    assert!(rig.fleet.list_feature_asks(project, None).await.expect("listed").asks.is_empty());
}

/// With the Operator off he confirms, merges, renames and splits from Bridge, which reaches Fleet by no door.
#[tokio::test]
async fn bridge_and_the_operator_keep_the_catalog_and_a_jobs_agent_does_not() {
    let rig = rig();
    started(&rig).await;
    let project = a_project(&rig).await;
    libraried(&rig, &project).await;
    a_job_in(&rig, &project).await;
    assert!(rig.fleet.session_holding(&api::Caller::at(format!("127.0.0.1:{JOB_PORT}").parse().unwrap())).is_some(), "the Job's agent is placed");

    let new_area = |slug: &str| UpsertFeatures { project: project.clone(), areas: vec![AreaSeed { slug: slug.into(), title: slug.into() }], features: Vec::new() };
    let retitle = |title: &str| ChangeFeature { project: project.clone(), feature: ACTIONS.to_string(), into: None, slug: None, title: Some(title.to_string()), area: None, why: None };
    let titled = || async { rig.fleet.store().lock().await.feature_named(&project, ACTIONS).expect("read").expect("held").title };
    let areas = || async { rig.fleet.store().lock().await.feature_areas(&project).expect("read").len() };
    let before = areas().await;

    for agent in [a_placed_job(), a_job()] {
        let refused = rig.fleet.upsert_features(agent, new_area("elsewhere")).await.expect_err("an agent keeps no catalog");
        assert_eq!(code(refused), (422, "fleet.not_the_operator".to_string()));
        let refused = rig.fleet.change_feature(agent, retitle("Buttons")).await.expect_err("an agent changes no feature");
        assert_eq!(code(refused), (422, "fleet.not_the_operator".to_string()));
    }
    assert_eq!((areas().await, titled().await.as_str()), (before, ACTIONS), "nothing was kept");

    rig.fleet.upsert_features(None, new_area("from-bridge")).await.expect("Bridge keeps an area on its own");
    rig.fleet.change_feature(None, retitle("From Bridge")).await.expect("Bridge retitles");
    assert_eq!((areas().await, titled().await.as_str()), (before + 1, "From Bridge"));

    rig.fleet.upsert_features(the_operator(), new_area("from-operator")).await.expect("the Operator keeps an area");
    rig.fleet.change_feature(the_operator(), retitle("From the Operator")).await.expect("the Operator retitles");
    assert_eq!((areas().await, titled().await.as_str()), (before + 2, "From the Operator"));
}

#[tokio::test]
async fn a_feature_left_pending_while_the_operator_was_off_is_not_told_when_it_comes_on_and_one_named_after_is() {
    let rig = rig();
    let project = a_project(&rig).await;
    libraried(&rig, &project).await;
    let job = a_job_in(&rig, &project).await;
    commits_filed(&rig, &job, &[trailer("rank-votes", "abc1", &[])]).await;

    let operator = started(&rig).await;
    let told = |containing: &str| {
        let process = rig.stand_in.process_of(&operator).expect("the Operator's process");
        let count = process.sent.lock().unwrap().iter().filter(|line| line.contains(containing)).count();
        count
    };
    rig.fleet.sources_handed().await;
    commits_filed(&rig, &job, &[trailer("vote-tally", "abc2", &[])]).await;
    rig.fleet.sources_handed().await;
    eventually(|| async { told("vote-tally") >= 1 }).await;
    assert_eq!(told("rank-votes"), 0, "what piled up while it was off is not handed to it");
}
