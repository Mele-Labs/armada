//! A Judge on demand through a real Fleet and the judge test double: what a call is shown and where it
//! reads, the reading landing on the Job's record, kept per artifact, and the person's acts on a note.
//! `docs/concepts/judge.md`; the pure half is `crates/acceptance/tests/judge.rs`.

use std::sync::Arc;
use std::time::Duration;

use api::{HostedSessions, JudgeReadings, Next, Refusal};
use ipc::{
    AskJudge, CloseSession, DismissJudgeNote, Event, JudgeArtifact, JudgeKind, JudgeNoteHow, JudgeReading, JudgeState,
    MarkJudgeRead, SendJudgeNote, SessionId,
};
use store::{AttachmentState, Holder, KeptAttachment, KeptJudgeReading};
use testkit::FakeJudge;

use super::session_host::{eventually, rig_fitted, Rig};
use super::session_plan::{group, plan, recorded};

/// Every brief opens with it, so the double answers this call and no other.
const ASKED: &str = "You are the Judge";
const REQUEST: &str = "Rotate the keys every night, and never leave two rotations on one day.";
const LATER: &str = "Keep the previous key readable for a day.";
const AGENT_SAID: &str = "I checked everything and the lock is already there.";

const KEYS_RS: &str = "crates/store/src/keys.rs";
const KEYS: &str = "impl Keys {\n    pub fn rotate(&mut self, day: Day) -> Result<(), Error> {\n        self.insert(day)\n    }\n}\n";

const NOTES: &str = r#"{"notes": [
    {"about": "Write the rotation behind a lock", "claim": "Two rotations can land on one day.",
     "expected": "The rotation takes a lock the store holds.", "found": "The store has no lock.",
     "impact": "Someone rotating their keys at midnight finds two of them on one day.",
     "cites": ["crates/store/src/keys.rs:2"]},
    {"about": "Keep the keys", "claim": "The new table ships without a migration.",
     "expected": "A migration file beside the table.", "found": "No task writes one."},
    {"about": "", "claim": "Yesterday's key is unreadable after a rotation.",
     "found": "keys.rs reads \"pub fn rotate(&mut self, day: Day)\" and keeps one key.",
     "cites": ["crates/store/src/keys.rs"]}
]}"#;

fn answered_with(answer: &str) -> (Rig, Arc<FakeJudge>) {
    answered_by(FakeJudge::answering(&[(ASKED, answer)]).costing(0.4))
}

fn answered_by(judge: FakeJudge) -> (Rig, Arc<FakeJudge>) {
    let judge = Arc::new(judge);
    let held = Arc::clone(&judge);
    let rig = rig_fitted(|fittings| fittings.judge = held);
    let file = std::path::Path::new(&rig.root).join(KEYS_RS);
    std::fs::create_dir_all(file.parent().expect("a folder")).expect("made");
    std::fs::write(file, KEYS).expect("written");
    (rig, judge)
}

fn the_plan(id: &SessionId) -> ipc::RecordPlan {
    let mut keys = group(None, "Keep the keys", "rust-engineer", &["crates/store"], &[
        (None, "Add a key_rotations table with a unique day"),
        (None, "Write the rotation behind a lock"),
    ]);
    keys.gives = Some("Old keys stay readable for a day".into());
    plan(id, vec![keys, group(None, "Run it nightly", "rust-engineer", &["crates/fleet"], &[(None, "Schedule the rotation at midnight")])])
}

/// A Job the person asked something of, whose agent answered, who said more, and which recorded a plan.
async fn a_job_with_a_plan(rig: &Rig) -> SessionId {
    let id = rig.start().await;
    rig.send(&id, REQUEST).await;
    rig.stand_in.init(0);
    rig.stand_in.finishes(0, AGENT_SAID);
    eventually(|| async { rig.rows(&id).await.iter().any(|row| format!("{row:?}").contains(AGENT_SAID)) }).await;
    rig.send(&id, LATER).await;
    recorded(rig, the_plan(&id)).await;
    id
}

fn the_plan_artifact() -> JudgeArtifact {
    JudgeArtifact { kind: JudgeKind::Plan, id: "1".into(), title: "The plan".into() }
}

fn artifact(kind: JudgeKind, id: &str, title: &str) -> JudgeArtifact {
    JudgeArtifact { kind, id: id.into(), title: title.into() }
}

async fn ask(rig: &Rig, id: &SessionId, artifact: JudgeArtifact, prompt: &str) -> Result<JudgeReading, Refusal> {
    Arc::clone(&rig.fleet).ask_judge(AskJudge { session_id: id.clone(), artifact, prompt: prompt.into() }).await
}

async fn readings(rig: &Rig, id: &SessionId) -> Vec<JudgeReading> {
    Arc::clone(&rig.fleet).get_session(id.clone()).await.expect("read").session.judge_readings
}

/// The Job's reading of `kind` once it has ended.
async fn ended(rig: &Rig, id: &SessionId, kind: JudgeKind) -> JudgeReading {
    let held = |readings: Vec<JudgeReading>| readings.into_iter().find(|one| one.artifact.kind == kind);
    eventually(|| async { held(readings(rig, id).await).is_some_and(|one| one.state != JudgeState::Reading) }).await;
    held(readings(rig, id).await).expect("a reading")
}

fn asked_of(judge: &FakeJudge) -> Vec<(String, Option<String>)> {
    judge.asked().into_iter().zip(judge.directories()).filter(|(asked, _)| asked.starts_with(ASKED)).collect()
}

fn told(rig: &Rig, containing: &str) -> usize {
    rig.stand_in.process(0).sent.lock().unwrap().iter().filter(|line| line.contains(containing)).count()
}

fn code(refusal: Refusal) -> String {
    refusal.error().code.clone()
}

/// The Job holds `kind` named `target`, with `detail`, in its ledger.
async fn holds(rig: &Rig, id: &SessionId, kind: &str, target: &str, detail: &[(&str, &str)]) {
    let at = rig.fleet.now().as_str().to_string();
    rig.fleet
        .store()
        .lock()
        .await
        .attach(
            &KeptAttachment {
                holder: Holder::session(id.as_str()),
                kind: kind.into(),
                manifest_id: rig.manifest.as_str().to_string(),
                target: target.into(),
                state: AttachmentState::Standing,
                detail: detail.iter().map(|(key, value)| (key.to_string(), value.to_string())).collect(),
                since: at.clone(),
                changed_at: at,
            },
            false,
        )
        .expect("attached");
}

#[tokio::test]
async fn a_press_answers_reading_and_lands_done_with_each_note_pinned() {
    let (rig, judge) = answered_with(NOTES);
    let id = a_job_with_a_plan(&rig).await;
    let mut watching = rig.fleet.events().subscribe();

    let started = ask(&rig, &id, the_plan_artifact(), "").await.expect("started");
    assert_eq!((started.state, started.notes.len(), started.prompt.as_str()), (JudgeState::Reading, 0, ""));
    let reading = ended(&rig, &id, JudgeKind::Plan).await;

    assert_eq!(reading.state, JudgeState::Done, "{:?}", reading.failed);
    assert_eq!(reading.id, started.id);
    let pinned: Vec<Option<&str>> = reading.notes.iter().map(|one| one.about.as_deref()).collect();
    assert_eq!(pinned, [Some("t2"), Some("g1"), None]);
    assert_eq!(reading.notes[0].r#where.as_deref(), Some("Plan, task Write the rotation behind a lock"));
    assert_eq!(reading.cost_micros, Some(400_000), "what the vendor billed is kept with the reading");
    assert!(reading.finished_at.is_some() && reading.failed.is_none() && !reading.read);

    // One call, on models.review, in the checkout Fleet serves: shown the person's words and the plan's,
    // never the agent's, and no id.
    let asked = asked_of(&judge);
    assert_eq!(asked.len(), 1);
    let (brief, directory) = &asked[0];
    for shown in [REQUEST, LATER, "Write the rotation behind a lock", "Keep the keys", "Old keys stay readable for a day"] {
        assert!(brief.contains(shown), "the brief lacks {shown:?}:\n{brief}");
    }
    assert!(!brief.contains(AGENT_SAID), "the agent's words are withheld:\n{brief}");
    assert!(!brief.contains("t2") && !brief.contains("g1"), "no id is shown:\n{brief}");
    assert_eq!(directory.as_deref(), Some(rig.root.as_str()));
    assert_eq!(judge.models().last().map(String::as_str), Some("a-model"), "models.review");

    // The Job changed on the start and on the finish.
    let mut states = Vec::new();
    while states.last() != Some(&JudgeState::Done) {
        match tokio::time::timeout(Duration::from_secs(5), watching.next()).await.expect("an event") {
            Some(Next::Send(delivered)) => {
                if let Event::SessionChanged(record) = delivered.event {
                    let held = record.judge_readings.iter().find(|one| one.artifact.kind == JudgeKind::Plan);
                    if let Some(one) = held.filter(|_| record.id == id) {
                        if states.last() != Some(&one.state) {
                            states.push(one.state);
                        }
                    }
                }
            }
            _ => panic!("the stream ended"),
        }
    }
    assert_eq!(states, [JudgeState::Reading, JudgeState::Done]);
}

#[tokio::test]
async fn a_press_on_a_reading_answers_it_and_a_press_after_it_is_a_fresh_reading() {
    let (rig, judge) = answered_by(FakeJudge::answering(&[(ASKED, NOTES)]).taking(Duration::from_millis(300)));
    let id = a_job_with_a_plan(&rig).await;

    let first = ask(&rig, &id, the_plan_artifact(), "").await.expect("started");
    let second = ask(&rig, &id, the_plan_artifact(), "check the lock").await.expect("answered");
    assert_eq!((second.id.as_str(), second.state, second.prompt.as_str()), (first.id.as_str(), JudgeState::Reading, ""));
    let done = ended(&rig, &id, JudgeKind::Plan).await;
    assert_eq!(done.id, first.id);
    assert_eq!(asked_of(&judge).len(), 1, "one call for two presses");

    let again = ask(&rig, &id, the_plan_artifact(), "").await.expect("asked again");
    assert_ne!(again.id, first.id, "a re-ask gets a fresh id");
    assert_eq!(again.state, JudgeState::Reading);
    assert!(again.notes.is_empty(), "the old notes are gone");
    let done = ended(&rig, &id, JudgeKind::Plan).await;
    assert_eq!((done.id, done.notes.len()), (again.id, 3));
    assert_eq!(readings(&rig, &id).await.len(), 1, "one reading per artifact");
    assert_eq!(asked_of(&judge).len(), 2);
}

#[tokio::test]
async fn his_line_reaches_the_brief_and_without_one_the_question_is_what_was_missed() {
    let (rig, judge) = answered_with(NOTES);
    let id = a_job_with_a_plan(&rig).await;
    let line = "Verify that the rotation never leaves two keys on one day";

    ask(&rig, &id, the_plan_artifact(), line).await.expect("started");
    let reading = ended(&rig, &id, JudgeKind::Plan).await;
    assert_eq!(reading.prompt, line);
    ask(&rig, &id, the_plan_artifact(), "").await.expect("started");
    eventually(|| async { asked_of(&judge).len() == 2 }).await;

    let asked = asked_of(&judge);
    assert!(asked[0].0.contains(line) && !asked[0].0.contains("what the person or the agent missed"), "{}", asked[0].0);
    assert!(asked[1].0.contains("what the person or the agent missed") && !asked[1].0.contains(line), "{}", asked[1].0);
}

#[tokio::test]
async fn each_kind_is_read_from_where_fleet_holds_it() {
    let (rig, judge) = answered_with(r#"{"notes": []}"#);
    let id = a_job_with_a_plan(&rig).await;
    let page = std::path::Path::new(&rig.root).join("rotation-notes.html");
    std::fs::write(&page, "<h1>Rotation notes</h1><p>Keys rotate at midnight.</p><p>The old key stays for a day.</p>").expect("written");
    let page = page.to_str().expect("a path").to_string();

    holds(&rig, &id, "issue", "41", &[("title", "Keys never rotate"), ("body", "The nightly job was never scheduled.")]).await;
    holds(&rig, &id, "artifact", "note:n1", &[("form", "note"), ("text", "Remember the leap day."), ("title", "A reminder")]).await;
    holds(&rig, &id, "artifact", "sketch:Key flow", &[("form", "sketch"), ("title", "Key flow"), ("scene", "{\"nodes\":[{\"id\":\"a\",\"title\":\"Vault\"}]}")]).await;
    holds(&rig, &id, "artifact", "https://staging.example/keys", &[("form", "window"), ("title", "Keys page"), ("steps", "Press Rotate")]).await;
    holds(&rig, &id, "artifact", &page, &[("form", "document"), ("title", "Rotation notes")]).await;

    let cases = [
        (artifact(JudgeKind::Plan, "1", "The plan"), "Schedule the rotation at midnight"),
        (artifact(JudgeKind::Issue, "41", "Keys never rotate"), "The nightly job was never scheduled."),
        (artifact(JudgeKind::Note, "note:n1", "A reminder"), "Remember the leap day."),
        (artifact(JudgeKind::Sketch, "sketch:Key flow", "Key flow"), "Vault"),
        (artifact(JudgeKind::Page, "https://staging.example/keys", "Keys page"), "Press Rotate"),
        (artifact(JudgeKind::Doc, &page, "Rotation notes"), "The old key stays for a day."),
    ];
    for (n, (one, words)) in cases.into_iter().enumerate() {
        let kind = one.kind;
        ask(&rig, &id, one, "").await.expect("started");
        let reading = ended(&rig, &id, kind).await;
        assert_eq!(reading.state, JudgeState::Done, "{kind:?}: {:?}", reading.failed);
        let asked = asked_of(&judge);
        assert_eq!(asked.len(), n + 1);
        assert!(asked[n].0.contains(words), "the brief for {kind:?} lacks {words:?}:\n{}", asked[n].0);
        assert!(asked[n].0.contains(REQUEST) && !asked[n].0.contains(AGENT_SAID), "{kind:?}");
    }
    let held: Vec<JudgeKind> = readings(&rig, &id).await.into_iter().map(|one| one.artifact.kind).collect();
    assert_eq!(held.len(), 6, "one reading each, oldest asked first");
    assert_eq!(held[0], JudgeKind::Plan);
}

#[tokio::test]
async fn a_commit_is_read_as_its_diff_from_the_checkout_and_one_the_checkout_lacks_fails() {
    let patch = "diff --git a/crates/store/src/keys.rs b/crates/store/src/keys.rs\n--- a/crates/store/src/keys.rs\n+++ b/crates/store/src/keys.rs\n@@ -1,2 +1,3 @@\n impl Keys {\n+    pub fn rotate_at_midnight(&mut self) {}\n }\n";
    let judge = Arc::new(FakeJudge::answering(&[(ASKED, r#"{"notes": []}"#)]));
    let held = Arc::clone(&judge);
    let rig = rig_fitted(|fittings| {
        fittings.judge = held;
        fittings.work = testkit::FakeWorkProduct::changed(&[]).holding_commit("a1b2c3d", patch);
    });
    let id = a_job_with_a_plan(&rig).await;

    ask(&rig, &id, artifact(JudgeKind::Commit, "a1b2c3d", "Rotate at midnight"), "").await.expect("started");
    let reading = ended(&rig, &id, JudgeKind::Commit).await;
    assert_eq!(reading.state, JudgeState::Done, "{:?}", reading.failed);
    let asked = asked_of(&judge);
    assert_eq!(asked.len(), 1);
    assert!(asked[0].0.contains("pub fn rotate_at_midnight(&mut self) {}"), "the diff is in the brief:\n{}", asked[0].0);
    assert!(asked[0].0.contains(REQUEST) && !asked[0].0.contains(AGENT_SAID));

    ask(&rig, &id, artifact(JudgeKind::Commit, "deadbeef", "A commit nobody has"), "").await.expect("answered at once");
    eventually(|| async { readings(&rig, &id).await.iter().any(|one| one.artifact.id == "deadbeef" && one.state == JudgeState::Failed) }).await;
    let failed = readings(&rig, &id).await.into_iter().find(|one| one.artifact.id == "deadbeef").expect("a reading");
    assert!(failed.failed.expect("Fleet's words").contains("the commit deadbeef would not read"));
    assert_eq!(asked_of(&judge).len(), 1, "no call for a commit that could not be shown");
}

#[tokio::test]
async fn what_fleet_holds_nothing_readable_of_fails_the_reading_with_its_reason() {
    let (rig, judge) = answered_with(NOTES);
    let id = a_job_with_a_plan(&rig).await;
    holds(&rig, &id, "issue", "41", &[]).await;

    let cases = [
        (artifact(JudgeKind::Commit, "abc123", "Rotate"), "the commit abc123 would not read"),
        (artifact(JudgeKind::Note, "note:gone", "A note"), "Fleet holds no note named note:gone"),
        (artifact(JudgeKind::PullRequest, "9", "Rotate"), "Fleet holds no pull request named 9"),
        (artifact(JudgeKind::Issue, "41", "An issue"), "only the number of issue 41"),
        (artifact(JudgeKind::Plan, "7", "The plan"), "recorded again"),
        (artifact(JudgeKind::Command, "ls", "ls"), "Fleet holds no command named ls"),
        (artifact(JudgeKind::Subagent, "s1", "A subagent"), "Fleet holds no subagent named s1"),
    ];
    for (one, reason) in cases {
        let kind = one.kind;
        let started = ask(&rig, &id, one, "").await.expect("a press is answered at once");
        assert_eq!(started.state, JudgeState::Reading);
        let reading = ended(&rig, &id, kind).await;
        assert_eq!(reading.state, JudgeState::Failed, "{kind:?}");
        let failed = reading.failed.expect("Fleet's words");
        assert!(failed.contains(reason), "{kind:?}: {failed}");
        assert!(reading.notes.is_empty() && reading.cost_micros.is_none(), "{kind:?}");
    }
    assert!(asked_of(&judge).is_empty(), "no call was made for what could not be shown");
}

#[tokio::test]
async fn an_answer_quoting_words_it_was_never_shown_fails_the_reading_and_keeps_nothing() {
    let invented = NOTES.replace(r#"\"pub fn rotate(&mut self, day: Day)\""#, r#"\"the scheduler takes the lock first\""#);
    let (rig, _) = answered_with(&invented);
    let id = a_job_with_a_plan(&rig).await;

    ask(&rig, &id, the_plan_artifact(), "").await.expect("started");
    let reading = ended(&rig, &id, JudgeKind::Plan).await;
    assert_eq!(reading.state, JudgeState::Failed);
    assert!(reading.notes.is_empty(), "nothing is half kept");
    let failed = reading.failed.expect("Fleet's words");
    assert!(failed.contains("the scheduler takes the lock first"), "{failed}");
    assert_eq!(reading.cost_micros, Some(400_000), "what was spent is kept on a failed reading too");
}

#[tokio::test]
async fn an_answer_citing_a_file_the_checkout_lacks_fails_the_reading() {
    let missing = NOTES.replace("crates/store/src/keys.rs:2", "crates/store/src/locks.rs:9");
    let (rig, _) = answered_with(&missing);
    let id = a_job_with_a_plan(&rig).await;

    ask(&rig, &id, the_plan_artifact(), "").await.expect("started");
    let reading = ended(&rig, &id, JudgeKind::Plan).await;
    assert_eq!(reading.state, JudgeState::Failed);
    assert!(reading.failed.expect("Fleet's words").contains("crates/store/src/locks.rs"));
    assert!(reading.notes.is_empty());
}

/// What the call printed when it was not logged in: a long stream of events, then the result that
/// names the failure and, in the real run, a cost of zero.
fn a_long_stream_ending_in(result: &str) -> String {
    let init = format!(
        r#"{{"type":"system","subtype":"init","original_cwd":"/Users/user/work/repo","tools":[{}]}}"#,
        (0..120).map(|n| format!("\"Command{n}\"")).collect::<Vec<_>>().join(",")
    );
    format!("{init}\n{result}")
}

/// The reading of a Job whose call exited 3 after printing `stdout` and `stderr`.
async fn failed_after_printing(stdout: &str, stderr: &str) -> JudgeReading {
    let judge = Arc::new(FakeJudge::that_fails_after_printing(stdout, stderr));
    let rig = rig_fitted(|fittings| fittings.judge = judge);
    let id = a_job_with_a_plan(&rig).await;
    ask(&rig, &id, the_plan_artifact(), "").await.expect("started");
    let reading = ended(&rig, &id, JudgeKind::Plan).await;
    assert_eq!(reading.state, JudgeState::Failed);
    reading
}

#[tokio::test]
async fn a_call_that_failed_is_worded_in_one_short_line_from_what_the_call_said_and_not_its_stream() {
    let result = r#"{"type":"result","subtype":"success","is_error":true,"result":"Not logged in · Please run /login","total_cost_usd":0,"duration_api_ms":0}"#;
    let stream = a_long_stream_ending_in(result);
    assert!(stream.len() > 1500, "the stream is long: {}", stream.len());

    let failed = failed_after_printing(&stream, "").await.failed.expect("Fleet's words");
    assert!(failed.contains("code 3") && failed.contains("Not logged in · Please run /login"), "{failed}");
    assert!(failed.chars().count() <= 260, "one short line, not the stream: {failed}");
    assert!(!failed.contains('{') && !failed.contains("original_cwd") && !failed.contains('\n'), "{failed}");

    // No result event to read: the last line the call wrote to stderr.
    let failed = failed_after_printing(&a_long_stream_ending_in("not an event"), "warming up\nNot logged in\n").await.failed.expect("words");
    assert!(failed.contains("code 3") && failed.ends_with("Not logged in"), "{failed}");
    assert!(!failed.contains("original_cwd") && !failed.contains("warming up"), "{failed}");

    // A said text longer than the cut is cut, with the cut marked.
    let long = format!(r#"{{"type":"result","is_error":true,"result":"{}"}}"#, "x".repeat(900));
    let failed = failed_after_printing(&long, "").await.failed.expect("words");
    assert!(failed.chars().count() <= 260 && failed.ends_with('…'), "{failed}");
}

#[tokio::test]
async fn a_failed_call_that_billed_nothing_leaves_the_cost_absent_and_one_that_billed_keeps_it() {
    let result = |cost: &str| format!(r#"{{"type":"result","is_error":true,"result":"Not logged in","total_cost_usd":{cost}}}"#);
    let nothing = failed_after_printing(&a_long_stream_ending_in(&result("0")), "").await;
    assert_eq!(nothing.cost_micros, None, "a reported zero is not a cost, and the wire says absent means the call did not say");

    let unsaid = failed_after_printing(&a_long_stream_ending_in(r#"{"type":"result","is_error":true,"result":"Not logged in"}"#), "").await;
    assert_eq!(unsaid.cost_micros, None);

    let billed = failed_after_printing(&a_long_stream_ending_in(&result("0.37")), "").await;
    assert_eq!(billed.cost_micros, Some(370_000), "what a failed call did cost is kept");
}

async fn act_on(rig: &Rig, id: &SessionId, reading: &str, note: &str, send: bool) -> Result<JudgeReading, Refusal> {
    let fleet = Arc::clone(&rig.fleet);
    match send {
        true => fleet.send_judge_note(SendJudgeNote { session_id: id.clone(), reading: reading.into(), note: note.into() }).await,
        false => fleet.dismiss_judge_note(DismissJudgeNote { session_id: id.clone(), reading: reading.into(), note: note.into() }).await,
    }
}

#[tokio::test]
async fn a_note_is_sent_as_the_persons_message_or_dismissed_and_the_agent_told_nothing() {
    let (rig, _) = answered_with(NOTES);
    let id = a_job_with_a_plan(&rig).await;
    ask(&rig, &id, the_plan_artifact(), "").await.expect("started");
    let reading = ended(&rig, &id, JudgeKind::Plan).await;
    let how = |one: &JudgeReading, at: usize| one.notes[at].settled.as_ref().map(|settled| settled.how);

    let sent = act_on(&rig, &id, &reading.id, "n1", true).await.expect("sent");
    assert_eq!(how(&sent, 0), Some(JudgeNoteHow::Sent));
    eventually(|| async { told(&rig, "Two rotations can land on one day.") == 1 }).await;
    assert_eq!(told(&rig, "A Judge read your plan, at Plan, task Write the rotation behind a lock"), 1, "the part, as the person reads it");
    assert_eq!(told(&rig, "crates/store/src/keys.rs:2"), 1, "and what it cites");
    assert_eq!(told(&rig, "Impact: Someone rotating their keys at midnight finds two of them on one day."), 1, "and the impact");
    assert_eq!(told(&rig, &reading.id), 0, "no id reaches the agent");

    let dismissed = act_on(&rig, &id, &reading.id, "n2", false).await.expect("dismissed");
    assert_eq!(how(&dismissed, 1), Some(JudgeNoteHow::Dismissed));
    assert_eq!(told(&rig, "The new table ships without a migration."), 0, "the agent is told nothing");

    // Settled is answered as it stands, with nothing sent twice.
    let again = act_on(&rig, &id, &reading.id, "n2", true).await.expect("as it stands");
    assert_eq!(how(&again, 1), Some(JudgeNoteHow::Dismissed));
    act_on(&rig, &id, &reading.id, "n1", true).await.expect("as it stands");
    assert_eq!(told(&rig, "Two rotations can land on one day."), 1);
    assert_eq!(how(&readings(&rig, &id).await[0], 2), None, "the third waits");

    assert_eq!(code(act_on(&rig, &id, &reading.id, "n9", true).await.expect_err("no such note")), "fleet.no_such_judge_note");
    assert_eq!(code(act_on(&rig, &id, "no-such", "n1", false).await.expect_err("no such reading")), "fleet.no_such_judge_reading");
    let nobody = SessionId::carried("no-such-job");
    assert_eq!(code(act_on(&rig, &nobody, &reading.id, "n1", true).await.expect_err("no such job")), "fleet.no_such_session");
}

#[tokio::test]
async fn a_reading_marked_read_says_so_and_a_new_press_clears_it() {
    let (rig, _) = answered_with(NOTES);
    let id = a_job_with_a_plan(&rig).await;
    ask(&rig, &id, the_plan_artifact(), "").await.expect("started");
    let reading = ended(&rig, &id, JudgeKind::Plan).await;

    let mark = |reading: &str| MarkJudgeRead { session_id: id.clone(), reading: reading.into() };
    let read = Arc::clone(&rig.fleet).mark_judge_read(mark(&reading.id)).await.expect("marked");
    assert!(read.read);
    assert!(readings(&rig, &id).await[0].read, "the Job's record carries it");
    assert_eq!(told(&rig, "Two rotations"), 0, "the agent is told nothing");

    let refused = Arc::clone(&rig.fleet).mark_judge_read(mark("no-such")).await.expect_err("no such reading");
    assert_eq!(code(refused), "fleet.no_such_judge_reading");

    ask(&rig, &id, the_plan_artifact(), "").await.expect("asked again");
    assert!(!readings(&rig, &id).await[0].read);
}

#[tokio::test]
async fn a_press_is_refused_for_no_job_and_for_a_job_that_ended() {
    let (rig, judge) = answered_with(NOTES);
    let nobody = SessionId::carried("no-such-job");
    assert_eq!(code(ask(&rig, &nobody, the_plan_artifact(), "").await.expect_err("no job")), "fleet.no_such_session");

    let id = a_job_with_a_plan(&rig).await;
    Arc::clone(&rig.fleet).close_session(CloseSession { session_id: id.clone() }).await.expect("closed");
    assert_eq!(code(ask(&rig, &id, the_plan_artifact(), "").await.expect_err("ended")), "fleet.session_closed");
    assert!(asked_of(&judge).is_empty());
}

#[tokio::test]
async fn a_reading_left_going_by_a_fleet_that_stopped_reads_as_failed_and_a_press_runs_it_again() {
    let (rig, judge) = answered_with(NOTES);
    let id = a_job_with_a_plan(&rig).await;
    let orphan = KeptJudgeReading {
        id: "orphan".into(),
        kind: "plan".into(),
        artifact_id: "1".into(),
        title: "The plan".into(),
        prompt: String::new(),
        state: "reading".into(),
        asked_at: "2026-10-31T09:00:00.000Z".into(),
        finished_at: None,
        failed: None,
        read: false,
        cost_micros: None,
        notes: Vec::new(),
    };
    rig.fleet.store().lock().await.keep_judge_reading(id.as_str(), &orphan).expect("kept");

    let held = readings(&rig, &id).await;
    assert_eq!((held[0].state, held[0].failed.as_deref()), (JudgeState::Failed, Some("Fleet restarted before the reading finished")));
    assert!(asked_of(&judge).is_empty(), "reading it runs nothing");

    let again = ask(&rig, &id, the_plan_artifact(), "").await.expect("again");
    assert_eq!((again.state, again.id == "orphan"), (JudgeState::Reading, false));
    assert_eq!(ended(&rig, &id, JudgeKind::Plan).await.state, JudgeState::Done);
}
