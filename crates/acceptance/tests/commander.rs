//! The Commander's claim: **When a Job's Drone ends with nothing handed in, or
//! a group is red with its retries spent, the Job's Commander steers it
//! without the owner. It reads the Job, then raises the cap inside its
//! ceiling, splits or restarts a task, redirects a Drone, or sends a repair
//! Drone onto the Job's branch. The owner sees in the Job's thread and on each
//! task's mark that the Commander did it, and he can write to the Commander
//! and get an answer. Where the fix is out of its reach (skip or edit a Check,
//! edit the Manifest, pass a gate, attest, merge, push), it puts one card to
//! him. The Job never advances on anything the Commander said.** #2163.

//! The repair, `#2170`, is asserted in part here and in part in `fleet`'s
//! `tests::job_repair`. Here: where a repair is aimed, what it is granted and
//! what it is held off, and that the step it puts back is gated on its work.
//! There, against a Fleet: the Drone runs in the Job's worktree, nothing is
//! cut, the move is the Commander's, and the loop bound is its only count.

//! What is asserted here is what a Commander is given and shown, without a Fleet:
//! the signers and the plan marks, the ceiling on a cap, the reach, a launch that
//! cuts every write tool, a key that is never a repository's, and a thread that
//! crosses the wire. That a call from a placed connection is refused, answered
//! and recorded as the Commander's is `api`'s `tests::commander_reach`.
//!
//! **Against a real Fleet and a scripted host**, in `fleet`'s
//! `tests::commander_conversation` and `tests::commander_door`: the owner's
//! message is a row and a turn, one sent mid-turn waits, wakes during a turn are
//! one turn, a raise lands as the Commander's inside three times, and a call
//! naming another Job is refused. **The wakes, `#2167`, are its
//! `tests::commanding_wakes`**: each of the six troubles wakes the Commander
//! once, with its row, and not on the next tick or after a restart; a fourth
//! wake on one problem with the Job unmoved is not made. **The card, `#2166`'s
//! second part, is its `tests::commander_card` and the bound's cases in
//! `tests::commanding_wakes`**: one card goes to him and outlives a Fleet
//! restart, it counts as the Job waiting on him without moving the Job, the
//! night records it as blocked, his answer wakes the Commander, and at the loop
//! bound a card goes out and a running Job keeps running. What crosses the wire
//! is asserted here, and **what the owner sees of it is Bridge's**: the thread, each
//! task's mark and the card in Now and in Your move are drawn by the Jobs surface
//! from these same values, and asserted in `apps/desktop`'s
//! `mock/job-commander.test.tsx` and `packages/surfaces/jobs`' `commander-rows.test.ts`,
//! since nothing in this workspace renders. A Fleet cannot be assembled from here without the fixture
//! that crate keeps private, and its thread is a file.

// Shared with the other milestones' tests, and none of them uses all of it.
#[allow(dead_code)]
mod bench;

use adapter_traits::{Environment, McpConfig, Model};
use adapter_traits::{Grant, Repair, Toolbelt};
use adapters::{denied_to_a_commander, door_tools, Commanding, HeadlessAgent};
use api::{offerable, offerable_to_a_commander, CommanderReach, COMMANDER_ALLOWED};
use core_model::{
    Actor, Attempt, DroneStanding, EscalationTrigger, JobStatus, PlanAuthor, Refusals, Standing,
    StepId, Stuck, TransitionReason, Via,
};
use fleet::job_repair::JobRepair;
use ipc::Breakage;
use testkit::FakeWorkProduct;

use bench::commander::{
    a_plan_four_acts_were_made_on, a_plan_recorded_by, a_step_the_commander_stopped,
    failed_in_preparation, status_moves_received, stopped_by_fleet_and_resumed_by_the_commander, stopped_on_the_fix,
};
use bench::overview::{detail_of, round_trip_detail, section_of};
use bench::plan::PLAN;
use bench::{a_fix_diff, Bench};

fn a_bench() -> Bench {
    Bench::with(FakeWorkProduct::untouched())
}

#[test]
fn a_commanders_move_on_a_job_reaches_a_bridge_as_the_commanders() {
    let bench = a_bench();
    stopped_by_fleet_and_resumed_by_the_commander(&bench);

    let received = status_moves_received(&bench);

    let signers: Vec<&str> = received.iter().map(|moved| moved.actor.as_wire()).collect();
    assert_eq!(
        signers,
        vec!["human", "fleet", "fleet", "commander"],
        "approved by a person, run and stopped by Fleet, resumed by the Commander"
    );
    let resumed = received.last().expect("the Commander moved the Job");
    assert_eq!(
        (resumed.from.domain(), resumed.to.domain()),
        (JobStatus::Escalated, JobStatus::Running)
    );
}

#[test]
fn a_step_the_commander_stopped_reads_as_the_commanders_beside_fleets_own() {
    let bench = a_bench();
    let mut run = bench.created("raise the bound");
    bench.approved_and_dispatched(&mut run);
    let step = bench.step(0);

    let (moved, bytes) = a_step_the_commander_stopped(&bench, &mut run, &step);

    assert_eq!(moved.actor.as_wire(), "commander");
    assert!(
        bytes.contains(r#""actor":"commander""#),
        "the spelling a client dispatches on is on the wire: {bytes}"
    );
    assert_eq!(
        bench::arc::step_signers(&bench),
        vec![Actor::Fleet, Actor::Commander],
        "the dispatch was Fleet's, the stop the Commander's"
    );
}

#[test]
fn a_plan_the_commander_recorded_is_told_apart_from_a_steps_and_a_persons() {
    let by_commander = a_plan_recorded_by(PlanAuthor::Commander);
    let by_person = a_plan_recorded_by(PlanAuthor::Person);
    let by_step = a_plan_recorded_by(PlanAuthor::Step {
        step_id: StepId::new(PLAN),
        attempt: Attempt::stored(1).expect("one-based"),
    });

    assert_eq!(by_commander.0.recorded_by, ipc::ChangedBy::Commander);
    assert!(
        by_commander.1.contains(r#""by":"commander""#),
        "{}",
        by_commander.1
    );
    assert_eq!(by_person.0.recorded_by, ipc::ChangedBy::Person);
    assert!(matches!(
        by_step.0.recorded_by,
        ipc::ChangedBy::Step { ref step_id, attempt: 1 } if step_id.as_str() == PLAN
    ));
}

#[test]
fn the_commander_is_one_more_signer_and_one_more_door_and_not_a_person() {
    let signers: Vec<&str> = Actor::ALL.iter().map(Actor::as_wire).collect();
    let mut distinct = signers.clone();
    distinct.sort_unstable();
    distinct.dedup();
    assert_eq!(distinct.len(), signers.len(), "no two signers share a spelling");
    assert_eq!(Actor::from_wire("commander"), Some(Actor::Commander));

    assert_eq!(Via::from_wire("commander"), Some(Via::Commander));
    assert!(!Via::Commander.is_a_person());
    assert_ne!(Via::Commander, Via::Helm, "Helm's door is not the Commander's");
}

/// Every act the claim says a Commander never takes. All are offered by the
/// door to Helm, so a refusal here is the allowlist's and not an absence.
const NEVER_THE_COMMANDERS: &[&str] = &[
    // Pass a gate.
    "override_verdict",
    "rerun_gate",
    "rerun_failed_checks",
    "set_when_blocked",
    "submit_for_verification",
    "approve_dispatch",
    // Attest, merge, push.
    "attest_complete",
    "merge_pull_request",
    "merge_pull_request_by_number",
    "enable_auto_merge",
    "enable_job_auto_merge",
    "delete_branch",
    // A Check, the Manifest.
    "edit_manifest",
    "write_manifest_proposal",
    "save_manifest_file",
];

#[test]
fn a_commander_is_offered_every_read_and_its_own_acts_and_nothing_that_passes_a_gate() {
    let reach = CommanderReach::of_the_allowlist(ipc::JobId::carried("01JOB14"));

    let mut granted: Vec<&str> = offerable_to_a_commander()
        .filter(|row| row.kind == "command" && reach.may(row.operation))
        .map(|row| row.operation)
        .collect();
    granted.sort_unstable();
    let mut settled = vec![
        "raise_turn_cap",
        "raise_cost_cap",
        "redirect_drone",
        "redirect_one_drone",
        "restart_step",
        "restart_task",
        "edit_task",
        "add_task",
        "drop_task",
        "move_plan",
        // The one way it reaches the owner: a card that outlives its turn.
        "ask_the_owner",
        // #2170. A repair of its own, and never `add_job_step`.
        "send_repair_drone",
    ];
    settled.sort_unstable();
    assert_eq!(granted, settled, "the acts a Commander may take, and no more");
    assert_eq!(COMMANDER_ALLOWED.len(), settled.len());

    for row in offerable().filter(|row| row.kind == "query") {
        assert!(reach.may(row.operation), "every read is its: {}", row.operation);
    }
    assert!(
        !offerable().any(|row| row.operation == "ask_the_owner"),
        "the card is offered to a Commander and to no other caller, Helm included"
    );
    assert!(
        !offerable_to_a_commander().any(|row| row.operation == "answer_commander_card"),
        "and his pick is offered to no agent"
    );
    for operation in NEVER_THE_COMMANDERS {
        assert!(
            offerable().any(|row| row.operation == *operation),
            "`{operation}` is offered to someone, so refusing it is the allowlist's doing"
        );
        assert!(!reach.may(operation), "`{operation}` is not a Commander's");
    }
}

/// #2165. The ceiling is a multiple of what the workflow would inherit, one
/// for each raiser, and a raise has no field to say which it is.
#[test]
fn a_cap_a_commander_raises_stops_at_three_times_the_workflows_and_names_no_raiser() {
    let inherited = 300;
    let most = |raiser| fleet::raising::Ceiling::on(raiser, inherited).most();
    assert_eq!(most(ipc::RaisedBy::Commander), Some(900), "three times");
    assert_eq!(most(ipc::RaisedBy::Helm), Some(600), "Helm keeps two");
    assert_eq!(most(ipc::RaisedBy::Person), None, "a person has none");
    assert!(fleet::raising::Ceiling::on(ipc::RaisedBy::Commander, inherited).admits(900));
    assert!(!fleet::raising::Ceiling::on(ipc::RaisedBy::Commander, inherited).admits(901));

    // A body that says it is a person's decodes to a raise that says nothing
    // of the kind: Fleet places the raiser, and nothing in the bytes can.
    let claimed = br#"{"turn_cap":900,"raised_by":"person"}"#;
    let raise: ipc::TurnRaise = ipc::decode("a turn cap", claimed).expect("decodes");
    let carried = ipc::encode(&raise).expect("encodes");
    assert!(!carried.contains("raised_by"), "{carried}");
}

/// #2168. The same four acts, made by each of the three callers a plan has: a
/// task each one touched wears its hand on the wire and every event names it,
/// and the task none of them touched wears nobody's.
#[test]
fn a_plan_act_is_signed_by_whoever_made_it_and_the_owners_is_not_the_commanders() {
    let callers = [
        (PlanAuthor::Person, Actor::Human, "person"),
        (PlanAuthor::Helm, Actor::Helm, "helm"),
        (PlanAuthor::Commander, Actor::Commander, "commander"),
    ];
    for (author, actor, spelled) in callers {
        let (plan, bytes, told) = a_plan_four_acts_were_made_on(author, actor);

        let mark = |id: &str| {
            plan.tasks
                .iter()
                .find(|task| task.id == id)
                .map(|task| task.by)
                .unwrap_or_else(|| panic!("the plan holds {id}"))
        };
        let theirs = Some(match spelled {
            "person" => ipc::TaskBy::Person,
            "helm" => ipc::TaskBy::Helm,
            _ => ipc::TaskBy::Commander,
        });
        assert_eq!(mark("T4"), theirs, "{spelled}: the task it added and moved");
        assert_eq!(mark("T1"), theirs, "{spelled}: the task it edited");
        assert_eq!(mark("T2"), theirs, "{spelled}: the task it dropped");
        assert_eq!(mark("T3"), None, "{spelled}: nobody touched it, a step wrote it");
        assert!(
            bytes.contains(&format!(r#""by":{{"who":"{spelled}"}}"#)),
            "the spelling a client dispatches on is on the wire: {bytes}"
        );
        let signers: Vec<&str> = told.iter().map(|moved| moved.actor.as_wire()).collect();
        assert_eq!(signers, vec![actor.as_wire(); 4], "{spelled}: four acts, four events");
    }
}

/// #2166. A Commander has no owner to ask, so a write tool is not there to be
/// asked about: its launch holds the door alone, the three reads, and every
/// other built-in denied by name.
#[test]
fn a_commanders_launch_is_in_the_jobs_worktree_with_every_write_tool_cut() {
    let launch = HeadlessAgent::at("/usr/local/bin/agent")
        .render_commander(
            &Commanding::in_worktree(
                "/repos/armada/.armada/worktrees/14-bound-the-reader",
                Model::named("a-model").expect("a model"),
                McpConfig::only_these("/var/armada/helm-mcp.json").expect("a path"),
                Environment::nothing(),
            )
            .expect("an absolute worktree"),
        )
        .expect("renders");
    let args = launch.args();
    let after = |flag: &str| {
        args.iter()
            .position(|arg| arg == flag)
            .and_then(|at| args.get(at + 1))
            .cloned()
    };

    assert_eq!(
        launch.directory(),
        "/repos/armada/.armada/worktrees/14-bound-the-reader"
    );
    assert!(args.iter().any(|arg| arg == "--strict-mcp-config"));
    assert_eq!(after("--tools").as_deref(), Some("Read,Grep,Glob"));
    assert_eq!(after("--permission-mode").as_deref(), Some("dontAsk"));
    assert!(!args.iter().any(|arg| arg == "--permission-prompt-tool"));
    let allowed = after("--allowedTools").expect("an allowlist");
    assert_eq!(allowed, format!("Read,Grep,Glob,{}", door_tools()));
    for write in ["Edit", "Write", "NotebookEdit", "Bash"] {
        assert!(denied_to_a_commander().contains(&write), "{write}");
        assert!(!allowed.contains(write), "{write}");
    }
}

/// #2166. One conversation per Job, and its key is prefixed so it can never be another's, whatever
/// the other is called. A repository's Helm used to be a conversation here too; it is a hosted
/// Session now (`docs/concepts/helm.md`) and keeps no key in this table.
#[test]
fn a_jobs_conversation_can_never_be_a_repositorys() {
    let job = core_model::JobId::carried(core_model::Ulid::carried("01JOB14"));
    let of_job = fleet::helm::ConversationKey::of_job(&job);
    assert!(of_job.as_str().starts_with("commander:"), "{}", of_job.as_str());
    for other in ["01JOB14", "01JOB15", "commander:01JOB14"] {
        let other = fleet::helm::ConversationKey::of_job(&core_model::JobId::carried(
            core_model::Ulid::carried(other),
        ));
        assert_eq!(
            of_job == other,
            other.as_str() == of_job.as_str(),
            "one key per Job: {}",
            other.as_str()
        );
    }
}

/// #2166. A Commander's reach is one Job's: the process Fleet started for a Job
/// is a reach that names it, and a reach cannot be made without one.
#[test]
fn a_commanders_reach_is_one_jobs() {
    let reach = CommanderReach::of_the_allowlist(ipc::JobId::carried("01JOB14"));
    assert_eq!(reach.job().as_str(), "01JOB14");
    assert_ne!(
        reach,
        CommanderReach::of_the_allowlist(ipc::JobId::carried("01JOB15")),
        "the same allowlist, a different Job, is a different reach"
    );
}

/// #2166. What the owner says and what the Commander does crosses the wire as a
/// thread: his words and a turn's rows, tagged as a client dispatches on them,
/// and his blank message does not decode.
#[test]
fn the_owners_words_and_a_turns_rows_cross_as_a_thread() {
    let asked = ipc::CommanderMessage::Asked(ipc::HelmAsked {
        ts: ipc::Instant::carried("2026-10-12T09:00:00.000Z"),
        text: "why did step one stop".to_string(),
        files: Vec::new(),
        tags: Vec::new(),
    });
    let woke = ipc::CommanderMessage::Woke(ipc::CommanderWoke {
        ts: ipc::Instant::carried("2026-10-12T09:00:01.000Z"),
        why: "a Drone ended with nothing handed in".to_string(),
    });
    let said = ipc::encode(&asked).expect("encodes");
    assert!(said.contains(r#""message":"asked""#), "{said}");
    assert!(
        ipc::encode(&woke).expect("encodes").contains(r#""message":"woke""#),
        "the one message only this socket has"
    );
    let back: ipc::CommanderMessage = ipc::decode("a thread line", said.as_bytes()).expect("reads");
    assert_eq!(back, asked);

    assert!(
        ipc::decode::<ipc::AskCommander>("words", br#"{"text":"   "}"#).is_err(),
        "an empty message is refused before a turn is spent on it"
    );
    let changed = ipc::encode(&ipc::CommanderChanged {
        job_id: ipc::JobId::carried("01JOB14"),
        state: ipc::CommanderState::Working,
    })
    .expect("encodes");
    assert_eq!(changed, r#"{"job_id":"01JOB14","state":"working"}"#);
}

/// #2166. The card is a closed set he picks from: a line with something in it,
/// two to four labels that differ, and an answer that names an option and
/// carries no prose.
#[test]
fn a_card_crosses_the_wire_as_a_closed_set_and_a_crowded_or_blank_one_does_not_decode() {
    let card = br#"{"text":"Group 3 is red on desktop_test","reach":"A Check is the owner's to skip","question":"Skip it?","options":["Skip desktop_test","Leave it"]}"#;
    let asked: ipc::AskTheOwner = ipc::decode("a card", card).expect("decodes");
    assert_eq!(asked.options.labels(), ["Skip desktop_test", "Leave it"]);

    for refused in [
        br#"{"text":"a","reach":"b","question":"c","options":["only"]}"#.as_slice(),
        br#"{"text":"a","reach":"b","question":"c","options":["same","same"]}"#.as_slice(),
        br#"{"text":"a","reach":"b","question":"c","options":["1","2","3","4","5"]}"#.as_slice(),
        br#"{"text":"a","reach":"b","question":"c","options":["x",""]}"#.as_slice(),
        br#"{"text":"  ","reach":"b","question":"c","options":["x","y"]}"#.as_slice(),
        br#"{"reach":"b","question":"c","options":["x","y"]}"#.as_slice(),
    ] {
        assert!(
            ipc::decode::<ipc::AskTheOwner>("a card", refused).is_err(),
            "{}",
            String::from_utf8_lossy(refused)
        );
    }

    let out = ipc::CommanderCard {
        card_id: ipc::CardId::carried("01CARD"),
        job_id: ipc::JobId::carried("01JOB14"),
        by: ipc::CardBy::Fleet,
        raised_at: ipc::Instant::carried("2026-10-13T09:00:00.000Z"),
        text: asked.text.as_str().to_string(),
        reach: asked.reach.as_str().to_string(),
        question: asked.question.as_str().to_string(),
        options: asked
            .options
            .labels()
            .iter()
            .enumerate()
            .map(|(at, label)| ipc::CardOption {
                id: (at + 1).to_string(),
                label: label.clone(),
            })
            .collect(),
    };
    let going = ipc::encode(&ipc::Event::CommanderCard(ipc::CommanderCardChanged {
        job_id: ipc::JobId::carried("01JOB14"),
        card: Some(out),
    }))
    .expect("encodes");
    assert!(going.contains(r#""kind":"commander.card""#), "{going}");
    assert!(going.contains(r#""by":"fleet""#), "{going}");
    assert!(going.contains(r#""options":[{"id":"1","label":"Skip desktop_test"},{"id":"2","label":"Leave it"}]"#), "{going}");

    let answered = ipc::encode(&ipc::Event::CommanderCard(ipc::CommanderCardChanged {
        job_id: ipc::JobId::carried("01JOB14"),
        card: None,
    }))
    .expect("encodes");
    assert_eq!(
        answered,
        r#"{"kind":"commander.card","job_id":"01JOB14"}"#,
        "the second message says the card is gone by carrying none, as job.asking does"
    );

    let pick = ipc::encode(&ipc::AnswerCommanderCard {
        card_id: ipc::CardId::carried("01CARD"),
        option_id: "2".to_string(),
    })
    .expect("encodes");
    assert_eq!(pick, r#"{"card_id":"01CARD","option_id":"2"}"#, "an id and an id, no prose");
}

/// #2166. A Job's own Commander model is chosen with both halves stated, so a
/// client that dropped one cannot clear it by accident.
#[test]
fn a_job_names_its_commanders_model_and_effort_with_both_keys_present() {
    let both: ipc::SetCommanderModel =
        ipc::decode("a choice", br#"{"model":"a-model","effort":"low"}"#).expect("decodes");
    assert_eq!(
        (both.model.as_deref(), both.effort.as_deref()),
        (Some("a-model"), Some("low"))
    );
    let cleared: ipc::SetCommanderModel =
        ipc::decode("a choice", br#"{"model":null,"effort":null}"#).expect("decodes");
    assert_eq!((cleared.model, cleared.effort), (None, None));
    for dropped in [br#"{"model":"a-model"}"#.as_slice(), br#"{"effort":"low"}"#.as_slice()] {
        assert!(
            ipc::decode::<ipc::SetCommanderModel>("a choice", dropped).is_err(),
            "{}",
            String::from_utf8_lossy(dropped)
        );
    }
}

/// #2169. A Job with a card out reaches a Bridge as one waiting on the owner,
/// with the card itself beside it: the row that puts it among those Your move
/// lists, and the detail the card is drawn from.
#[test]
fn a_job_with_a_card_out_reaches_a_bridge_waiting_on_the_owner_with_the_card_beside_it() {
    let bench = a_bench();
    let mut run = bench.created("steer the red group");
    bench.approved_and_dispatched(&mut run);
    let mut detail = detail_of(&run.job, None);
    assert_eq!(section_of(&detail.job), Some("running"), "a running Job is not waiting before the card");

    detail.job.asking = true;
    detail.commander_card = Some(ipc::CommanderCard {
        card_id: ipc::CardId::carried("01CARD"),
        job_id: detail.job.id.clone(),
        by: ipc::CardBy::Commander,
        raised_at: ipc::Instant::carried("2026-10-13T09:00:00.000Z"),
        text: "Group 3 is red on desktop_test".to_string(),
        reach: "A Check is skipped or edited by you".to_string(),
        question: "What should Group 3 do?".to_string(),
        options: vec![
            ipc::CardOption { id: "1".to_string(), label: "Skip desktop_test".to_string() },
            ipc::CardOption { id: "2".to_string(), label: "Leave it".to_string() },
        ],
    });

    let received = round_trip_detail(&detail);
    assert_eq!(section_of(&received.job), Some("needs-you"), "the row Your move reads");
    let card = received.commander_card.expect("the card is served on its Job");
    assert_eq!(card.question, "What should Group 3 do?");
    assert_eq!(
        card.options.iter().map(|one| one.id.as_str()).collect::<Vec<_>>(),
        ["1", "2"],
        "an answer names the card and one of these"
    );
    assert_eq!(received.job.status, detail.job.status, "no status moved");
}

/// #2170. A repair has no branch to name, so it is aimed at the Job's own
/// worktree and branch and at the step that stopped, and at nothing beside them.
#[tokio::test]
async fn a_repair_drone_is_aimed_at_the_jobs_own_branch_and_the_step_that_stopped() {
    let bench = a_bench();
    let run = stopped_on_the_fix(&bench).await;

    let repair = JobRepair::onto(&run.job, run.worktree.clone(), Breakage::MergeConflict)
        .expect("a Job with a stopped step has one to put back");

    assert_eq!(repair.worktree(), &run.worktree, "the Job's own worktree");
    assert_eq!(repair.worktree().branch(), run.worktree.branch());
    assert_eq!(repair.restarts(), &bench.step(1), "the step it stopped, not another");
    assert!(
        bench.vcs.created().len() == 1,
        "the Job's worktree is the only one made: nothing is cut for a repair"
    );
}

/// #2170. Its work is the step's: the step it puts back is gated on a diff that
/// holds nothing but the repair's file, and with no file written it fails.
#[tokio::test]
async fn a_repairs_work_reaches_the_gate_of_the_step_it_puts_back() {
    for (written, advances) in [(true, true), (false, false)] {
        let work = match written {
            true => FakeWorkProduct::changed(&["src/parse.rs"]),
            false => FakeWorkProduct::untouched(),
        };
        let bench = Bench::with(work);
        let run = stopped_on_the_fix(&bench).await;
        let repair = JobRepair::onto(&run.job, run.worktree.clone(), Breakage::Dependencies)
            .expect("a step to put back");

        let ruling = bench.gate(&run, repair.restarts(), &a_fix_diff()).await;

        assert_eq!(
            ruling.advanced(),
            advances,
            "the Job's own Check ruled on the step with the repair's file written: {written}"
        );
    }
}

/// #2170. What a repair is granted is the fault's, and a Check or the Manifest
/// is never among it, wherever the repository declares one.
#[tokio::test]
async fn a_repair_is_granted_what_the_fault_needs_and_held_off_every_manifest() {
    let bench = a_bench();
    let run = stopped_on_the_fix(&bench).await;
    let bootstrap = vec![String::from("pnpm install")];
    let manifests = vec![String::from("packages/a/armada.yml")];
    let belt = |broke| {
        JobRepair::onto(&run.job, run.worktree.clone(), broke)
            .expect("a step to put back")
            .belted(Toolbelt::evidence_only(), &bootstrap, &manifests)
    };

    let conflict = belt(Breakage::MergeConflict);
    assert_eq!(
        conflict.granted(),
        [Grant::RepairTheWorktree(Repair::TheIndex)],
        "a conflict is staged, and nothing is run"
    );
    let missing = belt(Breakage::MissingBinary);
    assert_eq!(
        missing.granted(),
        [Grant::RepairTheWorktree(Repair::TheInstall(String::from("pnpm install")))]
    );
    for belt in [&conflict, &missing] {
        assert_eq!(
            belt.held_off(),
            ["armada.yml", ".armada", "packages/a/armada.yml"],
            "every place a Check is declared"
        );
        assert!(
            !belt.granted().iter().any(|grant| matches!(
                grant,
                Grant::DispatchAJob | Grant::RecordThePlan | Grant::WorkThePlan
            )),
            "no dispatch and no plan"
        );
    }
}

/// #2170. A Job that broke before its first step is repaired like one that
/// stopped on a step, and put back at the first: the repair is aimed at it, in
/// the Job's own worktree, and the Commander's queueing of the Job is a move the
/// machine admits and a Bridge receives as the Commander's. That the first
/// step's gate sees the repair's file is `fleet`'s `tests::job_repair_early`,
/// which has the Drone and the baseline.
#[test]
fn a_repair_of_a_job_that_broke_in_preparation_is_aimed_at_the_first_step_and_queues_the_job() {
    let bench = a_bench();
    let mut run = failed_in_preparation(&bench, EscalationTrigger::NotPrepared);

    let repair = JobRepair::onto(&run.job, run.worktree.clone(), Breakage::MergeConflict)
        .expect("a first step to begin");
    assert_eq!(repair.restarts(), &bench.step(0), "the first, since none had begun");
    assert_eq!(repair.worktree(), &run.worktree, "the Job's own worktree");

    bench.moved(&mut run, core_model::Target::Queued, Actor::Commander);
    let last = status_moves_received(&bench).pop().expect("a move");
    assert_eq!(
        (last.to.as_wire(), last.actor.as_wire()),
        ("queued", "commander"),
        "back in the queue, signed by the Commander"
    );
}

/// #2170. Of the Jobs that never had a Drone, only a failed preparation is a
/// repair's. The rest are the Manifest's or the machine's, and the Drone cannot
/// edit the one or add to the other.
#[test]
fn only_a_failed_preparation_is_a_repair_among_the_jobs_that_never_had_a_drone() {
    let standing = Standing {
        drone: DroneStanding::Gone,
        worktree_on_disk: true,
        checks_passed: true,
        workflow_held: true,
        checks_rerunning: false,
    };
    for (trigger, repairable) in [
        (EscalationTrigger::NotPrepared, true),
        (EscalationTrigger::NotConfigurable, false),
        (EscalationTrigger::WouldNotStart, false),
        (EscalationTrigger::ResourceExhausted, false),
    ] {
        let bench = a_bench();
        let run = failed_in_preparation(&bench, trigger);
        let reason = TransitionReason::Escalation(trigger);

        let stuck = Stuck::of(&run.job, Some(&reason), standing, Refusals::none(), None)
            .expect("an escalated Job is classified");

        assert_eq!(stuck.unprepared(), repairable, "{}", trigger.as_wire());
    }
}
