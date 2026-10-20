//! The local model reading which workflow a request is, through a fake on the same seam, so no
//! weights are fetched. `docs/spikes/workflow-pick-by-model.md` has the real model measured on 38
//! requests; here the fake puts a text on an axis by the topic it names, so a similarity is 1 for the
//! same topic and 0 for another, and what is held is Fleet's half: when a pick is clear, when it is
//! not, what is kept between reads, and that the reader is off.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use adapter_traits::{NotEmbedded, TextModel, TextReading};
use config::ResolvedWorkflow;
use core_model::WorkflowId;

use crate::language_model::{wanted_by, LanguageModel};
use crate::session_host::workflow_pick::{picked, picked_by, Vectors, IN_FORCE};

const TOPICS: [&[&str]; 3] = [&["crash", "broken"], &["screen", "option"], &["draft", "design"]];

struct Topics {
    embedded: Mutex<Vec<String>>,
    fails: bool,
    calls: AtomicUsize,
}

impl Topics {
    fn new(fails: bool) -> Topics {
        Topics { embedded: Mutex::new(Vec::new()), fails, calls: AtomicUsize::new(0) }
    }
}

impl TextModel for Topics {
    fn model(&self) -> &str {
        "fake@0"
    }

    fn embed(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, NotEmbedded> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        if self.fails {
            return Err(NotEmbedded("out of memory".into()));
        }
        let mut vectors = Vec::new();
        for text in texts {
            self.embedded.lock().unwrap().push((*text).to_string());
            let lower = text.to_lowercase();
            let mut vector: Vec<f32> =
                TOPICS.iter().map(|words| f32::from(u8::from(words.iter().any(|word| lower.contains(word))))).collect();
            vector.push(if vector.iter().all(|one| *one == 0.0) { 1.0 } else { 0.0 });
            let length = vector.iter().map(|one| one * one).sum::<f32>().sqrt();
            vectors.push(vector.iter().map(|one| one / length).collect());
        }
        Ok(vectors)
    }

    fn read(&self, _: TextReading, _: &[&str]) -> Result<Vec<f32>, NotEmbedded> {
        Err(NotEmbedded("not this reader".into()))
    }
}

fn workflow(id: &str, asked: &str) -> ResolvedWorkflow {
    let def = config::WorkflowDef::parse(
        std::path::Path::new("fixture.yml"),
        &format!(
            "version: 1\nworkflow_id: {id}\nname: {id}\nfor_requests: \"{asked}\"\nsteps:\n  - id: only\n    label: \"only\"\n    \
             evidence: {{submitted: {{type: diff}}}}\n    delivers: true\n    advance_gate: auto\n"
        ),
        &config::Roster::offering_nothing(),
    )
    .unwrap_or_else(|refused| panic!("the fixture workflow did not parse: {refused}"));
    let manifest = config::Manifest::parse(std::path::Path::new("fixture-armada.yml"), "version: 1\nid: 01FIXTUREMANIFEST\n")
        .expect("the fixture manifest parses");
    ResolvedWorkflow::resolve(&def, &manifest).unwrap_or_else(|refused| panic!("the fixture workflow did not resolve: {refused}"))
}

fn held() -> BTreeMap<WorkflowId, ResolvedWorkflow> {
    [
        workflow("bug", "Something that crashes and is wanted fixed"),
        workflow("feature", "A new screen or option"),
        workflow("design", "A draft before anything is built"),
    ]
    .into_iter()
    .map(|one| (one.id().clone(), one))
    .collect()
}

#[test]
fn a_request_on_one_workflows_topic_picks_it() {
    let model = Topics::new(false);
    let got = picked(&model, &Vectors::default(), "the importer crashes on an empty file", &held());
    assert_eq!(got.map(|id| id.as_str().to_string()), Some("bug".to_string()));
}

#[test]
fn a_request_between_two_workflows_picks_neither() {
    let model = Topics::new(false);
    let got = picked(&model, &Vectors::default(), "the option screen is broken", &held());
    assert_eq!(got, None, "two workflows tie, so Claude is asked");
}

#[test]
fn a_request_on_no_workflows_topic_picks_none() {
    let model = Topics::new(false);
    assert_eq!(picked(&model, &Vectors::default(), "rename the config file", &held()), None);
}

#[test]
fn a_repository_holding_no_workflow_picks_none() {
    let model = Topics::new(false);
    assert_eq!(picked(&model, &Vectors::default(), "the importer crashes", &BTreeMap::new()), None);
    assert_eq!(model.calls.load(Ordering::SeqCst), 0, "nothing to read against, so nothing embedded");
}

#[test]
fn a_model_that_fails_picks_none() {
    let model = Topics::new(true);
    assert_eq!(picked(&model, &Vectors::default(), "the importer crashes", &held()), None);
}

#[test]
fn a_second_read_embeds_only_its_request() {
    let model = Topics::new(false);
    let vectors = Vectors::default();
    picked(&model, &vectors, "the importer crashes", &held());
    model.embedded.lock().unwrap().clear();
    picked(&model, &vectors, "another crash", &held());
    assert_eq!(*model.embedded.lock().unwrap(), vec!["another crash".to_string()]);
}

#[tokio::test]
async fn the_reader_is_off_and_asks_nothing_of_a_loaded_model() {
    assert!(!IN_FORCE);
    assert!(!wanted_by(&[IN_FORCE]), "off, it does not make Fleet fetch the model");
    let language_model = LanguageModel::default();
    let model = Arc::new(Topics::new(false));
    language_model.loaded(model.clone());
    let got = picked_by(&language_model, "the importer crashes", &held()).await;
    assert_eq!(got, None);
    assert_eq!(model.calls.load(Ordering::SeqCst), 0);
}
