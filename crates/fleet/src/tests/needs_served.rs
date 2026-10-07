//! `armada need` through a real Fleet, and the rest of what the ledger holds
//! for needs: a terminal's declaration holds a Job behind it, the route answers
//! who is ahead and refuses what it cannot keep, a clone's old files are carried
//! in once, and a Job's own slot and branch are rows. The Job and session order
//! is `needs`.

use adapter_traits::Landing;
use adapters::needs::Needs;
use api::{Needs as _, Sessions as _};
use ipc::{NeedAct, NeedCall};
use store::{AttachmentState, Holder};

use super::needs::{at_the_gate, git, holding_under, line, migration, PATH};
use crate::tests::tmp::TempDir;

fn call(act: NeedAct, branch: &str, what: Option<&str>, value: Option<&str>) -> NeedCall {
    NeedCall {
        act,
        manifest_id: None,
        branch: branch.to_string(),
        path: PATH.to_string(),
        what: what.map(str::to_string),
        value: value.map(str::to_string),
    }
}

/// `armada need` from a terminal: Fleet finds who holds the branch. A branch no
/// Job or session stands on is its own holder, and is given back when git no
/// longer has it.
#[tokio::test]
async fn a_terminals_need_holds_a_job_behind_it_until_it_is_given_back_or_its_branch_is_gone() {
    let home = TempDir::new();
    let fleet = holding_under(&home, "forge");
    let (job, _) = at_the_gate(&fleet, &home).await;
    git(home.path(), &["branch", "terminal"]);

    let told = fleet
        .act_on_need(call(
            NeedAct::Declare,
            "terminal",
            Some("a new migration"),
            None,
        ))
        .await
        .expect("declared");
    assert!(!told.already && told.ahead.is_empty());
    fleet
        .act_on_need(call(NeedAct::Took, "terminal", None, Some("V96")))
        .await
        .expect("took");
    fleet.needs_declared(&job, &migration(None)).await;
    fleet
        .merge_pull_request(&job)
        .await
        .expect_err("the Job waits behind the terminal");

    // Given back, the Job is free.
    let gave = fleet
        .act_on_need(call(NeedAct::Release, "terminal", None, None))
        .await
        .expect("released");
    assert!(gave.gave_back);
    fleet.vcs().now_landed(Landing::Merged {
        url: String::from(crate::tests::merging::PULL_REQUEST),
    });
    fleet.merge_pull_request(&job).await.expect("free");

    // A branch that is deleted gives its need back without anybody saying so.
    git(home.path(), &["branch", "doomed"]);
    fleet
        .act_on_need(call(
            NeedAct::Declare,
            "doomed",
            Some("a new migration"),
            None,
        ))
        .await
        .expect("declared");
    assert_eq!(fleet.list_needs(None).await.expect("list").needs.len(), 1);
    git(home.path(), &["branch", "-D", "doomed"]);
    assert!(fleet.list_needs(None).await.expect("list").needs.is_empty());
}

/// What the route answers: the order, what was taken, a repeat, and the two
/// refusals a person can cause.
#[tokio::test]
async fn the_route_answers_who_is_ahead_and_refuses_what_it_cannot_keep() {
    let home = TempDir::new();
    let fleet = holding_under(&home, "forge");
    let (job, job_branch) = at_the_gate(&fleet, &home).await;
    git(home.path(), &["branch", "terminal"]);

    // A Job's own branch declares as the Job.
    let one = fleet
        .act_on_need(call(NeedAct::Declare, &job_branch, Some("a minor"), None))
        .await
        .expect("declared");
    let holder = one.mine.expect("its need").holder;
    assert_eq!(
        (holder.kind, holder.id.as_str()),
        (ipc::HolderKind::Job, job.as_str())
    );
    fleet
        .act_on_need(call(NeedAct::Took, &job_branch, None, Some("23.41")))
        .await
        .expect("took");

    let two = fleet
        .act_on_need(call(NeedAct::Declare, "terminal", Some("a minor"), None))
        .await
        .expect("declared");
    assert_eq!(two.ahead.len(), 1);
    assert_eq!(two.ahead[0].took.as_deref(), Some("23.41"));
    assert_eq!(two.ahead[0].held_by, job_branch);
    let again = fleet
        .act_on_need(call(NeedAct::Declare, "terminal", Some("a minor"), None))
        .await
        .expect("declared");
    assert!(again.already, "a repeat records nothing");

    let nothing = fleet
        .act_on_need(call(NeedAct::Release, "never-declared", None, None))
        .await
        .expect("a release with nothing to give back is an answer");
    assert!(!nothing.gave_back);

    let refused = fleet
        .act_on_need(call(NeedAct::Took, "never-declared", None, Some("1")))
        .await
        .expect_err("it never declared");
    match refused {
        api::Refusal::Unacceptable(wire) => {
            assert_eq!(wire.code, "fleet.need_not_declared");
            assert!(wire.message.contains("armada need"), "{}", wire.message);
        }
        other => panic!("a 422: {other:?}"),
    }
    let blank = fleet
        .act_on_need(call(NeedAct::Declare, "terminal", Some("  "), None))
        .await
        .expect_err("a need says what");
    assert!(matches!(blank, api::Refusal::Unacceptable(_)));
}

/// A clone's files are carried in once: the branch of a live Job resolves to the
/// Job, any other to the branch alone, the order a clone had is kept, the files
/// stay on disk, and a need given back is not brought back by the next start.
#[tokio::test]
async fn a_clones_files_are_carried_into_the_ledger_once() {
    let home = TempDir::new();
    let fleet = holding_under(&home, "forge");
    let (job, job_branch) = at_the_gate(&fleet, &home).await;
    git(home.path(), &["branch", "terminal"]);
    let files = Needs::of(home.path()).expect("the clone's files");
    files
        .declare("terminal", PATH, "a new migration")
        .expect("declared");
    files.took("terminal", PATH, "V96").expect("took");
    files
        .declare(&job_branch, PATH, "a new migration")
        .expect("declared");

    let served = fleet.served_named(None).expect("served");
    let jobs = vec![fleet.load(&job).await.expect("the Job")];
    fleet.needs_converted(&served, &jobs).await;

    assert_eq!(
        line(&fleet, PATH).await,
        [ledger_branch("terminal"), Holder::job(job.as_str())],
        "the order the clone had, the Job resolved from its branch"
    );
    assert_eq!(files.files().len(), 2, "nothing is deleted from disk");

    // Given back, it stays given back across a second start.
    fleet
        .act_on_need(call(NeedAct::Release, "terminal", None, None))
        .await
        .expect("released");
    fleet.needs_converted(&served, &jobs).await;
    assert_eq!(line(&fleet, PATH).await, [Holder::job(job.as_str())]);
}

fn ledger_branch(branch: &str) -> Holder {
    crate::needing::ledger::branch_holder(branch)
}

/// A Job's own branch and slot are rows, so `who_owns` names the Job that holds
/// them, and what it held is settled when it ends.
#[tokio::test]
async fn a_job_holds_its_branch_and_slot_on_the_ledger_until_it_ends() {
    let home = TempDir::new();
    let fleet = holding_under(&home, "forge");
    let (job, branch) = at_the_gate(&fleet, &home).await;

    let owners = fleet
        .who_owns("branch".into(), branch.clone(), None)
        .await
        .expect("who owns");
    assert_eq!(owners.holders.len(), 1, "{owners:?}");
    assert_eq!(owners.holders[0].holder.kind, ipc::HolderKind::Job);
    assert_eq!(owners.holders[0].holder.id, job.as_str());
    let held = fleet
        .store()
        .lock()
        .await
        .attachments_of(&Holder::job(job.as_str()))
        .expect("rows");
    assert!(
        held.iter()
            .any(|row| row.kind == "slot" && row.state == AttachmentState::Standing),
        "{held:?}"
    );

    fleet.vcs().now_landed(Landing::Merged {
        url: String::from(crate::tests::merging::PULL_REQUEST),
    });
    fleet.merge_pull_request(&job).await.expect("lands");
    let held = fleet
        .store()
        .lock()
        .await
        .attachments_of(&Holder::job(job.as_str()))
        .expect("rows");
    let branch_row = held
        .iter()
        .find(|row| row.kind == "branch")
        .expect("a branch row");
    assert_eq!(branch_row.state, AttachmentState::Spent, "{held:?}");
}
