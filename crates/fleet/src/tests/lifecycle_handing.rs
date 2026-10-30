//! A ticket goes with its plan group: a Job that starts another for a group whose ticket it holds
//! lets go of the ticket, and the started Job holds it. **No forge, tracker or process is reached.**

use std::sync::Arc;

use adapter_traits::TicketState;
use api::HostedSessions;
use core_model::{Lifecycle, Observed, Standing, StateId};
use ipc::{SessionId, SessionRecord};
use store::AttachmentState;
use testkit::FakeTrackers;

use super::lifecycle_keeping::holds;
use super::session_host::{rig_fitted, rig_placing, Rig};
use super::session_plan::{group, plan, recorded};
use crate::lifecycles::holding::rebuilt;

async fn record(rig: &Rig, id: &SessionId) -> SessionRecord {
    Arc::clone(&rig.fleet).get_session(id.clone()).await.expect("a session").session
}

/// A Job holding `tickets`, its plan naming one in each group's title, and the tickets taken on.
async fn planning(rig: &Rig, tickets: &[&str]) -> SessionId {
    let id = rig.start().await;
    for ticket in tickets {
        holds(rig, &id, "issue", ticket, AttachmentState::Standing, &[]).await;
    }
    rig.fleet.lifecycles_moved(record(rig, &id).await).await;
    let groups = tickets.iter().map(|ticket| group(None, &format!("Fix {ticket}"), "rust-engineer", &[], &[(None, "Do it")])).collect();
    recorded(rig, plan(&id, groups)).await;
    id
}

/// `child` is started for `group` of `parent`'s plan, and Fleet looks at it as its row changes.
async fn started_for(rig: &Rig, parent: &SessionId, group: &str) -> SessionId {
    let child = rig.start().await;
    holds(rig, &child, "started_by", parent.as_str(), AttachmentState::Spent, &[("group", group)]).await;
    rig.fleet.lifecycles_moved(record(rig, &child).await).await;
    child
}

/// `(key, ended, handed_to)` of every ticket the Job holds, by key: the ledger lists its rows in no order a test should rely on.
async fn tickets(rig: &Rig, id: &SessionId) -> Vec<(String, bool, String)> {
    let kept = rig.fleet.store().lock().await.lifecycle_instances_of(id.as_str()).expect("reads");
    let mut held: Vec<_> = kept.into_iter().filter(|one| one.lifecycle == "ticket").map(|one| (one.key, !one.ended_at.is_empty(), one.handed_to)).collect();
    held.sort();
    held
}

fn ticket(key: &str, ended: bool, handed_to: &str) -> (String, bool, String) {
    (key.to_string(), ended, handed_to.to_string())
}

#[tokio::test]
async fn a_ticket_goes_to_the_job_started_for_its_group_and_the_parent_lets_go_of_that_one_alone() {
    let rig = rig_placing(None);
    let parent = planning(&rig, &["armada-95v", "armada-7gs"]).await;

    let child = started_for(&rig, &parent, "g1").await;

    assert_eq!(tickets(&rig, &child).await, [ticket("armada-95v", false, "")], "the started Job holds it");
    assert_eq!(
        tickets(&rig, &parent).await,
        [ticket("armada-7gs", false, ""), ticket("armada-95v", true, child.as_str())],
        "the parent's ends as handed, and the group not handed is still its own"
    );
    let kept = rig.fleet.store().lock().await.lifecycle_instances_of(parent.as_str()).expect("reads");
    let job = kept.iter().find(|one| one.lifecycle == "job").expect("the child, as a Job the parent holds");
    assert_eq!(
        job.links,
        [("job".to_string(), parent.as_str().to_string()), ("ticket".to_string(), "armada-95v".to_string())],
        "linked to the ticket, beside the Job that started it"
    );
    let issues: Vec<String> = record(&rig, &child).await.attachments.into_iter().filter(|one| one.kind == "issue").map(|one| one.target).collect();
    assert_eq!(issues, ["armada-95v"], "the child holds it on its ledger too");
}

#[tokio::test]
async fn looking_at_the_started_job_again_hands_nothing_twice() {
    let rig = rig_placing(None);
    let parent = planning(&rig, &["armada-95v"]).await;
    let child = started_for(&rig, &parent, "g1").await;
    let before = (tickets(&rig, &parent).await, tickets(&rig, &child).await);

    rig.fleet.lifecycles_moved(record(&rig, &child).await).await;
    rig.fleet.lifecycles_moved(record(&rig, &child).await).await;

    assert_eq!((tickets(&rig, &parent).await, tickets(&rig, &child).await), before);
}

#[tokio::test]
async fn a_group_that_names_no_ticket_or_one_the_job_does_not_hold_hands_nothing() {
    let rig = rig_placing(None);
    let parent = rig.start().await;
    holds(&rig, &parent, "issue", "armada-95v", AttachmentState::Standing, &[]).await;
    rig.fleet.lifecycles_moved(record(&rig, &parent).await).await;
    recorded(
        &rig,
        plan(
            &parent,
            vec![
                group(None, "Tidy up", "rust-engineer", &[], &[(None, "Nothing named")]),
                group(None, "Fix armada-zzz", "rust-engineer", &[], &[(None, "Not held")]),
            ],
        ),
    )
    .await;

    let tidy = started_for(&rig, &parent, "g1").await;
    let elsewhere = started_for(&rig, &parent, "g2").await;

    assert_eq!(tickets(&rig, &parent).await, [ticket("armada-95v", false, "")]);
    assert!(tickets(&rig, &tidy).await.is_empty() && tickets(&rig, &elsewhere).await.is_empty());
}

#[tokio::test]
async fn a_job_started_with_no_group_hands_nothing() {
    let rig = rig_placing(None);
    let parent = planning(&rig, &["armada-95v"]).await;
    let child = rig.start().await;
    holds(&rig, &child, "started_by", parent.as_str(), AttachmentState::Spent, &[]).await;

    rig.fleet.lifecycles_moved(record(&rig, &child).await).await;

    assert_eq!(tickets(&rig, &parent).await, [ticket("armada-95v", false, "")]);
    assert!(tickets(&rig, &child).await.is_empty());
}

/// The tickets that hold the Job open, each as a person names it. **Not the Jobs it started**: the parent
/// of a handed group is held open by the Job doing it, which is what Oversee waits on.
async fn holding(rig: &Rig, id: &SessionId) -> Vec<String> {
    let store = rig.fleet.store().lock().await;
    rig.fleet.lifecycles_holding(&store, id.as_str()).into_iter().filter(|one| one.lifecycle == "ticket").map(|one| one.label).collect()
}

/// What a tracker with a fixed set of statuses declares for the ticket's states, as beads does.
const FIXED: &[(TicketState, &str)] = &[(TicketState::ToDo, "open"), (TicketState::InProgress, "in_progress"), (TicketState::Closed, "closed")];

#[tokio::test]
async fn once_handed_the_ticket_holds_the_started_job_open_and_no_longer_the_parent_and_is_read_for_the_started_job_alone() {
    let rig = rig_fitted(|fittings| fittings.trackers = Arc::new(FakeTrackers::empty().ticket_words(FIXED)));
    let parent = planning(&rig, &["armada-95v"]).await;
    rig.fleet.lifecycle_seen(parent.as_str(), "ticket", "armada-95v", Observed::Tracker("open")).await.expect("read");
    assert_eq!(holding(&rig, &parent).await, ["armada-95v"], "held open by it until it is handed");

    let child = started_for(&rig, &parent, "g1").await;
    assert!(holding(&rig, &parent).await.is_empty(), "the parent lets go of it");

    let moved = rig.fleet.lifecycle_seen(child.as_str(), "ticket", "armada-95v", Observed::Tracker("in_progress")).await;
    assert_eq!(moved.map(|one| one.moved.to), Some(Standing::In(StateId::new("in_progress").expect("a state"))), "the started Job reads it");
    assert_eq!(holding(&rig, &child).await, ["armada-95v"], "and it holds that Job open");
    assert!(
        rig.fleet.lifecycle_seen(parent.as_str(), "ticket", "armada-95v", Observed::Tracker("closed")).await.is_none(),
        "what the tracker says now moves nothing for the parent, whose instance stands where it was"
    );
}

#[tokio::test]
async fn the_ticket_a_job_handed_on_is_rebuilt_as_handed_for_that_job_and_the_started_job_holds_its_own() {
    let rig = rig_placing(None);
    let parent = planning(&rig, &["armada-95v", "armada-7gs"]).await;
    let child = started_for(&rig, &parent, "g1").await;

    let store = rig.fleet.store().lock().await;
    let lifecycles = Lifecycle::built_in();
    let handed = |id: &SessionId| -> Vec<(String, bool)> {
        let held = rebuilt(&lifecycles, &store.lifecycle_instances_of(id.as_str()).expect("reads"));
        let mut tickets: Vec<_> =
            held.all().iter().filter(|one| one.subject().lifecycle().as_str() == "ticket").map(|one| (one.subject().key().to_string(), one.is_handed())).collect();
        tickets.sort();
        tickets
    };
    assert_eq!(handed(&parent), [("armada-7gs".to_string(), false), ("armada-95v".to_string(), true)], "a rule over the parent's held tickets passes the one it gave");
    assert_eq!(handed(&child), [("armada-95v".to_string(), false)], "and acts on it for the Job that has it");
}
