//! A Command with `serve`, held by Fleet — issue #623's definition of done,
//! case by case, against real processes.
//!
//! **A real server and a real `ready`**: `python3 -m http.server` and `curl`,
//! because whether a port answers and is free again is what a fake cannot
//! stand in for — `crate::tests::ports_dispatch`'s reason, one module over.
//!
//! **Over 500 lines and one file**, for `crate::tests::rehearsing`'s reason:
//! every case stands on the one fixture above it — a Manifest, a Fleet on a
//! range of its own, a running Job — and split files would each import it to
//! assert one line of the definition of done.

use std::sync::Arc;

use api::{Next, Subscription};
use config::Manifest;
use ipc::{Event, ServerPhase, ServerState, StartedBy};
use testkit::{FakeHarness, FakeVcs, FakeWorkProduct};

use crate::checkouts::Checkout;
use crate::daemon::{Fittings, Fleet};
use crate::ports::{detect_ceiling, BindConnectProbe, PortProbe, PortRange};
use crate::servers::Unservable;
use crate::slots::Concurrency;
use crate::tests::daemon::fittings;
use crate::tests::tmp::TempDir;

pub(super) type Fixture = Fleet<FakeHarness, FakeVcs, FakeWorkProduct>;

/// `storybook` serves and answers; `falls_over` exits on its first line while
/// its `ready` never passes; `never_built`'s `run` fails, so its `serve` never
/// starts. `fmt` is a Command, and never a server.
pub(super) const MANIFEST: &str = r#"version: 1
id: 01FIXTUREMANIFEST
ports:
  storybook: {}
commands:
  storybook:
    serve: python3 -m http.server ${port.storybook} --bind 127.0.0.1
    ready: curl -sf http://127.0.0.1:${port.storybook}/
    links:
      - url: http://localhost:${port.storybook}
        name: Storybook
  falls_over:
    serve: "/bin/sh -c 'echo about to fall over; exit 7'"
    ready: /usr/bin/false
  never_built:
    run: "/bin/sh -c 'echo building; exit 2'"
    serve: python3 -m http.server ${port.storybook} --bind 127.0.0.1
  fmt:
    run: /usr/bin/true
"#;

pub(super) fn a_fleet_serving(home: &TempDir, events: &api::Broadcaster) -> Arc<Fixture> {
    a_fleet_holding(home, events, MANIFEST, MANIFEST)
}

/// `on_disk` is what a Job snapshots when it is created; `held` is what Fleet
/// runs on. **The two differ where `armada.yml` changed after a Job froze
/// it**, which is the case a Job's servers must not follow.
pub(super) fn a_fleet_holding(
    home: &TempDir,
    events: &api::Broadcaster,
    on_disk: &str,
    held: &str,
) -> Arc<Fixture> {
    Arc::new(Fleet::assembled(fittings_holding(
        home, events, on_disk, held,
    )))
}

/// [`a_fleet_holding`]'s fittings, for a case that changes one before assembly.
pub(super) fn fittings_holding(
    home: &TempDir,
    events: &api::Broadcaster,
    on_disk: &str,
    held: &str,
) -> Fittings<FakeHarness, FakeVcs, FakeWorkProduct> {
    let path = home.path().join("armada.yml");
    std::fs::write(&path, on_disk).expect("the file a Job snapshots");
    let mut fittings = fittings(home, FakeWorkProduct::changed(&["src/log.rs"]));
    fittings.starting().manifest = Manifest::parse(&path, held).expect("the Manifest Fleet holds");
    fittings.events = events.clone();
    fittings.concurrency = Concurrency::of(2);
    fittings.port_range = a_range_of_its_own();
    fittings
}

/// How many ports a fixture's range holds, and the floor of the band they are
/// taken from.
const SPAN: u16 = 20;
const BAND_FLOOR: u16 = 20_000;

/// Ports other fleet tests take by number: `tests::daemon`'s default range,
/// and `ports`', `listener`'s, `repositories`' and `workspace_ports`' own. A
/// span here is probed and bound by both sides with neither's lock.
const NAMED_BY_OTHER_TESTS: std::ops::RangeInclusive<u16> = 41_000..=44_999;

/// A range no other test is claiming from. **Each test is a process of its
/// own with a store of its own**, so two claiming from one range could be
/// handed one span, and the second server would find its port taken — which
/// one store per Fleet rules out everywhere but here.
///
/// **Below the platform's ephemeral floor, which is the rule Armada already
/// follows in production.** `settings.port-range-ceiling` defaults to *the
/// platform's ephemeral port floor minus one*, and `detect_ceiling` is what
/// reads it, "so nothing here hands out a port the kernel will also hand out".
/// This fixture did the exact opposite until 30 Sep 2026: it bound
/// `127.0.0.1:0`, took the ephemeral port the kernel handed back, and built
/// its range there — inside the band every other test's `bind(:0)` is served
/// from. It then dropped the listener, so even that one port was released
/// before use, and `base + 1 ..= base + 19` were never looked at at all.
///
/// `tests::preview::two_checkouts_of_one_repository_serve_at_once_on_their_own_spans`
/// starts two servers and asserts they take different ports, so it is the one
/// that reaches for the second: it refused the merge line three times on
/// 30 Sep with `it ended before serving … exit 1` and `Something else is
/// answering on port <n> now`, on ports 59053, 64669 and one before them —
/// every one of them above 49152, this machine's `net.inet.ip.portrange.first`.
/// It passed alone every time. 3,835 tests run in parallel here.
///
/// **The slot is picked from the process id and then walked**, so two test
/// processes rarely start from the same one.
///
/// **And it is locked before it is probed.** A span found free is let go
/// before the server binds it, so a probe alone let two processes take one
/// span: `it_stops_when_the_job_ends_before_its_span_goes_and_its_port_is_free`
/// failed 2 in 2,053 stress runs with python exiting 1 on port 40020.
pub(super) fn a_range_of_its_own() -> PortRange {
    let (base, _released) = a_span_held_below_the_floor();
    PortRange::of(base, base + SPAN - 1, 1)
}

/// The base of a free span below the ephemeral floor, locked to this process
/// for its life, with every port in it still bound by the caller.
fn a_span_held_below_the_floor() -> (u16, Vec<std::net::TcpListener>) {
    let top = detect_ceiling().saturating_sub(SPAN);
    let slots = top.saturating_sub(BAND_FLOOR) / SPAN;
    assert!(slots > 0, "no room for a span below {top}");
    let first = u16::try_from(std::process::id() % u32::from(slots)).unwrap_or(0);
    for step in 0..slots {
        let base = BAND_FLOOR + ((first + step) % slots) * SPAN;
        if NAMED_BY_OTHER_TESTS.contains(&base) || NAMED_BY_OTHER_TESTS.contains(&(base + SPAN - 1))
        {
            continue;
        }
        let Some(lock) = a_slot_locked(base) else {
            continue;
        };
        if let Some(held) = a_span_held(base) {
            kept_until_exit(lock);
            return (base, held);
        }
    }
    panic!("no run of {SPAN} free ports in {slots} slots below {top}");
}

/// An exclusive `flock` on this slot's file, or `None` where another test
/// process holds it. The kernel lets go when the holder exits, however it
/// exits, so a test that crashed leaves no slot taken.
fn a_slot_locked(base: u16) -> Option<std::fs::File> {
    let dir = std::env::temp_dir().join("armada-fleet-port-spans");
    std::fs::create_dir_all(&dir).expect("the directory slot locks live in");
    let file = std::fs::File::create(dir.join(format!("{base}.lock"))).expect("a slot's lock file");
    file.try_lock().ok().map(|()| file)
}

/// Hold a slot's lock until the process exits. Under nextest each test is a
/// process of its own, so that is the test's life; the fixture hands back only
/// the Fleet, and has nowhere shorter-lived to keep it.
fn kept_until_exit(lock: std::fs::File) {
    static KEPT: std::sync::Mutex<Vec<std::fs::File>> = std::sync::Mutex::new(Vec::new());
    KEPT.lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .push(lock);
}

/// Whether every port from `base` up holds nothing right now.
///
/// **Bound all at once and released all at once.** Taking each and releasing
/// it before taking the next is how the span comes back part-claimed by
/// somebody else while this is still counting.
fn a_span_is_free(base: u16) -> bool {
    a_span_held(base).is_some()
}

/// Every port from `base` up, bound, or `None` if any one of them is taken.
fn a_span_held(base: u16) -> Option<Vec<std::net::TcpListener>> {
    (0..SPAN)
        .map(|step| std::net::TcpListener::bind(("127.0.0.1", base + step)).ok())
        .collect()
}

/// **No deadline.** Python starting and `ready` passing took longer than 30s
/// only when the whole machine stalled for 30s, and that is not a failure of
/// anything these tests claim.
pub(super) async fn next_event(
    watching: &mut Subscription,
    wanted: impl Fn(&Event) -> bool,
) -> Event {
    loop {
        match watching.next().await {
            Some(Next::Send(delivered)) if wanted(&delivered.event) => return delivered.event,
            Some(_) => continue,
            None => panic!("the stream closed"),
        }
    }
}

/// This server serving — or a failure naming how it ended instead, rather
/// than a wait that runs out and says nothing.
pub(super) async fn serving(watching: &mut Subscription, id: &str) -> ServerState {
    match next_event(watching, |event| {
        matches!(
            event,
            Event::ServerServing(state) | Event::ServerExited(state) if state.id == id
        )
    })
    .await
    {
        Event::ServerServing(state) => state,
        Event::ServerExited(state) => panic!("it ended before serving: {state:?}"),
        _ => unreachable!("the filter admits only this server"),
    }
}

pub(super) async fn exited(watching: &mut Subscription, id: &str) -> ServerState {
    match next_event(
        watching,
        |event| matches!(event, Event::ServerExited(state) if state.id == id),
    )
    .await
    {
        Event::ServerExited(state) => state,
        _ => unreachable!("the filter admits only this server ending"),
    }
}

/// **A server started with no Job uses the main checkout's span and stops on
/// Stop**, and a second Stop says there is nothing left to stop.
#[tokio::test]
async fn a_server_with_no_job_uses_the_main_checkouts_span_and_stops_on_stop() {
    let home = TempDir::new();
    let events = api::Broadcaster::new();
    let fleet = a_fleet_serving(&home, &events);
    let port = *fleet
        .main_checkout_ports(&fleet.first())
        .await
        .get("storybook")
        .expect("the main checkout's span");
    let mut watching = events.subscribe();

    let (started, _) = Arc::clone(&fleet)
        .hold_server(
            Checkout::main(fleet.first()),
            "storybook",
            StartedBy::Person,
            None,
        )
        .await
        .expect("it starts");
    assert_eq!(started.ports.first().map(|one| one.port), Some(port));
    serving(&mut watching, &started.id).await;

    let stopped = fleet.stopped_server(&started.id).await.expect("it stops");
    assert!(stopped.stopped);
    assert_eq!(stopped.phase, ServerPhase::Exited);
    assert!(BindConnectProbe.free(port), "its port is free");
    assert!(matches!(
        fleet.stopped_server(&started.id).await,
        Err(Unservable::NotRunning { .. })
    ));
}

/// **The span is what is probed, not the base alone.** This is the fixture's
/// own claim rather than a Fleet one, and it exists because the old version of
/// `a_range_of_its_own` bound one port and claimed twenty: a port taken
/// anywhere above the base was invisible to it, and the server that later
/// reached for that port exited with `exit 1` in the middle of somebody else's
/// merge. Deterministic, unlike the failure it is for — the occupied port is
/// held here rather than waited for.
///
/// **The span is the test's own, below the ephemeral floor.** Built around a
/// `bind(:0)` port instead, the release half failed on the merge line (`the
/// span is free once 49395 is let go`): any process's next connection can be
/// handed a port in the ephemeral band.
#[test]
fn a_span_holding_one_taken_port_is_not_free() {
    let (base, mut held) = a_span_held_below_the_floor();

    // Above the base and inside the span, which is exactly what went unchecked.
    let taken = held.swap_remove(usize::from(SPAN / 2));
    let port = taken.local_addr().expect("its own address").port();
    drop(held);
    assert!(!a_span_is_free(base), "a span holding {port} is not free");

    // And the same span once it is released, so this is measuring the port
    // rather than something else about the range.
    drop(taken);
    assert!(
        a_span_is_free(base),
        "the span is free once {port} is let go"
    );
}

/// Every port in a range this hands out binds, and every one of them sits
/// below the floor the kernel hands ephemeral ports out from — which is the
/// half that stops another test's `bind(:0)` landing inside this span.
#[test]
fn a_range_of_its_own_hands_out_a_free_span_below_the_ephemeral_floor() {
    let range = a_range_of_its_own();
    assert!(
        range.ceiling() <= detect_ceiling(),
        "{} is not below the ephemeral floor",
        range.ceiling()
    );
    for port in range.base()..=range.ceiling() {
        std::net::TcpListener::bind(("127.0.0.1", port))
            .unwrap_or_else(|why| panic!("{port} in the handed-out range is taken: {why}"));
    }
}

/// **A span handed out stays its taker's**, so a second process walking to it
/// moves on rather than probing ports the first has not bound yet. A second
/// open of the lock file stands in for that process: `flock` is held per open,
/// not per process.
#[test]
fn a_span_handed_out_is_not_handed_out_again() {
    let (base, released) = a_span_held_below_the_floor();
    drop(released);
    assert!(
        a_slot_locked(base).is_none(),
        "{base} is free to bind, and still not free to take"
    );
}
