//! The Started Jobs claim: **a parent Job reads all the Jobs it started in one act, stops them in one
//! act, and is refused on another Job's.** `armada-djw3` and `armada-io3f`.
//!
//! Its case is 4 Nov: a grooming Job started eleven Jobs, could not see their state without reading
//! transcripts, and stopping eight of them took eight steps by the owner.
//!
//! This crate has no store, so the claim drives the public `fleet::started_jobs` functions and
//! `ipc::StartedJob::of` on the records Fleet would have served. What needs a store, a slot or an
//! agent is `fleet`'s own tests (`tests::started_jobs`).
//!
//! Written before the code it names, and asserted in the order the parent meets it.
//!
//! | Proved | Not proved, and the group that carries it |
//! |---|---|
//! | A parent's children are the Jobs its `started_to` links name, oldest first: not a grandchild, a fork or another parent's Job | That Fleet reads the links off the caller's ledger and places the caller by its connection, and refuses a call that places none. `fleet`'s `tests::started_jobs` |
//! | Each child reads as one line: id, title, state, its workflow step and what the step waits for, what it waits on and on whom, its pull requests | That the line carries the items Needs you shows, Fleet's own among them, and the step's label from the Job's own copy of its workflow. `fleet`'s `tests::started_jobs` |
//! | An ended child reads with its end reason and what it said with `all_done`, and a live one has no result | That the summary survives the end, read from the store. `fleet`'s `tests::started_jobs` |
//! | A withdraw is ruled for a child not yet begun, and refused with a code of its own for one that has begun | That a withdrawn child ends with no agent run and its Press Start item leaves the parent's Needs you. `fleet`'s `tests::started_jobs` |
//! | An end needs a why, and a pause, a resume, a change and a drop are ruled for any child | That the child is told why, its slot goes back, its work stays on its branch and a beads issue it claimed is handed back with a note naming the parent. `fleet`'s `tests::started_jobs` |
//! | Every command is answered in order and a refused one leaves the others ruled | That a change reaches a child whichever Job it waits on, one that is ending included, and the Job waited on is told; and that a drop leaves the card held. `fleet`'s `tests::started_jobs` |
//! | A grandchild, another parent's Job and a Job that does not exist are refused with `fleet.started_not_yours` and a sentence saying so, whatever the act | That the same refusals stand for the read and the command through `armada job`. `api`'s `tests::started_jobs`, and `armada`'s `tests::job` |

// Shared with the other milestones' tests, and none of them uses all of it.
#[allow(dead_code)]
mod bench;

use fleet::started_jobs::{children_in, ruled, Child, Do, Refused, BEGUN, NEEDS_WHY, NOT_YOURS};
use ipc::{SessionState, StartBase, StartCondition, StartedAct, StartedCommand, StartedJob};

use bench::started_jobs::{
    anothers, chained, ended, grandchild, held, parent, running, ANOTHERS, CHAINED, ENDED, GRANDCHILD, HELD, PARENT, RUNNING, SUMMARY,
};

fn command(session: &str, act: StartedAct) -> StartedCommand {
    StartedCommand { session_id: ipc::SessionId::carried(session), act, why: None, condition: None, base: None }
}

fn ending(session: &str, why: &str) -> StartedCommand {
    StartedCommand { why: Some(why.into()), ..command(session, StartedAct::End) }
}

#[test]
fn a_parent_job_reads_all_the_jobs_it_started_in_one_act_stops_them_in_one_act_and_is_refused_on_another_jobs() {
    // ---- Read ----------------------------------------------------------------------------------
    let parent = parent();
    let children = children_in(&parent.attachments);
    assert_eq!(children, [HELD, CHAINED, RUNNING, ENDED], "oldest started first, and no fork, grandchild or other parent's Job");

    let lines: Vec<StartedJob> = [
        (held(), None, None),
        (chained(), None, None),
        (running(), Some("Build"), None),
        (ended(), None, Some(SUMMARY)),
    ]
    .into_iter()
    .map(|(record, step, said)| StartedJob::of(&record, step.map(str::to_string), said.map(str::to_string)))
    .collect();
    assert_eq!(lines.iter().map(|one| one.summary.id.as_str()).collect::<Vec<_>>(), children, "one line for each, in the same order");

    let [held_line, chained_line, running_line, ended_line] = &lines[..] else { unreachable!() };
    assert_eq!(held_line.summary.title.as_deref(), Some("Move the rail onto beads"));
    assert!(held_line.held && held_line.summary.state == SessionState::Live, "waiting on its start card");
    assert!(held_line.waiting_for.iter().any(|item| item.id == format!("unsent:{HELD}")), "Fleet's own item for its held first message");

    let waits = chained_line.waits.as_ref().expect("it waits on another Job");
    assert_eq!((waits.on.id.as_str(), &waits.condition, waits.base), (RUNNING, &StartCondition::PrLands, StartBase::Main), "on whom, and until when");

    let step = running_line.step.as_ref().expect("it is on a step");
    assert_eq!((step.index, step.label.as_str()), (1, "Build"));
    assert_eq!(step.waiting_on, ["Waiting on PR #41: Approved"], "what the step waits for");
    assert_eq!(running_line.summary.pull_requests.iter().map(|pull| pull.number).collect::<Vec<_>>(), [41]);
    assert_eq!(running_line.summary.branch.as_deref(), Some("armada/rail-reader"));
    assert!(running_line.waiting_for.iter().any(|item| item.id == "which-store"), "what it asks the person");
    assert!(running_line.result.is_none() && held_line.result.is_none(), "a live Job has no result yet");

    assert_eq!(ended_line.summary.state, SessionState::Ended);
    let result = ended_line.result.as_ref().expect("an ended Job has a result");
    assert_eq!((result.end_reason.as_deref(), result.summary.as_deref()), (Some("closed"), Some(SUMMARY)));

    // ---- Command -------------------------------------------------------------------------------
    // The parent's children as Fleet reads them when the command arrives.
    let family = [
        Child { id: HELD.into(), live: true, begun: false },
        Child { id: CHAINED.into(), live: true, begun: false },
        Child { id: RUNNING.into(), live: true, begun: true },
        Child { id: ENDED.into(), live: false, begun: true },
    ];
    let call = vec![
        command(HELD, StartedAct::Withdraw),
        command(RUNNING, StartedAct::Withdraw),
        ending(RUNNING, "The rail moved to beads, so this reader is not needed"),
        command(RUNNING, StartedAct::End),
        command(RUNNING, StartedAct::Pause),
        command(RUNNING, StartedAct::Resume),
        StartedCommand { condition: Some(StartCondition::Ends), ..command(CHAINED, StartedAct::Change) },
        command(CHAINED, StartedAct::Drop),
    ];
    let answers: Vec<Result<Do, Refused>> = call.iter().map(|one| ruled(&family, one)).collect();
    assert_eq!(answers.len(), call.len(), "one answer for each command");
    assert_eq!(answers[0], Ok(Do::Withdraw), "a card nobody pressed is withdrawn");
    assert_eq!(answers[1].as_ref().expect_err("it has begun").code, BEGUN, "a child that began is ended, not withdrawn");
    assert_eq!(answers[2], Ok(Do::End { why: "The rail moved to beads, so this reader is not needed".into() }));
    assert_eq!(answers[3].as_ref().expect_err("no reason given").code, NEEDS_WHY, "an end says why");
    assert_eq!(answers[4], Ok(Do::Pause));
    assert_eq!(answers[5], Ok(Do::Resume));
    assert_eq!(answers[6], Ok(Do::Change { condition: Some(StartCondition::Ends), base: None }));
    assert_eq!(answers[7], Ok(Do::Drop));
    assert_eq!(answers.iter().filter(|one| one.is_err()).count(), 2, "a refused command leaves the rest ruled");

    // ---- Refused on another Job's --------------------------------------------------------------
    // The grandchild is a Job, and so is another parent's: they are real, and not this parent's.
    assert_eq!(children_in(&running().attachments), [grandchild().id.as_str()], "the grandchild is its starter's child");
    for session in [GRANDCHILD, ANOTHERS, "01JOBNOSUCHJOB", PARENT] {
        assert_ne!(anothers().id, parent.id);
        for act in [StartedAct::Withdraw, StartedAct::Pause, StartedAct::Resume, StartedAct::Drop] {
            let refused = ruled(&family, &command(session, act)).expect_err("not the parent's");
            assert_eq!(refused.code, NOT_YOURS, "{session} {act:?}");
            assert!(refused.said.contains(session) && refused.said.contains("was not started by this job"), "{}", refused.said);
            assert!(!refused.said.contains(['-', '\u{2014}']), "no dash in {}", refused.said);
        }
        let refused = ruled(&family, &ending(session, "no longer needed")).expect_err("not the parent's");
        assert_eq!(refused.code, NOT_YOURS, "an end with a why is refused the same");
    }
}
