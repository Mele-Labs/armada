//! A project's glossary, written before the code: **a Job writes the terms it introduces when it ends, a term is one
//! per project whatever its case, a word a Job uses against a term is flagged on its feature and never refused, and Armada's
//! own glossary is kept once.** `docs/concepts/feature-catalog.md`, *Glossary*.

use api::{Features, HostedSessions, Projects};
use ipc::{AreaSeed, FeatureLinkKind, FeatureSeed, GlossaryTermWrite, NewProject, ProjectMember, SessionId, StartSession, UpsertFeatures, WriteFeatureSections};

use super::agent_starts::{caller, rig, Rig};
use super::session_host::rig_placing;
use crate::clashing::Who;
use crate::feature_library::Seed;

const VOTING: &str = "voting-on-takes";

async fn a_project(rig: &Rig) -> String {
    let members = Some(vec![ProjectMember { manifest_id: rig.manifest.clone(), folder: None }]);
    let new = NewProject { name: "Shop".to_string(), members, place: None, values: None };
    rig.fleet.add_project(new).await.expect("added").projects.last().expect("the project").id.clone()
}

/// A project of the repository a `session_host` rig holds.
async fn a_project_of(rig: &super::session_host::Rig) -> String {
    let members = Some(vec![ProjectMember { manifest_id: rig.manifest.clone(), folder: None }]);
    let new = NewProject { name: "Armada".to_string(), members, place: None, values: None };
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

async fn libraried(rig: &Rig, project: &str) {
    let upsert = UpsertFeatures {
        project: project.to_string(),
        areas: vec![AreaSeed { slug: "takes".into(), title: "Takes".into() }],
        features: vec![FeatureSeed {
            slug: VOTING.to_string(),
            area: "takes".to_string(),
            title: VOTING.to_string(),
            summary: "One line.".to_string(),
            state: None,
            paths: vec!["crates/pages/voting".to_string()],
            entry_points: Vec::new(),
            manifest_id: None,
        }],
    };
    rig.fleet.upsert_features(None, upsert).await.expect("seeded");
}

fn term(word: &str, meaning: &str, not: &[&str]) -> GlossaryTermWrite {
    GlossaryTermWrite { term: word.to_string(), meaning: meaning.to_string(), not: not.iter().map(|one| one.to_string()).collect() }
}

fn writing(id: &SessionId, terms: Vec<GlossaryTermWrite>) -> WriteFeatureSections {
    WriteFeatureSections { feature: VOTING.to_string(), sections: Vec::new(), glossary: terms, gists: Vec::new(), project: None, session_id: Some(id.clone()) }
}

#[tokio::test]
async fn a_term_a_job_writes_is_kept_with_the_words_not_to_use_and_served() {
    let rig = rig(None);
    let project = a_project(&rig).await;
    libraried(&rig, &project).await;
    let id = an_agent_for(&rig, &project).await;

    let written = rig.fleet.write_feature_sections(caller(), writing(&id, vec![term("Take", "One option on a page, voted on.", &["pick", " Choice ", "pick"])])).await.expect("written");
    assert_eq!(written.glossary, ["Take"]);
    assert!(written.written.is_empty(), "no section was written");

    let served = rig.fleet.get_glossary(None, Some(project.clone())).await.expect("served");
    assert_eq!(served.project, project);
    assert_eq!(served.terms.len(), 1);
    let kept = &served.terms[0];
    assert_eq!((kept.term.as_str(), kept.meaning.as_str(), kept.job.as_str()), ("Take", "One option on a page, voted on.", id.as_str()));
    assert_eq!(kept.not, ["pick", "Choice"], "trimmed, and a word is named once");
}

#[tokio::test]
async fn a_call_with_neither_sections_nor_terms_and_a_term_with_no_meaning_are_refused() {
    let rig = rig(None);
    let project = a_project(&rig).await;
    libraried(&rig, &project).await;
    let id = an_agent_for(&rig, &project).await;

    let empty = rig.fleet.write_feature_sections(caller(), writing(&id, Vec::new())).await.expect_err("nothing to write");
    assert_eq!(empty.error().code, "fleet.feature_section_empty");
    let blank = rig.fleet.write_feature_sections(caller(), writing(&id, vec![term("Take", "  ", &[])])).await.expect_err("no meaning");
    assert_eq!(blank.error().code, "fleet.glossary_blank");
    assert!(rig.fleet.get_glossary(None, Some(project)).await.expect("served").terms.is_empty());
}

#[tokio::test]
async fn a_repeat_write_replaces_the_term_whatever_its_case() {
    let rig = rig(None);
    let project = a_project(&rig).await;
    libraried(&rig, &project).await;
    let id = an_agent_for(&rig, &project).await;

    rig.fleet.write_feature_sections(caller(), writing(&id, vec![term("Take", "The old meaning.", &["pick"])])).await.expect("written");
    rig.fleet.write_feature_sections(caller(), writing(&id, vec![term("take", "The new meaning.", &["option"])])).await.expect("written");

    let served = rig.fleet.get_glossary(None, Some(project)).await.expect("served");
    assert_eq!(served.terms.len(), 1, "one term, not two");
    let kept = &served.terms[0];
    assert_eq!((kept.term.as_str(), kept.meaning.as_str()), ("take", "The new meaning."));
    assert_eq!(kept.not, ["option"], "the later write replaces the words too");
}

#[tokio::test]
async fn a_not_word_in_a_commit_is_flagged_on_the_jobs_feature_and_the_commit_is_never_refused() {
    let rig = rig(None);
    let project = a_project(&rig).await;
    libraried(&rig, &project).await;
    let id = an_agent_for(&rig, &project).await;
    rig.fleet.writes().wrote(&Who::Session(id.as_str().to_string()), rig.manifest.as_str(), ["crates/pages/voting/ranks.rs".to_string()], &rig.fleet.now());
    rig.fleet.feature_writes_filed().await;
    rig.fleet.write_feature_sections(caller(), writing(&id, vec![term("Take", "One option on a page.", &["pick"])])).await.expect("written");
    let commit = |message: &str| ipc::CheckCommitFeature {
        session_id: id.clone(),
        command: format!("git add crates/pick.rs && git commit -m \"{message}\" -m \"Feature: {VOTING}\""),
        files: vec!["crates/pages/voting/ranks.rs".to_string()],
    };
    let flags = || async {
        let page = rig.fleet.get_feature_page(None, Some(project.clone()), VOTING.to_string(), None, false).await.expect("page");
        page.links.into_iter().filter(|link| link.kind == FeatureLinkKind::GlossaryFlag).collect::<Vec<_>>()
    };

    assert!(rig.fleet.check_commit_feature(commit("Rank each take")).await.expect("asked").allowed);
    assert!(flags().await.is_empty(), "the word is not in the message, and a path given to git add is not read");

    assert!(rig.fleet.check_commit_feature(commit("Let a voter Pick a rank")).await.expect("asked").allowed, "a flag never refuses");
    assert!(rig.fleet.check_commit_feature(commit("Pick again")).await.expect("asked").allowed);
    let found = flags().await;
    assert_eq!(found.len(), 1, "the same word by the same Job is flagged once");
    assert_eq!((found[0].target.as_str(), found[0].title.as_str(), found[0].job.as_deref()), ("pick", "Take", Some(id.as_str())));
}

#[tokio::test]
async fn armadas_glossary_is_kept_once_for_a_project_that_holds_its_repository() {
    let rig = rig_placing(None);
    for path in Seed::armada().expect("reads").marker {
        std::fs::create_dir_all(std::path::Path::new(&rig.root).join(path)).expect("made");
    }
    let project = a_project_of(&rig).await;
    let carried = Seed::armada().expect("reads").glossary;
    assert!(!carried.is_empty() && carried.iter().all(|one| !one.not.is_empty()), "the seed lists only terms with words not to use");

    rig.fleet.glossaries_seeded().await;
    let kept = rig.fleet.get_glossary(None, Some(project.clone())).await.expect("served").terms;
    assert_eq!(kept.len(), carried.len());
    assert!(kept.iter().all(|one| one.job == "seed"));

    // A term rewritten since stays rewritten, and a second pass keeps nothing.
    rig.fleet
        .store()
        .lock()
        .await
        .put_glossary_term(&store::KeptTerm { project: project.clone(), term: "fleet".into(), meaning: "Rewritten.".into(), not: Vec::new(), job: "s-1".into(), at: "2026-11-18T10:00:00.000Z".into() })
        .expect("kept");
    rig.fleet.store().lock().await.put_glossary_term(&store::KeptTerm { project: project.clone(), term: "Gone".into(), meaning: "Added.".into(), not: Vec::new(), job: "s-1".into(), at: "2026-11-18T10:00:00.000Z".into() }).expect("kept");
    rig.fleet.glossaries_seeded().await;
    let again = rig.fleet.get_glossary(None, Some(project)).await.expect("served").terms;
    assert_eq!(again.len(), carried.len() + 1, "the seed was not kept a second time");
    assert_eq!(again.iter().find(|one| one.term == "fleet").map(|one| one.meaning.as_str()), Some("Rewritten."));
}

#[tokio::test]
async fn a_project_that_is_not_armadas_is_given_no_glossary() {
    let rig = rig_placing(None);
    let project = a_project_of(&rig).await;
    rig.fleet.glossaries_seeded().await;
    assert!(rig.fleet.get_glossary(None, Some(project)).await.expect("served").terms.is_empty());
}
