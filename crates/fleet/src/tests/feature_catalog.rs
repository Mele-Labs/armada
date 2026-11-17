//! The Operator keeps the catalog, written before the code: **Armada's own catalog is kept once from the seed Fleet carries,
//! on the Operator's word, and each feature is stamped with the moment it was kept.** What Jobs and he name later is told to
//! the Operator, who brings one card for it. `docs/concepts/feature-catalog.md`.

use std::collections::BTreeSet;

use api::{Features, Projects, Refusal, Story};
use ipc::{AreaSeed, AskKind, AskState, FeatureSeed, FileAsk, NewProject, ProjectMember, SeedFeatures, UpsertFeatures};

use super::operator_session::switched;
use super::peer::Placing;
use super::session_host::{eventually, rig_placing, Rig};
use crate::feature_library::Seed;

const OPERATOR_PORT: u16 = 50999;
const HOST_PORT: u16 = 12345;

fn rig() -> Rig {
    let peers = Placing::nothing();
    peers.holding(1000, OPERATOR_PORT, HOST_PORT);
    rig_placing(Some(peers))
}

fn the_operator() -> Option<api::Caller> {
    Some(api::Caller::at(format!("127.0.0.1:{OPERATOR_PORT}").parse().unwrap()))
}

fn a_job() -> Option<api::Caller> {
    Some(api::Caller::at("127.0.0.1:50000".parse().unwrap()))
}

fn code(refused: Refusal) -> (u16, String) {
    (refused.status(), refused.error().code.clone())
}

/// The Operator's session, started and its process up.
async fn started(rig: &Rig) -> String {
    switched(rig, true).await;
    rig.fleet.operator_session_kept().await;
    let id = rig.fleet.operator_session_id().await.expect("started");
    eventually(|| async { rig.stand_in.starts().iter().any(|one| one.session == id) }).await;
    id
}

fn heard(rig: &Rig, containing: &str) -> usize {
    rig.stand_in.process(0).sent.lock().unwrap().iter().filter(|line| line.contains(containing)).count()
}

async fn a_project(rig: &Rig) -> String {
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
    let page = rig.fleet.get_feature_page(None, Some(project), first.slug.clone()).await.expect("page");
    assert!(page.paths.iter().all(|path| std::path::Path::new(&rig.root).join(path).exists()), "only what the repository holds is owned: {:?}", page.paths);
}

#[tokio::test]
async fn a_project_with_armadas_repository_and_no_feature_is_told_to_the_operator_once() {
    let rig = rig();
    started(&rig).await;
    armadas(&rig);
    let project = a_project(&rig).await;

    rig.fleet.sources_handed().await;
    eventually(|| async { heard(&rig, "seed_features") >= 1 }).await;
    assert_eq!(heard(&rig, &project), 1, "names the project");
    rig.fleet.sources_handed().await;
    assert_eq!(heard(&rig, "seed_features"), 1, "told once");

    rig.fleet.seed_features(the_operator(), SeedFeatures { project: project.clone() }).await.expect("seeded");
    let other = rig.fleet.seed_features(the_operator(), SeedFeatures { project }).await.expect_err("kept once");
    assert_eq!(other.status(), 409);
}

#[tokio::test]
async fn a_project_that_is_not_armadas_is_never_told_of_a_seed() {
    let rig = rig();
    started(&rig).await;
    a_project(&rig).await;
    rig.fleet.sources_handed().await;
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    assert_eq!(heard(&rig, "seed_features"), 0);
}

// Asks from outside a Job.

const VOTING: &str = "voting-on-takes";
const ACTIONS: &str = "page-actions";

/// A small library: two features in one area, `VOTING` owning `crates/pages/voting`.
async fn libraried(rig: &Rig, project: &str) {
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
    rig.fleet.upsert_features(upsert).await.expect("kept");
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
