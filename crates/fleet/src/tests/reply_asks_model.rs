//! The language model's reader of whether a reply asks, through a fake model on the same seam, so no
//! weights are fetched. `docs/spikes/reply-asks-classifier.md` has the real model measured.

use std::sync::Arc;
use std::time::{Duration, Instant};

use adapter_traits::{NotEmbedded, TextModel, TextReading};

use crate::session_host::asks::asking_line;
use crate::session_host::asks_model::{decided, read, ModelRead, Reader, Reads};

/// Reads a text that names a walk as an ask and every other as a tell.
struct Fake;

impl TextModel for Fake {
    fn model(&self) -> &str {
        "fake@0"
    }

    fn embed(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, NotEmbedded> {
        Ok(texts.iter().map(|_| vec![0.0; 4]).collect())
    }

    fn read(&self, reading: TextReading, texts: &[&str]) -> Result<Vec<f32>, NotEmbedded> {
        assert_eq!(reading, TextReading::ReplyAsks);
        Ok(texts.iter().map(|text| if text.contains("walk") { 0.99 } else { 0.01 }).collect())
    }
}

/// An ask the rules have no phrase for: what is owed is a walk.
const WALK: &str = "The page is built.\n\nThe walk is ready at the usual address.";
const WALK_LINE: &str = "The walk is ready at the usual address.";

fn read_as(score: f32, line: &str) -> ModelRead {
    ModelRead { score, line: Some(line.to_string()) }
}

#[test]
fn the_model_reads_an_ask_the_rules_miss_and_names_its_sentence() {
    assert_eq!(asking_line(WALK), None);
    let read = read(&Fake, WALK).expect("the fake always reads");
    assert!(read.score > 0.9, "{read:?}");
    assert_eq!(read.line.as_deref(), Some(WALK_LINE));
    assert_eq!(decided(Reader::Either, None, Some(&read)).as_deref(), Some(WALK_LINE));
    assert_eq!(decided(Reader::Model, None, Some(&read)).as_deref(), Some(WALK_LINE));
    assert_eq!(decided(Reader::Rules, None, Some(&read)), None);
}

#[test]
fn where_both_call_it_the_rules_sentence_is_shown() {
    let rules = Some(String::from("Want me to land it?"));
    let read = read_as(0.99, "It is built.");
    assert_eq!(decided(Reader::Either, rules.clone(), Some(&read)), rules);
    assert_eq!(decided(Reader::Model, rules.clone(), Some(&read)), rules);
}

#[test]
fn the_model_alone_drops_what_it_reads_as_a_tell_and_either_keeps_it() {
    let rules = Some(String::from("Want me to land it?"));
    let read = read_as(0.1, "It is built.");
    assert_eq!(decided(Reader::Model, rules.clone(), Some(&read)), None);
    assert_eq!(decided(Reader::Either, rules.clone(), Some(&read)), rules);
    assert_eq!(decided(Reader::Either, None, Some(&read)), None);
}

#[test]
fn until_the_model_has_read_a_reply_the_rules_stand_in() {
    let rules = Some(String::from("Want me to land it?"));
    for reader in [Reader::Rules, Reader::Model, Reader::Either] {
        assert_eq!(decided(reader, rules.clone(), None), rules);
    }
}

/// The first derive answers by the rules and starts the read; the Session is named once the read is
/// in, and the next derive has the model's answer.
#[tokio::test]
async fn a_reply_is_read_once_off_the_runtime_and_its_session_named() {
    let reads = Arc::new(Reads::default());
    let model: Arc<dyn TextModel> = Arc::new(Fake);
    assert_eq!(reads.asking_line(Reader::Either, Some(Arc::clone(&model)), "s1", WALK), None);
    let deadline = Instant::now() + Duration::from_secs(30);
    let settled = loop {
        let settled = reads.settled();
        if !settled.is_empty() || Instant::now() > deadline {
            break settled;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    };
    assert_eq!(settled, vec![String::from("s1")]);
    assert_eq!(reads.asking_line(Reader::Either, Some(model), "s1", WALK).as_deref(), Some(WALK_LINE));
    assert!(reads.settled().is_empty());
}

#[test]
fn with_the_rules_in_force_the_model_is_not_asked() {
    let reads = Arc::new(Reads::default());
    assert_eq!(reads.asking_line(Reader::Rules, Some(Arc::new(Fake)), "s1", WALK), None);
}

/// Fleet fetches and loads the model exactly while a reader in force reads with it.
#[tokio::test]
async fn the_model_is_fetched_and_loaded_only_while_a_reader_in_force_reads_with_it() {
    assert!(crate::session_host::asks_model::SHIPPED.reads_model());
    let home = super::tmp::TempDir::new();
    let fittings = super::daemon::fitted_with(&home, testkit::FakeWorkProduct::changed(&[]), testkit::FakeHarness::that_listens());
    let fleet = Arc::new(crate::daemon::Fleet::assembled(fittings));
    let started = crate::language_model::keep_loading(fleet, home.path().join("models"), |_| panic!("nothing is loaded"), |_| {});
    assert_eq!(started.is_some(), crate::language_model::wanted());
    assert!(!crate::language_model::wanted_by(&[false, false, false]), "with every reader on rules nothing is fetched");
    if let Some(task) = started {
        task.abort();
    }
}
