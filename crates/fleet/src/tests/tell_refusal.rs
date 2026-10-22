//! The reader of whether a reply turns a Fleet tell down, through a fake model on the same seam, so no
//! weights are fetched. `docs/spikes/tell-refusal-reader.md` has the real head measured. The replies are
//! paraphrases, not any Job's words.

use std::sync::Arc;
use std::time::{Duration, Instant};

use adapter_traits::{NotEmbedded, TextModel, TextReading};
use ipc::WaitingSource;

use crate::session_host::asks_model::{ModelRead, Reads};
use crate::session_host::standing::{derived, standing, Seen, Told, STUCK_AFTER};
use crate::session_host::tell_refusal::{read, refusal, IN_FORCE, REFUSED_AT};

/// Reads a text that says it will not as a refusal and every other as taking the tell up.
struct Fake;

impl TextModel for Fake {
    fn model(&self) -> &str {
        "fake@0"
    }

    fn embed(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, NotEmbedded> {
        Ok(texts.iter().map(|_| vec![0.0; 4]).collect())
    }

    fn read(&self, reading: TextReading, texts: &[&str]) -> Result<Vec<f32>, NotEmbedded> {
        assert_eq!(reading, TextReading::TellRefused);
        Ok(texts.iter().map(|text| if text.contains("will not") { 0.99 } else { 0.02 }).collect())
    }
}

const TELL_AT: &str = "2026-10-20T09:00:00.000Z";
const ENDED: &str = "2026-10-20T09:01:00.000Z";
const LATER: &str = "2026-10-20T09:02:00.000Z";
const REFUSES: &str = "The branch is half merged.\n\nI will not release the slot until the other job lands.";
const REFUSAL: &str = "I will not release the slot until the other job lands.";
const TAKES: &str = "The branch is pushed.\n\nThe slot is released.";

fn refuses(said: &str) -> Option<String> {
    refusal(&read(&Fake, said).expect("the fake always reads"), said)
}

fn none(_: &str) -> Option<String> {
    None
}

fn after_the_tell<'a>(said: &'a str, refused: &'a dyn Fn(&str) -> Option<String>) -> Seen<'a> {
    Seen {
        helm: false,
        running: None,
        subagents: false,
        held: false,
        called: false,
        turn_end: Some((ENDED, said)),
        spoke: None,
        done: None,
        dismissed: &[],
        asks: &|_| None,
        extracted: None,
        tell: Some(Told { at: TELL_AT, refused }),
    }
}

fn items(seen: &Seen) -> Vec<(String, WaitingSource, String)> {
    derived(seen).into_iter().map(|one| (one.id, one.source, one.text)).collect()
}

#[test]
fn the_model_names_the_sentence_that_says_no() {
    let scored = read(&Fake, REFUSES).expect("the fake always reads");
    assert!(scored.score >= REFUSED_AT, "{scored:?}");
    assert_eq!(scored.line.as_deref(), Some(REFUSAL));
    assert_eq!(refuses(REFUSES).as_deref(), Some(REFUSAL));
    assert_eq!(refuses(TAKES), None);
}

#[test]
fn a_score_under_the_threshold_is_no_refusal_and_one_with_no_sentence_shows_the_reply() {
    let weak = ModelRead { score: REFUSED_AT - 0.01, line: Some(REFUSAL.to_string()) };
    assert_eq!(refusal(&weak, REFUSES), None);
    let bare = ModelRead { score: 0.99, line: None };
    assert_eq!(refusal(&bare, "No."), Some(String::from("No.")));
}

/// A refusal is one item for the tell it follows, drawn as a reply that asks is, and no unread mark.
#[test]
fn a_reply_that_turns_the_tell_down_is_an_item_with_the_sentence_that_says_no() {
    let seen = after_the_tell(REFUSES, &refuses);
    assert_eq!(items(&seen), [(format!("refused:{TELL_AT}"), WaitingSource::Replied, REFUSAL.to_string())]);
    assert_eq!(derived(&seen)[0].since.as_str(), ENDED);
    assert_eq!(standing(&seen, STUCK_AFTER), ipc::SessionStanding::Idle);
}

#[test]
fn a_reply_that_takes_the_tell_up_or_a_model_not_loaded_raises_nothing() {
    for refused in [&refuses as &dyn Fn(&str) -> Option<String>, &none] {
        let seen = after_the_tell(TAKES, refused);
        assert!(derived(&seen).is_empty(), "{:?}", items(&seen));
        assert_eq!(standing(&seen, STUCK_AFTER), ipc::SessionStanding::Replied);
    }
    let unloaded = after_the_tell(REFUSES, &none);
    assert!(derived(&unloaded).is_empty());
    let untold = Seen { tell: None, ..after_the_tell(REFUSES, &refuses) };
    assert!(derived(&untold).is_empty());
}

/// Where the reply already reached him, a second line for it would be a second call for one answer.
#[test]
fn a_refusal_raises_nothing_where_he_was_already_asked_or_the_turn_is_not_over() {
    for seen in [
        Seen { held: true, ..after_the_tell(REFUSES, &refuses) },
        Seen { called: true, ..after_the_tell(REFUSES, &refuses) },
        Seen { subagents: true, ..after_the_tell(REFUSES, &refuses) },
        Seen { helm: true, ..after_the_tell(REFUSES, &refuses) },
        Seen { spoke: Some(LATER), ..after_the_tell(REFUSES, &refuses) },
        Seen { done: Some(LATER), ..after_the_tell(REFUSES, &refuses) },
    ] {
        assert!(derived(&seen).iter().all(|one| !one.id.starts_with("refused:")), "{:?}", items(&seen));
    }
    // A reply that asks is the ask's item, not a second one for the refusal.
    let asks = Seen { asks: &|_| Some(String::from("Want me to release it?")), ..after_the_tell(REFUSES, &refuses) };
    assert_eq!(items(&asks).len(), 1);
    assert_eq!(items(&asks)[0].0, format!("replied:{ENDED}"));
}

/// Dismissing it settles that tell for good, and tells the agent nothing: the id is only kept.
#[test]
fn a_dismissed_refusal_stays_gone_and_does_not_read_as_unread() {
    let gone = vec![format!("refused:{TELL_AT}")];
    let seen = Seen { dismissed: &gone, ..after_the_tell(REFUSES, &refuses) };
    assert!(derived(&seen).is_empty());
    assert_eq!(standing(&seen, STUCK_AFTER), ipc::SessionStanding::Idle);
}

/// The first derive has no read and raises nothing; the Session is named once the read is in, and the next
/// derive has it. The refusal reads are apart from the asks reader's, which holds the same text.
#[tokio::test]
async fn a_reply_is_read_once_off_the_runtime_and_its_session_named() {
    let reads = Arc::new(Reads::default());
    let model: Arc<dyn TextModel> = Arc::new(Fake);
    assert_eq!(reads.read_of(read, Arc::clone(&model), "s1", REFUSES), None);
    let deadline = Instant::now() + Duration::from_secs(30);
    let settled = loop {
        let settled = reads.settled();
        if !settled.is_empty() || Instant::now() > deadline {
            break settled;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    };
    assert_eq!(settled, vec![String::from("s1")]);
    let scored = reads.read_of(read, model, "s1", REFUSES).expect("read");
    assert_eq!(refusal(&scored, REFUSES).as_deref(), Some(REFUSAL));
    assert!(reads.settled().is_empty());
}

/// Out of fold, on the tells in scope, the head caught no refusal. Putting it in force is a line in
/// `tell_refusal`, made once the spike shows precision of 0.75 with one caught.
#[test]
fn the_reader_is_built_and_not_in_force() {
    assert!(!IN_FORCE);
}
