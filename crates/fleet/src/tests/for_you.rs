//! The claim, written before the code: **a Job's For you lists its own waiting items, read off what Fleet keeps, with
//! a sketch opening as its artifact and the agent's items under `waiting:`; placed rows come first in the order set
//! and the rest oldest first; a withdrawn row stays gone though the item stands; an id that is not a row is refused
//! by name; a row added for what is already listed is that row; the live Jobs it started are `building`; and a
//! change to what it reads tells `session.for_you`.** `crates/ipc/operations/get_for_you.toml`.

use std::sync::Arc;
use std::time::Duration;

use api::{HostedSessions, Next, Sessions, Subscription};
use ipc::{
    AddForYou, Event, ForYou, ForYouBy, ForYouKind, ForYouOpens, LineForYou, OrderForYou, SessionId, SessionTurn,
    SetWaitingFor, ShowSketch, WaitingInput, WithdrawForYou,
};

use super::agent_starts::{caller, handing, rig};
use super::page_actions::a_job;
use super::session_host::{eventually, rig_placing, Rig};

const SCENE: &str = r#"{"nodes":[{"id":"a","kind":"box","x":0,"y":0,"title":"Bay"}],"edges":[]}"#;

fn ask(id: &str, text: &str, approves: bool) -> WaitingInput {
    WaitingInput { id: id.into(), text: text.into(), act: None, options: Vec::new(), approves, steps: Vec::new(), artifact: None }
}

async fn states(rig: &Rig, id: &SessionId, items: Vec<WaitingInput>) {
    rig.fleet.waiting_for(None, SetWaitingFor { items, session_id: Some(id.clone()) }).await.expect("stated");
}

/// A sketch shown in the turn running, then the turn ended: a sketch waits on him only once its turn ends.
async fn sketches(rig: &Rig, id: &SessionId, at: usize) {
    let body = format!(r#"{{"title":"Bay flow","question":"Does this read right?","session_id":"{}","scene":{SCENE}}}"#, id.as_str());
    rig.fleet.show_sketch(None, ipc::decode::<ShowSketch>("a sketch to show", body.as_bytes()).unwrap()).await.unwrap();
    rig.stand_in.finishes(at, "Here it is.");
    eventually(|| async { !matches!(rig.turn(id).await, SessionTurn::Working { .. }) }).await;
}

async fn read(rig: &Rig, id: &SessionId) -> ForYou {
    Arc::clone(&rig.fleet).get_for_you(Some(id.clone())).await.expect("read")
}

fn ids(view: &ForYou) -> Vec<&str> {
    view.waiting.iter().map(|row| row.id.as_str()).collect()
}

/// A Job with an approval it stated, then a sketch shown after it.
async fn a_job_waiting(rig: &Rig) -> SessionId {
    let (id, at) = a_job(rig).await;
    states(rig, &id, vec![ask("plan-ok", "Approve the bay plan?", true)]).await;
    sketches(rig, &id, at).await;
    id
}

#[tokio::test]
async fn its_own_items_are_listed_oldest_first_each_opening_what_it_names() {
    let rig = rig_placing(None);
    let id = a_job_waiting(&rig).await;

    let view = read(&rig, &id).await;
    assert_eq!(view.job, id);
    assert_eq!(ids(&view), ["waiting:plan-ok", "sketch:Bay flow"]);
    let (agent, sketch) = (&view.waiting[0], &view.waiting[1]);
    assert_eq!((agent.kind, agent.added_by, agent.approves), (ForYouKind::Approve, ForYouBy::Agent, true));
    assert_eq!(agent.opens, ForYouOpens::Waiting { id: "plan-ok".into() });
    assert_eq!((sketch.kind, sketch.added_by), (ForYouKind::Sketch, ForYouBy::Fleet));
    assert_eq!(sketch.opens, ForYouOpens::Artifact { id: "sketch:Bay flow".into() });
    assert!(view.waiting.iter().all(|row| !row.placed && row.line.is_none()));
    assert!(view.building.is_empty() && view.landing.is_empty());

    states(&rig, &id, Vec::new()).await;
    assert_eq!(ids(&read(&rig, &id).await), ["sketch:Bay flow"], "an item gone from waiting_for leaves the tab");
}

#[tokio::test]
async fn an_agent_item_naming_an_artifact_opens_it_and_is_one_row_with_fleets() {
    let rig = rig_placing(None);
    let (id, at) = a_job(&rig).await;
    sketches(&rig, &id, at).await;
    states(&rig, &id, vec![WaitingInput { artifact: Some("Bay flow".into()), ..ask("bay", "Pick the bay flow", false) }]).await;

    let view = read(&rig, &id).await;
    assert_eq!(ids(&view), ["waiting:bay"], "the sketch and the item about it are one row");
    assert_eq!(view.waiting[0].opens, ForYouOpens::Artifact { id: "sketch:Bay flow".into() });
}

#[tokio::test]
async fn placed_rows_come_first_carry_their_line_and_a_withdrawn_one_stays_gone() {
    let rig = rig_placing(None);
    let id = a_job_waiting(&rig).await;
    let fleet = || Arc::clone(&rig.fleet);
    let session_id = Some(id.clone());

    let ordered = fleet().order_for_you(OrderForYou { session_id: session_id.clone(), ids: vec!["sketch:Bay flow".into()] }).await.unwrap();
    assert_eq!(ids(&ordered), ["sketch:Bay flow", "waiting:plan-ok"]);
    assert!(ordered.waiting[0].placed && !ordered.waiting[1].placed);

    let lined = fleet()
        .line_for_you(LineForYou { session_id: session_id.clone(), id: "sketch:Bay flow".into(), line: "  pick take A or B ".into() })
        .await
        .unwrap();
    assert_eq!(lined.waiting[0].line.as_deref(), Some("pick take A or B"));
    let cleared = fleet().line_for_you(LineForYou { session_id: session_id.clone(), id: "sketch:Bay flow".into(), line: " ".into() }).await.unwrap();
    assert_eq!(cleared.waiting[0].line, None, "a blank line clears it");

    let withdrawn = fleet().withdraw_for_you(WithdrawForYou { session_id: session_id.clone(), id: "waiting:plan-ok".into() }).await.unwrap();
    assert_eq!(ids(&withdrawn), ["sketch:Bay flow"]);
    states(&rig, &id, vec![ask("plan-ok", "Approve the bay plan?", true)]).await;
    assert_eq!(ids(&read(&rig, &id).await), ["sketch:Bay flow"], "stated again, it stays off the tab");
    fleet().withdraw_for_you(WithdrawForYou { session_id, id: "nothing:here".into() }).await.expect("a missing id is not refused");
}

#[tokio::test]
async fn an_id_that_is_not_a_row_is_refused_by_name_and_nothing_changes() {
    let rig = rig_placing(None);
    let id = a_job_waiting(&rig).await;
    let order = OrderForYou { session_id: Some(id.clone()), ids: vec!["sketch:Bay flow".into(), "walk:http://nowhere".into()] };
    let refused = Arc::clone(&rig.fleet).order_for_you(order).await.expect_err("refused");
    assert_eq!(refused.error().code, "fleet.for_you_unknown");
    assert!(refused.error().message.contains("walk:http://nowhere"), "{}", refused.error().message);
    assert!(read(&rig, &id).await.waiting.iter().all(|row| !row.placed), "refused whole");

    let line = LineForYou { session_id: Some(id.clone()), id: "added:9".into(), line: "why".into() };
    assert_eq!(Arc::clone(&rig.fleet).line_for_you(line).await.expect_err("refused").error().code, "fleet.for_you_unknown");
}

#[tokio::test]
async fn a_row_added_for_what_is_listed_is_that_row_and_a_new_one_is_kept() {
    let rig = rig_placing(None);
    let id = a_job_waiting(&rig).await;
    let add = |title: &str, opens: ForYouOpens, line: Option<&str>| AddForYou {
        session_id: Some(id.clone()),
        title: title.into(),
        kind: ForYouKind::Sketch,
        opens,
        line: line.map(str::to_string),
    };

    let same = Arc::clone(&rig.fleet)
        .add_for_you(add("The bay sketch", ForYouOpens::Artifact { id: "sketch:Bay flow".into() }, Some("a look only")))
        .await
        .unwrap();
    assert_eq!(same.id, "sketch:Bay flow", "one thing is one row");
    let new = Arc::clone(&rig.fleet).add_for_you(add("Read the deploy notes", ForYouOpens::Walk { url: "http://localhost:4000".into() }, None)).await.unwrap();
    assert_eq!(new.id, "added:1");

    let view = read(&rig, &id).await;
    assert_eq!(ids(&view), ["waiting:plan-ok", "sketch:Bay flow", "added:1"]);
    assert_eq!(view.waiting[1].line.as_deref(), Some("a look only"));
    assert_eq!((view.waiting[2].added_by, view.waiting[2].title.as_str()), (ForYouBy::Agent, "Read the deploy notes"));

    let blank = Arc::clone(&rig.fleet).add_for_you(add("  ", ForYouOpens::Plan, None)).await.expect_err("refused");
    assert_eq!(blank.error().code, "fleet.for_you_blank");
}

async fn told_for_you(watching: &mut Subscription, id: &SessionId) {
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            match watching.next().await {
                Some(Next::Send(delivered)) => {
                    if matches!(&delivered.event, Event::SessionForYou(changed) if changed.session_id == *id) {
                        return;
                    }
                }
                other => panic!("the stream ended or dropped: {other:?}"),
            }
        }
    })
    .await
    .expect("a session.for_you arrives");
}

#[tokio::test]
async fn a_change_to_its_waiting_items_tells_session_for_you() {
    let rig = rig_placing(None);
    let (id, _) = a_job(&rig).await;
    let mut watching = rig.fleet.events().subscribe();
    states(&rig, &id, vec![ask("q1", "Sort by name or by version?", false)]).await;
    told_for_you(&mut watching, &id).await;
}

#[tokio::test]
async fn the_live_jobs_it_started_are_building() {
    let rig = rig(None);
    let parent = rig.a_parent_with_a_plan().await;
    let held = Arc::clone(&rig.fleet).start_another_session(caller(), handing(&rig, "g1")).await.expect("held");

    let view = Arc::clone(&rig.fleet).get_for_you(Some(parent.clone())).await.expect("read");
    assert_eq!(view.building.iter().map(|one| one.job.clone()).collect::<Vec<_>>(), [held.session_id.clone()]);
}
