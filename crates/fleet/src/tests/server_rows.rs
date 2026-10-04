//! A server's row written from two threads at once: by the server's own task,
//! and by a merge that moved the checkout it serves.
//!
//! **Made deterministic through the clock**, as `super::preview`'s
//! `work_landing_as_the_server_comes_up_is_still_counted` is: the task's
//! reading is stopped, the merge lands, and the task is let go.

use std::sync::Arc;

use adapter_traits::RepositoryStanding;
use api::{Next, Subscription};
use ipc::{Event, ServerPhase, ServerState, StartedBy};

use crate::checkouts::Checkout;
use crate::servers::Place;
use crate::tests::preview::{up_or_ended, StopsTheServersReading, HELD_WITHOUT_A_PORT};
use crate::tests::servers::fittings_holding;
use crate::tests::tmp::TempDir;

/// **A merge landing as a server ends is on the row it ended with.** The task
/// reads the clock for `ended_at` on its way out, which is where the merge is
/// made to land.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn work_landing_as_the_server_ends_is_still_counted() {
    let home = TempDir::new();
    let events = api::Broadcaster::new();
    let clock = Arc::new(StopsTheServersReading {
        ticking: crate::tests::planted::Ticking::from_nine(),
        armed: std::sync::Mutex::new(None),
        reached: std::sync::Barrier::new(2),
        released: std::sync::Barrier::new(2),
    });
    let mut fittings = fittings_holding(&home, &events, HELD_WITHOUT_A_PORT, HELD_WITHOUT_A_PORT);
    fittings.clock = Arc::clone(&clock) as Arc<dyn crate::clock::Clock>;
    let fleet = Arc::new(crate::daemon::Fleet::assembled(fittings));
    let mut watching = events.subscribe();
    let root = fleet.first().root().to_string();

    let (started, _) = Arc::clone(&fleet)
        .hold_server(
            Place::Checkout(Checkout::main(fleet.first())),
            "idle",
            StartedBy::Person,
            false,
        )
        .await
        .expect("it starts");
    up_or_ended(&mut watching, &started.id).await;

    *clock.armed.lock().expect("unpoisoned") = Some(std::thread::current().id());
    let (stop, _) = fleet.servers().stopping(&started.id).expect("it is up");
    let _ = stop.send(true);
    let waiting = Arc::clone(&clock);
    tokio::task::spawn_blocking(move || waiting.reached.wait())
        .await
        .expect("the server's task reached its reading");

    fleet.told_servers_the_checkout_moved(
        &root,
        &RepositoryStanding::MovedOn {
            base: String::from("main"),
            commits: 3,
            head: String::from("cf4bcaea"),
        },
    );
    let letting_go = Arc::clone(&clock);
    tokio::task::spawn_blocking(move || letting_go.released.wait())
        .await
        .expect("the server's task let go");
    let exited = exited(&mut watching, &started.id).await;
    assert_eq!(exited.checkout.behind, Some(3), "the merge was dropped");

    let row = fleet
        .server_list()
        .servers
        .into_iter()
        .find(|state| state.id == started.id)
        .expect("kept as the last that ended");
    assert_eq!(row.phase, ServerPhase::Exited);
    assert_eq!(row.checkout.behind, Some(3), "the merge was dropped");
}

/// This server's `server.exited`. **No deadline**, for `up_or_ended`'s reason.
async fn exited(watching: &mut Subscription, id: &str) -> ServerState {
    loop {
        match watching.next().await {
            Some(Next::Send(delivered)) => match delivered.event {
                Event::ServerExited(state) if state.id == id => return state,
                _ => continue,
            },
            Some(_) => continue,
            None => panic!("the stream closed"),
        }
    }
}
