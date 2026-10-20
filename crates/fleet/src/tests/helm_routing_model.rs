//! The local model reading a thought before Claude is asked, through a fake on the same seam, so no
//! weights are fetched. `docs/spikes/thought-routing-by-model.md` has the real model measured on
//! the reads it was set against; here the fake puts a text on an axis by the topic it names, so a
//! similarity is 1 for the same topic and 0 for another, and what is held is Fleet's half: when a
//! match is clear, when it is not, and what is kept between reads.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use adapter_traits::{NotEmbedded, TextModel, TextReading};

use crate::helm::routing::matching::{clear_match, matched, matched_by, Vectors, LEADS_BY, ROUTES_AT};
use crate::helm::routing::Candidate;
use crate::language_model::{wanted_by, LanguageModel};
use crate::tests::helm_routing::a_live_record;

const TOPICS: [&[&str]; 3] = [&["retry", "retries", "charge"], &["clock", "time"], &["page", "list"]];

/// A text on the axis of the first topic it names; `both` names two and sits between them.
struct Topics {
    embedded: Mutex<Vec<String>>,
    calls: AtomicUsize,
}

impl Topics {
    fn new() -> Topics {
        Topics { embedded: Mutex::new(Vec::new()), calls: AtomicUsize::new(0) }
    }
}

impl TextModel for Topics {
    fn model(&self) -> &str {
        "fake@0"
    }

    fn embed(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, NotEmbedded> {
        self.calls.fetch_add(1, Ordering::SeqCst);
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

fn candidates(titles: &[(&str, &str)]) -> Vec<Candidate> {
    titles
        .iter()
        .map(|(id, title)| a_live_record(id, title, "billing", None))
        .filter_map(|record| Candidate::session(&record))
        .collect()
}

fn three() -> Vec<Candidate> {
    candidates(&[
        ("01SESSAAAAAAAAAAAAAAAAAAAA", "Back off the invoice retries"),
        ("01SESSBBBBBBBBBBBBBBBBBBBB", "Pin the clock"),
        ("01SESSCCCCCCCCCCCCCCCCCCCC", "Sessions list page"),
    ])
}

fn id_of(target: Option<ipc::ThoughtTarget>) -> Option<String> {
    target.map(|target| target.id)
}

#[test]
fn a_thought_about_one_sessions_topic_goes_to_it_at_once() {
    let target = matched(&Topics::new(), &Vectors::default(), "retries should wait longer", &three());
    assert_eq!(id_of(target).as_deref(), Some("01SESSAAAAAAAAAAAAAAAAAAAA"));
}

#[test]
fn a_thought_about_nothing_offered_is_left_to_claude() {
    assert_eq!(matched(&Topics::new(), &Vectors::default(), "rename the settings tab", &three()), None);
}

#[test]
fn two_sessions_that_match_equally_are_no_clear_match() {
    let twins = candidates(&[
        ("01SESSAAAAAAAAAAAAAAAAAAAA", "Retry invoice charges"),
        ("01SESSBBBBBBBBBBBBBBBBBBBB", "Retries for card charges"),
    ]);
    assert_eq!(matched(&Topics::new(), &Vectors::default(), "retries should wait longer", &twins), None);
}

#[test]
fn a_thought_between_two_topics_is_no_clear_match() {
    assert_eq!(matched(&Topics::new(), &Vectors::default(), "retry on the clock", &three()), None);
}

/// The model tells what a message is about, not whether it asks, so a question about a Session's
/// topic is Helm's to answer and not that Session's to be told.
#[test]
fn a_message_that_ends_in_a_question_mark_is_left_to_claude() {
    assert_eq!(matched(&Topics::new(), &Vectors::default(), "are the retries done?", &three()), None);
}

#[test]
fn a_long_message_is_left_to_claude() {
    let long = format!("retries {}", "and more ".repeat(60));
    assert_eq!(matched(&Topics::new(), &Vectors::default(), &long, &three()), None);
}

#[test]
fn nothing_running_is_nothing_to_match() {
    assert_eq!(matched(&Topics::new(), &Vectors::default(), "retries should wait longer", &[]), None);
}

#[test]
fn the_only_session_is_matched_on_its_score_alone() {
    let only = candidates(&[("01SESSBBBBBBBBBBBBBBBBBBBB", "Pin the clock")]);
    let model = Topics::new();
    assert_eq!(id_of(matched(&model, &Vectors::default(), "the clock should be fixed", &only)).as_deref(), Some("01SESSBBBBBBBBBBBBBBBBBBBB"));
    assert_eq!(matched(&model, &Vectors::default(), "retries should wait longer", &only), None);
}

#[test]
fn a_read_embeds_only_the_sessions_it_has_not_seen() {
    let (model, kept) = (Topics::new(), Vectors::default());
    matched(&model, &kept, "retries should wait longer", &three());
    assert_eq!(model.embedded.lock().unwrap().len(), 1 + 3, "the words and each Session");
    model.embedded.lock().unwrap().clear();

    matched(&model, &kept, "the clock should be fixed", &three());
    assert_eq!(*model.embedded.lock().unwrap(), vec!["the clock should be fixed".to_string()], "only the words");
    model.embedded.lock().unwrap().clear();

    let renamed = candidates(&[
        ("01SESSAAAAAAAAAAAAAAAAAAAA", "Back off the invoice retries"),
        ("01SESSBBBBBBBBBBBBBBBBBBBB", "Pin the time source"),
        ("01SESSCCCCCCCCCCCCCCCCCCCC", "Sessions list page"),
    ]);
    matched(&model, &kept, "the clock should be fixed", &renamed);
    let embedded = model.embedded.lock().unwrap().clone();
    assert_eq!(embedded.len(), 1 + 1, "the words and the Session whose title changed: {embedded:?}");
    assert_eq!(model.calls.load(Ordering::SeqCst), 3, "one call per read");
}

#[test]
fn a_model_that_fails_leaves_the_read_to_claude() {
    struct Broken;
    impl TextModel for Broken {
        fn model(&self) -> &str {
            "broken@0"
        }
        fn embed(&self, _: &[&str]) -> Result<Vec<Vec<f32>>, NotEmbedded> {
            Err(NotEmbedded("no".into()))
        }
        fn read(&self, _: TextReading, _: &[&str]) -> Result<Vec<f32>, NotEmbedded> {
            Err(NotEmbedded("no".into()))
        }
    }
    assert_eq!(matched(&Broken, &Vectors::default(), "retries should wait longer", &three()), None);
}

#[tokio::test]
async fn until_the_model_is_loaded_nothing_is_matched_and_after_it_is() {
    let language_model = LanguageModel::default();
    assert_eq!(matched_by(&language_model, "retries should wait longer", &three()).await, None);
    language_model.loaded(Arc::new(Topics::new()));
    let target = matched_by(&language_model, "retries should wait longer", &three()).await;
    assert_eq!(id_of(target).as_deref(), Some("01SESSAAAAAAAAAAAAAAAAAAAA"));
}

#[test]
fn a_clear_match_is_the_best_score_at_the_floor_and_ahead_by_the_lead() {
    assert_eq!(clear_match(&[]), None);
    assert_eq!(clear_match(&[ROUTES_AT]), Some(0));
    assert_eq!(clear_match(&[ROUTES_AT - 0.01]), None);
    assert_eq!(clear_match(&[0.2, ROUTES_AT + LEADS_BY, ROUTES_AT]), Some(1));
    assert_eq!(clear_match(&[0.2, ROUTES_AT + LEADS_BY - 0.02, ROUTES_AT]), None, "ahead by less than the lead");
}

/// Fetching and loading follow the readers in force: any one that reads with the model wants it.
#[test]
fn the_model_is_wanted_while_any_reader_in_force_reads_with_it() {
    assert!(!wanted_by(&[false, false]));
    assert!(wanted_by(&[false, true]));
    assert!(wanted_by(&[true, false]));
}
