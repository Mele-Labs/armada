//! Which features some typed words are about, read by meaning through a fake on the model's seam, so no weights are
//! fetched. The fake puts a text on an axis by the topic it names, so a similarity is 1 for the same topic and 0 for another,
//! and what is held is Fleet's half: the order, the floor, the cap, what is kept between reads, and the empty answer
//! while no model is loaded.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use adapter_traits::{NotEmbedded, TextModel, TextReading};
use api::{Features, Projects};
use ipc::{AreaSeed, FeatureSeed, FeatureState, NewProject, ProjectMember, UpsertFeatures};

use super::librarian::rig;
use super::session_host::Rig;

const TOPICS: [&[&str]; 3] = [&["cloudflare"], &["password", "login"], &["invoice", "billing"]];

struct Topics {
    texts: AtomicUsize,
}

impl TextModel for Topics {
    fn model(&self) -> &str {
        "fake@0"
    }

    fn embed(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, NotEmbedded> {
        self.texts.fetch_add(texts.len(), Ordering::SeqCst);
        Ok(texts
            .iter()
            .map(|text| {
                let lower = text.to_lowercase();
                let mut vector: Vec<f32> = TOPICS.iter().map(|words| f32::from(u8::from(words.iter().any(|word| lower.contains(word))))).collect();
                vector.push(if vector.iter().all(|one| *one == 0.0) { 1.0 } else { 0.0 });
                let length = vector.iter().map(|one| one * one).sum::<f32>().sqrt();
                vector.iter().map(|one| one / length).collect()
            })
            .collect())
    }

    fn read(&self, _: TextReading, _: &[&str]) -> Result<Vec<f32>, NotEmbedded> {
        Err(NotEmbedded("not this reader".into()))
    }
}

fn seed(slug: &str, title: &str, summary: &str, state: Option<FeatureState>) -> FeatureSeed {
    FeatureSeed { slug: slug.into(), area: "books".into(), title: title.into(), summary: summary.into(), state, paths: Vec::new(), entry_points: Vec::new(), manifest_id: None }
}

/// A project of the rig's repository holding these features in one area.
async fn project_of(rig: &Rig, features: Vec<FeatureSeed>) -> String {
    let members = Some(vec![ProjectMember { manifest_id: rig.manifest.clone(), folder: None }]);
    let project = rig.fleet.add_project(NewProject { name: "Ledger".to_string(), members, place: None, values: None }).await.expect("added").projects.last().expect("the project").id.clone();
    let upsert = UpsertFeatures { project: project.clone(), areas: vec![AreaSeed { slug: "books".into(), title: "Books".into() }], features };
    rig.fleet.upsert_features(None, upsert).await.expect("kept");
    project
}

fn held() -> Vec<FeatureSeed> {
    vec![
        seed("sign-in", "Sign in", "Letting a person in with a password", None),
        seed("edge-billing", "Edge billing", "Cloudflare charges and the invoice for them", None),
        seed("store-deploy", "Store deploy", "Putting the store on Cloudflare", None),
        seed("draft-cloudflare", "Draft cloudflare", "A cloudflare feature the Operator has yet to bring", Some(FeatureState::Pending)),
    ]
}

async fn matched(rig: &Rig, project: &str, words: &str) -> Vec<String> {
    rig.fleet.get_features_by_meaning(None, Some(project.to_string()), words.to_string()).await.expect("read").slugs
}

#[tokio::test]
async fn words_closest_to_one_features_text_rank_it_first() {
    let rig = rig();
    rig.fleet.language_model().loaded(Arc::new(Topics { texts: AtomicUsize::new(0) }));
    let project = project_of(&rig, held()).await;
    assert_eq!(matched(&rig, &project, "cloudflare writes").await, ["store-deploy", "edge-billing"], "the one on that topic alone, then the one that shares it with another");
    assert_eq!(matched(&rig, &project, "invoice").await, ["edge-billing"]);
}

#[tokio::test]
async fn a_feature_under_the_floor_and_a_pending_one_are_left_out() {
    let rig = rig();
    rig.fleet.language_model().loaded(Arc::new(Topics { texts: AtomicUsize::new(0) }));
    let project = project_of(&rig, held()).await;
    let slugs = matched(&rig, &project, "cloudflare writes").await;
    assert!(!slugs.contains(&"sign-in".to_string()), "a feature on another topic is under the floor");
    assert!(!slugs.contains(&"draft-cloudflare".to_string()), "a pending feature is no feature of the catalogue");
    assert!(matched(&rig, &project, "kitten photos").await.is_empty(), "words about nothing the project holds name nothing");
}

#[tokio::test]
async fn it_names_at_most_eight() {
    let rig = rig();
    rig.fleet.language_model().loaded(Arc::new(Topics { texts: AtomicUsize::new(0) }));
    let many = (0..10).map(|at| seed(&format!("edge-{at}"), &format!("Edge {at}"), "Cloudflare", None)).collect();
    let project = project_of(&rig, many).await;
    assert_eq!(matched(&rig, &project, "cloudflare").await.len(), 8);
}

#[tokio::test]
async fn with_no_model_loaded_the_answer_is_empty_and_no_refusal() {
    let rig = rig();
    let project = project_of(&rig, held()).await;
    let answer = rig.fleet.get_features_by_meaning(None, Some(project.clone()), "cloudflare writes".to_string()).await.expect("never refused for want of the model");
    assert_eq!((answer.project, answer.slugs), (project, Vec::<String>::new()));
}

#[tokio::test]
async fn blank_words_name_nothing_and_a_feature_is_embedded_once() {
    let rig = rig();
    let model = Arc::new(Topics { texts: AtomicUsize::new(0) });
    rig.fleet.language_model().loaded(Arc::clone(&model) as Arc<dyn TextModel>);
    let project = project_of(&rig, held()).await;
    assert!(matched(&rig, &project, "   ").await.is_empty());
    assert_eq!(model.texts.load(Ordering::SeqCst), 0, "blank words embed nothing");
    matched(&rig, &project, "cloudflare").await;
    assert_eq!(model.texts.load(Ordering::SeqCst), 1 + 3, "the words and the three features of the catalogue");
    matched(&rig, &project, "invoice").await;
    assert_eq!(model.texts.load(Ordering::SeqCst), 1 + 3 + 1, "the second read embeds only its words");
}
