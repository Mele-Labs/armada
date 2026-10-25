//! An adversarial review's claim: **a plan waiting for approval is shown to one read-only call by its
//! words alone, beside what the person asked and the repository's rules, and each finding that comes
//! back is pinned to the task or group whose words it quotes, or to the whole plan; an answer that
//! cites a file the checkout does not hold, or quotes words it was never shown, is refused whole.**
//!
//! | Proved | Not proved |
//! |---|---|
//! | The brief carries the request, his later words, the plan's summary, titles, gives and tasks, and the standing rules, and no id of the plan | That Fleet reads them off the Job's thread, plan and Manifest, and never the agent's messages. `fleet::tests::adversarial_review` |
//! | A finding quoting a task's words is pinned to that task, one quoting a group's title to the group, one about nothing to the plan | That a start answers `running` at once, makes one call on `models.review` over the served checkout with no bay, and lands `done` on the Job's plan, kept per version, with `session.changed` on start, finish and each settle. `fleet::tests::adversarial_review` |
//! | A cite of a path the checkout does not hold, or a quotation in neither the brief nor a cited file, refuses the answer | That a refused answer is a `failed` review with Fleet's words and keeps no finding. `fleet::tests::adversarial_review` |
//! | | That Send reaches the agent as the person's message naming the part, and Dismiss tells it nothing. `fleet::tests::adversarial_review` |
//! | | That Bridge draws each finding under its part. Bridge's half |

use fleet::adversarial_review::{brief, findings_read, Shown};
use ipc::{FindingAnchor, SessionPlan};

const KEYS_RS: &str = "crates/store/src/keys.rs";
const KEYS: &str = "impl Keys {\n    pub fn rotate(&mut self, day: Day) -> Result<(), Error> {\n        self.insert(day)\n    }\n}\n";

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

fn later() -> Vec<String> {
    vec!["Keep the previous key readable for a day.".to_string()]
}

fn checkout(path: &str) -> Option<String> {
    (path == KEYS_RS).then(|| KEYS.to_string())
}

fn answer(findings: &str) -> String {
    format!(r#"{{"findings": [{findings}]}}"#)
}

const ON_A_TASK: &str = r#"{"about": "Write the rotation behind a lock",
    "consequence": "Two rotations can land on one day.",
    "expected": "The rotation takes a lock the store holds.",
    "found": "The plan says \"Write the rotation behind a lock\" and the store has no lock.",
    "cites": ["crates/store/src/keys.rs:2"]}"#;
const ON_A_GROUP: &str = r#"{"about": "Keep the keys",
    "consequence": "The new table ships without a migration.",
    "expected": "The standing rules say \"every table change ships with a migration file\".",
    "found": "No task writes one.",
    "cites": []}"#;
const ON_THE_PLAN: &str = r#"{"about": "",
    "consequence": "Yesterday's key is unreadable after a rotation.",
    "expected": "The old key stays readable for a day, as he asked.",
    "found": "keys.rs reads \"pub fn rotate mut self\" and keeps one key.",
    "cites": ["crates/store/src/keys.rs"]}"#;

#[test]
fn a_plan_is_shown_by_its_words_and_each_finding_is_pinned_to_the_part_it_quotes() {
    let plan = plan();
    let later = later();
    let brief = brief(&Shown { request: REQUEST, later: &later, plan: &plan, standing: STANDING });

    for shown in [
        REQUEST,
        "Keep the previous key readable for a day.",
        "Rotate the API keys nightly and keep the last two.",
        "Keep the keys",
        "Old keys stay readable for a day",
        "Add a key_rotations table with a unique day",
        "Write the rotation behind a lock",
        "Run it nightly",
        "Schedule the rotation at midnight",
        STANDING,
    ] {
        assert!(brief.contains(shown), "the brief lacks {shown:?}:\n{brief}");
    }
    for id in ["g1", "g2", "t1", "t2", "t3"] {
        assert!(!brief.contains(id), "the brief names the plan's id {id}:\n{brief}");
    }

    let said = answer(&[ON_A_TASK, ON_A_GROUP, ON_THE_PLAN].join(","));
    let findings = findings_read(&said, &brief, &plan, &checkout).expect("the answer holds up");
    let pinned: Vec<&FindingAnchor> = findings.iter().map(|one| &one.about).collect();
    assert_eq!(
        pinned,
        [&FindingAnchor::Task { id: "t2".into() }, &FindingAnchor::Group { id: "g1".into() }, &FindingAnchor::Plan]
    );
    assert_eq!(findings[0].consequence, "Two rotations can land on one day.");
    assert_eq!(findings[0].cites, ["crates/store/src/keys.rs:2"]);
    assert!(findings.iter().all(|one| one.settled.is_none()), "every finding waits for the person");
    let ids: Vec<&str> = findings.iter().map(|one| one.id.as_str()).collect();
    assert_eq!(ids, ["f1", "f2", "f3"]);
}

#[test]
fn an_answer_citing_a_file_the_checkout_lacks_or_quoting_words_never_shown_is_refused() {
    let plan = plan();
    let brief = brief(&Shown { request: REQUEST, later: &[], plan: &plan, standing: STANDING });

    let elsewhere = ON_A_TASK.replace("crates/store/src/keys.rs:2", "crates/store/src/locks.rs:9");
    let refused = findings_read(&answer(&[ON_A_GROUP, &elsewhere].join(",")), &brief, &plan, &checkout)
        .expect_err("a cite the checkout lacks");
    assert!(refused.contains("crates/store/src/locks.rs"), "the refusal names the path: {refused}");

    let invented = ON_A_TASK.replace("Write the rotation behind a lock\\\"", "the scheduler takes the lock first\\\"");
    let refused = findings_read(&answer(&invented), &brief, &plan, &checkout).expect_err("words never shown");
    assert!(refused.contains("the scheduler takes the lock first"), "the refusal names the words: {refused}");

    assert!(findings_read("I could not find anything.", &brief, &plan, &checkout).is_err(), "no JSON is no answer");
    assert_eq!(findings_read(&answer(""), &brief, &plan, &checkout), Ok(Vec::new()), "nothing found is an answer");
}
