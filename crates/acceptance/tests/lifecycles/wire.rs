//! Bridge and the agent's tools can see lifecycles, instances and rules. **What Fleet holds is
//! converted field by field to the wire's own types, crosses as the codec writes it, is served at a
//! route Bridge calls, is an agent tool for one Job's own and for nothing else, and is spelled the
//! same in TypeScript.** Reads only; nothing here saves a piece or moves an instance.

use core_model::{HeldInstances, InstanceRef, Lifecycle, Observed};
use fleet::lifecycles::wire;
use ipc::{HeldInstance, InstanceLink, InstanceStanding, JobLifecycles, LifecycleRule, RuleAct, RuleWhen, SessionId};

use crate::bench::lifecycles::{lifecycle, rules_from, taken_on, MOVES_TICKETS, MY_STORE_TICKETS};

/// The TypeScript the wire is mirrored in, read as text at build time: nothing is opened.
const MIRROR: &str = include_str!("../../../../packages/protocol/src/lifecycles.ts");

/// The Job holds a ticket in review and the pull request that names it, as the readers left them.
fn what_the_job_holds() -> (Vec<Lifecycle>, HeldInstances) {
    let tickets = lifecycle(MY_STORE_TICKETS);
    let pull_request = Lifecycle::pull_request();
    let mut held = HeldInstances::new();
    held.take_on(taken_on(&tickets, "ARM-41")).expect("held");
    held.get_mut(&InstanceRef::of(&tickets, "ARM-41").expect("a key"))
        .expect("held")
        .observe(&tickets, &Observed::Tracker("Code Review"))
        .expect("of that lifecycle");
    held.take_on(taken_on(&pull_request, "812").linked_to(InstanceRef::of(&tickets, "ARM-41").expect("a key"))).expect("held");
    (vec![tickets, pull_request], held)
}

fn crossing(lifecycles: &[Lifecycle], held: &HeldInstances) -> JobLifecycles {
    JobLifecycles {
        session_id: SessionId::carried("01JOB"),
        lifecycles: lifecycles.iter().map(wire::lifecycle).collect(),
        instances: held
            .all()
            .iter()
            .map(|one| HeldInstance {
                lifecycle: one.subject().lifecycle().as_str().to_string(),
                key: one.subject().key().to_string(),
                label: one.subject().key().to_string(),
                standing: wire::standing(one.standing()),
                links: one
                    .links()
                    .iter()
                    .map(|link| InstanceLink { lifecycle: link.lifecycle().as_str().to_string(), key: link.key().to_string() })
                    .collect(),
            })
            .collect(),
        moves: Vec::new(),
        acts: Vec::new(),
    }
}

#[test]
fn what_a_job_holds_crosses_the_wire_as_its_own_types_and_reads_back_whole() {
    let (lifecycles, held) = what_the_job_holds();
    let sent = crossing(&lifecycles, &held);
    let bytes = ipc::encode(&sent).expect("encodes");
    let read: JobLifecycles = ipc::decode("a Job's lifecycles", bytes.as_bytes()).expect("reads back");
    assert_eq!(read, sent);

    let ticket = read.instances.iter().find(|one| one.lifecycle == "ticket").expect("the ticket");
    assert_eq!(ticket.standing, InstanceStanding::In { state: "in_review".to_string() });
    let pull_request = read.instances.iter().find(|one| one.lifecycle == "pr").expect("the pull request");
    assert_eq!(pull_request.links.iter().map(|link| link.key.as_str()).collect::<Vec<_>>(), ["ARM-41"], "it knows its ticket");
    assert_eq!(pull_request.standing, InstanceStanding::Unread, "nothing has read the forge yet");

    let definition = read.lifecycles.iter().find(|one| one.id == "ticket").expect("sent with the Job");
    assert_eq!(definition.ends, ["closed"]);
    assert!(definition.states.iter().any(|state| state.id == "in_review"), "the states are the Workspace's own");
}

#[test]
fn a_job_that_holds_nothing_is_three_empty_lists_and_a_newer_field_does_not_break_an_older_reader() {
    let empty = JobLifecycles { session_id: SessionId::carried("01JOB"), lifecycles: Vec::new(), instances: Vec::new(), moves: Vec::new(), acts: Vec::new() };
    let bytes = ipc::encode(&empty).expect("encodes");
    assert!(bytes.contains(r#""lifecycles":[]"#) && bytes.contains(r#""instances":[]"#) && bytes.contains(r#""moves":[]"#), "{bytes}");

    let newer = bytes.replacen('{', r#"{"a_field_from_a_newer_fleet":1,"#, 1);
    let read: JobLifecycles = ipc::decode("a Job's lifecycles", newer.as_bytes()).expect("an older Bridge reads it");
    assert_eq!(read, empty);
}

#[test]
fn a_rule_crosses_with_its_moment_its_act_and_what_the_act_will_do_unasked() {
    let rules = rules_from(&[MOVES_TICKETS]);
    let [file] = rules.files() else { panic!("one rule: {:?}", rules.files()) };
    let sent = wire::rule(file.rule());
    let bytes = ipc::encode(&sent).expect("encodes");
    let read: LifecycleRule = ipc::decode("a rule", bytes.as_bytes()).expect("reads back");

    assert_eq!(read, sent);
    assert_eq!(read.when, RuleWhen::StepStarts { step: "plan".to_string() });
    let [doing] = read.does.as_slice() else { panic!("one act: {:?}", read.does) };
    assert!(matches!(&doing.what, RuleAct::TrackerMove { to, .. } if to == "in_progress"));
    assert_eq!(
        (doing.kind.as_str(), doing.autonomy.as_str()),
        ("tracker_write", "asks"),
        "a write to the tracker asks unless the rule says otherwise, and the wire says so"
    );
}

#[test]
fn bridge_has_all_four_reads_and_the_event_and_an_agent_has_the_jobs_own_and_no_other() {
    let served = |operation: &str| api::SERVED.iter().find(|route| route.operation == operation).map(|route| (route.method, route.path));
    assert_eq!(served("list_repository_workspaces"), Some(("GET", "/workspaces")));
    assert_eq!(served("list_lifecycles"), Some(("GET", "/lifecycles")));
    assert_eq!(served("list_lifecycle_rules"), Some(("GET", "/lifecycles/rules")));
    assert_eq!(served("get_job_lifecycles"), Some(("GET", "/sessions/lifecycles")));
    assert_eq!(served("job_lifecycles.changed"), Some(("GET", "/events")), "a Job's lifecycles ride the one stream");

    let tools: Vec<&str> = api::offered().iter().map(|shape| shape.operation).collect();
    assert!(tools.contains(&"get_job_lifecycles"), "the agent can ask what its own Job holds");
    for bridge_only in ["list_repository_workspaces", "list_lifecycles", "list_lifecycle_rules"] {
        assert!(!tools.contains(&bridge_only), "{bridge_only} is Bridge's");
    }
}

/// Every key and every tag the Rust side writes is spelled in the mirror. The ipc crate walks every
/// type it has; this walks the ones the claim names.
#[test]
fn the_typescript_mirror_spells_what_a_job_and_a_rule_cross_as() {
    let (lifecycles, held) = what_the_job_holds();
    let job = ipc::encode(&crossing(&lifecycles, &held)).expect("encodes");
    let rules = rules_from(&[MOVES_TICKETS]);
    let rule = ipc::encode(&wire::rule(rules.files()[0].rule())).expect("encodes");

    for sent in [&job, &rule] {
        let mut keys: Vec<&str> = sent.split('"').skip(1).step_by(2).filter(|word| sent.contains(&format!(r#""{word}":"#))).collect();
        keys.sort_unstable();
        keys.dedup();
        for key in keys {
            assert!(
                MIRROR.contains(&format!("{key}: ")) || MIRROR.contains(&format!("{key}?: ")),
                "`{key}` crosses the wire and the mirror declares no field of that name"
            );
        }
        for tag in ["kind", "act"] {
            for tagged in sent.split(&format!(r#""{tag}":""#)).skip(1) {
                let value = tagged.split('"').next().expect("a value");
                // A tag is a discriminant in a union; a closed set that rides a string field, as an act's `kind` does, is
                // named in the comment that lists its words.
                let named = MIRROR.contains(&format!(r#"{tag}: "{value}""#)) || MIRROR.contains(&format!("`{value}`"));
                assert!(named, "`{tag}` is `{value}` on the wire and the mirror does not say so");
            }
        }
    }
    for type_name in ["JobLifecycles", "HeldInstance", "InstanceStanding", "LifecycleDefinition", "LifecycleRule", "RuleAct", "RuleWhen"] {
        assert!(MIRROR.contains(&format!("export type {type_name} ")), "{type_name} is not mirrored");
    }
}
