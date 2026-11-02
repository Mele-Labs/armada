//! A walk or done reminder a job disputed, learnt from: the model checks the sentences that set it off,
//! and where it agrees they asked nothing, they are kept and a near one is not reminded again. No
//! review job is started for a dispute learnt from. `crate::session_host::reply_claims`.
//!
//! The reply is the one the owner reported on 27 Oct 2026: a plan's summary that put his walk after
//! his approval, and was reminded to show it.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use adapter_traits::{NotEmbedded, TextModel, TextReading};
use api::Sessions;
use ipc::{SessionId, SessionRow, SessionTurn};
use testkit::FakeJudge;

use super::session_dispute::{disputing, others, serving_fitted};
use super::session_host::{eventually, Rig};
use crate::session_host::reply_claims::{walk_sentences, CHECKING, REMINDED_WALK, WALK_REMINDER};

const DISPUTED_REPLY: &str = "I've recorded the plan in Bridge with your three answers. Nothing gets built until you approve it.

1. **Mock, then your walk.** All seven changes go on the mock Fleet first, played before you see it:
   - Each repo's head gets its own `+` for a new project, and the dropdown goes.
   - Pressing a repo's head opens that repo's own tasks.
2. **Saving a repo's own tasks.** Once you approve the walk, Fleet keeps one task list per repo. It's made when you add the repo's first task.
3. **Tests and landing.** Tests go in only after you approve the look, then the pull request merges on a green `ci`.

A file tagged in a task stays in the task's text as a path and goes to the Job as written.";
const SET_OFF: &str = "Once you approve the walk, Fleet keeps one task list per repo.";
const WHY: &str = "The reply described a walk as a later step of a plan still waiting for approval; no walk exists yet.";
const ANSWERED: &str = "I disputed Fleet's notice: the walk comes after the plan is approved.";
/// Near [`SET_OFF`] by the fake model's reading: it names a task list.
const NEAR: &str = "Once you approve the walk, Fleet saves the task list for each repo.";
const ASKED: &str = "Open the walk and approve it.";
/// Near [`SET_OFF`] too, and not a sentence the model has been handed before.
const NEARER: &str = "Once you approve the walk, Fleet files the task list under each repo.";

#[test]
fn the_walk_reader_names_the_sentence_of_the_disputed_reply_that_set_it_off() {
    assert_eq!(walk_sentences(DISPUTED_REPLY), [SET_OFF]);
}

/// Puts a text that names a task list on one axis and every other on the other.
struct Fake;

impl TextModel for Fake {
    fn model(&self) -> &str {
        "fake@0"
    }

    fn embed(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, NotEmbedded> {
        Ok(texts.iter().map(|text| if text.contains("task list") { vec![1.0, 0.0] } else { vec![0.0, 1.0] }).collect())
    }

    fn read(&self, _: TextReading, _: &[&str]) -> Result<Vec<f32>, NotEmbedded> {
        Err(NotEmbedded(String::from("not read here")))
    }
}

/// A rig serving Armada, so a dispute not learnt from starts a review, its model answering `checked`.
async fn checked_by(checked: &str) -> Rig {
    let judge = Arc::new(FakeJudge::answering(&[(CHECKING, checked)]));
    serving_fitted("armada", move |fittings| fittings.judge = judge).await
}

/// How many lines the agent was sent that carry `words`.
fn sent_with(rig: &Rig, words: &str) -> usize {
    rig.stand_in.process(0).sent.lock().unwrap().iter().filter(|line| line.contains(words)).count()
}

/// The turn ended and whatever follows its end has had time to happen.
async fn settled(rig: &Rig, id: &SessionId) {
    eventually(|| async { rig.turn(id).await == SessionTurn::Idle }).await;
    tokio::time::sleep(Duration::from_millis(300)).await;
}

/// A job whose reply was reminded to show a walk, and that answered the reminder.
async fn reminded(rig: &Rig) -> SessionId {
    let id = rig.start().await;
    rig.send(&id, "plan the projects tab").await;
    rig.stand_in.init(0);
    eventually(|| async { rig.fleet.hosts().of(id.as_str()).state().turn_began.is_some() }).await;
    rig.stand_in.finishes(0, DISPUTED_REPLY);
    eventually(|| async { sent_with(rig, WALK_REMINDER) == 1 }).await;
    eventually(|| async {
        let tell = rig.fleet.store().lock().await.tell_of(id.as_str()).unwrap();
        tell.is_some_and(|tell| tell.kind == REMINDED_WALK)
    })
    .await;
    tokio::time::sleep(Duration::from_millis(20)).await;
    rig.stand_in.init(0);
    rig.stand_in.finishes(0, ANSWERED);
    settled(rig, &id).await;
    id
}

/// The agent's next turn, ending on `reply`.
async fn replies(rig: &Rig, id: &SessionId, reply: &str) {
    rig.send(id, "carry on").await;
    rig.stand_in.finishes(0, reply);
    settled(rig, id).await;
}

#[tokio::test]
async fn a_disputed_walk_reminder_the_model_agrees_with_keeps_the_sentence_and_starts_no_review() {
    let rig = checked_by("misread: yes").await;
    let id = reminded(&rig).await;

    let disputed = Arc::clone(&rig.fleet).dispute_fleet(None, disputing(&id, WHY)).await.expect("disputed");

    assert_eq!(rig.fleet.store().lock().await.claim_misreads(REMINDED_WALK).unwrap(), [SET_OFF]);
    assert_eq!(disputed.review_session_id, None);
    assert!(others(&rig, &id).await.is_empty(), "no review job");
    let row = rig.rows(&id).await.into_iter().find(|row| matches!(row, SessionRow::Disputed { .. }));
    assert!(matches!(row, Some(SessionRow::Disputed { review: None, .. })), "{row:?}");
}

#[tokio::test]
async fn a_disputed_walk_reminder_the_model_disagrees_with_keeps_nothing_and_starts_a_review() {
    let rig = checked_by("misread: no").await;
    let id = reminded(&rig).await;

    let disputed = Arc::clone(&rig.fleet).dispute_fleet(None, disputing(&id, WHY)).await.expect("disputed");

    assert!(rig.fleet.store().lock().await.claim_misreads(REMINDED_WALK).unwrap().is_empty());
    assert!(disputed.review_session_id.is_some(), "a review job");
}

#[tokio::test]
async fn once_learnt_the_same_reply_is_not_reminded_again() {
    let rig = checked_by("misread: yes").await;
    let id = reminded(&rig).await;
    Arc::clone(&rig.fleet).dispute_fleet(None, disputing(&id, WHY)).await.expect("disputed");

    replies(&rig, &id, DISPUTED_REPLY).await;

    assert_eq!(sent_with(&rig, WALK_REMINDER), 1, "the same sentence is held off without the model");
}

#[tokio::test]
async fn once_learnt_a_near_sentence_is_not_reminded_and_a_real_ask_still_is() {
    let rig = checked_by("misread: yes").await;
    rig.fleet.language_model().loaded(Arc::new(Fake));
    let id = reminded(&rig).await;
    Arc::clone(&rig.fleet).dispute_fleet(None, disputing(&id, WHY)).await.expect("disputed");

    replies(&rig, &id, NEAR).await;
    assert_eq!(sent_with(&rig, WALK_REMINDER), 1, "a near sentence is held off");

    replies(&rig, &id, ASKED).await;
    assert_eq!(sent_with(&rig, WALK_REMINDER), 2, "a walk asked for is still reminded");
}

/// [`Fake`]'s reading, with every batch it was handed kept and the embed failing on demand.
#[derive(Default)]
struct Counting {
    batches: Mutex<Vec<Vec<String>>>,
    failing: AtomicBool,
}

impl TextModel for Counting {
    fn model(&self) -> &str {
        "fake@0"
    }

    fn embed(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, NotEmbedded> {
        self.batches.lock().unwrap().push(texts.iter().map(ToString::to_string).collect());
        if self.failing.load(Ordering::SeqCst) {
            return Err(NotEmbedded(String::from("failed on purpose")));
        }
        Fake.embed(texts)
    }

    fn read(&self, reading: TextReading, texts: &[&str]) -> Result<Vec<f32>, NotEmbedded> {
        Fake.read(reading, texts)
    }
}

impl Counting {
    fn handed(&self) -> Vec<Vec<String>> {
        self.batches.lock().unwrap().clone()
    }
}

/// A rig whose walk reminder was disputed and learnt from, the model counting what it is handed from
/// here on: the reminder's own reading is not what these tests count.
async fn learnt() -> (Rig, SessionId, Arc<Counting>) {
    let rig = checked_by("misread: yes").await;
    let model = Arc::new(Counting::default());
    rig.fleet.language_model().loaded(Arc::clone(&model) as Arc<dyn TextModel>);
    let id = reminded(&rig).await;
    Arc::clone(&rig.fleet).dispute_fleet(None, disputing(&id, WHY)).await.expect("disputed");
    model.batches.lock().unwrap().clear();
    (rig, id, model)
}

fn batch(texts: &[&str]) -> Vec<String> {
    texts.iter().map(ToString::to_string).collect()
}

/// The turn's end used to embed the kept sentence again on every reply (31 of them in the report).
/// Now the kept misread is embedded once and a later reply hands the model only its own new sentence.
#[tokio::test]
async fn a_later_turns_end_embeds_only_the_sentences_not_seen_before() {
    let (rig, id, model) = learnt().await;

    replies(&rig, &id, NEAR).await;
    replies(&rig, &id, NEARER).await;
    replies(&rig, &id, ASKED).await;

    assert_eq!(model.handed(), [batch(&[NEAR, SET_OFF]), batch(&[NEARER]), batch(&[ASKED])]);
    assert_eq!(sent_with(&rig, WALK_REMINDER), 2, "the near sentences are held off and the real ask is reminded, as before");
}

/// The same reply twice is read from what is kept: the model is not asked again.
#[tokio::test]
async fn a_reply_said_twice_is_embedded_once() {
    let (rig, id, model) = learnt().await;

    replies(&rig, &id, NEAR).await;
    replies(&rig, &id, NEAR).await;

    assert_eq!(model.handed(), [batch(&[NEAR, SET_OFF])]);
    assert_eq!(sent_with(&rig, WALK_REMINDER), 1, "held off both times");
}

/// A batch that fails leaves the sentences as they were, so the walk is reminded, and keeps nothing:
/// the next turn's end embeds the whole batch again.
#[tokio::test]
async fn a_failed_embed_leaves_the_sentences_alone_and_keeps_nothing() {
    let (rig, id, model) = learnt().await;
    model.failing.store(true, Ordering::SeqCst);

    replies(&rig, &id, NEAR).await;
    assert_eq!(sent_with(&rig, WALK_REMINDER), 2, "no vectors, so the near sentence is not held off");

    // The next turn answers the reminder, which is not read for a claim.
    replies(&rig, &id, ANSWERED).await;
    model.failing.store(false, Ordering::SeqCst);
    replies(&rig, &id, NEAR).await;

    assert_eq!(model.handed(), [batch(&[NEAR, SET_OFF]), batch(&[NEAR, SET_OFF])]);
    assert_eq!(sent_with(&rig, WALK_REMINDER), 2, "read now, it is held off");
}
