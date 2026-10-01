//! What the proposer has settled, read off an answer that is still arriving.
//!
//! **The claim.** The owner asked on 30 Sep 2026 for the proposer not to report
//! the Job whole, and named four fields in an order that is his reasoning rather
//! than a layout: the workflow decides the Job's shape, the title is what makes
//! the row recognisable, done-when is the goal, and the settings are the part he
//! can still change.
//!
//! He chose one call read as it arrives over four smaller calls, and the cost he
//! took is that the field order becomes a contract with the model. These hold it:
//! four fields in his order, one at a time, never half of one, and a call that
//! dies halfway leaving exactly what had landed.
//!
//! **Pure, and no process.** `crate::tests::watching` runs a real pipe; this
//! reads text. `Settled::of` is the one place a prefix becomes fields.

use std::collections::BTreeMap;

use config::ResolvedWorkflow;
use core_model::{Urgency, WorkflowId};

use crate::proposing::{Brief, Proposal, Settled, Unresolved};
use crate::tests::proposing::{a_catalogue, a_models, A_REQUEST};

fn held() -> BTreeMap<WorkflowId, ResolvedWorkflow> {
    a_catalogue()
        .into_iter()
        .map(|workflow| (workflow.id().clone(), workflow))
        .collect()
}

fn settled(written: &str) -> Settled {
    Settled::of(written, &held(), &a_models())
}

/// The answer this suite reads, whole, in the order `ANSWER_FORMAT` asks for it.
/// **One string, cut at four places below**, so a field that moved in the format
/// moves here once rather than in five prefixes that can disagree.
const ANSWER: &str = "job: 1\n\
     workflow: feature\n\
     title: Say which of the two a clear gave back\n\
     done_when: The Cleared tab names the branch on every row whose worktree is gone\n\
     done_when: A row whose branch was also given back says so\n\
     settings: urgency=normal, model=opus\n\
     because: it adds a fact to a surface that already lists the rows\n";

/// How much of [`ANSWER`] has arrived, by the line count.
fn upto(lines: usize) -> String {
    ANSWER
        .lines()
        .take(lines)
        .map(|line| format!("{line}\n"))
        .collect()
}

fn a_workflow(named: &str) -> Option<WorkflowId> {
    Some(WorkflowId::carried(core_model::Ulid::carried(named)))
}

/// Nothing settles out of a line still being written. **The claim that stops a
/// title landing as `Say which of the two a cle`**: the format is one field per
/// line, so a newline is what says a value will not change.
#[test]
fn a_line_still_being_written_settles_nothing() {
    assert_eq!(
        settled("job: 1\nworkflow: feature\ntitle: Say which of the two a cle"),
        Settled {
            workflow: a_workflow("feature"),
            ..Settled::default()
        },
        "the half-written title was read"
    );
    assert_eq!(
        settled("job: 1\nworkflow: featu"),
        Settled::default(),
        "a half-written workflow id was read, and it would name nothing held"
    );
    assert_eq!(
        settled("job"),
        Settled::default(),
        "an answer with no newline in it at all settled something"
    );
}

/// The four, in his order, each arriving on its own and none arriving early.
#[test]
fn the_four_fields_settle_in_the_owners_order() {
    // 1. Workflow — the shape. Nothing else has landed.
    let one = settled(&upto(2));
    assert_eq!(one.workflow, a_workflow("feature"), "the workflow had not settled");
    assert_eq!(one.title, None, "the title landed before it was written");
    assert!(one.done_when.is_empty());
    assert_eq!(one.urgency, None);

    // 2. Title — what makes the row recognisable.
    let two = settled(&upto(3));
    assert_eq!(
        two.title.as_deref(),
        Some("Say which of the two a clear gave back")
    );
    assert!(two.done_when.is_empty(), "a done-when line landed early");
    assert_eq!(two.urgency, None);

    // 3. Done when — the goal. **One line at a time**, which is what the format
    // asks for: the line is written again per criterion rather than one line
    // that grows, so a person watching gets one more row each time.
    let three = settled(&upto(4));
    assert_eq!(
        three.done_when,
        vec!["The Cleared tab names the branch on every row whose worktree is gone"],
        "the first criterion did not land on its own"
    );
    assert_eq!(
        three.urgency, None,
        "the settings landed before they were written"
    );
    let four = settled(&upto(5));
    assert_eq!(
        four.done_when.len(),
        2,
        "the second criterion did not join the first"
    );

    // 4. Settings — the part he can still change. **Both pairs on one line**,
    // so the urgency and the model settle together: it is one field.
    let five = settled(&upto(6));
    assert_eq!(five.urgency, Some(Urgency::Normal));
    assert_eq!(five.model.as_deref(), Some("opus"));

    assert_eq!(four.model, None, "the model landed before it was written");

    // And nothing is taken away as the next field lands.
    assert_eq!(five.workflow, one.workflow);
    assert_eq!(five.title, two.title);
    assert_eq!(five.done_when, four.done_when);
}

/// **A workflow nothing holds never reaches a screen.** `Brief::read` answers
/// `Unresolved::NotHeld` for one and the request comes back, so drawing the name
/// meanwhile would put a workflow on a row that nothing froze.
#[test]
fn a_workflow_this_repository_does_not_hold_never_settles() {
    let read = settled("job: 1\nworkflow: choreograph\ntitle: Something\n");
    assert_eq!(read.workflow, None, "a workflow nothing holds settled");
    assert_eq!(
        read.title.as_deref(),
        Some("Something"),
        "the fields after it stopped being read because of it"
    );
}

/// **A call that dies after the workflow leaves a workflow and no title**, and
/// the plan is refused rather than minted.
#[test]
fn a_call_that_dies_after_the_workflow_leaves_the_workflow_and_no_title() {
    let died = upto(2);
    let read = settled(&died);
    assert_eq!(read.workflow, a_workflow("feature"));
    assert_eq!(read.title, None);

    let catalogue = held();
    let refused = Brief::about(A_REQUEST, &catalogue, &a_models())
        .read(&died, &catalogue, &a_models())
        .expect_err("a workflow with no title was read as a plan");
    assert!(
        refused.to_string().contains("given no name"),
        "the refusal did not say the Job was never named: {refused}"
    );
}

/// **The head Job only.** A request that becomes several is several rows and the
/// extras carry `dispatched_by`; what is being watched is the row the press made,
/// so a second block's fields must not join the first's.
#[test]
fn only_the_head_job_settles() {
    let read = settled(&format!(
        "{ANSWER}job: 2\nworkflow: bug\ntitle: The second one\n         done_when: The second Job's own line\nsettings: urgency=incident\n"
    ));
    assert_eq!(
        read.title.as_deref(),
        Some("Say which of the two a clear gave back"),
        "the second block's title reached a reader watching the first row"
    );
    assert_eq!(
        read.done_when.len(),
        2,
        "the second block's done-when line joined the first row's: {:?}",
        read.done_when
    );
    assert_eq!(
        read.urgency,
        Some(Urgency::Normal),
        "the second block's urgency was read as the first row's"
    );
    assert_eq!(read.model.as_deref(), Some("opus"));
}

/// **No `scope` and no `because` among the criteria.** Four fields because four
/// are what a person watching has somewhere to put, and a transcript on the wire
/// is what `ipc::proposing`'s dated correction is careful not to become.
#[test]
fn the_reading_carries_no_brief_and_no_reason() {
    let read = settled(&format!("{ANSWER}scope: only the Cleared tab\n"));
    assert_eq!(
        read.done_when.len(),
        2,
        "a scope or a reason line joined the criteria"
    );
}

/// What settled is what the finished answer says. **The reading cannot drift
/// from the plan**, which is the whole risk of reading a prefix: a field drawn
/// during the wait that the answer then contradicts is worse than drawing none.
#[test]
fn what_settled_is_what_the_plan_ends_up_holding() {
    let catalogue = held();
    let read = settled(ANSWER);
    let Proposal::Resolved(jobs) = Brief::about(A_REQUEST, &catalogue, &a_models())
        .read(ANSWER, &catalogue, &a_models())
        .expect("a plan")
    else {
        panic!("the answer did not resolve");
    };
    assert_eq!(jobs.len(), 1);
    assert_eq!(Some(jobs[0].workflow_id.clone()), read.workflow);
    assert_eq!(Some(jobs[0].title.clone()), read.title);
    assert_eq!(jobs[0].done_when, read.done_when);
    assert_eq!(Some(jobs[0].urgency), read.urgency);
    assert_eq!(jobs[0].model, read.model);
}

/// A request that names no done-when and no settings is the ordinary case, and
/// it reaches the gate as one: no criteria, and normal.
#[test]
fn an_answer_with_neither_line_is_held_to_the_workflow_alone() {
    let catalogue = held();
    let Proposal::Resolved(jobs) = Brief::about(A_REQUEST, &catalogue, &a_models())
        .read(
            "workflow: bug\ntitle: The log reader drops a line\n",
            &catalogue,
            &a_models(),
        )
        .expect("a plan")
    else {
        panic!("the answer did not resolve");
    };
    assert!(jobs[0].done_when.is_empty());
    assert_eq!(jobs[0].urgency, Urgency::Normal);
    // **Absent stays absent.** No model named is configuration deciding, and
    // never a model this call picked as a default.
    assert_eq!(jobs[0].model, None);
}

/// An urgency nothing spells is `normal` rather than a refusal. **A setting a
/// person changes at the gate is not a reason to throw a plan away.**
#[test]
fn an_urgency_nobody_can_spell_is_normal_and_not_a_refusal() {
    let catalogue = held();
    let Proposal::Resolved(jobs) = Brief::about(A_REQUEST, &catalogue, &a_models())
        .read(
            "workflow: bug\ntitle: The log reader drops a line\nsettings: urgency=extremely\n",
            &catalogue,
            &a_models(),
        )
        .expect("a plan")
    else {
        panic!("the answer did not resolve");
    };
    assert_eq!(jobs[0].urgency, Urgency::Normal);
    assert_eq!(
        settled("workflow: bug\nsettings: urgency=extremely\n").urgency,
        None,
        "an unspellable urgency settled anyway"
    );
}

/// An `incident` the request describes reaches the Job as one.
#[test]
fn an_incident_the_request_describes_reaches_the_job_as_one() {
    let catalogue = held();
    let Proposal::Resolved(jobs) = Brief::about(A_REQUEST, &catalogue, &a_models())
        .read(
            "workflow: bug\ntitle: The log reader drops a line\nsettings: urgency=incident\n",
            &catalogue,
            &a_models(),
        )
        .expect("a plan")
    else {
        panic!("the answer did not resolve");
    };
    assert_eq!(jobs[0].urgency, Urgency::Incident);
}

/// **The model is picked from what this machine holds, never invented.** The
/// owner took the model on 30 Sep 2026 over the argument that a model choosing
/// which model runs the work is the dial every later cost hangs off; the guard
/// he took with it is the list.
#[test]
fn the_model_it_names_is_one_this_machine_holds() {
    let catalogue = held();
    let Proposal::Resolved(jobs) = Brief::about(A_REQUEST, &catalogue, &a_models())
        .read(
            "workflow: bug\ntitle: The log reader drops a line\nsettings: urgency=normal, model=haiku\n",
            &catalogue,
            &a_models(),
        )
        .expect("a plan")
    else {
        panic!("the answer did not resolve");
    };
    assert_eq!(jobs[0].model.as_deref(), Some("haiku"));
    // And the list is in the question, because a call told nothing can only
    // guess.
    assert!(
        Brief::about(A_REQUEST, &catalogue, &a_models())
            .question()
            .contains("haiku"),
        "the question never told the call which models this machine holds"
    );
}

/// **A model nothing holds is refused for a workflow's reason and through its
/// own arm.** Not nearest-matched: a name nothing holds is not evidence about
/// which one was meant, and a Job spawned against a model this machine cannot
/// run fails at the spawn with the work already approved.
///
/// **And not `Unresolved::NotHeld`**, which it was for half a day: that arm's
/// refusal tells a person to say the request again differently, which cannot
/// fix a model name. `crate::tests::proposing::over_http` holds the code.
#[test]
fn a_model_this_machine_does_not_hold_refuses_the_request() {
    let catalogue = held();
    let answered = Brief::about(A_REQUEST, &catalogue, &a_models())
        .read(
            "workflow: bug\ntitle: The log reader drops a line\nsettings: model=gpt-9\n",
            &catalogue,
            &a_models(),
        )
        .expect("a reading, not a call failure");
    assert_eq!(
        answered,
        Proposal::Unresolved(Unresolved::ModelNotHeld {
            named: "gpt-9".to_string(),
            held: a_models(),
        }),
        "a model nothing holds was accepted, or refused as the workflow's own \
         refusal — whose advice is to rephrase"
    );
    // The set rides along, because it is already in hand and a refusal naming
    // only what was wrong leaves a person to go and look up what was right.
    let Proposal::Unresolved(why) = &answered else {
        panic!("a resolved plan");
    };
    assert!(
        why.to_string().contains("haiku"),
        "the refusal never said what this machine does run: {why}"
    );
    // And it never settles either, so nothing drew it while the call ran.
    assert_eq!(
        settled("workflow: bug\nsettings: model=gpt-9\n").model,
        None
    );
}

/// A settings line that names only a model still settles the settings. **One
/// line is one field**, and either pair on it is enough for that field to have
/// arrived.
#[test]
fn a_settings_line_naming_only_a_model_settles_the_settings() {
    let read = settled("workflow: bug\ntitle: A name\nsettings: model=sonnet\n");
    assert_eq!(read.model.as_deref(), Some("sonnet"));
    assert_eq!(read.urgency, None);
}
