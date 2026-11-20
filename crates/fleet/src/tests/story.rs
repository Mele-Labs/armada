//! His words sorted by a small model, the asks that open from them, how an ask moves, and a Job's story.
//!
//! **The model is the fake and the process is the stand-in**: what runs is which messages Fleet sorts, what it asks, and
//! what it keeps of the answer.

use std::sync::Arc;

use api::{HostedSessions, Story};
use ipc::{AskKind, AskState, MessageKind, SessionId, SetMessageKind, SortedBy};
use store::{AttachmentState, Holder, KeptAttachment};
use testkit::FakeJudge;

use super::session_dispute::serving_fitted;
use super::session_host::Rig;
use crate::story::sorting::{self, OpenAsk, Turn};

const BUG: &str = "kind: new_ask\nask_kind: bug\nask: none\ntitle: Votes show as never cast";

fn open(id: &str, title: &str) -> OpenAsk {
    OpenAsk { id: id.to_string(), kind: AskKind::Bug, title: title.to_string() }
}

#[test]
fn the_question_carries_the_open_asks_the_turns_and_the_corrections_he_made() {
    let corrections = vec![store::KeptCorrection { words: "make the badge blue".into(), kind: "new_ask".into(), ask_kind: Some("change_request".into()) }];
    let turns = vec![Turn { person: false, text: "Which badge?".into() }, Turn { person: true, text: "The red one".into() }];
    let asked = sorting::question("and keep it round", &[open("a1", "Badge is wrong")], &turns, &corrections);
    assert!(asked.contains("- 1: [bug] Badge is wrong"), "{asked}");
    assert!(asked.contains("agent: Which badge?\nperson: The red one"), "{asked}");
    assert!(asked.contains("Message: make the badge blue\nRight sorting: kind: new_ask, ask_kind: change_request"), "{asked}");
    assert!(asked.contains("The message:\nand keep it round"), "{asked}");
    let none = sorting::question("hi", &[], &[], &[]);
    assert!(none.contains("Open asks of this job:\nnone") && !none.contains("sorted himself"), "{none}");
}

#[test]
fn a_new_ask_is_read_with_its_kind_and_title_and_a_missing_one_is_filled() {
    let read = sorting::read_of(BUG, "votes show as never cast", &[]).expect("read");
    assert_eq!((read.kind, read.ask_kind, read.title.as_deref()), (MessageKind::NewAsk, Some(AskKind::Bug), Some("Votes show as never cast")));
    let bare = sorting::read_of("kind: new_ask\nask_kind: none\ntitle: none", "Make the badge blue\nplease", &[]).expect("read");
    assert_eq!((bare.ask_kind, bare.title.as_deref()), (Some(AskKind::ChangeRequest), Some("Make the badge blue")));
}

#[test]
fn a_revision_joins_an_open_ask_and_one_that_names_none_is_talk() {
    let asks = [open("a1", "Badge is wrong")];
    let joined = sorting::read_of("kind: revision\nask_kind: none\nask: 1\ntitle: none", "and round", &asks).expect("read");
    assert_eq!((joined.kind, joined.ask.as_deref()), (MessageKind::Revision, Some("a1")));
    let unknown = sorting::read_of("kind: answer\nask: 7", "yes", &asks).expect("read");
    assert_eq!((unknown.kind, unknown.ask), (MessageKind::Talk, None));
}

#[test]
fn an_answer_with_no_kind_is_not_sorted() {
    assert_eq!(sorting::read_of("I think this is a bug.", "x", &[]), None);
    assert_eq!(sorting::read_of("kind: maybe", "x", &[]), None);
}

async fn told_once(rig: &Rig, id: &SessionId, text: &str) {
    Arc::clone(&rig.fleet)
        .send_session_message_aiming(ipc::SendSessionMessage { session_id: id.clone(), text: text.into(), attachments: Vec::new(), mentions: Vec::new(), sketches: Vec::new() }, None)
        .await
        .expect("taken");
}

async fn sorted(rig: &Rig) {
    for task in rig.fleet.messages_taken_to_sort().await {
        task.await.expect("the read ends");
    }
}

async fn rig_saying(answer: &str) -> (Rig, Arc<FakeJudge>) {
    let judge = Arc::new(FakeJudge::saying(answer));
    let planted = Arc::clone(&judge);
    let rig = serving_fitted("armada", move |fittings| fittings.judge = planted).await;
    (rig, judge)
}

fn the_message_of(rows: &[ipc::SessionRow]) -> String {
    rows.iter()
        .find_map(|row| match row {
            ipc::SessionRow::Message { from: ipc::SessionVoice::You, id, .. } => Some(id.clone()),
            _ => None,
        })
        .expect("a message of his")
}

#[tokio::test]
async fn a_message_he_sends_is_sorted_after_delivery_and_opens_an_ask_in_its_job() {
    let (rig, _) = rig_saying(BUG).await;
    let id = rig.start_titled(Some("Fix votes")).await;
    told_once(&rig, &id, "the votes show as never cast").await;
    let row = the_message_of(&rig.rows(&id).await);

    let before = rig.fleet.list_message_kinds(id.clone()).await.expect("listed");
    assert!(before.messages.is_empty(), "nothing is sorted before the loop reads it: {before:?}");
    sorted(&rig).await;

    let kinds = rig.fleet.list_message_kinds(id.clone()).await.expect("listed");
    assert_eq!(kinds.messages.len(), 1, "{kinds:?}");
    let one = &kinds.messages[0];
    assert_eq!((one.row_id.as_str(), one.kind, one.ask_kind, one.sorted_by), (row.as_str(), MessageKind::NewAsk, Some(AskKind::Bug), SortedBy::Fleet));
    let asks = rig.fleet.list_job_asks(id.clone()).await.expect("listed").asks;
    assert_eq!(asks.len(), 1);
    assert_eq!(asks[0].title, "Votes show as never cast");
    assert_eq!(asks[0].first_words, "the votes show as never cast");
    assert_eq!((asks[0].state, asks[0].building.as_ref(), &asks[0].said_in), (AskState::Building, Some(&id), &id));

    sorted(&rig).await;
    assert_eq!(rig.fleet.list_job_asks(id.clone()).await.expect("listed").asks.len(), 1, "a message is read once");
}

#[tokio::test]
async fn a_read_that_fails_leaves_the_message_unsorted_and_the_message_delivered() {
    let judge = Arc::new(FakeJudge::that_fails("the sorting model"));
    let planted = Arc::clone(&judge);
    let rig = serving_fitted("armada", move |fittings| fittings.judge = planted).await;
    let id = rig.start_titled(Some("Fix votes")).await;
    told_once(&rig, &id, "the votes show as never cast").await;
    sorted(&rig).await;

    assert!(rig.fleet.list_message_kinds(id.clone()).await.expect("listed").messages.is_empty());
    assert!(rig.fleet.list_job_asks(id.clone()).await.expect("listed").asks.is_empty());
    let _ = the_message_of(&rig.rows(&id).await);
}

#[tokio::test]
async fn his_correction_is_kept_as_his_and_rides_in_the_next_prompt_as_an_example() {
    let (rig, judge) = rig_saying("kind: talk").await;
    let id = rig.start_titled(Some("Fix votes")).await;
    told_once(&rig, &id, "make the badge blue").await;
    let row = the_message_of(&rig.rows(&id).await);
    sorted(&rig).await;

    let set = SetMessageKind { session_id: id.clone(), row_id: row.clone(), kind: MessageKind::NewAsk, ask_kind: Some(AskKind::Idea), ask_id: None };
    let kept = Arc::clone(&rig.fleet).set_message_kind(set).await.expect("set");
    assert_eq!((kept.kind, kept.ask_kind, kept.sorted_by), (MessageKind::NewAsk, Some(AskKind::Idea), SortedBy::Owner));
    assert_eq!(rig.fleet.list_job_asks(id.clone()).await.expect("listed").asks.len(), 1);

    told_once(&rig, &id, "and the tab too").await;
    sorted(&rig).await;
    let asked: Vec<String> = judge.asked().into_iter().filter(|asked| asked.contains("The message:\nand the tab too")).collect();
    assert_eq!(asked.len(), 1, "{asked:?}");
    assert!(asked[0].contains("Message: make the badge blue\nRight sorting: kind: new_ask, ask_kind: idea"), "{}", asked[0]);

    // Back to talk: the ask he opened goes, and the message stays his.
    let set = SetMessageKind { session_id: id.clone(), row_id: row, kind: MessageKind::Talk, ask_kind: None, ask_id: None };
    let kept = Arc::clone(&rig.fleet).set_message_kind(set).await.expect("set");
    assert_eq!((kept.kind, kept.sorted_by), (MessageKind::Talk, SortedBy::Owner));
    assert!(rig.fleet.list_job_asks(id.clone()).await.expect("listed").asks.is_empty());
}

#[tokio::test]
async fn a_revision_he_sets_joins_the_ask_it_names_and_one_naming_none_is_refused() {
    let (rig, _) = rig_saying(BUG).await;
    let id = rig.start_titled(Some("Fix votes")).await;
    told_once(&rig, &id, "the votes show as never cast").await;
    sorted(&rig).await;
    let ask = rig.fleet.list_job_asks(id.clone()).await.expect("listed").asks[0].ask_id.clone();
    told_once(&rig, &id, "only on pages").await;
    let rows = rig.rows(&id).await;
    let second = rows
        .iter()
        .filter_map(|row| match row {
            ipc::SessionRow::Message { from: ipc::SessionVoice::You, id, text, .. } if text == "only on pages" => Some(id.clone()),
            _ => None,
        })
        .next()
        .expect("the second message");

    let bare = SetMessageKind { session_id: id.clone(), row_id: second.clone(), kind: MessageKind::Revision, ask_kind: None, ask_id: None };
    assert!(matches!(Arc::clone(&rig.fleet).set_message_kind(bare).await, Err(api::Refusal::Unacceptable(_))));
    let wrong = SetMessageKind { session_id: id.clone(), row_id: second.clone(), kind: MessageKind::Revision, ask_kind: None, ask_id: Some("nope".into()) };
    assert!(matches!(Arc::clone(&rig.fleet).set_message_kind(wrong).await, Err(api::Refusal::NotFound(_))));
    let joined = SetMessageKind { session_id: id.clone(), row_id: second, kind: MessageKind::Revision, ask_kind: None, ask_id: Some(ask.clone()) };
    let kept = Arc::clone(&rig.fleet).set_message_kind(joined).await.expect("set");
    assert_eq!((kept.kind, kept.ask_id.as_deref()), (MessageKind::Revision, Some(ask.as_str())));

    let asks = rig.fleet.list_job_asks(id.clone()).await.expect("listed").asks;
    assert_eq!(asks[0].said.len(), 1);
    assert_eq!((asks[0].said[0].words.as_str(), asks[0].said[0].kind), ("only on pages", MessageKind::Revision));
    // The ask holds a revision now, so the message that opened it cannot become talk.
    let first = asks[0].first_row.clone();
    let talk = SetMessageKind { session_id: id.clone(), row_id: first, kind: MessageKind::Talk, ask_kind: None, ask_id: None };
    assert!(matches!(Arc::clone(&rig.fleet).set_message_kind(talk).await, Err(api::Refusal::IllegalMove(_))));
}

async fn opened(rig: &Rig) -> (SessionId, String) {
    let id = rig.start_titled(Some("Fix votes")).await;
    told_once(rig, &id, "the votes show as never cast").await;
    sorted(rig).await;
    let ask = rig.fleet.list_job_asks(id.clone()).await.expect("listed").asks[0].ask_id.clone();
    (id, ask)
}

async fn pull(rig: &Rig, id: &SessionId, number: &str, merged: bool) {
    let mut detail = std::collections::BTreeMap::new();
    detail.insert("state".to_string(), if merged { "merged" } else { "open" }.to_string());
    let now = rig.fleet.now().as_str().to_string();
    let attachment = KeptAttachment {
        holder: Holder::session(id.as_str()),
        kind: "pr".into(),
        manifest_id: "armada".into(),
        target: number.into(),
        state: if merged { AttachmentState::Spent } else { AttachmentState::Standing },
        detail,
        since: now.clone(),
        changed_at: "9999-01-01T00:00:00.000Z".into(),
    };
    rig.fleet.store().lock().await.attach(&attachment, false).expect("attached");
}

#[tokio::test]
async fn the_ask_is_built_when_the_pull_request_of_its_job_merges_and_not_before() {
    let (rig, _) = rig_saying(BUG).await;
    let (id, ask) = opened(&rig).await;

    pull(&rig, &id, "3720", false).await;
    rig.fleet.asks_built_checked().await;
    assert_eq!(rig.fleet.list_job_asks(id.clone()).await.expect("listed").asks[0].state, AskState::Building);

    pull(&rig, &id, "3720", true).await;
    rig.fleet.asks_built_checked().await;
    let asks = rig.fleet.list_job_asks(id.clone()).await.expect("listed").asks;
    assert_eq!((asks[0].ask_id.as_str(), asks[0].state), (ask.as_str(), AskState::Built));
    let story = rig.fleet.get_job_story(id.clone()).await.expect("story");
    assert_eq!(story.asks[0].built.iter().map(|one| one.pull_request.as_str()).collect::<Vec<_>>(), ["3720"]);
    assert!(!story.asks[0].open);
    assert_eq!(story.lead, ["Votes show as never cast: built, pull request 3720"]);
}

#[tokio::test]
async fn an_ask_the_message_to_another_job_carries_is_handed_to_it() {
    let judge = Arc::new(FakeJudge::answering(&[("A person sent one message", BUG), ("Which of those asks does the message hand", "handed: 1")]));
    let planted = Arc::clone(&judge);
    let rig = serving_fitted("armada", move |fittings| fittings.judge = planted).await;
    let (id, _) = opened(&rig).await;
    let other = rig.start_titled(Some("Votes on the phone")).await;

    let words = "Take the never-cast votes bug from the page.";
    rig.fleet.asks_handed(id.as_str(), other.as_str(), words).await;

    let asks = rig.fleet.list_job_asks(id.clone()).await.expect("listed").asks;
    assert_eq!((asks[0].state, asks[0].building.as_ref()), (AskState::Handed, Some(&other)));
    assert_eq!(rig.fleet.list_job_asks(other.clone()).await.expect("listed").asks.len(), 1, "the other Job holds it now");
    let story = rig.fleet.get_job_story(id.clone()).await.expect("story");
    assert_eq!(story.asks[0].handed.len(), 1);
    assert_eq!(story.lead, ["Votes show as never cast: handed to Votes on the phone"]);
    let asked: Vec<String> = judge.asked().into_iter().filter(|asked| asked.contains("Which of those asks")).collect();
    assert!(asked.len() == 1 && asked[0].contains(words) && asked[0].contains("- 1: [bug] Votes show as never cast"), "{asked:?}");

    // The ask is the other Job's now, so the same words hand nothing a second time.
    rig.fleet.asks_handed(id.as_str(), other.as_str(), words).await;
    assert_eq!(judge.asked().into_iter().filter(|asked| asked.contains("Which of those asks")).count(), 1);
}

#[tokio::test]
async fn he_moves_an_ask_to_the_backlog_or_drops_it_and_files_it_under_features() {
    let (rig, _) = rig_saying(BUG).await;
    let (id, ask) = opened(&rig).await;
    let change = ipc::ChangeAsk { ask_id: ask.clone(), state: Some(AskState::Built), kind: None, title: None, features: None };
    assert!(matches!(rig.fleet.change_ask(change).await, Err(api::Refusal::Unacceptable(_))));
    let blank = ipc::ChangeAsk { ask_id: ask.clone(), state: None, kind: None, title: Some("  ".into()), features: None };
    assert!(matches!(rig.fleet.change_ask(blank).await, Err(api::Refusal::Unacceptable(_))));

    let change = ipc::ChangeAsk {
        ask_id: ask.clone(),
        state: Some(AskState::Backlog),
        kind: Some(AskKind::Idea),
        title: Some("Votes read as not cast".into()),
        features: Some(vec!["voting".into(), "pages".into()]),
    };
    let moved = rig.fleet.change_ask(change).await.expect("moved");
    assert_eq!((moved.state, moved.kind, moved.title.as_str(), moved.building), (AskState::Backlog, AskKind::Idea, "Votes read as not cast", None));
    assert_eq!(moved.features, ["pages", "voting"]);

    let by_feature = rig.fleet.list_feature_asks("armada".into(), Some("voting".into())).await.expect("listed");
    assert_eq!(by_feature.asks.len(), 1);
    assert!(rig.fleet.list_feature_asks("armada".into(), Some("other".into())).await.expect("listed").asks.is_empty());
    assert_eq!(rig.fleet.list_feature_asks("armada".into(), None).await.expect("listed").asks.len(), 1);
    let _ = id;
}

#[tokio::test]
async fn a_job_and_an_ask_that_do_not_exist_are_refused_with_a_404() {
    let (rig, _) = rig_saying(BUG).await;
    let nobody = SessionId::carried("nobody");
    assert!(matches!(rig.fleet.list_message_kinds(nobody.clone()).await, Err(api::Refusal::NotFound(_))));
    assert!(matches!(rig.fleet.list_job_asks(nobody.clone()).await, Err(api::Refusal::NotFound(_))));
    assert!(matches!(rig.fleet.get_job_story(nobody).await, Err(api::Refusal::NotFound(_))));
    let change = ipc::ChangeAsk { ask_id: "none".into(), state: None, kind: None, title: None, features: None };
    assert!(matches!(rig.fleet.change_ask(change).await, Err(api::Refusal::NotFound(_))));
}

mod telling {
    use ipc::{Ask, AskKind, AskState, AskWords, ArtifactForm, ArtifactRevision, ArtifactBy, ArtifactState, Instant, JobArtifact, MessageKind, SessionId};

    use crate::story::telling::{story_of, Entry};

    fn at(minute: u32) -> Instant {
        Instant::carried(format!("2026-11-16T10:{minute:02}:00.000Z"))
    }

    fn ask(id: &str, state: AskState, opened: u32, revised: Option<u32>) -> Entry {
        let job = SessionId::carried("s1");
        Entry {
            ask: Ask {
                ask_id: id.into(),
                kind: AskKind::Bug,
                title: format!("Ask {id}"),
                first_words: "first".into(),
                first_row: format!("{id}-row"),
                said_in: job.clone(),
                building: Some(job.clone()),
                features: Vec::new(),
                state,
                state_at: at(opened),
                opened_at: at(opened),
                said: revised
                    .map(|minute| vec![AskWords { session_id: job, row_id: format!("{id}-rev"), kind: MessageKind::Revision, words: "more".into(), at: at(minute) }])
                    .unwrap_or_default(),
            },
            built: Vec::new(),
            handed: Vec::new(),
        }
    }

    fn artifact(id: &str, minute: u32, state: ArtifactState) -> JobArtifact {
        JobArtifact {
            id: id.into(),
            title: id.into(),
            form: ArtifactForm::Sketch,
            revisions: vec![ArtifactRevision { number: 1, at: at(minute), by: ArtifactBy::Agent, form: ArtifactForm::Sketch }],
            state,
            words: None,
            item: None,
            in_plan: None,
            archived: None,
            for_line: None,
            presses: Vec::new(),
            votes: Vec::new(),
            seen: None,
            parts: Vec::new(),
        }
    }

    #[test]
    fn open_asks_come_first_and_each_artifact_goes_under_the_ask_he_last_spoke_on() {
        let entries = vec![ask("built", AskState::Built, 1, None), ask("open", AskState::Building, 10, Some(30))];
        let artifacts = [artifact("before", 0, ArtifactState::Approved), artifact("one", 5, ArtifactState::Approved), artifact("two", 20, ArtifactState::SentBack), artifact("three", 40, ArtifactState::Asking)];
        let story = story_of(&SessionId::carried("s1"), entries, &artifacts);

        let order: Vec<&str> = story.asks.iter().map(|one| one.ask.ask_id.as_str()).collect();
        assert_eq!(order, ["open", "built"]);
        let under = |at: usize| story.asks[at].shown.iter().map(|one| one.artifact.as_str()).collect::<Vec<_>>();
        assert_eq!(under(0), ["two", "three"], "after his words at 10 and 30");
        assert_eq!(under(1), ["one"], "after his first words at 1, before his next at 10");
        assert_eq!(story.shown_otherwise.iter().map(|one| one.artifact.as_str()).collect::<Vec<_>>(), ["before"]);
        assert_eq!((story.asks[0].open, story.asks[1].open), (true, false));
        assert_eq!(story.lead, ["Ask open: being built", "Ask built: built"]);
    }

    #[test]
    fn a_job_with_no_asks_has_an_empty_story_and_its_artifacts_stand_apart() {
        let story = story_of(&SessionId::carried("s1"), Vec::new(), &[artifact("a", 3, ArtifactState::Reading)]);
        assert!(story.asks.is_empty() && story.lead.is_empty());
        assert_eq!(story.shown_otherwise.len(), 1);
    }
}

#[test]
fn a_question_is_read_from_its_kind_line_and_joins_no_ask() {
    let read = sorting::read_of("kind: question\nask_kind: none\nask: 1\ntitle: none", "why is the badge red?", &[open("a1", "Badge is wrong")]).expect("read");
    assert_eq!((read.kind, read.ask, read.ask_kind, read.title), (MessageKind::Question, None, None, None));
    assert!(sorting::question("why?", &[], &[], &[]).contains("new_ask, revision, answer, question or talk"));
}

mod answered {
    use std::sync::Arc;

    use api::{HostedSessions, Story};
    use ipc::{MarkAnswerRead, MessageKind, SessionId, SessionRow, SessionVoice, SetMessageKind};
    use testkit::FakeJudge;

    use super::super::session_dispute::serving_fitted;
    use super::super::session_host::{eventually, Rig};
    use super::{rig_saying, sorted, told_once};
    use crate::story::questions;

    const WHY: &str = "why is the badge red?";
    const REPLY: &str = "It is red because the vote failed.\n\n```rust\nlet red = true;\n```\n\nI left it as is.";

    #[test]
    fn the_read_says_yes_or_no_and_anything_else_is_no_read() {
        assert_eq!(questions::read_of("answers: yes"), Some(true));
        assert_eq!(questions::read_of("- Answers: No."), Some(false));
        assert_eq!(questions::read_of("It does, I think."), None);
        assert_eq!(questions::read_of("answers: maybe"), None);
    }

    #[test]
    fn the_excerpt_is_prose_and_a_long_reply_keeps_its_close() {
        assert_eq!(questions::excerpt_of(REPLY), "It is red because the vote failed. I left it as is.");
        let long = format!("{}THE CLOSE", "a".repeat(9000));
        let cut = questions::reply_cut(&long);
        assert!(cut.chars().count() <= questions::REPLY_CHARS + 5 && cut.ends_with("THE CLOSE"), "{}", cut.len());
        assert!(questions::asked(WHY, &long).contains("THE CLOSE"));
    }

    async fn read(rig: &Rig) {
        for task in rig.fleet.questions_taken_to_read().await {
            task.await.expect("the read ends");
        }
    }

    async fn asked(rig: &Rig, id: &SessionId) -> String {
        told_once(rig, id, WHY).await;
        rig.rows(id)
            .await
            .iter()
            .find_map(|row| match row {
                SessionRow::Message { from: SessionVoice::You, id, text, .. } if text == WHY => Some(id.clone()),
                _ => None,
            })
            .expect("his question")
    }

    /// The Job's reply, put as its thread keeps one, and its turn's end.
    async fn replied(rig: &Rig, id: &SessionId, text: &str) -> String {
        let row = said_back(rig, id, text).await;
        rig.fleet.questions_due_at(id.as_str()).await;
        row
    }

    /// The Job's reply, put as its thread keeps one, with no turn's end after it.
    async fn said_back(rig: &Rig, id: &SessionId, text: &str) -> String {
        let row = rig.fleet.row_id(id.as_str());
        let reply = SessionRow::Message {
            id: row.clone(),
            at: rig.fleet.instant(),
            from: SessionVoice::Agent,
            text: text.into(),
            files: Vec::new(),
            tags: Vec::new(),
            notice: None,
            queued: false,
            edited_at: None,
            aside: None,
            sketches: Vec::new(),
        };
        rig.fleet.row_put(id.as_str(), reply).await;
        row
    }

    fn links(judge: &FakeJudge) -> usize {
        judge.asked().iter().filter(|one| one.contains(questions::ANSWERS)).count()
    }

    async fn unread(rig: &Rig, id: &SessionId) -> bool {
        Arc::clone(&rig.fleet).get_session(id.clone()).await.expect("read").session.answer_unread
    }

    #[tokio::test]
    async fn a_question_is_linked_to_the_reply_that_answers_it_and_marked_read() {
        let (rig, _) = rig_saying("kind: question\nanswers: yes").await;
        let id = rig.start_titled(Some("Fix votes")).await;
        let question = asked(&rig, &id).await;
        sorted(&rig).await;
        assert_eq!(rig.fleet.list_message_kinds(id.clone()).await.expect("listed").messages[0].kind, MessageKind::Question);
        // Sorted before any reply: due, and nothing to read yet.
        read(&rig).await;
        assert!(rig.fleet.list_job_questions(id.clone()).await.expect("listed").questions[0].answer.is_none());

        let reply = replied(&rig, &id, REPLY).await;
        read(&rig).await;
        let listed = rig.fleet.list_job_questions(id.clone()).await.expect("listed").questions;
        assert_eq!((listed.len(), listed[0].row_id.as_str(), listed[0].words.as_str()), (1, question.as_str(), WHY));
        let answer = listed[0].answer.clone().expect("answered");
        assert_eq!((answer.reply_row.as_str(), answer.read), (reply.as_str(), false));
        assert_eq!(answer.excerpt, "It is red because the vote failed. I left it as is.");
        assert!(unread(&rig, &id).await, "the row carries the unread answer");

        let marked = rig.fleet.mark_answer_read(MarkAnswerRead { session_id: id.clone(), row_id: question.clone() }).await.expect("marked");
        assert!(marked.answer.expect("answered").read);
        assert!(!unread(&rig, &id).await);
        let unknown = rig.fleet.mark_answer_read(MarkAnswerRead { session_id: id.clone(), row_id: "nope".into() }).await;
        assert!(matches!(unknown, Err(api::Refusal::NotFound(_))));
    }

    #[tokio::test]
    async fn the_turn_end_marks_a_question_due() {
        let (rig, _) = rig_saying("kind: question\nanswers: yes").await;
        let id = rig.start_titled(Some("Fix votes")).await;
        rig.send(&id, WHY).await;
        sorted(&rig).await;
        read(&rig).await;
        rig.stand_in.finishes(0, REPLY);
        eventually(|| async {
            read(&rig).await;
            rig.fleet.list_job_questions(id.clone()).await.expect("listed").questions.iter().any(|one| one.answer.is_some())
        })
        .await;
    }

    #[tokio::test]
    async fn a_reply_read_as_not_answering_leaves_the_question_open_and_is_not_read_again() {
        let (rig, judge) = rig_saying("kind: question\nanswers: no").await;
        let id = rig.start_titled(Some("Fix votes")).await;
        let question = asked(&rig, &id).await;
        sorted(&rig).await;
        replied(&rig, &id, "Looking now.").await;
        read(&rig).await;
        assert_eq!(links(&judge), 1);
        assert!(rig.fleet.list_job_questions(id.clone()).await.expect("listed").questions[0].answer.is_none());
        let open = rig.fleet.mark_answer_read(MarkAnswerRead { session_id: id.clone(), row_id: question }).await;
        assert!(matches!(open, Err(api::Refusal::IllegalMove(_))));

        rig.fleet.questions_due_at(id.as_str()).await;
        read(&rig).await;
        assert_eq!(links(&judge), 1, "the same reply is not read twice");
        assert!(!unread(&rig, &id).await);
    }

    #[test]
    fn the_bare_yes_the_cheapest_model_answers_is_read_and_so_is_markdown() {
        // What haiku printed, byte for byte, four runs of four, for the question left open on 17 Nov.
        assert_eq!(questions::read_of("yes\n"), Some(true));
        assert_eq!(questions::read_of("No."), Some(false));
        assert_eq!(questions::read_of("**Answers:** Yes"), Some(true));
        assert_eq!(questions::read_of("answers: **no**"), Some(false));
        assert_eq!(questions::read_of("Looking at the reply,\nanswers: yes"), Some(true), "the labelled line wins");
    }

    /// The live case: his question sorted after the turn that answered it ended, so the only read is the one sorting makes
    /// due, and the model answers a bare `yes`.
    #[tokio::test]
    async fn a_question_sorted_after_its_answer_is_linked_on_a_bare_yes() {
        let judge = Arc::new(FakeJudge::answering(&[("Sort it.", "kind: question"), ("whether it answers the question", "yes")]));
        let planted = Arc::clone(&judge);
        let rig = serving_fitted("armada", move |fittings| fittings.judge = planted).await;
        let id = rig.start_titled(Some("Fix votes")).await;
        asked(&rig, &id).await;
        let reply = said_back(&rig, &id, REPLY).await;
        sorted(&rig).await;
        read(&rig).await;
        let answer = rig.fleet.list_job_questions(id.clone()).await.expect("listed").questions[0].answer.clone();
        assert_eq!(answer.map(|one| one.reply_row), Some(reply));
    }

    #[tokio::test]
    async fn a_failed_read_is_tried_again_on_the_next_passes_three_in_all_then_waits_for_a_turn_end() {
        let judge = Arc::new(FakeJudge::that_fails("the linking model"));
        let planted = Arc::clone(&judge);
        let rig = serving_fitted("armada", move |fittings| fittings.judge = planted).await;
        let id = rig.start_titled(Some("Fix votes")).await;
        let question = asked(&rig, &id).await;
        sorted(&rig).await;
        let set = SetMessageKind { session_id: id.clone(), row_id: question, kind: MessageKind::Question, ask_kind: None, ask_id: None };
        Arc::clone(&rig.fleet).set_message_kind(set).await.expect("his correction");
        replied(&rig, &id, REPLY).await;
        read(&rig).await;
        assert_eq!(links(&judge), 1);
        assert!(rig.fleet.list_job_questions(id.clone()).await.expect("listed").questions[0].answer.is_none());

        read(&rig).await;
        read(&rig).await;
        assert_eq!(links(&judge), questions::TRIES as usize, "tried again on the next passes");
        read(&rig).await;
        assert_eq!(links(&judge), questions::TRIES as usize, "and then nothing reads it before a turn ends");
        rig.fleet.questions_due_at(id.as_str()).await;
        read(&rig).await;
        assert_eq!(links(&judge), questions::TRIES as usize + 1, "a turn's end starts the count over");
        assert!(rig.fleet.list_job_questions(id.clone()).await.expect("listed").questions[0].answer.is_none());
    }
}

/// The features a Job is on, read from the files it wrote, the commits that named them and its asks filed under them.
mod features_of_a_job {
    use api::{Features, Story};
    use ipc::SessionId;
    use store::{feature_link_kind, KeptAsk, KeptFeatureLink};

    use super::super::agent_starts::{rig, Rig};
    use super::super::feature_library::{a_project, an_agent_for, feature, named, seeded, wrote, VOTING};

    const ACTIONS: &str = "page-actions";

    async fn commit_linked(rig: &Rig, id: &SessionId, slug: &str, project: &str, sha: &str) {
        let mut store = rig.fleet.store().lock().await;
        let feature = store.feature_named(project, slug).unwrap().expect("held");
        let link = KeptFeatureLink {
            feature_id: feature.id,
            kind: feature_link_kind::COMMIT.into(),
            target: sha.into(),
            job: id.as_str().into(),
            title: String::new(),
            state: String::new(),
            via: "trailer".into(),
            at: rig.fleet.now().as_str().into(),
            gist: String::new(),
            gist_by: String::new(),
        };
        store.put_feature_link(&link).expect("linked");
    }

    async fn ask_filed(rig: &Rig, id: &SessionId, ask: &str, slugs: &[&str]) {
        let mut store = rig.fleet.store().lock().await;
        let now = rig.fleet.now().as_str().to_string();
        let kept = KeptAsk {
            ask_id: ask.into(),
            session_id: id.as_str().into(),
            manifest_id: None,
            project: None,
            kind: "bug".into(),
            title: format!("Ask {ask}"),
            state: "building".into(),
            building: Some(id.as_str().into()),
            opened_at: now.clone(),
            state_at: now,
        };
        store.keep_ask(&kept).expect("kept");
        store.keep_ask_features(ask, &slugs.iter().map(|slug| slug.to_string()).collect::<Vec<_>>()).expect("filed");
    }

    #[tokio::test]
    async fn each_feature_counts_the_files_written_the_commits_that_named_it_and_the_asks_filed_most_touched_first() {
        let rig = rig(None);
        let project = a_project(&rig).await;
        rig.fleet.upsert_features(None, seeded(&project)).await.expect("seeded");
        {
            let mut store = rig.fleet.store().lock().await;
            let ripple = store::KeptFeature { project: project.clone(), title: "Ripple".into(), ..feature("f-ripple", "ripple", &["crates/ripple"]) };
            store.put_feature(&ripple).expect("kept");
        }
        let id = an_agent_for(&rig, &project).await;
        wrote(&rig, &id, "crates/pages/voting/ranks.rs").await;
        wrote(&rig, &id, "crates/pages/voting/tally.rs").await;
        wrote(&rig, &id, "crates/pages/voting/ranks.rs").await;
        named(&rig, &id, VOTING).await;
        for sha in ["a1", "b2", "c3", "a1"] {
            commit_linked(&rig, &id, ACTIONS, &project, sha).await;
        }
        ask_filed(&rig, &id, "ask-1", &[ACTIONS, "ripple"]).await;
        ask_filed(&rig, &id, "ask-2", &["not-a-feature"]).await;

        let story = rig.fleet.get_job_story(id.clone()).await.expect("story");
        let said: Vec<(&str, u32, u32, u32)> = story.features.iter().map(|one| (one.slug.as_str(), one.files_written, one.commits_named, one.asks)).collect();
        assert_eq!(said, [(ACTIONS, 0, 3, 1), (VOTING, 2, 1, 0), ("ripple", 0, 0, 1)], "most touched first, a file and a commit once each");
        assert_eq!((story.features[2].title.as_str(), story.features[2].area.as_str()), ("Ripple", "takes"));
    }

    #[tokio::test]
    async fn a_job_on_no_feature_has_none_and_the_wire_carries_no_field() {
        let rig = rig(None);
        let project = a_project(&rig).await;
        rig.fleet.upsert_features(None, seeded(&project)).await.expect("seeded");
        let id = an_agent_for(&rig, &project).await;
        wrote(&rig, &id, "README.md").await;

        let story = rig.fleet.get_job_story(id.clone()).await.expect("story");
        assert!(story.features.is_empty(), "{:?}", story.features);
        assert!(!ipc::encode(&story).unwrap().contains("\"features\""));
    }
}
