//! A plan act a Helm session makes at the agent door, signed as Helm's. `#2168`.
//!
//! This was written first, against the route as it stood, and failed on
//! `left: Human, right: Helm`.
//!
//! **Through the real router**, so the mark the door leaves is what Fleet reads:
//! a Helm session's `add_task` used to come out of the same route as Bridge's
//! and was kept as the owner's.

use std::net::SocketAddr;
use std::pin::Pin;
use std::future::Future;
use std::sync::Arc;

use axum::body::Body;
use axum::http::Request;
use axum::Router;
use core_model::{Approach, NewTask, PlanChange, StepId, Timestamp};
use http_body_util::BodyExt;
use ipc::RunId;
use store::PlanHand;
use testkit::FakeWorkProduct;
use tower::ServiceExt;

use crate::daemon::Fleet;
use crate::helm::{Carry, Carrying, Heard, Hosting};
use crate::tests::daemon::{a_proposal, fittings};
use crate::tests::peer::Placing;
use crate::tests::tmp::TempDir;

const SESSION: u32 = 4242;
const RELAY: u16 = 52000;

struct Running;

impl Hosting for Running {
    fn carry<'a>(&'a self, _carry: Carry, _heard: &'a dyn Heard) -> Carrying<'a> {
        Box::pin(std::future::pending()) as Pin<Box<dyn Future<Output = _> + Send>>
    }

    fn model(&self) -> String {
        String::from("a-model")
    }

    fn running(&self) -> Vec<u32> {
        vec![SESSION]
    }
}

type Fixture = Fleet<testkit::FakeHarness, testkit::FakeVcs, FakeWorkProduct>;

fn served(home: &TempDir) -> (Arc<Fixture>, Router) {
    let mut fittings = fittings(home, FakeWorkProduct::changed(&[]));
    let peers = Placing::nothing();
    peers.holding(SESSION + 1, RELAY, fittings.host.port);
    peers.started(SESSION, SESSION + 1);
    fittings.peers = peers;
    let fleet = Arc::new(Fleet::assembled(fittings).hosting_helm_on(Arc::new(Running)));
    let events = fleet.events();
    let app = api::router(api::Served::sharing(
        Arc::clone(&fleet),
        RunId::carried("01RUN"),
        events,
    ));
    (fleet, app)
}

async fn a_plan_of_two(fleet: &Fixture) -> core_model::JobId {
    let job = fleet
        .propose(a_proposal("cover the writer's bound"))
        .await
        .expect("a Job at the gate");
    fleet
        .store()
        .lock()
        .await
        .change_plan(
            job.id(),
            &PlanChange::Recorded {
                approach: Approach::new("Bound the reader").expect("an approach"),
                tasks: vec![
                    NewTask::new("Stop at the end", "", &[], "").expect("a title"),
                    NewTask::new("Cover it", "", &[], "").expect("a title"),
                ],
            },
            PlanHand::Step(&StepId::new("implement")),
            &Timestamp::from_rfc3339("2026-10-12T10:00:00.000Z"),
        )
        .expect("recorded");
    job.id().clone()
}

async fn from_helm(app: &Router, tool: &str, arguments: &str) -> String {
    let peer: SocketAddr = format!("127.0.0.1:{RELAY}").parse().expect("an address");
    let call = format!(
        r#"{{"jsonrpc":"2.0","id":7,"method":"tools/call","params":{{"name":"{tool}","arguments":{arguments}}}}}"#
    );
    let request = Request::builder()
        .method("POST")
        .uri(api::DOOR_PATH)
        .header("content-type", "application/json")
        .extension(axum::extract::ConnectInfo(peer))
        .body(Body::from(call))
        .expect("a well-formed request");
    let response = app.clone().oneshot(request).await.expect("an answer");
    let body = response.into_body().collect().await.expect("a body").to_bytes();
    String::from_utf8_lossy(&body).to_string()
}

#[tokio::test]
async fn a_task_a_helm_session_adds_is_signed_as_helms() {
    let home = TempDir::new();
    let (fleet, app) = served(&home);
    let job = a_plan_of_two(&fleet).await;
    let mut watching = fleet.events().subscribe();

    let answered = from_helm(
        &app,
        "add_task",
        &format!(
            r#"{{"job_id":"{}","body":{{"title":"Cover the writer","note":"","scope":[],"expects":"","after":""}}}}"#,
            job.as_str()
        ),
    )
    .await;
    assert!(answered.contains("\"isError\":false"), "{answered}");

    let mut published = None;
    while let Ok(Some(api::Next::Send(delivered))) =
        tokio::time::timeout(std::time::Duration::from_millis(30), watching.next()).await
    {
        if let ipc::Event::JobPlanChanged(changed) = delivered.event {
            published = Some(changed);
        }
    }
    let published = published.expect("the add was told");
    assert_eq!(published.actor.domain(), core_model::Actor::Helm);

    let history = fleet
        .store()
        .lock()
        .await
        .plan_history(&job)
        .expect("the history");
    assert_eq!(
        history.last().expect("the add").by,
        core_model::PlanAuthor::Helm
    );
    let plan = fleet.plan_of(&job).await.expect("reads").expect("a plan");
    assert_eq!(
        ipc::WorkPlan::from(&plan).tasks[2].by,
        Some(ipc::TaskBy::Helm),
        "the new task wears Helm's hand on the wire"
    );
}

/// The same door, a task dropped.
#[tokio::test]
async fn a_task_a_helm_session_drops_is_signed_as_helms() {
    let home = TempDir::new();
    let (fleet, app) = served(&home);
    let job = a_plan_of_two(&fleet).await;

    let answered = from_helm(
        &app,
        "drop_task",
        &format!(
            r#"{{"job_id":"{}","body":{{"task":"T2","reason":"covered by T1"}}}}"#,
            job.as_str()
        ),
    )
    .await;
    assert!(answered.contains("\"isError\":false"), "{answered}");

    let history = fleet
        .store()
        .lock()
        .await
        .plan_history(&job)
        .expect("the history");
    assert_eq!(
        history.last().expect("the drop").by,
        core_model::PlanAuthor::Helm
    );
}
