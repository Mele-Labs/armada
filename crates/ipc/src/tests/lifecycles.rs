//! Workspaces, lifecycles and rules, and what a Job holds, on the wire: every part round-trips,
//! what is absent leaves no key, and `packages/protocol/src/lifecycles.ts` names every field and
//! every tag Rust sends.

use std::collections::BTreeMap;

use serde_json::Value;

use crate::{
    decode, encode, AppliedLifecycle, AppliedRule, Cursor, Delivered, Event, HeldInstance, InstanceLink, InstanceMove,
    InstanceStanding, Instant, JobLifecycles, LifecycleDefinition, LifecycleRule, LifecycleStateDefinition,
    LifecycleTransition, ManifestId, ActKindDefault, AutonomyLayer, RepositoryAutonomy, PieceKept, PieceLeftOut, PieceRank, PieceScope, RepositoryLifecycleRules,
    RepositoryLifecycles, RepositoryWorkspaces, RuleAct, RuleActReading, RuleAmong, RuleCondition, RuleDoing, RuleOnFailure,
    RuleTarget, RuleWhen, SessionId, StateReadFrom, StreamMessage, WorkspaceOfRepositories,
};

const MIRROR: &str = include_str!("../../../../packages/protocol/src/lifecycles.ts");

fn entered() -> RuleTarget {
    RuleTarget { among: RuleAmong::Entered, in_state: None }
}

fn linked() -> RuleTarget {
    RuleTarget { among: RuleAmong::LinkedFrom { lifecycle: "ticket".to_string() }, in_state: Some("in_progress".to_string()) }
}

/// One of every act, so that a variant with no sample here is a variant the mirror test never sees.
fn every_act() -> Vec<RuleAct> {
    vec![
        RuleAct::TrackerAssign { to: "ana".to_string(), target: linked() },
        RuleAct::TrackerMove { to: "in_review".to_string(), target: linked() },
        RuleAct::TrackerClose { target: linked() },
        RuleAct::ForgeRequestReviewers { from: vec!["ana".to_string()], target: entered() },
        RuleAct::ForgeMarkReady { target: entered() },
        RuleAct::ForgeMerge { target: entered() },
        RuleAct::ForgeAutoMerge { target: entered() },
        RuleAct::Draft { template: "review ask".to_string(), target: entered() },
        RuleAct::StartJob { workflow: Some("bug".to_string()), prompt: "fix it".to_string() },
        RuleAct::StartWatch { recipe: "checks-on-a-commit".to_string(), values: BTreeMap::new(), deploy: None },
        RuleAct::Tell { message: "approved".to_string() },
        RuleAct::Wake,
        RuleAct::RunCommand { command: "test".to_string() },
        RuleAct::RunSkill { skill: "review".to_string() },
        RuleAct::RunDrone { prompt: "look".to_string() },
    ]
}

fn a_rule() -> LifecycleRule {
    LifecycleRule {
        id: "ticket_to_review".to_string(),
        name: "Move the ticket when its pull request opens".to_string(),
        scope: PieceScope::Within { id: "my-store".to_string() },
        when: RuleWhen::Enters { lifecycle: "pr".to_string(), state: "ready".to_string() },
        conditions: vec![
            RuleCondition::Instances { among: RuleAmong::LinkedFrom { lifecycle: "ticket".to_string() }, quantity: "every".to_string(), in_state: "in_progress".to_string() },
            RuleCondition::Workflow { workflow: "feature".to_string() },
            RuleCondition::AtStep { step: "land".to_string() },
            RuleCondition::PullRequestOpened,
        ],
        does: every_act()
            .into_iter()
            .map(|what| RuleDoing { what, kind: "tracker_write".to_string(), autonomy: "asks".to_string() })
            .collect(),
        autonomy: Some("asks".to_string()),
        on_failure: RuleOnFailure { retries: 2, then: "ask_owner".to_string() },
    }
}

fn the_other_whens() -> Vec<RuleWhen> {
    vec![
        RuleWhen::StepStarts { step: "plan".to_string() },
        RuleWhen::StepEnds { step: "plan".to_string() },
        RuleWhen::Job { event: "landed".to_string() },
    ]
}

fn the_other_amongs() -> Vec<RuleAmong> {
    vec![RuleAmong::LinkingTo { lifecycle: "pr".to_string() }, RuleAmong::Held { lifecycle: "job".to_string() }]
}

fn a_lifecycle() -> LifecycleDefinition {
    let state = |id: &str, read: StateReadFrom| LifecycleStateDefinition { id: id.to_string(), label: id.to_string(), read };
    LifecycleDefinition {
        id: "ticket".to_string(),
        name: "Ticket".to_string(),
        scope: PieceScope::Everywhere,
        states: vec![
            state("todo", StateReadFrom::Tracker { words: vec!["To Do".to_string()] }),
            state("ready", StateReadFrom::Forge { fact: "ready".to_string() }),
            state("rolling_out", StateReadFrom::Watch { state: "running".to_string() }),
            state("running", StateReadFrom::Fleet { fact: "started".to_string() }),
        ],
        transitions: vec![LifecycleTransition { from: "todo".to_string(), to: "ready".to_string() }],
        ends: vec!["ready".to_string()],
    }
}

fn kept_by_kit() -> PieceKept {
    PieceKept {
        rank: PieceRank { reach: "workspace".to_string(), source: "kit".to_string(), home: true },
        path: Some("/home/.armada/lifecycles/ticket.yml".to_string()),
        overridden_by: Some(PieceRank { reach: "repository".to_string(), source: "repository".to_string(), home: false }),
        hides: Some("repository".to_string()),
    }
}

fn a_workspace() -> WorkspaceOfRepositories {
    WorkspaceOfRepositories {
        id: "my-store".to_string(),
        name: "My store".to_string(),
        repositories: vec![ManifestId::carried("01API"), ManifestId::carried("01WEB")],
        home: Some(ManifestId::carried("01API")),
        source: "repository".to_string(),
        path: "/repos/api/.armada/workspaces/my-store.yml".to_string(),
        overridden_by: Some("kit".to_string()),
        hides: Some("repository".to_string()),
    }
}

fn left_out() -> Vec<PieceLeftOut> {
    vec![PieceLeftOut { source: "kit".to_string(), path: "/home/.armada/rules/broken.yml".to_string(), said: "no `when`".to_string() }]
}

fn a_job() -> JobLifecycles {
    let in_review = InstanceStanding::In { state: "in_review".to_string() };
    JobLifecycles {
        session_id: SessionId::carried("job-1"),
        lifecycles: vec![a_lifecycle()],
        instances: vec![
            HeldInstance {
                lifecycle: "pr".to_string(),
                key: "812".to_string(),
                label: "PR #812".to_string(),
                standing: in_review.clone(),
                links: vec![InstanceLink { lifecycle: "ticket".to_string(), key: "ARM-41".to_string() }],
                ended_at: None,
            },
            HeldInstance {
                lifecycle: "ticket".to_string(),
                key: "ARM-41".to_string(),
                label: "ARM-41".to_string(),
                standing: InstanceStanding::Unmapped { said: "Code Review".to_string() },
                links: Vec::new(),
                ended_at: None,
            },
            HeldInstance {
                lifecycle: "deploy".to_string(),
                key: "web".to_string(),
                label: "web".to_string(),
                standing: InstanceStanding::Unread,
                links: Vec::new(),
                ended_at: Some(Instant::carried("2026-10-30T09:30:00.000Z")),
            },
        ],
        moves: vec![InstanceMove {
            lifecycle: "pr".to_string(),
            key: "812".to_string(),
            from: InstanceStanding::Unread,
            to: in_review,
            source: "forge".to_string(),
            at: Instant::carried("2026-10-30T09:00:00.000Z"),
        }],
        acts: vec![
            RuleActReading {
                rule: "move-ticket".to_string(),
                text: "Move ARM-41 to In review".to_string(),
                course: "acts".to_string(),
                standing: "done".to_string(),
                said: None,
                at: Instant::carried("2026-10-30T09:00:01.000Z"),
                settled_at: Some(Instant::carried("2026-10-30T09:00:02.000Z")),
                on: Some(InstanceLink { lifecycle: "ticket".to_string(), key: "ARM-41".to_string() }),
                entered: Some(InstanceLink { lifecycle: "pr".to_string(), key: "812".to_string() }),
            },
            RuleActReading {
                rule: "ask-review".to_string(),
                text: "Ask Maya for review".to_string(),
                course: "asks".to_string(),
                standing: "stuck".to_string(),
                said: Some("The forge refused".to_string()),
                at: Instant::carried("2026-10-30T09:00:03.000Z"),
                settled_at: None,
                on: None,
                entered: None,
            },
        ],
        rules: Vec::new(),
        changes: Vec::new(),
    }
}

/// Every key of every object in `value`.
fn keys(value: &Value, into: &mut Vec<String>) {
    match value {
        Value::Object(fields) => {
            for (key, inside) in fields {
                into.push(key.clone());
                keys(inside, into);
            }
        }
        Value::Array(items) => items.iter().for_each(|item| keys(item, into)),
        _ => {}
    }
}

fn an_autonomy() -> RepositoryAutonomy {
    let default = |kind: &str, autonomy: &str| ActKindDefault { kind: kind.to_string(), autonomy: autonomy.to_string() };
    RepositoryAutonomy {
        manifest_id: ManifestId::carried("01API"),
        armada: vec![default("tracker_write", "asks"), default("start_watch", "acts")],
        layers: vec![AutonomyLayer {
            id: "my_store".to_string(),
            scope: PieceScope::Within { id: "my-store".to_string() },
            kept: kept_by_kit(),
            defaults: vec![default("tracker_write", "acts")],
        }],
        left_out: left_out(),
    }
}

fn round_trips<T>(value: &T)
where
    T: serde::Serialize + serde::de::DeserializeOwned + PartialEq + std::fmt::Debug,
{
    let spelled = encode(value).expect("plain data");
    let back: T = decode("a value", spelled.as_bytes()).expect("reads");
    assert_eq!(&back, value, "{spelled}");
}

#[test]
fn every_part_round_trips() {
    round_trips(&a_job());
    round_trips(&RepositoryLifecycles {
        manifest_id: ManifestId::carried("01API"),
        lifecycles: vec![AppliedLifecycle { lifecycle: a_lifecycle(), kept: kept_by_kit() }],
        left_out: left_out(),
    });
    round_trips(&RepositoryLifecycleRules {
        manifest_id: ManifestId::carried("01API"),
        rules: vec![AppliedRule { rule: a_rule(), kept: kept_by_kit() }],
        left_out: left_out(),
    });
    round_trips(&RepositoryWorkspaces { manifest_id: ManifestId::carried("01API"), workspaces: vec![a_workspace()], left_out: left_out() });
    round_trips(&an_autonomy());
}

/// Nothing is sent as `null`: a part that is absent leaves no key.
#[test]
fn what_is_absent_leaves_no_key() {
    let carried = AppliedLifecycle {
        lifecycle: LifecycleDefinition { scope: PieceScope::Everywhere, ..a_lifecycle() },
        kept: PieceKept { rank: PieceRank { reach: "everywhere".to_string(), source: "armada".to_string(), home: false }, path: None, overridden_by: None, hides: None },
    };
    let spelled = encode(&carried).expect("plain data");
    assert!(spelled.contains(r#""kept":{"rank":{"reach":"everywhere","source":"armada"}}"#), "{spelled}");
    assert!(!spelled.contains("null"), "{spelled}");

    let in_a_kit = WorkspaceOfRepositories { home: None, overridden_by: None, hides: None, ..a_workspace() };
    let spelled = encode(&in_a_kit).expect("plain data");
    for absent in ["home", "overridden_by", "hides"] {
        assert!(!spelled.contains(absent), "{absent}: {spelled}");
    }

    let bare = RuleTarget { among: RuleAmong::Entered, in_state: None };
    assert_eq!(encode(&bare).expect("plain data"), r#"{"among":{"kind":"entered"}}"#);
    let job = JobLifecycles { session_id: SessionId::carried("job-1"), lifecycles: Vec::new(), instances: Vec::new(), moves: Vec::new(), acts: Vec::new(), rules: Vec::new(), changes: Vec::new() };
    assert_eq!(encode(&job).expect("plain data"), r#"{"session_id":"job-1","lifecycles":[],"instances":[],"moves":[]}"#);

    let planned = RuleActReading {
        rule: "r".to_string(),
        text: "t".to_string(),
        course: "acts".to_string(),
        standing: "planned".to_string(),
        said: None,
        at: Instant::carried("2026-10-30T09:00:00.000Z"),
        settled_at: None,
        on: None,
        entered: None,
    };
    assert_eq!(encode(&planned).expect("plain data"), r#"{"rule":"r","text":"t","course":"acts","standing":"planned","at":"2026-10-30T09:00:00.000Z"}"#);
}

/// An instance the Job's end did not end leaves no `ended_at`, and one read from a Fleet older than the field has none.
#[test]
fn an_instance_the_job_did_not_end_has_no_ended_at() {
    let live = a_job().instances.remove(0);
    assert!(!encode(&live).expect("plain data").contains("ended_at"));
    let back: HeldInstance = decode("an instance", br#"{"lifecycle":"pr","key":"812","label":"PR #812","standing":{"kind":"unread"},"links":[]}"#).expect("reads");
    assert_eq!(back.ended_at, None);
    let ended = a_job().instances.remove(2);
    assert!(encode(&ended).expect("plain data").contains(r#""ended_at":"2026-10-30T09:30:00.000Z""#));
}

/// A Fleet older than `acts` answers without it, and a Bridge reads that as none.
#[test]
fn a_job_read_without_acts_has_none() {
    let spelled = r#"{"session_id":"job-1","lifecycles":[],"instances":[],"moves":[]}"#;
    let back: JobLifecycles = decode("a Job's lifecycles", spelled.as_bytes()).expect("reads");
    assert!(back.acts.is_empty());
}

#[test]
fn the_event_carries_the_jobs_lifecycles_whole_beside_its_own_kind() {
    let spelled = encode(&StreamMessage::Event(Delivered { cursor: Cursor::at(3), event: Event::JobLifecyclesChanged(a_job()) })).expect("plain data");
    assert!(spelled.contains(r#""kind":"job_lifecycles.changed""#), "{spelled}");
    assert!(spelled.contains(r#""session_id":"job-1""#) && spelled.contains(r#""label":"PR #812""#), "{spelled}");
    let back: StreamMessage = decode("a stream message", spelled.as_bytes()).expect("reads");
    assert!(matches!(back, StreamMessage::Event(Delivered { event: Event::JobLifecyclesChanged(held), .. }) if held == a_job()));
}

/// An unknown field does not break a parse, so a Fleet that sends one more thing than this Bridge knows still reads.
#[test]
fn a_field_this_build_does_not_know_does_not_break_a_read() {
    let spelled = r#"{"session_id":"job-1","lifecycles":[],"instances":[],"moves":[],"added_later":true}"#;
    let back: JobLifecycles = decode("a Job's lifecycles", spelled.as_bytes()).expect("reads");
    assert_eq!(back.session_id.as_str(), "job-1");
}

/// The mirror is written by hand, so it is read here: every key Rust sends is a field in
/// `lifecycles.ts`, and every tag a word it spells.
#[test]
fn the_typescript_mirror_names_every_field_and_every_tag_rust_sends() {
    let rule = a_rule();
    let others = the_other_whens().into_iter().map(|when| LifecycleRule { when, ..a_rule() });
    let amongs = the_other_amongs().into_iter().map(|among| RuleTarget { among, in_state: None });
    let mut sent = vec![
        serde_json::to_value(a_job()).unwrap(),
        serde_json::to_value(AppliedLifecycle { lifecycle: a_lifecycle(), kept: kept_by_kit() }).unwrap(),
        serde_json::to_value(AppliedRule { rule, kept: kept_by_kit() }).unwrap(),
        serde_json::to_value(RepositoryWorkspaces { manifest_id: ManifestId::carried("01API"), workspaces: vec![a_workspace()], left_out: left_out() }).unwrap(),
        serde_json::to_value(RepositoryLifecycles { manifest_id: ManifestId::carried("01API"), lifecycles: Vec::new(), left_out: Vec::new() }).unwrap(),
        serde_json::to_value(RepositoryLifecycleRules { manifest_id: ManifestId::carried("01API"), rules: Vec::new(), left_out: Vec::new() }).unwrap(),
        serde_json::to_value(an_autonomy()).unwrap(),
        serde_json::to_value(PieceScope::Within { id: "x".to_string() }).unwrap(),
    ];
    sent.extend(others.map(|rule| serde_json::to_value(rule).unwrap()));
    sent.extend(amongs.map(|target| serde_json::to_value(target).unwrap()));
    let mut fields = Vec::new();
    sent.iter().for_each(|value| keys(value, &mut fields));
    fields.sort();
    fields.dedup();
    assert!(fields.len() > 60, "{fields:?}");
    for field in &fields {
        assert!(
            MIRROR.contains(&format!("{field}: ")) || MIRROR.contains(&format!("{field}?: ")),
            "lifecycles.ts does not declare `{field}`"
        );
    }

    let tags = [
        "everywhere", "within", "tracker", "forge", "watch", "fleet", "entered", "linked_from", "linking_to", "held", "enters",
        "step_starts", "step_ends", "job", "instances", "workflow", "at_step", "pull_request_opened", "tracker_assign",
        "tracker_move", "tracker_close", "forge_request_reviewers", "forge_mark_ready", "forge_merge", "forge_auto_merge",
        "draft", "start_job", "start_watch", "tell", "wake", "run_command", "run_skill", "run_drone", "unread", "in",
        "unmapped",
    ];
    for tag in tags {
        assert!(MIRROR.contains(&format!("\"{tag}\"")), "lifecycles.ts does not spell the tag `{tag}`");
    }

    // The acts sampled are the acts the mirror spells, no more and no fewer.
    let sampled: Vec<String> = every_act()
        .iter()
        .map(|act| serde_json::to_value(act).unwrap()["act"].as_str().expect("tagged by act").to_string())
        .collect();
    let spelled = MIRROR.matches("{ act: \"").count();
    assert_eq!(sampled.len(), spelled, "{sampled:?}");
}

/// The step a Job is on says what it waits for on the Job's own row, and a row with nothing to say leaves the key out.
#[test]
fn a_rows_waiting_on_line_is_left_out_when_absent_and_read_back_when_there() {
    use super::session_activity::row;
    use crate::SessionRecord;

    let mut record = row(None);
    assert!(!encode(&record).expect("encodes").contains("workflow_waiting_on"));
    record.workflow_waiting_on = vec!["Waiting on PR #812: Approved".to_string(), "Waiting on ARM-41: Closed".to_string()];
    let bytes = encode(&record).expect("encodes");
    assert!(bytes.contains(r#""workflow_waiting_on":["Waiting on PR #812: Approved","Waiting on ARM-41: Closed"]"#), "{bytes}");
    let read: SessionRecord = decode("a row", bytes.as_bytes()).expect("reads back");
    assert_eq!(read.workflow_waiting_on, record.workflow_waiting_on, "in the order the step lists them");
    assert!(include_str!("../../../../packages/protocol/src/sessions.ts").contains("workflow_waiting_on?: string[];"));
}
