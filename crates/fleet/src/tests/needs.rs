//! The claim of #1059, Fleet's half, now on the session ledger: two Jobs on one
//! repository each declare a need on the file that lists migrations; the second
//! is told the first is ahead and what it took; the second, ready first, is
//! refused at its merge naming what it waits behind; once the first lands, its
//! need is spent and the second lands; a dropped Job's need is given back. A Job,
//! a session that declared through the intake and a terminal's `armada need`
//! stand in ONE order, because they are rows of one table. A clone's old files
//! are carried in once, and a Job's own slot and branch are rows too.
//!
//! **A real repository under the fixture's home**, because a need's holder is
//! found by a branch and a branch is what git says exists. What git does is
//! asserted in `adapters`; what is under test here is what Fleet does with the
//! answer.

use std::process::Command;
use std::time::Duration;

use adapter_traits::Landing;
use adapters::needs::Needs;
use api::{Needs as _, Sessions as _};
use config::Manifest;
use core_model::{JobId, JobStatus};
use ipc::mcp::NeedClaim;
use ipc::{NeedAct, NeedCall};
use store::{AttachmentState, Holder};

use crate::adrift::Adrift;
use crate::daemon::Fleet;
use crate::noticing::Noticing;
use crate::tests::daemon::{fittings, one, two_steps_gated_on_a_person};
use crate::tests::merging::{at_the_gate_having_delivered, Fixture};
use crate::tests::tmp::TempDir;
use testkit::FakeWorkProduct;

const MANIFEST: &str = "01FIXTUREMANIFEST";
const PATH: &str = "crates/store/src/migrations.rs";
/// The path the merge act watches for an undeclared number.
const WATCHED: &str = "protocol-version.toml";

fn git(repo: &std::path::Path, args: &[&str]) {
    let run = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["-c", "user.name=t", "-c", "user.email=t@t"])
        .args(args)
        .output()
        .expect("git on PATH");
    assert!(run.status.success(), "git {args:?}: {run:?}");
}

/// A Fleet holding delivered Jobs for a person, in a real repository.
fn holding_under(home: &TempDir, merge_by: &str) -> Fixture {
    holding_on(home, merge_by, false)
}

/// The same, with `main` known to the fake as the commit the repository's
/// `main` is at, so a branch's diff from its base can be read.
fn holding_on(home: &TempDir, merge_by: &str, base_known: bool) -> Fixture {
    git(
        home.path(),
        &["-c", "init.defaultBranch=main", "init", "--quiet"],
    );
    git(
        home.path(),
        &["commit", "--allow-empty", "-m", "start", "--quiet"],
    );
    let mut fittings = fittings(home, FakeWorkProduct::changed(&["src/log.rs"]));
    if base_known {
        std::fs::write(home.path().join(WATCHED), "major = 23\nminor = 36\n").expect("file");
        git(home.path(), &["add", "."]);
        git(home.path(), &["commit", "-m", "protocol", "--quiet"]);
        let sha = Command::new("git")
            .arg("-C")
            .arg(home.path())
            .args(["rev-parse", "main"])
            .output()
            .expect("git on PATH");
        let sha = String::from_utf8_lossy(&sha.stdout).trim().to_string();
        let held = std::mem::replace(&mut fittings.vcs, testkit::FakeVcs::new());
        fittings.vcs = held.with_ref_at("main", sha);
    }
    fittings.starting().workflows = one(two_steps_gated_on_a_person(
        "summarise",
        None,
        Some("summarise"),
    ));
    fittings.starting().manifest = Manifest::parse(
        std::path::Path::new("armada.yml"),
        &format!("version: 1\nid: 01FIXTUREMANIFEST\nmerge_by: {merge_by}\n"),
    )
    .expect("a manifest");
    fittings.noticing = Noticing::every(Duration::ZERO);
    Fleet::assembled(fittings)
}

/// A Job at its gate whose branch exists in the repository, as it does in life.
async fn at_the_gate(fleet: &Fixture, home: &TempDir) -> (JobId, String) {
    let job = at_the_gate_having_delivered(fleet, home).await;
    let branch = fleet
        .load(&job)
        .await
        .expect("the Job")
        .branch()
        .expect("a branch")
        .as_str()
        .to_string();
    git(home.path(), &["branch", &branch]);
    (job, branch)
}

fn migration(took: Option<&str>) -> Vec<NeedClaim> {
    vec![NeedClaim {
        path: PATH.to_string(),
        what: "a new migration".to_string(),
        took: took.map(str::to_string),
    }]
}

/// What the refusal says, and the wire code it carries.
fn refused(fleet: &Fixture, why: Adrift) -> (String, String) {
    let said = why.to_string();
    match fleet.refusal(why) {
        api::Refusal::IllegalMove(wire) => (wire.code.to_string(), said),
        other => panic!("a person's to answer, so a conflict: {other:?}"),
    }
}

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

/// Who stands in line on `path`, in the order they are served.
async fn line(fleet: &Fixture, path: &str) -> Vec<Holder> {
    let store = fleet.store().lock().await;
    crate::needing::ledger::standing(&store, MANIFEST, Some(path))
        .expect("the ledger")
        .into_iter()
        .map(|need| need.holder)
        .collect()
}

/// The states of every need `holder` ever held.
async fn states(fleet: &Fixture, holder: &Holder) -> Vec<AttachmentState> {
    let store = fleet.store().lock().await;
    store
        .attachments_of(holder)
        .expect("the ledger")
        .into_iter()
        .filter(|row| row.kind == "need")
        .map(|row| row.state)
        .collect()
}

async fn the_claim_under(merge_by: &str) {
    let home = TempDir::new();
    let fleet = holding_under(&home, merge_by);
    let (first, first_branch) = at_the_gate(&fleet, &home).await;
    let (second, _) = at_the_gate(&fleet, &home).await;

    // The first declares and says what it took. Nothing is ahead of it.
    fleet.needs_declared(&first, &migration(Some("V95"))).await;
    assert!(
        fleet.peer_news_for_the_brief(&first).await.is_none(),
        "nothing is ahead of the first"
    );

    // The second is told who is ahead and what that one took.
    fleet.needs_declared(&second, &migration(None)).await;
    let told = fleet
        .peer_news_for_the_brief(&second)
        .await
        .expect("the second is told");
    let told = told.text();
    assert!(told.contains(PATH), "{told}");
    assert!(told.contains(&first_branch), "names who is ahead: {told}");
    assert!(told.contains("took V95"), "and what it took: {told}");

    // Ready first, the second is refused, naming what it waits behind.
    let held = fleet
        .merge_pull_request(&second)
        .await
        .expect_err("it waits behind the first");
    let (code, said) = refused(&fleet, held);
    assert_eq!(code, "fleet.merge_waiting_behind");
    assert!(
        said.contains("a new migration") && said.contains(PATH),
        "{said}"
    );
    assert!(
        said.contains(&first_branch) && said.contains("V95"),
        "{said}"
    );
    assert_eq!(
        fleet.load(&second).await.expect("the Job").status(),
        JobStatus::AwaitingReview,
        "refused, so it is where it was"
    );
    assert_eq!(
        fleet.vcs().times_asked_to_merge(),
        0,
        "nothing reached the forge"
    );

    // The first lands, which spends its need; then the second lands.
    fleet.vcs().now_landed(Landing::Merged {
        url: String::from(crate::tests::merging::PULL_REQUEST),
    });
    let landed = fleet
        .merge_pull_request(&first)
        .await
        .expect("the first lands");
    assert_eq!(landed.status(), JobStatus::CompletedSuccess);
    assert_eq!(
        states(&fleet, &Holder::job(first.as_str())).await,
        [AttachmentState::Spent],
        "spent, not given back"
    );
    assert_eq!(line(&fleet, PATH).await, [Holder::job(second.as_str())]);
    let landed = fleet
        .merge_pull_request(&second)
        .await
        .expect("then the second");
    assert_eq!(landed.status(), JobStatus::CompletedSuccess);
}

#[tokio::test]
async fn the_second_job_waits_behind_the_first_under_forge() {
    the_claim_under("forge").await;
}

#[tokio::test]
async fn the_second_job_waits_behind_the_first_under_push() {
    the_claim_under("push").await;
}

/// Declaring what is already declared changes nothing, and a Job may wait on
/// two paths at once.
#[tokio::test]
async fn declaring_again_records_nothing_and_a_second_path_does_not_replace_the_first() {
    let home = TempDir::new();
    let fleet = holding_under(&home, "forge");
    let (job, _) = at_the_gate(&fleet, &home).await;
    fleet.needs_declared(&job, &migration(Some("V95"))).await;
    let before = fleet
        .store()
        .lock()
        .await
        .attachments_of(&Holder::job(job.as_str()))
        .expect("rows");
    fleet.needs_declared(&job, &migration(None)).await;
    let after = fleet
        .store()
        .lock()
        .await
        .attachments_of(&Holder::job(job.as_str()))
        .expect("rows");
    assert_eq!(before, after, "the same need again, nothing written");

    let mut two = migration(None);
    two[0].path = "docs/INDEX.md".into();
    fleet.needs_declared(&job, &two).await;
    assert_eq!(line(&fleet, PATH).await.len(), 1);
    assert_eq!(line(&fleet, "docs/INDEX.md").await.len(), 1);
}

/// A dropped Job gives its need back, so the Job behind it is no longer held.
#[tokio::test]
async fn a_dropped_jobs_need_is_given_back_and_no_longer_blocks() {
    let home = TempDir::new();
    let fleet = holding_under(&home, "forge");
    let (first, _) = at_the_gate(&fleet, &home).await;
    let (second, _) = at_the_gate(&fleet, &home).await;
    fleet.needs_declared(&first, &migration(Some("V95"))).await;
    fleet.needs_declared(&second, &migration(None)).await;
    fleet
        .merge_pull_request(&second)
        .await
        .expect_err("held while the first stands");

    let dropped = fleet.reject(&first).await.expect("a person rejects it");
    assert!(dropped.status().is_terminal());
    assert_eq!(
        states(&fleet, &Holder::job(first.as_str())).await,
        [AttachmentState::GivenBack]
    );

    fleet.vcs().now_landed(Landing::Merged {
        url: String::from(crate::tests::merging::PULL_REQUEST),
    });
    fleet
        .merge_pull_request(&second)
        .await
        .expect("nothing is ahead any more");
}

fn session_started(cwd: &str) -> ipc::SessionReport {
    ipc::SessionReport {
        harness: "a_harness".into(),
        session_id: ipc::SessionId::carried("a-session"),
        fact: ipc::SessionFact::Started {
            cwd: cwd.into(),
            title: Some("the session".into()),
            origin: ipc::SessionOrigin::Terminal,
        },
    }
}

fn session_fact(fact: ipc::SessionFact) -> ipc::SessionReport {
    ipc::SessionReport {
        harness: "a_harness".into(),
        session_id: ipc::SessionId::carried("a-session"),
        fact,
    }
}

/// One order, not two: a session's need through the intake and a Job's
/// declaration are rows of one table, so each is ahead of the other by when it
/// declared, and the session ending gives its need back.
#[tokio::test]
async fn a_job_and_a_session_stand_in_one_order() {
    let home = TempDir::new();
    let fleet = holding_under(&home, "forge");
    let (job, job_branch) = at_the_gate(&fleet, &home).await;

    // The session declared first, through the intake the mod reports on.
    fleet
        .report_session(session_started(&home.path().display().to_string()))
        .await
        .expect("started");
    fleet
        .report_session(session_fact(ipc::SessionFact::Attached {
            attachment: ipc::AttachmentReport {
                kind: "need".into(),
                target: PATH.into(),
                detail: [
                    ("what".to_string(), "a new migration".to_string()),
                    ("took".to_string(), "V96".to_string()),
                ]
                .into(),
            },
        }))
        .await
        .expect("attached");
    fleet.needs_declared(&job, &migration(None)).await;

    let held = fleet
        .merge_pull_request(&job)
        .await
        .expect_err("the Job waits behind the session");
    let (_, said) = refused(&fleet, held);
    assert!(
        said.contains("the session") && said.contains("V96"),
        "{said}"
    );

    // And both are in the one listing, in the order they declared.
    assert_eq!(
        line(&fleet, PATH).await,
        [Holder::session("a-session"), Holder::job(job.as_str())]
    );
    let listed = fleet.list_needs(None).await.expect("the list");
    let names: Vec<&str> = listed
        .needs
        .iter()
        .map(|need| need.held_by.as_str())
        .collect();
    assert_eq!(names, ["the session", job_branch.as_str()]);

    // The session ending gives its need back, which frees the Job.
    fleet
        .report_session(session_fact(ipc::SessionFact::Ended {
            reason: "done".into(),
        }))
        .await
        .expect("ended");
    fleet.vcs().now_landed(Landing::Merged {
        url: String::from(crate::tests::merging::PULL_REQUEST),
    });
    fleet
        .merge_pull_request(&job)
        .await
        .expect("nothing is ahead");
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

/// The same rule `armada land` holds a session to: a Job whose branch moves the
/// protocol minor with no need declared is refused at its merge, naming what to
/// run, and lands once it has declared. Asked of the ledger.
#[tokio::test]
async fn a_job_that_took_a_minor_undeclared_is_refused() {
    let home = TempDir::new();
    let fleet = holding_on(&home, "forge", true);
    let (job, branch) = at_the_gate(&fleet, &home).await;
    let file = home.path().join(WATCHED);
    git(home.path(), &["branch", "-f", &branch]);
    git(home.path(), &["checkout", "--quiet", &branch]);
    std::fs::write(&file, "major = 23\nminor = 37\n").expect("file");
    git(home.path(), &["commit", "-am", "a minor", "--quiet"]);
    // `main` must be the fork: put it back where the branch left it.
    git(home.path(), &["checkout", "--quiet", "main"]);

    let held = fleet
        .merge_pull_request(&job)
        .await
        .expect_err("a minor taken with no need");
    let (code, said) = refused(&fleet, held);
    assert_eq!(code, "fleet.merge_waiting_behind");
    assert!(
        said.contains(&format!("armada need {WATCHED} \"a minor\"")),
        "{said}"
    );
    assert_eq!(fleet.vcs().times_asked_to_merge(), 0);

    fleet
        .needs_declared(
            &job,
            &[NeedClaim {
                path: WATCHED.to_string(),
                what: "a minor".to_string(),
                took: Some("23.37".to_string()),
            }],
        )
        .await;
    fleet.vcs().now_landed(Landing::Merged {
        url: String::from(crate::tests::merging::PULL_REQUEST),
    });
    fleet
        .merge_pull_request(&job)
        .await
        .expect("declared, it lands");
}
