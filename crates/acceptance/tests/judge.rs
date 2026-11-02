//! The Judge's claim: **any artifact of a Job can be judged by one read-only call that is shown its
//! words, the request the person dispatched with, his later words and the repository's rules; his
//! "verify that" line reaches the brief, and without one the question is what was missed; each note
//! that comes back is pinned to the part it names, a plan's group or task, a diff's `path:line`, a
//! document's paragraph; an answer that cites a file the checkout lacks or quotes words it was never
//! shown is refused whole.**
//!
//! | Proved | Not proved |
//! |---|---|
//! | Every kind of artifact is laid in the brief by its words, with the request, his later words and the standing rules, and the plan with no id of it | That Fleet reads each kind off the Job's thread, ledger, plan and checkout, and never the agent's messages. `fleet::tests::judge` |
//! | His line is in the brief and the question is not "what was missed"; with no line the question is what the person or the agent missed | That a press answers `reading` at once, a second press on a reading answers the first, and a press after it is a fresh `id`, with `session.changed` on start and finish. `fleet::tests::judge` |
//! | A note is pinned to the plan task or group it quotes, to the `path:line` of a diff it names or quotes, to the paragraph of a document, else to the artifact | That Send reaches the agent as the person's message quoting the note, Dismiss tells it nothing, and a note settled is kept settled. `fleet::tests::judge`, `store::tests::judge_readings` |
//! | A cite of a path the checkout does not hold, or a quotation in neither the brief nor a cited file, refuses the answer | That a refused answer is a `failed` reading with Fleet's words and keeps no note. `fleet::tests::judge` |
//! | | That Bridge draws each note inside its artifact. Bridge's half |

use fleet::judge::{brief, notes_read, Content, Shown};
use ipc::{JudgeKind, SessionPlan};

const KEYS_RS: &str = "crates/store/src/keys.rs";
const KEYS: &str = "impl Keys {\n    pub fn rotate(&mut self, day: Day) -> Result<(), Error> {\n        self.insert(day)\n    }\n}\n";

const DIFF: &str = "diff --git a/crates/store/src/keys.rs b/crates/store/src/keys.rs\n\
--- a/crates/store/src/keys.rs\n\
+++ b/crates/store/src/keys.rs\n\
@@ -1,2 +1,5 @@\n\
\x20impl Keys {\n\
+    pub fn rotate(&mut self, day: Day) -> Result<(), Error> {\n\
+        self.insert(day)\n\
+    }\n\
\x20}\n";

fn plan() -> SessionPlan {
    let json = r#"{
        "version": 2,
        "summary": "Rotate the API keys nightly and keep the last two.",
        "recorded_at": "2026-10-25T09:00:00.000Z",
        "approved": false,
        "scope": ["crates/store", "crates/fleet"],
        "groups": [
            {"id": "g1", "agent": "rust-engineer", "title": "Keep the keys", "gives": "Old keys stay readable for a day",
             "paths": ["crates/store"], "tasks": [
                {"id": "t1", "text": "Add a key_rotations table with a unique day", "state": "open"},
                {"id": "t2", "text": "Write the rotation behind a lock", "state": "open"}]},
            {"id": "g2", "agent": "rust-engineer", "title": "Run it nightly",
             "paths": ["crates/fleet"], "tasks": [
                {"id": "t3", "text": "Schedule the rotation at midnight", "state": "open"}]}
        ]
    }"#;
    ipc::decode("a plan", json.as_bytes()).expect("a plan")
}

const REQUEST: &str = "Rotate the keys every night, and never leave two rotations on one day.";
const STANDING: &str = "Every table change ships with a migration file.";
const LATER: &str = "Keep the previous key readable for a day.";

fn checkout(path: &str) -> Option<String> {
    (path == KEYS_RS).then(|| KEYS.to_string())
}

fn the_plan() -> Content {
    Content { kind: JudgeKind::Plan, title: "The plan".into(), plan: Some(plan()), text: String::new() }
}

fn a_pull_request() -> Content {
    Content { kind: JudgeKind::PullRequest, title: "Rotate the keys nightly".into(), plan: None, text: DIFF.into() }
}

fn a_document() -> Content {
    let text = "Keys rotate at midnight.\n\nThe previous key stays readable for a day after a rotation.\n\nNothing else changes.";
    Content { kind: JudgeKind::Doc, title: "Rotation notes".into(), plan: None, text: text.into() }
}

fn brief_of(artifact: &Content, prompt: &str) -> String {
    brief(&Shown { request: REQUEST, later: &[LATER.to_string()], artifact, prompt, standing: STANDING })
}

fn answer(notes: &str) -> String {
    format!(r#"{{"notes": [{notes}]}}"#)
}

#[test]
fn any_artifact_is_shown_by_its_words_beside_the_request_and_the_rules() {
    let words = [
        (JudgeKind::Job, "the whole job diff text"),
        (JudgeKind::PullRequest, "the pull request diff text"),
        (JudgeKind::Commit, "the commit diff text"),
        (JudgeKind::File, "the file and its diff text"),
        (JudgeKind::Page, "https://staging.example/keys and its walk steps"),
        (JudgeKind::Doc, "the document text"),
        (JudgeKind::Sketch, "the sketch scene text"),
        (JudgeKind::Command, "the command and its output text"),
        (JudgeKind::Subagent, "the subagent report text"),
        (JudgeKind::Issue, "the issue title and body text"),
        (JudgeKind::Note, "the note text"),
    ];
    for (kind, text) in words {
        let artifact = Content { kind, title: format!("A {kind:?}"), plan: None, text: text.into() };
        let brief = brief_of(&artifact, "");
        for shown in [REQUEST, LATER, STANDING, text, artifact.title.as_str()] {
            assert!(brief.contains(shown), "the brief for {kind:?} lacks {shown:?}:\n{brief}");
        }
    }

    let plan = the_plan();
    let brief = brief_of(&plan, "");
    for shown in [
        "Rotate the API keys nightly and keep the last two.",
        "Keep the keys",
        "Old keys stay readable for a day",
        "Add a key_rotations table with a unique day",
        "Write the rotation behind a lock",
        "Run it nightly",
        "Schedule the rotation at midnight",
    ] {
        assert!(brief.contains(shown), "the brief lacks {shown:?}:\n{brief}");
    }
    for id in ["g1", "g2", "t1", "t2", "t3"] {
        assert!(!brief.contains(id), "the brief names the plan's id {id}:\n{brief}");
    }
}

#[test]
fn his_line_reaches_the_brief_and_without_one_the_question_is_what_was_missed() {
    let plan = the_plan();
    let line = "Verify that the rotation never leaves two keys on one day";

    let asked = brief_of(&plan, line);
    assert!(asked.contains(line), "his line is not in the brief:\n{asked}");
    assert!(!asked.contains("what the person or the agent missed"), "an open question beside his line:\n{asked}");

    let open = brief_of(&plan, "  ");
    assert!(open.contains("what the person or the agent missed"), "the open question is not asked:\n{open}");
    assert!(!open.contains(line));
}

#[test]
fn each_note_is_pinned_to_the_part_it_names() {
    let plan = the_plan();
    let on_a_task = r#"{"about": "Write the rotation behind a lock",
        "claim": "Two rotations can land on one day.",
        "expected": "The rotation takes a lock the store holds.",
        "found": "The plan says \"Write the rotation behind a lock\" and the store has no lock.",
        "cites": ["crates/store/src/keys.rs:2"]}"#;
    let on_a_group = r#"{"about": "Keep the keys",
        "claim": "The new table ships without a migration.",
        "expected": "The standing rules say \"every table change ships with a migration file\".",
        "found": "No task writes one."}"#;
    let on_the_plan = r#"{"about": "",
        "claim": "Yesterday's key is unreadable after a rotation.",
        "impact": "Someone signing in with yesterday's key is turned away."}"#;
    let brief = brief_of(&plan, "");
    let notes = notes_read(&answer(&[on_a_task, on_a_group, on_the_plan].join(",")), &brief, &plan, &checkout)
        .expect("the answer holds up");
    let about: Vec<Option<&str>> = notes.iter().map(|one| one.about.as_deref()).collect();
    assert_eq!(about, [Some("t2"), Some("g1"), None]);
    assert_eq!(notes[0].r#where.as_deref(), Some("Plan, task Write the rotation behind a lock"));
    assert_eq!(notes[1].r#where.as_deref(), Some("Plan, group Keep the keys"));
    assert_eq!(notes[2].r#where, None);
    assert_eq!(notes[0].claim, "Two rotations can land on one day.");
    assert_eq!(notes[0].cites, ["crates/store/src/keys.rs:2"]);
    assert!(notes.iter().all(|one| one.settled.is_none()), "every note waits for the person");
    let ids: Vec<&str> = notes.iter().map(|one| one.id.as_str()).collect();
    assert_eq!(ids, ["n1", "n2", "n3"]);

    let pull = a_pull_request();
    let brief = brief_of(&pull, "");
    let named = r#"{"about": "crates/store/src/keys.rs:3", "claim": "The insert takes no lock."}"#;
    let quoted = r#"{"about": "pub fn rotate(&mut self, day: Day)", "claim": "A second call on one day inserts twice."}"#;
    let nowhere = r#"{"about": "crates/store/src/keys.rs:40", "claim": "Nothing is written to the log."}"#;
    let elsewhere = r#"{"about": "crates/fleet/src/clock.rs:3", "claim": "The clock is read twice."}"#;
    let notes = notes_read(&answer(&[named, quoted, nowhere, elsewhere].join(",")), &brief, &pull, &checkout)
        .expect("the answer holds up");
    let about: Vec<Option<&str>> = notes.iter().map(|one| one.about.as_deref()).collect();
    assert_eq!(about, [Some("crates/store/src/keys.rs:3"), Some("crates/store/src/keys.rs:2"), None, None]);
    assert_eq!(notes[0].r#where.as_deref(), Some("crates/store/src/keys.rs:3"));

    let page = a_document();
    let brief = brief_of(&page, "");
    let second = r#"{"about": "The previous key stays readable for a day", "claim": "The code keeps one key."}"#;
    let notes = notes_read(&answer(second), &brief, &page, &checkout).expect("the answer holds up");
    assert_eq!(notes[0].about.as_deref(), Some("p:The previous key stays readable for a day after a rotation."));
    assert_eq!(notes[0].r#where.as_deref(), Some("The previous key stays readable for a day..."));
}

#[test]
fn an_answer_citing_a_file_the_checkout_lacks_or_quoting_words_never_shown_is_refused() {
    let plan = the_plan();
    let brief = brief_of(&plan, "");

    let elsewhere = r#"{"about": "", "claim": "Two rotations can land on one day.", "cites": ["crates/store/src/locks.rs:9"]}"#;
    let refused = notes_read(&answer(elsewhere), &brief, &plan, &checkout).expect_err("a cite the checkout lacks");
    assert!(refused.contains("crates/store/src/locks.rs"), "the refusal names the path: {refused}");

    let invented = r#"{"about": "", "claim": "The plan says \"the scheduler takes the lock first\" and it does not."}"#;
    let refused = notes_read(&answer(invented), &brief, &plan, &checkout).expect_err("words never shown");
    assert!(refused.contains("the scheduler takes the lock first"), "the refusal names the words: {refused}");

    assert!(notes_read("I could not find anything.", &brief, &plan, &checkout).is_err(), "no JSON is no answer");
    assert_eq!(notes_read(&answer(""), &brief, &plan, &checkout), Ok(Vec::new()), "nothing found is an answer");
}
