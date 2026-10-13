//! The card a Job's Commander puts to the owner, under a real Fleet and a
//! scripted host. `#2166`.
//!
//! **What a card does that a held call does not**: it is kept, so it is still
//! there after Fleet restarts; it counts as the Job waiting on the owner without
//! moving the Job; the night records it as blocked and never answers it; and his
//! answer wakes the Commander. The per-Job model over the machine's is here too,
//! because it is read at the same place, the start of a turn.

use std::sync::Arc;
use std::time::Duration;

use adapter_traits::Effort;
use api::{Conversations, Refusal};
use core_model::{JobId, JobStatus};
use ipc::{AnswerCommanderCard, AskTheOwner, CardId, CardOptions, CardText, CommanderCard, Event};

use crate::helm::Seat;
use crate::tests::commander_conversation::{
    hosted, resolved, said, settled, woke_rows, working, Hosted, Scripted,
};
use crate::tests::tmp::TempDir;

fn the_question() -> AskTheOwner {
    AskTheOwner {
        text: CardText::said("Group 3 is red on desktop_test").expect("a line"),
        reach: CardText::said("A Check is the owner's to skip").expect("a line"),
        question: CardText::said("Skip desktop_test on Group 3?").expect("a line"),
        options: CardOptions::labelled(vec![
            "Skip desktop_test".to_string(),
            "Leave it red".to_string(),
        ])
        .expect("two options"),
    }
}

async fn raised(fleet: &Arc<Hosted>, job: &JobId) -> Result<CommanderCard, Refusal> {
    Arc::clone(fleet)
        .ask_the_owner(resolved(fleet, job).await, the_question())
        .await
}

async fn picked(
    fleet: &Arc<Hosted>,
    job: &JobId,
    card: &str,
    option: &str,
) -> Result<ipc::CommanderCardAnswered, Refusal> {
    Arc::clone(fleet)
        .answer_commander_card(
            resolved(fleet, job).await,
            AnswerCommanderCard {
                card_id: CardId::carried(card),
                option_id: option.to_string(),
            },
        )
        .await
}

async fn detail(fleet: &Hosted, job: &JobId) -> ipc::JobDetail {
    use api::Queries;
    fleet
        .get_job(ipc::JobId::from(job))
        .await
        .expect("the Job reads")
}

/// The next `commander.card` on the stream for this Job: the card, or `None`
/// for the one that says it was answered.
async fn next_card_event(
    watching: &mut api::Subscription,
    job: &JobId,
) -> Option<CommanderCard> {
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            match watching.next().await {
                Some(api::Next::Send(delivered)) => {
                    if let Event::CommanderCard(changed) = delivered.event {
                        if changed.job_id == ipc::JobId::from(job) {
                            return changed.card;
                        }
                    }
                }
                Some(api::Next::Missed(dropped)) => panic!("dropped {dropped} events"),
                None => panic!("the stream closed"),
            }
        }
    })
    .await
    .expect("the event is published")
}

/// **The card, raised, answered, and waking the Commander with the answer.**
#[tokio::test]
async fn a_card_is_raised_answered_and_wakes_the_commander_with_his_pick() {
    let home = TempDir::new();
    let host = Scripted::answering();
    let fleet = hosted(&home, &host);
    let job = working(&fleet, &home, "bound the reader").await;
    let status = fleet.load(&job).await.expect("reads").status();
    let mut watching = fleet.events().subscribe();

    let card = raised(&fleet, &job).await.expect("the card is put out");

    assert_eq!(card.by, ipc::CardBy::Commander);
    let options: Vec<(&str, &str)> = card
        .options
        .iter()
        .map(|one| (one.id.as_str(), one.label.as_str()))
        .collect();
    assert_eq!(options, [("1", "Skip desktop_test"), ("2", "Leave it red")]);
    assert_eq!(
        next_card_event(&mut watching, &job).await,
        Some(card.clone()),
        "the card goes out on the stream"
    );
    assert_eq!(detail(&fleet, &job).await.commander_card, Some(card.clone()));
    assert!(host.carried().is_empty(), "raising a card wakes nobody");
    assert_eq!(
        fleet.load(&job).await.expect("reads").status(),
        status,
        "a card moves no status"
    );

    // A second while one is out is told which, and a stale or unoffered pick is not kept.
    let second = raised(&fleet, &job).await.expect_err("one card out per Job");
    assert!(
        second.error().message.contains("Skip desktop_test on Group 3?"),
        "{}",
        second.error().message
    );
    for (id, option) in [(card.card_id.as_str(), "3"), ("01OLDCARD", "1")] {
        let refused = picked(&fleet, &job, id, option).await.expect_err("not taken");
        assert_eq!(refused.status(), 409, "{id} {option}");
    }
    assert!(detail(&fleet, &job).await.commander_card.is_some(), "still out");

    let answered = picked(&fleet, &job, card.card_id.as_str(), "1")
        .await
        .expect("his pick is taken");
    assert!(answered.woke);
    settled(&fleet, &job).await;

    let reason =
        "The owner answered your card, \"Skip desktop_test on Group 3?\": Skip desktop_test";
    assert_eq!(woke_rows(&said(&fleet, &job).await), [reason]);
    let carried = host.carried();
    assert_eq!(carried.len(), 1, "his answer is one turn");
    assert!(
        carried[0].turn.contains(&format!("Fleet woke you: {reason}")),
        "{}",
        carried[0].turn
    );
    assert_eq!(next_card_event(&mut watching, &job).await, None, "and it goes in");
    let after = detail(&fleet, &job).await;
    assert_eq!(after.commander_card, None);
    assert!(!after.job.asking, "nothing is waiting on him now");

    let again = picked(&fleet, &job, card.card_id.as_str(), "1")
        .await
        .expect_err("answered once");
    assert_eq!(again.status(), 409);
}

/// **Your move and the Board's waiting list read the row**, which says `asking`
/// for as long as the card is out, whatever the Job's status. A Fleet that
/// restarts still has the card.
#[tokio::test]
async fn a_job_with_a_card_out_is_waiting_on_the_owner_and_still_is_after_a_restart() {
    let home = TempDir::new();
    let host = Scripted::answering();
    let fleet = hosted(&home, &host);
    let job = working(&fleet, &home, "bound the reader").await;
    let quiet = detail(&fleet, &job).await;
    assert!(!quiet.job.asking);
    assert!(fleet.alerts(None).await.expect("alerts").blocked.is_empty());

    let card = raised(&fleet, &job).await.expect("put out");

    let row = detail(&fleet, &job).await.job;
    assert!(row.asking, "the row Your move is drawn from");
    assert_eq!(
        row.status.domain(),
        JobStatus::Running,
        "a Job whose card is out keeps running: the card is beside it"
    );
    let listed = {
        use api::Queries;
        fleet.list_jobs(None).await.expect("lists").jobs
    };
    assert!(listed.iter().any(|one| one.id == row.id && one.asking));
    let alerts = fleet.alerts(None).await.expect("alerts");
    assert_eq!(
        alerts.blocked.iter().map(|one| one.job_id.clone()).collect::<Vec<_>>(),
        [row.id.clone()],
        "the Board's waiting list names it"
    );

    drop(fleet);
    let restarted = hosted(&home, &host);
    assert_eq!(
        detail(&restarted, &job).await.commander_card,
        Some(card),
        "kept, not held: Fleet restarting loses nothing"
    );
    assert!(detail(&restarted, &job).await.job.asking);
}

/// **Beside an escalation**, not instead of it: the Job is stopped for its own
/// reason and the card is a second thing it waits on.
#[tokio::test]
async fn a_card_beside_an_escalated_job_leaves_the_escalation_alone() {
    let home = TempDir::new();
    let host = Scripted::answering();
    let fleet = hosted(&home, &host);
    let job = working(&fleet, &home, "bound the reader").await;
    let record = fleet.load(&job).await.expect("reads");
    fleet
        .move_job(
            &record,
            core_model::Target::Escalated(core_model::EscalationTrigger::Stalled),
            core_model::Actor::Fleet,
        )
        .await
        .expect("escalated");

    let card = raised(&fleet, &job).await.expect("put out");

    assert_eq!(
        fleet.load(&job).await.expect("reads").status(),
        JobStatus::Escalated
    );
    assert_eq!(detail(&fleet, &job).await.commander_card, Some(card));
    assert_eq!(
        fleet.alerts(None).await.expect("alerts").blocked.len(),
        1,
        "one row, not two: the Job is listed once"
    );
}

/// **A Job that is over has nobody to ask.**
#[tokio::test]
async fn a_job_that_is_over_takes_no_card_and_shows_none() {
    let home = TempDir::new();
    let host = Scripted::answering();
    let fleet = hosted(&home, &host);
    let job = working(&fleet, &home, "bound the reader").await;
    let card = raised(&fleet, &job).await.expect("put out");
    let record = fleet.load(&job).await.expect("reads");
    fleet
        .move_job(&record, core_model::Target::Killed, core_model::Actor::Human)
        .await
        .expect("killed");

    assert_eq!(detail(&fleet, &job).await.commander_card, None);
    assert!(!detail(&fleet, &job).await.job.asking);
    assert!(fleet.alerts(None).await.expect("alerts").blocked.is_empty());
    let late = raised(&fleet, &job).await.expect_err("nobody is waiting");
    assert_eq!(late.status(), 409);
    drop(card);
}

/// **Sleep mode leaves a card as blocked**, as it does an escalated Job it will
/// not answer, and never picks for him. Whether the card went out before the
/// night began or during it.
#[tokio::test]
async fn the_night_records_a_card_as_blocked_and_never_answers_it() {
    let home = TempDir::new();
    let host = Scripted::answering();
    let fleet = hosted(&home, &host);
    let early = working(&fleet, &home, "bound the reader").await;
    let late = working(&fleet, &home, "trim the log").await;
    raised(&fleet, &early).await.expect("a card before the night");

    fleet.switch_sleep(true).await.expect("the night begins");
    Arc::clone(&fleet).sleep_pass().await.expect("a pass");
    let held = fleet.sleep_state().await.expect("reads");
    let early_handle = fleet.load(&early).await.expect("reads").handle();
    assert_eq!(
        held.blocked
            .iter()
            .map(|one| (one.who.clone(), one.text.clone()))
            .collect::<Vec<_>>(),
        [(
            early_handle.clone(),
            "Commander asks, waiting on you: Skip desktop_test on Group 3?".to_string()
        )],
        "the card out before the night is swept in"
    );

    raised(&fleet, &late).await.expect("a card in the night");
    Arc::clone(&fleet).sleep_pass().await.expect("a pass");
    Arc::clone(&fleet).sleep_pass().await.expect("and again");
    let state = fleet.sleep_state().await.expect("reads");
    assert_eq!(state.blocked.len(), 2, "each once, however many passes");
    assert!(state.decided.is_empty(), "the night picks nothing for him");
    assert!(
        detail(&fleet, &early).await.commander_card.is_some(),
        "and the card is still out for the morning"
    );
    assert!(host.carried().is_empty(), "nobody woke the Commander to answer it");
}

/// **A Job's own model and effort win over the machine's, each half on its
/// own**, and clearing goes back to the machine.
#[tokio::test]
async fn a_jobs_own_commander_model_and_effort_win_over_the_machines_each_on_its_own() {
    let home = TempDir::new();
    let host = Scripted::answering();
    let fleet = hosted(&home, &host);
    let job = working(&fleet, &home, "bound the reader").await;

    async fn turn_on(fleet: &Arc<Hosted>, host: &Arc<Scripted>, job: &JobId, why: &str) -> adapter_traits::Model {
        let before = host.carried().len();
        Arc::clone(fleet).wake_commander(job, why).await.expect("woken");
        settled(fleet, job).await;
        let carried = host.carried();
        assert_eq!(carried.len(), before + 1, "{why}");
        match &carried[before].seat {
            Seat::Commander(commander) => commander.model.clone(),
            other => panic!("not a Commander's seat: {other:?}"),
        }
    }

    let machine = turn_on(&fleet, &host, &job, "no choice made").await;
    assert_eq!(
        (machine.as_str(), machine.effort()),
        ("the-commander-model", None),
        "nothing chosen: the machine's"
    );

    fleet
        .set_commander_model(&job, Some("another-model"), Some("low"))
        .await
        .expect("both chosen");
    let both = turn_on(&fleet, &host, &job, "both chosen").await;
    assert_eq!((both.as_str(), both.effort()), ("another-model", Some(Effort::Low)));
    let read = detail(&fleet, &job).await;
    assert_eq!(read.commander_model_override.as_deref(), Some("another-model"));
    assert_eq!(read.commander_effort_override.as_deref(), Some("low"));

    fleet
        .set_commander_model(&job, None, Some("high"))
        .await
        .expect("effort alone");
    let effort_only = turn_on(&fleet, &host, &job, "effort alone").await;
    assert_eq!(
        (effort_only.as_str(), effort_only.effort()),
        ("the-commander-model", Some(Effort::High)),
        "the model defers to the machine's when the Job names none"
    );

    fleet
        .set_commander_model(&job, Some("another-model"), Some("harness default"))
        .await
        .expect("the harness's own effort is a choice");
    let harness = turn_on(&fleet, &host, &job, "harness effort").await;
    assert_eq!((harness.as_str(), harness.effort()), ("another-model", None));

    fleet
        .set_commander_model(&job, None, None)
        .await
        .expect("cleared");
    let cleared = turn_on(&fleet, &host, &job, "cleared").await;
    assert_eq!((cleared.as_str(), cleared.effort()), ("the-commander-model", None));
    assert_eq!(detail(&fleet, &job).await.commander_model_override, None);
}

#[tokio::test]
async fn a_model_the_roster_does_not_offer_or_an_effort_nobody_spells_is_refused_and_keeps_nothing() {
    let home = TempDir::new();
    let host = Scripted::answering();
    let fleet = hosted(&home, &host);
    let job = working(&fleet, &home, "bound the reader").await;
    fleet
        .set_commander_model(&job, Some("another-model"), Some("low"))
        .await
        .expect("chosen");

    fleet
        .set_commander_model(&job, Some("a-model-nobody-has"), Some("high"))
        .await
        .expect_err("not on the roster");
    fleet
        .set_commander_model(&job, Some("a-model"), Some("extreme"))
        .await
        .expect_err("not an effort");

    let read = detail(&fleet, &job).await;
    assert_eq!(
        (
            read.commander_model_override.as_deref(),
            read.commander_effort_override.as_deref()
        ),
        (Some("another-model"), Some("low")),
        "a refusal changes neither half"
    );
}
